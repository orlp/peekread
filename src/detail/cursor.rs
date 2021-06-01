use std::io::*;
use std::any::Any;

use crate::PeekRead;
use crate::detail::PeekReadImpl;

/// The internal state of a [`PeekCursor`]. See [`PeekReadImpl`].
///
/// `buffer_size` is the amount of buffering the user has requested using [`PeekCursor::buffered`].
///
/// All other fields here are just provided to help you make your implementation possible,
/// you may use them in any way you see fit. [`PeekCursor::new`] initializes 
/// these fields to their default value.
#[non_exhaustive]
#[derive(Debug)]
pub struct PeekCursorState {
    pub buffer_size: usize,
    pub peek_pos: u64,
    pub buf: Vec<u8>,
    pub buf_pos: usize,
    pub any: Option<Box<dyn Any>>,
}

impl PeekCursorState {
    /// Returns a new default-initialized [`PeekCursorState`].
    pub fn new() -> Self {
        Self {
            buffer_size: 0,
            peek_pos: 0,
            buf: Vec::new(),
            buf_pos: 0,
            any: None,
        }
    }
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
    /// Sets the size of the read buffer used for this cursor. Whenever a
    /// read from the underlying stream is done, it will attempt to read at
    /// least this many bytes at once.
    ///
    /// The default is `buffered(0)`, meaning no reads are done any larger than
    /// necessary. An exception may be made for streams that have the same cost
    /// when reading regardless of the size, such as when reading from a `&[u8]`.
    pub fn buffered(&mut self, buffer_size: usize) -> &mut Self {
        self.state.buffer_size = buffer_size;
        self
    }

    /// Creates a new [`PeekCursor`].
    ///
    /// Unless you are trying to implement [`PeekRead`] you will never call this, you
    /// should look at [`PeekRead::peek`] instead. If you are trying to implement 
    /// [`PeekRead`], see [`PeekReadImpl`].
    pub fn new(inner: &'a mut dyn PeekReadImpl) -> Self {
        Self::with_state(inner, PeekCursorState::new())
    }

    /// Creates a new [`PeekCursor`] with the given state.
    ///
    /// Unless you are trying to implement [`PeekRead`] you will never call this, you
    /// should look at [`PeekRead::peek`] instead. If you are trying to implement 
    /// [`PeekRead`], see [`PeekReadImpl`].
    pub fn with_state(inner: &'a mut dyn PeekReadImpl, state: PeekCursorState) -> Self {
        Self {
            inner,
            state,
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

    fn read_to_end(
        &mut self,
        buf: &mut Vec<u8>,
    ) -> Result<usize> {
        self.inner.peek_read_to_end(&mut self.state, buf)
    }

    fn read_to_string(
        &mut self,
        buf: &mut String,
    ) -> Result<usize> {
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
        Self {
            inner,
            state,
        }
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
