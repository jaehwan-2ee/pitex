import AppPorts
import EditorFeature
import Foundation

#if os(macOS)
import AppKit

@MainActor
private final class SessionTextView: NSTextView {
    let sessionUndoManager = UndoManager()
    private var pendingReveal: (range: NSRange, highlight: Bool)?

    /// The UTF-16 range an in-flight completion replaces. Supplied by the
    /// adapter from the shared context detector so `\cite{a,k` completes
    /// only `k` — NSTextView's word-boundary default would swallow `a,k`.
    var userCompletionRange: (() -> NSRange?)?

    override var rangeForUserCompletion: NSRange {
        userCompletionRange?() ?? super.rangeForUserCompletion
    }

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

    /// Supplies candidates for the native completion popup. Called on the
    /// main actor with the live text and caret UTF-16 offset; returns the
    /// UTF-16 range the chosen candidate replaces plus the candidate
    /// strings in display order. A nil result (or empty list) keeps the
    /// popup closed — the data source stays outside this adapter.
    public var completionSource: (@MainActor (_ text: String, _ caretUTF16Offset: Int) -> (range: NSRange, candidates: [String])?)?

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
        sessionTextView.sessionUndoManager.disableUndoRegistration()
        nativeView.string = snapshot.text
        sessionTextView.sessionUndoManager.enableUndoRegistration()
        nativeView.delegate = self
        nativeView.userCompletionRange = { [weak self, weak nativeView] in
            guard let self, let nativeView, let completionSource = self.completionSource
            else { return nil }
            return completionSource(nativeView.string, nativeView.selectedRange().location)?.range
        }
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

    /// The documented NSTextView hook for a custom undo manager — the
    /// `undoManager` override alone reports the session manager to the
    /// responder chain while internal registration still resolves the
    /// delegate, so without this ⌘Z dispatches to a manager that never
    /// saw the edits.
    public func undoManager(for view: NSTextView) -> UndoManager? {
        sessionTextView.sessionUndoManager
    }

    public func textDidChange(_ notification: Notification) {
        guard !isApplyingSessionSnapshot else { return }
        desiredText = textView.string
        onTextDidChange?()
        scheduleCompletionTrigger()
        submitPendingChangeIfNeeded()
    }

    /// The native completion popup's candidate query after `complete:`
    /// resolves `rangeForUserCompletion`. The shared context detector owns
    /// the candidate list; `words` is the fallback for plain text.
    public func textView(
        _ textView: NSTextView,
        completions words: [String],
        forPartialWordRange charRange: NSRange,
        indexOfSelectedItem index: UnsafeMutablePointer<Int>?
    ) -> [String] {
        let caret = min(
            textView.selectedRange().location + textView.selectedRange().length,
            (textView.string as NSString).length
        )
        return completionSource?(textView.string, caret)?.candidates ?? words
    }

    /// Shows the native completion popup when the caret sits in a
    /// completable context (a `\command` prefix, `\cite{…}` or `\ref{…}`
    /// group); a no-op elsewhere. F5/⌥⎋ reach the same popup through
    /// NSTextView's own key bindings.
    public func requestCompletion() {
        presentCompletionIfContextual()
    }

    /// Debounced so the popup follows typing without re-evaluating the
    /// context on every keystroke. While the popup is open the partial
    /// completion is marked text, which suppresses retriggering.
    private var completionTriggerTask: Task<Void, Never>?

    private func scheduleCompletionTrigger() {
        completionTriggerTask?.cancel()
        guard completionSource != nil else { return }
        completionTriggerTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(200))
            guard !Task.isCancelled, let self else { return }
            self.presentCompletionIfContextual()
        }
    }

    private func presentCompletionIfContextual() {
        guard let completionSource, !textView.hasMarkedText() else { return }
        let selection = textView.selectedRange()
        guard selection.length == 0,
              let result = completionSource(textView.string, selection.location),
              !result.candidates.isEmpty else { return }
        textView.complete(nil)
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
        // A session-applied snapshot is not a user edit — registering it
        // would push a whole-document undo entry (and, on AppKit, reset
        // the coalesced typing history around it).
        sessionTextView.sessionUndoManager.disableUndoRegistration()
        textView.string = snapshot.text
        sessionTextView.sessionUndoManager.enableUndoRegistration()
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
