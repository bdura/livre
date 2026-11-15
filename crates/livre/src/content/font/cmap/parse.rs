//! Reading a CMap: PostScript tokens on an operand stack, and the sections that drain it.
//!
//! Every operand a CMap uses is also PDF object syntax, so tokens are read with the
//! [`extraction`](crate::extraction) primitives. Only the sections that define mappings are
//! interpreted; the PostScript around them (`findresource`, `dict`, `begin`, `def`, ...)
//! clears the stack and is otherwise ignored.

use std::{collections::BTreeMap, fmt::Write};

use winnow::{
    combinator::{alt, delimited, fail, peek},
    dispatch,
    error::ErrMode,
    stream::AsChar,
    token::{any, one_of, take_while},
    BStr, ModalResult, Parser,
};

use super::{CMap, Code, CodespaceRange, Range, RangeDestination, WritingMode};
use crate::extraction::{
    multicomment0, take_till_delimiter, Extract, LiteralString, Name, RawDict,
};

/// An operand on a CMap's stack.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// An integer, as `begincodespacerange` counts and `/WMode` values are written.
    Integer(i64),
    /// A name: a glyph name in a `bfchar` destination, or a dictionary key.
    Name(Name),
    /// The bytes of a hex string with an even number of digits: `<41>` and `<0041>` stay
    /// distinct.
    Hex(Vec<u8>),
    /// An array, the destination of a `bfrange` that lists one destination per code.
    Array(Vec<Operand>),
    /// A dictionary, a literal string, a real, or a hex string with an odd number of digits,
    /// which is ambiguous as a code.
    Skipped,
}

/// What a CMap maps codes to. The type reads itself from its operand and knows how a
/// range steps through it.
pub trait Destination: Sized + Clone {
    /// The destination an operand of a `bf` or `cid` section spells, or `None`, with a
    /// warning, when it is malformed.
    fn from_operand(operand: &Operand) -> Option<Self>;

    /// The destination of the code `offset` codes after the one `self` is mapped from.
    fn stepped(&self, offset: u32) -> Option<Self>;
}

#[derive(Debug, PartialEq)]
enum Item<'de> {
    Operand(Operand),
    Keyword(&'de [u8]),
}

fn item<'de>(input: &mut &'de BStr) -> ModalResult<Item<'de>> {
    alt((operand.map(Item::Operand), keyword.map(Item::Keyword))).parse_next(input)
}

/// A bare identifier, or a PostScript procedure brace.
fn keyword<'de>(input: &mut &'de BStr) -> ModalResult<&'de [u8]> {
    alt((take_till_delimiter(1..), one_of(b"{}").take()))
        .parse_next(input)
        .map_err(ErrMode::Backtrack)
}

fn operand(input: &mut &BStr) -> ModalResult<Operand> {
    dispatch! {peek(any);
        b'/' => Name::extract.map(Operand::Name),
        b'[' => Vec::extract.map(Operand::Array),
        b'<' => alt((RawDict::recognize.value(Operand::Skipped), hex)),
        b'(' => LiteralString::recognize.value(Operand::Skipped),
        b'0'..=b'9' | b'+' | b'-' | b'.' => number,
        _ => fail,
    }
    .parse_next(input)
}

fn number(input: &mut &BStr) -> ModalResult<Operand> {
    alt((
        f64::extract
            .take()
            .verify(|bytes: &[u8]| bytes.contains(&b'.'))
            .value(Operand::Skipped),
        i64::extract.map(Operand::Integer),
    ))
    .parse_next(input)
}

impl Extract<'_> for Operand {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        operand(input)
    }
}

/// A hex string, read digit by digit rather than through
/// [`HexadecimalString`](crate::extraction::HexadecimalString), whose padding of an odd last
/// digit would turn `<0066006>` into a well-formed four-byte string.
fn hex(input: &mut &BStr) -> ModalResult<Operand> {
    let inside: &[u8] = delimited(
        b'<',
        take_while(0.., |b: u8| {
            b.is_hex_digit() || b.is_space() || b.is_newline()
        }),
        b'>',
    )
    .parse_next(input)?;

    let digits: Vec<u8> = inside
        .iter()
        .filter(|b| b.is_hex_digit())
        .map(|&b| (b as char).to_digit(16).expect("filtered") as u8)
        .collect();

    if digits.len() % 2 == 1 {
        tracing::warn!(
            "Hex string <{}> in a CMap has an odd number of digits: ignored",
            String::from_utf8_lossy(inside)
        );
        return Ok(Operand::Skipped);
    }

    let bytes = digits
        .chunks(2)
        .map(|pair| pair[0] << 4 | pair[1])
        .collect();
    Ok(Operand::Hex(bytes))
}

/// `bytes` as a hex string, for warnings.
pub(super) fn to_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(2 * bytes.len() + 2);
    hex.push('<');
    for byte in bytes {
        write!(hex, "{byte:02X}").expect("writing to a string");
    }
    hex.push('>');
    hex
}

/// The code a source operand spells: a hex string of one to four bytes.
fn code(operand: &Operand) -> Option<Code> {
    let Operand::Hex(bytes) = operand else {
        return None;
    };
    if !(1..=4).contains(&bytes.len()) {
        return None;
    }
    let value = bytes
        .iter()
        .fold(0u32, |value, &byte| value << 8 | u32::from(byte));
    Some(Code::new(value, bytes.len() as u8))
}

/// The low and high codes of a range, or `None`, with a warning, if they do not make one.
fn bounds(section: &str, low: &Operand, high: &Operand) -> Option<(Code, Code)> {
    let (Some(low_code), Some(high_code)) = (code(low), code(high)) else {
        tracing::warn!("{section} entry {low:?} {high:?} is not a pair of codes: dropped");
        return None;
    };
    if low_code.byte_count() != high_code.byte_count() {
        let (Operand::Hex(low), Operand::Hex(high)) = (low, high) else {
            unreachable!("both are codes");
        };
        tracing::warn!(
            "{section} entry {} {} has mismatched byte lengths: dropped",
            to_hex(low),
            to_hex(high)
        );
        return None;
    }
    if high_code.value() < low_code.value() {
        tracing::warn!("{section} entry {low_code:?} {high_code:?} is decreasing: dropped");
        return None;
    }
    Some((low_code, high_code))
}

/// The CMap being read: the stack, and what the sections have defined so far.
struct Reader<D> {
    stack: Vec<Operand>,
    codespace: Vec<CodespaceRange>,
    // A map, so that a code mapped twice keeps its last mapping.
    singles: BTreeMap<Code, D>,
    ranges: Vec<Range<D>>,
    writing_mode: WritingMode,
}

/// Whether the reader goes on after a keyword.
enum Flow {
    Continue,
    Stop,
}

impl<D: Destination> Reader<D> {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            codespace: Vec::new(),
            singles: BTreeMap::new(),
            ranges: Vec::new(),
            writing_mode: WritingMode::default(),
        }
    }

    fn keyword(&mut self, keyword: &[u8]) -> Flow {
        match keyword {
            b"endcodespacerange" => self.drain("codespacerange", 2, Self::codespace_range),
            b"endbfchar" => self.drain("bfchar", 2, Self::bf_char),
            b"endbfrange" => self.drain("bfrange", 3, Self::bf_range),
            b"usecmap" => {
                if let Some(Operand::Name(name)) = self.stack.last() {
                    tracing::warn!("usecmap /{} not supported", String::from_utf8_lossy(name));
                }
            }
            b"def" => {
                if let [.., Operand::Name(key), Operand::Integer(mode)] = self.stack.as_slice() {
                    if key.0 == b"WMode" {
                        self.writing_mode = match mode {
                            1 => WritingMode::Vertical,
                            _ => WritingMode::Horizontal,
                        };
                    }
                }
            }
            b"endcmap" => return Flow::Stop,
            // `begin...` keywords, and the PostScript scaffolding around the sections. The
            // count conventionally preceding a `begin...` keyword is ignored: a section has
            // the entries it has.
            _ => {}
        }
        self.stack.clear();
        Flow::Continue
    }

    /// Hand the stack to `entry` in chunks of `arity` operands.
    fn drain(&mut self, section: &str, arity: usize, entry: fn(&mut Self, &[Operand])) {
        let stack = std::mem::take(&mut self.stack);
        let chunks = stack.chunks_exact(arity);
        if !chunks.remainder().is_empty() {
            tracing::warn!(
                "{section} section ends with {} stray operands: ignored",
                chunks.remainder().len()
            );
        }
        for chunk in chunks {
            entry(self, chunk);
        }
    }

    fn codespace_range(&mut self, chunk: &[Operand]) {
        let [low, high] = chunk else {
            unreachable!("chunks of 2")
        };
        if let Some((low, high)) = bounds("codespacerange", low, high) {
            self.codespace.push(CodespaceRange {
                byte_count: low.byte_count(),
                low: low.value(),
                high: high.value(),
            });
        }
    }

    fn bf_char(&mut self, chunk: &[Operand]) {
        let [source, destination] = chunk else {
            unreachable!("chunks of 2")
        };
        let Some(code) = code(source) else {
            tracing::warn!("bfchar source {source:?} is not a code: dropped");
            return;
        };
        if let Some(destination) = D::from_operand(destination) {
            self.singles.insert(code, destination);
        }
    }

    fn bf_range(&mut self, chunk: &[Operand]) {
        let [low, high, destination] = chunk else {
            unreachable!("chunks of 3")
        };
        let Some((low, high)) = bounds("bfrange", low, high) else {
            return;
        };
        let destination = match destination {
            Operand::Array(operands) => {
                let Some(destinations) = operands.iter().map(D::from_operand).collect() else {
                    tracing::warn!("bfrange {low:?} {high:?} has a malformed destination: dropped");
                    return;
                };
                RangeDestination::Listed(destinations)
            }
            operand => match D::from_operand(operand) {
                Some(destination) => RangeDestination::Stepped(destination),
                None => return,
            },
        };
        self.ranges.push(Range {
            low,
            high: high.value(),
            destination,
        });
    }

    fn finish(self) -> CMap<D> {
        let mut ranges = self.ranges;
        ranges.sort_by_key(|range| range.low);
        CMap {
            codespace: self.codespace,
            singles: self.singles.into_iter().collect(),
            ranges,
            writing_mode: self.writing_mode,
        }
    }
}

impl<D: Destination> CMap<D> {
    /// Read a CMap from the decoded bytes of its stream.
    ///
    /// Never fails: entries that are malformed are dropped with a warning, and a CMap with an
    /// unparseable tail is the CMap read up to it, with a warning naming the byte offset.
    /// Reading stops at `endcmap`. `usecmap` is not supported: the parent's mappings are
    /// missing, with a warning.
    pub fn parse(bytes: &[u8]) -> Self {
        let mut reader = Reader::new();
        let mut input: &BStr = bytes.into();

        loop {
            // Cannot fail: zero or more comments and spaces.
            let _ = multicomment0(&mut input);
            if input.is_empty() {
                break;
            }
            match item(&mut input) {
                Ok(Item::Operand(operand)) => reader.stack.push(operand),
                Ok(Item::Keyword(keyword)) => {
                    if let Flow::Stop = reader.keyword(keyword) {
                        break;
                    }
                }
                Err(_) => {
                    tracing::warn!(
                        "CMap unparseable from byte {}: the rest is ignored",
                        bytes.len() - input.len()
                    );
                    break;
                }
            }
        }

        reader.finish()
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"<0041>", Operand::Hex(vec![0x00, 0x41]))]
    #[case(b"<41>", Operand::Hex(vec![0x41]))]
    #[case(b"<00 4 1>", Operand::Hex(vec![0x00, 0x41]))]
    #[case(b"<0066006>", Operand::Skipped)]
    #[case(b"<</A 1>>", Operand::Skipped)]
    #[case(b"(text)", Operand::Skipped)]
    #[case(b"12", Operand::Integer(12))]
    #[case(b"1.5", Operand::Skipped)]
    #[case(b"/WMode", Operand::Name("WMode".into()))]
    #[case(
        b"[<41> <0042>]",
        Operand::Array(vec![Operand::Hex(vec![0x41]), Operand::Hex(vec![0x00, 0x42])])
    )]
    #[case(
        b"[<41> % first\n<42>]",
        Operand::Array(vec![Operand::Hex(vec![0x41]), Operand::Hex(vec![0x42])])
    )]
    fn operands(#[case] input: &[u8], #[case] expected: Operand) {
        let mut input: &BStr = input.into();
        assert_eq!(item(&mut input).unwrap(), Item::Operand(expected));
        assert!(input.is_empty(), "{input:?}");
    }

    #[rstest]
    #[case(b"beginbfchar")]
    #[case(b"findresource")]
    #[case(b"{")]
    fn keywords(#[case] input: &[u8]) {
        let mut input_stream: &BStr = input.into();
        assert_eq!(item(&mut input_stream).unwrap(), Item::Keyword(input));
    }

    /// A destination that only checks the reader: the bytes of the hex string, the last one
    /// stepped.
    #[derive(Debug, Clone, PartialEq)]
    struct Bytes(Vec<u8>);

    impl Destination for Bytes {
        fn from_operand(operand: &Operand) -> Option<Self> {
            match operand {
                Operand::Hex(bytes) => Some(Self(bytes.clone())),
                _ => None,
            }
        }

        fn stepped(&self, offset: u32) -> Option<Self> {
            let mut bytes = self.0.clone();
            let last = bytes.last_mut()?;
            *last = last.checked_add(u8::try_from(offset).ok()?)?;
            Some(Self(bytes))
        }
    }

    fn get(cmap: &CMap<Bytes>, code: Code) -> Option<Vec<u8>> {
        cmap.get(code).map(|Bytes(bytes)| bytes)
    }

    #[test]
    fn one_and_two_byte_codes_do_not_collide() {
        let cmap = CMap::<Bytes>::parse(indoc! {b"
            2 beginbfchar
            <41> <01>
            <0041> <02>
            endbfchar
        "});

        assert_eq!(get(&cmap, Code::one_byte(0x41)), Some(vec![0x01]));
        assert_eq!(get(&cmap, Code::two_bytes(0x41)), Some(vec![0x02]));
    }

    #[test]
    fn a_range_is_one_entry() {
        let cmap = CMap::<Bytes>::parse(indoc! {b"
            1 beginbfrange
            <0000> <FFFF> <0000>
            endbfrange
        "});

        assert_eq!(cmap.ranges.len(), 1);
        assert_eq!(get(&cmap, Code::two_bytes(0x0012)), Some(vec![0x00, 0x12]));
        assert_eq!(get(&cmap, Code::one_byte(0x12)), None);
    }

    #[test]
    fn a_single_mapping_beats_a_range() {
        let cmap = CMap::<Bytes>::parse(indoc! {b"
            1 beginbfrange
            <10> <20> <00>
            endbfrange
            1 beginbfchar
            <15> <FF>
            endbfchar
        "});

        assert_eq!(get(&cmap, Code::one_byte(0x14)), Some(vec![0x04]));
        assert_eq!(get(&cmap, Code::one_byte(0x15)), Some(vec![0xFF]));
        assert_eq!(get(&cmap, Code::one_byte(0x16)), Some(vec![0x06]));
        assert_eq!(get(&cmap, Code::one_byte(0x21)), None);
        assert_eq!(get(&cmap, Code::one_byte(0x0F)), None);
    }

    #[test]
    fn codespace() {
        let cmap = CMap::<Bytes>::parse(indoc! {b"
            2 begincodespacerange
            <00> <80>
            <8140> <FFFC>
            endcodespacerange
        "});

        assert_eq!(
            cmap.codespace(),
            [
                CodespaceRange {
                    byte_count: 1,
                    low: 0x00,
                    high: 0x80
                },
                CodespaceRange {
                    byte_count: 2,
                    low: 0x8140,
                    high: 0xFFFC
                },
            ]
        );
    }

    #[test]
    fn writing_mode() {
        let horizontal = CMap::<Bytes>::parse(b"begincmap endcmap");
        let vertical = CMap::<Bytes>::parse(b"/WMode 1 def begincmap endcmap");

        assert_eq!(horizontal.writing_mode(), WritingMode::Horizontal);
        assert_eq!(vertical.writing_mode(), WritingMode::Vertical);
    }

    #[test]
    #[tracing_test::traced_test]
    fn postscript_is_skipped_and_endcmap_stops() {
        let cmap = CMap::<Bytes>::parse(indoc! {b"
            %!PS-Adobe-3.0 Resource-CMap
            /CIDInit /ProcSet findresource begin
            12 dict begin
            begincmap
            /CIDSystemInfo 3 dict dup begin
              /Registry (Adobe) def
              /Ordering (UCS) def
              /Supplement 0 def
            end def
            /CMapName /Adobe-Identity-UCS def
            /CMapType 2 def
            /Parent usecmap
            1 beginbfchar
            <41> <01>
            endbfchar
            endcmap
            1 beginbfchar
            <42> <02>
            endbfchar
            ) junk ]] >>
        "});

        assert_eq!(get(&cmap, Code::one_byte(0x41)), Some(vec![0x01]));
        assert_eq!(get(&cmap, Code::one_byte(0x42)), None);
        assert!(logs_contain("usecmap /Parent not supported"));
        assert!(!logs_contain("unparseable"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn an_unparseable_tail_keeps_what_was_read() {
        let cmap = CMap::<Bytes>::parse(indoc! {b"
            1 beginbfchar
            <41> <01>
            endbfchar
            ) junk
        "});

        assert_eq!(get(&cmap, Code::one_byte(0x41)), Some(vec![0x01]));
        assert!(logs_contain("CMap unparseable from byte 34"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn mismatched_range_lengths_are_dropped() {
        let cmap = CMap::<Bytes>::parse(indoc! {b"
            2 beginbfrange
            <41> <0041> <00>
            <42> <43> <10>
            endbfrange
        "});

        assert_eq!(cmap.ranges.len(), 1);
        assert_eq!(get(&cmap, Code::one_byte(0x41)), None);
        assert_eq!(get(&cmap, Code::one_byte(0x43)), Some(vec![0x11]));
        assert!(logs_contain(
            "bfrange entry <41> <0041> has mismatched byte lengths: dropped"
        ));
    }
}
