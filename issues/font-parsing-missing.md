---
title: Font parsing not yet implemented (v0.6.0 roadmap item)
severity: medium
---

According to the README.md roadmap, font parsing is the next major feature planned for v0.6.0:

```markdown
- [ ] (`v0.6.0`) Parser for fonts definition - starting with "simple fonts".
      This step will allow iterating over characters and their actual position.
```

Currently, Livre can parse page content streams and extract text operators, but it cannot parse font definitions. This means:

- Text content can be extracted, but character positions are not accurate
- Font metrics (width, height, etc.) are not available
- Text rendering information is incomplete

**Location**: README.md, roadmap section

**Impact**: Users cannot get accurate character positioning or font information from PDFs. This limits the library's usefulness for text extraction use cases that require precise layout information.

**Reproduction**: Try to extract text with accurate positioning from any PDF with non-trivial fonts.

**Suggested fix**: Implement font parsing for simple fonts as described in the roadmap. This should include:
- Parsing font dictionaries
- Extracting font metrics
- Calculating character positions based on font information
