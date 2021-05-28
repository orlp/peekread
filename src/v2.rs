
pub trait PeekRead<'a>: BufRead {
    type Peek: BufRead + Seek;

    /// Returns a seekable Peek object that you can read from.
    /// It will act like a stream containing the remaining contents
    /// of `self`. Reading from the [`Peek`] does not consume
    /// from the original stream, where the data can be read again.
    pub fn peek(&'a mut self) -> Peek;

    /// Push data onto the front of the stream.
    fn unread(&'a mut self, data: &[u8]);
}