//! Pitex — native Linux/Windows port of the macOS LaTeX environment
//! (Ubuntu 24.04 full build; 22.04 via `--no-default-features` and Windows
//! via `modern-gtk` without `vte`, see `compat`).
//!
//! GUI entry point: `pitex_shell::run()`. The workspace model, SyncTeX
//! runner, agent coordinator, settings store, PDF renderer and localization
//! tables are platform-neutral; `app_ui` binds them to GTK4/libadwaita.

pub mod agent;
mod catalog;
pub mod app_ui;
pub mod chat_markdown;
mod chat_view;
pub mod compat;
pub mod completion;
pub mod composer;
pub use app_ports::identity;
#[cfg(feature = "equation-preview")]
pub mod equation_preview;
pub mod embedded_preview;

pub mod fold;
pub mod ghost_completion;
pub mod git;
pub mod git_diff;
mod git_list;
pub mod l10n;
pub mod markdown_preview;
pub mod model;
pub mod panes;
pub mod pdf;
pub mod remote;
mod search;
mod ssh_ui;
pub mod settings;
pub mod synctex;
#[cfg(not(feature = "vte"))]
pub mod terminal;
pub mod turn_state;
pub mod turn_view;
pub mod update;
mod window_ipc;

// Unit tests that mutate/read the process PATH must not overlap.
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, unix))]
/// Writes an executable test fixture from a child process. A write descriptor held in
/// this multithreaded test process can leak into another thread's fork and make
/// exec fail with ETXTBSY ("Text file busy").
pub(crate) fn write_test_executable(path: &std::path::Path, body: &str, mode: u32) {
    use std::io::Write;
    let mut child = std::process::Command::new("/bin/sh")
        .args(["-c", "cat > \"$1\" && chmod \"$2\" \"$1\"", "sh"])
        .arg(path)
        .arg(format!("{mode:o}"))
        .env("PATH", "/usr/bin:/bin")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(body.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "could not write {}", path.display());
}

/// Launch the native application (GTK4 main loop). `app_version` is the
/// semantic version the updater compares against release tags.
pub fn run(app_version: &str) -> i32 {
    app_ui::run(app_version)
}
