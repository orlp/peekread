use std::io::{BufRead, Error, ErrorKind, Read, Result};

use crate::PeekRead;

pub struct PeekReader<R> {
    buf_storage: Vec<u8>,
    buf_begin: usize,
    peek_pos: usize,
    inner: R,
}

impl<R: Read> PeekReader<R> {
    const MIN_READ: usize = 8 * 1024;
    const MIN_GC_SIZE: usize = 3 * Self::MIN_READ;

    pub fn new(reader: R) -> Self {
        Self {
            buf_storage: Vec::new(),
            buf_begin: 0,
            peek_pos: 0,
            inner: reader,
        }
    }

    fn buffer(&self) -> &[u8] {
        &self.buf_storage[self.buf_begin..]
    }

    fn ensure_buffer_size(&mut self, nbytes: usize) -> Result<()> {
        let nbytes_needed = nbytes.saturating_sub(self.buffer().len());
        if nbytes_needed > 0 {
            self.gc_buffer();
            let read_size = nbytes_needed.max(Self::MIN_READ);
            self.inner
                .by_ref()
                .take(read_size as u64)
                .read_to_end(&mut self.buf_storage)?;
        }
        Ok(())
    }

    fn gc_buffer(&mut self) {
        // If our capacity is at least half unused (and sufficiently big),
        // move the elements back to the start.
        let cap = self.buf_storage.capacity();
        if cap >= Self::MIN_GC_SIZE && self.buf_begin >= self.buf_storage.capacity() / 2 {
            self.buf_storage.drain(0..self.buf_begin);
            self.buf_begin = 0;
        }
    }
}

impl<R: Read> PeekRead for PeekReader<R> {
    fn peek(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.ensure_buffer_size(self.peek_pos + buf.len().min(Self::MIN_READ))?;
        let written = (&self.buffer()[self.peek_pos..]).read(buf).unwrap(); // Can't fail.
        self.peek_pos += written;
        Ok(written)
    }

    fn peek_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.ensure_buffer_size(self.peek_pos + buf.len())?;
        self.peek_pos += (&self.buffer()[self.peek_pos..]).read(buf).unwrap(); // Can't fail.
        Ok(())
    }

    fn peek_position(&self) -> usize {
        self.peek_pos
    }

    fn set_peek_position(&mut self, pos: usize) {
        self.peek_pos = pos;
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.buffer().len() == 0 && buf.len() >= self.buf_storage.capacity() {
            return self.inner.read(buf);
        }
        let written = self.fill_buf()?.read(buf).unwrap(); // Can't fail.
        self.consume(written);
        Ok(written)
    }
}

impl<R: Read> BufRead for PeekReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.buffer().len() == 0 {
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
