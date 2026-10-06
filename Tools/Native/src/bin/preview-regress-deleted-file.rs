//! A deleted editor override must report a missing file, not typeset stale bytes.
use pitex_native_tools::{
    preview_regression::{self, Session},
    repo_root, TempDir,
};
use serde_json::{json, Value};
use std::{
    env,
    error::Error,
    fs,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

fn wait(
    session: &mut Session,
    timeout: u64,
    predicate: impl Fn(&Value) -> bool,
) -> Result<Option<Value>, Box<dyn Error>> {
    let end = Instant::now() + Duration::from_secs(timeout);
    while let Some(remaining) = end.checked_duration_since(Instant::now()) {
        match session.next_event_or_eof(remaining)? {
            Some(event) => {
                event
                    .get("event")
                    .ok_or("missing helper event key 'event'")?;
                if predicate(&event) {
                    return Ok(Some(event));
                }
            }
            None => break,
        }
    }
    Ok(None)
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64() != Some(0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::check_interrupt()?;
    let bin = match env::var_os("PITEX_BIN") {
        Some(bin) => PathBuf::from(bin),
        None => repo_root()?.join("PreviewEngine/out/bin"),
    };
    let work = TempDir::new("pitex-b1low-")?;
    let root = work.path().join("proj");
    let out = work.path().join("out");
    fs::create_dir_all(root.join("part"))?;
    fs::create_dir_all(&out)?;
    let main = root.join("main.tex");
    let chap = root.join("part/chap.tex");
    fs::write(&main, "\\documentclass{article}\\begin{document}\nTop.\\input{part/chap}\nEnd.\n\\end{document}\n")?;
    fs::write(&chap, "Chapter text.\n")?;

    let mut command = Command::new(bin.join("pitex-preview"));
    command
        .args(["--root"])
        .arg(&root)
        .args(["--main", "main.tex", "--out"])
        .arg(&out);
    if env::var_os("PITEX_PREVIEW_CACHE").is_none() {
        command.env("PITEX_PREVIEW_CACHE", work.path().join("cache"));
    }
    let mut session = Session::spawn(&mut command, &out)?;
    wait(&mut session, 15, |event| event["event"] == "ready")?.ok_or("no ready")?;
    session.send(&json!({
        "op": "update", "generation": 1,
        "files": [
            {"path": main, "text": fs::read_to_string(&main)?},
            {"path": chap, "text": fs::read_to_string(&chap)?}
        ], "closed": []
    }))?;
    wait(&mut session, 90, |event| {
        event["event"] == "published" && truthy(&event["complete"])
    })?
    .ok_or("baseline publication failed")?;

    // The editor override lands before the file is deleted and then closed.
    session.send(&json!({
        "op": "update", "generation": 2,
        "files": [{"path": chap, "text": "Chapter text.\n"}], "closed": []
    }))?;
    wait(&mut session, 30, |event| event["event"] == "idle")?;
    fs::remove_file(&chap)?;
    session.send(&json!({"op": "update", "generation": 3, "files": [], "closed": [chap]}))?;
    let event = wait(&mut session, 90, |event| {
        event["event"] == "published" || event["event"] == "failed"
    })?
    .ok_or("no event after delete+close")?;
    let stale = if event["event"] == "published" && truthy(&event["complete"]) {
        let errors = event
            .get("errors")
            .ok_or("missing helper event key 'errors'")?;
        errors.as_f64() == Some(0.0) || errors == false
    } else {
        false
    };
    if stale {
        return Err("deleted file still typeset clean from stale cached bytes".into());
    }
    if !event.to_string().to_lowercase().contains("not found") {
        return Err(format!("no missing-file signal: {event}").into());
    }
    let message = if truthy(&event["message"]) {
        &event["message"]
    } else {
        &event["code"]
    };
    let message = match message {
        Value::String(message) => message.clone(),
        Value::Null => "None".to_string(),
        other => other.to_string(),
    };
    println!("PASS: deletion surfaces file-not-found: {message}");
    preview_regression::check_interrupt()?;
    Ok(())
}

fn main() {
    preview_regression::watch_interrupt();
    if let Err(error) = run() {
        eprintln!("Error: {error:?}");
        std::process::exit(if preview_regression::interrupted(error.as_ref()) {
            130
        } else {
            1
        });
    }
}
