import AppPorts
import XCTest

/// The history row model — the same cases as the Linux `agent.rs` tests.
final class AgentSessionHistoryTests: XCTestCase {
    func testTheBadgeComesFromTheFilePrefix() {
        XCTAssertEqual(AgentSessionHistory.source(fileName: "claude-0b1c.jsonl"), "claude")
        XCTAssertEqual(AgentSessionHistory.source(fileName: "agy-x.jsonl"), "agy")
        XCTAssertEqual(AgentSessionHistory.source(fileName: "codex-x.jsonl"), "codex")
        XCTAssertEqual(AgentSessionHistory.source(fileName: "grok-x.jsonl"), "grok")
        XCTAssertEqual(AgentSessionHistory.source(fileName: "2026-11-20T10-00-00_abc.jsonl"), "pi")
        XCTAssertEqual(AgentSessionHistory.source(fileName: "claudette.jsonl"), "pi")
        XCTAssertEqual(AgentSessionHistory.sourceLabel("claude"), "Claude Code")
        XCTAssertEqual(AgentSessionHistory.sourceLabel("agy"), "Antigravity")
        XCTAssertEqual(AgentSessionHistory.sourceLabel("pi"), "pi")
    }

    func testRelativeAge() {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        func ago(_ seconds: TimeInterval) -> AgentSessionHistory.Age {
            AgentSessionHistory.age(of: now.addingTimeInterval(-seconds), now: now)
        }
        XCTAssertEqual(ago(20), .justNow)
        XCTAssertEqual(ago(-9), .justNow, "a clock a little ahead is still now")
        XCTAssertEqual(ago(5 * 60), .minutes(5))
        XCTAssertEqual(ago(3 * 3_600 + 59), .hours(3))
        XCTAssertEqual(ago(30 * 3_600), .yesterday)
        XCTAssertEqual(ago(4 * 86_400), .days(4))
        XCTAssertEqual(ago(40 * 86_400), .date)
    }

    func testSearchMatchesEveryWordIgnoringCase() {
        XCTAssertTrue(AgentSessionHistory.matches(title: "Fix the Bibliography", query: "bib fix"))
        XCTAssertTrue(AgentSessionHistory.matches(title: "anything", query: "  "))
        XCTAssertFalse(AgentSessionHistory.matches(title: "Fix the intro", query: "bib"))
        XCTAssertTrue(AgentSessionHistory.matches(title: "한글 질문", query: "질문"))
    }
}
