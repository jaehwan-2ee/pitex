import AppPorts
import EditorFeature
import Foundation

#if os(macOS)
import AppKit

@MainActor
private final class SessionTextView: NSTextView {
    var findController: EditorFindController?

    override func performFindPanelAction(_ sender: Any?) { performTextFinderAction(sender) }

    override func performTextFinderAction(_ sender: Any?) {
        let tag = (sender as? NSMenuItem)?.tag ?? (sender as? NSControl)?.tag ?? 1
        guard let action = NSTextFinder.Action(rawValue: tag), let scrollView = enclosingScrollView else { return }
        if findController == nil { findController = EditorFindController(textView: self, scrollView: scrollView) }
        findController?.perform(action)
    }

    override func validateUserInterfaceItem(_ item: any NSValidatedUserInterfaceItem) -> Bool {
        if item.action == #selector(performTextFinderAction(_:)) || item.action == #selector(performFindPanelAction(_:)),
           let action = NSTextFinder.Action(rawValue: item.tag), let findController {
            return findController.finder.validateAction(action)
        }
        return super.validateUserInterfaceItem(item)
    }

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
    nonisolated(unsafe) private var undoObservers: [NSObjectProtocol] = []

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
        // AppKit can change storage during Undo/Redo without notifying the
        // text-view delegate. Synchronize after the entire native group ends.
        for name in [Notification.Name.NSUndoManagerDidUndoChange, Notification.Name.NSUndoManagerDidRedoChange] {
            undoObservers.append(NotificationCenter.default.addObserver(
                forName: name, object: nativeUndoManager, queue: .main
            ) { [weak self] _ in
                MainActor.assumeIsolated {
                    guard let self, self.desiredText != self.textView.string else { return }
                    self.pendingNativeMutation = nil
                    self.textDidChange(Notification(name: NSText.didChangeNotification, object: self.textView))
                }
            })
        }
    }

    deinit { undoObservers.forEach(NotificationCenter.default.removeObserver) }

    public static func make(session: any DocumentSessionPort) async throws -> EditorMacAdapter {
        let snapshot = await session.snapshot()
        return EditorMacAdapter(session: session, snapshot: snapshot)
    }

    private var needsRefresh = false

    /// An async refresh can land while a submit is in flight — applying it
    /// then would clobber the pending keystroke, so it is deferred until
    /// the submit Task drains. The `isSubmitting` check runs AFTER the
    /// snapshot fetch: a keystroke during the await starts a submit, and
    /// the just-fetched (already stale) snapshot must not clobber it.
    /// `desiredText != committed.text` covers the same window before the
    /// submit Task has even started.
    public func refreshFromSession() async {
        let snapshot = await session.snapshot()
        if isSubmitting || desiredText != committed.text { needsRefresh = true; return }
        apply(snapshot)
    }

    /// Scrolls so the 0-based source line sits at the viewport top — the
    /// preview→editor half of Markdown scroll sync.
    public func scrollToLine(_ line: Int) {
        guard let layoutManager = textView.layoutManager,
              let textContainer = textView.textContainer,
              let scrollView = textView.enclosingScrollView else { return }
        let text = textView.string as NSString
        var index = 0
        var current = 0
        while current < line, index < text.length {
            let next = NSMaxRange(text.lineRange(for: NSRange(location: index, length: 0)))
            guard next > index else { break }
            index = next
            current += 1
        }
        layoutManager.ensureLayout(for: textContainer)
        let point: NSPoint
        if index < text.length {
            let glyph = layoutManager.glyphIndexForCharacter(at: index)
            let rect = layoutManager.boundingRect(forGlyphRange: NSRange(location: glyph, length: 1),
                                                  in: textContainer)
            point = NSPoint(x: 0, y: rect.minY + textView.textContainerOrigin.y)
        } else {
            point = NSPoint(x: 0, y: scrollView.documentView?.frame.maxY ?? 0)
        }
        scrollView.contentView.scroll(to: point)
        scrollView.reflectScrolledClipView(scrollView.contentView)
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

    private var pendingNativeMutation: DocumentMutation?

    public func textView(_ textView: NSTextView, shouldChangeTextIn range: NSRange,
                         replacementString: String?) -> Bool {
        sessionTextView.findController?.finder.noteClientStringWillChange()
        if !isApplyingSessionSnapshot, !isSubmitting, let replacementString {
            pendingNativeMutation = DocumentMutation(baseRevision: committed.revision,
                range: DocumentTextRange(location: range.location, length: range.length),
                replacement: replacementString)
        } else {
            pendingNativeMutation = nil
        }
        return true
    }

    public func textDidChange(_ notification: Notification) {
        guard !isApplyingSessionSnapshot else { return }
        desiredText = textView.string
        sessionTextView.findController?.contentDidChange()
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
        var mutation = pendingNativeMutation ?? DocumentMutation(
            baseRevision: committed.revision,
            range: DocumentTextRange(location: 0, length: committed.text.utf16.count),
            replacement: submittedText
        )
        pendingNativeMutation = nil
        let session = self.session

        Task { @MainActor [weak self, session] in
            guard let self else { return }
            // commitSave/recordExternalChange/resolveConflict advance the
            // revision without touching text — app-side code calls them
            // before every build and agent run, so rejections on a stale
            // base are the common path. While the session text still equals
            // the committed text the pending edit rebases cleanly; a truly
            // diverged session falls back to applying it, which drops the
            // in-flight keystroke. ponytail: a 3-way rebase is the upgrade
            // path if diverged-session conflicts ever become common.
            var attempts = 0
            while true {
                do {
                    switch try await session.submit(mutation) {
                    case let .applied(snapshot):
                        committed = snapshot
                        if desiredText == submittedText, snapshot.text != submittedText {
                            apply(snapshot)
                        }
                    case let .rejected(current):
                        if current.text == committed.text, attempts < 3 {
                            committed = current
                            mutation = DocumentMutation(
                                baseRevision: current.revision,
                                range: mutation.range,
                                replacement: mutation.replacement
                            )
                            attempts += 1
                            continue
                        }
                        apply(current)
                    }
                } catch {
                    apply(await session.snapshot())
                }
                break
            }
            isSubmitting = false
            submitPendingChangeIfNeeded()
            if needsRefresh, !isSubmitting {
                needsRefresh = false
                await refreshFromSession()
            }
        }
    }

    /// Applies a session snapshot by replacing only the differing middle
    /// range: AppKit's own range tracking keeps the caret anchored, and a
    /// live IME composition is discarded first so it cannot re-commit into
    /// the shifted text and double its syllables.
    private func apply(_ snapshot: DocumentSnapshot) {
        pendingNativeMutation = nil
        committed = snapshot
        desiredText = snapshot.text
        guard textView.string != snapshot.text else { return }

        // The flag goes up before discarding the composition — a delegate
        // callback out of discardMarkedText/unmarkText must not start a
        // submit mid-apply.
        isApplyingSessionSnapshot = true
        if textView.hasMarkedText() {
            textView.inputContext?.discardMarkedText()
            textView.unmarkText()
        }
        let (range, replacement) = Self.differingRange(from: textView.string, to: snapshot.text)
        let selection = textView.selectedRange()
        sessionTextView.findController?.finder.noteClientStringWillChange()
        // A session-applied snapshot is not a user edit — registering it
        // would push an undo entry (and, on AppKit, reset the coalesced
        // typing history around it).
        sessionTextView.sessionUndoManager.disableUndoRegistration()
        textView.textStorage?.beginEditing()
        textView.textStorage?.replaceCharacters(in: range, with: replacement)
        textView.textStorage?.endEditing()
        sessionTextView.sessionUndoManager.enableUndoRegistration()
        // Undo entries recorded against the pre-replacement text restore
        // stale ranges after an external edit — the history restarts here.
        sessionTextView.sessionUndoManager.removeAllActions()
        isApplyingSessionSnapshot = false
        textView.setSelectedRange(Self.map(selection, over: range, replacementLength: replacement.utf16.count))
        sessionTextView.findController?.contentDidChange()
    }

    /// Common-prefix/suffix shrink of a full-text replacement, in UTF-16
    /// units (NSTextStorage coordinates); a surrogate pair is never split.
    static func differingRange(from old: String, to new: String) -> (range: NSRange, replacement: String) {
        let oldUnits = Array(old.utf16)
        let newUnits = Array(new.utf16)
        var prefix = 0
        while prefix < oldUnits.count, prefix < newUnits.count,
              oldUnits[prefix] == newUnits[prefix] { prefix += 1 }
        if prefix > 0, prefix < oldUnits.count, UTF16.isLeadSurrogate(oldUnits[prefix - 1]) {
            prefix -= 1
        }
        var suffix = 0
        while suffix < oldUnits.count - prefix, suffix < newUnits.count - prefix,
              oldUnits[oldUnits.count - 1 - suffix] == newUnits[newUnits.count - 1 - suffix] { suffix += 1 }
        if suffix > 0, UTF16.isLeadSurrogate(oldUnits[oldUnits.count - suffix - 1]) {
            suffix -= 1
        }
        let range = NSRange(location: prefix, length: oldUnits.count - prefix - suffix)
        return (range, String(decoding: newUnits[prefix ..< newUnits.count - suffix], as: UTF16.self))
    }

    /// Maps a UTF-16 selection across a replaceCharacters edit: before the
    /// range unchanged, past it shifted by the length delta, overlapping it
    /// clamped onto the end of the replacement.
    static func map(_ selection: NSRange, over edit: NSRange, replacementLength: Int) -> NSRange {
        func map(_ offset: Int) -> Int {
            if offset <= edit.location { return offset }
            if offset >= NSMaxRange(edit) { return offset + replacementLength - edit.length }
            return edit.location + replacementLength
        }
        let start = map(selection.location)
        return NSRange(location: start, length: map(NSMaxRange(selection)) - start)
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
