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
fn caps_switch_preserves_native_input_and_document_sync() {
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
    // The surrounding session configures Caps Lock as the actual IBus
    // language switch. Check the adapter and canonical document together.
    for phase in 0..4 {
        if phase != 0 { keys(&["key", "Caps_Lock"]); }
        let korean = phase % 2 == 1;
        keys(&["key", "--delay", "0", "g", "k", "s", "1", "space"]);
        assert_eq!(adapter.text(), if korean { "한1 " } else { "gks1 " });
        assert_eq!(adapter.text(), state.borrow().text);
        keys(&["key", "ctrl+a", "BackSpace"]);
        assert_eq!(adapter.text(), "");
        keys(&["key", "--delay", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]);
        assert_eq!(adapter.text(), "1234567890");
        keys(&["key", "--delay", "0", "BackSpace", "BackSpace", "Home", "Delete"]);
        assert_eq!(adapter.text(), "2345678");
        keys(&["key", "ctrl+a", "BackSpace", "Num_Lock", "KP_1", "KP_2", "KP_3"]);
        assert_eq!(adapter.text(), "123");
        keys(&["key", "BackSpace", "Home", "Delete"]);
        assert_eq!(adapter.text(), "2");
        assert_eq!(adapter.text(), state.borrow().text);
        keys(&["key", "ctrl+a", "BackSpace", "Num_Lock"]);
        if korean {
            keys(&["key", "g", "k", "BackSpace", "k", "s", "Caps_Lock", "Caps_Lock"]);
            assert_eq!(adapter.text(), "한", "IBus must own jamo deletion in every preedit mode");
            assert_eq!(adapter.text(), state.borrow().text);
            keys(&["key", "ctrl+a", "BackSpace"]);
        }
    }
    window.close();
}
