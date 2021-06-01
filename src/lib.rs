#![allow(unused_imports)]


/// Details for those wishing to implement [`PeekRead`].
pub mod detail;

mod foreign_impl;
mod seekreader;
mod bufreader;

use std::io::*;
pub use bufreader::BufPeekReader;
pub use detail::cursor::PeekCursor;


/// A trait for a [`Read`] stream that supports peeking ahead in the stream.
///
/// In addition to a normal read cursor it can create a separate 'peek cursor' which can go ahead of the
/// regular read cursor, but never behind it. Reading from the peek cursor does not affect the read
/// cursor in any way.
///
/// [`unread`]: PeekBufReader::unread
pub trait PeekRead: Read {
    /// Returns a [`PeekCursor`] which implements [`BufRead`] + [`Seek`], allowing you to peek ahead
    /// in a stream of data. Reading from this or seeking on it won't affect the read cursor, only
    /// the peek cursor.
    ///
    /// You can't seek before the read cursor, `peek().seek(SeekFrom::Start(0))` is defined to be the read cursor position.
    ///
    /// By default reads from the [`PeekCursor`] are unbuffered where possible and will only read as
    /// much as necessary from the underlying stream, if reading can block or otherwise invokes a cost.
    /// To change this use [`PeekCursor::buffered`].
    fn peek(&mut self) -> PeekCursor<'_>;
}


/// A wrapper for a [`Read`] + [`Seek`] stream that implements [`PeekRead`] using seeking.
pub struct SeekPeekReader;
