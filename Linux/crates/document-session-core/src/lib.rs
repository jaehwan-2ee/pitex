//! Port of `Packages/TexCore/Sources/DocumentSessionCore` — the canonical
//! revisioned document session, session registry, and atomic persistence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tex_domain::{fnv1a64, NormalizedRelativePath, StableDocumentID};

// ---------------------------------------------------------------------------
// DiskContentHash

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiskContentHash {
    pub raw_value: u64,
}

impl DiskContentHash {
    pub fn new(raw_value: u64) -> Self {
        Self { raw_value }
    }

    pub fn hashing(text: &str) -> Self {
        Self {
            raw_value: fnv1a64(text.as_bytes()),
        }
    }

    pub fn hashing_bytes(data: &[u8]) -> Self {
        Self {
            raw_value: fnv1a64(data),
        }
    }
}

impl fmt::Display for DiskContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw_value)
    }
}

// ---------------------------------------------------------------------------
// Document session state

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentConflict {
    ExternalModification {
        baseline: DiskContentHash,
        #[serde(rename = "observedDisk")]
        observed_disk: DiskContentHash,
    },
    SaveCollision {
        baseline: DiskContentHash,
        #[serde(rename = "observedDisk")]
        observed_disk: DiskContentHash,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentSaveState {
    Clean,
    Dirty,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    #[serde(rename = "documentID")]
    pub document_id: StableDocumentID,
    pub path: NormalizedRelativePath,
    pub revision: u64,
    pub text: String,
    #[serde(rename = "diskBaselineHash")]
    pub disk_baseline_hash: DiskContentHash,
    #[serde(rename = "contentHash")]
    pub content_hash: DiskContentHash,
    #[serde(rename = "saveState")]
    pub save_state: DocumentSaveState,
    pub conflict: Option<DocumentConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocumentMutation {
    ReplaceText(String),
    RecordExternalChange { observed_disk_hash: DiskContentHash },
    RecordSaveConflict { observed_disk_hash: DiskContentHash },
    CommitSave { written_disk_hash: DiskContentHash },
    ResolveConflict {
        text: String,
        disk_baseline_hash: DiskContentHash,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentSessionError {
    StaleRevision { expected: u64, actual: u64 },
    SavedContentHashMismatch {
        expected: DiskContentHash,
        written: DiskContentHash,
    },
    RevisionExhausted,
}

impl fmt::Display for DocumentSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => {
                write!(f, "stale revision (expected {expected}, actual {actual})")
            }
            Self::SavedContentHashMismatch { expected, written } => {
                write!(f, "saved content hash mismatch (expected {expected}, written {written})")
            }
            Self::RevisionExhausted => write!(f, "revision exhausted"),
        }
    }
}
impl std::error::Error for DocumentSessionError {}

struct SessionInner {
    revision: u64,
    text: String,
    disk_baseline_hash: DiskContentHash,
    conflict: Option<DocumentConflict>,
}

/// Cloneable handle to the canonical revisioned session. In Swift this is an
/// `actor`; here the shared state lives behind a mutex and all operations are
/// serialized, preserving the same semantics.
#[derive(Clone)]
pub struct DocumentSession {
    document_id: StableDocumentID,
    path: NormalizedRelativePath,
    inner: Arc<Mutex<SessionInner>>,
}

impl DocumentSession {
    pub fn new(
        file: &project_core::ProjectFile,
        initial_text: String,
        disk_baseline_hash: Option<DiskContentHash>,
    ) -> Self {
        Self {
            document_id: file.document_id.clone(),
            path: file.path.clone(),
            inner: Arc::new(Mutex::new(SessionInner {
                revision: 0,
                disk_baseline_hash: disk_baseline_hash
                    .unwrap_or_else(|| DiskContentHash::hashing(&initial_text)),
                text: initial_text,
                conflict: None,
            })),
        }
    }

    pub fn document_id(&self) -> &StableDocumentID {
        &self.document_id
    }

    pub fn path(&self) -> &NormalizedRelativePath {
        &self.path
    }

    /// Identity comparison — equivalent to Swift's `===` on the actor.
    pub fn same_session(&self, other: &DocumentSession) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        let inner = self.inner.lock().unwrap();
        self.make_snapshot(&inner)
    }

    pub fn apply(
        &self,
        mutation: DocumentMutation,
        expected_revision: u64,
    ) -> Result<DocumentSnapshot, DocumentSessionError> {
        let mut inner = self.inner.lock().unwrap();
        if expected_revision != inner.revision {
            return Err(DocumentSessionError::StaleRevision {
                expected: expected_revision,
                actual: inner.revision,
            });
        }
        if inner.revision == u64::MAX {
            return Err(DocumentSessionError::RevisionExhausted);
        }

        match mutation {
            DocumentMutation::ReplaceText(replacement) => {
                inner.text = replacement;
            }
            DocumentMutation::RecordExternalChange { observed_disk_hash } => {
                if observed_disk_hash != inner.disk_baseline_hash && inner.conflict.is_none() {
                    inner.conflict = Some(DocumentConflict::ExternalModification {
                        baseline: inner.disk_baseline_hash,
                        observed_disk: observed_disk_hash,
                    });
                }
            }
            DocumentMutation::RecordSaveConflict { observed_disk_hash } => {
                if inner.conflict.is_none() {
                    inner.conflict = Some(DocumentConflict::SaveCollision {
                        baseline: inner.disk_baseline_hash,
                        observed_disk: observed_disk_hash,
                    });
                }
            }
            DocumentMutation::CommitSave { written_disk_hash } => {
                let current_hash = DiskContentHash::hashing(&inner.text);
                if written_disk_hash != current_hash {
                    return Err(DocumentSessionError::SavedContentHashMismatch {
                        expected: current_hash,
                        written: written_disk_hash,
                    });
                }
                inner.disk_baseline_hash = written_disk_hash;
                inner.conflict = None;
            }
            DocumentMutation::ResolveConflict {
                text,
                disk_baseline_hash,
            } => {
                inner.text = text;
                inner.disk_baseline_hash = disk_baseline_hash;
                inner.conflict = None;
            }
        }

        inner.revision += 1;
        Ok(self.make_snapshot(&inner))
    }

    fn make_snapshot(&self, inner: &SessionInner) -> DocumentSnapshot {
        let content_hash = DiskContentHash::hashing(&inner.text);
        let save_state = if inner.conflict.is_some() {
            DocumentSaveState::Conflicted
        } else if content_hash != inner.disk_baseline_hash {
            DocumentSaveState::Dirty
        } else {
            DocumentSaveState::Clean
        };
        DocumentSnapshot {
            document_id: self.document_id.clone(),
            path: self.path.clone(),
            revision: inner.revision,
            text: inner.text.clone(),
            disk_baseline_hash: inner.disk_baseline_hash,
            content_hash,
            save_state,
            conflict: inner.conflict,
        }
    }
}

// ---------------------------------------------------------------------------
// Originated mutations (registry-level API)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentMutationOrigin {
    TextKit,
    ExternalDisk,
    AiProposal,
    Recovery,
}

pub struct OriginatedDocumentMutation {
    pub mutation: DocumentMutation,
    pub origin: DocumentMutationOrigin,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginatedDocumentMutationError {
    ExpectedRevisionRequired { origin: DocumentMutationOrigin },
}

impl fmt::Display for OriginatedDocumentMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for OriginatedDocumentMutationError {}

#[derive(Debug)]
pub enum ApplyOriginatedError {
    Revision(OriginatedDocumentMutationError),
    Session(DocumentSessionError),
}

impl fmt::Display for ApplyOriginatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Revision(e) => write!(f, "{e}"),
            Self::Session(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for ApplyOriginatedError {}

impl DocumentSession {
    pub fn apply_originated(
        &self,
        originated: OriginatedDocumentMutation,
    ) -> Result<DocumentSnapshot, ApplyOriginatedError> {
        let revision = if let Some(expected) = originated.expected_revision {
            expected
        } else {
            if originated.origin != DocumentMutationOrigin::TextKit {
                return Err(ApplyOriginatedError::Revision(
                    OriginatedDocumentMutationError::ExpectedRevisionRequired {
                        origin: originated.origin,
                    },
                ));
            }
            self.snapshot().revision
        };
        self.apply(originated.mutation, revision)
            .map_err(ApplyOriginatedError::Session)
    }
}

// ---------------------------------------------------------------------------
// Registry

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentSessionRegistryError {
    ProjectRootMustBeFileURL,
    DocumentIdentityMismatch {
        path: NormalizedRelativePath,
        existing: StableDocumentID,
        requested: StableDocumentID,
    },
}

impl fmt::Display for DocumentSessionRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DocumentSessionRegistryError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RegistryKey {
    canonical_project_root: String,
    path: NormalizedRelativePath,
}

fn canonical_root(project_root: &Path) -> Result<String, DocumentSessionRegistryError> {
    // `standardizedFileURL + resolvingSymlinksInPath` on Linux: canonicalize
    // resolves symlinks and normalizes. Falls back to lexical cleanup when
    // the path does not exist yet.
    match project_root.canonicalize() {
        Ok(p) => Ok(p.to_string_lossy().into_owned()),
        Err(_) => Ok(normalize_absolute_path(project_root)),
    }
}

fn normalize_absolute_path(path: &Path) -> String {
    let mut components: Vec<String> = Vec::new();
    for comp in path.to_string_lossy().split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            other => components.push(other.to_string()),
        }
    }
    format!("/{}", components.join("/"))
}

#[derive(Default)]
pub struct DocumentSessionRegistry {
    sessions: Mutex<HashMap<RegistryKey, DocumentSession>>,
}

impl DocumentSessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(
        &self,
        project_root: &Path,
        file: &project_core::ProjectFile,
        initial_text: String,
        disk_baseline_hash: Option<DiskContentHash>,
    ) -> Result<DocumentSession, DocumentSessionRegistryError> {
        let key = RegistryKey {
            canonical_project_root: canonical_root(project_root)?,
            path: file.path.clone(),
        };
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(existing) = sessions.get(&key) {
            if existing.document_id() != &file.document_id {
                return Err(DocumentSessionRegistryError::DocumentIdentityMismatch {
                    path: file.path.clone(),
                    existing: existing.document_id().clone(),
                    requested: file.document_id.clone(),
                });
            }
            return Ok(existing.clone());
        }
        let session = DocumentSession::new(file, initial_text, disk_baseline_hash);
        sessions.insert(key, session.clone());
        Ok(session)
    }

    pub fn close(
        &self,
        project_root: &Path,
        session: &DocumentSession,
    ) -> Result<bool, DocumentSessionRegistryError> {
        let key = RegistryKey {
            canonical_project_root: canonical_root(project_root)?,
            path: session.path().clone(),
        };
        let mut sessions = self.sessions.lock().unwrap();
        match sessions.get(&key) {
            Some(registered) if registered.same_session(session) => {
                sessions.remove(&key);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn session(
        &self,
        project_root: &Path,
        path: &NormalizedRelativePath,
    ) -> Result<Option<DocumentSession>, DocumentSessionRegistryError> {
        let key = RegistryKey {
            canonical_project_root: canonical_root(project_root)?,
            path: path.clone(),
        };
        Ok(self.sessions.lock().unwrap().get(&key).cloned())
    }

    pub fn count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }
}

// ---------------------------------------------------------------------------
// Atomic persistence

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedDocument {
    pub text: String,
    pub hash: DiskContentHash,
}

impl PersistedDocument {
    pub fn new(text: String, hash: Option<DiskContentHash>) -> Self {
        Self {
            hash: hash.unwrap_or_else(|| DiskContentHash::hashing(&text)),
            text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPersistenceConflict {
    pub expected_baseline_hash: Option<DiskContentHash>,
    pub local: PersistedDocument,
    pub observed_disk: Option<PersistedDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentLoadOutcome {
    Loaded(PersistedDocument),
    ExternalConflict(DocumentPersistenceConflict),
    PermissionFailure { path: String },
    InterruptedWrite { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentSaveOutcome {
    Saved(PersistedDocument),
    StaleBaseline(DocumentPersistenceConflict),
    PermissionFailure { path: String },
    InterruptedWrite { path: String },
}

/// Read/write of exact UTF-8 with atomic-rename saves and disk-baseline
/// conflict detection — `FoundationAtomicDocumentStore` semantics.
pub struct AtomicDocumentStore;

enum StoreError {
    Permission,
    InvalidUtf8,
    Other,
}

impl AtomicDocumentStore {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self, url: &Path, expected_baseline_hash: Option<DiskContentHash>) -> DocumentLoadOutcome {
        match read_document(url) {
            Ok(disk) => {
                if let Some(expected) = expected_baseline_hash {
                    if disk.hash != expected {
                        return DocumentLoadOutcome::ExternalConflict(DocumentPersistenceConflict {
                            expected_baseline_hash,
                            local: disk.clone(),
                            observed_disk: Some(disk),
                        });
                    }
                }
                DocumentLoadOutcome::Loaded(disk)
            }
            Err(StoreError::Permission) => DocumentLoadOutcome::PermissionFailure {
                path: url.to_string_lossy().into_owned(),
            },
            Err(_) => DocumentLoadOutcome::InterruptedWrite {
                path: url.to_string_lossy().into_owned(),
            },
        }
    }

    pub fn save(
        &self,
        text: &str,
        url: &Path,
        expected_baseline_hash: Option<DiskContentHash>,
    ) -> DocumentSaveOutcome {
        let local = PersistedDocument::new(text.to_string(), None);
        let parent = url.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
        let file_name = url
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let temp_name = format!(".{file_name}.texspark-{}.tmp", uuid_v4());
        let temporary_url = parent.join(&temp_name);

        let result = (|| -> Result<DocumentSaveOutcome, StoreError> {
            // O_EXCL create of the temporary file — write(withoutOverwriting).
            {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary_url)
                    .map_err(classify_io)?;
                file.write_all(text.as_bytes()).map_err(classify_io)?;
                file.sync_all().map_err(classify_io)?;
            }

            let observed_disk = if url.exists() {
                Some(read_document(url)?)
            } else {
                None
            };

            let baseline_ok = observed_disk.as_ref().map(|d| d.hash) == expected_baseline_hash
                && observed_disk.is_some() == expected_baseline_hash.is_some();
            if !baseline_ok {
                return Ok(DocumentSaveOutcome::StaleBaseline(DocumentPersistenceConflict {
                    expected_baseline_hash,
                    local,
                    observed_disk,
                }));
            }

            std::fs::rename(&temporary_url, url).map_err(classify_io)?;
            synchronize_directory(&parent)?;
            Ok(DocumentSaveOutcome::Saved(local))
        })();

        // `defer { try? removeItem(temporaryURL) }` — on success the rename
        // consumed the path so removal is a no-op anyway.
        let _ = std::fs::remove_file(&temporary_url);

        match result {
            Ok(outcome) => outcome,
            Err(StoreError::Permission) => DocumentSaveOutcome::PermissionFailure {
                path: url.to_string_lossy().into_owned(),
            },
            Err(_) => DocumentSaveOutcome::InterruptedWrite {
                path: url.to_string_lossy().into_owned(),
            },
        }
    }
}

/// `fsync` on the containing directory after rename. Tolerates EINVAL,
/// EOPNOTSUPP, and EROFS on Linux (matching the Swift Glibc branch).
fn synchronize_directory(directory: &Path) -> Result<(), StoreError> {
    use std::os::unix::io::AsRawFd;
    let file = std::fs::File::open(directory).map_err(classify_io)?;
    let fd = file.as_raw_fd();
    let result = unsafe { libc::fsync(fd) };
    if result == 0 {
        return Ok(());
    }
    let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    if code == libc::EINVAL || code == libc::EOPNOTSUPP || code == libc::EROFS {
        return Ok(());
    }
    Err(if code == libc::EACCES || code == libc::EPERM {
        StoreError::Permission
    } else {
        StoreError::Other
    })
}

fn read_document(url: &Path) -> Result<PersistedDocument, StoreError> {
    let data = std::fs::read(url).map_err(classify_io)?;
    let text = String::from_utf8(data).map_err(|_| StoreError::InvalidUtf8)?;
    Ok(PersistedDocument::new(text, None))
}

fn classify_io(error: std::io::Error) -> StoreError {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::PermissionDenied => StoreError::Permission,
        _ => match error.raw_os_error() {
            Some(code) if code == libc::EACCES || code == libc::EPERM || code == libc::EROFS => {
                StoreError::Permission
            }
            _ => StoreError::Other,
        },
    }
}

fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    getrandom_16(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

fn getrandom_16(buf: &mut [u8; 16]) {
    // getrandom(2) first — the kernel CSPRNG with no fd or path dependency.
    let mut filled = 0usize;
    while filled < buf.len() {
        let rc = unsafe {
            libc::getrandom(
                buf[filled..].as_mut_ptr().cast::<libc::c_void>(),
                buf.len() - filled,
                0,
            )
        };
        if rc > 0 {
            filled += rc as usize;
            continue;
        }
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        break;
    }
    if filled == buf.len() {
        return;
    }
    // /dev/urandom fallback (paranoia; getrandom cannot fail on Linux ≥3.17).
    use std::io::Read;
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        if file.read_exact(&mut buf[filled..]).is_ok() {
            return;
        }
    }
    // Last resort: uniqueness mixing, never expected to run. Time + pid +
    // counter keeps ids distinct even within a single process.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut state = (nanos as u64)
        ^ ((std::process::id() as u64) << 32)
        ^ COUNTER.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    for byte in buf[filled..].iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = state as u8;
    }
}
