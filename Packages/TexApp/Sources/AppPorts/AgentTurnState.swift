import Foundation

/// Latest provider usage (`usage_update`); each event carries the fields it knows.
public struct AgentUsage: Equatable, Sendable {
    public var inputTokens: Int?
    public var cachedInputTokens: Int?
    public var outputTokens: Int?
    public var thinkingTokens: Int?
    public var costUsd: Double?
    public var contextUsed: Int?
    public var contextMax: Int?

    public init() {}

    public mutating func merge(_ event: [String: Any]) {
        func count(_ key: String) -> Int? {
            guard let value = (event[key] as? NSNumber)?.doubleValue, value.isFinite, value >= 0 else { return nil }
            // Int(Double) traps past Int.max; a bogus huge count saturates like the Rust host.
            return Int(exactly: value.rounded(.down)) ?? Int.max
        }
        inputTokens = count("inputTokens") ?? inputTokens
        cachedInputTokens = count("cachedInputTokens") ?? cachedInputTokens
        outputTokens = count("outputTokens") ?? outputTokens
        thinkingTokens = count("thinkingTokens") ?? thinkingTokens
        contextUsed = count("contextUsed") ?? contextUsed
        contextMax = count("contextMax") ?? contextMax
        if let cost = (event["costUsd"] as? NSNumber)?.doubleValue, cost.isFinite, cost >= 0 {
            costUsd = cost
        }
    }
}

/// Context used of the model's window: Paseo's meter, amber from 70 %, red above 90 %.
public struct AgentContextMeter: Equatable, Sendable {
    public enum Band: Equatable, Sendable {
        case normal
        case warning
        case danger
    }

    /// Used share of the window, clamped to 0–100.
    public let percent: Double
    public let used: Int
    public let max: Int

    public init?(used: Int, max: Int) {
        guard max > 0 else { return nil }
        self.used = used
        self.max = max
        percent = Swift.min(Swift.max(Double(used) / Double(max) * 100, 0), 100)
    }

    public var band: Band {
        if percent > 90 { return .danger }
        if percent >= 70 { return .warning }
        return .normal
    }

    public var roundedPercent: Int { Int(percent.rounded()) }

    /// "38% · 76k/200k".
    public var label: String {
        "\(roundedPercent)% · \(Self.compactTokens(used))/\(Self.compactTokens(max))"
    }

    /// 950, 7.6k, 76k, 1.2M.
    public static func compactTokens(_ value: Int) -> String {
        switch value {
        case ..<1_000:
            return "\(value)"
        case ..<10_000:
            return String(format: "%.1fk", Double(value) / 1_000)
        case ..<1_000_000:
            return "\(Int((Double(value) / 1_000).rounded()))k"
        default:
            let millions = Double(value) / 1_000_000
            return abs(millions - millions.rounded()) < 0.05
                ? "\(Int(millions.rounded()))M"
                : String(format: "%.1fM", millions)
        }
    }
}

/// `agent_end.code` from the bridge, plus `crashed` for a process that exited mid-chat.
public enum AgentErrorCode: String, Equatable, Sendable {
    case signedOut = "signed_out"
    case rateLimited = "rate_limited"
    case network
    case contextFull = "context_full"
    case other
    case crashed

    public init(code: String?) {
        self = code.flatMap(AgentErrorCode.init(rawValue:)).flatMap { $0 == .crashed ? nil : $0 } ?? .other
    }

    /// The `Localizable.strings` key of the friendly line; `%@` is the CLI name.
    public func messageKey(canCompact: Bool) -> String {
        switch self {
        case .signedOut: "assistant.error.signed_out"
        case .rateLimited: "assistant.error.rate_limited"
        case .network: "assistant.error.network"
        case .contextFull: canCompact ? "assistant.error.context_full_compact" : "assistant.error.context_full"
        case .other: "assistant.error.other"
        case .crashed: "assistant.error.crashed"
        }
    }

    /// The CLI a provider id drives, for "Sign in to Claude Code in a terminal".
    public static func cliName(provider: String) -> String {
        switch provider {
        case "claude": "Claude Code"
        case "agy": "Antigravity"
        case "codex": "Codex"
        case "grok": "Grok"
        case "pi": "Pitex Agent"
        case "apple-fm": "Apple Foundation Models"
        default: provider
        }
    }
}

/// What a failed turn's row shows and whether it still offers Retry.
public struct AgentTurnError: Equatable, Sendable {
    public var code: AgentErrorCode
    public var provider: String
    public var canCompact: Bool
    public var retry: Bool

    public init(code: AgentErrorCode, provider: String, canCompact: Bool, retry: Bool) {
        self.code = code
        self.provider = provider
        self.canCompact = canCompact
        self.retry = retry
    }

    /// `agent_end` of a failed turn: `outcome` failed, or (pi, which sends no outcome) an
    /// assistant error. nil for a completed or stopped turn.
    public static func failure(agentEnd event: [String: Any], fallback: String?) -> (code: AgentErrorCode, text: String)? {
        let outcome = event["outcome"] as? String
        let text = (event["error"] as? String) ?? fallback
        switch outcome {
        case "completed", "canceled": return nil
        case "failed": return (AgentErrorCode(code: event["code"] as? String), text ?? "The agent run failed.")
        default: return text.map { (AgentErrorCode(code: event["code"] as? String), $0) }
        }
    }
}

/// The message Retry sends again: the user's text and its images, as typed.
public struct AgentRetryPayload: Equatable, Sendable {
    public var text: String
    public var images: [AgentImage]

    public init(text: String, images: [AgentImage]) {
        self.text = text
        self.images = images
    }
}

public enum AgentThinking {
    /// A thinking block is open while it streams, then closed, unless the user toggled it.
    public static func isExpanded(streaming: Bool, userChoice: Bool?) -> Bool {
        userChoice ?? streaming
    }
}

public enum AgentTranscriptText {
    /// `message_end` against the row streamed for that message: a streamed row keeps its own
    /// segment, since message_end can carry a whole turn (text, tools, more text).
    public static func finalText(streamed: String, messageEnd: String) -> String {
        streamed.isEmpty ? messageEnd : streamed
    }
}

/// Auto-scroll follows new content only while the view is at (or near) the bottom;
/// otherwise "Jump to latest" shows until the user goes back down.
public struct AgentScrollPin: Equatable, Sendable {
    /// How close to the bottom still counts as "at the bottom".
    public static let slack = 40.0

    public private(set) var pinned = true
    /// New content arrived while unpinned.
    public private(set) var unseen = false

    public init() {}

    /// The view moved (the user, or a programmatic scroll).
    public mutating func scrolled(distanceFromBottom: Double) {
        pinned = distanceFromBottom <= Self.slack
        if pinned { unseen = false }
    }

    /// New content: true when the view should scroll to the bottom.
    public mutating func contentAdded() -> Bool {
        if !pinned { unseen = true }
        return pinned
    }

    /// "Jump to latest", or the user sent a message.
    public mutating func jump() {
        pinned = true
        unseen = false
    }
}
