// livre's layer: what livre extracted, drawn over the pdf.js render.
//
// livre knows where each shown string starts and ends, from the font widths, and at what
// font size. So each run is a box from its start to its end, one font size tall, with a
// dot at its baseline origin and a tick at each character's start, to compare glyph by
// glyph with the pdf.js render. Each text object also gets a hull around its runs' boxes,
// shown only while the inspector highlights it: Word exports split a line into many small
// text objects, whose hulls would otherwise cover the page in overlapping frames.
// The box height is the font size, not the glyphs' real extent: compare widths and
// baselines with the pdf.js boxes, not heights.

import type { Page } from "./pkg/livre_wasm.js";
import type { PageViewport } from "./pdfjs.js";
import { element } from "./svg.js";

type Point = [number, number];
/** A run box: baseline start, baseline end, then the end and start one font size up. */
type Corners = [Point, Point, Point, Point];

/**
 * Replace the `<g class="livre">` in `svg` with livre's markers for `page`.
 *
 * `page` may be partial when it has an error. `viewport` is the pdf.js `PageViewport`
 * the page was rendered with; points go through `convertToViewportPoint`, so rotation,
 * crop box and scale apply to both overlays alike. A page with no text objects yields
 * an empty group.
 *
 * Each run's group carries `data-object` and `data-run`, its indices in
 * `page.text_objects` and in that object's `runs`; each hull carries `data-object`.
 * The page reads them to show a run in its inspector and to highlight a hull.
 */
export function drawLivreLayer(svg: SVGSVGElement, page: Page, viewport: PageViewport): SVGGElement {
  svg.querySelector(":scope > g.livre")?.remove();
  const layer = element("g", { class: "livre" });

  page.text_objects.forEach((object, objectIndex) => {
    if (object.runs.length === 0) return;
    const box = new Bounds();
    object.runs.forEach((run, runIndex) => {
      // The height goes straight up in user space, not across the baseline: boxes fit unrotated text only.
      const corners: Corners = [
        viewport.convertToViewportPoint(run.start.x, run.start.y),
        viewport.convertToViewportPoint(run.end.x, run.end.y),
        viewport.convertToViewportPoint(run.end.x, run.end.y + run.size),
        viewport.convertToViewportPoint(run.start.x, run.start.y + run.size),
      ];
      for (const corner of corners) box.add(...corner);
      // A quarter of the font size marks each start without hiding the glyphs; straight up, like the box's height.
      const ticks = run.characters.map(({ start }): [Point, Point] => [
        viewport.convertToViewportPoint(start.x, start.y),
        viewport.convertToViewportPoint(start.x, start.y + run.size / 4),
      ]);
      layer.append(runMarker(corners, ticks, objectIndex, runIndex));
    });
    // Behind the markers, so it never takes their pointer events.
    layer.prepend(
      element("rect", {
        class: "hull",
        x: box.xmin,
        y: box.ymin,
        width: box.xmax - box.xmin,
        height: box.ymax - box.ymin,
        "data-object": objectIndex,
      }),
    );
  });

  svg.append(layer);
  return layer;
}

/** A run's box through `corners`, its origin dot, and a tick from each pair of `ticks`. */
function runMarker(
  corners: Corners,
  ticks: [Point, Point][],
  objectIndex: number,
  runIndex: number,
): SVGGElement {
  const [[bx, by], , , [tx, ty]] = corners;
  const group = element("g", { class: "run", "data-object": objectIndex, "data-run": runIndex });
  group.append(
    element("polygon", { class: "box", points: corners.map(([x, y]) => `${x},${y}`).join(" ") }),
    // An invisible, wider stroke on the start edge, so a run that does not advance (a
    // composite font) can still be hovered without pixel precision.
    element("line", { class: "hit", x1: bx, y1: by, x2: tx, y2: ty }),
    ...ticks.map(([[x1, y1], [x2, y2]]) => element("line", { class: "tick", x1, y1, x2, y2 })),
    element("circle", { cx: bx, cy: by, r: 2 }),
  );
  return group;
}

class Bounds {
  xmin = Infinity;
  ymin = Infinity;
  xmax = -Infinity;
  ymax = -Infinity;

  add(x: number, y: number): void {
    this.xmin = Math.min(this.xmin, x);
    this.ymin = Math.min(this.ymin, y);
    this.xmax = Math.max(this.xmax, x);
    this.ymax = Math.max(this.ymax, y);
  }
}
