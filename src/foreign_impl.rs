use crate::detail::{PeekCursorState, PeekReadImpl};
use crate::util::seek_add_offset;
use crate::{PeekCursor, PeekRead};
use std::io::{self, Cursor, Empty, Read, Result, Seek, SeekFrom, Take};

impl<T: PeekRead + ?Sized> PeekRead for &mut T {
    #[inline]
    fn peek(&mut self) -> PeekCursor<'_> {
        (**self).peek()
    }
}

impl<T: PeekRead + ?Sized> PeekRead for Box<T> {
    #[inline]
    fn peek(&mut self) -> PeekCursor<'_> {
        (**self).peek()
    }
}

impl PeekRead for Empty {
    fn peek(&mut self) -> PeekCursor<'_> {
        PeekCursor::new(self)
    }
}

impl PeekReadImpl for Empty {
    fn peek_seek(&mut self, _state: &mut PeekCursorState, _pos: SeekFrom) -> Result<u64> {
        Ok(0)
    }

    fn peek_read(&mut self, _state: &mut PeekCursorState, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn peek_fill_buf(&mut self, _state: &mut PeekCursorState) -> Result<&[u8]> {
        Ok(&[])
    }

    fn peek_consume(&mut self, _state: &mut PeekCursorState, _amt: usize) {}
}

impl PeekRead for &[u8] {
    fn peek(&mut self) -> PeekCursor<'_> {
        PeekCursor::new(self)
    }
}

impl PeekReadImpl for &[u8] {
    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        state.peek_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => seek_add_offset(state.peek_pos, offset)?,
            SeekFrom::End(offset) => seek_add_offset(self.len() as u64, offset)?,
        };
        Ok(state.peek_pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        let written = self
            .get(state.peek_pos as usize..)
            .unwrap_or_default()
            .read(buf)?;
        state.peek_pos += written as u64;
        Ok(written)
    }

    fn peek_read_exact(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<()> {
        self.get(state.peek_pos as usize..)
            .unwrap_or_default()
            .read_exact(buf)?;
        state.peek_pos += buf.len() as u64;
        Ok(())
    }

    fn peek_fill_buf(&mut self, state: &mut PeekCursorState) -> Result<&[u8]> {
        Ok(self.get(state.peek_pos as usize..).unwrap_or_default())
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        state.peek_pos += amt as u64;
    }
}

impl<T: AsRef<[u8]>> PeekRead for Cursor<T> {
    fn peek(&mut self) -> PeekCursor<'_> {
        PeekCursor::new(self)
    }
}

impl<T: AsRef<[u8]>> PeekReadImpl for Cursor<T> {
    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        let start_pos = self.stream_position()? as usize;
        let slice = self.get_ref().as_ref();
        slice
            .get(start_pos..)
            .unwrap_or_default()
            .peek_seek(state, pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        let start_pos = self.stream_position()? as usize;
        let slice = self.get_ref().as_ref();
        slice
            .get(start_pos..)
            .unwrap_or_default()
            .peek_read(state, buf)
    }

    fn peek_read_exact(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<()> {
        let start_pos = self.stream_position()? as usize;
        let slice = self.get_ref().as_ref();
        slice
            .get(start_pos..)
            .unwrap_or_default()
            .peek_read_exact(state, buf)
    }

    fn peek_fill_buf<'a>(&'a mut self, state: &'a mut PeekCursorState) -> Result<&'a [u8]> {
        let start_pos = self.stream_position()? as usize;
        let slice = self.get_ref().as_ref();
        Ok(slice
            .get(start_pos + state.peek_pos as usize..)
            .unwrap_or_default())
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        state.peek_pos += amt as u64;
    }
}

impl<T: PeekRead> PeekRead for Take<T> {
    fn peek(&mut self) -> PeekCursor<'_> {
        PeekCursor::new(self)
    }
}

impl<T: PeekRead> PeekReadImpl for Take<T> {
    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        let limit_from_start = state.peek_pos + self.limit();
        state.peek_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => seek_add_offset(state.peek_pos, offset)?,
            SeekFrom::End(offset) => {
                // Is there a more efficient way without specialization?
                let end = {
                    let mut dummy: u8 = 0;
                    let mut peeker = self.peek();
                    peeker.seek(SeekFrom::Start(limit_from_start))?;
                    let is_eof = peeker.read(std::slice::from_mut(&mut dummy))? == 0;

                    if is_eof {
                        // Have to scan to find real end.
                        peeker.seek(SeekFrom::Start(0))?;
                        io::copy(&mut peeker, &mut io::sink())?
                    } else {
                        limit_from_start
                    }
                };

                seek_add_offset(end, offset)?
            }
        };
        state.peek_pos = state.peek_pos.min(limit_from_start);
        self.set_limit(limit_from_start - state.peek_pos);
        Ok(state.peek_pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        if self.limit() == 0 {
            return Ok(0);
        }

        let limit = self.limit();
        let mut peeker = self.peek();
        peeker.seek(SeekFrom::Start(state.peek_pos))?;
        let written = peeker.take(limit).read(buf)? as u64;
        state.peek_pos += written;
        self.set_limit(limit - written);
        Ok(written as usize)
    }

    fn peek_fill_buf<'a>(&'a mut self, state: &'a mut PeekCursorState) -> Result<&'a [u8]> {
        if self.limit() == 0 {
            return Ok(&[]);
        }

        let mut peeker = self.peek();
        peeker.seek(SeekFrom::Start(state.peek_pos))?;
        let read = peeker.read(&mut state.buf)?;
        Ok(&state.buf[..read])
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        let limit = self.limit();
        let limit_from_start = limit + state.peek_pos;
        state.peek_pos += amt as u64;
        state.peek_pos = state.peek_pos.min(limit_from_start);
        self.set_limit(limit_from_start - state.peek_pos);
    }
}

// TODO: Not sure if this is possible, there are then two peek cursors.
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }
