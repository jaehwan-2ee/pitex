import AppPorts
import EditorFeature
import Foundation

#if os(macOS)
import AppKit

@MainActor
private final class SessionTextView: NSTextView {
    let sessionUndoManager = UndoManager()
    private var pendingReveal: (range: NSRange, highlight: Bool)?

    override var undoManager: UndoManager? { sessionUndoManager }

    func reveal(_ range: NSRange, highlight: Bool) {
        setSelectedRange(range)
        pendingReveal = (range, highlight)
        revealIfReady()
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        needsLayout = true
    }

    override func layout() {
        super.layout()
        revealIfReady()
    }

    private func revealIfReady() {
        guard let request = pendingReveal, let window,
              let scrollView = enclosingScrollView,
              scrollView.contentSize.width > 0, scrollView.contentSize.height > 0,
              let textContainer else { return }
        pendingReveal = nil
        layoutManager?.ensureLayout(for: textContainer)
        scrollRangeToVisible(request.range)
        window.makeFirstResponder(self)
        // The native find indicator must be shown after mounting and scrolling.
        // A zero-length range clears an earlier indicator when highlighting is off.
        var start = request.range.location
        var end = start
        if request.highlight {
            (string as NSString).getLineStart(&start, end: nil, contentsEnd: &end, for: request.range)
        }
        showFindIndicator(for: NSRange(location: start, length: end - start))
    }
}

@MainActor
public final class EditorMacAdapter: NSObject, NSTextViewDelegate {
    public let textView: NSTextView
    public let nativeUndoManager: UndoManager

    public var text: String { textView.string }
    public var selectedRange: NSRange { textView.selectedRange() }
    public var hasMarkedText: Bool { textView.hasMarkedText() }
    public var markedRange: NSRange { textView.markedRange() }

    /// Called on the main actor after user-driven text changes. Used to drive
    /// syntax highlighting and change monitors without taking ownership of text.
    public var onTextDidChange: (@MainActor () -> Void)?

    /// Called after the selection changes (cursor moves, clicks, typing).
    /// Used for caret-driven chrome such as bracket matching.
    public var onSelectionDidChange: (@MainActor () -> Void)?

    private let session: any DocumentSessionPort
    private let sessionTextView: SessionTextView
    private var committed: DocumentSnapshot
    private var desiredText: String
    private var isApplyingSessionSnapshot = false
    private var isSubmitting = false

    private init(session: any DocumentSessionPort, snapshot: DocumentSnapshot) {
        self.session = session
        committed = snapshot
        desiredText = snapshot.text

        let nativeView = SessionTextView(frame: NSRect(x: 0, y: 0, width: 1, height: 1))
        sessionTextView = nativeView
        textView = nativeView
        nativeUndoManager = nativeView.sessionUndoManager
        super.init()

        nativeView.isRichText = false
        nativeView.importsGraphics = false
        nativeView.allowsUndo = true
        nativeView.string = snapshot.text
        nativeView.delegate = self
    }

    public static func make(session: any DocumentSessionPort) async throws -> EditorMacAdapter {
        let snapshot = await session.snapshot()
        return EditorMacAdapter(session: session, snapshot: snapshot)
    }

    public func refreshFromSession() async {
        apply(await session.snapshot())
    }

    /// A newly activated document may not be mounted by SwiftUI yet.
    public func revealSelection(_ range: NSRange, highlight: Bool = false) {
        let count = text.utf16.count
        let location = min(max(range.location, 0), count)
        sessionTextView.reveal(NSRange(location: location, length: min(max(range.length, 0), count - location)), highlight: highlight)
    }

    public func textViewDidChangeSelection(_ notification: Notification) {
        onSelectionDidChange?()
    }

    public func textDidChange(_ notification: Notification) {
        guard !isApplyingSessionSnapshot else { return }
        desiredText = textView.string
        onTextDidChange?()
        submitPendingChangeIfNeeded()
    }

    private func submitPendingChangeIfNeeded() {
        guard !isSubmitting, desiredText != committed.text else { return }
        isSubmitting = true
        let submittedText = desiredText
        let mutation = DocumentMutation(
            baseRevision: committed.revision,
            range: DocumentTextRange(location: 0, length: committed.text.utf16.count),
            replacement: submittedText
        )
        let session = self.session

        Task { @MainActor [weak self, session] in
            do {
                let result = try await session.submit(mutation)
                guard let self else { return }
                switch result {
                case let .applied(snapshot):
                    committed = snapshot
                    if desiredText == submittedText, snapshot.text != submittedText {
                        apply(snapshot)
                    }
                case let .rejected(current):
                    apply(current)
                }
            } catch {
                guard let self else { return }
                apply(await session.snapshot())
            }
            self?.isSubmitting = false
            self?.submitPendingChangeIfNeeded()
        }
    }

    private func apply(_ snapshot: DocumentSnapshot) {
        committed = snapshot
        desiredText = snapshot.text
        guard textView.string != snapshot.text else { return }

        let previousSelection = textView.selectedRange()
        isApplyingSessionSnapshot = true
        textView.string = snapshot.text
        let maximum = snapshot.text.utf16.count
        let location = min(previousSelection.location, maximum)
        let length = min(previousSelection.length, maximum - location)
        textView.setSelectedRange(NSRange(location: location, length: length))
        isApplyingSessionSnapshot = false
    }
}

#else

@MainActor
public final class EditorMacAdapter {
    private init() {}

    public static func make(session: any DocumentSessionPort) async throws -> EditorMacAdapter {
        throw PlatformPortError.unavailable(feature: "TextKit editor adapter", platform: "non-macOS")
    }
}
#endif
