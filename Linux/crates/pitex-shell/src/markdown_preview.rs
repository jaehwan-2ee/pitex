//! `MarkdownPreview` — the inspector's Markdown half, hosting the shared
//! renderer page (`Assets/markdown-preview`, inlined into
//! `Mac/Resources/markdown-preview.html`) inside a WebKitGTK 6.0 `WebView`.
//! The `markdown-preview` feature is off on the Ubuntu 22.04 and Windows
//! builds; those show the localized "not available" status page instead and
//! none of the WebKit paths compile in.

use gtk4::prelude::*;
use gtk4::{gio, glib};
#[cfg(feature = "markdown-preview")]
use webkit6::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::app_ui::a11y;
#[cfg(not(feature = "markdown-preview"))]
use crate::l10n::tr;

#[cfg(feature = "markdown-preview")]
const PREVIEW_HTML: &str =
    include_str!("../../../../Mac/Resources/markdown-preview.html");

/// Extensions that count as executable even without a mode bit — `.app` is
/// a directory, so this check runs before the file-type tests below.
const REFUSED_EXTENSIONS: &[&str] = &[
    "app", "command", "tool", "terminal", "workflow", "desktop", "appimage",
    "jar", "scpt", "applescript", "sh", "bash", "zsh", "fish", "csh", "ksh",
];

/// What a preview link click should do — the WebKit policy handler and the
/// unit tests share this one decision (`PreviewLinkAction` on macOS).
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
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| REFUSED_EXTENSIONS.iter().any(|r| e.eq_ignore_ascii_case(r)))
        .unwrap_or(false)
    {
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

#[cfg(feature = "markdown-preview")]
struct WebState {
    view: webkit6::WebView,
    /// First `load_html` finished — render calls queue until then.
    loaded: Cell<bool>,
    queued: RefCell<Option<String>>,
}

#[derive(Clone)]
pub struct MarkdownPreview {
    /// Inspector child — the webview container, or the unavailable page.
    pub widget: gtk4::Widget,
    /// Lazily created on the first render — non-Markdown sessions never pay
    /// the WebKit process cost.
    #[cfg(feature = "markdown-preview")]
    web: Rc<RefCell<Option<Rc<WebState>>>>,
    /// (document url, revision, font-size bits, dark) of the last render —
    /// unchanged keys skip the webview round-trip entirely.
    rendered: Rc<RefCell<Option<(PathBuf, u64, u64, bool)>>>,
    /// The debounced live render — cancel + re-arm like `AUTOSAVE_SOURCE`.
    render_source: Rc<RefCell<Option<glib::SourceId>>>,
    /// Mutes editor→preview pushes briefly after a preview-driven
    /// programmatic editor scroll (feedback-loop guard).
    editor_quiet: Rc<Cell<Option<Instant>>>,
}

impl MarkdownPreview {
    pub fn new(language: &'static str) -> Self {
        #[cfg(feature = "markdown-preview")]
        let widget: gtk4::Widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0).upcast();
        #[cfg(not(feature = "markdown-preview"))]
        let widget: gtk4::Widget = {
            let page = libadwaita::StatusPage::new();
            page.set_title(&tr(language, "preview.markdown.unavailable"));
            page.set_icon_name(Some("text-x-generic-symbolic"));
            page.set_vexpand(true);
            page.upcast()
        };
        let _ = language; // used only by the no-feature status page
        a11y(&widget, "pitex.preview.markdown", "preview.title");
        Self {
            widget,
            #[cfg(feature = "markdown-preview")]
            web: Rc::new(RefCell::new(None)),
            rendered: Rc::new(RefCell::new(None)),
            render_source: Rc::new(RefCell::new(None)),
            editor_quiet: Rc::new(Cell::new(None)),
        }
    }

    /// The url whose text is currently rendered — `None` before the first
    /// push, used to force a render on document activation.
    pub fn rendered_url(&self) -> Option<PathBuf> {
        self.rendered.borrow().as_ref().map(|k| k.0.clone())
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
        #[cfg(feature = "markdown-preview")]
        {
            let key = (url.to_path_buf(), revision, font_size.to_bits(), dark);
            if self.rendered.borrow().as_ref() == Some(&key) {
                return;
            }
            self.rendered.replace(Some(key));
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
            let web = self.ensure_webview();
            if !web.loaded.get() {
                *web.queued.borrow_mut() = Some(script);
                return;
            }
            web.view.evaluate_javascript(
                &script,
                None,
                None,
                gtk4::gio::Cancellable::NONE,
                |_| {},
            );
        }
        #[cfg(not(feature = "markdown-preview"))]
        let _ = (url, revision, text, font_size, dark);
    }

    /// Editor→preview scroll sync — `pitexScrollToLine(line)` on the page.
    /// No-op until the webview exists (non-Markdown sessions keep it lazy).
    pub fn scroll_to_line(&self, line: i32) {
        #[cfg(feature = "markdown-preview")]
        if let Some(web) = self.web.borrow().as_ref() {
            if web.loaded.get() {
                web.view.evaluate_javascript(
                    &format!("pitexScrollToLine({line})"),
                    None,
                    None,
                    gtk4::gio::Cancellable::NONE,
                    |_| {},
                );
            }
        }
        #[cfg(not(feature = "markdown-preview"))]
        let _ = line;
    }

    /// Called right before a preview-driven programmatic editor scroll —
    /// the vadjustment echo is muted for a short window afterwards.
    pub fn begin_programmatic_editor_scroll(&self) {
        self.editor_quiet
            .set(Some(Instant::now() + Duration::from_millis(120)));
    }

    /// True inside the loop-guard window after `begin_programmatic_editor_scroll`.
    pub fn editor_scroll_suppressed(&self) -> bool {
        self.editor_quiet
            .get()
            .map(|t| Instant::now() < t)
            .unwrap_or(false)
    }

    #[cfg(feature = "markdown-preview")]
    fn ensure_webview(&self) -> Rc<WebState> {
        if let Some(web) = self.web.borrow().as_ref() {
            return web.clone();
        }
        let manager = webkit6::UserContentManager::new();
        manager.register_script_message_handler("pitexScroll", None);
        manager.connect_script_message_received(Some("pitexScroll"), |_, value| {
            if !value.is_number() {
                return;
            }
            let line = value.to_double();
            crate::app_ui::STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(st) = state.try_borrow() {
                        st.scroll_editor_to_line(line);
                    }
                }
            });
        });
        let view = webkit6::WebView::builder()
            .user_content_manager(&manager)
            .build();
        let web = Rc::new(WebState {
            view,
            loaded: Cell::new(false),
            queued: RefCell::new(None),
        });
        // Strict navigation: only the initial `load_html` is allowed. Every
        // later navigation (link clicks, meta refresh, location=…) is
        // cancelled — clicked links go through `preview_link_action` instead.
        let initial = Cell::new(true);
        web.view.connect_decide_policy(move |_, decision, kind| {
            use webkit6::{NavigationType, PolicyDecisionType};
            let (uri, is_link) = match kind {
                PolicyDecisionType::NavigationAction
                | PolicyDecisionType::NewWindowAction => decision
                    .downcast_ref::<webkit6::NavigationPolicyDecision>()
                    .and_then(|d| d.navigation_action())
                    .map(|mut a| {
                        (
                            a.request().and_then(|r| r.uri()).map(|u| u.to_string()),
                            a.navigation_type() == NavigationType::LinkClicked,
                        )
                    })
                    .unwrap_or((None, false)),
                PolicyDecisionType::Response => {
                    decision.use_();
                    return true;
                }
                _ => {
                    decision.ignore();
                    return true;
                }
            };
            if kind == PolicyDecisionType::NavigationAction && initial.replace(false) {
                decision.use_();
                return true;
            }
            decision.ignore();
            if is_link || kind == PolicyDecisionType::NewWindowAction {
                if let Some(uri) = uri {
                    crate::app_ui::STATE.with(|s| {
                        if let Some(state) = s.borrow().as_ref() {
                            if let Ok(st) = state.try_borrow() {
                                st.open_preview_link(&uri);
                            }
                        }
                    });
                }
            }
            true
        });
        let weak = Rc::downgrade(&web);
        web.view.connect_load_changed(move |view, event| {
            if event != webkit6::LoadEvent::Finished {
                return;
            }
            let Some(web) = weak.upgrade() else { return };
            web.loaded.set(true);
            let queued = web.queued.borrow_mut().take();
            if let Some(script) = queued {
                view.evaluate_javascript(&script, None, None, gtk4::gio::Cancellable::NONE, |_| {});
            }
        });
        // file:/// origin so relative file:// images resolve.
        web.view.load_html(PREVIEW_HTML, Some("file:///"));
        *self.web.borrow_mut() = Some(web.clone());
        if let Some(container) = self.widget.downcast_ref::<gtk4::Box>() {
            web.view.set_vexpand(true);
            container.append(&web.view);
        }
        web
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

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
        assert_eq!(preview_link_action("javascript:alert(1)"), PreviewLinkAction::Ignore);
        assert_eq!(preview_link_action("ftp://x"), PreviewLinkAction::Ignore);
        // Missing file → ignore.
        assert_eq!(
            preview_link_action(&format!(
                "file://{}",
                dir.join("missing.pdf").display()
            )),
            PreviewLinkAction::Ignore
        );
        // Plain file → open (the returned path is canonicalized).
        let pdf = dir.join("paper.pdf");
        std::fs::write(&pdf, b"%PDF").unwrap();
        assert_eq!(
            preview_link_action(&format!("file://{}", pdf.display())),
            PreviewLinkAction::OpenFile(std::fs::canonicalize(&pdf).unwrap())
        );
        // Percent-encoded path with spaces resolves.
        let spaced = dir.join("my notes.txt");
        std::fs::write(&spaced, b"x").unwrap();
        let escaped = format!("file://{}", spaced.display()).replace(' ', "%20");
        assert_eq!(
            preview_link_action(&escaped),
            PreviewLinkAction::OpenFile(std::fs::canonicalize(&spaced).unwrap())
        );
        // Exec-bit file → refuse.
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("run.sh");
        std::fs::write(&script, b"#!/bin/sh\n").unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();
        assert_eq!(
            preview_link_action(&format!("file://{}", script.display())),
            PreviewLinkAction::Refuse
        );
        // .app bundle (a directory) is refused by extension alone.
        let app = dir.join("Evil.app");
        std::fs::create_dir(&app).unwrap();
        assert_eq!(
            preview_link_action(&format!("file://{}", app.display())),
            PreviewLinkAction::Refuse
        );
        // Symlinks are judged by their target, not the link's extension:
        // `paper.pdf -> run.sh` (exec bit) and `doc.pdf -> Evil.app`
        // (directory bundle) are both refused, while `notes.pdf ->
        // real.pdf` opens the resolved target.
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_script = dir.join("paper-link.pdf");
            symlink(&script, &link_script).unwrap();
            assert_eq!(
                preview_link_action(&format!("file://{}", link_script.display())),
                PreviewLinkAction::Refuse
            );
            let link_app = dir.join("doc.pdf");
            symlink(&app, &link_app).unwrap();
            assert_eq!(
                preview_link_action(&format!("file://{}", link_app.display())),
                PreviewLinkAction::Refuse
            );
            let real = dir.join("real.pdf");
            std::fs::write(&real, b"%PDF").unwrap();
            let link_pdf = dir.join("notes.pdf");
            symlink(&real, &link_pdf).unwrap();
            assert_eq!(
                preview_link_action(&format!("file://{}", link_pdf.display())),
                PreviewLinkAction::OpenFile(std::fs::canonicalize(&real).unwrap())
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
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
