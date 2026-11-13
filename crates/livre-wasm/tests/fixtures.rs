use livre_wasm::{analyze, Analysis, Page, Rect, Run, Stage, StageError, TextObject};

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
                    font: "F1".into(),
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
    for field in [
        "media_box",
        "media_box_defaulted",
        "user_unit",
        "text_objects",
        "runs",
        "bytes",
        "xmin",
    ] {
        assert!(json.contains(&format!("\"{field}\":")), "{field} in {json}");
    }
}
