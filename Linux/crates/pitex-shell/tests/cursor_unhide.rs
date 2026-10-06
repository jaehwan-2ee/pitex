#![cfg(target_os = "linux")]

use gtk4::prelude::*;
use sourceview5::prelude::*;

/// GTK's GtkTextView hides the pointer by setting the widget cursor to
/// "none" while typing; `unhide_pointer_on_typing` must put "text" back.
#[test]
fn commit_restores_text_cursor() {
    if gtk4::init().is_err() {
        return; // no display
    }
    let view = sourceview5::View::new();
    let window = gtk4::Window::new();
    window.set_child(Some(&view));
    window.present();

    pitex_shell::compat::unhide_pointer_on_typing(&view);
    // Simulate GTK's obscure-on-commit.
    view.set_cursor_from_name(Some("none"));
    view.buffer().insert_at_cursor("x");

    let ctx = gtk4::glib::MainContext::default();
    while ctx.pending() {
        ctx.iteration(false);
    }

    assert_eq!(
        view.cursor().and_then(|c| c.name().map(|n| n.to_string())),
        Some("text".to_string())
    );
    window.destroy();
}
