//! Track-A parity smoke tests — drive the real window chrome under Xvfb
//! and write PNG screenshots to `$PITEX_SHOT_DIR` (default
//! `/tmp/pitex-shots`). Run the compat (22.04) build with:
//!   xvfb-run -a dbus-run-session -- cargo test -p pitex-shell --lib \
//!     --no-default-features --features markdown-preview parity_shots \
//!     -- --ignored --nocapture --test-threads=1

use super::*;
// The WebKit test handle exists only on Unix — the Windows engine is
// WebView2, and `webkit6` isn't a dependency there.
#[cfg(all(unix, feature = "markdown-preview"))]
use webkit6::prelude::WebViewExt;
use std::time::Instant;

fn shot_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(
        std::env::var("PITEX_SHOT_DIR").unwrap_or_else(|_| "/tmp/pitex-shots".into()),
    );
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn pump_for(dur: std::time::Duration) {
    let context = glib::MainContext::default();
    let deadline = Instant::now() + dur;
    while Instant::now() < deadline {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn pump_until(mut done: impl FnMut() -> bool, secs: u64) {
    let context = glib::MainContext::default();
    let deadline = Instant::now() + std::time::Duration::from_secs(secs);
    while Instant::now() < deadline {
        while context.pending() {
            context.iteration(false);
        }
        if done() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// Render `widget` to a PNG (same path as `git_diff_surface_renders_to_png`).
fn snap(widget: &impl IsA<gtk4::Widget>, name: &str) -> std::path::PathBuf {
    let widget = widget.upcast_ref::<gtk4::Widget>();
    // A zero-size widget snapshots nothing — wait for a real allocation.
    pump_until(|| widget.width() > 0 && widget.height() > 0, 10);
    assert!(widget.width() > 0, "{name}: widget never got a size");
    let (w, h) = (widget.width() as f64, widget.height() as f64);
    // The paintable can return an empty snapshot while the window is still
    // being mapped — retry for a few frames before giving up.
    let mut node = None;
    for _ in 0..20 {
        let paintable = gtk4::WidgetPaintable::new(Some(widget));
        let snapshot = gtk4::Snapshot::new();
        paintable.snapshot(&snapshot, w, h);
        if let Some(n) = snapshot.to_node() {
            node = Some(n);
            break;
        }
        pump_for(std::time::Duration::from_millis(100));
    }
    let node = node.expect("snapshot produced a node");
    let renderer = widget.native().unwrap().renderer().unwrap();
    let texture = renderer.render_texture(&node, None);
    let out = shot_dir().join(format!("{name}.png"));
    texture.save_to_png(&out).unwrap();
    eprintln!("wrote {}", out.display());
    out
}

/// The `WindowTitle` lives inside the header bar — its ancestor proves the
/// header exists; `is_mapped` proves it's on screen.
fn header() -> adw::HeaderBar {
    UI.with(|ui| {
        let title = ui.window_title.borrow().as_ref().unwrap().clone();
        title
            .ancestor(adw::HeaderBar::static_type())
            .and_then(|w| w.downcast::<adw::HeaderBar>().ok())
            .expect("window title sits inside a header bar")
    })
}

fn fresh_app(tag: &str) -> adw::Application {
    let root = std::env::temp_dir().join(format!("pitex-parity-{tag}-{}", std::process::id()));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
    // GTK can only be initialized once per process — the shot tests share
    // one, so later tests reuse the first init instead of re-initializing.
    if !gtk4::is_initialized() {
        adw::init().unwrap();
    }
    let app = adw::Application::builder()
        .application_id(&format!("app.pitex.Parity{tag}"))
        .build();
    app.register(None::<&gio::Cancellable>).unwrap();
    app
}

/// A2: the header bar (with its window controls) must be visible on every
/// root-stack page — welcome, loading, error and ready alike.
#[test]
#[ignore = "requires a GTK display (use xvfb-run)"]
fn header_bar_visible_on_every_page() {
    let app = fresh_app("pages");
    build_window(&app, "parity-shots");
    let state = STATE.with(|s| s.borrow().as_ref().unwrap().clone());
    let window = UI.with(|ui| ui.window.borrow().as_ref().unwrap().clone());
    window.present();
    pump_for(std::time::Duration::from_millis(400));

    let header = header();
    for (phase, name) in [
        (crate::model::WorkspacePhase::NoProject, "welcome"),
        (
            crate::model::WorkspacePhase::Loading(std::path::PathBuf::from("/tmp/demo")),
            "loading",
        ),
        (
            crate::model::WorkspacePhase::Failed("demo failure".to_string()),
            "error",
        ),
        (crate::model::WorkspacePhase::Ready, "ready"),
    ] {
        state.borrow_mut().model.phase = phase;
        state.borrow_mut().refresh_phase();
        pump_for(std::time::Duration::from_millis(250));
        assert!(header.is_mapped(), "header not mapped on the {name} page");
        snap(&window, &format!("page-{name}"));
    }
    window.close();
}

/// A3: the compat pickers must be the native GTK widgets for the build —
/// `FontButton`/`ColorButton` on 22.04, `*DialogButton` on modern GTK.
#[test]
#[ignore = "requires a GTK display (use xvfb-run)"]
fn pickers_use_native_widgets() {
    let app = fresh_app("pickers");
    let window = gtk4::ApplicationWindow::new(&app);
    window.set_default_size(420, 120);
    let column = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    column.set_margin_start(12);
    column.set_margin_end(12);

    let font = compat::FontPicker::new();
    font.set_font_desc(&gtk4::pango::FontDescription::from_string("Serif 14"));
    let color = compat::ColorWell::new();
    color.set_rgba(&gtk4::gdk::RGBA::new(0.2, 0.5, 0.9, 1.0));

    #[cfg(feature = "modern-gtk")]
    {
        assert!(font.widget().is::<gtk4::FontDialogButton>());
        assert!(color.widget().is::<gtk4::ColorDialogButton>());
    }
    #[cfg(not(feature = "modern-gtk"))]
    {
        #[allow(deprecated)]
        {
            assert!(font.widget().is::<gtk4::FontButton>());
            assert!(color.widget().is::<gtk4::ColorButton>());
        }
    }

    column.append(&gtk4::Label::new(Some("Font")));
    column.append(font.widget());
    column.append(&gtk4::Label::new(Some("Color")));
    column.append(color.widget());
    window.set_child(Some(&column));
    window.present();
    pump_for(std::time::Duration::from_millis(400));
    snap(&window, "pickers-native");
    window.close();
}

/// A1: the Markdown preview renders heading/table/KaTeX, syncs scroll,
/// follows the dark theme and exports a PDF — all on the 22.04 build.
#[cfg(all(unix, feature = "markdown-preview"))]
#[test]
#[ignore = "requires a GTK display (use xvfb-run)"]
fn markdown_preview_renders_scrolls_and_exports() {
    let app = fresh_app("md");
    build_window(&app, "parity-shots");
    let state = STATE.with(|s| s.borrow().as_ref().unwrap().clone());
    let window = UI.with(|ui| ui.window.borrow().as_ref().unwrap().clone());
    window.present();

    let dir = std::env::temp_dir().join(format!("pitex-mddoc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = String::from(
        "# Parity Heading\n\n| Col A | Col B |\n| --- | --- |\n| 1 | 2 |\n\n$\\int_0^1 x\\,dx$\n\n",
    );
    for n in 0..80 {
        doc.push_str(&format!("Filler paragraph line {n} to make the page scrollable.\n\n"));
    }
    let md = dir.join("demo.md");
    std::fs::write(&md, &doc).unwrap();

    state.borrow_mut().open_selected(md.clone());
    pump_until(
        || matches!(state.borrow().model.phase, crate::model::WorkspacePhase::Ready),
        10,
    );
    assert!(matches!(
        state.borrow().model.phase,
        crate::model::WorkspacePhase::Ready
    ));

    state.borrow().render_markdown_now();
    // WebKit spawns a process and loads the HTML shell before the first
    // render lands — give it real wall-clock time under Xvfb.
    pump_until(
        || state.borrow().markdown.webview_for_test().is_some(),
        15,
    );
    pump_for(std::time::Duration::from_secs(4));

    let view = state
        .borrow()
        .markdown
        .webview_for_test()
        .expect("markdown render created the webview");
    let eval = |js: &str| -> Rc<RefCell<Option<String>>> {
        let out = Rc::new(RefCell::new(None));
        let cell = out.clone();
        view.evaluate_javascript(js, None, None, gtk4::gio::Cancellable::NONE, move |r| {
            let text = match r {
                Ok(v) if v.is_number() => format!("{}", v.to_double()),
                Ok(v) => v.to_string(),
                Err(e) => format!("ERR {e}"),
            };
            *cell.borrow_mut() = Some(text);
        });
        out
    };
    let eval_wait = |js: &str| -> String {
        let out = eval(js);
        pump_until(|| out.borrow().is_some(), 5);
        let result = out.borrow().clone().unwrap_or_else(|| "TIMEOUT".into());
        result
    };

    // Heading rendered into an <h1>; the KaTeX span exists; table exists.
    assert_eq!("1", eval_wait("document.querySelectorAll('h1').length"));
    assert!(
        eval_wait("document.querySelectorAll('.katex').length") != "0",
        "KaTeX math did not render"
    );
    assert_eq!("1", eval_wait("document.querySelectorAll('table').length"));
    // Widen the preview half so the screenshot reads clearly.
    UI.with(|ui| {
        if let Some(inner) = ui.inner_paned.borrow().as_ref() {
            inner.set_position(560);
        }
    });
    pump_for(std::time::Duration::from_millis(300));
    snap(&window, "markdown-preview-light");

    // Sync scroll: scroll_to_line scrolls the preview page.
    state.borrow().markdown.scroll_to_line(60);
    pump_for(std::time::Duration::from_millis(500));
    let scroll_y: f64 = eval_wait("document.scrollingElement.scrollTop")
        .parse()
        .unwrap_or(0.0);
    assert!(scroll_y > 0.0, "preview did not scroll (scrollTop={scroll_y})");
    snap(&window, "markdown-preview-scrolled");

    // Dark theme: pin the preview theme to dark, re-render — the page must
    // mark <html> with `dark` (the stylesheet hooks on that class).
    state.borrow_mut().store.set_markdown_theme("dark");
    state.borrow().render_markdown_now();
    pump_for(std::time::Duration::from_secs(1));
    assert_eq!(
        "true",
        eval_wait("document.documentElement.classList.contains('dark')"),
        "dark flag did not reach the preview page"
    );
    snap(&window, "markdown-preview-dark");

    // PDF download → a real PDF file.
    let pdf = dir.join("demo.pdf");
    state.borrow().markdown.export_pdf(&pdf);
    pump_until(|| pdf.exists() && std::fs::metadata(&pdf).map(|m| m.len() > 0).unwrap_or(false), 15);
    let head = std::fs::read(&pdf).unwrap();
    assert!(head.starts_with(b"%PDF"), "exported file is not a PDF");
    eprintln!("pdf export: {} bytes at {}", head.len(), pdf.display());

    window.close();
}
