import BuildFeature
import XCTest

/// Mirrors the Linux `embedded_preview_final_floor` regression: the final
/// build floor is an edit revision, so late transport updates never cover
/// the final PDF while real later edits resume drafting.
final class EmbeddedPreviewLedgerTests: XCTestCase {
    private let main = "/project/main.tex"

    /// Queues one update; `dirty` is the main buffer's content hash.
    private func send(_ ledger: inout EmbeddedPreviewLedger, dirty: UInt64? = nil) -> UInt64 {
        ledger.recordUpdate(sources: [main: dirty ?? 0], revision: ledger.editRevision)
    }

    func testRemoteUploadEditsKeepTheirNewerLocalPreview() {
        var ledger = EmbeddedPreviewLedger()
        ledger.noteEdit()
        let uploadedRevision = ledger.editRevision
        // Upload takes time; the unsaved next edit reaches the local helper.
        ledger.noteEdit()
        let draft = send(&ledger, dirty: 8)
        XCTAssertEqual(ledger.accept(seq: 1, generation: draft, complete: true), .display(releasing: nil))
        ledger.noteFinalBuildStarted(revision: uploadedRevision)
        XCTAssertEqual(ledger.noteFinalBuildPublished(), .keepPreview)
        XCTAssertEqual(ledger.finalFloor, uploadedRevision)
    }

    func testLatePrebuildUpdatesNeverCoverTheFinalPDF() {
        var ledger = EmbeddedPreviewLedger()
        let first = send(&ledger)
        XCTAssertEqual(ledger.accept(seq: 1, generation: first, complete: true), .display(releasing: nil))

        // Edit, then Build before the coalescing window fired.
        ledger.noteEdit()
        ledger.noteFinalBuildStarted()
        XCTAssertEqual(ledger.noteFinalBuildPublished(), .showFinal(releasing: 1))

        // The queued edit flushes after the pre-save: newer generation, same edit.
        let late = send(&ledger)
        XCTAssertEqual(ledger.accept(seq: 2, generation: late, complete: true), .belowFinalFloor)
        // A save/rescan refresh sends yet another generation.
        let rescan = send(&ledger)
        XCTAssertEqual(ledger.accept(seq: 3, generation: rescan, complete: true), .belowFinalFloor)

        ledger.noteEdit()
        let edited = send(&ledger, dirty: 7)
        XCTAssertEqual(ledger.accept(seq: 4, generation: edited, complete: true), .display(releasing: nil))
    }

    func testPreviewsDuringARunningBuild() {
        var ledger = EmbeddedPreviewLedger()
        XCTAssertEqual(ledger.accept(seq: 1, generation: send(&ledger), complete: true), .display(releasing: nil))

        // A pre-build edit published while the build runs yields to the final.
        ledger.noteEdit()
        ledger.noteFinalBuildStarted()
        XCTAssertEqual(ledger.accept(seq: 2, generation: send(&ledger, dirty: 1), complete: true),
                       .display(releasing: 1))
        XCTAssertEqual(ledger.noteFinalBuildPublished(), .showFinal(releasing: 2))

        // An edit typed while the next build runs keeps drafting after it.
        ledger.noteFinalBuildStarted()
        ledger.noteEdit()
        XCTAssertEqual(ledger.accept(seq: 3, generation: send(&ledger, dirty: 2), complete: true),
                       .display(releasing: nil))
        XCTAssertEqual(ledger.noteFinalBuildPublished(), .keepPreview)
        XCTAssertEqual(ledger.displayed?.seq, 3)
    }

    func testLaggingPublicationsDisplayButNeverRegress() {
        var ledger = EmbeddedPreviewLedger()
        let g1 = send(&ledger, dirty: 1)
        let g2 = send(&ledger, dirty: 2)
        let g3 = send(&ledger, dirty: 3)
        // Behind the newest sent generation is fine.
        XCTAssertEqual(ledger.accept(seq: 5, generation: g2, complete: false), .display(releasing: nil))
        // Older generation or older/equal seq than the display never shows.
        XCTAssertEqual(ledger.accept(seq: 6, generation: g1, complete: true), .stale)
        XCTAssertEqual(ledger.accept(seq: 5, generation: g3, complete: true), .stale)
        // Same generation, newer seq (a later snapshot of the same pass).
        XCTAssertEqual(ledger.accept(seq: 7, generation: g2, complete: true), .display(releasing: 5))
        // A generation never sent is a protocol error, not a display.
        XCTAssertEqual(ledger.accept(seq: 8, generation: g3 + 1, complete: true), .stale)
    }

    func testForgottenGenerationIsIneligibleOnceAFloorExists() {
        var ledger = EmbeddedPreviewLedger()
        ledger.noteEdit()
        let old = send(&ledger, dirty: 1)
        ledger.noteFinalBuildStarted()
        _ = ledger.noteFinalBuildPublished()
        ledger.noteEdit()
        for text in 2...UInt64(EmbeddedPreviewLedger.historyLimit + 1) { _ = send(&ledger, dirty: text) }
        XCTAssertNil(ledger.update(old))
        XCTAssertEqual(ledger.accept(seq: 1, generation: old, complete: true), .belowFinalFloor)
    }

    func testIdleAdvancesOnlyTheDisplayedPublication() {
        var ledger = EmbeddedPreviewLedger()
        let g1 = send(&ledger, dirty: 1)
        XCTAssertEqual(ledger.accept(seq: 1, generation: g1, complete: true), .display(releasing: nil))
        let g2 = send(&ledger)
        XCTAssertFalse(ledger.noteIdle(generation: g2, seq: 9))
        XCTAssertTrue(ledger.noteIdle(generation: g2, seq: 1))
        XCTAssertEqual(ledger.displayed?.generation, g2)
        // A later publication of the pre-idle generation is a regression.
        XCTAssertEqual(ledger.accept(seq: 2, generation: g1, complete: true), .stale)
    }

    func testContextChangeDropsTheFinalFloor() {
        var ledger = EmbeddedPreviewLedger()
        ledger.noteFinalBuildStarted()
        _ = ledger.noteFinalBuildPublished()
        XCTAssertEqual(ledger.accept(seq: 1, generation: send(&ledger), complete: true), .belowFinalFloor)
        ledger.resetContext()
        XCTAssertEqual(ledger.accept(seq: 2, generation: send(&ledger), complete: true), .display(releasing: nil))
    }
}
