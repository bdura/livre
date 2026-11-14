use livre_wasm::{analyze, Analysis, FontKind, Page, Rect, Run, Stage, StageError, TextObject};

const TEXT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../livre/tests/resources/text.pdf"
));
const LETTER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../livre/tests/resources/letter.pdf"
));
const TEXT_XREF_STREAM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../livre/tests/resources/text-xref-stream.pdf"
));
/// Text state set outside or before `BT`, carried into later text objects.
const FONT_BEFORE_BT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../livre/tests/resources/font-before-bt.pdf"
));
/// Helvetica with explicit widths: a kerned `TJ`, `Tc`, `Tw` and `Tz`, and a `Td` after a `TJ`.
const KERNED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../livre/tests/resources/kerned.pdf"
));
/// The expected runs of `kerned.pdf`: `x y end_x end_y (text)` per line, `#` lines aside.
const KERNED_RUNS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../livre/tests/resources/kerned.txt"
));

/// The checks every fixture passes: a clean parse of `page_count` pages with a media box
/// each, and at least one shown string on the first page.
fn assert_clean(analysis: &Analysis, page_count: usize) {
    assert_eq!(analysis.document, None);
    assert_eq!(analysis.pages.len(), page_count);

    for page in &analysis.pages {
        assert_eq!(page.error, None, "page {}", page.index);
        assert!(!page.media_box_defaulted, "page {}", page.index);
    }

    let first = &analysis.pages[0];
    let run = first
        .text_objects
        .iter()
        .flat_map(|object| &object.runs)
        .next()
        .expect("the first page shows some text");

    assert!(!run.bytes.is_empty());
    assert_eq!(run.bytes.len() % 2, 0);
    assert!(run
        .bytes
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
}

fn first_page_runs(analysis: &Analysis) -> Vec<&Run> {
    analysis.pages[0]
        .text_objects
        .iter()
        .flat_map(|object| &object.runs)
        .collect()
}

#[test]
fn text_pdf_parses_without_errors() {
    assert_clean(&analyze(TEXT), 1);
}

#[test]
fn letter_pdf_parses_without_errors() {
    assert_clean(&analyze(LETTER), 1);
}

#[test]
fn xref_stream_pdf_parses_without_errors() {
    let analysis = analyze(TEXT_XREF_STREAM);
    assert_clean(&analysis, 1);
    assert_eq!(first_page_runs(&analysis), first_page_runs(&analyze(TEXT)));
}

#[test]
fn font_before_bt_pdf_parses_without_errors() {
    let analysis = analyze(FONT_BEFORE_BT);
    assert_clean(&analysis, 1);

    let text_objects = &analysis.pages[0].text_objects;
    assert_eq!(text_objects.len(), 4);
    assert!(text_objects.iter().all(|object| object.runs.len() == 1));

    let texts: Vec<&str> = first_page_runs(&analysis)
        .iter()
        .map(|run| run.text.as_str())
        .collect();
    assert_eq!(texts, ["first", "second", "big", "small again"]);
}

#[test]
fn kerned_pdf_parses_without_errors() {
    let analysis = analyze(KERNED);
    assert_clean(&analysis, 1);
    assert_eq!(analysis.pages[0].text_objects.len(), 4);
}

/// The first text object of `kerned.pdf` is one kerned `TJ` line: its runs advance along
/// the line instead of clustering at its start, and span the width `kerned.txt` expects.
#[test]
fn kerned_pdf_first_line_spreads_across_its_width() {
    // `[x, y, end_x, end_y]` of every expected run.
    let expected: Vec<Vec<f32>> = KERNED_RUNS
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            line.split(' ')
                .take(4)
                .map(|field| field.parse().unwrap())
                .collect()
        })
        .collect();
    let first_y = expected[0][1];
    let line: Vec<&Vec<f32>> = expected.iter().filter(|run| run[1] == first_y).collect();
    let width = line[line.len() - 1][2] - line[0][0];

    let analysis = analyze(KERNED);
    let runs = &analysis.pages[0].text_objects[0].runs;
    assert_eq!(runs.len(), line.len());

    assert!(
        runs.windows(2).all(|pair| pair[0].x < pair[1].x),
        "x not strictly increasing: {runs:?}"
    );
    let actual = runs[runs.len() - 1].end_x - runs[0].x;
    assert!(
        (actual - width).abs() < 1e-2,
        "line width: expected {width}, got {actual}"
    );
}

#[test]
fn kerned_pdf_runs_name_their_font() {
    let analysis = analyze(KERNED);
    let run = &analysis.pages[0].text_objects[0].runs[0];

    assert_eq!(run.font, "F1");
    assert_eq!(run.font_kind, Some(FontKind::Simple));
    assert_eq!(run.base_font.as_deref(), Some("Helvetica"));
}

#[test]
fn garbage_is_a_document_error() {
    let analysis = analyze(b"not a pdf");
    assert!(matches!(
        analysis.document,
        Some(StageError {
            stage: Stage::Document,
            ..
        })
    ));
    assert!(analysis.pages.is_empty());
}

#[test]
fn truncated_fixture_is_a_document_error() {
    let analysis = analyze(&LETTER[..LETTER.len() / 2]);
    assert!(matches!(
        analysis.document,
        Some(StageError {
            stage: Stage::Document,
            ..
        })
    ));
    assert!(analysis.pages.is_empty());
}

#[test]
fn run_fields_round_trip_through_json() {
    let analysis = Analysis {
        version: "0.0.0".into(),
        document: None,
        pages: vec![Page {
            index: 0,
            media_box: Rect {
                xmin: 0.0,
                ymin: 0.0,
                xmax: 612.0,
                ymax: 792.0,
            },
            media_box_defaulted: true,
            user_unit: 1.0,
            text_objects: vec![TextObject {
                runs: vec![Run {
                    x: 1.0,
                    y: 2.0,
                    end_x: 3.0,
                    end_y: 2.0,
                    font: "F1".into(),
                    font_kind: Some(FontKind::Simple),
                    base_font: Some("Helvetica".into()),
                    size: 12.0,
                    text: "A".into(),
                    bytes: "41".into(),
                }],
            }],
            error: Some(StageError {
                stage: Stage::TextObject,
                message: "IncompleteTextObject".into(),
            }),
        }],
    };

    let json = serde_json::to_string(&analysis).unwrap();

    assert!(json.contains(r#""stage":"text-object""#), "{json}");
    assert!(json.contains(r#""font_kind":"simple""#), "{json}");
    assert!(json.contains(r#""base_font":"Helvetica""#), "{json}");
    for field in [
        "media_box",
        "media_box_defaulted",
        "user_unit",
        "text_objects",
        "runs",
        "bytes",
        "end_x",
        "end_y",
        "xmin",
    ] {
        assert!(json.contains(&format!("\"{field}\":")), "{field} in {json}");
    }
}
