//! The descendant of a `Type0` font, a CIDFont, read for its metrics: horizontal widths from
//! `/W` and `/DW`, vertical metrics from `/W2` and `/DW2` (ISO 32000-2, 9.7.4).

use std::fmt;

use winnow::{
    combinator::{alt, trace},
    error::{ContextError, ErrMode, FromExternalError},
    BStr, ModalResult, Parser,
};

use super::{Cid, GlyphWidth};
use crate::{
    extraction::{Extract, Name, RawDict},
    follow_refs::{Build, BuildFromRawDict, Builder},
};

/// The descendant of a `Type0` font (ISO 32000-2, 9.7.4): CID-keyed metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct CidFont {
    /// Whether the glyphs are CFF or TrueType outlines; informational only.
    pub subtype: CidFontSubtype,
    /// The font's PostScript name, `/BaseFont`.
    pub base_font: Name,
    /// The horizontal advance of each CID, from `/W` and `/DW`.
    pub widths: CidWidths,
    /// The vertical advance and position vector of each CID, from `/W2` and `/DW2`.
    pub vertical: VerticalMetrics,
}

/// A CIDFont's `/Subtype`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CidFontSubtype {
    /// `CIDFontType0`: glyphs in a CFF font program.
    Type0,
    /// `CIDFontType2`: glyphs in a TrueType font program.
    Type2,
}

/// The cause attached to the error of a CIDFont with an unknown `/Subtype`.
#[derive(Debug)]
struct UnknownCidSubtype(Name);

impl fmt::Display for UnknownCidSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown CIDFont subtype /{}",
            String::from_utf8_lossy(&self.0)
        )
    }
}

impl std::error::Error for UnknownCidSubtype {}

/// `/CIDFontType0` or `/CIDFontType2`; any other name is a backtrack error naming it.
impl Extract<'_> for CidFontSubtype {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        let checkpoint = *input;
        let name = Name::extract(input)?;
        match name.0.as_slice() {
            b"CIDFontType0" => Ok(Self::Type0),
            b"CIDFontType2" => Ok(Self::Type2),
            _ => Err(ErrMode::Backtrack(ContextError::from_external_error(
                &checkpoint,
                UnknownCidSubtype(name),
            ))),
        }
    }
}

impl Build for CidFontSubtype {
    fn build<B>(input: &mut &BStr, _builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        Self::extract(input)
    }
}

/// `/W` and `/DW`: the horizontal advance of each CID, in thousandths of text space.
#[derive(Debug, Clone, PartialEq)]
pub struct CidWidths {
    /// `/DW`, 1000 when absent.
    default: GlyphWidth,
    /// Sorted by `first`, never overlapping.
    ranges: Vec<CidRange<GlyphWidth>>,
}

impl CidWidths {
    /// The width of `cid`: its `/W` entry, else `/DW`.
    pub fn width(&self, cid: Cid) -> GlyphWidth {
        lookup(&self.ranges, cid).unwrap_or(self.default)
    }
}

/// `/W2` and `/DW2`: the vertical metrics of each CID.
#[derive(Debug, Clone, PartialEq)]
pub struct VerticalMetrics {
    /// From `/DW2`, `[880 -1000]` when absent.
    default: VerticalMetric,
    /// Sorted by `first`, never overlapping.
    ranges: Vec<CidRange<VerticalMetric>>,
}

impl VerticalMetrics {
    /// The metric of `cid`: its `/W2` entry, else the `/DW2` default.
    pub fn metric(&self, cid: Cid) -> VerticalMetric {
        lookup(&self.ranges, cid).unwrap_or(self.default)
    }
}

/// A glyph's vertical metrics, in thousandths of text space (ISO 32000-2, 9.7.4.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerticalMetric {
    /// `w1y`, the vertical displacement of the current point; negative moves down.
    pub advance: f32,
    /// `vx`, the horizontal component of the position vector. `None` for the `/DW2` default,
    /// whose `vx` is half the CID's horizontal width.
    pub position_x: Option<f32>,
    /// `vy`, the vertical component of the position vector.
    pub position_y: f32,
}

impl VerticalMetric {
    /// The metric `/DW2 [vy w1y]` gives every CID absent from `/W2`.
    fn default_from([position_y, advance]: [f32; 2]) -> Self {
        Self {
            advance,
            position_x: None,
            position_y,
        }
    }
}

/// Consecutive CIDs from `first` to `last`, inclusive, sharing one value or each with its own.
#[derive(Debug, Clone, PartialEq)]
struct CidRange<T> {
    first: Cid,
    last: Cid,
    values: RangeValues<T>,
}

/// The values of a [`CidRange`]: `Each` holds exactly one value per CID of the range.
#[derive(Debug, Clone, PartialEq)]
enum RangeValues<T> {
    Same(T),
    Each(Vec<T>),
}

impl<T: Copy> CidRange<T> {
    fn get(&self, cid: Cid) -> T {
        match &self.values {
            RangeValues::Same(value) => *value,
            RangeValues::Each(values) => values[usize::from(cid.0 - self.first.0)],
        }
    }

    /// The part of the range from `first` to `last`, both within it.
    fn slice(&self, first: u16, last: u16) -> Self {
        let values = match &self.values {
            RangeValues::Same(value) => RangeValues::Same(*value),
            RangeValues::Each(values) => {
                let offset = usize::from(first - self.first.0);
                let length = usize::from(last - first) + 1;
                RangeValues::Each(values[offset..offset + length].to_vec())
            }
        };
        Self {
            first: Cid(first),
            last: Cid(last),
            values,
        }
    }
}

/// The value of `cid` in `ranges`, sorted by `first` and disjoint, if one covers it.
fn lookup<T: Copy>(ranges: &[CidRange<T>], cid: Cid) -> Option<T> {
    let index = ranges
        .partition_point(|range| range.first <= cid)
        .checked_sub(1)?;
    let range = &ranges[index];
    (cid <= range.last).then(|| range.get(cid))
}

/// Adds `range` to `ranges`, sorted by `first` and disjoint. The CIDs `ranges` already covers
/// keep their value: the first entry of the array wins, and an overlap warns.
fn insert<T: Copy>(ranges: &mut Vec<CidRange<T>>, range: CidRange<T>, key: &str) {
    let start = ranges.partition_point(|existing| existing.last < range.first);
    let mut pieces = Vec::new();
    let mut cursor = u32::from(range.first.0);
    for existing in ranges[start..]
        .iter()
        .take_while(|existing| existing.first <= range.last)
    {
        if u32::from(existing.first.0) > cursor {
            pieces.push((cursor, u32::from(existing.first.0) - 1));
        }
        cursor = cursor.max(u32::from(existing.last.0) + 1);
    }
    if cursor <= u32::from(range.last.0) {
        pieces.push((cursor, u32::from(range.last.0)));
    }

    if pieces != [(u32::from(range.first.0), u32::from(range.last.0))] {
        tracing::warn!(
            "`/{key}` entry for CIDs {} to {} overlaps an earlier one, which wins",
            range.first.0,
            range.last.0
        );
    }
    for (first, last) in pieces {
        // Both bounds lie within `range`, so within `u16`.
        let piece = range.slice(first as u16, last as u16);
        let index = ranges.partition_point(|existing| existing.first < piece.first);
        ranges.insert(index, piece);
    }
}

/// An element of a `/W` or `/W2` array: a CID or a metric, or an array of metrics.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WElement {
    Number(f32),
    Array(Vec<f32>),
}

impl Extract<'_> for WElement {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace(
            "livre-w-element",
            alt((
                Vec::<f32>::extract.map(Self::Array),
                f32::extract.map(Self::Number),
            )),
        )
        .parse_next(input)
    }
}

/// The enclosing array's [`Build`] resolves a referenced element, so this only extracts.
impl Build for WElement {
    fn build<B>(input: &mut &BStr, _builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        Self::extract(input)
    }
}

/// `number` as a CID, if it is an integer in `0..=65535`.
fn to_cid(number: f32) -> Option<Cid> {
    (number.fract() == 0.0 && (0.0..=f32::from(u16::MAX)).contains(&number))
        .then_some(Cid(number as u16))
}

/// Folds a `/W` or `/W2` array into disjoint ranges sorted by `first`, a metric being
/// `ARITY` numbers turned into a value by `metric`.
///
/// `c [m m ...]` gives consecutive CIDs from `c` one metric each; `cfirst clast m` gives the
/// whole range one metric. Anything else warns and stops the fold, keeping the ranges before
/// it. An array whose length is not a multiple of `ARITY` warns and drops its remainder.
fn fold<T: Copy, const ARITY: usize>(
    elements: &[WElement],
    key: &str,
    metric: impl Fn([f32; ARITY]) -> T,
) -> Vec<CidRange<T>> {
    let mut ranges = Vec::new();
    let mut rest = elements;
    while !rest.is_empty() {
        let (range, consumed) = match rest {
            [WElement::Number(first), WElement::Array(values), ..] => {
                let Some(first) = to_cid(*first) else {
                    tracing::warn!("`/{key}` CID {first} is not a CID: rest of the array ignored");
                    break;
                };
                let chunks = values.chunks_exact(ARITY);
                if !chunks.remainder().is_empty() {
                    tracing::warn!(
                        "`/{key}` array from CID {} has {} numbers, not a multiple of {ARITY}: \
                         remainder dropped",
                        first.0,
                        values.len()
                    );
                }
                let mut values: Vec<T> = chunks
                    .map(|chunk| metric(chunk.try_into().expect("chunk of ARITY numbers")))
                    .collect();
                let room = usize::from(u16::MAX - first.0) + 1;
                if values.len() > room {
                    tracing::warn!(
                        "`/{key}` array from CID {} runs past CID 65535: cut",
                        first.0
                    );
                    values.truncate(room);
                }
                let range = (!values.is_empty()).then(|| CidRange {
                    first,
                    // `values` fits in `room`, so the sum stays within `u16`.
                    last: Cid(first.0 + (values.len() - 1) as u16),
                    values: RangeValues::Each(values),
                });
                (range, 2)
            }
            [WElement::Number(first), WElement::Number(last), tail @ ..]
                if tail.len() >= ARITY
                    && tail[..ARITY]
                        .iter()
                        .all(|element| matches!(element, WElement::Number(_))) =>
            {
                let (Some(first), Some(last)) = (to_cid(*first), to_cid(*last)) else {
                    tracing::warn!(
                        "`/{key}` range {first} {last} does not bound CIDs: rest of the array \
                         ignored"
                    );
                    break;
                };
                let numbers: [f32; ARITY] = std::array::from_fn(|i| match tail[i] {
                    WElement::Number(number) => number,
                    WElement::Array(_) => unreachable!("checked by the guard"),
                });
                let range = if first <= last {
                    Some(CidRange {
                        first,
                        last,
                        values: RangeValues::Same(metric(numbers)),
                    })
                } else {
                    tracing::warn!("`/{key}` range {} {} is empty: ignored", first.0, last.0);
                    None
                };
                (range, 2 + ARITY)
            }
            _ => {
                tracing::warn!(
                    "`/{key}` has {} malformed trailing elements: ignored",
                    rest.len()
                );
                break;
            }
        };
        if let Some(range) = range {
            insert(&mut ranges, range, key);
        }
        rest = &rest[consumed..];
    }
    ranges
}

impl CidWidths {
    /// The widths from `/DW`, 1000 when absent, and the elements of `/W`.
    fn from_parts(default: Option<f32>, elements: Option<&[WElement]>) -> Self {
        Self {
            default: GlyphWidth(default.unwrap_or(1000.0)),
            ranges: elements.map_or_else(Vec::new, |elements| {
                fold(elements, "W", |[width]: [f32; 1]| GlyphWidth(width))
            }),
        }
    }
}

impl VerticalMetrics {
    /// The metrics from `/DW2`, `[880 -1000]` when absent, and the elements of `/W2`.
    fn from_parts(default: Option<[f32; 2]>, elements: Option<&[WElement]>) -> Self {
        Self {
            default: VerticalMetric::default_from(default.unwrap_or([880.0, -1000.0])),
            ranges: elements.map_or_else(Vec::new, |elements| {
                fold(elements, "W2", |[advance, x, y]: [f32; 3]| VerticalMetric {
                    advance,
                    position_x: Some(x),
                    position_y: y,
                })
            }),
        }
    }
}

#[derive(Debug, BuildFromRawDict)]
struct CidFontDict {
    subtype: CidFontSubtype,
    base_font: Name,
    #[livre(rename = "DW")]
    dw: Option<f32>,
    #[livre(rename = "W")]
    w: Option<Vec<WElement>>,
    #[livre(rename = "DW2")]
    dw2: Option<[f32; 2]>,
    #[livre(rename = "W2")]
    w2: Option<Vec<WElement>>,
}

/// Reads `/Subtype`, `/BaseFont`, `/DW`, `/W`, `/DW2` and `/W2`. A `/Subtype` other than
/// `CIDFontType0` or `CIDFontType2` is a backtrack error labelled `/Subtype`; malformed
/// `/W` or `/W2` entries warn and are dropped.
impl BuildFromRawDict for CidFont {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let CidFontDict {
            subtype,
            base_font,
            dw,
            w,
            dw2,
            w2,
        } = CidFontDict::build_from_raw_dict(dict, builder)?;

        Ok(Self {
            subtype,
            base_font,
            widths: CidWidths::from_parts(dw, w.as_deref()),
            vertical: VerticalMetrics::from_parts(dw2, w2.as_deref()),
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// The CIDFont in the dictionary literal `input`, with no reference to follow.
    fn cid_font(input: &[u8]) -> CidFont {
        CidFont::build(&mut input.as_ref(), &()).unwrap()
    }

    /// A `CIDFontType2` dictionary literal with `entries` appended.
    fn dict(entries: &str) -> Vec<u8> {
        format!("<</Type/Font/Subtype/CIDFontType2/BaseFont/Calibri {entries}>>").into_bytes()
    }

    fn width(font: &CidFont, cid: u16) -> f32 {
        font.widths.width(Cid(cid)).0
    }

    #[rstest]
    #[case(1, 722.0)]
    #[case(2, 667.0)]
    #[case(3, 600.0)]
    #[case(5, 600.0)]
    #[case(6, 500.0)]
    #[case(0, 500.0)]
    fn widths_in_both_forms(#[case] cid: u16, #[case] expected: f32) {
        let font = cid_font(&dict("/DW 500 /W [1 [722 667] 3 5 600]"));
        assert_eq!(width(&font, cid), expected);
    }

    #[test]
    fn no_widths_gives_1000() {
        let font = cid_font(&dict(""));
        for cid in [0, 1, 500, u16::MAX] {
            assert_eq!(width(&font, cid), 1000.0);
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn overlap_keeps_the_first_entry() {
        let font = cid_font(&dict("/W [1 [722] 1 2 600]"));

        assert_eq!(width(&font, 1), 722.0);
        // The later range keeps the CIDs the earlier one does not cover.
        assert_eq!(width(&font, 2), 600.0);
        assert_eq!(width(&font, 3), 1000.0);
        assert!(logs_contain("overlaps an earlier one"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn dangling_number_keeps_the_entries_before() {
        let font = cid_font(&dict("/W [1 [722] 3]"));

        assert_eq!(width(&font, 1), 722.0);
        assert_eq!(width(&font, 3), 1000.0);
        assert!(logs_contain("malformed trailing elements"));
    }

    #[test]
    fn vertical_metrics_in_both_forms() {
        let font = cid_font(&dict("/W2 [1 [-500 250 880] 2 3 -600 300 900]"));
        let metric = |cid| font.vertical.metric(Cid(cid));

        assert_eq!(
            metric(1),
            VerticalMetric {
                advance: -500.0,
                position_x: Some(250.0),
                position_y: 880.0
            }
        );
        assert_eq!(metric(3).advance, -600.0);
        assert_eq!(
            metric(4),
            VerticalMetric {
                advance: -1000.0,
                position_x: None,
                position_y: 880.0
            }
        );
    }

    #[test]
    fn vertical_default_from_dw2() {
        let font = cid_font(&dict("/DW2 [800 -900]"));

        assert_eq!(
            font.vertical.metric(Cid(7)),
            VerticalMetric {
                advance: -900.0,
                position_x: None,
                position_y: 800.0
            }
        );
    }

    #[rstest]
    #[case(b"CIDFontType0", CidFontSubtype::Type0)]
    #[case(b"CIDFontType2", CidFontSubtype::Type2)]
    fn both_subtypes_parse(#[case] subtype: &[u8], #[case] expected: CidFontSubtype) {
        let input = [b"<</Type/Font/Subtype/", subtype, b"/BaseFont/Calibri>>"].concat();

        let font = cid_font(&input);
        assert_eq!(font.subtype, expected);
        assert_eq!(font.base_font, Name::from("Calibri"));
    }

    #[test]
    fn unknown_subtype_is_labelled() {
        let error = CidFont::build(
            &mut b"<</Type/Font/Subtype/Foo/BaseFont/Calibri>>"
                .as_slice()
                .as_ref(),
            &(),
        )
        .unwrap_err()
        .into_inner()
        .unwrap();

        let message = error.to_string();
        assert!(message.contains("/Subtype"), "{message}");
        assert!(message.contains("Foo"), "{message}");
    }
}
