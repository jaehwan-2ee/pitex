import Foundation

/// Composer logic shared with the GTK shell (`composer.rs`): the message
/// queue, image attachments with their caps, and `@` file mentions.
public enum AgentComposer {
    /// Claude's own limits; the bridge refuses the same.
    public static let maxImages = 8
    public static let maxImageBytes = 5 * 1024 * 1024

    /// The image types Claude and Antigravity accept, by file extension.
    public static func imageMIMEType(path: String) -> String? {
        switch (path as NSString).pathExtension.lowercased() {
        case "png": "image/png"
        case "jpg", "jpeg": "image/jpeg"
        case "gif": "image/gif"
        case "webp": "image/webp"
        default: nil
        }
    }

    /// "12 KB", "1.5 MB" for the chip.
    public static func byteSize(_ bytes: Int) -> String {
        if bytes >= 1024 * 1024 {
            return String(format: "%.1f MB", Double(bytes) / (1024 * 1024))
        }
        return "\((bytes + 1023) / 1024) KB"
    }

    /// What Return does with the draft.
    public enum Route: Equatable, Sendable {
        case send
        case steer
        case queue
    }

    /// Return queues while a turn runs or an approval waits (Paseo sends nothing into a turn
    /// that is waiting on the user). Cmd+Return steers when the backend can.
    public static func route(running: Bool, approvalPending: Bool, now: Bool, canSteer: Bool) -> Route {
        if !running && !approvalPending { return .send }
        if now && canSteer && !approvalPending { return .steer }
        return .queue
    }
}

/// One image the user attached: pi's `ImageContent` plus a name and size for the chip.
public struct AgentImage: Equatable, Hashable, Identifiable, Sendable {
    public enum Problem: Error, Equatable, Sendable {
        case tooLarge
        case tooMany
        case unsupported

        /// The `Localizable.strings` key the panel shows.
        public var key: String {
            switch self {
            case .tooLarge: "assistant.image.too_large"
            case .tooMany: "assistant.image.too_many"
            case .unsupported: "assistant.image.unsupported"
            }
        }
    }

    public let id = UUID()
    public let name: String
    public let mimeType: String
    public let data: Data

    /// Checks the type and the size cap; `attached` is how many the draft holds already.
    public init(name: String, mimeType: String, data: Data, attached: Int) throws {
        guard ["image/png", "image/jpeg", "image/gif", "image/webp"].contains(mimeType) else { throw Problem.unsupported }
        guard attached < AgentComposer.maxImages else { throw Problem.tooMany }
        guard data.count <= AgentComposer.maxImageBytes else { throw Problem.tooLarge }
        self.name = name
        self.mimeType = mimeType
        self.data = data
    }

    /// `{type, mimeType, data}`: pi's ImageContent, base64.
    public var wire: [String: Any] {
        ["type": "image", "mimeType": mimeType, "data": data.base64EncodedString()]
    }
}

/// Messages typed while a turn runs (Paseo's queue): one is sent per agent_end.
public struct AgentMessageQueue: Equatable, Sendable {
    public enum State: Equatable, Sendable {
        case queued
        /// Sent at agent_end; removed when the prompt is accepted, back to queued if refused.
        case sending
    }

    public struct Item: Equatable, Identifiable, Sendable {
        public let id: Int
        public var text: String
        public var images: [AgentImage]
        public var state: State
    }

    public private(set) var items: [Item] = []
    /// Stop and a refused send hold the queue until the user sends again.
    public var paused = false
    private var nextID = 0

    public init() {}

    @discardableResult
    public mutating func enqueue(_ text: String, images: [AgentImage] = []) -> Int {
        nextID += 1
        items.append(Item(id: nextID, text: text, images: images, state: .queued))
        return nextID
    }

    /// The head to send at agent_end, marked sending; nil while paused or already sending.
    public mutating func takeNext() -> Item? {
        guard !paused, let head = items.first, head.state == .queued else { return nil }
        items[0].state = .sending
        return items[0]
    }

    /// The backend accepted it.
    public mutating func sent(_ id: Int) {
        items.removeAll { $0.id == id }
    }

    /// The backend refused it: back at the front, and the queue waits for the user.
    public mutating func failed(_ id: Int) {
        guard let index = items.firstIndex(where: { $0.id == id }) else { return }
        var item = items.remove(at: index)
        item.state = .queued
        items.insert(item, at: 0)
        paused = true
    }

    /// Edit, Delete and Send now take the row out.
    public mutating func remove(_ id: Int) -> Item? {
        guard let index = items.firstIndex(where: { $0.id == id }) else { return nil }
        return items.remove(at: index)
    }

    public mutating func clear() {
        items = []
        paused = false
    }
}

/// `@query` before the caret, in Character offsets.
public struct AgentFileMention: Equatable, Sendable {
    public var start: Int
    public var end: Int
    public var query: String

    public init(start: Int, end: Int, query: String) {
        self.start = start
        self.end = end
        self.query = query
    }

    /// Paseo's rule: the `@` nearest the caret, not inside an email-like word,
    /// with no whitespace or quote between it and the caret.
    public static func active(in text: String, cursor: Int) -> AgentFileMention? {
        let chars = Array(text)
        let cursor = min(max(cursor, 0), chars.count)
        var at = cursor
        while at > 0 {
            at -= 1
            let c = chars[at]
            if c == "@" {
                if at > 0, chars[at - 1].isLetter || chars[at - 1].isNumber || chars[at - 1] == "_" || chars[at - 1] == "+" {
                    continue
                }
                return AgentFileMention(start: at, end: cursor, query: String(chars[(at + 1)..<cursor]))
            }
            if c.isWhitespace || "\"'<>".contains(c) { return nil }
        }
        return nil
    }

    /// Fuzzy path match: every query character in order. File-name hits rank first,
    /// then earlier and tighter matches, then shorter paths.
    public static func matches(_ files: [String], query: String, limit: Int = 8) -> [String] {
        let query = query.lowercased()
        let scored: [(Int, String)] = files.compactMap { path in
            let lower = path.lowercased()
            let name = lower.split(separator: "/").last.map(String.init) ?? lower
            if query.isEmpty || name.hasPrefix(query) { return (0, path) }
            if name.contains(query) { return (1, path) }
            if lower.contains(query) { return (2, path) }
            guard let spread = subsequenceSpread(lower, query) else { return nil }
            return (3 + spread, path)
        }
        return scored
            .sorted { ($0.0, $0.1.count, $0.1) < ($1.0, $1.1.count, $1.1) }
            .prefix(limit)
            .map(\.1)
    }

    private static func subsequenceSpread(_ text: String, _ query: String) -> Int? {
        let chars = Array(text)
        var index = 0
        var first: Int?
        var last = 0
        for q in query {
            while index < chars.count, chars[index] != q { index += 1 }
            guard index < chars.count else { return nil }
            if first == nil { first = index }
            last = index
            index += 1
        }
        return last - (first ?? 0)
    }

    /// Replaces `@query` with `@path ` (quoted when the path has a space); returns the new
    /// text and the caret after the inserted space.
    public func apply(to text: String, path: String) -> (text: String, cursor: Int) {
        let chars = Array(text)
        let token = path.contains(where: \.isWhitespace)
            ? "@\"\(path.replacingOccurrences(of: "\"", with: "\\\""))\" "
            : "@\(path) "
        let before = String(chars[..<min(start, chars.count)])
        let after = String(chars[min(end, chars.count)...])
        return (before + token + after, before.count + token.count)
    }
}
