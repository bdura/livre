# livre-font-data

Font data for [Livre](https://github.com/bdura/livre), a type-safe PDF parser:

- the [Adobe Glyph List](https://github.com/adobe-type-tools/agl-aglfn), which maps
  glyph names such as `Adieresis` or `ffi` to the Unicode text they stand for;
- the metrics of the fourteen standard Type 1 fonts (the "Core 14"), which a PDF
  may use without embedding them.

The upstream files are vendored unmodified under `data/`. The `.bin` files
beside them are compact lookup tables derived from those files and committed,
so the crate has no build script and no dependencies.

## Provenance

| File | Upstream | Revision |
| --- | --- | --- |
| `data/agl/glyphlist.txt` | [`glyphlist.txt`](https://github.com/adobe-type-tools/agl-aglfn/blob/4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6/glyphlist.txt) in `adobe-type-tools/agl-aglfn` (Adobe Glyph List 2.0) | commit `4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6` |
| `data/afm/<Font>.afm` (14 files) | Adobe's `Core14_AFMs` archive, as redistributed unchanged by Apache PDFBox in [`pdfbox/src/main/resources/org/apache/pdfbox/resources/afm/<Font>.afm`](https://github.com/apache/pdfbox/tree/4d4e48d106b27fe397fbd9c25cc1555777589bef/pdfbox/src/main/resources/org/apache/pdfbox/resources/afm) | commit `4d4e48d106b27fe397fbd9c25cc1555777589bef` |
| `data/afm/NOTICE` | `MustRead.html` from the same archive and directory, byte for byte, renamed | commit `4d4e48d106b27fe397fbd9c25cc1555777589bef` |

The fourteen AFM files are `Courier.afm`, `Courier-Bold.afm`, `Courier-Oblique.afm`,
`Courier-BoldOblique.afm`, `Helvetica.afm`, `Helvetica-Bold.afm`,
`Helvetica-Oblique.afm`, `Helvetica-BoldOblique.afm`, `Times-Roman.afm`,
`Times-Bold.afm`, `Times-Italic.afm`, `Times-BoldItalic.afm`, `Symbol.afm`
and `ZapfDingbats.afm`.

## Regenerating the lookup tables

The tests check every committed `.bin` file against the table generated from its
source. After changing a source file or the table format, regenerate them with:

```sh
LIVRE_FONT_DATA_UPDATE=1 cargo test -p livre-font-data
```

One run is enough for a blob that already exists. A new blob takes one more step,
because `include_bytes!` needs the file before the crate compiles: create it empty,
regenerate with the freshness tests alone (the lookup tests were compiled against
the empty file), then run the tests plainly.

```sh
touch data/afm/ <Font >.bin
LIVRE_FONT_DATA_UPDATE=1 cargo test -p livre-font-data _fresh
cargo test -p livre-font-data
```

## Licence

See [`LICENSE`](LICENSE), which has three sections:

- the crate's code is under the MIT licence;
- the Adobe Glyph List is under Adobe's BSD-style licence (SPDX `Adobe-Glyph`),
  copied from the header of `glyphlist.txt`;
- the Core 14 AFM files may be used, copied and distributed for any purpose,
  provided the copyright notices are retained, the files travel with Adobe's
  notice (`data/afm/NOTICE`), and modifications are prominently noted.
  None of the vendored files is modified.
