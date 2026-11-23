import EditorMacAdapter
import Foundation
import XCTest

final class LineCommentAndFoldRangeTests: XCTestCase {
    func testLineCommentUsesTheTextItIsGiven() {
        let edit = EditorMacAdapter.lineCommentEdit(
            text: "keep me",
            selection: NSRange(location: 0, length: 7)
        )
        XCTAssertEqual(edit.replacement, "%keep me")
        XCTAssertEqual(edit.range, NSRange(location: 0, length: 7))
    }

    func testLineCommentClampsASelectionPastTheBuffer() {
        let text = "keep me"
        let edit = EditorMacAdapter.lineCommentEdit(
            text: text,
            selection: NSRange(location: 40, length: 10)
        )
        XCTAssertEqual(edit.replacement, "%keep me")
        let limit = (text as NSString).length
        XCTAssertLessThanOrEqual(edit.range.location, limit)
        XCTAssertLessThanOrEqual(edit.range.location + edit.range.length, limit)
    }

    func testLineCommentRoundTripLeavesTheOriginalLines() {
        let commented = EditorMacAdapter.lineCommentEdit(
            text: "alpha\nbeta",
            selection: NSRange(location: 0, length: 10)
        )
        XCTAssertEqual(commented.replacement, "%alpha\n%beta")
        let restored = EditorMacAdapter.lineCommentEdit(
            text: commented.replacement,
            selection: NSRange(location: 0, length: (commented.replacement as NSString).length)
        )
        XCTAssertEqual(restored.replacement, "alpha\nbeta")
    }

    func testTabIndentsEverySelectedLineAndKeepsTheBlockSelected() {
        let text = "\\begin{itemize}\n\\item a\n  \\item b\n\n\\end{itemize}\nrest"
        // From inside line 2 to inside line 5.
        let edit = EditorMacAdapter.lineIndentEdit(text: text, selection: NSRange(location: 18, length: 20), outdent: false)
        let result = (text as NSString).replacingCharacters(in: edit.range, with: edit.replacement)
        XCTAssertEqual(result, "\\begin{itemize}\n\t\\item a\n\t  \\item b\n\n\t\\end{itemize}\nrest")
        XCTAssertEqual((result as NSString).substring(with: edit.selection), "\t\\item a\n\t  \\item b\n\n\t\\end{itemize}\n")
    }

    func testShiftTabUndoesTabAndKeepsRelativeIndentation() {
        let text = "a\n  b\nc"
        let all = NSRange(location: 0, length: (text as NSString).length)
        let indented = EditorMacAdapter.lineIndentEdit(text: text, selection: all, outdent: false)
        XCTAssertEqual(indented.replacement, "\ta\n\t  b\n\tc")
        let outdented = EditorMacAdapter.lineIndentEdit(text: indented.replacement, selection: indented.selection, outdent: true)
        XCTAssertEqual(outdented.replacement, text)
        XCTAssertEqual(outdented.selection, all)
    }

    func testShiftTabRemovesUpToFourSpacesWhenThereIsNoTab() {
        let text = "      x\n  y\nz"
        let edit = EditorMacAdapter.lineIndentEdit(text: text, selection: NSRange(location: 0, length: 12), outdent: true)
        XCTAssertEqual(edit.replacement, "  x\ny\n")
    }

    func testShiftTabOnACaretOutdentsItsLineAndKeepsTheCaretOnTheText() {
        let text = "a\n\t\tbody\nz"
        let edit = EditorMacAdapter.lineIndentEdit(text: text, selection: NSRange(location: 6, length: 0), outdent: true)
        let result = (text as NSString).replacingCharacters(in: edit.range, with: edit.replacement)
        XCTAssertEqual(result, "a\n\tbody\nz")
        XCTAssertEqual(edit.selection, NSRange(location: 5, length: 0))
    }

    func testFoldRangePastTheBufferIsSkipped() {
        XCTAssertNil(EditorMacAdapter.clampedCharacterRange(start: 100, end: 120, limit: 10))
    }

    func testFoldRangeIsClampedToTheBuffer() {
        XCTAssertEqual(
            EditorMacAdapter.clampedCharacterRange(start: 2, end: 50, limit: 10),
            NSRange(location: 2, length: 8)
        )
    }

    func testEmptyFoldRangeIsSkipped() {
        XCTAssertNil(EditorMacAdapter.clampedCharacterRange(start: 4, end: 4, limit: 10))
    }
}
