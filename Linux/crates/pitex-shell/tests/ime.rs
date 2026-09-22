//! Real IBus Hangul input. Run alone under dbus-run-session + Xvfb with
//! GTK_IM_MODULE=ibus and `ibus engine hangul` (see the Ubuntu 22.04 CI job).
#![cfg(target_os = "linux")]
use app_ports::{DocumentMutationResult, DocumentSnapshot};
use gtk4::prelude::*;
use gtk_editor_adapter::{FnSessionClient, GtkEditorAdapter};
use std::{cell::RefCell, rc::Rc, process::Command, time::{Duration, Instant}};

fn pump(duration: Duration) {
    let end = Instant::now() + duration;
    let context = gtk4::glib::MainContext::default();
    while Instant::now() < end {
        while context.pending() { context.iteration(false); }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn keys(arguments: &[&str]) {
    let mut child = Command::new("xdotool").args(arguments).spawn().unwrap();
    loop {
        pump(Duration::from_millis(10));
        if let Some(status) = child.try_wait().unwrap() { assert!(status.success()); break; }
    }
    pump(Duration::from_millis(250));
}

#[test]
#[ignore = "requires a real ibus-hangul session and GTK display"]
fn hangul_backspace_keeps_working_after_commit_and_with_lock_modifiers() {
    gtk4::init().unwrap();
    let state = Rc::new(RefCell::new(DocumentSnapshot { revision: 0, text: String::new() }));
    let (read, write) = (state.clone(), state.clone());
    let adapter = Rc::new(GtkEditorAdapter::make(Rc::new(FnSessionClient::new(
        move || read.borrow().clone(),
        move |mutation| {
            let mut snapshot = write.borrow_mut();
            assert_eq!(snapshot.revision, mutation.base_revision);
            let mut units: Vec<_> = snapshot.text.encode_utf16().collect();
            units.splice(mutation.range.location..mutation.range.location + mutation.range.length,
                mutation.replacement.encode_utf16());
            snapshot.text = String::from_utf16(&units).unwrap();
            snapshot.revision += 1;
            DocumentMutationResult::Applied(snapshot.clone())
        },
    ))));
    let window = gtk4::Window::builder().title("PitexIMERegression").default_width(480).default_height(240).build();
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(adapter.view()));
    let current = adapter.clone();
    pitex_shell::compat::fix_ime_backspace(&scroller, move || Some(current.clone()));
    window.set_child(Some(&scroller));
    window.present();
    adapter.view().grab_focus();
    pump(Duration::from_millis(400));
    let output = Command::new("xdotool").args(["search", "--name", "^PitexIMERegression$"]).output().unwrap();
    assert!(output.status.success());
    let id = String::from_utf8(output.stdout).unwrap();
    keys(&["windowfocus", "--sync", id.lines().next().unwrap()]);
    assert!(Command::new("ibus").args(["engine", "hangul"]).status().unwrap().success());
    pump(Duration::from_millis(500));
    keys(&["key", "Hangul"]);
    for (caps_lock, num_lock) in [(false, false), (true, false), (false, true), (true, true)] {
        keys(&["key", "g", "k", "s", "space"]);
        assert_eq!(adapter.text(), "한 ", "IBus must produce real Korean input for this check");
        if caps_lock { keys(&["key", "Caps_Lock"]); }
        if num_lock { keys(&["key", "Num_Lock"]); }
        for _ in 0..8 {
            let before = adapter.text();
            if before.is_empty() { break; }
            keys(&["key", "BackSpace"]);
            assert_ne!(adapter.text(), before, "Backspace was swallowed (Caps Lock={caps_lock}, Num Lock={num_lock}, preedit={})", adapter.has_marked_text());
            assert_eq!(adapter.text(), state.borrow().text);
        }
        assert!(adapter.text().is_empty());
        if caps_lock { keys(&["key", "Caps_Lock"]); }
        if num_lock { keys(&["key", "Num_Lock"]); }
    }
    keys(&["key", "g", "k"]);
    assert!(adapter.has_marked_text(), "Hangul composition must still be owned by IBus");
    keys(&["key", "BackSpace"]);
    assert!(adapter.has_marked_text(), "Backspace inside a syllable must keep the remaining jamo in composition");
    window.close();
}
