# The extraction benchmark's corpus: the `bench` entries of `corpus.toml`,
# fetched by hash, plus `tracemonkey-objstm.pdf`, a qpdf rewrite of
# `tracemonkey.pdf` with object streams.
#
# `$out` holds `<name>.pdf` for each entry; the dev shell exports it as
# `LIVRE_BENCH_CORPUS`.
pkgs: let
  inherit (pkgs) lib;

  manifest = builtins.fromTOML (builtins.readFile ./corpus.toml);

  documents = builtins.filter (document: document.bench) manifest.document;

  # The arXiv URLs have no extension, so each fetch is named explicitly.
  fetch = document:
    pkgs.fetchurl {
      name = "${document.name}.pdf";
      inherit (document) url sha256;
    };
in
  pkgs.runCommand "livre-bench-corpus" {nativeBuildInputs = [pkgs.qpdf];} ''
    mkdir -p $out
    ${lib.concatMapStrings (document: ''
        cp ${fetch document} $out/${document.name}.pdf
      '')
      documents}
    # `--deterministic-id` makes `/ID` a hash of the content, so the output is
    # reproducible. The pair differs only in object streams.
    qpdf --object-streams=generate --deterministic-id \
      $out/tracemonkey.pdf $out/tracemonkey-objstm.pdf
  ''
