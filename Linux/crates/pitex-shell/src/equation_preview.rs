//! Equation hover/caret preview for the GTK editor — the Linux host of
//! `editor_feature::equation_preview::EquationPreviewEngine` (the macOS
//! host is `Mac/Sources/Features/EquationPreview.swift`).
//!
//! One persistent WebKitGTK 6 view per window loads MathJax once from the
//! embedded `Assets/equation-preview` tree over `pitex-equation://preview/`
//! (a private `WebContext`, ephemeral network session, strict CSP and
//! navigation policy) and doubles as the display surface of a
//! non-autohide, non-focusable `GtkPopover` anchored to the source range.
//! Pure decisions (what, when, which result) live in the engine; this file
//! only performs its commands.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant, SystemTime};

use build_core::equation_exact::{ExactEquationProfile, ExactEquationRenderer};
use build_core::CancellationToken;
use editor_feature::equation_preview::{
    EquationPreviewAppearance, EquationPreviewCommand, EquationPreviewContent, EquationPreviewEngine,
    EquationPreviewPlacement, EquationPreviewPresentation, EquationPreviewScheme, EquationPreviewSettings,
    EquationPreviewUnavailableReason, EquationRenderFailure, EquationRenderOutcome, EquationRenderRequest,
    ExactEquationOutcome, ExactEquationRequest,
};
use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use language_core::equation_preview::{MathIncludeKey, MathIncludeRequest, MathPreviewHash, MathSourceScan, MathSourceScanner};
use webkit6::prelude::*;

// `(relative path, bytes)` for every html/js/css file of Assets/equation-preview.
include!(concat!(env!("OUT_DIR"), "/equation_assets.rs"));

pub(crate) const SCHEME: &str = "pitex-equation";
const PAGE_URI: &str = "pitex-equation://preview/renderer.html";
const RENDER_TIMEOUT_MS: u64 = 5_000;
const MAX_WIDTH: i32 = 720;
const MAX_HEIGHT: i32 = 360;
const BADGE_HEIGHT: i32 = 16;
const PADDING: i32 = 12;

/// The embedded asset for a `pitex-equation://preview/<path>` URI. Exact
/// table lookup: traversal, queries, fragments and escapes never match.
pub(crate) fn asset(uri: &str) -> Option<(&'static [u8], &'static str)> {
    let path = uri.strip_prefix("pitex-equation://preview/")?;
    if path.is_empty() || path.contains(['?', '#', '%', '\\']) || path.split('/').any(|c| c.is_empty() || c == "." || c == "..") {
        return None;
    }
    let (_, bytes) = EQUATION_ASSETS.iter().find(|(name, _)| *name == path)?;
    let mime = match path.rsplit('.').next()? {
        "html" => "text/html",
        "js" => "text/javascript",
        "css" => "text/css",
        _ => return None,
    };
    Some((bytes, mime))
}

/// A private web context whose only custom scheme serves the embedded tree.
fn renderer_context() -> webkit6::WebContext {
    let context = webkit6::WebContext::new();
    context.register_uri_scheme(SCHEME, |request| {
        let uri = request.uri().map(|u| u.to_string()).unwrap_or_default();
        match asset(&uri) {
            Some((bytes, mime)) => {
                let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from_static(bytes));
                request.finish(&stream, bytes.len() as i64, Some(mime));
            }
            None => {
                let mut error = glib::Error::new(gio::IOErrorEnum::NotFound, "not an equation-preview asset");
                request.finish_error(&mut error);
            }
        }
    });
    context
}

/// The renderer web view: ephemeral session, no storage/media/WebGL/devtools,
/// no window opening, transparent over the popover, never focusable.
fn renderer_view(context: &webkit6::WebContext) -> webkit6::WebView {
    let settings = webkit6::Settings::new();
    settings.set_enable_developer_extras(false);
    settings.set_enable_webgl(false);
    settings.set_enable_media(false);
    settings.set_enable_media_stream(false);
    settings.set_enable_html5_local_storage(false);
    settings.set_enable_html5_database(false);
    settings.set_enable_page_cache(false);
    settings.set_enable_back_forward_navigation_gestures(false);
    settings.set_allow_file_access_from_file_urls(false);
    settings.set_allow_universal_access_from_file_urls(false);
    settings.set_javascript_can_open_windows_automatically(false);
    let session = webkit6::NetworkSession::new_ephemeral();
    let view = webkit6::WebView::builder()
        .web_context(context)
        .network_session(&session)
        .settings(&settings)
        .build();
    view.set_can_focus(false);
    view.set_focusable(false);
    view.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    view
}

/// Unsaved text of an open document, by absolute path.
pub type OpenTextProvider = Box<dyn Fn(&Path) -> Option<String>>;

struct Renderer {
    view: webkit6::WebView,
    /// Page identity once `pitexEquation.ready` resolved.
    version: RefCell<Option<String>>,
}

pub struct EquationPreviewHost {
    engine: RefCell<EquationPreviewEngine>,
    view: RefCell<glib::WeakRef<sourceview5::View>>,
    buffer: RefCell<glib::WeakRef<sourceview5::Buffer>>,
    renderer: RefCell<Option<Rc<Renderer>>>,
    context: webkit6::WebContext,
    /// Consecutive renderer crashes/hangs since the last explicit re-enable
    /// (settings off→on); at the cap the feature reports unavailable and a
    /// slow >5s call still spends budget — deliberate ceiling, N4.
    renderer_failures: Cell<u32>,
    popover: gtk4::Popover,
    stack: gtk4::Stack,
    /// The page's measured size: WebKitWebView's natural size lags its
    /// content, so the view is an overlay on this exactly-sized box.
    web_sizer: gtk4::Box,
    web_overlay: gtk4::Overlay,
    picture: gtk4::Picture,
    picture_scroller: gtk4::ScrolledWindow,
    message: gtk4::Label,
    badge: gtk4::Label,
    content: gtk4::Box,
    epoch: Instant,
    revision: Cell<u64>,
    wake_generation: Cell<u64>,
    show_token: Cell<u64>,
    /// Content size the popup surface was mapped at.
    shown_size: Cell<(i32, i32)>,
    presentation: RefCell<Option<EquationPreviewPresentation>>,
    /// key → (job id, cancel): the id rejects a canceled job's late report
    /// that would otherwise clobber the replacement under the same key.
    exact_jobs: RefCell<HashMap<String, (u64, CancellationToken)>>,
    exact_job_id: Cell<u64>,
    exact_profile: RefCell<Option<ExactEquationProfile>>,
    build_command: RefCell<String>,
    document: RefCell<Option<PathBuf>>,
    root: RefCell<Option<PathBuf>>,
    project_root: RefCell<Option<PathBuf>>,
    external_stamps: RefCell<HashMap<String, Option<SystemTime>>>,
    has_preedit: Cell<bool>,
    /// Last pushed `enabled` flag: the renderer's crash budget resets only
    /// on the explicit false→true transition (re-enable), never on ordinary
    last_enabled: Cell<Option<bool>>,
    /// UI language for the badge/message/accessibility strings.
    language: Cell<&'static str>,
    /// The toplevel whose activation re-checks included files.
    hooked_window: RefCell<glib::WeakRef<gtk4::Window>>,
    open_text: RefCell<OpenTextProvider>,
    self_weak: RefCell<Weak<EquationPreviewHost>>,
}

impl EquationPreviewHost {
    pub fn new() -> Rc<Self> {
        let context = renderer_context();
        let badge = gtk4::Label::new(None);
        badge.set_xalign(1.0);
        badge.add_css_class("dim-label");
        badge.add_css_class("caption");
        badge.set_height_request(BADGE_HEIGHT);
        let message = gtk4::Label::new(None);
        message.add_css_class("dim-label");
        message.set_wrap(true);
        message.set_max_width_chars(60);
        message.set_margin_top(PADDING);
        message.set_margin_bottom(PADDING);
        message.set_margin_start(PADDING);
        message.set_margin_end(PADDING);
        let picture = gtk4::Picture::new();
        picture.set_can_shrink(false);
        let picture_scroller = gtk4::ScrolledWindow::new();
        // The paper keeps the popover's background as a 6pt frame, matching
        // the macOS `insetBy(padding/2)` exact body.
        picture_scroller.set_margin_top(PADDING / 2);
        picture_scroller.set_margin_bottom(PADDING / 2);
        picture_scroller.set_margin_start(PADDING / 2);
        picture_scroller.set_margin_end(PADDING / 2);
        picture_scroller.set_child(Some(&picture));
        let stack = gtk4::Stack::new();
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        stack.set_interpolate_size(false);
        let web_sizer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let web_overlay = gtk4::Overlay::new();
        web_overlay.set_child(Some(&web_sizer));
        stack.add_named(&web_overlay, Some("fast"));
        stack.add_named(&picture_scroller, Some("exact"));
        stack.add_named(&message, Some("message"));
        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .accessible_role(gtk4::AccessibleRole::Img)
            .build();
        content.add_css_class("pitex-equation-preview");
        content.append(&stack);
        content.append(&badge);
        let popover = gtk4::Popover::new();
        // Supplemental: never grabs input or focus, never closes itself.
        popover.set_autohide(false);
        popover.set_can_focus(false);
        popover.set_focusable(false);
        popover.set_has_arrow(true);
        popover.set_child(Some(&content));
        let host = Rc::new(Self {
            engine: RefCell::new(EquationPreviewEngine::new(
                EquationPreviewSettings::default(),
                EquationPreviewAppearance { scheme: EquationPreviewScheme::Light, font_size: 16.0 },
            )),
            view: RefCell::new(glib::WeakRef::new()),
            buffer: RefCell::new(glib::WeakRef::new()),
            renderer: RefCell::new(None),
            renderer_failures: Cell::new(0),
            context,
            popover,
            stack,
            web_sizer,
            web_overlay,
            picture,
            picture_scroller,
            message,
            badge,
            content,
            epoch: Instant::now(),
            revision: Cell::new(0),
            wake_generation: Cell::new(0),
            show_token: Cell::new(0),
            shown_size: Cell::new((0, 0)),
            presentation: RefCell::new(None),
            exact_jobs: RefCell::new(HashMap::new()),
            exact_job_id: Cell::new(0),
            exact_profile: RefCell::new(None),
            build_command: RefCell::new(String::new()),
            document: RefCell::new(None),
            root: RefCell::new(None),
            project_root: RefCell::new(None),
            external_stamps: RefCell::new(HashMap::new()),
            has_preedit: Cell::new(false),
            last_enabled: Cell::new(None),
            language: Cell::new("en"),
            hooked_window: RefCell::new(glib::WeakRef::new()),
            open_text: RefCell::new(Box::new(|_| None)),
            self_weak: RefCell::new(Weak::new()),
        });
        *host.self_weak.borrow_mut() = Rc::downgrade(&host);
        let pointer = gtk4::EventControllerMotion::new();
        let weak = Rc::downgrade(&host);
        pointer.connect_enter(move |_, _, _| {
            if let Some(host) = weak.upgrade() {
                debug_log(|| "pointer entered popover".into());
                host.run(|engine, now| engine.pointer_in_popover(true, now));
            }
        });
        let weak = Rc::downgrade(&host);
        pointer.connect_leave(move |_| {
            if let Some(host) = weak.upgrade() {
                debug_log(|| "pointer left popover".into());
                host.run(|engine, now| engine.pointer_in_popover(false, now));
            }
        });
        host.content.add_controller(pointer);
        host
    }

    pub fn set_open_text_provider(&self, provider: OpenTextProvider) {
        *self.open_text.borrow_mut() = provider;
    }

    /// UI language for the badge/message/accessibility strings.
    pub fn set_language(&self, language: &'static str) {
        self.language.set(language);
        self.content.update_property(&[gtk4::accessible::Property::Label(&crate::l10n::tr(
            language,
            "equation_preview.accessibility_label",
        ))]);
    }

    fn now(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    /// One engine call, then its commands, then the wake-up timer.
    fn run(&self, event: impl FnOnce(&mut EquationPreviewEngine, u64) -> Vec<EquationPreviewCommand>) {
        let now = self.now();
        let commands = {
            let Ok(mut engine) = self.engine.try_borrow_mut() else {
                // run() never re-enters while borrowed (apply runs after the
                // borrow drops); a failure here would silently drop the
                // event, so make it loud rather than quiet (N6).
                glib::g_warning!("pitex", "equation-preview: engine borrow failed, event dropped");
                return;
            };
            event(&mut engine, now)
        };
        self.apply(commands);
    }

    // ── Workspace inputs ────────────────────────────────────────────────

    pub fn set_project_root(&self, root: Option<PathBuf>) {
        if *self.project_root.borrow() != root {
            if let Some(token) = self.workspace_token() {
                ExactEquationRenderer::remove_workspace_artifacts(&token);
            }
        }
        *self.project_root.borrow_mut() = root;
    }

    /// Active document switch (or first open).
    pub fn set_document(&self, url: Option<PathBuf>) {
        *self.document.borrow_mut() = url.clone();
        let revision = self.revision.get() + 1;
        self.revision.set(revision);
        let file_id = url.map(|u| u.to_string_lossy().into_owned());
        self.run(|engine, now| engine.open_document(file_id, revision, now));
    }

    /// Settings, appearance, build command and main file — each a no-op
    /// when unchanged, so callers may push after any related change.
    pub fn sync(
        &self,
        mut settings: EquationPreviewSettings,
        appearance: EquationPreviewAppearance,
        build_command: &str,
        root: Option<PathBuf>,
    ) {
        let is_tex = self
            .document
            .borrow()
            .as_ref()
            .and_then(|d| d.extension())
            .map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "tex" | "ltx" | "sty" | "cls"))
            .unwrap_or(false);
        self.set_root_file(root);
        // Only an explicit re-enable (persisted off → on) re-arms the crash
        // budget and rebuilds a dead renderer — routine sync pushes (edits,
        // theme, tabs) must not replenish it.
        let toggled_on = settings.enabled && self.last_enabled.get() == Some(false);
        self.last_enabled.set(Some(settings.enabled));
        if toggled_on {
            self.reset_renderer_failures();
            if let Some(host) = self.self_weak.borrow().upgrade() {
                host.ensure_renderer();
            }
        }
        self.hook_window();
        settings.enabled &= is_tex;
        self.run(|engine, now| {
            let mut commands = engine.set_settings(settings, now);
            commands.extend(engine.set_appearance(appearance, now));
            commands
        });
        if *self.build_command.borrow() != build_command {
            *self.build_command.borrow_mut() = build_command.to_string();
            let profile = ExactEquationProfile::resolve(build_command);
            let identity = profile.as_ref().map(|p| p.identity());
            *self.exact_profile.borrow_mut() = profile;
            self.run(|engine, now| engine.set_exact_profile(identity, now));
        }
    }

    /// Returning to the window (after editing elsewhere) re-reads the
    /// included files the context uses; hooked once per toplevel.
    fn hook_window(&self) {
        let Some(window) = self.view.borrow().upgrade().and_then(|v| v.root()).and_downcast::<gtk4::Window>() else {
            return;
        };
        if self.hooked_window.borrow().upgrade().as_ref() == Some(&window) {
            return;
        }
        *self.hooked_window.borrow_mut() = window.downgrade();
        let weak = self.self_weak.borrow().clone();
        window.connect_is_active_notify(move |window| {
            if !window.is_active() {
                return;
            }
            if let Some(host) = weak.upgrade() {
                host.refresh_external_files();
            }
        });
    }

    fn set_root_file(&self, root: Option<PathBuf>) {
        if *self.root.borrow() == root {
            return;
        }
        *self.root.borrow_mut() = root.clone();
        let root_id = root.as_ref().map(|r| r.to_string_lossy().into_owned());
        if root.is_none() || root == *self.document.borrow() {
            self.run(|engine, now| engine.set_root_file(root_id, None, now));
            return;
        }
        let root = root.unwrap_or_default();
        let weak = self.self_weak.borrow().clone();
        glib::spawn_future_local(async move {
            let path = root.clone();
            // Open-buffer text is read inside the future: sync/attach call
            // this while holding an AppState borrow, and a try_borrow-fail
            // here would silently fall back to stale disk content (N3).
            let open = weak.upgrade().and_then(|host| (host.open_text.borrow())(&path));
            let scanned = gio::spawn_blocking(move || scan_disk(&path, open)).await.ok().flatten();
            let Some(host) = weak.upgrade() else { return };
            if *host.root.borrow() != Some(root) {
                return;
            }
            let scan = scanned.map(|(stamp, scan)| {
                host.external_stamps.borrow_mut().insert(root_id.clone().unwrap_or_default(), stamp);
                scan
            });
            host.run(|engine, now| engine.set_root_file(root_id, scan, now));
        });
    }

    pub fn request_exact(&self) {
        self.run(|engine, now| engine.request_exact(now));
    }

    /// App activation/save: re-read changed files the context depends on.
    pub fn refresh_external_files(&self) {
        let ids = self.engine.borrow().external_file_ids();
        if ids.is_empty() {
            return;
        }
        let stamps = self.external_stamps.borrow().clone();
        let weak = self.self_weak.borrow().clone();
        glib::spawn_future_local(async move {
            // Read open-buffer text inside the future (N3: callers can hold
            // an AppState borrow) and reuse scan_disk for its 4MiB cap (C12).
            let open: HashMap<String, String> = weak
                .upgrade()
                .map(|host| {
                    ids.iter()
                        .filter_map(|id| (host.open_text.borrow())(Path::new(id)).map(|t| (id.clone(), t)))
                        .collect()
                })
                .unwrap_or_default();
            let changed = gio::spawn_blocking(move || {
                ids.into_iter()
                    .filter_map(|id| {
                        let stamp = std::fs::metadata(&id).and_then(|m| m.modified()).ok();
                        if stamps.get(&id).copied().flatten() == stamp && stamps.contains_key(&id) {
                            return None;
                        }
                        let scan = scan_disk(Path::new(&id), open.get(&id).cloned()).map(|(_, scan)| scan);
                        Some((id, stamp, scan))
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default();
            let Some(host) = weak.upgrade() else { return };
            for (id, stamp, scan) in changed {
                host.external_stamps.borrow_mut().insert(id.clone(), stamp);
                host.run(|engine, now| engine.external_file_changed(&id, scan, now));
            }
        });
    }

    /// Workspace close: cancel TeX, hide, remove this workspace's temp tree.
    pub fn shutdown(&self) {
        for token in self.exact_jobs.borrow_mut().drain().map(|(_, (_, t))| t) {
            token.cancel();
        }
        self.hide_popover();
        self.set_document(None);
        if let Some(token) = self.workspace_token() {
            ExactEquationRenderer::remove_workspace_artifacts(&token);
        }
    }

    // ── Editor binding ──────────────────────────────────────────────────

    /// Rebind to the session's fresh view/buffer, mirroring
    /// `GhostCompletionCoordinator::attach`.
    pub fn attach(self: &Rc<Self>, view: &sourceview5::View, buffer: &sourceview5::Buffer) {
        self.hide_popover();
        if self.popover.parent().is_some() {
            self.popover.unparent();
        }
        self.popover.set_parent(view);
        *self.view.borrow_mut() = view.downgrade();
        *self.buffer.borrow_mut() = buffer.downgrade();
        self.has_preedit.set(false);
        self.ensure_renderer();

        let weak = Rc::downgrade(self);
        buffer.connect_changed(move |buffer| {
            let Some(host) = weak.upgrade() else { return };
            if !host.is_current_buffer(buffer) {
                return;
            }
            // Post-edit caret first, at the new revision, then the edit.
            debug_log(|| "edit".into());
            let revision = host.revision.get() + 1;
            host.revision.set(revision);
            let (location, length) = selection_of(buffer);
            host.run(|engine, now| {
                let mut commands = engine.selection_changed(location, length, revision, now);
                commands.extend(engine.text_changed(revision, now));
                commands
            });
        });
        let weak = Rc::downgrade(self);
        buffer.connect_mark_set(move |buffer, _iter, mark| {
            if mark != &buffer.get_insert() && mark != &buffer.selection_bound() {
                return;
            }
            let Some(host) = weak.upgrade() else { return };
            if !host.is_current_buffer(buffer) {
                return;
            }
            let (location, length) = selection_of(buffer);
            let revision = host.revision.get();
            host.run(|engine, now| engine.selection_changed(location, length, revision, now));
        });
        let weak = Rc::downgrade(self);
        view.connect_preedit_changed(move |_, text| {
            if let Some(host) = weak.upgrade() {
                host.has_preedit.set(!text.is_empty());
                let composing = !text.is_empty();
                host.run(|engine, now| engine.composition_changed(composing, now));
            }
        });

        let motion = gtk4::EventControllerMotion::new();
        let weak = Rc::downgrade(self);
        motion.connect_motion(move |controller, x, y| {
            let Some(host) = weak.upgrade() else { return };
            let location = host.character_at(controller, x, y);
            debug_log(|| format!("hover ({x:.0},{y:.0}) -> {location:?}"));
            host.run(|engine, now| engine.hover(location, now));
        });
        let weak = Rc::downgrade(self);
        motion.connect_leave(move |_| {
            if let Some(host) = weak.upgrade() {
                debug_log(|| "pointer left editor".into());
                host.run(|engine, now| engine.hover(None, now));
            }
        });
        view.add_controller(motion);

        let focus = gtk4::EventControllerFocus::new();
        let weak = Rc::downgrade(self);
        focus.connect_enter(move |_| {
            if let Some(host) = weak.upgrade() {
                host.run(|engine, now| engine.focus_changed(true, now));
            }
        });
        let weak = Rc::downgrade(self);
        focus.connect_leave(move |_| {
            if let Some(host) = weak.upgrade() {
                host.run(|engine, now| engine.focus_changed(false, now));
            }
        });
        view.add_controller(focus);

        // Escape in the capture phase, only while a preview is visible and
        // no input method owns the key.
        let keys = gtk4::EventControllerKey::new();
        keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _code, state| {
            let Some(host) = weak.upgrade() else { return glib::Propagation::Proceed };
            let modifiers = state
                & (gdk::ModifierType::SHIFT_MASK
                    | gdk::ModifierType::CONTROL_MASK
                    | gdk::ModifierType::ALT_MASK
                    | gdk::ModifierType::SUPER_MASK
                    | gdk::ModifierType::META_MASK);
            if key != gdk::Key::Escape || !modifiers.is_empty() || host.has_preedit.get() {
                return glib::Propagation::Proceed;
            }
            let consumed = {
                let now = host.now();
                let Ok(mut engine) = host.engine.try_borrow_mut() else {
                    glib::g_warning!("pitex", "equation-preview: engine borrow failed, escape dropped");
                    return glib::Propagation::Proceed;
                };
                // The engine's `consumed` decides: it is true only when a
                // preview is visible or pending, so find/completion still
                // receive Escape otherwise (N5).
                let (consumed, commands) = engine.escape(now);
                drop(engine);
                host.apply(commands);
                consumed
            };
            if consumed { glib::Propagation::Stop } else { glib::Propagation::Proceed }
        });
        view.add_controller(keys);

        let weak = Rc::downgrade(self);
        crate::app_ui::on_view_scroll(view, move || {
            if let Some(host) = weak.upgrade() {
                host.reposition();
            }
        });
        // Resizing moves the anchor without a scroll: adjustment page-size.
        let weak = Rc::downgrade(self);
        let hooks: Rc<RefCell<Vec<(gtk4::Adjustment, glib::SignalHandlerId)>>> = Rc::default();
        let rehook = Rc::new(move |view: &sourceview5::View| {
            for (adjustment, id) in hooks.borrow_mut().drain(..) {
                adjustment.disconnect(id);
            }
            for adjustment in [view.hadjustment(), view.vadjustment()].into_iter().flatten() {
                let weak = weak.clone();
                let id = adjustment.connect_notify_local(Some("page-size"), move |_, _| {
                    if let Some(host) = weak.upgrade() {
                        host.reposition();
                    }
                });
                hooks.borrow_mut().push((adjustment, id));
            }
        });
        rehook(view);
        for property in ["hadjustment", "vadjustment"] {
            let rehook = rehook.clone();
            view.connect_notify_local(Some(property), move |view, _| rehook(view));
        }

        let revision = self.revision.get() + 1;
        self.revision.set(revision);
        let file_id = self.document.borrow().as_ref().map(|d| d.to_string_lossy().into_owned());
        let (location, length) = selection_of(buffer);
        let focused = view.has_focus();
        self.run(|engine, now| {
            let mut commands = engine.open_document(file_id, revision, now);
            commands.extend(engine.focus_changed(focused, now));
            commands.extend(engine.selection_changed(location, length, revision, now));
            commands
        });
    }

    fn is_current_buffer(&self, buffer: &sourceview5::Buffer) -> bool {
        self.buffer.borrow().upgrade().as_ref() == Some(buffer)
    }

    /// The character under the pointer (char offset), or None over empty
    /// space, outside the text, or while a button is held (selecting).
    fn character_at(&self, controller: &gtk4::EventControllerMotion, x: f64, y: f64) -> Option<usize> {
        let buttons = gdk::ModifierType::BUTTON1_MASK | gdk::ModifierType::BUTTON2_MASK | gdk::ModifierType::BUTTON3_MASK;
        if controller.current_event_state().intersects(buttons) {
            return None;
        }
        let view = self.view.borrow().upgrade()?;
        let (bx, by) = view.window_to_buffer_coords(gtk4::TextWindowType::Widget, x as i32, y as i32);
        // iter_at_location rounds to the nearest boundary (the next character
        // in a glyph's right half); iter_at_position keeps the one under the
        // pointer and reports the trailing half separately.
        let (iter, _trailing) = view.iter_at_position(bx, by)?;
        let rect = view.iter_location(&iter);
        let inside = bx >= rect.x() && bx < rect.x() + rect.width().max(1) && by >= rect.y() && by < rect.y() + rect.height();
        inside.then(|| iter.offset().max(0) as usize)
    }

    // ── Renderer ────────────────────────────────────────────────────────

    /// Web processes that have died or hung since the last successful page
    /// load. Restarting is automatic a few times, then the preview just
    /// stays unavailable — an always-failing page must not respawn forever.
    const MAX_RENDERER_RESTARTS: u32 = 3;

    fn ensure_renderer(self: &Rc<Self>) {
        if self.renderer.borrow().is_some() || self.renderer_failures.get() >= Self::MAX_RENDERER_RESTARTS {
            return;
        }
        let view = renderer_view(&self.context);
        let renderer = Rc::new(Renderer { view: view.clone(), version: RefCell::new(None) });

        // Only the renderer page itself may load; everything else is refused.
        let loaded = Rc::new(Cell::new(false));
        let gate = loaded.clone();
        view.connect_decide_policy(move |_, decision, kind| {
            use webkit6::PolicyDecisionType;
            match kind {
                PolicyDecisionType::NavigationAction => {
                    let uri = decision
                        .downcast_ref::<webkit6::NavigationPolicyDecision>()
                        .and_then(|d| d.navigation_action())
                        .and_then(|mut a| a.request())
                        .and_then(|r| r.uri())
                        .map(|u| u.to_string());
                    if !gate.get() && uri.as_deref() == Some(PAGE_URI) {
                        decision.use_();
                    } else {
                        decision.ignore();
                    }
                }
                PolicyDecisionType::Response => decision.use_(),
                _ => decision.ignore(),
            }
            true
        });
        let weak = Rc::downgrade(self);
        let page = Rc::downgrade(&renderer);
        view.connect_load_changed(move |view, event| {
            if event != webkit6::LoadEvent::Finished {
                return;
            }
            loaded.set(true);
            let (Some(host), Some(page)) = (weak.upgrade(), page.upgrade()) else { return };
            let weak = Rc::downgrade(&host);
            call(view, "window.pitexEquation.ready.then((identity) => ({ identity }))", "{}".into(), 10_000, move |reply| {
                let Some(host) = weak.upgrade() else { return };
                if !host.is_current_renderer(&page) {
                    return;
                }
                match reply.and_then(|r| r.get("identity").and_then(|v| v.as_str()).map(str::to_string)) {
                    Some(identity) => {
                        debug_log(|| format!("renderer ready {identity}"));
                        *page.version.borrow_mut() = Some(identity.clone());
                        // No budget reset here: a page that reaches ready and
                        // then dies mid-render must still count to the cap.
                        host.run(|engine, now| engine.renderer_ready(&identity, now));
                    }
                    None => {
                        debug_log(|| "renderer page unavailable".into());
                        // Startup timeout/JS error spends the same bounded
                        // budget as a crash — otherwise a dead page sits in
                        // `renderer` forever and ensure_renderer never retries.
                        host.replace_renderer();
                    }
                }
            });
        });
        let weak = Rc::downgrade(self);
        let page = Rc::downgrade(&renderer);
        view.connect_web_process_terminated(move |_, _| {
            if let (Some(host), Some(page)) = (weak.upgrade(), page.upgrade()) {
                // A replacement's stale web process must never kill the
                // renderer that took its place.
                if host.is_current_renderer(&page) {
                    host.replace_renderer();
                }
            }
        });
        view.load_uri(PAGE_URI);
        self.web_overlay.add_overlay(&view);
        *self.renderer.borrow_mut() = Some(renderer);
    }

    fn is_current_renderer(&self, renderer: &Rc<Renderer>) -> bool {
        self.renderer.borrow().as_ref().is_some_and(|r| Rc::ptr_eq(r, renderer))
    }

    /// A hung or crashed page: end its process and start a fresh one.
    /// Bounded by `MAX_RENDERER_RESTARTS`; only an explicit re-enable
    /// (`reset_renderer_failures` from settings off→on) re-arms the budget —
    /// automatic pushes and re-attach do not, N4/B5.
    fn replace_renderer(&self) {
        self.renderer_failures.set(self.renderer_failures.get() + 1);
        // Take the slot before terminating so a synchronous
        // web-process-terminated re-entry sees None and goes stale instead of
        // killing the replacement mid-swap.
        let old = self.renderer.borrow_mut().take();
        self.run(|engine, _| engine.renderer_unavailable());
        if let Some(old) = old {
            self.web_overlay.remove_overlay(&old.view);
            old.view.terminate_web_process();
        }
        if self.renderer_failures.get() < Self::MAX_RENDERER_RESTARTS {
            if let Some(host) = self.self_weak.borrow().upgrade() {
                host.ensure_renderer();
            }
        }
    }

    /// User action after renderer failure: clears the restart budget so the
    /// next preview can try a fresh page again.
    fn reset_renderer_failures(&self) {
        self.renderer_failures.set(0);
    }

    fn start_render(&self, request: EquationRenderRequest) {
        #[cfg(test)]
        tests::ROOT_DEFINITION_OBSERVATION.with(|seen| {
            let (q, z) = request.definitions.iter().fold((false, false), |(q, z), definition|
                (q || definition == "\\newcommand{\\Q}{q}", z || definition == "\\newcommand{\\Q}{z}"));
            seen.set(Some((request.generation, q, z)));
        });
        let Some(renderer) = self.renderer.borrow().clone() else { return };
        let weak = self.self_weak.borrow().clone();
        if renderer.version.borrow().is_none() {
            let key = request.key.clone();
            glib::idle_add_local_once(move || {
                if let Some(host) = weak.upgrade() {
                    host.run(|engine, now| engine.fast_render_completed(&key, EquationRenderOutcome::Failed(EquationRenderFailure::RendererFailed), now));
                }
            });
            return;
        }
        let payload = serde_json::json!({
            "source": request.source,
            "displayMode": request.display_mode,
            "contextKey": request.context_key(),
            "definitions": request.definitions,
            "fontSize": request.font_size,
        });
        let key = request.key.clone();
        let page = renderer.clone();
        call(&renderer.view, "window.pitexEquation.render(input)", payload.to_string(), RENDER_TIMEOUT_MS, move |reply| {
            let Some(host) = weak.upgrade() else { return };
            let outcome = match reply {
                None => {
                    if host.is_current_renderer(&page) {
                        host.replace_renderer();
                    }
                    EquationRenderOutcome::Failed(EquationRenderFailure::RendererFailed)
                }
                Some(reply) => match (reply.get("ok").and_then(|v| v.as_bool()), reply.get("svg").and_then(|v| v.as_str())) {
                    (Some(true), Some(svg)) => EquationRenderOutcome::Svg(svg.to_string()),
                    _ => {
                        debug_log(|| format!("render failed {reply}"));
                        EquationRenderOutcome::Failed(match reply.get("error").and_then(|v| v.as_str()) {
                            Some("undefined") => EquationRenderFailure::UndefinedCommand,
                            Some("limit") => EquationRenderFailure::TooLarge,
                            _ => EquationRenderFailure::Invalid,
                        })
                    }
                },
            };
            host.run(|engine, now| engine.fast_render_completed(&key, outcome, now));
        });
    }

    // ── Exact TeX preview ───────────────────────────────────────────────

    fn workspace_token(&self) -> Option<String> {
        self.project_root.borrow().as_ref().map(|root| {
            MathPreviewHash::hex(MathPreviewHash::combine(MathPreviewHash::BASIS, &root.to_string_lossy()))
        })
    }

    fn start_exact(&self, request: ExactEquationRequest) {
        let profile = self.exact_profile.borrow().clone();
        let directory = self
            .root
            .borrow()
            .clone()
            .or_else(|| self.document.borrow().clone())
            .and_then(|p| p.parent().map(Path::to_path_buf));
        let (Some(profile), Some(token), Some(directory)) = (profile, self.workspace_token(), directory) else {
            self.run(|engine, now| engine.exact_render_completed(&request.key, ExactEquationOutcome::Failed, now));
            return;
        };
        let cancel = CancellationToken::new();
        // A stale completion under the same key must not remove or answer
        // the replacement job — pair the token with a generation id.
        let job_id = self.exact_job_id.get() + 1;
        self.exact_job_id.set(job_id);
        self.exact_jobs.borrow_mut().insert(request.key.clone(), (job_id, cancel.clone()));
        // The same TeX-aware PATH the real build resolves through — a bare
        // process PATH misses /usr/local/texlive on desktop launches.
        let environment = build_core::augmented_environment(&HashMap::new());
        let renderer = ExactEquationRenderer::new(profile, directory, environment, token);
        let weak = self.self_weak.borrow().clone();
        glib::spawn_future_local(async move {
            let document = request.document.clone();
            let job = cancel.clone();
            let pdf = gio::spawn_blocking(move || renderer.render(&document, Some(&job)).ok())
                .await
                .ok()
                .flatten();
            #[cfg(test)]
            tests::COMPLETED_EXACT_JOBS.with(|jobs| jobs.borrow_mut().push(job_id));
            let Some(host) = weak.upgrade() else { return };
            // This job was superseded (canceled then re-requested under the
            // same key): drop the outcome entirely.
            if !matches!(host.exact_jobs.borrow().get(&request.key), Some((id, _)) if *id == job_id) {
                return;
            }
            host.exact_jobs.borrow_mut().remove(&request.key);
            let outcome = pdf.map(ExactEquationOutcome::Pdf).unwrap_or(ExactEquationOutcome::Failed);
            host.run(|engine, now| engine.exact_render_completed(&request.key, outcome, now));
        });
    }

    // ── Include resolution ──────────────────────────────────────────────

    fn resolve(&self, requests: Vec<MathIncludeRequest>) {
        let document = self.document.borrow().clone();
        let weak = self.self_weak.borrow().clone();
        // Every requested key must get an answer — a dead scanning task would
        // otherwise leave the engine's pending-resolution keys forever.
        let keys: Vec<MathIncludeKey> = requests.iter().map(|r| r.key.clone()).collect();
        glib::spawn_future_local(async move {
            // Open-buffer text inside the future: engine commands can reach
            // here under the caller's AppState borrow, where try_borrow
            // would fail and silently yield stale disk content (N3).
            let open: HashMap<String, String> = weak
                .upgrade()
                .map(|host| {
                    requests
                        .iter()
                        .flat_map(|r| r.candidates.iter())
                        .filter_map(|c| (host.open_text.borrow())(Path::new(c)).map(|t| (c.clone(), t)))
                        .collect()
                })
                .unwrap_or_default();
            let resolved = gio::spawn_blocking(move || {
                let mut results: Vec<(MathIncludeKey, Option<String>)> = Vec::new();
                let mut scans: HashMap<String, MathSourceScan> = HashMap::new();
                let mut stamps: Vec<(String, Option<SystemTime>)> = Vec::new();
                for request in requests {
                    let found = request.candidates.iter().find(|c| open.contains_key(*c) || Path::new(c).is_file()).cloned();
                    if let Some(found) = &found {
                        let active = document.as_ref().is_some_and(|d| d.to_string_lossy() == found.as_str());
                        if !active && !scans.contains_key(found) {
                            if let Some((stamp, scan)) = scan_disk(Path::new(found), open.get(found).cloned()) {
                                stamps.push((found.clone(), stamp));
                                scans.insert(found.clone(), scan);
                            }
                        }
                    }
                    results.push((request.key, found));
                }
                (results, scans, stamps)
            })
            .await;
            let Some(host) = weak.upgrade() else { return };
            // A panicked worker still answers — every key as not-found.
            let (results, scans, stamps) = resolved.unwrap_or_else(|_| {
                (keys.into_iter().map(|key| (key, None)).collect(), HashMap::new(), Vec::new())
            });
            host.external_stamps.borrow_mut().extend(stamps);
            host.run(|engine, now| engine.includes_resolved(results, scans, now));
        });
    }

    // ── Commands ────────────────────────────────────────────────────────

    fn apply(&self, commands: Vec<EquationPreviewCommand>) {
        for command in commands {
            debug_log(|| describe(&command));
            match command {
                EquationPreviewCommand::Render(request) => self.start_render(request),
                EquationPreviewCommand::RenderExact(request) => self.start_exact(request),
                EquationPreviewCommand::CancelExact { key } => {
                    if let Some((_, token)) = self.exact_jobs.borrow().get(&key) {
                        token.cancel();
                    }
                }
                EquationPreviewCommand::Show(presentation) => self.show(presentation),
                EquationPreviewCommand::Hide => {
                    *self.presentation.borrow_mut() = None;
                    self.hide_popover();
                }
                EquationPreviewCommand::ResolveIncludes(requests) => self.resolve(requests),
            }
        }
        self.arm_wake();
    }

    /// One timer for the engine's earliest deadline; a generation counter
    /// retires superseded timers instead of removing sources.
    fn arm_wake(&self) {
        let generation = self.wake_generation.get() + 1;
        self.wake_generation.set(generation);
        let Some(deadline) = self.engine.try_borrow().ok().and_then(|e| e.next_deadline()) else { return };
        let delay = deadline.saturating_sub(self.now());
        let weak = self.self_weak.borrow().clone();
        glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            let Some(host) = weak.upgrade() else { return };
            if host.wake_generation.get() != generation {
                return;
            }
            let buffer = host.buffer.borrow().upgrade();
            let now = host.now().max(deadline);
            let commands = {
                let Ok(mut engine) = host.engine.try_borrow_mut() else {
                    glib::g_warning!("pitex", "equation-preview: engine borrow failed, poll dropped");
                    return;
                };
                let text = || buffer.as_ref().map(|b| b.text(&b.start_iter(), &b.end_iter(), true).to_string()).unwrap_or_default();
                engine.poll(now, &text)
            };
            host.apply(commands);
        });
    }

    // ── Presentation ────────────────────────────────────────────────────

    fn hide_popover(&self) {
        self.show_token.set(self.show_token.get() + 1);
        if self.popover.is_visible() {
            self.popover.popdown();
        }
    }

    fn show(&self, presentation: EquationPreviewPresentation) {
        *self.presentation.borrow_mut() = Some(presentation.clone());
        let token = self.show_token.get() + 1;
        self.show_token.set(token);
        let lang = self.language.get();
        match presentation.content.clone() {
            EquationPreviewContent::Fast { svg } => {
                let Some(renderer) = self.renderer.borrow().clone() else { return };
                let foreground = hex(&self.content.color());
                let font_size = self.engine.borrow().appearance().font_size;
                let payload = serde_json::json!({
                    "svg": svg,
                    "fontSize": font_size,
                    "theme": { "foreground": foreground, "background": "transparent" },
                });
                let weak = self.self_weak.borrow().clone();
                call(&renderer.view, "window.pitexEquation.show(input)", payload.to_string(), 2_000, move |reply| {
                    let Some(host) = weak.upgrade() else { return };
                    if host.show_token.get() != token {
                        return;
                    }
                    let size = reply.filter(|r| r.get("ok").and_then(|v| v.as_bool()) == Some(true)).and_then(|r| {
                        Some((r.get("width")?.as_f64()?, r.get("height")?.as_f64()?))
                    });
                    let Some((width, height)) = size else {
                        host.hide_popover();
                        return;
                    };
                    debug_log(|| format!("page content {width:.0}x{height:.0}"));
                    let width = (width.ceil() as i32).clamp(80, MAX_WIDTH);
                    let height = (height.ceil() as i32).clamp(1, MAX_HEIGHT - BADGE_HEIGHT);
                    host.web_sizer.set_size_request(width, height);
                    host.stack.set_visible_child_name("fast");
                    host.present(&crate::l10n::tr(lang, "equation_preview.fast_badge"));
                });
            }
            EquationPreviewContent::Exact { pdf } => {
                let weak = self.self_weak.borrow().clone();
                let scale = self.view.borrow().upgrade().map(|v| v.scale_factor()).unwrap_or(1).max(1) as f64;
                // TeX typesets at 10pt: magnify to the fast preview's size.
                let zoom = self.engine.borrow().appearance().font_size / 10.0;
                glib::spawn_future_local(async move {
                    let image = gio::spawn_blocking(move || exact_image(&pdf, scale * zoom)).await.ok().flatten();
                    let Some(host) = weak.upgrade() else { return };
                    if host.show_token.get() != token {
                        return;
                    }
                    let Some((pixels, width, height, stride)) = image else {
                        host.message.set_text(&crate::l10n::tr(lang, "equation_preview.unavailable"));
                        host.stack.set_visible_child_name("message");
                        host.present("");
                        return;
                    };
                    let texture = crate::pdf::texture_from_pixels(pixels.into(), width, height, stride);
                    host.picture.set_paintable(Some(&texture));
                    // Logical size: the bitmap was rendered at the view's scale.
                    let (w, h) = ((width as f64 / scale).ceil() as i32, (height as f64 / scale).ceil() as i32);
                    host.picture.set_size_request(w, h);
                    // Margins add PADDING around the request.
                    host.picture_scroller.set_size_request(w.clamp(80, MAX_WIDTH - PADDING), h.clamp(1, MAX_HEIGHT - BADGE_HEIGHT - PADDING));
                    host.stack.set_visible_child_name("exact");
                    host.present(&crate::l10n::tr(lang, "equation_preview.exact_badge"));
                });
            }
            EquationPreviewContent::Unavailable(reason) => {
                let key = if reason == EquationPreviewUnavailableReason::ExactUnavailable {
                    "equation_preview.exact_unavailable"
                } else {
                    "equation_preview.unavailable"
                };
                self.message.set_text(&crate::l10n::tr(lang, key));
                self.stack.set_visible_child_name("message");
                self.present("");
            }
        }
    }

    fn present(&self, badge: &str) {
        let Some(presentation) = self.presentation.borrow().clone() else { return };
        let Some(view) = self.view.borrow().upgrade() else { return };
        let Some(rect) = anchor_rect(&view, &presentation) else {
            self.hide_popover_only();
            return;
        };
        self.badge.set_text(badge);
        self.badge.set_visible(!badge.is_empty());
        let source = anchor_text(&view, &presentation);
        self.content.update_property(&[gtk4::accessible::Property::Description(&source)]);
        self.popover.set_position(match presentation.placement {
            EquationPreviewPlacement::Above => gtk4::PositionType::Top,
            EquationPreviewPlacement::Below => gtk4::PositionType::Bottom,
        });
        self.popover.set_pointing_to(Some(&rect));
        let (_, natural) = self.content.preferred_size();
        let size = (natural.width(), natural.height());
        if self.popover.is_visible() && self.shown_size.get() == size {
            self.popover.present();
        } else {
            // A visible popup surface grows with its child but never
            // shrinks; remap it at the new size.
            if self.popover.is_visible() {
                self.popover.popdown();
            }
            self.popover.popup();
        }
        self.shown_size.set(size);
        debug_log(|| format!("presented {}x{}", size.0, size.1));
    }

    /// Scrolled out of view: hide the surface but keep the presentation.
    fn hide_popover_only(&self) {
        if self.popover.is_visible() {
            self.popover.popdown();
        }
    }

    fn reposition(&self) {
        let Some(presentation) = self.presentation.borrow().clone() else { return };
        let Some(view) = self.view.borrow().upgrade() else { return };
        match anchor_rect(&view, &presentation) {
            Some(rect) if self.popover.is_visible() => self.popover.set_pointing_to(Some(&rect)),
            Some(_) => self.show(presentation), // scrolled back into view
            None => self.hide_popover_only(),
        }
    }
}

/// Developer diagnostics (`PITEX_EQUATION_PREVIEW_DEBUG=1`), never shown in the UI.
fn debug_log(message: impl FnOnce() -> String) {
    thread_local! {
        static ENABLED: bool = std::env::var_os("PITEX_EQUATION_PREVIEW_DEBUG").is_some();
    }
    if ENABLED.with(|enabled| *enabled) {
        thread_local! {
            static START: Instant = Instant::now();
        }
        let elapsed = START.with(|start| start.elapsed().as_secs_f64() * 1000.0);
        eprintln!("[equation-preview +{elapsed:.1}ms] {}", message());
    }
}

fn describe(command: &EquationPreviewCommand) -> String {
    match command {
        EquationPreviewCommand::Render(r) => format!("render gen={} display={} defs={} source={:?}", r.generation, r.display_mode, r.definitions.len(), r.source),
        EquationPreviewCommand::RenderExact(r) => format!("exact gen={} key={}", r.generation, r.key),
        EquationPreviewCommand::CancelExact { key } => format!("cancel exact {key}"),
        EquationPreviewCommand::Show(p) => format!(
            "show gen={} anchor={:?} trigger={:?} content={}",
            p.generation,
            p.anchor,
            p.trigger,
            match &p.content {
                EquationPreviewContent::Fast { svg } => format!("fast({} bytes)", svg.len()),
                EquationPreviewContent::Exact { pdf } => format!("exact({} bytes)", pdf.len()),
                EquationPreviewContent::Unavailable(reason) => format!("unavailable({reason:?})"),
            }
        ),
        EquationPreviewCommand::Hide => "hide".into(),
        EquationPreviewCommand::ResolveIncludes(requests) => format!("resolve {} includes", requests.len()),
    }
}

/// The selection as (char location, char length) — a collapsed caret is length 0.
fn selection_of(buffer: &sourceview5::Buffer) -> (usize, usize) {
    match buffer.selection_bounds() {
        Some((start, end)) => {
            let (a, b) = (start.offset().max(0) as usize, end.offset().max(0) as usize);
            (a.min(b), a.abs_diff(b))
        }
        None => (buffer.iter_at_mark(&buffer.get_insert()).offset().max(0) as usize, 0),
    }
}

/// The visible part of the region in widget coordinates, or None when it
/// scrolled out of view.
fn anchor_rect(view: &sourceview5::View, presentation: &EquationPreviewPresentation) -> Option<gdk::Rectangle> {
    let buffer = view.buffer();
    let count = buffer.char_count().max(0) as usize;
    let start = buffer.iter_at_offset(presentation.anchor.start.min(count) as i32);
    let end = buffer.iter_at_offset(presentation.anchor.end.min(count) as i32);
    let a = view.iter_location(&start);
    let b = view.iter_location(&end);
    let visible = view.visible_rect();
    let (x0, x1) = if a.y() == b.y() {
        (a.x(), (b.x() + b.width()).max(a.x() + 1))
    } else {
        (visible.x(), visible.x() + visible.width())
    };
    let (y0, y1) = (a.y(), b.y() + b.height());
    let top = y0.max(visible.y());
    let bottom = y1.min(visible.y() + visible.height());
    let left = x0.max(visible.x());
    let right = x1.min(visible.x() + visible.width());
    if bottom <= top || right <= left {
        return None;
    }
    let (wx, wy) = view.buffer_to_window_coords(gtk4::TextWindowType::Widget, left, top);
    Some(gdk::Rectangle::new(wx, wy, right - left, bottom - top))
}

fn anchor_text(view: &sourceview5::View, presentation: &EquationPreviewPresentation) -> String {
    let buffer = view.buffer();
    let count = buffer.char_count().max(0) as usize;
    let start = buffer.iter_at_offset(presentation.anchor.start.min(count) as i32);
    let end = buffer.iter_at_offset(presentation.anchor.end.min(count) as i32);
    buffer.text(&start, &end, true).to_string()
}

fn hex(color: &gdk::RGBA) -> String {
    let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", channel(color.red()), channel(color.green()), channel(color.blue()))
}

/// Size-bounded read + scan (open-buffer text wins); runs off the main loop.
fn scan_disk(path: &Path, open: Option<String>) -> Option<(Option<SystemTime>, MathSourceScan)> {
    if let Some(text) = open {
        return Some((None, MathSourceScanner::scan(&text)));
    }
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > 4 * 1024 * 1024 {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    Some((metadata.modified().ok(), MathSourceScanner::scan(&text)))
}

/// Page 1 on white paper at `scale`, cropped to the ink (+6pt margin).
/// Returns premultiplied BGRA (pixels, width, height, stride).
fn exact_image(pdf: &[u8], scale: f64) -> Option<(Vec<u8>, i32, i32, i32)> {
    let document = crate::pdf::PdfDocument::from_data(pdf)?;
    let (pixels, width, height, stride) = document.render_page(0, scale)?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, -1, -1);
    for y in 0..height {
        let row = &pixels[(y * stride) as usize..];
        for x in 0..width {
            let p = &row[(x * 4) as usize..(x * 4 + 3) as usize];
            if p.iter().any(|&c| c < 245) {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if max_x < min_x || max_y < min_y {
        return None;
    }
    let pad = (6.0 * scale) as i32;
    let (left, top) = ((min_x - pad).max(0), (min_y - pad).max(0));
    let (right, bottom) = ((max_x + pad).min(width - 1), (max_y + pad).min(height - 1));
    let (w, h) = (right - left + 1, bottom - top + 1);
    let mut cropped = Vec::with_capacity((w * h * 4) as usize);
    for y in top..=bottom {
        let start = (y * stride + left * 4) as usize;
        cropped.extend_from_slice(&pixels[start..start + (w * 4) as usize]);
    }
    Some((cropped, w, h, w * 4))
}

/// `body` (an expression of `input`, the parsed payload) is awaited and its
/// JSON returned; None on bridge error or timeout. The payload travels as a
/// GVariant argument — TeX is never spliced into script text.
fn call(
    view: &webkit6::WebView,
    body: &'static str,
    payload: String,
    timeout_ms: u64,
    done: impl FnOnce(Option<serde_json::Value>) + 'static,
) {
    let done: Rc<RefCell<Option<Box<dyn FnOnce(Option<serde_json::Value>)>>>> = Rc::new(RefCell::new(Some(Box::new(done))));
    let arguments = glib::VariantDict::new(None);
    arguments.insert("payload", payload);
    let script = format!("const input = JSON.parse(payload); return JSON.stringify(await ({body}));");
    let finish = done.clone();
    view.call_async_javascript_function(&script, Some(&arguments.end()), None, None, gio::Cancellable::NONE, move |result| {
        let reply = result
            .ok()
            .filter(|value| value.is_string())
            .and_then(|value| serde_json::from_str(value.to_str().as_str()).ok());
        let callback = finish.borrow_mut().take();
        if let Some(callback) = callback {
            callback(reply);
        }
    });
    glib::timeout_add_local_once(Duration::from_millis(timeout_ms), move || {
        // A timeout callback can terminate WebKit and synchronously re-enter
        // the completion above. Release the RefMut before invoking it.
        let callback = done.borrow_mut().take();
        if let Some(callback) = callback {
            callback(None);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        // Test-only barrier: observe the real completed worker callback,
        // then assert its consumer-visible effect on the replacement job.
        pub(super) static COMPLETED_EXACT_JOBS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
        pub(super) static ROOT_DEFINITION_OBSERVATION: Cell<Option<(u64, bool, bool)>> = const { Cell::new(None) };
    }

    #[test]
    fn external_file_read_cap_and_open_buffer_precedence() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let path = std::env::temp_dir().join(format!("pitex-read-cap-{}-{}.tex", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
        }
        let _cleanup = Cleanup(path.clone());
        file.write_all(b"\\newcommand{\\Q}{disk}\n").unwrap();
        file.set_len(4 * 1024 * 1024).unwrap();
        let (_, at_limit) = scan_disk(&path, None).expect("file at the exact read limit must remain usable");
        assert!(at_limit.definitions.iter().any(|definition| definition.math_jax_source == "\\newcommand{\\Q}{disk}"));
        file.set_len(4 * 1024 * 1024 + 1).unwrap();
        assert!(scan_disk(&path, None).is_none(), "oversized unopened file must not be read/scanned");
        let (_, open) = scan_disk(&path, Some("\\newcommand{\\Q}{unsaved}".into()))
            .expect("open-buffer text must remain authoritative over an oversized disk file");
        assert!(open.definitions.iter().any(|definition| definition.math_jax_source == "\\newcommand{\\Q}{unsaved}"));
        assert!(!open.definitions.iter().any(|definition| definition.math_jax_source == "\\newcommand{\\Q}{disk}"));
    }

    /// The scheme handler serves exactly the embedded page tree.
    #[test]
    fn scheme_serves_only_embedded_assets() {
        let (page, mime) = asset("pitex-equation://preview/renderer.html").expect("page");
        assert_eq!(mime, "text/html");
        assert!(std::str::from_utf8(page).unwrap().contains("Content-Security-Policy"));
        assert!(asset("pitex-equation://preview/vendor/mathjax/tex-svg-nofont.js").is_some());
        assert!(asset("pitex-equation://preview/vendor/mathjax/mathjax-tex-font/svg.js").is_some());
        for uri in [
            "pitex-equation://preview/../renderer.html",
            "pitex-equation://preview/vendor/../renderer.html",
            "pitex-equation://preview/renderer.html?x=1",
            "pitex-equation://preview/renderer%2ehtml",
            "pitex-equation://preview//renderer.html",
            "pitex-equation://other/renderer.html",
            "file:///etc/passwd",
            "pitex-equation://preview/vendor/VERSIONS",
            "pitex-equation://preview/vendor/manifest.json",
        ] {
            assert!(asset(uri).is_none(), "{uri}");
        }
    }

    fn pump(until: impl Fn() -> bool, limit: Duration) -> bool {
        let context = glib::MainContext::default();
        let started = Instant::now();
        while !until() {
            if started.elapsed() > limit {
                return false;
            }
            context.iteration(false);
            std::thread::sleep(Duration::from_millis(1));
        }
        true
    }

    /// `body` against the live page through the production bridge.
    fn evaluate(view: &webkit6::WebView, body: &'static str, payload: serde_json::Value) -> serde_json::Value {
        let slot: Rc<RefCell<Option<Option<serde_json::Value>>>> = Rc::new(RefCell::new(None));
        let sink = slot.clone();
        call(view, body, payload.to_string(), 20_000, move |reply| *sink.borrow_mut() = Some(reply));
        assert!(pump(|| slot.borrow().is_some(), Duration::from_secs(25)), "bridge call timed out");
        let reply = slot.borrow_mut().take().flatten();
        reply.expect("bridge returned no JSON")
    }

    fn render(view: &webkit6::WebView, source: &str, context: &str, definitions: &[&str]) -> serde_json::Value {
        evaluate(view, "window.pitexEquation.render(input)", serde_json::json!({
            "source": source, "displayMode": true, "contextKey": context,
            "definitions": definitions, "fontSize": 16,
        }))
    }

    fn error(reply: &serde_json::Value) -> Option<&str> {
        reply.get("error").and_then(|e| e.as_str())
    }

    #[track_caller]
    fn svg(reply: &serde_json::Value) -> &str {
        assert_eq!(reply.get("ok").and_then(|v| v.as_bool()), Some(true), "{reply}");
        reply.get("svg").and_then(|v| v.as_str()).expect("svg")
    }

    /// Only the previously unexercised host paths; no old contract replay.
    fn new_host_boundaries() {
        use std::os::unix::fs::DirBuilderExt;
        COMPLETED_EXACT_JOBS.with(|jobs| jobs.borrow_mut().clear());
        ROOT_DEFINITION_OBSERVATION.with(|seen| seen.set(None));
        assert!(std::process::Command::new("pdflatex").arg("--version").output().unwrap().status.success(),
                "new host exact-race proof requires the real TeX engine");
        let directory = std::env::temp_dir().join(format!("pitex-host-proof-{}-{}",
            std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::DirBuilder::new().mode(0o700).create(&directory).unwrap();
        let host = EquationPreviewHost::new();
        let buffer = sourceview5::Buffer::new(None);
        let editor = sourceview5::View::with_buffer(&buffer);
        let window = gtk4::Window::new();
        window.set_default_size(640, 420);
        window.set_child(Some(&editor));
        struct Cleanup { host: Rc<EquationPreviewHost>, window: gtk4::Window, directory: PathBuf }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                self.host.shutdown();
                if self.host.popover.parent().is_some() { self.host.popover.unparent(); }
                let page = self.host.renderer.borrow_mut().take();
                if let Some(page) = page {
                    self.host.web_overlay.remove_overlay(&page.view);
                    page.view.terminate_web_process();
                }
                self.window.close();
                let _ = std::fs::remove_dir_all(&self.directory);
            }
        }
        let _cleanup = Cleanup { host: host.clone(), window: window.clone(), directory: directory.clone() };
        host.attach(&editor, &buffer);
        let first = host.renderer.borrow().as_ref().unwrap().clone();
        // The scheme response cannot run before the main context is pumped.
        // Inject at document END, after the real immutable API is created;
        // preserve its real methods but hold only its startup ready reply.
        first.view.user_content_manager().unwrap().add_script(&webkit6::UserScript::new(
            "if (!window.pitexEquation || typeof window.pitexEquation.render !== 'function') throw Error('real renderer API absent'); window.pitexEquation = {...window.pitexEquation, ready: new Promise(() => {})}; window.__pitexStartupReadyHeld = true;",
            webkit6::UserContentInjectedFrames::TopFrame, webkit6::UserScriptInjectionTime::End,
            &[PAGE_URI], &[],
        ));
        let loaded = Rc::new(Cell::new(None));
        let flag = loaded.clone();
        first.view.connect_load_changed(move |_, event| {
            if event == webkit6::LoadEvent::Finished { flag.set(Some(Instant::now())); }
        });
        window.present();
        assert!(pump(|| loaded.get().is_some(), Duration::from_secs(15)), "startup probe page never loaded");
        let installed = evaluate(&first.view, "Promise.resolve({ held: window.__pitexStartupReadyHeld === true })", serde_json::json!({}));
        assert_eq!(installed["held"].as_bool(), Some(true), "startup fault injection did not actually run");
        assert!(first.version.borrow().is_none());
        assert_eq!(host.renderer_failures.get(), 0);
        let waiting = loaded.get().unwrap();
        assert!(pump(|| host.renderer.borrow().as_ref().is_some_and(|page|
            !Rc::ptr_eq(page, &first) && page.version.borrow().is_some()), Duration::from_secs(15)),
            "held startup ready reply must time out and load a replacement");
        assert!(waiting.elapsed() >= Duration::from_secs(9), "replacement did not exercise the 10s ready timeout");
        assert_eq!(host.renderer_failures.get(), 1, "startup timeout must spend exactly one failure");
        eprintln!("NEW_HOST_N2_PASS: real ready timeout -> one bounded replacement -> fresh ready ({:?})", waiting.elapsed());

        let root = directory.join("main.tex");
        let chapter = directory.join("chapter.tex");
        let gate = directory.join("gate.tex");
        let unsaved = "\\documentclass{article}\n\\input{gate}\n\\newcommand{\\Q}{q}\n\\begin{document}\n\\input{chapter}\n\\end{document}\n";
        std::fs::write(&root, unsaved.replace("{q}", "{z}")).unwrap();
        std::fs::write(&chapter, "$\\Q$").unwrap();
        std::fs::write(&gate, "% initially empty\n").unwrap();
        let state = Rc::new(RefCell::new(unsaved.to_string()));
        let open = state.clone();
        let open_root = root.clone();
        host.set_open_text_provider(Box::new(move |path| {
            if path != open_root.as_path() { return None; }
            open.try_borrow().ok().map(|text| text.clone())
        }));
        host.set_project_root(Some(directory.clone()));
        host.set_document(Some(chapter));
        buffer.set_text("$\\Q$");
        buffer.place_cursor(&buffer.iter_at_offset(2));
        editor.grab_focus();
        assert!(pump(|| editor.has_focus(), Duration::from_secs(5)), "current native editor must focus before input");
        let caller = state.borrow_mut();
        host.sync(EquationPreviewSettings::default(),
                  EquationPreviewAppearance { scheme: EquationPreviewScheme::Light, font_size: 16.0 },
                  "pdflatex -no-shell-escape {file}", Some(root.clone()));
        drop(caller);
        let text = || buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).to_string();
        assert!(pump(|| ROOT_DEFINITION_OBSERVATION.with(|seen| {
            let Some((generation, true, false)) = seen.get() else { return false };
            host.presentation.borrow().as_ref().is_some_and(|presentation|
                presentation.generation == generation && matches!(presentation.content, EquationPreviewContent::Fast { .. }))
        }), Duration::from_secs(10)),
            "actual root render must use unsaved Q=q, not disk Q=z, and present that generation's real SVG");
        eprintln!("NEW_HOST_N3_PASS: sync under caller RefMut -> deferred open buffer Q=q, not disk Q=z -> real fast preview");

        // C1: a real blocked pdflatex worker, not a simulated completion.
        // Disable/re-enable cancels it and retries the SAME exact key before
        // yielding the main context, forcing its callback to see the new id.
        std::fs::write(&gate,
            "\\newwrite\\proofFile\n\\immediate\\openout\\proofFile=first-started.txt\n\\immediate\\write\\proofFile{started}\n\\immediate\\closeout\\proofFile\n\\def\\proofSpin{\\proofSpin}\\proofSpin\n").unwrap();
        host.request_exact();
        let token = host.workspace_token().unwrap();
        let workspace = ExactEquationRenderer::workspace_root(&token);
        let marker = |name: &str| -> Option<PathBuf> {
            std::fs::read_dir(&workspace).ok()?.flatten().map(|entry| entry.path())
                .find(|path| path.join(name).is_file())
        };
        assert!(pump(|| marker("first-started.txt").is_some(), Duration::from_secs(10)),
                "first real TeX worker must reach its blocking gate");
        let first_directory = marker("first-started.txt").unwrap();
        let (key, old_id) = host.exact_jobs.borrow().iter().map(|(key, (id, _))| (key.clone(), *id)).next().unwrap();
        host.sync(EquationPreviewSettings { enabled: false, ..EquationPreviewSettings::default() },
                  EquationPreviewAppearance { scheme: EquationPreviewScheme::Light, font_size: 16.0 },
                  "pdflatex -no-shell-escape {file}", Some(root.clone()));
        assert!(host.exact_jobs.borrow().get(&key).unwrap().1.is_cancelled());
        std::fs::write(&gate, format!(
            "\\newwrite\\proofFile\n\\immediate\\openout\\proofFile=second-started.txt\n\\immediate\\write\\proofFile{{started}}\n\\immediate\\closeout\\proofFile\n\\newread\\proofGate\n\\loop\\openin\\proofGate=\"{}\"\\relax\\ifeof\\proofGate\\closein\\proofGate\\repeat\n\\closein\\proofGate\n",
            directory.join("release.tex").display())).unwrap();
        host.sync(EquationPreviewSettings::default(),
                  EquationPreviewAppearance { scheme: EquationPreviewScheme::Light, font_size: 16.0 },
                  "pdflatex -no-shell-escape {file}", Some(root));
        let deadline = host.engine.borrow().next_deadline().unwrap();
        let commands = host.engine.borrow_mut().poll(deadline, &text);
        let replacement_key = commands.iter().find_map(|command| match command {
            EquationPreviewCommand::RenderExact(request) => Some(request.key.clone()),
            _ => None,
        }).expect("re-enable must retry the explicitly requested exact target");
        assert_eq!(replacement_key, key, "race proof requires the same exact key");
        host.apply(commands);
        let new_id = host.exact_jobs.borrow().get(&key).unwrap().0;
        assert_ne!(old_id, new_id);
        assert!(pump(|| COMPLETED_EXACT_JOBS.with(|jobs| jobs.borrow().contains(&old_id))
            && marker("second-started.txt").is_some(), Duration::from_secs(10)),
            "old real cancellation callback must run while the new same-key TeX worker is blocked");
        assert!(!first_directory.exists(), "canceled request files must be cleaned up");
        assert_eq!(host.exact_jobs.borrow().get(&key).map(|(id, _)| *id), Some(new_id),
                   "old completion must not remove the replacement job");
        assert!(!COMPLETED_EXACT_JOBS.with(|jobs| jobs.borrow().contains(&new_id)),
                "new job must still be pending at the stale-completion boundary");
        std::fs::write(directory.join("release.tex"), "released\n").unwrap();
        assert!(pump(|| matches!(host.presentation.borrow().as_ref().map(|p| &p.content),
            Some(EquationPreviewContent::Exact { .. })) && host.picture.paintable().is_some(), Duration::from_secs(10)),
            "replacement same-key job must deliver its real PDF/native image after the old callback");
        assert!(host.exact_jobs.borrow().is_empty());
        assert!(COMPLETED_EXACT_JOBS.with(|jobs| jobs.borrow().contains(&new_id)));
        assert!(std::fs::read_dir(&workspace).unwrap().next().is_none(), "both request directories must be gone");
        eprintln!("NEW_HOST_C1_PASS: actual canceled same-key callback kept job {new_id}; replacement real TeX PDF/native image delivered");
        eprintln!("NEW_HOST_PROOFS_COMPLETE (C1/N2/N3 only; old renderer contract not replayed)");
    }

    /// The shipped page in real WebKitGTK: offline loading, definition
    /// isolation across contexts/documents and renders, error classes,
    /// bounds, the independent SVG sanitizer, and bounded web-process
    /// restart/replacement through the real host. Prints cold/warm timings.
    /// GTK can only init on one thread, so all webview coverage lives in this
    /// single test.
    #[test]
    #[ignore = "needs WebKitGTK 6.0, a display and pdflatex: xvfb-run -a cargo test -p pitex-shell --lib equation_preview -- --ignored"]
    fn renderer_page_contract() {
        gtk4::init().expect("gtk init");
        if std::env::var("PITEX_EQUATION_NEW_HOST_PROOFS_ONLY").as_deref() == Ok("1") {
            new_host_boundaries();
            return;
        }
        let context = renderer_context();
        let view = renderer_view(&context);
        let loads: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let sink = loads.clone();
        view.connect_resource_load_started(move |_, resource, _| {
            sink.borrow_mut().push(resource.uri().map(|u| u.to_string()).unwrap_or_default());
        });
        let finished = Rc::new(Cell::new(false));
        let flag = finished.clone();
        view.connect_load_changed(move |_, event| {
            if event == webkit6::LoadEvent::Finished {
                flag.set(true);
            }
        });
        let window = gtk4::Window::new();
        window.set_child(Some(&view));
        window.present();
        let cold = Instant::now();
        view.load_uri(PAGE_URI);
        assert!(pump(|| finished.get(), Duration::from_secs(30)), "page load");
        let identity = evaluate(&view, "window.pitexEquation.ready.then((identity) => ({ identity }))", serde_json::json!({}));
        let identity = identity["identity"].as_str().expect("identity").to_string();
        assert!(identity.contains("mathjax-4.1.3") && identity.contains("mathjax-tex@4.1.3"), "{identity}");
        let first = render(&view, "x^2", "k0", &[]);
        eprintln!("equation-preview cold load + first render: {:?}", cold.elapsed());
        assert!(svg(&first).starts_with("<svg"));

        // Document A defines \R; document B does not: no leak either way.
        let a = render(&view, "x \\in \\R", "docA", &["\\newcommand{\\R}{\\mathbb{R}}"]);
        assert!(svg(&a).contains("<path") || svg(&a).contains("<use"));
        assert_eq!(error(&render(&view, "x \\in \\R", "docB", &[])), Some("undefined"));
        svg(&render(&view, "x \\in \\R", "docA", &["\\newcommand{\\R}{\\mathbb{R}}"]));
        // Redefinition under a new context replaces the old body.
        let bold = render(&view, "\\R", "docA2", &["\\newcommand{\\R}{\\mathbf{R}}"]);
        assert_ne!(svg(&bold), svg(&render(&view, "\\R", "docA", &["\\newcommand{\\R}{\\mathbb{R}}"])));
        // A definition inside one equation stays local to that render.
        svg(&render(&view, "\\newcommand{\\Q}{q}\\Q", "docA", &["\\newcommand{\\R}{\\mathbb{R}}"]));
        assert_eq!(error(&render(&view, "\\Q", "docA", &["\\newcommand{\\R}{\\mathbb{R}}"])), Some("undefined"));
        // ...even when it tries to escape the per-render group.
        render(&view, "\\endgroup\\gdef\\Z{z}\\Z", "docA", &["\\newcommand{\\R}{\\mathbb{R}}"]);
        assert_eq!(error(&render(&view, "\\Z", "docA", &["\\newcommand{\\R}{\\mathbb{R}}"])), Some("undefined"));
        // Definition-mediated escape: a definition whose body pops the group
        // must not let its \gdef (or the other definitions) survive the next
        // render — definitions are replayed inside a fresh sandbox now.
        let hostile_defs = [
            "\\newcommand{\\R}{\\endgroup\\gdef\\W{w}\\mathbb{R}}",
            "\\newcommand{\\S}{s}",
        ];
        render(&view, "x \\in \\R", "docD", &hostile_defs);
        assert_eq!(error(&render(&view, "\\W", "docD", &hostile_defs)), Some("undefined"));
        // The same escape built at render time through an argument
        // (\x{endgroup}) rather than stored in a definition body (C2).
        let arg_defs = ["\\newcommand{\\x}[1]{\\csname #1\\endcsname}"];
        render(&view, "\\x{endgroup}\\newcommand{\\evil}{1}", "docE", &arg_defs);
        assert_eq!(error(&render(&view, "\\evil", "docE", &arg_defs)), Some("undefined"));
        // The honest definition is replayed fresh, so it still works.
        svg(&render(&view, "\\S", "docD", &hostile_defs));
        // Same context key, same defs, honest render still sees \R.
        svg(&render(&view, "x \\in \\R", "docD", &["\\newcommand{\\R}{\\mathbb{R}}"]));
        // One broken definition does not drop the others.
        svg(&render(&view, "\\S", "docC", &["\\newcommand{\\R}{", "\\newcommand{\\S}{s}"]));

        // Error classes the engine relies on.
        assert_eq!(error(&render(&view, "\\frac{", "k0", &[])), Some("tex"));
        assert_eq!(error(&render(&view, "\\begin{foo}x\\end{foo}", "k0", &[])), Some("undefined"));
        assert_eq!(error(&render(&view, "\\def\\a{\\a}\\a", "k0", &[])), Some("limit"));
        assert_eq!(error(&render(&view, &"x".repeat(40_000), "k0", &[])), Some("limit"));

        // Extensions autoload from the local tree only; Korean text renders.
        svg(&render(&view, "\\cancel{x} + \\text{한국어}", "k0", &[]));
        let aligned = format!("\\begin{{aligned}}{}\\end{{aligned}}", (0..60).map(|i| format!("a_{{{i}}} &= b_{{{i}}}\\\\")).collect::<String>());
        let warm = Instant::now();
        svg(&render(&view, &aligned, "k0", &[]));
        eprintln!("equation-preview 60-row aligned render: {:?}", warm.elapsed());
        let mut samples: Vec<Duration> = (0..20)
            .map(|i| {
                let started = Instant::now();
                svg(&render(&view, &format!("\\frac{{a_{i}}}{{b}} + \\sqrt{{x^{i}}}"), "k0", &[]));
                started.elapsed()
            })
            .collect();
        samples.sort();
        eprintln!("equation-preview warm render median {:?}, max {:?}", samples[samples.len() / 2], samples[samples.len() - 1]);
        // Per-render sandbox + definition replay on the warm path.
        let mut sandboxed: Vec<Duration> = (0..20)
            .map(|i| {
                let started = Instant::now();
                svg(&render(&view, &format!("x_{{{i}}} \\in \\R"), "docWarm", &["\\newcommand{\\R}{\\mathbb{R}}", "\\newcommand{\\Tr}{\\operatorname{Tr}}"]));
                started.elapsed()
            })
            .collect();
        sandboxed.sort();
        eprintln!("equation-preview sandboxed warm median {:?}, max {:?}", sandboxed[sandboxed.len() / 2], sandboxed[sandboxed.len() - 1]);
        // Representative definition-heavy context: 300 macro bodies, then
        // a render that uses two of them.
        let many_defs: Vec<String> = (0..300)
            .map(|i| format!("\\newcommand{{\\eqmac{}{}}}{{x_{{{i}}}}}", (b'A' + (i / 26) as u8) as char, (b'A' + (i % 26) as u8) as char))
            .collect();
        let def_refs: Vec<&str> = many_defs.iter().map(String::as_str).collect();
        let heavy = Instant::now();
        svg(&render(&view, "\\eqmacAA + \\eqmacLN", "docHeavy", &def_refs));
        eprintln!("equation-preview 300-definition replay + render: {:?}", heavy.elapsed());

        // Untrusted TeX: \href/\style/\class/\cssId from the html extension
        // leave nothing active in the emitted SVG.
        let hostile = svg(&render(
            &view,
            "\\require{html}\\href{javascript:alert(1)}{x}\\style{background:url(https://example.com/a)}{y}\\class{evil}{z}\\cssId{display}{w}",
            "k0",
            &[],
        ))
        .to_string();
        for needle in ["javascript:", "url(", "<a", "https://", "evil", "id=\"display\"", "<script", "<foreignObject", "onclick", "onload"] {
            assert!(!hostile.contains(needle), "{needle} survived: {hostile}");
        }
        // show() re-sanitizes whatever native code hands back and measures.
        let shown = evaluate(&view, "window.pitexEquation.show(input)", serde_json::json!({
            "svg": format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4ex\" height=\"2ex\" onload=\"alert(1)\"><script>alert(1)</script><foreignObject/><path d=\"M0 0L10 10\"/></svg>"),
            "fontSize": 16, "theme": { "foreground": "#000000", "background": "transparent" },
        }));
        assert_eq!(shown["ok"].as_bool(), Some(true));
        assert!(shown["width"].as_f64().unwrap() > 0.0 && shown["height"].as_f64().unwrap() > 0.0);
        let markup = evaluate(&view, "Promise.resolve({ html: document.getElementById('display').innerHTML })", serde_json::json!({}));
        let markup = markup["html"].as_str().unwrap();
        assert!(!markup.contains("script") && !markup.contains("onload") && !markup.contains("foreignObject"), "{markup}");
        // Everything the page ever fetched came from the embedded tree.
        let loads = loads.borrow();
        assert!(!loads.is_empty());
        assert!(loads.iter().all(|uri| uri.starts_with("pitex-equation://preview/")), "{loads:?}");
        assert!(loads.iter().any(|uri| uri.ends_with("/extensions/cancel.js")), "{loads:?}");
        window.close();

        // A1/B5 through the real host: killing the web process replaces the
        // renderer a bounded number of times, then it stays down until an
        // explicit reset re-arms the budget.
        let host = EquationPreviewHost::new();
        let buffer = sourceview5::Buffer::new(None);
        let editor = sourceview5::View::with_buffer(&buffer);
        let host_window = gtk4::Window::new();
        host_window.set_child(Some(&editor));
        host_window.present();
        host.attach(&editor, &buffer);
        let ready = || host.renderer.borrow().as_ref().is_some_and(|r| r.version.borrow().is_some());
        assert!(pump(ready, Duration::from_secs(15)), "first renderer never ready");
        // Each kill is one allowed restart; the third leaves it down (cap 3).
        for kill in 1..=3 {
            // Clone the view out: holding the borrow across terminate would
            // deadlock the synchronous web-process-terminated re-entry.
            let dead = host.renderer.borrow().as_ref().unwrap().view.clone();
            dead.terminate_web_process();
            if kill < 3 {
                assert!(pump(ready, Duration::from_secs(15)), "restart {kill} never ready");
            } else {
                assert!(
                    pump(|| host.renderer.borrow().is_none(), Duration::from_secs(10)),
                    "restart cap not honored"
                );
            }
        }
        // Ordinary sync pushes must not re-arm the budget (B5).
        let sync_once = |enabled: bool| {
            host.sync(
                EquationPreviewSettings::from_persisted(enabled, true, "above", "fast", 80),
                EquationPreviewAppearance { scheme: EquationPreviewScheme::Light, font_size: 16.0 },
                "pdflatex",
                None,
            );
        };
        sync_once(true);
        assert!(host.renderer.borrow().is_none(), "routine sync re-armed the budget");
        // The explicit off→on toggle is the recovery path.
        sync_once(false);
        sync_once(true);
        assert!(pump(ready, Duration::from_secs(15)), "off→on toggle did not recover");
        // Observed harness behavior (not a product claim): dropping this
        // widget tree after webview kills left the test process running past
        // `ok`, flooding popover warnings — the popover's live WebKitWebView
        // teardown needs main-context iterations the harness no longer runs.
        // Keep this legacy tree alive through the added cases and process
        // exit. External SIGKILL against the real app recovered with a
        // responsive main loop (L1); this workaround remains test-only.
        std::mem::forget(host);
        std::mem::forget(host_window);
        std::mem::forget(editor);
        std::mem::forget(buffer);
        new_host_boundaries();
    }
}
