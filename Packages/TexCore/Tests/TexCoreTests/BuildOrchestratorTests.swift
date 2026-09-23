import BuildCore
import Foundation
import XCTest

final class BuildOrchestratorTests: XCTestCase {
    func testSuccessfulMultiPassBuildRunsInOrderAndPublishesOrderedLogs() async throws {
        let fixture = try Fixture()
        let executor = FakeExecutor(behaviors: [
            .success(chunks: [chunk("first\n")], pdf: nil),
            .success(chunks: [chunk("second\n")], pdf: nil),
            .success(chunks: [chunk("third\n")], pdf: Data("pdf-v1".utf8))
        ])
        let orchestrator = BuildOrchestrator(executor: executor)
        try await orchestrator.select(
            target: fixture.target,
            pipeline: .stages([.init(.pdflatex), .init(.bibtex), .init(.pdflatex)])
        )
        let events = EventRecorder()

        let outcome = try await orchestrator.build(id: try BuildID(rawValue: "multi")) {
            await events.append($0)
        }

        let executables = await executor.executables()
        let logSequences = await events.logSequences()
        let successfulPDF = await orchestrator.successfulPDF()
        XCTAssertEqual(executables, ["pdflatex", "bibtex", "pdflatex"])
        XCTAssertEqual(logSequences, [0, 1, 2])
        XCTAssertEqual(outcome.lifecycle.exitCode, 0)
        XCTAssertEqual(outcome.artifactDisposition, .replacedWithSuccessfulPDF)
        XCTAssertEqual(successfulPDF, Data("pdf-v1".utf8))
    }

    func testToolMissingFailsClosedWithoutTryingAnotherStage() async throws {
        let fixture = try Fixture()
        let executor = FakeExecutor(behaviors: [.toolMissing("xelatex")])
        let orchestrator = BuildOrchestrator(executor: executor)
        try await orchestrator.select(
            target: fixture.target,
            pipeline: .stages([.init(.xelatex), .init(.pdflatex)])
        )

        await XCTAssertThrowsErrorAsync(
            try await orchestrator.build(id: try BuildID(rawValue: "missing"))
        ) { error in
            guard case BuildOrchestratorError.processError = error else {
                return XCTFail("Unexpected error: \(error)")
            }
        }
        let requestCount = await executor.requestCount()
        XCTAssertEqual(requestCount, 1)
    }

    func testParserHandlesEveryUTF8AndLineBoundarySplit() throws {
        let root = URL(fileURLWithPath: "/tmp/project")
        let bytes = Array("main.tex:12:4: warning: café 漢字\n".utf8)

        for split in 0...bytes.count {
            var parser = BuildLogParser(projectRoot: root)
            var issues = parser.consume(Data(bytes[..<split]), channel: .standardError)
            issues += parser.consume(Data(bytes[split...]), channel: .standardError)
            issues += parser.finish()
            XCTAssertEqual(issues.count, 1, "split \(split)")
            XCTAssertEqual(issues[0].message, "warning: café 漢字")
            XCTAssertEqual(issues[0].file, "main.tex")
            XCTAssertEqual(issues[0].line, 12)
            XCTAssertEqual(issues[0].column, 4)
            XCTAssertTrue(issues[0].isClickable)
        }
    }

    /// consume() must produce identical records whether the log arrives
    /// whole or chunked at any byte boundary — including splits inside a
    /// multi-byte UTF-8 sequence and between \r and \n.
    func testParserChunkBoundariesNeverChangeRecords() {
        let root = URL(fileURLWithPath: "/tmp/project")
        let log = "(./main.tex\n(./chapters/intro.tex\r\n"
            + "! Undefined control sequence.\r\nl.12 \\bad\r\n"
            + "Overfull \\hbox (12.0pt too wide) at lines 5--6\n"
            + "./main.tex:9:2: warning: café 한국어\n"
            + "Underfull \\vbox detected at line 3\r\n"
            + ")\n"
        var reference = BuildLogParser(projectRoot: root)
        var expected = reference.consume(Data(log.utf8), channel: .standardOutput)
        expected += reference.finish()

        let bytes = Array(log.utf8)
        for size in 1...64 {
            var parser = BuildLogParser(projectRoot: root)
            var issues: [BuildIssueRecord] = []
            var offset = 0
            while offset < bytes.count {
                let end = min(offset + size, bytes.count)
                issues += parser.consume(Data(bytes[offset..<end]), channel: .standardOutput)
                offset = end
            }
            issues += parser.finish()
            XCTAssertEqual(issues, expected, "chunk size \(size)")
        }
    }

    func testParserTracksNestedFilesClassicErrorsWarningsAndControlCharacters() {
        var parser = BuildLogParser(projectRoot: URL(fileURLWithPath: "/work/project"))
        var issues = parser.consume("(./main.tex\n(./chapters/one.tex\n! Undefined control sequence.\nl.27 \\bad\n)\nOverfull \\hbox (2.0pt too wide) at lines 40--41\nWarning--empty journal\u{1B}\n", channel: .standardOutput)
        issues += parser.finish()

        XCTAssertEqual(issues.map(\.severity), [.error, .warning, .warning])
        XCTAssertEqual(issues[0].file, "chapters/one.tex")
        XCTAssertEqual(issues[0].line, 27)
        XCTAssertEqual(issues[1].file, "main.tex")
        XCTAssertEqual(issues[1].line, 40)
        XCTAssertTrue(issues[2].message.contains("\\u{1B}"))
    }

    func testDuplicateIssuesAreEmittedOnce() async throws {
        let fixture = try Fixture()
        let line = chunk("main.tex:3: warning: duplicate\n", channel: .standardError)
        let executor = FakeExecutor(behaviors: [
            .success(chunks: [line, line], pdf: Data("pdf".utf8))
        ])
        let orchestrator = BuildOrchestrator(executor: executor)
        try await orchestrator.select(target: fixture.target, pipeline: .singlePass(.pdflatex))
        let outcome = try await orchestrator.build(id: try BuildID(rawValue: "dedupe"))
        XCTAssertEqual(outcome.issues.count, 1)
    }

    func testCancellationWinsRaceAndProhibitsRemainingPasses() async throws {
        let fixture = try Fixture()
        let executor = FakeExecutor(behaviors: [
            .waitForCancellation(partialPDF: Data("partial".utf8)),
            .success(chunks: [], pdf: Data("must-not-run".utf8))
        ])
        let orchestrator = BuildOrchestrator(executor: executor)
        try await orchestrator.select(
            target: fixture.target,
            pipeline: .stages([.init(.pdflatex), .init(.pdflatex)])
        )
        let id = try BuildID(rawValue: "cancel")
        let task = Task { try await orchestrator.build(id: id) }
        await executor.waitUntilStarted()
        try await orchestrator.cancel(gracePeriodMilliseconds: 0)

        let outcome = try await task.value
        guard case .cancelled = outcome.lifecycle else {
            return XCTFail("Cancellation must win over the child result")
        }
        let requestCount = await executor.requestCount()
        XCTAssertEqual(requestCount, 1)
        XCTAssertFalse(FileManager.default.fileExists(atPath: fixture.outputURL.path))
    }

    func testPartialPDFCannotReplaceLastSuccessfulPDF() async throws {
        let fixture = try Fixture()
        let executor = FakeExecutor(behaviors: [
            .success(chunks: [], pdf: Data("good".utf8)),
            .failure(exitCode: 1, chunks: [], partialPDF: Data("bad".utf8))
        ])
        let orchestrator = BuildOrchestrator(executor: executor)
        try await orchestrator.select(target: fixture.target, pipeline: .singlePass(.pdflatex))
        _ = try await orchestrator.build(id: try BuildID(rawValue: "good"))

        await XCTAssertThrowsErrorAsync(
            try await orchestrator.build(id: try BuildID(rawValue: "bad"))
        ) { error in
            XCTAssertEqual(error as? BuildOrchestratorError, .processFailed(stage: 0, exitCode: 1))
        }
        XCTAssertEqual(try Data(contentsOf: fixture.outputURL), Data("good".utf8))
        let successfulPDF = await orchestrator.successfulPDF()
        let failedOutcome = await orchestrator.lastOutcome()
        XCTAssertEqual(successfulPDF, Data("good".utf8))
        XCTAssertEqual(
            failedOutcome?.artifactDisposition,
            .preservedLastSuccessfulPDF(partialOutputWasDiscarded: true)
        )
    }

    func testCustomLoginShellCommandIsAuthoritativeAndRunsExactlyOnce() async throws {
        let fixture = try Fixture()
        let authority = try ShellAuthority(
            source: .userConfiguration,
            approvedByUser: true,
            disclosure: "Test-approved custom build"
        )
        let command = try LoginShellCommandPlan(
            shellExecutable: "/bin/sh",
            command: "printf custom",
            authority: authority
        )
        let executor = FakeExecutor(behaviors: [
            .success(chunks: [], pdf: Data("custom-pdf".utf8))
        ])
        let orchestrator = BuildOrchestrator(executor: executor)
        try await orchestrator.select(target: fixture.target, pipeline: .custom(command))

        _ = try await orchestrator.build(id: try BuildID(rawValue: "custom"))

        let requestCount = await executor.requestCount()
        let shellCommands = await executor.shellCommands()
        let executables = await executor.executables()
        XCTAssertEqual(requestCount, 1)
        XCTAssertEqual(shellCommands, ["printf custom"])
        XCTAssertEqual(executables, [])
    }

    func testCleanupOnlyRemovesDeclaredGeneratedChildren() async throws {
        let fixture = try Fixture(extraGenerated: ["main.aux", "build"])
        try Data("aux".utf8).write(to: fixture.root.appendingPathComponent("main.aux"))
        try FileManager.default.createDirectory(
            at: fixture.root.appendingPathComponent("build"),
            withIntermediateDirectories: false
        )
        let executor = FakeExecutor(behaviors: [])
        let orchestrator = BuildOrchestrator(executor: executor)
        try await orchestrator.select(target: fixture.target, pipeline: .singlePass(.pdflatex))

        let removed = try await orchestrator.cleanup(.removeKnownAuxiliaryFiles(["main.aux"]))
        XCTAssertEqual(removed, ["main.aux"])
        await XCTAssertThrowsErrorAsync(
            try await orchestrator.cleanup(.removeKnownAuxiliaryFiles(["main.tex"]))
        ) { error in
            XCTAssertEqual(error as? BuildOrchestratorError, .undeclaredGeneratedPath("main.tex"))
        }
        await XCTAssertThrowsErrorAsync(
            try await orchestrator.cleanup(.removeKnownAuxiliaryFiles(["../outside.aux"]))
        ) { error in
            XCTAssertEqual(error as? BuildOrchestratorError, .pathTraversal("../outside.aux"))
        }
        XCTAssertTrue(FileManager.default.fileExists(atPath: fixture.sourceURL.path))
    }

    func testTargetRejectsTraversalAbsolutePathsAndSourceContainingBuildDirectory() throws {
        let root = URL(fileURLWithPath: "/tmp/project")
        XCTAssertThrowsError(try BuildTarget(
            projectRoot: root,
            sourcePath: "../main.tex",
            outputPDFPath: "main.pdf",
            declaredGeneratedPaths: ["main.pdf"]
        ))
        XCTAssertThrowsError(try BuildTarget(
            projectRoot: root,
            sourcePath: "main.tex",
            outputPDFPath: "/tmp/main.pdf",
            declaredGeneratedPaths: ["/tmp/main.pdf"]
        ))
        XCTAssertThrowsError(try BuildTarget(
            projectRoot: root,
            sourcePath: "build/main.tex",
            outputPDFPath: "build/main.pdf",
            declaredGeneratedPaths: ["build", "build/main.pdf"],
            buildDirectoryPath: "build"
        ))
    }
}

private struct Fixture {
    let root: URL
    let sourceURL: URL
    let outputURL: URL
    let target: BuildTarget

    init(extraGenerated: Set<String> = []) throws {
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("BuildOrchestratorTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
        sourceURL = root.appendingPathComponent("main.tex")
        outputURL = root.appendingPathComponent("main.pdf")
        try Data("source".utf8).write(to: sourceURL)
        target = try BuildTarget(
            projectRoot: root,
            sourcePath: "main.tex",
            outputPDFPath: "main.pdf",
            declaredGeneratedPaths: extraGenerated.union(["main.pdf"]),
            buildDirectoryPath: extraGenerated.contains("build") ? "build" : nil
        )
    }
}

private actor FakeExecutor: BuildProcessExecuting {
    enum Behavior: Sendable {
        case success(chunks: [BuildProcessOutput], pdf: Data?)
        case failure(exitCode: Int32, chunks: [BuildProcessOutput], partialPDF: Data?)
        case toolMissing(String)
        case waitForCancellation(partialPDF: Data?)
    }

    private var behaviors: [Behavior]
    private var requests: [BuildProcessRequest] = []
    private var pending: CheckedContinuation<BuildProcessResult, any Error>?
    private var pendingRequest: BuildProcessRequest?
    private var pendingPartialPDF: Data?
    private var startedWaiters: [CheckedContinuation<Void, Never>] = []

    init(behaviors: [Behavior]) {
        self.behaviors = behaviors
    }

    func execute(
        _ request: BuildProcessRequest,
        output: @escaping @Sendable (BuildProcessOutput) async -> Void
    ) async throws -> BuildProcessResult {
        requests.append(request)
        guard !behaviors.isEmpty else { throw BuildProcessExecutorError.launchFailed("unexpected request") }
        let behavior = behaviors.removeFirst()
        switch behavior {
        case let .success(chunks, pdf):
            for chunk in chunks { await output(chunk) }
            try write(pdf, for: request)
            return BuildProcessResult(exitCode: 0)
        case let .failure(exitCode, chunks, partialPDF):
            for chunk in chunks { await output(chunk) }
            try write(partialPDF, for: request)
            return BuildProcessResult(exitCode: exitCode)
        case let .toolMissing(tool):
            throw BuildProcessExecutorError.toolNotFound(tool)
        case let .waitForCancellation(partialPDF):
            pendingRequest = request
            pendingPartialPDF = partialPDF
            let waiters = startedWaiters
            startedWaiters.removeAll()
            for waiter in waiters { waiter.resume() }
            return try await withCheckedThrowingContinuation { continuation in
                pending = continuation
            }
        }
    }

    func cancel(buildID: BuildID) async {
        guard let pendingRequest, pendingRequest.buildID == buildID, let pending else { return }
        try? write(pendingPartialPDF, for: pendingRequest)
        self.pending = nil
        self.pendingRequest = nil
        pendingPartialPDF = nil
        pending.resume(returning: BuildProcessResult(exitCode: 0))
    }

    func waitUntilStarted() async {
        if pendingRequest != nil { return }
        await withCheckedContinuation { continuation in
            startedWaiters.append(continuation)
        }
    }

    func requestCount() -> Int { requests.count }

    func executables() -> [String] {
        requests.compactMap {
            if case let .direct(plan) = $0.command { return plan.executable }
            return nil
        }
    }

    func shellCommands() -> [String] {
        requests.compactMap {
            if case let .loginShell(plan) = $0.command { return plan.command }
            return nil
        }
    }

    private func write(_ data: Data?, for request: BuildProcessRequest) throws {
        guard let data else { return }
        try data.write(to: request.projectRoot.appendingPathComponent("main.pdf"), options: .atomic)
    }
}

private actor EventRecorder {
    private var events: [BuildEvent] = []
    func append(_ event: BuildEvent) { events.append(event) }
    func logSequences() -> [UInt64] {
        events.compactMap {
            if case let .log(entry) = $0 { return entry.sequence }
            return nil
        }
    }
}

private func chunk(
    _ text: String,
    channel: BuildLogChannel = .standardOutput
) -> BuildProcessOutput {
    BuildProcessOutput(channel: channel, bytes: Data(text.utf8))
}

private extension BuildLifecycle {
    var exitCode: Int32? {
        if case let .succeeded(exitCode, _) = self { return exitCode }
        return nil
    }
}

private func XCTAssertThrowsErrorAsync<T>(
    _ expression: @autoclosure () async throws -> T,
    _ errorHandler: (any Error) -> Void = { _ in }
) async {
    do {
        _ = try await expression()
        XCTFail("Expected expression to throw")
    } catch {
        errorHandler(error)
    }
}
