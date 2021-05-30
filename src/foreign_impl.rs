use crate::{PeekRead, PeekCursor};
use crate::detail::{PeekReadImpl, PeekCursorState};
use std::io::*;

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
        let slice = self.get_ref().as_ref();
        state.peek_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => add_offset(state.peek_pos, offset),
            SeekFrom::End(offset) => add_offset(slice.len() as u64, offset),
        };
        state.peek_pos = state.peek_pos.min(slice.len() as u64);
        Ok(state.peek_pos)
    }

    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        let slice = self.get_ref().as_ref();
        let written = slice.get(state.peek_pos as usize..).unwrap_or_default().read(buf)?;
        state.peek_pos += written as u64;
        Ok(written)
    }

    fn peek_fill_buf(&mut self, state: &mut PeekCursorState) -> Result<&[u8]> {
        let slice = self.get_ref().as_ref();
        Ok(slice.get(state.peek_pos as usize..).unwrap_or_default())
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        state.peek_pos += amt as u64;
    }
}



// TODO: Not sure if this is possible, there are then two peek cursors.
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }