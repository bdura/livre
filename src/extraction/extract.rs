use winnow::{BStr, PResult, Parser};

/// The [`Extract`] trait marks a type as extractable from a stream of bytes.
///
/// To cope with the presence of *indirect objects*, complex objects may implement the
/// [`Build`](crate::follow_refs::Build) trait instead, if their components may include references.
pub trait Extract<'de>: Sized {
    fn extract(input: &mut &'de BStr) -> PResult<Self>;

    /// Consume the input, without trying to parse.
    ///
    /// Especially useful for struct/map parsing, since we just need to extract
    /// the *bytes* associated with the type (see [`RawDict`](super::RawDict) and
    /// [`FromRawDict`](super::FromRawDict)).
    ///
    /// Some types (if not all) may benefit from using a dedicated logic.
    fn recognize(input: &mut &'de BStr) -> PResult<&'de [u8]> {
        Self::extract.take().parse_next(input)
    }
}

/// Direct extraction of an [`Extract`] type.
///
/// Most of the time the type can be inferred from context, making this function very handy.
pub fn extract<'de, T>(input: &mut &'de BStr) -> PResult<T>
where
    T: Extract<'de>,
{
    T::extract(input)
}
