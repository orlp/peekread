pub(crate) mod cursor;

use std::io::*;
use crate::{PeekRead, PeekCursor};
pub use cursor::PeekCursorState;
use cursor::DefaultImplPeekCursor;

/// A helper trait used to implement [`PeekRead`].
///
/// In order to implement [`PeekRead`] for one of your types you must first implement this trait
/// on your type and then implement [`PeekRead::peek`] returning a [`PeekCursor`] (which you'll
/// find you can only construct for types implementing [`PeekReadImpl`]).
///
/// The [`PeekCursor`] contains a [`PeekCursorState`] object. In this object there is some storage
/// available to aid you in case the object you're implementing [`PeekReadImpl`] on does not
/// have the needed storage available to keep the cursor state (e.g. the [`PeekRead`] implementation
/// for `&[u8]`).
pub trait PeekReadImpl: BufRead {
    /// Used to implement `self.peek().seek(pos)`. See [`Seek::seek`].
    fn peek_seek(&mut self, state: &mut PeekCursorState, pos: SeekFrom) -> Result<u64>;
    
    /// Used to implement `self.peek().read(buf)`. See [`Read::read`].
    fn peek_read(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<usize>;

    /// Used to implement `self.peek().fill_buf()`. See [`BufRead::fill_buf`].
    fn peek_fill_buf(&mut self, state: &mut PeekCursorState) -> Result<&[u8]>;

    /// Used to implement `self.peek().consume()`. See [`BufRead::consume`].
    fn peek_consume(&mut self, state: &mut PeekCursorState, amt: usize);


    // Start default methods.
    /// Used to implement `self.peek().stream_position()`. See [`Seek::stream_position`].
    fn peek_stream_position(&mut self, state: &mut PeekCursorState) -> Result<u64> {
        DefaultImplPeekCursor::new(self, state).stream_position()
    }

    /// Used to implement `self.peek().read_exact(buf)`. See [`Read::read_exact`].
    fn peek_read_exact(&mut self, state: &mut PeekCursorState, buf: &mut [u8]) -> Result<()> {
        DefaultImplPeekCursor::new(self, state).read_exact(buf)
    }

    /// Used to implement `self.peek().read_to_end(buf)`. See [`Read::read_to_end`].
    fn peek_read_to_end(
        &mut self,
        state: &mut PeekCursorState,
        buf: &mut Vec<u8>,
    ) -> Result<usize> {
        DefaultImplPeekCursor::new(self, state).read_to_end(buf)
    }

    /// Used to implement `self.peek().read_to_string(buf)`. See [`Read::read_to_string`].
    fn peek_read_to_string(
        &mut self,
        state: &mut PeekCursorState,
        buf: &mut String,
    ) -> Result<usize> {
        DefaultImplPeekCursor::new(self, state).read_to_string(buf)
    }
}
