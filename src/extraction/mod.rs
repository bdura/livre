//! Definition of the extraction logic, organised around three main traits:
//! [`Extract`], [`Build`], and [`Builder`] - and their implementation for primitive types.
//!
//! Livre aims to provide a sufficient set of tools to be able to parse *any* PDF object in a
//! type-safe way. To that end, we define an [`Extract`] trait, which defines a way for a type
//! to extract itself from a stream of bytes. Every "primitive" type (e.g. `f64`, `bool`,
//! but also some PDF-specific types) implement that trait.
//!
//! Types that describe more high-level aspects such as the PDF structure, the page layout
//! or the representation of fonts are not managed by this module.
//!
//! ## Complex and indirect objects
//!
//! More complex objects can be extracted by composing these primitives. However, since
//! the PDF specification allows references to *indirect objects* (cf section 7.3.10 from
//! the specification), Livre needs to defines a way for the parsing logic to follow these
//! references in order to allow the reconstruction some of any PDF object.
//!
//! To cope with indirect references, Livre introduces the [`Build`] and [`Builder`] traits:
//!
//! - an object that implements [`Builder`] can follow and instantiate references. It usually
//!   involves a mapping between [`ReferenceId`]s and their locations within the input file.
//! - the [`Build`] trait is a generalisation of the [`Extract`] trait which can leverage
//!   a [`Builder`] to follow references.
//!
//! All primitive types are [`Extract`], which also makes them trivially [`Build`]. Two special
//! types are also [`Extract`]:
//!
//! - [`Reference`] is a generic type that holds a reference to an indirect object.
//! - [`OptRef`], also generic, contains either the object itself, or a reference to it.
//!
//! These two type can be "instantiated" with the help of a [`Builder`], making objects that rely
//! on them [`Build`].
//!
//! ## PDF dictionaries
//!
//! The PDF specification also defines dictionaries, that can hold heterogeneous data.
//! Livre can represent those as generic [`HashMap`](std::collections::HashMap)s,
//! or extract them as structured types using the [`FromRawDict`] trait. [`FromRawDict`]
//! is derivable thanks to the [`livre_derive`] crate.
//!
//! ## Derivability
//!
//! Through the [`livre_derive`] helper crate, Livre provides (and uses) a derive macro
//! for [`FromRawDict`].

pub use livre_derive::FromRawDict;
use winnow::{
    ascii::multispace1,
    combinator::terminated,
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

mod primitives;
mod special;
mod utilities;

pub use special::{
    multicomment0, multicomment1, Comment, HexadecimalString, Indirect, LiteralString, MaybeArray,
    Name, Nil, Object, OptRef, RawDict, Reference, ReferenceId, Stream,
};

/// The [`Extract`] trait marks a type as extractable from a stream of bytes.
///
/// To cope with the presence of *indirect objects*, complex objects may implement the [`Build`]
/// trait instead, if their components may include references.
pub trait Extract<'de>: Sized {
    fn extract(input: &mut &'de BStr) -> PResult<Self>;

    /// Consume the input, without trying to parse.
    ///
    /// Especially useful for struct/map parsing, since we just need to extract
    /// the *bytes* associated with the type (see [`RawDict`]/[`FromRawDict`]).
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

/// The `FromRawDict` trait allows for the construction of complex types using a pre-parsed
/// dictionary.
///
/// This type can be derived using the [`livre_derive`] helper crate.
pub trait FromRawDict<'de>: Sized {
    /// Build a type from a raw dictionary. Note that the supplied dict is not consumed.
    /// Rather, the method takes hold of a mutable reference to extract only the fields
    /// that are needed, removing them from the dictionary.
    ///
    /// This means that we can break a single [`RawDict`] into multiple structured objects,
    /// which is particularly useful for compound PDF objects such as [`Stream`]s.
    fn from_raw_dict(dict: &mut RawDict<'de>) -> PResult<Self>;
}

/// Any type that is [`FromRawDict`] is trivially [`Extract`]: you first extract the [`RawDict`],
/// and apply [`FromRawDict`].
impl<'de, T> Extract<'de> for T
where
    T: FromRawDict<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        let mut dict = RawDict::extract(input)?;
        let result = Self::from_raw_dict(&mut dict)?;
        Ok(result)
    }
}

/// Trait that can follow indirect references.
///
/// A `Builder` holds every information to follow indirect references. It usually involves
/// a mapping between [`ReferenceId`]s and their locations within the input file.
pub trait Builder<'de>: Sized {
    /// Follow a reference and provide an (optional) pointer to the start of the indirect object.
    ///
    /// This is the entrypoint for the builder. This method provides the stream slice
    /// that describes the referenced entity. It returns an optional in case the reference
    /// is unknown to the builder.
    fn follow_reference(&self, reference_id: ReferenceId) -> Option<&'de BStr>;

    /// Build an object from the input. Direct analogue to the [`extract`] function.
    fn build<T>(&self, input: &mut &'de BStr) -> PResult<T>
    where
        T: Build<'de>,
    {
        T::build(input, self)
    }

    /// Follow a reference and extract it directly.
    ///
    /// This method checks that the reference is known to the builder, and returns a parsing error
    /// if that is not the case. It includes the mechanism to extract a *indirect object*.
    ///
    /// This method is usually the one that is used in practice.
    fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> PResult<T>
    where
        T: Build<'de>,
    {
        let mut input = self
            .follow_reference(id)
            .ok_or(ErrMode::Cut(ContextError::new()))?;

        // NOTE: we do not check the presence of `endobj` here... It's double-edged:
        // - it is (usually marginally) faster
        // - it removes a sanity check
        let reference_id =
            terminated(ReferenceId::extract, (b" obj", multispace1)).parse_next(&mut input)?;

        debug_assert_eq!(reference_id, id);

        T::build(&mut input, self)
    }
}

pub trait ParserBuilder: Sized {
    fn as_parser(&self) -> LivreBuilder<'_, Self> {
        LivreBuilder(self)
    }
}

impl<'de, B> ParserBuilder for B where B: Builder<'de> {}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LivreBuilder<'b, B>(pub &'b B);

impl<'de, T, B> Parser<&'de BStr, T, ContextError> for LivreBuilder<'_, B>
where
    B: Builder<'de>,
    T: Build<'de>,
{
    fn parse_next(&mut self, input: &mut &'de BStr) -> PResult<T, ContextError> {
        T::build(input, self.0)
    }
}

/// The dummy builder, which does not follow references at all.
impl<'de> Builder<'de> for () {
    fn follow_reference(&self, _reference_id: ReferenceId) -> Option<&'de BStr> {
        None
    }
}

/// Generalisation on the [`Extract`] trait, which allows the extraction logic to follow references.
pub trait Build<'de>: Sized {
    /// Build an object that rely on a reference, which would be instantiated with the help of the
    /// supplied `builder`.
    ///
    /// The [`Build`] trait, like the [`Extract`] trait, is a linear parser above all, hence we
    /// supply an `input`. References found during parsing, if any, are first parsed as such, and
    /// then instantiated by the `builder`.
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}

/// [`Extract`] types are trivially [`Build`], since there is no reference to follow.
impl<'de, T> Build<'de> for T
where
    T: Extract<'de>,
{
    fn build<B>(input: &mut &'de BStr, _: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        extract(input)
    }
}
