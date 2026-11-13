//! A recompile whose PDF, log, and SyncTeX match the kept publication must
//! not write those bytes again. A real edit still publishes a new PDF.
//!
//! PITEX_BIN must contain both helpers. TeX Live must be on PATH.

use pitex_native_tools::{
    preview_regression::{self, Session},
    TempDir,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
use std::error::Error;

const DOC: &str = r"\documentclass{article}
\begin{document}
Hello from the idle preview sample.
\end{document}
";

fn ensure(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into().into())
    }
}

fn sha256(path: &str) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn bytes_written(event: &Value) -> Result<u64> {
    event["bytes_written"]
        .as_u64()
        .ok_or_else(|| format!("published event missing bytes_written: {event}").into())
}

fn collect(session: &mut Session, generation: u64, timeout: Duration) -> Result<Vec<Value>> {
    let deadline = Instant::now() + timeout;
    let mut events = Vec::new();
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(format!("no complete publication for generation {generation}").into());
        };
        let Some(event) = session.next_event(remaining.min(Duration::from_secs(1)))? else {
            continue;
        };
        if matches!(event["event"].as_str(), Some("failed" | "error")) {
            return Err(event.to_string().into());
        }
        if event["event"] == "published" || event["event"] == "idle" {
            events.push(event);
        }
        let last = events.last().unwrap();
        if last["event"] == "published" && last["complete"] == true && last["generation"] == generation
        {
            return Ok(events);
        }
        if last["event"] == "idle" && last["generation"] == generation {
            return Ok(events);
        }
    }
}

fn run() -> Result<()> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(PathBuf::from(
        std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?,
    ))?;
    let temp = TempDir::new("pitex-publish-writes-")?;
    let root = fs::canonicalize(temp.path())?;
    let out = root.join("out");
    let cache = root.join("cache");
    fs::create_dir(&out)?;
    fs::create_dir(&cache)?;
    let main = root.join("main.tex");
    fs::write(&main, DOC)?;
    let mut session = Session::spawn(
        Command::new(bin.join("pitex-preview"))
            .arg("--root")
            .arg(&root)
            .arg("--main")
            .arg("main.tex")
            .arg("--out")
            .arg(&out)
            .arg("--cache")
            .arg(&cache)
            .stderr(Stdio::null()),
        &out,
    )?;
    let ready_deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if Instant::now() > ready_deadline {
            return Err("no ready event".into());
        }
        let Some(event) = session.next_event(Duration::from_secs(1))? else {
            continue;
        };
        if event["event"] == "ready" {
            break;
        }
        if matches!(event["event"].as_str(), Some("failed" | "error")) {
            return Err(event.to_string().into());
        }
    }

    session.send(&json!({
        "op": "update",
        "generation": 1,
        "files": [{"path": &main, "text": DOC}],
        "closed": []
    }))?;
    let first = collect(&mut session, 1, Duration::from_secs(90))?;
    let mut seen: HashMap<String, (u64, u64)> = HashMap::new();
    let mut kept_pdf: Option<(String, u64)> = None;
    for event in &first {
        if event["event"] != "published" {
            continue;
        }
        let pdf = event["pdf"].as_str().ok_or("pdf path missing")?;
        let hash = sha256(pdf)?;
        let wrote = bytes_written(event)?;
        let inode = fs::metadata(pdf)?.ino();
        let pdf_len = fs::metadata(pdf)?.len();
        if let Some((previous_inode, _)) = seen.get(&hash) {
            ensure(
                inode == *previous_inode,
                format!("same pdf bytes were stored in a new file: {event}"),
            )?;
            ensure(
                wrote < pdf_len,
                format!("unchanged pdf was written again ({wrote} bytes, pdf is {pdf_len}): {event}"),
            )?;
        } else {
            ensure(
                wrote >= pdf_len,
                format!("first copy of a pdf wrote {wrote} of {pdf_len} bytes: {event}"),
            )?;
        }
        seen.insert(hash.clone(), (inode, pdf_len));
        if event["complete"] == true {
            let inode = fs::metadata(pdf)?.ino();
            kept_pdf = Some((hash, inode));
            if let Some(seq) = event["seq"].as_i64() {
                session.send(&json!({"op": "release", "seq": seq - 1}))?;
            }
        }
    }
    let (kept_hash, kept_inode) = kept_pdf.ok_or("generation 1 did not complete")?;
    let kept_pdf_path = first
        .iter()
        .rev()
        .find(|event| event["event"] == "published" && event["complete"] == true)
        .and_then(|event| event["pdf"].as_str())
        .ok_or("complete publication has no pdf")?
        .to_string();
    ensure(
        fs::metadata(&kept_pdf_path)?.ino() == kept_inode,
        "releasing the previous publication removed the kept pdf",
    )?;

    let same = DOC;
    session.send(&json!({
        "op": "update",
        "generation": 2,
        "files": [{"path": &main, "text": same}],
        "closed": []
    }))?;
    let identical = collect(&mut session, 2, Duration::from_secs(20))?;
    ensure(
        identical.iter().all(|event| event["event"] != "published"),
        format!("identical source published again: {identical:?}"),
    )?;
    let kept = kept_seq(&first)?;
    let idle = identical
        .iter()
        .any(|event| event["event"] == "idle" && event["seq"].as_i64() == Some(kept));
    ensure(idle, format!("identical source did not idle on the kept seq: {identical:?}"))?;

    let edited = DOC.replace("idle preview sample", "edited preview sample");
    session.send(&json!({
        "op": "update",
        "generation": 3,
        "files": [{"path": &main, "text": edited}],
        "closed": []
    }))?;
    let edit = collect(&mut session, 3, Duration::from_secs(60))?;
    let published = edit
        .iter()
        .rev()
        .find(|event| event["event"] == "published" && event["complete"] == true)
        .ok_or("edit did not publish")?;
    let wrote = bytes_written(published)?;
    ensure(wrote > 0, "a real edit published without writing")?;
    let pdf = published["pdf"].as_str().ok_or("pdf path missing")?;
    let edit_hash = sha256(pdf)?;
    ensure(edit_hash != kept_hash, "edit left the pdf bytes unchanged")?;
    let rendered = pdftotext(pdf)?;
    ensure(
        rendered.contains("edited preview sample"),
        format!("edited text missing from pdf: {rendered}"),
    )?;
    let edit_inode = fs::metadata(pdf)?.ino();

    let commented = format!("{edited}% trailing\n");
    session.send(&json!({
        "op": "update",
        "generation": 4,
        "files": [{"path": &main, "text": commented}],
        "closed": []
    }))?;
    let trailing = collect(&mut session, 4, Duration::from_secs(60))?;
    for event in &trailing {
        if event["event"] == "published" {
            let wrote = bytes_written(event)?;
            ensure(wrote == 0, format!("unchanged output wrote {wrote} bytes: {event}"))?;
            let path = event["pdf"].as_str().ok_or("pdf path missing")?;
            ensure(sha256(path)? == edit_hash, "unchanged publish replaced the pdf")?;
            ensure(
                fs::metadata(path)?.ino() == edit_inode,
                "unchanged publish did not hard-link the kept pdf",
            )?;
        }
    }
    ensure(
        trailing
            .iter()
            .any(|event| event["event"] == "idle" || event["bytes_written"] == 0),
        format!("trailing comment was not treated as unchanged: {trailing:?}"),
    )?;
    Ok(())
}

fn kept_seq(events: &[Value]) -> Result<i64> {
    events
        .iter()
        .rev()
        .find(|event| event["event"] == "published" && event["complete"] == true)
        .and_then(|event| event["seq"].as_i64())
        .ok_or_else(|| "missing kept seq".into())
}

fn pdftotext(pdf: &str) -> Result<String> {
    let result = preview_regression::output(Command::new("pdftotext").args([pdf, "-"]), None, true)?;
    ensure(result.status.success(), "pdftotext failed")?;
    Ok(String::from_utf8(result.stdout)?)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("preview-regress-publish-writes: {error}");
        std::process::exit(1);
    }
}
