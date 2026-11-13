use livre::{extraction::Extract, InMemoryDocument};

/// Builds a PDF out of its body sections, appending one cross-reference section per update.
///
/// Each section is a list of `(object number, object body)`. Objects are written with generation
/// 0, and each section's trailer points to the previous cross-reference section through `/Prev`.
/// `first_prev` gives the first section a `/Prev` entry, which it normally does not have.
///
/// Returns the file along with the offset of its last cross-reference section.
fn build_pdf(sections: &[&[(usize, &str)]], first_prev: Option<usize>) -> (Vec<u8>, usize) {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut prev = first_prev;

    for (i, objects) in sections.iter().enumerate() {
        let mut offsets = Vec::new();

        for &(number, body) in objects.iter() {
            offsets.push((number, pdf.len()));
            pdf.extend(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
        }

        let startxref = pdf.len();
        pdf.extend(b"xref\n");
        if i == 0 {
            pdf.extend(b"0 1\n0000000000 65535 f \n");
        }
        for (number, offset) in offsets {
            pdf.extend(format!("{number} 1\n{offset:010} 00000 n \n").as_bytes());
        }

        let prev_entry = prev.map(|p| format!(" /Prev {p}")).unwrap_or_default();
        pdf.extend(
            format!(
                "trailer\n<< /Size 5 /Root 1 0 R{prev_entry} >>\nstartxref\n{startxref}\n%%EOF\n"
            )
            .as_bytes(),
        );

        prev = Some(startxref);
    }

    (pdf, prev.unwrap())
}

const ORIGINAL: &[(usize, &str)] = &[
    (1, "<< /Type /Catalog /Pages 2 0 R >>"),
    (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
    (3, "<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>"),
    (4, "<< /Length 8 >>\nstream\n(old) Tj\nendstream"),
];

const UPDATE: &[(usize, &str)] = &[(4, "<< /Length 8 >>\nstream\n(new) Tj\nendstream")];

fn first_page_content(pdf: &[u8]) -> Vec<u8> {
    let doc = InMemoryDocument::extract(&mut pdf.as_ref()).unwrap();
    let pages = doc.pages().unwrap();
    pages[0].build_content(&doc).unwrap()
}

#[test]
fn newer_section_takes_precedence() {
    let (pdf, _) = build_pdf(&[ORIGINAL, UPDATE], None);

    assert_eq!(b"(new) Tj".as_slice(), first_page_content(&pdf));
}

#[test]
fn prev_cycle_terminates() {
    // The original section's `/Prev` points at the update, which points back at it. The update's
    // offset depends on the length of that `/Prev` entry, hence the fixed point.
    let mut target = 0;
    let pdf = loop {
        let (pdf, last) = build_pdf(&[ORIGINAL, UPDATE], Some(target));
        if last == target {
            break pdf;
        }
        target = last;
    };

    assert_eq!(b"(new) Tj".as_slice(), first_page_content(&pdf));
}
