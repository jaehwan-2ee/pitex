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

pub fn top_level_args() -> [&'static str; 2] {
    ["rev-parse", "--show-toplevel"]
}
pub fn status_args() -> [&'static str; 5] {
    ["status", "--porcelain=v1", "-z", "--branch", "-uall"]
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
        "--pretty=tformat:%h%x1f%an%x1f%ar%x1f%D%x1f%s%x1e".into(),
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
pub fn create_branch_args(name: &str) -> [String; 3] {
    ["switch".into(), "-c".into(), name.into()]
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

    let fields: Vec<&str> = raw.split('\0').filter(|f| !f.is_empty()).collect();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        index += 1;
        if let Some(header) = field.strip_prefix("## ") {
            (branch, upstream, ahead, behind) = parse_branch_header(header);
            continue;
        }
        let chars: Vec<char> = field.chars().collect();
        if chars.len() < 4 || chars[2] != ' ' {
            continue;
        }
        let x = chars[0];
        let y = chars[1];
        let path: String = chars[3..].iter().collect();
        let mut original_path = None;
        if "RC".contains(x) && index < fields.len() {
            original_path = Some(fields[index].to_string());
            index += 1;
        }
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
    fn status_handles_no_commits_and_detached() {
        let s = parse_status("## No commits yet on main\0?? a.tex\0", "/r");
        assert_eq!(s.branch, "main");
        let s = parse_status("## HEAD (no branch)\0", "/r");
        assert_eq!(s.branch, "HEAD");
    }

    #[test]
    fn log_parses_refs_and_head() {
        let raw = "abc1234\x1fJane\x1f2 days ago\x1fHEAD -> main, origin/main, tag: v1.2.0\x1fRelease v1.2.0\x1edef5678\x1fBob\x1f3 days ago\x1f\x1fAdd feature\x1e";
        let commits = parse_log(raw);
        assert_eq!(commits.len(), 2);
        assert!(commits[0].is_head);
        assert_eq!(commits[0].refs, ["main", "origin/main", "tag: v1.2.0"]);
        assert_eq!(commits[0].subject, "Release v1.2.0");
        assert!(!commits[1].is_head);
        assert!(commits[1].refs.is_empty());
    }

    #[test]
    fn branches_parse_lines() {
        assert_eq!(parse_branches("main\nfeature\n"), ["main", "feature"]);
    }
}
