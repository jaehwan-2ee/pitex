import Foundation

/// The history list's row model, shared with the GTK shell (`agent.rs`):
/// which backend owns a saved chat, how old it is, and the title search.
public enum AgentSessionHistory {
    /// Bridge mirrors are named `<backend>-<id>.jsonl`; any other file is pi's own.
    public static func source(fileName: String) -> String {
        ["claude", "agy", "codex", "grok"].first { fileName.hasPrefix("\($0)-") } ?? "pi"
    }

    /// The row badge: product names, the same in every language.
    public static func sourceLabel(_ source: String) -> String {
        switch source {
        case "claude": "Claude Code"
        case "agy": "Antigravity"
        case "codex": "Codex"
        case "grok": "Grok"
        default: "pi"
        }
    }

    /// How a row's age reads: "Just now", "5 min ago", "3 h ago", "Yesterday", "4 days ago", then the date.
    public enum Age: Equatable, Sendable {
        case justNow
        case minutes(Int)
        case hours(Int)
        case yesterday
        case days(Int)
        case date
    }

    public static func age(of date: Date, now: Date) -> Age {
        let seconds = max(0, Int(now.timeIntervalSince(date)))
        switch seconds {
        case ..<60: return .justNow
        case ..<3_600: return .minutes(seconds / 60)
        case ..<86_400: return .hours(seconds / 3_600)
        case ..<172_800: return .yesterday
        case ..<604_800: return .days(seconds / 86_400)
        default: return .date
        }
    }

    /// Every word of `query` appears in the title, ignoring case.
    public static func matches(title: String, query: String) -> Bool {
        query.split(whereSeparator: \.isWhitespace).allSatisfy {
            title.range(of: $0, options: [.caseInsensitive, .diacriticInsensitive]) != nil
        }
    }
}
