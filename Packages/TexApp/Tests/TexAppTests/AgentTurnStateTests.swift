import AppPorts
import Foundation
import XCTest

/// Usage, meter, thinking, scroll pinning, retry and text segmentation — the same cases
/// as the Linux `turn_state.rs` and agent tests.
final class AgentTurnStateTests: XCTestCase {
    func testMeterClampsAndBandsAt70And90() throws {
        let meter = try XCTUnwrap(AgentContextMeter(used: 76_000, max: 200_000))
        XCTAssertEqual(meter.label, "38% · 76k/200k")
        XCTAssertEqual(meter.band, .normal)
        XCTAssertEqual(AgentContextMeter(used: 139_999, max: 200_000)?.band, .normal)
        XCTAssertEqual(AgentContextMeter(used: 140_000, max: 200_000)?.band, .warning)
        XCTAssertEqual(AgentContextMeter(used: 180_000, max: 200_000)?.band, .warning)
        XCTAssertEqual(AgentContextMeter(used: 180_001, max: 200_000)?.band, .danger)
        XCTAssertEqual(AgentContextMeter(used: 300_000, max: 200_000)?.percent, 100)
        XCTAssertNil(AgentContextMeter(used: 5, max: 0))
        XCTAssertEqual(AgentContextMeter.compactTokens(950), "950")
        XCTAssertEqual(AgentContextMeter.compactTokens(7_600), "7.6k")
        XCTAssertEqual(AgentContextMeter.compactTokens(1_000_000), "1M")
        XCTAssertEqual(AgentContextMeter.compactTokens(1_250_000), "1.2M")
    }

    func testUsageMergeKeepsFieldsAnUpdateOmits() {
        var usage = AgentUsage()
        usage.merge(["inputTokens": 10, "contextUsed": 100, "contextMax": 1000])
        usage.merge(["outputTokens": 5, "costUsd": 0.25, "contextUsed": 120, "turnId": "t"])
        XCTAssertEqual(usage.inputTokens, 10)
        XCTAssertEqual(usage.outputTokens, 5)
        XCTAssertEqual(usage.contextUsed, 120)
        XCTAssertEqual(usage.contextMax, 1000)
        XCTAssertEqual(usage.costUsd, 0.25)
        usage.merge(["contextUsed": -1])
        XCTAssertEqual(usage.contextUsed, 120)
        usage.merge(["contextMax": 1e20])
        XCTAssertEqual(usage.contextMax, Int.max, "a huge count saturates instead of trapping")
    }

    func testFailedTurnsAndTheirLines() {
        let failed = AgentTurnError.failure(
            agentEnd: ["outcome": "failed", "code": "rate_limited", "error": "API Error: 429"], fallback: nil
        )
        XCTAssertEqual(failed?.code, .rateLimited)
        XCTAssertEqual(failed?.text, "API Error: 429")
        XCTAssertNil(AgentTurnError.failure(agentEnd: ["outcome": "canceled", "error": "stop"], fallback: nil))
        XCTAssertNil(AgentTurnError.failure(agentEnd: ["outcome": "completed"], fallback: nil))
        // pi sends no outcome: an assistant error still fails the turn, as `other`.
        XCTAssertEqual(AgentTurnError.failure(agentEnd: [:], fallback: "boom")?.code, .other)
        XCTAssertNil(AgentTurnError.failure(agentEnd: [:], fallback: nil))
        XCTAssertEqual(AgentErrorCode(code: "crashed"), .other, "only the host reports a crash")
        XCTAssertEqual(AgentErrorCode.contextFull.messageKey(canCompact: true), "assistant.error.context_full_compact")
        XCTAssertEqual(AgentErrorCode.contextFull.messageKey(canCompact: false), "assistant.error.context_full")
        XCTAssertEqual(AgentErrorCode.cliName(provider: "claude"), "Claude Code")
        XCTAssertEqual(AgentErrorCode.cliName(provider: "apple-fm"), "Apple Foundation Models")
    }

    func testRetryPayloadKeepsTheImages() throws {
        let image = try AgentImage(name: "a.png", mimeType: "image/png", data: Data([1, 2, 3]), attached: 0)
        let payload = AgentRetryPayload(text: "fix the bib", images: [image])
        XCTAssertEqual(payload, AgentRetryPayload(text: "fix the bib", images: [image]))
        XCTAssertNotEqual(payload, AgentRetryPayload(text: "fix the bib", images: []))
    }

    func testThinkingIsOpenWhileStreamingThenClosedUnlessToggled() {
        XCTAssertTrue(AgentThinking.isExpanded(streaming: true, userChoice: nil))
        XCTAssertFalse(AgentThinking.isExpanded(streaming: false, userChoice: nil))
        XCTAssertTrue(AgentThinking.isExpanded(streaming: false, userChoice: true))
        XCTAssertFalse(AgentThinking.isExpanded(streaming: true, userChoice: false))
    }

    func testTextToolTextKeepsEachStreamedSegment() {
        // message_end carried "First.Second." for a turn streamed as two rows.
        XCTAssertEqual(AgentTranscriptText.finalText(streamed: "Second.", messageEnd: "First.Second."), "Second.")
        XCTAssertEqual(AgentTranscriptText.finalText(streamed: "", messageEnd: "Only end"), "Only end")
    }

    func testScrollPinFollowsAtTheBottomAndOffersJumpWhenScrolledUp() {
        var pin = AgentScrollPin()
        XCTAssertTrue(pin.contentAdded())
        pin.scrolled(distanceFromBottom: 40)
        XCTAssertTrue(pin.pinned)
        pin.scrolled(distanceFromBottom: 300)
        XCTAssertFalse(pin.pinned)
        XCTAssertFalse(pin.contentAdded(), "scrolled up: new content does not move the view")
        XCTAssertTrue(pin.unseen)
        pin.jump()
        XCTAssertTrue(pin.pinned)
        XCTAssertFalse(pin.unseen)
        pin.scrolled(distanceFromBottom: 500)
        _ = pin.contentAdded()
        pin.scrolled(distanceFromBottom: 0)
        XCTAssertTrue(pin.pinned)
        XCTAssertFalse(pin.unseen)
    }
}
