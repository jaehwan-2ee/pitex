//! Session start cost through the long-lived preview protocol. A cold build
//! whose AUX and TOC files appear in the first pass must still rerun to resolve
//! references, but the rerun resumes the running engine at `\begin{document}`
//! instead of launching a new one (Linux), and the TeX Live file list is asked of
//! kpsewhich once per session rather than once per process.
use pitex_native_tools::{
    preview_regression::{self, Session},
    TempDir,
};
use serde_json::json;
use std::{
    error::Error,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
const TIMEOUT: Duration = Duration::from_secs(120);
const SOURCE: &str = "\\documentclass{article}\n\\usepackage{hyperref}\n  \\begin{document}\n\\tableofcontents\n\\section{First}\\label{first}\nSee Section~\\ref{second} on page~\\pageref{second}.\n\\section{Second}\\label{second}\nSecondBody.\n\\end{document}\n";

fn real_kpsewhich() -> Result<PathBuf, Box<dyn Error>> {
    std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH unset")?)
        .map(|dir| dir.join("kpsewhich"))
        .find(|path| path.is_file())
        .ok_or_else(|| "kpsewhich not on PATH".into())
}

/// Runs one cold session; returns (PDF text, driver log, kpsewhich ls-R calls).
fn cold_session(
    bin: &Path,
    root: &Path,
    cache: &Path,
    path: &str,
    counter: &Path,
) -> Result<(String, String, usize), Box<dyn Error>> {
    let out = root.join(format!("preview-{}", fs::read_dir(root)?.count()));
    fs::create_dir(&out)?;
    let _ = fs::remove_file(counter);
    let mut session = Session::spawn(
        Command::new(bin.join("pitex-preview"))
            .arg("--root")
            .arg(root)
            .arg("--main")
            .arg("main.tex")
            .arg("--out")
            .arg(&out)
            .arg("--cache")
            .arg(cache)
            .env("PATH", path)
            .env_remove("PITEX_PREVIEW_LS_R")
            .stderr(Stdio::null()),
        &out,
    )?;
    session.send(&json!({"op":"update","generation":1,"files":[],"closed":[]}))?;
    let event = session.wait_published(1, TIMEOUT)?;
    if event["errors"] != 0 {
        return Err(format!("preview errors: {event}").into());
    }
    let pdf = event["pdf"].as_str().ok_or("PDF missing")?;
    let text = preview_regression::output(Command::new("pdftotext").args([pdf, "-"]), None, true)?;
    drop(session);
    let calls = fs::read_to_string(counter).unwrap_or_default().lines().count();
    Ok((
        String::from_utf8(text.stdout)?,
        fs::read_to_string(out.join("driver.log"))?,
        calls,
    ))
}

fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(PathBuf::from(
        std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?,
    ))?;
    let temp = TempDir::new("pitex-session-start-regression-")?;
    let work = fs::canonicalize(temp.path())?;
    let shims = work.join("shims");
    fs::create_dir(&shims)?;
    let counter = work.join("ls-r-calls");
    let shim = shims.join("kpsewhich");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\ncase \"$*\" in *ls-R*) echo call >> '{}';; esac\nexec '{}' \"$@\"\n",
            counter.display(),
            real_kpsewhich()?.display()
        ),
    )?;
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755))?;
    let path = format!(
        "{}:{}",
        shims.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let root = work.join("project");
    fs::create_dir(&root)?;
    fs::write(root.join("main.tex"), SOURCE)?;
    let cache = work.join("cache");
    // The first session may build the format; measure the next one.
    cold_session(&bin, &root, &cache, &path, &counter)?;
    let (text, log, calls) = cold_session(&bin, &root, &cache, &path, &counter)?;
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for expected in ["Contents", "1 First", "2 Second", "See Section 2 on page 1"] {
        if !text.contains(expected) {
            return Err(format!("references not resolved by the rerun ({expected}): {text}").into());
        }
    }
    if !log.contains("[rerun]") {
        return Err(format!("no auxiliary rerun: {log}").into());
    }
    // macOS takes no checkpoint before the first page is output (platform
    // fonts are not fork-safe there), so its AUX rerun still starts afresh.
    let launches = log.matches("[process] launched").count();
    if launches != if cfg!(target_os = "macos") { 2 } else { 1 } {
        return Err(format!("the AUX rerun launched {launches} engine processes").into());
    }
    // Each launch would list ls-R again without the driver's exported list.
    if calls != 1 {
        return Err(format!("kpsewhich listed ls-R {calls} times in one session, expected 1").into());
    }
    if fs::read_to_string(root.join("main.tex"))? != SOURCE
        || root.join("main.aux").exists()
        || root.join("main.toc").exists()
    {
        return Err("preview wrote into the project".into());
    }
    println!("PASS: session-start: cold AUX/TOC rerun resolved in one engine process, one ls-R listing");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(if preview_regression::interrupted(error.as_ref()) {
            130
        } else {
            1
        });
    }
}
