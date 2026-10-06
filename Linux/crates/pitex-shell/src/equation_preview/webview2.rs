//! Windows MathJax host. All resources are answered from the embedded
//! allowlist; the synthetic HTTPS origin never reaches the network.
//!
//! MathJax starts hidden under the editor's HWND before a popup exists.
//! Mapping the GTK popup reparents the controller to its native surface;
//! unmapping returns it to the editor before GTK destroys the popup HWND.
//! Payloads and async replies use WebView2 JSON messages, not script text.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;

use gtk4::glib::translate::ToGlibPtr;
use gtk4::glib::{self, gobject_ffi, ControlFlow};
use gtk4::prelude::*;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    take_pwstr, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, FocusChangedEventHandler,
    MoveFocusRequestedEventHandler, NavigationCompletedEventHandler,
    NavigationStartingEventHandler, NewWindowRequestedEventHandler,
    PermissionRequestedEventHandler, ProcessFailedEventHandler, WebMessageReceivedEventHandler,
    WebResourceRequestedEventHandler,
};
use windows::core::{s, w, Interface, BOOL, HRESULT, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Shell::SHCreateMemStream;

const ORIGIN: &str = "https://pitex-equation.invalid/";
const PAGE_URI: &str = "https://pitex-equation.invalid/renderer.html";
type Completion = Box<dyn FnOnce(Option<serde_json::Value>)>;

// The runtime loader remains optional, just as for Markdown preview.
type CreateEnvironmentFn = unsafe extern "system" fn(
    PCWSTR,
    PCWSTR,
    *mut core::ffi::c_void,
    *mut core::ffi::c_void,
) -> HRESULT;

fn loader() -> Option<CreateEnvironmentFn> {
    static LOADER: OnceLock<Option<CreateEnvironmentFn>> = OnceLock::new();
    *LOADER.get_or_init(|| unsafe {
        let module = LoadLibraryW(w!("WebView2Loader.dll")).ok()?;
        Some(core::mem::transmute(GetProcAddress(
            module,
            s!("CreateCoreWebView2EnvironmentWithOptions"),
        )?))
    })
}

extern "C" {
    fn gdk_win32_surface_get_type() -> usize;
    fn gdk_win32_surface_get_handle(
        surface: *mut gtk4::gdk::ffi::GdkSurface,
    ) -> *mut core::ffi::c_void;
}

fn hwnd(widget: &impl IsA<gtk4::Widget>) -> Option<HWND> {
    let surface = widget.native()?.surface()?;
    unsafe {
        let raw: *mut gtk4::gdk::ffi::GdkSurface = surface.to_glib_none().0;
        if gobject_ffi::g_type_check_instance_is_a(
            raw as *mut gobject_ffi::GTypeInstance,
            gdk_win32_surface_get_type(),
        ) == 0
        {
            return None;
        }
        let handle = gdk_win32_surface_get_handle(raw);
        (!handle.is_null()).then_some(HWND(handle))
    }
}

/// Kept intentionally small: no raw TeX or SVG appears in executable JS.
const BRIDGE: &str = r#"
window.chrome.webview.addEventListener('message', async ({data}) => {
  if (!data || !Number.isSafeInteger(data.id)) return;
  let result = null;
  try {
    const input = JSON.parse(data.payload);
    if (data.action === 'ready') result = {identity: await window.pitexEquation.ready};
    else if (data.action === 'render') result = await window.pitexEquation.render(input);
    else if (data.action === 'show') result = await window.pitexEquation.show(input);
  } catch (_) {}
  window.chrome.webview.postMessage({id: data.id, result});
});
window.addEventListener('pointerenter', (event) => {
  if (event.target === document.documentElement) window.chrome.webview.postMessage({hover: true});
}, true);
window.addEventListener('pointerleave', (event) => {
  if (event.target === document.documentElement) window.chrome.webview.postMessage({hover: false});
}, true);
"#;

#[derive(Clone)]
pub(super) struct Engine {
    inner: Rc<Inner>,
}

struct Inner {
    placeholder: gtk4::Box,
    editor: RefCell<glib::WeakRef<sourceview5::View>>,
    live: RefCell<Option<Live>>,
    creating: Cell<bool>,
    closed: Cell<bool>,
    navigated: Cell<bool>,
    next_call: Cell<u64>,
    pending: RefCell<HashMap<u64, Completion>>,
    loaded: Box<dyn Fn()>,
    failed: Box<dyn Fn()>,
    hovered: Box<dyn Fn(bool)>,
}

struct Live {
    controller: ICoreWebView2Controller,
    webview: ICoreWebView2,
    parent: Cell<HWND>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(live) = self.live.get_mut().take() {
            let _ = unsafe { live.controller.Close() };
        }
    }
}

impl Engine {
    pub(super) fn new(
        editor: &sourceview5::View,
        loaded: impl Fn() + 'static,
        failed: impl Fn() + 'static,
        hovered: impl Fn(bool) + 'static,
    ) -> Self {
        let placeholder = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        placeholder.set_hexpand(true);
        placeholder.set_vexpand(true);
        placeholder.set_can_focus(false);
        placeholder.set_focusable(false);
        let inner = Rc::new(Inner {
            placeholder: placeholder.clone(),
            editor: RefCell::new(editor.downgrade()),
            live: RefCell::new(None),
            creating: Cell::new(false),
            closed: Cell::new(false),
            navigated: Cell::new(false),
            next_call: Cell::new(0),
            pending: RefCell::new(HashMap::new()),
            loaded: Box::new(loaded),
            failed: Box::new(failed),
            hovered: Box::new(hovered),
        });
        let weak = Rc::downgrade(&inner);
        placeholder.connect_map(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.sync();
            }
        });
        let weak = Rc::downgrade(&inner);
        placeholder.connect_unmap(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.park();
            }
        });
        let weak = Rc::downgrade(&inner);
        placeholder.add_tick_callback(move |_, _| {
            if let Some(inner) = weak.upgrade() {
                inner.sync();
                ControlFlow::Continue
            } else {
                ControlFlow::Break
            }
        });
        Self { inner }
    }

    pub(super) fn widget(&self) -> &gtk4::Box {
        &self.inner.placeholder
    }

    pub(super) fn set_editor(&self, editor: &sourceview5::View) {
        *self.inner.editor.borrow_mut() = editor.downgrade();
        self.inner.park();
    }

    pub(super) fn start(&self) {
        self.inner.start();
        let weak = Rc::downgrade(&self.inner);
        // Startup gets the same bounded recovery as rendering timeouts.
        glib::timeout_add_local_once(Duration::from_secs(15), move || {
            if let Some(inner) = weak.upgrade() {
                if inner.live.borrow().is_none() {
                    inner.fail();
                }
            }
        });
    }

    pub(super) fn close(&self) {
        if self.inner.closed.replace(true) {
            return;
        }
        let live = self.inner.live.borrow_mut().take();
        if let Some(live) = live {
            let _ = unsafe { live.controller.Close() };
        }
        let pending = self
            .inner
            .pending
            .borrow_mut()
            .drain()
            .map(|(_, done)| done)
            .collect::<Vec<_>>();
        for done in pending {
            glib::idle_add_local_once(move || done(None));
        }
    }

    pub(super) fn call(
        &self,
        body: &'static str,
        payload: String,
        timeout_ms: u64,
        done: impl FnOnce(Option<serde_json::Value>) + 'static,
    ) {
        let action = match body {
            "window.pitexEquation.ready.then((identity) => ({ identity }))" => "ready",
            "window.pitexEquation.render(input)" => "render",
            "window.pitexEquation.show(input)" => "show",
            _ => {
                glib::idle_add_local_once(move || done(None));
                return;
            }
        };
        let webview = self.inner.live.borrow().as_ref().map(|l| l.webview.clone());
        let Some(webview) = webview else {
            glib::idle_add_local_once(move || done(None));
            return;
        };
        let id = self.inner.next_call.get() + 1;
        self.inner.next_call.set(id);
        self.inner.pending.borrow_mut().insert(id, Box::new(done));
        let message = HSTRING::from(
            serde_json::json!({ "id": id, "action": action, "payload": payload }).to_string(),
        );
        let weak = Rc::downgrade(&self.inner);
        if unsafe { webview.PostWebMessageAsJson(&message) }.is_err() {
            glib::idle_add_local_once(move || {
                if let Some(inner) = weak.upgrade() {
                    inner.complete(id, None);
                }
            });
            return;
        }
        glib::timeout_add_local_once(Duration::from_millis(timeout_ms), move || {
            if let Some(inner) = weak.upgrade() {
                inner.complete(id, None);
            }
        });
    }
}

impl Inner {
    fn start(self: &Rc<Self>) {
        if self.closed.get() || self.creating.get() || self.live.borrow().is_some() {
            return;
        }
        let Some(loader) = loader() else {
            self.fail();
            return;
        };
        let parent = self.editor.borrow().upgrade().and_then(|v| hwnd(&v));
        let Some(parent) = parent else {
            // During initial attach the editor may not yet have mapped.
            let weak = Rc::downgrade(self);
            glib::timeout_add_local_once(Duration::from_millis(100), move || {
                if let Some(inner) = weak.upgrade() {
                    inner.start();
                }
            });
            return;
        };
        self.creating.set(true);
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let Some(dir) = dirs::data_local_dir() else {
            self.fail();
            return;
        };
        let dir = dir.join("pitex").join("EquationWebView2");
        if std::fs::create_dir_all(&dir).is_err() {
            self.fail();
            return;
        }
        let weak = Rc::downgrade(self);
        let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |result, environment| {
                if let Some(inner) = weak.upgrade() {
                    if inner.closed.get() {
                        return Ok(());
                    }
                    match (result, environment) {
                        (Ok(()), Some(environment)) => inner.environment_ready(environment, parent),
                        _ => inner.fail(),
                    }
                }
                Ok(())
            },
        ));
        if unsafe {
            loader(
                PCWSTR::null(),
                PCWSTR(HSTRING::from(dir.as_os_str()).as_ptr()),
                std::ptr::null_mut(),
                handler.as_raw(),
            )
        }
        .is_err()
        {
            self.fail();
        }
    }

    fn environment_ready(self: &Rc<Self>, environment: ICoreWebView2Environment, parent: HWND) {
        let weak = Rc::downgrade(self);
        let resources = environment.clone();
        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |result, controller| {
                if let Some(inner) = weak.upgrade() {
                    match (result, controller) {
                    (Ok(()), Some(controller)) if !inner.closed.get() => {
                        if inner
                            .controller_ready(controller.clone(), &resources, parent)
                            .is_err()
                        {
                            let _ = unsafe { controller.Close() };
                            inner.fail();
                            }
                        }
                        (_, Some(controller)) => {
                            let _ = unsafe { controller.Close() };
                        }
                        _ => inner.fail(),
                    }
                } else if let Some(controller) = controller {
                    let _ = unsafe { controller.Close() };
                }
                Ok(())
            },
        ));
        // InPrivate prevents storage of the preview's TeX and macro state.
        let result = (|| unsafe {
            let environment10 = environment.cast::<ICoreWebView2Environment10>()?;
            let options = environment10.CreateCoreWebView2ControllerOptions()?;
            options.SetIsInPrivateModeEnabled(true)?;
            environment10.CreateCoreWebView2ControllerWithOptions(parent, &options, &handler)
        })();
        if result.is_err() {
            self.fail();
        }
    }

    fn controller_ready(
        self: &Rc<Self>,
        controller: ICoreWebView2Controller,
        environment: &ICoreWebView2Environment,
        parent: HWND,
    ) -> windows::core::Result<()> {
        unsafe {
            controller.SetIsVisible(false)?;
            let webview = controller.CoreWebView2()?;
            let settings = webview.Settings()?;
            settings.SetAreDevToolsEnabled(false)?;
            settings.SetAreDefaultContextMenusEnabled(false)?;
            settings.SetAreDefaultScriptDialogsEnabled(false)?;
            settings.SetAreHostObjectsAllowed(false)?;
            settings.SetIsStatusBarEnabled(false)?;
            settings.SetIsZoomControlEnabled(false)?;
            settings.SetIsBuiltInErrorPageEnabled(false)?;
            settings.SetIsWebMessageEnabled(true)?;
            if let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() {
                let _ = settings3.SetAreBrowserAcceleratorKeysEnabled(false);
            }
            if let Ok(controller2) = controller.cast::<ICoreWebView2Controller2>() {
                let _ = controller2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
                    A: 0,
                    R: 0,
                    G: 0,
                    B: 0,
                });
            }
            if let Ok(controller3) = controller.cast::<ICoreWebView2Controller3>() {
                let _ = controller3.SetShouldDetectMonitorScaleChanges(true);
            }
            self.wire(&webview, &controller, environment)?;
            webview.AddScriptToExecuteOnDocumentCreated(
                &HSTRING::from(BRIDGE),
                None::<&ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler>,
            )?;
            *self.live.borrow_mut() = Some(Live {
                controller,
                webview: webview.clone(),
                parent: Cell::new(parent),
            });
            webview.Navigate(&HSTRING::from(PAGE_URI))?;
        }
        Ok(())
    }

    fn wire(
        self: &Rc<Self>,
        webview: &ICoreWebView2,
        controller: &ICoreWebView2Controller,
        environment: &ICoreWebView2Environment,
    ) -> windows::core::Result<()> {
        let mut token = 0i64;
        let weak = Rc::downgrade(self);
        let nav = NavigationStartingEventHandler::create(Box::new(move |_, args| {
            if let (Some(inner), Some(args)) = (weak.upgrade(), args) {
                let mut uri = PWSTR::null();
                unsafe {
                    args.Uri(&mut uri)?;
                }
                let allowed = !inner.navigated.replace(true) && take_pwstr(uri) == PAGE_URI;
                if !allowed {
                    unsafe {
                        args.SetCancel(true)?;
                    }
                }
            }
            Ok(())
        }));
        unsafe {
            webview.add_NavigationStarting(&nav, &mut token)?;
        }
        let env = environment.clone();
        let resources = WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else {
                return Ok(());
            };
            let request = unsafe { args.Request()? };
            let mut uri = PWSTR::null();
            unsafe {
                request.Uri(&mut uri)?;
            }
            let uri = take_pwstr(uri);
            let found = uri
                .strip_prefix(ORIGIN)
                .and_then(|path| super::asset(&format!("pitex-equation://preview/{path}")));
            let (bytes, mime, status, reason) = match found {
                Some((bytes, mime)) => (bytes, mime, 200, "OK"),
                None => (&[][..], "text/plain", 403, "Forbidden"),
            };
            let stream = unsafe { SHCreateMemStream(Some(bytes)) };
            let headers = HSTRING::from(format!("Content-Type: {mime}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff"));
            let response = unsafe {
                env.CreateWebResourceResponse(
                    stream.as_ref(),
                    status,
                    &HSTRING::from(reason),
                    &headers,
                )?
            };
            unsafe {
                args.SetResponse(&response)?;
            }
            Ok(())
        }));
        unsafe {
            webview.AddWebResourceRequestedFilter(
                &HSTRING::from("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )?;
            webview.add_WebResourceRequested(&resources, &mut token)?;
        }
        let weak = Rc::downgrade(self);
        let loaded = NavigationCompletedEventHandler::create(Box::new(move |_, args| {
            let (Some(inner), Some(args)) = (weak.upgrade(), args) else {
                return Ok(());
            };
            let mut success = BOOL(0);
            unsafe {
                args.IsSuccess(&mut success)?;
            }
            if success.as_bool() {
                let weak = Rc::downgrade(&inner);
                glib::idle_add_local_once(move || {
                    if let Some(inner) = weak.upgrade() {
                        if !inner.closed.get() {
                            (inner.loaded)();
                        }
                    }
                });
            } else {
                inner.fail();
            }
            Ok(())
        }));
        unsafe {
            webview.add_NavigationCompleted(&loaded, &mut token)?;
        }
        let weak = Rc::downgrade(self);
        let messages = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            let (Some(inner), Some(args)) = (weak.upgrade(), args) else {
                return Ok(());
            };
            let mut source = PWSTR::null();
            unsafe {
                args.Source(&mut source)?;
            }
            if take_pwstr(source) != PAGE_URI {
                return Ok(());
            }
            let mut message = PWSTR::null();
            unsafe {
                args.WebMessageAsJson(&mut message)?;
            }
            let Ok(message) = serde_json::from_str::<serde_json::Value>(&take_pwstr(message))
            else {
                return Ok(());
            };
            let weak = Rc::downgrade(&inner);
            glib::idle_add_local_once(move || {
                let Some(inner) = weak.upgrade() else { return };
                if inner.closed.get() {
                    return;
                }
                if let Some(id) = message.get("id").and_then(|v| v.as_u64()) {
                    inner.complete(id, message.get("result").filter(|v| !v.is_null()).cloned());
                } else if let Some(hover) = message.get("hover").and_then(|v| v.as_bool()) {
                    (inner.hovered)(hover);
                }
            });
            Ok(())
        }));
        unsafe {
            webview.add_WebMessageReceived(&messages, &mut token)?;
        }
        let weak = Rc::downgrade(self);
        let crashed = ProcessFailedEventHandler::create(Box::new(move |_, _| {
            if let Some(inner) = weak.upgrade() {
                inner.fail();
            }
            Ok(())
        }));
        let new_window = NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                unsafe {
                    args.SetHandled(true)?;
                }
            }
            Ok(())
        }));
        let permission = PermissionRequestedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                unsafe {
                    args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
                }
            }
            Ok(())
        }));
        let focus = MoveFocusRequestedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                unsafe {
                    args.SetHandled(true)?;
                }
            }
            Ok(())
        }));
        let weak = Rc::downgrade(self);
        let got_focus = FocusChangedEventHandler::create(Box::new(move |_, _| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(inner) = weak.upgrade() {
                    if let Some(editor) = inner.editor.borrow().upgrade() {
                        editor.grab_focus();
                    }
                }
            });
            Ok(())
        }));
        unsafe {
            webview.add_ProcessFailed(&crashed, &mut token)?;
            webview.add_NewWindowRequested(&new_window, &mut token)?;
            webview.add_PermissionRequested(&permission, &mut token)?;
            controller.add_MoveFocusRequested(&focus, &mut token)?;
            controller.add_GotFocus(&got_focus, &mut token)?;
        }
        Ok(())
    }

    fn complete(&self, id: u64, result: Option<serde_json::Value>) {
        let done = self.pending.borrow_mut().remove(&id);
        if let Some(done) = done {
            done(result);
        }
    }

    fn fail(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            if let Some(inner) = weak.upgrade() {
                if !inner.closed.get() {
                    (inner.failed)();
                }
            }
        });
    }

    fn park(&self) {
        let live = self.live.borrow();
        let Some(live) = live.as_ref() else { return };
        unsafe {
            let _ = live.controller.SetIsVisible(false);
        }
        if let Some(parent) = self.editor.borrow().upgrade().and_then(|v| hwnd(&v)) {
            if live.parent.get() != parent
                && unsafe { live.controller.SetParentWindow(parent) }.is_ok()
            {
                live.parent.set(parent);
            }
        }
    }

    fn sync(&self) {
        if !self.placeholder.is_mapped() {
            self.park();
            return;
        }
        let live = self.live.borrow();
        let Some(live) = live.as_ref() else { return };
        let Some(parent) = hwnd(&self.placeholder) else {
            return;
        };
        let Some(native) = self.placeholder.native() else {
            return;
        };
        let Some(bounds) = self
            .placeholder
            .compute_bounds(&native.upcast::<gtk4::Widget>())
        else {
            return;
        };
        let scale = self.placeholder.scale_factor() as f32;
        let rect = RECT {
            left: (bounds.x() * scale) as i32,
            top: (bounds.y() * scale) as i32,
            right: ((bounds.x() + bounds.width()) * scale) as i32,
            bottom: ((bounds.y() + bounds.height()) * scale) as i32,
        };
        unsafe {
            if live.parent.get() != parent {
                if live.controller.SetParentWindow(parent).is_err() {
                    return;
                }
                live.parent.set(parent);
            }
            let _ = live.controller.SetBounds(rect);
            let _ = live.controller.NotifyParentWindowPositionChanged();
            let _ = live.controller.SetIsVisible(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires Windows GTK and the WebView2 Evergreen runtime"]
    fn windows_renderer_page_contract() {
        gtk4::init().expect("GTK initialization");
        let editor = sourceview5::View::new();
        let window = gtk4::Window::new();
        window.set_child(Some(&editor));
        window.present();
        let loaded = Rc::new(Cell::new(false));
        let sink = loaded.clone();
        let failure = Rc::new(Cell::new(false));
        let failed = failure.clone();
        let engine = Engine::new(
            &editor,
            move || sink.set(true),
            move || failed.set(true),
            |_| {},
        );
        engine.start();
        let pump = |condition: &dyn Fn() -> bool| {
            let end = std::time::Instant::now() + Duration::from_secs(30);
            while !condition() && !failure.get() && std::time::Instant::now() < end {
                glib::MainContext::default().iteration(false);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(!failure.get(), "WebView2 renderer failed");
            assert!(condition(), "WebView2 timed out");
        };
        pump(&|| loaded.get());
        let evaluate = |body, payload: serde_json::Value| {
            let result = Rc::new(RefCell::new(None));
            let sink = result.clone();
            engine.call(body, payload.to_string(), 20_000, move |reply| {
                *sink.borrow_mut() = Some(reply);
            });
            pump(&|| result.borrow().is_some());
            let reply = result
                .borrow_mut()
                .take()
                .flatten()
                .expect("renderer reply");
            reply
        };
        let identity = evaluate(
            "window.pitexEquation.ready.then((identity) => ({ identity }))",
            serde_json::json!({}),
        );
        assert!(identity["identity"].as_str().is_some());
        let rendered = evaluate(
            "window.pitexEquation.render(input)",
            serde_json::json!({
                "source": "\\frac{x}{2} + \\R", "displayMode": true, "contextKey": "windows-contract",
                "definitions": ["\\newcommand{\\R}{\\mathbb{R}}"], "fontSize": 16,
            }),
        );
        assert_eq!(rendered["ok"], true);
        assert!(rendered["svg"].as_str().unwrap().starts_with("<svg"));
        let shown = evaluate(
            "window.pitexEquation.show(input)",
            serde_json::json!({
                "svg": rendered["svg"], "fontSize": 16,
                "theme": {"foreground": "#ffffff", "background": "transparent"},
            }),
        );
        assert_eq!(shown["ok"], true);
        assert!(shown["width"].as_f64().unwrap() > 0.0);
        assert!(shown["height"].as_f64().unwrap() > 0.0);
        // Exercise the native popup lifecycle, including a remap after GTK
        // disposes its popup HWND. MathJax keeps the original controller.
        let popover = gtk4::Popover::new();
        popover.set_autohide(false);
        popover.set_parent(&editor);
        popover.set_child(Some(engine.widget()));
        engine.widget().set_size_request(240, 100);
        for _ in 0..2 {
            popover.popup();
            pump(&|| {
                let live = engine.inner.live.borrow();
                live.as_ref().is_some_and(|live| {
                    engine.widget().is_mapped()
                        && hwnd(engine.widget()).is_some_and(|parent| live.parent.get() == parent)
                })
            });
            popover.popdown();
            pump(&|| {
                let live = engine.inner.live.borrow();
                live.as_ref().is_some_and(|live| {
                    !engine.widget().is_mapped()
                        && hwnd(&editor).is_some_and(|parent| live.parent.get() == parent)
                })
            });
        }
        engine.close();
        popover.unparent();
        window.close();
    }
}
