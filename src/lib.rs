#![allow(unused_imports)]

use std::io::*;

mod cursor;
mod bufreader;
mod seekreader;
use cursor::DefaultImplPeekCursor;
pub use cursor::PeekCursor;
pub use bufreader::PeekBufReader;

/// A trait for a [`Read`] stream that supports buffered reading and peeking.
///
/// In addition to a normal read cursor it has a separate 'peek cursor' which can go ahead of the
/// regular read cursor, but never behind it. In case the read cursor passes the peek cursor the
/// peek cursor is automatically advanced to match it. However in the case the read cursor moves
/// backwards (e.g. due to a [`Seek`] or [`unread`]), the peek cursor does not automatically move
/// with it.
///
/// Reading from the peek cursor does not affect the read cursor in any way.
///
/// [`unread`]: PeekBufReader::unread
pub trait PeekRead: BufRead {
    /// Returns a [`PeekCursor`] which implements [`BufRead`] + [`Seek`]. Reading from this or
    /// seeking on it won't affect the read cursor, only the peek cursor. You can't seek before the
    /// read cursor, `peek().seek(SeekFrom::Start(0))` is defined to be the read cursor position.
    ///
    /// There is only one peek cursor, so operations on the [`PeekCursor`]s returned by separate
    /// calls to this function manipulate the same (persistent) underlying cursor state.
    fn peek(&mut self) -> PeekCursor<'_>;
}

/// A helper trait used to implement [`PeekRead`].
///
/// In order to implement [`PeekRead`] for one of your types you must first implement this trait
/// on your type and then implement [`PeekRead::peek`] returning `PeekCursor::new(self)`.
pub trait PeekReadImpl: BufRead {
    /// Used to implement `self.peek().read(buf)`. See [`Read::read`].
    fn peek_read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Used to implement `self.peek().fill_buf()`. See [`BufRead::fill_buf`].
    fn peek_fill_buf(&mut self) -> Result<&[u8]>;

    /// Used to implement `self.peek().consume()`. See [`BufRead::consume`].
    fn peek_consume(&mut self, amt: usize);

    /// Used to implement `self.peek().seek(pos)`. See [`Seek::seek`].
    fn peek_seek(&mut self, pos: SeekFrom) -> Result<u64>;

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
            self.get_mut()
                .peek_seek(SeekFrom::Start((eof_offset + offset).max(0) as u64))
        } else {
            self.get_mut().peek_seek(pos)
        }
    }
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

// TODO: Empty

// TODO: Not sure if this is possible, there are then two peek cursors.
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }

// Impossible (no space to store peek cursor position).
// &[u8], StdinLock<'_>, Cursor<T>
