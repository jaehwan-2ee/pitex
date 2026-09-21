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
