//! Derive macros for [`livre`](https://docs.rs/livre), a type-safe PDF parser.
//!
//! Three derives are provided, each re-exported by `livre` next to the trait it
//! implements:
//!
//! - [`Extract`](derive@Extract) parses a type directly from bytes;
//! - [`FromRawDict`](derive@FromRawDict) fills a struct from a PDF dictionary;
//! - [`BuildFromRawDict`](derive@BuildFromRawDict) does the same, following
//!   indirect references through a `Builder`.
//!
//! The generated code names everything through absolute `::livre::` and
//! `::winnow::` paths, so it does not depend on the caller's imports. The
//! deriving crate must therefore have `livre` and `winnow` as dependencies.
//!
//! Every misuse (an unsupported shape, an invalid attribute combination) is
//! reported as a compile error spanned on the offending item.
//!
//! # Supported shapes
//!
//! ## `Extract`
//!
//! - **Tuple structs**: the fields are extracted in order, as a tuple.
//! - **Unit structs**: nothing is consumed.
//! - **Enums** whose variants each wrap a single field: each variant is tried
//!   in declaration order, and the first whose payload extracts wins.
//!
//! Named structs are refused: derive `FromRawDict` instead, which provides
//! `Extract` through a blanket impl. Unions are refused, and so is any
//! `#[livre(...)]` attribute on an `Extract` field.
//!
//! ```rust,ignore
//! use livre::extraction::Extract;
//!
//! #[derive(Extract)]
//! struct Pair(i32, i32); // parses `1 2`
//!
//! #[derive(Extract)]
//! enum Number {
//!     Integer(i32),
//!     Real(f32),
//! }
//! ```
//!
//! ## `FromRawDict` and `BuildFromRawDict`
//!
//! Both accept structs with named fields only; enums, tuple structs, unit
//! structs and unions are refused. Each field reads one dictionary key:
//!
//! - an `Option<T>` field is `None` when its key is absent (and when its value
//!   is `null`);
//! - any other field requires its key, unless it carries `default`.
//!
//! Keys are popped from the dictionary, so a flattened field only sees the
//! keys its siblings left. `FromRawDict` also implements `Extract`, and
//! `BuildFromRawDict` also implements `Build`, through blanket impls in
//! `livre`.
//!
//! ```rust,ignore
//! use livre::extraction::FromRawDict;
//!
//! #[derive(FromRawDict)]
//! struct Info {
//!     title: Option<String>,   // key `/Title`, optional
//!     last_modified: String,   // key `/LastModified`, required
//! }
//! ```
//!
//! # Keys
//!
//! A field's key is its name converted from `snake_case` to `PascalCase`: the
//! name is split on `_`, and the first character of each non-empty segment
//! is uppercased (`last_modified` reads `/LastModified`). Use `rename` when
//! the PDF key does not follow that convention.
//!
//! A missing required key, or a value that fails to parse, yields a backtrack
//! error labelled with the key (`/Length`), so `alt` over alternatives still
//! works.
//!
//! # Field attributes
//!
//! All attributes go in `#[livre(...)]`, may be combined in one list or split
//! across several, and may each appear once per field. They apply to the
//! dictionary derives only.
//!
//! ## `rename = "Key"`
//!
//! Reads the given key, without its slash, instead of the converted name. The
//! key must not be empty.
//!
//! ```rust,ignore
//! #[derive(FromRawDict)]
//! struct Trailer {
//!     #[livre(rename = "ID")]
//!     id: Option<Vec<String>>,
//! }
//! ```
//!
//! ## `from = Type`
//!
//! Extracts `Type` and converts it with `Into` into the field's type. On an
//! `Option<T>` field, `Option<Type>` is extracted and the inner value
//! converted.
//!
//! ```rust,ignore
//! #[derive(FromRawDict)]
//! struct Node {
//!     // `/Kids` may be a single reference or an array of them.
//!     #[livre(from = MaybeArray<Reference<Page>>)]
//!     kids: Vec<Reference<Page>>,
//! }
//! ```
//!
//! ## `default`
//!
//! Supplies the field's value when its key is absent. It has four forms:
//!
//! | Form | Absent key gives |
//! |---|---|
//! | `default` | `Default::default()` |
//! | `default = 1.0` (a literal) | the literal |
//! | `default = \|\| make()` (a closure) | the closure's result |
//! | `default = make()` (any other expression) | the expression, evaluated lazily |
//!
//! ```rust,ignore
//! #[derive(FromRawDict)]
//! struct Page {
//!     #[livre(default)]
//!     annots: Vec<Reference<Annot>>,
//!     #[livre(default = 1.0)]
//!     user_unit: f32,
//!     #[livre(default = Rotation::Zero)]
//!     rotate: Rotation,
//! }
//! ```
//!
//! `default` is refused on an `Option<T>` field, which already defaults to
//! `None`.
//!
//! Combined with `from`, the value is converted first, then defaulted: the
//! default is typed as the field, not as the `from` type.
//!
//! ```rust,ignore
//! #[derive(FromRawDict)]
//! struct StreamConfig {
//!     // An absent `/Filter` gives an empty `Vec`.
//!     #[livre(from = MaybeArray<Filter>, default)]
//!     filter: Vec<Filter>,
//! }
//! ```
//!
//! ## `flatten`
//!
//! Fills the field from the same dictionary, through its own `FromRawDict`
//! (or `BuildFromRawDict`) implementation. The field has no key. `flatten`
//! cannot be combined with any other attribute. When the field's type is a
//! bare type parameter, the parameter is bound by the derived trait.
//!
//! ```rust,ignore
//! #[derive(FromRawDict)]
//! struct Stream<T> {
//!     #[livre(flatten)]
//!     config: StreamConfig,
//!     #[livre(flatten)]
//!     structured: T,
//! }
//! ```

use syn::{parse_macro_input, DeriveInput};

mod attrs;
mod dict;
mod extract;
mod generics;
mod plan;

/// Turns the result of a derive into tokens, errors becoming compile errors.
fn expand(result: syn::Result<proc_macro2::TokenStream>) -> proc_macro::TokenStream {
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Derives `livre::follow_refs::BuildFromRawDict` for a struct with named
/// fields: each field is read from its dictionary key, following indirect
/// references through the builder, and `Build` follows through a blanket impl.
/// Fields accept `rename`, `from`, `default` and `flatten`, where a flattened
/// field must itself be `BuildFromRawDict`. See the [crate docs](crate) for
/// keys and attributes.
#[proc_macro_derive(BuildFromRawDict, attributes(livre))]
pub fn derive_build_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(dict::derive(input, &dict::BUILD_FROM_RAW_DICT))
}

/// Derives `livre::extraction::FromRawDict` for a struct with named fields:
/// each field is extracted from its dictionary key, and `Extract` follows
/// through a blanket impl. Fields accept `rename`, `from`, `default` and
/// `flatten`, where a flattened field must itself be `FromRawDict`. See the
/// [crate docs](crate) for keys and attributes.
#[proc_macro_derive(FromRawDict, attributes(livre))]
pub fn derive_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(dict::derive(input, &dict::FROM_RAW_DICT))
}

/// Derives `livre::extraction::Extract` for a tuple struct (fields extracted
/// in order), a unit struct (nothing consumed) or an enum of single-field
/// variants (tried in declaration order). Named structs should derive
/// [`FromRawDict`](derive@FromRawDict) instead, and fields take no
/// `#[livre(...)]` attribute. See the [crate docs](crate).
#[proc_macro_derive(Extract, attributes(livre))]
pub fn derive_extract(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(extract::derive(parse_macro_input!(input as DeriveInput)))
}
