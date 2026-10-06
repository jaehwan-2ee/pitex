//! Port of `Packages/TexCore/Sources/ProjectCore` — project graph, workspace
//! records, tabs, and restoration diagnostics.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use tex_domain::{NormalizedRelativePath, StableDocumentID, StableProjectID};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectFile {
    #[serde(rename = "documentID")]
    pub document_id: StableDocumentID,
    pub path: NormalizedRelativePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IncludeEdge {
    pub source: StableDocumentID,
    pub target: StableDocumentID,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectGraphConstructionError {
    DuplicateDocumentID(StableDocumentID),
    DuplicatePath(NormalizedRelativePath),
}

impl fmt::Display for ProjectGraphConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDocumentID(id) => write!(f, "duplicate document id {id}"),
            Self::DuplicatePath(p) => write!(f, "duplicate path {p}"),
        }
    }
}
impl std::error::Error for ProjectGraphConstructionError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectGraphDiagnostic {
    MissingReference {
        source: StableDocumentID,
        target: StableDocumentID,
        missing: StableDocumentID,
    },
    IncludeCycle {
        documents: Vec<StableDocumentID>,
    },
}

fn diagnostic_sort_key(d: &ProjectGraphDiagnostic) -> String {
    match d {
        ProjectGraphDiagnostic::MissingReference {
            source,
            target,
            missing,
        } => format!("0|{}|{}|{}", source.raw_value(), target.raw_value(), missing.raw_value()),
        ProjectGraphDiagnostic::IncludeCycle { documents } => {
            let joined = documents
                .iter()
                .map(|d| d.raw_value())
                .collect::<Vec<_>>()
                .join("|");
            format!("1|{joined}")
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectGraph {
    pub project_id: StableProjectID,
    pub files: Vec<ProjectFile>,
    pub include_edges: Vec<IncludeEdge>,
}

impl ProjectGraph {
    pub fn new(
        project_id: StableProjectID,
        files: Vec<ProjectFile>,
        include_edges: Vec<IncludeEdge>,
    ) -> Result<Self, ProjectGraphConstructionError> {
        let mut sorted_files = files;
        sorted_files.sort_by(|a, b| {
            a.document_id
                .cmp(&b.document_id)
                .then(a.path.cmp(&b.path))
        });
        let mut document_ids: HashSet<StableDocumentID> = HashSet::new();
        let mut paths: HashSet<NormalizedRelativePath> = HashSet::new();
        for file in &sorted_files {
            if !document_ids.insert(file.document_id.clone()) {
                return Err(ProjectGraphConstructionError::DuplicateDocumentID(
                    file.document_id.clone(),
                ));
            }
            if !paths.insert(file.path.clone()) {
                return Err(ProjectGraphConstructionError::DuplicatePath(
                    file.path.clone(),
                ));
            }
        }
        let mut sorted_edges = include_edges;
        sorted_edges.sort_by(|a, b| a.source.cmp(&b.source).then(a.target.cmp(&b.target)));
        Ok(Self {
            project_id,
            files: sorted_files,
            include_edges: sorted_edges,
        })
    }

    pub fn diagnostics(&self) -> Vec<ProjectGraphDiagnostic> {
        let known_documents: BTreeSet<StableDocumentID> =
            self.files.iter().map(|f| f.document_id.clone()).collect();
        let mut result: Vec<ProjectGraphDiagnostic> = Vec::new();

        for edge in &self.include_edges {
            if !known_documents.contains(&edge.source) {
                result.push(ProjectGraphDiagnostic::MissingReference {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    missing: edge.source.clone(),
                });
            }
            if !known_documents.contains(&edge.target) {
                result.push(ProjectGraphDiagnostic::MissingReference {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    missing: edge.target.clone(),
                });
            }
        }

        let mut adjacency: BTreeMap<StableDocumentID, Vec<StableDocumentID>> = BTreeMap::new();
        let mut reverse_adjacency: BTreeMap<StableDocumentID, Vec<StableDocumentID>> =
            BTreeMap::new();
        for document in &known_documents {
            adjacency.insert(document.clone(), Vec::new());
            reverse_adjacency.insert(document.clone(), Vec::new());
        }
        for edge in &self.include_edges {
            if known_documents.contains(&edge.source) && known_documents.contains(&edge.target) {
                adjacency
                    .entry(edge.source.clone())
                    .or_default()
                    .push(edge.target.clone());
                reverse_adjacency
                    .entry(edge.target.clone())
                    .or_default()
                    .push(edge.source.clone());
            }
        }
        for targets in adjacency.values_mut() {
            targets.sort();
        }
        for sources in reverse_adjacency.values_mut() {
            sources.sort();
        }

        // Kosaraju SCC — same visitation order as the Swift implementation.
        let mut visited: HashSet<StableDocumentID> = HashSet::new();
        let mut finish_order: Vec<StableDocumentID> = Vec::new();
        fn visit_forward(
            document: &StableDocumentID,
            adjacency: &BTreeMap<StableDocumentID, Vec<StableDocumentID>>,
            visited: &mut HashSet<StableDocumentID>,
            finish_order: &mut Vec<StableDocumentID>,
        ) {
            if !visited.insert(document.clone()) {
                return;
            }
            if let Some(targets) = adjacency.get(document) {
                for target in targets {
                    visit_forward(target, adjacency, visited, finish_order);
                }
            }
            finish_order.push(document.clone());
        }
        for document in &known_documents {
            visit_forward(document, &adjacency, &mut visited, &mut finish_order);
        }

        visited.clear();
        fn visit_reverse(
            document: &StableDocumentID,
            reverse: &BTreeMap<StableDocumentID, Vec<StableDocumentID>>,
            visited: &mut HashSet<StableDocumentID>,
            component: &mut Vec<StableDocumentID>,
        ) {
            if !visited.insert(document.clone()) {
                return;
            }
            component.push(document.clone());
            if let Some(sources) = reverse.get(document) {
                for source in sources {
                    visit_reverse(source, reverse, visited, component);
                }
            }
        }
        for document in finish_order.iter().rev() {
            if visited.contains(document) {
                continue;
            }
            let mut component = Vec::new();
            visit_reverse(
                document,
                &reverse_adjacency,
                &mut visited,
                &mut component,
            );
            component.sort();
            let is_self_cycle = component.len() == 1
                && adjacency
                    .get(&component[0])
                    .map(|t| t.contains(&component[0]))
                    .unwrap_or(false);
            if component.len() > 1 || is_self_cycle {
                result.push(ProjectGraphDiagnostic::IncludeCycle {
                    documents: component,
                });
            }
        }

        result.sort_by_key(diagnostic_sort_key);
        result
    }
}

// ---------------------------------------------------------------------------
// ProjectWorkspace

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectRootIdentity {
    #[serde(rename = "projectID")]
    pub project_id: StableProjectID,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectGraphSnapshot {
    pub files: Vec<ProjectFile>,
    pub include_edges: Vec<IncludeEdge>,
}

// Custom Serialize/Deserialize so construction validation runs on decode,
// exactly like the Swift `init(from decoder:)`.
impl Serialize for ProjectGraphSnapshot {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Raw<'a> {
            files: &'a [ProjectFile],
            #[serde(rename = "includeEdges")]
            include_edges: &'a [IncludeEdge],
        }
        Raw {
            files: &self.files,
            include_edges: &self.include_edges,
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for ProjectGraphSnapshot {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            files: Vec<ProjectFile>,
            #[serde(rename = "includeEdges")]
            include_edges: Vec<IncludeEdge>,
        }
        let raw = Raw::deserialize(d)?;
        Self::new(raw.files, raw.include_edges).map_err(serde::de::Error::custom)
    }
}

impl ProjectGraphSnapshot {
    pub fn new(
        files: Vec<ProjectFile>,
        include_edges: Vec<IncludeEdge>,
    ) -> Result<Self, ProjectWorkspaceError> {
        let mut sorted_files = files;
        sorted_files.sort_by(|a, b| {
            a.document_id
                .cmp(&b.document_id)
                .then(a.path.cmp(&b.path))
        });
        let mut document_ids: HashSet<StableDocumentID> = HashSet::new();
        let mut paths: HashSet<NormalizedRelativePath> = HashSet::new();
        for file in &sorted_files {
            if !document_ids.insert(file.document_id.clone()) {
                return Err(ProjectWorkspaceError::DuplicateDocumentID(
                    file.document_id.clone(),
                ));
            }
            if !paths.insert(file.path.clone()) {
                return Err(ProjectWorkspaceError::DuplicatePath(file.path.clone()));
            }
        }
        let mut sorted_edges = include_edges;
        sorted_edges.sort_by(|a, b| a.source.cmp(&b.source).then(a.target.cmp(&b.target)));
        Ok(Self {
            files: sorted_files,
            include_edges: sorted_edges,
        })
    }

    pub fn from_graph(graph: &ProjectGraph) -> Result<Self, ProjectWorkspaceError> {
        Self::new(graph.files.clone(), graph.include_edges.clone())
    }

    pub fn project_graph(
        &self,
        project_id: StableProjectID,
    ) -> Result<ProjectGraph, ProjectGraphConstructionError> {
        ProjectGraph::new(project_id, self.files.clone(), self.include_edges.clone())
    }

    pub fn diagnostics(
        &self,
        project_id: StableProjectID,
    ) -> Result<Vec<ProjectGraphDiagnostic>, ProjectGraphConstructionError> {
        Ok(self.project_graph(project_id)?.diagnostics())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildCommandPreference {
    pub executable: String,
    pub arguments: Vec<String>,
}

impl BuildCommandPreference {
    pub fn new(
        executable: impl Into<String>,
        arguments: Vec<String>,
    ) -> Result<Self, ProjectWorkspaceError> {
        let executable = executable.into();
        if executable.is_empty() {
            return Err(ProjectWorkspaceError::EmptyBuildCommand);
        }
        Ok(Self {
            executable,
            arguments,
        })
    }
}

impl Serialize for BuildCommandPreference {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Raw<'a> {
            executable: &'a str,
            arguments: &'a [String],
        }
        Raw {
            executable: &self.executable,
            arguments: &self.arguments,
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for BuildCommandPreference {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            executable: String,
            arguments: Vec<String>,
        }
        let raw = Raw::deserialize(d)?;
        Self::new(raw.executable, raw.arguments).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectBuildTarget {
    pub id: String,
    pub document_id: StableDocumentID,
    pub command: BuildCommandPreference,
}

impl ProjectBuildTarget {
    pub fn new(
        id: impl Into<String>,
        document_id: StableDocumentID,
        command: BuildCommandPreference,
    ) -> Result<Self, ProjectWorkspaceError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ProjectWorkspaceError::EmptyBuildTargetID);
        }
        Ok(Self {
            id,
            document_id,
            command,
        })
    }
}

impl Serialize for ProjectBuildTarget {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Raw<'a> {
            id: &'a str,
            #[serde(rename = "documentID")]
            document_id: &'a StableDocumentID,
            command: &'a BuildCommandPreference,
        }
        Raw {
            id: &self.id,
            document_id: &self.document_id,
            command: &self.command,
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ProjectBuildTarget {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            id: String,
            #[serde(rename = "documentID")]
            document_id: StableDocumentID,
            command: BuildCommandPreference,
        }
        let raw = Raw::deserialize(d)?;
        Self::new(raw.id, raw.document_id, raw.command).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalRevisionMarker {
    pub document_id: StableDocumentID,
    pub revision: String,
}

impl ExternalRevisionMarker {
    pub fn new(
        document_id: StableDocumentID,
        revision: impl Into<String>,
    ) -> Result<Self, ProjectWorkspaceError> {
        let revision = revision.into();
        if revision.is_empty() {
            return Err(ProjectWorkspaceError::EmptyExternalRevision);
        }
        Ok(Self {
            document_id,
            revision,
        })
    }
}

impl Serialize for ExternalRevisionMarker {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Raw<'a> {
            #[serde(rename = "documentID")]
            document_id: &'a StableDocumentID,
            revision: &'a str,
        }
        Raw {
            document_id: &self.document_id,
            revision: &self.revision,
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ExternalRevisionMarker {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "documentID")]
            document_id: StableDocumentID,
            revision: String,
        }
        let raw = Raw::deserialize(d)?;
        Self::new(raw.document_id, raw.revision).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectWorkspaceError {
    UnsupportedSchemaVersion { found: i64, supported: i64 },
    DuplicateDocumentID(StableDocumentID),
    DuplicatePath(NormalizedRelativePath),
    DuplicateTab(StableDocumentID),
    TabOutsideGraph(StableDocumentID),
    InvalidActiveTab(StableDocumentID),
    DuplicateBuildTarget(String),
    UnknownBuildTarget(String),
    BuildTargetOutsideGraph(StableDocumentID),
    DuplicateExternalRevision(StableDocumentID),
    ExternalRevisionOutsideGraph(StableDocumentID),
    EmptyBuildTargetID,
    EmptyBuildCommand,
    EmptyExternalRevision,
}

impl fmt::Display for ProjectWorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProjectWorkspaceError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectWorkspaceRecord {
    pub schema_version: i64,
    pub root: ProjectRootIdentity,
    pub graph: ProjectGraphSnapshot,
    pub tabs: Vec<StableDocumentID>,
    pub active_tab: Option<StableDocumentID>,
    pub build_targets: Vec<ProjectBuildTarget>,
    pub selected_build_target_id: Option<String>,
    pub external_revisions: Vec<ExternalRevisionMarker>,
}

impl ProjectWorkspaceRecord {
    pub const CURRENT_SCHEMA_VERSION: i64 = 1;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schema_version: i64,
        root: ProjectRootIdentity,
        graph: ProjectGraphSnapshot,
        tabs: Vec<StableDocumentID>,
        active_tab: Option<StableDocumentID>,
        build_targets: Vec<ProjectBuildTarget>,
        selected_build_target_id: Option<String>,
        external_revisions: Vec<ExternalRevisionMarker>,
    ) -> Result<Self, ProjectWorkspaceError> {
        if schema_version != Self::CURRENT_SCHEMA_VERSION {
            return Err(ProjectWorkspaceError::UnsupportedSchemaVersion {
                found: schema_version,
                supported: Self::CURRENT_SCHEMA_VERSION,
            });
        }
        let known_documents: HashSet<StableDocumentID> =
            graph.files.iter().map(|f| f.document_id.clone()).collect();
        let mut tab_set: HashSet<StableDocumentID> = HashSet::new();
        for tab in &tabs {
            if !known_documents.contains(tab) {
                return Err(ProjectWorkspaceError::TabOutsideGraph(tab.clone()));
            }
            if !tab_set.insert(tab.clone()) {
                return Err(ProjectWorkspaceError::DuplicateTab(tab.clone()));
            }
        }
        if let Some(active) = &active_tab {
            if !tab_set.contains(active) {
                return Err(ProjectWorkspaceError::InvalidActiveTab(active.clone()));
            }
        }

        let mut sorted_targets = build_targets;
        sorted_targets.sort_by(|a, b| a.id.cmp(&b.id));
        let mut target_ids: HashSet<String> = HashSet::new();
        for target in &sorted_targets {
            if !target_ids.insert(target.id.clone()) {
                return Err(ProjectWorkspaceError::DuplicateBuildTarget(
                    target.id.clone(),
                ));
            }
            if !known_documents.contains(&target.document_id) {
                return Err(ProjectWorkspaceError::BuildTargetOutsideGraph(
                    target.document_id.clone(),
                ));
            }
        }
        if let Some(selected) = &selected_build_target_id {
            if !target_ids.contains(selected) {
                return Err(ProjectWorkspaceError::UnknownBuildTarget(
                    selected.clone(),
                ));
            }
        }

        let mut sorted_revisions = external_revisions;
        sorted_revisions.sort_by(|a, b| a.document_id.cmp(&b.document_id));
        let mut revision_documents: HashSet<StableDocumentID> = HashSet::new();
        for marker in &sorted_revisions {
            if !known_documents.contains(&marker.document_id) {
                return Err(ProjectWorkspaceError::ExternalRevisionOutsideGraph(
                    marker.document_id.clone(),
                ));
            }
            if !revision_documents.insert(marker.document_id.clone()) {
                return Err(ProjectWorkspaceError::DuplicateExternalRevision(
                    marker.document_id.clone(),
                ));
            }
        }

        Ok(Self {
            schema_version,
            root,
            graph,
            tabs,
            active_tab,
            build_targets: sorted_targets,
            selected_build_target_id,
            external_revisions: sorted_revisions,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn current(
        root: ProjectRootIdentity,
        graph: ProjectGraphSnapshot,
        tabs: Vec<StableDocumentID>,
        active_tab: Option<StableDocumentID>,
        build_targets: Vec<ProjectBuildTarget>,
        selected_build_target_id: Option<String>,
        external_revisions: Vec<ExternalRevisionMarker>,
    ) -> Result<Self, ProjectWorkspaceError> {
        Self::new(
            Self::CURRENT_SCHEMA_VERSION,
            root,
            graph,
            tabs,
            active_tab,
            build_targets,
            selected_build_target_id,
            external_revisions,
        )
    }
}

impl Serialize for ProjectWorkspaceRecord {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Raw<'a> {
            #[serde(rename = "schemaVersion")]
            schema_version: i64,
            root: &'a ProjectRootIdentity,
            graph: &'a ProjectGraphSnapshot,
            tabs: &'a [StableDocumentID],
            #[serde(rename = "activeTab", skip_serializing_if = "Option::is_none")]
            active_tab: &'a Option<StableDocumentID>,
            #[serde(rename = "buildTargets")]
            build_targets: &'a [ProjectBuildTarget],
            #[serde(
                rename = "selectedBuildTargetID",
                skip_serializing_if = "Option::is_none"
            )]
            selected_build_target_id: &'a Option<String>,
            #[serde(rename = "externalRevisions")]
            external_revisions: &'a [ExternalRevisionMarker],
        }
        Raw {
            schema_version: self.schema_version,
            root: &self.root,
            graph: &self.graph,
            tabs: &self.tabs,
            active_tab: &self.active_tab,
            build_targets: &self.build_targets,
            selected_build_target_id: &self.selected_build_target_id,
            external_revisions: &self.external_revisions,
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for ProjectWorkspaceRecord {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "schemaVersion")]
            schema_version: i64,
            root: ProjectRootIdentity,
            graph: ProjectGraphSnapshot,
            tabs: Vec<StableDocumentID>,
            #[serde(rename = "activeTab")]
            active_tab: Option<StableDocumentID>,
            #[serde(rename = "buildTargets")]
            build_targets: Vec<ProjectBuildTarget>,
            #[serde(rename = "selectedBuildTargetID")]
            selected_build_target_id: Option<String>,
            #[serde(rename = "externalRevisions")]
            external_revisions: Vec<ExternalRevisionMarker>,
        }
        let raw = Raw::deserialize(d)?;
        Self::new(
            raw.schema_version,
            raw.root,
            raw.graph,
            raw.tabs,
            raw.active_tab,
            raw.build_targets,
            raw.selected_build_target_id,
            raw.external_revisions,
        )
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectWorkspaceRestorationDiagnostic {
    MissingDocument {
        document_id: StableDocumentID,
        path: NormalizedRelativePath,
    },
}

pub struct ProjectWorkspaceRestoration {
    pub workspace: ProjectWorkspace,
    pub persisted_record: ProjectWorkspaceRecord,
    pub diagnostics: Vec<ProjectWorkspaceRestorationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspace {
    pub record: ProjectWorkspaceRecord,
}

impl ProjectWorkspace {
    pub fn new(record: ProjectWorkspaceRecord) -> Self {
        Self { record }
    }

    pub fn open(&mut self, document_id: &StableDocumentID) -> Result<(), ProjectWorkspaceError> {
        if !self
            .record
            .graph
            .files
            .iter()
            .any(|f| &f.document_id == document_id)
        {
            return Err(ProjectWorkspaceError::TabOutsideGraph(document_id.clone()));
        }
        let mut tabs = self.record.tabs.clone();
        if !tabs.contains(document_id) {
            tabs.push(document_id.clone());
        }
        self.replace(Some(tabs), Some(Some(document_id.clone())), None)
    }

    pub fn close(&mut self, document_id: &StableDocumentID) -> Result<(), ProjectWorkspaceError> {
        let mut tabs = self.record.tabs.clone();
        let Some(index) = tabs.iter().position(|t| t == document_id) else {
            return Ok(());
        };
        tabs.remove(index);
        let mut active = self.record.active_tab.clone();
        if active.as_ref() == Some(document_id) {
            active = if tabs.is_empty() {
                None
            } else {
                Some(tabs[index.min(tabs.len() - 1)].clone())
            };
        }
        self.replace(Some(tabs), Some(active), None)
    }

    pub fn activate(&mut self, document_id: &StableDocumentID) -> Result<(), ProjectWorkspaceError> {
        if !self.record.tabs.contains(document_id) {
            return Err(ProjectWorkspaceError::InvalidActiveTab(document_id.clone()));
        }
        self.replace(None, Some(Some(document_id.clone())), None)
    }

    pub fn select_build_target(&mut self, id: &str) -> Result<(), ProjectWorkspaceError> {
        if !self.record.build_targets.iter().any(|t| t.id == id) {
            return Err(ProjectWorkspaceError::UnknownBuildTarget(id.to_string()));
        }
        self.replace(None, None, Some(Some(id.to_string())))
    }

    pub fn restore(
        persisted_record: ProjectWorkspaceRecord,
        available_paths: &HashSet<NormalizedRelativePath>,
    ) -> ProjectWorkspaceRestoration {
        let diagnostics = persisted_record
            .graph
            .files
            .iter()
            .filter(|file| !available_paths.contains(&file.path))
            .map(|file| ProjectWorkspaceRestorationDiagnostic::MissingDocument {
                document_id: file.document_id.clone(),
                path: file.path.clone(),
            })
            .collect();
        ProjectWorkspaceRestoration {
            workspace: ProjectWorkspace::new(persisted_record.clone()),
            persisted_record,
            diagnostics,
        }
    }

    fn replace(
        &mut self,
        tabs: Option<Vec<StableDocumentID>>,
        active_tab: Option<Option<StableDocumentID>>,
        selected_build_target_id: Option<Option<String>>,
    ) -> Result<(), ProjectWorkspaceError> {
        self.record = ProjectWorkspaceRecord::current(
            self.record.root.clone(),
            self.record.graph.clone(),
            tabs.unwrap_or_else(|| self.record.tabs.clone()),
            active_tab.unwrap_or_else(|| self.record.active_tab.clone()),
            self.record.build_targets.clone(),
            selected_build_target_id.unwrap_or_else(|| self.record.selected_build_target_id.clone()),
            self.record.external_revisions.clone(),
        )?;
        Ok(())
    }
}


