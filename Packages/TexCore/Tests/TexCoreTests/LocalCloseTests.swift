import DocumentSessionCore
import XCTest

final class LocalCloseTests: XCTestCase {
    func testCleanSessionsCloseWithoutAPrompt() {
        let action = localCloseAction(saveStates: [.clean, .clean], choice: nil)
        XCTAssertEqual(action, .closeWithoutSaving)
    }

    func testDirtySessionStaysOpenUntilTheUserChooses() {
        XCTAssertEqual(
            localCloseAction(saveStates: [.dirty], choice: nil),
            .stayOpen
        )
        XCTAssertEqual(
            localCloseAction(saveStates: [.dirty], choice: .cancel),
            .stayOpen
        )
        XCTAssertEqual(
            localCloseAction(saveStates: [.clean, .dirty], choice: .discard),
            .closeWithoutSaving
        )
        XCTAssertEqual(
            localCloseAction(saveStates: [.dirty], choice: .save),
            .saveThenClose
        )
    }

    func testConflictedSessionIsNotTreatedAsClean() {
        XCTAssertEqual(
            localCloseAction(saveStates: [.conflicted], choice: .cancel),
            .stayOpen
        )
        XCTAssertEqual(
            localCloseAction(saveStates: [.conflicted], choice: .save),
            .saveThenClose
        )
    }
}
