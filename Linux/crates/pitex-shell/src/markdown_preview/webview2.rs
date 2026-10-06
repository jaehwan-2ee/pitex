//! WebView2 (Microsoft Edge, Evergreen runtime) engine — there is no
//! WebKitGTK on Windows, so the shared renderer page lives in an
//! `ICoreWebView2` hosted as a *child HWND* of the GTK toplevel.
//!
//! Airspace: a child HWND always draws above GTK's own rendering of the
//! same window. Popovers/menus/dialogs are unaffected — they're real Win32
//! popup windows — but in-surface overlay widgets can't paint over the web
//! view. Toasts are the only such overlay, so the shared `set_covered`
//! hint hides the controller while one is open.
//!
//! Bounds: GTK4 dropped the `size-allocate` signal, so a frame-clock tick
//! syncs the HWND's pixel rect with the placeholder's allocation — ticks
//! only run while the widget maps and frames render, and `sync` early-outs
//! when nothing moved.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::sync::OnceLock;

use gtk4::glib::translate::ToGlibPtr;
use gtk4::glib::{gobject_ffi, ControlFlow};
use gtk4::prelude::*;

use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    take_pwstr, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, NavigationCompletedEventHandler,
    NavigationStartingEventHandler, NewWindowRequestedEventHandler, PrintToPdfCompletedHandler,
    WebMessageReceivedEventHandler,
};
use windows::core::{s, w, Interface, BOOL, HRESULT, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

use super::{EngineHooks, NavVerdict, NavigationPolicy, PREVIEW_HTML};

// Only two GDK-Win32 entries are needed — pulling `gdk4-win32` for them
// drags in `StaticType` impls that reference symbols current GTK4 no
// longer exports (hcursor/display-manager plumbing), breaking the link.
extern "C" {
    fn gdk_win32_surface_get_type() -> usize;
    fn gdk_win32_surface_get_handle(
        surface: *mut gtk4::gdk::ffi::GdkSurface,
    ) -> *mut core::ffi::c_void;
}

/// The stub loader's only two exports this app needs — every other WebView2
/// API is a COM vtable call. Resolving them via `LoadLibraryW` (not the sys
/// crate's `#[link]` imports, which would make the DLL a hard dependency of
/// `pitex.exe` itself) means a missing loader only degrades the preview to
/// the status page; the app still starts.
type GetVersionFn = unsafe extern "system" fn(PCWSTR, *mut PWSTR) -> HRESULT;
type CreateEnvironmentFn = unsafe extern "system" fn(
    PCWSTR,
    PCWSTR,
    *mut core::ffi::c_void,
    *mut core::ffi::c_void,
) -> HRESULT;

struct Loader {
    get_version: GetVersionFn,
    create_environment: CreateEnvironmentFn,
}

/// Resolved once and kept for the process lifetime — WebView2's callbacks
/// can outlive any individual call path, so the module is never freed.
fn loader() -> Option<&'static Loader> {
    static LOADER: OnceLock<Option<Loader>> = OnceLock::new();
    LOADER
        .get_or_init(|| {
            let module = unsafe { LoadLibraryW(w!("WebView2Loader.dll")) }.ok()?;
            Some(Loader {
                get_version: unsafe {
                    core::mem::transmute(GetProcAddress(
                        module,
                        s!("GetAvailableCoreWebView2BrowserVersionString"),
                    )?)
                },
                create_environment: unsafe {
                    core::mem::transmute(GetProcAddress(
                        module,
                        s!("CreateCoreWebView2EnvironmentWithOptions"),
                    )?)
                },
            })
        })
        .as_ref()
}

/// Probe for the Evergreen runtime without creating it — the caller shows
/// the status page when this is false and no engine is ever built. Covers
/// both "loader DLL absent" and "runtime not installed".
pub(super) fn runtime_available() -> bool {
    let Some(loader) = loader() else {
        return false;
    };
    let mut version = PWSTR::null();
    if unsafe { (loader.get_version)(PCWSTR::null(), &mut version) }.is_err() {
        return false;
    }
    !take_pwstr(version).is_empty()
}

/// The widget half of the preview — `placeholder` anchors the child HWND's
/// bounds in the GTK layout; everything COM lives behind `inner` so the
/// async callbacks can upgrade a `Weak`.
pub(super) struct Engine {
    inner: Rc<EngineInner>,
}

struct EngineInner {
    /// Inert GTK child whose allocation the HWND mirrors.
    placeholder: gtk4::Box,
    /// Set once `CreateCoreWebView2Controller` completes.
    live: RefCell<Option<Live>>,
    /// Environment→controller creation is in flight.
    creating: Cell<bool>,
    /// An in-surface overlay (toast) is open — hide the HWND.
    covered: Cell<bool>,
    hooks: EngineHooks,
    policy: RefCell<NavigationPolicy>,
}

struct Live {
    controller: ICoreWebView2Controller,
    webview: ICoreWebView2,
}

impl Drop for EngineInner {
    fn drop(&mut self) {
        if let Some(live) = self.live.borrow_mut().take() {
            let _ = unsafe { live.controller.Close() };
        }
    }
}

impl Engine {
    /// Attach the placeholder under `host`; the controller itself is created
    /// lazily once the placeholder maps (its toplevel HWND exists by then).
    /// `None` means the WebView2 runtime is missing — the status-page path.
    pub(super) fn create(host: &gtk4::Box, hooks: EngineHooks) -> Option<Self> {
        if !runtime_available() {
            return None;
        }
        let placeholder = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        placeholder.set_vexpand(true);
        placeholder.set_hexpand(true);
        let inner = Rc::new(EngineInner {
            placeholder: placeholder.clone(),
            live: RefCell::new(None),
            creating: Cell::new(false),
            covered: Cell::new(false),
            hooks,
            policy: RefCell::new(NavigationPolicy::new()),
        });
        // Signals connect before the append — adding to an already-mapped
        // tree maps the widget synchronously.
        {
            let weak = Rc::downgrade(&inner);
            placeholder.connect_map(move |_| {
                if let Some(inner) = weak.upgrade() {
                    inner.ensure_controller();
                    inner.sync();
                }
            });
        }
        {
            let weak = Rc::downgrade(&inner);
            placeholder.connect_unmap(move |_| {
                if let Some(inner) = weak.upgrade() {
                    inner.hide();
                }
            });
        }
        {
            let weak = Rc::downgrade(&inner);
            // `_local`: the closure's Weak isn't Send — notify handlers on
            // glib Objects otherwise demand Send + Sync.
            placeholder.connect_notify_local(Some("scale-factor"), move |_, _| {
                if let Some(inner) = weak.upgrade() {
                    inner.sync();
                }
            });
        }
        {
            let weak = Rc::downgrade(&inner);
            let _ = placeholder.add_tick_callback(move |_, _| match weak.upgrade() {
                Some(inner) => {
                    inner.sync();
                    ControlFlow::Continue
                }
                None => ControlFlow::Break,
            });
        }
        host.append(&placeholder);
        Some(Self { inner })
    }

    /// `pitexRender`/`pitexScrollToLine` — fire-and-forget page JavaScript.
    /// A `None` `live` means creation is still in flight; the shared queue
    /// keeps the newest render until `loaded` fires.
    pub(super) fn eval(&self, script: &str) {
        let live = self.inner.live.borrow();
        if let Some(live) = live.as_ref() {
            let script = HSTRING::from(script);
            let _ = unsafe {
                live.webview
                    .ExecuteScript(&script, None::<&ICoreWebView2ExecuteScriptCompletedHandler>)
            };
        }
    }

    /// WebView2's own print path — same paginated output the shared print
    /// CSS produces. `failed_message` covers the bare `is_successful =
    /// false` case which carries no HRESULT text.
    pub(super) fn print_to_pdf(&self, path: &Path, failed_message: &str) {
        let webview = self
            .inner
            .live
            .borrow()
            .as_ref()
            .map(|live| live.webview.clone());
        let Some(webview) = webview else {
            return;
        };
        let Ok(webview) = webview.cast::<ICoreWebView2_7>() else {
            return;
        };
        let result = HSTRING::from(path);
        let inner = Rc::downgrade(&self.inner);
        let failed = failed_message.to_string();
        let _ = unsafe {
            webview.PrintToPdf(
                &result,
                None::<&ICoreWebView2PrintSettings>,
                &PrintToPdfCompletedHandler::create(Box::new(move |result, success| {
                    let message = match result {
                        Err(error) => Some(error.message().to_string()),
                        Ok(()) if !success => Some(failed.clone()),
                        Ok(()) => None,
                    };
                    if let (Some(inner), Some(message)) = (inner.upgrade(), message) {
                        inner.toast(message);
                    }
                    Ok(())
                })),
            )
        };
    }

    /// Toast cover hint — see the module docs on airspace.
    pub(super) fn set_covered(&self, covered: bool) {
        self.inner.covered.set(covered);
        self.inner.sync();
    }
}

impl EngineInner {
    /// The toplevel's HWND — `None` until the placeholder is realized.
    /// The GType check keeps a non-Win32 surface (e.g. a WSLg/Wayland
    /// backend) from reaching the Win32 getter: `None`, not UB.
    fn parent_hwnd(&self) -> Option<HWND> {
        let surface = self.placeholder.native()?.surface()?;
        unsafe {
            let raw: *mut gtk4::gdk::ffi::GdkSurface = surface.to_glib_none().0;
            let instance = raw as *mut gobject_ffi::GTypeInstance;
            if gobject_ffi::g_type_check_instance_is_a(instance, gdk_win32_surface_get_type())
                == 0
            {
                return None;
            }
            let hwnd = gdk_win32_surface_get_handle(raw);
            (!hwnd.is_null()).then_some(HWND(hwnd))
        }
    }

    /// Kick the async environment→controller chain. `creating` suppresses
    /// re-entry; `live` short-circuits after success. A failure leaves the
    /// placeholder inert — the preview column stays empty, never crashes.
    fn ensure_controller(self: &Rc<Self>) {
        if self.creating.replace(true) || self.live.borrow().is_some() {
            return;
        }
        let (Some(parent), Some(loader)) = (self.parent_hwnd(), loader()) else {
            // Surface not realized yet, or the loader DLL vanished between
            // the `new()` probe and now — the next map retries the former,
            // the latter stays inert.
            self.creating.set(false);
            return;
        };
        // The loader needs an STA thread — GTK's main loop is one already
        // (S_FALSE), and an MTA (RPC_E_CHANGED_MODE) just makes the
        // environment call fail into the same inert path.
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        // Evergreen runtime only; per-user data under %LOCALAPPDATA%\pitex.
        let user_data = dirs::data_local_dir().map(|dir| {
            let dir = dir.join("pitex").join("WebView2");
            let _ = std::fs::create_dir_all(&dir);
            HSTRING::from(dir.as_os_str())
        });
        let empty = HSTRING::from("");
        let user_data = PCWSTR(user_data.as_ref().unwrap_or(&empty).as_ptr());
        let weak = Rc::downgrade(self);
        let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |result, environment| {
                if let Some(inner) = weak.upgrade() {
                    inner.environment_ready(result, environment, parent);
                }
                Ok(())
            },
        ));
        if unsafe {
            (loader.create_environment)(
                PCWSTR::null(),
                user_data,
                std::ptr::null_mut(), // no ICoreWebView2EnvironmentOptions
                handler.as_raw(),
            )
        }
        .is_err()
        {
            // Synchronous failure — allow a retry on the next map.
            self.creating.set(false);
        }
    }

    fn environment_ready(
        self: &Rc<Self>,
        result: windows::core::Result<()>,
        environment: Option<ICoreWebView2Environment>,
        parent: HWND,
    ) {
        let (Ok(()), Some(environment)) = (result, environment) else {
            return;
        };
        let weak = Rc::downgrade(self);
        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |result, controller| {
                if let Some(inner) = weak.upgrade() {
                    inner.controller_ready(result, controller);
                }
                Ok(())
            },
        ));
        let _ = unsafe { environment.CreateCoreWebView2Controller(parent, &handler) };
    }

    fn controller_ready(
        self: &Rc<Self>,
        result: windows::core::Result<()>,
        controller: Option<ICoreWebView2Controller>,
    ) {
        let (Ok(()), Some(controller)) = (result, controller) else {
            return;
        };
        // The controller stays in raw-pixel bounds mode (the default for a
        // non-composition controller); monitor-scale detection lets the
        // runtime re-rasterize when the window crosses DPIs.
        if let Ok(controller3) = controller.cast::<ICoreWebView2Controller3>() {
            unsafe {
                let _ = controller3.SetShouldDetectMonitorScaleChanges(true);
            }
        }
        let Ok(webview) = (unsafe { controller.CoreWebView2() }) else {
            return;
        };
        Self::lock_down(&webview);
        self.add_bridge_shim(&webview);
        self.wire_events(&webview);
        *self.live.borrow_mut() = Some(Live {
            controller,
            webview: webview.clone(),
        });
        self.sync();
        Self::navigate(&webview);
    }

    /// Match the WebKit build's fixed page: no DevTools, context menus, zoom
    /// UI, status bar, script dialogs, error pages, or host objects. Web
    /// messaging stays on — `pitexScroll` rides it.
    fn lock_down(webview: &ICoreWebView2) {
        let Ok(settings) = (unsafe { webview.Settings() }) else {
            return;
        };
        unsafe {
            let _ = settings.SetAreDevToolsEnabled(false);
            let _ = settings.SetAreDefaultContextMenusEnabled(false);
            let _ = settings.SetIsZoomControlEnabled(false);
            let _ = settings.SetIsStatusBarEnabled(false);
            let _ = settings.SetAreDefaultScriptDialogsEnabled(false);
            let _ = settings.SetIsBuiltInErrorPageEnabled(false);
            let _ = settings.SetAreHostObjectsAllowed(false);
            let _ = settings.SetIsWebMessageEnabled(true);
        }
    }

    /// `window.webkit.messageHandlers.pitexScroll.postMessage` →
    /// `window.chrome.webview.postMessage` — injected at document-created
    /// so the shared page is served verbatim, never forked.
    fn add_bridge_shim(&self, webview: &ICoreWebView2) {
        const SHIM: &str = "window.webkit={messageHandlers:{pitexScroll:{postMessage:function(m){window.chrome.webview.postMessage(m);}}}};";
        let _ = unsafe {
            webview.AddScriptToExecuteOnDocumentCreated(
                &HSTRING::from(SHIM),
                None::<&ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler>,
            )
        };
    }

    fn wire_events(self: &Rc<Self>, webview: &ICoreWebView2) {
        let mut token = 0i64;
        // Strict navigation: only the initial page load proceeds; anything
        // later is cancelled — user-initiated ones take the
        // `preview_link_action` route instead.
        let weak = Rc::downgrade(self);
        let handler = NavigationStartingEventHandler::create(Box::new(move |_, args| {
            let (Some(inner), Some(args)) = (weak.upgrade(), args) else {
                return Ok(());
            };
            let mut uri = PWSTR::null();
            unsafe { args.Uri(&mut uri)? };
            let uri = take_pwstr(uri);
            let mut user = BOOL(0);
            unsafe { args.IsUserInitiated(&mut user)? };
            let uri = (!uri.is_empty()).then_some(uri);
            match inner
                .policy
                .borrow_mut()
                .navigate(uri.as_deref(), user.as_bool())
            {
                NavVerdict::Allow => {}
                NavVerdict::Cancel => unsafe { args.SetCancel(true)? },
                NavVerdict::Route(uri) => {
                    unsafe { args.SetCancel(true)? };
                    inner.open_link(uri);
                }
            }
            Ok(())
        }));
        let _ = unsafe { webview.add_NavigationStarting(&handler, &mut token) };
        // `target=_blank`/`window.open` — handled here means WebView2 never
        // opens its own window; the uri takes the same link route.
        let weak = Rc::downgrade(self);
        let handler = NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
            let (Some(inner), Some(args)) = (weak.upgrade(), args) else {
                return Ok(());
            };
            unsafe { args.SetHandled(true)? };
            let mut uri = PWSTR::null();
            unsafe { args.Uri(&mut uri)? };
            let uri = take_pwstr(uri);
            if !uri.is_empty() {
                inner.open_link(uri);
            }
            Ok(())
        }));
        let _ = unsafe { webview.add_NewWindowRequested(&handler, &mut token) };
        // Page load finished → mark ready and drain the queued render.
        let weak = Rc::downgrade(self);
        let handler = NavigationCompletedEventHandler::create(Box::new(move |_, args| {
            let (Some(inner), Some(args)) = (weak.upgrade(), args) else {
                return Ok(());
            };
            let mut success = BOOL(0);
            unsafe { args.IsSuccess(&mut success)? };
            if success.as_bool() {
                inner.loaded();
            }
            Ok(())
        }));
        let _ = unsafe { webview.add_NavigationCompleted(&handler, &mut token) };
        // Preview→editor scroll sync — the shim posts the page's
        // `pitexScroll` payload (a bare JSON number).
        let weak = Rc::downgrade(self);
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            let (Some(inner), Some(args)) = (weak.upgrade(), args) else {
                return Ok(());
            };
            let mut json = PWSTR::null();
            unsafe { args.WebMessageAsJson(&mut json)? };
            if let Ok(line) = serde_json::from_str::<f64>(&take_pwstr(json)) {
                inner.scroll(line);
            }
            Ok(())
        }));
        let _ = unsafe { webview.add_WebMessageReceived(&handler, &mut token) };
    }

    /// The shared page as a `file:` document under the user-data dir —
    /// relative `file:` images in Markdown then resolve like they do under
    /// WebKit's `file:///`-origin `load_html`. Written on every launch so a
    /// new app version replaces it.
    fn navigate(webview: &ICoreWebView2) {
        let Some(dir) = dirs::data_local_dir() else {
            return;
        };
        let page = dir.join("pitex").join("markdown-preview.html");
        if std::fs::write(&page, PREVIEW_HTML).is_err() {
            return;
        }
        let uri = gtk4::gio::File::for_path(&page).uri();
        let uri = HSTRING::from(uri.as_str());
        let _ = unsafe { webview.Navigate(&uri) };
    }

    /// Child-HWND placement — the placeholder's allocation in toplevel
    /// coords scaled to physical pixels (bounds are raw pixels), plus
    /// visibility: mapped and not covered by an in-surface overlay.
    fn sync(&self) {
        let live = self.live.borrow();
        let Some(live) = live.as_ref() else {
            return;
        };
        let visible = self.placeholder.is_mapped() && !self.covered.get();
        unsafe {
            let _ = live.controller.SetIsVisible(visible);
            let _ = live.controller.NotifyParentWindowPositionChanged();
        }
        if !visible {
            return;
        }
        if let Some(bounds) = self.client_bounds() {
            let _ = unsafe { live.controller.SetBounds(bounds) };
        }
    }

    fn hide(&self) {
        if let Some(live) = self.live.borrow().as_ref() {
            let _ = unsafe { live.controller.SetIsVisible(false) };
        }
    }

    /// Placeholder bounds in toplevel *physical* pixels — `compute_bounds`
    /// reports logical units, `scale_factor` carries the monitor DPI.
    fn client_bounds(&self) -> Option<RECT> {
        // `Root` is an interface — `downcast` to Widget doesn't satisfy its
        // `IsA<Root>` bound; `Native` *is*-a Widget (GtkNative's required
        // prerequisite), so `upcast` lands on the toplevel widget.
        let toplevel = self.placeholder.native()?.upcast::<gtk4::Widget>();
        let bounds = self.placeholder.compute_bounds(&toplevel)?;
        let scale = f64::from(self.placeholder.scale_factor());
        Some(RECT {
            left: (f64::from(bounds.x()) * scale) as i32,
            top: (f64::from(bounds.y()) * scale) as i32,
            right: (f64::from(bounds.x() + bounds.width()) * scale) as i32,
            bottom: (f64::from(bounds.y() + bounds.height()) * scale) as i32,
        })
    }

    fn open_link(&self, uri: String) {
        (self.hooks.open_link)(uri);
    }

    fn loaded(&self) {
        (self.hooks.loaded)();
    }

    fn scroll(&self, line: f64) {
        (self.hooks.scroll)(line);
    }

    fn toast(&self, message: String) {
        (self.hooks.toast)(message);
    }
}
