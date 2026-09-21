//! Port of the `WorkspaceModel` extension in
//! `Mac/Sources/Features/GitIntegrationView.swift` — git subprocess calls
//! for the Git Integration pane. Every invocation runs on a worker thread
//! and reports back through `WorkspaceMessage`; `app_ui.rs` applies them.

use crate::app_ui::AppState;
use crate::model::{GitRefresh, WorkspaceMessage};
use git_core::{self, GitChange, GitChangeKind};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run_git<I, S>(dir: &Path, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<OsString> = args
        .into_iter()
        .map(|a| OsString::from(a.as_ref()))
        .collect();
    let output = Command::new("git")
        .args(&args)
        .current_dir(dir)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

/// `refreshGit()`'s off-thread half — detect the repo, then read status,
/// branches, and the commit log.
fn collect_git(project_root: &Path) -> Result<Option<GitRefresh>, String> {
    let Ok(top) = run_git(project_root, git_core::top_level_args()) else {
        return Ok(None);
    };
    let root = PathBuf::from(top.trim());
    let status_raw = run_git(&root, git_core::status_args()).map_err(|e| e.to_string())?;
    let branches_raw = run_git(&root, git_core::branch_args()).unwrap_or_default();
    // Empty on a repository with no commits — not an error for the panel.
    let log_raw = run_git(&root, git_core::log_args(80)).unwrap_or_default();
    Ok(Some(GitRefresh {
        status: git_core::parse_status(&status_raw, &root.to_string_lossy()),
        commits: git_core::parse_log(&log_raw),
        branches: git_core::parse_branches(&branches_raw),
    }))
}

impl AppState {
    /// `refreshGit()` — repo detection, status, branches, and log on a
    /// worker; the `GitRefreshed` message applies the result.
    pub fn refresh_git(&self) {
        let Some(tx) = self.tx.clone() else { return };
        let Some(root) = self.model.project_url.clone() else {
            let _ = tx.send(WorkspaceMessage::GitRefreshed(Ok(None)));
            return;
        };
        std::thread::spawn(move || {
            let _ = tx.send(WorkspaceMessage::GitRefreshed(collect_git(&root)));
        });
    }

    /// `runGit` — sequential steps inside the repository; first failure wins.
    fn git_operation(&mut self, steps: Vec<Vec<String>>, clear_commit: bool) {
        let Some(root) = self.model.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.tx.clone() else { return };
        self.model.git_busy = true;
        self.model.git_error = None;
        self.refresh_git_panel();
        std::thread::spawn(move || {
            let dir = PathBuf::from(root);
            let mut error = None;
            for args in steps {
                if let Err(e) = run_git(&dir, &args) {
                    error = Some(e);
                    break;
                }
            }
            let _ = tx.send(WorkspaceMessage::GitOpFinished {
                error,
                clear_commit,
            });
        });
    }

    /// `gitOperationAny` — argv candidates tried in order until one exits 0
    /// (`restore --staged` → `reset HEAD` → `rm --cached` on a no-commit repo).
    fn git_operation_any(&mut self, candidates: Vec<Vec<String>>) {
        let Some(root) = self.model.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.tx.clone() else { return };
        self.model.git_busy = true;
        self.model.git_error = None;
        self.refresh_git_panel();
        std::thread::spawn(move || {
            let dir = PathBuf::from(root);
            let mut last_error = None;
            for args in candidates {
                match run_git(&dir, &args) {
                    Ok(_) => {
                        last_error = None;
                        break;
                    }
                    Err(e) => last_error = Some(e),
                }
            }
            let _ = tx.send(WorkspaceMessage::GitOpFinished {
                error: last_error,
                clear_commit: false,
            });
        });
    }

    pub fn git_stage(&mut self, change: &GitChange) {
        self.git_operation(vec![git_core::stage_args(&change.path).to_vec()], false);
    }

    pub fn git_unstage(&mut self, change: &GitChange) {
        self.git_operation_any(vec![
            git_core::unstage_args(&change.path).to_vec(),
            git_core::unstage_fallback_args(&change.path).to_vec(),
            git_core::unstage_no_head_args(&change.path).to_vec(),
        ]);
    }

    pub fn git_stage_all(&mut self) {
        self.git_operation(
            vec![git_core::stage_all_args().map(String::from).to_vec()],
            false,
        );
    }

    pub fn git_unstage_all(&mut self) {
        self.git_operation_any(vec![
            git_core::unstage_all_args().map(String::from).to_vec(),
            git_core::unstage_all_no_head_args()
                .map(String::from)
                .to_vec(),
        ]);
    }

    /// VSCode "Commit": staged changes only; when nothing is staged the
    /// button commits everything (stage-all first, like the VSCode prompt).
    pub fn git_commit(&mut self) {
        let Some(status) = self.model.git_status.as_ref() else {
            return;
        };
        let message = self.model.git_commit_message.trim().to_string();
        if message.is_empty() || (status.staged.is_empty() && status.unstaged.is_empty()) {
            return;
        }
        let mut steps: Vec<Vec<String>> = Vec::new();
        if status.staged.is_empty() {
            steps.push(git_core::stage_all_args().map(String::from).to_vec());
        }
        steps.push(git_core::commit_args(&message, false));
        self.git_operation(steps, true);
    }

    pub fn git_pull(&mut self) {
        self.git_operation(
            vec![git_core::pull_args().map(String::from).to_vec()],
            false,
        );
    }
    pub fn git_push(&mut self) {
        self.git_operation(
            vec![git_core::push_args().map(String::from).to_vec()],
            false,
        );
    }
    pub fn git_sync(&mut self) {
        self.git_operation(
            vec![
                git_core::pull_args().map(String::from).to_vec(),
                git_core::push_args().map(String::from).to_vec(),
            ],
            false,
        );
    }
    pub fn git_switch(&mut self, branch: &str) {
        self.git_operation(vec![git_core::switch_args(branch).to_vec()], false);
    }
    pub fn git_create_branch(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        self.git_operation(vec![git_core::create_branch_args(name).to_vec()], false);
    }

    /// Discard like VSCode: untracked files are deleted, staged-new files
    /// are `git rm -f`ed, everything else checks out HEAD (staged) or the
    /// index (unstaged).
    pub fn git_discard(&mut self, change: &GitChange) {
        if change.kind == GitChangeKind::Untracked {
            self.git_operation(vec![git_core::clean_args(&change.path).to_vec()], false);
        } else if change.staged {
            let args = if change.kind == GitChangeKind::Added {
                git_core::remove_file_args(&change.path).to_vec()
            } else {
                git_core::discard_staged_args(&change.path).to_vec()
            };
            self.git_operation(vec![args], false);
        } else {
            self.git_operation(vec![git_core::discard_args(&change.path).to_vec()], false);
        }
    }

    /// `git init` at the project root — the panel's only op when the
    /// project is not yet a repository.
    pub fn git_init(&mut self) {
        let Some(tx) = self.tx.clone() else { return };
        let Some(root) = self.model.project_url.clone() else {
            return;
        };
        self.model.git_busy = true;
        self.model.git_error = None;
        self.refresh_git_panel();
        std::thread::spawn(move || {
            let error = run_git(&root, git_core::init_args()).err();
            let _ = tx.send(WorkspaceMessage::GitOpFinished {
                error,
                clear_commit: false,
            });
        });
    }

    /// `openGitChange` — open the file at its repo-relative path.
    pub fn git_open_change(&mut self, change: &GitChange) {
        let Some(root) = self.model.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.tx.clone() else { return };
        self.model
            .activate_document(PathBuf::from(root).join(&change.path), tx);
    }
}
