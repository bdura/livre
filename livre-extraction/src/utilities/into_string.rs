use std::borrow::Cow;

use crate::Extract;

/// Provides a way to convert the entirety of the input to string,
/// by trying to cast it as a UTF-8 encoded string.
/// Never fails.
///
/// ```
/// # use livre_extraction::{DbgStr, Extract};
/// // This is a trivial example, but would also work with ill-defined inputs.
/// let input = b"This is a test";
/// let (_, DbgStr(dbg)) = DbgStr::extract(input).unwrap();
/// assert_eq!(dbg, "This is a test")
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DbgStr<'input>(pub Cow<'input, str>);

impl<'input> Extract<'input> for DbgStr<'input> {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let obj = String::from_utf8_lossy(input);
        Ok((b"", Self(obj)))
    }
}
