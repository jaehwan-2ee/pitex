use pitex_shell::embedded_preview::PreviewStatus;
use pitex_shell::model::{WorkspaceMessage, WorkspaceModel, WorkspaceSyncTeXState};
use pitex_shell::settings::{Preferences, SettingsStore};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

fn temp_project(tag: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "pitex-open-synctex-{tag}-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn write_source(root: &Path) -> PathBuf {
    let tex = root.join("main.tex");
    std::fs::write(
        &tex,
        "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    )
    .unwrap();
    tex
}

fn write_pdf(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, b"%PDF-1.4\n% existing output\n").unwrap();
}

fn write_synctex(path: &Path, source: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(
        path,
        format!(
            "SyncTeX Version:1\nInput:1:{}\n",
            source.display()
        ),
    )
    .unwrap();
}

fn set_mtime(path: &Path, when: SystemTime) {
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(when).unwrap();
}

fn gzip_synctex(plain: &Path, gz: &Path) {
    let mut child = std::process::Command::new("gzip")
        .arg("-c")
        .arg(plain)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("gzip");
    let mut out = std::fs::File::create(gz).unwrap();
    std::io::copy(child.stdout.as_mut().unwrap(), &mut out).unwrap();
    assert!(child.wait().unwrap().success(), "gzip failed");
    std::fs::remove_file(plain).unwrap();
}

struct Opened {
    model: WorkspaceModel,
    rx: std::sync::mpsc::Receiver<WorkspaceMessage>,
    store: SettingsStore,
    root: PathBuf,
}

fn open_project(root: &Path) -> Opened {
    let mut store = SettingsStore::new(Preferences::standard());
    store.set_live_compile_enabled(false);
    let mut model = WorkspaceModel::new();
    let (tx, rx) = std::sync::mpsc::channel();
    model.set_event_sink(tx.clone());
    model.open(root.to_path_buf(), tx);
    let message = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let WorkspaceMessage::OpenFinished(result) = message else {
        panic!("open did not finish, got {message:?}");
    };
    let opened = result.unwrap_or_else(|err| {
        let message = match err {
            pitex_shell::model::OpenFailure::Error(message) => message,
            pitex_shell::model::OpenFailure::RemoteOpen { error, .. } => error,
        };
        panic!("open failed: {message}");
    });
    model.apply_open(&mut store, opened);
    Opened {
        model,
        rx,
        store,
        root: root.to_path_buf(),
    }
}

fn pump_binding(opened: &mut Opened) -> WorkspaceMessage {
    loop {
        let message = opened
            .rx
            .recv_timeout(Duration::from_secs(5))
            .expect("SyncTeX binding was not requested for the existing PDF");
        if matches!(message, WorkspaceMessage::BindingRefreshed { .. }) {
            return message;
        }
    }
}

fn apply_binding(opened: &mut Opened, message: WorkspaceMessage) {
    let WorkspaceMessage::BindingRefreshed {
        id,
        root,
        pdf,
        result,
    } = message
    else {
        unreachable!();
    };
    let result = result.map_err(|err| format!("binding the existing SyncTeX failed: {err}"));
    if let Err(err) = &result {
        panic!("{err}");
    }
    opened.model.apply_binding_refreshed(id, root, pdf, result);
}

fn assert_resolved(opened: &Opened, pdf: &Path) {
    assert!(
        opened.model.embedded.pending_since.is_none(),
        "a matching SyncTeX file must not start another compile"
    );
    assert!(
        opened.model.active_run.is_none(),
        "a matching SyncTeX file must not start the external compiler"
    );
    assert_eq!(
        opened
            .model
            .retained_pdf
            .as_ref()
            .map(|r| r.pdf.as_ref()),
        Some(&b"%PDF-1.4\n% existing output\n"[..]),
        "the existing PDF stays on screen"
    );
    assert!(
        matches!(opened.model.synctex_state, WorkspaceSyncTeXState::Current),
        "SyncTeX did not become current after open: {:?}",
        opened.model.synctex_state
    );
    let binding = opened
        .model
        .synctex_binding
        .as_ref()
        .expect("SyncTeX binding missing");
    assert_eq!(binding.pdf_url, pdf.canonicalize().unwrap_or_else(|_| pdf.to_path_buf()));
    let source = opened.root.join("main.tex").canonicalize().unwrap();
    assert!(
        binding.source_paths.values().any(|p| p == &source),
        "forward sync has no recorded input for main.tex: {:?}",
        binding.source_paths
    );
}

#[test]
fn open_existing_output_resolves_synctex_without_a_manual_compile() {
    let data = temp_project("xdg");
    std::env::set_var("XDG_DATA_HOME", &data);
    std::env::set_var("XDG_CONFIG_HOME", data.join("config"));

    let mut failures = Vec::new();
    for (name, scenario) in [
        ("matching sibling", matching_sibling_binds_and_does_not_compile as fn()),
        ("gzip companion", gzip_companion_binds_the_same_way as fn()),
        (
            "live leftover",
            live_leftover_without_inputs_loses_to_the_sibling_pair as fn(),
        ),
        (
            "pdf newer than synctex",
            pdf_newer_than_synctex_compiles_in_the_background as fn(),
        ),
        (
            "pdf without synctex",
            pdf_without_synctex_compiles_in_the_background as fn(),
        ),
    ] {
        if let Err(err) = std::panic::catch_unwind(scenario) {
            let message = err
                .downcast_ref::<String>()
                .map(|s| s.clone())
                .or_else(|| err.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_else(|| "scenario panicked".into());
            failures.push(format!("{name}: {message}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn matching_sibling_binds_and_does_not_compile() {
    let root = temp_project("match");
    let tex = write_source(&root);
    let pdf = root.join("main.pdf");
    let sync = root.join("main.synctex");
    write_pdf(&pdf);
    write_synctex(&sync, &tex);
    let now = SystemTime::now();
    set_mtime(&pdf, now.checked_sub(Duration::from_secs(30)).unwrap());
    set_mtime(&sync, now);

    let mut opened = open_project(&root);
    let message = pump_binding(&mut opened);
    let WorkspaceMessage::BindingRefreshed { pdf: bound, .. } = &message else {
        unreachable!();
    };
    assert_eq!(bound, &pdf, "bound a different PDF than the one on screen");
    apply_binding(&mut opened, message);
    assert_resolved(&opened, &pdf);
    let _ = std::fs::remove_dir_all(&root);
}

fn gzip_companion_binds_the_same_way() {
    let root = temp_project("gzip");
    let tex = write_source(&root);
    let pdf = root.join("main.pdf");
    let plain = root.join("main.synctex");
    write_pdf(&pdf);
    write_synctex(&plain, &tex);
    let gz = root.join("main.synctex.gz");
    gzip_synctex(&plain, &gz);
    let now = SystemTime::now();
    set_mtime(&pdf, now.checked_sub(Duration::from_secs(30)).unwrap());
    set_mtime(&gz, now);

    let mut opened = open_project(&root);
    let message = pump_binding(&mut opened);
    apply_binding(&mut opened, message);
    assert_resolved(&opened, &pdf);
    let _ = std::fs::remove_dir_all(&root);
}

fn live_leftover_without_inputs_loses_to_the_sibling_pair() {
    let root = temp_project("leftover");
    let tex = write_source(&root);
    let live_dir = root.join(".pitex-live/main");
    write_pdf(&live_dir.join("main.pdf"));
    std::fs::write(live_dir.join("main.synctex"), b"SyncTeX Version:1\n").unwrap();
    let pdf = root.join("main.pdf");
    write_pdf(&pdf);
    let sync = root.join("main.synctex");
    write_synctex(&sync, &tex);
    let now = SystemTime::now();
    set_mtime(&pdf, now.checked_sub(Duration::from_secs(30)).unwrap());
    set_mtime(&sync, now);

    let mut opened = open_project(&root);
    let message = pump_binding(&mut opened);
    let WorkspaceMessage::BindingRefreshed { pdf: bound, .. } = &message else {
        unreachable!();
    };
    assert_eq!(
        bound, &pdf,
        "a .pitex-live companion with no inputs hid the sibling pair"
    );
    apply_binding(&mut opened, message);
    assert_resolved(&opened, &pdf);
    assert_eq!(
        opened.model.retained_pdf.as_ref().map(|r| r.artifact.as_str()),
        Some("main.pdf")
    );
    let _ = std::fs::remove_dir_all(&root);
}

fn pdf_newer_than_synctex_compiles_in_the_background() {
    let root = temp_project("stale");
    let tex = write_source(&root);
    let pdf = root.join("main.pdf");
    let sync = root.join("main.synctex");
    write_pdf(&pdf);
    write_synctex(&sync, &tex);
    let now = SystemTime::now();
    set_mtime(&sync, now.checked_sub(Duration::from_secs(60)).unwrap());
    set_mtime(&pdf, now);

    let mut opened = open_project(&root);
    assert_background_compile(&mut opened);
    let _ = std::fs::remove_dir_all(&root);
}

fn pdf_without_synctex_compiles_in_the_background() {
    let root = temp_project("nosync");
    write_source(&root);
    write_pdf(&root.join("main.pdf"));
    let mut opened = open_project(&root);
    assert_background_compile(&mut opened);
    let _ = std::fs::remove_dir_all(&root);
}

fn assert_background_compile(opened: &mut Opened) {
    assert!(
        opened.model.retained_pdf.is_some(),
        "the existing PDF must stay visible while SyncTeX is produced"
    );
    let shown = opened.model.retained_pdf.as_ref().unwrap().pdf.clone();
    assert!(
        opened.model.embedded.pending_since.is_some(),
        "opening without a matching SyncTeX file did not arm the built-in compiler"
    );
    assert!(
        opened.model.active_run.is_none(),
        "the recovery compile must not also start the external compiler"
    );
    opened.model.flush_embedded_preview(&opened.store);
    match &opened.model.embedded.status {
        PreviewStatus::Unavailable(reason) => {
            assert!(
                reason.contains("not found") || reason.contains("embedded"),
                "recovery did not reach the built-in compiler: {reason}"
            );
        }
        PreviewStatus::Updating | PreviewStatus::Current => {}
        other => panic!("built-in compiler was not started, status {other:?}"),
    }
    assert_eq!(
        opened.model.retained_pdf.as_ref().map(|r| r.pdf.clone()),
        Some(shown),
        "starting the compiler cleared the PDF that was already on screen"
    );
    assert!(
        !matches!(opened.model.synctex_state, WorkspaceSyncTeXState::Current),
        "a stale or missing companion was treated as current"
    );
    let _ = opened.rx.recv_timeout(Duration::from_millis(200));
}
