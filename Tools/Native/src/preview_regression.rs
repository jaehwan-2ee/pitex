//! Shared process I/O for the real embedded-engine regressions.
use serde_json::Value;
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub use crate::{check_interrupt, interrupted, watch_interrupt};

fn own_process_group(command: &mut Command) {
    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

fn kill_group(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn log_tail(out: &Path, count: usize) -> String {
    let bytes = std::fs::read(out.join("driver.log")).unwrap_or_default();
    String::from_utf8_lossy(&bytes)
        .chars()
        .rev()
        .take(count)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

pub struct Session {
    pub child: Child,
    pub out: PathBuf,
    events: Receiver<std::io::Result<Vec<u8>>>,
    displayed: Option<Value>,
}

impl Session {
    pub fn spawn(command: &mut Command, out: &Path) -> Result<Self> {
        watch_interrupt();
        own_process_group(command);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().ok_or("no helper stdout")?;
        let (sender, events) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                match reader.read_until(b'\n', &mut bytes) {
                    Ok(0) => break,
                    Ok(_) => {
                        if bytes.last() != Some(&b'\n') {
                            break;
                        }
                        bytes.pop();
                        if sender.send(Ok(bytes)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            out: out.to_path_buf(),
            events,
            displayed: None,
        })
    }

    pub fn send(&mut self, request: &Value) -> Result<()> {
        let stdin = self.child.stdin.as_mut().ok_or("no helper stdin")?;
        serde_json::to_writer(&mut *stdin, request)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn next_event(&mut self, timeout: Duration) -> Result<Option<Value>> {
        let deadline = Instant::now() + timeout;
        loop {
            check_interrupt()?;
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Ok(None);
            };
            match self
                .events
                .recv_timeout(remaining.min(Duration::from_millis(100)))
            {
                Ok(bytes) => return Ok(Some(serde_json::from_slice(&bytes?)?)),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!(
                        "helper exited: {:?}: {}",
                        self.child.try_wait()?,
                        log_tail(&self.out, 3000)
                    )
                    .into())
                }
            }
        }
    }

    /// The deleted-file probe treats an exited helper as silence until its deadline.
    pub fn next_event_or_eof(&mut self, timeout: Duration) -> Result<Option<Value>> {
        let deadline = Instant::now() + timeout;
        loop {
            check_interrupt()?;
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Ok(None);
            };
            match self
                .events
                .recv_timeout(remaining.min(Duration::from_millis(100)))
            {
                Ok(bytes) => return Ok(Some(serde_json::from_slice(&bytes?)?)),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    thread::sleep(remaining.min(Duration::from_millis(100)));
                }
            }
        }
    }

    pub fn wait_published(&mut self, generation: u64, timeout: Duration) -> Result<Value> {
        let deadline = Instant::now() + timeout;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            let Some(event) = self.next_event(remaining.min(Duration::from_secs(1)))? else {
                continue;
            };
            if matches!(event["event"].as_str(), Some("failed" | "error")) {
                return Err(event.to_string().into());
            }
            if event["event"] == "published" {
                if let Some(seq) = self.displayed.take() {
                    self.send(&serde_json::json!({"op": "release", "seq": seq}))?;
                }
                self.displayed = Some(event["seq"].clone());
                if event["complete"] == true && event["generation"] == generation {
                    return Ok(event);
                }
            }
        }
        Err(format!(
            "no complete publication for generation {generation}: {}",
            log_tail(&self.out, 2000)
        )
        .into())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        kill_group(&mut self.child);
    }
}

/// SHA-256 of the helper pair in `bin`, in the order the vocabulary audit
/// records them: `pitex-preview`, then `pitex-preview-xetex`.
pub fn helper_hashes(bin: &Path) -> Result<[String; 2]> {
    use sha2::{Digest, Sha256};
    let hash = |name: &str| -> Result<String> {
        Ok(format!(
            "{:x}",
            Sha256::digest(std::fs::read(bin.join(name))?)
        ))
    };
    Ok([hash("pitex-preview")?, hash("pitex-preview-xetex")?])
}

/// Prints a passing case. With PITEX_EVIDENCE set, it also appends one JSON
/// line for `preview-audit-pdftex-vocabulary --evidence`. `reference` names
/// the stock engine (`pdftex`, `xetex`) whose same-run output the case
/// compared; `sources` are the TeX inputs whose control sequences are recorded.
pub fn pass(case: &str, reference: Option<&str>, sources: &[&str], detail: &str) -> Result<()> {
    println!("PASS: {case}, {detail}");
    let Some(path) = std::env::var_os("PITEX_EVIDENCE") else {
        return Ok(());
    };
    static HASHES: std::sync::OnceLock<std::result::Result<[String; 2], String>> =
        std::sync::OnceLock::new();
    let hashes = HASHES
        .get_or_init(|| {
            let bin = std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?;
            helper_hashes(Path::new(&bin)).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone())?;
    let exe = std::env::current_exe()?;
    let tool = exe
        .file_stem()
        .ok_or("tool name missing")?
        .to_string_lossy();
    let pattern = regex::Regex::new(r"\\([A-Za-z]+)")?;
    let names = sources
        .iter()
        .flat_map(|source| pattern.captures_iter(source).map(|m| m[1].to_string()))
        .collect::<std::collections::BTreeSet<_>>();
    let record = serde_json::json!({"tool": tool, "case": case, "reference": reference,
        "helper_sha256": hashes[0], "engine_sha256": hashes[1], "control_sequences": names});
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{record}")?;
    Ok(())
}

pub fn output_timeout(command: &mut Command, timeout: Duration) -> Result<Output> {
    output(command, Some(timeout), false)
}

pub fn output(
    command: &mut Command,
    timeout: Option<Duration>,
    inherit_stderr: bool,
) -> Result<Output> {
    watch_interrupt();
    own_process_group(command);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(if inherit_stderr {
            Stdio::inherit()
        } else {
            Stdio::piped()
        })
        .spawn()?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let read_stdout = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let read_stderr = child.stderr.take().map(|mut stderr| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        })
    });
    let deadline = timeout.map(|timeout| Instant::now() + timeout);
    let status = loop {
        if let Err(error) = check_interrupt() {
            kill_group(&mut child);
            let _ = read_stdout.join();
            if let Some(reader) = read_stderr {
                let _ = reader.join();
            }
            return Err(error);
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            kill_group(&mut child);
            let _ = read_stdout.join();
            if let Some(reader) = read_stderr {
                let _ = reader.join();
            }
            return Err(format!(
                "command timed out after {} seconds: {command:?}",
                timeout.unwrap().as_secs()
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    // A subprocess can exit while its descendants still own a pipe. The
    // timeout and interruption also cover draining both captured streams.
    while !read_stdout.is_finished()
        || read_stderr
            .as_ref()
            .is_some_and(|reader| !reader.is_finished())
    {
        if let Err(error) = check_interrupt() {
            kill_group(&mut child);
            let _ = read_stdout.join();
            if let Some(reader) = read_stderr {
                let _ = reader.join();
            }
            return Err(error);
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            kill_group(&mut child);
            let _ = read_stdout.join();
            if let Some(reader) = read_stderr {
                let _ = reader.join();
            }
            return Err(format!(
                "command timed out after {} seconds: {command:?}",
                timeout.unwrap().as_secs()
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    let stdout = read_stdout.join().map_err(|_| "stdout reader panicked")??;
    let stderr = if let Some(reader) = read_stderr {
        reader.join().map_err(|_| "stderr reader panicked")??
    } else {
        Vec::new()
    };
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn process_io_preserves_lines_and_times_out_descendant_pipes() -> Result<()> {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf 'output'; printf 'error' >&2"]);
        let captured = output_timeout(&mut command, Duration::from_secs(2))?;
        assert!(captured.status.success());
        assert_eq!(captured.stdout, b"output");
        assert_eq!(captured.stderr, b"error");

        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf '{\"event\":\"ready\"}\\n{\"event\":'"]);
        let mut session = Session::spawn(&mut command, Path::new("/nonexistent"))?;
        assert_eq!(
            session.next_event(Duration::from_secs(2))?.unwrap()["event"],
            "ready"
        );
        assert!(session.next_event(Duration::from_secs(2)).is_err());
        let started = Instant::now();
        assert!(session
            .next_event_or_eof(Duration::from_millis(100))?
            .is_none());
        assert!(started.elapsed() >= Duration::from_millis(90));

        let started = Instant::now();
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 5 & exit 0"]);
        assert!(output_timeout(&mut command, Duration::from_millis(100)).is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        Ok(())
    }
}
