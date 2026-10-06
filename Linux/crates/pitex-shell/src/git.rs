//! Port of the `WorkspaceModel` extension in
//! `Mac/Sources/Features/GitIntegrationView.swift` — git subprocess calls
//! for the Git Integration pane. Every invocation runs on a worker thread
//! and reports back through `WorkspaceMessage`; `app_ui.rs` applies them.
//!
//! `GitRunner` is the single choke point deciding whether a command runs
//! locally or on the device: while a remote workspace is open every call
//! lands over SSH through `RemoteSync.run_git` (the login-shell runner),
//! queued with file saves and downloads, using explicitly saved remote
//! files. Worktree-changing Git operations pull device-side changes into
//! the mirror afterward; index/ref-only operations just refresh Git. Local
//! projects take `local_git_output` byte for byte.

use crate::app_ui::AppState;
use crate::model::{GitDiff, GitDiffSource, GitRefresh, WorkspaceMessage, WorkspaceModel};
use git_core::{self, GitChange, GitChangeKind, GitCommitFile};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// Identifies the mounted workspace, including the particular SSH sync
/// instance. Closing and reopening the same path starts a new generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorkspaceContext {
    project: Option<PathBuf>,
    remote: Option<usize>,
    generation: u64,
}

#[derive(Default)]
pub(crate) struct GitCache {
    context: Option<GitWorkspaceContext>,
    root: String,
    status: Option<Arc<git_core::GitStatus>>,
    refs: Option<String>,
    branch: String,
    history_at: Option<Instant>,
    diffs: VecDeque<((String, Option<String>), Arc<git_core::GitDiffContent>)>,
    diff_bytes: usize,
}

impl GitCache {
    const DIFF_BUDGET: usize = 16 * 1024 * 1024;

    fn select(&mut self, context: &GitWorkspaceContext, root: &str) {
        if self.context.as_ref() != Some(context) || self.root != root {
            *self = Self { context: Some(context.clone()), root: root.into(), ..Self::default() };
        }
    }

    fn remember_diff(&mut self, key: (String, Option<String>), content: Arc<git_core::GitDiffContent>) {
        // This is a 16 MiB estimated allocation budget for cached
        // payloads, not a hard resident-memory cap; active UI payloads
        // and allocator bookkeeping are outside the cache's ownership.
        let bytes = Self::diff_cost(&key, &content);
        if bytes > Self::DIFF_BUDGET { return; }
        while self.diff_bytes.saturating_add(bytes) > Self::DIFF_BUDGET || self.diffs.len() >= 16 {
            if let Some((old_key, old)) = self.diffs.pop_front() {
                self.diff_bytes = self.diff_bytes.saturating_sub(Self::diff_cost(&old_key, &old));
            } else { break; }
        }
        self.diff_bytes += bytes;
        self.diffs.push_back((key, content));
    }

    fn diff_cost(key: &(String, Option<String>), content: &git_core::GitDiffContent) -> usize {
        content.estimated_bytes.saturating_add(key.0.capacity())
            .saturating_add(key.1.as_ref().map_or(0, String::capacity))
            .saturating_add(std::mem::size_of::<((String, Option<String>), Arc<git_core::GitDiffContent>)>())
    }
}

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
    #[cfg(test)]
    CountingLocal(Arc<Mutex<Vec<Vec<String>>>>),
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
            #[cfg(test)]
            Self::CountingLocal(_) => dir.to_string_lossy().into_owned(),
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
            #[cfg(test)]
            Self::CountingLocal(calls) => {
                calls.lock().unwrap().push(args.to_vec());
                local_git_output(Path::new(dir), args)
            }
            Self::Remote(sync) => sync
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
            #[cfg(test)]
            Self::CountingLocal(_) => self.output(&self.project_dir(dir), args),
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

    fn read_diff(&self, dir: &str, args: &[String], ok: &[i32], cancel: &build_core::CancellationToken) -> Result<String, String> {
        if cancel.is_cancelled() { return Err("Git diff cancelled".into()); }
        let output = if self.is_remote() {
            // The SSH API is synchronous; discard superseded responses
            // before parsing and publishing them.
            self.output(dir, args)?
        } else {
            #[cfg(test)]
            if let Self::CountingLocal(calls) = self { calls.lock().unwrap().push(args.to_vec()); }
            let plan = build_core::DirectCommandPlan::new(
                "git", args.to_vec(),
                build_core::WorkingDirectoryPolicy::Explicit(dir.into()),
                build_core::EnvironmentPolicy::Inherit { overrides: Default::default() },
            ).map_err(|e| e.to_string())?;
            let result = build_core::ProcessRunner::default().run(&plan, Path::new(dir), None,
                None, Some(cancel), None).map_err(|e| e.to_string())?;
            GitOutput {
                code: match result.termination { build_core::ProcessTermination::Exited { code } => code, _ => -1 },
                stdout: String::from_utf8_lossy(&result.standard_output).into_owned(),
                stderr: String::from_utf8_lossy(&result.standard_error).into_owned(),
            }
        };
        if cancel.is_cancelled() { return Err("Git diff cancelled".into()); }
        if ok.contains(&output.code) { Ok(output.stdout) } else { Err(output.error_text()) }
    }
}

fn git_rewrites_worktree(args: &[String]) -> bool {
    git_core::changes_worktree(args)
}

fn collect_commit_diff(
    runner: &GitRunner, root: &str, context: &GitWorkspaceContext,
    cache: &Mutex<GitCache>, hash: &str, file: Option<GitCommitFile>,
    cancel: &build_core::CancellationToken,
) -> Result<Arc<git_core::GitDiffContent>, String> {
    if cancel.is_cancelled() { return Err("Git diff cancelled".into()); }
    let key = (hash.to_string(), file.as_ref().map(|f| f.path.clone()));
    {
        let mut cache = cache.lock().unwrap();
        cache.select(context, root);
        if let Some((_, content)) = cache.diffs.iter().find(|(cached, _)| cached == &key) {
            return Ok(content.clone());
        }
    }
    let sections = if let Some(file) = file {
        let raw = runner.read_diff(root, &git_core::file_diff_args(hash, &file.path), &[0], cancel)?;
        vec![git_core::GitDiffFileSection {
            binary: raw.contains("Binary files"), rows: git_core::parse_file_diff(&raw), file,
        }]
    } else {
        let raw = runner.read_diff(root, &git_core::commit_diff_args(hash), &[0], cancel)?;
        git_core::parse_commit_diff_sections(&raw)
    };
    if cancel.is_cancelled() { return Err("Git diff cancelled".into()); }
    let content = Arc::new(git_core::GitDiffContent::new(sections));
    let mut cache = cache.lock().unwrap();
    cache.select(context, root);
    cache.remember_diff(key, content.clone());
    Ok(content)
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
    runner: &GitRunner,
    context: &GitWorkspaceContext,
    cache: &Mutex<GitCache>,
) -> Result<Option<GitRefresh>, String> {
    let top = match runner.output_in(project_root, git_core::top_level_args().map(String::from).as_slice()) {
        Ok(output) => output,
        // An unreachable device must not masquerade as "not a repo".
        Err(e) => return Err(e),
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
        *cache.lock().unwrap() = GitCache::default();
        return Ok(None);
    }
    let root = top.stdout.trim().to_string();
    let status_raw = runner.run(&root, git_core::status_args().map(String::from).as_slice()).map_err(|e| e.to_string())?;
    // `show-ref` returns 1 with empty output in an unborn repository.
    let refs_raw = runner.run_codes(&root, git_core::refs_args().map(String::from).as_slice(), &[0, 1])?;
    let refs = git_core::history_refs(&refs_raw);
    let status = git_core::parse_status(&status_raw, &root);
    let mut cache = cache.lock().unwrap();
    cache.select(context, &root);
    let refs_changed = cache.refs.as_ref() != Some(&refs);
    let history_changed = refs_changed || cache.branch != status.branch
        || cache.history_at.map_or(true, |at| at.elapsed() >= Duration::from_secs(60));
    let commits = if history_changed {
        // A branch with no commits has no log; an empty refs fingerprint
        // lets us avoid that failing subprocess entirely.
        let raw = if refs.is_empty() { String::new() } else { runner.run(&root, git_core::log_args(80).as_slice())? };
        cache.history_at = Some(Instant::now());
        Some(git_core::parse_log(&raw))
    } else { None };
    let branches = refs_changed.then(|| git_core::branches_from_refs(&refs_raw));
    cache.refs = Some(refs);
    cache.branch = status.branch.clone();
    // Compare on the worker; an unchanged poll keeps the model and scroll position.
    let status = match cache.status.take() {
        Some(old) if *old == status => old,
        _ => Arc::new(status),
    };
    cache.status = Some(status.clone());
    Ok(Some(GitRefresh {
        status,
        commits,
        branches,
    }))
}

impl WorkspaceModel {
    pub(crate) fn git_context(&self) -> GitWorkspaceContext {
        GitWorkspaceContext {
            project: self.project_url.clone(),
            remote: self.remote.as_ref().map(|remote| Arc::as_ptr(&remote.sync) as usize),
            generation: self.git_context_seq.get(),
        }
    }

    /// `refreshGit()` — repo detection, status, branches, and log on a
    /// worker; the `GitRefreshed` message applies the result. Remote Git
    /// reads only the files uploaded by an explicit Save action.
    pub fn refresh_git(&self) {
        if self.console_section != crate::model::ConsoleSection::Git
            || !self.bottom_panel_visible || self.git_busy { return; }
        let Some(tx) = self.sink() else { return };
        let context = self.git_context();
        let request = self.git_refresh_seq.get().wrapping_add(1);
        let Some(root) = self.project_url.clone() else {
            self.git_refresh_seq.set(request);
            let _ = tx.send(WorkspaceMessage::GitRefreshed { context, request, result: Ok(None) });
            return;
        };
        if self.git_refresh_pending.replace(true) { return; }
        self.git_refresh_seq.set(request);
        let cache = self.git_cache.clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let result = collect_git(&root, &runner, &context, &cache);
            let _ = tx.send(WorkspaceMessage::GitRefreshed { context, request, result });
        });
    }

    /// `runGit` — sequential steps inside the repository; first failure
    /// wins. Remote ops pull afterward, so device-side writes
    /// (checkout, switch, pull) reach the mirror and open editors.
    fn git_operation(&mut self, steps: Vec<Vec<String>>, clear_commit: bool) {
        if self.git_busy { return; }
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        self.git_busy = true;
        self.git_error = None;
        self.git_refresh_seq.set(self.git_refresh_seq.get().wrapping_add(1));
        self.git_refresh_pending.set(false);
        let context = self.git_context();
        // Index/ref-only commands cannot rewrite the saved worktree.
        let needs_pull = steps.iter().any(|args| git_rewrites_worktree(args));
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let mut error = None;
            for args in &steps {
                if let Err(e) = runner.run(&root, args) {
                    error = Some(e);
                    break;
                }
            }
            if needs_pull { report_pull(&tx, &runner); }
            let _ = tx.send(WorkspaceMessage::GitOpFinished {
                context,
                error,
                clear_commit,
            });
        });
    }

    /// `gitOperationAny` — argv candidates tried in order until one exits 0
    /// (`restore --staged` → `reset HEAD` → `rm --cached` on a no-commit repo).
    fn git_operation_any(&mut self, candidates: Vec<Vec<String>>) {
        if self.git_busy { return; }
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        self.git_busy = true;
        self.git_error = None;
        self.git_refresh_seq.set(self.git_refresh_seq.get().wrapping_add(1));
        self.git_refresh_pending.set(false);
        let context = self.git_context();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
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
            let _ = tx.send(WorkspaceMessage::GitOpFinished {
                context,
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
        if self.git_busy { return; }
        let Some(tx) = self.sink() else { return };
        let Some(root) = self.project_url.clone() else {
            return;
        };
        self.git_busy = true;
        self.git_error = None;
        self.git_refresh_seq.set(self.git_refresh_seq.get().wrapping_add(1));
        self.git_refresh_pending.set(false);
        let context = self.git_context();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let error = runner
                .run_in(&root, git_core::init_args().map(String::from).as_slice())
                .err();
            let _ = tx.send(WorkspaceMessage::GitOpFinished {
                context,
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
        let context = self.git_context();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        // pi runs locally — its working directory is the mirror, since a
        // remote `status.root` does not exist on this filesystem.
        let work_dir = self.project_url.clone();
        std::thread::spawn(move || {
            let result = suggest_commit_message(&status, work_dir.as_deref(), &runner);
            let _ = tx.send(WorkspaceMessage::GitSuggestFinished { context, result });
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
        if self.git_expanded_commits.contains(&commit.full_hash) {
            self.git_expanded_commits.remove(&commit.full_hash);
            return;
        }
        self.git_expanded_commits.insert(commit.full_hash.clone());
        if self.git_commit_files.contains_key(&commit.full_hash) || self.git_commit_files_busy.contains(&commit.full_hash) {
            return;
        }
        let Some(root) = self.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let Some(tx) = self.sink() else { return };
        self.git_commit_files_busy.insert(commit.full_hash.clone());
        let hash = commit.full_hash.clone();
        let key = commit.full_hash.clone();
        let context = self.git_context();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let files = runner
                .run(&root, git_core::commit_files_args(&hash).as_slice())
                .map(|raw| git_core::parse_commit_files(&raw))
                .unwrap_or_default();
            let _ = tx.send(WorkspaceMessage::GitCommitFilesLoaded {
                context,
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
        let context = self.git_context();
        let cache = self.git_cache.clone();
        let cancel = self.git_diff_cancel.borrow().as_ref().unwrap().clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let result = collect_commit_diff(&runner, &root, &context, &cache, &hash, Some(file), &cancel);
            let _ = tx.send(WorkspaceMessage::GitDiffLoaded { context, id, root, result });
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
        let context = self.git_context();
        let cache = self.git_cache.clone();
        let cancel = self.git_diff_cancel.borrow().as_ref().unwrap().clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let result = collect_commit_diff(&runner, &root, &context, &cache, &hash, None, &cancel);
            let _ = tx.send(WorkspaceMessage::GitDiffLoaded { context, id, root, result });
        });
    }

    /// `openGitWorkingDiff` — clicking a Changes row opens its
    /// working-tree diff the same way; the row's ↗ button still opens
    /// the file itself. Remote diffs read the last explicitly saved files.
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
        let context = self.git_context();
        let cancel = self.git_diff_cancel.borrow().as_ref().unwrap().clone();
        let runner = GitRunner::for_workspace(self.remote.as_ref());
        std::thread::spawn(move || {
            let args = git_core::working_file_diff_args(&change);
            // `git diff --no-index` (untracked files) exits 1 on differences.
            let ok: &[i32] = if change.kind == GitChangeKind::Untracked {
                &[0, 1]
            } else {
                &[0]
            };
            let result = runner.read_diff(&root, args.as_slice(), ok, &cancel).map(|stdout| {
                Arc::new(git_core::GitDiffContent::new(vec![git_core::GitDiffFileSection {
                    binary: stdout.contains("Binary files"),
                    rows: git_core::parse_file_diff(&stdout),
                    file: GitCommitFile {
                        path: change.path.clone(),
                        kind: change.kind,
                    },
                }]))
            });
            let _ = tx.send(WorkspaceMessage::GitDiffLoaded { context, id, root, result });
        });
    }

    /// `closeGitDiff` — the diff overlay's close button.
    pub fn git_close_diff(&mut self) {
        if let Some(cancel) = self.git_diff_cancel.borrow_mut().take() { cancel.cancel(); }
        let old = self.git_diff.take();
        if old.is_some() { std::thread::spawn(move || drop(old)); }
    }

    /// `openGitDiff*` sessions get a monotonically increasing id — the
    /// dispatch drops worker results for superseded sessions (the Swift
    /// `gitDiff?.source == …` guards).
    fn next_git_diff_id(&self) -> u64 {
        let id = self.git_diff_seq.get() + 1;
        self.git_diff_seq.set(id);
        if let Some(cancel) = self.git_diff_cancel.replace(Some(build_core::CancellationToken::new())) { cancel.cancel(); }
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
        let context = context(&root);
        let cache = Mutex::new(GitCache::default());
        let first = collect_git(&root, &runner, &context, &cache).unwrap().unwrap().status;
        let next = collect_git(&root, &runner, &context, &cache).unwrap().unwrap().status;
        assert!(Arc::ptr_eq(&first, &next));
        assert_eq!(next.unstaged[0].path, path);
        run_git(&root, git_core::stage_args(path)).unwrap();
        let staged = collect_git(&root, &runner, &context, &cache).unwrap().unwrap().status;
        assert!(!Arc::ptr_eq(&staged, &next));
        assert_eq!(staged.staged[0].path, path);
        assert!(staged.unstaged.is_empty());
        run_git(&root, git_core::unstage_no_head_args(path)).unwrap();
        let unstaged = collect_git(&root, &runner, &context, &cache).unwrap().unwrap().status;
        assert!(unstaged.staged.is_empty());
        assert_eq!(unstaged.unstaged[0].path, path);
        std::fs::remove_dir_all(root).unwrap();
    }

    struct TestProject(PathBuf);
    impl TestProject {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("pitex-git-service-{}-{}", std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn init(&self) {
            run_git(&self.0, ["init", "-q", "--initial-branch=main"]).unwrap();
            run_git(&self.0, ["config", "user.name", "Git Service Test"]).unwrap();
            run_git(&self.0, ["config", "user.email", "git-service@example.invalid"]).unwrap();
        }
        fn commit(&self, message: &str) -> String {
            run_git(&self.0, ["add", "--all"]).unwrap();
            run_git(&self.0, ["commit", "-qm", message]).unwrap();
            run_git(&self.0, ["rev-parse", "HEAD"]).unwrap().trim().into()
        }
    }
    impl Drop for TestProject { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    fn context(root: &Path) -> GitWorkspaceContext {
        GitWorkspaceContext { project: Some(root.into()), remote: None, generation: 1 }
    }

    #[test]
    fn refresh_nonrepository_is_read_only_and_detects_explicit_init_and_nested_roots() {
        let project = TestProject::new();
        std::fs::write(project.0.join("main.tex"), "source").unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = GitRunner::CountingLocal(calls.clone());
        let cache = Mutex::new(GitCache::default());
        let context = context(&project.0);
        for _ in 0..3 { assert!(collect_git(&project.0, &runner, &context, &cache).unwrap().is_none()); }
        assert!(!project.0.join(".git").exists());
        assert_eq!(std::fs::read_to_string(project.0.join("main.tex")).unwrap(), "source");
        assert!(calls.lock().unwrap().iter().all(|args| args == &["rev-parse", "--show-toplevel"]));
        project.init();
        let parent_root = run_git(&project.0, git_core::top_level_args()).unwrap().trim().to_string();
        let unborn = collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap();
        assert!(unborn.commits.unwrap().is_empty());
        assert!(unborn.status.staged.is_empty(), "explicit init must not automatically track files");
        let child = project.0.join("nested");
        std::fs::create_dir_all(&child).unwrap();
        let child_context = super::GitWorkspaceContext { project: Some(child.clone()), ..context.clone() };
        assert_eq!(collect_git(&child, &runner, &child_context, &cache).unwrap().unwrap().status.root,
            parent_root);
        run_git(&child, ["init", "-q"]).unwrap();
        let nested_root = run_git(&child, git_core::top_level_args()).unwrap().trim().to_string();
        assert_eq!(collect_git(&child, &runner, &child_context, &cache).unwrap().unwrap().status.root,
            nested_root);
        std::fs::remove_dir_all(child.join(".git")).unwrap();
        assert_eq!(collect_git(&child, &runner, &child_context, &cache).unwrap().unwrap().status.root,
            parent_root);
        std::fs::remove_dir_all(project.0.join(".git")).unwrap();
        assert!(collect_git(&child, &runner, &child_context, &cache).unwrap().is_none());
    }

    #[test]
    fn history_poll_counts_and_invalidation_cover_refs_detached_head_and_relative_dates() {
        let project = TestProject::new(); project.init();
        std::fs::write(project.0.join("main.tex"), "one").unwrap();
        let first_hash = project.commit("initial");
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = GitRunner::CountingLocal(calls.clone());
        let cache = Mutex::new(GitCache::default());
        let context = context(&project.0);
        let first = collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap();
        assert_eq!(calls.lock().unwrap().len(), 4);
        assert_eq!(first.branches.unwrap(), ["main"]);
        let warm = collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap();
        assert_eq!(calls.lock().unwrap().len(), 7);
        assert!(warm.commits.is_none()); assert!(warm.branches.is_none());
        assert!(Arc::ptr_eq(&first.status, &warm.status));
        std::fs::write(project.0.join("main.tex"), "two").unwrap();
        let edited = collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap();
        assert!(edited.commits.is_none()); assert_eq!(edited.status.unstaged.len(), 1);
        project.commit("next");
        assert_eq!(collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap().commits.unwrap()[0].subject, "next");
        run_git(&project.0, ["branch", "feature"]).unwrap();
        assert_eq!(collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap().branches.unwrap(), ["feature", "main"]);
        run_git(&project.0, ["switch", "--detach", first_hash.as_str()]).unwrap();
        let detached = collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap();
        assert!(detached.commits.unwrap().iter().any(|c| c.is_head && c.full_hash == first_hash));
        cache.lock().unwrap().history_at = Some(Instant::now() - Duration::from_secs(61));
        assert!(collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap().commits.is_some());
        let other_context = GitWorkspaceContext { generation: 2, ..context.clone() };
        assert!(collect_git(&project.0, &runner, &other_context, &cache).unwrap().unwrap().commits.is_some());
        run_git(&project.0, ["switch", "--orphan", "empty"]).unwrap();
        let orphan = collect_git(&project.0, &runner, &context, &cache).unwrap().unwrap();
        assert_eq!(orphan.status.branch, "empty");
        assert!(!orphan.commits.unwrap().is_empty(), "all-ref history survives an unborn HEAD");
    }

    #[test]
    fn immutable_diff_cache_uses_one_command_and_preserves_file_kinds_and_special_paths() {
        let project = TestProject::new(); project.init();
        run_git(&project.0, ["config", "core.quotePath", "true"]).unwrap();
        // Use the same legal Unicode-and-space name on both platforms;
        // Git appends a TAB delimiter even when this header is quoted.
        let spaced_path = "한글 name.tex";
        std::fs::write(project.0.join(spaced_path), "space\n").unwrap();
        #[cfg(unix)] let path = "한글\tname\n.tex";
        #[cfg(windows)] let path = spaced_path;
        std::fs::write(project.0.join(path), "first\n").unwrap();
        let hash = project.commit("initial");
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = GitRunner::CountingLocal(calls.clone());
        let cache = Mutex::new(GitCache::default());
        let context = context(&project.0);
        let cancel = build_core::CancellationToken::new();
        let root = project.0.to_string_lossy();
        let first = collect_commit_diff(&runner, &root, &context, &cache, &hash, None, &cancel).unwrap();
        assert!(first.sections.iter().any(|section| section.file == GitCommitFile { path: path.into(), kind: GitChangeKind::Added }));
        assert!(first.sections.iter().any(|section| section.file == GitCommitFile { path: spaced_path.into(), kind: GitChangeKind::Added }));
        assert_eq!(calls.lock().unwrap().len(), 1);
        let next = collect_commit_diff(&runner, &root, &context, &cache, &hash, None, &cancel).unwrap();
        assert!(Arc::ptr_eq(&first, &next)); assert_eq!(calls.lock().unwrap().len(), 1);
        let names = runner.run(&root, &git_core::commit_files_args(&hash)).unwrap();
        assert!(git_core::parse_commit_files(&names).iter().any(|file| file.path == path));
        let other = GitWorkspaceContext { generation: 2, ..context };
        collect_commit_diff(&runner, &root, &other, &cache, &hash, None, &cancel).unwrap();
        assert_eq!(calls.lock().unwrap().len(), 3, "other workspace must not reuse cached content");
        cancel.cancel();
        assert!(collect_commit_diff(&runner, &root, &other, &cache, &hash, None, &cancel).is_err());
        assert_eq!(calls.lock().unwrap().len(), 3, "cancelled requests must not start git");
    }

    #[cfg(unix)]
    #[test]
    fn superseded_local_diff_kills_its_process_group() {
        let project = TestProject::new();
        let cancel = build_core::CancellationToken::new();
        let background_cancel = cancel.clone();
        std::thread::spawn(move || { std::thread::sleep(Duration::from_millis(50)); background_cancel.cancel(); });
        let started = Instant::now();
        let result = GitRunner::Local.read_diff(&project.0.to_string_lossy(),
            &["-c".into(), "alias.pause=!sleep 10".into(), "pause".into()], &[0], &cancel);
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(2), "cancelled child remained running");
    }

    #[test]
    fn mirror_refresh_is_limited_to_worktree_rewriting_operations() {
        for args in [git_core::stage_all_args().map(String::from).to_vec(),
            git_core::push_args().map(String::from).to_vec(), git_core::unstage_no_head_args("main.tex").to_vec(),
            git_core::unstage_all_args().map(String::from).to_vec(), git_core::init_args().map(String::from).to_vec()] {
            assert!(!git_rewrites_worktree(&args), "{args:?}");
        }
        for args in [git_core::commit_args("message", false), git_core::pull_args().map(String::from).to_vec(), git_core::switch_args("main").to_vec(),
            git_core::discard_args("main.tex").to_vec(), git_core::remove_file_args("main.tex").to_vec()] {
            assert!(git_rewrites_worktree(&args), "{args:?}");
        }
    }

    #[test]
    fn close_and_reopen_same_path_invalidates_git_context_and_cancels_diff() {
        let project = TestProject::new();
        let mut model = WorkspaceModel::new();
        model.project_url = Some(project.0.clone());
        let previous = model.git_context();
        model.next_git_diff_id();
        let cancel = model.git_diff_cancel.borrow().as_ref().unwrap().clone();
        let request = model.git_refresh_seq.get();
        model.close();
        model.project_url = Some(project.0.clone());
        assert_ne!(previous, model.git_context());
        assert_ne!(request, model.git_refresh_seq.get());
        assert!(cancel.is_cancelled());
    }

    #[test]
    fn diff_cache_budget_counts_empty_rows_and_fold_storage_and_evicts_consistently() {
        let mut cache = GitCache::default();
        let context_line = git_core::GitDiffLine {
            number: 1, text: String::new(), kind: git_core::GitDiffLineKind::Context,
        };
        let rows = (0..200_000).map(|_| git_core::GitDiffRow::Pair {
            left: Some(context_line.clone()), right: Some(context_line.clone()),
        }).collect();
        let large = Arc::new(git_core::GitDiffContent::new(vec![git_core::GitDiffFileSection {
            file: GitCommitFile { path: "empty-lines.tex".into(), kind: GitChangeKind::Modified },
            binary: false, rows,
        }]));
        assert_eq!(large.text_bytes, 0);
        assert!(large.estimated_bytes > GitCache::DIFF_BUDGET, "empty text still allocates rows and folded pairs");
        cache.remember_diff(("large".into(), None), large);
        assert!(cache.diffs.is_empty()); assert_eq!(cache.diff_bytes, 0);

        let payload = || Arc::new(git_core::GitDiffContent::new(vec![git_core::GitDiffFileSection {
            file: GitCommitFile { path: "large-metadata.tex".into(), kind: GitChangeKind::Modified },
            binary: false, rows: vec![git_core::GitDiffRow::Meta("x".repeat(9 * 1024 * 1024))],
        }]));
        cache.remember_diff(("first".into(), None), payload());
        assert_eq!(cache.diffs.len(), 1);
        cache.remember_diff(("second".into(), None), payload());
        assert_eq!(cache.diffs.len(), 1, "two payloads exceed the estimated budget");
        assert_eq!(cache.diffs[0].0.0, "second");
        assert_eq!(cache.diff_bytes, GitCache::diff_cost(&cache.diffs[0].0, &cache.diffs[0].1));
        assert!(cache.diff_bytes <= GitCache::DIFF_BUDGET);
    }

    #[test]
    #[ignore = "actual subprocess service benchmark; run with --ignored --nocapture"]
    fn git_service_performance() {
        let project = TestProject::new(); project.init();
        for i in 0..2_000 { std::fs::write(project.0.join(format!("file-{i}.tex")), "source\n").unwrap(); }
        let hash = project.commit("initial");
        for i in 0..20 { std::fs::write(project.0.join("file-0.tex"), format!("revision {i}\n")).unwrap(); project.commit(&format!("revision {i}")); }
        std::fs::write(project.0.join("file-1.tex"), "pending\n").unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = GitRunner::CountingLocal(calls.clone());
        let root = project.0.to_string_lossy();
        let baseline_started = Instant::now();
        for _ in 0..20 {
            runner.run(&root, &git_core::top_level_args().map(String::from)).unwrap();
            runner.run(&root, &git_core::status_args().map(String::from)).unwrap();
            runner.run(&root, &git_core::branch_args().map(String::from)).unwrap();
            runner.run(&root, &git_core::log_args(80)).unwrap();
        }
        let baseline = baseline_started.elapsed();
        assert_eq!(calls.lock().unwrap().len(), 80);
        calls.lock().unwrap().clear();
        let cache = Mutex::new(GitCache::default()); let context = context(&project.0);
        collect_git(&project.0, &runner, &context, &cache).unwrap();
        calls.lock().unwrap().clear();
        let started = Instant::now();
        for _ in 0..20 { collect_git(&project.0, &runner, &context, &cache).unwrap(); }
        let optimized = started.elapsed();
        assert_eq!(calls.lock().unwrap().len(), 60);
        assert!(calls.lock().unwrap().iter().all(|a| a[0] != "log" && a[0] != "branch"));
        calls.lock().unwrap().clear();
        let cancel = build_core::CancellationToken::new();
        for _ in 0..10 { collect_commit_diff(&runner, &root, &context, &cache, &hash, None, &cancel).unwrap(); }
        assert_eq!(calls.lock().unwrap().len(), 1);
        println!("Git service: 2000 files, 21 commits, 20 polls: baseline={baseline:?}/80 processes, cached={optimized:?}/60 processes; 10 immutable diff opens=1 process");
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
