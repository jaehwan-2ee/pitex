import BuildFeature
import Foundation
import XCTest

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// The helper's stdin never grows without bound and never loses protocol
/// state: queued updates coalesce to the newest snapshot, their deltas are
/// taken against what the helper received in full, releases are never
/// dropped, and a stalled reader blocks no caller.
final class EmbeddedPreviewWriterTests: XCTestCase {
    private typealias Buffer = EmbeddedPreviewOutbox.Buffer
    private typealias File = EmbeddedPreviewOutbox.File

    // MARK: Outbox

    private func deliver(_ outbox: inout EmbeddedPreviewOutbox) -> EmbeddedPreviewOutbox.Request? {
        let request = outbox.next()
        outbox.written()
        return request
    }

    func testReplacedUpdateNeverTakesAClosedPathWithIt() {
        var outbox = EmbeddedPreviewOutbox()
        outbox.enqueue(update: 1, buffers: ["/p/a.tex": Buffer(hash: 1, text: "a1")])
        _ = deliver(&outbox)
        // In flight (reader stalled): b added.
        outbox.enqueue(update: 2, buffers: ["/p/a.tex": Buffer(hash: 1, text: "a1"), "/p/b.tex": Buffer(hash: 2, text: "b")])
        XCTAssertEqual(outbox.next(), .update(generation: 2, files: [File(path: "/p/b.tex", text: "b")], closed: []))
        // Queued only, then replaced: a saved (closed), later c edited.
        outbox.enqueue(update: 3, buffers: ["/p/b.tex": Buffer(hash: 2, text: "b")])
        outbox.enqueue(update: 4, buffers: ["/p/b.tex": Buffer(hash: 2, text: "b"), "/p/c.tex": Buffer(hash: 3, text: "c")])
        XCTAssertNil(outbox.next(), "one request in flight at a time")
        outbox.written()
        XCTAssertEqual(outbox.next(), .update(generation: 4, files: [File(path: "/p/c.tex", text: "c")], closed: ["/p/a.tex"]))
    }

    func testCloseThenReopenWhileQueuedSendsOnlyRealDifferences() {
        var outbox = EmbeddedPreviewOutbox()
        outbox.enqueue(update: 1, buffers: ["/p/a.tex": Buffer(hash: 1, text: "a1")])
        _ = deliver(&outbox)
        outbox.enqueue(release: 9)
        XCTAssertEqual(outbox.next(), .releases([9]))
        // Closed, then reopened with the same unsaved text, before either was written.
        outbox.enqueue(update: 2, buffers: [:])
        outbox.enqueue(update: 3, buffers: ["/p/a.tex": Buffer(hash: 1, text: "a1")])
        outbox.written()
        XCTAssertEqual(outbox.next(), .update(generation: 3, files: [], closed: []))
        outbox.written()
        // Reopened with different text, closed in between.
        outbox.enqueue(update: 4, buffers: [:])
        outbox.enqueue(update: 5, buffers: ["/p/a.tex": Buffer(hash: 7, text: "a7")])
        XCTAssertEqual(outbox.next(), .update(generation: 5, files: [File(path: "/p/a.tex", text: "a7")], closed: []))
    }

    func testReleasesAreKeptDeduplicatedAndWrittenFirst() {
        var outbox = EmbeddedPreviewOutbox()
        outbox.enqueue(update: 1, buffers: [:])
        _ = outbox.next() // stuck in flight
        outbox.enqueue(update: 2, buffers: ["/p/a.tex": Buffer(hash: 1, text: "a")])
        for seq: UInt64 in [3, 4, 3, 5, 4] { outbox.enqueue(release: seq) }
        XCTAssertEqual(outbox.pendingReleaseCount, 3)
        outbox.written()
        XCTAssertEqual(outbox.next(), .releases([3, 4, 5]))
        outbox.written()
        XCTAssertEqual(outbox.next(), .update(generation: 2, files: [File(path: "/p/a.tex", text: "a")], closed: []))
    }

    func testQuitSupersedesQueuedWork() {
        var outbox = EmbeddedPreviewOutbox()
        outbox.enqueue(update: 1, buffers: [:])
        outbox.enqueue(release: 2)
        outbox.enqueueQuit()
        outbox.enqueue(update: 3, buffers: [:])
        XCTAssertEqual(outbox.next(), .quit)
        outbox.written()
        XCTAssertNil(outbox.next())
    }

    // MARK: Real pipe

    private struct PipeReader {
        let descriptor: Int32
        var buffer = Data()

        /// Reads until `count` complete lines or EOF (or the deadline).
        mutating func lines(_ count: Int, timeout: TimeInterval = 20) -> (lines: [String], eof: Bool) {
            var lines: [String] = []
            let deadline = Date().addingTimeInterval(timeout)
            var chunk = [UInt8](repeating: 0, count: 1 << 16)
            while lines.count < count, Date() < deadline {
                while let newline = buffer.firstIndex(of: 0x0A), lines.count < count {
                    lines.append(String(decoding: buffer[buffer.startIndex..<newline], as: UTF8.self))
                    buffer.removeSubrange(buffer.startIndex...newline)
                }
                if lines.count == count { break }
                let read = chunk.withUnsafeMutableBytes { systemRead(descriptor, $0.baseAddress!, $0.count) }
                if read > 0 {
                    buffer.append(contentsOf: chunk[0..<read])
                } else if read == 0 {
                    return (lines, true)
                } else {
                    usleep(2_000)
                }
            }
            return (lines, false)
        }
    }

    private func makePipe() -> (read: Int32, write: Int32) {
        var fds: [Int32] = [-1, -1]
        XCTAssertEqual(pipe(&fds), 0)
        _ = fcntl(fds[0], F_SETFL, fcntl(fds[0], F_GETFL) | O_NONBLOCK)
        return (fds[0], fds[1])
    }

    private func object(_ line: String) -> [String: Any] {
        (try? JSONSerialization.jsonObject(with: Data(line.utf8))) as? [String: Any] ?? [:]
    }

    override func setUp() {
        super.setUp()
        signal(SIGPIPE, SIG_IGN)
    }

    func testStalledReaderKeepsOneWriteAndTheNewestUpdate() {
        let (readEnd, writeEnd) = makePipe()
        defer { close(readEnd) }
        let writer = EmbeddedPreviewWriter(fileDescriptor: writeEnd)
        // Far larger than a pipe buffer: stays in flight while nobody reads.
        writer.update(generation: 1, buffers: ["/p/a.tex": Buffer(hash: 1, text: String(repeating: "x", count: 1 << 20))])
        let big = String(repeating: "y", count: 256 << 10)
        for generation in UInt64(2)...300 {
            // Rapid typing in b; a closed (saved) from generation 3 on.
            var buffers = ["/p/b.tex": Buffer(hash: generation, text: big + "\(generation)")]
            if generation < 3 { buffers["/p/a.tex"] = Buffer(hash: 1, text: "unused") }
            writer.update(generation: generation, buffers: buffers)
        }
        for seq: UInt64 in [1, 2, 1, 3, 2] { writer.release(seq) }
        let backlog = writer.backlog()
        guard case .update(generation: 1, _, _) = backlog.inFlight else {
            return XCTFail("the first update stays in flight behind the stalled reader: \(String(describing: backlog.inFlight))")
        }
        XCTAssertTrue(backlog.pendingUpdate)
        XCTAssertEqual(backlog.pendingReleases, 3)

        var reader = PipeReader(descriptor: readEnd)
        let drained = reader.lines(5).lines.map(object)
        guard drained.count == 5 else { return XCTFail("expected 5 lines, got \(drained.count)") }
        XCTAssertEqual(drained.map { $0["op"] as? String }, ["update", "release", "release", "release", "update"])
        XCTAssertEqual(drained.first?["generation"] as? Int, 1)
        XCTAssertEqual(drained[1...3].map { $0["seq"] as? Int }, [1, 2, 3], "every release once, none dropped")
        let newest = drained.last ?? [:]
        XCTAssertEqual(newest["generation"] as? Int, 300, "intermediate updates coalesced away")
        XCTAssertEqual(newest["closed"] as? [String], ["/p/a.tex"], "the close queued in a replaced update survives")
        let files = newest["files"] as? [[String: String]] ?? []
        XCTAssertEqual(files.map { $0["path"] }, ["/p/b.tex"])
        XCTAssertEqual(files.first?["text"], big + "300")

        writer.finish()
        let tail = reader.lines(2)
        XCTAssertEqual(tail.lines, [#"{"op":"quit"}"#])
        XCTAssertTrue(tail.eof, "quit is followed by EOF")
    }

    func testFinishNeverWaitsForAStalledReader() {
        let (readEnd, writeEnd) = makePipe()
        defer { close(readEnd) }
        let writer = EmbeddedPreviewWriter(fileDescriptor: writeEnd)
        writer.update(generation: 1, buffers: ["/p/a.tex": Buffer(hash: 1, text: String(repeating: "x", count: 4 << 20))])
        writer.release(7)
        let started = Date()
        writer.finish()
        let backlog = writer.backlog()
        XCTAssertLessThan(Date().timeIntervalSince(started), 1, "finish and the writer queue never block on the pipe")
        XCTAssertNil(backlog.inFlight, "the stuck write was cancelled, not waited for")
        XCTAssertFalse(backlog.pendingUpdate)
        XCTAssertEqual(backlog.pendingReleases, 0)
        // The reader gets the partial line and then EOF: the helper exits.
        var reader = PipeReader(descriptor: readEnd)
        let rest = reader.lines(Int.max, timeout: 10)
        XCTAssertTrue(rest.eof)
    }
}

@inline(__always)
private func systemRead(_ descriptor: Int32, _ buffer: UnsafeMutableRawPointer, _ count: Int) -> Int {
    #if canImport(Darwin)
    return Darwin.read(descriptor, buffer, count)
    #elseif canImport(Glibc)
    return Glibc.read(descriptor, buffer, count)
    #endif
}
