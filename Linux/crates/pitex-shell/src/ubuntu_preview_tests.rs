use super::*;
use crate::embedded_preview::PreviewStatus;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn drive_until(label: &str, condition: impl Fn() -> bool) {
    let context = glib::MainContext::default();
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    loop {
        for _ in 0..100 {
            if !context.pending() {
                break;
            }
            context.iteration(false);
        }
        if condition() {
            return;
        }
        assert!(std::time::Instant::now() < deadline, "timed out: {label}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn backend_row(widget: &gtk4::Widget, title: &str) -> Option<adw::ComboRow> {
    if let Ok(row) = widget.clone().downcast::<adw::ComboRow>() {
        if row.title() == title {
            return Some(row);
        }
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        if let Some(row) = backend_row(&widget, title) {
            return Some(row);
        }
        child = widget.next_sibling();
    }
    None
}

fn pdf_text(bytes: &[u8]) -> String {
    let mut process = Command::new("pdftotext")
        .args(["-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pdftotext is installed for the real-PDF check");
    process.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Run alone under Xvfb with PITEX_PREVIEW_HELPER pointing at the shipped
/// Rust helper and XeLaTeX/Poppler installed. GTK 4.6 and libadwaita 1.1
/// must exercise the same editor, timers, helper and PDF viewer as 24.04.
#[test]
#[ignore = "requires an isolated GTK display, real embedded helper and TeX runtime"]
fn local_unsaved_edits_render_through_embedded_preview() {
    let _environment = crate::TEST_ENV_LOCK.lock().unwrap();
    assert!(crate::embedded_preview::SUPPORTED);
    assert!(
        crate::embedded_preview::helper_path().is_some(),
        "real helper required"
    );
    let root = std::env::temp_dir().join(format!(
        "pitex-gtk-embedded-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    for (name, directory) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_RUNTIME_DIR", "run"),
        ("PI_CODING_AGENT_DIR", "pi"),
    ] {
        let path = root.join(directory);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::env::set_var(name, path);
    }
    // Startup may inspect the Assistant runtime, but this preview test
    // must neither download it nor start an unrelated network operation.
    let launcher = crate::agent::pi_paths::runtime_executable();
    std::fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    std::fs::write(&launcher, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(
        crate::agent::pi_paths::runtime_directory().join("package.json"),
        format!(
            r#"{{"name":"{}","version":"{}"}}"#,
            crate::agent::pi_installer::PACKAGE_NAME,
            crate::agent::pi_installer::DESIRED_VERSION
        ),
    )
    .unwrap();
    let mut store = SettingsStore::new(Preferences::standard());
    store.set_auto_install_updates(false);
    store.set_restore_session(false);
    store.set_auto_save(false);
    store.set_live_compile_enabled(true);
    store.set_live_preview_backend("embedded");
    store.prefs_mut().set("appearance.language", "en");

    let project = root.join("project");
    std::fs::create_dir(&project).unwrap();
    let source = "\\documentclass{article}\n\\begin{document}\nSeed text.\n\\end{document}\n";
    let file = project.join("main.tex");
    std::fs::write(&file, source).unwrap();
    let finished = Arc::new(AtomicBool::new(false));
    let watchdog = finished.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        if !watchdog.load(Ordering::Acquire) {
            eprintln!("FAIL: embedded preview stopped processing GTK events");
            std::process::abort();
        }
    });

    adw::init().unwrap();
    let app = adw::Application::builder()
        .application_id("app.pitex.EmbeddedPreviewTest")
        .build();
    app.register(None::<&gio::Cancellable>).unwrap();
    build_window(&app, "embedded-preview-test");
    let state = STATE.with(|slot| slot.borrow().as_ref().unwrap().clone());
    let window = UI.with(|ui| ui.window.borrow().as_ref().unwrap().clone());
    state.borrow_mut().model.bottom_panel_visible = false;
    state.borrow_mut().open_selected(file.clone());
    drive_until("local project opened in the real editor", || {
        let state = state.borrow();
        matches!(state.model.phase, WorkspacePhase::Ready) && state.editor.is_some()
    });

    crate::panes::show_settings(&state, window.upcast_ref());
    let settings = gtk4::Window::list_toplevels()
        .into_iter()
        .find_map(|widget| widget.downcast::<adw::PreferencesWindow>().ok())
        .unwrap();
    let row = backend_row(
        settings.upcast_ref(),
        &tr("en", "settings.compile.live_backend"),
    )
    .expect("the embedded/compiler backend picker exists on GTK 4.6");
    assert_eq!(row.model().unwrap().n_items(), 2);
    row.set_selected(1);
    assert_eq!(state.borrow().store.live_preview_backend(), "compiler");
    assert!(!state.borrow().model.embedded_wanted(&state.borrow().store));
    row.set_selected(0);
    assert_eq!(state.borrow().store.live_preview_backend(), "embedded");
    assert!(state.borrow().model.embedded_wanted(&state.borrow().store));
    settings.close();

    let editor = state.borrow().editor.as_ref().unwrap().clone();
    let buffer = editor.buffer();
    let mut last_hash = None;
    for marker in ["First unsaved marker.", "Second unsaved marker."] {
        let previous_generation = state.borrow().model.embedded.generation;
        let offset = editor.text().find("\\end{document}").unwrap() as i32;
        buffer.begin_user_action();
        buffer.insert(&mut buffer.iter_at_offset(offset), &format!("{marker}\n"));
        buffer.end_user_action();
        drive_until("unsaved edit reached the model and real PDF viewer", || {
            let state = state.borrow();
            state
                .model
                .document_snapshot
                .as_ref()
                .is_some_and(|s| s.text.contains(marker))
                && state.model.embedded.generation > previous_generation
                && state.model.embedded.status == PreviewStatus::Current
                && state.model.displaying_editing_preview()
                && state
                    .model
                    .retained_pdf
                    .as_ref()
                    .is_some_and(|p| Some(p.hash) != last_hash)
                && state.rendered_pdf_key.get() != 0
                && state.displayed_pdf_key.get() == state.rendered_pdf_key.get()
        });
        let state = state.borrow();
        let pdf = state.model.retained_pdf.as_ref().unwrap();
        assert!(pdf.pdf.starts_with(b"%PDF-"));
        assert!(
            pdf_text(&pdf.pdf).contains(marker),
            "the PDF contains the actual unsaved editor text"
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            source,
            "preview leaves source bytes unchanged"
        );
        assert!(
            state.model.latest_built_pdf_name.is_none(),
            "no project compiler build ran"
        );
        assert!(state.model.build_log_text.is_empty());
        assert!(state
            .preview_header_text()
            .contains(&tr("en", "preview.embedded_label")));
        UI.with(|ui| {
            let picture = ui.pdf_picture.borrow();
            let picture = picture.as_ref().unwrap();
            assert!(
                picture.is_mapped() && picture.paintable().is_some(),
                "the PDF is displayed in the window"
            );
            assert!(ui
                .pdf_name_label
                .borrow()
                .as_ref()
                .unwrap()
                .text()
                .contains(&tr("en", "preview.embedded_label")));
        });
        last_hash = Some(pdf.hash);
    }
    state.borrow_mut().model.stop_embedded();
    state.borrow_mut().shutdown_agent();
    window.close();
    finished.store(true, Ordering::Release);
    std::fs::remove_dir_all(root).unwrap();
    eprintln!(
        "PASS: real GTK backend picker and two unsaved embedded PDF generations; disk unchanged"
    );
}
