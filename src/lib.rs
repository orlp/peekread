//! This crate allows you to take an arbitrary [`Read`] stream and 'peek ahead'
//! into the stream without consuming the original stream.
//!
//! This is done through the [`PeekRead`] trait which has the method
//! [`peek`]. When this method is called it returns a new [`PeekCursor`] object implementing
//! [`Read`], [`BufRead`] and [`Seek`] that allows you to read from the stream
//! without affecting the original stream.
//! 
//! The [`PeekRead`] trait is directly
//! implemented on a select few types, but for most you will have to wrap your
//! type in a [`SeekPeekReader`] or [`BufPeekReader`] that implements the peeking
//! behavior using respectively seeking or buffering.
//! 
//! # Examples
//! One could try various different parsers on the same stream until one succeeds:
//! ```no_run
//! # use std::io::{Result, Read, BufRead};
//! # use std::fs::File;
//! # enum ParseResult { Html(()), Jpg(()), Png(()), Gif(()), Js(()), Unknown }
//! # fn parse_as_html<T>(f: T) -> () { () }
//! # fn parse_as_jpg<T>(f: T) -> Result<()> { Ok(()) }
//! # fn parse_as_gif<T>(f: T) -> Result<()> { Ok(()) }
//! # fn parse_as_png<T>(f: T) -> Result<()> { Ok(()) }
//! # fn parse_as_javascript<T>(f: T) -> Result<()> { Ok(()) }
//! # fn foo() -> Result<ParseResult> {
//! # use peekread::{PeekRead, SeekPeekReader};
//! let mut f = SeekPeekReader::new(File::open("ambiguous.txt")?);
//! 
//! // HTML is so permissive its parser never fails, so check for signature.
//! if f.starts_with("<!DOCTYPE html>\n") {
//!     Ok(ParseResult::Html(parse_as_html(f)))
//! } else {
//!     // Can pass PeekCursor to functions accepting T: Read without them
//!     // having to be aware of peekread.
//!     parse_as_jpg(f.peek()).map(ParseResult::Jpg)
//!        .or_else(|_| parse_as_png(f.peek()).map(ParseResult::Png))
//!        .or_else(|_| parse_as_gif(f.peek()).map(ParseResult::Gif))
//!        .or_else(|_| parse_as_javascript(f.peek()).map(ParseResult::Js))
//! }
//! # }
//! ```
//! 
//! [`peek`]: [`PeekRead::peek`]


/// Details for those wishing to implement [`PeekRead`].
pub mod detail;

mod bufreader;
mod foreign_impl;
mod seekreader;
mod util;

pub use bufreader::BufPeekReader;
pub use detail::cursor::PeekCursor;
pub use seekreader::SeekPeekReader;
use std::io::{Read, Result};
#[cfg(doc)]
use std::io::{BufRead, BufReader, Seek};

/// A trait for a [`Read`] stream that supports peeking ahead in the stream.
///
/// In addition to a normal read cursor it can create a separate 'peek cursor'
/// which can go ahead of the regular read cursor, but never behind it. Reading
/// from the peek cursor does not affect the read cursor in any way.
pub trait PeekRead: Read {
    /// Returns a [`PeekCursor`] which implements [`BufRead`] + [`Seek`],
    /// allowing you to peek ahead in a stream of data. Reading from this or
    /// seeking on it won't affect the read cursor, only the peek cursor.
    ///
    /// You can't seek before the read cursor, `peek().seek(SeekFrom::Start(0))`
    /// is defined to be the read cursor position.
    ///
    /// Despite implementing [`BufRead`] for convenience, by default reads from
    /// the [`PeekCursor`] are unbuffered where possible and will only read
    /// as much as necessary from the underlying stream, if reading can
    /// block or otherwise invoke a cost. This can be circumvented by
    /// buffering the underlying stream (e.g. with
    /// [`BufPeekReader::set_min_read_size`], or for [`SeekPeekReader`] by
    /// wrapping the inner stream in a [`BufReader`]), or one can wrap the
    /// peek cursor itself in [`BufReader`], although this will only buffer
    /// reads from this particular peek cursor.
    fn peek(&mut self) -> PeekCursor<'_>;

    /// Convenience method to check if the upcoming bytes in a stream equal the
    /// given string of bytes, without advancing the stream.
    fn starts_with<B: AsRef<[u8]>>(&mut self, bytes: B) -> Result<bool> {
        let bytes = bytes.as_ref();
        let mut buf = [0u8; 32]; // Prevent allocation, check 32 bytes at a time.
        let mut peeker = self.peek();
        for chunk in bytes.chunks(32) {
            let partial_buf = &mut buf[..chunk.len()];
            if let Err(e) = peeker.read_exact(partial_buf) {
                return match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => Ok(false),
                    _ => Err(e),
                };
            }

            if partial_buf != chunk {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Convenience method to consume a specific string of bytes if they are
    /// next up in the stream, leaving the stream unchanged otherwise. Returns
    /// whether the string was found and removed.
    fn consume_prefix<B: AsRef<[u8]>>(&mut self, bytes: B) -> Result<bool> {
        let bytes = bytes.as_ref();
        let should_strip = self.starts_with(bytes)?;
        if should_strip {
            std::io::copy(&mut self.take(bytes.len() as u64),
                          &mut std::io::sink())?;
        }
        Ok(should_strip)
    }
}
