//! The sorted string table every blob in `tables/` is stored as, read in place.
//!
//! Layout, all integers little-endian:
//!
//! ```text
//! u32       n            number of records
//! u32 × n   offset[n]    byte offset of each record from the blob start,
//!                        in ascending bytewise key order
//! records, each:
//!   u8      key_len
//!   [u8]    key
//!   u8      value_len
//!   [u8]    value
//! ```
//!
//! The writer is `generate::write_table`, compiled for tests only.

use std::cmp::Ordering;

/// Size in bytes of the record count and of each index entry.
pub(crate) const WORD: usize = 4;

/// A sorted key-value table read in place from its blob.
#[derive(Clone, Copy)]
pub(crate) struct Table<'a> {
    bytes: &'a [u8],
    len: usize,
}

impl<'a> Table<'a> {
    /// The table over `bytes`. A count whose index does not fit in the blob
    /// is malformed, and the table is then empty.
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        let len = match bytes.first_chunk::<WORD>() {
            Some(word) => u32::from_le_bytes(*word) as usize,
            None => 0,
        };
        // The count and `len` offsets must fit: `(len + 1) * WORD <= bytes.len()`,
        // rearranged so that nothing can overflow on 32-bit targets.
        let fits = len <= (bytes.len() / WORD).saturating_sub(1);
        Self {
            bytes,
            len: if fits { len } else { 0 },
        }
    }

    /// The value under `key`, if any. Never panics: a malformed blob reads
    /// as a table with no matching key.
    pub(crate) fn get(&self, key: &[u8]) -> Option<&'a [u8]> {
        let (mut low, mut high) = (0, self.len());
        while low < high {
            let mid = low + (high - low) / 2;
            let (record_key, value) = self.record(mid)?;
            match record_key.cmp(key) {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => return Some(value),
            }
        }
        None
    }

    /// The number of records; `0` if the blob is too short to hold the count
    /// or the index it declares.
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    /// The little-endian `u32` at byte `at`.
    fn word(&self, at: usize) -> Option<usize> {
        let bytes = self.bytes.get(at..at.checked_add(WORD)?)?;
        let word = u32::from_le_bytes(bytes.try_into().ok()?);
        usize::try_from(word).ok()
    }

    /// The key and value of record `index`.
    fn record(&self, index: usize) -> Option<(&'a [u8], &'a [u8])> {
        let offset = self.word(index.checked_mul(WORD)?.checked_add(WORD)?)?;
        let (key, rest) = self.field(offset)?;
        let (value, _) = self.field(rest)?;
        Some((key, value))
    }

    /// The length-prefixed field at byte `at`, and the offset just past it.
    fn field(&self, at: usize) -> Option<(&'a [u8], usize)> {
        let len = usize::from(*self.bytes.get(at)?);
        let start = at.checked_add(1)?;
        let end = start.checked_add(len)?;
        Some((self.bytes.get(start..end)?, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::write_table;
    use rstest::rstest;

    fn entries() -> Vec<(Vec<u8>, Vec<u8>)> {
        vec![
            (b"beta".to_vec(), b"2".to_vec()),
            (b"alpha".to_vec(), b"1".to_vec()),
            (b"gamma".to_vec(), b"three".to_vec()),
        ]
    }

    #[rstest]
    #[case(b"alpha", b"1")]
    #[case(b"beta", b"2")]
    #[case(b"gamma", b"three")]
    fn round_trips_every_key(#[case] key: &[u8], #[case] value: &[u8]) {
        let blob = write_table(&entries());
        let table = Table::new(&blob);
        assert_eq!(table.len(), 3);
        assert_eq!(table.get(key), Some(value));
    }

    #[rstest]
    #[case(b"")]
    #[case(b"alph")]
    #[case(b"delta")]
    #[case(b"zeta")]
    fn misses_an_absent_key(#[case] key: &[u8]) {
        let blob = write_table(&entries());
        assert_eq!(Table::new(&blob).get(key), None);
    }

    #[rstest]
    #[case(&[])]
    #[case(&write_table(&[]))]
    fn misses_on_the_empty_table(#[case] blob: &[u8]) {
        let table = Table::new(blob);
        assert_eq!(table.len(), 0);
        assert_eq!(table.get(b"alpha"), None);
    }

    #[test]
    fn a_blob_truncated_in_its_index_misses_every_key() {
        let blob = write_table(&entries());
        // The count and the first index entry, then half of the second.
        let truncated = &blob[..2 * WORD + 2];
        let table = Table::new(truncated);
        assert_eq!(table.len(), 0);
        for (key, _) in entries() {
            assert_eq!(table.get(&key), None);
        }
    }

    #[rstest]
    #[case::just_past_the_end(0)]
    #[case::largest(u32::MAX - 1)]
    fn a_count_whose_index_overruns_the_blob_misses_every_key(#[case] excess: u32) {
        let mut blob = write_table(&entries());
        // The smallest count whose index ends past the blob, plus `excess`.
        let count = u32::try_from(blob.len() / WORD)
            .unwrap()
            .saturating_add(excess);
        blob[..WORD].copy_from_slice(&count.to_le_bytes());
        let table = Table::new(&blob);
        assert_eq!(table.len(), 0);
        for (key, _) in entries() {
            assert_eq!(table.get(&key), None);
        }
    }
}
