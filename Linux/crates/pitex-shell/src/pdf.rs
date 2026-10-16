//! PDF rendering and text extraction through poppler-glib — the Linux
//! equivalent of the macOS app's PDFKit `PDFDocument` usage (preview panes
//! plus the `document.string` extraction folded into agent prompts).

use gtk4::prelude::*;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::Path;
use std::sync::{Arc, mpsc::{self, Sender}};

// ─── poppler-glib / cairo FFI ───────────────────────────────────────────────

type PopplerDocument = c_void;
type PopplerPage = c_void;
type Cairo = c_void;
type CairoSurface = c_void;
type GError = c_void;
type GBytes = c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SelectionRect { pub x1: f64, pub y1: f64, pub x2: f64, pub y2: f64 }

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SelectionRegion { pub x: c_int, pub y: c_int, pub width: c_int, pub height: c_int }

#[derive(Debug, Default)]
pub struct SelectedText { pub text: String, pub regions: Vec<SelectionRegion> }

#[link(name = "poppler-glib")]
extern "C" {
    // `poppler_document_new_from_bytes(GBytes*, const char*, GError**)` — the
    // supported replacement for the deprecated `poppler_document_new_from_data`
    // (which takes a `password` parameter before `error`; a three-argument
    // declaration of it would misalign every call).
    fn poppler_document_new_from_bytes(
        bytes: *mut GBytes,
        password: *const c_char,
        error: *mut *mut GError,
    ) -> *mut PopplerDocument;
    fn poppler_document_get_n_pages(document: *mut PopplerDocument) -> c_int;
    fn poppler_document_get_page(document: *mut PopplerDocument, index: c_int) -> *mut PopplerPage;
    fn poppler_page_get_size(page: *mut PopplerPage, width: *mut c_double, height: *mut c_double);
    fn poppler_page_render(page: *mut PopplerPage, cairo: *mut Cairo);
    fn poppler_page_get_text(page: *mut PopplerPage) -> *mut c_char;
    fn poppler_page_get_selected_text(page: *mut PopplerPage, style: c_int, selection: *mut SelectionRect) -> *mut c_char;
    fn poppler_page_get_selected_region(page: *mut PopplerPage, scale: f64, style: c_int, selection: *mut SelectionRect) -> *mut c_void;
}
#[link(name = "gobject-2.0")]
extern "C" {
    fn g_object_unref(object: *mut c_void);
}
#[link(name = "glib-2.0")]
extern "C" {
    fn g_free(mem: *mut c_void);
    fn g_bytes_new(data: *const c_void, size: usize) -> *mut GBytes;
    fn g_bytes_unref(bytes: *mut GBytes);
}
#[link(name = "cairo")]
extern "C" {
    fn cairo_image_surface_create(format: c_int, width: c_int, height: c_int) -> *mut CairoSurface;
    fn cairo_create(surface: *mut CairoSurface) -> *mut Cairo;
    fn cairo_destroy(cr: *mut Cairo);
    fn cairo_surface_destroy(surface: *mut CairoSurface);
    fn cairo_scale(cr: *mut Cairo, sx: c_double, sy: c_double);
    fn cairo_set_source_rgb(cr: *mut Cairo, r: c_double, g: c_double, b: c_double);
    fn cairo_paint(cr: *mut Cairo);
    fn cairo_image_surface_get_data(surface: *mut CairoSurface) -> *mut u8;
    fn cairo_image_surface_get_stride(surface: *mut CairoSurface) -> c_int;
    fn cairo_surface_flush(surface: *mut CairoSurface);
    fn cairo_region_num_rectangles(region: *mut c_void) -> c_int;
    fn cairo_region_get_rectangle(region: *mut c_void, index: c_int, rectangle: *mut SelectionRegion);
    fn cairo_region_destroy(region: *mut c_void);
}

const CAIRO_FORMAT_ARGB32: c_int = 0;

/// One loaded PDF — page count and per-page rendering.
pub struct PdfDocument {
    raw: *mut PopplerDocument,
    page_count: usize,
    /// The `GBytes` the document was created from — poppler keeps the data
    /// alive through its own reference, but we hold ours as well so the
    /// buffer provably outlives the document (matching the old pinned
    /// `_backing` contract), released in `drop`.
    bytes: *mut GBytes,
}
// `raw`/`bytes` are raw pointers, so the type is `!Send`/`!Sync` by default —
// correct, since poppler document objects are not thread-safe and this is only
// touched only on the thread that creates it. The renderer owns a worker-local copy.

impl PdfDocument {
    /// Load from bytes in memory (the build stores PDF data directly).
    pub fn from_data(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        // `g_bytes_new` copies the input into refcounted storage.
        let bytes = unsafe { g_bytes_new(data.as_ptr() as *const c_void, data.len()) };
        if bytes.is_null() {
            return None;
        }
        let raw = unsafe {
            poppler_document_new_from_bytes(bytes, std::ptr::null(), std::ptr::null_mut())
        };
        if raw.is_null() {
            unsafe { g_bytes_unref(bytes) };
            return None;
        }
        let pages = unsafe { poppler_document_get_n_pages(raw) };
        if pages <= 0 {
            unsafe {
                g_object_unref(raw as *mut c_void);
                g_bytes_unref(bytes);
            }
            return None;
        }
        Some(Self {
            raw,
            page_count: pages as usize,
            bytes,
        })
    }
    pub fn from_file(path: &Path) -> Option<Self> {
        Self::from_data(&std::fs::read(path).ok()?)
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Page size in PDF points (origin bottom-left).
    pub fn page_size(&self, index: usize) -> Option<(f64, f64)> {
        if index >= self.page_count {
            return None;
        }
        let page = unsafe { poppler_document_get_page(self.raw, index as c_int) };
        if page.is_null() {
            return None;
        }
        let (mut w, mut h) = (0.0, 0.0);
        unsafe {
            poppler_page_get_size(page, &mut w, &mut h);
            g_object_unref(page as *mut c_void);
        }
        Some((w, h))
    }

    /// Render `index` at `scale` (1.0 = 72dpi) to premultiplied BGRA bytes —
    /// the caller wraps them in a `gdk::MemoryTexture`.
    /// Returns (pixels, width, height, stride).
    pub fn render_page(&self, index: usize, scale: f64) -> Option<(Vec<u8>, i32, i32, i32)> {
        let (w_pt, h_pt) = self.page_size(index)?;
        let width = (w_pt * scale).ceil().max(1.0) as i32;
        let height = (h_pt * scale).ceil().max(1.0) as i32;
        if width > 8192 || height > 8192 {
            return None;
        }
        let page = unsafe { poppler_document_get_page(self.raw, index as c_int) };
        if page.is_null() {
            return None;
        }
        let surface = unsafe { cairo_image_surface_create(CAIRO_FORMAT_ARGB32, width, height) };
        let cr = unsafe { cairo_create(surface) };
        unsafe {
            cairo_set_source_rgb(cr, 1.0, 1.0, 1.0);
            cairo_paint(cr);
            cairo_scale(cr, scale, scale);
            poppler_page_render(page, cr);
            cairo_destroy(cr);
            cairo_surface_flush(surface);
            g_object_unref(page as *mut c_void);
        }
        let stride = unsafe { cairo_image_surface_get_stride(surface) };
        let data = unsafe { cairo_image_surface_get_data(surface) };
        let pixels = if data.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(data, (stride * height) as usize).to_vec() }
        };
        unsafe { cairo_surface_destroy(surface) };
        Some((pixels, width, height, stride))
    }

    /// Poppler's glyph selection follows reading order, including reverse drags.
    /// Coordinates and returned regions use top-left PDF points.
    pub fn select_text(&self, index: usize, mut selection: SelectionRect) -> SelectedText {
        let mut result = SelectedText::default();
        if index >= self.page_count { return result; }
        let page = unsafe { poppler_document_get_page(self.raw, index as c_int) };
        if page.is_null() { return result; }
        unsafe {
            let text = poppler_page_get_selected_text(page, 0, &mut selection); // POPPLER_SELECTION_GLYPH
            if !text.is_null() {
                result.text = std::ffi::CStr::from_ptr(text).to_string_lossy().into_owned();
                g_free(text.cast());
            }
            let region = poppler_page_get_selected_region(page, 1.0, 0, &mut selection);
            if !region.is_null() {
                for index in 0..cairo_region_num_rectangles(region) {
                    let mut rectangle = SelectionRegion::default();
                    cairo_region_get_rectangle(region, index, &mut rectangle);
                    result.regions.push(rectangle);
                }
                cairo_region_destroy(region);
            }
            g_object_unref(page);
        }
        result
    }

    /// All pages' text concatenated — PDFKit `document.string` equivalent.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for index in 0..self.page_count {
            let page = unsafe { poppler_document_get_page(self.raw, index as c_int) };
            if page.is_null() {
                continue;
            }
            let c = unsafe { poppler_page_get_text(page) };
            if !c.is_null() {
                let s = unsafe { std::ffi::CStr::from_ptr(c) }
                    .to_string_lossy()
                    .into_owned();
                if !out.is_empty() && !s.is_empty() {
                    out.push('\n');
                }
                out.push_str(&s);
                unsafe { g_free(c as *mut c_void) };
            }
            unsafe { g_object_unref(page as *mut c_void) };
        }
        out
    }
}
impl Drop for PdfDocument {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.raw as *mut c_void);
            g_bytes_unref(self.bytes);
        }
    }
}

/// PDF text extraction for the agent envelope (PDFKit `document.string`).
pub fn extract_text(data: &[u8]) -> Option<String> {
    PdfDocument::from_data(data).map(|d| d.text())
}

/// Wrap rendered pixels in a `gdk::Texture` (CAIRO_FORMAT_ARGB32 on
/// little-endian = B8g8r8a8 premultiplied).
pub fn texture_from_pixels(
    pixels: Arc<[u8]>,
    width: i32,
    height: i32,
    stride: i32,
) -> gtk4::gdk::Texture {
    let bytes = gtk4::glib::Bytes::from_owned(pixels);
    gtk4::gdk::MemoryTexture::new(
        width,
        height,
        gtk4::gdk::MemoryFormat::B8g8r8a8Premultiplied,
        &bytes,
        stride as usize,
    )
    .upcast()
}

/// Gap between stacked pages, in pixels. Matches the preview column's box
/// spacing and PDFView's `displaysPageBreaks` separation.
pub const PDF_PAGE_GAP: f64 = 8.0;

/// One page in the continuous preview column (PDFView `singlePageContinuous`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PdfPageSpan {
    pub index: usize,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Page rectangles at `scale` (pixels per PDF point), top to bottom.
pub fn pdf_page_spans(sizes: &[(f64, f64)], scale: f64, gap: f64) -> Vec<PdfPageSpan> {
    let mut y = 0.0;
    let mut spans = Vec::with_capacity(sizes.len());
    for (index, &(w_pt, h_pt)) in sizes.iter().enumerate() {
        let width = (w_pt * scale).ceil().max(1.0);
        let height = (h_pt * scale).ceil().max(1.0);
        spans.push(PdfPageSpan { index, y, width, height });
        y += height;
        if index + 1 != sizes.len() {
            y += gap;
        }
    }
    spans
}

/// Page that owns the viewport's top edge. The gap's lower half belongs to
/// the following page, so scrolling to a page's `y` keeps that page current.
pub fn pdf_page_at_viewport(spans: &[PdfPageSpan], scroll_y: f64, _viewport_h: f64) -> usize {
    if spans.is_empty() {
        return 0;
    }
    let y = scroll_y.max(0.0);
    for (i, span) in spans.iter().enumerate() {
        let boundary = if let Some(next) = spans.get(i + 1) {
            span.y + span.height + (next.y - (span.y + span.height)) / 2.0
        } else {
            f64::MAX
        };
        if y < boundary {
            return span.index;
        }
    }
    spans.len() - 1
}

/// Inclusive page range intersecting `[top, bottom]`.
pub fn pdf_pages_intersecting(spans: &[PdfPageSpan], top: f64, bottom: f64) -> (usize, usize) {
    if spans.is_empty() {
        return (0, 0);
    }
    let mut first = None;
    let mut last = 0usize;
    for span in spans {
        if span.y + span.height >= top && span.y <= bottom {
            if first.is_none() {
                first = Some(span.index);
            }
            last = span.index;
        }
    }
    (first.unwrap_or(0), last)
}

/// Hand-drag: the pointer moved by `delta`, so the scroll offset moves opposite
/// the pointer (the page follows the cursor) and stays inside the adjustment.
pub fn pdf_drag_offset(origin: f64, delta: f64, lower: f64, upper: f64, page: f64) -> f64 {
    let max = (upper - page).max(lower);
    (origin - delta).clamp(lower, max)
}

/// Classify an in-progress primary drag.
///
/// `None` — still inside the slop, or a vertical drag that has not yet
/// traveled far enough to be a pan. `Some(true)` — scroll the continuous
/// document (a mostly vertical drag, and there is somewhere to scroll).
/// `Some(false)` — text selection, including every drag once it moves sideways.
pub fn pdf_drag_is_page_pan(dx: f64, dy: f64, scrollable: bool) -> Option<bool> {
    let adx = dx.abs();
    let ady = dy.abs();
    if adx <= 8.0 && ady < 16.0 {
        return None;
    }
    if scrollable && ady >= 16.0 && adx <= 8.0 {
        return Some(true);
    }
    Some(false)
}

/// Only plain metadata/pixels cross threads; each worker owns its Poppler document.
#[derive(Clone, Debug)]
pub struct PdfInfo { sizes: Vec<(f64, f64)> }
impl PdfInfo {
    pub fn page_count(&self) -> usize { self.sizes.len() }
    pub fn page_size(&self, index: usize) -> Option<(f64, f64)> { self.sizes.get(index).copied() }
}
#[derive(Clone)]
pub struct RenderedPage {
    pub pixels: Arc<[u8]>, pub width: i32, pub height: i32, pub stride: i32,
}
enum RenderRequest {
    Load(u64, Arc<[u8]>),
    Render { hash: u64, key: u64, page: usize, scale: f64 },
    Select { hash: u64, key: u64, id: u64, page: usize, rectangle: SelectionRect },
}
pub struct PdfRenderer { sender: Sender<RenderRequest> }
impl PdfRenderer {
    pub fn new(sink: Sender<crate::model::WorkspaceMessage>) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut document: Option<(u64, PdfDocument)> = None;
            let mut cache: std::collections::VecDeque<(u64, RenderedPage)> = std::collections::VecDeque::new();
            let mut pending = None;
            loop {
                let first = match pending.take().or_else(|| receiver.recv().ok()) {
                    Some(request) => request,
                    None => break,
                };
                if let RenderRequest::Load(hash, data) = first {
                    cache.clear();
                    document = PdfDocument::from_data(&data).map(|pdf| (hash, pdf));
                    let info = document.as_ref().map(|(_, pdf)| PdfInfo {
                        sizes: (0..pdf.page_count()).map(|i| pdf.page_size(i).unwrap_or((612.0, 792.0))).collect(),
                    });
                    let _ = sink.send(crate::model::WorkspaceMessage::PdfLoaded { hash, info });
                    continue;
                }
                // A scroll tick asks for several visible pages at once. Collapse
                // only repeats of the same page (and keep the latest selection);
                // a load waits until this batch finishes.
                let mut incoming = vec![first];
                while let Ok(next) = receiver.try_recv() {
                    if matches!(next, RenderRequest::Load(..)) {
                        pending = Some(next);
                        break;
                    }
                    incoming.push(next);
                }
                let mut renders: Vec<RenderRequest> = Vec::new();
                let mut select = None;
                for request in incoming {
                    match request {
                        RenderRequest::Render { page, .. } => {
                            if let Some(pos) = renders.iter().position(|queued| {
                                matches!(queued, RenderRequest::Render { page: queued_page, .. } if *queued_page == page)
                            }) {
                                renders[pos] = request;
                            } else {
                                renders.push(request);
                            }
                        }
                        RenderRequest::Select { .. } => select = Some(request),
                        RenderRequest::Load(..) => {}
                    }
                }
                for request in renders {
                    let RenderRequest::Render { hash, key, page, scale } = request else { continue };
                    let Some((current, pdf)) = &document else { continue };
                    if hash != *current { continue; }
                    let raster = cache.iter().find(|(k, _)| *k == key).map(|(_, p)| p.clone())
                        .or_else(|| pdf.render_page(page, scale).map(|(pixels, width, height, stride)|
                            RenderedPage { pixels: pixels.into(), width, height, stride }));
                    if let Some(raster) = raster {
                        if !cache.iter().any(|(k, _)| *k == key) && raster.pixels.len() <= 32 * 1024 * 1024 {
                            cache.push_back((key, raster.clone()));
                            // A few neighbors stay warm while the viewport scrolls.
                            while cache.len() > 6 || cache.iter().map(|(_, p)| p.pixels.len()).sum::<usize>() > 32 * 1024 * 1024 {
                                cache.pop_front();
                            }
                        }
                        let _ = sink.send(crate::model::WorkspaceMessage::PdfRendered { key, raster });
                    }
                }
                if let Some(RenderRequest::Select { hash, key, id, page, rectangle }) = select {
                    let Some((current, pdf)) = &document else { continue };
                    if hash != *current { continue; }
                    let selection = pdf.select_text(page, rectangle);
                    let _ = sink.send(crate::model::WorkspaceMessage::PdfSelected { key, id, selection });
                }
            }
        });
        Self { sender }
    }
    pub fn load(&self, hash: u64, data: Arc<[u8]>) { let _ = self.sender.send(RenderRequest::Load(hash, data)); }
    pub fn render(&self, hash: u64, key: u64, page: usize, scale: f64) {
        let _ = self.sender.send(RenderRequest::Render { hash, key, page, scale });
    }
    pub fn select(&self, hash: u64, key: u64, id: u64, page: usize, rectangle: SelectionRect) {
        let _ = self.sender.send(RenderRequest::Select { hash, key, id, page, rectangle });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letter_spans() -> Vec<PdfPageSpan> {
        // Two letter pages and one short page, at 2 px/pt.
        pdf_page_spans(&[(612.0, 792.0), (612.0, 792.0), (612.0, 100.0)], 2.0, PDF_PAGE_GAP)
    }

    #[test]
    fn pages_stack_with_the_preview_gap() {
        let spans = letter_spans();
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].y, 0.0);
        assert_eq!(spans[0].height, 1584.0);
        assert_eq!(spans[0].width, 1224.0);
        assert_eq!(spans[1].y, 1584.0 + PDF_PAGE_GAP);
        assert_eq!(spans[2].y, spans[1].y + spans[1].height + PDF_PAGE_GAP);
        assert_eq!(spans[2].height, 200.0);
    }

    #[test]
    fn viewport_top_selects_the_page_under_it() {
        let spans = letter_spans();
        assert_eq!(pdf_page_at_viewport(&spans, 0.0, 800.0), 0);
        assert_eq!(pdf_page_at_viewport(&spans, spans[0].height - 1.0, 800.0), 0);
        // Mid-gap belongs to the next page; the page top itself stays put.
        let gap_mid = spans[0].height + PDF_PAGE_GAP / 2.0;
        assert_eq!(pdf_page_at_viewport(&spans, gap_mid - 0.1, 800.0), 0);
        assert_eq!(pdf_page_at_viewport(&spans, gap_mid, 800.0), 1);
        assert_eq!(pdf_page_at_viewport(&spans, spans[1].y, 800.0), 1);
        assert_eq!(pdf_page_at_viewport(&spans, spans[2].y + 10_000.0, 800.0), 2);
        assert_eq!(pdf_page_at_viewport(&[], 0.0, 100.0), 0);
    }

    #[test]
    fn visible_range_includes_partial_pages_only() {
        let spans = letter_spans();
        assert_eq!(pdf_pages_intersecting(&spans, 0.0, 100.0), (0, 0));
        let across = spans[0].height - 10.0;
        assert_eq!(pdf_pages_intersecting(&spans, across, across + PDF_PAGE_GAP + 20.0), (0, 1));
        assert_eq!(pdf_pages_intersecting(&spans, spans[2].y + 1.0, spans[2].y + 50.0), (2, 2));
    }

    #[test]
    fn drag_follows_the_pointer_and_clamps() {
        assert_eq!(pdf_drag_offset(100.0, 40.0, 0.0, 500.0, 200.0), 60.0);
        assert_eq!(pdf_drag_offset(10.0, 40.0, 0.0, 500.0, 200.0), 0.0);
        assert_eq!(pdf_drag_offset(290.0, -50.0, 0.0, 500.0, 200.0), 300.0);
        assert_eq!(pdf_drag_offset(0.0, -10.0, 0.0, 100.0, 100.0), 0.0);
    }

    #[test]
    fn vertical_drag_pans_and_sideways_drag_selects() {
        assert_eq!(pdf_drag_is_page_pan(0.0, 10.0, true), None);
        assert_eq!(pdf_drag_is_page_pan(2.0, 20.0, true), Some(true));
        assert_eq!(pdf_drag_is_page_pan(2.0, 40.0, false), Some(false));
        assert_eq!(pdf_drag_is_page_pan(30.0, 80.0, true), Some(false));
        assert_eq!(pdf_drag_is_page_pan(9.0, 4.0, true), Some(false));
    }
}
