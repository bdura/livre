# PDF fixtures generated from the committed ones with a pinned qpdf.
#
# `$out` holds the generated PDFs: the `fixtures` app copies them into
# `crates/livre/tests/resources/`, the `fixtures` check compares them against
# the committed files.
pkgs:
pkgs.runCommand "livre-fixtures" {nativeBuildInputs = [pkgs.qpdf];} ''
  mkdir -p $out
  # qpdf writes cross-reference streams with a PNG predictor; `--deterministic-id`
  # makes `/ID` a hash of the content, so the output is reproducible.
  qpdf --object-streams=generate --deterministic-id \
    ${../crates/livre/tests/resources/text.pdf} $out/text-xref-stream.pdf
''
