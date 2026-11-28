//! The corpus and the staged extraction pipeline shared by the bench targets.
//!
//! The pipeline is cut into the steps the benches time separately: [`open`], [`fonts`],
//! [`content`] and [`text`], plus [`end_to_end`]. No step panics on a failing page: a
//! page whose fonts or content fail to build is skipped by [`text`], the way a reader
//! extracting what it can would skip it.

use std::hint::black_box;
use std::path::{Path, PathBuf};

use livre::{
    content::{operators::Operator, Fonts, Run, TextObjects},
    extraction::{multicomment0, Extract},
    structure::Page,
    InMemoryDocument,
};
use winnow::{
    combinator::{iterator, preceded},
    BStr, ModalResult,
};

/// The variable naming the directory of fetched documents.
const CORPUS_VARIABLE: &str = "LIVRE_BENCH_CORPUS";

/// The documents read from `tests/resources` rather than from the fetched corpus.
const REPOSITORY_FILES: [&str; 2] = ["composite.pdf", "encodings.pdf"];

/// A document of the corpus: its file name and its raw bytes.
pub struct Document {
    pub name: &'static str,
    pub bytes: Vec<u8>,
}

/// What extracting a document yields, as [`check`] counts it.
pub struct Expected {
    pub name: &'static str,
    pub pages: usize,
    /// Pages where building the content or the fonts fails, where `next_object` returns an
    /// error, or where the operators stop before the end of the content.
    pub failing_pages: usize,
    /// The `char`s of every [`Run::text`] of every page, failing pages included up to
    /// their failure.
    pub characters: usize,
}

/// Every document of the corpus, in the order the benches run them.
pub const EXPECTED: [Expected; 12] = [
    Expected {
        name: "2201.00037.pdf",
        pages: 33,
        failing_pages: 0,
        characters: 82_266,
    },
    Expected {
        name: "1602.06541.pdf",
        pages: 16,
        failing_pages: 0,
        characters: 59_809,
    },
    Expected {
        name: "2201.00022.pdf",
        pages: 14,
        failing_pages: 0,
        characters: 49_032,
    },
    Expected {
        name: "2201.00201.pdf",
        pages: 9,
        failing_pages: 0,
        characters: 42_975,
    },
    Expected {
        name: "2201.00021.pdf",
        pages: 10,
        failing_pages: 0,
        characters: 39_152,
    },
    Expected {
        name: "2201.00178.pdf",
        pages: 16,
        failing_pages: 0,
        characters: 38_614,
    },
    Expected {
        name: "2201.00200.pdf",
        pages: 7,
        failing_pages: 0,
        characters: 31_352,
    },
    Expected {
        name: "tracemonkey.pdf",
        pages: 14,
        failing_pages: 0,
        characters: 67_230,
    },
    Expected {
        name: "tracemonkey-objstm.pdf",
        pages: 14,
        failing_pages: 0,
        characters: 67_230,
    },
    Expected {
        name: "GeoTopo-book.pdf",
        pages: 117,
        failing_pages: 0,
        characters: 110_033,
    },
    Expected {
        name: "composite.pdf",
        pages: 2,
        failing_pages: 0,
        characters: 16,
    },
    Expected {
        name: "encodings.pdf",
        pages: 1,
        failing_pages: 0,
        characters: 19,
    },
];

/// Reads every document of [`EXPECTED`]: the repository's from `tests/resources`, the
/// others from the directory `LIVRE_BENCH_CORPUS` names.
///
/// # Panics
///
/// If `LIVRE_BENCH_CORPUS` is unset or a file cannot be read.
pub fn corpus() -> Vec<Document> {
    let Some(directory) = std::env::var_os(CORPUS_VARIABLE) else {
        panic!(
            "{CORPUS_VARIABLE} is unset: it names the directory of the benchmark's documents. \
             The dev shell (`nix develop`) sets it."
        );
    };
    let directory = PathBuf::from(directory);
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/resources");

    EXPECTED
        .iter()
        .map(|&Expected { name, .. }| {
            let path = if REPOSITORY_FILES.contains(&name) {
                resources.join(name)
            } else {
                directory.join(name)
            };
            let bytes = std::fs::read(&path).unwrap_or_else(|error| {
                panic!(
                    "cannot read {}: {error}. {CORPUS_VARIABLE} should name the directory the \
                     dev shell (`nix develop`) sets.",
                    path.display()
                )
            });
            Document { name, bytes }
        })
        .collect()
}

/// Open: parse the document and list its pages.
pub fn open(bytes: &[u8]) -> ModalResult<(InMemoryDocument, Vec<Page>)> {
    let doc = InMemoryDocument::extract(&mut BStr::new(bytes))?;
    let pages = doc.pages()?;
    Ok((doc, pages))
}

/// Fonts: resolve every page's fonts.
pub fn fonts(doc: &InMemoryDocument, pages: &[Page]) -> Vec<ModalResult<Fonts>> {
    pages.iter().map(|page| page.fonts(doc)).collect()
}

/// Content: concatenate every page's content streams.
pub fn content(doc: &InMemoryDocument, pages: &[Page]) -> Vec<ModalResult<Vec<u8>>> {
    pages.iter().map(|page| page.build_content(doc)).collect()
}

/// Text: read every page's operators and turn them into runs, for each page whose content
/// and fonts were built. Whether each page read to the end of its content, in page order.
pub fn text(contents: &[ModalResult<Vec<u8>>], fonts: &[ModalResult<Fonts>]) -> Vec<bool> {
    text_with(contents, fonts, |run| {
        black_box(run);
    })
}

/// Every step, from the bytes. `None` if the document does not open.
pub fn end_to_end(bytes: &[u8]) -> Option<Vec<bool>> {
    let (doc, pages) = open(bytes).ok()?;
    let fonts = fonts(&doc, &pages);
    let contents = content(&doc, &pages);
    Some(text(&contents, &fonts))
}

/// [`text`], handing each run to `on_run`. A page is complete when its content and fonts
/// were built, no `next_object` failed, and the operators reached the end of the content.
fn text_with(
    contents: &[ModalResult<Vec<u8>>],
    fonts: &[ModalResult<Fonts>],
    mut on_run: impl FnMut(Run),
) -> Vec<bool> {
    contents
        .iter()
        .zip(fonts)
        .map(|(content, fonts)| {
            let (Ok(content), Ok(fonts)) = (content, fonts) else {
                return false;
            };
            let mut input = BStr::new(content);
            let mut operators = iterator(&mut input, preceded(multicomment0, Operator::extract));
            let mut objects = TextObjects::new(&mut operators, fonts);
            loop {
                match objects.next_object() {
                    Ok(Some(stream)) => stream.for_each(&mut on_run),
                    Ok(None) => break,
                    Err(_) => return false,
                }
            }
            // The operators stop at the first token they cannot parse: anything left but
            // comments and whitespace is content the page did not read.
            multicomment0(&mut input).is_ok() && input.is_empty()
        })
        .collect()
}

/// Run the whole pipeline on `document` once and compare its pages, failing pages and
/// characters with its row of [`EXPECTED`].
///
/// # Panics
///
/// If the document does not open, has no row, or any count differs from its row.
pub fn check(document: &Document) {
    let name = document.name;
    let expected = EXPECTED
        .iter()
        .find(|expected| expected.name == name)
        .unwrap_or_else(|| panic!("{name}: no row in the expected table"));

    let (doc, pages) =
        open(&document.bytes).unwrap_or_else(|error| panic!("{name}: does not open: {error}"));
    let fonts = fonts(&doc, &pages);
    let contents = content(&doc, &pages);
    let mut characters = 0;
    let complete = text_with(&contents, &fonts, |run| {
        characters += run.text.chars().count()
    });
    let failing_pages = complete.iter().filter(|&&complete| !complete).count();

    let actual = [
        ("pages", expected.pages, pages.len()),
        ("failing pages", expected.failing_pages, failing_pages),
        ("characters", expected.characters, characters),
    ];
    for (what, expected, actual) in actual {
        assert_eq!(
            expected, actual,
            "{name}: expected {expected} {what}, extracted {actual}"
        );
    }
}
