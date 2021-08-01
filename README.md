# peekread

This crate allows you to take an arbitrary `Read` stream and 'peek ahead'
into the stream without consuming the original stream. This is done through the
`PeekRead` trait which has the method `peek`. When this method is called it
returns a new `PeekCursor` object implementing `Read`, `BufRead` and `Seek` that
allows you to read from the stream without affecting the original stream.
The `PeekRead` trait is directly implemented on a select few types, but for most
you will have to wrap your type in a `SeekPeekReader` or `BufPeekReader` that
implements the peeking behavior using respectively seeking or buffering.
Please refer to the [**the documentation**](https://docs.rs/peekread) for more information.

The minimum required stable Rust version for `peekread` is 1.31.0. To start using
`peekread` add the following to your `Cargo.toml`:

```toml
[dependencies]
peekread = "0.1"
```

# Example

A short example:

```rust
use peekread::{PeekRead, SeekPeekReader};

let mut f = SeekPeekReader::new(File::open("ambiguous")?);

// HTML is so permissive its parser never fails, so check for signature.
if f.starts_with("<!DOCTYPE") {
    Ok(ParseResult::Html(parse_as_html(f)))
} else {
    // Can pass PeekCursor to functions accepting T: Read without them
    // having to be aware of peekread.
    parse_as_jpg(f.peek()).map(ParseResult::Jpg)
       .or_else(|_| parse_as_png(f.peek()).map(ParseResult::Png))
       .or_else(|_| parse_as_gif(f.peek()).map(ParseResult::Gif))
       .or_else(|_| parse_as_javascript(f.peek()).map(ParseResult::Js))
}
```

# License

`peekread` is released under the Zlib license, a permissive license. It is
OSI and FSF approved and GPL compatible.