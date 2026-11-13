# Runs the built demo's wasm bindings under Node, on the samples its page offers:
# `demo.nix` only checks that the site builds and type-checks.
{
  pkgs,
  # `packages.demo`.
  demo,
}:
pkgs.runCommand "livre-demo-test" {
  nativeBuildInputs = [pkgs.nodejs];
  DEMO_SITE = demo;
} ''
  node --test ${../demo/test/site.test.mjs}
  touch $out
''
