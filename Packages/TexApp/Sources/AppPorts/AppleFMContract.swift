import Foundation

public enum AppleFMAvailability: Equatable, Sendable {
    case available(contextTokens: Int)
    case unavailable(AppleFMUnavailableReason)
}

public enum AppleFMUnavailableReason: String, Equatable, Sendable {
    case deviceNotEligible
    case appleIntelligenceNotEnabled
    case modelNotReady
    case unsupportedSystem
    case unknown

    public var message: String {
        switch self {
        case .deviceNotEligible:
            "This Mac can't run Apple Intelligence, so Apple Foundation Models are unavailable."
        case .appleIntelligenceNotEnabled:
            "Turn on Apple Intelligence in System Settings, then try again."
        case .modelNotReady:
            "Apple Intelligence is still preparing the on-device model. Try again when it finishes."
        case .unsupportedSystem:
            "Apple Foundation Models need macOS 26 or later on Apple silicon."
        case .unknown:
            "Apple Foundation Models are unavailable."
        }
    }
}

public enum AppleFMHost {
    public static let unavailableMessage =
        "Apple Foundation Models run in the Pitex Mac app when Apple Intelligence is on. This build cannot start them."
}

public enum AppleFMCatalog {
    public static let providerID = "apple-fm"
    public static let modelID = "system"
    public static let modelProvider = "apple"
    public static let modelName = "Apple Foundation Model"
    public static let defaultContextTokens = 4096
    public static let thinkingLevel = "off"

    public static func model(contextTokens: Int) -> AppleFMJSON {
        .object([
            "id": .string(modelID),
            "provider": .string(modelProvider),
            "name": .string(modelName),
            "contextWindow": .int(contextTokens),
        ])
    }
}

public enum AppleFMPromptFit: Equatable, Sendable {
    case ready(String)
    case trimmed(String)
    case rejected(String)
}

public enum AppleFMPromptBudget {
    public static func inputByteBudget(contextTokens: Int) -> Int {
        let tokens = max(contextTokens, 1)
        let bytesPerToken = 4
        let inputShareDenominator = 2
        return tokens * bytesPerToken / inputShareDenominator
    }

    public static func fit(_ message: String, contextTokens: Int) -> AppleFMPromptFit {
        let budget = inputByteBudget(contextTokens: contextTokens)
        if budget < 64 {
            return .rejected(AppleFMUnavailableReason.unknown.message)
        }
        if message.utf8.count <= budget {
            return .ready(message)
        }
        let shortened = omitEditorContext(message)
        if shortened.utf8.count <= budget {
            return .trimmed(shortened)
        }
        let tail = userText(in: shortened)
        let notice = "…\n(truncated to fit the on-device context window)\n"
        let noticeCost = notice.utf8.count
        guard budget > noticeCost else {
            return .rejected("The prompt is larger than the on-device context window.")
        }
        let room = budget - noticeCost
        let kept = suffix(of: tail, utf8Limit: room)
        return .trimmed(notice + kept)
    }

    private static func omitEditorContext(_ message: String) -> String {
        guard message.hasPrefix("<editor-context>"),
              let end = message.range(of: "\n</editor-context>\n") else {
            return message
        }
        let rest = message[end.upperBound...]
        return """
        <editor-context>
        Editor context omitted to fit the on-device context window.
        </editor-context>

        \(rest)
        """
    }

    private static func userText(in message: String) -> String {
        guard let end = message.range(of: "\n</editor-context>\n") else { return message }
        return String(message[end.upperBound...])
    }

    private static func suffix(of text: String, utf8Limit: Int) -> String {
        let bytes = Array(text.utf8)
        guard bytes.count > utf8Limit else { return text }
        var start = bytes.count - utf8Limit
        while start < bytes.count, bytes[start] & 0b1100_0000 == 0b1000_0000 {
            start += 1
        }
        return String(decoding: bytes[start...], as: UTF8.self)
    }
}

public enum AppleFMToolPermission: Equatable, Sendable {
    case allow
    case ask(kind: String, title: String)
    case refuse
}

public enum AppleFMTools {
    public static let read = "read_file"
    public static let write = "write_file"
    public static let run = "run_command"

    public static func permission(for name: String, rememberedKinds: Set<String> = []) -> AppleFMToolPermission {
        switch name {
        case read:
            return .allow
        case write where rememberedKinds.contains("file"):
            return .allow
        case write:
            return .ask(kind: "file", title: "Write file")
        case run where rememberedKinds.contains("command"):
            return .allow
        case run:
            return .ask(kind: "command", title: "Run command")
        default:
            return .refuse
        }
    }
}

public enum AppleFMApprovalDecision: Equatable, Sendable {
    case approve
    case deny
    case alwaysAllow

    public init(wire: String) {
        switch wire {
        case "approve": self = .approve
        case "always_allow": self = .alwaysAllow
        default: self = .deny
        }
    }
}

public enum AppleFMPaths {
    public static func relative(_ raw: String) -> String? {
        if raw.isEmpty || raw.contains("\0") { return nil }
        if raw.hasPrefix("/") || raw.hasPrefix("\\") { return nil }
        if raw.contains(":") { return nil }
        let parts = raw.split(whereSeparator: { $0 == "/" || $0 == "\\" }).map(String.init)
        if parts.isEmpty || parts.contains(where: { $0 == ".." || $0 == "" }) { return nil }
        let cleaned = parts.filter { $0 != "." }
        if cleaned.isEmpty { return nil }
        return cleaned.joined(separator: "/")
    }
}

public enum AppleFMJSON: Equatable, Sendable {
    case string(String)
    case int(Int)
    case double(Double)
    case bool(Bool)
    case object([String: AppleFMJSON])
    case array([AppleFMJSON])

    public func string(_ key: String) -> String? {
        guard case .object(let fields) = self, case .string(let value) = fields[key] else { return nil }
        return value
    }

    public func object(_ key: String) -> [String: AppleFMJSON]? {
        guard case .object(let fields) = self, case .object(let value) = fields[key] else { return nil }
        return value
    }

    public func array(_ key: String) -> [AppleFMJSON]? {
        guard case .object(let fields) = self, case .array(let value) = fields[key] else { return nil }
        return value
    }

    public subscript(key: String) -> AppleFMJSON? {
        guard case .object(let fields) = self else { return nil }
        return fields[key]
    }
}

public enum AppleFMEvents {
    public static func response(
        command: String,
        id: String? = nil,
        success: Bool,
        data: AppleFMJSON? = nil,
        error: String? = nil
    ) -> AppleFMJSON {
        var fields: [String: AppleFMJSON] = [
            "type": .string("response"),
            "command": .string(command),
            "success": .bool(success),
        ]
        if let id { fields["id"] = .string(id) }
        if let data { fields["data"] = data }
        if let error { fields["error"] = .string(error) }
        return .object(fields)
    }

    public static func state(contextTokens: Int, id: String?) -> AppleFMJSON {
        response(
            command: "get_state",
            id: id,
            success: true,
            data: .object([
                "model": AppleFMCatalog.model(contextTokens: contextTokens),
                "thinkingLevel": .string(AppleFMCatalog.thinkingLevel),
            ])
        )
    }

    public static func models(contextTokens: Int, id: String?) -> AppleFMJSON {
        response(
            command: "get_available_models",
            id: id,
            success: true,
            data: .object(["models": .array([AppleFMCatalog.model(contextTokens: contextTokens)])])
        )
    }

    public static func thinkingLevels(id: String?) -> AppleFMJSON {
        response(
            command: "get_available_thinking_levels",
            id: id,
            success: true,
            data: .object(["levels": .array([.string(AppleFMCatalog.thinkingLevel)])])
        )
    }

    public static func sessionStats(totalTokens: Int, contextTokens: Int, id: String?) -> AppleFMJSON {
        let percent = contextTokens > 0
            ? Double(min(totalTokens, contextTokens)) / Double(contextTokens) * 100
            : 0
        return response(
            command: "get_session_stats",
            id: id,
            success: true,
            data: .object([
                "tokens": .object(["total": .int(totalTokens)]),
                "cost": .double(0),
                "contextUsage": .object([
                    "tokens": .int(min(totalTokens, contextTokens)),
                    "contextWindow": .int(contextTokens),
                    "percent": .double(percent),
                ]),
            ])
        )
    }

    public static func commands(id: String?) -> AppleFMJSON {
        response(command: "get_commands", id: id, success: true, data: .object(["commands": .array([])]))
    }

    public static func setModel(provider: String, modelID: String, contextTokens: Int, id: String?) -> AppleFMJSON {
        guard provider == AppleFMCatalog.modelProvider, modelID == AppleFMCatalog.modelID else {
            return response(command: "set_model", id: id, success: false, error: "Apple Foundation Models only provide the on-device system model.")
        }
        return response(command: "set_model", id: id, success: true, data: AppleFMCatalog.model(contextTokens: contextTokens))
    }

    public static func setThinkingLevel(_ level: String, id: String?) -> AppleFMJSON {
        guard level == AppleFMCatalog.thinkingLevel else {
            return response(
                command: "set_thinking_level",
                id: id,
                success: false,
                error: "Apple Foundation Models do not expose reasoning levels."
            )
        }
        return response(command: "set_thinking_level", id: id, success: true)
    }

    public static func notify(_ message: String) -> AppleFMJSON {
        .object([
            "type": .string("extension_ui_request"),
            "method": .string("notify"),
            "message": .string(message),
        ])
    }

    public static func approval(id: String, kind: String, title: String, detail: String) -> AppleFMJSON {
        .object([
            "type": .string("approval_request"),
            "id": .string(id),
            "kind": .string(kind),
            "title": .string(title),
            "detail": .string(detail),
        ])
    }

    public static func toolStart(id: String, name: String, arguments: [String: String]) -> AppleFMJSON {
        var args: [String: AppleFMJSON] = [:]
        for (key, value) in arguments { args[key] = .string(value) }
        return .object([
            "type": .string("tool_execution_start"),
            "toolCallId": .string(id),
            "toolName": .string(name),
            "args": .object(args),
        ])
    }

    public static func toolEnd(id: String, text: String, isError: Bool) -> AppleFMJSON {
        .object([
            "type": .string("tool_execution_end"),
            "toolCallId": .string(id),
            "isError": .bool(isError),
            "result": .object([
                "content": .array([.object(["type": .string("text"), "text": .string(text)])]),
            ]),
        ])
    }
}

public struct AppleFMStreamCursor: Equatable, Sendable {
    public private(set) var text = ""
    public private(set) var opened = false

    public init() {}

    public mutating func consume(snapshot: String) -> [AppleFMJSON] {
        var events = openIfNeeded()
        let delta: String
        if snapshot.hasPrefix(text) {
            delta = String(snapshot.dropFirst(text.count))
            text = snapshot
        } else {
            delta = ""
            text = snapshot
        }
        if !delta.isEmpty {
            events.append(Self.textEvent("text_delta", delta: delta))
        }
        return events
    }

    public mutating func close(stopReason: String, errorMessage: String?) -> [AppleFMJSON] {
        var events = openIfNeeded()
        var message: [String: AppleFMJSON] = [
            "role": .string("assistant"),
            "stopReason": .string(stopReason),
            "content": .array([.object(["type": .string("text"), "text": .string(text)])]),
        ]
        if let errorMessage { message["errorMessage"] = .string(errorMessage) }
        events.append(.object(["type": .string("message_end"), "message": .object(message)]))
        events.append(.object(["type": .string("turn_end")]))
        var end: [String: AppleFMJSON] = ["type": .string("agent_end")]
        if stopReason == "error" {
            end["messages"] = .array([.object(message)])
        }
        events.append(.object(end))
        opened = false
        text = ""
        return events
    }

    private mutating func openIfNeeded() -> [AppleFMJSON] {
        guard !opened else { return [] }
        opened = true
        return [
            .object(["type": .string("agent_start")]),
            .object(["type": .string("message_start"), "message": .object(["role": .string("assistant")])]),
            Self.textEvent("text_start", delta: nil),
        ]
    }

    private static func textEvent(_ kind: String, delta: String?) -> AppleFMJSON {
        var assistant: [String: AppleFMJSON] = [
            "type": .string(kind),
            "contentIndex": .int(0),
        ]
        if let delta { assistant["delta"] = .string(delta) }
        return .object([
            "type": .string("message_update"),
            "assistantMessageEvent": .object(assistant),
        ])
    }
}
