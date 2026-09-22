#if os(macOS)
import AppKit

/// AppKit owns matching, options, replacement and highlights. This container
/// adds only the current-match counter beside the native find bar.
@MainActor
final class EditorFindController: NSObject, @preconcurrency NSTextFinderBarContainer {
    let finder = NSTextFinder()
    let counter = NSTextField(labelWithString: "0 / 0")
    private weak var textView: NSTextView?
    private weak var scrollView: NSScrollView?
    private var matchesObserver: NSKeyValueObservation?
    nonisolated(unsafe) private var selectionObserver: NSObjectProtocol?

    init(textView: NSTextView, client: any NSTextFinderClient, scrollView: NSScrollView) {
        self.textView = textView
        self.scrollView = scrollView
        super.init()
        counter.font = .monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular)
        counter.textColor = .secondaryLabelColor
        counter.alignment = .center
        counter.setAccessibilityIdentifier("pitex.search.count")
        counter.setAccessibilityLabel(String(localized: "editor.find"))
        counter.setContentHuggingPriority(.required, for: .horizontal)
        finder.client = client
        finder.findBarContainer = self
        finder.isIncrementalSearchingEnabled = true
        matchesObserver = finder.observe(\.incrementalMatchRanges, options: [.new]) { [weak self] _, _ in
            MainActor.assumeIsolated { self?.updateCount() }
        }
        selectionObserver = NotificationCenter.default.addObserver(
            forName: NSTextView.didChangeSelectionNotification, object: textView, queue: .main
        ) { [weak self] _ in MainActor.assumeIsolated { self?.updateCount() } }
    }

    deinit { if let selectionObserver { NotificationCenter.default.removeObserver(selectionObserver) } }

    func perform(_ action: NSTextFinder.Action) {
        finder.performAction(action)
        updateCount()
    }

    private func updateCount() {
        let ranges = finder.incrementalMatchRanges.map(\.rangeValue)
        let selection = textView?.selectedRange()
        let current = selection.flatMap { ranges.firstIndex(of: $0) }.map { $0 + 1 } ?? 0
        counter.stringValue = "\(current) / \(ranges.count)"
    }

    var findBarView: NSView? {
        didSet {
            guard let bar = findBarView else { scrollView?.findBarView = nil; return }
            let row = NSStackView(views: [bar, counter])
            row.orientation = .horizontal
            row.spacing = 8
            row.edgeInsets = NSEdgeInsets(top: 0, left: 0, bottom: 0, right: 8)
            row.frame = NSRect(x: 0, y: 0, width: scrollView?.bounds.width ?? 600, height: bar.frame.height)
            row.autoresizingMask = [.width]
            bar.setContentHuggingPriority(.defaultLow, for: .horizontal)
            scrollView?.findBarView = row
        }
    }
    var isFindBarVisible: Bool {
        get { scrollView?.isFindBarVisible ?? false }
        set { scrollView?.isFindBarVisible = newValue }
    }
    func findBarViewDidChangeHeight() {
        if let bar = findBarView { scrollView?.findBarView?.setFrameSize(NSSize(width: scrollView?.bounds.width ?? 600, height: bar.frame.height)) }
        scrollView?.findBarViewDidChangeHeight()
    }
    var contentView: NSView? { scrollView?.contentView }
}
#endif
