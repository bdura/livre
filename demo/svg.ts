// The overlays' one DOM helper.

const SVG = "http://www.w3.org/2000/svg";

/** A new SVG element `name` with `attributes` set, numbers written as they print. */
export function element<K extends keyof SVGElementTagNameMap>(
  name: K,
  attributes: Record<string, string | number> = {},
): SVGElementTagNameMap[K] {
  const node = document.createElementNS(SVG, name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, String(value));
  return node;
}
