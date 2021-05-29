use std::io::*;

use crate::PeekReadImpl;

/// An object implementing [`BufRead`] and [`Seek`] to peek ahead in a stream without
/// affecting the original stream.
///
/// This object is only created by [`PeekRead::peek`].
///
/// [`PeekRead`]: crate::PeekRead
/// [`PeekRead::peek`]: crate::PeekRead::peek
pub struct PeekCursor<'a> {
    inner: &'a mut dyn PeekReadImpl,
}

impl<'a> PeekCursor<'a> {
    pub(crate) fn new(inner: &'a mut dyn PeekReadImpl) -> Self {
        Self { inner }
    }
}

impl<'a> Read for PeekCursor<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.peek_read(buf)
    }
}

impl<'a> BufRead for PeekCursor<'a> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.peek_fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.peek_consume(amt)
    }
}


impl<'a> Seek for PeekCursor<'a> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.peek_seek(pos)
    }
}


/// A `PeekCursor` that does not override the default implementations.
/// Used to provide our default implementations without being circular.
#[derive(Debug)]
pub(crate) struct DefaultImplPeekCursor<'a, T: ?Sized + PeekReadImpl> {
    inner: &'a mut T,
}

impl<'a, T: ?Sized + PeekReadImpl> DefaultImplPeekCursor<'a, T> {
    pub fn new(inner: &'a mut T) -> Self  {
        Self { inner }
    }
}

impl<'a, T: ?Sized + PeekReadImpl> Read for DefaultImplPeekCursor<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.peek_read(buf)
    }
}

impl<'a, T: ?Sized + PeekReadImpl> BufRead for DefaultImplPeekCursor<'a, T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.peek_fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.peek_consume(amt)
    }
}

impl<'a, T: ?Sized + PeekReadImpl> Seek for DefaultImplPeekCursor<'a, T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.peek_seek(pos)
    }
}
