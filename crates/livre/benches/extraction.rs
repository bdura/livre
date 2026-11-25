//! Time each step of text extraction on every document of the corpus.
//!
//! One group per document, with the benches `open`, `fonts`, `content`, `text` and
//! `end_to_end`. Filter by step with `cargo bench -p livre --bench extraction -- text`.
//! Each document is checked against its expected outcome before anything is timed.

mod common;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

fn extraction(c: &mut Criterion) {
    let corpus = common::corpus();
    for document in &corpus {
        common::check(document);
    }

    for document in &corpus {
        let bytes = document.bytes.as_slice();
        let (doc, pages) = common::open(bytes).expect("checked above");
        let fonts = common::fonts(&doc, &pages);
        let contents = common::content(&doc, &pages);

        let mut group = c.benchmark_group(document.name);
        group.bench_function("open", |b| b.iter(|| common::open(black_box(bytes))));
        group.bench_function("fonts", |b| {
            b.iter(|| common::fonts(black_box(&doc), black_box(&pages)))
        });
        group.bench_function("content", |b| {
            b.iter(|| common::content(black_box(&doc), black_box(&pages)))
        });
        group.bench_function("text", |b| {
            b.iter(|| common::text(black_box(&contents), black_box(&fonts)))
        });
        group.bench_function("end_to_end", |b| {
            b.iter(|| common::end_to_end(black_box(bytes)))
        });
        group.finish();
    }
}

criterion_group!(benches, extraction);
criterion_main!(benches);
