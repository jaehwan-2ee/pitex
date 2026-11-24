import Foundation

/// The optional `detail` object a bridge backend puts on `tool_execution_*`
/// events (Claude Code, Antigravity, Grok, Codex). pi's own provider sends
/// none, and its rows keep the plain name + detail rendering.
///
/// `oldText` and `newText` are not kept: the diff carries the change.
public struct AgentToolDetail: Equatable, Sendable {
    public enum Kind: String, Sendable {
        case shell, read, edit, write, search, fetch, think, other
        case subAgent = "sub_agent"
    }

    /// The localized phrases a card title or badge is built from. The app
    /// maps each to its `Localizable.strings` key (`assistant.tool.*`).
    public enum Phrase: Sendable {
        case ran, edited, wrote, read, lines, searched, matches, fetched, subagent, thought, exit
    }

    /// Cap for the diff text a card keeps; the bridge caps at the same size.
    public static let diffCap = 12_000

    public var kind: Kind
    public var path: String?
    public var command: String?
    public var cwd: String?
    public var exitCode: Int?
    public var diff: String?
    public var query: String?
    public var url: String?
    public var numMatches: Int?
    public var label: String?
    public var startLine: Int?
    public var endLine: Int?
    /// The Task/Agent call this one ran inside (the event's `parentToolCallId`).
    public var parentCallID: String?
    /// sub_agent: the agent type and how many calls it started.
    public var subAgentType: String?
    public var toolUses: Int?

    public init(kind: Kind) {
        self.kind = kind
    }

    /// Decodes the event's `detail` object; nil without a known `kind`.
    public init?(_ object: [String: Any]?) {
        guard let object, let raw = object["kind"] as? String else { return nil }
        func text(_ key: String) -> String? {
            guard let value = object[key] as? String, !value.isEmpty else { return nil }
            return value
        }
        kind = Kind(rawValue: raw) ?? .other
        path = text("path")
        command = text("command")
        cwd = text("cwd")
        exitCode = object["exitCode"] as? Int
        diff = text("diff").map(Self.boundedDiff)
        query = text("query")
        url = text("url")
        numMatches = object["numMatches"] as? Int
        label = text("label")
        startLine = object["startLine"] as? Int
        endLine = object["endLine"] as? Int
        subAgentType = text("subAgentType")
        toolUses = object["toolUses"] as? Int
    }

    /// Paseo's merge rule: a later event fills in fields; an untyped
    /// (`other`) detail never replaces a typed one.
    public mutating func merge(_ incoming: AgentToolDetail) {
        if incoming.kind == .other, kind != .other { return }
        kind = incoming.kind
        path = incoming.path ?? path
        command = incoming.command ?? command
        cwd = incoming.cwd ?? cwd
        exitCode = incoming.exitCode ?? exitCode
        diff = incoming.diff ?? diff
        query = incoming.query ?? query
        url = incoming.url ?? url
        numMatches = incoming.numMatches ?? numMatches
        label = incoming.label ?? label
        startLine = incoming.startLine ?? startLine
        endLine = incoming.endLine ?? endLine
        parentCallID = incoming.parentCallID ?? parentCallID
        subAgentType = incoming.subAgentType ?? subAgentType
        toolUses = incoming.toolUses ?? toolUses
    }

    /// `status` on the event: running, completed, failed or canceled.
    public enum Status: String, Sendable {
        case running, completed, failed, canceled
    }

    public static func status(_ raw: String?) -> Status? {
        raw.flatMap(Status.init(rawValue:))
    }

    /// Added and removed line counts inside the diff's hunks.
    public var diffCounts: (added: Int, removed: Int) {
        guard let diff else { return (0, 0) }
        var added = 0
        var removed = 0
        var inHunk = false
        for line in diff.split(separator: "\n", omittingEmptySubsequences: false) {
            if line.hasPrefix("@@") {
                inHunk = true
            } else if inHunk, line.hasPrefix("+") {
                added += 1
            } else if inHunk, line.hasPrefix("-") {
                removed += 1
            }
        }
        return (added, removed)
    }

    /// The file path relative to `root` when inside it, else as given.
    public func displayPath(root: URL?) -> String {
        guard let path else { return "" }
        guard let root else { return path }
        let base = root.standardizedFileURL.path
        let prefix = base.hasSuffix("/") ? base : base + "/"
        let file = URL(fileURLWithPath: path).standardizedFileURL.path
        return file.hasPrefix(prefix) ? String(file.dropFirst(prefix.count)) : path
    }

    /// The file an "Open file" button may open: read/edit/write cards whose
    /// path lies inside the project.
    public func openableURL(root: URL?) -> URL? {
        guard [.read, .edit, .write].contains(kind), let path, let root else { return nil }
        let url = path.hasPrefix("/") ? URL(fileURLWithPath: path) : root.appendingPathComponent(path)
        let base = root.resolvingSymlinksInPath().standardizedFileURL.path
        let file = url.resolvingSymlinksInPath().standardizedFileURL.path
        return file.hasPrefix(base.hasSuffix("/") ? base : base + "/") ? url.standardizedFileURL : nil
    }

    /// One-line card title, e.g. `Ran latexmk -pdf main.tex`,
    /// `Read main.tex (lines 10-40)`, `Searched "\cite" · 12 matches`.
    /// `phrase` returns the localized `%@` template for each phrase.
    public func title(fallback: String, root: URL?, phrase: (Phrase) -> String) -> String {
        func fill(_ which: Phrase, _ argument: String) -> String {
            phrase(which).replacingOccurrences(of: "%@", with: argument)
        }
        let shownPath = displayPath(root: root)
        switch kind {
        case .shell:
            let line = (command ?? fallback).split(separator: "\n", omittingEmptySubsequences: false).first ?? ""
            return fill(.ran, String(line))
        case .edit where !shownPath.isEmpty:
            return fill(.edited, shownPath)
        case .write where !shownPath.isEmpty:
            return fill(.wrote, shownPath)
        case .read where !shownPath.isEmpty:
            guard let startLine else { return fill(.read, shownPath) }
            let range = endLine.map { "\(startLine)-\($0)" } ?? "\(startLine)-"
            return fill(.read, "\(shownPath) (\(fill(.lines, range)))")
        case .search:
            var title = fill(.searched, "\"\(query ?? label ?? fallback)\"")
            if let numMatches { title += " · " + fill(.matches, String(numMatches)) }
            return title
        case .fetch:
            let target = url ?? fallback
            return fill(.fetched, URLComponents(string: target)?.host ?? target)
        case .subAgent:
            return fill(.subagent, label ?? fallback)
        case .think:
            return label ?? phrase(.thought)
        default:
            return label ?? fallback
        }
    }

    /// The header badge: `exit 1` for a failing command, `+3 -1` for a diff.
    public func badge(phrase: (Phrase) -> String) -> String? {
        if kind == .subAgent { return subAgentType }
        if kind == .shell {
            guard let exitCode, exitCode != 0 else { return nil }
            return phrase(.exit).replacingOccurrences(of: "%@", with: String(exitCode))
        }
        let counts = diffCounts
        guard counts.added + counts.removed > 0 else { return nil }
        return "+\(counts.added) -\(counts.removed)"
    }

    /// Cuts a long diff at a line boundary; a leading backslash is a meta
    /// line for `GitSupport.parseFileDiff`.
    static func boundedDiff(_ diff: String) -> String {
        guard diff.utf8.count > diffCap else { return diff }
        let prefix = String(decoding: diff.utf8.prefix(diffCap), as: UTF8.self)
        let cut = prefix.lastIndex(of: "\n").map { String(prefix[..<$0]) } ?? prefix
        return cut + "\n\\ diff truncated"
    }
}
