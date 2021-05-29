use std::io::*;
use crate::PeekRead;


pub struct PeekCursor<'a> {
    inner: &'a mut dyn PeekRead,
}

impl<'a> PeekCursor<'a> {
    pub fn new(inner: &'a mut dyn PeekRead) -> Self  {
        Self { inner }
    }
}

impl<'a> Read for PeekCursor<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.peek(buf)
    }
}

impl<'a> BufRead for PeekCursor<'a> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.fill_peek_buf()
    }

    fn consume(&mut self, amt: usize) {
        let _ = self.inner.peek_seek(SeekFrom::Current(amt as i64));
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
pub(crate) struct DefaultImplPeekCursor<'a, T: ?Sized + PeekRead> {
    inner: &'a mut T,
}

impl<'a, T: ?Sized + PeekRead> DefaultImplPeekCursor<'a, T> {
    pub fn new(inner: &'a mut T) -> Self  {
        Self { inner }
    }
}

impl<'a, T: ?Sized + PeekRead> Read for DefaultImplPeekCursor<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.peek(buf)
    }
}

impl<'a, T: ?Sized + PeekRead> BufRead for DefaultImplPeekCursor<'a, T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.fill_peek_buf()
    }

    fn consume(&mut self, amt: usize) {
        let _ = self.inner.peek_seek(SeekFrom::Current(amt as i64));
    }
}

impl<'a, T: ?Sized + PeekRead> Seek for DefaultImplPeekCursor<'a, T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.peek_seek(pos)
    }
}
