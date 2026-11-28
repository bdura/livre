// livre's demo page: livre (as wasm) analyses the whole document once, pdf.js renders
// the current page underneath, and the overlay <svg> sits on top of the canvas.

import init, { analyze_pdf, type Analysis, type Page, type StageError } from "./pkg/livre_wasm.js";
import type { PageViewport, PDFDocumentLoadingTask, PDFDocumentProxy, PdfJs, RenderTask } from "./pdfjs.js";
import { drawLivreLayer } from "./overlay-livre.js";
import { drawGroundTruthLayer } from "./overlay-pdfjs.js";

// Pinned: pdf.js changes its API across majors. The worker must be the same version, and
// `pdfjs.d.ts` describes this one.
const PDFJS = "https://cdn.jsdelivr.net/npm/pdfjs-dist@6.5.438/build";
const pdfjs: PdfJs = await import(`${PDFJS}/pdf.min.mjs`);
pdfjs.GlobalWorkerOptions.workerSrc = `${PDFJS}/pdf.worker.min.mjs`;

const REPOSITORY = "https://github.com/bdura/livre";
// Zoom multiplies the fit-to-width scale; the buttons step through these factors.
const ZOOM_STEPS = [0.5, 0.67, 0.8, 1, 1.25, 1.5, 2, 2.5, 3, 4];
const MIN_ZOOM = 0.5;
const MAX_ZOOM = 4;
// The widest a page is drawn at 100%.
const FIT_MAX_WIDTH = 768;
// Safari draws nothing on a larger canvas: past it, the page renders at a lower resolution.
const MAX_CANVAS_PIXELS = 16_000_000;
const ISSUES = `${REPOSITORY}/issues/new`;

/** The element `id`, which the page must hold as a `kind`. */
function $<T extends Element>(id: string, kind: abstract new () => T): T {
  const node = document.getElementById(id);
  if (!(node instanceof kind)) throw new Error(`#${id} is missing or not a ${kind.name}`);
  return node;
}

const ui = {
  fixture: $("fixture", HTMLSelectElement),
  opened: $("opened", HTMLOptGroupElement),
  open: $("open", HTMLButtonElement),
  file: $("file", HTMLInputElement),
  urlForm: $("url-form", HTMLFormElement),
  url: $("url", HTMLInputElement),
  urlLoad: $("url-load", HTMLButtonElement),
  prev: $("prev", HTMLButtonElement),
  next: $("next", HTMLButtonElement),
  page: $("page", HTMLInputElement),
  total: $("total", HTMLOutputElement),
  banners: $("banners", HTMLElement),
  dropzone: $("dropzone", HTMLElement),
  viewer: $("viewer", HTMLDivElement),
  canvas: $("canvas", HTMLCanvasElement),
  overlay: $("overlay", SVGSVGElement),
  zoomOut: $("zoom-out", HTMLButtonElement),
  zoomReset: $("zoom-reset", HTMLButtonElement),
  zoomIn: $("zoom-in", HTMLButtonElement),
  json: $("json", HTMLPreElement),
  version: $("version", HTMLSpanElement),
  layerLivre: $("layer-livre", HTMLInputElement),
  layerPdfjs: $("layer-pdfjs", HTMLInputElement),
  tabs: [...document.querySelectorAll<HTMLButtonElement>('[role="tab"]')],
  objects: $("objects", HTMLOListElement),
  objectsCount: $("objects-count", HTMLSpanElement),
  runEmpty: $("run-empty", HTMLParagraphElement),
  runDetails: $("run-details", HTMLDivElement),
  runText: $("run-text", HTMLDivElement),
  runPinned: $("run-pinned", HTMLSpanElement),
  runFont: $("run-font", HTMLElement),
  runOrigin: $("run-origin", HTMLElement),
  runEnd: $("run-end", HTMLElement),
  runBytes: $("run-bytes", HTMLElement),
  runObject: $("run-object", HTMLButtonElement),
  runPlace: $("run-place", HTMLSpanElement),
  countRuns: $("count-runs", HTMLDivElement),
  countObjects: $("count-objects", HTMLDivElement),
  countItems: $("count-items", HTMLDivElement),
};

/** A run by its indices in `page.text_objects` and in that object's `runs`. */
interface RunIndices {
  object: number;
  run: number;
}

/** Everything the page knows about the loaded document. */
interface State {
  name: string | null;
  /** The document's value in the menu. */
  key: string | null;
  /** The link the document was fetched from, null for a sample or a local file. */
  link: string | null;
  bytes: Uint8Array | null;
  /** livre's analysis; its `pages` may be shorter than the document on failure. */
  analysis: Analysis | null;
  /** Null if pdf.js could not open the file. */
  pdf: PDFDocumentProxy | null;
  /** One-based. */
  pageNumber: number;
  numPages: number;
  /**
   * The viewport of the page on screen: overlays map PDF user space to the <svg> with
   * `viewport.convertToViewportPoint(x, y)`.
   */
  viewport: PageViewport | null;
  /** The run the inspector keeps showing. */
  pinned: RunIndices | null;
  /** A factor of the fit-to-width scale. */
  zoom: number;
}

export const state: State = {
  name: null,
  key: null,
  link: null,
  bytes: null,
  analysis: null,
  pdf: null,
  pageNumber: 1,
  numPages: 0,
  viewport: null,
  pinned: null,
  zoom: 1,
};

/** The rendered page an overlay draws over. */
export interface Sheet {
  pdf: PDFDocumentProxy;
  pageNumber: number;
  viewport: PageViewport;
}

/**
 * Functions drawing into the overlay after each render, on an emptied <svg> whose
 * `viewBox` matches `sheet.viewport`.
 */
export const overlays: ((svg: SVGSVGElement, sheet: Sheet) => void | Promise<void>)[] = [];

// With no DTO for the page (livre failed earlier), draw nothing: the banner explains why.
overlays.push((svg, { viewport }) => {
  const page = livrePage();
  if (!page) return;
  drawLivreLayer(svg, page, viewport);
  // The markers are new elements, after a resize say: mark them again.
  markActiveRun(hoveredRun ?? state.pinned);
  highlightHull(highlightedObject);
});

// Drawn whatever livre made of the page: it is the reference livre is judged against.
overlays.push(async (svg, { pdf, pageNumber, viewport }) => {
  // Each render makes a new viewport: a stale one means a later render owns the <svg>.
  const stale = () => viewport !== state.viewport;
  try {
    const page = await pdf.getPage(pageNumber);
    if (stale()) return;
    const layer = await drawGroundTruthLayer(svg, page, viewport, () => !stale());
    if (layer) ui.countItems.textContent = String(layer.childElementCount);
  } catch (e) {
    // A load destroys the previous document mid-fetch; only a current failure matters.
    if (!stale()) throw e;
  }
});

// The run under the pointer and the text object hovered in the inspector's list.
let hoveredRun: RunIndices | null = null;
let highlightedObject: number | null = null;
let loadToken = 0;

/** A file or link the visitor opened, kept for the menu. */
interface Opened {
  name: string;
  link: string | null;
  bytes: Uint8Array;
}
// Each keeps its bytes in memory: a few, not the whole session.
const MAX_OPENED = 8;
// By menu value, oldest first.
const opened = new Map<string, Opened>();
// The pdf.js loading task of the current document: destroying it frees its worker side.
let loadingTask: PDFDocumentLoadingTask | null = null;
let renderTask: RenderTask | null = null;
// Bumped by every render call, so that only the latest one draws.
let renderGeneration = 0;

await init();

/**
 * Analyse and open `bytes`, then show page 1. They are a `sample`, or a document fetched
 * from `link` if not null, or a local file; the last two join the menu. Later loads
 * supersede earlier ones.
 */
async function load(
  name: string,
  bytes: Uint8Array,
  { sample = false, link = null }: { sample?: boolean; link?: string | null } = {},
): Promise<void> {
  const token = ++loadToken;

  const key = sample ? name : link !== null ? `link:${link}` : `file:${name}`;
  if (!sample) remember(key, { name, link, bytes });

  void loadingTask?.destroy();
  Object.assign(state, {
    name,
    key,
    link,
    bytes,
    analysis: analyze_pdf(bytes),
    pdf: null,
    pageNumber: 1,
    numPages: 0,
    viewport: null,
    zoom: 1,
  } satisfies Partial<State>);
  updateZoom();
  showSource();

  let pdfError: unknown = null;
  try {
    // pdf.js transfers the buffer it is given to its worker: hand it a copy.
    const task = pdfjs.getDocument({
      data: bytes.slice(),
      // Font program warnings are pdf.js internals, not something the visitor can act on.
      verbosity: pdfjs.VerbosityLevel.ERRORS,
    });
    loadingTask = task;
    const pdf = await task.promise;
    if (token !== loadToken) return;
    state.pdf = pdf;
    state.numPages = pdf.numPages;
  } catch (e) {
    if (token !== loadToken) return;
    pdfError = e;
  }

  await show(1, pdfError);
}

async function loadFixture(name: string): Promise<void> {
  const response = await fetch(`./fixtures/${encodeURIComponent(name)}`);
  if (!response.ok) {
    showBanners(null, `could not fetch fixtures/${name}: HTTP ${response.status}`);
    return;
  }
  await load(name, new Uint8Array(await response.arrayBuffer()), { sample: true });
}

async function loadFile(file: File): Promise<void> {
  await load(file.name, new Uint8Array(await file.arrayBuffer()));
}

/** Open the menu entry `key`: a document opened earlier, from memory, or else a sample. */
async function loadEntry(key: string): Promise<void> {
  const entry = opened.get(key);
  if (entry) await load(entry.name, entry.bytes, { link: entry.link });
  else await loadFixture(key);
}

/**
 * Keep an opened file or link in the menu, so that the visitor can come back to it.
 * Opening it again replaces the entry in place; past `MAX_OPENED`, the oldest one is dropped.
 */
function remember(key: string, entry: Opened): void {
  opened.set(key, entry);
  for (const old of opened.keys()) {
    if (opened.size <= MAX_OPENED) break;
    opened.delete(old);
  }

  ui.opened.replaceChildren(
    ...[...opened].map(([value, { name, link }]) => {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = link === null ? name : `${name} · ${new URL(link, window.location.href).hostname}`;
      if (link !== null) option.title = link;
      return option;
    }),
  );
  ui.opened.hidden = false;
}

/**
 * Fetch the PDF at `url` from the browser and open it. Most hosts send no CORS header,
 * which the browser reports as a bare network error: the banner suggests the way around.
 */
async function loadUrl(url: string): Promise<void> {
  const token = loadToken;
  // Something else was opened while the link downloaded: it wins.
  const superseded = () => token !== loadToken;
  ui.urlForm.setAttribute("aria-busy", "true");
  syncLinkButton();
  try {
    let response: Response;
    try {
      response = await fetch(downloadUrl(url));
    } catch {
      if (!superseded()) {
        showBanners(
          null,
          `could not fetch ${url}: the host is unreachable or does not allow cross-origin downloads. Download the PDF and open it instead.`,
        );
      }
      return;
    }
    if (!response.ok) {
      if (!superseded()) showBanners(null, `could not fetch ${url}: HTTP ${response.status}`);
      return;
    }
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (superseded()) return;
    await load(linkName(url), bytes, { link: url });
  } finally {
    ui.urlForm.removeAttribute("aria-busy");
    syncLinkButton();
  }
}

/**
 * Where to download a link from. github.com sends no CORS header, but its raw host does:
 * a file page (`/owner/repo/blob/ref/path`, or `/raw/` in its place) maps onto it.
 * Anything else is fetched as given.
 */
function downloadUrl(url: string): string {
  let link: URL;
  try {
    link = new URL(url, window.location.href);
  } catch {
    return url;
  }
  const [, owner, repo, kind, ...rest] = link.pathname.split("/");
  const github = link.hostname === "github.com" || link.hostname === "www.github.com";
  if (!github ||(kind !== "blob" && kind !== "raw") || rest.length < 2) return url;
  return `https://raw.githubusercontent.com/${owner}/${repo}/${rest.join("/")}`;
}

/** The file name a link points to, or its host when the path names none. */
function linkName(url: string): string {
  const { hostname, pathname } = new URL(url, window.location.href);
  const last = pathname.split("/").findLast(Boolean);
  if (!last) return hostname;
  try {
    return decodeURIComponent(last);
  } catch {
    return last;
  }
}

/** Keep `?url=` in the address bar in step with the document, so a linked PDF can be shared. */
function setLinkParam(url: string | null): void {
  const address = new URL(window.location.href);
  if (url === null) address.searchParams.delete("url");
  else address.searchParams.set("url", url);
  window.history.replaceState(null, "", address);
}

/** Show page `n`, clamped to the pages pdf.js found. */
async function show(n: number, pdfError: unknown = null): Promise<void> {
  state.pageNumber = Math.min(Math.max(1, n), Math.max(1, state.numPages));
  state.pinned = null;
  hoveredRun = null;
  highlightedObject = null;
  updatePager();
  ui.viewer.scrollTo(0, 0);
  showBanners(livrePage(), pdfError === null ? null : `pdf.js could not open this file: ${message(pdfError)}`);
  showJson();
  showInspector();
  await render();
}

/**
 * Select the shown document in the menu. The link field and `?url=` hold the link of the
 * shown document, and are emptied for any other.
 */
function showSource(): void {
  const { key, link } = state;
  if (key !== null) ui.fixture.value = key;
  ui.fixture.title = link ?? "";
  ui.url.value = link ?? "";
  setLinkParam(link);
  syncLinkButton();
}

/**
 * The link button says what submitting would do: nothing while a download runs or when
 * the field holds the link already shown, so it reads "Shown" then.
 */
function syncLinkButton(): void {
  const busy = ui.urlForm.hasAttribute("aria-busy");
  const shown = state.link !== null && ui.url.value.trim() === state.link;
  ui.urlLoad.disabled = busy || shown;
  ui.urlLoad.textContent = busy ? "Loading…" : shown ? "Shown" : "Load";
}

/** livre's DTO for the current page, if livre got that far. */
function livrePage(): Page | null {
  return state.analysis?.pages[state.pageNumber - 1] ?? null;
}

function updatePager(): void {
  const { pageNumber, numPages } = state;
  ui.total.value = String(numPages);
  ui.page.max = String(numPages);
  ui.page.value = String(pageNumber);
  ui.page.disabled = numPages === 0;
  ui.prev.disabled = pageNumber <= 1;
  ui.next.disabled = pageNumber >= numPages;
}

async function render(): Promise<void> {
  const generation = ++renderGeneration;
  renderTask?.cancel();
  renderTask = null;

  const { pdf, pageNumber } = state;
  if (!pdf) {
    clearSheet();
    return;
  }

  // A later render, navigation or load took over; a load also destroys the old document.
  const superseded = () => generation !== renderGeneration || pdf !== state.pdf;

  let page;
  try {
    page = await pdf.getPage(pageNumber);
  } catch (e) {
    if (superseded()) return;
    throw e;
  }
  if (superseded()) return;

  const unscaled = page.getViewport({ scale: 1 });
  const viewport = page.getViewport({ scale: sheetScale(unscaled.width) });
  const dpr = Math.min(
    window.devicePixelRatio || 1,
    Math.sqrt(MAX_CANVAS_PIXELS / (viewport.width * viewport.height)),
  );

  // Drawn off-screen and swapped in, so a zoomed page never blanks while it renders.
  const canvas = document.createElement("canvas");
  canvas.id = ui.canvas.id;
  canvas.className = ui.canvas.className;
  canvas.width = Math.floor(viewport.width * dpr);
  canvas.height = Math.floor(viewport.height * dpr);
  canvas.style.width = `${viewport.width}px`;
  canvas.style.height = `${viewport.height}px`;

  const task = page.render({
    canvas,
    viewport,
    transform: dpr === 1 ? undefined : [dpr, 0, 0, dpr, 0, 0],
  });
  renderTask = task;
  try {
    await task.promise;
  } catch (e) {
    if ((e instanceof Error && e.name === "RenderingCancelledException") || superseded()) return;
    throw e;
  }
  if (renderTask === task) renderTask = null;
  if (superseded()) return;

  ui.canvas.replaceWith(canvas);
  ui.canvas = canvas;
  const { overlay } = ui;
  overlay.setAttribute("width", String(viewport.width));
  overlay.setAttribute("height", String(viewport.height));
  overlay.setAttribute("viewBox", `0 0 ${viewport.width} ${viewport.height}`);
  state.viewport = viewport;

  drawOverlays({ pdf, pageNumber, viewport });
}

/** The scale of a page `width` units wide: the viewer's width, at most `FIT_MAX_WIDTH`, times the zoom. */
function sheetScale(width: number): number {
  const style = getComputedStyle(ui.viewer);
  const room = ui.viewer.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
  return (Math.min(room, FIT_MAX_WIDTH) / width) * state.zoom;
}

/**
 * Zoom to `target`, keeping the point at `anchor` (client coordinates) in place, or the
 * middle of the viewer. The page is stretched at once and rendered again once zooming stops.
 */
function zoomTo(target: number, anchor?: { x: number; y: number }): void {
  const zoom = Math.min(Math.max(target, MIN_ZOOM), MAX_ZOOM);
  if (zoom === state.zoom) return;
  state.zoom = zoom;
  updateZoom();
  const viewport = state.viewport;
  if (!viewport) return;
  // A render under way is for the old zoom.
  renderGeneration++;
  renderTask?.cancel();

  const box = ui.viewer.getBoundingClientRect();
  const x = anchor?.x ?? box.left + box.width / 2;
  const y = anchor?.y ?? box.top + box.height / 2;
  const before = ui.canvas.getBoundingClientRect();

  // The page's size in PDF units, whatever the scale it was last rendered at.
  const scale = sheetScale(viewport.width / viewport.scale);
  const width = (viewport.width / viewport.scale) * scale;
  const height = (viewport.height / viewport.scale) * scale;
  ui.canvas.style.width = `${width}px`;
  ui.canvas.style.height = `${height}px`;
  // The viewBox stays: the drawn overlay stretches with the page until it is redrawn.
  ui.overlay.setAttribute("width", String(width));
  ui.overlay.setAttribute("height", String(height));

  const after = ui.canvas.getBoundingClientRect();
  const ratio = after.width / before.width;
  ui.viewer.scrollBy(after.left + (x - before.left) * ratio - x, after.top + (y - before.top) * ratio - y);
  scheduleRender();
}

function updateZoom(): void {
  ui.zoomReset.textContent = `${Math.round(state.zoom * 100)}%`;
  ui.zoomOut.disabled = state.zoom <= MIN_ZOOM;
  ui.zoomIn.disabled = state.zoom >= MAX_ZOOM;
}

let renderTimer: ReturnType<typeof setTimeout> | undefined;

/** Render once the resizing or zooming that calls this has stopped. */
function scheduleRender(): void {
  clearTimeout(renderTimer);
  renderTimer = setTimeout(render, 150);
}

function clearSheet(): void {
  state.viewport = null;
  const { canvas, overlay } = ui;
  canvas.width = canvas.height = 0;
  canvas.style.width = canvas.style.height = "0";
  overlay.removeAttribute("viewBox");
  overlay.replaceChildren();
}

function drawOverlays(sheet: Sheet): void {
  ui.overlay.replaceChildren();
  for (const draw of overlays) void draw(ui.overlay, sheet);
}

/**
 * Rebuild the banner area: livre's document-level failure, the current page's
 * failure, and `other`, a message about a failure outside livre.
 */
function showBanners(page: Page | null, other: string | null): void {
  const banners: HTMLElement[] = [];
  if (state.analysis?.document) banners.push(stageBanner(state.analysis.document, null));
  if (page?.error) banners.push(stageBanner(page.error, page));
  ui.banners.replaceChildren(...banners, ...(other ? [plainBanner(other)] : []));
}

function stageBanner({ stage, message }: StageError, page: Page | null): HTMLElement {
  const where = page ? ` on page ${page.index + 1}` : "";
  const report = [
    `Stage: ${stage}${where}`,
    `File: ${state.name}`,
    `livre: ${state.analysis?.version}`,
    "",
    "```",
    message,
    "```",
  ].join("\n");

  const copy = document.createElement("button");
  copy.type = "button";
  copy.className = "strip-action";
  copy.textContent = "Copy details";
  copy.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(report);
      copy.textContent = "Copied";
    } catch {
      copy.textContent = "Copy failed";
    }
  });

  const issue = document.createElement("a");
  const issueTitle = `${stage} stage fails on ${state.name}`;
  issue.href = `${ISSUES}?title=${encodeURIComponent(issueTitle)}&body=${encodeURIComponent(report)}`;
  issue.target = "_blank";
  issue.rel = "noopener";
  issue.className = "strip-action";
  issue.textContent = "Open an issue ↗";

  const title = document.createElement("strong");
  title.className = "font-medium text-amber-950 dark:text-amber-100";
  const code = document.createElement("code");
  code.className = "rounded bg-amber-500/15 px-1 font-mono";
  code.textContent = stage;
  title.append("livre failed at the ", code, ` stage${where}.`);

  const pre = document.createElement("pre");
  pre.className = "mt-1 max-h-24 overflow-auto font-mono text-xs break-all whitespace-pre-wrap text-amber-900/70 dark:text-amber-200/60";
  pre.textContent = message;

  return strip([title, pre], [copy, issue]);
}

function plainBanner(text: string): HTMLElement {
  const title = document.createElement("strong");
  title.className = "font-medium text-amber-950 dark:text-amber-100";
  title.textContent = text;
  return strip([title], []);
}

/** An amber strip under the header: a warning sign, `body`, then `actions` on the right. */
function strip(body: HTMLElement[], actions: HTMLElement[]): HTMLElement {
  const banner = document.createElement("div");
  banner.className =
    "flex items-start gap-3 border-b border-amber-300/60 bg-amber-50 px-4 py-2.5 text-amber-900 dark:border-amber-500/20 dark:bg-amber-500/10 dark:text-amber-200";
  banner.setAttribute("role", "alert");

  const sign = document.createElement("span");
  sign.className = "text-amber-500 dark:text-amber-400";
  sign.setAttribute("aria-hidden", "true");
  sign.textContent = "⚠";

  const main = document.createElement("div");
  main.className = "min-w-0 flex-1";
  main.append(...body);

  const side = document.createElement("div");
  side.className = "flex shrink-0 flex-wrap gap-2";
  side.append(...actions);

  banner.append(sign, main, side);
  return banner;
}

function showJson(): void {
  const page = livrePage();
  const shown = page ?? state.analysis?.document ?? { error: "livre found no such page" };
  ui.json.textContent = JSON.stringify(shown, null, 2);
}

/** Refill the inspector for the current page: counts, text object list, run panel. */
function showInspector(): void {
  const page = livrePage();
  const objects = page?.text_objects ?? [];
  const runs = objects.reduce((total, object) => total + object.runs.length, 0);
  // A dash, not 0, when livre never reached the page.
  ui.countRuns.textContent = page ? String(runs) : "–";
  ui.countObjects.textContent = page ? String(objects.length) : "–";
  // Filled in by the pdf.js layer once it has the page's text content.
  ui.countItems.textContent = "–";
  ui.objectsCount.textContent = String(objects.length);

  ui.objects.replaceChildren(
    ...objects.map((object, index) => {
      const item = document.createElement("li");
      item.dataset.object = String(index);

      // Runs are joined as shown: no gap threshold decides yet where a space falls between them.
      const text = object.runs.length ? JSON.stringify(object.runs.map((run) => run.text).join("")) : "(no runs)";
      const count = `${object.runs.length} run${object.runs.length === 1 ? "" : "s"}`;
      const row = listRow(String(index + 1), text, count);
      if (object.runs.length === 0) {
        row.classList.add("pl-6");
        item.append(row);
        return item;
      }

      const details = document.createElement("details");
      details.className = "group";
      const summary = document.createElement("summary");
      summary.className = "cursor-pointer list-none [&::-webkit-details-marker]:hidden";
      const chevron = document.createElement("span");
      chevron.className = "w-3 shrink-0 text-zinc-400 transition-transform group-open:rotate-90 dark:text-zinc-600";
      chevron.textContent = "›";
      row.prepend(chevron);
      summary.append(row);

      const runs = document.createElement("ol");
      runs.className = "pl-6";
      runs.append(
        ...object.runs.map((run, runIndex) => {
          const line = document.createElement("li");
          const button = document.createElement("button");
          button.type = "button";
          button.className = "block w-full text-left";
          button.dataset.object = String(index);
          button.dataset.run = String(runIndex);
          button.append(listRow(String(runIndex + 1), JSON.stringify(run.text), String(round(run.size))));
          line.append(button);
          return line;
        }),
      );

      details.append(summary, runs);
      item.append(details);
      return item;
    }),
  );

  showRun(null);
}

/** A line of the text object list: a number, some text truncated to fit, a note on the right. */
function listRow(number: string, text: string, note: string): HTMLDivElement {
  const row = document.createElement("div");
  row.className =
    "flex items-baseline gap-3 rounded-lg px-2.5 py-1.5 hover:bg-zinc-100 dark:hover:bg-white/5 [[data-active]_&]:bg-livre/10";

  const index = document.createElement("span");
  index.className = "w-8 shrink-0 text-right font-mono text-xs text-zinc-400 tabular-nums dark:text-zinc-600";
  index.textContent = number;

  const content = document.createElement("span");
  content.className = "min-w-0 flex-1 truncate font-mono text-xs text-zinc-800 dark:text-zinc-200";
  content.textContent = text;

  const aside = document.createElement("span");
  aside.className = "shrink-0 text-xs text-zinc-400 tabular-nums dark:text-zinc-500";
  aside.textContent = note;

  row.append(index, content, aside);
  return row;
}

/** Show the run at `indices` in the Run tab, or the empty state. */
function showRun(indices: RunIndices | null): void {
  const run = indices && livrePage()?.text_objects[indices.object]?.runs[indices.run];
  ui.runEmpty.hidden = Boolean(run);
  ui.runDetails.hidden = !run;
  markActiveRun(run ? indices : null);
  if (!run) return;

  // JSON quoting shows control characters such as the NULs of 2-byte composite strings.
  ui.runText.textContent = JSON.stringify(run.text);
  ui.runPinned.hidden = !samePlace(indices, state.pinned);
  ui.runFont.textContent = run.font_kind
    ? `${run.font} · ${run.base_font ?? "(no /BaseFont)"} (${run.font_kind}) · ${round(run.size)}`
    : "(none)";
  ui.runOrigin.textContent = `(${round(run.start.x)}, ${round(run.start.y)})`;
  ui.runEnd.textContent = `(${round(run.end.x)}, ${round(run.end.y)})`;
  ui.runBytes.textContent = run.bytes.match(/../g)?.join(" ") ?? "";
  const runs = livrePage()?.text_objects[indices.object]?.runs.length ?? 0;
  ui.runObject.textContent = `Object ${indices.object + 1}`;
  ui.runObject.dataset.object = String(indices.object);
  ui.runObject.dataset.run = String(indices.run);
  ui.runPlace.textContent = ` · run ${indices.run + 1} of ${runs}`;
}

/** Mark the run at `indices` both on the page and in the text object list. */
function markActiveRun(indices: RunIndices | null): void {
  for (const marker of ui.overlay.querySelectorAll("g.livre .run.active")) marker.classList.remove("active");
  for (const line of ui.objects.querySelectorAll<HTMLElement>("[data-active]")) delete line.dataset.active;
  if (!indices) return;
  const place = `[data-object="${indices.object}"][data-run="${indices.run}"]`;
  ui.overlay.querySelector(`g.livre .run${place}`)?.classList.add("active");
  const line = ui.objects.querySelector<HTMLElement>(`button${place}`);
  if (line) line.dataset.active = "";
}

/** Highlight the hull and runs of `object` and dim the rest of livre's layer, or undo it. */
function highlightHull(object: number | null): void {
  highlightedObject = object;
  for (const shape of ui.overlay.querySelectorAll("g.livre .highlight")) shape.classList.remove("highlight");
  ui.overlay.classList.toggle("focus-object", object !== null);
  if (object === null) return;
  for (const shape of ui.overlay.querySelectorAll(`g.livre [data-object="${object}"]`)) {
    shape.classList.add("highlight");
  }
}

/** Open the text object of `indices` in its tab, with the run's line in view. */
function revealRun(indices: RunIndices): void {
  selectTab($("tab-objects", HTMLButtonElement));
  const item = ui.objects.querySelector(`li[data-object="${indices.object}"]`);
  const details = item?.querySelector("details");
  if (details) details.open = true;
  const line = item?.querySelector<HTMLButtonElement>(`button[data-run="${indices.run}"]`);
  line?.scrollIntoView({ block: "nearest" });
  line?.focus({ preventScroll: true });
}

/** Pin the run at `indices`, or release it if it is already pinned, and show it. */
function pinRun(indices: RunIndices): void {
  state.pinned = samePlace(indices, state.pinned) ? null : indices;
  showRun(indices);
  selectTab($("tab-run", HTMLButtonElement));
}

/** The indices of the marker `target` belongs to, if any. */
function runIndices(target: EventTarget | null): RunIndices | null {
  if (!(target instanceof Element)) return null;
  const marker = target.closest<SVGGElement>("g.livre .run");
  if (!marker) return null;
  return { object: Number(marker.dataset.object), run: Number(marker.dataset.run) };
}

/** The indices of the run line `target` belongs to in the list, or of the Run tab's link. */
function listRunIndices(target: EventTarget | null): RunIndices | null {
  if (!(target instanceof Element)) return null;
  const line = target.closest<HTMLElement>("[data-object][data-run]");
  if (!line) return null;
  return { object: Number(line.dataset.object), run: Number(line.dataset.run) };
}

function samePlace(a: RunIndices | null, b: RunIndices | null): boolean {
  return Boolean(a && b && a.object === b.object && a.run === b.run);
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function selectTab(selected: HTMLButtonElement): void {
  for (const tab of ui.tabs) {
    const active = tab === selected;
    tab.setAttribute("aria-selected", String(active));
    tab.tabIndex = active ? 0 : -1;
    $(tab.getAttribute("aria-controls") ?? "", HTMLElement).hidden = !active;
  }
}

/** The `version.json` the site is built with. */
interface Version {
  version: string;
  commit: string;
}

async function showVersion(): Promise<void> {
  try {
    const response = await fetch("./version.json");
    if (!response.ok) return;
    const { version, commit }: Version = await response.json();
    // A dirty tree is built from the commit it started at: link to that one.
    const [hash = "", dirty] = commit.split("-");
    if (!/^[0-9a-f]{40}$/.test(hash)) {
      ui.version.textContent = `livre ${version} · ${commit}`;
      return;
    }
    const link = document.createElement("a");
    link.href = `${REPOSITORY}/commit/${hash}`;
    link.className = "hover:text-zinc-700 dark:hover:text-zinc-300";
    link.textContent = `${hash.slice(0, 7)}${dirty ? `-${dirty}` : ""}`;
    ui.version.replaceChildren(`livre ${version} · `, link);
  } catch {
    // No version.json in local development: leave the footer bare.
  }
}

ui.fixture.addEventListener("change", () => loadEntry(ui.fixture.value));
ui.open.addEventListener("click", () => ui.file.click());
ui.urlForm.addEventListener("submit", (event) => {
  event.preventDefault();
  void loadUrl(ui.url.value.trim());
});
ui.url.addEventListener("input", syncLinkButton);
ui.file.addEventListener("change", () => {
  const file = ui.file.files?.[0];
  if (file) void loadFile(file);
  // Opening the same file again must fire `change` again.
  ui.file.value = "";
});
// A class on the <svg>, not on the group, so the choice survives the redraw of each render.
const syncLayers = () => {
  ui.overlay.classList.toggle("hide-livre", !ui.layerLivre.checked);
  ui.overlay.classList.toggle("hide-pdfjs", !ui.layerPdfjs.checked);
};
ui.layerLivre.addEventListener("change", syncLayers);
ui.layerPdfjs.addEventListener("change", syncLayers);
syncLayers();
ui.prev.addEventListener("click", () => show(state.pageNumber - 1));
ui.next.addEventListener("click", () => show(state.pageNumber + 1));
ui.page.addEventListener("change", () => show(Number(ui.page.value) || 1));

// The markers live in the <svg>, rebuilt on each render: listen on the <svg> itself.
ui.overlay.addEventListener("pointerover", (event) => {
  const indices = runIndices(event.target);
  if (!indices) return;
  hoveredRun = indices;
  showRun(indices);
});
ui.overlay.addEventListener("pointerout", (event) => {
  // Moving between the parts of one marker is not leaving it.
  if (!runIndices(event.target) || runIndices(event.relatedTarget ?? document.body)) return;
  hoveredRun = null;
  showRun(state.pinned);
});
ui.overlay.addEventListener("click", (event) => {
  const indices = runIndices(event.target);
  if (indices) pinRun(indices);
});

ui.objects.addEventListener("pointerover", (event) => {
  if (!(event.target instanceof Element)) return;
  const item = event.target.closest<HTMLLIElement>("li[data-object]");
  if (item) highlightHull(Number(item.dataset.object));
  hoveredRun = listRunIndices(event.target);
  showRun(hoveredRun ?? state.pinned);
});
ui.objects.addEventListener("pointerleave", () => {
  highlightHull(null);
  hoveredRun = null;
  showRun(state.pinned);
});
ui.objects.addEventListener("click", (event) => {
  const indices = listRunIndices(event.target);
  if (indices) pinRun(indices);
});
ui.runObject.addEventListener("click", () => {
  const indices = listRunIndices(ui.runObject);
  if (indices) revealRun(indices);
});

for (const tab of ui.tabs) tab.addEventListener("click", () => selectTab(tab));
// Arrow keys move between tabs, as the WAI-ARIA tabs pattern expects.
ui.tabs[0]?.parentElement?.addEventListener("keydown", (event) => {
  const step = ({ ArrowRight: 1, ArrowLeft: -1 } as Record<string, number>)[event.key];
  if (!step) return;
  const current = ui.tabs.findIndex((tab) => tab === document.activeElement);
  const next = ui.tabs[(current + step + ui.tabs.length) % ui.tabs.length];
  if (!next) return;
  selectTab(next);
  next.focus();
});

ui.dropzone.addEventListener("dragover", (event) => {
  event.preventDefault();
  ui.dropzone.dataset.dragging = "";
});
ui.dropzone.addEventListener("dragleave", (event) => {
  // `dragleave` also fires when entering a child: only leaving the zone counts.
  if (!(event.relatedTarget instanceof Node && ui.dropzone.contains(event.relatedTarget))) {
    delete ui.dropzone.dataset.dragging;
  }
});
ui.dropzone.addEventListener("drop", (event) => {
  event.preventDefault();
  delete ui.dropzone.dataset.dragging;
  const file = event.dataTransfer?.files[0];
  if (file) void loadFile(file);
});

window.addEventListener("resize", scheduleRender);

ui.zoomIn.addEventListener("click", () => zoomTo(ZOOM_STEPS.find((step) => step > state.zoom + 0.001) ?? MAX_ZOOM));
ui.zoomOut.addEventListener("click", () =>
  zoomTo(ZOOM_STEPS.findLast((step) => step < state.zoom - 0.001) ?? MIN_ZOOM),
);
ui.zoomReset.addEventListener("click", () => zoomTo(1));
// A trackpad pinch arrives as a wheel event with `ctrlKey` set.
ui.viewer.addEventListener(
  "wheel",
  (event) => {
    if (!event.ctrlKey && !event.metaKey) return;
    // Over the page, the browser's own zoom would scale the whole interface.
    event.preventDefault();
    const lines = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16 : 1;
    const delta = Math.min(Math.max(event.deltaY * lines, -50), 50);
    zoomTo(state.zoom * Math.exp(-delta / 100), { x: event.clientX, y: event.clientY });
  },
  { passive: false },
);

void showVersion();
// `?url=` opens a linked PDF, so that a shared address shows the same document.
const linked = new URLSearchParams(window.location.search).get("url");
if (linked) {
  // Shown while it downloads, and left there to correct if it fails.
  ui.url.value = linked;
  await loadUrl(linked);
}
else await loadFixture(ui.fixture.value);
