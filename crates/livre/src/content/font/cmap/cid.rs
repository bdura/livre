//! The destination of an encoding CMap: CIDs (ISO 32000-2, 9.7.5 and 9.7.6).

use super::{parse::to_hex, CMap, Code, CodespaceRange, Destination, Operand, WritingMode};
use crate::extraction::Name;

/// A character identifier: the index a CIDFont's widths and glyphs are keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cid(pub u16);

impl Destination for Cid {
    /// An integer in `0..=65535`, as `cidchar` and `cidrange` write it.
    fn from_operand(operand: &Operand) -> Option<Self> {
        match operand {
            Operand::Integer(value) => match u16::try_from(*value) {
                Ok(cid) => Some(Self(cid)),
                Err(_) => {
                    tracing::warn!("CID destination {value} is out of range: dropped");
                    None
                }
            },
            Operand::Hex(bytes) => {
                tracing::warn!(
                    "CID destination {} is not an integer: dropped",
                    to_hex(bytes)
                );
                None
            }
            operand => {
                tracing::warn!("CID destination {operand:?} is not an integer: dropped");
                None
            }
        }
    }

    /// The CID `offset` after this one; `None` past 65535.
    fn stepped(&self, offset: u32) -> Option<Self> {
        let offset = u16::try_from(offset).ok()?;
        self.0.checked_add(offset).map(Self)
    }
}

/// The codespace of `Identity-H` and `Identity-V`: every two-byte code.
pub static IDENTITY_CODESPACE: [CodespaceRange; 1] = [CodespaceRange::new(2, 0, 0xFFFF)];

/// How a `Type0` font's `/Encoding` splits strings into codes and maps them to CIDs
/// (ISO 32000-2, 9.7.5).
#[derive(Debug, Clone, PartialEq)]
pub enum EncodingCMap {
    /// `Identity-H` or `Identity-V`: two-byte codes, CID equal to code.
    Identity(WritingMode),
    /// An embedded CMap stream.
    Embedded {
        /// The stream's codespace ranges and `cidchar` and `cidrange` mappings.
        cmap: CMap<Cid>,
        /// Set by `/Identity-H usecmap` or `/Identity-V usecmap`: a two-byte code the CMap
        /// does not map is its own CID, and the codespace includes every two-byte code.
        use_identity: bool,
    },
}

impl EncodingCMap {
    /// The predefined CMap `name` (ISO 32000-2, Table 118).
    ///
    /// Only the identities are shipped: any other name, such as `UniGB-UCS2-H`, warns and
    /// is read as `Identity-H`.
    pub fn from_name(name: &Name) -> Self {
        match name.0.as_slice() {
            b"Identity-H" => Self::Identity(WritingMode::Horizontal),
            b"Identity-V" => Self::Identity(WritingMode::Vertical),
            other => {
                tracing::warn!(
                    "predefined CMap /{} not shipped: treated as Identity-H",
                    String::from_utf8_lossy(other)
                );
                Self::Identity(WritingMode::Horizontal)
            }
        }
    }

    /// An embedded CMap, from the decoded `content` of its stream and the `/WMode` of the
    /// stream dictionary.
    ///
    /// A `/WMode` in the body wins; the dictionary's is the fallback (1 is vertical, any
    /// other value horizontal). Never fails, as [`CMap::parse`].
    pub fn from_stream(content: &[u8], dict_wmode: Option<u8>) -> Self {
        let mut cmap = CMap::<Cid>::parse(content);

        if cmap.writing_mode.is_none() {
            cmap.writing_mode = dict_wmode.map(|mode| match mode {
                1 => WritingMode::Vertical,
                _ => WritingMode::Horizontal,
            });
        }

        let use_identity = cmap.use_identity;
        // The parent's codespace is inherited along with its mappings.
        if use_identity && !cmap.codespace.contains(&IDENTITY_CODESPACE[0]) {
            cmap.codespace.push(IDENTITY_CODESPACE[0]);
        }

        Self::Embedded { cmap, use_identity }
    }

    /// The CID `code` maps to, or `None` when the CMap does not map it.
    pub fn cid(&self, code: Code) -> Option<Cid> {
        let identity = || {
            (code.byte_count() == 2)
                .then(|| u16::try_from(code.value()).ok().map(Cid))
                .flatten()
        };
        match self {
            Self::Identity(_) => identity(),
            Self::Embedded { cmap, use_identity } => cmap
                .get(code)
                .or_else(|| use_identity.then(identity).flatten()),
        }
    }

    /// The codespace ranges strings are split by.
    pub fn codespace(&self) -> &[CodespaceRange] {
        match self {
            Self::Identity(_) => &IDENTITY_CODESPACE,
            Self::Embedded { cmap, .. } => cmap.codespace(),
        }
    }

    /// The direction glyphs are laid out in.
    pub fn writing_mode(&self) -> WritingMode {
        match self {
            Self::Identity(mode) => *mode,
            Self::Embedded { cmap, .. } => cmap.writing_mode(),
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[test]
    fn cid_range() {
        let cmap = CMap::<Cid>::parse(indoc! {b"
            1 begincidrange
            <20> <7E> 1
            endcidrange
        "});

        assert_eq!(cmap.get(Code::one_byte(0x20)), Some(Cid(1)));
        assert_eq!(cmap.get(Code::one_byte(0x41)), Some(Cid(34)));
        assert_eq!(cmap.get(Code::one_byte(0x7F)), None);
    }

    #[test]
    fn cid_char() {
        let cmap = CMap::<Cid>::parse(indoc! {b"
            1 begincidchar
            <8140> 200
            endcidchar
        "});

        assert_eq!(cmap.get(Code::two_bytes(0x8140)), Some(Cid(200)));
        assert_eq!(cmap.get(Code::one_byte(0x81)), None);
    }

    #[test]
    fn identity_usecmap() {
        let encoding = EncodingCMap::from_stream(
            indoc! {b"
                /Identity-H usecmap
                1 begincidchar
                <0041> 5
                endcidchar
            "},
            None,
        );

        assert_eq!(encoding.cid(Code::two_bytes(0x0041)), Some(Cid(5)));
        assert_eq!(encoding.cid(Code::two_bytes(0x0042)), Some(Cid(0x42)));
        assert_eq!(encoding.codespace(), IDENTITY_CODESPACE);
        assert_eq!(encoding.writing_mode(), WritingMode::Horizontal);
    }

    #[test]
    #[tracing_test::traced_test]
    fn other_usecmap_is_not_supported() {
        let encoding = EncodingCMap::from_stream(b"/UniJIS-UCS2-H usecmap", None);

        assert_eq!(encoding.cid(Code::two_bytes(0x0041)), None);
        assert!(logs_contain("usecmap /UniJIS-UCS2-H not supported"));
    }

    #[rstest]
    #[case::body_vertical(b"/WMode 1 def".as_slice(), None, WritingMode::Vertical)]
    #[case::dictionary_vertical(b"".as_slice(), Some(1), WritingMode::Vertical)]
    #[case::body_wins(b"/WMode 0 def".as_slice(), Some(1), WritingMode::Horizontal)]
    #[case::identity_v(b"/Identity-V usecmap".as_slice(), None, WritingMode::Vertical)]
    #[case::neither(b"".as_slice(), None, WritingMode::Horizontal)]
    fn writing_mode(
        #[case] content: &[u8],
        #[case] dict_wmode: Option<u8>,
        #[case] expected: WritingMode,
    ) {
        let encoding = EncodingCMap::from_stream(content, dict_wmode);

        assert_eq!(encoding.writing_mode(), expected);
    }

    #[test]
    #[tracing_test::traced_test]
    fn notdef_range_is_dropped() {
        let cmap = CMap::<Cid>::parse(indoc! {b"
            1 beginnotdefrange
            <00> <1F> 0
            endnotdefrange
        "});

        assert!(cmap.is_empty());
        assert!(logs_contain("notdefrange section not supported"));
    }

    #[rstest]
    #[case("Identity-H", WritingMode::Horizontal)]
    #[case("Identity-V", WritingMode::Vertical)]
    fn identities(#[case] name: &str, #[case] mode: WritingMode) {
        let encoding = EncodingCMap::from_name(&name.into());

        assert_eq!(encoding, EncodingCMap::Identity(mode));
        assert_eq!(encoding.codespace(), IDENTITY_CODESPACE);
        assert_eq!(encoding.cid(Code::two_bytes(0x1234)), Some(Cid(0x1234)));
        assert_eq!(encoding.cid(Code::one_byte(0x12)), None);
    }

    #[test]
    #[tracing_test::traced_test]
    fn other_predefined_names_degrade_to_identity() {
        let encoding = EncodingCMap::from_name(&"UniGB-UCS2-H".into());

        assert_eq!(encoding, EncodingCMap::Identity(WritingMode::Horizontal));
        assert!(logs_contain(
            "predefined CMap /UniGB-UCS2-H not shipped: treated as Identity-H"
        ));
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_bf_entry_is_dropped_from_a_cid_cmap() {
        let cmap = CMap::<Cid>::parse(indoc! {b"
            1 beginbfchar
            <41> <0041>
            endbfchar
            1 begincidchar
            <42> 7
            endcidchar
        "});

        assert_eq!(cmap.get(Code::one_byte(0x41)), None);
        assert_eq!(cmap.get(Code::one_byte(0x42)), Some(Cid(7)));
        assert!(logs_contain(
            "CID destination <0041> is not an integer: dropped"
        ));
    }
}
