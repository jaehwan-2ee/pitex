//! Coverage for `TeXProjectResolver` — the BuildSupport.swift port that
//! resolves main documents from `% !TeX root` directives, the `subfiles`
//! document class, and real include/bibliography edges rather than basenames.

use pitex_shell::model::{standardize, ResolutionError, TeXProjectResolver};
use std::path::{Path, PathBuf};

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "pitex-resolver-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) -> PathBuf {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
    standardize(path.to_path_buf())
}

const MAIN: &str = "\\documentclass{article}\n\\begin{document}\n\\end{document}\n";

#[test]
fn resolve_uses_root_directive_before_heuristics() {
    let root = temp_dir("directive");
    let main = write(&root.join("main.tex"), MAIN);
    let chapter = write(
        &root.join("sections/intro.tex"),
        "% !TeX root = ../main.tex\n\\section{Intro}\n",
    );
    let files = vec![main.clone(), chapter.clone()];
    let mut resolver = TeXProjectResolver::new();
    let resolved = resolver
        .resolve(Some(&chapter), &files, None)
        .unwrap();
    assert_eq!(resolved, Some(main));
}

#[test]
fn resolve_accepts_subfiles_documentclass_hint() {
    let root = temp_dir("subfiles");
    let main = write(&root.join("main.tex"), MAIN);
    let chapter = write(
        &root.join("chapters/one.tex"),
        "\\documentclass[../main.tex]{subfiles}\n\\begin{document}\n\\end{document}\n",
    );
    let files = vec![main.clone(), chapter.clone()];
    let mut resolver = TeXProjectResolver::new();
    let resolved = resolver
        .resolve(Some(&chapter), &files, None)
        .unwrap();
    assert_eq!(resolved, Some(main));
}

#[test]
fn resolve_finds_owner_through_nested_include() {
    let root = temp_dir("include");
    let main = write(
        &root.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\input{sections/intro}\n\\end{document}\n",
    );
    let chapter = write(&root.join("sections/intro.tex"), "\\section{Intro}\n");
    let files = vec![main.clone(), chapter.clone()];
    let mut resolver = TeXProjectResolver::new();
    let resolved = resolver
        .resolve(Some(&chapter), &files, None)
        .unwrap();
    assert_eq!(resolved, Some(main));
}

#[test]
fn resolve_maps_bibliography_and_bbl_to_main() {
    let root = temp_dir("biblio");
    let main = write(
        &root.join("main.tex"),
        "\\documentclass{article}\n\\addbibresource{refs.bib}\n\\begin{document}\n\\end{document}\n",
    );
    let bib = write(&root.join("refs.bib"), "@book{a, title={A}}\n");
    let bbl = write(&root.join("main.bbl"), "");
    let files = vec![main.clone(), bib.clone(), bbl.clone()];
    let mut resolver = TeXProjectResolver::new();
    assert_eq!(
        resolver.resolve(Some(&bib), &files, None).unwrap(),
        Some(main.clone())
    );
    // A .bbl active document binds to the same-stem main document.
    assert_eq!(
        resolver.resolve(Some(&bbl), &files, None).unwrap(),
        Some(main)
    );
}

#[test]
fn resolve_ambiguous_mains_error() {
    let root = temp_dir("ambiguous");
    let shared = write(&root.join("shared/part.tex"), "\\section{S}\n");
    let a = write(
        &root.join("a.tex"),
        "\\documentclass{article}\n\\input{shared/part}\n",
    );
    let b = write(
        &root.join("b.tex"),
        "\\documentclass{article}\n\\input{shared/part}\n",
    );
    let files = vec![a, b, shared.clone()];
    let mut resolver = TeXProjectResolver::new();
    assert_eq!(
        resolver.resolve(Some(&shared), &files, None),
        Err(ResolutionError::Ambiguous)
    );
}

#[test]
fn resolve_without_main_returns_none() {
    let root = temp_dir("nomains");
    let chapter = write(&root.join("chapter.tex"), "\\section{Only}\n");
    let files = vec![chapter.clone()];
    let mut resolver = TeXProjectResolver::new();
    assert_eq!(resolver.resolve(Some(&chapter), &files, None).unwrap(), None);
}

#[test]
fn root_directive_cycle_is_invalid() {
    let root = temp_dir("cycle");
    let a = write(&root.join("a.tex"), "% !TeX root = b.tex\n");
    write(&root.join("b.tex"), "% !TeX root = a.tex\n");
    let files = vec![a.clone()];
    let mut resolver = TeXProjectResolver::new();
    assert_eq!(
        resolver.resolve(Some(&a), &files, None),
        Err(ResolutionError::InvalidRoot)
    );
}

#[test]
fn unsaved_active_text_drives_resolution() {
    let root = temp_dir("activetext");
    let main = write(&root.join("main.tex"), MAIN);
    // On disk the chapter has no directive yet; the editor buffer does.
    let chapter = write(&root.join("chapter.tex"), "\\section{C}\n");
    let files = vec![main.clone(), chapter.clone()];
    let mut resolver = TeXProjectResolver::new();
    resolver.active_text = Some((
        chapter.clone(),
        "% !TeX root = main.tex\n\\section{C}\n".to_string(),
    ));
    let resolved = resolver
        .resolve(Some(&chapter), &files, None)
        .unwrap();
    assert_eq!(resolved, Some(main));
}

#[test]
fn initial_document_prefers_main_tex() {
    let root = temp_dir("initial");
    let chapter = write(&root.join("aaa_chapter.tex"), "\\section{C}\n");
    let main = write(&root.join("main.tex"), MAIN);
    let files = vec![chapter, main.clone()];
    let mut resolver = TeXProjectResolver::new();
    assert_eq!(resolver.initial_document(&files), Some(main));
}

#[test]
fn project_root_for_nested_chapter_finds_owner() {
    let root = temp_dir("projectroot");
    write(
        &root.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\input{sections/intro}\n\\end{document}\n",
    );
    let chapter = write(&root.join("sections/intro.tex"), "\\section{I}\n");
    let mut resolver = TeXProjectResolver::new();
    assert_eq!(
        resolver.project_root(&chapter),
        standardize(root.clone())
    );
}

#[test]
fn project_root_without_owner_stays_local() {
    let root = temp_dir("localroot");
    let file = write(&root.join("standalone.tex"), "\\section{S}\n");
    let mut resolver = TeXProjectResolver::new();
    assert_eq!(
        resolver.project_root(&file),
        standardize(file.parent().unwrap().to_path_buf())
    );
}

#[test]
fn project_root_expands_to_cover_dependencies() {
    // The chapter declares its main in a sibling subtree — the resolved root
    // must climb until it covers the chapter as well as the main's deps.
    let base = temp_dir("expandroot");
    let main = write(
        &base.join("book/main.tex"),
        "\\documentclass{article}\n\\input{../parts/one}\n",
    );
    let chapter = write(
        &base.join("parts/one.tex"),
        "% !TeX root = ../book/main.tex\n\\section{O}\n",
    );
    let mut resolver = TeXProjectResolver::new();
    let root = resolver.project_root(&chapter);
    assert!(standardize(main.clone()).starts_with(&root));
    assert!(chapter.starts_with(&root));
    assert_eq!(root, standardize(base.clone()));
}
