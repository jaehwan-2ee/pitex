import Foundation
@testable import RemoteCore
import XCTest

private actor RemotePipeOutput {
    var stdout = Data()
    var stderr = Data()

    func append(_ bytes: Data, isError: Bool) {
        if isError { stderr.append(bytes) } else { stdout.append(bytes) }
    }
}

final class RemoteProcessPipeTests: XCTestCase {
    func testRapidExitStreamsDeliverBothPipesAndStatus() async throws {
        try await withThrowingTaskGroup(of: Void.self) { group in
            for index in 0..<40 {
                group.addTask {
                    let output = RemotePipeOutput()
                    let status = try await ProcessPipe.stream(
                        executable: URL(fileURLWithPath: "/bin/sh"),
                        arguments: ["-c", "printf out; printf err >&2; exit \(index % 5)"]
                    ) { bytes, isError in await output.append(bytes, isError: isError) }
                    let stdout = await output.stdout
                    let stderr = await output.stderr
                    XCTAssertEqual(status, Int32(index % 5))
                    XCTAssertEqual(stdout, Data("out".utf8))
                    XCTAssertEqual(stderr, Data("err".utf8))
                }
            }
            try await group.waitForAll()
        }
    }

    func testRunDrainsLargeInputAndStderrBeforeReturning() async throws {
        let input = Data(repeating: 0x61, count: 1_048_576)
        let result = try await ProcessPipe.run(
            executable: URL(fileURLWithPath: "/bin/sh"),
            arguments: ["-c", "head -c 1048576 /dev/zero >&2; cat"],
            input: input, outputFile: nil
        )
        XCTAssertEqual(result.status, 0)
        XCTAssertEqual(result.standardOutput, input)
        XCTAssertEqual(result.standardError.count, 1_048_576)
    }

    func testCancellationWaitsForStreamExit() async throws {
        let output = RemotePipeOutput()
        let task = Task {
            try await ProcessPipe.stream(
                executable: URL(fileURLWithPath: "/bin/sh"),
                arguments: ["-c", "printf ready; exec /bin/sleep 30"]
            ) { bytes, isError in await output.append(bytes, isError: isError) }
        }
        let deadline = ContinuousClock.now + .seconds(5)
        while await output.stdout.isEmpty {
            guard ContinuousClock.now < deadline else {
                task.cancel()
                XCTFail("the child did not start")
                return
            }
            try await Task.sleep(for: .milliseconds(10))
        }
        let started = ContinuousClock.now
        task.cancel()
        do {
            _ = try await task.value
            XCTFail("a cancelled stream must report cancellation")
        } catch SSHError.cancelled { }
        XCTAssertLessThan(started.duration(to: .now), .seconds(2))
    }

    func testFailedLaunchDoesNotWaitForAnExitNotification() async throws {
        let missing = URL(fileURLWithPath: "/private/tmp/pitex-no-executable-\(UUID().uuidString)")
        do {
            _ = try await ProcessPipe.stream(executable: missing, arguments: []) { _, _ in }
            XCTFail("a missing executable must fail")
        } catch SSHError.connection { }
        do {
            _ = try await ProcessPipe.run(executable: missing, arguments: [], input: nil, outputFile: nil)
            XCTFail("a missing executable must fail")
        } catch SSHError.connection { }
    }
}
