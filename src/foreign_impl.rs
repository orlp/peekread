use crate::{PeekRead, PeekCursor};
use crate::detail::{PeekReadImpl, PeekCursorState};
use std::io::*;
use std::io;

fn add_offset(base: u64, offset: i64) -> u64 {
    (base as i64).saturating_add(offset).max(0) as u64
}

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

    fn peek_consume(&mut self, _state: &mut PeekCursorState, _amt: usize) { }
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
            SeekFrom::Current(offset) => add_offset(state.peek_pos, offset),
            SeekFrom::End(offset) => add_offset(self.len() as u64, offset),
        };
        state.peek_pos = state.peek_pos.min(self.len() as u64);
        Ok(state.peek_pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        let written = self.get(state.peek_pos as usize..).unwrap_or_default().read(buf)?;
        state.peek_pos += written as u64;
        Ok(written)
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
        self.get_ref().as_ref().peek_seek(state, pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        self.get_ref().as_ref().peek_read(state, buf)
    }

    fn peek_fill_buf<'a>(&'a mut self, state: &'a mut PeekCursorState) -> Result<&'a [u8]> {
        let slice = self.get_ref().as_ref();
        Ok(slice.get(state.peek_pos as usize..).unwrap_or_default())
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
            SeekFrom::Current(offset) => add_offset(state.peek_pos, offset),
            SeekFrom::End(offset) => {
                let end = io::copy(&mut self.peek().take(limit_from_start), &mut io::sink())?;
                add_offset(end, offset)
            }
        };
        self.set_limit(limit_from_start.saturating_sub(state.peek_pos));
        Ok(state.peek_pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        if self.limit() == 0 {
            return Ok(0);
        }

        let limit = self.limit();
        let mut reader = self.peek();
        reader.seek(SeekFrom::Start(state.peek_pos))?;
        let written = reader.take(limit).read(buf)? as u64;
        state.peek_pos += written;
        self.set_limit(limit - written);
        Ok(written as usize)
    }

    fn peek_fill_buf<'a>(&'a mut self, state: &'a mut PeekCursorState) -> Result<&'a [u8]> {
        if self.limit() == 0 {
            return Ok(&[]);
        }

        let limit = self.limit();
        let mut reader = self.peek();
        reader.seek(SeekFrom::Start(state.peek_pos))?;
        reader.take(limit.min(state.buffer_size as u64)).read_to_end(&mut state.buf)?;
        state.buf_pos = 0;
        Ok(&state.buf)
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        let limit = self.limit();
        state.peek_pos += amt as u64;
        self.set_limit(limit.saturating_sub(amt as u64));
    }
}










// impl<T: PeekReadImpl> PeekReadImpl for Take<T> {
//     fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
//         let limit = self.limit();
//         state.peek_pos = match pos {
//             SeekFrom::Start(offset) => offset,
//             SeekFrom::Current(offset) => add_offset(state.peek_pos, offset),
//             SeekFrom::End(offset) => add_offset(limit as u64, offset),
//         };
//     }

//     fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
//         let limit = self.limit();
//         let remaining = limit.saturating_sub(self.peek_stream_position(state)?) as usize;
//         dbg!(remaining);
//         let max_peek = remaining.min(buf.len());
//         let written = self.get_mut().peek_read(state, &mut buf[..max_peek])?;

//     }

//     fn peek_fill_buf(&mut self, state: &mut PeekCursorState) -> Result<&[u8]> {
//         let limit = self.limit() as usize;
//         if limit == 0 {
//             return Ok(&[]);
//         }

//         let buf = self.get_mut().peek_fill_buf(state)?;
//         let n = buf.len().min(limit);
//         Ok(&buf[..n])
//     }

//     fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
//         self.get_mut().consume(amt);
//         let limit = self.limit();
//         self.set_limit(limit.saturating_sub(amt as u64));
//     }

// }





// TODO: Not sure if this is possible, there are then two peek cursors.
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }
