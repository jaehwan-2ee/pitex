//! Port of `EquationExactRendererTests.swift`.

use build_core::equation_exact::{
    ExactEquationEngine, ExactEquationProfile, ExactEquationRenderError, ExactEquationRenderer,
    ExactEquationShellEscape,
};
use build_core::CancellationToken;
use language_core::equation_preview::{ExactEquationDocument, MathSourceScanner};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn require_engine(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| {
            #[cfg(windows)]
            if dir.join(format!("{name}.exe")).is_file() { return true; }
            dir.join(name).is_file()
        }))
        .unwrap_or(false)
}

#[test]
fn profile_follows_the_project_build_command() {
    use ExactEquationEngine::*;
    use ExactEquationShellEscape::*;
    let profile = |engine, shell_escape| ExactEquationProfile { engine, shell_escape };
    let cases: [(&str, Option<ExactEquationProfile>); 10] = [
        (
            "xelatex -interaction=nonstopmode -synctex=1 {file}",
            Some(profile(XeLaTeX, ProjectDefault)),
        ),
        ("pdflatex -shell-escape {file}", Some(profile(PdfLaTeX, Enabled))),
        (
            "/Library/TeX/texbin/lualatex --no-shell-escape {file}",
            Some(profile(LuaLaTeX, Disabled)),
        ),
        (
            "latexmk -pdf -interaction=nonstopmode -synctex=1 {file}",
            Some(profile(PdfLaTeX, ProjectDefault)),
        ),
        (
            "latexmk -xelatex -shell-restricted {file}",
            Some(profile(XeLaTeX, Restricted)),
        ),
        ("latexmk -pdflua {file}", Some(profile(LuaLaTeX, ProjectDefault))),
        ("tectonic --synctex {file}", None),
        ("make pdf", None),
        ("pdflatex {file} && bibtex main", None),
        ("", None),
    ];
    for (command, expected) in cases {
        assert_eq!(ExactEquationProfile::resolve(command), expected, "{command}");
    }
}

#[test]
fn profile_recognizes_windows_tex_paths_without_parsing_shell_arguments() {
    let expected = Some(ExactEquationProfile {
        engine: ExactEquationEngine::XeLaTeX,
        shell_escape: ExactEquationShellEscape::Disabled,
    });
    for command in [
        r#"C:\texlive\2026\bin\windows\xelatex.exe -no-shell-escape {file}"#,
        r#""C:\Program Files\MiKTeX\miktex\bin\x64\xelatex.exe" -no-shell-escape {file}"#,
    ] {
        assert_eq!(ExactEquationProfile::resolve(command), expected, "{command}");
    }
    for command in [
        r#""C:\Program Files\MiKTeX\xelatex.exe"-shell-escape {file}"#,
        r#""C:\Program Files\MiKTeX\xelatex.exe -shell-escape {file}"#,
        r#"latexmk.exe -pdflatex="lualatex -shell-escape" {file}"#,
        r#"xelatex.exe {file} && bibtex.exe main"#,
    ] {
        assert_eq!(ExactEquationProfile::resolve(command), None, "{command}");
    }
}

#[test]
fn arguments_carry_the_shell_escape_policy_and_nothing_from_the_document() {
    let disabled = ExactEquationProfile {
        engine: ExactEquationEngine::PdfLaTeX,
        shell_escape: ExactEquationShellEscape::Disabled,
    }
    .arguments("/tmp/x");
    assert_eq!(
        disabled,
        vec![
            "-interaction=batchmode",
            "-halt-on-error",
            "-output-directory=/tmp/x",
            "-jobname=pitex-equation",
            "-no-shell-escape",
            "pitex-equation.tex"
        ]
    );
    let lua = ExactEquationProfile {
        engine: ExactEquationEngine::LuaLaTeX,
        shell_escape: ExactEquationShellEscape::ProjectDefault,
    }
    .arguments("/o");
    assert!(!lua.iter().any(|a| a.contains("shell")));
    assert!(lua.iter().all(|a| a.starts_with("--") || a == "pitex-equation.tex"));
}

fn test_token() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!("test-{}-{}", std::process::id(), COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

fn renderer(token: &str, main_directory: PathBuf) -> ExactEquationRenderer {
    ExactEquationRenderer::new(
        ExactEquationProfile {
            engine: ExactEquationEngine::PdfLaTeX,
            shell_escape: ExactEquationShellEscape::Disabled,
        },
        main_directory,
        HashMap::new(),
        token.to_string(),
    )
}

fn workspace_contents(token: &str) -> Vec<String> {
    let root = ExactEquationRenderer::workspace_root(token);
    std::fs::read_dir(&root)
        .map(|entries| {
            let mut names: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        })
        .unwrap_or_default()
}

/// A real minimal document through the project's engine: PDF bytes come
/// back, the project directory resolves preamble inputs, and nothing is
/// left in the temporary tree.
#[test]
fn renders_with_project_preamble_and_cleans_up() {
    if !require_engine("pdflatex") {
        // Honest skip reporting: stdout marker survives --nocapture-only
        // runs and reads as a skip, not a silently green pass (C13).
        eprintln!("SKIPPED: pdflatex is not installed");
        println!("SKIPPED: pdflatex is not installed");
        return;
    }
    let project = std::env::temp_dir().join(format!("pitex-exact-test-{}", test_token()));
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("commands.tex"), "\\newcommand{\\R}{\\mathbb{R}}\n").unwrap();
    let main = "\\documentclass{article}\n\\usepackage{amssymb}\n\\input{commands}\n\\begin{document}\n$x \\in \\R$\n\\end{document}\n";
    let scan = MathSourceScanner::scan(main);
    let document = ExactEquationDocument::make(
        scan.preamble.as_deref(),
        &[],
        &[],
        &scan.regions[0].source_text(main),
    );
    let token = test_token();
    let r = renderer(&token, project.clone());
    let pdf = r.render(&document, None).unwrap();
    assert_eq!(&pdf[..5], b"%PDF-");
    // Second render through the SAME workspace token: the base must be
    // reused, not re-created (mirrors the Swift reuse bug).
    let main2 = "\\documentclass{article}\n\\usepackage{amssymb}\n\\input{commands}\n\\begin{document}\n$y \\notin \\R$\n\\end{document}\n";
    let scan2 = MathSourceScanner::scan(main2);
    let document2 = ExactEquationDocument::make(
        scan2.preamble.as_deref(),
        &[],
        &[],
        &scan2.regions[0].source_text(main2),
    );
    let pdf2 = r.render(&document2, None).unwrap();
    assert_eq!(&pdf2[..5], b"%PDF-");
    assert_ne!(pdf, pdf2);
    assert_eq!(workspace_contents(&token), Vec::<String>::new());
    ExactEquationRenderer::remove_workspace_artifacts(&token);
    assert!(!ExactEquationRenderer::workspace_root(&token).exists());
    let mut project_files: Vec<String> = std::fs::read_dir(&project)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    project_files.sort();
    assert_eq!(project_files, ["commands.tex"]);
    let _ = std::fs::remove_dir_all(&project);
}

#[test]
fn unknown_macro_fails_without_output() {
    if !require_engine("pdflatex") {
        // Honest skip reporting: stdout marker survives --nocapture-only
        // runs and reads as a skip, not a silently green pass (C13).
        eprintln!("SKIPPED: pdflatex is not installed");
        println!("SKIPPED: pdflatex is not installed");
        return;
    }
    let token = test_token();
    let r = renderer(&token, std::env::temp_dir());
    let document =
        ExactEquationDocument::make(None, &[], &[], "$\\undefinedmacro$");
    match r.render(&document, None) {
        Err(ExactEquationRenderError::CompileFailed { log }) => {
            assert!(log.contains("Undefined control sequence"), "{log}");
        }
        other => panic!("expected a compile failure, got {other:?}"),
    }
    assert_eq!(workspace_contents(&token), Vec::<String>::new());
    ExactEquationRenderer::remove_workspace_artifacts(&token);
}

/// Shell escape stays off when the project has it off, even if the document
/// asks for \write18.
#[test]
fn disabled_shell_escape_is_enforced() {
    if !require_engine("pdflatex") {
        // Honest skip reporting: stdout marker survives --nocapture-only
        // runs and reads as a skip, not a silently green pass (C13).
        eprintln!("SKIPPED: pdflatex is not installed");
        println!("SKIPPED: pdflatex is not installed");
        return;
    }
    let token = test_token();
    let marker = std::env::temp_dir().join(format!("pitex-write18-{}", test_token()));
    let r = renderer(&token, std::env::temp_dir());
    #[cfg(windows)]
    let shell_command = format!("echo pitex > \"{}\"", marker.to_string_lossy().replace('\\', "/"));
    #[cfg(unix)]
    let shell_command = format!("touch \"{}\"", marker.display());
    let document = ExactEquationDocument::make(
        Some(&format!(
            "\\documentclass{{article}}\n\\immediate\\write18{{{}}}\n",
            shell_command
        )),
        &[],
        &[],
        "$x$",
    );
    r.render(&document, None).expect("disabled shell escape must still compile the equation");
    assert!(!Path::new(&marker).exists());
    ExactEquationRenderer::remove_workspace_artifacts(&token);
}

#[test]
fn cancellation_stops_the_engine_and_cleans_up() {
    if !require_engine("pdflatex") {
        // Honest skip reporting: stdout marker survives --nocapture-only
        // runs and reads as a skip, not a silently green pass (C13).
        eprintln!("SKIPPED: pdflatex is not installed");
        println!("SKIPPED: pdflatex is not installed");
        return;
    }
    let token = test_token();
    let r = renderer(&token, std::env::temp_dir());
    // An endless loop keeps the engine busy until cancellation.
    let document = "\\documentclass{article}\n\\begin{document}\n\\def\\loop{\\loop}\\loop\n\\end{document}\n";
    let cancel = CancellationToken::new();
    let thread_cancel = cancel.clone();
    let handle = std::thread::spawn(move || r.render(document, Some(&thread_cancel)));
    std::thread::sleep(std::time::Duration::from_millis(300));
    cancel.cancel();
    match handle.join().unwrap() {
        Err(ExactEquationRenderError::Cancelled) => {}
        other => panic!("expected cancellation, got {other:?}"),
    }
    assert_eq!(workspace_contents(&token), Vec::<String>::new());
    ExactEquationRenderer::remove_workspace_artifacts(&token);
}
