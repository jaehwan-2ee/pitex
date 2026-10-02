// Pitex offline pdf.js proof — component build on pdf_viewer.mjs.
// Served ONLY from pitex-pdfjs:// (custom scheme). One shared PDFWorker per
// view; each document generation fetches /doc/<gen>.pdf from the same origin.
// bridge.js (classic, loaded first) owns console/CSP/error reporting.
import * as pdfjs from "/pdf.mjs";
import { AnnotationMode, AnnotationEditorType } from "/pdf.mjs";
import { EventBus, PDFLinkService, PDFViewer } from "/pdf_viewer.mjs";

pdfjs.GlobalWorkerOptions.workerSrc = "/pdf.worker.mjs";

const post = (m) => window.__pitexPost(m);

const ORIGIN = location.origin;                    // pitex-pdfjs://app
const GEN = g => `${ORIGIN}/doc/${g}.pdf`;

const container = document.getElementById("viewerContainer");
const viewerEl  = document.getElementById("viewer");
const eventBus  = new EventBus();
const linkService = new PDFLinkService({ eventBus });
const pdfViewer = new PDFViewer({
  container, viewer: viewerEl, eventBus, linkService,
  // textLayerMode omitted — default is TextLayerMode.ENABLE (pdf_viewer.mjs:8036)
  // Fit-fix (C-v8 → PDFView autoscale parity): removePageBorders zeroes
  // SCROLLBAR_PADDING AND the 9px transparent .page border so the page element
  // fits inside W. Public PDFViewer option — vendor pdf_viewer.mjs unchanged.
  removePageBorders: true,
  annotationMode: AnnotationMode.ENABLE,       // links only, no forms
  annotationEditorMode: AnnotationEditorType.DISABLE,
  scriptingManager: null,                      // enableScripting = false
  imageResourcesPath: "",
});
linkService.setViewer(pdfViewer);

// ---- PDFView-autoscale fit mode (a2/a4) ------------------------------------
// canvas = W·P/(P+8) ⇒ pdf.js scale = W / ((P+8)·PDF_TO_CSS_UNITS).
// W = container.clientWidth (excludes a legacy vertical scrollbar); P = the
// CURRENT page's ROTATED width in pt (getViewport({scale:1}).width). 8 = the
// PDFView pageBreakMargins left+right (4+4). No constant is fitted to 700.
const CSS_PER_PT = 96 / 72;                    // PixelsPerInch.PDF_TO_CSS_UNITS
let fitMode = true;                            // ON at cold load (a4)
let lastFitW = -1;
async function applyFit() {
  const doc = current.doc; if (!doc) return;
  const g = current.gen;
  const page = await doc.getPage(pdfViewer.currentPageNumber || 1)
    .catch(() => null);               // doc torn down mid-fit -> stop
  if (!page || current.gen !== g || current.doc !== doc) return; // B10: superseded
  const P = page.getViewport({ scale: 1 }).width; // rotated width, pt
  const W = container.clientWidth;
  if (W <= 0 || !(P > 0)) return;
  pdfViewer.currentScale = Math.min(4.0, W / ((P + 8) * CSS_PER_PT));
}
// Re-fit while fitMode when the usable width changes (a4). ResizeObserver
// covers a box resize; a scrollbar toggling may not resize the box, so we also
// re-check after a fit settles (scalechanging/pagesinit). |ΔW|<1 px guard
// stops oscillation; one re-fit per frame via the rAF-settled flag.
let refitQueued = false;
function maybeRefit() {
  if (!fitMode || refitQueued) return;
  const W = container.clientWidth;
  if (Math.abs(W - lastFitW) < 1) return;
  refitQueued = true;
  requestAnimationFrame(async () => {
    refitQueued = false;
    if (fitMode) { await applyFit(); lastFitW = container.clientWidth; }
  });
}
new ResizeObserver(() => maybeRefit()).observe(container);
// corr3: a scrollbar toggling may not resize the box -> re-check after a fit.
eventBus.on("scalechanging", () => { if (fitMode) maybeRefit(); });
eventBus.on("pagerendered", () => { if (fitMode) maybeRefit(); });

// One shared worker for the view's lifetime: getDocument adopts a passed-in
// worker (api.js) and destroy() only kills owned ones.
const worker = new pdfjs.PDFWorker({ name: "pitex" });

let current = { gen: -1, doc: null, task: null, media: null, pos: null };
let pending = null;      // single pending slot, newest wins
let pendingGoto = null;  // F3: goto issued while its gen was in-flight
let loading = null;
let highlightSync = false;

function bridgeState(extra = {}) {
  post({type: "state", gen: current.gen, pages: current.doc?.numPages ?? 0,
        page: pdfViewer.currentPageNumber, scale: pdfViewer.currentScale,
        fitMode, ...extra});                  // a4: report the mode
}
const stale = g => g !== current.gen;

// ONE pagesinit listener for all generations (B10): reads current at fire
// time, so a superseded generation can't restore or report through a stale
// handler. pos/scale live on `current`.
eventBus.on("pagesinit", async () => {
  const g = current.gen;
  const pos = current.pos;
  // a4/corr2: restore BY THE fitMode FLAG, not the scale NAME — our fit sets a
  // NUMERIC currentScale, so a saved number would silently kill the mode.
  if (pos?.fitMode || !pos) {
    // fitMode flag set, or cold load (no pos) → mode ON and apply the fit.
    fitMode = true; await applyFit();
    if (current.gen !== g) return;   // B10: superseded during the fit await
    lastFitW = container.clientWidth;
  } else {
    fitMode = false;
    if (typeof pos.scale === "number")
      pdfViewer.currentScale = pos.scale;
  }
  if (pos) {
    pdfViewer.scrollPageIntoView({
      pageNumber: pos.page,
      destArray: [null, {name: "XYZ"}, pos.x, pos.y, null],
    });
  }
  // F3: a goto issued while this gen was in-flight now applies.
  if (pendingGoto && pendingGoto.gen === current.gen) {
    const a = pendingGoto; pendingGoto = null;
    applyGoto(a.page, a.x, a.v, a.w, a.h);
  }
  post({type: "displayed", gen: g, page: current.pos?.page ?? 1});
});

// pagerendered carries the render error (null on success) so the host can
// require an actual rendered page (B11).
eventBus.on("pagerendered", (d) => {
  // T5: tag the render with the gen of the PAGE VIEW that produced it —
  // a late completion from a superseded document's view reports gen -1,
  // never the new gen, so it can't fake a qualifying paint.
  const src = d.source;
  const cur = pdfViewer.getPageView(d.pageNumber - 1);
  post({type: "pagerendered", gen: (src && src === cur) ? current.gen : -1,
        page: d.pageNumber,
        error: d.error ? String(d.error) : null,
        cssTransform: d.cssTransform === true, detail: d.isDetailView === true});
});
eventBus.on("pagechanging", () => bridgeState());


// --- document lifecycle -----------------------------------------------------
// Order: getDocument(new, shared worker) → await → record position →
// setDocument (cancels old rendering, does NOT destroy the old doc) →
// pagesinit: restore scale + scroll → await old.destroy().
async function loadNext(gen, mediaBoxes, sameTarget) {
  loading = (async () => {
    const task = pdfjs.getDocument({
      url: GEN(gen), worker,
      cMapUrl: "/cmaps/", cMapPacked: true,
      standardFontDataUrl: "/standard_fonts/",
      wasmUrl: "/wasm/", iccUrl: "/iccs/",
      enableXfa: false, enableScripting: false,
      // no verbosity override — WARNINGS must stay on for the fake-worker oracle
    });
    const doc = await task.promise;
    if (pending !== null && pending.gen !== gen) {
      await task.destroy().catch(e =>
        post({type: "destroy-error", gen, text: String(e)}));
      return;
    }
    let pos = null;
    if (sameTarget && current.gen >= 0) {
      const pv = pdfViewer.getPageView(pdfViewer.currentPageNumber - 1);
      if (pv) {
        const wrap = pv.div.querySelector(".canvasWrapper");
        const r = wrap.getBoundingClientRect();
        const cr = container.getBoundingClientRect();
        const [px, py] = pv.viewport.convertToPdfPoint(
          cr.left - r.left, cr.top - r.top);
        // corr2: record the MODE, not just the scale — restores go by flag.
        pos = { page: pdfViewer.currentPageNumber, x: px, y: py,
                scale: pdfViewer.currentScale, fitMode };
      }
    }
    const old = current;
    current = { gen, doc, task, media: mediaBoxes, pos };
    pdfViewer.setDocument(doc);
    linkService.setDocument(doc, ORIGIN + "/");
    if (old.task) {
      try { await old.task.destroy(); }
      catch (e) {
        if (!String(e?.name || e).includes("RenderingCancelled"))
          post({type: "destroy-error", gen: old.gen, text: String(e)});
      }
    }
    post({type: "loaded", gen, pages: doc.numPages});
    post({type: "worker-ok", destroyed: worker.destroyed === true});
  })().catch(e => post({type: "load-error", gen, text: String(e)}));
  await loading;
  loading = null;
  if (pending) { const p = pending; pending = null;
                 await loadNext(p.gen, p.mediaBoxes, p.sameTarget); }
}

// --- commands (all async-safe for callAsyncJavaScript) ----------------------
window.pitex = {
  // generation + per-page MediaBoxes + same-target flag from native.
  load(gen, mediaBoxes, sameTarget) {
    const m = { gen, mediaBoxes, sameTarget: sameTarget === true };
    if (loading) { pending = m; return; }
    loadNext(gen, mediaBoxes, m.sameTarget);
  },
  async workerIsReal() { await worker.promise; return worker.port instanceof Worker; },
  setHighlightSync(on) {
    highlightSync = on === true;
    if (!highlightSync)
      container.querySelectorAll(".pitex-hl").forEach(n => n.remove());
  },
  // Forward SyncTeX parity with Preview.swift:519-529: scroll ALWAYS, then
  // guard highlightSync for the overlay. Rect is a MediaBox-relative
  // {x,v,w,h} SyncTeX box (v measured from the top edge).
  goto(gen, page, x, v, w, h) {
    // F3: during an in-flight swap, stash the newest goto; pagesinit
    // applies it once its gen is current.
    if (gen > current.gen) { pendingGoto = {gen, page, x, v, w, h}; return; }
    if (stale(gen)) return;
    applyGoto(page, x, v, w, h);
  },
  // F4/F7: zoom entry points — non-compounding per-call factor, and the
  // "Automatically Resize" parity item = pdf.js page-width fit.
  zoomBy(f) {
    if (typeof f !== "number" || !isFinite(f)) return null;
    fitMode = false;   // a4: explicit zoom turns fit mode OFF
    pdfViewer.currentScale =
      Math.min(4.0, Math.max(0.1, pdfViewer.currentScale * f));
    return pdfViewer.currentScale;
  },
  // "Automatically Resize" = PDFView autoscale parity → fitMode ON + re-fit.
  autoResize() {
    fitMode = true;
    applyFit().then(() => { lastFitW = container.clientWidth; });
    return true;
  },
  async text(gen) {
    if (stale(gen)) return null;
    let out = "";
    for (let p = 1; p <= current.doc.numPages; p++) {
      const t = await current.doc.getPage(p).then(pg => pg.getTextContent());
      out += t.items.map(i => i.str).join("") + "\n";
    }
    return out;
  },
  // C7: drive a view state for the position-restore check.
  setView(gen, scale, page) {
    if (stale(gen)) return false;
    fitMode = false;   // a4: explicit scale turns fit mode OFF
    pdfViewer.currentScaleValue = String(scale);
    pdfViewer.scrollPageIntoView({pageNumber: page,
      destArray: [null, {name: "XYZ"}, null, null, null]});
    return true;
  },
  position(gen) {
    if (stale(gen)) return null;
    return {page: pdfViewer.currentPageNumber, scale: pdfViewer.currentScale,
            scrollTop: container.scrollTop, fitMode};   // a4: report the mode
  },
  state() { bridgeState(); return current.gen; },
  async shutdown() {
    const t = current.task;
    current = {gen: -1, doc: null, task: null, media: null, pos: null};
    pdfViewer.setDocument(null);
    if (t) await t.destroy().catch(() => {});
    return true;
  },
};

// Overlay lives in the page's .canvasWrapper so it scrolls with the page
// and uses content-box coordinates (pdf_viewer.css gives .page a border).
// Forward-sync scroll + highlight. F2: parity with the PDFView control's
// `go(to: bounds.insetBy(-8,-8))`. SyncTeX `v` is the BASELINE measured
// down from the page top; the highlight box spans user-space y
// [mb.y1 - v, mb.y1 - v + h]. pdf.js XYZ aligns its point with the
// viewport's TOP edge, so the destination is the box top + 8pt margin:
// (mb.x0 + x - 8, mb.y1 - v + h + 8).
function applyGoto(page, x, v, w, h) {
  const pv = pdfViewer.getPageView(page - 1);
  if (!pv) return;
  const mb = current.media[page - 1];
  pdfViewer.scrollPageIntoView({
    pageNumber: page,
    destArray: [null, {name: "XYZ"}, mb[0] + x - 8, mb[3] - v + h + 8, null]});
  if (!highlightSync) return;
  // no convertToViewportRectangle in 6.3.289 — convert both corners.
  const [ax, ay] = pv.viewport.convertToViewportPoint(mb[0] + x,     mb[3] - v);
  const [bx, by] = pv.viewport.convertToViewportPoint(mb[0] + x + w, mb[3] - v + h);
  overlay(pv, Math.min(ax, bx), Math.min(ay, by),
          Math.max(Math.abs(bx - ax), 4), Math.max(Math.abs(by - ay), 4));
}

function overlay(pv, x, y, w, h) {
  container.querySelectorAll(".pitex-hl").forEach(n => n.remove());
  const wrap = pv.div.querySelector(".canvasWrapper");
  const el = document.createElement("div");
  el.className = "pitex-hl";
  el.style.cssText = `left:${x}px;top:${y}px;width:${w}px;height:${h}px`;
  wrap.appendChild(el);
  setTimeout(() => el.remove(), 1500);
}

function inverseAt(clientX, clientY) {
  const pageEl = document.elementFromPoint(clientX, clientY)?.closest?.(".page");
  if (!pageEl) return null;
  const page = parseInt(pageEl.dataset.pageNumber, 10);
  const pv = pdfViewer.getPageView(page - 1);
  if (!pv) return null;
  const wrap = pageEl.querySelector(".canvasWrapper");
  const r = wrap.getBoundingClientRect();       // content box
  const [px, py] = pv.viewport.convertToPdfPoint(clientX - r.left, clientY - r.top);
  const mb = current.media[page - 1];
  return { page, x: px - mb[0], y: mb[3] - py, gen: current.gen };
}

// Cmd+click: capture-phase mousedown reports the inverse point; the click
// that follows is suppressed so links/selection don't fire (preventDefault
// on mousedown alone does NOT cancel the click). suppressClick is set from
// metaKey on EVERY mousedown so it can't stick (M5).
let suppressClick = false;
container.addEventListener("mousedown", e => {
  suppressClick = e.metaKey && e.button === 0;
  if (!suppressClick) return;
  e.preventDefault(); e.stopPropagation();
  const inv = inverseAt(e.clientX, e.clientY);
  if (inv) post({type: "inverse", ...inv});
}, true);
container.addEventListener("click", e => {
  if (suppressClick) { suppressClick = false; e.preventDefault(); e.stopPropagation(); return; }
  const a = e.target.closest?.("a");
  if (a && /^https?:/.test(a.href)) { e.preventDefault(); post({type: "link", url: a.href}); }
}, true);

post({type: "ready"});
