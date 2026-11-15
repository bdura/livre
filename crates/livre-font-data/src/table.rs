//! The sorted string table every blob in `tables/` is stored as, read in place.
//!
//! Layout, all integers little-endian:
//!
//! ```text
//! ┌───────┬───────────┬───────────┬───┬─────────────┬──────────┬──────────┬───┐
//! │ u32 n │ offset[0] │ offset[1] │ … │ offset[n-1] │ record 0 │ record 1 │ … │
//! └───────┴─────┬─────┴───────────┴───┴─────────────┴──────────┴──────────┴───┘
//!               │  u32 each, from the blob start,       ▲
//!               └───────────────────────────────────────┘
//!                  in ascending bytewise key order
//!
//! record: ┌─────────┬─────┬───────────┬───────┐
//!         │ key_len │ key │ value_len │ value │   lengths are u8
//!         └─────────┴─────┴───────────┴───────┘
//! ```
//!
//! Records vary in length, so the offsets give the random access that
//! [`Table::get`]'s binary search needs: about log₂ n key comparisons, each
//! on bytes read in place, with no allocation.
//!
//! The reader assumes nothing it could check, even where the data is known:
//! every read is bounds-checked, every value carries its length although AFM
//! widths are always two bytes, and the glyph list's values are checked as
//! UTF-8 although the writer only emits valid text. One format serves every
//! blob, the crate forbids `unsafe`, and a corrupt blob then reads as a miss
//! rather than a panic or undefined behaviour, at the cost of a few bytes and
//! comparisons per lookup.
//!
//! The writer, `write_table`, is compiled for tests only: the blobs are
//! generated and committed, never built at run time.

use std::cmp::Ordering;

/// Size in bytes of the record count and of each index entry.
const WORD: usize = 4;

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

    /// The value under `key`, if any, by binary search over the offsets.
    /// Never panics: a malformed blob reads as a table with no matching key.
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

/// Lay `entries` out in the layout above, the inverse of [`Table::get`].
///
/// Panics if two keys are equal or a key or value is longer than 255 bytes.
#[cfg(test)]
pub(crate) fn write_table(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for pair in sorted.windows(2) {
        assert_ne!(pair[0].0, pair[1].0, "duplicate key");
    }

    let count = u32::try_from(sorted.len()).expect("too many records");
    let mut index = Vec::with_capacity(WORD * (sorted.len() + 1));
    index.extend_from_slice(&count.to_le_bytes());
    let mut records = Vec::new();
    let records_start = WORD * (sorted.len() + 1);

    for (key, value) in sorted {
        let offset = u32::try_from(records_start + records.len()).expect("blob too large");
        index.extend_from_slice(&offset.to_le_bytes());
        for field in [key, value] {
            let len = u8::try_from(field.len()).expect("field longer than 255 bytes");
            records.push(len);
            records.extend_from_slice(field);
        }
    }

    index.extend_from_slice(&records);
    index
}

#[cfg(test)]
mod tests {
    use super::*;
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
