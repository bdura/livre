//! Implementation of the [`Extract`] trait for primitive types: `bool`,
//! (un-)signed integers, floating point numbers, unit type, vectors, arrays,
//! optionals & tuple.
//!
//! Note that we do not implement the trait for `String`, because of the way PDFs
//! represent text.

use super::Extract;

mod array;
mod boolean;
mod numbers;
mod option;
mod tuple;
mod unit;
mod vec;

pub use numbers::recognize_number;
