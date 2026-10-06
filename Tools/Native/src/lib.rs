use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub mod preview_regression;
pub use preview_regression::output as run_output;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub fn watch_interrupt() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        #[cfg(unix)]
        unsafe {
            extern "C" fn interrupt(_: libc::c_int) {
                INTERRUPTED.store(true, Ordering::Relaxed);
            }
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = interrupt as *const () as libc::sighandler_t;
            libc::sigemptyset(&mut action.sa_mask);
            libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut());
        }
        #[cfg(windows)]
        unsafe {
            #[link(name = "kernel32")]
            extern "system" {
                fn SetConsoleCtrlHandler(
                    handler: Option<extern "system" fn(u32) -> i32>,
                    add: i32,
                ) -> i32;
            }
            extern "system" fn interrupt(event: u32) -> i32 {
                if event == 0 || event == 1 {
                    INTERRUPTED.store(true, Ordering::Relaxed);
                    1
                } else {
                    0
                }
            }
            SetConsoleCtrlHandler(Some(interrupt), 1);
        }
    });
}

pub fn check_interrupt() -> Result<(), Box<dyn Error>> {
    if INTERRUPTED.load(Ordering::Relaxed) {
        Err(io::Error::new(io::ErrorKind::Interrupted, "interrupted").into())
    } else {
        Ok(())
    }
}

pub fn interrupted(error: &(dyn Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::Interrupted)
}

/// Report failure only after the check's temporary directories and children are dropped.
pub fn exit_on_error(result: Result<(), Box<dyn Error>>) {
    if let Err(error) = result {
        if interrupted(error.as_ref()) {
            std::process::exit(130);
        }
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn stop_child(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn repo_root() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(root) = std::env::var_os("PITEX_TOOLS_REPO_ROOT") {
        return Ok(PathBuf::from(root).canonicalize()?);
    }
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

/// A private temporary directory removed on success and error, like TemporaryDirectory.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(prefix: &str) -> io::Result<Self> {
        watch_interrupt();
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..100 {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "{prefix}{}-{nonce}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Cannot create temporary directory",
        ))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn close(self) -> io::Result<()> {
        match fs::remove_dir_all(&self.0) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn run_checked(command: &mut Command, timeout: Option<Duration>) -> Result<(), Box<dyn Error>> {
    watch_interrupt();
    check_interrupt()?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    let started = Instant::now();
    let status = loop {
        if let Err(error) = check_interrupt() {
            stop_child(&mut child);
            return Err(error);
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if timeout.is_some_and(|timeout| started.elapsed() >= timeout) {
            stop_child(&mut child);
            return Err(format!(
                "Command {command:?} timed out after {} seconds",
                timeout.unwrap().as_secs()
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    if status.success() {
        Ok(())
    } else {
        Err(format!("Command {command:?} failed with {status}").into())
    }
}

/// Python json.dumps' default spacing and ASCII escapes for generated source paths/output.
pub fn python_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => {
            let mut quoted = String::from("\"");
            for character in value.chars() {
                match character {
                    '"' => quoted.push_str("\\\""),
                    '\\' => quoted.push_str("\\\\"),
                    '\n' => quoted.push_str("\\n"),
                    '\r' => quoted.push_str("\\r"),
                    '\t' => quoted.push_str("\\t"),
                    '\u{8}' => quoted.push_str("\\b"),
                    '\u{c}' => quoted.push_str("\\f"),
                    c if !(' '..='\u{7e}').contains(&c) => {
                        let mut buffer = [0; 2];
                        for unit in c.encode_utf16(&mut buffer) {
                            quoted.push_str(&format!("\\u{unit:04x}"));
                        }
                    }
                    c => quoted.push(c),
                }
            }
            quoted.push('"');
            quoted
        }
        serde_json::Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!(
                    "{}: {}",
                    python_json(&key.clone().into()),
                    python_json(value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(python_json)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        serde_json::Value::Number(value) if value.is_f64() => {
            let value = value.as_f64().unwrap();
            if value != 0.0 && (value.abs() < 0.0001 || value.abs() >= 1e16) {
                let scientific = format!("{value:e}");
                let (mantissa, exponent) = scientific.split_once('e').unwrap();
                let exponent: i32 = exponent.parse().unwrap();
                format!("{mantissa}e{exponent:+03}")
            } else {
                let mut decimal = value.to_string();
                if !decimal.contains('.') {
                    decimal.push_str(".0");
                }
                decimal
            }
        }
        value => value.to_string(),
    }
}
