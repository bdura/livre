use std::{fs::File, io::Read};

use livre::{
    content::{operators::Operator, DefaultUser, Point, Run, TextObjects},
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

/// Every run of every page, in order.
fn runs(path: &str) -> Vec<Run> {
    let doc = read_document(path);
    let mut runs = Vec::new();

    for page in doc.pages().unwrap().iter() {
        let content = page.build_content(&doc).unwrap();
        let mut stream = BStr::new(&content);

        let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));
        let fonts = page.fonts(&doc).unwrap();
        let mut objects = TextObjects::new(&mut it, &fonts);

        while let Some(text_object) = objects.next_object().unwrap() {
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
    for Run { position, text, .. } in runs(path) {
        println!("- {:?} {:?}", position, text.decode());
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

    fn assert_close(Point { x, y, .. }: Point<DefaultUser>, (ex, ey): (f32, f32)) {
        assert!(
            (x - ex).abs() < 1e-2 && (y - ey).abs() < 1e-2,
            "expected ({ex}, {ey}), got ({x}, {y})"
        );
    }

    // First line, `X = 0`, `Y = 72`.
    assert_close(runs[0].position, (72.0, 709.6821));
    // Last line, `Y = 362.92529`: every run of a line shares its baseline.
    let last_y = runs[runs.len() - 1].position.y;
    assert!((last_y - 418.7568).abs() < 1e-2, "last y = {last_y}");

    let ys: Vec<f32> = runs.iter().map(|run| run.position.y).collect();
    assert!(
        ys.iter().all(|y| (418.0..710.0).contains(y)),
        "a run left the text column"
    );
    assert!(
        ys.iter().all(|y| *y <= ys[0]),
        "no run sits above the first line"
    );
}

/// A run as `font-before-bt.txt` lists it: position, font name and size, decoded text.
#[derive(Debug, PartialEq)]
struct GoldenRun {
    x: f32,
    y: f32,
    font: String,
    size: f32,
    text: String,
}

/// Parses `x y font size text…`, one run per line; the text runs to the end of the line.
fn read_golden(path: &str) -> Vec<GoldenRun> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.splitn(5, ' ');
            let mut next = || fields.next().unwrap();
            GoldenRun {
                x: next().parse().unwrap(),
                y: next().parse().unwrap(),
                font: next().to_string(),
                size: next().parse().unwrap(),
                text: next().to_string(),
            }
        })
        .collect()
}

/// `font-before-bt.pdf` selects its font once in the first text object, then sets a 24-point
/// font outside `BT` inside `q`/`Q`: the text state must persist across text objects and be
/// restored by `Q`. Each entry is `(text object index, run)`.
fn font_runs(path: &str) -> Vec<(usize, GoldenRun)> {
    let doc = read_document(path);
    let mut runs = Vec::new();

    for page in doc.pages().unwrap().iter() {
        let content = page.build_content(&doc).unwrap();
        let mut stream = BStr::new(&content);

        let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));
        let fonts = page.fonts(&doc).unwrap();
        let mut objects = TextObjects::new(&mut it, &fonts);

        let mut index = 0;
        while let Some(mut text_object) = objects.next_object().unwrap() {
            while let Some(Run { position, text, .. }) = text_object.next() {
                let font = text_object
                    .text_object()
                    .parameters
                    .font
                    .as_ref()
                    .expect("a run is only yielded with a font selected");
                runs.push((
                    index,
                    GoldenRun {
                        x: position.x,
                        y: position.y,
                        font: String::from_utf8_lossy(&font.name.0).into_owned(),
                        size: font.size.0,
                        text: text.decode(),
                    },
                ));
            }
            index += 1;
        }

        it.finish().unwrap();
    }

    runs
}

#[test]
fn font_before_bt_matches_its_golden_runs() {
    let expected = read_golden("tests/resources/font-before-bt.txt");
    let actual = font_runs("tests/resources/font-before-bt.pdf");

    assert_eq!(expected.len(), 4);
    assert_eq!(actual.len(), expected.len(), "runs: {actual:?}");

    for (i, ((object, run), golden)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(*object, i, "run {i} in text object {object}");
        assert!(
            (run.x - golden.x).abs() < 1e-2 && (run.y - golden.y).abs() < 1e-2,
            "run {i}: expected ({}, {}), got ({}, {})",
            golden.x,
            golden.y,
            run.x,
            run.y
        );
        assert_eq!(run.font, golden.font, "run {i}");
        assert_eq!(run.size, golden.size, "run {i}");
        assert_eq!(run.text, golden.text, "run {i}");
    }
}

/// A PDF of `objects`, given as `(object number, object body)` and written with generation
/// 0, with object 1 as its catalog.
fn build_pdf(objects: &[(usize, &str)]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();

    for &(number, body) in objects {
        offsets.push((number, pdf.len()));
        pdf.extend(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }

    let startxref = pdf.len();
    pdf.extend(b"xref\n0 1\n0000000000 65535 f \n");
    for (number, offset) in offsets {
        pdf.extend(format!("{number} 1\n{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    pdf
}

/// The reproduction of a page whose `/Resources` has no `/Font`, showing only a path: the
/// page tree parses, and the page has no text object.
#[test]
fn page_without_font_resources_parses() {
    let content = "0 0 1 rg 20 20 160 160 re f";
    let pdf = build_pdf(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /ProcSet [/PDF] >> /Contents 4 0 R >>",
        ),
        (
            4,
            &format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
        ),
    ]);
    let doc = InMemoryDocument::extract(&mut pdf.as_slice().as_ref()).unwrap();

    let pages = doc.pages().unwrap();
    assert_eq!(pages.len(), 1);

    let page = &pages[0];
    let fonts = page.fonts(&doc).unwrap();
    let content = page.build_content(&doc).unwrap();
    let mut input = BStr::new(&content);
    let mut it = iterator(&mut input, preceded(multispace0, Operator::extract));
    let mut objects = TextObjects::new(&mut it, &fonts);

    assert!(objects.next_object().unwrap().is_none());
}

/// A run as `kerned.txt` lists it: start, end and decoded text.
#[derive(Debug)]
struct KernedRun {
    x: f32,
    y: f32,
    end_x: f32,
    end_y: f32,
    text: String,
}

/// Parses `x y end_x end_y (text)`, one run per line, skipping blank and `#` lines. The
/// parentheses around the text keep a space-only run intact.
fn read_kerned(path: &str) -> Vec<KernedRun> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.splitn(5, ' ');
            let mut number = || fields.next().unwrap().parse().unwrap();
            let (x, y, end_x, end_y) = (number(), number(), number(), number());
            let text = fields.next().unwrap();
            let text = text
                .strip_prefix('(')
                .and_then(|text| text.strip_suffix(')'))
                .unwrap_or_else(|| panic!("text not in parentheses: {line:?}"));
            KernedRun {
                x,
                y,
                end_x,
                end_y,
                text: text.to_string(),
            }
        })
        .collect()
}

/// `kerned.pdf` exercises every term of the advance on Helvetica widths: `TJ` numbers,
/// `Tc`, `Tw` on the space, `Tz`, and a `Td` after a `TJ` that moves from the line matrix
/// rather than from where the text ended (line four, `(C)` at `(72, 628)`).
#[test]
fn kerned_runs_start_and_end_where_expected() {
    let expected = read_kerned("tests/resources/kerned.txt");
    let actual = runs("tests/resources/kerned.pdf");

    assert_eq!(expected.len(), 12);
    assert_eq!(actual.len(), expected.len(), "runs: {actual:?}");

    let close = |a: f32, b: f32| (a - b).abs() < 1e-2;

    for (i, (run, golden)) in actual.iter().zip(&expected).enumerate() {
        assert!(
            close(run.position.x, golden.x) && close(run.position.y, golden.y),
            "run {i} {:?}: expected start ({}, {}), got ({}, {})",
            golden.text,
            golden.x,
            golden.y,
            run.position.x,
            run.position.y
        );
        assert!(
            close(run.end.x, golden.end_x) && close(run.end.y, golden.end_y),
            "run {i} {:?}: expected end ({}, {}), got ({}, {})",
            golden.text,
            golden.end_x,
            golden.end_y,
            run.end.x,
            run.end.y
        );
        assert_eq!(run.text.decode(), golden.text, "run {i}");
    }
}
