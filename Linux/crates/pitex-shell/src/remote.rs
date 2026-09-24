//! Port of `RemoteSupport.swift` — the app side of the SSH feature:
//! `RemoteWorkspace` session state, `beginRemoteSession` (called from
//! `open` before any file is read) and the status/recents strings. The
//! `WorkspaceBuildExecutor` routing lives with the model (model.rs).

use remote_core::{
    MirrorWriteGate, MirrorWrites, RemoteBuildExecutor, RemoteMirror, RemoteSync, SshConnection,
};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// `MirrorWriteGate` over the process-wide `MirrorWrites` — Rust has no
/// `&'static` → `Arc` conversion, so the shared gate gets a named shell.
/// Public so tests and embedders can pre-register an engine through
/// `RemoteSync::install` with the same gate the app uses.
pub struct SharedGate;
impl MirrorWriteGate for SharedGate {
    fn begin_sync_commit(&self) {
        MirrorWrites::shared().begin_sync_commit();
    }
    fn end_sync_commit(&self) {
        MirrorWrites::shared().end_sync_commit();
    }
}

/// `RemoteWorkspace.Status` — `offline` carries the last transfer's error.
#[derive(Debug)]
pub enum RemoteStatus {
    Synced,
    Syncing,
    Offline(String),
}

/// A project opened through "Open via SSH": the local mirror the editor
/// works on, its sync engine and the device's build executor.
pub struct RemoteWorkspace {
    pub project: remote_core::RemoteProject,
    pub sync: Arc<RemoteSync>,
    pub executor: Arc<RemoteBuildExecutor>,
    pub status: RemoteStatus,
    /// Files changed both here and on the device since the last sync.
    pub conflicts: Vec<String>,
    /// `lastPull` — `None` until the first successful pull (stale then).
    pub last_pull: Option<Instant>,
}

impl RemoteWorkspace {
    pub fn new(mirror: RemoteMirror) -> Self {
        let sync = RemoteSync::shared(mirror.clone(), Some(Arc::new(SharedGate)));
        let executor = Arc::new(RemoteBuildExecutor::new(sync.clone()));
        Self {
            project: mirror.project.clone(),
            sync,
            executor,
            status: RemoteStatus::Syncing,
            conflicts: Vec::new(),
            last_pull: None,
        }
    }

    pub fn device_name(&self) -> &str {
        &self.project.connection.name
    }
}

/// Outcome of `begin_remote_session` for the open worker.
pub enum RemoteOpen {
    /// `selected` is not inside a mirror — a plain local open.
    NotRemote,
    /// The workspace is ready (synced, or offline with a cached mirror).
    Ready(Box<RemoteWorkspace>),
    /// Nothing cached and the device is unreachable — fail the open.
    Failed { device: String, error: String },
}

/// `beginRemoteSession(for:)` — a mirror path makes this a remote
/// workspace and pulls the device's current state; a failed pull still
/// opens the cached copy (offline) when there is one, and fails the open
/// when there is none.
pub fn begin_remote_session(selected: &Path) -> RemoteOpen {
    let Some(mirror) = RemoteMirror::containing(selected, &RemoteMirror::default_store()) else {
        return RemoteOpen::NotRemote;
    };
    let mut workspace = RemoteWorkspace::new(mirror.clone());
    let cached = std::fs::read_dir(mirror.root())
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    let result = workspace
        .sync
        .pull()
        .map(|_| {
            workspace.last_pull = Some(Instant::now());
        })
        .and_then(|()| workspace.sync.push());
    match result {
        Ok(pushed) => {
            workspace.conflicts = pushed.conflicts;
            workspace.status = RemoteStatus::Synced;
            RemoteOpen::Ready(Box::new(workspace))
        }
        Err(error) => {
            if !cached {
                return RemoteOpen::Failed {
                    device: workspace.project.connection.name.clone(),
                    error: error.to_string(),
                };
            }
            workspace.status = RemoteStatus::Offline(error.to_string());
            RemoteOpen::Ready(Box::new(workspace))
        }
    }
}

/// `summary(_:)` — `user@destination:port` for the connection rows.
pub fn connection_summary(connection: &SshConnection) -> String {
    let mut text = connection.destination.clone();
    if let Some(user) = &connection.user {
        if !user.is_empty() {
            text = format!("{user}@{text}");
        }
    }
    if let Some(port) = connection.port {
        text += &format!(":{port}");
    }
    text
}

/// `remote.statusText` — the window-title subtitle while a remote project
/// is open.
pub fn status_text(language: &str, workspace: &RemoteWorkspace) -> String {
    let device = workspace.device_name();
    if !workspace.conflicts.is_empty() {
        return crate::l10n::trn(
            language,
            "remote.status.conflicts",
            &[device, &workspace.conflicts.len().to_string()],
        );
    }
    match &workspace.status {
        RemoteStatus::Synced => crate::l10n::trn(language, "remote.status.synced", &[device]),
        RemoteStatus::Syncing => crate::l10n::trn(language, "remote.status.syncing", &[device]),
        RemoteStatus::Offline(_) => {
            crate::l10n::trn(language, "remote.status.offline", &[device])
        }
    }
}

/// `recentTitle(for:)` — an Open Recent label naming the device a mirror
/// belongs to.
pub fn recent_title(language: &str, url: &Path) -> String {
    let name = url
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| url.display().to_string());
    match RemoteMirror::containing(url, &RemoteMirror::default_store()) {
        Some(mirror) => crate::l10n::trn(
            language,
            "remote.recent_title",
            &[&name, &mirror.project.connection.name],
        ),
        None => name,
    }
}
