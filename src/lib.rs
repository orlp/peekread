#![allow(unused_imports)]


/// Details for those wishing to implement [`PeekRead`].
pub mod detail;

mod foreign_impl;
mod seekreader;
// mod bufreader;

use std::io::*;
// pub use bufreader::PeekBufReader;
pub use detail::cursor::PeekCursor;


/// A trait for a [`Read`] stream that supports buffered reading and peeking.
///
/// In addition to a normal read cursor it has a separate 'peek cursor' which can go ahead of the
/// regular read cursor, but never behind it. Reading from the peek cursor does not affect the read
/// cursor in any way.
///
/// [`unread`]: PeekBufReader::unread
pub trait PeekRead: Read {
    /// Returns a [`PeekCursor`] which implements [`BufRead`] + [`Seek`], allowing you to peek ahead
    /// in a stream of data. Reading from this or seeking on it won't affect the read cursor, only
    /// the peek cursor.
    ///
    /// You can't seek before the read cursor, `peek().seek(SeekFrom::Start(0))` is defined to be the read cursor position.
    ///
    /// By default reads from the [`PeekCursor`] are unbuffered where possible and will only read as
    /// much as necessary from the underlying stream, if reading can block or otherwise invokes a cost.
    /// To change this use [`PeekCursor::buffered`].
    fn peek(&mut self) -> PeekCursor<'_>;
}

// Generic implementations.
/*
impl<T: PeekReadImpl> PeekReadImpl for Take<T> {
    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize> {
        let remaining = self.limit().saturating_sub(self.peek_stream_position(state)?) as usize;
        dbg!(remaining);
        let max_peek = remaining.min(buf.len());
        self.get_mut().peek_read(state, &mut buf[..max_peek])
    }

    fn peek_fill_buf(&mut self, state: &mut PeekCursorState) -> Result<&[u8]> {
        let limit = self.limit() as usize;
        if limit == 0 {
            return Ok(&[]);
        }

        let buf = self.get_mut().peek_fill_buf(state)?;
        let n = buf.len().min(limit);
        Ok(&buf[..n])
    }

    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize) {
        self.get_mut().consume(amt);
        let limit = self.limit();
        self.set_limit(limit.saturating_sub(amt as u64));
    }

    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64> {
        if let SeekFrom::End(offset) = pos {
            let limit = self.limit();
            let eof_offset = self.get_mut().peek_seek(state, SeekFrom::Start(limit))? as i64;
            self.get_mut()
                .peek_seek(state, SeekFrom::Start((eof_offset + offset).max(0) as u64))
        } else {
            self.get_mut().peek_seek(state, pos)
        }
    }
}
*/
