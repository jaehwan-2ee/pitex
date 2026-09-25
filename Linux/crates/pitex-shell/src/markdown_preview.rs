//! `MarkdownPreview` — the inspector's Markdown half, hosting the shared
//! renderer page (`Assets/markdown-preview`, inlined into
//! `Mac/Resources/markdown-preview.html`) in a platform web view: WebKitGTK
//! 6.0 on Unix, Microsoft Edge WebView2 (a child HWND of the GTK window) on
//! Windows. Builds without the `markdown-preview` feature show the
//! localized "not available" status page instead and none of the engine
//! paths compile in.

use gtk4::prelude::*;
use gtk4::{gio, glib};
#[cfg(feature = "markdown-preview")]
use std::cell::Cell;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::app_ui::a11y;
use crate::l10n::tr;

#[cfg(all(feature = "markdown-preview", unix))]
mod webkit6;
#[cfg(all(feature = "markdown-preview", windows))]
mod webview2;

#[cfg(all(feature = "markdown-preview", unix))]
use self::webkit6 as engine;
#[cfg(all(feature = "markdown-preview", windows))]
use self::webview2 as engine;

#[cfg(feature = "markdown-preview")]
const PREVIEW_HTML: &str = include_str!("../../../../Mac/Resources/markdown-preview.html");

/// Extensions that count as executable even without a mode bit — `.app` is
/// a directory, so this check runs before the file-type tests below.
const REFUSED_EXTENSIONS: &[&str] = &[
    "app",
    "command",
    "tool",
    "terminal",
    "workflow",
    "desktop",
    "appimage",
    "jar",
    "scpt",
    "applescript",
    "sh",
    "bash",
    "zsh",
    "fish",
    "csh",
    "ksh",
];

/// Windows executables and script-host types — `start file.exe` runs them,
/// so a Markdown link must never reach the OS handler with one.
#[cfg(windows)]
const REFUSED_EXTENSIONS_WINDOWS: &[&str] = &[
    "exe",
    "com",
    "msi",
    "msp",
    "scr",
    "pif",
    "hta",
    "cpl",
    "msc",
    "dll",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "psd1",
    "reg",
    "vb",
    "vbs",
    "vbe",
    "js",
    "jse",
    "ws",
    "wsf",
    "wsh",
    "wsc",
    "lnk",
    "appx",
    "appxbundle",
    "msix",
    "msixbundle",
];

/// What a preview link click should do — the navigation policy handlers and
/// the unit tests share this one decision (`PreviewLinkAction` on macOS).
#[derive(Debug, PartialEq)]
pub enum PreviewLinkAction {
    /// http/https/mailto — the OS handler (`launch_default_for_uri`).
    External(String),
    /// A safe local file — the OS default app.
    OpenFile(PathBuf),
    /// Executable or refused extension — the UI shows the blocked toast.
    Refuse,
    /// Missing file or unsupported scheme (`javascript:`…) — do nothing.
    Ignore,
}

/// `previewLinkAction` — strict: `file:` resolves to a percent-decoded path
/// (query/fragment dropped) that must exist; executables and the extension
/// denylist are refused so a Markdown document can never run code; every
/// other scheme is ignored.
pub fn preview_link_action(uri: &str) -> PreviewLinkAction {
    let lower = uri.to_ascii_lowercase();
    if lower.starts_with("http:") || lower.starts_with("https:") || lower.starts_with("mailto:") {
        return PreviewLinkAction::External(uri.to_string());
    }
    if !lower.starts_with("file:") {
        return PreviewLinkAction::Ignore;
    }
    let bare = uri.split(['?', '#']).next().unwrap_or("");
    let Some(path) = gio::File::for_uri(bare).path() else {
        return PreviewLinkAction::Ignore;
    };
    // Symlinks resolve first: `paper.pdf -> Evil.app` must be judged by its
    // target's extension and mode, not the link's own. Canonicalize also
    // doubles as the existence check (missing → Err → ignore).
    let Ok(path) = std::fs::canonicalize(&path) else {
        return PreviewLinkAction::Ignore;
    };
    let refused_extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| REFUSED_EXTENSIONS.iter().any(|r| e.eq_ignore_ascii_case(r)))
        .unwrap_or(false);
    #[cfg(windows)]
    let refused_extension = refused_extension
        || path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                REFUSED_EXTENSIONS_WINDOWS
                    .iter()
                    .any(|r| e.eq_ignore_ascii_case(r))
            })
            .unwrap_or(false);
    if refused_extension {
        return PreviewLinkAction::Refuse;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
                return PreviewLinkAction::Refuse;
            }
        }
    }
    PreviewLinkAction::OpenFile(path)
}

/// `pitex.pref.markdown.theme` resolver — `system` (and any unknown value)
/// follows the app's effective appearance; light/dark pin the page.
pub fn markdown_preview_dark(theme: &str, app_is_dark: bool) -> bool {
    match theme {
        "light" => false,
        "dark" => true,
        _ => app_is_dark,
    }
}

/// (document url, revision, font-size bits, dark) of the last render —
/// unchanged keys skip the engine round-trip entirely.
#[derive(Default)]
struct RenderDedup(Option<(PathBuf, u64, u64, bool)>);

impl RenderDedup {
    /// First call or a changed input → `true` (push); a repeated key →
    /// `false` (the page already shows exactly this).
    fn needs_render(&mut self, url: &Path, revision: u64, font_size: f64, dark: bool) -> bool {
        let key = (url.to_path_buf(), revision, font_size.to_bits(), dark);
        if self.0.as_ref() == Some(&key) {
            return false;
        }
        self.0 = Some(key);
        true
    }

    fn url(&self) -> Option<PathBuf> {
        self.0.as_ref().map(|key| key.0.clone())
    }
}

/// The preview→editor feedback-loop guard — `arm` mutes editor→preview
/// pushes for a short window so a programmatic editor scroll's vadjustment
/// echo can't feed back into the page.
#[derive(Default)]
struct ScrollGuard {
    until: Option<Instant>,
}

impl ScrollGuard {
    fn arm(&mut self) {
        self.until = Some(Instant::now() + Duration::from_millis(120));
    }

    fn muted(&self) -> bool {
        self.until.map(|t| Instant::now() < t).unwrap_or(false)
    }
}

/// One navigation policy, shared by both engines: the initial page load is
/// the only navigation ever allowed; user-initiated ones (link clicks,
/// `target=_blank`) are cancelled and routed through `preview_link_action`.
#[cfg(feature = "markdown-preview")]
pub(crate) enum NavVerdict {
    /// Proceed — only ever the engine's own first page load.
    Allow,
    /// Cancel silently (redirects, meta refresh, `location=`…).
    Cancel,
    /// Cancel and hand the uri to `open_preview_link`.
    Route(String),
}

#[cfg(feature = "markdown-preview")]
pub(crate) struct NavigationPolicy {
    /// The engine's own page load — consumed by the first navigation event.
    initial: bool,
}

#[cfg(feature = "markdown-preview")]
impl NavigationPolicy {
    pub(crate) fn new() -> Self {
        Self { initial: true }
    }

    /// A main-frame navigation. `link` marks user intent to open `uri`
    /// (WebKit `LinkClicked` / WebView2 `IsUserInitiated`).
    pub(crate) fn navigate(&mut self, uri: Option<&str>, link: bool) -> NavVerdict {
        if self.initial {
            self.initial = false;
            return NavVerdict::Allow;
        }
        match uri.filter(|_| link) {
            Some(uri) => NavVerdict::Route(uri.to_string()),
            None => NavVerdict::Cancel,
        }
    }

    /// A `target=_blank`/`window.open` request — never a popup inside the
    /// preview; the uri takes the link route when present.
    pub(crate) fn new_window(&mut self, uri: Option<&str>) -> NavVerdict {
        match uri {
            Some(uri) => NavVerdict::Route(uri.to_string()),
            None => NavVerdict::Cancel,
        }
    }
}

/// Callbacks an engine uses to reach back into the shared state. Kept on
/// `Rc` so each signal/COM handler can clone the one it needs.
#[cfg(feature = "markdown-preview")]
pub(crate) struct EngineHooks {
    /// The initial page load finished — drain the queued render.
    pub loaded: Rc<dyn Fn()>,
    /// A cancelled navigation carrying user intent — the uri takes the
    /// `preview_link_action` route.
    pub open_link: Rc<dyn Fn(String)>,
    /// Preview→editor scroll sync — the top visible source line.
    pub scroll: Rc<dyn Fn(f64)>,
    /// Show an error toast (PDF export failures).
    pub toast: Rc<dyn Fn(String)>,
}

#[cfg(feature = "markdown-preview")]
struct WebState {
    /// `None` while the async engine creation is in flight or failed.
    engine: RefCell<Option<engine::Engine>>,
    /// Initial page load finished — render calls queue until then.
    loaded: Cell<bool>,
    queued: RefCell<Option<String>>,
}

#[cfg(feature = "markdown-preview")]
impl WebState {
    fn eval(&self, script: &str) {
        if let Some(engine) = self.engine.borrow().as_ref() {
            engine.eval(script);
        }
    }

    /// `loaded` hook body — mark ready and flush the newest queued render.
    fn finish_load(&self) {
        self.loaded.set(true);
        if let Some(script) = self.queued.borrow_mut().take() {
            self.eval(&script);
        }
    }

    fn print_to_pdf(&self, path: &Path, failed_message: &str) {
        if let Some(engine) = self.engine.borrow().as_ref() {
            engine.print_to_pdf(path, failed_message);
        }
    }

    fn set_covered(&self, covered: bool) {
        if let Some(engine) = self.engine.borrow().as_ref() {
            engine.set_covered(covered);
        }
    }
}

#[derive(Clone)]
pub struct MarkdownPreview {
    /// Inspector child — the webview container, or the unavailable page.
    pub widget: gtk4::Widget,
    /// Lazily created on the first render — non-Markdown sessions never pay
    /// the engine startup cost.
    #[cfg(feature = "markdown-preview")]
    web: Rc<RefCell<Option<Rc<WebState>>>>,
    /// The toolbar's document name, like the PDF toolbar's.
    #[cfg(feature = "markdown-preview")]
    name: gtk4::Label,
    /// In-surface overlays currently covering the web view (toasts) — the
    /// WebView2 child HWND hides while the count is non-zero.
    #[cfg(feature = "markdown-preview")]
    overlays: Rc<Cell<u32>>,
    /// UI language for the PDF-failure toast — only the engine path reads it.
    #[cfg(feature = "markdown-preview")]
    language: &'static str,
    /// Deduped render key — see `RenderDedup`.
    rendered: Rc<RefCell<RenderDedup>>,
    /// The debounced live render — cancel + re-arm like `AUTOSAVE_SOURCE`.
    render_source: Rc<RefCell<Option<glib::SourceId>>>,
    /// Mutes editor→preview pushes briefly after a preview-driven
    /// programmatic editor scroll (feedback-loop guard).
    editor_quiet: Rc<RefCell<ScrollGuard>>,
}

impl MarkdownPreview {
    pub fn new(language: &'static str) -> Self {
        #[cfg(feature = "markdown-preview")]
        let name = gtk4::Label::new(None);
        #[cfg(feature = "markdown-preview")]
        let widget: gtk4::Widget = {
            // The engine attaches below the toolbar on first render. Windows
            // without the WebView2 runtime gets the status page instead.
            if engine::runtime_available() {
                let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                column.append(&download_toolbar(&name, language));
                column.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
                column.upcast()
            } else {
                let page = libadwaita::StatusPage::new();
                page.set_title(&tr(language, "preview.markdown.unavailable"));
                page.set_description(Some(&tr(language, "preview.markdown.runtime_missing")));
                page.set_icon_name(Some("text-x-generic-symbolic"));
                page.set_vexpand(true);
                page.upcast()
            }
        };
        #[cfg(not(feature = "markdown-preview"))]
        let widget: gtk4::Widget = {
            let page = libadwaita::StatusPage::new();
            page.set_title(&tr(language, "preview.markdown.unavailable"));
            page.set_icon_name(Some("text-x-generic-symbolic"));
            page.set_vexpand(true);
            page.upcast()
        };
        a11y(&widget, "pitex.preview.markdown", "preview.title");
        Self {
            widget,
            #[cfg(feature = "markdown-preview")]
            web: Rc::new(RefCell::new(None)),
            #[cfg(feature = "markdown-preview")]
            name,
            #[cfg(feature = "markdown-preview")]
            overlays: Rc::new(Cell::new(0)),
            #[cfg(feature = "markdown-preview")]
            language,
            rendered: Rc::new(RefCell::new(RenderDedup::default())),
            render_source: Rc::new(RefCell::new(None)),
            editor_quiet: Rc::new(RefCell::new(ScrollGuard::default())),
        }
    }

    /// The url whose text is currently rendered — `None` before the first
    /// push, used to force a render on document activation.
    pub fn rendered_url(&self) -> Option<PathBuf> {
        self.rendered.borrow().url()
    }

    /// Cancel a pending debounced render (document switched away).
    pub fn cancel_render(&self) {
        if let Some(id) = self.render_source.borrow_mut().take() {
            id.remove();
        }
    }

    /// Debounce ~150ms per edit — the same cancel-and-re-arm shape as
    /// `schedule_autosave`; the timer re-reads state at fire time.
    pub fn schedule_render(&self) {
        self.cancel_render();
        let source = self.render_source.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(150), move || {
            source.borrow_mut().take();
            crate::app_ui::STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(st) = state.try_borrow() {
                        st.render_markdown_now();
                    }
                }
            });
        });
        *self.render_source.borrow_mut() = Some(id);
    }

    /// Push the full document into the page via `pitexRender`. Unchanged
    /// payloads are skipped; before the initial load finishes only the
    /// newest one waits in `queued`.
    pub fn render(&self, url: &Path, revision: u64, text: &str, font_size: f64, dark: bool) {
        if !self
            .rendered
            .borrow_mut()
            .needs_render(url, revision, font_size, dark)
        {
            return;
        }
        #[cfg(feature = "markdown-preview")]
        {
            self.name.set_text(
                &url.file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default(),
            );
            // `<base href>` — the document's directory as a file:// URL so
            // relative images resolve; needs the trailing slash.
            let base = url
                .parent()
                .map(|d| format!("{}/", gio::File::for_path(d).uri()))
                .unwrap_or_else(|| "file:///".into());
            let script = format!(
                "pitexRender({})",
                serde_json::json!({
                    "text": text,
                    "baseHref": base,
                    "fontSize": font_size,
                    "dark": dark,
                })
            );
            let Some(web) = self.ensure_webview() else {
                return;
            };
            if !web.loaded.get() {
                *web.queued.borrow_mut() = Some(script);
                return;
            }
            web.eval(&script);
        }
        #[cfg(not(feature = "markdown-preview"))]
        let _ = text;
    }

    /// Download: prints the rendered page to `path` as a paginated PDF. The
    /// dark palette is screen-only CSS, so the file always comes out light.
    pub fn export_pdf(&self, path: &Path) {
        #[cfg(feature = "markdown-preview")]
        if let Some(web) = self.web.borrow().as_ref() {
            web.print_to_pdf(path, &tr(self.language, "preview.markdown.pdf_failed"));
        }
        #[cfg(not(feature = "markdown-preview"))]
        let _ = path;
    }

    /// Editor→preview scroll sync — `pitexScrollToLine(line)` on the page.
    /// No-op until the engine exists (non-Markdown sessions keep it lazy).
    pub fn scroll_to_line(&self, line: i32) {
        #[cfg(feature = "markdown-preview")]
        if let Some(web) = self.web.borrow().as_ref() {
            if web.loaded.get() {
                web.eval(&format!("pitexScrollToLine({line})"));
            }
        }
        #[cfg(not(feature = "markdown-preview"))]
        let _ = line;
    }

    /// Called right before a preview-driven programmatic editor scroll —
    /// the vadjustment echo is muted for a short window afterwards.
    pub fn begin_programmatic_editor_scroll(&self) {
        self.editor_quiet.borrow_mut().arm();
    }

    /// True inside the loop-guard window after `begin_programmatic_editor_scroll`.
    pub fn editor_scroll_suppressed(&self) -> bool {
        self.editor_quiet.borrow().muted()
    }

    /// Airspace bookkeeping: a WebView2 child HWND can't be overdrawn by
    /// in-surface GTK widgets (toasts), so the web layer hides while one is
    /// open. Engines GTK composites itself ignore the hint.
    pub fn overlay_opened(&self) {
        #[cfg(feature = "markdown-preview")]
        self.note_overlay(1);
    }

    /// See `overlay_opened`.
    pub fn overlay_closed(&self) {
        #[cfg(feature = "markdown-preview")]
        self.note_overlay(-1);
    }

    /// Test handle to the lazily created `WebView` — lets the Xvfb smoke
    /// tests evaluate JS (scroll position, dark palette) against the page.
    /// Unix only: the WebView2 engine exposes no equivalent.
    #[cfg(all(test, unix, feature = "markdown-preview"))]
    pub(crate) fn webview_for_test(&self) -> Option<::webkit6::WebView> {
        self.web
            .borrow()
            .as_ref()?
            .engine
            .borrow()
            .as_ref()
            .map(|engine| engine.view.clone())
    }

    #[cfg(feature = "markdown-preview")]
    fn note_overlay(&self, delta: i32) {
        let count = self.overlays.get().saturating_add_signed(delta).max(0);
        self.overlays.set(count);
        if let Some(web) = self.web.borrow().as_ref() {
            web.set_covered(count > 0);
        }
    }

    #[cfg(feature = "markdown-preview")]
    fn ensure_webview(&self) -> Option<Rc<WebState>> {
        if let Some(web) = self.web.borrow().as_ref() {
            return Some(web.clone());
        }
        // The unavailable status page has no host column — never reaches here.
        let host = self.widget.downcast_ref::<gtk4::Box>()?.clone();
        let web = Rc::new(WebState {
            engine: RefCell::new(None),
            loaded: Cell::new(false),
            queued: RefCell::new(None),
        });
        let weak = Rc::downgrade(&web);
        let hooks = EngineHooks {
            loaded: Rc::new(move || {
                if let Some(web) = weak.upgrade() {
                    web.finish_load();
                }
            }),
            open_link: Rc::new(|uri| {
                crate::app_ui::STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(st) = state.try_borrow() {
                            st.open_preview_link(&uri);
                        }
                    }
                });
            }),
            scroll: Rc::new(|line| {
                crate::app_ui::STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(st) = state.try_borrow() {
                            st.scroll_editor_to_line(line);
                        }
                    }
                });
            }),
            toast: Rc::new(|message| {
                crate::app_ui::STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(st) = state.try_borrow() {
                            st.toast(&message);
                        }
                    }
                });
            }),
        };
        *web.engine.borrow_mut() = Some(engine::Engine::create(&host, hooks)?);
        web.set_covered(self.overlays.get() > 0);
        *self.web.borrow_mut() = Some(web.clone());
        Some(web)
    }
}

/// Document name + Download, mirroring the PDF preview's toolbar.
#[cfg(feature = "markdown-preview")]
fn download_toolbar(name: &gtk4::Label, lang: &'static str) -> gtk4::Box {
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    toolbar.set_margin_start(10);
    toolbar.set_margin_end(10);
    toolbar.set_margin_top(6);
    toolbar.set_margin_bottom(6);
    name.set_hexpand(true);
    name.set_xalign(0.0);
    name.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    name.add_css_class("caption");
    toolbar.append(name);
    let download = gtk4::Button::from_icon_name("document-save-symbolic");
    crate::compat::initial_tooltip(&download, &tr(lang, "preview.download"));
    a11y(&download, "pitex.preview.markdown.download", "preview.download");
    download.connect_clicked(|button| {
        let pdf_name = crate::app_ui::STATE.with(|s| {
            let slot = s.borrow();
            let state = slot.as_ref()?.try_borrow().ok()?;
            let url = state
                .model
                .active_document_url
                .as_ref()?
                .with_extension("pdf");
            Some(url.file_name()?.to_string_lossy().into_owned())
        });
        let window = button
            .root()
            .and_then(|r| r.downcast::<gtk4::Window>().ok());
        crate::compat::save_file(window.as_ref(), "Save PDF", pdf_name.as_deref(), |path| {
            crate::app_ui::STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(st) = state.try_borrow() {
                        st.markdown.export_pdf(&path);
                    }
                }
            });
        });
    });
    toolbar.append(&download);
    toolbar
}

#[cfg(all(test, feature = "markdown-preview"))]
mod tests {
    use super::*;

    /// `file:` URI for `path` — the same form relative document links
    /// resolve to (`file:///…` on Unix, `file:///C:/…` on Windows).
    fn file_uri(path: &Path) -> String {
        gio::File::for_path(path).uri().to_string()
    }

    #[test]
    fn preview_link_action_rules() {
        let dir = std::env::temp_dir().join(format!("pitex-link-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // https/mailto → external.
        assert_eq!(
            preview_link_action("https://example.com/x"),
            PreviewLinkAction::External("https://example.com/x".into())
        );
        assert_eq!(
            preview_link_action("mailto:a@b.c"),
            PreviewLinkAction::External("mailto:a@b.c".into())
        );
        // Unsupported schemes are ignored.
        assert_eq!(
            preview_link_action("javascript:alert(1)"),
            PreviewLinkAction::Ignore
        );
        assert_eq!(preview_link_action("ftp://x"), PreviewLinkAction::Ignore);
        // Missing file → ignore.
        assert_eq!(
            preview_link_action(&file_uri(&dir.join("missing.pdf"))),
            PreviewLinkAction::Ignore
        );
        // Plain file → open (the returned path is canonicalized).
        let pdf = dir.join("paper.pdf");
        std::fs::write(&pdf, b"%PDF").unwrap();
        assert_eq!(
            preview_link_action(&file_uri(&pdf)),
            PreviewLinkAction::OpenFile(std::fs::canonicalize(&pdf).unwrap())
        );
        // Percent-encoded path with spaces resolves.
        let spaced = dir.join("my notes.txt");
        std::fs::write(&spaced, b"x").unwrap();
        assert_eq!(
            preview_link_action(&file_uri(&spaced)),
            PreviewLinkAction::OpenFile(std::fs::canonicalize(&spaced).unwrap())
        );
        // A refused extension never reaches the OS handler (`sh` is on the
        // shared denylist; Windows adds its own executable/script list).
        let script = dir.join("run.sh");
        std::fs::write(&script, b"#!/bin/sh\n").unwrap();
        assert_eq!(
            preview_link_action(&file_uri(&script)),
            PreviewLinkAction::Refuse
        );
        // Exec-bit files are refused even with a harmless name — Unix only;
        // Windows has no mode bits (its denylist is extension-based).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let exec = dir.join("run");
            std::fs::write(&exec, b"#!/bin/sh\n").unwrap();
            let mut perms = std::fs::metadata(&exec).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&exec, perms).unwrap();
            assert_eq!(
                preview_link_action(&file_uri(&exec)),
                PreviewLinkAction::Refuse
            );
        }
        // .app bundle (a directory) is refused by extension alone.
        let app = dir.join("Evil.app");
        std::fs::create_dir(&app).unwrap();
        assert_eq!(
            preview_link_action(&file_uri(&app)),
            PreviewLinkAction::Refuse
        );
        // Symlinks are judged by their target, not the link's extension:
        // `paper.pdf -> run` (exec bit) and `doc.pdf -> Evil.app`
        // (directory bundle) are both refused, while `notes.pdf ->
        // real.pdf` opens the resolved target.
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let exec = dir.join("run");
            let link_script = dir.join("paper-link.pdf");
            symlink(&exec, &link_script).unwrap();
            assert_eq!(
                preview_link_action(&file_uri(&link_script)),
                PreviewLinkAction::Refuse
            );
            let link_app = dir.join("doc.pdf");
            symlink(&app, &link_app).unwrap();
            assert_eq!(
                preview_link_action(&file_uri(&link_app)),
                PreviewLinkAction::Refuse
            );
            let real = dir.join("real.pdf");
            std::fs::write(&real, b"%PDF").unwrap();
            let link_pdf = dir.join("notes.pdf");
            symlink(&real, &link_pdf).unwrap();
            assert_eq!(
                preview_link_action(&file_uri(&link_pdf)),
                PreviewLinkAction::OpenFile(std::fs::canonicalize(&real).unwrap())
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The render dedupe key — the engine round-trip is skipped only when
    /// every input is identical.
    #[test]
    fn render_dedup_skips_only_unchanged() {
        let mut dedup = RenderDedup::default();
        let url = Path::new("/tmp/doc.md");
        assert!(dedup.needs_render(url, 1, 14.0, false));
        assert!(!dedup.needs_render(url, 1, 14.0, false));
        assert!(dedup.needs_render(url, 2, 14.0, false));
        assert!(dedup.needs_render(url, 2, 16.0, false));
        assert!(dedup.needs_render(url, 2, 16.0, true));
        assert!(dedup.needs_render(Path::new("/tmp/other.md"), 2, 16.0, true));
        assert_eq!(dedup.url().as_deref(), Some(Path::new("/tmp/other.md")));
    }

    /// The preview→editor loop guard — muting holds inside the window and
    /// only there.
    #[test]
    fn scroll_guard_mutes_briefly() {
        let mut guard = ScrollGuard::default();
        assert!(!guard.muted());
        guard.arm();
        assert!(guard.muted());
        guard.until = Some(Instant::now() - Duration::from_millis(1));
        assert!(!guard.muted());
    }

    /// The shared navigation policy — first load allowed, then everything
    /// cancels; user intent routes the uri out of the preview.
    #[test]
    fn navigation_policy_allows_only_the_page_load() {
        let mut policy = NavigationPolicy::new();
        assert!(matches!(
            policy.navigate(Some("file:///preview.html"), false),
            NavVerdict::Allow
        ));
        // A non-user navigation afterwards cancels silently.
        assert!(matches!(
            policy.navigate(Some("https://example.com"), false),
            NavVerdict::Cancel
        ));
        // Link clicks route the uri.
        match policy.navigate(Some("https://example.com"), true) {
            NavVerdict::Route(uri) => assert_eq!(uri, "https://example.com"),
            _ => panic!("link click must route"),
        }
        // A user nav with no uri still cancels.
        assert!(matches!(policy.navigate(None, true), NavVerdict::Cancel));
        // New-window requests always route, never consume `initial`.
        match policy.new_window(Some("mailto:a@b.c")) {
            NavVerdict::Route(uri) => assert_eq!(uri, "mailto:a@b.c"),
            _ => panic!("new window must route"),
        }
        assert!(matches!(policy.new_window(None), NavVerdict::Cancel));
    }

    #[test]
    fn markdown_theme_resolver() {
        assert!(!markdown_preview_dark("system", false));
        assert!(markdown_preview_dark("system", true));
        assert!(!markdown_preview_dark("light", true));
        assert!(markdown_preview_dark("dark", false));
        assert!(!markdown_preview_dark("bogus", false));
    }
}
