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
                let mut request = match pending.take().or_else(|| receiver.recv().ok()) {
                    Some(request) => request, None => break,
                };
                // Collapse consecutive zoom/page requests, preserving load boundaries.
                if matches!(request, RenderRequest::Render { .. }) {
                    while let Ok(next) = receiver.try_recv() {
                        if matches!(next, RenderRequest::Load(..)) { pending = Some(next); break; }
                        request = next;
                    }
                }
                match request {
                    RenderRequest::Load(hash, data) => {
                        cache.clear();
                        document = PdfDocument::from_data(&data).map(|pdf| (hash, pdf));
                        let info = document.as_ref().map(|(_, pdf)| PdfInfo {
                            sizes: (0..pdf.page_count()).map(|i| pdf.page_size(i).unwrap_or((612.0, 792.0))).collect(),
                        });
                        let _ = sink.send(crate::model::WorkspaceMessage::PdfLoaded { hash, info });
                    }
                    RenderRequest::Render { hash, key, page, scale } => {
                        let Some((current, pdf)) = &document else { continue };
                        if hash != *current { continue; }
                        let raster = cache.iter().find(|(k, _)| *k == key).map(|(_, p)| p.clone())
                            .or_else(|| pdf.render_page(page, scale).map(|(pixels, width, height, stride)|
                                RenderedPage { pixels: pixels.into(), width, height, stride }));
                        if let Some(raster) = raster {
                            if !cache.iter().any(|(k, _)| *k == key) && raster.pixels.len() <= 32 * 1024 * 1024 {
                                cache.push_back((key, raster.clone()));
                                while cache.len() > 3 || cache.iter().map(|(_, p)| p.pixels.len()).sum::<usize>() > 32 * 1024 * 1024 {
                                    cache.pop_front();
                                }
                            }
                            let _ = sink.send(crate::model::WorkspaceMessage::PdfRendered { key, raster });
                        }
                    }
                }
            }
        });
        Self { sender }
    }
    pub fn load(&self, hash: u64, data: Arc<[u8]>) { let _ = self.sender.send(RenderRequest::Load(hash, data)); }
    pub fn render(&self, hash: u64, key: u64, page: usize, scale: f64) {
        let _ = self.sender.send(RenderRequest::Render { hash, key, page, scale });
    }
}
