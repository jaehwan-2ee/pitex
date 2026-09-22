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
    /// Full 40-char id — `hash` is abbreviated for display only.
    public let fullHash: String
    /// `%B` — subject + body, for "Copy Commit Message".
    public let message: String

    public init(hash: String, subject: String, author: String, relativeDate: String, refs: [String], isHead: Bool, fullHash: String? = nil, message: String? = nil) {
        self.hash = hash
        self.subject = subject
        self.author = author
        self.relativeDate = relativeDate
        self.refs = refs
        self.isHead = isHead
        self.fullHash = fullHash ?? hash
        self.message = message ?? subject
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

/// One aligned side-by-side row, used inside collapsed folds.
public struct GitDiffPair: Sendable, Equatable {
    public let left: GitDiffLine?
    public let right: GitDiffLine?
    public init(left: GitDiffLine?, right: GitDiffLine?) {
        self.left = left
        self.right = right
    }
}

/// Render item for the diff viewer — `pair` is a visible row, `fold` a
/// collapsed run of unchanged lines (expandable), `gap` a hunk boundary
/// covering lines git did not emit, `note` a dimmed informational line.
public enum GitDiffItem: Sendable, Equatable {
    case pair(left: GitDiffLine?, right: GitDiffLine?)
    case fold(id: Int, pairs: [GitDiffPair])
    case gap(oldLines: Int)
    case note(String)
}

/// One file's rendered diff inside a commit diff session.
public struct GitDiffFileSection: Sendable, Equatable {
    public let file: GitCommitFile
    public let binary: Bool
    public let rows: [GitDiffRow]

    public init(file: GitCommitFile, binary: Bool, rows: [GitDiffRow]) {
        self.file = file
        self.binary = binary
        self.rows = rows
    }

    /// VSCode-style `+added −removed` counts for the file header.
    public var additions: Int {
        rows.reduce(0) { $0 + ($1.right?.kind == .added ? 1 : 0) }
    }
    public var deletions: Int {
        rows.reduce(0) { $0 + ($1.left?.kind == .removed ? 1 : 0) }
    }
}

private extension GitDiffRow {
    var left: GitDiffLine? { if case let .pair(l, _) = self { l } else { nil } }
    var right: GitDiffLine? { if case let .pair(_, r) = self { r } else { nil } }
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
    public static let statusArgs = ["--no-optional-locks", "status", "--porcelain=v1", "-z", "--branch", "-uall"]
    public static let branchArgs = ["branch", "--format=%(refname:short)"]
    /// `hash | author | relative | decorations | subject`, records separated
    /// by \u{1e} — parsing survives spaces/newlines inside subjects.
    public static func logArgs(limit: Int = 80) -> [String] {
        // `--exclude=refs/notes/*` keeps `git-ai` authorship notes out of
        // the graph — they are not user history (`--exclude` must precede
        // the `--all` it filters).
        ["log", "--exclude=refs/notes/*", "--all", "-n", String(limit), "--date=relative",
         "--pretty=tformat:%h%x1f%an%x1f%ar%x1f%D%x1f%s%x1f%H%x1f%B%x1e"]
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
    /// `git switch -c` — `at` pins the start point (graph context menu's
    /// "Create Branch" branches off the selected commit, like VSCode).
    public static func createBranchArgs(_ name: String, at: String? = nil) -> [String] {
        ["switch", "-c", name] + (at.map { [$0] } ?? [])
    }
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

    /// Context window for commit diffs: large enough that `git show`
    /// emits whole files, letting the view fold unchanged regions itself
    /// (VSCode's `hideUnchangedRegions` behavior needs the full text).
    public static let diffContextLines = 100_000

    /// One file's patch inside a commit (`diff --git` header + hunks).
    public static func fileDiffArgs(_ hash: String, path: String) -> [String] {
        ["show", "--format=", "--diff-merges=first-parent",
         "--unified=\(diffContextLines)", hash, "--", path]
    }

    /// Every file's patch inside a commit — the "Open Changes" payload.
    public static func commitDiffArgs(_ hash: String) -> [String] {
        ["show", "--format=", "--diff-merges=first-parent",
         "--unified=\(diffContextLines)", hash]
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

    /// `parseFileDiff` rows → render items for the diff viewer: unchanged
    /// runs longer than `2 * edgeContext` fold into a `fold` bar (VSCode's
    /// collapsed unchanged regions), header/meta noise disappears, and a
    /// surviving `@@` boundary (a file longer than `diffContextLines`)
    /// becomes a non-expandable `gap` marker.
    public static func displayItems(_ rows: [GitDiffRow], edgeContext: Int = 3) -> [GitDiffItem] {
        var items: [GitDiffItem] = []
        var run: [GitDiffPair] = []
        var foldID = 0
        var lastOld: Int?

        func flush() {
            defer { run.removeAll() }
            guard run.count > 2 * edgeContext + 1 else {
                for p in run { items.append(.pair(left: p.left, right: p.right)) }
                return
            }
            for p in run.prefix(edgeContext) { items.append(.pair(left: p.left, right: p.right)) }
            items.append(.fold(id: foldID, pairs: Array(run.dropFirst(edgeContext).dropLast(edgeContext))))
            foldID += 1
            for p in run.suffix(edgeContext) { items.append(.pair(left: p.left, right: p.right)) }
        }

        for row in rows {
            switch row {
            case let .pair(left, right):
                if left?.kind == .context, right?.kind == .context {
                    run.append(GitDiffPair(left: left, right: right))
                } else {
                    flush()
                    items.append(.pair(left: left, right: right))
                }
                if let l = left { lastOld = l.number }
            case let .hunk(header):
                flush()
                let start = hunkStarts(header).old
                let hidden = max(0, start - (lastOld ?? 0) - 1)
                if hidden > 0 { items.append(.gap(oldLines: hidden)) }
            case let .meta(text):
                if text.hasPrefix("\\") {
                    flush()
                    items.append(.note(text))
                }
            }
        }
        flush()
        return items
    }

    /// `commitDiffArgs` output → one section per file. The path comes from
    /// the `+++ b/` header (deleted files fall back to `--- a/`); kind is
    /// resolved by the caller against the `--name-status` list.
    public static func parseCommitDiff(_ raw: String) -> [(path: String, binary: Bool, rows: [GitDiffRow])] {
        var blocks: [(path: String, binary: Bool, text: String)] = []
        var current = ""
        for rawLine in raw.split(separator: "\n", omittingEmptySubsequences: false) {
            let line = String(rawLine)
            if line.hasPrefix("diff --git "), !current.isEmpty {
                blocks.append(block(current))
                current = ""
            }
            current += line + "\n"
        }
        if !current.isEmpty { blocks.append(block(current)) }
        return blocks.map { (path: $0.path, binary: $0.binary, rows: parseFileDiff($0.text)) }

        func block(_ text: String) -> (path: String, binary: Bool, text: String) {
            var path = ""
            var binary = false
            for line in text.split(separator: "\n", omittingEmptySubsequences: false) {
                if line.hasPrefix("+++ b/") {
                    path = unquoteGitPath(String(line.dropFirst(6)))
                } else if line.hasPrefix("Binary files") {
                    binary = true
                } else if path.isEmpty, line.hasPrefix("diff --git ") {
                    // Deleted files keep only the `a/` side in the header.
                    let parts = line.split(separator: " ")
                    if let last = parts.last, last.hasPrefix("b/") {
                        path = unquoteGitPath(String(last.dropFirst(2)))
                    }
                }
            }
            if path.isEmpty {
                for line in text.split(separator: "\n", omittingEmptySubsequences: false)
                where line.hasPrefix("--- a/") {
                    path = unquoteGitPath(String(line.dropFirst(6)))
                    break
                }
            }
            return (path, binary, text)
        }
    }

    /// `git show`'s C-style quoting of non-ASCII paths: `"a/\303\244"` →
    /// UTF-8 octal escapes decoded back to the real name.
    public static func unquoteGitPath(_ path: String) -> String {
        var p = path
        if p.count > 1, p.hasPrefix("\""), p.hasSuffix("\"") {
            p = String(p.dropFirst().dropLast())
        }
        guard p.contains("\\") else { return p }
        var bytes: [UInt8] = []
        var i = p.startIndex
        while i < p.endIndex {
            let c = p[i]
            if c == "\\" {
                let next = p.index(after: i)
                guard next < p.endIndex else { break }
                let n = p[next]
                if n == "\\" || n == "\"" {
                    bytes.append(contentsOf: String(n).utf8)
                    i = p.index(after: next)
                } else if n.isNumber {
                    var octal = ""
                    var j = next
                    while j < p.endIndex, octal.count < 3, p[j].isNumber {
                        octal.append(p[j])
                        j = p.index(after: j)
                    }
                    if let byte = UInt8(octal, radix: 8) {
                        bytes.append(byte)
                        i = j
                    } else {
                        bytes.append(contentsOf: String(c).utf8)
                        i = next
                    }
                } else {
                    bytes.append(contentsOf: String(c).utf8)
                    i = next
                }
            } else {
                bytes.append(contentsOf: String(c).utf8)
                i = p.index(after: i)
            }
        }
        return String(decoding: bytes, as: UTF8.self)
    }

    /// Common leading/trailing character counts for a changed line pair —
    /// the intra-line highlight ranges in a side-by-side diff (VSCode's
    /// darker change regions inside a modified line).
    public static func commonAffixes(_ old: String, _ new: String) -> (prefix: Int, suffix: Int) {
        let a = Array(old)
        let b = Array(new)
        var prefix = 0
        while prefix < a.count, prefix < b.count, a[prefix] == b[prefix] { prefix += 1 }
        var suffix = 0
        while suffix < a.count - prefix, suffix < b.count - prefix,
              a[a.count - 1 - suffix] == b[b.count - 1 - suffix] { suffix += 1 }
        return (prefix, suffix)
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

        // Porcelain's prefix is three ASCII bytes. Read it without building
        // an Array<Character> (or counting graphemes) for every file path.
        var fields = raw.utf8.split(separator: 0).makeIterator()
        let conflictCodes: Set<String> = ["DD", "AU", "UD", "UA", "DU", "AA", "UU"]
        while let field = fields.next() {
            if field.starts(with: [35, 35, 32]) {
                (branch, upstream, ahead, behind) = parseBranchHeader(String(decoding: field.dropFirst(3), as: UTF8.self))
                continue
            }
            var prefix = field.makeIterator()
            guard let xByte = prefix.next(), let yByte = prefix.next(), prefix.next() == 32,
                  xByte < 128, yByte < 128, prefix.next() != nil else { continue }
            let x = Character(UnicodeScalar(xByte))
            let y = Character(UnicodeScalar(yByte))
            let path = String(decoding: field.dropFirst(3), as: UTF8.self)
            var originalPath: String?
            if x == "R" || x == "C", let original = fields.next() {
                originalPath = String(decoding: original, as: UTF8.self)
            }
            if x == "?" && y == "?" {
                unstaged.append(GitChange(path: path, originalPath: nil, kind: .untracked, staged: false))
                continue
            }
            if x == "!" { continue }
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
                isHead: isHead,
                fullHash: fields.count > 5 ? fields[5].trimmingCharacters(in: .whitespacesAndNewlines) : hash,
                // %B is the last field; rejoining tail fragments keeps a
                // (pathological) \x1f inside the body from truncating it.
                message: fields.count > 6
                    ? fields[6...].joined(separator: "\u{1f}")
                        .trimmingCharacters(in: .newlines)
                    : fields[4]
            )
        }
    }

    public static func parseBranches(_ raw: String) -> [String] {
        raw.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
    }
}
