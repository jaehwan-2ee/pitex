//! Rust port of `Packages/TexApp/Sources/AppPorts` — the platform port
//! contracts the feature layer codes against. Traits are object-safe and
//! synchronous; Linux adapters provide the implementations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformPortError {
    Unavailable { feature: String, platform: String },
    InvalidCapability,
    AccessDenied(PathBuf),
    StaleCapability(PathBuf),
    ProcessLaunchFailed { executable: PathBuf, reason: String },
    OperationInProgress(String),
}
impl fmt::Display for PlatformPortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PlatformPortError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileCapabilityAccess {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileCapability {
    #[serde(with = "serde_bytes_compat")]
    pub bookmark: Vec<u8>,
    pub access: FileCapabilityAccess,
}

mod serde_bytes_compat {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        // Swift `Data` Codable = array of bytes.
        bytes.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        Vec::<u8>::deserialize(d)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileAccessLease {
    pub id: uuid::Uuid,
    pub url: PathBuf,
    pub access: FileCapabilityAccess,
}

/// Security-scoped-capability equivalent. On Linux the broker issues
/// canonical-path bookmarks; access enforcement is a no-op contract.
pub trait FileCapabilityBroker: Send + Sync {
    fn issue_capability(
        &self,
        url: &std::path::Path,
        access: FileCapabilityAccess,
    ) -> Result<FileCapability, PlatformPortError>;
    fn begin_access(
        &self,
        capability: &FileCapability,
    ) -> Result<FileAccessLease, PlatformPortError>;
    fn end_access(&self, lease: FileAccessLease) -> Result<(), PlatformPortError>;
}

#[derive(Debug, Clone)]
pub struct ProcessRequest {
    pub id: uuid::Uuid,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub working_directory: Option<PathBuf>,
    pub standard_input: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessResult {
    pub termination_status: i32,
    pub standard_output: Vec<u8>,
    pub standard_error: Vec<u8>,
}

pub trait ProcessExecuting: Send + Sync {
    fn execute(&self, request: &ProcessRequest) -> Result<ProcessResult, PlatformPortError>;
    fn terminate(&self, id: uuid::Uuid) -> Result<(), PlatformPortError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PDFPoint {
    pub x: f64,
    pub y: f64,
}
impl Eq for PDFPoint {}
impl std::hash::Hash for PDFPoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PDFRectangle {
    pub origin: PDFPoint,
    pub width: f64,
    pub height: f64,
}
impl Eq for PDFRectangle {}
impl std::hash::Hash for PDFRectangle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.origin.hash(state);
        self.width.to_bits().hash(state);
        self.height.to_bits().hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PDFCoordinateSpace {
    PdfBottomLeft { page: i64, media_box: PDFRectangle },
    ViewTopLeft { page: i64, bounds: PDFRectangle },
}

pub trait PDFCoordinateConverting: Send + Sync {
    fn convert(
        &self,
        point: PDFPoint,
        from: PDFCoordinateSpace,
        to: PDFCoordinateSpace,
    ) -> Result<PDFPoint, PlatformPortError>;
}

pub trait WorkspaceOpening: Send + Sync {
    fn open_document(&self, url: &std::path::Path) -> Result<(), PlatformPortError>;
    fn reveal_in_file_manager(&self, urls: &[PathBuf]) -> Result<(), PlatformPortError>;
}

pub trait WallClock: Send + Sync {
    fn now_milliseconds(&self) -> u64;
}

pub trait UuidGenerating: Send + Sync {
    fn make_uuid(&self) -> uuid::Uuid;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Error,
    Fault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub level: LogLevel,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

pub trait ApplicationLogging: Send + Sync {
    fn log(&self, record: &LogRecord) -> Result<(), PlatformPortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSnapshot {
    pub revision: u64,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentTextRange {
    pub location: usize,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMutation {
    pub base_revision: u64,
    pub range: DocumentTextRange,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentMutationResult {
    Applied(DocumentSnapshot),
    Rejected { current: DocumentSnapshot },
}

/// UTF-16-range mutation port owned by the editor adapter.
pub trait DocumentSessionPort: Send + Sync {
    fn snapshot(&self) -> DocumentSnapshot;
    fn submit(
        &self,
        mutation: &DocumentMutation,
    ) -> Result<DocumentMutationResult, PlatformPortError>;
}
