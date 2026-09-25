//! Manual smoke harness for the non-VTE embedded terminal:
//! `cargo run -p pitex-shell --example term_smoke --no-default-features`
//! under Xvfb, then drive it with xdotool. Exists only so verification can
//! exercise the real widget without navigating the full app.

#[cfg(not(feature = "vte"))]
fn main() {
    use gtk4::prelude::*;
    use libadwaita as adw;

    let app = adw::Application::builder()
        .application_id("dev.pitex.termsmoke")
        .build();
    app.connect_activate(|app| {
        let term = pitex_shell::terminal::EmbeddedTerminal::new();
        term.connect_exited(|t| t.feed("\r\n\x1b[90m(shell exited)\x1b[0m\r\n"));
        term.spawn_shell(Some(std::path::Path::new("/tmp")), |ok| {
            eprintln!("[term-smoke] spawn_shell: {ok}");
        });
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("term-smoke")
            .default_width(880)
            .default_height(560)
            .content(&term.widget())
            .build();
        window.present();
        // The terminal owns the session: dropping it while the widget is
        // still shown would kill the PTY and blank the grid. Keep it alive
        // for the app's lifetime (the real app stores it in UI state).
        std::mem::forget(term);
    });
    app.run();
}

#[cfg(feature = "vte")]
fn main() {}
