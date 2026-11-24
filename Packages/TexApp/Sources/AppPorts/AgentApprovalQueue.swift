import Foundation

/// One CLI approval the assistant is waiting on.
///
/// `AgentCoordinator` keeps these in `AgentApprovalQueue` and shows `head`.
public struct AgentApprovalItem: Equatable, Sendable {
    public var id: String
    /// command, file, permissions, plan, or question.
    public var kind: String
    public var title: String
    public var detail: String
    /// The typed tool detail (the edit diff, the command line).
    public var toolDetail: AgentToolDetail?
    /// Buttons from the bridge; empty means Approve / Deny / Always allow.
    public var actions: [AgentApprovalAction] = []
    public var questions: [AgentApprovalQuestion] = []
    /// Markdown of a plan to review.
    public var plan: String?

    public init(id: String, kind: String, title: String, detail: String) {
        self.id = id
        self.kind = kind
        self.title = title
        self.detail = detail
    }
}

/// FIFO of unanswered approvals, keyed by request id.
///
/// A second request waits behind the one on screen. Answering removes the
/// head. `drain()` is what stop and new-session use so every outstanding
/// request is denied, not only the one currently shown.
public struct AgentApprovalQueue: Equatable, Sendable {
    private var items: [AgentApprovalItem] = []

    public init() {}

    public var head: AgentApprovalItem? { items.first }

    public var count: Int { items.count }

    /// Inserts `item` at the tail. The same id updates in place so a repeat
    /// notification cannot open a second slot or change who is at the head.
    public mutating func enqueue(_ item: AgentApprovalItem) {
        if let index = items.firstIndex(where: { $0.id == item.id }) {
            items[index] = item
            return
        }
        items.append(item)
    }

    /// Drops a request the bridge settled itself (`approval_resolved`).
    public mutating func remove(id: String) {
        items.removeAll { $0.id == id }
    }

    /// Removes the request the panel is showing. With `id`, only when that
    /// request is still the head, so a click on a card the bridge already
    /// settled cannot answer the request queued behind it.
    public mutating func dequeueHead(id: String? = nil) -> AgentApprovalItem? {
        guard let head = items.first, id == nil || head.id == id else { return nil }
        return items.removeFirst()
    }

    /// Every unanswered request, oldest first. The queue is empty afterwards.
    public mutating func drain() -> [AgentApprovalItem] {
        let queued = items
        items = []
        return queued
    }

    public mutating func clear() {
        items = []
    }
}
