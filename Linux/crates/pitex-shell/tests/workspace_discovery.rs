//! Regression coverage for `discover_tex_files`/`standardize` — the macOS
//! `FileManager.enumerator` never descends into symlinked directories, so the
//! Rust traversal must not either (an ancestor-pointing link would loop
//! forever otherwise), while symlinked *files* still resolve like
//! `isRegularFileKey`.

use pitex_shell::model::{standardize, WorkspaceModel};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "pitex-discovery-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn discovery_does_not_descend_into_symlink_cycles() {
    let root = temp_dir("cycle");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::create_dir_all(root.join("shared")).unwrap();
    std::fs::write(root.join("main.tex"), "\\documentclass{article}").unwrap();
    std::fs::write(root.join("sub/chapter.tex"), "x").unwrap();
    std::fs::write(root.join("shared/extra.tex"), "x").unwrap();
    // Ancestor and self links would loop a link-following descent forever.
    std::os::unix::fs::symlink("..", root.join("sub/back")).unwrap();
    std::os::unix::fs::symlink(".", root.join("sub/self")).unwrap();
    // Non-cyclic dir link: the enumerator lists it but does not traverse it.
    std::os::unix::fs::symlink("../shared", root.join("sub/alias")).unwrap();
    // File links resolve through to their target.
    std::os::unix::fs::symlink("main.tex", root.join("link.tex")).unwrap();

    // Run on a thread so a regression hangs a watchdog, not the harness.
    let probe = root.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(WorkspaceModel::discover_tex_files(&probe, &probe, true));
    });
    let files = rx
        .recv_timeout(Duration::from_secs(15))
        .expect("discovery hung — a symlinked directory was traversed")
        .unwrap();

    let mut expected = vec![
        standardize(root.join("main.tex")),
        standardize(root.join("shared/extra.tex")),
        standardize(root.join("sub/chapter.tex")),
    ];
    expected.sort();
    let mut actual = files.clone();
    actual.sort();
    assert_eq!(actual, expected);
    // link.tex resolved to main.tex must not produce a duplicate entry.
    assert_eq!(
        files.iter().collect::<HashSet<_>>().len(),
        files.len(),
        "duplicate entries in {files:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn discovery_skips_symlinks_escaping_the_project_root() {
    let root = temp_dir("project");
    let outside = temp_dir("outside");
    std::fs::write(root.join("main.tex"), "x").unwrap();
    std::fs::write(outside.join("secret.tex"), "x").unwrap();
    std::os::unix::fs::symlink(outside.join("secret.tex"), root.join("evil.tex")).unwrap();

    let files = WorkspaceModel::discover_tex_files(&root, &root, true).unwrap();
    assert_eq!(files, vec![standardize(root.join("main.tex"))]);
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn standardize_keeps_unresolvable_dotdot_on_relative_paths() {
    // Nonexistent components force the lexical (non-canonicalize) branch.
    assert_eq!(
        standardize(PathBuf::from("pitex-missing-xyz/../../keep")),
        PathBuf::from("../keep")
    );
    assert_eq!(
        standardize(PathBuf::from("pitex-missing-xyz/../keep")),
        PathBuf::from("keep")
    );
    // Absolute paths still clamp `..` at the root.
    assert_eq!(
        standardize(PathBuf::from("/pitex-missing-xyz/../..")),
        PathBuf::from("/")
    );
    // Existing paths canonicalize as before.
    assert_eq!(
        standardize(PathBuf::from("/tmp/../tmp")),
        PathBuf::from("/tmp").canonicalize().unwrap()
    );
}
