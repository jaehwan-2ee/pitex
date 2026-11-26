import AppPorts
import XCTest

final class AgentStdinWriterTests: XCTestCase {
    func testAWriteLargerThanThePipeReturnsWhileTheChildStopsReading() {
        let pipe = Pipe()
        let writer = AgentStdinWriter(pipe.fileHandleForWriting, onFailure: { _ in })
        let prompt = Data(repeating: UInt8(ascii: "x"), count: 4 * 1024 * 1024) + Data("\n".utf8)
        let next = Data("{\"type\":\"abort\"}\n".utf8)
        let returned = expectation(description: "both writes returned")
        DispatchQueue.global().async {
            writer.write(prompt)
            writer.write(next)
            returned.fulfill()
        }
        wait(for: [returned], timeout: 5)

        let received = pipe.fileHandleForReading.readData(ofLength: prompt.count + next.count)
        XCTAssertEqual(received, prompt + next, "commands arrive whole and in order")
    }

    #if canImport(Darwin)
    // Only Darwin has F_SETNOSIGPIPE; elsewhere this write raises SIGPIPE.
    func testAWriteToAClosedChildReportsTheFailureOnce() {
        let pipe = Pipe()
        try? pipe.fileHandleForReading.close()
        let failed = expectation(description: "failure reported")
        failed.expectedFulfillmentCount = 1
        failed.assertForOverFulfill = true
        let writer = AgentStdinWriter(pipe.fileHandleForWriting, onFailure: { _ in failed.fulfill() })
        writer.write(Data("{}\n".utf8))
        writer.write(Data("{}\n".utf8))
        wait(for: [failed], timeout: 5)
    }
    #endif
}
