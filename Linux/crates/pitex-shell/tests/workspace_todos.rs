//! Coverage for the TODOs pane data path: `parse_todos` collects
//! `% TODO:`/`% DONE:` comments (trailing comments included) and the
//! disk-path edit helpers rewrite exactly the comment line — guarded by
//! the same session-clean rules as the Swift `editTodoOnDisk`.

use pitex_shell::model::{DocumentTodoItem, TodoLineEdit, WorkspaceModel};
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "pitex-todos-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn todo(file: &PathBuf, line: usize, text: &str, done: bool) -> DocumentTodoItem {
    DocumentTodoItem {
        file: "main.tex".into(),
        url: file.clone(),
        line,
        text: text.into(),
        done,
    }
}

#[test]
fn parse_todos_collects_pending_done_and_trailing_comments() {
    let text = "\\section{A}\n% TODO: first\nlet x % TODO:   spaced  \n% DONE: finished\n%% not a todo\n% TODO-ish: no colon match\n% TODO:\n";
    let items = WorkspaceModel::parse_todos(
        text,
        &PathBuf::from("/p/main.tex"),
        Some(std::path::Path::new("/p")),
    );
    assert_eq!(items.len(), 4, "{items:?}");
    assert_eq!(items[0].line, 2);
    assert_eq!(items[0].text, "first");
    assert!(!items[0].done);
    assert_eq!(items[1].line, 3);
    // Leading whitespace after the marker is stripped; trailing is kept,
    // matching the Swift `drop(while:)` parse.
    assert_eq!(items[1].text, "spaced  ");
    assert!(!items[1].done);
    assert_eq!(items[2].line, 4);
    assert_eq!(items[2].text, "finished");
    assert!(items[2].done);
    // An empty body stays an item — the pane can still navigate to it.
    assert_eq!(items[3].line, 7);
    assert_eq!(items[3].text, "");
}

#[test]
fn todo_disk_edits_toggle_rename_and_delete() {
    let root = temp_dir("edit");
    let file = root.join("main.tex");
    std::fs::write(&file, "a\n% TODO: task one\nb\n").unwrap();

    let mut model = WorkspaceModel::new();
    model.project_url = Some(root.clone());
    model.project_files = vec![file.clone()];

    let item = todo(&file, 2, "task one", false);
    // No session is dirty for the file → the disk path writes directly.
    model.edit_todo(&item, TodoLineEdit::ToggleDone);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "a\n% DONE: task one\nb\n"
    );
    model.edit_todo(&item, TodoLineEdit::Rename("renamed".into()));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "a\n% DONE: renamed\nb\n"
    );
    model.edit_todo(&item, TodoLineEdit::Delete);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "a\nb\n");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn todo_disk_edit_preserves_prefix_on_trailing_comments() {
    let root = temp_dir("trailing");
    let file = root.join("main.tex");
    std::fs::write(&file, "x = 1 % TODO: trailing\n").unwrap();

    let mut model = WorkspaceModel::new();
    model.project_url = Some(root.clone());
    model.project_files = vec![file.clone()];

    let item = todo(&file, 1, "trailing", false);
    model.edit_todo(&item, TodoLineEdit::ToggleDone);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "x = 1 % DONE: trailing\n"
    );
    // Rename rewrites only the comment text — the code prefix stays.
    model.edit_todo(&item, TodoLineEdit::Rename("new words".into()));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "x = 1 % DONE: new words\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn todo_append_writes_to_build_target() {
    let root = temp_dir("append");
    let file = root.join("main.tex");
    std::fs::write(&file, "body\n").unwrap();

    let mut model = WorkspaceModel::new();
    model.project_url = Some(root.clone());
    model.project_files = vec![file.clone()];
    model.pinned_build_target = Some(file.clone());

    model.append_todo_to_target();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "body\n% TODO: \n"
    );
    // A file without a trailing newline gets one before the comment.
    std::fs::write(&file, "body").unwrap();
    model.append_todo_to_target();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "body\n% TODO: \n"
    );
    let _ = std::fs::remove_dir_all(&root);
}
