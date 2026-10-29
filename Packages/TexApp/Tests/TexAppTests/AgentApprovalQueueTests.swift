import AppPorts
import XCTest

/// The approval policy `AgentCoordinator` uses: show the oldest request,
/// advance when it is answered, and deny every queued request on stop.
final class AgentApprovalQueueTests: XCTestCase {
    func testAgentCoordinatorShowsTheOldestApprovalUntilItIsAnswered() {
        var queue = AgentApprovalQueue()
        queue.enqueue(item("1", detail: "first"))
        queue.enqueue(item("2", detail: "second"))

        XCTAssertEqual(queue.count, 2)
        XCTAssertEqual(queue.head?.id, "1")
        XCTAssertEqual(queue.head?.detail, "first")

        XCTAssertEqual(queue.dequeueHead()?.id, "1")
        XCTAssertEqual(queue.head?.id, "2")
        XCTAssertEqual(queue.head?.detail, "second")

        XCTAssertEqual(queue.dequeueHead()?.id, "2")
        XCTAssertNil(queue.head)
        XCTAssertNil(queue.dequeueHead())
    }

    func testRepeatedApprovalIdStaysOneSlotAndKeepsItsPlace() {
        var queue = AgentApprovalQueue()
        queue.enqueue(item("1", detail: "old"))
        queue.enqueue(item("2", detail: "other"))
        queue.enqueue(item("1", detail: "updated"))

        XCTAssertEqual(queue.count, 2)
        XCTAssertEqual(queue.head?.id, "1")
        XCTAssertEqual(queue.head?.detail, "updated")
        XCTAssertEqual(queue.dequeueHead()?.id, "1")
        XCTAssertEqual(queue.head?.id, "2")
    }

    func testStopDrainsEveryQueuedApprovalInOrder() {
        var queue = AgentApprovalQueue()
        queue.enqueue(item("7", detail: "edit"))
        queue.enqueue(item("8", detail: "shell"))

        XCTAssertEqual(queue.drain().map(\.id), ["7", "8"])
        XCTAssertNil(queue.head)
        XCTAssertEqual(queue.count, 0)
        XCTAssertTrue(queue.drain().isEmpty)
    }

    func testClearDropsTheQueueWithoutProducingAnswers() {
        var queue = AgentApprovalQueue()
        queue.enqueue(item("9", detail: "gone"))
        queue.clear()
        XCTAssertNil(queue.head)
        XCTAssertTrue(queue.drain().isEmpty)
    }

    private func item(_ id: String, detail: String) -> AgentApprovalItem {
        AgentApprovalItem(id: id, kind: "command", title: "Run command", detail: detail)
    }
}
