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
    for Run { start, text, .. } in runs(path) {
        println!("- {:?} {:?}", start, text);
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
    assert_close(runs[0].start, (72.0, 709.6821));
    // Last line, `Y = 362.92529`: every run of a line shares its baseline.
    let last_y = runs[runs.len() - 1].start.y;
    assert!((last_y - 418.7568).abs() < 1e-2, "last y = {last_y}");

    let ys: Vec<f32> = runs.iter().map(|run| run.start.y).collect();
    assert!(
        ys.iter().all(|y| (418.0..710.0).contains(y)),
        "a run left the text column"
    );
    assert!(
        ys.iter().all(|y| *y <= ys[0]),
        "no run sits above the first line"
    );
}

/// A run as `font-before-bt.txt` lists it: start, font name and size, decoded text.
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
            while let Some(Run { start, text, .. }) = text_object.next() {
                let font = text_object
                    .text_object()
                    .parameters
                    .font
                    .as_ref()
                    .expect("a run is only yielded with a font selected");
                runs.push((
                    index,
                    GoldenRun {
                        x: start.x,
                        y: start.y,
                        font: String::from_utf8_lossy(&font.name.0).into_owned(),
                        size: font.size.0,
                        text,
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

/// A run as a golden sidecar (`kerned.txt`, `encodings.txt`) lists it: start, end and
/// decoded text.
#[derive(Debug)]
struct SidecarRun {
    x: f32,
    y: f32,
    end_x: f32,
    end_y: f32,
    text: String,
}

/// Parses `x y end_x end_y (text)`, one run per line, skipping blank and `#` lines. The
/// parentheses around the text keep a space-only run intact; the text is UTF-8.
fn read_sidecar(path: &str) -> Vec<SidecarRun> {
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
            SidecarRun {
                x,
                y,
                end_x,
                end_y,
                text: text.to_string(),
            }
        })
        .collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-2
}

/// Asserts that `actual` has the start, end and text of every run of the sidecar at `path`,
/// positions to `1e-2`.
fn assert_matches_sidecar(actual: &[Run], path: &str) {
    let expected = read_sidecar(path);
    assert_eq!(actual.len(), expected.len(), "runs: {actual:?}");

    for (i, (run, golden)) in actual.iter().zip(&expected).enumerate() {
        assert!(
            close(run.start.x, golden.x) && close(run.start.y, golden.y),
            "run {i} {:?}: expected start ({}, {}), got ({}, {})",
            golden.text,
            golden.x,
            golden.y,
            run.start.x,
            run.start.y
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
        assert_eq!(run.text, golden.text, "run {i}");
    }
}

/// `kerned.pdf` exercises every term of the advance on Helvetica widths: `TJ` numbers,
/// `Tc`, `Tw` on the space, `Tz`, and a `Td` after a `TJ` that moves from the line matrix
/// rather than from where the text ended (line four, `(C)` at `(72, 628)`).
#[test]
fn kerned_runs_start_and_end_where_expected() {
    assert_eq!(read_sidecar("tests/resources/kerned.txt").len(), 12);
    assert_matches_sidecar(
        &runs("tests/resources/kerned.pdf"),
        "tests/resources/kerned.txt",
    );
}

/// `encodings.pdf` has one font per decoding route: Helvetica on `StandardEncoding` with AFM
/// widths, `/Differences` over a `/Widths` table, a `/ToUnicode` over `WinAnsiEncoding`, and
/// the built-in Symbol table. `encodings.txt` shows the arithmetic.
#[test]
fn encodings_runs_decode_and_advance_as_expected() {
    let actual = runs("tests/resources/encodings.pdf");
    assert_eq!(read_sidecar("tests/resources/encodings.txt").len(), 4);
    assert_matches_sidecar(&actual, "tests/resources/encodings.txt");

    // The `/Differences` run: `fi` through the glyph list, `Adieresis`, `uni2014` through
    // the AGL rule, and code 4, unmapped and outside `/FirstChar..LastChar`, with no text
    // and no width.
    let expected = [
        (1, 72.0, 78.0, Some("\u{FB01}")),
        (2, 78.0, 86.004, Some("Ä")),
        (3, 86.004, 98.004, Some("—")),
        (4, 98.004, 98.004, None),
    ];
    let characters = &actual[1].characters;
    assert_eq!(characters.len(), expected.len(), "{characters:?}");

    for (character, (code, start_x, end_x, text)) in characters.iter().zip(expected) {
        assert_eq!(character.code.value(), code);
        assert_eq!(character.code.bytes(), 1);
        assert!(
            close(character.start.x, start_x) && close(character.start.y, 680.0),
            "code {code}: expected start ({start_x}, 680), got {:?}",
            character.start
        );
        assert!(
            close(character.end.x, end_x) && close(character.end.y, 680.0),
            "code {code}: expected end ({end_x}, 680), got {:?}",
            character.end
        );
        assert_eq!(character.text.as_deref(), text, "code {code}");
    }
}

/// `letter.pdf` uses `WinAnsiEncoding` TrueType fonts for its header: the first text
/// objects show a single space each, and text object 42 (`/F3`, Times New Roman Bold) is
/// the first to show words, split into runs by its `TJ` offsets. Text object 60 has an
/// accented capital, `0xC9` in WinAnsi.
#[test]
fn letter_header_decodes_to_words() {
    let runs = font_runs("tests/resources/letter.pdf");

    for (object, run) in &runs[..3] {
        assert_eq!(run.text, " ", "text object {object}");
    }

    let text_of = |index: usize| -> String {
        runs.iter()
            .filter(|(object, _)| *object == index)
            .map(|(_, run)| run.text.as_str())
            .collect()
    };

    assert_eq!(text_of(42), "Groupe Hospitalier Henri Mondor");
    assert_eq!(text_of(60), "SANTÉ PUBLIQUE");
}

/// `text.pdf` shows every string in `Type0` fonts with two-byte codes, which livre does
/// not decode yet: every run is replacement characters only, one per code. PR 5 of the
/// content-extraction spec (composite fonts) turns this around.
#[rstest]
#[case("tests/resources/text.pdf")]
#[case("tests/resources/text-xref-stream.pdf")]
fn composite_runs_are_replacement_characters_until_pr5(#[case] path: &str) {
    let runs = runs(path);
    assert!(!runs.is_empty());

    for (i, run) in runs.iter().enumerate() {
        assert!(
            run.text.chars().all(|c| c == '\u{FFFD}'),
            "run {i}: {:?}",
            run.text
        );
        assert_eq!(run.characters.len(), run.text.chars().count(), "run {i}");
        assert!(
            run.characters
                .iter()
                .all(|character| character.code.bytes() == 2 && character.text.is_none()),
            "run {i}"
        );
    }
}
