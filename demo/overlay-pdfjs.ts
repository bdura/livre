// pdf.js's layer: the ground truth livre's layer is compared against.
//
// pdf.js knows each text item's full transform and advance, which livre lacks, so its
// items are drawn as boxes: one font height tall, one advance wide, sitting on the
// baseline. Where a livre marker is not at the left edge of a box on the same baseline,
// livre is wrong or does not support what the page does.

import type { Matrix, PageViewport, PDFPageProxy, TextItem } from "./pdfjs.js";
import { element } from "./svg.js";

/**
 * Replace the `<g class="pdfjs">` in `svg` with a box per non-empty text item of
 * `pdfPage`, placed with `viewport`, the `PageViewport` the page was rendered with.
 *
 * The group goes first in `svg`, under livre's layer, so livre's markers keep their
 * pointer events where the two overlap. The text content is fetched first; if `isCurrent()`
 * is then false, a later render owns `svg`: it is left untouched and null is returned.
 */
export async function drawGroundTruthLayer(
  svg: SVGSVGElement,
  pdfPage: PDFPageProxy,
  viewport: PageViewport,
  isCurrent: () => boolean = () => true,
): Promise<SVGGElement | null> {
  const { items } = await pdfPage.getTextContent();
  if (!isCurrent()) return null;
  const layer = element("g", { class: "pdfjs" });

  for (const item of items) {
    // Marked-content markers carry no `str`; whitespace-only items are kept, they are text.
    if (!("str" in item) || !item.str) continue;
    layer.append(itemBox(item, viewport));
  }

  svg.querySelector(":scope > g.pdfjs")?.remove();
  svg.prepend(layer);
  return layer;
}

function itemBox(item: TextItem, viewport: PageViewport): SVGRectElement {
  // Text space to viewport space: rotation, crop box, y flip and scale in one matrix.
  const [a, b, , , e, f] = multiply(viewport.transform, item.transform);
  // `width` and `height` are in user space, so only the viewport's scale applies.
  const width = item.width * viewport.scale;
  const height = item.height * viewport.scale;
  const angle = (Math.atan2(b, a) * 180) / Math.PI;

  const box = element("rect", {
    class: "item",
    x: 0,
    // The viewport's y axis points down: the box extends upwards from the baseline.
    y: -height,
    width,
    height,
    transform: `translate(${e} ${f})${angle ? ` rotate(${angle})` : ""}`,
  });
  const [, , , , x, y] = item.transform;
  const title = element("title");
  title.textContent = [
    `text: ${JSON.stringify(item.str)}`,
    `font: ${item.fontName}`,
    `(x, y): (${round(x)}, ${round(y)})`,
  ].join("\n");
  box.append(title);
  return box;
}

/** The affine product `m1 × m2`, as pdf.js's `Util.transform`. */
function multiply(m1: Matrix, m2: Matrix): Matrix {
  return [
    m1[0] * m2[0] + m1[2] * m2[1],
    m1[1] * m2[0] + m1[3] * m2[1],
    m1[0] * m2[2] + m1[2] * m2[3],
    m1[1] * m2[2] + m1[3] * m2[3],
    m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
    m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
  ];
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}
