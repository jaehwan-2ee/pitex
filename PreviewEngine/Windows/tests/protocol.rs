use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};

// Exercise the same query preparation/restoration used by the Windows app;
// the helper crate stays independent of GTK and application dev dependencies.
#[path = "../../../Linux/crates/synctex-core/src/cli_workspace.rs"]
mod cli_workspace;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pitex Windows 미리보기 {} {nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// Native subprocess fixture: no shell quoting/PowerShell/MSYS dependency.
const COMPILER: &str = r#"
use std::{env, fs, path::PathBuf, process::Command, thread, time::Duration};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--descendant") {
        loop { thread::sleep(Duration::from_secs(1)); }
    }
    let out = PathBuf::from(args.iter().find_map(|a| a.strip_prefix("-output-directory=")).unwrap());
    let main = PathBuf::from(args.last().unwrap());
    let stem = main.file_stem().unwrap();
    let log = out.join(stem).with_extension("log");
    let text = fs::read_to_string(&main).unwrap_or_default();
    if text.contains("LOOP") {
        let child = Command::new(env::current_exe().unwrap()).arg("--descendant").spawn().unwrap();
        fs::write(out.join("descendant.pid"), child.id().to_string()).unwrap();
        fs::write(out.join("compiler.pid"), std::process::id().to_string()).unwrap();
        loop { thread::sleep(Duration::from_secs(1)); }
    }
    let chapter = fs::read_to_string("chapter.tex");
    if text.contains("FAIL") || (text.contains("INCLUDE") && chapter.is_err()) {
        fs::write(log, "! Fixture compilation failed.\nNo pages of output.\n").unwrap();
        std::process::exit(1);
    }
    let body = format!("{} {}", text, chapter.unwrap_or_default());
    fs::write(out.join(stem).with_extension("pdf"), format!("%PDF-1.4\n/Type /Page\n{body}\n%%EOF\n")).unwrap();
    let cwd = env::current_dir().unwrap();
    fs::write(out.join(stem).with_extension("synctex"), format!("SyncTeX Version:1\nInput:1:{}\nInput:2:{}\nContent:\n", cwd.join(main).display(), cwd.join("chapter.tex").display())).unwrap();
    let recoverable = if text.contains("ERROR_PDF") { "! Undefined control sequence.\n" } else { "" };
    fs::write(log, format!("{recoverable}Output written on main.pdf (1 page).\n")).unwrap();
    if text.contains("ERROR_PDF") { std::process::exit(1); }
}
"#;

fn compiler() -> &'static Path {
    static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        // MinGW's linker cannot create its output under a Unicode path.
        // Compile in ASCII, then run the copied executable from Unicode to
        // keep native process/path coverage without testing linker behavior.
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let build_directory = Temp(std::env::temp_dir().join(format!(
            "pitex-preview-compiler-{}-{nonce}",
            std::process::id()
        )));
        fs::create_dir_all(&build_directory.0).unwrap();
        let directory = Temp::new();
        let name = if cfg!(windows) {
            "compiler.exe"
        } else {
            "compiler"
        };
        let source = build_directory.0.join("compiler.rs");
        let compiled = build_directory.0.join(name);
        let executable = directory.0.join(name);
        fs::write(&source, COMPILER).unwrap();
        assert!(Command::new("rustc")
            .arg(&source)
            .args(["--edition=2021", "-o"])
            .arg(&compiled)
            .status()
            .unwrap()
            .success());
        fs::copy(&compiled, &executable).unwrap();
        // One tiny fixture is shared across tests for this test process.
        std::mem::forget(directory);
        executable
    })
}

struct Peer {
    child: Child,
    events: Receiver<Value>,
    out: PathBuf,
}
impl Peer {
    fn new(root: &Path, out: &Path, engine: &Path) -> Self {
        fs::create_dir_all(out).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_pitex-preview"))
            .arg("--root")
            .arg(root)
            .args(["--main", "main.tex", "--out"])
            .arg(out)
            .arg("--engine")
            .arg(engine)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, events) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if sender.send(serde_json::from_str(&line).unwrap()).is_err() {
                    break;
                }
            }
        });
        let mut peer = Self {
            child,
            events,
            out: out.to_path_buf(),
        };
        assert_eq!(peer.event()["event"], "ready");
        peer
    }
    fn send(&mut self, request: Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        serde_json::to_writer(&mut *stdin, &request).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }
    fn update(&mut self, generation: u64, files: Value, closed: Value) {
        self.send(json!({"op":"update", "generation":generation, "files":files, "closed":closed}));
    }
    fn event(&mut self) -> Value {
        self.events
            .recv_timeout(Duration::from_secs(30))
            .expect("helper event")
    }
    fn published(&mut self, generation: u64) -> Value {
        let value = self.event();
        assert_eq!(value["event"], "published", "{value}");
        assert_eq!(value["generation"], generation);
        assert_eq!(value["complete"], true);
        assert_eq!(value["coherent"], true);
        value
    }
    fn release(&mut self, value: &Value) {
        self.send(json!({"op":"release", "seq":value["seq"]}));
    }
    fn quit(&mut self) {
        self.send(json!({"op":"quit"}));
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "helper exits on quit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn sources(temp: &Temp) -> PathBuf {
    let root = temp.0.join("프로젝트 sources");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.tex"), "INCLUDE original source").unwrap();
    fs::write(root.join("chapter.tex"), "disk chapter").unwrap();
    root
}

fn body(event: &Value) -> String {
    fs::read_to_string(event["pdf"].as_str().unwrap()).unwrap()
}

#[test]
fn unsaved_includes_sync_mapping_close_disk_changes_failure_and_recovery() {
    let temp = Temp::new();
    let root = sources(&temp);
    let main = root.join("main.tex");
    let chapter = root.join("chapter.tex");
    let mut peer = Peer::new(&root, &temp.0.join("preview"), compiler());
    peer.update(
        1,
        json!([{ "path":chapter, "text":"unsaved chapter 한글" }]),
        json!([]),
    );
    let first = peer.published(1);
    assert!(body(&first).contains("unsaved chapter 한글"));
    assert_eq!(fs::read_to_string(&chapter).unwrap(), "disk chapter");
    let sync = fs::read_to_string(first["synctex"].as_str().unwrap()).unwrap();
    assert!(sync.contains(&root.to_string_lossy().replace('\\', "/")));
    assert!(!sync.contains("/preview/sources/"));
    peer.release(&first);

    peer.update(2, json!([]), json!([chapter]));
    let second = peer.published(2);
    assert!(body(&second).contains("disk chapter"));
    peer.release(&second);

    fs::write(&chapter, "disk changed to longer text").unwrap();
    peer.update(3, json!([]), json!([]));
    let third = peer.published(3);
    assert!(body(&third).contains("disk changed to longer text"));
    // The newest good PDF survives a failed later generation.
    fs::remove_file(&chapter).unwrap();
    peer.update(4, json!([]), json!([]));
    let failed = peer.event();
    assert_eq!(failed["event"], "failed");
    assert_eq!(failed["generation"], 4);
    assert!(Path::new(third["pdf"].as_str().unwrap()).exists());

    fs::write(&chapter, "restored chapter").unwrap();
    peer.update(5, json!([]), json!([]));
    let recovered = peer.published(5);
    assert!(body(&recovered).contains("restored chapter"));
    peer.release(&third);
    peer.release(&recovered);

    peer.update(6, json!([{ "path":main, "text":"ERROR_PDF" }]), json!([]));
    let errors = peer.published(6);
    assert_eq!(errors["errors"], 1);
    assert!(errors["first_error"]
        .as_str()
        .unwrap()
        .contains("Undefined control"));
    peer.release(&errors);
    assert_eq!(
        fs::read_to_string(&main).unwrap(),
        "INCLUDE original source"
    );
    peer.quit();
}

#[test]
fn publications_are_bounded_and_latest_update_wins_when_released() {
    let temp = Temp::new();
    let root = sources(&temp);
    let main = root.join("main.tex");
    let mut peer = Peer::new(&root, &temp.0.join("preview"), compiler());
    let mut held = Vec::new();
    for generation in 1..=4 {
        peer.update(
            generation,
            json!([{ "path":main, "text":format!("generation {generation}") }]),
            json!([]),
        );
        held.push(peer.published(generation));
    }
    peer.update(
        5,
        json!([{ "path":main, "text":"obsolete fifth" }]),
        json!([]),
    );
    peer.update(
        6,
        json!([{ "path":main, "text":"newest sixth" }]),
        json!([]),
    );
    assert!(peer
        .events
        .recv_timeout(Duration::from_millis(200))
        .is_err());
    let dirs = fs::read_dir(&peer.out)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with('p'))
        .count();
    assert_eq!(dirs, 4);
    peer.release(&held.remove(0));
    let newest = peer.published(6);
    assert!(body(&newest).contains("newest sixth"));
    peer.quit();
}

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < deadline, "file {} appears", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        };
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return false;
        }
        let mut code = 0;
        let result = unsafe { GetExitCodeProcess(handle, &mut code) } != 0 && code == 259;
        unsafe { CloseHandle(handle) };
        result
    }
    #[cfg(unix)]
    {
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        // A zombie has already stopped; it may await the container's PID 1.
        let zombie = fs::read_to_string(format!("/proc/{pid}/stat"))
            .map(|s| s.contains(") Z "))
            .unwrap_or(false);
        !zombie && unsafe { kill(pid as i32, 0) } == 0
    }
}

fn assert_stopped(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(pid) {
        assert!(Instant::now() < deadline, "process {pid} was stopped");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn cancellation_discards_stale_output_and_shutdown_stops_descendants() {
    let temp = Temp::new();
    let root = sources(&temp);
    let main = root.join("main.tex");
    let out = temp.0.join("preview");
    let mut peer = Peer::new(&root, &out, compiler());
    peer.update(1, json!([{ "path":main, "text":"LOOP" }]), json!([]));
    let descendant = out.join("build/descendant.pid");
    wait_for_file(&descendant);
    let pid: u32 = fs::read_to_string(&descendant).unwrap().parse().unwrap();
    peer.update(
        2,
        json!([{ "path":main, "text":"newest after loop" }]),
        json!([]),
    );
    let recovered = peer.published(2);
    assert!(body(&recovered).contains("newest after loop"));
    assert_stopped(pid);
    peer.release(&recovered);
    peer.update(3, json!([{ "path":main, "text":"LOOP" }]), json!([]));
    wait_for_file(&descendant);
    let pid: u32 = fs::read_to_string(&descendant).unwrap().parse().unwrap();
    peer.quit();
    assert_stopped(pid);
}

#[cfg(windows)]
#[test]
fn helper_crash_closes_job_and_stops_the_compiler_tree() {
    let temp = Temp::new();
    let root = sources(&temp);
    let out = temp.0.join("preview");
    let mut peer = Peer::new(&root, &out, compiler());
    peer.update(
        1,
        json!([{ "path":root.join("main.tex"), "text":"LOOP" }]),
        json!([]),
    );
    let parent = out.join("build/compiler.pid");
    wait_for_file(&parent);
    let parent: u32 = fs::read_to_string(parent).unwrap().parse().unwrap();
    let descendant: u32 = fs::read_to_string(out.join("build/descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    peer.child.kill().unwrap();
    peer.child.wait().unwrap();
    assert_stopped(parent);
    assert_stopped(descendant);
}

#[test]
fn invalid_override_cannot_modify_outside_project_and_can_recover() {
    let temp = Temp::new();
    let root = sources(&temp);
    let outside = temp.0.join("outside.tex");
    fs::write(&outside, "protected").unwrap();
    let mut peer = Peer::new(&root, &temp.0.join("preview"), compiler());
    peer.update(
        1,
        json!([{ "path":outside, "text":"overwrite" }]),
        json!([]),
    );
    assert_eq!(peer.event()["event"], "error");
    assert_eq!(fs::read_to_string(outside).unwrap(), "protected");
    peer.update(2, json!([]), json!([]));
    peer.published(2);
    peer.quit();
}

#[cfg(unix)]
#[test]
fn file_symlink_alias_uses_canonical_buffer_override_and_closes_to_disk() {
    let temp = Temp::new();
    let root = sources(&temp);
    let target = root.join("canonical.tex");
    let alias = root.join("chapter.tex");
    fs::rename(&alias, &target).unwrap();
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    let mut peer = Peer::new(&root, &temp.0.join("preview"), compiler());
    peer.update(
        1,
        json!([{ "path":alias, "text":"symlink draft" }]),
        json!([]),
    );
    let first = peer.published(1);
    assert!(body(&first).contains("symlink draft"));
    assert_eq!(fs::read_to_string(&target).unwrap(), "disk chapter");
    peer.release(&first);
    peer.update(2, json!([]), json!([alias]));
    assert!(body(&peer.published(2)).contains("disk chapter"));
    peer.quit();
}

#[test]
#[ignore = "requires an installed XeLaTeX distribution"]
fn real_tex() {
    let temp = Temp::new();
    let root = sources(&temp);
    fs::write(root.join("main.tex"), "\\documentclass{article}\n\\begin{document}\n\\input{chapter}\nSee section~\\ref{draft}.\n\\section{Draft}\\label{draft}\n\\end{document}\n").unwrap();
    fs::write(root.join("chapter.tex"), "Saved chapter text.\n").unwrap();
    let mut peer = Peer::new(&root, &temp.0.join("preview"), Path::new("xelatex"));
    peer.update(
        1,
        json!([{ "path":root.join("chapter.tex"), "text":"Unsaved chapter preview.\n" }]),
        json!([]),
    );
    let first = peer.published(1);
    assert_eq!(first["errors"], 0, "{first}");
    let bytes = fs::read(first["pdf"].as_str().unwrap()).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    let sync = fs::read_to_string(first["synctex"].as_str().unwrap()).unwrap();
    assert!(
        sync.contains(&root.to_string_lossy().replace('\\', "/")),
        "{sync}"
    );
    assert!(sync.contains("chapter.tex"));
    assert_eq!(
        fs::read_to_string(root.join("chapter.tex")).unwrap(),
        "Saved chapter text.\n"
    );
    assert!(!root.join("main.aux").exists());
    verify_native_synctex(&root, &first);
    peer.release(&first);
    peer.update(2, json!([]), json!([root.join("chapter.tex")]));
    let second = peer.published(2);
    assert_eq!(second["errors"], 0);
    peer.quit();
}

/// Query the published bytes through the installed native CLI. Metadata
/// string replacement alone does not prove forward/inverse navigation.
fn verify_native_synctex(root: &Path, publication: &Value) {
    let chapter = root.join("chapter.tex");
    let pdf = Path::new(publication["pdf"].as_str().unwrap());
    // A Korean Windows username can put TEMP itself under Unicode. Query
    // filenames are ASCII, but the real native tool must also tolerate a
    // Unicode current directory, using the production staging implementation.
    let query_parent = root.parent().unwrap().join("SyncTeX query 작업");
    fs::create_dir_all(&query_parent).unwrap();
    let workspace = cli_workspace::CliWorkspace::prepare_in(pdf, root, &query_parent)
        .expect("production native query workspace must prepare the original Unicode PDF");
    let input = workspace
        .input_alias(&chapter.to_string_lossy())
        .expect("production native query workspace must resolve the original Unicode source");
    let forward = Command::new("synctex")
        .args(["view", "-i"])
        .arg(format!("1:1:{input}"))
        .arg("-o")
        .arg(cli_workspace::PDF_FILE)
        .current_dir(workspace.directory())
        .output()
        .expect("native synctex tool must be installed for real_tex acceptance");
    let forward_text = workspace
        .restore(&String::from_utf8_lossy(&forward.stdout))
        .expect("production native query adapter must restore original PDF/source identities");
    assert!(
        forward.status.success(),
        "native synctex view failed for Unicode source/PDF paths:\n{forward_text}\n{}",
        String::from_utf8_lossy(&forward.stderr)
    );
    let field = |text: &str, name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}:")))
            .map(|value| value.trim().to_owned())
    };
    let page = field(&forward_text, "Page").unwrap_or_else(|| {
        panic!(
            "forward query must return a page for {} in {}:\n{forward_text}\n{}",
            chapter.display(),
            pdf.display(),
            String::from_utf8_lossy(&forward.stderr)
        )
    });
    let x = field(&forward_text, "x").expect("forward query must return x");
    let y = field(&forward_text, "y").expect("forward query must return y");
    let output =
        field(&forward_text, "Output").expect("forward query must identify the original PDF");
    assert_eq!(
        Path::new(&output).canonicalize().unwrap(),
        pdf.canonicalize().unwrap(),
        "forward query must retain the original Unicode publication PDF: {forward_text}"
    );
    assert_eq!(page.parse::<usize>().unwrap(), 1, "{forward_text}");
    assert!(x.parse::<f64>().unwrap().is_finite(), "{forward_text}");
    assert!(y.parse::<f64>().unwrap().is_finite(), "{forward_text}");

    let inverse = Command::new("synctex")
        .args(["edit", "-o"])
        .arg(format!("{page}:{x}:{y}:{}", cli_workspace::PDF_FILE))
        .current_dir(workspace.directory())
        .output()
        .expect("native synctex inverse query");
    let inverse_text = workspace
        .restore(&String::from_utf8_lossy(&inverse.stdout))
        .expect("production native query adapter must restore inverse source identity");
    assert!(
        inverse.status.success(),
        "native synctex edit failed:\n{inverse_text}\n{}",
        String::from_utf8_lossy(&inverse.stderr)
    );
    let input =
        field(&inverse_text, "Input").expect("inverse query must return the original source");
    assert_eq!(
        Path::new(&input)
            .canonicalize()
            .unwrap_or_else(|error| panic!(
                "inverse source {input:?} must exist: {error}; {inverse_text}"
            )),
        chapter.canonicalize().unwrap(),
        "inverse query must identify the original Unicode source: {inverse_text}"
    );
    assert_eq!(
        field(&inverse_text, "Line").as_deref(),
        Some("1"),
        "{inverse_text}"
    );
}
