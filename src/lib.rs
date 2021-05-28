use std::io::{Error, ErrorKind, BufRead, Result};


/// A BufRead object that supports peeking. It has a separate 'peek cursor' which
/// can go ahead of the regular read cursor, but never behind it. In case the
/// read cursor passes the peek cursor the peek cursor is automatically advanced.
pub trait PeekRead: BufRead {
    /// Gets the position of the peek cursor relative to the read cursor.
    fn peek_position(&self) -> usize;

    /// Sets the position of the peek cursor.
    fn set_peek_position(&mut self, pos: usize);

    /// Pushes the given data into the stream at the front, pushing the read
    /// cursor back. The peek cursor can do three things depending on `peek_cursor_behavior`:
    ///
    ///   1. [`UnreadPeekCursor::Fixed`] leaves the position of the peek cursor unchanged, which
    ///      means that `peek_position()` becomes `data.len()` higher (since the read cursor
    ///      was moved back and [`peek_position`] is relative).
    ///   2. [`UnreadPeekCursor::Shift`] moves the peek cursor back by `data.len()` bytes, which
    ///      leaves `peek_position()` unchanged.
    ///   3. [`UnreadPeekCursor::ShiftIfZero`] is equivalent to [`UnreadPeekCursor::Shift`] if
    ///      `peek_position() == 0` and [`UnreadPeekCursor::Fixed`] otherwise.
    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor);

    /// Exactly like [`Read::read`], but advances the peek cursor instead.
    fn peek(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Exactly like [`Read::read_exact`], but advances the peek cursor instead.
    fn peek_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        default_peek_exact(self, buf)
    }

    fn peek_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {

        // FIXME: implement.
        Ok(0)
    }

    fn peek_to_string(&mut self, buf: &mut String) -> Result<usize> {
        // FIXME: implement.
        Ok(0)
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    fn peek_bytes(self) -> PeekBytes<Self>
    where
        Self: Sized,
    {
        PeekBytes { inner: self }
    }
}


pub enum UnreadPeekCursor {
    Fixed,
    Shift,
    ShiftIfZero
}


pub struct PeekBytes<R> {
    inner: R,
}

// TODO:
// impl<T: PeekRead> PeekRead for Take<T> { }
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }



pub(crate) fn default_peek_exact<R: PeekRead + ?Sized>(
    this: &mut R,
    mut buf: &mut [u8],
) -> Result<()> {
    while !buf.is_empty() {
        match this.peek(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    if !buf.is_empty() {
        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    } else {
        Ok(())
    }
}
