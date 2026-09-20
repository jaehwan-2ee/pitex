import AppKit
import EditorMacAdapter
import LanguageCore
import SwiftUI

struct EditorContainerView: NSViewRepresentable {
    let adapter: EditorMacAdapter
    var minimapVisible = true
    var foldingEnabled = true
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
        weak var textView: NSTextView?
        var foldEngine: FoldEngine?
        var bracketMatcher: BracketMatcher?
        var syncMonitor: Any?
        var onSyncRequest: ((Int, Int) -> Void)?
        var lastAppearanceKey: AppearanceKey?

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
