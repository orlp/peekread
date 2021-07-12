use std::io::*;

use crate::{PeekRead, PeekCursor, detail::{PeekReadImpl, PeekCursorState}};

/// A wrapper for a [`Read`] stream that implements [`PeekRead`] using a buffer to store peeked data.
#[derive(Debug)]
pub struct BufPeekReader<R> {
    // Where we store the peeked but not yet read data.
    // This data lives in the buffer buf_storage[buf_begin..].
    // We can thus have free space at the front.
    buf_storage: Vec<u8>,
    buf_begin: usize,

    desired_front_space: usize,
    min_read_size: usize,

    inner: R,
}

impl<R: Read> BufPeekReader<R> {
    const MIN_RECLAIM_SIZE: usize = 1024 * 20;

    /// Creates a new [`BufPeekReader`].
    pub fn new(reader: R) -> Self {
        Self {
            buf_storage: Vec::new(),
            buf_begin: 0,
            desired_front_space: 32,
            min_read_size: 0,
            inner: reader,
        }
    }

    /// Pushes the given data into the stream at the front, pushing the read cursor back.
    pub fn unread(&mut self, data: &[u8]) {
        let n = data.len();
        self.ensure_space_at_front(n);
        self.buf_begin -= n;
        self.buf_storage[self.buf_begin..self.buf_begin + n].copy_from_slice(data)
    }

    /// Sets the minimum size used when reading from the underlying stream. Setting this allows
    /// for efficient buffered reads on any stream similar to [`BufReader`], but is disabled
    /// by default since doing bigger reads than requested might unnecessarily block.
    pub fn set_min_read_size(&mut self, nbytes: usize) {
        self.min_read_size = nbytes;
    }

    /// Gets the minimum read size. See [`Self::set_min_read_size`].
    pub fn min_read_size(&self) -> usize {
        self.min_read_size
    }

    /// Returns a reference to the internally buffered data.
    ///
    /// Unlike [`BufRead::fill_buf`], this will not attempt to fill the buffer if it is empty.
    pub fn buffer(&self) -> &[u8] {
        &self.buf_storage[self.buf_begin..]
    }

    /// Gets a reference to the underlying reader.
    ///
    /// It is inadvisable to directly read from the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Gets a mutable reference to the underlying reader.
    ///
    /// It is inadvisable to directly read from the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Unwraps this `BufPeekReader<R>`, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner
    }

    // Try to fill the buffer so that it's at least nbytes in length
    // (may fail to do so if EOF is reached - no error is reported then).
    fn request_buffer(&mut self, nbytes: usize) -> Result<()> {
        let nbytes_needed = nbytes.saturating_sub(self.buffer().len());
        if nbytes_needed > 0 {
            self.reclaim_space_from_front();
            let read_size = nbytes_needed.max(self.min_read_size);
            self.buf_storage.reserve(read_size);
            self.inner
                .by_ref()
                .take(read_size as u64)
                .read_to_end(&mut self.buf_storage)?;
        }
        Ok(())
    }

    fn reclaim_space_from_front(&mut self) {
        // If our capacity is at least half unused (and sufficiently big),
        // move the elements back to the start.
        let cap = self.buf_storage.capacity();
        if cap >= Self::MIN_RECLAIM_SIZE && self.buf_begin >= self.buf_storage.capacity() / 2 {
            // Shrink desired front space a bit since we're relatively lacking space at the end.
            self.desired_front_space = self.desired_front_space * 2 / 3;
            let front_space = self.desired_front_space.min(self.buf_begin);
            self.buf_storage
                .drain(front_space..self.buf_begin);
            self.buf_begin = front_space;
        }
    }

    fn ensure_space_at_front(&mut self, nbytes: usize) {
        if nbytes > self.buf_begin {
            // Not enough space at the front, we have to reallocate or move elements. To prevent
            // this from occurring all the time when doing big unreads, increase our desired front space.
            self.desired_front_space = (2 * self.desired_front_space).max(nbytes) + 32;
            let extra_front_space = self.desired_front_space - self.buf_begin;
            self.buf_storage
                .splice(0..0, std::iter::repeat(0).take(extra_front_space));
            self.buf_begin += extra_front_space;
        }
    }
}

impl<R: Read> PeekRead for BufPeekReader<R> {
    fn peek(&mut self) -> PeekCursor<'_> {
        PeekCursor::new(self)
    }
}

impl<R: Read> PeekReadImpl for BufPeekReader<R> {
    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        self.request_buffer(state.peek_pos as usize + buf.len())?;
        let mut peek_buffer = self.buffer().get(state.peek_pos as usize..).unwrap_or_default();
        let written = peek_buffer.read(buf).unwrap(); // Can't fail.
        state.peek_pos += written as u64;
        Ok(written)
    }

    fn peek_fill_buf(&mut self, state: &mut PeekCursorState) -> Result<&[u8]> {
        self.request_buffer(state.peek_pos as usize + 1)?;
        Ok(self.buffer().get(state.peek_pos as usize..).unwrap_or_default())
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        state.peek_pos += amt as u64;
    }

    fn peek_read_exact(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<()> {
        self.request_buffer(state.peek_pos as usize + buf.len())?;
        let mut peek_buffer = self.buffer().get(state.peek_pos as usize..).unwrap_or_default();
        let written = peek_buffer.read_exact(buf)?;
        state.peek_pos += buf.len() as u64;
        Ok(written)
    }

    fn peek_stream_position(&mut self, state: &mut PeekCursorState) -> Result<u64> {
        Ok(state.peek_pos)
    }

    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        match pos {
            SeekFrom::Start(offset) => state.peek_pos = offset,
            SeekFrom::Current(offset) => {
                state.peek_pos = (state.peek_pos as i64 + offset).max(0) as u64
            }
            SeekFrom::End(offset) => {
                self.request_buffer(usize::MAX)?;
                state.peek_pos = (self.buffer().len() as i64 + offset).max(0) as u64;
            }
        }
        Ok(state.peek_pos)
    }
}

impl<R: Read> Read for BufPeekReader<R> {
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

impl<R: Read> BufRead for BufPeekReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.buffer().is_empty() {
            self.buf_begin = 0;
            self.buf_storage.clear();
            self.inner
                .by_ref()
                .take(self.min_read_size as u64)
                .read_to_end(&mut self.buf_storage)?;
        }

        Ok(self.buffer())
    }

    fn consume(&mut self, amt: usize) {
        self.buf_begin = (self.buf_begin + amt).min(self.buf_storage.len());
    }
}
