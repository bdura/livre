// Runs the built site's wasm bindings under Node, on the samples its page offers.
// `nix/demo-test.nix` runs it against `packages.demo`, passed as `DEMO_SITE`.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { before, test } from "node:test";

const site = process.env.DEMO_SITE;
assert.ok(site, "DEMO_SITE must point at the built site");

const { analyze_pdf, initSync } = await import(join(site, "pkg/livre_wasm.js"));

before(() => {
  initSync({ module: readFileSync(join(site, "pkg/livre_wasm_bg.wasm")) });
});

const html = readFileSync(join(site, "index.html"), "utf8");
const samples = [...html.matchAll(/<option value="([^"]+\.pdf)"/g)].map((match) => match[1]);

const analyze = (name) => analyze_pdf(readFileSync(join(site, "fixtures", name)));
const runs = (page) => page.text_objects.flatMap((object) => object.runs);

test("the page offers samples", () => {
  assert.ok(samples.length > 0);
});

test("every relative asset the page loads is in the site", () => {
  const assets = [...html.matchAll(/(?:src|href)="([^":]+)"/g)].map((match) => match[1]);
  const fetched = ["version.json", ...samples.map((name) => `fixtures/${name}`)];
  for (const asset of [...assets, ...fetched]) {
    assert.ok(existsSync(join(site, asset)), asset);
  }
});

for (const name of samples) {
  test(`${name} analyses without errors`, () => {
    const analysis = analyze(name);

    // The page tests absent options against `null`, not `undefined`.
    assert.equal(analysis.document, null);
    assert.ok(analysis.pages.length > 0);
    for (const page of analysis.pages) {
      assert.equal(page.error, null, `page ${page.index}`);
      assert.equal(page.media_box_defaulted, false);
    }

    const [run] = runs(analysis.pages[0]);
    assert.ok(run, "the first page shows some text");
    assert.equal(typeof run.x, "number");
    assert.match(run.bytes, /^(?:[0-9a-f]{2})+$/);
  });
}

test("the xref stream sample reads like the plain one", () => {
  assert.deepEqual(runs(analyze("text-xref-stream.pdf").pages[0]), runs(analyze("text.pdf").pages[0]));
});

test("garbage is a document-stage error, not a throw", () => {
  const analysis = analyze_pdf(new TextEncoder().encode("not a pdf"));
  assert.equal(analysis.document?.stage, "document");
  assert.deepEqual(analysis.pages, []);
});

test("the version file names the crate version", () => {
  const { version } = JSON.parse(readFileSync(join(site, "version.json"), "utf8"));
  assert.equal(version, analyze("text.pdf").version);
});
