//! Real-engine regression for microtype compatibility and interrupted updates.
//! PITEX_BIN must contain both helpers; TeX Live and pdftotext must be on PATH.

use pitex_native_tools::preview_regression;
use pitex_native_tools::TempDir;
use preview_regression::Session;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::fd::FromRawFd;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const PROBES: &str = r"
\newcount\PitexPars
\let\PitexOriginalPar\par
\def\PitexPar{\global\advance\PitexPars1\PitexOriginalPar}
\partokenname\PitexPar
\partokencontext=0 \setbox0=\vbox{Probe zero}
\typeout{PITEX-PAR-ZERO=\the\PitexPars}
\partokencontext=1 \setbox0=\vbox{Probe one}
\typeout{PITEX-PAR-ONE=\the\PitexPars}
\partokencontext=2 \setbox0=\vbox{Probe two\vadjust{Adjustment}}
\typeout{PITEX-PAR-TWO=\the\PitexPars}
\partokenname\par
";

fn probe_lines(path: &Path) -> Result<Vec<String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| line.starts_with("PITEX-PAR-"))
        .map(str::to_owned)
        .collect())
}

fn ensure(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into().into())
    }
}

// The driver launches this executable as its engine when this private variable
// is present. The first worker completes the handshake but cannot checkpoint;
// a later worker executes the real engine with the same arguments and open fd.
#[cfg(unix)]
fn engine_shim(marker: &Path) -> Result<()> {
    if !marker.exists() {
        let fd = std::env::var("TEXPRESSO_FD")?.parse::<i32>()?;
        // The driver gives this worker ownership of the connected socket.
        let mut socket = unsafe { fs::File::from_raw_fd(fd) };
        let mut handshake = [0; 12];
        socket.read_exact(&mut handshake)?;
        ensure(handshake == *b"TEXPRESSOS01", "invalid engine handshake")?;
        socket.write_all(b"TEXPRESSOC01")?;
        fs::File::create(marker)?;
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
    let engine =
        std::env::var_os("PITEX_LIVE_UPDATES_REAL_ENGINE").ok_or("missing real engine path")?;
    let mut args = std::env::args_os();
    let argv0 = args.next().ok_or("missing engine argv0")?;
    let error = Command::new(engine).arg0(argv0).args(args).exec();
    Err(error.into())
}

#[cfg(not(unix))]
fn engine_shim(_: &Path) -> Result<()> {
    Err("the embedded engine regression requires a Unix socket".into())
}

fn update(session: &mut Session, generation: u64, main: &Path, source: &str) -> Result<()> {
    session.send(&json!({"op": "update", "generation": generation,
        "files": [{"path": main, "text": source}], "closed": []}))
}

fn wait_ready(session: &mut Session) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let Some(event) = session.next_event(remaining)? else {
            break;
        };
        ensure(
            event["event"] != "error" && event["event"] != "failed",
            event.to_string(),
        )?;
        if event["event"] == "ready" {
            return Ok(());
        }
    }
    Err("no complete publication for generation 0".into())
}

fn check(event: &Value, marker: &str, expected_probes: &[String]) -> Result<()> {
    ensure(event["errors"] == 0, event.to_string())?;
    let log = event["log"].as_str().ok_or("publication has no log path")?;
    let actual_probes = probe_lines(Path::new(log))?;
    ensure(
        actual_probes == expected_probes,
        format!("{actual_probes:?}"),
    )?;
    let pdf = event["pdf"].as_str().ok_or("publication has no PDF path")?;
    let output =
        preview_regression::output(Command::new("pdftotext").args([pdf, "-"]), None, true)?;
    ensure(
        output.status.success(),
        format!("pdftotext exited: {}", output.status),
    )?;
    ensure(
        String::from_utf8(output.stdout)?.contains(marker),
        format!("preview omitted {marker}"),
    )
}

fn exercise(
    interrupted: bool,
    bin_dir: &Path,
    work: &Path,
    root: &Path,
    main: &Path,
    text: &str,
    expected_probes: &[String],
) -> Result<()> {
    let out = work.join(if interrupted { "interrupted" } else { "normal" });
    fs::create_dir(&out)?;
    let mut command = Command::new(bin_dir.join("pitex-preview"));
    command
        .arg("--root")
        .arg(root)
        .args(["--main", "main.tex", "--out"])
        .arg(&out)
        .arg("--cache")
        .arg(work.join("cache"))
        .stderr(Stdio::null());
    let marker = work.join("worker-ready");
    if interrupted {
        command
            .arg("--engine")
            .arg(std::env::current_exe()?)
            .env("PITEX_LIVE_UPDATES_ENGINE_SHIM", &marker)
            .env(
                "PITEX_LIVE_UPDATES_REAL_ENGINE",
                bin_dir.join("pitex-preview-xetex"),
            );
    }
    let mut session = Session::spawn(&mut command, &out)?;
    wait_ready(&mut session)?;
    update(&mut session, 1, main, text)?;
    if interrupted {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !marker.exists() && Instant::now() < deadline {
            preview_regression::check_interrupt()?;
            thread::sleep(Duration::from_millis(10));
        }
        preview_regression::check_interrupt()?;
        ensure(marker.exists(), "test worker never handshook")?;
        update(
            &mut session,
            2,
            main,
            &text.replace("LiveMarker0", "LiveMarker2"),
        )?;
        check(
            &session.wait_published(2, Duration::from_secs(30))?,
            "LiveMarker2",
            expected_probes,
        )?;
        ensure(
            fs::read_to_string(out.join("driver.log"))?.contains("[kill] worker might be stuck"),
            "interrupted worker was not restarted",
        )?;
        println!("PASS: interrupted worker restarts before its first checkpoint");
        std::io::stdout().flush()?;
        return Ok(());
    }
    check(
        &session.wait_published(1, Duration::from_secs(60))?,
        "LiveMarker0",
        expected_probes,
    )?;
    for generation in 2..12 {
        let marker = format!("LiveMarker{generation}");
        update(
            &mut session,
            generation,
            main,
            &text.replace("LiveMarker0", &marker),
        )?;
        check(
            &session.wait_published(generation, Duration::from_secs(30))?,
            &marker,
            expected_probes,
        )?;
    }
    for generation in 12..32 {
        preview_regression::check_interrupt()?;
        update(
            &mut session,
            generation,
            main,
            &text.replace("LiveMarker0", &format!("LiveMarker{generation}")),
        )?;
        thread::sleep(Duration::from_millis(100));
    }
    check(
        &session.wait_published(31, Duration::from_secs(30))?,
        "LiveMarker31",
        expected_probes,
    )?;
    update(
        &mut session,
        32,
        main,
        &text.replace("LiveMarker0", r"\PitexUndefined LiveMarker32"),
    )?;
    let broken = session.wait_published(32, Duration::from_secs(30))?;
    ensure(
        broken["errors"].as_u64().unwrap_or(0) > 0
            && broken["first_error"]
                .as_str()
                .unwrap_or("")
                .contains("Undefined control sequence"),
        broken.to_string(),
    )?;
    update(
        &mut session,
        33,
        main,
        &text.replace("LiveMarker0", "LiveMarker33"),
    )?;
    check(
        &session.wait_published(33, Duration::from_secs(30))?,
        "LiveMarker33",
        expected_probes,
    )?;
    println!("PASS: microtype, paragraph tokens, repeated edits, error and recovery");
    std::io::stdout().flush()?;
    Ok(())
}

fn run() -> Result<()> {
    if let Some(marker) = std::env::var_os("PITEX_LIVE_UPDATES_ENGINE_SHIM") {
        return engine_shim(Path::new(&marker));
    }
    preview_regression::watch_interrupt();
    preview_regression::check_interrupt()?;
    let bin_dir = PathBuf::from(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN is not set")?);
    // Path.resolve() accepts an empty or nonexistent helper directory. Keep the
    // same current-directory default for an explicitly empty PITEX_BIN value.
    let bin_dir = if bin_dir.is_absolute() {
        bin_dir
    } else {
        std::env::current_dir()?.join(bin_dir)
    };
    let bin_dir = fs::canonicalize(&bin_dir).unwrap_or(bin_dir);
    let temporary = TempDir::new("pitex-live-updates-")?;
    let work = fs::canonicalize(temporary.path())?;
    let root = work.join("project");
    fs::create_dir(&root)?;
    let main = root.join("main.tex");
    let paragraph = format!(
        "{}\\par\n",
        "A paragraph with enough text for protrusion and several pages. ".repeat(12)
    );
    let text = format!(
        "\\documentclass[twocolumn]{{article}}\n\\usepackage{{microtype}}\n\\usepackage{{hyperref}}\n\\begin{{document}}\n{PROBES}\nLiveMarker0.\n{}\\end{{document}}\n",
        paragraph.repeat(35),
    );
    fs::write(&main, &text)?;
    let reference = work.join("reference");
    fs::create_dir(&reference)?;
    let reference_result = preview_regression::output(
        Command::new("xelatex")
            .args(["-interaction=nonstopmode", "-halt-on-error"])
            .arg(format!("-output-directory={}", reference.display()))
            .arg("main.tex")
            .current_dir(&root)
            .stdout(Stdio::null()),
        Some(Duration::from_secs(60)),
        true,
    )?;
    ensure(
        reference_result.status.success(),
        format!("xelatex exited: {}", reference_result.status),
    )?;
    let expected_probes = probe_lines(&reference.join("main.log"))?;
    ensure(
        expected_probes == ["PITEX-PAR-ZERO=0", "PITEX-PAR-ONE=1", "PITEX-PAR-TWO=3"],
        format!("{expected_probes:?}"),
    )?;
    exercise(
        false,
        &bin_dir,
        &work,
        &root,
        &main,
        &text,
        &expected_probes,
    )?;
    exercise(true, &bin_dir, &work, &root, &main, &text, &expected_probes)?;
    preview_regression::check_interrupt()
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
