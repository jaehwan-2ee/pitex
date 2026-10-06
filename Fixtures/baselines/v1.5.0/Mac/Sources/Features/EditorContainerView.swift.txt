import AppKit
import EditorMacAdapter
import LanguageCore
import SwiftUI

struct EditorContainerView: NSViewRepresentable {
    let adapter: EditorMacAdapter
    var minimapVisible = true
    var foldingEnabled = true
    /// The workspace's ghost-text completion session — its overlay mounts
    /// here and its accept/dismiss keys are intercepted by the Coordinator.
    var completion: GhostCompletionCoordinator?
    /// Fired when the user Cmd-clicks a location: (line, column), 1-based.
    var onSyncRequest: ((Int, Int) -> Void)?

    func makeNSView(context: Context) -> NSScrollView {
        let textView = adapter.textView
        textView.isEditable = true
        textView.isSelectable = true
        textView.isRichText = false
        textView.usesFindPanel = true
        textView.isIncrementalSearchingEnabled = true
        textView.allowsUndo = true
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isContinuousSpellCheckingEnabled = false
        textView.font = AppearanceSettings.shared.editorFont
        // Room for the gutter overlay (fold triangles + numbers) on the left.
        textView.textContainerInset = NSSize(width: LineNumberGutterView.width + 10, height: 12)
        textView.minSize = NSSize(width: 0, height: 0)
        textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.autoresizingMask = [.width]
        textView.textContainer?.widthTracksTextView = true
        textView.setAccessibilityIdentifier("pitex.editor.text")
        textView.setAccessibilityLabel(String(localized: "editor.title"))

        let scrollView = NSScrollView()
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.borderType = .noBorder
        scrollView.drawsBackground = true
        scrollView.clipsToBounds = true
        scrollView.documentView = textView
        scrollView.setAccessibilityIdentifier("pitex.editor.scroll")

        // A plain overlay gutter is used instead of NSRulerView: ruler tiling
        // shifts the clip view's bounds.origin by ruleThickness on every tile
        // pass, and repeated layouts accumulate that offset until the document
        // scrolls off-screen entirely.
        let gutter = LineNumberGutterView(textView: textView, scrollView: scrollView)
        scrollView.addSubview(gutter)
        context.coordinator.gutter = gutter

        let minimap = MinimapOverlayView(textView: textView, scrollView: scrollView)
        minimap.setAccessibilityIdentifier("pitex.minimap")
        scrollView.addSubview(minimap)
        context.coordinator.minimap = minimap

        // Folding: engine collapses lines through the layout-manager delegate;
        // the chip overlay draws the "…" affordance at folded headers.
        let foldEngine = FoldEngine()
        foldEngine.isEnabled = foldingEnabled
        foldEngine.attach(to: textView)
        context.coordinator.foldEngine = foldEngine
        gutter.foldEngine = foldEngine

        let chips = FoldChipOverlayView(textView: textView, scrollView: scrollView, engine: foldEngine)
        scrollView.addSubview(chips)
        context.coordinator.chips = chips
        foldEngine.onChange = { [weak gutter, weak chips, weak minimap] in
            gutter?.needsDisplay = true
            chips?.needsDisplay = true
            minimap?.needsDisplay = true
        }

        // Bracket matching paints the translucent wash on caret movement.
        let bracketMatcher = BracketMatcher()
        bracketMatcher.attach(to: adapter)
        context.coordinator.bracketMatcher = bracketMatcher

        // Ghost-text completion: the translucent suggestion layer overlays
        // the scroll view like the gutter/minimap; Tab/Esc are intercepted
        // by a keyDown monitor while a ghost is visible.
        let ghost = GhostCompletionOverlayView(textView: textView, scrollView: scrollView)
        scrollView.addSubview(ghost)
        context.coordinator.ghost = ghost
        completion?.overlay = ghost
        context.coordinator.completion = completion
        context.coordinator.ghostMonitor = NSEvent.addLocalMonitorForEvents(
            matching: .keyDown
        ) { [weak coordinator = context.coordinator] event in
            let consumed = MainActor.assumeIsolated {
                coordinator?.handleGhostKey(event) ?? false
            }
            return consumed ? nil : event
        }

        // Cmd-click on a source location → forward SyncTeX, the reference
        // editor's Ctrl-click equivalent. A local event monitor is used
        // instead of an NSClickGestureRecognizer: recognizers lose clicks to
        // the text view's own mouseDown handling, while the monitor sees the
        // raw event first and can swallow it before selection/link behavior.
        context.coordinator.syncMonitor = NSEvent.addLocalMonitorForEvents(
            matching: .leftMouseDown
        ) { [weak coordinator = context.coordinator] event in
            let consumed = MainActor.assumeIsolated {
                coordinator?.handleSyncClick(event) ?? false
            }
            return consumed ? nil : event
        }
        context.coordinator.textView = textView

        context.coordinator.lastAppearanceKey = appearanceKey
        applyAppearance(to: scrollView)
        return scrollView
    }

    static func dismantleNSView(_ nsView: NSScrollView, coordinator: Coordinator) {
        if let monitor = coordinator.syncMonitor {
            NSEvent.removeMonitor(monitor)
        }
        if let monitor = coordinator.ghostMonitor {
            NSEvent.removeMonitor(monitor)
        }
        if coordinator.completion?.overlay === coordinator.ghost {
            coordinator.completion?.overlay = nil
        }
        coordinator.foldEngine?.detach()
        coordinator.bracketMatcher?.detach()
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        // EditorMacAdapter and DocumentSession are the only text owners. SwiftUI never mirrors text.
        // WorkspaceView keys this view by adapter identity, keeping the
        // document, overlays and their notification observers together.
        context.coordinator.onSyncRequest = onSyncRequest
        context.coordinator.foldEngine?.isEnabled = foldingEnabled
        context.coordinator.minimap?.isHidden = !minimapVisible
        // `completion` is a value input — if it arrived after makeNSView
        // ran (e.g. a body re-eval mid-open), bind it on the update pass
        // rather than leaving the overlay orphaned for the session.
        if let completion, context.coordinator.completion !== completion {
            context.coordinator.completion = completion
            completion.overlay = context.coordinator.ghost
        }
        // Re-applying fonts/colors and repainting every overlay on each
        // SwiftUI pass costs an O(document) minimap draw per keystroke; only
        // re-apply when an appearance input actually changed.
        guard context.coordinator.lastAppearanceKey != appearanceKey else { return }
        context.coordinator.lastAppearanceKey = appearanceKey
        applyAppearance(to: scrollView)
        context.coordinator.gutter?.needsDisplay = true
        context.coordinator.minimap?.needsDisplay = true
        context.coordinator.chips?.needsDisplay = true
    }

    /// The inputs applyAppearance reads: font settings plus the color
    /// revision bumped whenever any role color changes.
    private var appearanceKey: AppearanceKey {
        let appearance = AppearanceSettings.shared
        return AppearanceKey(
            colorRevision: appearance.colorRevision,
            fontFamily: appearance.fontFamily,
            fontSize: appearance.fontSize
        )
    }

    struct AppearanceKey: Equatable {
        var colorRevision: Int
        var fontFamily: String
        var fontSize: Double
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    final class Coordinator: NSObject {
        weak var minimap: MinimapOverlayView?
        weak var gutter: LineNumberGutterView?
        weak var chips: FoldChipOverlayView?
        weak var ghost: GhostCompletionOverlayView?
        weak var textView: NSTextView?
        var foldEngine: FoldEngine?
        var bracketMatcher: BracketMatcher?
        var syncMonitor: Any?
        var ghostMonitor: Any?
        var completion: GhostCompletionCoordinator?
        var onSyncRequest: ((Int, Int) -> Void)?
        var lastAppearanceKey: AppearanceKey?

        /// Tab/Esc while a ghost is showing — the Copilot-style accept and
        /// dismiss pair. A keyDown monitor sees the event before the text
        /// view: Tab must not insert a tab character and Esc must not fall
        /// through to the find bar while a suggestion is up.
        func handleGhostKey(_ event: NSEvent) -> Bool {
            guard let completion, completion.suggestion != nil,
                  let textView,
                  event.window === textView.window,
                  textView.window?.firstResponder === textView
            else { return false }
            // Only the bare key accepts/dismisses — Shift-Tab and chorded
            // Escapes keep their normal meaning. Caps Lock is not a chord.
            let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            guard modifiers.subtracting(.capsLock).isEmpty else { return false }
            switch event.keyCode {
            case 48: // kVK_Tab
                return completion.accept()
            case 53: // kVK_Escape
                completion.dismiss()
                return true
            default:
                return false
            }
        }

        /// Cmd+click inside this editor fires forward SyncTeX and reports the
        /// event as consumed so the caret and selection are left alone; every
        /// other click passes through to the text view untouched, keeping
        /// double/triple-click word and line selection intact.
        func handleSyncClick(_ event: NSEvent) -> Bool {
            guard event.modifierFlags.contains(.command),
                  let textView,
                  event.window === textView.window,
                  let onSyncRequest
            else { return false }
            let point = textView.convert(event.locationInWindow, from: nil)
            guard textView.bounds.contains(point),
                  let layoutManager = textView.layoutManager,
                  let textContainer = textView.textContainer
            else { return false }
            // The glyph under the point — characterIndexForInsertion rounds
            // to the nearest insertion point, which lands on the next line
            // for clicks in a line's lower half.
            let containerPoint = NSPoint(
                x: point.x - textView.textContainerOrigin.x,
                y: point.y - textView.textContainerOrigin.y
            )
            layoutManager.ensureLayout(for: textContainer)
            guard layoutManager.numberOfGlyphs > 0 else { return false }
            let glyph = layoutManager.glyphIndex(for: containerPoint, in: textContainer)
            guard glyph < layoutManager.numberOfGlyphs else { return false }
            let index = layoutManager.characterIndexForGlyph(at: glyph)
            let (line, column) = Self.lineAndColumn(for: index, in: textView.string as NSString)
            onSyncRequest(line, column)
            return true
        }

        /// 1-based (line, column) for a UTF-16 character index.
        static func lineAndColumn(for index: Int, in text: NSString) -> (Int, Int) {
            var line = 1
            var location = 0
            let bounded = min(index, text.length)
            while location < bounded {
                let next = text.range(of: "\n", range: NSRange(location: location, length: bounded - location))
                guard next.location != NSNotFound else { break }
                line += 1
                location = next.location + 1
            }
            return (line, max(bounded - location, 0))
        }
    }

    /// Applies the appearance theme to the editor chrome. Called from
    /// updateNSView so @Published appearance changes re-render automatically.
    private func applyAppearance(to scrollView: NSScrollView) {
        let appearance = AppearanceSettings.shared
        let textView = adapter.textView
        textView.font = appearance.editorFont
        textView.backgroundColor = appearance.color(for: .editorBackground)
        textView.insertionPointColor = appearance.color(for: .bodyText)
        // Do NOT set textView.textColor: the setter repaints the entire
        // text storage and would erase the highlighter's token colors on
        // every updateNSView pass. Typing attributes cover the pre-highlight
        // window; the highlighter owns all text colors after that.
        textView.typingAttributes = [
            .font: appearance.editorFont,
            .foregroundColor: appearance.color(for: .bodyText),
        ]
        scrollView.backgroundColor = appearance.color(for: .editorBackground)
    }
}

/// Line-number gutter overlaid on the scroll view's left edge, matching the
/// reference editor's numbered margin. Implemented as a plain subview (like
/// the minimap) rather than NSRulerView: ruler tiling shifts the clip view's
/// bounds.origin horizontally and repeated layouts accumulate that offset
/// until the document scrolls off-screen.
final class LineNumberGutterView: NSView {
    static let width: CGFloat = 56
    private weak var textView: NSTextView?
    private weak var scrollView: NSScrollView?
    /// Supplies foldable regions; triangles are drawn for their header lines.
    weak var foldEngine: FoldEngine?
    /// Triangle rects in view coordinates, rebuilt each draw for hit-testing.
    private var triangleRects: [(rect: NSRect, line: Int)] = []
    /// nonisolated(unsafe): removed once in deinit; tokens are only touched
    /// on the main thread while the view is alive.
    nonisolated(unsafe) private var observers: [NSObjectProtocol] = []

    /// Top-down coordinates matching the text view's document space —
    /// without this NSString.draw renders labels upward from their origin
    /// and every number lands one line too high.
    override var isFlipped: Bool { true }

    init(textView: NSTextView, scrollView: NSScrollView) {
        self.textView = textView
        self.scrollView = scrollView
        super.init(frame: .zero)
        clipsToBounds = true
        autoresizingMask = [.maxXMargin, .height]
        observers.append(NotificationCenter.default.addObserver(
            forName: NSText.didChangeNotification,
            object: textView,
            queue: .main
        ) { [weak self] _ in
            self?.lineStartsDirty = true
            self?.needsDisplay = true
        })
        scrollView.contentView.postsBoundsChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.boundsDidChangeNotification,
            object: scrollView.contentView,
            queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        scrollView.contentView.postsFrameChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.frameDidChangeNotification,
            object: scrollView.contentView,
            queue: .main
        ) { [weak self] _ in self?.reposition() })
        reposition()
    }

    required init?(coder: NSCoder) { nil }

    deinit {
        observers.forEach { NotificationCenter.default.removeObserver($0) }
    }

    /// Line-start offsets (NSString lineRange semantics), rebuilt lazily on
    /// the first draw after a text change — counting up to the first visible
    /// line per draw was O(topLine) per scroll frame.
    private var cachedLineStarts: [Int] = []
    private var lineStartsDirty = true

    private func lineStarts(for text: NSString) -> [Int] {
        if lineStartsDirty {
            var starts = [0]
            var location = 0
            while location < text.length {
                let next = NSMaxRange(text.lineRange(for: NSRange(location: location, length: 0)))
                guard next > location else { break }
                starts.append(next)
                location = next
            }
            cachedLineStarts = starts
            lineStartsDirty = false
        }
        return cachedLineStarts
    }

    private func reposition() {
        guard let scrollView else { return }
        frame = scrollView.contentView.frame
        frame.size.width = Self.width
        needsDisplay = true
    }

    /// Only fold triangles are interactive; other clicks fall through to the
    /// text view underneath the inset area.
    override func hitTest(_ point: NSPoint) -> NSView? {
        let local = convert(point, from: superview)
        guard bounds.contains(local) else { return nil }
        for triangle in triangleRects where triangle.rect.contains(local) { return self }
        return nil
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        for triangle in triangleRects where triangle.rect.contains(point) {
            foldEngine?.toggle(atLine: triangle.line)
            return
        }
    }

    override func draw(_ dirtyRect: NSRect) {
        guard let textView, let layoutManager = textView.layoutManager,
              let container = textView.textContainer else { return }

        let appearance = AppearanceSettings.shared
        appearance.color(for: .gutterBackground).setFill()
        bounds.fill()

        // glyphRange(forBoundingRect:in:) takes container coordinates; the
        // visible rect is in textView coordinates, so drop the inset.
        let visibleRect = textView.visibleRect
            .offsetBy(dx: -textView.textContainerInset.width,
                      dy: -textView.textContainerInset.height)
        let glyphRange = layoutManager.glyphRange(forBoundingRect: visibleRect, in: container)
        let charRange = layoutManager.characterRange(forGlyphRange: glyphRange, actualGlyphRange: nil)
        let text = textView.string as NSString

        // charRange.location can fall mid-line; anchor the walk at the
        // start of the line containing it so numbers and rows stay paired.
        let firstLine = text.lineRange(for: NSRange(location: charRange.location, length: 0))
        // The first visible line's number is its index in the cached
        // line-start table — a binary search instead of a rescan from 0.
        let starts = lineStarts(for: text)
        var lo = 0, hi = starts.count - 1
        while lo < hi {
            let mid = (lo + hi + 1) / 2
            if starts[mid] <= firstLine.location { lo = mid } else { hi = mid - 1 }
        }
        var lineNumber = lo + 1
        let length = text.length

        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular),
            .foregroundColor: appearance.color(for: .lineNumbers)
        ]

        triangleRects = []
        var cursor = firstLine.location
        var lastLineStart = -1
        while cursor <= length {
            let lineRange = text.lineRange(for: NSRange(location: cursor, length: 0))
            if lineRange.location == lastLineStart { break }
            lastLineStart = lineRange.location
            // An empty range at EOF is a real final line only when the text
            // ends with a newline; otherwise it is a phantom — stop.
            if lineRange.length == 0
                && !(cursor > 0 && text.character(at: cursor - 1) == 0x0A) {
                break
            }
            let glyph = layoutManager.glyphRange(forCharacterRange: lineRange, actualCharacterRange: nil)
            // A wrapped source line gets its number beside the first visual
            // row, not at the vertical centre of all its continuation rows.
            let lineRect = glyph.length > 0
                ? layoutManager.lineFragmentRect(forGlyphAt: glyph.location, effectiveRange: nil)
                : layoutManager.extraLineFragmentRect
            // The gutter is a scrollView subview; convert the document-space
            // line position into overlay coordinates.
            let yPos = convert(
                NSPoint(x: 0, y: lineRect.minY + textView.textContainerInset.height),
                from: textView
            ).y
            if yPos > bounds.maxY { break }
            let lineIndex = lineNumber - 1
            let hidden = foldEngine?.isLineHidden(lineIndex) ?? false
            if yPos + lineRect.height >= bounds.minY, !hidden {
                // Vertically centre the number on the line fragment so the
                // first row's 1 sits level with its text (the reference
                // editor's gutter alignment).
                let label = "\(lineNumber)" as NSString
                let size = label.size(withAttributes: attrs)
                let centredY = yPos + (lineRect.height - size.height) / 2
                label.draw(
                    at: NSPoint(x: Self.width - size.width - 10, y: centredY),
                    withAttributes: attrs
                )
                if let region = foldEngine?.region(atLine: lineIndex) {
                    drawFoldTriangle(
                        centredOn: yPos + lineRect.height / 2,
                        folded: region.folded,
                        line: lineIndex
                    )
                }
            }
            lineNumber += 1
            let next = NSMaxRange(lineRange)
            if next <= cursor { break }
            cursor = next
        }
    }

    /// Fold triangle at the gutter's left edge: ▾ while expanded, ▸ once
    /// folded, in the theme's fold accent colour.
    private func drawFoldTriangle(centredOn y: CGFloat, folded: Bool, line: Int) {
        let size: CGFloat = 7
        let x: CGFloat = 9
        let rect = NSRect(
            x: x - size / 2 - 3,
            y: y - size / 2 - 3,
            width: size + 6,
            height: size + 6
        )
        triangleRects.append((rect, line))
        let path = NSBezierPath()
        if folded {
            path.move(to: NSPoint(x: x - size / 2, y: y - size / 2))
            path.line(to: NSPoint(x: x - size / 2, y: y + size / 2))
            path.line(to: NSPoint(x: x + size / 2, y: y))
        } else {
            // Flipped coordinates: the wide edge sits above the apex — ▾.
            path.move(to: NSPoint(x: x - size / 2, y: y - size / 2))
            path.line(to: NSPoint(x: x + size / 2, y: y - size / 2))
            path.line(to: NSPoint(x: x, y: y + size / 2))
        }
        path.close()
        AppearanceSettings.shared.color(for: .foldAccent).setFill()
        path.fill()
    }
}

/// A thumbnail of the editor's actual laid-out rows. Wrapping, folding, the
/// viewport marker and click navigation all use the same document coordinates.
final class MinimapOverlayView: NSView {
    private weak var textView: NSTextView?
    private weak var scrollView: NSScrollView?
    nonisolated(unsafe) private var observers: [NSObjectProtocol] = []
    private let padding: CGFloat = 4

    override var isFlipped: Bool { true }

    init(textView: NSTextView, scrollView: NSScrollView) {
        self.textView = textView
        self.scrollView = scrollView
        super.init(frame: .zero)
        wantsLayer = true
        clipsToBounds = true
        autoresizingMask = [.minXMargin, .height]
        // Attribute edits include highlighting and theme changes; use the
        // editor's colors instead of independently tokenizing the document.
        observers.append(NotificationCenter.default.addObserver(
            forName: NSTextStorage.didProcessEditingNotification,
            object: textView.textStorage, queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        textView.postsFrameChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.frameDidChangeNotification,
            object: textView, queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        scrollView.contentView.postsBoundsChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.boundsDidChangeNotification,
            object: scrollView.contentView, queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        scrollView.contentView.postsFrameChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.frameDidChangeNotification,
            object: scrollView.contentView, queue: .main
        ) { [weak self] _ in self?.reposition() })
        reposition()
    }

    required init?(coder: NSCoder) { nil }

    deinit { observers.forEach { NotificationCenter.default.removeObserver($0) } }

    private func reposition() {
        guard let scrollView else { return }
        let viewport = scrollView.contentView.frame.insetBy(dx: 8, dy: 8)
        let width = min(72, max(viewport.width, 0))
        frame = NSRect(x: viewport.maxX - width, y: viewport.minY,
                       width: width, height: max(viewport.height, 0))
        needsDisplay = true
    }

    private func verticalScale() -> CGFloat {
        guard let textView, let layout = textView.layoutManager,
              let container = textView.textContainer else { return 0 }
        layout.ensureLayout(for: container)
        return min(max(bounds.height - 2 * padding, 0) / max(textView.bounds.height, 1), 0.28)
    }

    override func draw(_ dirtyRect: NSRect) {
        // Hidden minimaps still receive needsDisplay from text/scroll
        // notifications; skip the O(document) row walk entirely.
        guard !isHidden,
              let textView, let layout = textView.layoutManager,
              let container = textView.textContainer, let storage = textView.textStorage else { return }
        NSColor(white: 0.5, alpha: 0.16).setFill()
        NSBezierPath(roundedRect: bounds, xRadius: 6, yRadius: 6).fill()
        let scale = verticalScale()
        guard scale > 0 else { return }
        let horizontalScale = max(bounds.width - 10, 0) / max(container.size.width, 1)
        let text = storage.string as NSString
        let glyphs = NSRange(location: 0, length: layout.numberOfGlyphs)
        layout.enumerateLineFragments(forGlyphRange: glyphs) { _, used, _, range, _ in
            guard used.height > 0 else { return } // collapsed rows
            let characters = layout.characterRange(forGlyphRange: range, actualGlyphRange: nil)
            let first = text.rangeOfCharacter(from: .whitespacesAndNewlines.inverted, range: characters)
            guard first.location != NSNotFound else { return }
            let last = text.rangeOfCharacter(from: .whitespacesAndNewlines.inverted, options: .backwards, range: characters)
            let content = NSRange(location: first.location, length: NSMaxRange(last) - first.location)
            let contentGlyphs = layout.glyphRange(forCharacterRange: content, actualCharacterRange: nil)
            let rect = layout.boundingRect(forGlyphRange: contentGlyphs, in: container)
            let color = storage.attribute(.foregroundColor, at: first.location, effectiveRange: nil) as? NSColor
                ?? AppearanceSettings.shared.color(for: .bodyText)
            color.withAlphaComponent(0.85).setFill()
            NSRect(x: 5 + rect.minX * horizontalScale,
                   y: self.padding + (rect.minY + textView.textContainerOrigin.y) * scale,
                   width: max(rect.width * horizontalScale, 1),
                   height: max(rect.height * scale * 0.62, 0.5)).fill()
        }

        let visible = textView.visibleRect
        let viewport = NSRect(x: 1, y: padding + visible.minY * scale,
                              width: max(bounds.width - 2, 0), height: visible.height * scale)
        NSColor.white.withAlphaComponent(0.14).setFill()
        NSBezierPath(roundedRect: viewport, xRadius: 4, yRadius: 4).fill()
    }

    override func mouseDown(with event: NSEvent) { scroll(to: event) }
    override func mouseDragged(with event: NSEvent) { scroll(to: event) }

    private func scroll(to event: NSEvent) {
        guard let textView, let scrollView else { return }
        let scale = verticalScale()
        guard scale > 0 else { return }
        let local = convert(event.locationInWindow, from: nil)
        let visible = scrollView.contentView.bounds
        let target = (local.y - padding) / scale - visible.height / 2
        let maximum = max(textView.bounds.height - visible.height, 0)
        textView.scroll(NSPoint(x: visible.minX, y: min(max(target, 0), maximum)))
        scrollView.reflectScrolledClipView(scrollView.contentView)
    }
}

/// Ghost-text layer for inline AI completion — a translucent suggestion
/// drawn at the caret glyph, Copilot-style. A plain scroll-view subview
/// like the gutter/minimap; it never intercepts clicks (`hitTest` → nil).
final class GhostCompletionOverlayView: NSView {
    private weak var textView: NSTextView?
    private weak var scrollView: NSScrollView?
    /// nonisolated(unsafe): removed once in deinit; tokens are only touched
    /// on the main thread while the view is alive.
    nonisolated(unsafe) private var observers: [NSObjectProtocol] = []
    /// The suggestion and the UTF-16 caret index it was generated for —
    /// drawing anchors here so a scroll repaint never drifts to a moved
    /// caret (any caret move clears the suggestion anyway).
    private(set) var suggestion: String?
    private(set) var anchor = 0

    /// Top-down coordinates matching the text view's document space.
    override var isFlipped: Bool { true }

    init(textView: NSTextView, scrollView: NSScrollView) {
        self.textView = textView
        self.scrollView = scrollView
        super.init(frame: .zero)
        autoresizingMask = [.width, .height]
        // Repaint on edits, scrolls and resizes — the anchor is in document
        // coordinates, so its viewport position must be recomputed.
        observers.append(NotificationCenter.default.addObserver(
            forName: NSText.didChangeNotification, object: textView, queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        scrollView.contentView.postsBoundsChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.boundsDidChangeNotification,
            object: scrollView.contentView, queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        scrollView.contentView.postsFrameChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.frameDidChangeNotification,
            object: scrollView.contentView, queue: .main
        ) { [weak self] _ in self?.reposition() })
        reposition()
    }

    required init?(coder: NSCoder) { nil }

    deinit { observers.forEach { NotificationCenter.default.removeObserver($0) } }

    private func reposition() {
        guard let scrollView else { return }
        frame = scrollView.contentView.frame
        needsDisplay = true
    }

    func show(_ suggestion: String, anchor: Int) {
        self.suggestion = suggestion
        self.anchor = anchor
        needsDisplay = true
    }

    func clear() {
        guard suggestion != nil else { return }
        suggestion = nil
        needsDisplay = true
    }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override func draw(_ dirtyRect: NSRect) {
        guard let suggestion, !suggestion.isEmpty,
              let textView,
              let layout = textView.layoutManager,
              let container = textView.textContainer
        else { return }
        layout.ensureLayout(for: container)
        let text = textView.string as NSString
        let charIndex = min(max(anchor, 0), text.length)
        let glyph = layout.glyphIndexForCharacter(at: charIndex)

        // The caret line's glyph rect — at EOF there is no glyph, so fall
        // back to the extra line fragment (trailing newline) or the last
        // glyph's trailing edge, the same way the caret itself is drawn.
        var origin = NSPoint.zero
        var lineHeight = textView.font?.pointSize ?? 12
        if glyph < layout.numberOfGlyphs {
            let rect = layout.boundingRect(
                forGlyphRange: NSRange(location: glyph, length: 1), in: container)
            origin = rect.origin
            lineHeight = max(rect.height, 1)
        } else if layout.extraLineFragmentRect.height > 0 {
            origin = layout.extraLineFragmentRect.origin
            lineHeight = layout.extraLineFragmentRect.height
        } else if layout.numberOfGlyphs > 0 {
            let rect = layout.boundingRect(
                forGlyphRange: NSRange(location: layout.numberOfGlyphs - 1, length: 1),
                in: container)
            origin = NSPoint(x: rect.maxX, y: rect.minY)
            lineHeight = rect.height
        }
        // Container → text-view coordinates, then into this overlay.
        origin.x += textView.textContainerOrigin.x
        origin.y += textView.textContainerOrigin.y
        let local = convert(origin, from: textView)

        let font = textView.font ?? NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: NSColor.secondaryLabelColor.withAlphaComponent(0.38),
        ]
        let nsSuggestion = suggestion as NSString
        if suggestion.contains("\n") {
            // Multi-line replies render as a small translucent block below
            // the caret line — the simpler correct treatment.
            let block = nsSuggestion.boundingRect(
                with: NSSize(
                    width: CGFloat.greatestFiniteMagnitude,
                    height: CGFloat.greatestFiniteMagnitude),
                options: [.usesLineFragmentOrigin, .usesFontLeading],
                attributes: attrs)
            let x = max(local.x, 4)
            let y = local.y + lineHeight + 2
            let frame = NSRect(
                x: x - 4, y: y - 2,
                width: ceil(block.width) + 8, height: ceil(block.height) + 4)
            NSColor.secondaryLabelColor.withAlphaComponent(0.12).setFill()
            NSBezierPath(roundedRect: frame, xRadius: 5, yRadius: 5).fill()
            nsSuggestion.draw(at: NSPoint(x: x, y: y), withAttributes: attrs)
        } else {
            // Single-line: draw to the right of the caret, vertically
            // centred on the line fragment like the gutter's numbers.
            let size = nsSuggestion.size(withAttributes: attrs)
            nsSuggestion.draw(
                at: NSPoint(x: local.x, y: local.y + max((lineHeight - size.height) / 2, 0)),
                withAttributes: attrs)
        }
    }
}

/// Copilot-style inline LaTeX completion. A dedicated `pi --mode rpc`
/// subprocess — separate from the chat `AgentCoordinator`, so completion
/// prompts never touch the transcript or contend with a running chat
/// request — answers ~50/20-line continuation prompts; the reply renders
/// as translucent ghost text at the caret. Tab accepts, Esc/typing/caret
/// moves dismiss. The whole feature is a silent no-op while the setting is
/// off, the document is not `.tex`, or pi cannot be resolved.
@MainActor
final class GhostCompletionCoordinator {
    /// Workspace state read at fire time so the setting toggle and document
    /// switches apply immediately without re-attaching.
    struct Context {
        var projectRoot: URL?
        var fileName = "document.tex"
        var isTeX = false
        var enabled = false
    }

    var contextProvider: () -> Context = { Context() }
    /// The overlay the suggestion renders into — bound by the editor
    /// container when the overlay mounts, released on dismantle.
    weak var overlay: GhostCompletionOverlayView? {
        didSet {
            if let suggestion { overlay?.show(suggestion, anchor: suggestionAnchor) }
        }
    }
    /// The visible ghost — the key handler tests this before consuming
    /// Tab/Esc.
    private(set) var suggestion: String?
    private var suggestionAnchor = 0
    private weak var textView: NSTextView?
    /// nonisolated(unsafe): removed in `shutdown`/`attach`; tokens are only
    /// touched on the main actor while the coordinator is alive.
    nonisolated(unsafe) private var observers: [NSObjectProtocol] = []
    private var debounceTask: Task<Void, Never>?
    private var eventTask: Task<Void, Never>?
    private var prepareTask: Task<Void, Never>?
    private var process: PiAgentProcess?
    /// Spawn/resolution failures latch here until the next `attach`, so a
    /// missing pi costs one probe per document — not one per idle pause.
    private var piUnavailable = false
    /// The prompt staged while the subprocess is still starting.
    private var stagedPrompt: String?
    /// The prompt id whose reply may produce a suggestion; a new request
    /// supersedes it so stale replies are dropped.
    private var pendingRequestID: String?
    /// True while a request that may answer is in flight — armed at send
    /// time rather than on the prompt `response`, whose position relative
    /// to `message_end`/`agent_end` is not guaranteed by the wire order.
    private var armed = false
    /// Set by the first `agent_start` after arming — it marks this
    /// request's own run. Leftover events from a superseded, aborted run
    /// can land after the prompt was sent but always before its run
    /// starts, so `message_end` is only honoured while this is set.
    private var runStarted = false

    /// Rebinds the text/selection observers — the adapter is rebuilt per
    /// document like the macOS environment, mirroring
    /// `highlighter.attach(to:fileExtension:)`.
    func attach(to adapter: EditorMacAdapter) {
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        observers.removeAll()
        let textView = adapter.textView
        self.textView = textView
        observers.append(NotificationCenter.default.addObserver(
            forName: NSText.didChangeNotification, object: textView, queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.editorActivity() }
        })
        observers.append(NotificationCenter.default.addObserver(
            forName: NSTextView.didChangeSelectionNotification, object: textView, queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.editorActivity() }
        })
        debounceTask?.cancel()
        supersede()
        dismiss()
        // A fresh document gets one more chance to find pi.
        piUnavailable = false
    }

    /// Edits and caret moves both count as "the user kept typing": drop the
    /// visible ghost, supersede the in-flight request and re-arm the 600ms
    /// debounce. O(1) here — context gathering waits for the timer.
    private func editorActivity() {
        dismiss()
        debounceTask?.cancel()
        supersede()
        debounceTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(600))
            guard !Task.isCancelled else { return }
            self?.fire()
        }
    }

    /// Tag-and-abort: a new request id makes late replies stale, and the
    /// abort stops the old run early instead of burning provider tokens.
    private func supersede() {
        stagedPrompt = nil
        if pendingRequestID != nil || armed {
            try? process?.send(.abort)
        }
        pendingRequestID = nil
        armed = false
        runStarted = false
    }

    /// The debounced half: gates re-checked at fire time (the setting may
    /// have flipped mid-burst), then the ~50/20-line window is staged for
    /// the subprocess.
    private func fire() {
        let context = contextProvider()
        guard context.enabled,
              context.isTeX,
              !piUnavailable,
              let textView,
              textView.isEditable,
              !textView.hasMarkedText(),
              let root = context.projectRoot
        else { return }
        let selection = textView.selectedRange()
        // A collapsed caret only — a dragged selection never completes.
        guard selection.length == 0 else { return }
        let text = textView.string as NSString
        let caret = min(selection.location, text.length)
        stagedPrompt = Self.prompt(fileName: context.fileName, text: text, caret: caret)
        if let process, process.isRunning {
            sendStaged()
        } else {
            startProcess(root: root)
        }
    }

    /// `prepareAgent`'s lazy counterpart: discovery runs off the main
    /// actor; resolution/spawn failures latch `piUnavailable` silently.
    private func startProcess(root: URL) {
        guard prepareTask == nil else { return }
        prepareTask = Task { [weak self] in
            defer { self?.prepareTask = nil }
            let tools = await PiToolchain.discover()
            guard !Task.isCancelled, let self else { return }
            guard let executable = PiExecutableLocator.resolve(environment: tools.environment)
            else {
                piUnavailable = true
                return
            }
            do {
                let launch = try tools.launch(
                    executable, arguments: ["--mode", "rpc", "--no-session"])
                let agent = PiAgentProcess(
                    executableURL: launch.executable,
                    workingDirectory: root,
                    arguments: launch.arguments,
                    environment: AgentCoordinator.childEnvironment(
                        executable: launch.executable, environment: tools.environment)
                )
                try agent.start { [weak self, weak agent] in
                    Task { @MainActor in self?.processDidExit(agent) }
                }
                process = agent
                eventTask?.cancel()
                eventTask = Task { [weak self] in
                    for await event in agent.events {
                        self?.handle(event)
                    }
                }
                sendStaged()
            } catch {
                piUnavailable = true
            }
        }
    }

    /// Identity-guarded like `AgentCoordinator.processDidExit`: a queued
    /// onExit from the old process must not clear a replacement that
    /// already spawned between the exit and this MainActor hop.
    private func processDidExit(_ exited: PiAgentProcess?) {
        guard exited == nil || process === exited else { return }
        process = nil
        armed = false
        runStarted = false
        pendingRequestID = nil
    }

    /// abort + new_session + tagged prompt: the abort ends a superseded
    /// run, `new_session` keeps every request's context cheap so earlier
    /// completion prompts never accumulate, and the id lets `handle` drop
    /// replies to superseded asks.
    private func sendStaged() {
        guard let prompt = stagedPrompt, let process, process.isRunning else { return }
        stagedPrompt = nil
        let id = UUID().uuidString
        pendingRequestID = id
        runStarted = false
        do {
            try process.send(.abort)
            try process.send(.newSession)
            try process.send(.prompt(message: prompt, streamingBehavior: nil), id: id)
            armed = true
        } catch {
            pendingRequestID = nil
        }
    }

    /// The `response`/`agent_start`/`message_end`/`agent_end` state
    /// machine — everything else (abort/new_session acks, thinking
    /// deltas, tool chatter) is ignored. Arming happens at send time and
    /// `runStarted` marks the request's own run, so the machine is
    /// correct whether the prompt `response` is a dispatch-time ack or
    /// the run's final result, and stale events from a just-aborted run
    /// can never produce a suggestion.
    private func handle(_ event: PiRPCEvent) {
        switch event.type {
        case "response":
            guard event.responseCommand == "prompt",
                  event.string("id") == pendingRequestID
            else { return }
            pendingRequestID = nil
            if !event.responseSucceeded {
                armed = false
                runStarted = false
            }
        case "agent_start":
            if armed { runStarted = true }
        case "message_end":
            guard armed, runStarted,
                  let message = event.nested("message"),
                  message["role"] as? String == "assistant",
                  let content = message["content"] as? [[String: Any]]
            else { return }
            let text = content
                .filter { ($0["type"] as? String) == "text" }
                .compactMap { $0["text"] as? String }
                .joined()
            applySuggestion(text)
        case "agent_end":
            if runStarted {
                runStarted = false
                armed = false
            }
        default:
            break
        }
    }

    /// The model's reply becomes the ghost only after cleanup — a stray
    /// code-fence wrapper is dropped, one trailing newline is stripped, and
    /// blank replies show nothing.
    private func applySuggestion(_ raw: String) {
        var suggestion = raw
        if suggestion.hasPrefix("```") {
            var lines = suggestion.components(separatedBy: "\n")
            lines.removeFirst()
            if lines.last?.trimmingCharacters(in: .whitespaces) == "```" {
                lines.removeLast()
            }
            suggestion = lines.joined(separator: "\n")
        }
        if suggestion.hasSuffix("\n") { suggestion.removeLast() }
        guard !suggestion.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        else { return }
        self.suggestion = suggestion
        suggestionAnchor = textView?.selectedRange().location ?? 0
        overlay?.show(suggestion, anchor: suggestionAnchor)
    }

    /// Tab → insert through the normal edit path: `insertText` posts
    /// `didChange`, so the insert is undo-safe and the resulting
    /// text-change notification reschedules the next completion.
    @discardableResult
    func accept() -> Bool {
        guard let suggestion, let textView else { return false }
        dismiss()
        textView.insertText(suggestion, replacementRange: textView.selectedRange())
        return true
    }

    /// Esc/typing/caret move/setting-off — hide the ghost only; superseding
    /// the in-flight request is `editorActivity`'s job.
    func dismiss() {
        suggestion = nil
        overlay?.clear()
    }

    /// Workspace close — terminate the subprocess and drop every hook.
    func shutdown() {
        debounceTask?.cancel()
        prepareTask?.cancel()
        eventTask?.cancel()
        process?.terminate()
        process = nil
        stagedPrompt = nil
        pendingRequestID = nil
        armed = false
        runStarted = false
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        observers.removeAll()
        dismiss()
    }

    /// `~50 lines before / ~20 lines after, char-bounded` — the completion
    /// envelope. NSString ranges keep indexing in UTF-16 like the caret;
    /// the `<CURSOR>` marker sits exactly where the insert happens.
    static func prompt(fileName: String, text: NSString, caret: Int) -> String {
        let caret = min(caret, text.length)
        // `start` is the answer; `probe` walks one newline further up per
        // iteration — searching from `start` again would find the same
        // newline every time.
        var start = caret
        var probe = caret
        var lines = 0
        while probe > 0, lines < 50 {
            let previous = text.range(
                of: "\n", options: .backwards,
                range: NSRange(location: 0, length: probe))
            guard previous.location != NSNotFound else { start = 0; break }
            let candidate = previous.location + 1
            if caret - candidate > 6_000 { break }
            start = candidate
            probe = previous.location
            lines += 1
        }
        var end = caret
        var forward = 0
        while end < text.length, forward < 20 {
            let next = text.range(
                of: "\n", range: NSRange(location: end, length: text.length - end))
            guard next.location != NSNotFound else { end = text.length; break }
            let candidate = next.location + 1
            if candidate - caret > 2_000 { break }
            end = candidate
            forward += 1
        }
        // Hard cap: a giant line still leaves the window ~6000/~2000
        // chars wide rather than exceeding the line-granular bound.
        start = max(start, caret - 6_000)
        end = min(end, caret + 2_000)
        let before = text.substring(with: NSRange(location: start, length: caret - start))
        let after = text.substring(with: NSRange(location: caret, length: end - caret))
        return """
            You are the inline autocompletion engine of the Pitex LaTeX editor. \
            Reply with ONLY the exact text to insert at <CURSOR>: no markdown, \
            no code fences, no explanation. Prefer completing the current \
            command, environment, or line in at most three lines. Reply with \
            nothing when there is no useful continuation.

            File: \(fileName)
            ---
            \(before)<CURSOR>\(after)
            """
    }
}
