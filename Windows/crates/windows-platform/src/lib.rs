//! Windows platform adapters — the counterpart of `linux-platform`, wired to
//! the same `app-ports` contracts. File capabilities are bookmark bytes like
//! the Linux/macOS brokers; access checks consult file metadata (the read-only
//! attribute) since NTFS ACL evaluation needs no extra crate for the app's
//! purposes. Process termination uses `taskkill /T`, document opening uses
//! `start`, and default-editor registration writes `HKCU\Software\Classes`.

use std::collections::BTreeMap;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// `pitex.exe` is a GUI-subsystem app — a console child spawned without
/// `CREATE_NO_WINDOW` pops a console window per call. The child still gets
/// a hidden console, and GUI children (explorer) simply ignore it.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use app_ports::{
    FileAccessLease, FileCapability, FileCapabilityAccess, LogLevel, LogRecord, PDFCoordinateSpace,
    PDFPoint, PlatformPortError, ProcessRequest, ProcessResult,
};

// ─── File capability broker ────────────────────────────────────────────────

/// Mirrors `MacFileCapabilityBroker`/`LinuxFileCapabilityBroker`: a capability
/// is issued for an absolute path plus the requested access, serialized as
/// bytes. `begin_access` re-resolves the path and reports `staleCapability`
/// when the recorded path no longer resolves (mirroring
/// `bookmarkDataIsStale`).
pub struct WindowsFileCapabilityBroker {
    active: Mutex<BTreeMap<Uuid, (PathBuf, FileCapabilityAccess)>>,
}
impl Default for WindowsFileCapabilityBroker {
    fn default() -> Self {
        Self::new()
    }
}
impl WindowsFileCapabilityBroker {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(BTreeMap::new()),
        }
    }

    /// Read access requires the path to exist; write access additionally
    /// requires the read-only attribute to be clear. Directories report
    /// writable when they are not marked read-only.
    fn check_access(path: &Path, access: FileCapabilityAccess) -> bool {
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        match access {
            FileCapabilityAccess::ReadOnly => true,
            FileCapabilityAccess::ReadWrite => !meta.permissions().readonly(),
        }
    }
}

impl app_ports::FileCapabilityBroker for WindowsFileCapabilityBroker {
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
        // bookmark = "pitex-cap\0<access>\0<canonical path as UTF-8>" — UTF-8
        // round-trips every valid Windows path and needs no unsafe OsStr
        // decoding (unlike the Unix raw-byte form).
        let mut bookmark = b"pitex-cap\0".to_vec();
        bookmark.push(match access {
            FileCapabilityAccess::ReadOnly => b'r',
            FileCapabilityAccess::ReadWrite => b'w',
        });
        bookmark.push(0);
        bookmark.extend_from_slice(canonical.to_string_lossy().as_bytes());
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
            let text = std::str::from_utf8(path_bytes)
                .map_err(|_| PlatformPortError::InvalidCapability)?;
            PathBuf::from(text)
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
        let lease = FileAccessLease {
            id: Uuid::new_v4(),
            url: resolved.clone(),
            access: capability.access,
        };
        self.active
            .lock()
            .unwrap()
            .insert(lease.id, (resolved, capability.access));
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
/// captured through temp files, `terminate` maps to `taskkill /T`.
pub struct WindowsProcessExecutor {
    running: Mutex<BTreeMap<Uuid, Arc<RunningProcess>>>,
}
struct RunningProcess {
    child: Mutex<std::process::Child>,
    /// Captured at spawn so `terminate` never has to lock `child` (which
    /// `execute` holds across `wait()` for the process's whole lifetime).
    /// `taskkill` targets the pid tree, matching the Linux process-group
    /// semantics of the port.
    pid: u32,
}
impl Default for WindowsProcessExecutor {
    fn default() -> Self {
        Self::new()
    }
}
impl WindowsProcessExecutor {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(BTreeMap::new()),
        }
    }
}
impl app_ports::ProcessExecuting for WindowsProcessExecutor {
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
        if !executable.is_file() {
            return Err(PlatformPortError::ProcessLaunchFailed {
                executable: executable.clone(),
                reason: "The executable does not exist or is not executable.".to_string(),
            });
        }

        let temporary_directory =
            std::env::temp_dir().join(format!("texapp-process-{}", request.id.as_simple()));
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
        command.creation_flags(CREATE_NO_WINDOW);
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
        let pid = child.id();
        let running = Arc::new(RunningProcess {
            child: Mutex::new(child),
            pid,
        });
        self.running
            .lock()
            .unwrap()
            .insert(request.id, running.clone());

        let status = running.child.lock().unwrap().wait();
        self.running.lock().unwrap().remove(&request.id);

        let termination_status = match status {
            Ok(s) => s.code().unwrap_or(1),
            Err(_) => 1,
        };
        let standard_output = std::fs::read(&output_path).unwrap_or_default();
        let standard_error = std::fs::read(&error_path).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&temporary_directory);
        Ok(ProcessResult {
            termination_status,
            standard_output,
            standard_error,
        })
    }

    fn terminate(&self, id: Uuid) -> Result<(), PlatformPortError> {
        let running = self.running.lock().unwrap();
        match running.get(&id) {
            Some(process) => {
                // Kill the captured pid's whole tree — the `SIGTERM` analogue.
                // `taskkill` is detached so it never touches `child`'s mutex.
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &process.pid.to_string(), "/T", "/F"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                Ok(())
            }
            None => Err(PlatformPortError::InvalidCapability),
        }
    }
}

// ─── Workspace opener ──────────────────────────────────────────────────────

pub struct WindowsWorkspaceOpener;
impl app_ports::WorkspaceOpening for WindowsWorkspaceOpener {
    fn open_document(&self, url: &Path) -> Result<(), PlatformPortError> {
        // `start` resolves the file association like openURL does on macOS;
        // it is a cmd builtin, so it has to go through `cmd /c`.
        spawn_reaped(
            std::process::Command::new("cmd")
                .args(["/c", "start", "", &url.to_string_lossy()])
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
        // `explorer /select,<path>` selects the file — the counterpart of
        // activateFileViewerSelecting/ShowItems. It takes one path; with
        // several, open the shared parent instead.
        let (program, args) = if urls.len() == 1 {
            (
                "explorer".to_string(),
                vec![format!("/select,{}", urls[0].to_string_lossy())],
            )
        } else {
            let parent = urls[0].parent().unwrap_or(&urls[0]).to_path_buf();
            (
                "explorer".to_string(),
                vec![parent.to_string_lossy().into_owned()],
            )
        };
        spawn_reaped(
            std::process::Command::new(program)
                .args(&args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null()),
        )
        .map_err(|_| PlatformPortError::AccessDenied(urls[0].clone()))?;
        Ok(())
    }
}

/// Spawn a detached child and reap it on a background thread — a dropped
/// `std::process::Child` leaves the handle open until exit.
fn spawn_reaped(command: &mut std::process::Command) -> std::io::Result<()> {
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

// ─── Default-editor registration ───────────────────────────────────────────

/// `DefaultEditorRegistration` — the Windows counterpart of
/// `LSSetDefaultRoleHandlerForContentType`/`xdg-mime`: the app claims its
/// declared extensions under `HKCU\Software\Classes` with a ProgID whose
/// `shell\open\command` points at the running executable. Per-user classes
/// need no elevation.
pub struct WindowsDefaultEditorRegistration;
impl WindowsDefaultEditorRegistration {
    /// Declared extensions — the counterpart of the desktop entry's MimeType
    /// list / the bundle's document types.
    const DECLARED_EXTENSIONS: &'static [&'static str] = &[".tex", ".bib"];

    /// Registers the app as the default handler for every declared type.
    pub fn register_as_default() -> Result<(), PlatformPortError> {
        let exe = std::env::current_exe().map_err(|_| PlatformPortError::InvalidCapability)?;
        let command = format!("\"{}\" \"%1\"", exe.to_string_lossy());
        for extension in Self::DECLARED_EXTENSIONS {
            let progid = format!("Pitex{extension}");
            // HKCU\Software\Classes\.tex -> Pitex.tex
            Self::reg_add(&format!("HKCU\\Software\\Classes\\{extension}"), &progid);
            // ProgID friendly name + open command.
            Self::reg_add(
                &format!("HKCU\\Software\\Classes\\{progid}"),
                &format!("Pitex {extension} Document"),
            );
            Self::reg_add(
                &format!("HKCU\\Software\\Classes\\{progid}\\shell\\open\\command"),
                &command,
            );
        }
        Ok(())
    }

    /// `/ve` writes the key's (Default) value — the only slot this
    /// registration needs.
    fn reg_add(key: &str, data: &str) {
        let _ = std::process::Command::new("reg")
            .args(["add", key, "/ve", "/d", data, "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

// ─── Logger ────────────────────────────────────────────────────────────────

/// OSLog equivalent: level-prefixed lines to stderr with sorted metadata.
/// Identical to the Linux implementation — stderr is platform-neutral.
pub struct WindowsApplicationLogger {
    pub subsystem: String,
    pub category: String,
}
impl app_ports::ApplicationLogging for WindowsApplicationLogger {
    fn log(&self, record: &LogRecord) -> Result<(), PlatformPortError> {
        let metadata = record
            .metadata
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ");
        let rendered = if metadata.is_empty() {
            record.message.clone()
        } else {
            format!("{} {}", record.message, metadata)
        };
        let level = match record.level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Notice => "notice",
            LogLevel::Error => "error",
            LogLevel::Fault => "fault",
        };
        eprintln!(
            "[{}:{}:{}] {}",
            self.subsystem, self.category, level, rendered
        );
        Ok(())
    }
}

// ─── PDF coordinate converter ──────────────────────────────────────────────

/// Identical math to `MacPDFCoordinateConverter` — coordinate spaces are
/// platform-neutral.
pub struct WindowsPDFCoordinateConverter;
impl app_ports::PDFCoordinateConverting for WindowsPDFCoordinateConverter {
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
            | PDFCoordinateSpace::ViewTopLeft {
                bounds: media_box, ..
            } => media_box,
        };
        let destination_rect = match to {
            PDFCoordinateSpace::PdfBottomLeft { media_box, .. }
            | PDFCoordinateSpace::ViewTopLeft {
                bounds: media_box, ..
            } => media_box,
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

/// The `AppEnvironment` equivalent: the Windows adapter bundle the shell
/// wires into feature states — same shape as `LinuxEnvironment`.
pub struct WindowsEnvironment {
    pub files: WindowsFileCapabilityBroker,
    pub processes: WindowsProcessExecutor,
    pub pdf_coordinates: WindowsPDFCoordinateConverter,
    pub workspace: WindowsWorkspaceOpener,
    pub clock: SystemWallClock,
    pub uuids: SystemUuidGenerator,
    pub logger: WindowsApplicationLogger,
}
impl WindowsEnvironment {
    pub fn make(logging_subsystem: &str) -> Self {
        Self {
            files: WindowsFileCapabilityBroker::new(),
            processes: WindowsProcessExecutor::new(),
            pdf_coordinates: WindowsPDFCoordinateConverter,
            workspace: WindowsWorkspaceOpener,
            clock: SystemWallClock,
            uuids: SystemUuidGenerator,
            logger: WindowsApplicationLogger {
                subsystem: logging_subsystem.to_string(),
                category: "application".to_string(),
            },
        }
    }
}
