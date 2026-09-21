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

/// `nestProjectChildren` — reorders the built tree so the main document leads
/// the root list and its direct dependencies (bibliographies, included
/// chapters) nest one level beneath it. Missing `main` returns the tree
/// unchanged; directories left empty by a move are pruned.
pub fn nest_project_children(
    mut tree: Vec<ProjectFileNode>,
    main_rel: &str,
    child_rels: &[String],
) -> Vec<ProjectFileNode> {
    let Some(mut main) = remove_node(&mut tree, main_rel) else {
        return tree;
    };
    let mut children = main.children.take().unwrap_or_default();
    for child in child_rels {
        if let Some(node) = remove_node(&mut tree, child) {
            children.push(node);
        }
    }
    // Same-stem .bib files nest under the main document even when dependency
    // resolution missed them (indirect includes, stale children list).
    let main_stem = main.name.rsplit_once('.').map(|(s, _)| s).unwrap_or(&main.name).to_string();
    for bib_path in bib_paths(&main_stem, &tree) {
        if !children.iter().any(|n| n.path == bib_path) {
            if let Some(node) = remove_node(&mut tree, &bib_path) {
                children.push(node);
            }
        }
    }
    main.children = (!children.is_empty()).then_some(children);
    tree.insert(0, main);
    tree
}

/// All .bib file paths in the tree whose stem equals `stem`.
fn bib_paths(stem: &str, nodes: &[ProjectFileNode]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        if node.is_directory {
            out.extend(bib_paths(stem, node.children.as_deref().unwrap_or(&[])));
        } else if node.name.to_lowercase().ends_with(".bib")
            && node.name.rsplit_once('.').map(|(s, _)| s) == Some(stem)
        {
            out.push(node.path.clone());
        }
    }
    out
}

/// `extractOutputPDFs` — pulls the build output out of the file tree:
/// `.pdf` files whose stem matches the main document's (`manuscript.tex` →
/// `manuscript.pdf`, wherever the build wrote it). The sidebar pins them
/// below the tree — artifacts, not sources. Empty stem → no-op.
pub fn extract_output_pdfs(
    mut tree: Vec<ProjectFileNode>,
    main_stem: &str,
) -> (Vec<ProjectFileNode>, Vec<ProjectFileNode>) {
    if main_stem.is_empty() {
        return (tree, Vec::new());
    }
    let mut outputs = Vec::new();
    for path in pdf_paths(main_stem, &tree) {
        if let Some(node) = remove_node(&mut tree, &path) {
            outputs.push(node);
        }
    }
    (tree, outputs)
}

/// All .pdf paths in the tree whose stem equals `stem`.
fn pdf_paths(stem: &str, nodes: &[ProjectFileNode]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        if node.is_directory {
            out.extend(pdf_paths(stem, node.children.as_deref().unwrap_or(&[])));
        } else if node.name.to_lowercase().ends_with(".pdf")
            && node.name.rsplit_once('.').map(|(s, _)| s) == Some(stem)
        {
            out.push(node.path.clone());
        }
    }
    out
}

/// Detaches the node with `path` wherever it sits, pruning directory nodes
/// left empty by the removal.
fn remove_node(nodes: &mut Vec<ProjectFileNode>, path: &str) -> Option<ProjectFileNode> {
    if let Some(pos) = nodes.iter().position(|n| n.path == path) {
        return Some(nodes.remove(pos));
    }
    for i in 0..nodes.len() {
        let found = nodes[i]
            .children
            .as_mut()
            .and_then(|c| remove_node(c, path));
        if found.is_some() {
            if nodes[i]
                .children
                .as_ref()
                .map(|c| c.is_empty())
                .unwrap_or(false)
            {
                nodes.remove(i);
            }
            return found;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(nodes: &[ProjectFileNode]) -> Vec<&str> {
        nodes.iter().map(|n| n.path.as_str()).collect()
    }

    #[test]
    fn nest_project_children_moves_main_first_and_nests_children() {
        let tree = build_project_file_tree(&[
            "appendix.tex".into(),
            "chapters/one.tex".into(),
            "main.tex".into(),
            "refs.bib".into(),
        ]);
        let nested = nest_project_children(
            tree,
            "main.tex",
            &["refs.bib".into(), "chapters/one.tex".into()],
        );
        // Main leads the root list; unrelated files keep their place.
        assert_eq!(paths(&nested), ["main.tex", "appendix.tex"]);
        let main = &nested[0];
        assert_eq!(
            paths(main.children.as_deref().unwrap()),
            ["refs.bib", "chapters/one.tex"]
        );
        // The directory emptied by the move is pruned.
        assert!(nested.iter().all(|n| n.path != "chapters"));
    }

    #[test]
    fn nest_project_children_nests_same_stem_bib_without_dependency() {
        let tree = build_project_file_tree(&[
            "manuscript.bib".into(),
            "manuscript.tex".into(),
            "other.bib".into(),
        ]);
        // No dependency list — the same-stem .bib still nests under the main
        // document; differently-named .bib files stay at the root.
        let nested = nest_project_children(tree, "manuscript.tex", &[]);
        assert_eq!(paths(&nested), ["manuscript.tex", "other.bib"]);
        assert_eq!(
            paths(nested[0].children.as_deref().unwrap()),
            ["manuscript.bib"]
        );
    }

    #[test]
    fn nest_project_children_without_main_is_noop() {
        let tree = build_project_file_tree(&["a.tex".into(), "b.bib".into()]);
        let nested = nest_project_children(tree.clone(), "main.tex", &["b.bib".into()]);
        assert_eq!(nested, tree);
    }

    fn all_paths(nodes: &[ProjectFileNode]) -> Vec<String> {
        nodes
            .iter()
            .flat_map(|n| {
                let mut v = vec![n.path.clone()];
                v.extend(all_paths(n.children.as_deref().unwrap_or(&[])));
                v
            })
            .collect()
    }

    #[test]
    fn extract_output_pdfs_pulls_main_stem_pdf_at_any_depth() {
        let tree = build_project_file_tree(&[
            "main.tex".into(),
            "main.pdf".into(),
            "figures/diagram.pdf".into(),
            "build/main.pdf".into(),
            "refs.bib".into(),
        ]);
        let (filtered, outputs) = extract_output_pdfs(tree, "main");
        let mut out: Vec<&str> = outputs.iter().map(|n| n.path.as_str()).collect();
        out.sort();
        assert_eq!(out, ["build/main.pdf", "main.pdf"]);
        let remaining = all_paths(&filtered);
        assert!(!remaining.iter().any(|p| p == "main.pdf" || p == "build/main.pdf"));
        assert!(remaining.iter().any(|p| p == "main.tex"));
        assert!(remaining.iter().any(|p| p == "figures/diagram.pdf"));
        // A directory emptied by the extraction is pruned, not left hollow.
        assert!(!filtered.iter().any(|n| n.name == "build"));
    }

    #[test]
    fn extract_output_pdfs_without_main_is_noop() {
        let tree = build_project_file_tree(&["main.tex".into(), "main.pdf".into()]);
        let (filtered, outputs) = extract_output_pdfs(tree.clone(), "");
        assert_eq!(filtered, tree);
        assert!(outputs.is_empty());
    }
}
