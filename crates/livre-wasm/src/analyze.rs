use livre::{
    content::{operators::Operator, parse_text_object, GraphicsState},
    extraction::Extract,
    structure::Page as LivrePage,
    InMemoryDocument,
};
use winnow::{
    ascii::multispace0,
    combinator::{iterator, preceded},
    error::ContextError,
    BStr,
};

use crate::dto::{Analysis, Page, Rect, Run, Stage, StageError, TextObject};

/// US Letter, the fallback for a page tree without the (required) `MediaBox`.
const LETTER: Rect = Rect {
    xmin: 0.0,
    ymin: 0.0,
    xmax: 612.0,
    ymax: 792.0,
};

/// How much of an unparsed content stream tail goes into an `Operators` error.
const LEFTOVER_PREVIEW: usize = 200;

fn stage_error(stage: Stage, error: impl std::fmt::Debug) -> StageError {
    // winnow's `Display` is nearly empty; `Debug` at least names the failing context.
    StageError {
        stage,
        message: format!("{error:?}"),
    }
}

/// Extract the text runs of every page of a PDF.
///
/// Never fails and never panics on its input: each failure is recorded as a [`StageError`]
/// on the document or on the page it stopped, keeping whatever was extracted before it.
pub fn analyze(bytes: &[u8]) -> Analysis {
    let mut analysis = Analysis {
        version: livre::VERSION.to_string(),
        document: None,
        pages: Vec::new(),
    };

    let doc = match InMemoryDocument::extract(&mut BStr::new(bytes)) {
        Ok(doc) => doc,
        Err(e) => {
            analysis.document = Some(stage_error(Stage::Document, e));
            return analysis;
        }
    };

    let pages = match doc.pages() {
        Ok(pages) => pages,
        Err(e) => {
            analysis.document = Some(stage_error(Stage::Pages, e));
            return analysis;
        }
    };

    analysis.pages = pages
        .iter()
        .enumerate()
        .map(|(index, page)| analyze_page(&doc, index, page))
        .collect();

    analysis
}

fn analyze_page(doc: &InMemoryDocument, index: usize, page: &LivrePage) -> Page {
    let (media_box, media_box_defaulted) = match page.inheritable_props.media_box {
        Some(r) => (
            Rect {
                xmin: r.xmin,
                ymin: r.ymin,
                xmax: r.xmax,
                ymax: r.ymax,
            },
            false,
        ),
        None => (LETTER, true),
    };

    let mut out = Page {
        index,
        media_box,
        media_box_defaulted,
        user_unit: page.user_unit,
        text_objects: Vec::new(),
        error: None,
    };

    match page.build_content(doc) {
        Ok(content) => out.error = extract_text_objects(&content, &mut out.text_objects),
        Err(e) => out.error = Some(stage_error(Stage::Content, e)),
    }

    out
}

/// Push the text objects of `content` onto `text_objects`, returning the error that
/// stopped the parse, if any.
fn extract_text_objects(content: &[u8], text_objects: &mut Vec<TextObject>) -> Option<StageError> {
    let mut stream = BStr::new(content);
    let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));
    // One per content stream: a `cm` at the top of the page applies to every text object.
    let mut graphics_state = GraphicsState::default();

    loop {
        let mut text_object = match parse_text_object(&mut it, &mut graphics_state) {
            Ok(Some(text_object)) => text_object,
            Ok(None) => break,
            Err(e) => return Some(stage_error(Stage::TextObject, e)),
        };

        let mut runs = Vec::new();
        while let Some(((x, y), raw)) = text_object.next_raw() {
            // The font in effect is the one of the run just yielded.
            let (font, size) = match &text_object.text_object().parameters.font {
                Some(font) => (
                    String::from_utf8_lossy(&font.name.0).into_owned(),
                    font.size,
                ),
                None => (String::new(), 0.0),
            };
            runs.push(Run {
                x,
                y,
                font,
                size,
                text: raw.decode(),
                bytes: raw.0.iter().map(|b| format!("{b:02x}")).collect(),
            });
        }
        text_objects.push(TextObject { runs });
    }

    if let Err(e) = it.finish() {
        return Some(stage_error(Stage::Operators, e));
    }

    // An operator livre cannot parse stops the iterator without an error: whatever
    // remains past the trailing whitespace is that operator onwards.
    if multispace0::<&BStr, ContextError>(&mut stream).is_ok() && !stream.is_empty() {
        let preview = &stream[..LEFTOVER_PREVIEW.min(stream.len())];
        return Some(StageError {
            stage: Stage::Operators,
            message: format!(
                "unparsed content from: {}",
                String::from_utf8_lossy(preview)
            ),
        });
    }

    None
}
