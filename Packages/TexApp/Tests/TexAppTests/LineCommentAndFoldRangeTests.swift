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
