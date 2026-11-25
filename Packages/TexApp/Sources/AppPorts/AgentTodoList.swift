import Foundation

/// One row of the agent's todo list (`todo_update.items`, `get_messages.todos`).
public struct AgentTodoItem: Equatable, Sendable {
    public enum Status: String, Sendable {
        case pending
        case inProgress = "in_progress"
        case completed
    }

    public var id: String?
    public var text: String
    public var status: Status
    public var activeForm: String?

    public init(id: String? = nil, text: String, status: Status, activeForm: String? = nil) {
        self.id = id
        self.text = text
        self.status = status
        self.activeForm = activeForm
    }

    /// The bridge's list; rows without text are dropped.
    public static func list(_ value: Any?) -> [AgentTodoItem] {
        guard let rows = value as? [Any] else { return [] }
        func text(_ row: [String: Any], _ key: String) -> String? {
            let value = (row[key] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines)
            return value?.isEmpty == false ? value : nil
        }
        return rows.compactMap { raw in
            guard let row = raw as? [String: Any], let label = text(row, "text") else { return nil }
            let status = (row["status"] as? String).flatMap(Status.init(rawValue:)) ?? .pending
            return AgentTodoItem(id: text(row, "id"), text: label, status: status, activeForm: text(row, "activeForm"))
        }
    }
}

/// One transcript line for a change between two todo snapshots (Paseo's task activity).
public enum AgentTodoActivity: Equatable, Sendable {
    case created(Int)
    case added(String)
    case started(String)
    case completed(String)

    /// `Localizable.strings` key; its one `%@` takes `argument`.
    public var key: String {
        switch self {
        case .created: return "assistant.todo.created"
        case .added: return "assistant.todo.added"
        case .started: return "assistant.todo.started"
        case .completed: return "assistant.todo.completed"
        }
    }

    public var argument: String {
        switch self {
        case .created(let count): return String(count)
        case .added(let text), .started(let text), .completed(let text): return text
        }
    }
}

/// The checklist's diff and summary; the same rules as the Linux `agent.rs`.
public enum AgentTodoList {
    /// What changed from `previous` to `current`: a new list, new rows, and rows that
    /// started or finished. Rows match by id, else by text (the first unmatched row
    /// with it), so a dropped or moved row does not shift the rest. A list that shares
    /// no row with the previous one is a new list.
    public static func activities(from previous: [AgentTodoItem], to current: [AgentTodoItem]) -> [AgentTodoActivity] {
        if current.isEmpty { return [] }
        var unmatched = previous
        let activities: [AgentTodoActivity] = current.compactMap { item in
            let found = unmatched.firstIndex { was in
                switch (was.id, item.id) {
                case let (a?, b?): return a == b
                case (nil, nil): return was.text == item.text
                default: return false
                }
            }
            guard let found else { return .added(item.text) }
            let was = unmatched.remove(at: found).status
            guard was != item.status else { return nil }
            switch item.status {
            case .completed: return .completed(item.text)
            case .inProgress: return .started(item.text)
            case .pending: return nil
            }
        }
        return unmatched.count == previous.count ? [.created(current.count)] : activities
    }

    /// (done, total) for the checklist header and its accessible label.
    public static func progress(_ items: [AgentTodoItem]) -> (done: Int, total: Int) {
        (items.filter { $0.status == .completed }.count, items.count)
    }

    /// What the agent is doing now: the first in-progress row's active form, or its text.
    public static func current(_ items: [AgentTodoItem]) -> String? {
        guard let item = items.first(where: { $0.status == .inProgress }) else { return nil }
        return item.activeForm ?? item.text
    }
}
