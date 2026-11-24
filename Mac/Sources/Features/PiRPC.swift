import AppPorts
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
    case prompt(message: String, images: [AgentImage] = [], streamingBehavior: String?)
    /// pi's own `steer`: into the running turn now (Claude Code, Codex).
    case steer(message: String, images: [AgentImage] = [])
    case abort
    case newSession
    case switchSession(path: String)
    /// Bridge backends with `capabilities.fork`: a new chat holding the first `keepTurns` turns.
    /// `turns` is how many prompts the chat on screen has; the bridge refuses a cut when its count differs.
    case forkSession(keepTurns: Int, turns: Int)
    /// Bridge backends with `capabilities.rewind`: drop the turns after the first `keepTurns`,
    /// restoring the conversation and/or the agent's file edits; `dryRun` only lists the files.
    case rewind(keepTurns: Int, turns: Int, conversation: Bool, files: Bool, dryRun: Bool)
    case getMessages
    /// Bridge backends: mirror the CLI's own sessions before the history list opens.
    case refreshSessions
    case getState
    case getAvailableModels
    case getSessionStats
    case getCommands
    case getAvailableThinkingLevels
    case setModel(provider: String, modelID: String)
    case setThinkingLevel(level: String)
    case extensionUICancelled(id: String)
    case approvalResponse(id: String, answer: AgentApprovalAnswer)
    /// Bridge backends: pick a permission mode from `get_state.mode.options`.
    case setMode(modeID: String)

    var object: [String: Any] {
        switch self {
        case let .prompt(message, images, streamingBehavior):
            var object: [String: Any] = ["type": "prompt", "message": message]
            if !images.isEmpty { object["images"] = images.map(\.wire) }
            if let streamingBehavior { object["streamingBehavior"] = streamingBehavior }
            return object
        case let .steer(message, images):
            var object: [String: Any] = ["type": "steer", "message": message]
            if !images.isEmpty { object["images"] = images.map(\.wire) }
            return object
        case .abort:
            return ["type": "abort"]
        case .newSession:
            return ["type": "new_session"]
        case let .switchSession(path):
            return ["type": "switch_session", "sessionPath": path]
        case let .forkSession(keepTurns, turns):
            return ["type": "fork_session", "keepTurns": keepTurns, "turns": turns]
        case let .rewind(keepTurns, turns, conversation, files, dryRun):
            var object: [String: Any] = ["type": "rewind", "keepTurns": keepTurns, "turns": turns, "conversation": conversation, "files": files]
            if dryRun { object["dryRun"] = true }
            return object
        case .getMessages:
            return ["type": "get_messages"]
        case .refreshSessions:
            return ["type": "refresh_sessions"]
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
        case let .approvalResponse(id, answer):
            return answer.wireFields.merging(["type": "approval_response", "id": id]) { _, wire in wire }
        case let .setMode(modeID):
            return ["type": "set_mode", "modeId": modeID]
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
    /// Claude Code's `argumentHint` ("<optional instructions>").
    let argumentHint: String?
    let source: String

    var id: String { name }

    init?(_ object: Any) {
        guard let dictionary = object as? [String: Any],
              let name = dictionary["name"] as? String
        else { return nil }
        self.name = name
        description = dictionary["description"] as? String
        argumentHint = (dictionary["argumentHint"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        source = dictionary["source"] as? String ?? ""
    }
}

/// `get_state.capabilities`: may the composer steer a running turn, and attach images.
struct AgentCapabilities: Equatable {
    var steer = false
    var images = false
    /// `fork_session` works; otherwise Fork quotes the earlier chat into a new one.
    var fork = false
    /// What `rewind` can restore.
    var rewindConversation = false
    var rewindFiles = false

    init(steer: Bool = false, images: Bool = false) {
        self.steer = steer
        self.images = images
    }

    /// The bridges report `capabilities`; pi does not, but has `steer` and takes
    /// images when the model's `input` lists them.
    init(state: [String: Any]) {
        if let caps = state["capabilities"] as? [String: Any] {
            steer = caps["steer"] as? Bool ?? false
            images = caps["images"] as? Bool ?? false
            fork = caps["fork"] as? Bool ?? false
            let rewind = caps["rewind"] as? [String: Any]
            rewindConversation = rewind?["conversation"] as? Bool ?? false
            rewindFiles = rewind?["files"] as? Bool ?? false
            return
        }
        steer = true
        if let input = (state["model"] as? [String: Any])?["input"] as? [String] {
            images = input.contains("image")
        } else {
            images = true
        }
    }
}
