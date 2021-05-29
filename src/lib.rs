use std::io::*;

mod cursor;
mod reader;
pub use reader::PeekReader;
pub use cursor::PeekCursor;
use cursor::DefaultImplPeekCursor;

/// A trait for a [`Read`] stream that supports buffered reading and peeking.
///
/// It has a separate 'peek cursor' which can go ahead of the regular read cursor, but never behind
/// it. In case the read cursor passes the peek cursor the peek cursor is automatically advanced to
/// match it.
pub trait PeekRead: BufRead {
    /// Returns a [`PeekCursor`] which implements [`BufRead`] + [`Seek`]. Reading from this or
    /// seeking on it won't affect the read cursor, only the peek cursor. You can't seek before the
    /// read cursor, `peek().seek(SeekFrom::Start(0))` is defined to be the read cursor position.
    ///
    /// There is only one peek cursor, so operations on the [`PeekCursor`]s returned by separate
    /// calls to this function manipulate the same (persistent) underlying cursor state.
    fn peek(&mut self) -> PeekCursor<'_>;

    /// Pushes the given data into the stream at the front, pushing the read
    /// cursor back. The peek cursor can do three things depending on `peek_cursor_behavior`:
    ///
    ///   1. [`UnreadPeekCursor::Fixed`] leaves the position of the peek cursor unchanged, which
    ///      means that `.peek().stream_position()` becomes `data.len()` higher (since the read
    ///      cursor was moved back and the stream position is calculated relative to it).
    ///   2. [`UnreadPeekCursor::Shift`] moves the peek cursor back by `data.len()` bytes, which
    ///      leaves `.peek().stream_position()` unchanged.
    ///   3. [`UnreadPeekCursor::ShiftIfZero`] is equivalent to [`UnreadPeekCursor::Shift`] if
    ///      `.peek().stream_position()` is zero and [`UnreadPeekCursor::Fixed`] otherwise.
    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor);
}

impl<T: PeekReadImpl> PeekRead for T {
    fn peek(&mut self) -> PeekCursor<'_> {
        PeekCursor::new(self)
    }

    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor) {
        self.unread(data, peek_cursor_behavior)
    }
}


/// Enum argument for [`PeekRead::unread`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnreadPeekCursor {
    Fixed,
    Shift,
    ShiftIfZero,
}

/// A helper trait used to implement [`PeekRead`].
///
/// You can't implement [`PeekRead`] directly, instead you must implement this trait which will
/// then automatically implement [`PeekRead`] for you.
pub trait PeekReadImpl: BufRead {
    /// Used to implement `self.peek().read(buf)`. See [`Read::read`].
    fn peek_read(&mut self, buf: &mut [u8]) -> Result<usize>;
    
    /// Used to implement `self.peek().fill_buf()`. See [`BufRead::fill_buf`].
    fn peek_fill_buf(&mut self) -> Result<&[u8]>;
    
    /// Used to implement `self.peek().consume()`. See [`BufRead::consume`].
    fn peek_consume(&mut self, amt: usize);

    /// Used to implement `self.peek().seek(pos)`. See [`Seek::seek`].
    fn peek_seek(&mut self, pos: SeekFrom) -> Result<u64>;

    /// Used to implement `self.unread(data, peek_cursor_behavior)`. See [`PeekRead::unread`].
    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor);

    // Start default methods.
    /// Used to implement `self.peek().stream_position()`. See [`Seek::stream_position`].
    fn peek_stream_position(&mut self) -> Result<u64> {
        DefaultImplPeekCursor::new(self).stream_position()
    }

    /// Used to implement `self.peek().read_exact(buf)`. See [`Read::read_exact`].
    fn peek_read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        DefaultImplPeekCursor::new(self).read_exact(buf)
    }

    /// Used to implement `self.peek().read_to_end(buf)`. See [`Read::read_to_end`].
    fn peek_read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        DefaultImplPeekCursor::new(self).read_to_end(buf)
    }

    /// Used to implement `self.peek().read_to_string(buf)`. See [`Read::read_to_string`].
    fn peek_read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        DefaultImplPeekCursor::new(self).read_to_string(buf)
    }
}


// Generic implementations.
impl<T: PeekReadImpl> PeekReadImpl for Take<T> {
    fn peek_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let remaining = self.limit().saturating_sub(self.peek_stream_position()?) as usize;
        dbg!(remaining);
        let max_peek = remaining.min(buf.len());
        self.get_mut().peek_read(&mut buf[..max_peek])
    }

    fn peek_fill_buf(&mut self) -> Result<&[u8]> {
        let limit = self.limit() as usize;
        if limit == 0 {
            return Ok(&[]);
        }

        let buf = self.get_mut().peek_fill_buf()?;
        let n = buf.len().min(limit);
        Ok(&buf[..n])
    }
    
    fn peek_consume(&mut self, amt: usize) {
        self.get_mut().consume(amt);
        let limit = self.limit();
        self.set_limit(limit.saturating_sub(amt as u64));
    }

    fn peek_seek(&mut self, pos: SeekFrom) -> Result<u64> {
        if let SeekFrom::End(offset) = pos {
            let limit = self.limit();
            let eof_offset = self.get_mut().peek_seek(SeekFrom::Start(limit))? as i64;
            self.get_mut().peek_seek(SeekFrom::Start((eof_offset + offset).max(0) as u64))
        } else {
            self.get_mut().peek_seek(pos)
        }
    }

    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor) {
        self.get_mut().unread(data, peek_cursor_behavior);
        self.set_limit(self.limit() + data.len() as u64);
    }
 }

impl<T: PeekReadImpl + ?Sized> PeekReadImpl for &mut T {
    #[inline]
    fn peek_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).peek_read(buf)
    }

    #[inline]
    fn peek_fill_buf(&mut self) -> Result<&[u8]> {
        (**self).peek_fill_buf()
    }

    #[inline]
    fn peek_consume(&mut self, amt: usize) {
        (**self).peek_consume(amt)
    }

    #[inline]
    fn peek_seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).peek_seek(pos)
    }

    #[inline]
    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor) {
        (**self).unread(data, peek_cursor_behavior)
    }
}

impl<T: PeekReadImpl + ?Sized> PeekReadImpl for Box<T> {
    #[inline]
    fn peek_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).peek_read(buf)
    }

    #[inline]
    fn peek_fill_buf(&mut self) -> Result<&[u8]> {
        (**self).peek_fill_buf()
    }

    #[inline]
    fn peek_consume(&mut self, amt: usize) {
        (**self).peek_consume(amt)
    }

    #[inline]
    fn peek_seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).peek_seek(pos)
    }

    #[inline]
    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor) {
        (**self).unread(data, peek_cursor_behavior)
    }
}

// TODO: Not sure if this is possible, there are then two peek cursors.
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }


// Impossible that BufRead does support:
// &[u8], Empty, StdinLock<'_>, Cursor<T> 
