import Foundation
import XCTest
@testable import BuildCore

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

final class ProcessRunnerTests: XCTestCase {
    func testExecutableSearchUsesChildPATHAndWorkingDirectory() async throws {
        try await withTemporaryDirectory { directory in
            let bin = directory.appendingPathComponent("tools with spaces", isDirectory: true)
            try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: false)
            let tool = bin.appendingPathComponent("pitex-test-tool")
            try "#!/bin/sh\nprintf '%s' \"$PATH\"\n".write(to: tool, atomically: true, encoding: .utf8)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: tool.path)
            for path in [bin.path, "tools with spaces"] {
                for environment in [EnvironmentPolicy.replace(["PATH": path]), .inherit(overrides: ["PATH": path])] {
                    let plan = try DirectCommandPlan(executable: "pitex-test-tool", arguments: [], environment: environment)
                    let result = try await ProcessRunner().run(plan, projectRoot: directory)
                    XCTAssertEqual(result.termination, .exited(code: 0))
                    XCTAssertEqual(String(decoding: result.standardOutput, as: UTF8.self), path)
                }
            }
            let excluded = try DirectCommandPlan(executable: "env", arguments: [], environment: .replace(["PATH": bin.path]))
            do {
                _ = try await ProcessRunner().run(excluded, projectRoot: directory)
                XCTFail("must not fall back to the parent PATH")
            } catch let error as ProcessRunnerError {
                XCTAssertEqual(error, .spawnFailed(code: ENOENT))
            }
        }
    }

    func testDirectArgumentsAreNotInterpretedByAShell() async throws {
        try await withTemporaryDirectory { directory in
            let metacharacters = "literal ; $(touch NEVER) * ' quote"
            let plan = try DirectCommandPlan(
                executable: "/usr/bin/printf",
                arguments: ["%s", metacharacters],
                environment: .replace([:])
            )
            let result = try await ProcessRunner().run(plan, projectRoot: directory)

            XCTAssertEqual(result.termination, .exited(code: 0))
            XCTAssertEqual(String(decoding: result.standardOutput, as: UTF8.self), metacharacters)
            XCTAssertFalse(FileManager.default.fileExists(atPath: directory.appendingPathComponent("NEVER").path))
        }
    }

    func testWorkingDirectoryAndEnvironmentPolicies() async throws {
        try await withTemporaryDirectory { directory in
            let child = directory.appendingPathComponent("child", isDirectory: true)
            try FileManager.default.createDirectory(at: child, withIntermediateDirectories: false)
            let pwd = try DirectCommandPlan(
                executable: "/bin/pwd",
                arguments: [],
                workingDirectory: .explicit("child"),
                environment: .replace([:])
            )
            let pwdResult = try await ProcessRunner().run(pwd, projectRoot: directory)
            let reportedDirectory = URL(
                fileURLWithPath: String(decoding: pwdResult.standardOutput, as: UTF8.self)
                    .trimmingCharacters(in: .whitespacesAndNewlines)
            )
            XCTAssertEqual(
                reportedDirectory.resolvingSymlinksInPath().path,
                child.resolvingSymlinksInPath().path
            )

            let environment = try DirectCommandPlan(
                executable: "/usr/bin/env",
                arguments: [],
                environment: .replace(["PROCESS_RUNNER_SENTINEL": "exact value"])
            )
            let environmentResult = try await ProcessRunner().run(environment, projectRoot: directory)
            XCTAssertEqual(String(decoding: environmentResult.standardOutput, as: UTF8.self), "PROCESS_RUNNER_SENTINEL=exact value\n")
        }
    }

    func testStreamsSeparateByteSafeIncrementalChannels() async throws {
        try await withTemporaryDirectory { directory in
            let recorder = ChunkRecorder()
            let plan = try DirectCommandPlan(executable: fixture("emit-log.sh").path, arguments: [])
            let result = try await ProcessRunner().run(plan, projectRoot: directory) { chunk in
                await recorder.record(chunk)
            }
            let chunks = await recorder.chunks

            XCTAssertEqual(result.termination, .exited(code: 0))
            XCTAssertEqual(String(decoding: result.standardOutput, as: UTF8.self), "stdout-one\n€ stdout-unicode\n")
            XCTAssertEqual(String(decoding: result.standardError, as: UTF8.self), "stderr-one\n漢 stderr-unicode\n")
            XCTAssertGreaterThanOrEqual(chunks.filter { $0.channel == .standardOutput }.count, 3)
            XCTAssertGreaterThanOrEqual(chunks.filter { $0.channel == .standardError }.count, 3)
            XCTAssertEqual(chunks.map(\.sequence), Array(0..<UInt64(chunks.count)))
            XCTAssertEqual(joined(chunks, channel: .standardOutput), result.standardOutput)
            XCTAssertEqual(joined(chunks, channel: .standardError), result.standardError)
        }
    }

    func testTimeoutTerminatesWithinBoundAndReportsActualSignal() async throws {
        try await withTemporaryDirectory { directory in
            let runner = ProcessRunner(terminationGracePeriod: .milliseconds(100))
            let plan = try DirectCommandPlan(executable: "/bin/sleep", arguments: ["300"])
            let clock = ContinuousClock()
            let started = clock.now
            let result = try await runner.run(plan, projectRoot: directory, timeout: .milliseconds(100))

            XCTAssertEqual(result.stopReason, .timedOut)
            XCTAssertTrue(
                result.termination == .signaled(signal: SIGTERM)
                    || result.termination == .signaled(signal: SIGKILL)
            )
            XCTAssertLessThan(started.duration(to: clock.now), .seconds(2))
        }
    }

    func testCallerCancellationIsBoundedAndIdempotent() async throws {
        try await withTemporaryDirectory { directory in
            let runner = ProcessRunner(terminationGracePeriod: .milliseconds(100))
            let plan = try DirectCommandPlan(executable: "/bin/sleep", arguments: ["300"])
            let task = Task { try await runner.run(plan, projectRoot: directory) }
            try await Task.sleep(for: .milliseconds(75))
            let clock = ContinuousClock()
            let started = clock.now
            task.cancel()
            task.cancel()
            let result = try await task.value

            XCTAssertEqual(result.stopReason, .cancelled)
            XCTAssertTrue(
                result.termination == .signaled(signal: SIGTERM)
                    || result.termination == .signaled(signal: SIGKILL)
            )
            XCTAssertLessThan(started.duration(to: clock.now), .seconds(2))
        }
    }

    func testCancellationTerminatesEntireDescendantProcessGroup() async throws {
        let directory = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let runner = ProcessRunner(terminationGracePeriod: .milliseconds(100))
        let plan = try DirectCommandPlan(executable: fixture("child-tree.sh").path, arguments: [directory.path])
        let task = Task { try await runner.run(plan, projectRoot: directory) }
        let pidFiles = ["parent.pid", "child.pid", "grandchild.pid"]
        try await waitUntil(timeout: .seconds(2)) {
            pidFiles.allSatisfy { FileManager.default.fileExists(atPath: directory.appendingPathComponent($0).path) }
        }
        let pids = try pidFiles.map { name -> Int32 in
            let value = try String(contentsOf: directory.appendingPathComponent(name), encoding: .utf8)
            return Int32(value.trimmingCharacters(in: .whitespacesAndNewlines))!
        }

        task.cancel()
        let result = try await task.value
        XCTAssertEqual(result.stopReason, .cancelled)
        try await waitUntil(timeout: .seconds(2)) { pids.allSatisfy { !processExists($0) } }
        XCTAssertTrue(pids.allSatisfy { !processExists($0) })

        try FileManager.default.removeItem(at: directory)
        XCTAssertFalse(FileManager.default.fileExists(atPath: directory.path))
    }

    func testLoginShellRequiresAndReturnsExplicitAuthority() async throws {
        try await withTemporaryDirectory { directory in
            let authority = try ShellAuthority(
                source: .userConfiguration,
                approvedByUser: true,
                disclosure: "Test explicitly permits its isolated login-shell command"
            )
            let plan = try LoginShellCommandPlan(
                shellExecutable: "/bin/sh",
                command: "printf shell-authorized",
                environment: .replace(["HOME": directory.path]),
                authority: authority
            )
            let result = try await ProcessRunner().runLoginShell(plan, projectRoot: directory)

            XCTAssertEqual(result.termination, .exited(code: 0))
            XCTAssertEqual(String(decoding: result.standardOutput, as: UTF8.self), "shell-authorized")
            XCTAssertEqual(result.shellAuthority, authority)
        }
    }

    func testMissingExecutableReturnsSpawnError() async throws {
        try await withTemporaryDirectory { directory in
            let plan = try DirectCommandPlan(executable: "definitely-not-a-real-g003-executable", arguments: [])
            do {
                _ = try await ProcessRunner().run(plan, projectRoot: directory)
                XCTFail("missing executable unexpectedly ran")
            } catch let error as ProcessRunnerError {
                XCTAssertEqual(error, .spawnFailed(code: ENOENT))
            }
        }
    }

    func testSignalTerminationIsReportedExactly() async throws {
        try await withTemporaryDirectory { directory in
            let plan = try DirectCommandPlan(executable: "/bin/sh", arguments: ["-c", "kill -TERM $$"])
            let result = try await ProcessRunner().run(plan, projectRoot: directory)
            XCTAssertEqual(result.stopReason, .completed)
            XCTAssertEqual(result.termination, .signaled(signal: SIGTERM))
        }
    }
}

private actor ChunkRecorder {
    private(set) var chunks: [ProcessOutputChunk] = []
    func record(_ chunk: ProcessOutputChunk) { chunks.append(chunk) }
}

private func joined(_ chunks: [ProcessOutputChunk], channel: ProcessOutputChannel) -> Data {
    chunks.filter { $0.channel == channel }.reduce(into: Data()) { $0.append($1.bytes) }
}

private func fixture(_ name: String) -> URL {
    var root = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 { root.deleteLastPathComponent() }
    return root.appendingPathComponent("Fixtures/process").appendingPathComponent(name)
}

private func makeTemporaryDirectory() throws -> URL {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("ProcessRunnerTests-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: false)
    return url
}

private func withTemporaryDirectory<T>(_ body: (URL) async throws -> T) async throws -> T {
    let directory = try makeTemporaryDirectory()
    defer { try? FileManager.default.removeItem(at: directory) }
    return try await body(directory)
}

private func waitUntil(timeout: Duration, condition: () -> Bool) async throws {
    let clock = ContinuousClock()
    let deadline = clock.now.advanced(by: timeout)
    while !condition(), clock.now < deadline {
        try await Task.sleep(for: .milliseconds(20))
    }
}

private func processExists(_ pid: Int32) -> Bool {
    if kill(pid, 0) == 0 { return true }
    return errno != ESRCH
}
