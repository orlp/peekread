#![no_main]

use core::fmt::Debug;
use peekread::{PeekRead, BufPeekReader, SeekPeekReader, PeekCursor};
use std::io::{BufRead, Read, Seek, Result, SeekFrom, Cursor};
use libfuzzer_sys::arbitrary::{self, Arbitrary};

mod make_as_trait_impl;

#[derive(Arbitrary, Debug)]
pub enum Peeker {
    Buf,
    Seek,
    Cursor
}

#[derive(Arbitrary, Debug)]
pub struct Target {
    pub refdat: Vec<u8>,
    pub top_level_ops: Vec<TopLevelOp>,
    pub peeker: Peeker,
}


#[derive(Arbitrary, Debug)]
pub enum TopLevelOp {
    SeqWithPeek(Vec<Op>),
    SeqWithoutPeek(Vec<Op>),
    Unread(Vec<u8>),
}

#[derive(Arbitrary, Debug, Clone)]
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

impl Op {
    fn make_reasonable(self) -> Op {
        match self {
            Self::Read(n)        => Self::Read(n        % 10000),
            Self::Consume(n)     => Self::Consume(n     % 10000),
            Self::ReadExact(n)   => Self::ReadExact(n   % 10000),
            Self::SeekStart(n)   => Self::SeekStart(n   % 10000),
            Self::SeekEnd(n)     => Self::SeekEnd(n     % 10000),
            Self::SeekCurrent(n) => Self::SeekCurrent(n % 10000),
            unchanged            => unchanged,
        }
    }
}


make_as_trait!(Seek);
impl<T: AsRef<[u8]>> HasSeek for Cursor<T> { }
impl<'a> HasSeek for PeekCursor<'a> { }
impl<T: Seek + Read> HasSeek for SeekPeekReader<T> { }
impl<T> AsSeek for BufPeekReader<T> { }

make_as_trait!(BufRead);
impl<T: AsRef<[u8]>> HasBufRead for Cursor<T> { }
impl<'a> HasBufRead for PeekCursor<'a> { }
impl<T: Read> HasBufRead for BufPeekReader<T> { }
impl<T> AsBufRead for SeekPeekReader<T> { }


fn assert_reseq<T: Eq + Debug>(x: Result<T>, y: Result<T>) {
    match (x, y) {
        (Ok(xo), Ok(yo)) => assert_eq!(xo, yo),
        (Err(xe), Err(ye)) => assert_eq!(xe.kind(), ye.kind()),
        (x, y) => assert!(false, "Result assert mismatch, left = {:?}, right = {:?}", x, y)
    }
}


fn check_ops<T: Read + AsSeek + AsBufRead, U: Read + AsSeek + AsBufRead>(ops: &[Op], refcursor: &mut T, peekcursor: &mut U) {
    let mut ref_bufsize: usize = 0;
    let mut peek_bufsize: usize = 0;
    for op in ops {
        let op = op.clone().make_reasonable();
        println!("{:?}", &op);

        match op {
            Op::Read(n) => {
                let mut vr = vec![0; n]; let mut vp = vec![0; n];
                let rr = refcursor.read(&mut vr);
                let pr = peekcursor.read(&mut vp);
                if let &Ok(written) = &rr {
                    ref_bufsize = ref_bufsize.saturating_sub(written);
                    peek_bufsize = peek_bufsize.saturating_sub(written);
                }
                assert_reseq(rr, pr);
                assert_eq!(vr, vp);
            }
            Op::ReadExact(n) => {
                let mut vr = vec![0; n]; let mut vp = vec![0; n];
                let rr = refcursor.read_exact(&mut vr);
                let rp = peekcursor.read_exact(&mut vp);
                if rr.is_ok() { // Results are unspecified in failure.
                    ref_bufsize = ref_bufsize.saturating_sub(n);
                    peek_bufsize = peek_bufsize.saturating_sub(n);
                    assert_eq!(vr, vp);
                }
                assert_reseq(rr, rp);
            }
            Op::ReadToEnd => {
                let mut vr = Vec::new(); let mut vp = Vec::new();
                assert_reseq(refcursor.read_to_end(&mut vr), peekcursor.read_to_end(&mut vp));
                assert_eq!(vr, vp);
                ref_bufsize = 0; peek_bufsize = 0;
            },
            Op::ReadToString => {
                let mut vr = String::new(); let mut vp = String::new();
                assert_reseq(refcursor.read_to_string(&mut vr), peekcursor.read_to_string(&mut vp));
                assert_eq!(vr, vp);
                ref_bufsize = 0; peek_bufsize = 0;
            },
            _ => ()
        }

        if let (Some(refcursor), Some(peekcursor)) = (refcursor.as_seek_mut(), peekcursor.as_seek_mut()) {
            match op {
                Op::SeekStart(n) =>   assert_reseq(refcursor.seek(SeekFrom::Start(n)), peekcursor.seek(SeekFrom::Start(n))),
                Op::SeekEnd(n) =>     assert_reseq(refcursor.seek(SeekFrom::End(n)), peekcursor.seek(SeekFrom::End(n))),
                Op::SeekCurrent(n) => assert_reseq(refcursor.seek(SeekFrom::Current(n)), peekcursor.seek(SeekFrom::Current(n))),
                Op::StreamPosition => assert_reseq(refcursor.stream_position(), peekcursor.stream_position()),
                _ => ()
            }
        }

        if let (Some(refcursor), Some(peekcursor)) = (refcursor.as_buf_read_mut(), peekcursor.as_buf_read_mut()) {
            match op {
                Op::FillBuff => {
                    let ref_buf = refcursor.fill_buf().unwrap();
                    let peek_buf = peekcursor.fill_buf().unwrap();
                    ref_bufsize = ref_buf.len();
                    peek_bufsize = peek_buf.len();
                    let n = ref_bufsize.min(peek_bufsize);
                    assert_eq!(&ref_buf[..n], &peek_buf[..n]);
                },
                Op::Consume(n) => {
                    let n = n.min(ref_bufsize).min(peek_bufsize);
                    refcursor.consume(n); peekcursor.consume(n);
                    ref_bufsize -= n; peek_bufsize -= n;
                },
                _ => (),
            }
        }
    }

}


use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: Target| {

    let mut seek_reference = Cursor::new(data.refdat.clone());
    let mut buf_reference = Cursor::new(data.refdat.clone());
    let mut cursor_reference = Cursor::new(data.refdat);
    let mut seek_peeked = SeekPeekReader::new(seek_reference.clone());
    let mut buf_peeked = BufPeekReader::new(buf_reference.clone());
    let mut cursor_peeked = cursor_reference.clone();

    for top_level_op in &data.top_level_ops {
        match top_level_op {
            TopLevelOp::SeqWithPeek(ops) => {
                println!("seq with peek");
                let mut seek_rest = Vec::new();
                let mut buf_rest = Vec::new();
                let mut cursor_rest = Vec::new();
                seek_reference.clone().read_to_end(&mut seek_rest).unwrap();
                buf_reference.clone().read_to_end(&mut buf_rest).unwrap();
                cursor_reference.clone().read_to_end(&mut cursor_rest).unwrap();
                match data.peeker {
                    Peeker::Seek => check_ops(&ops, &mut Cursor::new(seek_rest), &mut seek_peeked.peek()),
                    Peeker::Buf => check_ops(&ops, &mut Cursor::new(buf_rest), &mut buf_peeked.peek()),
                    Peeker::Cursor => check_ops(&ops, &mut Cursor::new(cursor_rest), &mut cursor_peeked.peek()),
                };
            },
            TopLevelOp::SeqWithoutPeek(ops) => {
                println!("seq without peek");
                match data.peeker {
                    Peeker::Seek => check_ops(&ops, &mut seek_reference, &mut seek_peeked),
                    Peeker::Buf => check_ops(&ops, &mut buf_reference, &mut buf_peeked),
                    Peeker::Cursor => check_ops(&ops, &mut cursor_reference, &mut cursor_peeked),
                };
            },
            TopLevelOp::Unread(data) => {
                println!("unread {:?}", data);
                buf_peeked.unread(&data);

                let mut buf_rest = Vec::new();
                buf_reference.clone().read_to_end(&mut buf_rest).unwrap();
                buf_rest.splice(0..0, data.iter().copied());
                buf_reference = Cursor::new(buf_rest);
            }
        }
    }
});

