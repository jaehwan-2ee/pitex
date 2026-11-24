import AppPorts
import XCTest

/// `todo_update` decode, the snapshot diff into activity rows, and the checklist
/// summary — the same cases as the Linux `agent.rs` tests.
final class AgentTodoListTests: XCTestCase {
    private func item(_ text: String, _ status: AgentTodoItem.Status, id: String? = nil) -> AgentTodoItem {
        AgentTodoItem(id: id, text: text, status: status)
    }

    func testDecodeKeepsRowsWithTextAndReadsStatusAndActiveForm() {
        let items = AgentTodoItem.list([
            ["text": "Build the PDF", "status": "in_progress", "activeForm": "Building the PDF"],
            ["id": "7", "text": "Fix the citation", "status": "completed"],
            ["text": "  ", "status": "pending"],
            ["text": "Rebuild", "status": "bogus"],
            "not a row",
        ])
        XCTAssertEqual(items, [
            AgentTodoItem(text: "Build the PDF", status: .inProgress, activeForm: "Building the PDF"),
            AgentTodoItem(id: "7", text: "Fix the citation", status: .completed),
            AgentTodoItem(text: "Rebuild", status: .pending),
        ])
        XCTAssertEqual(AgentTodoItem.list(nil), [])
    }

    func testSnapshotDiffsBecomeActivityRows() {
        let pending = [item("Build the PDF", .pending), item("Fix the citation", .pending), item("Rebuild", .pending)]
        XCTAssertEqual(AgentTodoList.activities(from: [], to: pending), [.created(3)])
        XCTAssertEqual(AgentTodoList.activities(from: [], to: []), [])
        let started = [item("Build the PDF", .inProgress), item("Fix the citation", .pending), item("Rebuild", .pending)]
        XCTAssertEqual(AgentTodoList.activities(from: pending, to: started), [.started("Build the PDF")])
        let twoDone = [item("Build the PDF", .completed), item("Fix the citation", .completed), item("Rebuild", .inProgress)]
        XCTAssertEqual(
            AgentTodoList.activities(from: started, to: twoDone),
            [.completed("Build the PDF"), .completed("Fix the citation"), .started("Rebuild")]
        )
        XCTAssertEqual(AgentTodoList.activities(from: twoDone, to: twoDone + [item("Check the log", .pending)]), [.added("Check the log")])
        XCTAssertEqual(AgentTodoList.activities(from: started, to: pending), [], "back to pending is not news")
        XCTAssertEqual(AgentTodoList.activities(from: twoDone, to: twoDone), [])
        let a = item("A", .pending, id: "1")
        let b = item("B", .pending, id: "2")
        XCTAssertEqual(AgentTodoList.activities(from: [a, b], to: [item("B", .completed, id: "2"), a]), [.completed("B")])
        // Rows without ids match by text, so dropping one does not shift the rest.
        XCTAssertEqual(
            AgentTodoList.activities(
                from: [item("Build the PDF", .pending), item("Fix the citation", .inProgress), item("Rebuild", .pending)],
                to: [item("Fix the citation", .completed), item("Rebuild", .inProgress)]
            ),
            [.completed("Fix the citation"), .started("Rebuild")]
        )
        XCTAssertEqual(
            AgentTodoList.activities(from: [item("Rebuild", .pending)], to: [item("Write the abstract", .inProgress)]),
            [.created(1)],
            "a list that shares no row with the old one is a new list"
        )
        XCTAssertEqual(
            AgentTodoList.activities(from: [item("Rebuild", .pending), item("Rebuild", .pending)], to: [item("Rebuild", .completed), item("Rebuild", .pending)]),
            [.completed("Rebuild")],
            "duplicate texts pair in order"
        )
        XCTAssertEqual(AgentTodoActivity.started("Build the PDF").key, "assistant.todo.started")
        XCTAssertEqual(AgentTodoActivity.created(3).argument, "3")
    }

    func testProgressCountsDoneRowsAndNamesTheCurrentStep() {
        let items = [
            item("Build the PDF", .completed),
            item("Fix the citation", .completed),
            AgentTodoItem(text: "Rebuild", status: .inProgress, activeForm: "Rebuilding"),
            item("Check the log", .pending),
            item("Commit", .pending),
        ]
        XCTAssertEqual(AgentTodoList.progress(items).done, 2)
        XCTAssertEqual(AgentTodoList.progress(items).total, 5)
        XCTAssertEqual(AgentTodoList.current(items), "Rebuilding")
        XCTAssertEqual(AgentTodoList.current([item("Fix", .inProgress)]), "Fix")
        XCTAssertNil(AgentTodoList.current(Array(items.prefix(2))))
    }
}
