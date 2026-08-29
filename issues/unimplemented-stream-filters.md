---
title: Multiple stream filters are parsed but return errors
severity: medium
---

In `src/filtering.rs`, the `Filter` enum includes variants for many PDF stream filters, but only `FlateDecode` is actually implemented. The following filters are defined in the enum but their `Extract` implementation returns errors:

- ASCII85Decode
- ASCIIHexDecode
- LZWDecode
- RunLengthDecode
- CCITTFaxDecode
- JBIG2Decode
- DCTDecode
- JPXDecode
- Crypt

When a PDF document uses any of these filters, parsing will fail with an "unsupported filter" error.

**Location**: `src/filtering.rs`, lines 27-36 (commented out in the enum definition) and lines 41-49 (error handling in `Extract` impl)

**Impact**: PDFs using compression formats other than FlateDecode (zlib) cannot be parsed. This includes PDFs with:
- JPEG compression (DCTDecode)
- JPEG2000 compression (JPXDecode)
- LZW compression (LZWDecode)
- ASCII85 or ASCIIHex encoded streams

**Reproduction**: Load a PDF that uses any of these filters and attempt to parse it.

**Suggested fix**: Implement the missing filter decoders or return a more informative error that includes which filter was encountered.
