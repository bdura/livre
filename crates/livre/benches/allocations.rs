//! Count the allocations and allocated bytes of each step of text extraction on every
//! document of the corpus.
//!
//! Prints one tab-separated row per document and step: document, step, allocations,
//! bytes. The counts are deterministic, so each step runs once and two runs of the same
//! code print the same table. Command-line arguments are ignored, so the target runs
//! under `cargo bench` and `cargo test --all-targets` alike.

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The system allocator, counting the allocations it serves and the bytes it hands out.
///
/// `alloc` and `alloc_zeroed` count one allocation each. `realloc` counts one allocation
/// of the new size: growing a buffer is work too. Deallocations are not counted.
struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

impl Counting {
    fn record(size: usize) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(size, Ordering::Relaxed);
    }
}

// SAFETY: every call is forwarded unchanged to `System`; counting has no effect on it.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::record(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        Self::record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        Self::record(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The allocations and bytes `step` allocates, its result dropped after counting.
fn count<T>(step: impl FnOnce() -> T) -> (usize, usize) {
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let bytes = BYTES.load(Ordering::Relaxed);
    let result = black_box(step());
    let counted = (
        ALLOCATIONS.load(Ordering::Relaxed) - allocations,
        BYTES.load(Ordering::Relaxed) - bytes,
    );
    drop(result);
    counted
}

fn main() {
    let corpus = common::corpus();
    for document in &corpus {
        common::check(document);
    }

    println!("document\tstep\tallocations\tbytes");
    for document in &corpus {
        let bytes = document.bytes.as_slice();
        let (doc, pages) = common::open(bytes).expect("checked above");
        let fonts = common::fonts(&doc, &pages);
        let contents = common::content(&doc, &pages);

        let rows = [
            ("open", count(|| common::open(black_box(bytes)))),
            ("fonts", count(|| common::fonts(&doc, &pages))),
            ("content", count(|| common::content(&doc, &pages))),
            ("text", count(|| common::text(&contents, &fonts))),
            ("end_to_end", count(|| common::end_to_end(black_box(bytes)))),
        ];
        for (step, (allocations, bytes)) in rows {
            println!("{}\t{step}\t{allocations}\t{bytes}", document.name);
        }
    }
}
