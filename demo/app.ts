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
const ISSUES = `${REPOSITORY}/issues/new`;

/** The element `id`, which the page must hold as a `kind`. */
function $<T extends Element>(id: string, kind: abstract new () => T): T {
  const node = document.getElementById(id);
  if (!(node instanceof kind)) throw new Error(`#${id} is missing or not a ${kind.name}`);
  return node;
}

const ui = {
  fixture: $("fixture", HTMLSelectElement),
  fileOption: $("file-option", HTMLOptionElement),
  open: $("open", HTMLButtonElement),
  file: $("file", HTMLInputElement),
  prev: $("prev", HTMLButtonElement),
  next: $("next", HTMLButtonElement),
  page: $("page", HTMLInputElement),
  total: $("total", HTMLOutputElement),
  banners: $("banners", HTMLElement),
  dropzone: $("dropzone", HTMLElement),
  viewer: $("viewer", HTMLDivElement),
  canvas: $("canvas", HTMLCanvasElement),
  overlay: $("overlay", SVGSVGElement),
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
  runBytes: $("run-bytes", HTMLElement),
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
}

export const state: State = {
  name: null,
  bytes: null,
  analysis: null,
  pdf: null,
  pageNumber: 1,
  numPages: 0,
  viewport: null,
  pinned: null,
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
// The pdf.js loading task of the current document: destroying it frees its worker side.
let loadingTask: PDFDocumentLoadingTask | null = null;
let renderTask: RenderTask | null = null;
// Bumped by every render call, so that only the latest one draws.
let renderGeneration = 0;

await init();

/** Analyse and open `bytes`, then show page 1. Later loads supersede earlier ones. */
async function load(name: string, bytes: Uint8Array): Promise<void> {
  const token = ++loadToken;

  void loadingTask?.destroy();
  Object.assign(state, {
    name,
    bytes,
    analysis: analyze_pdf(bytes),
    pdf: null,
    pageNumber: 1,
    numPages: 0,
    viewport: null,
  } satisfies Partial<State>);
  showSource(name);

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
  await load(name, new Uint8Array(await response.arrayBuffer()));
}

async function loadFile(file: File): Promise<void> {
  await load(file.name, new Uint8Array(await file.arrayBuffer()));
}

/** Show page `n`, clamped to the pages pdf.js found. */
async function show(n: number, pdfError: unknown = null): Promise<void> {
  state.pageNumber = Math.min(Math.max(1, n), Math.max(1, state.numPages));
  state.pinned = null;
  hoveredRun = null;
  highlightedObject = null;
  updatePager();
  showBanners(livrePage(), pdfError === null ? null : `pdf.js could not open this file: ${message(pdfError)}`);
  showJson();
  showInspector();
  await render();
}

/** Name the shown document in the sample select, through a stand-in option for a file. */
function showSource(name: string): void {
  const fixture = [...ui.fixture.options].find((option) => option !== ui.fileOption && option.value === name);
  ui.fileOption.hidden = Boolean(fixture);
  if (fixture) {
    ui.fixture.value = name;
  } else {
    ui.fileOption.textContent = name;
    ui.fileOption.selected = true;
  }
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
  const scale = ui.viewer.clientWidth / unscaled.width;
  const viewport = page.getViewport({ scale });
  const dpr = window.devicePixelRatio || 1;

  const { canvas, overlay } = ui;
  canvas.width = Math.floor(viewport.width * dpr);
  canvas.height = Math.floor(viewport.height * dpr);
  canvas.style.width = `${viewport.width}px`;
  canvas.style.height = `${viewport.height}px`;
  overlay.setAttribute("width", String(viewport.width));
  overlay.setAttribute("height", String(viewport.height));
  overlay.setAttribute("viewBox", `0 0 ${viewport.width} ${viewport.height}`);
  state.viewport = viewport;

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

  drawOverlays({ pdf, pageNumber, viewport });
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
      item.className =
        "flex items-baseline gap-3 rounded-lg px-2.5 py-1.5 hover:bg-zinc-100 dark:hover:bg-white/5";
      item.dataset.object = String(index);

      const number = document.createElement("span");
      number.className = "w-8 shrink-0 text-right font-mono text-xs text-zinc-400 tabular-nums dark:text-zinc-600";
      number.textContent = String(index + 1);

      const text = document.createElement("span");
      text.className = "min-w-0 flex-1 truncate font-mono text-xs text-zinc-800 dark:text-zinc-200";
      const [first] = object.runs;
      text.textContent = first ? JSON.stringify(first.text) : "(no runs)";

      const count = document.createElement("span");
      count.className = "shrink-0 text-xs text-zinc-400 dark:text-zinc-500";
      count.textContent = `${object.runs.length} run${object.runs.length === 1 ? "" : "s"}`;

      item.append(number, text, count);
      return item;
    }),
  );

  showRun(null);
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
  ui.runFont.textContent = `${run.font || "(none)"} · ${round(run.size)}`;
  ui.runOrigin.textContent = `(${round(run.x)}, ${round(run.y)})`;
  ui.runBytes.textContent = run.bytes.match(/../g)?.join(" ") ?? "";
}

function markActiveRun(indices: RunIndices | null): void {
  for (const marker of ui.overlay.querySelectorAll("g.livre .run.active")) marker.classList.remove("active");
  if (!indices) return;
  ui.overlay
    .querySelector(`g.livre .run[data-object="${indices.object}"][data-run="${indices.run}"]`)
    ?.classList.add("active");
}

function highlightHull(object: number | null): void {
  highlightedObject = object;
  for (const hull of ui.overlay.querySelectorAll("g.livre .hull.highlight")) hull.classList.remove("highlight");
  if (object === null) return;
  ui.overlay.querySelector(`g.livre .hull[data-object="${object}"]`)?.classList.add("highlight");
}

/** The indices of the marker `target` belongs to, if any. */
function runIndices(target: EventTarget | null): RunIndices | null {
  if (!(target instanceof Element)) return null;
  const marker = target.closest<SVGGElement>("g.livre .run");
  if (!marker) return null;
  return { object: Number(marker.dataset.object), run: Number(marker.dataset.run) };
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

ui.fixture.addEventListener("change", () => loadFixture(ui.fixture.value));
ui.open.addEventListener("click", () => ui.file.click());
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
  if (!indices) return;
  state.pinned = samePlace(indices, state.pinned) ? null : indices;
  showRun(indices);
  selectTab($("tab-run", HTMLButtonElement));
});

ui.objects.addEventListener("pointerover", (event) => {
  if (!(event.target instanceof Element)) return;
  const item = event.target.closest<HTMLLIElement>("li[data-object]");
  if (item) highlightHull(Number(item.dataset.object));
});
ui.objects.addEventListener("pointerleave", () => highlightHull(null));

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

let resizeTimer: ReturnType<typeof setTimeout> | undefined;
window.addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(render, 150);
});

void showVersion();
await loadFixture(ui.fixture.value);
