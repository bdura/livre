//! The built-in encoding of an embedded Type 1 font program (`/FontFile`), read from the
//! `/Encoding` entry of its clear-text part (Adobe Type 1 Font Format, 2.3).
//!
//! The clear text ends at `eexec`. Its `/Encoding` is either `StandardEncoding` or an array
//! filled by `dup <code> /<glyph> put` statements, up to the `def` that stores it.

use super::{Base, GlyphName, STANDARD};
use crate::extraction::Name;

/// The built-in encoding of the Type 1 program `program`, `None` if its clear text has no
/// `/Encoding`.
///
/// Only the bytes before `eexec` are read. Codes outside `0..=255` and statements other
/// than `dup <code> /<glyph> put` are skipped.
pub(crate) fn builtin_encoding(program: &[u8]) -> Option<Base> {
    let clear_text = program
        .windows(5)
        .position(|window| window == b"eexec")
        .map_or(program, |end| &program[..end]);

    let mut tokens = Tokens(clear_text).skip_while(|token| *token != b"/Encoding");
    tokens.next()?;

    let mut tokens = tokens.peekable();
    if tokens.peek() == Some(&&b"StandardEncoding"[..]) {
        return Some(Base::Table(&STANDARD));
    }

    let tokens: Vec<&[u8]> = tokens.take_while(|token| *token != b"def").collect();
    let names = tokens
        .windows(4)
        .filter_map(|window| match window {
            [b"dup", code, name, b"put"] => {
                let code = u8::try_from(integer(code)?).ok()?;
                let name = name.strip_prefix(b"/")?;
                Some((code, GlyphName::from(Name(name.to_vec()))))
            }
            _ => None,
        })
        .collect();
    Some(Base::Names(names))
}

/// A PostScript integer: decimal, or `radix#digits`.
fn integer(token: &[u8]) -> Option<i64> {
    let token = std::str::from_utf8(token).ok()?;
    match token.split_once('#') {
        Some((radix, digits)) => i64::from_str_radix(digits, radix.parse().ok()?).ok(),
        None => token.parse().ok(),
    }
}

/// The tokens of PostScript clear text, enough to spot names, integers and operators:
/// whitespace and `{}[]` separate tokens, `/` starts one, and comments are skipped.
struct Tokens<'a>(&'a [u8]);

impl<'a> Iterator for Tokens<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (&first, rest) = self.0.split_first()?;
            match first {
                b'%' => {
                    let end = rest
                        .iter()
                        .position(|&b| b == b'\n' || b == b'\r')
                        .unwrap_or(rest.len());
                    self.0 = &rest[end..];
                }
                b'{' | b'}' | b'[' | b']' => {
                    let (token, rest) = self.0.split_at(1);
                    self.0 = rest;
                    return Some(token);
                }
                _ if first.is_ascii_whitespace() || first == 0 => self.0 = rest,
                _ => {
                    let end = rest
                        .iter()
                        .position(|&b| b.is_ascii_whitespace() || b"{}[]/%(".contains(&b) || b == 0)
                        .map_or(self.0.len(), |end| end + 1);
                    let (token, rest) = self.0.split_at(end);
                    self.0 = rest;
                    return Some(token);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    fn names(encoding: &Base) -> Vec<(u8, &str)> {
        let Base::Names(differences) = encoding else {
            panic!("not a custom encoding: {encoding:?}");
        };
        differences
            .iter()
            .map(|(code, name)| (code, name.as_str()))
            .collect()
    }

    /// The clear text of a pdfTeX Computer Modern subset, shortened.
    const CMR10: &[u8] = indoc! {b"
        %!PS-AdobeFont-1.0: CMR10 003.002
        %%Title: CMR10
        12 dict begin
        /FontInfo 7 dict dup begin
        /Notice (Copyright (c) 1997, 2009 American Mathematical Society) readonly def
        end readonly def
        /FontName /ABCDEF+CMR10 def
        /PaintType 0 def
        /FontType 1 def
        /FontMatrix [0.001 0 0 0.001 0 0 ]readonly def
        /Encoding 256 array
        0 1 255 {1 index exch /.notdef put} for
        dup 11 /ff put
        dup 12 /fi put
        dup 65 /A put
        dup 8#141 /a put
        dup 300 /toolarge put
        readonly def
        /FontBBox{-40 -250 1009 750}readonly def
        currentdict end
        currentfile eexec
        dup 66 /B put
    "};

    #[test]
    fn custom_array() {
        let encoding = builtin_encoding(CMR10).unwrap();

        assert_eq!(
            names(&encoding),
            [(11, "ff"), (12, "fi"), (65, "A"), (97, "a")]
        );
    }

    #[test]
    fn standard_encoding() {
        let program = b"/FontName /Foo def\n/Encoding StandardEncoding def\ncurrentfile eexec";

        assert_eq!(builtin_encoding(program), Some(Base::Table(&STANDARD)));
    }

    #[test]
    fn no_encoding_before_eexec() {
        let program = b"/FontName /Foo def\ncurrentfile eexec\n/Encoding StandardEncoding def";

        assert_eq!(builtin_encoding(program), None);
    }

    #[test]
    fn names_glued_to_codes() {
        let program = b"/Encoding 256 array\ndup 32/space put\ndup 33/exclam put readonly def";

        let encoding = builtin_encoding(program).unwrap();
        assert_eq!(names(&encoding), [(32, "space"), (33, "exclam")]);
    }
}
