//! Rust port of `Packages/TexApp/Sources/MacPlatform` — the Linux
//! implementations of every AppPorts contract. Where macOS uses
//! security-scoped bookmarks, Linux uses canonicalized-path leases.

use app_ports::{
    FileAccessLease, FileCapability, FileCapabilityAccess, LogLevel, LogRecord,
    PDFCoordinateSpace, PDFPoint, PlatformPortError,
    ProcessRequest, ProcessResult,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use url::Url;
use uuid::Uuid;

// ─── File capability broker ────────────────────────────────────────────────

/// Linux has no security-scoped bookmarks; a capability is the canonical
/// absolute path plus the requested access, serialized as bytes. `begin_access`
/// re-resolves the path and reports `staleCapability` when the recorded inode
/// no longer resolves (mirroring `bookmarkDataIsStale`).
pub struct LinuxFileCapabilityBroker {
    active: Mutex<BTreeMap<Uuid, (PathBuf, FileCapabilityAccess)>>,
}
impl Default for LinuxFileCapabilityBroker {
    fn default() -> Self {
        Self::new()
    }
}
impl LinuxFileCapabilityBroker {
    pub fn new() -> Self {
        Self { active: Mutex::new(BTreeMap::new()) }
    }

    fn check_access(path: &Path, access: FileCapabilityAccess) -> bool {
        let readable = access::faccessat(path, libc::R_OK);
        match access {
            FileCapabilityAccess::ReadOnly => readable,
            FileCapabilityAccess::ReadWrite => readable && access::faccessat(path, libc::W_OK),
        }
    }
}
mod access {
    use std::ffi::CString;
    use std::path::Path;
    pub fn faccessat(path: &Path, mode: libc::c_int) -> bool {
        let Ok(c) = CString::new(path.as_os_str().as_encoded_bytes()) else {
            return false;
        };
        unsafe { libc::faccessat(libc::AT_FDCWD, c.as_ptr(), mode, 0) == 0 }
    }
}

/// Spawn a detached child and reap it on a background thread — dropping a
/// `std::process::Child` without `wait()` leaves a zombie until exit.
fn spawn_reaped(command: &mut std::process::Command) -> std::io::Result<()> {
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
impl app_ports::FileCapabilityBroker for LinuxFileCapabilityBroker {
    fn issue_capability(
        &self,
        url: &Path,
        access: FileCapabilityAccess,
    ) -> Result<FileCapability, PlatformPortError> {
        let exists = url.exists();
        if !exists || !Self::check_access(url, access) {
            return Err(PlatformPortError::AccessDenied(url.to_path_buf()));
        }
        let canonical = url
            .canonicalize()
            .map_err(|_| PlatformPortError::AccessDenied(url.to_path_buf()))?;
        // bookmark = "pitex-cap\0<access>\0<canonical path bytes>"
        let mut bookmark = b"pitex-cap\0".to_vec();
        bookmark.push(match access {
            FileCapabilityAccess::ReadOnly => b'r',
            FileCapabilityAccess::ReadWrite => b'w',
        });
        bookmark.push(0);
        bookmark.extend_from_slice(canonical.as_os_str().as_encoded_bytes());
        Ok(FileCapability { bookmark, access })
    }

    fn begin_access(
        &self,
        capability: &FileCapability,
    ) -> Result<FileAccessLease, PlatformPortError> {
        if capability.bookmark.is_empty() {
            return Err(PlatformPortError::InvalidCapability);
        }
        let resolved: PathBuf = {
            let marker = b"pitex-cap\0";
            if capability.bookmark.len() <= marker.len() + 1
                || !capability.bookmark.starts_with(marker)
                || capability.bookmark[marker.len() + 1] != 0
            {
                return Err(PlatformPortError::InvalidCapability);
            }
            let path_bytes = &capability.bookmark[marker.len() + 2..];
            use std::os::unix::ffi::OsStrExt;
            PathBuf::from(std::ffi::OsStr::from_bytes(path_bytes))
        };
        // A bookmark resolves to a canonical path; if the file vanished or the
        // canonical path changed underneath us, the capability is stale.
        let canonical = resolved
            .canonicalize()
            .map_err(|_| PlatformPortError::StaleCapability(resolved.clone()))?;
        if canonical != resolved {
            return Err(PlatformPortError::StaleCapability(resolved));
        }
        if !Self::check_access(&resolved, capability.access) {
            return Err(PlatformPortError::AccessDenied(resolved));
        }
        let lease = FileAccessLease { id: Uuid::new_v4(), url: resolved.clone(), access: capability.access };
        self.active.lock().unwrap().insert(lease.id, (resolved, capability.access));
        Ok(lease)
    }

    fn end_access(&self, lease: FileAccessLease) -> Result<(), PlatformPortError> {
        match self.active.lock().unwrap().remove(&lease.id) {
            Some(_) => Ok(()),
            None => Err(PlatformPortError::InvalidCapability),
        }
    }
}

// ─── Process executor ──────────────────────────────────────────────────────

/// Mirrors `MacProcessExecutor`: one running process per request id, output
/// captured through temp files, `terminate` maps to SIGTERM.
pub struct LinuxProcessExecutor {
    running: Mutex<BTreeMap<Uuid, Arc<RunningProcess>>>,
}
struct RunningProcess {
    child: Mutex<std::process::Child>,
    /// Captured at spawn so `terminate` never has to lock `child` (which
    /// `execute` holds across `wait()` for the process's whole lifetime).
    /// The pid cannot be recycled while the entry is live: `wait()` has not
    /// reaped it yet, so a SIGTERM to a zombie is a harmless no-op.
    pid: libc::pid_t,
}
impl Default for LinuxProcessExecutor {
    fn default() -> Self {
        Self::new()
    }
}
impl LinuxProcessExecutor {
    pub fn new() -> Self {
        Self { running: Mutex::new(BTreeMap::new()) }
    }
}
impl app_ports::ProcessExecuting for LinuxProcessExecutor {
    fn execute(&self, request: &ProcessRequest) -> Result<ProcessResult, PlatformPortError> {
        {
            let running = self.running.lock().unwrap();
            if running.contains_key(&request.id) {
                return Err(PlatformPortError::OperationInProgress(format!(
                    "process {}",
                    request.id
                )));
            }
        }
        let executable = &request.executable;
        let is_executable = executable.is_file() && access::faccessat(executable, libc::X_OK);
        if !is_executable {
            return Err(PlatformPortError::ProcessLaunchFailed {
                executable: executable.clone(),
                reason: "The executable does not exist or is not executable.".to_string(),
            });
        }

        let temporary_directory = std::env::temp_dir()
            .join(format!("texapp-process-{}", request.id.as_simple()));
        std::fs::create_dir_all(&temporary_directory).map_err(|e| {
            PlatformPortError::ProcessLaunchFailed {
                executable: executable.clone(),
                reason: e.to_string(),
            }
        })?;
        let output_path = temporary_directory.join("stdout");
        let error_path = temporary_directory.join("stderr");
        let (output_file, error_file) = match (
            std::fs::File::create(&output_path),
            std::fs::File::create(&error_path),
        ) {
            (Ok(o), Ok(e)) => (o, e),
            _ => {
                let _ = std::fs::remove_dir_all(&temporary_directory);
                return Err(PlatformPortError::ProcessLaunchFailed {
                    executable: executable.clone(),
                    reason: "Could not create process output files.".to_string(),
                });
            }
        };

        let mut command = std::process::Command::new(executable);
        command.args(&request.arguments);
        if request.environment.is_empty() {
            command.envs(std::env::vars());
        } else {
            command.env_clear();
            command.envs(request.environment.iter());
        }
        if let Some(dir) = &request.working_directory {
            command.current_dir(dir);
        }
        command.stdout(std::process::Stdio::from(output_file));
        command.stderr(std::process::Stdio::from(error_file));
        if request.standard_input.is_some() {
            command.stdin(std::process::Stdio::piped());
        } else {
            command.stdin(std::process::Stdio::null());
        }

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&temporary_directory);
                return Err(PlatformPortError::ProcessLaunchFailed {
                    executable: executable.clone(),
                    reason: e.to_string(),
                });
            }
        };
        if let Some(input) = &request.standard_input {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(input).is_err() {
                    let _ = child.kill();
                }
                drop(stdin);
            }
        }
        let pid = child.id() as libc::pid_t;
        let running = Arc::new(RunningProcess {
            child: Mutex::new(child),
            pid,
        });
        self.running.lock().unwrap().insert(request.id, running.clone());

        let status = running.child.lock().unwrap().wait();
        self.running.lock().unwrap().remove(&request.id);

        let termination_status = match status {
            Ok(s) => s.code().unwrap_or_else(|| {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    s.signal().map(|sig| 128 + sig).unwrap_or(1)
                }
                #[cfg(not(unix))]
                {
                    1
                }
            }),
            Err(_) => 1,
        };
        let standard_output = std::fs::read(&output_path).unwrap_or_default();
        let standard_error = std::fs::read(&error_path).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&temporary_directory);
        Ok(ProcessResult { termination_status, standard_output, standard_error })
    }

    fn terminate(&self, id: Uuid) -> Result<(), PlatformPortError> {
        let running = self.running.lock().unwrap();
        match running.get(&id) {
            Some(process) => {
                // Signal the captured pid — never `child.kill()`, which would
                // block on the mutex `execute` holds across `wait()`.
                unsafe { libc::kill(process.pid, libc::SIGTERM) };
                Ok(())
            }
            None => Err(PlatformPortError::InvalidCapability),
        }
    }
}

// ─── Workspace opener ──────────────────────────────────────────────────────

pub struct LinuxWorkspaceOpener;
impl app_ports::WorkspaceOpening for LinuxWorkspaceOpener {
    fn open_document(&self, url: &Path) -> Result<(), PlatformPortError> {
        spawn_reaped(
            std::process::Command::new("xdg-open")
                .arg(url)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null()),
        )
        .map_err(|_| PlatformPortError::AccessDenied(url.to_path_buf()))?;
        Ok(())
    }
    fn reveal_in_file_manager(&self, urls: &[PathBuf]) -> Result<(), PlatformPortError> {
        if urls.is_empty() {
            return Err(PlatformPortError::InvalidCapability);
        }
        // org.freedesktop.FileManager1.ShowItems selects the files like
        // activateFileViewerSelecting does; fall back to xdg-open on the
        // parent directory when the D-Bus service is absent.
        let uri_list: Vec<String> = urls
            .iter()
            .filter_map(|u| Url::from_file_path(u).ok().map(|u| u.to_string()))
            .collect();
        if uri_list.is_empty() {
            return Err(PlatformPortError::InvalidCapability);
        }
        // dbus-send's `array:string:` syntax splits on commas and offers no
        // escaping — a filename containing one would corrupt the array, so
        // skip the D-Bus path entirely and take the xdg-open fallback.
        let dbus_ok = uri_list.iter().all(|u| !u.contains(','))
            && std::process::Command::new("dbus-send")
                .args([
                    "--session",
                    "--print-reply",
                    "--dest=org.freedesktop.FileManager1",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1.ShowItems",
                    &format!("array:string:{}", uri_list.join(",")),
                    "string:",
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        match dbus_ok {
            true => Ok(()),
            _ => {
                let parent = urls[0].parent().unwrap_or(&urls[0]);
                spawn_reaped(
                    std::process::Command::new("xdg-open")
                        .arg(parent)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null()),
                )
                .map_err(|_| PlatformPortError::AccessDenied(urls[0].clone()))?;
                Ok(())
            }
        }
    }
}

// ─── Default-editor registration ───────────────────────────────────────────

/// `DefaultEditorRegistration` — the freedesktop counterpart of
/// `LSSetDefaultRoleHandlerForContentType`: the app claims its declared
/// content types through `xdg-mime`. When no packaged desktop entry exists,
/// a session-local entry pointing at the running executable is installed so
/// the claim resolves.
pub struct LinuxDefaultEditorRegistration;
impl LinuxDefaultEditorRegistration {
    /// Desktop-file ID under which the declared types are claimed.
    const DESKTOP_ID: &'static str = "dev.pitex.app.desktop";
    /// Declared content types — the desktop entry's MimeType list, matching
    /// the bundle's document type declarations.
    const DECLARED_TYPES: &'static [&'static str] = &["text/x-tex", "text/x-bib"];

    /// Registers the app as the default handler for every declared type.
    pub fn register_as_default() -> Result<(), PlatformPortError> {
        Self::ensure_desktop_entry().map_err(|_| PlatformPortError::InvalidCapability)?;
        for mime in Self::DECLARED_TYPES {
            let _ = std::process::Command::new("xdg-mime")
                .args(["default", Self::DESKTOP_ID, mime])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        Ok(())
    }

    /// `xdg-mime default` only sticks when the desktop ID resolves under an
    /// applications directory; install a minimal session-local entry when
    /// the app was launched unpackaged.
    fn ensure_desktop_entry() -> std::io::Result<()> {
        if Self::installed_desktop_entry().is_some() {
            return Ok(());
        }
        let applications = dirs::data_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir"))?
            .join("applications");
        std::fs::create_dir_all(&applications)?;
        let exe = std::env::current_exe()?;
        let mime: String = Self::DECLARED_TYPES.iter().map(|t| format!("{t};")).collect();
        // Desktop Entry spec: inside a double-quoted Exec argument, `\`, `"`,
        // `` ` `` and `$` are backslash-escaped, and a literal `%` doubles
        // (field codes like `%u` must still reach the launcher verbatim).
        let exec_path = exe
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
            .replace('%', "%%");
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName=Pitex\nComment=LaTeX environment\n\
             Exec=\"{exec_path}\" %u\nIcon=dev.pitex.app\nTerminal=false\n\
             Categories=Development;TextEditor;\nStartupNotify=true\nMimeType={mime}\n",
        );
        std::fs::write(applications.join(Self::DESKTOP_ID), entry)
    }

    /// Resolves an existing entry across the XDG applications directories —
    /// user-local first, then the system data dirs.
    fn installed_desktop_entry() -> Option<PathBuf> {
        let mut scan: Vec<PathBuf> = Vec::new();
        if let Some(d) = dirs::data_dir() {
            scan.push(d.join("applications"));
        }
        let system = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
        for d in system.split(':').filter(|d| !d.is_empty()) {
            scan.push(PathBuf::from(d).join("applications"));
        }
        scan.into_iter()
            .map(|d| d.join(Self::DESKTOP_ID))
            .find(|p| p.is_file())
    }
}

// ─── Logger ────────────────────────────────────────────────────────────────

/// OSLog equivalent: level-prefixed lines to stderr with sorted metadata.
pub struct LinuxApplicationLogger {
    pub subsystem: String,
    pub category: String,
}
impl app_ports::ApplicationLogging for LinuxApplicationLogger {
    fn log(&self, record: &LogRecord) -> Result<(), PlatformPortError> {
        let metadata = record
            .metadata
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ");
        let rendered =
            if metadata.is_empty() { record.message.clone() } else { format!("{} {}", record.message, metadata) };
        let level = match record.level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Notice => "notice",
            LogLevel::Error => "error",
            LogLevel::Fault => "fault",
        };
        eprintln!("[{}:{}:{}] {}", self.subsystem, self.category, level, rendered);
        Ok(())
    }
}

// ─── PDF coordinate converter ──────────────────────────────────────────────

/// Identical math to `MacPDFCoordinateConverter` — coordinate spaces are
/// platform-neutral.
pub struct LinuxPDFCoordinateConverter;
impl app_ports::PDFCoordinateConverting for LinuxPDFCoordinateConverter {
    fn convert(
        &self,
        point: PDFPoint,
        from: PDFCoordinateSpace,
        to: PDFCoordinateSpace,
    ) -> Result<PDFPoint, PlatformPortError> {
        let source_page = match from {
            PDFCoordinateSpace::PdfBottomLeft { page, .. }
            | PDFCoordinateSpace::ViewTopLeft { page, .. } => page,
        };
        let destination_page = match to {
            PDFCoordinateSpace::PdfBottomLeft { page, .. }
            | PDFCoordinateSpace::ViewTopLeft { page, .. } => page,
        };
        if source_page != destination_page {
            return Err(PlatformPortError::InvalidCapability);
        }
        let source_rect = match from {
            PDFCoordinateSpace::PdfBottomLeft { media_box, .. }
            | PDFCoordinateSpace::ViewTopLeft { bounds: media_box, .. } => media_box,
        };
        let destination_rect = match to {
            PDFCoordinateSpace::PdfBottomLeft { media_box, .. }
            | PDFCoordinateSpace::ViewTopLeft { bounds: media_box, .. } => media_box,
        };
        if !(source_rect.width > 0.0 && source_rect.height > 0.0)
            || !(destination_rect.width > 0.0 && destination_rect.height > 0.0)
        {
            return Err(PlatformPortError::InvalidCapability);
        }
        let unit = to_unit_bottom_left(point, from);
        Ok(from_unit_bottom_left(unit, to))
    }
}
fn to_unit_bottom_left(point: PDFPoint, space: PDFCoordinateSpace) -> PDFPoint {
    match space {
        PDFCoordinateSpace::PdfBottomLeft { media_box: b, .. } => PDFPoint {
            x: (point.x - b.origin.x) / b.width,
            y: (point.y - b.origin.y) / b.height,
        },
        PDFCoordinateSpace::ViewTopLeft { bounds: b, .. } => PDFPoint {
            x: (point.x - b.origin.x) / b.width,
            y: 1.0 - ((point.y - b.origin.y) / b.height),
        },
    }
}
fn from_unit_bottom_left(point: PDFPoint, space: PDFCoordinateSpace) -> PDFPoint {
    match space {
        PDFCoordinateSpace::PdfBottomLeft { media_box: b, .. } => PDFPoint {
            x: b.origin.x + point.x * b.width,
            y: b.origin.y + point.y * b.height,
        },
        PDFCoordinateSpace::ViewTopLeft { bounds: b, .. } => PDFPoint {
            x: b.origin.x + point.x * b.width,
            y: b.origin.y + (1.0 - point.y) * b.height,
        },
    }
}

// ─── Clock / UUID ──────────────────────────────────────────────────────────

pub struct SystemWallClock;
impl app_ports::WallClock for SystemWallClock {
    fn now_milliseconds(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

pub struct SystemUuidGenerator;
impl app_ports::UuidGenerating for SystemUuidGenerator {
    fn make_uuid(&self) -> Uuid {
        Uuid::new_v4()
    }
}

// ─── Environment bundle ────────────────────────────────────────────────────

/// The `AppEnvironment` equivalent: the Linux adapter bundle the shell wires
/// into feature states. Construction is infallible on Linux — the app IS the
/// Linux implementation, unlike `AppShell.make` which fails off-macOS.
pub struct LinuxEnvironment {
    pub files: LinuxFileCapabilityBroker,
    pub processes: LinuxProcessExecutor,
    pub pdf_coordinates: LinuxPDFCoordinateConverter,
    pub workspace: LinuxWorkspaceOpener,
    pub clock: SystemWallClock,
    pub uuids: SystemUuidGenerator,
    pub logger: LinuxApplicationLogger,
}
impl LinuxEnvironment {
    pub fn make(logging_subsystem: &str) -> Self {
        Self {
            files: LinuxFileCapabilityBroker::new(),
            processes: LinuxProcessExecutor::new(),
            pdf_coordinates: LinuxPDFCoordinateConverter,
            workspace: LinuxWorkspaceOpener,
            clock: SystemWallClock,
            uuids: SystemUuidGenerator,
            logger: LinuxApplicationLogger {
                subsystem: logging_subsystem.to_string(),
                category: "application".to_string(),
            },
        }
    }
}
