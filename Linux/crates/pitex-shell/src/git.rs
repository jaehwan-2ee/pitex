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
use std::sync::Arc;

pub(crate) fn run_git<I, S>(dir: &Path, args: I) -> Result<String, String>
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
        Ok(String::from_utf8(output.stdout)
            .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

/// `refreshGit()`'s off-thread half — detect the repo, then read status,
/// branches, and the commit log.
fn collect_git(project_root: &Path, previous: Option<Arc<git_core::GitStatus>>) -> Result<Option<GitRefresh>, String> {
    let Ok(top) = run_git(project_root, git_core::top_level_args()) else {
        return Ok(None);
    };
    let root = PathBuf::from(top.trim());
    let status_raw = run_git(&root, git_core::status_args()).map_err(|e| e.to_string())?;
    let branches_raw = run_git(&root, git_core::branch_args()).unwrap_or_default();
    // Empty on a repository with no commits — not an error for the panel.
    let log_raw = run_git(&root, git_core::log_args(80)).unwrap_or_default();
    let status = git_core::parse_status(&status_raw, &root.to_string_lossy());
    // Compare on the worker; an unchanged poll keeps the model and scroll position.
    let status = match previous {
        Some(old) if *old == status => old,
        _ => Arc::new(status),
    };
    Ok(Some(GitRefresh {
        status,
        commits: git_core::parse_log(&log_raw),
        branches: git_core::parse_branches(&branches_raw),
    }))
}

impl AppState {
    /// `refreshGit()` — repo detection, status, branches, and log on a
    /// worker; the `GitRefreshed` message applies the result.
    pub fn refresh_git(&self) {
        if self.model.console_section != crate::model::ConsoleSection::Git
            || !self.model.bottom_panel_visible { return; }
        let Some(tx) = self.tx.clone() else { return };
        let Some(root) = self.model.project_url.clone() else {
            let _ = tx.send(WorkspaceMessage::GitRefreshed(Ok(None)));
            return;
        };
        if self.git_refresh_pending.replace(true) { return; }
        *self.git_refresh_root.borrow_mut() = Some(root.clone());
        let previous = self.model.git_status.clone();
        std::thread::spawn(move || {
            let _ = tx.send(WorkspaceMessage::GitRefreshed(collect_git(&root, previous)));
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
    /// `at` pins the start point — the graph context menu branches off
    /// the selected commit, like VSCode's "Create Branch…" there.
    pub fn git_create_branch(&mut self, name: &str, at: Option<&str>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        self.git_operation(vec![git_core::create_branch_args(name, at)], false);
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

    /// `suggestCommitMessage()` — the "Suggest" half of the commit box: a
    /// one-shot `pi --print` run with the pending diff inline. A dedicated
    /// subprocess (like ghost completion) keeps the chat transcript and any
    /// in-flight ask untouched; failures surface in the panel's error line.
    pub fn git_suggest_message(&mut self) {
        let Some(status) = self.model.git_status.clone() else {
            return;
        };
        if self.model.git_suggest_busy {
            return;
        }
        let Some(tx) = self.tx.clone() else { return };
        self.model.git_suggest_busy = true;
        self.model.git_error = None;
        self.refresh_git_commit_button();
        std::thread::spawn(move || {
            let _ = tx.send(WorkspaceMessage::GitSuggestFinished(
                suggest_commit_message(&status),
            ));
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

/// Off-thread half of `git_suggest_message`: gather the diff, discover the
/// toolchain, then run one-shot `pi --print`. Mirrors the Swift extension
/// in `GitIntegrationView.swift` step for step.
fn suggest_commit_message(status: &git_core::GitStatus) -> Result<String, String> {
    use crate::agent::{locate_pi_executable_in, AgentCoordinator, PiToolchain};
    let root = PathBuf::from(&status.root);
    let staged = !status.staged.is_empty();
    let diff = run_git(&root, git_core::diff_args(staged)).unwrap_or_default();
    let tools = PiToolchain::discover(
        std::env::vars().collect(),
        &dirs::home_dir().unwrap_or_default(),
        &PiToolchain::SYSTEM_DIRECTORIES,
    );
    let Some(executable) = locate_pi_executable_in(&tools.environment) else {
        return Err("Pitex Agent is not installed yet — see Settings → AI.".into());
    };
    let files: Vec<String> = (if staged { &status.staged } else { &status.unstaged })
        .iter()
        .take(100)
        .map(|c| format!("{} {}", c.kind.badge(), c.path))
        .collect();
    let total = if staged { status.staged.len() } else { status.unstaged.len() };
    let prompt = commit_message_prompt(&status.branch, &files, &diff, total);
    let arguments = vec![
        "--no-session".to_string(),
        "--no-tools".to_string(),
        "--print".to_string(),
        prompt,
    ];
    let (executable, arguments) = tools.launch(&executable, &arguments)?;
    let environment = AgentCoordinator::child_environment(&executable, &tools.environment);
    let plan = build_core::DirectCommandPlan::new(
        executable.to_string_lossy().into_owned(),
        arguments,
        build_core::WorkingDirectoryPolicy::Explicit(status.root.clone()),
        build_core::EnvironmentPolicy::Inherit {
            overrides: environment,
        },
    )
    .map_err(|e| e.to_string())?;
    let result = build_core::ProcessRunner::default()
        .run(
            &plan,
            &root,
            None,
            Some(std::time::Duration::from_secs(90)),
            None,
            None,
        )
        .map_err(|e| e.to_string())?;
    let text = cleaned_commit_message(&String::from_utf8_lossy(&result.standard_output));
    if result.termination == (build_core::ProcessTermination::Exited { code: 0 })
        && !text.is_empty()
    {
        return Ok(text);
    }
    let detail = String::from_utf8_lossy(&result.standard_error)
        .trim()
        .to_string();
    Err(if detail.is_empty() {
        "Pitex Agent did not return a commit message.".into()
    } else {
        detail
            .chars()
            .skip(detail.chars().count().saturating_sub(400))
            .collect()
    })
}

/// Conventional-commit ask: branch + change list + the diff itself. The
/// diff is capped so a generated/mass-rename changeset cannot blow past the
/// model's context on a one-shot prompt.
fn commit_message_prompt(branch: &str, files: &[String], diff: &str, total_files: usize) -> String {
    const LIMIT: usize = 20_000;
    let body = if diff.len() > LIMIT {
        let mut end = LIMIT;
        while !diff.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}\n… [diff truncated]", &diff[..end])
    } else {
        diff.to_string()
    };
    let names = files.join("\n");
    let mut file_list: String = names.chars().take(5_000).collect();
    if total_files > files.len() || file_list.len() < names.len() {
        file_list.push_str(&format!("\n… [file list truncated; {total_files} files total]"));
    }
    format!(
        "Write the git commit message for this change set on branch '{branch}'.\n\
         Changed files:\n{}\n\nDiff:\n{body}\n\n\
         Reply with ONLY the commit message: one imperative subject line of at \
         most 72 characters, then optionally a blank line and a short body. No \
         quotes, no code fences, no commentary.",
        file_list
    )
}

/// `pi --print` should already answer with just the message; this drops a
/// wrapping code fence or quote pair when a model adds one anyway.
fn cleaned_commit_message(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    if let Some(rest) = text.strip_prefix("```") {
        let mut lines: Vec<&str> = rest.lines().collect();
        if lines.last().map(|l| l.starts_with("```")).unwrap_or(false) {
            lines.pop();
        }
        text = lines.join("\n").trim().to_string();
    }
    if text.len() > 1 && text.starts_with('"') && text.ends_with('"') {
        text = text[1..text.len() - 1].to_string();
    }
    text.chars().take(500).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_refresh_reuses_snapshot_but_staging_updates_it() {
        let _environment = crate::TEST_ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join(format!("pitex-git-refresh-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        run_git(&root, ["init", "-q"]).unwrap();
        let path = "한글 file.tex";
        std::fs::write(root.join(path), "hello").unwrap();
        let first = collect_git(&root, None).unwrap().unwrap().status;
        let next = collect_git(&root, Some(first.clone())).unwrap().unwrap().status;
        assert!(Arc::ptr_eq(&first, &next));
        assert_eq!(next.unstaged[0].path, path);
        run_git(&root, git_core::stage_args(path)).unwrap();
        let staged = collect_git(&root, Some(next.clone())).unwrap().unwrap().status;
        assert!(!Arc::ptr_eq(&staged, &next));
        assert_eq!(staged.staged[0].path, path);
        assert!(staged.unstaged.is_empty());
        run_git(&root, git_core::unstage_no_head_args(path)).unwrap();
        let unstaged = collect_git(&root, Some(staged)).unwrap().unwrap().status;
        assert!(unstaged.staged.is_empty());
        assert_eq!(unstaged.unstaged[0].path, path);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn commit_prompt_is_bounded_for_mass_changes_and_unicode_paths() {
        let files = vec!["한".repeat(4_096); 100];
        let prompt = commit_message_prompt("main", &files, &"한".repeat(100_000), 1_012_101);
        assert!(prompt.len() < 50_000);
        assert!(prompt.contains("1012101 files total"));
        assert!(prompt.contains("[diff truncated]"));
    }
}
