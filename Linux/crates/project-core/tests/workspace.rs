//! Port of `Packages/TexCore/Tests/TexCoreTests/ProjectWorkspaceTests.swift`.

use project_core::*;
use std::collections::HashSet;
use tex_domain::{NormalizedRelativePath, StableDocumentID, StableProjectID};

fn project_id() -> StableProjectID {
    StableProjectID::new("multifile").unwrap()
}
fn main_id() -> StableDocumentID {
    StableDocumentID::new("main").unwrap()
}
fn intro_id() -> StableDocumentID {
    StableDocumentID::new("intro").unwrap()
}
fn details_id() -> StableDocumentID {
    StableDocumentID::new("details").unwrap()
}
fn main_path() -> NormalizedRelativePath {
    NormalizedRelativePath::new("main.tex").unwrap()
}
fn intro_path() -> NormalizedRelativePath {
    NormalizedRelativePath::new("sections/intro.tex").unwrap()
}
fn details_path() -> NormalizedRelativePath {
    NormalizedRelativePath::new("sections/details.tex").unwrap()
}

fn files() -> Vec<ProjectFile> {
    vec![
        ProjectFile {
            document_id: main_id(),
            path: main_path(),
        },
        ProjectFile {
            document_id: intro_id(),
            path: intro_path(),
        },
        ProjectFile {
            document_id: details_id(),
            path: details_path(),
        },
    ]
}

fn make_record(
    tabs: Vec<StableDocumentID>,
    active: Option<StableDocumentID>,
    selected_build_target_id: Option<String>,
) -> Result<ProjectWorkspaceRecord, ProjectWorkspaceError> {
    let targets = vec![
        ProjectBuildTarget::new(
            "pdf",
            main_id(),
            BuildCommandPreference::new("latexmk", vec!["-pdf".into(), "main.tex".into()])?,
        )?,
        ProjectBuildTarget::new(
            "draft",
            main_id(),
            BuildCommandPreference::new("pdflatex", vec!["-draftmode".into(), "main.tex".into()])?,
        )?,
    ];
    ProjectWorkspaceRecord::current(
        ProjectRootIdentity {
            project_id: project_id(),
        },
        ProjectGraphSnapshot::new(
            files(),
            vec![
                IncludeEdge {
                    source: main_id(),
                    target: intro_id(),
                },
                IncludeEdge {
                    source: main_id(),
                    target: details_id(),
                },
            ],
        )?,
        tabs,
        active,
        targets,
        selected_build_target_id,
        vec![ExternalRevisionMarker::new(intro_id(), "disk-2")?],
    )
}

#[test]
fn multi_file_tabs_preserve_opening_order_and_duplicate_open_coalesces() {
    let mut workspace = ProjectWorkspace::new(
        make_record(vec![main_id()], Some(main_id()), None).unwrap(),
    );

    workspace.open(&intro_id()).unwrap();
    workspace.open(&details_id()).unwrap();
    workspace.open(&intro_id()).unwrap();

    assert_eq!(
        workspace.record.tabs,
        vec![main_id(), intro_id(), details_id()]
    );
    assert_eq!(workspace.record.active_tab, Some(intro_id()));
}

#[test]
fn record_has_deterministic_json_round_trip() {
    let record = make_record(
        vec![main_id(), intro_id(), details_id()],
        Some(intro_id()),
        Some("pdf".into()),
    )
    .unwrap();
    let first = serde_json::to_string(&record).unwrap();
    let restored: ProjectWorkspaceRecord = serde_json::from_str(&first).unwrap();
    let second = serde_json::to_string(&restored).unwrap();

    assert_eq!(restored, record);
    assert_eq!(second, first);
}

#[test]
fn missing_file_on_reopen_is_reported_without_changing_persisted_record() {
    let record = make_record(
        vec![main_id(), intro_id(), details_id()],
        Some(details_id()),
        None,
    )
    .unwrap();
    let available: HashSet<NormalizedRelativePath> =
        [main_path(), intro_path()].into_iter().collect();
    let restoration = ProjectWorkspace::restore(record.clone(), &available);

    assert_eq!(
        restoration.diagnostics,
        vec![ProjectWorkspaceRestorationDiagnostic::MissingDocument {
            document_id: details_id(),
            path: details_path()
        }]
    );
    assert_eq!(restoration.persisted_record, record);
    assert_eq!(restoration.workspace.record.tabs, record.tabs);
    assert_eq!(restoration.workspace.record.active_tab, Some(details_id()));
}

#[test]
fn include_cycle_and_missing_reference_remain_in_snapshot() {
    let missing = StableDocumentID::new("missing").unwrap();
    let graph = ProjectGraphSnapshot::new(
        files(),
        vec![
            IncludeEdge {
                source: main_id(),
                target: intro_id(),
            },
            IncludeEdge {
                source: intro_id(),
                target: main_id(),
            },
            IncludeEdge {
                source: main_id(),
                target: missing.clone(),
            },
        ],
    )
    .unwrap();

    let mut cycle_members = vec![intro_id(), main_id()];
    cycle_members.sort();
    assert_eq!(
        graph.diagnostics(project_id()).unwrap(),
        vec![
            ProjectGraphDiagnostic::MissingReference {
                source: main_id(),
                target: missing.clone(),
                missing: missing.clone(),
            },
            ProjectGraphDiagnostic::IncludeCycle {
                documents: cycle_members,
            },
        ]
    );
    assert!(graph.include_edges.contains(&IncludeEdge {
        source: main_id(),
        target: missing,
    }));
}

#[test]
fn closing_active_tab_selects_stable_neighbor() {
    let mut workspace = ProjectWorkspace::new(
        make_record(
            vec![main_id(), intro_id(), details_id()],
            Some(intro_id()),
            None,
        )
        .unwrap(),
    );

    workspace.close(&intro_id()).unwrap();
    assert_eq!(workspace.record.tabs, vec![main_id(), details_id()]);
    assert_eq!(workspace.record.active_tab, Some(details_id()));

    workspace.close(&details_id()).unwrap();
    assert_eq!(workspace.record.active_tab, Some(main_id()));

    workspace.close(&main_id()).unwrap();
    assert_eq!(workspace.record.active_tab, None);
}

#[test]
fn build_target_and_command_preferences_persist() {
    let mut workspace = ProjectWorkspace::new(
        make_record(vec![main_id()], Some(main_id()), None).unwrap(),
    );
    workspace.select_build_target("pdf").unwrap();

    let data = serde_json::to_string(&workspace.record).unwrap();
    let restored: ProjectWorkspaceRecord = serde_json::from_str(&data).unwrap();

    assert_eq!(restored.selected_build_target_id.as_deref(), Some("pdf"));
    let ids: Vec<&str> = restored.build_targets.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["draft", "pdf"]);
    assert_eq!(
        restored.build_targets.last().unwrap().command.executable,
        "latexmk"
    );
    assert_eq!(
        restored.build_targets.last().unwrap().command.arguments,
        vec!["-pdf", "main.tex"]
    );
}

#[test]
fn invalid_workspace_identities_and_references_are_rejected() {
    let mut dup_id_files = files();
    dup_id_files.push(ProjectFile {
        document_id: main_id(),
        path: details_path(),
    });
    assert_eq!(
        ProjectGraphSnapshot::new(dup_id_files, vec![]),
        Err(ProjectWorkspaceError::DuplicateDocumentID(main_id()))
    );

    let mut dup_path_files = files();
    dup_path_files.push(ProjectFile {
        document_id: StableDocumentID::new("other").unwrap(),
        path: main_path(),
    });
    assert_eq!(
        ProjectGraphSnapshot::new(dup_path_files, vec![]),
        Err(ProjectWorkspaceError::DuplicatePath(main_path()))
    );

    assert_eq!(
        make_record(vec![main_id()], Some(intro_id()), None).unwrap_err(),
        ProjectWorkspaceError::InvalidActiveTab(intro_id())
    );
    assert_eq!(
        make_record(
            vec![main_id()],
            Some(main_id()),
            Some("unknown".into())
        )
        .unwrap_err(),
        ProjectWorkspaceError::UnknownBuildTarget("unknown".into())
    );
}

#[test]
fn malformed_and_unsupported_restored_state_are_rejected() {
    let valid = serde_json::to_string(
        &make_record(vec![main_id()], Some(main_id()), None).unwrap(),
    )
    .unwrap();
    let mut object: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&valid).unwrap();
    object.insert("schemaVersion".into(), 2.into());
    let unsupported = serde_json::to_string(&object).unwrap();
    let err = serde_json::from_str::<ProjectWorkspaceRecord>(&unsupported).unwrap_err();
    assert!(
        err.to_string()
            .contains("UnsupportedSchemaVersion"),
        "unexpected error: {err}"
    );

    object.insert("schemaVersion".into(), 1.into());
    object.insert("tabs".into(), serde_json::json!(["main", "main"]));
    let malformed = serde_json::to_string(&object).unwrap();
    let err = serde_json::from_str::<ProjectWorkspaceRecord>(&malformed).unwrap_err();
    assert!(err.to_string().contains("DuplicateTab"), "unexpected: {err}");
}
