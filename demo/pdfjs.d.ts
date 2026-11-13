// The slice of pdf.js's API the page uses, at the version pinned in `app.ts`.
//
// pdf.js is imported from jsdelivr at runtime, so its own declarations are not at hand:
// these are written from pdfjs-dist's, and must follow it when the pin moves.

/** An affine matrix `[a, b, c, d, e, f]`. */
export type Matrix = [number, number, number, number, number, number];

/** The `pdf.min.mjs` module. */
export interface PdfJs {
  GlobalWorkerOptions: { workerSrc: string };
  VerbosityLevel: { ERRORS: number; WARNINGS: number; INFOS: number };
  getDocument(source: { data: Uint8Array; verbosity?: number }): PDFDocumentLoadingTask;
}

export interface PDFDocumentLoadingTask {
  promise: Promise<PDFDocumentProxy>;
  destroy(): Promise<void>;
}

export interface PDFDocumentProxy {
  numPages: number;
  /** One-based. */
  getPage(pageNumber: number): Promise<PDFPageProxy>;
}

export interface PDFPageProxy {
  getViewport(parameters: { scale: number }): PageViewport;
  render(parameters: { canvas: HTMLCanvasElement; viewport: PageViewport; transform?: Matrix }): RenderTask;
  getTextContent(): Promise<TextContent>;
}

export interface PageViewport {
  width: number;
  height: number;
  scale: number;
  /** User space to viewport space: rotation, crop box, y flip and scale. */
  transform: Matrix;
  convertToViewportPoint(x: number, y: number): [number, number];
}

export interface RenderTask {
  promise: Promise<void>;
  cancel(): void;
}

export interface TextContent {
  items: (TextItem | TextMarkedContent)[];
}

export interface TextItem {
  str: string;
  /** Text space to user space. */
  transform: Matrix;
  /** In user space. */
  width: number;
  /** In user space. */
  height: number;
  fontName: string;
}

/** A marked-content marker: it shows no text. */
export interface TextMarkedContent {
  type: "beginMarkedContent" | "beginMarkedContentProps" | "endMarkedContent";
  id?: string;
}
