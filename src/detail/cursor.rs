use std::any::Any;
use std::io::{BufRead, Read, Result, Seek, SeekFrom};

use crate::detail::PeekReadImpl;
use crate::PeekRead;

/// The internal state of a [`PeekCursor`]. See [`PeekReadImpl`].
///
/// All fields here are just provided to help you make your implementation possible,
/// you may use them in any way you see fit. [`PeekCursor::new`] initializes
/// these fields to their default value.
#[non_exhaustive]
#[derive(Debug)]
pub struct PeekCursorState {
    pub peek_pos: u64,
    pub buf: [u8; 1],
}

/// An object implementing [`BufRead`] and [`Seek`] to peek ahead in a stream without
/// affecting the original stream.
///
/// [`PeekRead`]: crate::PeekRead
/// [`PeekRead::peek`]: crate::PeekRead::peek
pub struct PeekCursor<'a> {
    inner: &'a mut dyn PeekReadImpl,
    state: PeekCursorState,
}

impl<'a> PeekCursor<'a> {
    /// Creates a new [`PeekCursor`].
    ///
    /// Unless you are trying to implement [`PeekRead`] you will never call this, you
    /// should look at [`PeekRead::peek`] instead. If you are trying to implement
    /// [`PeekRead`], see [`PeekReadImpl`].
    pub fn new(inner: &'a mut dyn PeekReadImpl) -> Self {
        Self {
            inner,
            state: PeekCursorState {
                peek_pos: 0,
                buf: [0],
            },
        }
    }
}

impl<'a> Seek for PeekCursor<'a> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.peek_seek(&mut self.state, pos)
    }

    fn stream_position(&mut self) -> Result<u64> {
        self.inner.peek_stream_position(&mut self.state)
    }
}

impl<'a> Read for PeekCursor<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.peek_read(&mut self.state, buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.inner.peek_read_exact(&mut self.state, buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.inner.peek_read_to_end(&mut self.state, buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.inner.peek_read_to_string(&mut self.state, buf)
    }
}

impl<'a> BufRead for PeekCursor<'a> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.peek_fill_buf(&mut self.state)
    }

    fn consume(&mut self, amt: usize) {
        self.inner.peek_consume(&mut self.state, amt)
    }
}

impl<'a> Drop for PeekCursor<'a> {
    fn drop(&mut self) {
        self.inner.peek_drop(&mut self.state)
    }
}

/// A `PeekCursor` that does not override the default implementations.
/// Used to provide our default implementations without being circular.
#[derive(Debug)]
pub(crate) struct DefaultImplPeekCursor<'a, T: ?Sized + PeekReadImpl> {
    inner: &'a mut T,
    state: &'a mut PeekCursorState,
}

impl<'a, T: ?Sized + PeekReadImpl> DefaultImplPeekCursor<'a, T> {
    pub fn new(inner: &'a mut T, state: &'a mut PeekCursorState) -> Self {
        Self { inner, state }
    }
}

impl<'a, T: ?Sized + PeekReadImpl> Seek for DefaultImplPeekCursor<'a, T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.peek_seek(self.state, pos)
    }
}

impl<'a, T: ?Sized + PeekReadImpl> Read for DefaultImplPeekCursor<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.peek_read(self.state, buf)
    }
}

impl<'a, T: ?Sized + PeekReadImpl> BufRead for DefaultImplPeekCursor<'a, T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.peek_fill_buf(self.state)
    }

    fn consume(&mut self, amt: usize) {
        self.inner.peek_consume(self.state, amt)
    }
}
