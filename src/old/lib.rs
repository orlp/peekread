#![allow(unused_imports)]

use std::io::*;

mod reader;
pub use reader::PeekReader;

mod cursor;
pub use cursor::PeekCursor;
use cursor::DefaultImplPeekCursor;
pub mod v2;

/// A BufRead object that supports peeking. It has a separate 'peek cursor' which
/// can go ahead of the regular read cursor, but never behind it. In case the
/// read cursor passes the peek cursor the peek cursor is automatically advanced.
pub trait PeekRead: BufRead {
    /// Returns a [`PeekCursor`] which implements `BufRead + Seek`. Reading from this or seeking on
    /// it won't affect the read cursor, only the peek cursor. You can't seek before the read
    /// cursor, `peek_cursor().seek(SeekFrom::Start(0))` is defined to be the read cursor position.
    ///
    /// There is only one peek cursor, so all calls to this function manipulate the same underlying
    /// cursor state.
    fn peek_cursor(&mut self) -> PeekCursor<'_>;

    /// Exactly like [`Read::read`], but reads from and advances the peek cursor instead.
    fn peek(&mut self, buf: &mut [u8]) -> Result<usize>;
    
    /// Exactly like [`BufRead::fill_buf`], but fills and returns a buffer at the peek cursor
    /// instead.
    fn fill_peek_buf(&mut self) -> Result<&[u8]>;

    /// Sets the position of the peek cursor relative to the read cursor and returns the new
    /// position. `SeekFrom::Start(0)` is the position of the read cursor.
    ///
    /// It's perfectly allowed to set a seek position beyond EOF, it will just
    /// result in failing reads later. This function can only fail when using
    /// [`SeekFrom::End`] and an IO error occurred before reaching EOF.
    fn peek_seek(&mut self, pos: SeekFrom) -> Result<u64>;

    /// Pushes the given data into the stream at the front, pushing the read
    /// cursor back. The peek cursor can do three things depending on `peek_cursor_behavior`:
    ///
    ///   1. [`UnreadPeekCursor::Fixed`] leaves the position of the peek cursor unchanged, which
    ///      means that `peek_position()` becomes `data.len()` higher (since the read cursor
    ///      was moved back and [`Self::peek_position`] is calculated relative to it).
    ///   2. [`UnreadPeekCursor::Shift`] moves the peek cursor back by `data.len()` bytes, which
    ///      leaves `peek_position()` unchanged.
    ///   3. [`UnreadPeekCursor::ShiftIfZero`] is equivalent to [`UnreadPeekCursor::Shift`] if
    ///      `peek_position()` is zero and [`UnreadPeekCursor::Fixed`] otherwise.
    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor);


    // Start default methods.
    /// Equivalent to `self.peek_cursor().stream_position()`.
    fn peek_position(&mut self) -> Result<u64> {
        DefaultImplPeekCursor::new(self).stream_position()
    }

    /// Equivalent to `self.peek_cursor().read_exact(buf)`.
    fn peek_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        DefaultImplPeekCursor::new(self).read_exact(buf)
    }

    /// Equivalent to `self.peek_cursor().read_to_end(buf)`.
    fn peek_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        DefaultImplPeekCursor::new(self).read_to_end(buf)
    }

    /// Equivalent to `self.peek_cursor().read_to_string(buf)`.
    fn peek_to_string(&mut self, buf: &mut String) -> Result<usize> {
        DefaultImplPeekCursor::new(self).read_to_string(buf)
    }

    /// Equivalent to `self.peek_cursor().bytes()`.
    fn peek_bytes(self) -> PeekBytes<Self>
    where
        Self: Sized,
    {
        PeekBytes { inner: self }
    }

    // by_ref?
}


/// Enum argument for [`PeekRead::unread`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnreadPeekCursor {
    Fixed,
    Shift,
    ShiftIfZero,
}

/// Iterator type for [`PeekRead::peek_bytes`].
#[derive(Debug)]
pub struct PeekBytes<R> {
    inner: R,
}

impl<R: PeekRead> Iterator for PeekBytes<R> {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Result<u8>> {
        let mut byte = 0;
        loop {
            return match self.inner.peek(std::slice::from_mut(&mut byte)) {
                Ok(0) => None,
                Ok(..) => Some(Ok(byte)),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => Some(Err(e)),
            };
        }
    }
}


// Generic implementations.
impl<T: PeekRead> PeekRead for Take<T> {
    fn peek_cursor<'a>(&'a mut self) -> PeekCursor<'a> {
        PeekCursor::new(self)
    }

    fn peek(&mut self, buf: &mut [u8]) -> Result<usize> {
        let remaining = self.limit().saturating_sub(self.peek_position()?) as usize;
        let max_peek = remaining.min(buf.len());
        self.get_mut().peek(&mut buf[..max_peek])
    }

    fn fill_peek_buf(&mut self) -> Result<&[u8]> {
        let limit = self.limit() as usize;
        if limit == 0 {
            return Ok(&[]);
        }

        let buf = self.get_mut().fill_peek_buf()?;
        let n = buf.len().min(limit);
        Ok(&buf[..n])
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

// TODO: Not sure if this is possible, there are then two peek cursors.
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }
