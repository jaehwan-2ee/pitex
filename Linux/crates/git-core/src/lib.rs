//! Port of `Packages/TexCore/Sources/GitCore/GitSupport.swift` — git
//! porcelain/log parsing and argv builders for the Git Integration panel.
//! Pure functions only; the shell runs the produced argv off-thread.

/// One entry of `git status --porcelain=v1`: what happened to a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Conflicted,
}

impl GitChangeKind {
    /// Status letter shown next to the file name (VSCode-style badge).
    pub fn badge(self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Copied => "C",
            Self::TypeChanged => "T",
            Self::Untracked => "U",
            Self::Conflicted => "!",
        }
    }
}

/// A changed path, staged or unstaged (`original_path` only for renames/copies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitChange {
    pub path: String,
    pub original_path: Option<String>,
    pub kind: GitChangeKind,
    pub staged: bool,
}

/// One row of the commit graph (`git log` summary + ref decorations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommit {
    pub hash: String,
    pub subject: String,
    pub author: String,
    pub relative_date: String,
    /// Branch/tag decorations (`main`, `origin/main`, `tag: v1.2.0`, …).
    pub refs: Vec<String>,
    pub is_head: bool,
    /// Full 40-char id — `hash` is abbreviated for display only.
    pub full_hash: String,
    /// `%B` — subject + body, for "Copy Commit Message".
    pub message: String,
}

/// Repository snapshot for the Git Integration panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatus {
    pub root: String,
    pub repo_name: String,
    /// Current branch, or `HEAD` when detached.
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub staged: Vec<GitChange>,
    pub unstaged: Vec<GitChange>,
}

/// A changed path inside one commit (`git show --name-status`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitFile {
    pub path: String,
    pub kind: GitChangeKind,
}

/// One text line on one side of a side-by-side diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitDiffLineKind {
    Context,
    Removed,
    Added,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffLine {
    pub number: u32,
    pub text: String,
    pub kind: GitDiffLineKind,
}

/// A row of a commit diff. `Pair` rows render side by side — removed
/// lines on the left, added on the right, context on both; a `None`
/// side means that column stays empty for the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDiffRow {
    /// `diff --git`, `index`, `---`/`+++`, mode/rename notes, "Binary
    /// files differ" — shown dimmed across the full width.
    Meta(String),
    /// `@@ -a,b +c,d @@` section divider.
    Hunk(String),
    Pair {
        left: Option<GitDiffLine>,
        right: Option<GitDiffLine>,
    },
}

/// One aligned side-by-side row, used inside collapsed folds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffPair {
    pub left: Option<GitDiffLine>,
    pub right: Option<GitDiffLine>,
}

/// Render item for the diff viewer — `Pair` is a visible row, `Fold` a
/// collapsed run of unchanged lines (expandable), `Gap` a hunk boundary
/// covering lines git did not emit, `Note` a dimmed informational line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDiffItem {
    Pair {
        left: Option<GitDiffLine>,
        right: Option<GitDiffLine>,
    },
    Fold { id: u32, pairs: Vec<GitDiffPair> },
    Gap { old_lines: u32 },
    Note(String),
}

/// One file's rendered diff inside a commit diff session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffFileSection {
    pub file: GitCommitFile,
    pub binary: bool,
    pub rows: Vec<GitDiffRow>,
}

impl GitDiffFileSection {
    /// VSCode-style `+added −removed` counts for the file header.
    pub fn additions(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| match r {
                GitDiffRow::Pair { right: Some(l), .. } => l.kind == GitDiffLineKind::Added,
                _ => false,
            })
            .count()
    }
    pub fn deletions(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| match r {
                GitDiffRow::Pair { left: Some(l), .. } => l.kind == GitDiffLineKind::Removed,
                _ => false,
            })
            .count()
    }
}

pub fn top_level_args() -> [&'static str; 2] {
    ["rev-parse", "--show-toplevel"]
}
pub fn status_args() -> [&'static str; 6] {
    ["--no-optional-locks", "status", "--porcelain=v1", "-z", "--branch", "-uall"]
}
pub fn branch_args() -> [&'static str; 2] {
    ["branch", "--format=%(refname:short)"]
}
/// `hash | author | relative | decorations | subject`, records separated
/// by \x1e — parsing survives spaces/newlines inside subjects.
pub fn log_args(limit: usize) -> [String; 7] {
    [
        "log".into(),
        // `git-ai` authorship notes live under refs/notes — not user
        // history, so the graph skips them (`--exclude` must precede
        // the `--all` it filters).
        "--exclude=refs/notes/*".into(),
        "--all".into(),
        "-n".into(),
        limit.to_string(),
        "--date=relative".into(),
        "--pretty=tformat:%h%x1f%an%x1f%ar%x1f%D%x1f%s%x1f%H%x1f%B%x1e".into(),
    ]
}

pub fn stage_args(path: &str) -> [String; 3] {
    ["add".into(), "--".into(), path.into()]
}
/// `restore --staged` fails on repositories with no commits — callers
/// retry with `reset HEAD --` (see `unstage_fallback_args`).
pub fn unstage_args(path: &str) -> [String; 4] {
    [
        "restore".into(),
        "--staged".into(),
        "--".into(),
        path.into(),
    ]
}
pub fn unstage_fallback_args(path: &str) -> [String; 4] {
    ["reset".into(), "HEAD".into(), "--".into(), path.into()]
}
/// No-HEAD repos can only unstage by dropping the index entry.
pub fn unstage_no_head_args(path: &str) -> [String; 4] {
    ["rm".into(), "--cached".into(), "--".into(), path.into()]
}
pub fn stage_all_args() -> [&'static str; 2] {
    ["add", "--all"]
}
pub fn unstage_all_args() -> [&'static str; 1] {
    ["reset"]
}
pub fn unstage_all_no_head_args() -> [&'static str; 4] {
    ["rm", "-r", "--cached", "."]
}
/// `commit -a` stages tracked files only; untracked rows need an
/// explicit `add` first (same as VSCode's "Commit All").
pub fn commit_args(message: &str, all: bool) -> Vec<String> {
    if all {
        vec!["commit".into(), "-am".into(), message.into()]
    } else {
        vec!["commit".into(), "-m".into(), message.into()]
    }
}
pub fn fetch_args() -> [&'static str; 3] {
    ["fetch", "--all", "--prune"]
}
pub fn pull_args() -> [&'static str; 1] {
    ["pull"]
}
pub fn push_args() -> [&'static str; 1] {
    ["push"]
}
pub fn init_args() -> [&'static str; 1] {
    ["init"]
}
pub fn switch_args(branch: &str) -> [String; 2] {
    ["switch".into(), branch.into()]
}
/// `git switch -c` — `at` pins the start point (graph context menu's
/// "Create Branch" branches off the selected commit, like VSCode).
pub fn create_branch_args(name: &str, at: Option<&str>) -> Vec<String> {
    let mut args = vec!["switch".into(), "-c".into(), name.into()];
    if let Some(at) = at {
        args.push(at.into());
    }
    args
}
/// Unstaged tracked discard; staged and untracked paths need
/// `discard_staged_args`/`clean_args` instead.
pub fn discard_args(path: &str) -> [String; 3] {
    ["checkout".into(), "--".into(), path.into()]
}
/// Staged discard — restores index+worktree from HEAD (fails for paths
/// added since the last commit; those use `remove_file_args`).
pub fn discard_staged_args(path: &str) -> [String; 4] {
    ["checkout".into(), "HEAD".into(), "--".into(), path.into()]
}
/// Staged-new discard — drops index entry and the worktree file.
pub fn remove_file_args(path: &str) -> [String; 4] {
    ["rm".into(), "-f".into(), "--".into(), path.into()]
}
pub fn clean_args(path: &str) -> [String; 4] {
    ["clean".into(), "-f".into(), "--".into(), path.into()]
}
/// `--staged` describes the pending commit; plain `diff` covers the
/// commit-everything fallback (untracked files produce no diff — the
/// caller names them in the prompt's file list).
pub fn diff_args(staged: bool) -> [&'static str; 2] {
    if staged {
        ["diff", "--staged"]
    } else {
        ["diff", "--"]
    }
}

/// `git show` file list for one commit: `STATUS<TAB>path` rows
/// (`R100<TAB>old<TAB>new` carries both paths). `--diff-merges=
/// first-parent` diffs merge commits against the first parent instead
/// of printing the (usually empty) combined diff.
pub fn commit_files_args(hash: &str) -> [String; 5] {
    [
        "show".into(),
        "--format=".into(),
        "--name-status".into(),
        "--diff-merges=first-parent".into(),
        hash.into(),
    ]
}

/// Context window for commit diffs: large enough that `git show`
/// emits whole files, letting the view fold unchanged regions itself
/// (VSCode's `hideUnchangedRegions` behavior needs the full text).
pub const DIFF_CONTEXT_LINES: usize = 100_000;

/// One file's patch inside a commit (`diff --git` header + hunks).
pub fn file_diff_args(hash: &str, path: &str) -> Vec<String> {
    vec![
        "show".into(),
        "--format=".into(),
        "--diff-merges=first-parent".into(),
        format!("--unified={DIFF_CONTEXT_LINES}"),
        hash.into(),
        "--".into(),
        path.into(),
    ]
}

/// Every file's patch inside a commit — the "Open Changes" payload.
pub fn commit_diff_args(hash: &str) -> Vec<String> {
    vec![
        "show".into(),
        "--format=".into(),
        "--diff-merges=first-parent".into(),
        format!("--unified={DIFF_CONTEXT_LINES}"),
        hash.into(),
    ]
}

/// One Changes row's patch: index ↔ worktree normally, HEAD ↔ index
/// when `staged` (rename rows list both paths so git pairs them
/// instead of showing add+delete), and `--no-index` against
/// `/dev/null` for untracked files — which exits 1 when the file
/// differs, so callers treat codes 0 and 1 as success there.
pub fn working_file_diff_args(change: &GitChange) -> Vec<String> {
    if change.kind == GitChangeKind::Untracked {
        return vec![
            "diff".into(),
            "--no-index".into(),
            format!("--unified={DIFF_CONTEXT_LINES}"),
            "--".into(),
            "/dev/null".into(),
            change.path.clone(),
        ];
    }
    if change.staged {
        let mut args = vec![
            "diff".into(),
            "--staged".into(),
            format!("--unified={DIFF_CONTEXT_LINES}"),
            "--".into(),
        ];
        if let Some(original) = &change.original_path {
            args.push(original.clone());
        }
        args.push(change.path.clone());
        return args;
    }
    vec![
        "diff".into(),
        format!("--unified={DIFF_CONTEXT_LINES}"),
        "--".into(),
        change.path.clone(),
    ]
}

/// `commit_files_args` output → badge + path rows. Rename/copy rows end
/// with the new path — the diff targets that name.
pub fn parse_commit_files(raw: &str) -> Vec<GitCommitFile> {
    raw.split('\n')
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let code = fields.next().and_then(|c| c.chars().next())?;
            let path = fields.last()?;
            let path = path
                .strip_prefix('"')
                .and_then(|p| p.strip_suffix('"'))
                .unwrap_or(path);
            Some(GitCommitFile {
                path: path.to_string(),
                kind: kind_from(code),
            })
        })
        .collect()
}

/// Unified diff (`file_diff_args` output) → aligned side-by-side rows.
/// `---`/`+++` headers land before the first `@@`, so only lines inside
/// a hunk count as removed/added/context. Within each change block the
/// removed lines pair with the added lines in order; uneven tails
/// leave the opposite side empty.
pub fn parse_file_diff(raw: &str) -> Vec<GitDiffRow> {
    let mut rows = Vec::new();
    let mut removed: Vec<GitDiffLine> = Vec::new();
    let mut added: Vec<GitDiffLine> = Vec::new();
    let mut old_line = 0u32;
    let mut new_line = 0u32;
    let mut in_hunk = false;

    fn flush(rows: &mut Vec<GitDiffRow>, removed: &mut Vec<GitDiffLine>, added: &mut Vec<GitDiffLine>) {
        let n = removed.len().max(added.len());
        let mut r = removed.drain(..);
        let mut a = added.drain(..);
        for _ in 0..n {
            rows.push(GitDiffRow::Pair {
                left: r.next(),
                right: a.next(),
            });
        }
    }

    for line in raw.split('\n') {
        if line.starts_with("@@") {
            flush(&mut rows, &mut removed, &mut added);
            (old_line, new_line) = hunk_starts(line);
            rows.push(GitDiffRow::Hunk(line.to_string()));
            in_hunk = true;
        } else if in_hunk && line.starts_with('-') {
            removed.push(GitDiffLine {
                number: old_line,
                text: line[1..].to_string(),
                kind: GitDiffLineKind::Removed,
            });
            old_line += 1;
        } else if in_hunk && line.starts_with('+') {
            added.push(GitDiffLine {
                number: new_line,
                text: line[1..].to_string(),
                kind: GitDiffLineKind::Added,
            });
            new_line += 1;
        } else if in_hunk && line.starts_with(' ') {
            flush(&mut rows, &mut removed, &mut added);
            let text = line[1..].to_string();
            rows.push(GitDiffRow::Pair {
                left: Some(GitDiffLine {
                    number: old_line,
                    text: text.clone(),
                    kind: GitDiffLineKind::Context,
                }),
                right: Some(GitDiffLine {
                    number: new_line,
                    text,
                    kind: GitDiffLineKind::Context,
                }),
            });
            old_line += 1;
            new_line += 1;
        } else if line.starts_with('\\') {
            flush(&mut rows, &mut removed, &mut added);
            rows.push(GitDiffRow::Meta(line.to_string()));
        } else if !line.is_empty() {
            flush(&mut rows, &mut removed, &mut added);
            // A new file block ends the current hunk so its ---/+++
            // headers are never read as removed/added lines.
            if line.starts_with("diff --git") || line.starts_with("Binary files") {
                in_hunk = false;
            }
            rows.push(GitDiffRow::Meta(line.to_string()));
        }
    }
    flush(&mut rows, &mut removed, &mut added);
    rows
}

/// `parse_file_diff` rows → render items for the diff viewer: unchanged
/// runs longer than `2 * edge_context` fold into a `fold` bar (VSCode's
/// collapsed unchanged regions), header/meta noise disappears, and a
/// surviving `@@` boundary (a file longer than `DIFF_CONTEXT_LINES`)
/// becomes a non-expandable `gap` marker.
pub fn display_items(rows: &[GitDiffRow], edge_context: usize) -> Vec<GitDiffItem> {
    let mut items = Vec::new();
    let mut run: Vec<GitDiffPair> = Vec::new();
    let mut fold_id = 0u32;
    let mut last_old: Option<u32> = None;

    fn flush(
        items: &mut Vec<GitDiffItem>,
        run: &mut Vec<GitDiffPair>,
        fold_id: &mut u32,
        edge_context: usize,
    ) {
        if run.len() <= 2 * edge_context + 1 {
            for p in run.drain(..) {
                items.push(GitDiffItem::Pair {
                    left: p.left,
                    right: p.right,
                });
            }
            return;
        }
        let tail = run.split_off(run.len() - edge_context);
        let middle = run.split_off(edge_context);
        for p in run.drain(..) {
            items.push(GitDiffItem::Pair {
                left: p.left,
                right: p.right,
            });
        }
        items.push(GitDiffItem::Fold {
            id: *fold_id,
            pairs: middle,
        });
        *fold_id += 1;
        for p in tail {
            items.push(GitDiffItem::Pair {
                left: p.left,
                right: p.right,
            });
        }
    }

    for row in rows {
        match row {
            GitDiffRow::Pair { left, right } => {
                if left.as_ref().map(|l| l.kind) == Some(GitDiffLineKind::Context)
                    && right.as_ref().map(|r| r.kind) == Some(GitDiffLineKind::Context)
                {
                    run.push(GitDiffPair {
                        left: left.clone(),
                        right: right.clone(),
                    });
                } else {
                    flush(&mut items, &mut run, &mut fold_id, edge_context);
                    items.push(GitDiffItem::Pair {
                        left: left.clone(),
                        right: right.clone(),
                    });
                }
                if let Some(l) = left {
                    last_old = Some(l.number);
                }
            }
            GitDiffRow::Hunk(header) => {
                flush(&mut items, &mut run, &mut fold_id, edge_context);
                let start = hunk_starts(header).0;
                let hidden = start.saturating_sub(last_old.unwrap_or(0) + 1);
                if hidden > 0 {
                    items.push(GitDiffItem::Gap { old_lines: hidden });
                }
            }
            GitDiffRow::Meta(text) => {
                if text.starts_with('\\') {
                    flush(&mut items, &mut run, &mut fold_id, edge_context);
                    items.push(GitDiffItem::Note(text.clone()));
                }
            }
        }
    }
    flush(&mut items, &mut run, &mut fold_id, edge_context);
    items
}

/// `commit_diff_args` output → one section per file, as
/// `(path, binary, rows)`. The path comes from the `+++ b/` header
/// (deleted files fall back to `--- a/`); kind is resolved by the
/// caller against the `--name-status` list.
pub fn parse_commit_diff(raw: &str) -> Vec<(String, bool, Vec<GitDiffRow>)> {
    fn block(text: &str) -> (String, bool) {
        let mut path = String::new();
        let mut binary = false;
        for line in text.split('\n') {
            if let Some(rest) = line.strip_prefix("+++ b/") {
                path = unquote_git_path(rest);
            } else if line.starts_with("Binary files") {
                binary = true;
            } else if path.is_empty() && line.starts_with("diff --git ") {
                // Deleted files keep only the `a/` side in the header.
                if let Some(rest) = line.split(' ').next_back().and_then(|l| l.strip_prefix("b/")) {
                    path = unquote_git_path(rest);
                }
            }
        }
        if path.is_empty() {
            for line in text.split('\n') {
                if let Some(rest) = line.strip_prefix("--- a/") {
                    path = unquote_git_path(rest);
                    break;
                }
            }
        }
        (path, binary)
    }

    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in raw.split('\n') {
        if line.starts_with("diff --git ") && !current.is_empty() {
            blocks.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
        .into_iter()
        .map(|text| {
            let (path, binary) = block(&text);
            (path, binary, parse_file_diff(&text))
        })
        .collect()
}

/// `git show`'s C-style quoting of non-ASCII paths: `"a/\303\244"` →
/// UTF-8 octal escapes decoded back to the real name.
pub fn unquote_git_path(path: &str) -> String {
    let p = path;
    let p = if p.len() > 1 && p.starts_with('"') && p.ends_with('"') {
        &p[1..p.len() - 1]
    } else {
        p
    };
    if !p.contains('\\') {
        return p.to_string();
    }
    let b = p.as_bytes();
    let mut bytes: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' {
            let Some(&n) = b.get(i + 1) else { break };
            if n == b'\\' || n == b'"' {
                bytes.push(n);
                i += 2;
            } else if n.is_ascii_digit() {
                let start = i + 1;
                let mut end = start;
                while end < b.len() && end - start < 3 && b[end].is_ascii_digit() {
                    end += 1;
                }
                match u8::from_str_radix(&p[start..end], 8) {
                    Ok(byte) => {
                        bytes.push(byte);
                        i = end;
                    }
                    Err(_) => {
                        bytes.push(c);
                        i += 1;
                    }
                }
            } else {
                bytes.push(c);
                i += 1;
            }
        } else {
            bytes.push(c);
            i += 1;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Common leading/trailing character counts for a changed line pair —
/// the intra-line highlight ranges in a side-by-side diff (VSCode's
/// darker change regions inside a modified line). Counts are Unicode
/// scalar values (Rust `chars`), close enough to grapheme clusters for
/// the shading range.
pub fn common_affixes(old: &str, new: &str) -> (usize, usize) {
    let a: Vec<char> = old.chars().collect();
    let b: Vec<char> = new.chars().collect();
    let mut prefix = 0;
    while prefix < a.len() && prefix < b.len() && a[prefix] == b[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < a.len() - prefix && suffix < b.len() - prefix && a[a.len() - 1 - suffix] == b[b.len() - 1 - suffix] {
        suffix += 1;
    }
    (prefix, suffix)
}

/// `@@ -old[,n] +new[,n] @@` → the starting line number of each side.
fn hunk_starts(header: &str) -> (u32, u32) {
    fn first_digits(s: &str) -> u32 {
        let mut digits = String::new();
        for c in s.chars() {
            if c.is_ascii_digit() {
                digits.push(c);
            } else if !digits.is_empty() {
                break;
            }
        }
        digits.parse().unwrap_or(0)
    }
    let inner = &header[2..];
    let old = first_digits(inner);
    let new = inner.find('+').map(|i| first_digits(&inner[i..])).unwrap_or(0);
    (old, new)
}

fn kind_from(code: char) -> GitChangeKind {
    match code {
        'A' => GitChangeKind::Added,
        'D' => GitChangeKind::Deleted,
        'R' => GitChangeKind::Renamed,
        'C' => GitChangeKind::Copied,
        'T' => GitChangeKind::TypeChanged,
        'U' => GitChangeKind::Conflicted,
        _ => GitChangeKind::Modified,
    }
}

/// `## main...origin/main [ahead 1, behind 2]` / `## No commits yet on main`.
fn parse_branch_header(header: &str) -> (String, Option<String>, u32, u32) {
    let mut branch = header;
    let mut ahead = 0;
    let mut behind = 0;
    if let Some(bracket) = header.rfind(" [") {
        branch = &header[..bracket];
        for part in header[bracket + 2..].trim_end_matches(']').split(',') {
            let piece = part.trim();
            if let Some(n) = piece.rsplit(' ').next().and_then(|s| s.parse::<u32>().ok()) {
                if piece.starts_with("ahead") {
                    ahead = n;
                }
                if piece.starts_with("behind") {
                    behind = n;
                }
            }
        }
    }
    let mut upstream = None;
    if let Some(dots) = branch.find("...") {
        upstream = Some(branch[dots + 3..].to_string());
        branch = &branch[..dots];
    }
    if let Some(rest) = branch.strip_prefix("No commits yet on ") {
        branch = rest;
    }
    if branch.starts_with("HEAD (") {
        branch = "HEAD";
    }
    (branch.to_string(), upstream, ahead, behind)
}

/// `git status --porcelain=v1 -z --branch` output → status fields.
/// In `-z` format each record is `XY path\0`; renames/copies append the
/// source path as the following bare field.
pub fn parse_status(raw: &str, root: &str) -> GitStatus {
    let mut branch = String::new();
    let mut upstream = None;
    let mut ahead = 0;
    let mut behind = 0;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();

    let mut fields = raw.split('\0').filter(|f| !f.is_empty());
    while let Some(field) = fields.next() {
        if let Some(header) = field.strip_prefix("## ") {
            (branch, upstream, ahead, behind) = parse_branch_header(header);
            continue;
        }
        let bytes = field.as_bytes();
        if bytes.len() < 4 || bytes[2] != b' ' || !bytes[0].is_ascii() || !bytes[1].is_ascii() {
            continue;
        }
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let path = field[3..].to_string();
        let original_path = if matches!(x, 'R' | 'C') { fields.next().map(str::to_string) } else { None };
        if x == '?' && y == '?' {
            unstaged.push(GitChange {
                path,
                original_path: None,
                kind: GitChangeKind::Untracked,
                staged: false,
            });
            continue;
        }
        if x == '!' {
            continue;
        }
        if matches!(
            (x, y),
            ('D', 'D')
                | ('A', 'U')
                | ('U', 'D')
                | ('U', 'A')
                | ('D', 'U')
                | ('A', 'A')
                | ('U', 'U')
        ) {
            unstaged.push(GitChange {
                path,
                original_path: None,
                kind: GitChangeKind::Conflicted,
                staged: false,
            });
            continue;
        }
        if x != ' ' {
            staged.push(GitChange {
                path: path.clone(),
                original_path,
                kind: kind_from(x),
                staged: true,
            });
        }
        if y != ' ' {
            unstaged.push(GitChange {
                path,
                original_path: None,
                kind: kind_from(y),
                staged: false,
            });
        }
    }
    let repo_name = root
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(root)
        .to_string();
    GitStatus {
        root: root.to_string(),
        repo_name,
        branch,
        upstream,
        ahead,
        behind,
        staged,
        unstaged,
    }
}

/// `log_args` output → commit rows; `%D` decorations become ref chips.
pub fn parse_log(raw: &str) -> Vec<GitCommit> {
    raw.split('\x1e')
        .filter(|r| !r.is_empty())
        .filter_map(|record| {
            let fields: Vec<&str> = record.split('\x1f').collect();
            if fields.len() < 5 {
                return None;
            }
            let hash = fields[0].trim();
            if hash.is_empty() {
                return None;
            }
            let mut refs = Vec::new();
            let mut is_head = false;
            for decoration in fields[3].split(',') {
                let mut d = decoration.trim();
                if let Some(rest) = d.strip_prefix("HEAD") {
                    is_head = true;
                    if let Some(arrow) = rest.strip_prefix(" -> ") {
                        d = arrow;
                    } else {
                        continue;
                    }
                }
                if !d.is_empty() {
                    refs.push(d.to_string());
                }
            }
            Some(GitCommit {
                hash: hash.to_string(),
                subject: fields[4].to_string(),
                author: fields[1].to_string(),
                relative_date: fields[2].to_string(),
                refs,
                is_head,
                full_hash: fields
                    .get(5)
                    .map(|h| h.trim().to_string())
                    .filter(|h| !h.is_empty())
                    .unwrap_or_else(|| hash.to_string()),
                // %B is the last field; rejoining tail fragments keeps a
                // (pathological) \x1f inside the body from truncating it.
                message: if fields.len() > 6 {
                    fields[6..].join("\u{1f}").trim_matches('\n').to_string()
                } else {
                    fields[4].to_string()
                },
            })
        })
        .collect()
}

pub fn parse_branches(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_parses_staged_unstaged_untracked() {
        let raw = "## main...origin/main [ahead 1, behind 2]\0 M modified.tex\0M  staged.tex\0?? new.tex\0";
        let s = parse_status(raw, "/tmp/paper");
        assert_eq!(s.branch, "main");
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (1, 2));
        assert_eq!(s.repo_name, "paper");
        assert_eq!(
            s.staged,
            vec![GitChange {
                path: "staged.tex".into(),
                original_path: None,
                kind: GitChangeKind::Modified,
                staged: true
            }]
        );
        assert_eq!(s.unstaged.len(), 2);
        assert_eq!(s.unstaged[0].kind, GitChangeKind::Modified);
        assert_eq!(s.unstaged[1].kind, GitChangeKind::Untracked);
    }

    #[test]
    fn status_parses_rename_pair_and_conflicts() {
        let raw = "## feature\0R  new.tex\0old.tex\0UU clash.tex\0";
        let s = parse_status(raw, "/r");
        assert_eq!(s.branch, "feature");
        assert_eq!(s.ahead, 0);
        assert_eq!(s.staged[0].kind, GitChangeKind::Renamed);
        assert_eq!(s.staged[0].original_path.as_deref(), Some("old.tex"));
        assert_eq!(s.unstaged[0].kind, GitChangeKind::Conflicted);
    }

    #[test]
    fn status_preserves_unicode_and_control_characters_in_paths() {
        let renamed = "한글/새 👩🏽‍💻\tname\n.tex";
        let original = "옛 이름/e\u{301}.tex";
        let s = parse_status(&format!("## main\0RM {renamed}\0{original}\0?? 123.tex\0!! ignored\0X\0"), "/repo");
        assert_eq!(s.staged[0].path, renamed);
        assert_eq!(s.staged[0].original_path.as_deref(), Some(original));
        assert_eq!(s.unstaged.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(), vec![renamed, "123.tex"]);
    }

    #[test]
    fn status_handles_no_commits_and_detached() {
        let s = parse_status("## No commits yet on main\0?? a.tex\0", "/r");
        assert_eq!(s.branch, "main");
        let s = parse_status("## HEAD (no branch)\0", "/r");
        assert_eq!(s.branch, "HEAD");
    }

    #[test]
    fn log_parses_refs_and_head() {
        // %h %an %ar %D %s %H %B — %B repeats the subject and adds the body.
        let raw = "abc1234\x1fJane\x1f2 days ago\x1fHEAD -> main, origin/main, tag: v1.2.0\x1fRelease v1.2.0\x1fabc1234fullhash00\x1fRelease v1.2.0\n\nBody line.\n\x1edef5678\x1fBob\x1f3 days ago\x1f\x1fAdd feature\x1fdef5678fullhash00\x1fAdd feature\n\x1e";
        let commits = parse_log(raw);
        assert_eq!(commits.len(), 2);
        assert!(commits[0].is_head);
        assert_eq!(commits[0].refs, ["main", "origin/main", "tag: v1.2.0"]);
        assert_eq!(commits[0].subject, "Release v1.2.0");
        assert_eq!(commits[0].full_hash, "abc1234fullhash00");
        assert_eq!(commits[0].message, "Release v1.2.0\n\nBody line.");
        assert!(!commits[1].is_head);
        assert!(commits[1].refs.is_empty());
        assert_eq!(commits[1].message, "Add feature");
    }

    #[test]
    fn branches_parse_lines() {
        assert_eq!(parse_branches("main\nfeature\n"), ["main", "feature"]);
    }

    fn ctx(n: u32) -> GitDiffLine {
        GitDiffLine {
            number: n,
            text: format!("c{n}"),
            kind: GitDiffLineKind::Context,
        }
    }

    #[test]
    fn commit_files_parse_name_status() {
        let files = parse_commit_files("M\tmain.tex\nA\trefs.bib\nR100\told.tex\tnew.tex\n");
        assert_eq!(
            files,
            vec![
                GitCommitFile { path: "main.tex".into(), kind: GitChangeKind::Modified },
                GitCommitFile { path: "refs.bib".into(), kind: GitChangeKind::Added },
                // Rename rows carry the new path last — the diff targets it.
                GitCommitFile { path: "new.tex".into(), kind: GitChangeKind::Renamed },
            ]
        );
        assert!(parse_commit_files("").is_empty());
    }

    #[test]
    fn file_diff_aligns_side_by_side() {
        let raw = "diff --git a/main.tex b/main.tex\nindex 1111111..2222222 100644\n--- a/main.tex\n+++ b/main.tex\n@@ -2,4 +2,5 @@ context\n keep\n-old line\n-another old\n+new line\n tail\n\\ No newline at end of file\n";
        let rows = parse_file_diff(raw);
        // 4 header metas + hunk + context + 2 change pairs + context + \-meta
        assert_eq!(rows.len(), 10);
        assert!(matches!(rows[4], GitDiffRow::Hunk(_)), "row 4 should be the @@ header");
        let GitDiffRow::Pair { left: l0, right: r0 } = &rows[5] else { panic!("row 5") };
        assert_eq!(
            l0.as_ref(),
            Some(&GitDiffLine { number: 2, text: "keep".into(), kind: GitDiffLineKind::Context })
        );
        assert_eq!(r0.as_ref().map(|l| l.number), Some(2));
        let GitDiffRow::Pair { left: l1, right: r1 } = &rows[6] else { panic!("row 6") };
        assert_eq!(
            l1.as_ref(),
            Some(&GitDiffLine { number: 3, text: "old line".into(), kind: GitDiffLineKind::Removed })
        );
        assert_eq!(
            r1.as_ref(),
            Some(&GitDiffLine { number: 3, text: "new line".into(), kind: GitDiffLineKind::Added })
        );
        let GitDiffRow::Pair { left: l2, right: r2 } = &rows[7] else { panic!("row 7") };
        assert_eq!(
            l2.as_ref(),
            Some(&GitDiffLine { number: 4, text: "another old".into(), kind: GitDiffLineKind::Removed })
        );
        assert!(r2.is_none());
        let GitDiffRow::Pair { left: l3, right: r3 } = &rows[8] else { panic!("row 8") };
        assert_eq!(l3.as_ref().map(|l| l.number), Some(5));
        assert_eq!(r3.as_ref().map(|l| l.number), Some(4));
        assert!(matches!(rows[9], GitDiffRow::Meta(_)), "row 9 should be the \\ meta");
    }

    #[test]
    fn file_diff_parses_no_index_new_file() {
        // `git diff --no-index /dev/null path` (untracked Changes rows)
        // emits a normal new-file block: `/dev/null` on the `---` side.
        let raw = "diff --git a/untracked.tex b/untracked.tex\nnew file mode 100644\nindex 0000000..d5a09df\n--- /dev/null\n+++ b/untracked.tex\n@@ -0,0 +1 @@\n+brand new\n";
        let rows = parse_file_diff(raw);
        // 5 header metas + hunk + one added pair.
        assert_eq!(rows.len(), 7);
        let GitDiffRow::Pair { left, right } = &rows[6] else { panic!("last row should be the added line") };
        assert!(left.is_none());
        assert_eq!(
            right.as_ref(),
            Some(&GitDiffLine { number: 1, text: "brand new".into(), kind: GitDiffLineKind::Added })
        );
    }

    #[test]
    fn file_diff_meta_only_for_binary() {
        let rows = parse_file_diff("diff --git a/a.pdf b/a.pdf\nBinary files differ\n");
        assert_eq!(
            rows,
            vec![
                GitDiffRow::Meta("diff --git a/a.pdf b/a.pdf".into()),
                GitDiffRow::Meta("Binary files differ".into()),
            ]
        );
    }

    #[test]
    fn commit_diff_splits_into_file_sections() {
        let raw = "diff --git a/a.tex b/a.tex\nindex 1111111..2222222 100644\n--- a/a.tex\n+++ b/a.tex\n@@ -1,2 +1,2 @@\n-old a\n+new a\n same\ndiff --git a/b.tex b/b.tex\nindex 3333333..4444444 100644\n--- a/b.tex\n+++ b/b.tex\n@@ -1,1 +1,2 @@\n keep b\n+added b\ndiff --git a/pic.pdf b/pic.pdf\nBinary files differ\n";
        let sections = parse_commit_diff(raw);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].0, "a.tex");
        assert!(!sections[0].1);
        // One removal+one addition aligned into a single pair row.
        let section0 = GitDiffFileSection {
            file: GitCommitFile { path: "a.tex".into(), kind: GitChangeKind::Modified },
            binary: sections[0].1,
            rows: sections[0].2.clone(),
        };
        assert_eq!(section0.additions(), 1);
        assert_eq!(section0.deletions(), 1);
        assert_eq!(sections[1].0, "b.tex");
        assert!(sections[2].1);
        assert_eq!(sections[2].0, "pic.pdf");
    }

    #[test]
    fn commit_diff_detects_binary_by_marker_alone() {
        let raw = "diff --git a/x.bin b/x.bin\nindex a..b 100644\nBinary files a/x.bin and b/x.bin differ\n";
        let sections = parse_commit_diff(raw);
        assert_eq!(sections.len(), 1);
        assert!(sections[0].1);
        assert_eq!(sections[0].0, "x.bin");
    }

    #[test]
    fn commit_diff_deleted_file_falls_back_to_old_path() {
        // Deleted files have `+++ /dev/null`, so the path comes from the
        // `diff --git` header's `b/` side instead.
        let raw = "diff --git a/gone.tex b/gone.tex\ndeleted file mode 100644\nindex 1111111..0000000\n--- a/gone.tex\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-bye\n";
        let sections = parse_commit_diff(raw);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].0, "gone.tex");
    }

    #[test]
    fn working_file_diff_args_forms() {
        let unified = format!("--unified={DIFF_CONTEXT_LINES}");
        // Unstaged tracked: index ↔ worktree.
        assert_eq!(
            working_file_diff_args(&GitChange {
                path: "a.tex".into(),
                original_path: None,
                kind: GitChangeKind::Modified,
                staged: false,
            }),
            vec!["diff", unified.as_str(), "--", "a.tex"]
        );
        // Unstaged deleted is the same form.
        assert_eq!(
            working_file_diff_args(&GitChange {
                path: "gone.tex".into(),
                original_path: None,
                kind: GitChangeKind::Deleted,
                staged: false,
            }),
            vec!["diff", unified.as_str(), "--", "gone.tex"]
        );
        // Staged: HEAD ↔ index.
        assert_eq!(
            working_file_diff_args(&GitChange {
                path: "a.tex".into(),
                original_path: None,
                kind: GitChangeKind::Modified,
                staged: true,
            }),
            vec!["diff", "--staged", unified.as_str(), "--", "a.tex"]
        );
        // Staged renames list both paths so git pairs them.
        assert_eq!(
            working_file_diff_args(&GitChange {
                path: "new.tex".into(),
                original_path: Some("old.tex".into()),
                kind: GitChangeKind::Renamed,
                staged: true,
            }),
            vec!["diff", "--staged", unified.as_str(), "--", "old.tex", "new.tex"]
        );
        // Untracked: --no-index against /dev/null (exits 1 on differences).
        assert_eq!(
            working_file_diff_args(&GitChange {
                path: "new.tex".into(),
                original_path: None,
                kind: GitChangeKind::Untracked,
                staged: false,
            }),
            vec!["diff", "--no-index", unified.as_str(), "--", "/dev/null", "new.tex"]
        );
    }

    #[test]
    fn common_affixes_finds_change_region() {
        assert_eq!(common_affixes("let x = old", "let x = new"), (8, 0));
        assert_eq!(common_affixes("abc123xyz", "abc456xyz"), (3, 3));
        // Full overlap must not double-count shared characters.
        let (p, s) = common_affixes("aaa", "aaaa");
        assert_eq!(p + s, 3);
    }

    #[test]
    fn display_items_fold_long_context() {
        // 20 unchanged context lines: VSCode folds the middle run,
        // keeping edge_context lines at each end of the fold.
        let mut rows: Vec<GitDiffRow> = (1..=20)
            .map(|n| GitDiffRow::Pair {
                left: Some(ctx(n)),
                right: Some(ctx(n)),
            })
            .collect();
        rows.push(GitDiffRow::Pair {
            left: Some(GitDiffLine { number: 21, text: "old".into(), kind: GitDiffLineKind::Removed }),
            right: Some(GitDiffLine { number: 21, text: "new".into(), kind: GitDiffLineKind::Added }),
        });
        let items = display_items(&rows, 3);
        // 3 context + fold(14) + 3 context + 1 change pair = 8 items.
        assert_eq!(items.len(), 8);
        let GitDiffItem::Fold { pairs, .. } = &items[3] else {
            panic!("middle item should be a fold")
        };
        assert_eq!(pairs.len(), 14);
        let GitDiffItem::Pair { left, right } = &items[7] else {
            panic!("last item should be the change")
        };
        assert_eq!(left.as_ref().map(|l| l.kind), Some(GitDiffLineKind::Removed));
        assert_eq!(right.as_ref().map(|l| l.kind), Some(GitDiffLineKind::Added));
    }

    #[test]
    fn display_items_short_context_stays_flat() {
        let rows: Vec<GitDiffRow> = (1..=6)
            .map(|n| GitDiffRow::Pair {
                left: Some(ctx(n)),
                right: Some(ctx(n)),
            })
            .collect();
        let items = display_items(&rows, 3);
        assert_eq!(items.len(), 6);
        assert!(items.iter().all(|i| matches!(i, GitDiffItem::Pair { .. })));
    }

    #[test]
    fn display_items_hunk_becomes_gap() {
        // A `@@` boundary (file longer than DIFF_CONTEXT_LINES) turns into
        // a non-expandable gap sized by the hidden old-side lines.
        let rows = vec![
            GitDiffRow::Pair {
                left: Some(GitDiffLine { number: 10, text: "c".into(), kind: GitDiffLineKind::Context }),
                right: Some(GitDiffLine { number: 10, text: "c".into(), kind: GitDiffLineKind::Context }),
            },
            GitDiffRow::Hunk("@@ -110,3 +110,3 @@".into()),
            GitDiffRow::Pair {
                left: Some(GitDiffLine { number: 110, text: "x".into(), kind: GitDiffLineKind::Removed }),
                right: Some(GitDiffLine { number: 110, text: "y".into(), kind: GitDiffLineKind::Added }),
            },
        ];
        let items = display_items(&rows, 3);
        assert_eq!(items.len(), 3);
        let GitDiffItem::Gap { old_lines } = items[1] else {
            panic!("hunk header should become a gap")
        };
        assert_eq!(old_lines, 99); // old lines 11...109 hidden
    }
}
