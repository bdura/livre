// livre's demo page: livre (as wasm) analyses the whole document once, pdf.js renders
// the current page underneath, and the overlay <svg> sits on top of the canvas.

import init, { analyze_pdf } from "./pkg/livre_wasm.js";
import { drawLivreLayer } from "./overlay-livre.js";

// Pinned: pdf.js changes its API across majors. The worker must be the same version.
const PDFJS = "https://cdn.jsdelivr.net/npm/pdfjs-dist@6.5.438/build";
const pdfjs = await import(`${PDFJS}/pdf.min.mjs`);
pdfjs.GlobalWorkerOptions.workerSrc = `${PDFJS}/pdf.worker.min.mjs`;

const ISSUES = "https://github.com/bdura/livre/issues/new";

const $ = (id) => document.getElementById(id);
const ui = {
  fixture: $("fixture"),
  file: $("file"),
  prev: $("prev"),
  next: $("next"),
  page: $("page"),
  total: $("total"),
  filename: $("filename"),
  banners: $("banners"),
  dropzone: $("dropzone"),
  viewer: $("viewer"),
  canvas: $("canvas"),
  overlay: $("overlay"),
  json: $("json"),
  version: $("version"),
  layerLivre: $("layer-livre"),
};

/**
 * Everything the page knows about the loaded document.
 *
 * `viewport` is the pdf.js `PageViewport` of the page on screen: overlays map PDF user
 * space to the <svg> with `viewport.convertToViewportPoint(x, y)`.
 */
export const state = {
  name: null,
  bytes: null,
  /** livre's `Analysis`; its `pages` may be shorter than the document on failure. */
  analysis: null,
  /** The pdf.js `PDFDocumentProxy`, or null if pdf.js could not open the file. */
  pdf: null,
  /** One-based. */
  pageNumber: 1,
  numPages: 0,
  viewport: null,
};

/**
 * Functions drawing into the overlay after each render, called as `draw(svg, state)`
 * on an emptied <svg> whose `viewBox` matches `state.viewport`.
 */
export const overlays = [];

// With no DTO for the page (livre failed earlier), draw nothing: the banner explains why.
overlays.push((svg, { viewport }) => {
  const page = livrePage();
  if (page) drawLivreLayer(svg, page, viewport);
});

let loadToken = 0;
// The pdf.js loading task of the current document: destroying it frees its worker side.
let loadingTask = null;
let renderTask = null;
// Bumped by every render call, so that only the latest one draws.
let renderGeneration = 0;

await init();

/** Analyse and open `bytes`, then show page 1. Later loads supersede earlier ones. */
async function load(name, bytes) {
  const token = ++loadToken;

  loadingTask?.destroy();
  Object.assign(state, {
    name,
    bytes,
    analysis: analyze_pdf(bytes),
    pdf: null,
    pageNumber: 1,
    numPages: 0,
    viewport: null,
  });
  ui.filename.textContent = name;

  let pdfError = null;
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

async function loadFixture(name) {
  const response = await fetch(`./fixtures/${encodeURIComponent(name)}`);
  if (!response.ok) {
    showBanners(null, `could not fetch fixtures/${name}: HTTP ${response.status}`);
    return;
  }
  await load(name, new Uint8Array(await response.arrayBuffer()));
}

async function loadFile(file) {
  await load(file.name, new Uint8Array(await file.arrayBuffer()));
}

/** Show page `n`, clamped to the pages pdf.js found. */
async function show(n, pdfError = null) {
  state.pageNumber = Math.min(Math.max(1, n), Math.max(1, state.numPages));
  updatePager();
  showBanners(livrePage(), pdfError && `pdf.js could not open this file: ${pdfError.message}`);
  showJson();
  await render();
}

/** livre's DTO for the current page, if livre got that far. */
function livrePage() {
  return state.analysis?.pages[state.pageNumber - 1] ?? null;
}

function updatePager() {
  const { pageNumber, numPages } = state;
  ui.total.value = numPages;
  ui.page.max = numPages;
  ui.page.value = pageNumber;
  ui.page.disabled = numPages === 0;
  ui.prev.disabled = pageNumber <= 1;
  ui.next.disabled = pageNumber >= numPages;
}

async function render() {
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
  overlay.setAttribute("width", viewport.width);
  overlay.setAttribute("height", viewport.height);
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
    if (e?.name === "RenderingCancelledException" || superseded()) return;
    throw e;
  }
  if (renderTask === task) renderTask = null;

  drawOverlays();
}

function clearSheet() {
  state.viewport = null;
  const { canvas, overlay } = ui;
  canvas.width = canvas.height = 0;
  canvas.style.width = canvas.style.height = "0";
  overlay.removeAttribute("viewBox");
  overlay.replaceChildren();
}

function drawOverlays() {
  ui.overlay.replaceChildren();
  for (const draw of overlays) draw(ui.overlay, state);
}

/**
 * Rebuild the banner area: livre's document-level failure, the current page's
 * failure, and `other`, a message about a failure outside livre.
 */
function showBanners(page, other) {
  const banners = [];
  if (state.analysis?.document) banners.push(stageBanner(state.analysis.document, null));
  if (page?.error) banners.push(stageBanner(page.error, page));
  ui.banners.replaceChildren(
    ...banners,
    ...(other ? [plainBanner(other)] : []),
  );
}

function stageBanner({ stage, message }, page) {
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

  const banner = document.createElement("div");
  banner.className = "banner";
  banner.setAttribute("role", "alert");

  const title = document.createElement("strong");
  title.textContent = `livre failed at the ${stage} stage${where}`;
  const pre = document.createElement("pre");
  pre.textContent = message;

  const copy = document.createElement("button");
  copy.type = "button";
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
  issue.textContent = "Open an issue";

  const actions = document.createElement("div");
  actions.className = "actions";
  actions.append(copy, issue);
  banner.append(title, pre, actions);
  return banner;
}

function plainBanner(text) {
  const banner = document.createElement("div");
  banner.className = "banner";
  banner.textContent = text;
  return banner;
}

function showJson() {
  const page = livrePage();
  const shown = page ?? state.analysis?.document ?? { error: "livre found no such page" };
  ui.json.textContent = JSON.stringify(shown, null, 2);
}

async function showVersion() {
  try {
    const response = await fetch("./version.json");
    if (!response.ok) return;
    const { version, commit } = await response.json();
    ui.version.textContent = `livre ${version} · ${commit}`;
  } catch {
    // No version.json in local development: leave the footer bare.
  }
}

ui.fixture.addEventListener("change", () => loadFixture(ui.fixture.value));
ui.file.addEventListener("change", () => {
  const [file] = ui.file.files;
  if (file) loadFile(file);
});
// A class on the <svg>, not on the group, so the choice survives the redraw of each render.
const syncLayers = () => ui.overlay.classList.toggle("hide-livre", !ui.layerLivre.checked);
ui.layerLivre.addEventListener("change", syncLayers);
syncLayers();
ui.prev.addEventListener("click", () => show(state.pageNumber - 1));
ui.next.addEventListener("click", () => show(state.pageNumber + 1));
ui.page.addEventListener("change", () => show(Number(ui.page.value) || 1));

ui.dropzone.addEventListener("dragover", (event) => {
  event.preventDefault();
  ui.dropzone.classList.add("dragging");
});
ui.dropzone.addEventListener("dragleave", () => ui.dropzone.classList.remove("dragging"));
ui.dropzone.addEventListener("drop", (event) => {
  event.preventDefault();
  ui.dropzone.classList.remove("dragging");
  const [file] = event.dataTransfer.files;
  if (file) loadFile(file);
});

let resizeTimer;
window.addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(render, 150);
});

showVersion();
await loadFixture(ui.fixture.value);
