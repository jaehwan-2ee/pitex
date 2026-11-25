import AppPorts
import XCTest

final class AgentTurnViewTests: XCTestCase {
    typealias Row = AgentTurnView.Row

    private let english: [String: String] = [
        "assistant.turn.worked": "Worked", "assistant.turn.worked_for": "Worked for %@",
        "assistant.turn.seconds": "%@s", "assistant.turn.minutes": "%1$@m %2$@s",
        "assistant.turn.edits.one": "1 edit", "assistant.turn.edits.other": "%@ edits",
        "assistant.turn.commands.one": "1 command", "assistant.turn.commands.other": "%@ commands",
        "assistant.turn.reads.one": "1 read", "assistant.turn.reads.other": "%@ reads",
        "assistant.turn.searches.one": "1 search", "assistant.turn.searches.other": "%@ searches",
        "assistant.turn.tools.one": "1 tool", "assistant.turn.tools.other": "%@ tools",
        "assistant.turn.thoughts.one": "1 thought", "assistant.turn.thoughts.other": "%@ thoughts",
    ]

    private func tool(_ id: String, _ kind: AgentToolDetail.Kind, path: String? = nil, parent: String? = nil) -> Row {
        Row(role: .tool, toolKind: kind, path: path, callID: id, parentCallID: parent)
    }

    func testFinishedTurnFoldsItsWorkAndKeepsTheFinalReply() {
        let rows = [
            Row(role: .user, workedSeconds: 72),
            Row(role: .thinking),
            Row(role: .assistant),
            tool("r1", .read, path: "a.tex"),
            tool("r2", .read, path: "a.tex"),
            tool("e1", .edit, path: "a.tex"),
            tool("e2", .write, path: "b.tex"),
            tool("s1", .shell),
            tool("g1", .search),
            Row(role: .notice),
            Row(role: .assistant),
        ]
        let items = AgentTurnView.items(rows, running: false)
        guard case let .summary(prompt, summary, folded) = items[1] else { return XCTFail("no summary: \(items)") }
        XCTAssertEqual(prompt, 0)
        XCTAssertEqual(folded, (1...8).map { .row(index: $0, children: []) })
        XCTAssertEqual(items[0], .row(index: 0, children: []))
        XCTAssertEqual(Array(items[2...]), [.row(index: 9, children: []), .row(index: 10, children: [])])
        XCTAssertEqual(summary.text { self.english[$0] ?? $0 }, "Worked for 1m 12s · 2 edits · 1 command · 1 read · 1 search · 1 thought")
    }

    func testRunningTurnAndTurnsWithoutWorkStayOpen() {
        let rows = [Row(role: .user), Row(role: .assistant), Row(role: .user), tool("s", .shell), Row(role: .assistant)]
        XCTAssertEqual(AgentTurnView.items(rows, running: true), rows.indices.map { .row(index: $0, children: []) })
        let done = AgentTurnView.items(rows, running: false)
        guard case let .summary(prompt, summary, _) = done[3] else { return XCTFail("\(done)") }
        XCTAssertEqual(prompt, 2)
        XCTAssertEqual(summary.text { self.english[$0] ?? $0 }, "Worked · 1 command")
        XCTAssertEqual(AgentTurnView.items(Array(rows[..<4]), running: false).count, 4, "no reply, no fold")
    }

    func testSubagentCallsNestUnderTheirCard() {
        let rows = [
            Row(role: .user, workedSeconds: 5),
            tool("a", .subAgent),
            tool("g", .search, parent: "a"),
            tool("inner", .subAgent, parent: "a"),
            tool("r", .read, path: "x", parent: "inner"),
            tool("r2", .read, path: "y"),
            Row(role: .assistant),
        ]
        let items = AgentTurnView.items(rows, running: false)
        guard case let .summary(_, summary, folded) = items[1] else { return XCTFail("\(items)") }
        XCTAssertEqual(folded, [.row(index: 1, children: [2, 3, 4]), .row(index: 5, children: [])])
        XCTAssertEqual([summary.tools, summary.reads, summary.searches], [1, 1, 0])
        XCTAssertEqual(AgentTurnView.items(Array(rows[..<3]), running: true)[1], .row(index: 1, children: [2]))
    }

    func testTurnCutsAndQuotes() {
        let roles: [Row.Role] = [.user, .assistant, .tool, .user, .assistant]
        XCTAssertEqual(AgentTurnView.turnCut(roles: roles, index: 0), 0)
        XCTAssertEqual(AgentTurnView.turnCut(roles: roles, index: 1), 1)
        XCTAssertNil(AgentTurnView.turnCut(roles: roles, index: 2))
        XCTAssertEqual(AgentTurnView.turnCut(roles: roles, index: 3, dropped: 2), 3)
        let quote = AgentTurnView.quote([(.user, "First"), (.assistant, "One\nTwo"), (.user, "Second")], keep: 1, header: "Earlier:", you: "You", assistant: "Assistant")
        XCTAssertEqual(quote, "Earlier:\n\n> **You:**\n> First\n>\n> **Assistant:**\n> One\n> Two")
        XCTAssertEqual(AgentTurnView.duration(0) { self.english[$0] ?? $0 }, "<1s")
        XCTAssertEqual(AgentTurnView.duration(125) { self.english[$0] ?? $0 }, "2m 5s")
    }

    func testSubagentDetailDecodesItsTypeAndCount() {
        var card = AgentToolDetail(["kind": "sub_agent", "label": "Find", "description": "Find", "subAgentType": "Explore"])!
        card.merge(AgentToolDetail(["kind": "sub_agent", "toolUses": 2])!)
        XCTAssertEqual(card.subAgentType, "Explore")
        XCTAssertEqual(card.toolUses, 2)
        XCTAssertEqual(card.badge { _ in "" }, "Explore")
    }
}
