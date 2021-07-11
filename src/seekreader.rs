use crate::{PeekRead, PeekCursor, detail::{PeekReadImpl, PeekCursorState}};
use crate::util::add_offset;
use std::io::*;

/// A wrapper for a [`Read`] + [`Seek`] stream that implements [`PeekRead`] using seeking.
#[derive(Debug)]
pub struct SeekPeekReader<R> {
    inner: R,
    start_pos: Option<u64>,
}

impl<R: Read + Seek> SeekPeekReader<R> {
    /// Creates a new [`SeekPeekReader`].
    ///
    /// When calling `.peek()` on this object the stream is restored to
    /// its original position when the [`PeekCursor`] is dropped using a seek.
    pub fn new(reader: R) -> Self {
        Self {
            inner: reader,
            start_pos: None,
        }
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

    /// Unwraps this `SeekPeekReader<R>`, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner
    }

    fn get_start_pos(&mut self) -> Result<u64> {
        if self.start_pos.is_none() {
            self.start_pos = Some(self.inner.stream_position()?);
        };

        Ok(self.start_pos.unwrap())
    }
}

impl<R: Seek + Read> Seek for SeekPeekReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.seek(pos)
    }
    
    fn stream_position(&mut self) -> Result<u64> {
        self.inner.stream_position()
    }
}

impl<R: Seek + Read> Read for SeekPeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }
    
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.inner.read_exact(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.inner.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.inner.read_to_string(buf)
    }
}

impl<R: Read + Seek> PeekRead for SeekPeekReader<R> {
    fn peek(&mut self) -> PeekCursor<'_> {
        self.start_pos = None;
        PeekCursor::new(self)
    }
}

impl<R: Read + Seek> PeekReadImpl for SeekPeekReader<R> {
    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        let start_pos = self.get_start_pos()?;
        let new_seek_pos = match pos {
            SeekFrom::Start(offset) => self.inner.seek(SeekFrom::Start(start_pos + offset))?,
            SeekFrom::Current(offset) => 
                // Avoid needless seeks.
                if offset == 0 {
                    start_pos + state.peek_pos
                } else {
                    self.inner.seek(SeekFrom::Start(start_pos + add_offset(state.peek_pos, offset)))?
                },
            SeekFrom::End(offset) => {
                let pos = self.inner.seek(SeekFrom::End(offset))?;
                if pos < start_pos {
                    self.inner.seek(SeekFrom::Start(0))?
                } else {
                    pos
                }
            }
        };
        state.peek_pos = new_seek_pos - start_pos;
        Ok(state.peek_pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        let written = self.inner.read(buf)?;
        state.peek_pos += written as u64;
        Ok(written)
    }

    fn peek_fill_buf<'a>(&'a mut self, state: &'a mut PeekCursorState) -> Result<&'a [u8]> {
        // With specialization we could provide a more optimal fill_buf here.
        self.inner.read(&mut state.buf)?;
        Ok(&state.buf)
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        state.peek_pos += amt as u64;
    }

    fn peek_drop(&mut self, _state: &mut PeekCursorState) {
        if let Some(start_pos) = self.start_pos {
            while let Err(e) = self.inner.seek(SeekFrom::Start(start_pos)) {
                if e.kind() != std::io::ErrorKind::Interrupted {
                    break;
                }
            }
        }
    }
}




