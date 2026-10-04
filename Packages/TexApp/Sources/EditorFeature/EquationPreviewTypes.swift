import LanguageCore

public enum EquationPreviewPlacement: String, Hashable, Sendable {
    case above, below
}

public enum EquationPreviewRendererMode: String, Hashable, Sendable {
    case fast
    case fastWithTeXFallback
}

/// `pitex.pref.equationPreview.*` as the engine consumes them.
public struct EquationPreviewSettings: Equatable, Sendable {
    public static let allowedDelays: [UInt64] = [0, 80, 150]

    public var enabled: Bool
    public var whileTyping: Bool
    public var placement: EquationPreviewPlacement
    public var renderer: EquationPreviewRendererMode
    public var delayMilliseconds: UInt64

    public init(
        enabled: Bool = true,
        whileTyping: Bool = true,
        placement: EquationPreviewPlacement = .above,
        renderer: EquationPreviewRendererMode = .fast,
        delayMilliseconds: UInt64 = 80
    ) {
        self.enabled = enabled
        self.whileTyping = whileTyping
        self.placement = placement
        self.renderer = renderer
        self.delayMilliseconds = Self.allowedDelays.contains(delayMilliseconds) ? delayMilliseconds : 80
    }

    /// From the persisted strings; unknown values fall back to defaults.
    public init(enabled: Bool, whileTyping: Bool, placement: String, renderer: String, delayMilliseconds: Int) {
        self.init(
            enabled: enabled,
            whileTyping: whileTyping,
            placement: EquationPreviewPlacement(rawValue: placement) ?? .above,
            renderer: EquationPreviewRendererMode(rawValue: renderer) ?? .fast,
            // Negative persisted values are invalid, not "instant": the
            // conservative default is 80, while a stored 0 stays 0 (C13).
            delayMilliseconds: delayMilliseconds < 0 ? 80 : UInt64(delayMilliseconds)
        )
    }
}

public enum EquationPreviewScheme: String, Hashable, Sendable {
    case light, dark, highContrastLight, highContrastDark
}

public struct EquationPreviewAppearance: Hashable, Sendable {
    public var scheme: EquationPreviewScheme
    /// Editor font size in points; the renderer derives em/ex metrics.
    public var fontSize: Double

    public init(scheme: EquationPreviewScheme = .light, fontSize: Double = 13) {
        self.scheme = scheme
        self.fontSize = fontSize
    }
}

public enum EquationPreviewTrigger: String, Hashable, Sendable {
    case caret, hover
}

/// Fast-preview cache identity — never the caret position.
public struct EquationPreviewKey: Hashable, Sendable {
    public let source: String
    public let displayMode: Bool
    public let contextKey: String
    public let rendererVersion: String
    /// Only appearance inputs that alter the rendered SVG belong here:
    /// fontSize changes metrics; the color scheme is applied by the host
    /// when presenting, so a theme change must re-present, not re-typeset.
    public let fontSize: Double

    /// Runs of spaces/tabs are one TeX space; line breaks stay because a
    /// `%` comment ends at them.
    public static func normalize(_ source: String) -> String {
        var result = ""
        result.reserveCapacity(source.utf8.count)
        var pendingSpace = false
        for character in source {
            if character == " " || character == "\t" {
                pendingSpace = true
                continue
            }
            if pendingSpace, !result.isEmpty, character != "\n", character != "\r\n" { result.append(" ") }
            pendingSpace = false
            result.append(character)
        }
        return result.trimmingNewlines()
    }
}

public struct EquationRenderRequest: Hashable, Sendable {
    public let generation: UInt64
    public let key: EquationPreviewKey
    /// Structured JSON arguments for the renderer page — never spliced into
    /// script text.
    public let source: String
    public let displayMode: Bool
    public let definitions: [String]
    public let fontSize: Double

    public var contextKey: String { key.contextKey }
}

public enum EquationRenderFailure: String, Hashable, Sendable {
    /// Unknown macro or environment — fast preview cannot know it.
    case undefinedCommand
    /// Syntax error (typically an unfinished edit).
    case invalid
    /// Exceeds the source/expansion bounds.
    case tooLarge
    /// The page failed (timeout, crashed process); not cached.
    case rendererFailed
}

public enum EquationRenderOutcome: Hashable, Sendable {
    case svg(String)
    case failed(EquationRenderFailure)
}

public struct ExactEquationRequest: Hashable, Sendable {
    public let generation: UInt64
    public let key: String
    /// Complete TeX document; the host compiles it with the project's
    /// engine in an isolated temporary directory.
    public let document: String
}

public enum ExactEquationOutcome: Hashable, Sendable {
    case pdf([UInt8])
    case failed
}

public enum EquationPreviewUnavailableReason: String, Hashable, Sendable {
    /// A command/environment the fast renderer does not know.
    case unsupported
    case tooLarge
    /// Exact fallback requested but the build command names no supported engine.
    case exactUnavailable
    case exactFailed
}

public struct EquationPreviewPresentation: Hashable, Sendable {
    public enum Content: Hashable, Sendable {
        case fast(svg: String)
        case exact(pdf: [UInt8])
        case unavailable(EquationPreviewUnavailableReason)
    }

    public let generation: UInt64
    public let content: Content
    /// Region in host text units (UTF-16), for anchoring.
    public let anchor: Range<Int>
    public let anchorUTF8: SourceRange
    public let displayMode: Bool
    public let placement: EquationPreviewPlacement
    public let trigger: EquationPreviewTrigger
}

public enum EquationPreviewCommand: Hashable, Sendable {
    case render(EquationRenderRequest)
    case renderExact(ExactEquationRequest)
    /// Kill the exact job for this key; its completion is still reported.
    case cancelExact(key: String)
    case show(EquationPreviewPresentation)
    case hide
    case resolveIncludes([MathIncludeRequest])
}

/// Bounded LRU by entry count and total cost.
struct EquationPreviewLRU<Key: Hashable, Value> {
    private var entries: [Key: (value: Value, cost: Int, stamp: UInt64)] = [:]
    private var clock: UInt64 = 0
    private var totalCost = 0
    let capacity: Int
    let byteBudget: Int

    init(capacity: Int, byteBudget: Int) {
        self.capacity = capacity
        self.byteBudget = byteBudget
    }

    var count: Int { entries.count }

    func contains(_ key: Key) -> Bool { entries[key] != nil }

    mutating func value(for key: Key) -> Value? {
        guard var entry = entries[key] else { return nil }
        clock += 1
        entry.stamp = clock
        entries[key] = entry
        return entry.value
    }

    mutating func insert(_ key: Key, _ value: Value, cost: Int) {
        guard cost <= byteBudget else { return }
        if let old = entries[key] { totalCost -= old.cost }
        clock += 1
        entries[key] = (value, cost, clock)
        totalCost += cost
        while entries.count > capacity || totalCost > byteBudget {
            // ponytail: O(n) eviction scan over <=256 entries; a linked list if capacity grows.
            guard let oldest = entries.min(by: { $0.value.stamp < $1.value.stamp }) else { break }
            totalCost -= oldest.value.cost
            entries[oldest.key] = nil
        }
    }

    mutating func removeAll() {
        entries.removeAll()
        totalCost = 0
    }
}

private extension String {
    func trimmingNewlines() -> String {
        let isBlank: (Character) -> Bool = { $0 == "\n" || $0 == "\r\n" || $0 == "\r" || $0 == " " }
        let leading = drop(while: isBlank)
        return String(leading.reversed().drop(while: isBlank).reversed())
    }
}
