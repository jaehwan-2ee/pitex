import AppPorts
import XCTest

#if os(macOS)
import AppKit
import EditorMacAdapter

final class EditorMacAdapterLayoutTests: XCTestCase {
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
