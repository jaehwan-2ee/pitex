//! Windows editing preview: persistent protocol peer, isolated source
//! snapshots, cancellable full XeLaTeX passes, and source-mapped SyncTeX.
//! Unlike the Unix engine, this backend does not use fork checkpoints.
mod process_tree;
mod synctex;

use flate2::read::GzDecoder;
use process_tree::ProcessTree;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufRead, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime};

const MAX_PUBLICATIONS: usize = 4;
const PASS_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_PASSES: u8 = 3;

fn emit(value: Value) {
    let mut stdout = io::stdout().lock();
    let _ = serde_json::to_writer(&mut stdout, &value);
    let _ = stdout.write_all(b"\n");
    let _ = stdout.flush();
}

fn error(code: &str, message: impl std::fmt::Display) {
    emit(json!({"event":"error", "code":code, "message":message.to_string()}));
}

/// TeX accepts ordinary drive/UNC paths, not Rust's verbatim \\?\ paths.
fn tex_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        if let Some(drive) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(drive);
        }
    }
    path
}

fn canonical(path: &Path) -> io::Result<PathBuf> {
    path.canonicalize().map(tex_path)
}

struct Config {
    root: PathBuf,
    main: PathBuf,
    out: PathBuf,
    engine: OsString,
}

fn config() -> Result<Config, String> {
    let mut root = None;
    let mut main = None;
    let mut out = None;
    let mut engine = std::env::var_os("PITEX_PREVIEW_TEX_ENGINE");
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("Missing value for {}", arg.to_string_lossy()))?;
        match arg.to_str() {
            Some("--root") => root = Some(PathBuf::from(value)),
            Some("--main") => main = Some(PathBuf::from(value)),
            Some("--out") => out = Some(PathBuf::from(value)),
            Some("--engine") => engine = Some(value),
            Some("--cache") => {} // Formats belong to the installed TeX distribution.
            _ => return Err(format!("Unknown option: {}", arg.to_string_lossy())),
        }
    }
    let root = canonical(&root.ok_or("--root is required")?).map_err(|e| e.to_string())?;
    let supplied_main = main.ok_or("--main is required")?;
    let main_path = canonical(&root.join(&supplied_main)).map_err(|e| e.to_string())?;
    let main = main_path
        .strip_prefix(&root)
        .map_err(|_| "Main file must be inside the project")?
        .to_path_buf();
    if !main_path.is_file() {
        return Err("Main file is not a file".into());
    }
    let out = out.ok_or("--out is required")?;
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let out = canonical(&out).map_err(|e| e.to_string())?;
    if out == root || root.starts_with(&out) {
        return Err("Session output must not contain the project".into());
    }
    Ok(Config {
        root,
        main,
        out,
        engine: engine.unwrap_or_else(|| "xelatex".into()),
    })
}

/// Existing files resolve through symlinks; new buffers use a lexical path
/// inside the canonical project. Never let an override escape the snapshot.
fn relative(root: &Path, supplied: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(supplied);
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    let path = canonical(&path).unwrap_or_else(|_| tex_path(path));
    let rel = path
        .strip_prefix(root)
        .map_err(|_| "Override is outside the project".to_string())?;
    if rel.as_os_str().is_empty()
        || rel
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err("Override must name a project file without parent traversal".into());
    }
    Ok(rel.to_path_buf())
}

#[derive(Clone, PartialEq, Eq)]
enum Stamp {
    Disk(u64, Option<SystemTime>),
    Buffer(String),
}

struct Running {
    tree: ProcessTree,
    generation: u64,
    pass: u8,
    started: Instant,
    total_started: Instant,
}

struct PublicationStage(PathBuf);
impl Drop for PublicationStage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Session {
    cfg: Config,
    overrides: BTreeMap<PathBuf, String>,
    stamps: HashMap<PathBuf, Stamp>,
    generation: u64,
    pending: bool,
    running: Option<Running>,
    completed: Option<(u64, Instant)>,
    publications: BTreeMap<u64, PathBuf>,
    seq: u64,
    // A source/auxiliary change can invalidate a previous compiler output.
    // Outputs are reset for every new generation, retained only across its passes.
    work: PathBuf,
    build: PathBuf,
}

impl Session {
    fn new(cfg: Config) -> Self {
        Self {
            work: cfg.out.join("sources"),
            build: cfg.out.join("build"),
            cfg,
            overrides: BTreeMap::new(),
            stamps: HashMap::new(),
            generation: 0,
            pending: false,
            running: None,
            completed: None,
            publications: BTreeMap::new(),
            seq: 0,
        }
    }

    fn request(&mut self, request: Value) -> bool {
        match request["op"].as_str() {
            Some("quit") => return false,
            Some("release") => {
                if let Some(dir) = request["seq"]
                    .as_u64()
                    .and_then(|seq| self.publications.remove(&seq))
                {
                    let _ = fs::remove_dir_all(dir);
                }
            }
            Some("update") => {
                let Some(generation) = request["generation"]
                    .as_u64()
                    .filter(|g| *g > self.generation)
                else {
                    error("protocol", "Update generation must increase");
                    return true;
                };
                let result = (|| -> Result<_, String> {
                    let files = request["files"]
                        .as_array()
                        .ok_or("files must be an array")?;
                    let closed = request["closed"]
                        .as_array()
                        .ok_or("closed must be an array")?;
                    let mut replacements = Vec::new();
                    let mut removals = Vec::new();
                    for file in files {
                        let path = file["path"].as_str().ok_or("file.path must be a string")?;
                        let text = file["text"].as_str().ok_or("file.text must be a string")?;
                        replacements.push((relative(&self.cfg.root, path)?, text.to_owned()));
                    }
                    for path in closed {
                        removals.push(relative(
                            &self.cfg.root,
                            path.as_str().ok_or("closed path must be a string")?,
                        )?);
                    }
                    Ok((replacements, removals))
                })();
                let (replacements, removals) = match result {
                    Ok(delta) => delta,
                    Err(message) => {
                        error("protocol", message);
                        return true;
                    }
                };
                for path in removals {
                    self.overrides.remove(&path);
                }
                self.overrides.extend(replacements);
                self.generation = generation;
                self.pending = true;
                // Dropping the tree kills the compiler and every child before
                // its output directory can be reset or sources can be changed.
                self.running = None;
                self.completed = None;
            }
            _ => error("protocol", "Unknown request operation"),
        }
        true
    }

    fn snapshot_directory(
        &mut self,
        directory: &Path,
        seen: &mut HashSet<PathBuf>,
    ) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(&self.cfg.root).unwrap().to_path_buf();
            let name = entry.file_name();
            if matches!(
                name.to_str(),
                Some(
                    ".git"
                        | ".hg"
                        | ".svn"
                        | ".pitex-live"
                        | ".pitex-preview"
                        | "node_modules"
                        | ".venv"
                        | "venv"
                        | "__pycache__"
                )
            ) || path.starts_with(&self.cfg.out)
            {
                continue;
            }
            let kind = entry.file_type()?;
            // Do not traverse directory symlinks/junctions (cycles and large
            // unrelated trees). File symlinks are materialized as normal files.
            if kind.is_dir() && !kind.is_symlink() {
                fs::create_dir_all(self.work.join(&relative))?;
                self.snapshot_directory(&path, seen)?;
            } else if path.is_file() {
                seen.insert(relative.clone());
                let canonical_relative = canonical(&path).ok().and_then(|path| {
                    path.strip_prefix(&self.cfg.root)
                        .ok()
                        .map(Path::to_path_buf)
                });
                let override_text = self.overrides.get(&relative).or_else(|| {
                    canonical_relative
                        .as_ref()
                        .and_then(|path| self.overrides.get(path))
                });
                if let Some(text) = override_text {
                    // A file symlink is materialized in the snapshot. Every
                    // alias must use the override for its canonical target.
                    let stamp = Stamp::Buffer(text.clone());
                    if self.stamps.get(&relative) != Some(&stamp) {
                        let target = self.work.join(&relative);
                        if let Some(parent) = target.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::write(target, text)?;
                        self.stamps.insert(relative, stamp);
                    }
                    continue;
                }
                let meta = fs::metadata(&path)?;
                let stamp = Stamp::Disk(meta.len(), meta.modified().ok());
                if self.stamps.get(&relative) != Some(&stamp) {
                    let target = self.work.join(&relative);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(&path, &target)?;
                    // A read-only source must not make closing an override fail.
                    #[cfg(windows)]
                    {
                        let mut permissions = fs::metadata(&target)?.permissions();
                        permissions.set_readonly(false);
                        fs::set_permissions(&target, permissions)?;
                    }
                    self.stamps.insert(relative, stamp);
                }
            }
        }
        Ok(())
    }

    fn snapshot(&mut self) -> io::Result<()> {
        fs::create_dir_all(&self.work)?;
        let mut seen = HashSet::new();
        self.snapshot_directory(&self.cfg.root.clone(), &mut seen)?;
        for (path, text) in &self.overrides {
            seen.insert(path.clone());
            let stamp = Stamp::Buffer(text.clone());
            if self.stamps.get(path) != Some(&stamp) {
                let target = self.work.join(path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(target, text)?;
                self.stamps.insert(path.clone(), stamp);
            }
        }
        let deleted: Vec<_> = self
            .stamps
            .keys()
            .filter(|p| !seen.contains(*p))
            .cloned()
            .collect();
        for path in deleted {
            match fs::remove_file(self.work.join(&path)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            self.stamps.remove(&path);
        }
        match fs::remove_dir_all(&self.build) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::create_dir_all(&self.build)
    }

    fn start_pass(&mut self, pass: u8, total_started: Instant) -> io::Result<()> {
        let capture = File::create(self.build.join("compiler-output.log"))?;
        let mut command = Command::new(&self.cfg.engine);
        command
            .current_dir(&self.work)
            .arg("-interaction=nonstopmode")
            .arg("-no-shell-escape")
            .arg("-synctex=-1")
            .arg(format!(
                "-output-directory={}",
                self.build.to_string_lossy()
            ))
            .arg(&self.cfg.main)
            .stdin(Stdio::null())
            .stdout(Stdio::from(capture.try_clone()?))
            .stderr(Stdio::from(capture));
        // Use the source snapshot first; distribution lookups retain their
        // default path via the trailing separator. No original source is
        // overwritten, even if an unsaved include is compiled.
        let separator = if cfg!(windows) { ";" } else { ":" };
        let source_search = self.work.to_string_lossy().replace('\\', "/");
        command.env(
            "TEXINPUTS",
            format!(
                "{source_search}//{separator}{}{separator}",
                std::env::var("TEXINPUTS").unwrap_or_default()
            ),
        );
        self.running = Some(Running {
            tree: ProcessTree::spawn(&mut command)?,
            generation: self.generation,
            pass,
            started: Instant::now(),
            total_started,
        });
        Ok(())
    }

    fn artifact(&self, extension: &str) -> PathBuf {
        let mut name = self.cfg.main.file_stem().unwrap().to_os_string();
        name.push(".");
        name.push(extension);
        self.build.join(name)
    }

    fn log(&self) -> String {
        fs::read_to_string(self.artifact("log"))
            .or_else(|_| fs::read_to_string(self.build.join("compiler-output.log")))
            .unwrap_or_default()
    }

    fn failed(&self, generation: u64, code: &str, message: impl std::fmt::Display) {
        let log = self.log();
        let tail: String = log
            .chars()
            .rev()
            .take(4000)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        emit(
            json!({"event":"failed", "generation":generation, "code":code,
            "message":message.to_string(), "log_tail":tail}),
        );
    }

    fn tick(&mut self) {
        if self.pending && self.publications.len() < MAX_PUBLICATIONS {
            self.pending = false;
            if let Err(e) = self.snapshot() {
                self.failed(self.generation, "io", e);
                return;
            }
            if let Err(e) = self.start_pass(1, Instant::now()) {
                if e.kind() == io::ErrorKind::NotFound {
                    error("no_tex", format!("XeLaTeX is unavailable: {e}. Install TeX Live or MiKTeX and make xelatex available."));
                } else {
                    self.failed(self.generation, "io", e);
                }
            }
        }
        let Some(run) = self.running.as_mut() else {
            if self.publications.len() < MAX_PUBLICATIONS {
                if let Some((generation, started)) = self.completed.take() {
                    if generation == self.generation {
                        if let Err(e) = self.publish(generation, started) {
                            self.failed(generation, "io", e);
                        }
                    }
                }
            }
            return;
        };
        if run.started.elapsed() > PASS_TIMEOUT {
            let generation = run.generation;
            self.running = None;
            self.failed(
                generation,
                "stuck",
                "Preview compilation exceeded 60 seconds",
            );
            return;
        }
        match run.tree.child.try_wait() {
            Ok(Some(status)) => {
                let run = self.running.take().unwrap();
                let log = self.log();
                // Nonstop TeX can finish a valid document with recoverable
                // errors (exit 1). Publish that PDF with error status, as the
                // Unix backend does. Crashes/fatal aborts keep the last PDF.
                if !status.success() && !log.contains("Output written on") {
                    let message = log
                        .lines()
                        .find(|l| l.starts_with('!'))
                        .unwrap_or("Preview compiler stopped without producing a document");
                    self.failed(run.generation, "crashed", message);
                    return;
                }
                let rerun = status.success()
                    && run.pass < MAX_PASSES
                    && (log.contains("Rerun to get")
                        || log.contains("Please rerun")
                        || log.contains("Label(s) may have changed")
                        || log.contains("rerunfilecheck Warning"));
                if rerun {
                    if let Err(e) = self.start_pass(run.pass + 1, run.total_started) {
                        self.failed(run.generation, "io", e);
                    }
                } else {
                    self.completed = Some((run.generation, run.total_started));
                }
            }
            Ok(None) => {}
            Err(e) => {
                let generation = run.generation;
                self.running = None;
                self.failed(generation, "crashed", e);
            }
        }
    }

    fn publish(&mut self, generation: u64, started: Instant) -> io::Result<()> {
        let bytes = fs::read(self.artifact("pdf"))?;
        if !bytes.starts_with(b"%PDF") {
            self.failed(
                generation,
                "no_pages",
                "Preview compiler did not produce a valid PDF",
            );
            return Ok(());
        }
        self.seq += 1;
        let temp = self.cfg.out.join(format!(".p{}", self.seq));
        let dir = self.cfg.out.join(format!("p{}", self.seq));
        fs::create_dir(&temp)?;
        let _cleanup = PublicationStage(temp.clone());
        fs::write(temp.join("main.pdf"), &bytes)?;
        let log = self.log();
        fs::write(temp.join("main.log"), &log)?;
        let sync = self.synctex()?;
        if let Some(sync) = sync.as_ref() {
            fs::write(temp.join("main.synctex"), sync)?;
        }
        fs::rename(&temp, &dir)?;
        self.publications.insert(self.seq, dir.clone());
        let errors: Vec<_> = log.lines().filter(|l| l.starts_with('!')).collect();
        let pages = page_count(&bytes, &log);
        emit(
            json!({"event":"published", "seq":self.seq, "generation":generation,
            "complete":true, "coherent":true, "pages":pages, "current_pages":pages,
            "errors":errors.len(), "first_error":errors.first().map(|line| line.trim_start_matches("! ")),
            "elapsed_ms":started.elapsed().as_millis() as u64, "dir":dir,
            "pdf":dir.join("main.pdf"), "log":dir.join("main.log"),
            "synctex":sync.map(|_| dir.join("main.synctex"))}),
        );
        Ok(())
    }

    fn synctex(&self) -> io::Result<Option<String>> {
        let bytes = match fs::read(self.artifact("synctex")) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let compressed = self.artifact("synctex.gz");
                if !compressed.exists() {
                    return Ok(None);
                }
                let mut bytes = Vec::new();
                GzDecoder::new(File::open(compressed)?).read_to_end(&mut bytes)?;
                bytes
            }
            Err(e) => return Err(e),
        };
        let source = synctex::decode(bytes)?;
        Ok(synctex::rewrite(
            &source,
            &self.work,
            &self.cfg.root,
            self.stamps.keys().map(PathBuf::as_path),
            synctex::legacy_path,
        ))
    }
}

fn page_count(pdf: &[u8], log: &str) -> usize {
    for line in log.lines().rev() {
        if line.starts_with("Output written on") {
            if let Some(pages) = line
                .rsplit('(')
                .next()
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse().ok())
            {
                return pages;
            }
        }
    }
    // XeTeX normally reports the count in the log; this also supports small
    // fixture PDFs and producers that leave page dictionaries uncompressed.
    pdf.windows(12)
        .filter(|w| w.starts_with(b"/Type /Page") && w[11] != b's')
        .count()
        .max(1)
}

fn requests() -> Receiver<Option<Result<Value, serde_json::Error>>> {
    let (sender, receiver) = mpsc::sync_channel(4);
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if sender.send(Some(serde_json::from_str(&line))).is_err() {
                return;
            }
        }
        let _ = sender.send(None);
    });
    receiver
}

fn main() {
    let cfg = match config() {
        Ok(cfg) => cfg,
        Err(message) => {
            error("usage", message);
            std::process::exit(2);
        }
    };
    let mut session = Session::new(cfg);
    let receiver = requests();
    emit(json!({"event":"ready", "backend":"windows-snapshot"}));
    loop {
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(None) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(Some(request)) => {
                let mut batch = vec![request];
                let mut eof = false;
                while let Ok(next) = receiver.try_recv() {
                    match next {
                        Some(request) => batch.push(request),
                        None => {
                            eof = true;
                            break;
                        }
                    }
                }
                for request in batch {
                    match request {
                        Ok(value) => {
                            if !session.request(value) {
                                return;
                            }
                        }
                        Err(e) => error("protocol", e),
                    }
                }
                if eof {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        session.tick();
    }
}
