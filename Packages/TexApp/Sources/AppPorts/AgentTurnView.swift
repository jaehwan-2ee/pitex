import Foundation

/// How the transcript is drawn (Paseo `collapseTurns` and subagent grouping): a subagent's
/// calls nest under its Task/Agent card, and a finished turn folds its work and earlier
/// replies into one summary row under the prompt, leaving the final reply in view.
/// The Linux panel does the same in `turn_view.rs`.
public enum AgentTurnView {
    /// The facts of one transcript row the grouping needs.
    public struct Row: Equatable, Sendable {
        public enum Role: Sendable { case user, assistant, thinking, tool, notice, activity }
        public var role: Role
        public var toolKind: AgentToolDetail.Kind?
        public var path: String?
        public var callID: String?
        public var parentCallID: String?
        /// On a prompt: how long its turn ran.
        public var workedSeconds: Int?

        public init(role: Role, toolKind: AgentToolDetail.Kind? = nil, path: String? = nil, callID: String? = nil, parentCallID: String? = nil, workedSeconds: Int? = nil) {
            self.role = role
            self.toolKind = toolKind
            self.path = path
            self.callID = callID
            self.parentCallID = parentCallID
            self.workedSeconds = workedSeconds
        }
    }

    public indirect enum Item: Equatable, Sendable {
        /// One row; a subagent card carries the rows of the calls it made.
        case row(index: Int, children: [Int])
        /// A finished turn's work, folded under its prompt (row `prompt`).
        case summary(prompt: Int, summary: Summary, rows: [Item])
    }

    /// "Worked for 1m 12s · 3 edits · 2 commands · 4 reads" (Paseo's overview counts).
    public struct Summary: Equatable, Sendable {
        public var seconds: Int?
        /// Files edited or written, each file once.
        public var edits = 0
        public var commands = 0
        /// Files read, each file once.
        public var reads = 0
        /// Searches and fetches.
        public var searches = 0
        /// Every other call (subagents, MCP and pi tools).
        public var tools = 0
        public var thoughts = 0

        public init(seconds: Int? = nil) { self.seconds = seconds }

        /// The non-zero counts in display order, as (`assistant.turn.<kind>` key stem, count).
        public var parts: [(kind: String, count: Int)] {
            [("edits", edits), ("commands", commands), ("reads", reads), ("searches", searches), ("tools", tools), ("thoughts", thoughts)]
                .filter { $0.1 > 0 }
                .map { (kind: $0.0, count: $0.1) }
        }

        /// The summary line; `localized(key)` returns the template for an `assistant.turn.*` key.
        public func text(_ localized: (String) -> String) -> String {
            var parts = [seconds.map { localized("assistant.turn.worked_for").replacingOccurrences(of: "%@", with: AgentTurnView.duration($0, localized)) }
                ?? localized("assistant.turn.worked")]
            for part in self.parts {
                parts.append(part.count == 1
                    ? localized("assistant.turn.\(part.kind).one")
                    : localized("assistant.turn.\(part.kind).other").replacingOccurrences(of: "%@", with: String(part.count)))
            }
            return parts.joined(separator: " · ")
        }
    }

    /// "1m 12s", "45s", "<1s".
    public static func duration(_ seconds: Int, _ localized: (String) -> String) -> String {
        switch seconds {
        case ..<1: return localized("assistant.turn.seconds").replacingOccurrences(of: "%@", with: "<1")
        case ..<60: return localized("assistant.turn.seconds").replacingOccurrences(of: "%@", with: String(seconds))
        default:
            return localized("assistant.turn.minutes")
                .replacingOccurrences(of: "%1$@", with: String(seconds / 60))
                .replacingOccurrences(of: "%2$@", with: String(seconds % 60))
        }
    }

    /// The items to draw; the running turn (the last, while `running`) stays open.
    public static func items(_ rows: [Row], running: Bool) -> [Item] {
        let children = subagentChildren(rows)
        let nested = Set(children.values.flatMap { $0 })
        func row(_ index: Int) -> Item { .row(index: index, children: children[index] ?? []) }
        let top = rows.indices.filter { !nested.contains($0) }
        var items: [Item] = []
        var start = 0
        while start < top.count {
            // A turn: a prompt and everything up to the next prompt (rows before the first prompt stand alone).
            var end = start + 1
            while end < top.count, rows[top[end]].role != .user { end += 1 }
            let turn = Array(top[start..<end])
            let prompt = turn[0]
            let reply = turn.lastIndex { rows[$0].role == .assistant }
            let foldable = rows[prompt].role == .user && !(running && end == top.count)
            var folded: [Int] = []
            if let reply, foldable {
                folded = turn[1..<reply].filter { rows[$0].role != .notice }
            }
            // Paseo folds only a turn that worked or answered more than once.
            let worked = folded.contains { [.tool, .thinking, .assistant].contains(rows[$0].role) }
            if !worked {
                items += turn.map(row)
            } else {
                items.append(row(prompt))
                items.append(.summary(prompt: prompt, summary: summarize(rows, folded, seconds: rows[prompt].workedSeconds), rows: folded.map(row)))
                // Notices (errors, compaction) stay in view, then the final reply and what follows it.
                let replyAt = reply ?? turn.count
                items += turn.indices.dropFirst().filter { $0 >= replyAt || rows[turn[$0]].role == .notice }.map { row(turn[$0]) }
            }
            start = end
        }
        return items
    }

    /// Subagent card row → the rows of the calls made inside it (nested subagents flatten into the outermost card).
    static func subagentChildren(_ rows: [Row]) -> [Int: [Int]] {
        var cardOf: [String: Int] = [:]
        var children: [Int: [Int]] = [:]
        for (index, row) in rows.enumerated() {
            let card = row.parentCallID.flatMap { cardOf[$0] }
            if let card { children[card, default: []].append(index) }
            guard let id = row.callID, !id.isEmpty else { continue }
            if row.toolKind == .subAgent {
                cardOf[id] = card ?? index
            } else if let card, cardOf[id] == nil {
                cardOf[id] = card
            }
        }
        return children
    }

    static func summarize(_ rows: [Row], _ folded: [Int], seconds: Int?) -> Summary {
        var summary = Summary(seconds: seconds)
        var edited = Set<String>()
        var read = Set<String>()
        for index in folded {
            let row = rows[index]
            if row.role == .thinking { summary.thoughts += 1 }
            guard row.role == .tool else { continue }
            let path = row.path ?? "#\(index)"
            switch row.toolKind ?? .other {
            case .edit, .write: edited.insert(path)
            case .read: read.insert(path)
            case .shell: summary.commands += 1
            case .search, .fetch: summary.searches += 1
            default: summary.tools += 1
            }
        }
        summary.edits = edited.count
        summary.reads = read.count
        return summary
    }

    /// Whole turns a Fork or Rewind on row `index` keeps: those before a prompt (whose text
    /// goes back into the composer), or those through an answer's turn. `dropped` counts
    /// prompts trimmed off the top of the transcript.
    public static func turnCut(roles: [Row.Role], index: Int, dropped: Int = 0) -> Int? {
        guard roles.indices.contains(index) else { return nil }
        let before = dropped + roles[..<index].filter { $0 == .user }.count
        switch roles[index] {
        case .user: return before
        case .assistant where before > 0: return before
        default: return nil
        }
    }

    /// The first `keep` turns as a Markdown quote for a new chat (backends without native fork).
    public static func quote(_ rows: [(role: Row.Role, text: String)], keep: Int, header: String, you: String, assistant: String) -> String {
        var prompts = 0
        var blocks: [String] = []
        for row in rows {
            if row.role == .user {
                if prompts == keep { break }
                prompts += 1
            }
            let who: String
            switch row.role {
            case .user: who = you
            case .assistant: who = assistant
            default: continue
            }
            let text = row.text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { continue }
            let body = text.split(separator: "\n", omittingEmptySubsequences: false).map { "> \($0)" }.joined(separator: "\n")
            blocks.append("> **\(who):**\n\(body)")
        }
        return blocks.isEmpty ? "" : "\(header)\n\n\(blocks.joined(separator: "\n>\n"))"
    }
}
