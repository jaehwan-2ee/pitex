import AppKit
import EditorMacAdapter
import LanguageCore

/// One foldable region: an environment (\begin{X} … \end{X}) or a section
/// command (\section … up to the next same-or-higher-level section). Line
/// indexes are 0-based; `hiddenLines` are the lines collapsed when folded
/// (everything after the header line through the closing line inclusive).
struct FoldRegion {
    let headerLine: Int
    let hiddenLineRange: ClosedRange<Int>
    /// Stable-ish identity carried across edits: env/section name + the
    /// header line's trimmed content, so reopening the same header restores
    /// the fold even when offsets shift.
    let signature: String
    var folded = false
}

/// Computes fold regions and collapses their lines by driving the layout
/// manager: a delegate method zeroes hidden lines' fragment heights and the
/// glyphs are marked not-shown so nothing renders in the gap. Mirrors the
/// reference editor's fold triangle → chip behaviour.
@MainActor
final class FoldEngine: NSObject, NSLayoutManagerDelegate {
    private weak var textView: NSTextView?
    private(set) var regions: [FoldRegion] = []
    private var lineStarts: [Int] = []
    /// Line indexes whose fragments collapse to zero height.
    private var hiddenLines = IndexSet()
    /// Snapshot read by the nonisolated layout-manager delegate. Layout for
    /// this text view only ever runs on the main thread, so the unsafe
    /// storage is never contended — it exists purely to satisfy the
    /// delegate's isolation requirement without unsafe-sendable boxing.
    private struct LayoutState: Sendable {
        var lineStarts: [Int] = []
        var hiddenLines = IndexSet()
    }
    nonisolated(unsafe) private var layoutState = LayoutState()
    /// Called whenever regions or fold state change so chrome (gutter
    /// triangles, fold chips) can redraw.
    var onChange: (@MainActor () -> Void)?
    /// When false all regions are reported unfolded and no lines hide.
    var isEnabled = true {
        didSet {
            guard isEnabled != oldValue else { return }
            if isEnabled {
                recompute()
            } else {
                for index in regions.indices { regions[index].folded = false }
                applyFolds()
                onChange?()
            }
        }
    }
    nonisolated(unsafe) private var observer: NSObjectProtocol?
    /// Recompute after a typing pause, sharing the highlighter cadence.
    private var recomputeTask: Task<Void, Never>?

    func attach(to textView: NSTextView) {
        self.textView = textView
        textView.layoutManager?.delegate = self
        observer = NotificationCenter.default.addObserver(
            forName: NSText.didChangeNotification,
            object: textView,
            queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self, self.isEnabled else { return }
                self.recomputeTask?.cancel()
                self.recomputeTask = Task { @MainActor [weak self] in
                    try? await Task.sleep(for: .milliseconds(120))
                    guard !Task.isCancelled else { return }
                    self?.recompute()
                }
            }
        }
        recompute()
    }

    func detach() {
        recomputeTask?.cancel()
        recomputeTask = nil
        if let observer { NotificationCenter.default.removeObserver(observer) }
        observer = nil
        // Unhide everything before relinquishing the delegate so no stale
        // zero-height fragments remain.
        if let layoutManager = textView?.layoutManager {
            clearHiddenGlyphs(layoutManager: layoutManager)
        }
        hiddenLines = []
        layoutState = LayoutState()
        textView?.layoutManager?.delegate = nil
        textView = nil
    }

    // MARK: - Region discovery

    /// Re-scans the document for foldable regions. Folded state is carried
    /// over by signature so unrelated edits do not reopen folded regions.
    func recompute() {
        guard isEnabled, let textView else { return }
        let text = textView.string as NSString
        let analysis = textView.textStorage.map { EditorAnalysis.shared(for: $0) }
        lineStarts = analysis?.lineStarts ?? Self.computeLineStarts(text)
        let previous = Set(regions.filter(\.folded).map(\.signature))
        regions = Self.findRegions(text: text, lineStarts: lineStarts, tokens: analysis?.tokens(for: .latex))
        for index in regions.indices {
            if previous.contains(regions[index].signature) {
                regions[index].folded = true
            }
        }
        applyFolds()
        onChange?()
    }

    static func computeLineStarts(_ text: NSString) -> [Int] {
        var starts = [0]
        for index in 0..<text.length where text.character(at: index) == 0x0A {
            starts.append(index + 1)
        }
        return starts
    }

    /// Token-level scan: pairs \begin{X}/\end{X} by name and folds section
    /// commands through the line before the next same-level-or-higher one.
    static func findRegions(text: NSString, lineStarts: [Int], tokens suppliedTokens: [LanguageToken]? = nil) -> [FoldRegion] {
        func lineIndex(forChar location: Int) -> Int {
            var lo = 0, hi = lineStarts.count - 1
            while lo < hi {
                let mid = (lo + hi + 1) / 2
                if lineStarts[mid] <= location { lo = mid } else { hi = mid - 1 }
            }
            return lo
        }

        var source = text as String
        source.makeContiguousUTF8()
        let tokens = suppliedTokens ?? DeterministicTeXLexer.tokenize(source, dialect: .latex)

        let sectionLevels: [String: Int] = [
            "part": 0, "chapter": 1, "section": 2,
            "subsection": 3, "subsubsection": 4, "paragraph": 5,
        ]

        // Map only the UTF-8 offsets we actually need (begin/end/section
        // tokens) in a single scalar walk — the old per-token
        // samePosition(in:) conversion was O(offset) each, quadratic overall.
        var wanted = Set<Int>()
        for token in tokens {
            guard case let .controlSequence(name) = token.kind,
                  name == "begin" || name == "end" || sectionLevels[name] != nil
            else { continue }
            wanted.insert(token.range.utf8Offset)
        }
        var utf16ByUTF8: [Int: Int] = [:]
        utf16ByUTF8.reserveCapacity(wanted.count)
        let wantedSorted = wanted.sorted()
        var next = 0
        var utf8Offset = 0
        var utf16Offset = 0
        for scalar in source.unicodeScalars {
            while next < wantedSorted.count && wantedSorted[next] <= utf8Offset {
                if wantedSorted[next] == utf8Offset { utf16ByUTF8[utf8Offset] = utf16Offset }
                next += 1
            }
            utf8Offset += scalar.utf8.count
            utf16Offset += scalar.utf16.count
        }
        while next < wantedSorted.count && wantedSorted[next] <= utf8Offset {
            if wantedSorted[next] == utf8Offset { utf16ByUTF8[utf8Offset] = utf16Offset }
            next += 1
        }
        func utf16Index(_ utf8Offset: Int) -> Int {
            utf16ByUTF8[utf8Offset] ?? -1
        }

        var regions: [FoldRegion] = []
        // Environment matching: stack of (name, headerLine, signature).
        var envStack: [(name: String, headerLine: Int, signature: String)] = []
        var i = 0
        var openSections: [(level: Int, headerLine: Int, signature: String)] = []

        func closeSections(atOrAbove level: Int, endLine: Int) {
            while let last = openSections.last, last.level >= level {
                if last.headerLine < endLine - 1 {
                    regions.append(FoldRegion(
                        headerLine: last.headerLine,
                        hiddenLineRange: (last.headerLine + 1)...(endLine - 1),
                        signature: last.signature
                    ))
                }
                openSections.removeLast()
            }
        }

        func signature(command: String, headerLine: Int) -> String {
            let range = text.lineRange(for: NSRange(location: lineStarts[headerLine], length: 0))
            let content = text.substring(with: range).trimmingCharacters(in: .whitespacesAndNewlines)
            return "\(command)|\(content)"
        }

        while i < tokens.count {
            let token = tokens[i]
            guard case let .controlSequence(name) = token.kind else { i += 1; continue }
            if name == "begin" || name == "end" {
                // The environment name follows as the next environmentName token.
                var nameValue: String?
                var j = i + 1
                lookahead: while j < tokens.count, j < i + 4 {
                    switch tokens[j].kind {
                    case let .environmentName(env): nameValue = env; break lookahead
                    case .whitespace, .leftBrace: j += 1
                    default: break lookahead
                    }
                }
                let start16 = utf16Index(token.range.utf8Offset)
                guard start16 >= 0 else { i += 1; continue }
                let line = lineIndex(forChar: start16)
                if name == "begin", let envName = nameValue {
                    envStack.append((envName, line, signature(command: "begin:\(envName)", headerLine: line)))
                } else if name == "end", let envName = nameValue,
                          let matchIndex = envStack.lastIndex(where: { $0.name == envName }) {
                    let entry = envStack.remove(at: matchIndex)
                    if entry.headerLine < line - 1 {
                        regions.append(FoldRegion(
                            headerLine: entry.headerLine,
                            hiddenLineRange: (entry.headerLine + 1)...line,
                            signature: entry.signature
                        ))
                    }
                }
            } else if let level = sectionLevels[name] {
                let start16 = utf16Index(token.range.utf8Offset)
                if start16 >= 0 {
                    let line = lineIndex(forChar: start16)
                    closeSections(atOrAbove: level, endLine: line)
                    openSections.append((level, line, signature(command: "section:\(name)", headerLine: line)))
                }
            }
            i += 1
        }
        closeSections(atOrAbove: -1, endLine: lineStarts.count)
        return regions.sorted { $0.headerLine < $1.headerLine }
    }

    // MARK: - Folding

    func region(atLine line: Int) -> FoldRegion? {
        regions.first { $0.headerLine == line }
    }

    /// True when `line` is currently collapsed inside a folded region — the
    /// gutter uses this to skip drawing numbers for hidden rows.
    func isLineHidden(_ line: Int) -> Bool { hiddenLines.contains(line) }

    /// UTF-16 offset of the first character of `line`, or -1 when out of range.
    func lineStart(for line: Int) -> Int {
        guard line >= 0, line < lineStarts.count else { return -1 }
        return lineStarts[line]
    }

    func toggle(atLine line: Int) {
        guard let index = regions.firstIndex(where: { $0.headerLine == line }) else { return }
        regions[index].folded.toggle()
        applyFolds()
        onChange?()
    }

    /// Unfolds the region whose header line is `line` (chip click).
    func unfold(atLine line: Int) {
        guard let index = regions.firstIndex(where: { $0.headerLine == line }),
              regions[index].folded else { return }
        regions[index].folded = false
        applyFolds()
        onChange?()
    }

    /// Re-derives the hidden line set and glyph visibility from `regions`.
    private func applyFolds() {
        guard let textView, let layoutManager = textView.layoutManager,
              let container = textView.textContainer else { return }
        var lines = IndexSet()
        var hiddenCharRanges: [NSRange] = []
        for region in regions where region.folded {
            guard region.headerLine + 1 < lineStarts.count else { continue }
            let first = lineStarts[region.hiddenLineRange.lowerBound]
            let lastLine = min(region.hiddenLineRange.upperBound, lineStarts.count - 1)
            let end = lastLine + 1 < lineStarts.count ? lineStarts[lastLine + 1] : (textView.string as NSString).length
            guard end > first else { continue }
            hiddenCharRanges.append(NSRange(location: first, length: end - first))
            for line in region.hiddenLineRange.lowerBound...min(region.hiddenLineRange.upperBound, lineStarts.count - 1) {
                lines.insert(line)
            }
        }
        if lines == hiddenLines && lines.isEmpty { return }
        hiddenLines = lines
        layoutState = LayoutState(lineStarts: lineStarts, hiddenLines: lines)
        // Rebuild layout so the delegate's zero-height fragments apply, then
        // mark the hidden glyphs not-shown so they do not paint.
        layoutManager.invalidateLayout(
            forCharacterRange: NSRange(location: 0, length: (textView.string as NSString).length),
            actualCharacterRange: nil
        )
        layoutManager.ensureLayout(for: container)
        for range in hiddenCharRanges {
            let glyphs = layoutManager.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            for glyph in glyphs.location..<NSMaxRange(glyphs) {
                layoutManager.setNotShownAttribute(true, forGlyphAt: glyph)
            }
            layoutManager.invalidateDisplay(forGlyphRange: glyphs)
        }
        textView.needsDisplay = true
    }

    private func clearHiddenGlyphs(layoutManager: NSLayoutManager) {
        guard let textView else { return }
        let text = textView.string as NSString
        var lines = IndexSet()
        for region in regions where region.folded {
            for line in region.hiddenLineRange.lowerBound...min(region.hiddenLineRange.upperBound, lineStarts.count - 1) {
                lines.insert(line)
            }
        }
        guard !lines.isEmpty else { return }
        for line in lines {
            let start = lineStarts[line]
            let end = line + 1 < lineStarts.count ? lineStarts[line + 1] : text.length
            let glyphs = layoutManager.glyphRange(
                forCharacterRange: NSRange(location: start, length: max(end - start, 0)),
                actualCharacterRange: nil
            )
            for glyph in glyphs.location..<NSMaxRange(glyphs) {
                layoutManager.setNotShownAttribute(false, forGlyphAt: glyph)
            }
        }
    }

    // MARK: - NSLayoutManagerDelegate

    /// Collapses hidden lines' fragments to zero height; returns true so the
    /// manager adopts the adjusted rects. Reads only the value-type snapshot,
    /// so it is safe under the delegate's nonisolated requirement.
    nonisolated func layoutManager(
        _ layoutManager: NSLayoutManager,
        shouldSetLineFragmentRect lineFragmentRect: UnsafeMutablePointer<NSRect>,
        lineFragmentUsedRect: UnsafeMutablePointer<NSRect>,
        baselineOffset: UnsafeMutablePointer<CGFloat>,
        in textContainer: NSTextContainer,
        forGlyphRange glyphRange: NSRange
    ) -> Bool {
        let state = layoutState
        let starts = state.lineStarts
        guard !starts.isEmpty, !state.hiddenLines.isEmpty else { return false }
        let charIndex = layoutManager.characterIndexForGlyph(at: glyphRange.location)
        var lo = 0, hi = starts.count - 1
        while lo < hi {
            let mid = (lo + hi + 1) / 2
            if starts[mid] <= charIndex { lo = mid } else { hi = mid - 1 }
        }
        guard state.hiddenLines.contains(lo) else { return false }
        lineFragmentRect.pointee.size.height = 0
        lineFragmentUsedRect.pointee.size.height = 0
        return true
    }
}

/// Paints the reference editor's translucent wash behind the bracket at the
/// cursor and its match. Uses the `.backgroundColor` attribute, which the
/// syntax highlighter never touches (it only rewrites `.foregroundColor`), so
/// highlight passes do not erase the match.
@MainActor
final class BracketMatcher {
    private weak var adapter: EditorMacAdapter?
    private var appliedRanges: [NSRange] = []
    nonisolated(unsafe) private var observer: NSObjectProtocol?

    func attach(to adapter: EditorMacAdapter) {
        self.adapter = adapter
        adapter.onSelectionDidChange = { [weak self] in self?.update() }
        observer = NotificationCenter.default.addObserver(
            forName: NSText.didChangeNotification,
            object: adapter.textView,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.update() }
        }
    }

    func detach() {
        if let observer { NotificationCenter.default.removeObserver(observer) }
        observer = nil
        clear()
        adapter = nil
    }

    func update() {
        clear()
        guard let textView = adapter?.textView else { return }
        let text = textView.string as NSString
        let cursor = textView.selectedRange().location
        // UTF-16 code units: { } [ ] ( )
        let openers: Set<unichar> = [0x7B, 0x5B, 0x28]
        let closers: Set<unichar> = [0x7D, 0x5D, 0x29]

        // Prefer the bracket immediately before the cursor; otherwise take
        // the opener under the cursor.
        var anchor: Int?
        var forward = true
        if cursor > 0, closers.contains(text.character(at: cursor - 1)) {
            anchor = cursor - 1
            forward = false
        } else if cursor > 0, openers.contains(text.character(at: cursor - 1)) {
            anchor = cursor - 1
            forward = true
        } else if cursor < text.length, openers.contains(text.character(at: cursor)) {
            anchor = cursor
            forward = true
        } else if cursor < text.length, closers.contains(text.character(at: cursor)) {
            anchor = cursor
            forward = false
        }
        guard let bracket = anchor else { return }
        guard let match = Self.findMatch(in: text, at: bracket, forward: forward) else { return }
        paint([NSRange(location: bracket, length: 1), NSRange(location: match, length: 1)])
    }

    /// Depth scan for the same bracket pair only — other bracket types nested
    /// inside are ignored. Skips brackets inside comments so `% }` never
    /// matches.
    private static func findMatch(
        in text: NSString,
        at location: Int,
        forward: Bool
    ) -> Int? {
        // UTF-16 code units: {} [] ()
        let closerFor: [unichar: unichar] = [0x7B: 0x7D, 0x5B: 0x5D, 0x28: 0x29]
        let openerFor: [unichar: unichar] = [0x7D: 0x7B, 0x5D: 0x5B, 0x29: 0x28]
        let anchor = text.character(at: location)
        let open: unichar
        let close: unichar
        if forward {
            open = anchor
            guard let match = closerFor[open] else { return nil }
            close = match
        } else {
            close = anchor
            guard let match = openerFor[close] else { return nil }
            open = match
        }
        var depth = 0
        var index = location
        while index >= 0, index < text.length {
            let char = text.character(at: index)
            if forward, char == 37 { // % — comment runs to end of line
                let lineRange = text.lineRange(for: NSRange(location: index, length: 0))
                index = NSMaxRange(lineRange)
                continue
            }
            if forward {
                if char == open { depth += 1 }
                if char == close {
                    depth -= 1
                    if depth == 0 { return index }
                }
                index += 1
            } else {
                if char == close { depth += 1 }
                if char == open {
                    depth -= 1
                    if depth == 0 { return index }
                }
                index -= 1
            }
        }
        return nil
    }

    private func paint(_ ranges: [NSRange]) {
        guard let storage = adapter?.textView.textStorage else { return }
        let color = AppearanceSettings.shared.color(for: .bracketMatch)
        storage.beginEditing()
        for range in ranges {
            storage.addAttribute(.backgroundColor, value: color, range: range)
        }
        storage.endEditing()
        appliedRanges = ranges
    }

    private func clear() {
        guard let storage = adapter?.textView.textStorage, !appliedRanges.isEmpty else { return }
        storage.beginEditing()
        for range in appliedRanges where NSMaxRange(range) <= storage.length {
            storage.removeAttribute(.backgroundColor, range: range)
        }
        storage.endEditing()
        appliedRanges = []
    }
}

/// Draws the small "…" chip at the end of a folded header line and handles
/// clicks to unfold — the reference editor's folded-region affordance.
final class FoldChipOverlayView: NSView {
    weak var engine: FoldEngine?
    private weak var textView: NSTextView?
    private weak var scrollView: NSScrollView?
    /// Chip rects in overlay coordinates, rebuilt each draw for hit-testing.
    private var chipRects: [(rect: NSRect, headerLine: Int)] = []
    nonisolated(unsafe) private var observers: [NSObjectProtocol] = []

    /// Top-down coordinates matching the text view's document space so chip
    /// rects and text land on their folded header lines.
    override var isFlipped: Bool { true }

    init(textView: NSTextView, scrollView: NSScrollView, engine: FoldEngine) {
        self.textView = textView
        self.scrollView = scrollView
        self.engine = engine
        super.init(frame: .zero)
        clipsToBounds = true
        autoresizingMask = [.width, .height]
        scrollView.contentView.postsFrameChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(
            forName: NSText.didChangeNotification, object: textView, queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.boundsDidChangeNotification, object: scrollView.contentView, queue: .main
        ) { [weak self] _ in self?.needsDisplay = true })
        observers.append(NotificationCenter.default.addObserver(
            forName: NSView.frameDidChangeNotification, object: scrollView.contentView, queue: .main
        ) { [weak self] _ in self?.reposition() })
        reposition()
    }

    required init?(coder: NSCoder) { nil }

    deinit { observers.forEach { NotificationCenter.default.removeObserver($0) } }

    /// Covers the document viewport; `hitTest` limits interaction to chips.
    private func reposition() {
        guard let scrollView else { return }
        frame = scrollView.contentView.frame
        needsDisplay = true
    }

    override func hitTest(_ point: NSPoint) -> NSView? {
        let local = convert(point, from: superview)
        guard bounds.contains(local) else { return nil }
        for chip in chipRects where chip.rect.contains(local) { return self }
        return nil
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        for chip in chipRects where chip.rect.contains(point) {
            engine?.unfold(atLine: chip.headerLine)
            return
        }
    }

    override func draw(_ dirtyRect: NSRect) {
        guard let engine, let textView,
              let layoutManager = textView.layoutManager,
              let container = textView.textContainer else { return }
        chipRects = []
        let accent = AppearanceSettings.shared.color(for: .foldAccent)
        let inset = textView.textContainerInset
        let text = textView.string as NSString
        let font = textView.font ?? NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)

        for region in engine.regions where region.folded {
            let headerStart = engine.lineStart(for: region.headerLine)
            guard headerStart >= 0, headerStart < text.length else { continue }
            let headerRange = text.lineRange(for: NSRange(location: headerStart, length: 0))
            let glyphs = layoutManager.glyphRange(forCharacterRange: headerRange, actualCharacterRange: nil)
            guard glyphs.length > 0 else { continue }
            let used = layoutManager.boundingRect(forGlyphRange: glyphs, in: container)
            // Chip sits just after the line's last glyph.
            var lastX = used.minX
            var lastGlyphRect = NSRect.zero
            for glyph in glyphs.location..<NSMaxRange(glyphs) {
                let rect = layoutManager.boundingRect(forGlyphRange: NSRange(location: glyph, length: 1), in: container)
                if rect.maxX > lastX { lastX = rect.maxX; lastGlyphRect = rect }
            }
            let chipText = " ⋯ " as NSString
            let attrs: [NSAttributedString.Key: Any] = [
                .font: NSFont.monospacedSystemFont(ofSize: max(font.pointSize - 2, 9), weight: .medium),
                .foregroundColor: accent,
            ]
            let chipSize = chipText.size(withAttributes: attrs)
            let chipHeight = chipSize.height + 4
            let originInView = NSPoint(
                x: lastX + inset.width + 6,
                y: used.minY + inset.height + (used.height - chipHeight) / 2
            )
            let rect = NSRect(
                origin: convert(originInView, from: textView),
                size: NSSize(width: chipSize.width + 4, height: chipHeight)
            )
            chipRects.append((rect, region.headerLine))
            guard rect.intersects(dirtyRect) else { continue }
            accent.withAlphaComponent(0.18).setFill()
            NSBezierPath(roundedRect: rect, xRadius: 3, yRadius: 3).fill()
            accent.withAlphaComponent(0.8).setStroke()
            NSBezierPath(roundedRect: rect.insetBy(dx: 0.5, dy: 0.5), xRadius: 3, yRadius: 3).stroke()
            chipText.draw(
                at: NSPoint(x: rect.minX + 2, y: rect.minY + 2),
                withAttributes: attrs
            )
        }
    }
}
