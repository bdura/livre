//! The wasm glue behind livre's browser demo: one call turns PDF bytes into the
//! [`Analysis`] the page draws.
//!
//! [`analyze`] is plain Rust and runs natively; only [`analyze_pdf`] needs a JS runtime.

mod analyze;
pub mod dto;

pub use analyze::analyze;
pub use dto::{Analysis, Page, Rect, Run, Stage, StageError, TextObject};

#[cfg(target_arch = "wasm32")]
use {tsify::Ts, wasm_bindgen::prelude::*};

/// [`analyze`], serialised to a plain JS object declared as the `Analysis` interface.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn analyze_pdf(bytes: &[u8]) -> Ts<Analysis> {
    // `Analysis` holds only strings, numbers, options and vectors: serialisation cannot fail.
    Ts::from_rust(&analyze(bytes)).unwrap()
}
