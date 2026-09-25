//! Port of the `WorkspaceModel` extension in
//! `Mac/Sources/Features/GitIntegrationView.swift` — git subprocess calls
//! for the Git Integration pane. Every invocation runs on a worker thread
//! and reports back through `WorkspaceMessage`; `app_ui.rs` applies them.
//!
//! `GitRunner` is the single choke point deciding whether a command runs
//! locally or on the device: while a remote workspace is open every call
//! lands over SSH through `RemoteSync.client.run_git` (the login-shell
//! runner), with a mirror push before and a mirror pull after each
//! operation. Local projects take `local_git_output` byte for byte.

use crate::app_ui::AppState;
use crate::model::{GitDiff, GitDiffSource, GitRefresh, WorkspaceMessage, WorkspaceModel};
use git_core::{self, GitChange, GitChangeKind, GitCommitFile};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::sync::Arc;

/// Raw subprocess result — `run_codes` turns it into the Ok/Err shape
/// the pane's callers expect.
struct GitOutput {
    code: i32,
    stdout: String,
    stderr: String,
}

impl GitOutput {
    fn error_text(&self) -> String {
        let stderr = self.stderr.trim();
        let stdout = self.stdout.trim();
        if stderr.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        }
    }
}

/// `run_git` with an explicit set of success exit codes — `git diff
/// --no-index` exits 1 when the compared files differ, which is not an
/// error for the untracked-file diff. The local-primitive choke point
/// stays exercised by the unit tests.
#[cfg(test)]
pub(crate) fn run_git_codes<I, S>(dir: &Path, args: I, ok: &[i32]) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|a| a.as_ref().to_string())
        .collect();
    match local_git_output(dir, &args) {
        Err(e) => Err(e),
        Ok(output) if ok.contains(&output.code) => Ok(output.stdout),
        Ok(output) => Err(output.error_text()),
    }
}

fn local_git_output(dir: &Path, args: &[String]) -> Result<GitOutput, String> {
    let mut command = Command::new("git");
    command.args(args).current_dir(dir);
    // The pane polls every few seconds — no console window per call.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = command.output().map_err(|e| e.to_string())?;
    Ok(GitOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8(output.stdout)
            .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned()),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
pub(crate) fn run_git<I, S>(dir: &Path, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    run_git_codes(dir, args, &[0])
}

/// Where git commands run — the macOS `GitRunner.run(in:remote:)` choke
/// point. `Local` spawns `git`; `Remote` sends it to the device through
/// `SshClient::run_git` while an SSH workspace is open.
#[derive(Clone)]
enum GitRunner {
    Local,
    Remote(Arc<remote_core::RemoteSync>),
}

impl GitRunner {
    fn for_workspace(remote: Option<&crate::remote::RemoteWorkspace>) -> Self {
        match remote {
            Some(remote) => Self::Remote(remote.sync.clone()),
            None => Self::Local,
        }
    }

    fn is_remote(&self) -> bool {
        matches!(self, Self::Remote(_))
    }

    /// `dir` expressed in the runner's own space: a local/mirror path
    /// maps to its device counterpart for `Remote` and passes through
    /// for `Local`.
    fn project_dir(&self, dir: &Path) -> String {
        match self {
            Self::Remote(sync) => sync
                .remote_path(dir)
                .unwrap_or_else(|| dir.to_string_lossy().into_owned()),
            Self::Local => dir.to_string_lossy().into_owned(),
        }
    }

    /// `git args` in `dir` — `dir` already in the runner's space: a
    /// filesystem path for `Local`, a device path for `Remote`
    /// (`status.root` comes back from `rev-parse --show-toplevel` *on
    /// the device*). `Err` is a transport failure — a non-zero git exit
    /// lands in `GitOutput.code`.
    fn output(&self, dir: &str, args: &[String]) -> Result<GitOutput, String> {
        match self {
            Self::Local => local_git_output(Path::new(dir), args),
            Self::Remote(sync) => sync
                .client
                .run_git(dir, args)
                .map(|result| GitOutput {
                    code: result.status,
                    stdout: result.stdout_text(),
                    stderr: result.stderr_text(),
                })
                .map_err(|error| error.to_string()),
        }
    }

    /// `output` with `dir` as a local/mirror path — `Remote` maps it to
    /// the device path first.
    fn output_in(&self, dir: &Path, args: &[String]) -> Result<GitOutput, String> {
        match self {
            Self::Local => local_git_output(dir, args),
            Self::Remote(_) => self.output(&self.project_dir(dir), args),
        }
    }

    fn run(&self, dir: &str, args: &[String]) -> Result<String, String> {
        self.run_codes(dir, args, &[0])
    }

    /// `Ok(stdout)` on an accepted exit code, `Err(stderr|stdout)` on a
    /// non-zero git exit or a transport failure — same shape `run_git`
    /// gave the local-only callers.
    fn run_codes(&self, dir: &str, args: &[String], ok: &[i32]) -> Result<String, String> {
        match self.output(dir, args) {
            Err(e) => Err(e),
            Ok(output) if ok.contains(&output.code) => Ok(output.stdout),
            Ok(output) => Err(output.error_text()),
        }
    }

    /// `run` with `dir` as a local/mirror path.
    fn run_in(&self, dir: &Path, args: &[String]) -> Result<String, String> {
        match self.output_in(dir, args) {
            Err(e) => Err(e),
            Ok(output) if output.code == 0 => Ok(output.stdout),
            Ok(output) => Err(output.error_text()),
        }
    }
}

/// Push the mirror before a remote git run so device-side git sees the
/// saved bytes — the `RemotePushFinished` report keeps `remote.status`
/// (offline) honest on failure. A no-op for `Local`.
fn report_push(tx: &Sender<WorkspaceMessage>, runner: &GitRunner) {
    let GitRunner::Remote(sync) = runner else { return };
    let _ = tx.send(WorkspaceMessage::RemotePushFinished {
        root: sync.mirror.directory.clone(),
        result: sync.push().map(|r| r.conflicts).map_err(|e| e.to_string()),
    });
}

/// Pull after a remote git op so device-side file rewrites (checkout,
/// switch, pull) land in the mirror — `RemotePullFinished` drives the
/// existing post-pull adoption for open documents. A no-op for `Local`.
fn report_pull(tx: &Sender<WorkspaceMessage>, runner: &GitRunner) {
    let GitRunner::Remote(sync) = runner else { return };
    let _ = tx.send(WorkspaceMessage::RemotePullFinished {
        root: sync.mirror.directory.clone(),
        result: sync.pull().map_err(|e| e.to_string()),
    });
}

/// `refreshGit()`'s off-thread half — detect the repo, then read status,
/// branches, and the commit log. A non-zero `rev-parse` is `Ok(None)`
/// ("not a repository"); a transport failure — SSH down for `Remote`,
/// no git binary for `Local` — is `Err` so the pane can say which.
fn collect_git(
    project_root: &Path,
    previous: Option<Arc<git_core::GitStatus>>,
    runner: &GitRunner,
) -> Result<Option<GitRefresh>, String> {
    let top = match runner.output_in(project_root, git_core::top_level_args().map(String::from).as_slice()) {
        Ok(output) => output,
        // An unreachable device must not masquerade as "not a repo".
        Err(e) => return if runner.is_remote() { Err(e) } else { Ok(None) },
    };
    if top.code != 0 {
        // A 127 from the device login shell means it found no `git` —
        // a transport-style failure like SSH down, not "not a repo".
        if let GitRunner::Remote(sync) = runner {
            if top.code == 127 {
                let detail = top.error_text();
                return Err(if detail.is_empty() {
                    format!("git was not found on {}", sync.mirror.project.connection.name)
                } else {
                    detail
                });
            }
        }
        return Ok(None);
    }
    let root = top.stdout.trim().to_string();
    let status_raw = runner.run(&root, git_core::status_args().map(String::from).as_slice()).map_err(|e| e.to_string())?;
    let branches_raw = runner.run(&root, git_core::branch_args().map(String::from).as_slice()).unwrap_or_default();
    // Empty on a repository with no commits — not an error for the panel.
    let log_raw = runner.run(&root, git_core::log_args(80).as_slice()).unwrap_or_default();
    let status = git_core::parse_status(&status_raw, &root);
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

impl WorkspaceModel {
    /// `refreshGit()` — repo detection, status, branches, and log on a
    /// worker; the `GitRefreshed` message applies the result. A remote
    /// workspace pushes the mirror first so device git reads saved edits.
    pub fn refresh_git(&self) {
        if self.console_section != crate::model::ConsoleSection::Git
            || !self.bottom_panel_visible { return; }
        let Some(tx) = self.sink() else { return };
        let Some(root) = self.project_url.clone() else {
            let _ = tx.send(WorkspaceMessage::GitRefreshed(Ok(None)));
            return;
        };
        if self.git_refresh_pending.replace(true) { return; }
        *self.git_refresh_root.borrow_mut() = Some(root.clone());
        let previous = self.git_status.clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            report_push(&tx, &runner);
            let _ = tx.send(WorkspaceMessage::GitRefreshed(collect_git(&root, previous, &runner)));
        });
    }

    /// `runGit` — sequential steps inside the repository; first failure
    /// wins. Remote ops push first and pull after, so device-side writes
    /// (checkout, switch, pull) reach the mirror and open editors.
    fn git_operation(&mut self, steps: Vec<Vec<String>>, clear_commit: bool) {
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        self.git_busy = true;
        self.git_error = None;
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            report_push(&tx, &runner);
            let mut error = None;
            for args in &steps {
                if let Err(e) = runner.run(&root, args) {
                    error = Some(e);
                    break;
                }
            }
            report_pull(&tx, &runner);
            let _ = tx.send(WorkspaceMessage::GitOpFinished {
                error,
                clear_commit,
            });
        });
    }

    /// `gitOperationAny` — argv candidates tried in order until one exits 0
    /// (`restore --staged` → `reset HEAD` → `rm --cached` on a no-commit repo).
    fn git_operation_any(&mut self, candidates: Vec<Vec<String>>) {
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        self.git_busy = true;
        self.git_error = None;
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            report_push(&tx, &runner);
            let mut last_error = None;
            for args in &candidates {
                match runner.run(&root, args) {
                    Ok(_) => {
                        last_error = None;
                        break;
                    }
                    Err(e) => last_error = Some(e),
                }
            }
            report_pull(&tx, &runner);
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
        let Some(status) = self.git_status.as_ref() else {
            return;
        };
        let message = self.git_commit_message.trim().to_string();
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

    /// `git init` at the project root — `remote_root` on the device for a
    /// remote workspace.
    pub fn git_init(&mut self) {
        let Some(tx) = self.sink() else { return };
        let Some(root) = self.project_url.clone() else {
            return;
        };
        self.git_busy = true;
        self.git_error = None;
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            report_push(&tx, &runner);
            let error = runner
                .run_in(&root, git_core::init_args().map(String::from).as_slice())
                .err();
            report_pull(&tx, &runner);
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
        let Some(status) = self.git_status.clone() else {
            return;
        };
        if self.git_suggest_busy {
            return;
        }
        let Some(tx) = self.sink() else { return };
        self.git_suggest_busy = true;
        self.git_error = None;
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        // pi runs locally — its working directory is the mirror, since a
        // remote `status.root` does not exist on this filesystem.
        let work_dir = self.project_url.clone();
        std::thread::spawn(move || {
            report_push(&tx, &runner);
            let _ = tx.send(WorkspaceMessage::GitSuggestFinished(
                suggest_commit_message(&status, work_dir.as_deref(), &runner),
            ));
        });
    }

    /// `openGitChange` — open the file at its repo-relative path. A remote
    /// `root` is a device path, so the change maps back into the mirror;
    /// a repo extending above `remote_root` lists changes that have no
    /// local copy and simply don't open.
    pub fn git_open_change(&mut self, change: &GitChange) {
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        let path = match &self.remote {
            Some(remote) => match remote.sync.mirror.local_path(&root, &change.path) {
                Some(path) => path,
                None => return,
            },
            None => PathBuf::from(root).join(&change.path),
        };
        self.activate_document(path, tx);
    }

    /// `toggleGitCommit` — a graph row's disclosure: the first expansion
    /// fetches the commit's `--name-status` list on a worker.
    pub fn git_toggle_commit(&mut self, commit: &git_core::GitCommit) {
        if self.git_expanded_commits.contains(&commit.hash) {
            self.git_expanded_commits.remove(&commit.hash);
            return;
        }
        self.git_expanded_commits.insert(commit.hash.clone());
        if self.git_commit_files.contains_key(&commit.hash) {
            return;
        }
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        self.git_commit_files_busy.insert(commit.hash.clone());
        let hash = commit.full_hash.clone();
        let key = commit.hash.clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let files = runner
                .run(&root, git_core::commit_files_args(&hash).as_slice())
                .map(|raw| git_core::parse_commit_files(&raw))
                .unwrap_or_default();
            let _ = tx.send(WorkspaceMessage::GitCommitFilesLoaded {
                hash: key,
                root,
                files,
            });
        });
    }

    /// `openGitDiff(_:file:)` — clicking a file under a commit opens its
    /// diff over the editor area.
    pub fn git_open_commit_diff(&mut self, commit: &git_core::GitCommit, file: GitCommitFile) {
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        let id = self.next_git_diff_id();
        self.git_diff = Some(GitDiff {
            id,
            source: GitDiffSource::Commit(commit.clone()),
            file: Some(file.clone()),
            sections: None,
            error: None,
        });
        let hash = commit.full_hash.clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let result = runner
                .run(&root, git_core::file_diff_args(&hash, &file.path).as_slice())
                .map(|stdout| {
                    vec![git_core::GitDiffFileSection {
                        binary: stdout.contains("Binary files"),
                        rows: git_core::parse_file_diff(&stdout),
                        file,
                    }]
                });
            let _ = tx.send(WorkspaceMessage::GitDiffLoaded { id, root, result });
        });
    }

    /// `openGitDiff(_:)` — the context menu's "Open Changes": the whole
    /// commit as a multi-file diff, like VSCode's multi-diff editor.
    pub fn git_open_commit_full_diff(&mut self, commit: &git_core::GitCommit) {
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        let id = self.next_git_diff_id();
        self.git_diff = Some(GitDiff {
            id,
            source: GitDiffSource::Commit(commit.clone()),
            file: None,
            sections: None,
            error: None,
        });
        let hash = commit.full_hash.clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let result = (|| {
                let diff = runner.run(&root, git_core::commit_diff_args(&hash).as_slice())?;
                let files = runner
                    .run(&root, git_core::commit_files_args(&hash).as_slice())
                    .map(|raw| git_core::parse_commit_files(&raw))
                    .unwrap_or_default();
                let kinds: std::collections::HashMap<String, GitChangeKind> = files
                    .into_iter()
                    .map(|f| (f.path, f.kind))
                    .collect();
                Ok(git_core::parse_commit_diff(&diff)
                    .into_iter()
                    .map(|(path, binary, rows)| git_core::GitDiffFileSection {
                        file: GitCommitFile {
                            kind: kinds.get(&path).copied().unwrap_or(GitChangeKind::Modified),
                            path,
                        },
                        binary,
                        rows,
                    })
                    .collect())
            })();
            let _ = tx.send(WorkspaceMessage::GitDiffLoaded { id, root, result });
        });
    }

    /// `openGitWorkingDiff` — clicking a Changes row opens its
    /// working-tree diff the same way; the row's ↗ button still opens
    /// the file itself. The mirror is pushed first so the diff reads the
    /// same bytes the editor shows.
    pub fn git_open_working_diff(&mut self, change: &GitChange) {
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        let file = GitCommitFile {
            path: change.path.clone(),
            kind: change.kind,
        };
        let id = self.next_git_diff_id();
        self.git_diff = Some(GitDiff {
            id,
            source: GitDiffSource::WorkingTree {
                staged: change.staged,
            },
            file: Some(file),
            sections: None,
            error: None,
        });
        let change = change.clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            report_push(&tx, &runner);
            let args = git_core::working_file_diff_args(&change);
            // `git diff --no-index` (untracked files) exits 1 on differences.
            let ok: &[i32] = if change.kind == GitChangeKind::Untracked {
                &[0, 1]
            } else {
                &[0]
            };
            let result = runner.run_codes(&root, args.as_slice(), ok).map(|stdout| {
                vec![git_core::GitDiffFileSection {
                    binary: stdout.contains("Binary files"),
                    rows: git_core::parse_file_diff(&stdout),
                    file: GitCommitFile {
                        path: change.path.clone(),
                        kind: change.kind,
                    },
                }]
            });
            let _ = tx.send(WorkspaceMessage::GitDiffLoaded { id, root, result });
        });
    }

    /// `closeGitDiff` — the diff overlay's close button.
    pub fn git_close_diff(&mut self) {
        self.git_diff = None;
    }

    /// `openGitDiff*` sessions get a monotonically increasing id — the
    /// dispatch drops worker results for superseded sessions (the Swift
    /// `gitDiff?.source == …` guards).
    fn next_git_diff_id(&self) -> u64 {
        let id = self.git_diff_seq.get() + 1;
        self.git_diff_seq.set(id);
        id
    }
}

impl AppState {
    /// `refreshGit()` — repo detection, status, branches, and log on a
    /// worker; the `GitRefreshed` message applies the result.
    pub fn refresh_git(&self) {
        self.model.refresh_git();
    }

    pub fn git_stage(&mut self, change: &GitChange) {
        self.model.git_stage(change);
        self.refresh_git_panel();
    }

    pub fn git_unstage(&mut self, change: &GitChange) {
        self.model.git_unstage(change);
        self.refresh_git_panel();
    }

    pub fn git_stage_all(&mut self) {
        self.model.git_stage_all();
        self.refresh_git_panel();
    }

    pub fn git_unstage_all(&mut self) {
        self.model.git_unstage_all();
        self.refresh_git_panel();
    }

    /// VSCode "Commit": staged changes only; when nothing is staged the
    /// button commits everything (stage-all first, like the VSCode prompt).
    pub fn git_commit(&mut self) {
        self.model.git_commit();
        self.refresh_git_panel();
    }

    pub fn git_pull(&mut self) {
        self.model.git_pull();
        self.refresh_git_panel();
    }
    pub fn git_push(&mut self) {
        self.model.git_push();
        self.refresh_git_panel();
    }
    pub fn git_sync(&mut self) {
        self.model.git_sync();
        self.refresh_git_panel();
    }
    pub fn git_switch(&mut self, branch: &str) {
        self.model.git_switch(branch);
        self.refresh_git_panel();
    }
    /// `at` pins the start point — the graph context menu branches off
    /// the selected commit, like VSCode's "Create Branch…" there.
    pub fn git_create_branch(&mut self, name: &str, at: Option<&str>) {
        self.model.git_create_branch(name, at);
        self.refresh_git_panel();
    }

    /// Discard like VSCode: untracked files are deleted, staged-new files
    /// are `git rm -f`ed, everything else checks out HEAD (staged) or the
    /// index (unstaged).
    pub fn git_discard(&mut self, change: &GitChange) {
        self.model.git_discard(change);
        self.refresh_git_panel();
    }

    /// `git init` at the project root — the panel's only op when the
    /// project is not yet a repository.
    pub fn git_init(&mut self) {
        self.model.git_init();
        self.refresh_git_panel();
    }

    /// `suggestCommitMessage()` — the "Suggest" half of the commit box: a
    /// one-shot `pi --print` run with the pending diff inline. A dedicated
    /// subprocess (like ghost completion) keeps the chat transcript and any
    /// in-flight ask untouched; failures surface in the panel's error line.
    pub fn git_suggest_message(&mut self) {
        self.model.git_suggest_message();
        self.refresh_git_commit_button();
    }

    /// `openGitChange` — open the file at its repo-relative path.
    pub fn git_open_change(&mut self, change: &GitChange) {
        self.model.git_open_change(change);
        self.refresh_git_diff();
    }

    /// `toggleGitCommit` — a graph row's disclosure: the first expansion
    /// fetches the commit's `--name-status` list on a worker.
    pub fn git_toggle_commit(&mut self, commit: &git_core::GitCommit) {
        self.model.git_toggle_commit(commit);
    }

    /// `openGitDiff(_:file:)` — clicking a file under a commit opens its
    /// diff over the editor area.
    pub fn git_open_commit_diff(&mut self, commit: &git_core::GitCommit, file: GitCommitFile) {
        self.model.git_open_commit_diff(commit, file);
        self.refresh_git_diff();
    }

    /// `openGitDiff(_:)` — the context menu's "Open Changes": the whole
    /// commit as a multi-file diff, like VSCode's multi-diff editor.
    pub fn git_open_commit_full_diff(&mut self, commit: &git_core::GitCommit) {
        self.model.git_open_commit_full_diff(commit);
        self.refresh_git_diff();
    }

    /// `openGitWorkingDiff` — clicking a Changes row opens its
    /// working-tree diff the same way; the row's ↗ button still opens
    /// the file itself.
    pub fn git_open_working_diff(&mut self, change: &GitChange) {
        self.model.git_open_working_diff(change);
        self.refresh_git_diff();
    }

    /// `closeGitDiff` — the diff overlay's close button.
    pub fn git_close_diff(&mut self) {
        self.model.git_close_diff();
        self.refresh_git_diff();
    }
}

/// Off-thread half of `git_suggest_message`: gather the diff, discover the
/// toolchain, then run one-shot `pi --print`. Mirrors the Swift extension
/// in `GitIntegrationView.swift` step for step.
fn suggest_commit_message(
    status: &git_core::GitStatus,
    work_dir: Option<&Path>,
    runner: &GitRunner,
) -> Result<String, String> {
    use crate::agent::{locate_pi_executable_in, AgentCoordinator, PiToolchain};
    // The pi child is a local process — its directory is the mirror, not
    // a device path that does not exist locally.
    let root = if runner.is_remote() {
        work_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(&status.root))
    } else {
        PathBuf::from(&status.root)
    };
    let staged = !status.staged.is_empty();
    let diff = runner
        .run(&status.root, git_core::diff_args(staged).map(String::from).as_slice())
        .unwrap_or_default();
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
        build_core::WorkingDirectoryPolicy::Explicit(root.to_string_lossy().into_owned()),
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
        let runner = GitRunner::Local;
        let first = collect_git(&root, None, &runner).unwrap().unwrap().status;
        let next = collect_git(&root, Some(first.clone()), &runner).unwrap().unwrap().status;
        assert!(Arc::ptr_eq(&first, &next));
        assert_eq!(next.unstaged[0].path, path);
        run_git(&root, git_core::stage_args(path)).unwrap();
        let staged = collect_git(&root, Some(next.clone()), &runner).unwrap().unwrap().status;
        assert!(!Arc::ptr_eq(&staged, &next));
        assert_eq!(staged.staged[0].path, path);
        assert!(staged.unstaged.is_empty());
        run_git(&root, git_core::unstage_no_head_args(path)).unwrap();
        let unstaged = collect_git(&root, Some(staged), &runner).unwrap().unwrap().status;
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
