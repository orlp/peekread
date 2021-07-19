use std::collections::VecDeque;
#[cfg(doc)]
use std::io::BufReader;
use std::io::{BufRead, Read, Result, SeekFrom};

use crate::util::seek_add_offset;
use crate::{
    detail::{PeekCursorState, PeekReadImpl},
    PeekCursor, PeekRead,
};

/// A wrapper for a [`Read`] stream that implements [`PeekRead`] using a buffer to store peeked data.
#[derive(Debug)]
pub struct BufPeekReader<R> {
    // Where we store the peeked but not yet read data.
    buf_storage: VecDeque<u8>,
    // A vec used for temporary storage.
    tmp: Vec<u8>,
    min_read_size: usize,
    inner: R,
}

impl<R: Read> BufPeekReader<R> {
    const MIN_READ_TO_END: usize = 32;

    /// Creates a new [`BufPeekReader`].
    pub fn new(reader: R) -> Self {
        Self {
            buf_storage: VecDeque::new(),
            tmp: Vec::new(),
            min_read_size: 0,
            inner: reader,
        }
    }

    /// Pushes the given data into the stream at the front, pushing the read cursor back.
    pub fn unread(&mut self, data: &[u8]) {
        self.buf_storage.reserve(data.len());
        for byte in data.iter().copied().rev() {
            self.buf_storage.push_front(byte);
        }
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
    pub fn buffer(&self) -> &VecDeque<u8> {
        &self.buf_storage
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
        let nbytes_needed = nbytes.saturating_sub(self.buf_storage.len());
        if nbytes_needed > 0 {
            let read_size = nbytes_needed.max(self.min_read_size);
            self.inner
                .by_ref()
                .take(read_size as u64)
                .read_to_end(&mut self.tmp)?;
            self.buf_storage.reserve(self.tmp.len());
            self.buf_storage.extend(self.tmp.drain(..));
        }
        Ok(())
    }

    // The buffered data starting from the peek position as two slices.
    fn peek_slices(&self, peek_pos: usize) -> (&[u8], &[u8]) {
        let (a, b) = self.buf_storage.as_slices();
        let first = a.get(peek_pos..).unwrap_or_default();
        let second = b
            .get(peek_pos.saturating_sub(a.len())..)
            .unwrap_or_default();
        (first, second)
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
        let (mut first, mut second) = self.peek_slices(state.peek_pos as usize);
        let mut written = first.read(buf).unwrap(); // Can't fail.
        written += second.read(&mut buf[written..]).unwrap(); // Can't fail.
        state.peek_pos += written as u64;
        Ok(written)
    }

    fn peek_fill_buf(&mut self, state: &mut PeekCursorState) -> Result<&[u8]> {
        self.request_buffer(state.peek_pos as usize + 1)?;
        let (first, second) = self.peek_slices(state.peek_pos as usize);
        if !first.is_empty() {
            Ok(first)
        } else {
            Ok(second)
        }
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        state.peek_pos += amt as u64;
    }

    fn peek_read_exact(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<()> {
        self.request_buffer(state.peek_pos as usize + buf.len())?;
        let (mut first, mut second) = self.peek_slices(state.peek_pos as usize);
        let written = first.read(buf).unwrap(); // Can't fail.
        second.read_exact(&mut buf[written..])?;
        state.peek_pos += buf.len() as u64;
        Ok(())
    }

    fn peek_stream_position(&mut self, state: &mut PeekCursorState) -> Result<u64> {
        Ok(state.peek_pos)
    }

    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        match pos {
            SeekFrom::Start(offset) => state.peek_pos = offset,
            SeekFrom::Current(offset) => {
                state.peek_pos = seek_add_offset(state.peek_pos, offset)?;
            }
            SeekFrom::End(offset) => {
                let mut requested_buffer_size = self.buf_storage.len();
                while self.buf_storage.len() == requested_buffer_size {
                    requested_buffer_size = (requested_buffer_size * 2).max(Self::MIN_READ_TO_END);
                    self.request_buffer(requested_buffer_size)?;
                }
                state.peek_pos = seek_add_offset(self.buf_storage.len() as u64, offset)?;
            }
        }
        Ok(state.peek_pos)
    }
}

impl<R: Read> Read for BufPeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let (mut first, mut second) = self.buf_storage.as_slices();
        let mut written = first.read(buf).unwrap(); // Can't fail.
        written += second.read(&mut buf[written..]).unwrap(); // Can't fail.
        self.inner.read(&mut buf[written..]).map(|inner_written| {
            self.consume(written);
            written + inner_written
        })
    }


    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let (mut first, mut second) = self.buf_storage.as_slices();
        let mut written = first.read(buf).unwrap(); // Can't fail.
        written += second.read(&mut buf[written..])?; // Can't fail.
        self.inner
            .read_exact(&mut buf[written..])
            .map(|_| self.consume(buf.len()))
    }
}

impl<R: Read> BufRead for BufPeekReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.request_buffer(self.min_read_size)?;
        let (first, second) = self.buf_storage.as_slices();
        if !first.is_empty() {
            Ok(first)
        } else {
            Ok(second)
        }
    }

    fn consume(&mut self, amt: usize) {
        for _ in 0..amt.min(self.buf_storage.len()) {
            self.buf_storage.pop_front();
        }
    }
}
