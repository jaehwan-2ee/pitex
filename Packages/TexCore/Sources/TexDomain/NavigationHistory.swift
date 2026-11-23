/// Back/forward history of positions a viewer jumped away from — the PDF
/// preview's link history. A new jump drops the forward entries, like a browser.
public struct NavigationHistory<Position: Equatable> {
    public private(set) var back: [Position] = []
    public private(set) var forward: [Position] = []

    public init() {}

    public var canGoBack: Bool { !back.isEmpty }
    public var canGoForward: Bool { !forward.isEmpty }

    /// Records `current` before a jump. A repeat of the newest entry (a
    /// double-click on a link) is not stored twice.
    public mutating func jump(from current: Position) {
        if back.last != current { back.append(current) }
        forward.removeAll()
    }

    /// The position to return to; `current` becomes the first forward entry.
    public mutating func goBack(from current: Position) -> Position? {
        guard let target = back.popLast() else { return nil }
        forward.append(current)
        return target
    }

    public mutating func goForward(from current: Position) -> Position? {
        guard let target = forward.popLast() else { return nil }
        back.append(current)
        return target
    }
}
