use std::{fs::File, io::Read};

use livre::{
    content::{operators::Operator, parse_text_object, GraphicsState},
    extraction::Extract,
    InMemoryDocument,
};
use rstest::rstest;
use winnow::{
    ascii::multispace0,
    combinator::{eof, iterator, preceded},
    error::ContextError,
    BStr, Parser,
};

fn read_document(path: &str) -> InMemoryDocument {
    let mut file = File::open(path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    InMemoryDocument::extract(&mut buffer.as_slice().as_ref()).unwrap()
}

/// Every run of every page: its position in default user space and its text.
fn runs(path: &str) -> Vec<((f32, f32), String)> {
    let doc = read_document(path);
    let mut runs = Vec::new();

    for page in doc.pages().unwrap().iter() {
        let content = page.build_content(&doc).unwrap();
        let mut stream = BStr::new(&content);

        let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));
        let mut graphics_state = GraphicsState::default();

        while let Some(text_object) = parse_text_object(&mut it, &mut graphics_state).unwrap() {
            runs.extend(text_object);
        }

        it.finish().unwrap();

        (multispace0::<&BStr, ContextError>, eof)
            .parse_next(&mut stream)
            .unwrap();
    }

    runs
}

#[rstest]
#[case("tests/resources/letter.pdf")]
#[case("tests/resources/text.pdf")]
#[case("tests/resources/text-xref-stream.pdf")]
fn test_content(#[case] path: &str) {
    for (position, text) in runs(path) {
        println!("- {:?} {:?}", position, text);
    }
}

/// `text.pdf` (Skia output) flips the page with `1 0 0 -1 0 792 cm` and positions every line
/// with its own `q .75 0 0 .75 72 Y cm … Q`, `Y` running from `72` to `362.92529`. Inside, every
/// text object sets `1 0 0 -1 X .47981739 Tm` then `0 -13.2773438 Td`, so a run sits at
/// `(0.75·X + 72, 792 − (0.75·13.75716 + Y))`. Without the CTM every run would sit at
/// `y = 13.76`.
#[rstest]
#[case("tests/resources/text.pdf")]
#[case("tests/resources/text-xref-stream.pdf")]
fn runs_are_positioned_through_the_ctm(#[case] path: &str) {
    let runs = runs(path);

    fn assert_close((x, y): (f32, f32), (ex, ey): (f32, f32)) {
        assert!(
            (x - ex).abs() < 1e-2 && (y - ey).abs() < 1e-2,
            "expected ({ex}, {ey}), got ({x}, {y})"
        );
    }

    // First line, `X = 0`, `Y = 72`.
    assert_close(runs[0].0, (72.0, 709.6821));
    // Last line, `Y = 362.92529`: every run of a line shares its baseline.
    let ((_, last_y), _) = runs[runs.len() - 1];
    assert!((last_y - 418.7568).abs() < 1e-2, "last y = {last_y}");

    let ys: Vec<f32> = runs.iter().map(|((_, y), _)| *y).collect();
    assert!(
        ys.iter().all(|y| (418.0..710.0).contains(y)),
        "a run left the text column"
    );
    assert!(
        ys.iter().all(|y| *y <= ys[0]),
        "no run sits above the first line"
    );
}
