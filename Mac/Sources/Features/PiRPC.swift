import Foundation

/// Minimal JSON value model for the pi `--mode rpc` wire protocol. The
/// protocol is JSONL: one LF-delimited object per line on stdin (commands)
/// and stdout (responses and events). Values are intentionally untyped
/// because the schema evolves with the pi release; the coordinator reads the
/// fields it needs and ignores the rest.
enum PiJSON {
    static func encode(_ object: [String: Any]) throws -> Data {
        var data = try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])
        data.append(0x0a)
        return data
    }

    static func decode(_ line: Data) -> [String: Any]? {
        guard !line.isEmpty,
              let object = try? JSONSerialization.jsonObject(with: line),
              let dictionary = object as? [String: Any]
        else { return nil }
        return dictionary
    }
}

/// Outbound commands accepted by the pi rpc subprocess.
enum PiRPCCommand {
    case prompt(message: String, streamingBehavior: String?)
    case steer(message: String)
    case abort
    case newSession
    case switchSession(path: String)
    case getMessages
    case getState
    case getAvailableModels
    case getSessionStats
    case getCommands
    case getAvailableThinkingLevels
    case setModel(provider: String, modelID: String)
    case setThinkingLevel(level: String)
    case extensionUICancelled(id: String)

    var object: [String: Any] {
        switch self {
        case let .prompt(message, streamingBehavior):
            var object: [String: Any] = ["type": "prompt", "message": message]
            if let streamingBehavior { object["streamingBehavior"] = streamingBehavior }
            return object
        case let .steer(message):
            return ["type": "steer", "message": message]
        case .abort:
            return ["type": "abort"]
        case .newSession:
            return ["type": "new_session"]
        case let .switchSession(path):
            return ["type": "switch_session", "sessionPath": path]
        case .getMessages:
            return ["type": "get_messages"]
        case .getState:
            return ["type": "get_state"]
        case .getAvailableModels:
            return ["type": "get_available_models"]
        case .getSessionStats:
            return ["type": "get_session_stats"]
        case .getCommands:
            return ["type": "get_commands"]
        case .getAvailableThinkingLevels:
            return ["type": "get_available_thinking_levels"]
        case let .setModel(provider, modelID):
            return ["type": "set_model", "provider": provider, "modelId": modelID]
        case let .setThinkingLevel(level):
            return ["type": "set_thinking_level", "level": level]
        case let .extensionUICancelled(id):
            return ["type": "extension_ui_response", "id": id, "cancelled": true]
        }
    }
}

/// One decoded stdout line from the pi rpc subprocess.
struct PiRPCEvent {
    let type: String
    let object: [String: Any]

    init?(_ object: [String: Any]) {
        guard let type = object["type"] as? String else { return nil }
        self.type = type
        self.object = object
    }

    var string: (String) -> String? {
        { object[$0] as? String }
    }

    var bool: (String) -> Bool {
        { (object[$0] as? Bool) ?? false }
    }

    var nested: (String) -> [String: Any]? {
        { object[$0] as? [String: Any] }
    }

    /// `true` when the event is a command acknowledgement rather than an
    /// asynchronous agent event.
    var isResponse: Bool { type == "response" }

    var responseSucceeded: Bool { bool("success") }

    var responseError: String? { string("error") }

    var responseCommand: String? { string("command") }

    var responseData: [String: Any]? { nested("data") }
}

/// A model entry returned by `get_available_models` / `get_state`.
struct PiModelDescriptor: Hashable, Identifiable {
    let id: String
    let provider: String
    let name: String
    /// The model's context window size in tokens, when pi reports it.
    let contextWindow: Int?

    init?(_ object: Any) {
        guard let dictionary = object as? [String: Any],
              let id = dictionary["id"] as? String,
              let provider = dictionary["provider"] as? String
        else { return nil }
        self.id = id
        self.provider = provider
        name = dictionary["name"] as? String ?? id
        contextWindow = dictionary["contextWindow"] as? Int
    }

    var pickerTitle: String { name == id ? id : "\(name) (\(id))" }
}

/// `get_session_stats` response payload — token totals, cost, and the
/// context-window occupancy pi reports for the current session.
struct PiSessionStats {
    var totalTokens = 0
    var cost = 0.0
    var contextTokens: Int?
    var contextWindow: Int?
    var contextPercent: Double?

    init?(_ object: Any) {
        guard let dictionary = object as? [String: Any] else { return nil }
        let tokens = dictionary["tokens"] as? [String: Any]
        let usage = dictionary["contextUsage"] as? [String: Any]
        totalTokens = tokens?["total"] as? Int ?? 0
        cost = dictionary["cost"] as? Double ?? 0
        contextTokens = usage?["tokens"] as? Int
        contextWindow = usage?["contextWindow"] as? Int
        contextPercent = usage?["percent"] as? Double
    }
}

/// One slash command returned by `get_commands` (extension, prompt, or
/// skill — pi expands the `/name` prefix itself when the prompt is sent raw).
struct PiSlashCommand: Identifiable {
    let name: String
    let description: String?
    let source: String

    var id: String { name }

    init?(_ object: Any) {
        guard let dictionary = object as? [String: Any],
              let name = dictionary["name"] as? String
        else { return nil }
        self.name = name
        description = dictionary["description"] as? String
        source = dictionary["source"] as? String ?? ""
    }
}
