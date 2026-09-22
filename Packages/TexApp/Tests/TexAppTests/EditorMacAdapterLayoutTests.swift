import AppPorts
import XCTest

#if os(macOS)
import AppKit
import Carbon
import EditorMacAdapter

final class EditorMacAdapterLayoutTests: XCTestCase {
    @MainActor
    func testNativeFindCounterAndNavigation() async throws {
        _ = NSApplication.shared
        let policy = NSApp.activationPolicy()
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        defer { NSApp.setActivationPolicy(policy) }
        let pasteboard = NSPasteboard(name: .find)
        let saved = (pasteboard.types ?? []).compactMap { type in pasteboard.data(forType: type).map { (type, $0) } }
        pasteboard.clearContents()
        defer {
            pasteboard.clearContents()
            for (type, data) in saved { pasteboard.setData(data, forType: type) }
        }
        let session = EditableDocumentSession(text: "한 😀 cat CAT cat")
        let adapter = try await EditorMacAdapter.make(session: session)
        let view = adapter.textView
        view.setSelectedRange(NSRange(location: 0, length: 0))
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 700, height: 300))
        scroll.documentView = view
        let window = NSWindow(contentRect: scroll.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = scroll
        window.makeKeyAndOrderFront(nil)
        defer { window.orderOut(nil) }
        func fields(_ root: NSView) -> [NSTextField] {
            (root as? NSTextField).map { [$0] } ?? root.subviews.flatMap { fields($0) }
        }
        let item = NSMenuItem(title: "", action: nil, keyEquivalent: "")
        item.tag = NSTextFinder.Action.showFindInterface.rawValue
        view.performFindPanelAction(item)
        try await Task.sleep(for: .milliseconds(100))
        let bar = try XCTUnwrap(scroll.findBarView)
        let query = try XCTUnwrap(fields(bar).first { $0.isEditable && !$0.isHiddenOrHasHiddenAncestor })
        let count = try XCTUnwrap(fields(bar).first { $0.accessibilityIdentifier() == "pitex.search.count" })
        query.selectText(nil)
        let fieldEditor = try XCTUnwrap(query.currentEditor() as? NSTextView)
        fieldEditor.insertText("cat", replacementRange: NSRange(location: 0, length: fieldEditor.string.utf16.count))
        for _ in 0..<100 {
            if count.stringValue == "1 / 3" { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        XCTAssertEqual(count.stringValue, "1 / 3")
        for expected in ["2 / 3", "3 / 3", "1 / 3"] {
            item.tag = NSTextFinder.Action.nextMatch.rawValue
            view.performTextFinderAction(item)
            try await Task.sleep(for: .milliseconds(200))
            XCTAssertEqual(count.stringValue, expected)
        }
        item.tag = NSTextFinder.Action.previousMatch.rawValue
        view.performTextFinderAction(item)
        try await Task.sleep(for: .milliseconds(200))
        XCTAssertEqual(count.stringValue, "3 / 3")
        item.tag = NSTextFinder.Action.selectAll.rawValue
        view.performTextFinderAction(item)
        try await Task.sleep(for: .milliseconds(100))
        XCTAssertEqual(view.selectedRanges.count, 3)
        item.tag = NSTextFinder.Action.nextMatch.rawValue
        view.performTextFinderAction(item)
        try await Task.sleep(for: .milliseconds(100))
        XCTAssertEqual(view.selectedRanges.count, 1)
        view.insertText(" cat", replacementRange: NSRange(location: view.string.utf16.count, length: 0))
        for _ in 0..<100 {
            if count.stringValue.hasSuffix("/ 4") { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        XCTAssertTrue(count.stringValue.hasSuffix("/ 4"), "count=\(count.stringValue), text=\(view.string)")
        item.tag = NSTextFinder.Action.showReplaceInterface.rawValue
        view.performTextFinderAction(item)
        let replacement = try XCTUnwrap(fields(bar).first { $0.isEditable && $0 !== query })
        replacement.selectText(nil)
        let replacementEditor = try XCTUnwrap(replacement.currentEditor() as? NSTextView)
        replacementEditor.insertText("dog", replacementRange: NSRange(location: 0, length: replacementEditor.string.utf16.count))
        item.tag = NSTextFinder.Action.replaceAll.rawValue
        view.performTextFinderAction(item)
        for _ in 0..<100 {
            if count.stringValue == "0 / 0" { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        XCTAssertEqual(view.string, "한 😀 dog dog dog dog")
        XCTAssertEqual(count.stringValue, "0 / 0")
        for _ in 0..<100 {
            if await session.snapshot().text == view.string { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        let snapshot = await session.snapshot()
        XCTAssertEqual(snapshot.text, view.string)
        item.tag = NSTextFinder.Action.hideFindInterface.rawValue
        view.performTextFinderAction(item)
        XCTAssertFalse(scroll.isFindBarVisible)
        item.tag = NSTextFinder.Action.showFindInterface.rawValue
        view.performTextFinderAction(item)
        XCTAssertTrue(scroll.isFindBarVisible)
    }

    @MainActor
    func testNativeDigitsAndDeletionWithCapsLock() async throws {
        _ = NSApplication.shared
        let rows = [kVK_ANSI_0, kVK_ANSI_1, kVK_ANSI_2, kVK_ANSI_3, kVK_ANSI_4,
                    kVK_ANSI_5, kVK_ANSI_6, kVK_ANSI_7, kVK_ANSI_8, kVK_ANSI_9]
        let keypad = [kVK_ANSI_Keypad0, kVK_ANSI_Keypad1, kVK_ANSI_Keypad2, kVK_ANSI_Keypad3, kVK_ANSI_Keypad4,
                      kVK_ANSI_Keypad5, kVK_ANSI_Keypad6, kVK_ANSI_Keypad7, kVK_ANSI_Keypad8, kVK_ANSI_Keypad9]
        for caps in [false, true] {
            for pad in [false, true] {
                let session = EditableDocumentSession()
                let adapter = try await EditorMacAdapter.make(session: session)
                let view = adapter.textView
                let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 400, height: 240),
                                      styleMask: [.titled], backing: .buffered, defer: false)
                window.contentView = view
                window.makeFirstResponder(view)
                defer { window.orderOut(nil) }
                var flags: NSEvent.ModifierFlags = caps ? [.capsLock] : []
                if pad { flags.insert(.numericPad) }
                func key(_ code: Int, _ characters: String) {
                    let event = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags,
                        timestamp: 0, windowNumber: window.windowNumber, context: nil,
                        characters: characters, charactersIgnoringModifiers: characters,
                        isARepeat: false, keyCode: UInt16(code))!
                    view.keyDown(with: event)
                }
                adapter.nativeUndoManager.groupsByEvent = false
                adapter.nativeUndoManager.beginUndoGrouping()
                view.setSelectedRange(NSRange(location: 0, length: view.string.utf16.count))
                for (digit, code) in (pad ? keypad : rows).enumerated() { key(code, String(digit)) }
                XCTAssertEqual(view.string, "0123456789", "Caps Lock=\(caps), keypad=\(pad)")
                for _ in 0..<10 { key(kVK_Delete, "\u{7f}") }
                XCTAssertEqual(view.string, "", "Caps Lock=\(caps), keypad=\(pad)")
                adapter.nativeUndoManager.endUndoGrouping()
                for _ in 0..<100 {
                    if await session.snapshot().text == view.string { break }
                    try await Task.sleep(for: .milliseconds(10))
                }
                let snapshot = await session.snapshot()
                XCTAssertEqual(snapshot.text, view.string)
            }
        }
    }

    @MainActor
    func testPartialUnicodeEditsKeepNativeUndoAndQueuedTyping() async throws {
        _ = NSApplication.shared
        let session = EditableDocumentSession()
        let adapter = try await EditorMacAdapter.make(session: session)
        let view = adapter.textView
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 400, height: 240),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = view
        window.makeFirstResponder(view)
        defer { window.orderOut(nil) }
        adapter.nativeUndoManager.groupsByEvent = false
        adapter.nativeUndoManager.beginUndoGrouping()
        view.insertText("한", replacementRange: NSRange(location: 1, length: 2))
        adapter.nativeUndoManager.endUndoGrouping()
        for _ in 0..<100 {
            if await session.snapshot().text == "a한b" { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        let first = await session.mutations.first
        XCTAssertEqual(first?.range, DocumentTextRange(location: 1, length: 2))
        XCTAssertEqual(first?.replacement, "한")
        XCTAssertEqual(view.string, "a한b")
        adapter.nativeUndoManager.undo()
        for _ in 0..<100 {
            if await session.snapshot().text == "a👩b" { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        let undone = await session.snapshot()
        XCTAssertEqual(undone.text, "a👩b")
        XCTAssertEqual(view.string, undone.text)
        adapter.nativeUndoManager.redo()
        adapter.nativeUndoManager.beginUndoGrouping()
        view.insertText("글", replacementRange: NSRange(location: 2, length: 0))
        view.insertText("!", replacementRange: NSRange(location: 3, length: 0))
        adapter.nativeUndoManager.endUndoGrouping()
        for _ in 0..<100 {
            if await session.snapshot().text == "a한글!b" { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        let final = await session.snapshot()
        XCTAssertEqual(final.text, "a한글!b")
        XCTAssertEqual(view.string, final.text)
    }

    @MainActor
    func testEditorTextLaysOutOnceHostedInAScrollView() async throws {
        let adapter = try await EditorMacAdapter.make(session: LayoutDocumentSession())
        let textView = adapter.textView
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = true
        textView.textContainer?.widthTracksTextView = true

        let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 400, height: 200))
        scrollView.documentView = textView
        scrollView.layoutSubtreeIfNeeded()

        XCTAssertEqual(textView.string, "\\documentclass{article}\n")
        let containerWidth = try XCTUnwrap(textView.textContainer?.size.width)
        XCTAssertGreaterThan(containerWidth, 0, "Text container never sized, so no text is ever drawn")
    }

    @MainActor
    func testRevealScrollsAndFocusesIncludingBeforeDocumentIsMounted() async throws {
        _ = NSApplication.shared
        let source = String(repeating: "Earlier line\n", count: 100) + "Target line\n"
        let target = (source as NSString).range(of: "Target line")
        for revealBeforeMount in [false, true] {
            let adapter = try await EditorMacAdapter.make(session: LayoutDocumentSession(text: source))
            let textView = adapter.textView
            textView.isVerticallyResizable = true
            textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
            textView.autoresizingMask = [.width]
            textView.textContainer?.widthTracksTextView = true

            if revealBeforeMount { adapter.revealSelection(NSRange(location: target.location, length: 0)) }
            let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 400, height: 140))
            scroll.documentView = textView
            let window = NSWindow(contentRect: scroll.frame, styleMask: [.titled], backing: .buffered, defer: false)
            window.contentView = scroll
            defer { window.orderOut(nil) }
            window.contentView?.layoutSubtreeIfNeeded()
            if !revealBeforeMount { adapter.revealSelection(NSRange(location: target.location, length: 0)) }

            XCTAssertEqual(adapter.selectedRange, NSRange(location: target.location, length: 0))
            XCTAssertTrue(window.firstResponder === textView, "Typing must go to the source editor")
            XCTAssertGreaterThan(scroll.contentView.bounds.minY, 0, "Off-screen source must be revealed")
            let layout = try XCTUnwrap(textView.layoutManager)
            let container = try XCTUnwrap(textView.textContainer)
            let glyphs = layout.glyphRange(forCharacterRange: target, actualCharacterRange: nil)
            let rect = layout.boundingRect(forGlyphRange: glyphs, in: container)
                .offsetBy(dx: textView.textContainerOrigin.x, dy: textView.textContainerOrigin.y)
            XCTAssertTrue(textView.visibleRect.intersects(rect))

            adapter.revealSelection(NSRange(location: source.utf16.count + 10, length: 20))
            XCTAssertEqual(adapter.selectedRange, NSRange(location: source.utf16.count, length: 0))
        }
    }
}

private actor EditableDocumentSession: DocumentSessionPort {
    private var current: DocumentSnapshot
    init(text: String = "a👩b") { current = DocumentSnapshot(revision: 0, text: text) }
    var mutations: [DocumentMutation] = []
    func snapshot() async -> DocumentSnapshot { current }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult {
        guard mutation.baseRevision == current.revision else { return .rejected(current: current) }
        mutations.append(mutation)
        let text = (current.text as NSString).replacingCharacters(
            in: NSRange(location: mutation.range.location, length: mutation.range.length),
            with: mutation.replacement)
        current = DocumentSnapshot(revision: current.revision + 1, text: text)
        return .applied(current)
    }
}

private actor LayoutDocumentSession: DocumentSessionPort {
    let text: String

    init(text: String = "\\documentclass{article}\n") { self.text = text }

    func snapshot() async -> DocumentSnapshot {
        DocumentSnapshot(revision: 0, text: text)
    }

    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult {
        .rejected(current: await snapshot())
    }
}
#endif
