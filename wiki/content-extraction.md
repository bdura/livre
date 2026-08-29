# Content Extraction in Livre

How to extract text and other content from PDF pages.

## Index

- [content-extraction](content-extraction.md) — **active.** Extracting text and content from pages.

## Content Streams

Each PDF page contains one or more **content streams** that define how to render the page's content. These streams contain a sequence of **operators** that:

- Set graphics state (color, line width, etc.)
- Define paths and shapes
- Show text
- Show images

Livre focuses on **text extraction**, with support for other content types planned for future versions.

## Text Operators

Livre parses text operators into the `TextOperator` enum:

### Text Showing Operators

These operators display text:

- **`Tj`** - Show text string
  ```rust
  TextOperator::TextShowingOperator(PDFString)
  ```

- **`TJ`** - Show text array (with individual glyph positioning)
  ```rust
  TextOperator::TextShowingOperator(Vec<TextArrayElement>)
  ```

- **`'`** - Show text string and move to next line
- **`"`** - Show text string with word spacing and move to next line

### Text Positioning Operators

These operators move the text position:

- **`Td`** - Move text position
  ```rust
  TextOperator::TextPositioningOperator { tx: f32, ty: f32 }
  ```

- **`TD`** - Move text position and set leading
  ```rust
  TextOperator::TextPositioningOperator { tx: f32, ty: f32, leading: Option<f32> }
  ```

- **`T*`** - Move to start of next line
- **`Tm`** - Set text matrix (6 numbers: a, b, c, d, e, f)

### Text State Operators

These operators modify text state:

- **`Tf`** - Set font and font size
  ```rust
  TextOperator::TextStateOperator(TextStateOperator::SetFontAndFontSize { font: Name, size: f32 })
  ```

- **`Tc`** - Set character spacing
- **`Tw`** - Set word spacing
- **`Tz`** - Set horizontal scaling
- **`TL`** - Set text leading
- **`Tr`** - Set text rendering mode
- **`Ts`** - Set text rise

### Text Object Operators

These operators define text objects (grouped text operations):

- **`BT`** - Begin text object
- **`ET`** - End text object

## Parsing Content

### Basic Content Extraction

To get the raw bytes of a page's content stream:

```rust
for page in doc.pages()? {
    let content_bytes = page.build_content(&doc)?;
    // content_bytes: Vec<u8>
}
```

### Parsing Operators

To parse the operators from a content stream:

```rust
use livre::content::operators::Operator;
use winnow::{ascii::multispace0, combinator::{iterator, preceded}, BStr};

let content = page.build_content(&doc)?;
let mut stream = BStr::new(&content);

let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));

for operator in &mut it {
    match operator {
        Operator::BeginText(_) => println!("Begin text"),
        Operator::Text(text_op) => println!("Text: {:?}", text_op),
        Operator::EndText(_) => println!("End text"),
        // ... other operators
    }
}

it.finish()?;
```

### Using parse_text_object

For simpler text extraction, use the `parse_text_object` function:

```rust
use livre::content::state::parse_text_object;

let content = page.build_content(&doc)?;
let mut stream = BStr::new(&content);

while let Some(text_object) = parse_text_object(&mut stream)? {
    // text_object: TextObject
    // text_object.operators: Vec<Operator>
    // text_object.matrix: TextMatrix
    // text_object.parameters: TextStateParameters
    
    for op in text_object.operators {
        // Process each operator
    }
}
```

## TextObject Structure

A `TextObject` represents a grouped set of text operations:

```rust
pub struct TextObject {
    pub matrix: TextMatrix,
    pub parameters: TextStateParameters,
    pub operators: Vec<Operator>,
}
```

### TextMatrix

The text matrix defines the transformation from text space to user space:

```rust
pub struct TextMatrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}
```

This is a 3x3 affine transformation matrix:
```
[ a  b  0 ]
[ c  d  0 ]
[ e  f  1 ]
```

### TextStateParameters

Text state parameters control text rendering:

```rust
pub struct TextStateParameters {
    pub character_spacing: f32,
    pub word_spacing: f32,
    pub horizontal_scaling: f32,
    pub leading: f32,
    pub font: Option<Name>,
    pub font_size: f32,
    pub rendering_mode: RenderingMode,
    pub rise: f32,
}
```

## Extracting Text Content

### Simple Text Extraction

```rust
use livre::content::operators::{text::TextOperator, Operator};

for page in doc.pages()? {
    let content = page.build_content(&doc)?;
    let mut stream = BStr::new(&content);
    
    let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));
    
    for operator in &mut it {
        if let Operator::Text(TextOperator::TextShowingOperator(text)) = operator {
            print!("{}", text);
        }
    }
    
    it.finish()?;
}
```

### Text with Positioning

```rust
use livre::content::operators::{text::TextOperator, Operator};

let mut x = 0.0;
let mut y = 0.0;

for operator in &mut it {
    match operator {
        Operator::Text(TextOperator::TextShowingOperator(text)) => {
            print!("Text at ({}, {}): {}", x, y, text);
        }
        Operator::TextPositioning(text_pos) => {
            x += text_pos.tx;
            y += text_pos.ty;
        }
        // ... handle other positioning operators
    }
}
```

## Text Encoding

PDF text can be encoded in various ways:

### PDFDocEncoding

Standard PDF encoding for text in the PDF document itself.

### Font-Specific Encoding

Each font defines its own encoding. Livre currently does not parse font encodings (planned for v0.6.0).

### Unicode

Modern PDFs typically use Unicode (UTF-16BE) for text.

### Handling Encodings

Livre returns text as `PDFString` which can be:
- `LiteralString` - Text in parentheses: `(Hello)`
- `HexadecimalString` - Text in angle brackets: `<48656C6C6F>`

To get a Rust `String`:

```rust
use livre::extraction::PDFString;

match pdf_string {
    PDFString::Literal(s) => s.value().to_string(),
    PDFString::Hexadecimal(s) => s.value().to_string(),
}
```

## Limitations

As of v0.5.0, Livre's content extraction has these limitations:

### Not Yet Implemented

- **Font parsing**: Cannot determine character widths or encoding
- **Graphics operators**: Path construction, filling, stroking
- **Image operators**: Image display (Do)
- **Color space operators**: Color setting
- **Clipping operators**: Clipping paths

### Text-Specific Limitations

- **Character positioning**: Without font metrics, character positions are approximate
- **Ligatures**: Not handled specially
- **Kerning**: Not applied
- **Text direction**: Always left-to-right (no RTL, vertical, or bidirectional text)
- **Font substitution**: Cannot map PDF fonts to system fonts

## Future Enhancements

### v0.6.0: Font Parsing

Planned features:
- Parse font dictionaries
- Extract font metrics (widths, bounding boxes)
- Calculate accurate character positions
- Support simple fonts (Type 1, TrueType)

### Beyond v0.6.0

- Graphics state tracking
- Path construction and rendering
- Image extraction
- Color space handling
- Advanced text features (RTL, vertical, bidi)

## Tips for Text Extraction

### Handling Line Breaks

PDF doesn't have explicit line break characters. Look for:
- `T*` operator (move to next line)
- `Td` with ty != 0 (vertical movement)
- `TD` operator (movement with leading)

### Handling Spacing

Text spacing comes from multiple sources:
- Character spacing (`Tc` operator)
- Word spacing (`Tw` operator)
- Horizontal scaling (`Tz` operator)
- Font metrics (not yet available)

### Handling Fonts

Without font parsing, you can still:
- Get font names from the `Tf` operator
- Get font sizes
- Extract text content
- Get approximate positioning

For accurate rendering, you'll need to wait for v0.6.0 or implement font parsing yourself.

### Performance

Content parsing can be slow for pages with:
- Many text operators
- Complex paths
- Large images

For better performance:
- Only parse pages you need
- Use `parse_text_object` for text-focused extraction
- Avoid holding all pages in memory at once
