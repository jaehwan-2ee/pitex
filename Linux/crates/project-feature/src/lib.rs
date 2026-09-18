//! Rust port of `Packages/TexApp/Sources/ProjectFeature`.

use app_ports::FileCapability;
use document_session_core::DocumentSnapshot;
use document_session_core::DocumentSaveState;
use project_core::{ProjectFile, ProjectGraph};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ProjectPermissionState {
    NotRequested,
    Requesting,
    Granted(FileCapability),
    Denied { reason: String },
    CapabilityExpired,
}

#[derive(Debug, Clone)]
pub enum ProjectOpenState {
    Closed,
    AwaitingPermission,
    Opening,
    Open(ProjectGraph),
    Failed(ProjectOpenError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectOpenError {
    PermissionDenied(String),
    InvalidCapability,
    ProjectReadFailed(String),
    DocumentNotInProject(ProjectFile),
    DocumentRouteMismatch,
    StaleDocumentSnapshot { current_revision: u64, received_revision: u64 },
}
impl fmt::Display for ProjectOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProjectOpenError {}

/// Presentation-only routing metadata. The durable `DocumentSession` remains
/// owned by the core layer.
#[derive(Debug, Clone)]
pub struct DocumentSessionRoute {
    pub file: ProjectFile,
    presented_revision: Option<u64>,
    save_state: Option<DocumentSaveState>,
}
impl DocumentSessionRoute {
    pub fn new(file: ProjectFile) -> Self {
        Self { file, presented_revision: None, save_state: None }
    }
    pub fn presented_revision(&self) -> Option<u64> {
        self.presented_revision
    }
    pub fn save_state(&self) -> Option<DocumentSaveState> {
        self.save_state
    }
    pub fn present(&mut self, snapshot: &DocumentSnapshot) -> Result<(), ProjectOpenError> {
        if snapshot.document_id != self.file.document_id || snapshot.path != self.file.path {
            return Err(ProjectOpenError::DocumentRouteMismatch);
        }
        if let Some(current) = self.presented_revision {
            if snapshot.revision < current {
                return Err(ProjectOpenError::StaleDocumentSnapshot {
                    current_revision: current,
                    received_revision: snapshot.revision,
                });
            }
        }
        self.presented_revision = Some(snapshot.revision);
        self.save_state = Some(snapshot.save_state);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProjectTab {
    pub route: DocumentSessionRoute,
}
impl ProjectTab {
    pub fn new(route: DocumentSessionRoute) -> Self {
        Self { route }
    }
    pub fn present(&mut self, snapshot: &DocumentSnapshot) -> Result<(), ProjectOpenError> {
        self.route.present(snapshot)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentOpenDisposition {
    Opened { index: usize },
    FocusedExisting { index: usize },
}

#[derive(Debug, Clone)]
pub struct ProjectFeatureState {
    permission: ProjectPermissionState,
    open_state: ProjectOpenState,
    tabs: Vec<ProjectTab>,
    selected_tab_index: Option<usize>,
}
impl Default for ProjectFeatureState {
    fn default() -> Self {
        Self::new()
    }
}
impl ProjectFeatureState {
    pub fn new() -> Self {
        Self {
            permission: ProjectPermissionState::NotRequested,
            open_state: ProjectOpenState::Closed,
            tabs: Vec::new(),
            selected_tab_index: None,
        }
    }
    pub fn permission(&self) -> &ProjectPermissionState {
        &self.permission
    }
    pub fn open_state(&self) -> &ProjectOpenState {
        &self.open_state
    }
    pub fn tabs(&self) -> &[ProjectTab] {
        &self.tabs
    }
    pub fn selected_tab_index(&self) -> Option<usize> {
        self.selected_tab_index
    }

    pub fn begin_permission_request(&mut self) {
        self.permission = ProjectPermissionState::Requesting;
        self.open_state = ProjectOpenState::AwaitingPermission;
    }
    pub fn receive_permission(&mut self, capability: FileCapability) {
        self.permission = ProjectPermissionState::Granted(capability);
        self.open_state = ProjectOpenState::Opening;
    }
    pub fn deny_permission(&mut self, reason: &str) {
        self.permission = ProjectPermissionState::Denied { reason: reason.to_string() };
        self.open_state = ProjectOpenState::Failed(ProjectOpenError::PermissionDenied(
            reason.to_string(),
        ));
    }
    pub fn expire_capability(&mut self) {
        self.permission = ProjectPermissionState::CapabilityExpired;
        self.open_state = ProjectOpenState::Failed(ProjectOpenError::InvalidCapability);
        self.tabs.clear();
        self.selected_tab_index = None;
    }
    pub fn finish_opening(&mut self, graph: ProjectGraph) -> Result<(), ProjectOpenError> {
        if !matches!(self.permission, ProjectPermissionState::Granted(_)) {
            self.open_state = ProjectOpenState::Failed(ProjectOpenError::InvalidCapability);
            return Err(ProjectOpenError::InvalidCapability);
        }
        self.open_state = ProjectOpenState::Open(graph);
        self.tabs.clear();
        self.selected_tab_index = None;
        Ok(())
    }
    pub fn fail_opening(&mut self, reason: &str) {
        self.open_state =
            ProjectOpenState::Failed(ProjectOpenError::ProjectReadFailed(reason.to_string()));
    }

    pub fn open_document(
        &mut self,
        file: ProjectFile,
    ) -> Result<DocumentOpenDisposition, ProjectOpenError> {
        match &self.open_state {
            ProjectOpenState::Open(graph) if graph.files.contains(&file) => {}
            _ => {
                return Err(ProjectOpenError::DocumentNotInProject(file));
            }
        }
        if let Some(existing) = self
            .tabs
            .iter()
            .position(|t| t.route.file.document_id == file.document_id || t.route.file.path == file.path)
        {
            self.selected_tab_index = Some(existing);
            return Ok(DocumentOpenDisposition::FocusedExisting { index: existing });
        }
        self.tabs.push(ProjectTab::new(DocumentSessionRoute::new(file)));
        let index = self.tabs.len() - 1;
        self.selected_tab_index = Some(index);
        Ok(DocumentOpenDisposition::Opened { index })
    }

    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.selected_tab_index = None;
        } else if let Some(selected) = self.selected_tab_index {
            self.selected_tab_index =
                Some((if selected > index { selected - 1 } else { selected })
                    .min(self.tabs.len() - 1));
        }
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.selected_tab_index = Some(index);
        }
    }

    pub fn present(
        &mut self,
        snapshot: &DocumentSnapshot,
        tab_index: usize,
    ) -> Result<(), ProjectOpenError> {
        match self.tabs.get_mut(tab_index) {
            Some(tab) => tab.present(snapshot),
            None => Err(ProjectOpenError::DocumentRouteMismatch),
        }
    }
}

/// One node of the hierarchical project file list. `path` is the
/// project-relative path ("/" separated); directories carry their children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFileNode {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub children: Option<Vec<ProjectFileNode>>,
}

/// Builds the directory tree shown in the project sidebar from flat
/// project-relative paths. Directories sort before files; siblings order
/// lexicographically by name so both platforms render identically.
pub fn build_project_file_tree(relative_paths: &[String]) -> Vec<ProjectFileNode> {
    let mut roots: Vec<ProjectFileNode> = Vec::new();
    for path in relative_paths {
        let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
        if components.is_empty() {
            continue;
        }
        insert(&components, "", &mut roots);
    }
    sort_nodes(&mut roots);
    roots
}

fn insert(components: &[&str], prefix: &str, nodes: &mut Vec<ProjectFileNode>) {
    let Some((&head, rest)) = components.split_first() else { return };
    let node_path = if prefix.is_empty() {
        head.to_string()
    } else {
        format!("{prefix}/{head}")
    };
    if rest.is_empty() {
        nodes.push(ProjectFileNode {
            path: node_path,
            name: head.to_string(),
            is_directory: false,
            children: None,
        });
        return;
    }
    if let Some(node) = nodes.iter_mut().find(|n| n.is_directory && n.name == head) {
        insert(rest, &node_path, node.children.get_or_insert_with(Vec::new));
    } else {
        let mut children = Vec::new();
        insert(rest, &node_path, &mut children);
        nodes.push(ProjectFileNode {
            path: node_path,
            name: head.to_string(),
            is_directory: true,
            children: Some(children),
        });
    }
}

fn sort_nodes(nodes: &mut Vec<ProjectFileNode>) {
    nodes.sort_by(|a, b| (b.is_directory, a.name.to_lowercase()).cmp(&(a.is_directory, b.name.to_lowercase())));
    for node in nodes.iter_mut() {
        if let Some(children) = node.children.as_mut() {
            sort_nodes(children);
        }
    }
}
