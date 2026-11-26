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

final class ConflictCloseTests: XCTestCase {
    func testNoConflictLeavesTheUsualCloseFlow() {
        XCTAssertEqual(conflictCloseAction(saveStates: [.clean, .dirty], choice: nil), .proceed)
    }

    // Local and SSH projects share this decision: a conflicted file refuses every save,
    // so quit and close must ask which side wins instead of attempting the save.
    func testConflictedSessionAsksWhichVersionWins() {
        XCTAssertEqual(conflictCloseAction(saveStates: [.dirty, .conflicted], choice: nil), .stayOpen)
        XCTAssertEqual(conflictCloseAction(saveStates: [.conflicted], choice: .cancel), .stayOpen)
        XCTAssertEqual(conflictCloseAction(saveStates: [.conflicted], choice: .save), .keepMine)
        XCTAssertEqual(conflictCloseAction(saveStates: [.conflicted], choice: .discard), .useExternal)
    }
}
