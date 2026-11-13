// livre's layer: what livre extracted, drawn over the pdf.js render.
//
// livre knows where each shown string starts and at what font size, but has no glyph
// widths and no graphics state. So each run is a dot at its baseline origin with a tick
// one font size tall, and each text object a dashed hull around its runs' markers: a
// hull of known points, never an estimated extent.

const SVG = "http://www.w3.org/2000/svg";

/**
 * Replace the `<g class="livre">` in `svg` with livre's markers for `page`.
 *
 * `page` is a page DTO from `analyze_pdf` (`{ text_objects: [{ runs }] }`), possibly
 * partial when the page has an error. `viewport` is the pdf.js `PageViewport` the
 * page was rendered with; points go through `convertToViewportPoint`, so rotation,
 * crop box and scale apply to both overlays alike. A page with no text objects
 * yields an empty group.
 *
 * Each run's group carries `data-object` and `data-run`, its indices in
 * `page.text_objects` and in that object's `runs`; each hull carries `data-object`.
 * The page reads them to show a run in its inspector and to highlight a hull.
 */
export function drawLivreLayer(svg, page, viewport) {
  svg.querySelector(":scope > g.livre")?.remove();
  const layer = element("g", { class: "livre" });

  page.text_objects.forEach((object, objectIndex) => {
    if (object.runs.length === 0) return;
    const box = new Bounds();
    let maxSize = 0;
    object.runs.forEach((run, runIndex) => {
      const [bx, by] = viewport.convertToViewportPoint(run.x, run.y);
      // The tick goes up in text space, so it follows the page's rotation.
      const [tx, ty] = viewport.convertToViewportPoint(run.x, run.y + run.size);
      box.add(bx, by);
      box.add(tx, ty);
      maxSize = Math.max(maxSize, run.size);
      layer.append(runMarker([bx, by], [tx, ty], objectIndex, runIndex));
    });
    const pad = maxSize * viewport.scale;
    // Behind the markers, so it never takes their pointer events.
    layer.prepend(
      element("rect", {
        class: "hull",
        x: box.xmin - pad,
        y: box.ymin - pad,
        width: box.xmax - box.xmin + 2 * pad,
        height: box.ymax - box.ymin + 2 * pad,
        "data-object": objectIndex,
      }),
    );
  });

  svg.append(layer);
  return layer;
}

function runMarker([bx, by], [tx, ty], objectIndex, runIndex) {
  const group = element("g", { class: "run", "data-object": objectIndex, "data-run": runIndex });
  group.append(
    // An invisible, wider stroke so hovering does not need to be pixel-perfect.
    element("line", { class: "hit", x1: bx, y1: by, x2: tx, y2: ty }),
    element("line", { x1: bx, y1: by, x2: tx, y2: ty }),
    element("circle", { cx: bx, cy: by, r: 2 }),
  );
  return group;
}

class Bounds {
  xmin = Infinity;
  ymin = Infinity;
  xmax = -Infinity;
  ymax = -Infinity;

  add(x, y) {
    this.xmin = Math.min(this.xmin, x);
    this.ymin = Math.min(this.ymin, y);
    this.xmax = Math.max(this.xmax, x);
    this.ymax = Math.max(this.ymax, y);
  }
}

function element(name, attributes = {}) {
  const node = document.createElementNS(SVG, name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
  return node;
}
