//! The writers that derive the blobs in `data/` from the vendored sources,
//! and the tests that keep the committed blobs in step with them.
//!
//! Run the tests with `LIVRE_FONT_DATA_UPDATE` set to rewrite the blobs.

use std::path::Path;

use crate::table::WORD;

/// The environment variable that makes [`assert_fresh`] rewrite blobs.
const UPDATE: &str = "LIVRE_FONT_DATA_UPDATE";

/// Lay `entries` out as a sorted string table (see `table.rs`).
///
/// Panics if two keys are equal or a key or value is longer than 255 bytes.
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

/// The entries of the Adobe Glyph List: each glyph name and the UTF-8 text
/// of its code points.
///
/// Panics on a malformed line or a code point that is not a Unicode scalar
/// value: the source is vendored, so either is a data error.
pub(crate) fn parse_glyph_list(text: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, codes) = line
                .split_once(';')
                .unwrap_or_else(|| panic!("no `;` in {line:?}"));
            let value: String = codes
                .split(' ')
                .map(|code| {
                    u32::from_str_radix(code, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .unwrap_or_else(|| panic!("bad code point {code:?} in {line:?}"))
                })
                .collect();
            (name.as_bytes().to_vec(), value.into_bytes())
        })
        .collect()
}

/// Check that the blob at `path`, relative to the crate root, holds `bytes`;
/// with `LIVRE_FONT_DATA_UPDATE` set, write `bytes` there instead.
pub(crate) fn assert_fresh(path: &str, bytes: Vec<u8>) {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    if std::env::var_os(UPDATE).is_some() {
        std::fs::write(&full, bytes).unwrap_or_else(|e| panic!("cannot write {path}: {e}"));
        return;
    }
    let stale = std::fs::read(&full).map_or(true, |committed| committed != bytes);
    assert!(
        !stale,
        "{path} is stale. Regenerate it with:\n    {UPDATE}=1 cargo test -p livre-font-data"
    );
}

#[test]
fn glyph_list_blob_is_fresh() {
    let source = include_str!("../data/agl/glyphlist.txt");
    assert_fresh(
        "data/agl/glyphlist.bin",
        write_table(&parse_glyph_list(source)),
    );
}
