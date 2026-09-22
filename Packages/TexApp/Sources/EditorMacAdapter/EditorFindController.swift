#if os(macOS)
import AppKit

/// Preserve AppKit navigation, options, replacement and highlights, with
/// a cached literal-term count that remains valid after document edits.
@MainActor
final class EditorFindController: NSObject, @preconcurrency NSTextFinderBarContainer, @preconcurrency NSTextFinderClient {
    let finder = NSTextFinder()
    let counter = NSTextField(labelWithString: "0 / 0")
    private weak var textView: NSTextView?
    private weak var scrollView: NSScrollView?
    private var matchesObserver: NSKeyValueObservation?
    private var countTask: Task<Void, Never>?
    private var countGeneration = 0
    private var counting = false
    private var matches: [NSRange] = []
    private var countedQuery = ""
    private var countedOptions: NSDictionary?
    private var contentRevision = 0
    private var countedRevision = -1
    nonisolated(unsafe) private var selectionObserver: NSObjectProtocol?

    init(textView: NSTextView, scrollView: NSScrollView) {
        self.textView = textView
        self.scrollView = scrollView
        super.init()
        counter.font = .monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular)
        counter.textColor = .secondaryLabelColor
        counter.alignment = .center
        counter.setAccessibilityIdentifier("pitex.search.count")
        counter.setAccessibilityLabel(String(localized: "editor.find"))
        counter.setContentHuggingPriority(.required, for: .horizontal)
        finder.client = self
        finder.findBarContainer = self
        finder.isIncrementalSearchingEnabled = true
        matchesObserver = finder.observe(\.incrementalMatchRanges, options: [.new]) { [weak self] _, _ in
            MainActor.assumeIsolated { self?.updateCount() }
        }
        selectionObserver = NotificationCenter.default.addObserver(
            forName: NSTextView.didChangeSelectionNotification, object: textView, queue: .main
        ) { [weak self] _ in MainActor.assumeIsolated { self?.updateCount() } }
    }

    deinit { countTask?.cancel(); if let selectionObserver { NotificationCenter.default.removeObserver(selectionObserver) } }

    func perform(_ action: NSTextFinder.Action) {
        finder.performAction(action)
        updateCount()
    }

    func contentDidChange() {
        contentRevision += 1
        updateCount()
    }

    private func searchField(in view: NSView?) -> NSTextField? {
        guard let view else { return nil }
        if let field = view as? NSTextField, field.isEditable { return field }
        return view.subviews.lazy.compactMap { self.searchField(in: $0) }.first
    }

    private func updateCount() {
        guard isFindBarVisible else { return }
        let query = searchField(in: findBarView)?.stringValue ?? ""
        let options = (NSPasteboard(name: .find).propertyList(forType: .textFinderOptions) as? NSDictionary) ?? [:]
        if query != countedQuery || contentRevision != countedRevision || countedOptions != options {
            countedQuery = query
            countedOptions = options
            countedRevision = contentRevision
            counting = true
            countGeneration += 1
            let generation = countGeneration
            countTask?.cancel()
            let text = string
            let insensitive = options[NSPasteboard.PasteboardType.TextFinderOptionKey.textFinderCaseInsensitiveKey.rawValue] as? Bool ?? true
            let kind = options[NSPasteboard.PasteboardType.TextFinderOptionKey.textFinderMatchingTypeKey.rawValue] as? Int ?? 0
            counter.stringValue = query.isEmpty ? "0 / 0" : "…"
            // AppKit clears incrementalMatchRanges when the document is
            // edited. Count literal find terms with its documented options
            // independently, so the total remains valid between key presses.
            countTask = Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(60))
                guard !Task.isCancelled else { return }
                let found = await Task.detached(priority: .userInitiated) {
                    guard !query.isEmpty else { return [NSRange]() }
                    var pattern = NSRegularExpression.escapedPattern(for: query)
                    if kind == 1 || kind == 2 { pattern = "\\b" + pattern }
                    if kind == 2 || kind == 3 { pattern += "\\b" }
                    var flags: NSRegularExpression.Options = [.useUnicodeWordBoundaries]
                    if insensitive { flags.insert(.caseInsensitive) }
                    let regex = try? NSRegularExpression(pattern: pattern, options: flags)
                    return regex?.matches(in: text, range: NSRange(location: 0, length: text.utf16.count)).map(\.range) ?? []
                }.value
                guard !Task.isCancelled, let self, generation == self.countGeneration else { return }
                self.counting = false
                self.matches = found
                self.showCount()
            }
        } else if !counting {
            showCount()
        }
    }

    private func showCount() {
        let current = textView.flatMap { matches.firstIndex(of: $0.selectedRange()) }.map { $0 + 1 } ?? 0
        counter.stringValue = "\(current) / \(matches.count)"
        counter.superview?.needsLayout = true
    }

    // Explicit native client: NSTextView's internal find client is private.
    var string: String { textView?.string ?? "" }
    var isSelectable: Bool { textView?.isSelectable ?? false }
    var isEditable: Bool { textView?.isEditable ?? false }
    var allowsMultipleSelection: Bool { true }
    var firstSelectedRange: NSRange { textView?.selectedRange() ?? NSRange(location: 0, length: 0) }
    var selectedRanges: [NSValue] {
        get { textView?.selectedRanges ?? [] }
        set { textView?.selectedRanges = newValue }
    }
    func scrollRangeToVisible(_ range: NSRange) {
        // The finder reports the active hit here; next/previous actions do
        // not call selectedRanges (that callback serves Select All).
        textView?.setSelectedRange(range)
        textView?.scrollRangeToVisible(range)
        updateCount()
    }
    func shouldReplaceCharacters(inRanges ranges: [NSValue], with strings: [String]) -> Bool {
        guard let textView, !textView.hasMarkedText() else { return false }
        return textView.shouldChangeText(inRanges: ranges, replacementStrings: strings)
    }
    func replaceCharacters(in range: NSRange, with string: String) {
        textView?.replaceCharacters(in: range, with: string)
    }
    func didReplaceCharacters() { textView?.didChangeText() }
    func contentView(at index: Int, effectiveCharacterRange range: NSRangePointer) -> NSView {
        range.pointee = NSRange(location: 0, length: string.utf16.count)
        return textView ?? NSView()
    }
    func rects(forCharacterRange range: NSRange) -> [NSValue]? {
        guard let view = textView, let manager = view.layoutManager, let container = view.textContainer else { return [] }
        let glyphs = manager.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
        var rects: [NSValue] = []
        manager.enumerateEnclosingRects(forGlyphRange: glyphs, withinSelectedGlyphRange: NSRange(location: NSNotFound, length: 0), in: container) { rect, _ in
            rects.append(NSValue(rect: rect.offsetBy(dx: view.textContainerOrigin.x, dy: view.textContainerOrigin.y)))
        }
        return rects
    }
    var visibleCharacterRanges: [NSValue] {
        guard let view = textView, let manager = view.layoutManager, let container = view.textContainer else { return [] }
        let rect = view.visibleRect.offsetBy(dx: -view.textContainerOrigin.x, dy: -view.textContainerOrigin.y)
        let glyphs = manager.glyphRange(forBoundingRect: rect, in: container)
        return [NSValue(range: manager.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil))]
    }

    var findBarView: NSView? {
        didSet { if isFindBarVisible { installBar() } }
    }
    var isFindBarVisible = false {
        didSet {
            if isFindBarVisible { installBar() }
            scrollView?.isFindBarVisible = isFindBarVisible
            if !isFindBarVisible {
                countTask?.cancel()
                counting = false
                matches = []
                countedRevision = -1
                findBarView?.removeFromSuperview()
                scrollView?.findBarView = nil
            }
        }
    }
    private func installBar() {
        guard let bar = findBarView else { return }
        // NSTextFinder checks the bar's superview when opening. Only mount
        // it once visibility is requested, just as NSScrollView does.
        let row = FindBarRow(bar: bar, counter: counter)
        row.frame.size.width = scrollView?.bounds.width ?? 600
        scrollView?.findBarView = row
    }
    func findBarViewDidChangeHeight() {
        if let bar = findBarView { scrollView?.findBarView?.setFrameSize(NSSize(width: scrollView?.bounds.width ?? 600, height: bar.frame.height)) }
        scrollView?.findBarViewDidChangeHeight()
    }
    func contentView() -> NSView? { scrollView?.contentView }
}
@MainActor
private final class FindBarRow: NSView {
    let bar: NSView
    let counter: NSTextField
    init(bar: NSView, counter: NSTextField) {
        self.bar = bar
        self.counter = counter
        super.init(frame: bar.frame)
        autoresizingMask = [.width]
        bar.translatesAutoresizingMaskIntoConstraints = true
        addSubview(bar)
        addSubview(counter)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }
    override var intrinsicContentSize: NSSize { NSSize(width: NSView.noIntrinsicMetric, height: bar.frame.height) }
    override func layout() {
        super.layout()
        let width = max(64, counter.intrinsicContentSize.width + 16)
        bar.frame = NSRect(x: 0, y: 0, width: max(0, bounds.width - width), height: bar.frame.height)
        counter.frame = NSRect(x: bounds.width - width, y: (bounds.height - 20) / 2, width: width - 8, height: 20)
    }
}
#endif
