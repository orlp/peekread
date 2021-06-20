pub fn add_offset(base: u64, offset: i64) -> u64 {
    (base as i64).saturating_add(offset).max(0) as u64
}
