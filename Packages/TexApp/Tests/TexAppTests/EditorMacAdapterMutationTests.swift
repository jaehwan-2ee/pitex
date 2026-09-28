import AppPorts
import XCTest

#if os(macOS)
import AppKit
@testable import EditorMacAdapter

/// Exercise the native buffer and the session independently: an acknowledgement
/// must never turn a correct AppKit edit into a different document.
final class EditorMacAdapterMutationTests: XCTestCase {
    private let paragraph = "In addition, it"
    private let following = "In this work, we use a gradient-free approach.\n"

    @MainActor
    func testEnterThenBackspacePreservesTheExistingBlankLine() async throws {
        let original = paragraph + "\n\n" + following
        let session = GatedEditorSession(text: original)
        addTeardownBlock { await session.releaseAll() }
        let (adapter, window) = try await mount(session)
        defer { window.orderOut(nil) }
        let view = adapter.textView
        view.setSelectedRange(NSRange(location: paragraph.utf16.count, length: 0))

        view.insertNewline(nil)
        assertNative(view, paragraph + "\n\n\n" + following, caret: paragraph.utf16.count + 1)
        try await waitForSubmission(1, session)
        // Backspace removes only the newline just inserted, even while its
        // acknowledgement is held. The pre-existing empty line must survive.
        view.deleteBackward(nil)
        assertNative(view, original, caret: paragraph.utf16.count)
        try await session.releaseAcknowledgement()
        try await waitForSubmission(2, session)
        assertNative(view, original, caret: paragraph.utf16.count)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: original)
        assertNative(view, original, caret: paragraph.utf16.count)
    }

    @MainActor
    func testActualNativeBufferWinsOverAnIncompleteChangeProposal() async throws {
        let original = paragraph + "\n\n" + following
        let expected = paragraph + " addition\n\n" + following
        let session = GatedEditorSession(text: original)
        addTeardownBlock { await session.releaseAll() }
        let (adapter, window) = try await mount(session)
        defer { window.orderOut(nil) }
        let view = adapter.textView
        let insertion = NSRange(location: paragraph.utf16.count, length: 0)

        // A should-change callback is a proposal, not a description of the
        // final storage transaction. Model a native edit whose final contents
        // differ from that proposal, using NSTextView's public edit contract.
        // Trusting the proposal here deletes one of the paragraph separators.
        XCTAssertTrue(view.shouldChangeText(
            in: NSRange(location: insertion.location, length: 1), replacementString: " addition"))
        view.textStorage?.replaceCharacters(in: insertion, with: " addition")
        view.setSelectedRange(NSRange(location: insertion.location + " addition".utf16.count, length: 0))
        view.didChangeText()

        assertNative(view, expected, caret: paragraph.utf16.count + " addition".utf16.count)
        try await waitForSubmission(1, session)
        await assertSession(session, equals: expected)
        try await finishSubmission(adapter, session)
        assertNative(view, expected, caret: paragraph.utf16.count + " addition".utf16.count)
        await assertSession(session, equals: expected)
    }

    @MainActor
    func testDelayedAcknowledgementKeepsRapidMixedUnicodeEditsAndCaret() async throws {
        let prefix = "한 👩🏽‍💻 e\u{301}: " + paragraph
        let original = prefix + "\n\n" + following
        let expected = prefix + " addition!\n\n" + following
        let session = GatedEditorSession(text: original)
        addTeardownBlock { await session.releaseAll() }
        let (adapter, window) = try await mount(session)
        defer { window.orderOut(nil) }
        let view = adapter.textView
        view.setSelectedRange(NSRange(location: prefix.utf16.count, length: 0))
        view.insertText(" a", replacementRange: view.selectedRange())
        try await waitForSubmission(1, session)

        view.insertNewline(nil)
        view.deleteBackward(nil)
        view.insertText("ddition?", replacementRange: view.selectedRange())
        view.deleteBackward(nil)
        view.insertText("!", replacementRange: view.selectedRange())
        let caret = (prefix + " addition!").utf16.count
        assertNative(view, expected, caret: caret)
        // A refresh during the pending submit must also preserve queued edits.
        await adapter.refreshFromSession()
        assertNative(view, expected, caret: caret)

        try await session.releaseAcknowledgement()
        try await waitForSubmission(2, session)
        // The first acknowledgement described only " a". It must not roll
        // back any later typing or move the UTF-16 caret before the emoji.
        assertNative(view, expected, caret: caret)
        await assertSession(session, equals: expected)
        try await finishSubmission(adapter, session)
        assertNative(view, expected, caret: caret)
        await assertSession(session, equals: expected)
    }

    @MainActor
    func testSnapshotSuspendedAcrossAWholeSubmitCannotRollBackTheDocument() async throws {
        let original = paragraph + "\n\n" + following
        let expected = paragraph + "!\n\n" + following
        let session = GatedEditorSession(text: original)
        addTeardownBlock { await session.releaseAll() }
        let (adapter, window) = try await mount(session)
        defer { window.orderOut(nil) }
        let view = adapter.textView
        await session.holdNextSnapshot()
        let refresh = Task { @MainActor in await adapter.refreshFromSession() }
        try await waitUntil("refresh has captured the old snapshot") {
            await session.hasHeldSnapshot
        }

        view.setSelectedRange(NSRange(location: paragraph.utf16.count, length: 0))
        view.insertText("!", replacementRange: view.selectedRange())
        try await waitForSubmission(1, session)
        // This drains the complete submit (including its acknowledgement)
        // before allowing the older, already captured snapshot to return.
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: expected)
        try await session.releaseSnapshot()
        await refresh.value

        assertNative(view, expected, caret: paragraph.utf16.count + 1)
        await assertSession(session, equals: expected)
        await adapter.refreshFromSession()
        assertNative(view, expected, caret: paragraph.utf16.count + 1)
    }

    @MainActor
    func testSaveRevisionBumpAndRejectedAcknowledgementPreserveQueuedBlankLines() async throws {
        let original = paragraph + "\n\n" + following
        let expected = paragraph + " more!\n\n" + following
        let session = GatedEditorSession(text: original)
        addTeardownBlock { await session.releaseAll() }
        let (adapter, window) = try await mount(session)
        defer { window.orderOut(nil) }
        let view = adapter.textView
        view.setSelectedRange(NSRange(location: paragraph.utf16.count, length: 0))
        view.insertText(" more", replacementRange: view.selectedRange())
        try await waitForSubmission(1, session)
        await session.commitSave()
        view.insertNewline(nil)
        try await session.releaseAcknowledgement()
        try await waitForSubmission(2, session)
        let rejected = await session.rejectionCount
        XCTAssertEqual(rejected, 1, "The save must make the queued edit's base revision stale")

        // More typing arrives while the stale-revision response itself is held.
        view.deleteBackward(nil)
        view.insertText("!", replacementRange: view.selectedRange())
        let caret = (paragraph + " more!").utf16.count
        assertNative(view, expected, caret: caret)
        try await session.releaseAcknowledgement()
        try await waitForSubmission(3, session) // rebased newline insertion
        assertNative(view, expected, caret: caret)
        try await session.releaseAcknowledgement()
        try await waitForSubmission(4, session) // remaining Backspace and typing
        try await finishSubmission(adapter, session)

        await session.commitSave()
        await adapter.refreshFromSession()
        let saved = await session.savedText
        XCTAssertEqual(saved, expected)
        await assertSession(session, equals: expected)
        assertNative(view, expected, caret: caret)
    }

    @MainActor
    func testNativeUndoAndRedoDuringPendingAcknowledgementPreserveUnicodeAndBlankLines() async throws {
        let prefix = "한 👩🏽‍💻 e\u{301}: " + paragraph
        let original = prefix + "\n\n" + following
        let expected = prefix + " addition\n\n" + following
        let session = GatedEditorSession(text: original)
        addTeardownBlock { await session.releaseAll() }
        let (adapter, window) = try await mount(session)
        defer { window.orderOut(nil) }
        let view = adapter.textView
        let undo = adapter.nativeUndoManager
        undo.groupsByEvent = false
        view.setSelectedRange(NSRange(location: prefix.utf16.count, length: 0))
        undo.beginUndoGrouping()
        view.insertText(" addition", replacementRange: view.selectedRange())
        undo.endUndoGrouping()
        try await waitForSubmission(1, session)

        XCTAssertTrue(undo.canUndo)
        undo.undo()
        XCTAssertEqual(view.string, original)
        let undoneSelection = view.selectedRange()
        try await session.releaseAcknowledgement()
        try await waitForSubmission(2, session)
        XCTAssertEqual(view.string, original)
        XCTAssertEqual(view.selectedRange(), undoneSelection)
        XCTAssertTrue(undo.canRedo)
        undo.redo()
        XCTAssertEqual(view.string, expected)
        let redoneSelection = view.selectedRange()
        try await session.releaseAcknowledgement()
        try await waitForSubmission(3, session)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: expected)
        XCTAssertEqual(view.string, expected)
        XCTAssertEqual(view.selectedRange(), redoneSelection)

        // Acknowledgements and refreshes must not erase the native undo group.
        XCTAssertTrue(undo.canUndo)
        undo.undo()
        XCTAssertEqual(view.string, original)
        try await waitForSubmission(4, session)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: original)
        XCTAssertTrue(undo.canRedo)
        undo.redo()
        XCTAssertEqual(view.string, expected)
        try await waitForSubmission(5, session)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: expected)
        XCTAssertEqual(view.string, expected)
    }

    @MainActor
    func testASCIIPrependBackspaceAndUndoPreserveNonemptyDocument() async throws {
        try await checkPrependAndUndo(original: "existing\n\nNext paragraph.\n", prefix: "A")
    }

    @MainActor
    func testUnicodePrependBackspaceAndUndoPreserveSurrogatesAndCombiningText() async throws {
        // Both emoji start with the same UTF-16 high surrogate. Diffing must
        // not split either pair or index before the fully shared old suffix.
        try await checkPrependAndUndo(original: "👩🏽‍💻 e\u{301}\n\n한글\n", prefix: "🙂")
    }

    @MainActor
    private func checkPrependAndUndo(original: String, prefix: String) async throws {
        let expected = prefix + original
        let session = GatedEditorSession(text: original)
        addTeardownBlock { await session.releaseAll() }
        let (adapter, window) = try await mount(session)
        defer { window.orderOut(nil) }
        let view = adapter.textView
        let undo = adapter.nativeUndoManager
        undo.groupsByEvent = false
        view.setSelectedRange(NSRange(location: 0, length: 0))
        undo.beginUndoGrouping()
        view.insertText(prefix, replacementRange: view.selectedRange())
        undo.endUndoGrouping()
        assertNative(view, expected, caret: prefix.utf16.count)
        try await waitForSubmission(1, session)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: expected)

        view.breakUndoCoalescing()
        undo.beginUndoGrouping()
        view.deleteBackward(nil)
        undo.endUndoGrouping()
        assertNative(view, original, caret: 0)
        try await waitForSubmission(2, session)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: original)

        view.breakUndoCoalescing()
        XCTAssertTrue(undo.canUndo)
        undo.undo()
        // Undoing a deletion may select the restored character; require its
        // contents without imposing a non-native selection policy on undo.
        XCTAssertEqual(view.string, expected)
        try await waitForSubmission(3, session)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: expected)
        XCTAssertEqual(view.string, expected)
        XCTAssertTrue(undo.canUndo)
        undo.undo()
        XCTAssertEqual(view.string, original)
        try await waitForSubmission(4, session)
        try await finishSubmission(adapter, session)
        await assertSession(session, equals: original)
        XCTAssertEqual(view.string, original)
    }

    @MainActor
    private func mount(_ session: GatedEditorSession) async throws -> (EditorMacAdapter, NSWindow) {
        _ = NSApplication.shared
        let adapter = try await EditorMacAdapter.make(session: session)
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 640, height: 300))
        let view = adapter.textView
        view.isAutomaticTextReplacementEnabled = false
        view.isAutomaticSpellingCorrectionEnabled = false
        view.isAutomaticQuoteSubstitutionEnabled = false
        view.isAutomaticDashSubstitutionEnabled = false
        view.isVerticallyResizable = true
        view.textContainer?.widthTracksTextView = true
        scroll.documentView = view
        let window = NSWindow(contentRect: scroll.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = scroll
        window.makeFirstResponder(view)
        return (adapter, window)
    }

    @MainActor
    private func assertNative(_ view: NSTextView, _ text: String, caret: Int,
                              file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(view.string, text, file: file, line: line)
        XCTAssertEqual(view.textStorage?.string, text, file: file, line: line)
        XCTAssertEqual(view.selectedRange(), NSRange(location: caret, length: 0), file: file, line: line)
    }

    @MainActor
    private func assertSession(_ session: GatedEditorSession, equals text: String,
                               file: StaticString = #filePath, line: UInt = #line) async {
        let actual = await session.currentText
        XCTAssertEqual(actual, text, file: file, line: line)
    }

    @MainActor
    private func waitForSubmission(_ count: Int, _ session: GatedEditorSession) async throws {
        try await waitUntil("submission \(count) is held") { await session.submissionCount >= count }
    }

    /// Request a refresh while the final acknowledgement is held. Its deferred
    /// snapshot read is a barrier proving that the adapter has consumed that
    /// acknowledgement, rather than merely that the session has returned it.
    @MainActor
    private func finishSubmission(_ adapter: EditorMacAdapter, _ session: GatedEditorSession) async throws {
        await adapter.refreshFromSession()
        let reads = await session.snapshotReadCount
        try await session.releaseAcknowledgement()
        try await waitUntil("the final acknowledgement has drained") {
            await session.snapshotReadCount > reads
        }
        await adapter.refreshFromSession()
    }

    @MainActor
    private func waitUntil(_ message: String, _ predicate: () async -> Bool,
                           file: StaticString = #filePath, line: UInt = #line) async throws {
        let deadline = ContinuousClock.now + .seconds(3)
        while ContinuousClock.now < deadline {
            if await predicate() { return }
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTFail("Timed out waiting for \(message)", file: file, line: line)
        throw GatedEditorSession.Failure.timedOut
    }
}

/// Only transport timing is artificial. Revisions and UTF-16 replacements have
/// the same semantics as DocumentSessionPort; no text is derived from the view.
private actor GatedEditorSession: DocumentSessionPort {
    enum Failure: Error { case timedOut, noAcknowledgement, noSnapshot, invalidRange }
    private var current: DocumentSnapshot
    private var acknowledgements: [(CheckedContinuation<DocumentMutationResult, Never>, DocumentMutationResult)] = []
    private var heldSnapshots: [(CheckedContinuation<DocumentSnapshot, Never>, DocumentSnapshot)] = []
    private var shouldHoldSnapshot = false
    private var isHolding = true
    private(set) var submissionCount = 0
    private(set) var snapshotReadCount = 0
    private(set) var rejectionCount = 0
    private(set) var savedText: String?
    var currentText: String { current.text }
    var hasHeldSnapshot: Bool { !heldSnapshots.isEmpty }

    init(text: String) { current = DocumentSnapshot(revision: 0, text: text) }

    func snapshot() async -> DocumentSnapshot {
        snapshotReadCount += 1
        let captured = current
        if shouldHoldSnapshot {
            shouldHoldSnapshot = false
            return await withCheckedContinuation { heldSnapshots.append(($0, captured)) }
        }
        return captured
    }

    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult {
        let result: DocumentMutationResult
        if mutation.baseRevision != current.revision {
            rejectionCount += 1
            result = .rejected(current: current)
        } else {
            let range = NSRange(location: mutation.range.location, length: mutation.range.length)
            let source = current.text as NSString
            guard range.location >= 0, range.length >= 0, range.location <= source.length,
                  range.length <= source.length - range.location else { throw Failure.invalidRange }
            current = DocumentSnapshot(revision: current.revision + 1,
                text: source.replacingCharacters(in: range, with: mutation.replacement))
            result = .applied(current)
        }
        submissionCount += 1
        guard isHolding else { return result }
        return await withCheckedContinuation { acknowledgements.append(($0, result)) }
    }

    func holdNextSnapshot() { shouldHoldSnapshot = true }

    func releaseSnapshot() throws {
        guard !heldSnapshots.isEmpty else { throw Failure.noSnapshot }
        let (continuation, captured) = heldSnapshots.removeFirst()
        continuation.resume(returning: captured)
    }

    func releaseAcknowledgement() throws {
        guard !acknowledgements.isEmpty else { throw Failure.noAcknowledgement }
        let (continuation, result) = acknowledgements.removeFirst()
        continuation.resume(returning: result)
    }

    func commitSave() {
        savedText = current.text
        current = DocumentSnapshot(revision: current.revision + 1, text: current.text)
    }

    func releaseAll() {
        isHolding = false
        shouldHoldSnapshot = false
        let pending = acknowledgements
        acknowledgements.removeAll()
        for (continuation, result) in pending { continuation.resume(returning: result) }
        let snapshots = heldSnapshots
        heldSnapshots.removeAll()
        for (continuation, captured) in snapshots { continuation.resume(returning: captured) }
    }
}
#endif
