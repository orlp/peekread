use std::io::{BufRead, Error, ErrorKind, Read, Result};

use crate::{PeekRead, PeekSeekFrom, UnreadPeekCursor};

/// A type implementing the functionality of PeekRead akin
/// to how BufReader implements BufRead.
pub struct PeekReader<R> {
    // Where we store the peeked but not yet read data.
    // This data lives in the buffer buf_storage[buf_begin..].
    // We can thus have free space at the front.
    buf_storage: Vec<u8>,
    buf_begin: usize,
    desired_front_space: usize,

    peek_pos: usize, // This is relative to the read pointer.
    inner: R,
}

impl<R: Read> PeekReader<R> {
    const MIN_READ: usize = 8 * 1024;

    pub fn new(reader: R) -> Self {
        Self {
            buf_storage: Vec::new(),
            buf_begin: 0,
            desired_front_space: 32,
            peek_pos: 0,
            inner: reader,
        }
    }

    fn buffer(&self) -> &[u8] {
        &self.buf_storage[self.buf_begin..]
    }

    // Try to fill the buffer so that it's at least nbytes in length
    // (may fail to do so if EOF is reached - no error is reported then).
    fn request_buffer(&mut self, nbytes: usize) -> Result<()> {
        let nbytes_needed = nbytes.saturating_sub(self.buffer().len());
        if nbytes_needed > 0 {
            self.request_space_at_end();
            let read_size = nbytes_needed.max(Self::MIN_READ);
            self.inner
                .by_ref()
                .take(read_size as u64)
                .read_to_end(&mut self.buf_storage)?;
        }
        Ok(())
    }

    // Ensure the buffer to be at least nbytes in length.
    fn ensure_buffer(&mut self, nbytes: usize) -> Result<()> {
        self.request_buffer(nbytes)?;
        if self.buffer().len() < nbytes {
            Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill peek buffer"))
        } else {
            Ok(())
        }
    }

    fn request_space_at_end(&mut self) {
        // If our capacity is at least half unused (and sufficiently big),
        // move the elements back to the start.
        let cap = self.buf_storage.capacity();
        if cap >= 3 * Self::MIN_READ && self.buf_begin >= self.buf_storage.capacity() / 2 {
            // Shrink desired front space a bit since we're relatively lacking space at the end.
            self.desired_front_space = self.desired_front_space.min(self.buf_begin) * 2 / 3;
            self.buf_storage.drain(self.desired_front_space..self.buf_begin);
            self.buf_begin = 0;
        }
    }

    fn ensure_space_at_front(&mut self, nbytes: usize) {
        if nbytes > self.buf_begin {
            // Not enough space at the front, we have to reallocate or move elements. To prevent
            // this from occurring all the time when doing big unreads, increase our desired front space.
            self.desired_front_space = 2 * self.desired_front_space.max(nbytes) + 32;
            let extra_front_space = self.desired_front_space - self.buf_begin;
            self.buf_storage.splice(0..0, std::iter::repeat(0).take(extra_front_space));
            self.buf_begin += extra_front_space;
        }
    }
}

impl<R: Read> PeekRead for PeekReader<R> {
    fn peek(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.request_buffer(self.peek_pos + buf.len().min(Self::MIN_READ))?;
        let mut peek_buffer = self.buffer().get(self.peek_pos..).unwrap_or_default();
        let written = peek_buffer.read(buf).unwrap(); // Can't fail.
        self.peek_pos += written;
        Ok(written)
    }

    fn peek_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.ensure_buffer(self.peek_pos + buf.len())?;
        let mut peek_buffer = &self.buffer()[self.peek_pos..];
        self.peek_pos += peek_buffer.read(buf).unwrap(); // Can't fail.
        Ok(())
    }

    fn peek_position(&self) -> usize {
        self.peek_pos
    }

    fn peek_seek(&mut self, pos: PeekSeekFrom) -> Result<usize> {
        match pos {
            PeekSeekFrom::ReadCursor(offset) => self.peek_pos = offset as usize,
            PeekSeekFrom::Current(offset) => self.peek_pos = (self.peek_pos as isize + offset).max(0) as usize,
            PeekSeekFrom::End(offset) => {
                self.request_buffer(usize::MAX)?;
                self.peek_pos = (self.buffer().len() as isize + offset).max(0) as usize;
            }
        }
        Ok(self.peek_pos)
    }

    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor) {
        let n = data.len();
        self.ensure_space_at_front(n);
        if peek_cursor_behavior == UnreadPeekCursor::Fixed ||
            peek_cursor_behavior == UnreadPeekCursor::ShiftIfZero && self.peek_pos > 0 {
            self.peek_pos += n;
        }
        self.buf_begin -= n;
        self.buf_storage[self.buf_begin..self.buf_begin+n].copy_from_slice(data)
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.buffer().is_empty() && buf.len() >= self.buf_storage.capacity() {
            return self.inner.read(buf);
        }
        let written = self.fill_buf()?.read(buf).unwrap(); // Can't fail.
        self.consume(written);
        Ok(written)
    }
}

impl<R: Read> BufRead for PeekReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.buffer().is_empty() {
            self.buf_begin = 0;
            self.buf_storage.clear();
            self.inner
                .by_ref()
                .take(Self::MIN_READ as u64)
                .read_to_end(&mut self.buf_storage)?;
        }

        Ok(self.buffer())
    }

    fn consume(&mut self, amt: usize) {
        self.buf_begin = (self.buf_begin + amt).min(self.buf_storage.len());
        self.peek_pos = self.peek_pos.saturating_sub(amt);
    }
}

