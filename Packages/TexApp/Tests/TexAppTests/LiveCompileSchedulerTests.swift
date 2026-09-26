import BuildFeature
import XCTest

final class LiveCompileSchedulerTests: XCTestCase {
    private func makeEnabled() -> LiveCompileScheduler {
        var scheduler = LiveCompileScheduler()
        scheduler.setEnabled(true)
        return scheduler
    }

    private func liveStart(
        _ requests: [LiveRequest],
        file: StaticString = #filePath,
        line: UInt = #line
    ) -> LiveRunToken {
        guard requests.count == 1, case let .startLive(token, _) = requests[0] else {
            XCTFail("expected one StartLive, got \(requests)", file: file, line: line)
            return LiveRunToken(id: 0, kind: .live, generation: 0)
        }
        return token
    }

    private func manualStart(
        _ requests: [LiveRequest],
        file: StaticString = #filePath,
        line: UInt = #line
    ) -> LiveRunToken {
        guard requests.count == 1, case let .startManual(token) = requests[0] else {
            XCTFail("expected one StartManual, got \(requests)", file: file, line: line)
            return LiveRunToken(id: 0, kind: .manual, generation: 0)
        }
        return token
    }

    func testDisabledIgnoresEditsAndPoll() {
        var scheduler = LiveCompileScheduler()
        XCTAssertTrue(scheduler.noteEdit(nowMs: 0).isEmpty)
        XCTAssertTrue(scheduler.poll(nowMs: 10_000, composing: false).isEmpty)
        XCTAssertNil(scheduler.pendingDeadline)
    }

    func testRapidEditsCoalesceToOneDeadline() {
        var scheduler = makeEnabled()
        XCTAssertTrue(scheduler.noteEdit(nowMs: 0).isEmpty)
        XCTAssertTrue(scheduler.noteEdit(nowMs: 300).isEmpty)
        XCTAssertTrue(scheduler.noteEdit(nowMs: 600).isEmpty)
        XCTAssertEqual(scheduler.pendingDeadline, 1_300)
        XCTAssertTrue(scheduler.poll(nowMs: 1_299, composing: false).isEmpty)
        let token = liveStart(scheduler.poll(nowMs: 1_300, composing: false))
        XCTAssertEqual(scheduler.active, token)
        // The started run consumed the pending edit — no second start.
        XCTAssertTrue(scheduler.poll(nowMs: 5_000, composing: false).isEmpty)
    }

    func testComposingHoldsTheDeadlineUntilLifted() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        XCTAssertTrue(scheduler.poll(nowMs: 700, composing: true).isEmpty)
        let token = liveStart(scheduler.poll(nowMs: 700, composing: false))
        XCTAssertTrue(scheduler.liveActive)
        XCTAssertEqual(scheduler.active, token)
    }

    func testEditCancelsLiveButNotManual() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let live = liveStart(scheduler.poll(nowMs: 700, composing: false))
        XCTAssertEqual(scheduler.noteEdit(nowMs: 800), [.cancel(token: live)])
        // A second edit does not re-cancel.
        XCTAssertTrue(scheduler.noteEdit(nowMs: 810).isEmpty)
        // Manual active: an edit just stays pending.
        var scheduler2 = makeEnabled()
        let manual = manualStart(scheduler2.requestManual())
        XCTAssertTrue(scheduler2.noteEdit(nowMs: 0).isEmpty)
        XCTAssertEqual(scheduler2.active, manual)
        XCTAssertEqual(scheduler2.pendingDeadline, 700)
    }

    func testCancelledLiveRestartsWithoutASecondDelay() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let first = liveStart(scheduler.poll(nowMs: 700, composing: false))
        scheduler.noteEdit(nowMs: 800) // cancels `first`, deadline 1500
        // Cancellation still in flight — pending is kept, nothing starts.
        XCTAssertTrue(scheduler.poll(nowMs: 1_500, composing: false).isEmpty)
        // The old run finishes after the new deadline: dispatch immediately.
        let (status, requests) = scheduler.completed(token: first, nowMs: 1_700, composing: false)
        XCTAssertEqual(status, .superseded(first))
        let second = liveStart(requests)
        XCTAssertNotEqual(first, second)
        let (status2, _) = scheduler.completed(token: second, nowMs: 2_000, composing: false)
        XCTAssertEqual(status2, .current(second))
    }

    func testSupersededLiveNeverPublishes() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let live = liveStart(scheduler.poll(nowMs: 700, composing: false))
        // An edit raced the in-flight run; it succeeds anyway — the
        // completion is superseded so the stale PDF is never published.
        scheduler.noteEdit(nowMs: 900)
        let (status, requests) = scheduler.completed(token: live, nowMs: 1_000, composing: false)
        XCTAssertEqual(status, .superseded(live))
        // Deadline (1600) not yet reached — poll later.
        XCTAssertTrue(requests.isEmpty)
        let next = liveStart(scheduler.poll(nowMs: 1_600, composing: false))
        let (status2, _) = scheduler.completed(token: next, nowMs: 2_000, composing: false)
        XCTAssertEqual(status2, .current(next))
    }

    func testManualTakesPriorityAndPendingEditsFollow() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let live = liveStart(scheduler.poll(nowMs: 700, composing: false))
        let requests = scheduler.requestManual()
        // Pending edit consumed by the manual request; the live run is
        // cancelled but the manual run must NOT start while it still
        // holds the slot — builds never overlap.
        XCTAssertNil(scheduler.pendingDeadline)
        XCTAssertEqual(requests, [.cancel(token: live)])
        XCTAssertEqual(scheduler.active, live)
        // The cancelled live run finishes: superseded, manual dispatches.
        let (status, requests2) = scheduler.completed(token: live, nowMs: 1_000, composing: false)
        XCTAssertEqual(status, .superseded(live))
        let manual = manualStart(requests2)
        XCTAssertTrue(scheduler.manualActive)
        // Edits during the manual run stay pending; the manual result
        // still publishes, then the newest source compiles immediately.
        scheduler.noteEdit(nowMs: 1_100)
        let (status2, requests3) = scheduler.completed(token: manual, nowMs: 2_000, composing: false)
        XCTAssertEqual(status2, .current(manual))
        let next = liveStart(requests3)
        let (status3, _) = scheduler.completed(token: next, nowMs: 2_500, composing: false)
        XCTAssertEqual(status3, .current(next))
    }

    func testStaleCompletionCannotFinishTheNewerRun() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let first = liveStart(scheduler.poll(nowMs: 700, composing: false))
        scheduler.noteEdit(nowMs: 800)
        let (_, requests) = scheduler.completed(token: first, nowMs: 1_600, composing: false)
        let second = liveStart(requests)
        // A duplicate completion for the old run changes nothing.
        let (status, requests2) = scheduler.completed(token: first, nowMs: 1_700, composing: false)
        XCTAssertEqual(status, .stale)
        XCTAssertTrue(requests2.isEmpty)
        XCTAssertEqual(scheduler.active, second)
    }

    func testStaleCompletionNeverDispatchesPendingWork() {
        // No run is active but an edit's deadline has passed; a stray
        // completion echoing an unknown token must not launch the build —
        // only the caller's timer (poll) may.
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let stray = LiveRunToken(id: 99, kind: .live, generation: 0)
        let (status, requests) = scheduler.completed(token: stray, nowMs: 10_000, composing: false)
        XCTAssertEqual(status, .stale)
        XCTAssertTrue(requests.isEmpty)
        XCTAssertEqual(scheduler.pendingDeadline, 700)
        XCTAssertNil(scheduler.active)
    }

    func testAcceptsActiveResultGuardsStreamingEvents() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let live = liveStart(scheduler.poll(nowMs: 700, composing: false))
        XCTAssertTrue(scheduler.acceptsActiveResult(live))
        // Superseded by a newer edit — its late log/issue events drop.
        scheduler.noteEdit(nowMs: 800)
        XCTAssertFalse(scheduler.acceptsActiveResult(live))
        // Cancel in flight; it finishes superseded, nothing due yet.
        let (status, requests) = scheduler.completed(token: live, nowMs: 1_000, composing: false)
        XCTAssertEqual(status, .superseded(live))
        XCTAssertTrue(requests.isEmpty)
        // The next live run accepts again.
        let next = liveStart(scheduler.poll(nowMs: 1_500, composing: false))
        XCTAssertTrue(scheduler.acceptsActiveResult(next))
        // A manual request supersedes it; the manual token accepts while
        // active even with pending live work queued behind it.
        XCTAssertEqual(scheduler.requestManual(), [.cancel(token: next)])
        XCTAssertFalse(scheduler.acceptsActiveResult(next))
        let (_, requests2) = scheduler.completed(token: next, nowMs: 1_600, composing: false)
        let manual = manualStart(requests2)
        scheduler.noteEdit(nowMs: 1_700)
        XCTAssertTrue(scheduler.acceptsActiveResult(manual))
        // Unknown/wrong-identity tokens never accept.
        let stray = LiveRunToken(id: 77, kind: .manual, generation: 0)
        XCTAssertFalse(scheduler.acceptsActiveResult(stray))
    }

    func testToggleOffInvalidatesAndCancelsOnlyLive() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let live = liveStart(scheduler.poll(nowMs: 700, composing: false))
        XCTAssertEqual(scheduler.setEnabled(false), [.cancel(token: live)])
        XCTAssertNil(scheduler.pendingDeadline)
        XCTAssertTrue(scheduler.noteEdit(nowMs: 1_000).isEmpty)
        // The cancelled run lands after the toggle: older generation →
        // superseded, nothing restarts while disabled.
        let (status, requests) = scheduler.completed(token: live, nowMs: 1_200, composing: false)
        XCTAssertEqual(status, .superseded(live))
        XCTAssertTrue(requests.isEmpty)
        XCTAssertTrue(scheduler.poll(nowMs: 10_000, composing: false).isEmpty)
        // A manual run survives a toggle-off — its logs/results stay
        // accepted despite the generation bump.
        var scheduler2 = makeEnabled()
        let manual = manualStart(scheduler2.requestManual())
        XCTAssertTrue(scheduler2.setEnabled(false).isEmpty)
        XCTAssertTrue(scheduler2.acceptsActiveResult(manual))
        let (status2, _) = scheduler2.completed(token: manual, nowMs: 5_000, composing: false)
        XCTAssertEqual(status2, .current(manual))
    }

    func testToggleOffRetainsQueuedManualBehindCancellingLive() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let live = liveStart(scheduler.poll(nowMs: 700, composing: false))
        XCTAssertEqual(scheduler.requestManual(), [.cancel(token: live)])
        // Toggle off while the live cancel is in flight: the explicit
        // manual request is not live work and must survive — only
        // invalidate() drops it.
        XCTAssertTrue(scheduler.setEnabled(false).isEmpty)
        XCTAssertTrue(scheduler.hasPendingWork)
        let (status, requests) = scheduler.completed(token: live, nowMs: 2_000, composing: false)
        XCTAssertEqual(status, .superseded(live))
        let manual = manualStart(requests)
        XCTAssertTrue(scheduler.manualActive)
        XCTAssertTrue(scheduler.acceptsActiveResult(manual))
        let (status2, requests2) = scheduler.completed(token: manual, nowMs: 5_000, composing: false)
        XCTAssertEqual(status2, .current(manual))
        XCTAssertTrue(requests2.isEmpty)
    }

    func testInvalidateClearsQueuedManualAndPending() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 0)
        let live = liveStart(scheduler.poll(nowMs: 700, composing: false))
        scheduler.requestManual()
        // Invalidate while the live cancellation is in flight: the queued
        // manual belonged to the old context and must not start.
        XCTAssertTrue(scheduler.invalidate().isEmpty) // live already cancel-requested
        let (status, requests) = scheduler.completed(token: live, nowMs: 2_000, composing: false)
        XCTAssertEqual(status, .superseded(live))
        XCTAssertTrue(requests.isEmpty)
        XCTAssertNil(scheduler.active)
    }

    func testDelayChangeRecomputesFromEditTime() {
        var scheduler = makeEnabled()
        scheduler.noteEdit(nowMs: 1_000)
        XCTAssertEqual(scheduler.pendingDeadline, 1_700)
        scheduler.setDelay(300)
        XCTAssertEqual(scheduler.pendingDeadline, 1_300)
        scheduler.setDelay(5_000)
        XCTAssertEqual(scheduler.pendingDeadline, 6_000)
    }

    func testManualCompletionPublishesAndPendingLiveStarts() {
        var scheduler = makeEnabled()
        let manual = manualStart(scheduler.requestManual())
        // Edits during the manual run stay pending.
        scheduler.noteEdit(nowMs: 100)
        let (status, requests) = scheduler.completed(token: manual, nowMs: 900, composing: false)
        XCTAssertEqual(status, .current(manual))
        // Deadline (800) already elapsed → live starts immediately.
        let live = liveStart(requests)
        let (status2, _) = scheduler.completed(token: live, nowMs: 2_000, composing: false)
        XCTAssertEqual(status2, .current(live))
    }
}
