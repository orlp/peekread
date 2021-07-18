use crate::{PeekRead, PeekCursor, detail::{PeekReadImpl, PeekCursorState}};
use crate::util::seek_add_offset;
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

    fn init_start_pos(&mut self) -> Result<u64> {
        let start_pos = self.start_pos.map(Ok).unwrap_or_else(|| self.inner.stream_position())?;
        self.start_pos = Some(start_pos);
        Ok(start_pos)
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
    fn peek_seek(&mut self, _state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        let start_pos = self.init_start_pos()?;
        let cur_pos = self.stream_position()?;
        let new_pos = match pos {
            SeekFrom::Start(offset) => self.inner.seek(SeekFrom::Start(start_pos + offset))?,
            SeekFrom::Current(offset) => self.inner.seek(SeekFrom::Current(offset))?,
            SeekFrom::End(offset) => {
                // TODO: can this be more efficient?
                let end_pos = self.inner.seek(SeekFrom::End(0))?.max(start_pos);
                match seek_add_offset(end_pos, offset) {
                    Ok(o) => self.inner.seek(SeekFrom::Start(o))?,
                    Err(e) => {
                        // Restore position.
                        self.inner.seek(SeekFrom::Start(cur_pos))?;
                        return Err(e)
                    }
                }
            }
        };

        if new_pos < start_pos {
            self.inner.seek(SeekFrom::Start(cur_pos))?;
            Err(Error::new(ErrorKind::InvalidInput, "invalid seek to a negative or overflowing position"))
        } else {
            Ok(new_pos - start_pos)
        }
    }

    fn peek_read(&mut self, _state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        self.init_start_pos()?;
        let written = self.inner.read(buf)?;
        Ok(written)
    }

    fn peek_read_exact(&mut self, _state: &mut PeekCursorState, buf: &mut [u8]) -> Result<()> {
        self.init_start_pos()?;
        self.inner.read_exact(buf)?;
        Ok(())
    }

    fn peek_fill_buf<'a>(&'a mut self, state: &'a mut PeekCursorState) -> Result<&'a [u8]> {
        self.init_start_pos()?;
        // With specialization we could provide a more optimal fill_buf here.
        let read = self.inner.read(&mut state.buf)?;
        self.inner.seek(SeekFrom::Current(-(read as i64)))?;
        Ok(&state.buf)
    }

    fn peek_consume(&mut self, _state: &mut PeekCursorState, amt: usize) {
        self.init_start_pos().ok();
        // With specialization we could provide a more optimal fill_buf here.
        self.inner.seek(SeekFrom::Current(amt as i64)).ok();
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




