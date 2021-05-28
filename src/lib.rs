use std::io::{BufRead, Error, ErrorKind, Read, Result, Take};

mod peekreader;
pub use peekreader::PeekReader;

/// A BufRead object that supports peeking. It has a separate 'peek cursor' which
/// can go ahead of the regular read cursor, but never behind it. In case the
/// read cursor passes the peek cursor the peek cursor is automatically advanced.
pub trait PeekRead: BufRead {
    /// Gets the position of the peek cursor relative to the read cursor.
    fn peek_position(&self) -> usize;

    /// Sets the position of the peek cursor. Returns the new `peek_position()`.
    ///
    /// It's perfectly allowed to set a seek position beyond EOF, it will just
    /// result in failing reads later. This function can only fail when using
    /// [`PeekSeekFrom::End`] and an IO error occurred before reaching EOF.
    fn peek_seek(&mut self, pos: PeekSeekFrom) -> Result<usize>;

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
        default_peek_to_end(self, buf)
    }

    fn peek_to_string(&mut self, buf: &mut String) -> Result<usize> {
        default_peek_to_string(self, buf)
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PeekSeekFrom {
    /// Sets the peek cursor offset from the read cursor by the specified amount of bytes.
    ReadCursor(usize),

    /// Sets the peek cursor offset from its current position by the specified amount of bytes.
    Current(isize),

    /// Sets the peek cursor offset from the end of the stream by the specified amount of bytes.
    ///
    /// Warning: this will cause the entire stream to be loaded into memory.
    End(isize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnreadPeekCursor {
    Fixed,
    Shift,
    ShiftIfZero,
}

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

fn default_peek_exact<R: PeekRead + ?Sized>(this: &mut R, mut buf: &mut [u8]) -> Result<()> {
    while !buf.is_empty() {
        match this.peek(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {} // Ignore interrupt.
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

fn default_peek_to_end<R: PeekRead + ?Sized>(this: &mut R, buf: &mut Vec<u8>) -> Result<usize> {
    // This implementation is slower than theoretically necessary to avoid unsafe code.
    let mut tmp = vec![0u8; 32];
    let mut written = 0;
    loop {
        match this.peek(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                written += (&tmp[..n]).read_to_end(buf)?;

                if n == tmp.len() {
                    // We can probably do bigger reads, double buffer size.
                    tmp.resize(n * 2, 0);
                }
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {} // Ignore interrupt.
            Err(e) => return Err(e),
        }
    }

    Ok(written)
}

fn default_peek_to_string<R: PeekRead + ?Sized>(this: &mut R, buf: &mut String) -> Result<usize> {
    // This implementation is slower than theoretically necessary to avoid unsafe code.
    let mut tmp = Vec::new();
    let bytes = this.read_to_end(&mut tmp)?;
    let string = std::str::from_utf8(&tmp)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "stream did not contain valid UTF-8"))?;
    *buf += string;
    Ok(bytes)
}

impl<T: PeekRead> PeekRead for Take<T> {
    fn peek_position(&self) -> usize {
        self.get_ref().peek_position()
    }

    fn peek_seek(&mut self, pos: PeekSeekFrom) -> Result<usize> {
        if let PeekSeekFrom::End(offset) = pos {
            let limit = self.limit() as usize;
            let eof_offset = self.get_mut().peek_seek(PeekSeekFrom::ReadCursor(limit))?;
            let seek_pos = (eof_offset as isize + offset).max(0) as usize;
            self.get_mut().peek_seek(PeekSeekFrom::ReadCursor(seek_pos))
        } else {
            self.get_mut().peek_seek(pos)
        }
    }

    fn unread(&mut self, data: &[u8], peek_cursor_behavior: UnreadPeekCursor) {
        self.get_mut().unread(data, peek_cursor_behavior);
        self.set_limit(self.limit() + data.len() as u64);
    }

    fn peek(&mut self, buf: &mut [u8]) -> Result<usize> {
        let remaining = (self.limit() as usize).saturating_sub(self.peek_position());
        let max_peek = remaining.min(buf.len());
        self.get_mut().peek(&mut buf[..max_peek])
    }
 }




// TODO This looks tricky but maybe possible?
// impl<T: PeekRead, U: PeekRead> PeekRead for Chain<T, U> { }