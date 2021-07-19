use std::convert::TryInto;
use std::io::{Error, ErrorKind, Result};

pub fn seek_add_offset(current: u64, offset: i64) -> Result<u64> {
    current
        .try_into()
        .ok()
        .and_then(|n: i64| n.checked_add(offset))
        .and_then(|n| n.try_into().ok())
        .ok_or_else(|| Error::new(
            ErrorKind::InvalidInput,
            "invalid seek to a negative or overflowing position",
        ))
}
