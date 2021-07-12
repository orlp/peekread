#![no_main]

use core::fmt::Debug;
use peekread::{PeekRead, BufPeekReader, SeekPeekReader, PeekCursor};
use std::io::{BufRead, Read, Seek, Result, SeekFrom, Cursor};
use libfuzzer_sys::arbitrary::{self, Arbitrary};

#[derive(Arbitrary, Debug)]
pub struct Target {
    pub refdat: Vec<u8>,
    pub seqs: Vec<Seq>,
}

#[derive(Arbitrary, Debug)]
pub struct Seq {
    with_peek: bool,
    ops: Vec<Op>,
}

#[derive(Arbitrary, Debug)]
pub enum Op {
    Read(usize),

    FillBuff,
    Consume(usize),
    ReadExact(usize),
    ReadToEnd,
    ReadToString,

    SeekStart(u64),
    SeekEnd(i64),
    SeekCurrent(i64),
    StreamPosition,
}


trait HasSeek : Seek { }
impl<T: AsRef<[u8]>> HasSeek for Cursor<T> { }
impl<'a> HasSeek for PeekCursor<'a> { }

trait MaybeSeek {
    fn maybe_seek(&mut self, pos: SeekFrom) -> Result<u64> {
        Ok(0)
    }

    fn maybe_stream_position(&mut self) -> Result<u64> {
        Ok(0)
    }

    fn has_seek_impl() -> bool {
        false
    }
}

impl<T: HasSeek> MaybeSeek for T {
    fn maybe_seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.seek(pos)
    }

    fn maybe_stream_position(&mut self) -> Result<u64> {
        self.stream_position()
    }

    fn has_seek_impl() -> bool {
        true
    }
}

impl<T> MaybeSeek for BufPeekReader<T> {}



fn assert_reseq<T: Eq + Debug>(x: Result<T>, y: Result<T>) {
    match (x, y) {
        (Ok(xo), Ok(yo)) => assert_eq!(xo, yo),
        (Err(xe), Err(ye)) => assert_eq!(xe.kind(), ye.kind()),
        _ => assert!(false, "Result assert mismatch")
    }
}


fn check_ops<T: Read + BufRead + MaybeSeek, U: Read + BufRead + MaybeSeek>(ops: &[Op], refcursor: &mut T, peekcursor: &mut U) {
    let do_seek = T::has_seek_impl() && U::has_seek_impl();
    for op in ops {
        match *op {
            Op::Read(n) => {
                let mut vr = vec![0; n]; let mut vp = vec![0; n];
                assert_reseq(refcursor.read(&mut vr), peekcursor.read(&mut vp));
                assert_eq!(vr, vp);
            }
            Op::FillBuff => { refcursor.fill_buf(); peekcursor.fill_buf(); },
            Op::Consume(n) => { refcursor.consume(n); peekcursor.consume(n); },
            Op::ReadExact(n) => {
                let mut vr = vec![0; n]; let mut vp = vec![0; n];
                assert_reseq(refcursor.read_exact(&mut vr), peekcursor.read_exact(&mut vp));
                assert_eq!(vr, vp);
            }
            Op::ReadToEnd => {
                let mut vr = Vec::new(); let mut vp = Vec::new();
                assert_reseq(refcursor.read_to_end(&mut vr), peekcursor.read_to_end(&mut vp));
                assert_eq!(vr, vp);
            },
            Op::ReadToString => {
                let mut vr = String::new(); let mut vp = String::new();
                assert_reseq(refcursor.read_to_string(&mut vr), peekcursor.read_to_string(&mut vp));
                assert_eq!(vr, vp);
            }
            Op::SeekStart(n) =>   if do_seek { assert_reseq(refcursor.maybe_seek(SeekFrom::Start(n)), peekcursor.maybe_seek(SeekFrom::Start(n))) },
            Op::SeekEnd(n) =>     if do_seek { assert_reseq(refcursor.maybe_seek(SeekFrom::End(n)), peekcursor.maybe_seek(SeekFrom::End(n))) },
            Op::SeekCurrent(n) => if do_seek { assert_reseq(refcursor.maybe_seek(SeekFrom::Current(n)), peekcursor.maybe_seek(SeekFrom::Current(n))) },
            Op::StreamPosition => if do_seek { assert_reseq(refcursor.maybe_stream_position(), peekcursor.maybe_stream_position()) },
        }
    }
}


use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: Target| {
    let mut reference = Cursor::new(data.refdat);
    let mut peeked = BufPeekReader::new(reference.clone());

    for seq in &data.seqs {
        if seq.with_peek {
            let mut rest = Vec::new();
            reference.clone().read_to_end(&mut rest);
            check_ops(&seq.ops, &mut Cursor::new(rest), &mut peeked.peek())
        } else {
            check_ops(&seq.ops, &mut reference, &mut peeked)
        }
    }
});

