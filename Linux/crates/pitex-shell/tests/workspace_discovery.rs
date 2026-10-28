//! Regression coverage for `discover_tex_files`/`standardize` — the macOS
//! `FileManager.enumerator` never descends into symlinked directories, so the
//! Rust traversal must not either (an ancestor-pointing link would loop
//! forever otherwise), while symlinked *files* still resolve like
//! `isRegularFileKey`.

use pitex_shell::model::{standardize, WorkspaceModel, MAX_LISTED_FIGURES};
use std::collections::HashSet;
use std::os::unix::fs::PermissionsExt;
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
    let scan = rx
        .recv_timeout(Duration::from_secs(15))
        .expect("discovery hung — a symlinked directory was traversed")
        .unwrap();
    assert!(!scan.limited);
    let files = scan.files;

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

    let scan = WorkspaceModel::discover_tex_files(&root, &root, true).unwrap();
    assert!(!scan.limited);
    assert_eq!(scan.files, vec![standardize(root.join("main.tex"))]);
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn discovery_lists_figures_but_not_build_artifacts() {
    let root = temp_dir("figures");
    std::fs::write(root.join("main.tex"), "\\documentclass{article}").unwrap();
    std::fs::write(root.join("refs.bib"), "").unwrap();
    // Every figure extension the project tree surfaces.
    for name in [
        "plot.png",
        "photo.jpg",
        "scan.jpeg",
        "figure.pdf",
        "chart.eps",
        "icon.svg",
        "anim.gif",
        "scan.tif",
        "print.tiff",
        "raw.bmp",
        "shot.webp",
    ] {
        std::fs::write(root.join(name), "x").unwrap();
    }
    // Generated artifacts stay hidden even though .pdf is a figure
    // extension — artifacts are everything else the build emits.
    for name in [
        "main.aux",
        "main.bbl",
        "main.blg",
        "main.fdb_latexmk",
        "main.fls",
        "main.log",
        "main.synctex.gz",
        "main.xdv",
        "main.out",
        "main.toc",
        "main.lof",
        "main.lot",
        "main.nav",
        "main.snm",
        "main.vrb",
        "main.run.xml",
        "main.bcf",
        "main.idx",
        "main.ind",
        "main.ilg",
        "main.acn",
        "main.acr",
        "main.alg",
        "main.glg",
        "main.glo",
        "main.gls",
        "main.ist",
        "main.dep",
        "main.dpth",
        "main.md5",
        "main.auxlock",
    ] {
        std::fs::write(root.join(name), "x").unwrap();
    }
    // Nested figures are discovered too.
    std::fs::create_dir_all(root.join("figs")).unwrap();
    std::fs::write(root.join("figs/nested.png"), "x").unwrap();

    let scan = WorkspaceModel::discover_tex_files(&root, &root, true).unwrap();
    assert!(!scan.limited);
    let files = scan.files;
    let listed: HashSet<PathBuf> = files.iter().cloned().collect();

    for name in [
        "main.tex",
        "refs.bib",
        "plot.png",
        "photo.jpg",
        "scan.jpeg",
        "figure.pdf",
        "chart.eps",
        "icon.svg",
        "anim.gif",
        "scan.tif",
        "print.tiff",
        "raw.bmp",
        "shot.webp",
        "figs/nested.png",
    ] {
        assert!(
            listed.contains(&standardize(root.join(name))),
            "{name} missing from {files:?}"
        );
    }
    for name in [
        "main.aux",
        "main.bbl",
        "main.blg",
        "main.fdb_latexmk",
        "main.fls",
        "main.log",
        "main.synctex.gz",
        "main.xdv",
        "main.out",
        "main.toc",
        "main.lof",
        "main.lot",
        "main.nav",
        "main.snm",
        "main.vrb",
        "main.run.xml",
        "main.bcf",
        "main.idx",
        "main.ind",
        "main.ilg",
        "main.acn",
        "main.acr",
        "main.alg",
        "main.glg",
        "main.glo",
        "main.gls",
        "main.ist",
        "main.dep",
        "main.dpth",
        "main.md5",
        "main.auxlock",
    ] {
        assert!(
            !listed.contains(&standardize(root.join(name))),
            "artifact {name} leaked into {files:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn initial_document_never_picks_a_figure() {
    // With no .tex mains, the resolver's last-ditch fallback must still land
    // on a text file — never a figure or other discovered file.
    let root = temp_dir("initial");
    std::fs::write(root.join("zeta.tex"), "x").unwrap();
    std::fs::write(root.join("aaa.png"), "x").unwrap();
    std::fs::write(root.join("bbb.pdf"), "x").unwrap();
    let mut resolver = pitex_shell::model::TeXProjectResolver::new();
    let scan = WorkspaceModel::discover_tex_files(&root, &root, true).unwrap();
    assert!(!scan.limited);
    let picked = resolver.initial_document(&scan.files).unwrap();
    assert_eq!(picked, standardize(root.join("zeta.tex")));
    let _ = std::fs::remove_dir_all(&root);
}

fn rss_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            return kb * 1024;
        }
    }
    0
}

struct ModeGuard(PathBuf);
impl Drop for ModeGuard {
    fn drop(&mut self) {
        if let Ok(meta) = std::fs::metadata(&self.0) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&self.0, perms);
        }
    }
}

#[test]
fn discovery_bounds_a_large_folder_and_skips_pruned_and_unreadable_trees() {
    let root = temp_dir("large");
    std::fs::write(root.join("main.tex"), "\\documentclass{article}").unwrap();
    std::fs::create_dir_all(root.join("chapters")).unwrap();
    std::fs::write(root.join("chapters/ch.tex"), "x").unwrap();
    std::fs::create_dir_all(root.join("pics")).unwrap();
    for i in 0..MAX_LISTED_FIGURES + 40 {
        std::fs::write(root.join(format!("pics/f{i}.png")), "x").unwrap();
    }
    std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    std::fs::write(root.join("node_modules/pkg/secret.tex"), "nope").unwrap();
    std::fs::create_dir_all(root.join("venv")).unwrap();
    std::fs::write(root.join("venv/hidden.tex"), "nope").unwrap();
    std::fs::create_dir_all(root.join("__pycache__")).unwrap();
    std::fs::write(root.join("__pycache__/hidden.tex"), "nope").unwrap();
    let locked = root.join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::write(locked.join("secret.tex"), "nope").unwrap();
    let mut perms = std::fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0);
    std::fs::set_permissions(&locked, perms).unwrap();
    let _guard = ModeGuard(locked);

    let before = rss_bytes();
    let started = std::time::Instant::now();
    let scan = WorkspaceModel::discover_tex_files(&root, &root, true).unwrap();
    let elapsed = started.elapsed();
    let after = rss_bytes();

    assert!(scan.limited, "figure cap should mark the listing incomplete");
    assert!(elapsed < Duration::from_secs(5), "scan took {elapsed:?}");
    assert!(
        after < before.saturating_add(64 * 1024 * 1024),
        "rss {before} -> {after}"
    );
    assert!(scan.files.iter().any(|p| p.ends_with("main.tex")));
    assert!(scan.files.iter().any(|p| p.ends_with("ch.tex")));
    for banned in ["node_modules", "venv", "__pycache__", "locked"] {
        assert!(
            !scan.files.iter().any(|p| p.components().any(|c| c.as_os_str() == banned)),
            "{banned} leaked into {:?}",
            scan.files
        );
    }
    let figures = scan
        .files
        .iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .count();
    assert_eq!(figures, MAX_LISTED_FIGURES);
    drop(_guard);
    let _ = std::fs::remove_dir_all(&root);
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
