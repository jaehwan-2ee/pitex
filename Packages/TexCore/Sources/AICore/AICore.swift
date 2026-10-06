import Foundation

public enum AICoreError: Error, Equatable, Sendable {
    case emptyValue
    case invalidURL
    case invalidTransition
    case contextLimitExceeded(maximumUTF8Bytes: Int)
    case retryLimitExceeded
    case cancelled
    case invalidEditRange
}

private func validatedOpaque(_ value: String, minimumLength: Int = 1) throws -> String {
    guard value.utf8.count >= minimumLength, !value.unicodeScalars.contains(where: { $0.value == 0 }) else { throw AICoreError.emptyValue }
    return value
}

public enum AIRole: String, Codable, Sendable { case system, user, assistant }
public struct ContextMessage: Hashable, Codable, Sendable {
    public let role: AIRole
    public let content: String
    public init(role: AIRole, content: String) throws { self.role = role; self.content = try validatedOpaque(content) }
}
public struct AIContext: Hashable, Codable, Sendable {
    public let messages: [ContextMessage]
    public let maximumUTF8Bytes: Int
    public init(messages: [ContextMessage], maximumUTF8Bytes: Int) throws {
        guard maximumUTF8Bytes > 0 else { throw AICoreError.contextLimitExceeded(maximumUTF8Bytes: maximumUTF8Bytes) }
        guard messages.reduce(0, { $0 + $1.content.utf8.count }) <= maximumUTF8Bytes else { throw AICoreError.contextLimitExceeded(maximumUTF8Bytes: maximumUTF8Bytes) }
        self.messages = messages; self.maximumUTF8Bytes = maximumUTF8Bytes
    }
    public func appending(_ message: ContextMessage) throws -> Self { try Self(messages: messages + [message], maximumUTF8Bytes: maximumUTF8Bytes) }
}

public enum StreamEvent: Hashable, Codable, Sendable {
    case markdownDelta(String)
    case usage(inputUnits: Int, outputUnits: Int)
    case completed
}
public enum StreamState: Hashable, Codable, Sendable {
    case idle
    case streaming(markdown: String)
    case completed(markdown: String)
    case failed(String)
    case cancelled

    public func consuming(_ event: StreamEvent) throws -> Self {
        switch (self, event) {
        case (.idle, let .markdownDelta(delta)): return .streaming(markdown: delta)
        case (let .streaming(markdown), let .markdownDelta(delta)): return .streaming(markdown: markdown + delta)
        case (let .streaming(markdown), .usage): return .streaming(markdown: markdown)
        case (let .streaming(markdown), .completed): return .completed(markdown: markdown)
        default: throw AICoreError.invalidTransition
        }
    }

    public func failing(_ message: String) throws -> Self {
        guard case .streaming = self else { throw AICoreError.invalidTransition }
        return .failed(try validatedOpaque(message))
    }

    public func cancelling() throws -> Self {
        guard case .streaming = self else { throw AICoreError.invalidTransition }
        return .cancelled
    }
}

public struct RetryPolicy: Hashable, Codable, Sendable {
    public let maximumAttempts: Int
    public let baseDelayMilliseconds: UInt64
    public let maximumDelayMilliseconds: UInt64
    public init(maximumAttempts: Int, baseDelayMilliseconds: UInt64, maximumDelayMilliseconds: UInt64) throws {
        guard maximumAttempts > 0, maximumAttempts <= 10, baseDelayMilliseconds <= maximumDelayMilliseconds else { throw AICoreError.retryLimitExceeded }
        self.maximumAttempts = maximumAttempts; self.baseDelayMilliseconds = baseDelayMilliseconds; self.maximumDelayMilliseconds = maximumDelayMilliseconds
    }
    public func delay(beforeAttempt attempt: Int) throws -> UInt64 {
        guard attempt > 1, attempt <= maximumAttempts else { throw AICoreError.retryLimitExceeded }
        let shift = min(attempt - 2, 62)
        let multiplied = baseDelayMilliseconds.multipliedReportingOverflow(by: UInt64(1) << UInt64(shift))
        return min(multiplied.overflow ? UInt64.max : multiplied.partialValue, maximumDelayMilliseconds)
    }
}

public struct CancellationToken: Hashable, Codable, Sendable {
    public let isCancelled: Bool
    public init(isCancelled: Bool = false) { self.isCancelled = isCancelled }
    public func checked() throws { if isCancelled { throw AICoreError.cancelled } }
    public func cancelling() -> Self { Self(isCancelled: true) }
}

public enum MarkdownBlock: Hashable, Codable, Sendable { case prose(String), fencedCode(language: String?, code: String) }
public enum MarkdownParser {
    public static func blocks(in markdown: String) -> [MarkdownBlock] {
        let parts = markdown.components(separatedBy: "```")
        return parts.enumerated().compactMap { index, part in
            guard !part.isEmpty else { return nil }
            if index.isMultiple(of: 2) { return .prose(part) }
            let lines = part.split(separator: "\n", omittingEmptySubsequences: false)
            let language = lines.first.map(String.init).flatMap { $0.isEmpty ? nil : $0 }
            return .fencedCode(language: language, code: lines.dropFirst().joined(separator: "\n"))
        }
    }
}

public struct TextEdit: Hashable, Codable, Sendable {
    public let utf8Offset: Int
    public let utf8Length: Int
    public let replacement: String
    public init(utf8Offset: Int, utf8Length: Int, replacement: String) throws {
        guard utf8Offset >= 0, utf8Length >= 0 else { throw AICoreError.invalidEditRange }
        self.utf8Offset = utf8Offset; self.utf8Length = utf8Length; self.replacement = replacement
    }
}
public enum EditProposalState: Hashable, Codable, Sendable {
    case proposed(id: String, edits: [TextEdit])
    case accepted(id: String, edits: [TextEdit])
    case rejected(id: String)
    case applied(id: String)

    public func accepting() throws -> Self {
        guard case let .proposed(id, edits) = self, !edits.isEmpty else { throw AICoreError.invalidTransition }
        return .accepted(id: id, edits: edits)
    }
    public func rejecting() throws -> Self {
        guard case let .proposed(id, _) = self else { throw AICoreError.invalidTransition }
        return .rejected(id: id)
    }
    public func applied() throws -> Self {
        guard case let .accepted(id, _) = self else { throw AICoreError.invalidTransition }
        return .applied(id: id)
    }
}
