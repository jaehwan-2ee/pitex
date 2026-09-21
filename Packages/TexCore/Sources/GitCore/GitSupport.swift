import Foundation

/// One entry of `git status --porcelain=v1`: what happened to a path.
public enum GitChangeKind: String, Codable, Sendable {
    case modified = "M"
    case added = "A"
    case deleted = "D"
    case renamed = "R"
    case copied = "C"
    case typeChanged = "T"
    case untracked = "?"
    case conflicted = "U"

    /// Status letter shown next to the file name (VSCode-style badge).
    public var badge: String {
        switch self {
        case .untracked: return "U"
        case .conflicted: return "!"
        default: return rawValue
        }
    }
}

/// A changed path, staged or unstaged (`originalPath` only for renames/copies).
public struct GitChange: Sendable, Equatable {
    public let path: String
    public let originalPath: String?
    public let kind: GitChangeKind
    public let staged: Bool

    public init(path: String, originalPath: String?, kind: GitChangeKind, staged: Bool) {
        self.path = path
        self.originalPath = originalPath
        self.kind = kind
        self.staged = staged
    }
}

/// One row of the commit graph (`git log` summary + ref decorations).
public struct GitCommit: Sendable, Equatable {
    public let hash: String
    public let subject: String
    public let author: String
    public let relativeDate: String
    /// Branch/tag decorations (`main`, `origin/main`, `tag: v1.2.0`, …).
    public let refs: [String]
    public let isHead: Bool

    public init(hash: String, subject: String, author: String, relativeDate: String, refs: [String], isHead: Bool) {
        self.hash = hash
        self.subject = subject
        self.author = author
        self.relativeDate = relativeDate
        self.refs = refs
        self.isHead = isHead
    }
}

/// A changed path inside one commit (`git show --name-status`).
public struct GitCommitFile: Sendable, Equatable {
    public let path: String
    public let kind: GitChangeKind

    public init(path: String, kind: GitChangeKind) {
        self.path = path
        self.kind = kind
    }
}

/// One text line on one side of a side-by-side diff.
public struct GitDiffLine: Sendable, Equatable {
    public enum Kind: Sendable, Equatable {
        case context
        case removed
        case added
    }

    public let number: Int
    public let text: String
    public let kind: Kind

    public init(number: Int, text: String, kind: Kind) {
        self.number = number
        self.text = text
        self.kind = kind
    }
}

/// A row of a commit diff. `pair` rows render side by side — removed
/// lines on the left, added on the right, context on both; a nil side
/// means that column stays empty for the row.
public enum GitDiffRow: Sendable, Equatable {
    /// `diff --git`, `index`, `---`/`+++`, mode/rename notes, "Binary
    /// files differ" — shown dimmed across the full width.
    case meta(String)
    /// `@@ -a,b +c,d @@` section divider.
    case hunk(String)
    case pair(left: GitDiffLine?, right: GitDiffLine?)
}

/// Repository snapshot for the Git Integration panel.
public struct GitStatus: Sendable, Equatable {
    public let root: String
    public let repoName: String
    /// Current branch, or the short hash when HEAD is detached.
    public let branch: String
    public let upstream: String?
    public let ahead: Int
    public let behind: Int
    public let staged: [GitChange]
    public let unstaged: [GitChange]

    public init(
        root: String,
        repoName: String,
        branch: String,
        upstream: String?,
        ahead: Int,
        behind: Int,
        staged: [GitChange],
        unstaged: [GitChange]
    ) {
        self.root = root
        self.repoName = repoName
        self.branch = branch
        self.upstream = upstream
        self.ahead = ahead
        self.behind = behind
        self.staged = staged
        self.unstaged = unstaged
    }
}

/// Pure parsing/argument builders — the Rust `git-core` crate ports this
/// file verbatim; platform shells only run the produced argv.
public enum GitSupport {
    public static let topLevelArgs = ["rev-parse", "--show-toplevel"]
    public static let statusArgs = ["status", "--porcelain=v1", "-z", "--branch", "-uall"]
    public static let branchArgs = ["branch", "--format=%(refname:short)"]
    /// `hash | author | relative | decorations | subject`, records separated
    /// by \u{1e} — parsing survives spaces/newlines inside subjects.
    public static func logArgs(limit: Int = 80) -> [String] {
        // `--exclude=refs/notes/*` keeps `git-ai` authorship notes out of
        // the graph — they are not user history (`--exclude` must precede
        // the `--all` it filters).
        ["log", "--exclude=refs/notes/*", "--all", "-n", String(limit), "--date=relative",
         "--pretty=tformat:%h%x1f%an%x1f%ar%x1f%D%x1f%s%x1e"]
    }

    public static func stageArgs(_ path: String) -> [String] { ["add", "--", path] }
    /// `restore --staged` fails on repositories with no commits — callers
    /// retry with `reset HEAD --` (see `unstageFallbackArgs`).
    public static func unstageArgs(_ path: String) -> [String] { ["restore", "--staged", "--", path] }
    public static func unstageFallbackArgs(_ path: String) -> [String] { ["reset", "HEAD", "--", path] }
    /// No-HEAD repos can only unstage by dropping the index entry.
    public static func unstageNoHeadArgs(_ path: String) -> [String] { ["rm", "--cached", "--", path] }
    public static func stageAllArgs() -> [String] { ["add", "--all"] }
    public static func unstageAllArgs() -> [String] { ["reset"] }
    public static func unstageAllNoHeadArgs() -> [String] { ["rm", "-r", "--cached", "."] }
    /// `commit -a` stages tracked files only; untracked rows need an
    /// explicit `add` first (same as VSCode's "Commit All").
    public static func commitArgs(_ message: String, all: Bool) -> [String] {
        all ? ["commit", "-am", message] : ["commit", "-m", message]
    }
    public static let fetchArgs = ["fetch", "--all", "--prune"]
    public static let pullArgs = ["pull"]
    public static let pushArgs = ["push"]
    public static let initArgs = ["init"]
    public static func switchArgs(_ branch: String) -> [String] { ["switch", branch] }
    public static func createBranchArgs(_ name: String) -> [String] { ["switch", "-c", name] }
    /// Unstaged tracked discard; staged and untracked paths need
    /// `discardStagedArgs`/`cleanArgs` instead.
    public static func discardArgs(_ path: String) -> [String] { ["checkout", "--", path] }
    /// Staged discard — restores index+worktree from HEAD (fails for paths
    /// added since the last commit; those use `removeFileArgs`).
    public static func discardStagedArgs(_ path: String) -> [String] { ["checkout", "HEAD", "--", path] }
    /// Staged-new discard — drops index entry and the worktree file.
    public static func removeFileArgs(_ path: String) -> [String] { ["rm", "-f", "--", path] }
    public static func cleanArgs(_ path: String) -> [String] { ["clean", "-f", "--", path] }
    /// `--staged` describes the pending commit; plain `diff` covers the
    /// commit-everything fallback (untracked files produce no diff — the
    /// caller names them in the prompt's file list).
    public static func diffArgs(staged: Bool) -> [String] { staged ? ["diff", "--staged"] : ["diff"] }

    /// `git show` file list for one commit: `STATUS<TAB>path` rows
    /// (`R100<TAB>old<TAB>new` carries both paths). `--diff-merges=
    /// first-parent` diffs merge commits against the first parent instead
    /// of printing the (usually empty) combined diff.
    public static func commitFilesArgs(_ hash: String) -> [String] {
        ["show", "--format=", "--name-status", "--diff-merges=first-parent", hash]
    }

    /// One file's patch inside a commit (`diff --git` header + hunks).
    public static func fileDiffArgs(_ hash: String, path: String) -> [String] {
        ["show", "--format=", "--diff-merges=first-parent", hash, "--", path]
    }

    /// `commitFilesArgs` output → badge + path rows. Rename/copy rows end
    /// with the new path — the diff targets that name.
    public static func parseCommitFiles(_ raw: String) -> [GitCommitFile] {
        raw.split(separator: "\n").compactMap { line in
            let fields = line.split(separator: "\t")
            guard fields.count >= 2, let code = fields[0].first else { return nil }
            var path = String(fields[fields.count - 1])
            if path.count > 1, path.hasPrefix("\""), path.hasSuffix("\"") {
                path = String(path.dropFirst().dropLast())
            }
            return GitCommitFile(path: path, kind: kind(from: code))
        }
    }

    /// Unified diff (`fileDiffArgs` output) → aligned side-by-side rows.
    /// `---`/`+++` headers land before the first `@@`, so only lines inside
    /// a hunk count as removed/added/context. Within each change block the
    /// removed lines pair with the added lines in order; uneven tails
    /// leave the opposite side empty.
    public static func parseFileDiff(_ raw: String) -> [GitDiffRow] {
        var rows: [GitDiffRow] = []
        var removed: [GitDiffLine] = []
        var added: [GitDiffLine] = []
        var oldLine = 0
        var newLine = 0
        var inHunk = false

        func flush() {
            for i in 0 ..< max(removed.count, added.count) {
                rows.append(.pair(
                    left: i < removed.count ? removed[i] : nil,
                    right: i < added.count ? added[i] : nil))
            }
            removed.removeAll()
            added.removeAll()
        }

        for rawLine in raw.split(separator: "\n", omittingEmptySubsequences: false) {
            let line = String(rawLine)
            if line.hasPrefix("@@") {
                flush()
                (oldLine, newLine) = hunkStarts(line)
                rows.append(.hunk(line))
                inHunk = true
            } else if inHunk, line.hasPrefix("-") {
                removed.append(GitDiffLine(number: oldLine, text: String(line.dropFirst()), kind: .removed))
                oldLine += 1
            } else if inHunk, line.hasPrefix("+") {
                added.append(GitDiffLine(number: newLine, text: String(line.dropFirst()), kind: .added))
                newLine += 1
            } else if inHunk, line.hasPrefix(" ") {
                flush()
                let text = String(line.dropFirst())
                rows.append(.pair(
                    left: GitDiffLine(number: oldLine, text: text, kind: .context),
                    right: GitDiffLine(number: newLine, text: text, kind: .context)))
                oldLine += 1
                newLine += 1
            } else if line.hasPrefix("\\") {
                flush()
                rows.append(.meta(line))
            } else if !line.isEmpty {
                flush()
                // A new file block ends the current hunk so its ---/+++
                // headers are never read as removed/added lines.
                if line.hasPrefix("diff --git") || line.hasPrefix("Binary files") {
                    inHunk = false
                }
                rows.append(.meta(line))
            }
        }
        flush()
        return rows
    }

    /// `@@ -old[,n] +new[,n] @@` → the starting line number of each side.
    private static func hunkStarts(_ header: String) -> (old: Int, new: Int) {
        func firstDigits(_ s: Substring) -> Int {
            var digits = ""
            for c in s {
                if c.isNumber {
                    digits.append(c)
                } else if !digits.isEmpty {
                    break
                }
            }
            return Int(digits) ?? 0
        }
        let inner = header.dropFirst(2)
        let old = firstDigits(inner)
        let new = inner.firstIndex(of: "+").map { firstDigits(inner[$0...]) } ?? 0
        return (old, new)
    }

    /// `git status --porcelain=v1 -z --branch` output → status fields.
    /// In `-z` format each record is `XY path\0`; renames/copies append the
    /// source path as the following bare field.
    public static func parseStatus(_ raw: String, root: String) -> GitStatus {
        var branch = ""
        var upstream: String?
        var ahead = 0
        var behind = 0
        var staged: [GitChange] = []
        var unstaged: [GitChange] = []

        let fields = raw.split(separator: "\0", omittingEmptySubsequences: true).map(String.init)
        var index = 0
        while index < fields.count {
            let field = fields[index]
            index += 1
            if field.hasPrefix("## ") {
                (branch, upstream, ahead, behind) = parseBranchHeader(String(field.dropFirst(3)))
                continue
            }
            guard field.count >= 4 else { continue }
            let chars = Array(field)
            let x = chars[0]
            let y = chars[1]
            guard chars[2] == " " else { continue }
            let path = String(chars[3...])
            var originalPath: String?
            if "RC".contains(x), index < fields.count {
                originalPath = fields[index]
                index += 1
            }
            if x == "?" && y == "?" {
                unstaged.append(GitChange(path: path, originalPath: nil, kind: .untracked, staged: false))
                continue
            }
            if x == "!" { continue }
            let conflictCodes: Set<String> = ["DD", "AU", "UD", "UA", "DU", "AA", "UU"]
            if conflictCodes.contains(String([x, y])) {
                unstaged.append(GitChange(path: path, originalPath: nil, kind: .conflicted, staged: false))
                continue
            }
            if x != " " {
                staged.append(GitChange(path: path, originalPath: originalPath, kind: kind(from: x), staged: true))
            }
            if y != " " {
                unstaged.append(GitChange(path: path, originalPath: nil, kind: kind(from: y), staged: false))
            }
        }
        let repoName = root.split(separator: "/").last.map(String.init) ?? root
        return GitStatus(
            root: root,
            repoName: repoName,
            branch: branch,
            upstream: upstream,
            ahead: ahead,
            behind: behind,
            staged: staged,
            unstaged: unstaged
        )
    }

    /// `## main...origin/main [ahead 1, behind 2]` / `## No commits yet on main`.
    private static func parseBranchHeader(_ header: String) -> (String, String?, Int, Int) {
        var branch = header
        var upstream: String?
        var ahead = 0
        var behind = 0
        if let bracket = header.range(of: " [", options: .backwards) {
            branch = String(header[..<bracket.lowerBound])
            let info = header[bracket.upperBound...].dropLast()
            for part in info.split(separator: ",") {
                let piece = part.trimmingCharacters(in: .whitespaces)
                if let n = piece.split(separator: " ").last.flatMap({ Int($0) }) {
                    if piece.hasPrefix("ahead") { ahead = n }
                    if piece.hasPrefix("behind") { behind = n }
                }
            }
        }
        if let dots = branch.range(of: "...") {
            upstream = String(branch[dots.upperBound...])
            branch = String(branch[..<dots.lowerBound])
        }
        if branch.hasPrefix("No commits yet on ") {
            branch = String(branch.dropFirst("No commits yet on ".count))
        }
        if branch.hasPrefix("HEAD (") {
            branch = "HEAD"
        }
        return (branch, upstream, ahead, behind)
    }

    private static func kind(from code: Character) -> GitChangeKind {
        switch code {
        case "A": return .added
        case "D": return .deleted
        case "R": return .renamed
        case "C": return .copied
        case "T": return .typeChanged
        case "U": return .conflicted
        default: return .modified
        }
    }

    /// `logArgs` output → commit rows; `%D` decorations become ref chips.
    public static func parseLog(_ raw: String) -> [GitCommit] {
        raw.split(separator: "\u{1e}", omittingEmptySubsequences: true).compactMap { record in
            let fields = record.split(separator: "\u{1f}", omittingEmptySubsequences: false).map(String.init)
            guard fields.count >= 5 else { return nil }
            let hash = fields[0].trimmingCharacters(in: .whitespacesAndNewlines)
            guard !hash.isEmpty else { return nil }
            var refs: [String] = []
            var isHead = false
            for decoration in fields[3].split(separator: ",") {
                var d = decoration.trimmingCharacters(in: .whitespaces)
                if d.hasPrefix("HEAD") {
                    isHead = true
                    if let arrow = d.range(of: " -> ") {
                        d = String(d[arrow.upperBound...])
                    } else {
                        continue
                    }
                }
                if !d.isEmpty { refs.append(d) }
            }
            return GitCommit(
                hash: hash,
                subject: fields[4],
                author: fields[1],
                relativeDate: fields[2],
                refs: refs,
                isHead: isHead
            )
        }
    }

    public static func parseBranches(_ raw: String) -> [String] {
        raw.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
    }
}
