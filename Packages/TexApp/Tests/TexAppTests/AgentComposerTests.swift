import AppPorts
import Foundation
import XCTest

/// The composer model — the same cases as the Linux `composer.rs` tests.
final class AgentComposerTests: XCTestCase {
    private func png(_ attached: Int = 0) throws -> AgentImage {
        try AgentImage(name: "a.png", mimeType: "image/png", data: Data([1, 2, 3]), attached: attached)
    }

    private func problem(_ make: () throws -> AgentImage) -> AgentImage.Problem? {
        do {
            _ = try make()
            return nil
        } catch {
            return error as? AgentImage.Problem
        }
    }

    func testImageCapsAndTypes() throws {
        let image = try png()
        XCTAssertEqual(image.wire["mimeType"] as? String, "image/png")
        XCTAssertEqual(image.wire["type"] as? String, "image")
        XCTAssertEqual(image.wire["data"] as? String, "AQID")
        XCTAssertEqual(problem { try png(AgentComposer.maxImages) }, .tooMany)
        XCTAssertNoThrow(try png(AgentComposer.maxImages - 1))
        let big = Data(count: AgentComposer.maxImageBytes + 1)
        XCTAssertEqual(problem { try AgentImage(name: "b.png", mimeType: "image/png", data: big, attached: 0) }, .tooLarge)
        XCTAssertNoThrow(try AgentImage(name: "b.png", mimeType: "image/png", data: big.dropLast(), attached: 0))
        XCTAssertEqual(problem { try AgentImage(name: "c.tiff", mimeType: "image/tiff", data: Data([1]), attached: 0) }, .unsupported)
        XCTAssertEqual(AgentImage.Problem.tooLarge.key, "assistant.image.too_large")
        XCTAssertEqual(AgentComposer.imageMIMEType(path: "shots/Red.PNG"), "image/png")
        XCTAssertEqual(AgentComposer.imageMIMEType(path: "a.jpeg"), "image/jpeg")
        XCTAssertNil(AgentComposer.imageMIMEType(path: "main.tex"))
        XCTAssertEqual(AgentComposer.byteSize(1500), "2 KB")
        XCTAssertEqual(AgentComposer.byteSize(3 * 1024 * 1024 / 2), "1.5 MB")
    }

    func testReturnQueuesDuringATurnAndWhileAnApprovalWaits() {
        XCTAssertEqual(AgentComposer.route(running: false, approvalPending: false, now: false, canSteer: true), .send)
        XCTAssertEqual(AgentComposer.route(running: false, approvalPending: false, now: true, canSteer: true), .send)
        XCTAssertEqual(AgentComposer.route(running: true, approvalPending: false, now: false, canSteer: true), .queue)
        XCTAssertEqual(AgentComposer.route(running: true, approvalPending: false, now: true, canSteer: true), .steer)
        XCTAssertEqual(AgentComposer.route(running: true, approvalPending: false, now: true, canSteer: false), .queue)
        XCTAssertEqual(AgentComposer.route(running: true, approvalPending: true, now: true, canSteer: true), .queue)
    }

    func testQueueDrainsOnePerTurnAndARefusedSendGoesBackToTheFront() throws {
        var queue = AgentMessageQueue()
        let a = queue.enqueue("first")
        let b = queue.enqueue("second", images: [try png()])
        let head = try XCTUnwrap(queue.takeNext())
        XCTAssertEqual(head.id, a)
        XCTAssertEqual(head.state, .sending)
        XCTAssertNil(queue.takeNext(), "one in flight at a time")
        queue.failed(a)
        XCTAssertEqual(queue.items.first?.id, a)
        XCTAssertEqual(queue.items.first?.state, .queued)
        XCTAssertTrue(queue.paused)
        XCTAssertNil(queue.takeNext(), "paused after a refusal")
        queue.paused = false
        XCTAssertEqual(queue.takeNext()?.id, a)
        queue.sent(a)
        let next = try XCTUnwrap(queue.takeNext())
        XCTAssertEqual(next.id, b)
        XCTAssertEqual(next.images.count, 1)
        queue.sent(b)
        XCTAssertTrue(queue.items.isEmpty)
    }

    func testEditDeleteAndSendNowTakeTheRowOut() {
        var queue = AgentMessageQueue()
        let a = queue.enqueue("one")
        let b = queue.enqueue("two")
        let c = queue.enqueue("three")
        XCTAssertEqual(queue.remove(b)?.text, "two")
        XCTAssertNil(queue.remove(b))
        XCTAssertEqual(queue.items.map(\.id), [a, c])
        queue.paused = true
        queue.clear()
        XCTAssertTrue(queue.items.isEmpty)
        XCTAssertFalse(queue.paused)
    }

    func testMentionTriggerSkipsEmailsAndStopsAtWhitespace() {
        XCTAssertEqual(AgentFileMention.active(in: "see @ch", cursor: 7), AgentFileMention(start: 4, end: 7, query: "ch"))
        XCTAssertEqual(AgentFileMention.active(in: "@", cursor: 1)?.query, "")
        XCTAssertNil(AgentFileMention.active(in: "mail me@host.com", cursor: 16))
        XCTAssertNil(AgentFileMention.active(in: "@a b", cursor: 4))
        XCTAssertNil(AgentFileMention.active(in: "@\"x", cursor: 3))
        XCTAssertEqual(AgentFileMention.active(in: "@intro and more", cursor: 6)?.query, "intro")
        XCTAssertEqual(AgentFileMention.active(in: "한글 @서론", cursor: 6)?.query, "서론")
    }

    func testMentionFilterIsFuzzyAndRanksFileNamesFirst() {
        let files = ["chapters/intro.tex", "main.tex", "figures/plot intro.png", "refs.bib", "chapters/method.tex"]
        XCTAssertEqual(AgentFileMention.matches(files, query: "intro"), ["chapters/intro.tex", "figures/plot intro.png"])
        XCTAssertEqual(AgentFileMention.matches(files, query: "chm"), ["chapters/method.tex"])
        XCTAssertEqual(AgentFileMention.matches(files, query: "MAIN"), ["main.tex"])
        XCTAssertEqual(AgentFileMention.matches(files, query: "", limit: 2).count, 2)
        XCTAssertTrue(AgentFileMention.matches(files, query: "zzz").isEmpty)
    }

    func testMentionInsertQuotesPathsWithSpaces() throws {
        let text = "look at @int please"
        let mention = try XCTUnwrap(AgentFileMention.active(in: text, cursor: 12))
        let applied = mention.apply(to: text, path: "chapters/intro.tex")
        XCTAssertEqual(applied.text, "look at @chapters/intro.tex  please")
        XCTAssertEqual(applied.cursor, 28)
        let quoted = try XCTUnwrap(AgentFileMention.active(in: "@plot", cursor: 5)).apply(to: "@plot", path: "figures/plot intro.png")
        XCTAssertEqual(quoted.text, "@\"figures/plot intro.png\" ")
        XCTAssertEqual(quoted.cursor, quoted.text.count)
    }
}
