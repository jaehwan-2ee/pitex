use document_session_core::DocumentSession;
use pitex_shell::model::WorkspaceModel;
use project_core::ProjectFile;
use std::fs::{self, File, FileTimes};
use tex_domain::{NormalizedRelativePath, StableDocumentID};

#[test]
fn bibliography_cache_reuses_unchanged_files_and_tracks_disk_and_live_edits() {
    let root = std::env::temp_dir().join(format!(
        "pitex-bib-cache-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let nested = root.join("refs");
    fs::create_dir_all(&nested).unwrap();
    let bib = nested.join("main.bib");
    fs::write(&bib, "@article{first, title={First}}\n").unwrap();

    let mut model = WorkspaceModel::new();
    model.project_url = Some(root.clone());
    model.project_files = vec![bib.clone()];
    model.refresh_structure();
    assert!(model.citation_keys.contains("first"));
    assert_eq!(model.bibliography_items[0].file, "refs/main.bib");
    let allocation = model.bibliography_items.as_ptr();
    model.refresh_structure();
    assert_eq!(
        model.bibliography_items.as_ptr(),
        allocation,
        "unchanged files were reparsed"
    );

    // Size changes also invalidate on filesystems with coarse timestamps.
    let modified = fs::metadata(&bib).unwrap().modified().unwrap();
    fs::write(&bib, "@book{external_edit, title={Changed on disk}}\n").unwrap();
    File::open(&bib)
        .unwrap()
        .set_times(FileTimes::new().set_modified(modified))
        .unwrap();
    model.refresh_structure();
    assert!(model.citation_keys.contains("external_edit"));
    assert!(!model.citation_keys.contains("first"));

    // The active unsaved buffer still contributes fresh completion keys.
    let file = ProjectFile {
        document_id: StableDocumentID::new("bib").unwrap(),
        path: NormalizedRelativePath::new("refs/main.bib").unwrap(),
    };
    model.active_document_url = Some(bib.clone());
    let allocation = model.bibliography_items.as_ptr();
    for key in ["unsaved_one", "unsaved_two"] {
        model.document_snapshot = Some(
            DocumentSession::new(&file, format!("@article{{{key}, title={{Live}}}}"), None)
                .snapshot(),
        );
        model.refresh_structure();
        assert!(model.citation_keys.contains(key));
        assert!(model.citation_keys.contains("external_edit"));
        assert_eq!(model.bibliography_items.as_ptr(), allocation);
    }
    assert!(!model.citation_keys.contains("unsaved_one"));
    model.active_document_url = None;
    model.document_snapshot = None;

    model.project_url = Some(nested.clone());
    model.refresh_structure();
    assert_eq!(model.bibliography_items[0].file, "main.bib");
    let other = nested.join("other.bib");
    fs::write(&other, "@book{added, title={New file}}").unwrap();
    model.project_files.push(other.clone());
    model.refresh_structure();
    assert!(model.citation_keys.contains("added"));
    model.project_files.pop();
    model.refresh_structure();
    assert!(!model.citation_keys.contains("added"));

    fs::remove_file(&bib).unwrap();
    model.refresh_structure();
    assert!(model.bibliography_items.is_empty());
    // Failed UTF-8 reads must not be cached, even if metadata stays the same.
    let recovered = "@article{recovered, title={OK}}";
    fs::write(&bib, vec![0xff; recovered.len()]).unwrap();
    model.refresh_structure();
    let modified = fs::metadata(&bib).unwrap().modified().unwrap();
    fs::write(&bib, recovered).unwrap();
    File::open(&bib)
        .unwrap()
        .set_times(FileTimes::new().set_modified(modified))
        .unwrap();
    model.refresh_structure();
    assert!(model.citation_keys.contains("recovered"));

    model.close();
    model.project_url = Some(nested);
    model.project_files = vec![bib];
    model.refresh_structure();
    assert!(model.citation_keys.contains("recovered"));
    fs::remove_dir_all(root).unwrap();
}
