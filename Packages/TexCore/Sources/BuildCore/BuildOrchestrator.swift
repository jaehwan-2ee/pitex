import Foundation

public enum BuildToolStage: String, CaseIterable, Codable, Sendable {
    case xelatex
    case pdflatex
    case lualatex
    case latexmk
    case latexmkXeLaTeX
    case latexmkLuaLaTeX
    case bibtex
    case makeindex
}

public struct BuildStagePlan: Hashable, Codable, Sendable {
    public let tool: BuildToolStage

    public init(_ tool: BuildToolStage) {
        self.tool = tool
    }
}

public enum BuildPipeline: Hashable, Codable, Sendable {
    case stages([BuildStagePlan])
    case custom(LoginShellCommandPlan)

    public static func singlePass(_ tool: BuildToolStage) -> Self {
        .stages([BuildStagePlan(tool)])
    }
}

public struct BuildTarget: Hashable, Sendable {
    public let projectRoot: URL
    public let sourcePath: String
    public let outputPDFPath: String
    public let declaredGeneratedPaths: Set<String>
    public let buildDirectoryPath: String?

    public init(
        projectRoot: URL,
        sourcePath: String,
        outputPDFPath: String,
        declaredGeneratedPaths: Set<String>,
        buildDirectoryPath: String? = nil
    ) throws {
        guard projectRoot.isFileURL, projectRoot.path.hasPrefix("/") else {
            throw BuildOrchestratorError.invalidProjectRoot
        }
        try Self.validateRelative(sourcePath)
        try Self.validateRelative(outputPDFPath)
        for path in declaredGeneratedPaths { try Self.validateRelative(path) }
        if let buildDirectoryPath { try Self.validateRelative(buildDirectoryPath) }
        guard sourcePath != outputPDFPath,
              declaredGeneratedPaths.contains(outputPDFPath) else {
            throw BuildOrchestratorError.undeclaredGeneratedPath(outputPDFPath)
        }
        if let buildDirectoryPath {
            guard declaredGeneratedPaths.contains(buildDirectoryPath) else {
                throw BuildOrchestratorError.undeclaredGeneratedPath(buildDirectoryPath)
            }
            let prefix = buildDirectoryPath + "/"
            guard sourcePath != buildDirectoryPath, !sourcePath.hasPrefix(prefix) else {
                throw BuildOrchestratorError.sourceDeletionRisk
            }
        }
        self.projectRoot = projectRoot.standardizedFileURL.resolvingSymlinksInPath()
        self.sourcePath = sourcePath
        self.outputPDFPath = outputPDFPath
        self.declaredGeneratedPaths = declaredGeneratedPaths
        self.buildDirectoryPath = buildDirectoryPath
    }

    fileprivate static func validateRelative(_ path: String) throws {
        guard !path.isEmpty,
              !path.hasPrefix("/"),
              !path.hasPrefix("~"),
              !path.contains("\\"),
              !path.unicodeScalars.contains(where: { $0.value == 0 }) else {
            throw BuildOrchestratorError.pathTraversal(path)
        }
        let components = path.split(separator: "/", omittingEmptySubsequences: false)
        guard components.allSatisfy({ !$0.isEmpty && $0 != "." && $0 != ".." }),
              !(components.first?.contains(":") ?? false) else {
            throw BuildOrchestratorError.pathTraversal(path)
        }
    }

    fileprivate func url(for relativePath: String) throws -> URL {
        try Self.validateRelative(relativePath)
        let candidate = projectRoot
            .appendingPathComponent(relativePath)
            .standardizedFileURL
            .resolvingSymlinksInPath()
        let prefix = projectRoot.path == "/" ? "/" : projectRoot.path + "/"
        guard candidate.path.hasPrefix(prefix) else {
            throw BuildOrchestratorError.pathTraversal(relativePath)
        }
        return candidate
    }
}

public struct BuildProcessRequest: Sendable {
    public enum Command: Sendable {
        case direct(DirectCommandPlan)
        case loginShell(LoginShellCommandPlan)
    }

    public let buildID: BuildID
    public let stageIndex: Int
    public let command: Command
    public let projectRoot: URL
    public let sourceDirectory: URL

    public init(
        buildID: BuildID,
        stageIndex: Int,
        command: Command,
        projectRoot: URL,
        sourceDirectory: URL
    ) {
        self.buildID = buildID
        self.stageIndex = stageIndex
        self.command = command
        self.projectRoot = projectRoot
        self.sourceDirectory = sourceDirectory
    }
}

public struct BuildProcessOutput: Sendable {
    public let channel: BuildLogChannel
    public let bytes: Data

    public init(channel: BuildLogChannel, bytes: Data) {
        self.channel = channel
        self.bytes = bytes
    }
}

public struct BuildProcessResult: Hashable, Sendable {
    public let exitCode: Int32

    public init(exitCode: Int32) {
        self.exitCode = exitCode
    }
}

public protocol BuildProcessExecuting: Sendable {
    func execute(
        _ request: BuildProcessRequest,
        output: @escaping @Sendable (BuildProcessOutput) async -> Void
    ) async throws -> BuildProcessResult

    func cancel(buildID: BuildID) async
}

public enum BuildProcessExecutorError: Error, Equatable, Sendable {
    case toolNotFound(String)
    case launchFailed(String)
}

public enum BuildOrchestratorError: Error, Equatable, Sendable {
    case noSelection
    case buildAlreadyRunning
    case emptyPipeline
    case invalidProjectRoot
    case pathTraversal(String)
    case undeclaredGeneratedPath(String)
    case sourceDeletionRisk
    case processFailed(stage: Int, exitCode: Int32)
    case processError(String)
    case missingPDF
    case artifactRestorationFailed
    case cancelled
    case noBuildRunning
}

public enum BuildArtifactDisposition: Hashable, Codable, Sendable {
    case replacedWithSuccessfulPDF
    case preservedLastSuccessfulPDF(partialOutputWasDiscarded: Bool)
    case noPDF(partialOutputWasDiscarded: Bool)
}

public struct BuildOutcome: Hashable, Codable, Sendable {
    public let buildID: BuildID
    public let lifecycle: BuildLifecycle
    public let issues: [BuildIssueRecord]
    public let artifactDisposition: BuildArtifactDisposition
}

public enum BuildEvent: Sendable {
    case lifecycle(BuildLifecycle)
    case log(BuildLogEntry)
    case issue(BuildIssueRecord)
    case stageStarted(index: Int, tool: BuildToolStage?)
}

public actor BuildOrchestrator {
    public typealias EventHandler = @Sendable (BuildEvent) async -> Void

    private let executor: any BuildProcessExecuting
    private let fileManager: FileManager
    private var target: BuildTarget?
    private var pipeline: BuildPipeline?
    private var active: ActiveBuild?
    private var lastSuccessfulPDF: Data?
    private var lastSuccessfulPDFPath: URL?
    private var mostRecentOutcome: BuildOutcome?
    private var logSequence: UInt64 = 0

    public init(
        executor: any BuildProcessExecuting,
        fileManager: FileManager = .default
    ) {
        self.executor = executor
        self.fileManager = fileManager
    }

    public func select(target: BuildTarget, pipeline: BuildPipeline) throws {
        guard active == nil else { throw BuildOrchestratorError.buildAlreadyRunning }
        if case let .stages(stages) = pipeline {
            guard !stages.isEmpty else { throw BuildOrchestratorError.emptyPipeline }
        }
        self.target = target
        self.pipeline = pipeline
    }

    public func selectedTarget() -> BuildTarget? { target }
    public func selectedPipeline() -> BuildPipeline? { pipeline }
    public func currentLifecycle() -> BuildLifecycle? { active?.lifecycle }
    public func successfulPDF() -> Data? { lastSuccessfulPDF }
    public func lastOutcome() -> BuildOutcome? { mostRecentOutcome }

    public func build(
        id: BuildID,
        eventHandler: @escaping EventHandler = { _ in }
    ) async throws -> BuildOutcome {
        guard active == nil else { throw BuildOrchestratorError.buildAlreadyRunning }
        guard let target, let pipeline else { throw BuildOrchestratorError.noSelection }

        let started = nowMilliseconds()
        active = ActiveBuild(
            id: id,
            lifecycle: .running(startedAtMilliseconds: started),
            cancellationRequested: false,
            parser: BuildLogParser(projectRoot: target.projectRoot),
            issues: [],
            issueKeys: [],
            eventHandler: eventHandler
        )
        logSequence = 0
        await eventHandler(.lifecycle(.running(startedAtMilliseconds: started)))

        let outputURL = try target.url(for: target.outputPDFPath)
        var preexistingPDF: Data?
        if fileManager.fileExists(atPath: outputURL.path) {
            do {
                preexistingPDF = try Data(contentsOf: outputURL)
            } catch {
                let lifecycle = BuildLifecycle.failed(
                    exitCode: nil,
                    finishedAtMilliseconds: nowMilliseconds()
                )
                _ = await finish(
                    id: id,
                    lifecycle: lifecycle,
                    disposition: .noPDF(partialOutputWasDiscarded: false),
                    eventHandler: eventHandler
                )
                throw BuildOrchestratorError.processError(
                    "Unable to preserve the existing PDF before building"
                )
            }
        } else {
            preexistingPDF = nil
        }
        if lastSuccessfulPDFPath == outputURL, let lastSuccessfulPDF {
            preexistingPDF = lastSuccessfulPDF
        }
        var stageFailure: BuildOrchestratorError?

        do {
            switch pipeline {
            case let .stages(stages):
                for (index, stage) in stages.enumerated() {
                    try ensureNotCancelled(id: id)
                    await eventHandler(.stageStarted(index: index, tool: stage.tool))
                    let plan = try commandPlan(for: stage.tool, target: target)
                    let request = BuildProcessRequest(
                        buildID: id,
                        stageIndex: index,
                        command: .direct(plan),
                        projectRoot: target.projectRoot,
                        sourceDirectory: try sourceDirectory(for: target)
                    )
                    let result = try await executor.execute(request) { output in
                        await self.receive(output, for: id, eventHandler: eventHandler)
                    }
                    try ensureNotCancelled(id: id)
                    guard result.exitCode == 0 else {
                        throw BuildOrchestratorError.processFailed(stage: index, exitCode: result.exitCode)
                    }
                }
            case let .custom(command):
                try ensureNotCancelled(id: id)
                await eventHandler(.stageStarted(index: 0, tool: nil))
                let request = BuildProcessRequest(
                    buildID: id,
                    stageIndex: 0,
                    command: .loginShell(command),
                    projectRoot: target.projectRoot,
                    sourceDirectory: try sourceDirectory(for: target)
                )
                let result = try await executor.execute(request) { output in
                    await self.receive(output, for: id, eventHandler: eventHandler)
                }
                try ensureNotCancelled(id: id)
                guard result.exitCode == 0 else {
                    throw BuildOrchestratorError.processFailed(stage: 0, exitCode: result.exitCode)
                }
            }
        } catch let error as BuildOrchestratorError {
            stageFailure = error
        } catch let error as BuildProcessExecutorError {
            stageFailure = .processError(String(describing: error))
        } catch {
            stageFailure = .processError(String(describing: error))
        }

        await flushParser(for: id, eventHandler: eventHandler)
        let partial = outputChanged(at: outputURL, comparedWith: preexistingPDF)
        let cancelled = active?.cancellationRequested == true

        if cancelled {
            let disposition = try await restoreForBuild(
                id: id,
                previous: preexistingPDF,
                outputURL: outputURL,
                partial: partial,
                eventHandler: eventHandler
            )
            let lifecycle = BuildLifecycle.cancelled(finishedAtMilliseconds: nowMilliseconds())
            return await finish(id: id, lifecycle: lifecycle, disposition: disposition, eventHandler: eventHandler)
        }
        if stageFailure != nil {
            let disposition = try await restoreForBuild(
                id: id,
                previous: preexistingPDF,
                outputURL: outputURL,
                partial: partial,
                eventHandler: eventHandler
            )
            let lifecycle = BuildLifecycle.failed(exitCode: failureExitCode(stageFailure), finishedAtMilliseconds: nowMilliseconds())
            _ = await finish(id: id, lifecycle: lifecycle, disposition: disposition, eventHandler: eventHandler)
            throw stageFailure!
        }
        guard let successfulData = try? Data(contentsOf: outputURL) else {
            let disposition = try await restoreForBuild(
                id: id,
                previous: preexistingPDF,
                outputURL: outputURL,
                partial: partial,
                eventHandler: eventHandler
            )
            let lifecycle = BuildLifecycle.failed(exitCode: nil, finishedAtMilliseconds: nowMilliseconds())
            _ = await finish(id: id, lifecycle: lifecycle, disposition: disposition, eventHandler: eventHandler)
            throw BuildOrchestratorError.missingPDF
        }

        lastSuccessfulPDF = successfulData
        lastSuccessfulPDFPath = outputURL
        let lifecycle = BuildLifecycle.succeeded(exitCode: 0, finishedAtMilliseconds: nowMilliseconds())
        return await finish(
            id: id,
            lifecycle: lifecycle,
            disposition: .replacedWithSuccessfulPDF,
            eventHandler: eventHandler
        )
    }

    public func cancel(
        gracePeriodMilliseconds: UInt64 = 1_000
    ) async throws {
        guard var active else { throw BuildOrchestratorError.noBuildRunning }
        guard !active.cancellationRequested else { return }
        _ = try BuildCancellation(
            requestedAtMilliseconds: nowMilliseconds(),
            gracePeriodMilliseconds: gracePeriodMilliseconds
        )
        active.cancellationRequested = true
        active.lifecycle = .cancelling
        self.active = active
        await active.eventHandler(.lifecycle(.cancelling))
        await executor.cancel(buildID: active.id)
    }

    @discardableResult
    public func cleanup(_ policy: CleanupPolicy) throws -> [String] {
        guard active == nil else { throw BuildOrchestratorError.buildAlreadyRunning }
        guard let target else { throw BuildOrchestratorError.noSelection }
        let candidates: Set<String>
        switch policy {
        case .preserveAll:
            return []
        case let .removeKnownAuxiliaryFiles(paths):
            candidates = paths
        case .removeBuildDirectory:
            guard let directory = target.buildDirectoryPath else { return [] }
            candidates = [directory]
        }

        var removed: [String] = []
        for path in candidates.sorted() {
            try BuildTarget.validateRelative(path)
            guard target.declaredGeneratedPaths.contains(path) else {
                throw BuildOrchestratorError.undeclaredGeneratedPath(path)
            }
            guard path != target.sourcePath, path != target.outputPDFPath else {
                if path == target.sourcePath { throw BuildOrchestratorError.sourceDeletionRisk }
                continue
            }
            let url = try target.url(for: path)
            if fileManager.fileExists(atPath: url.path) {
                try fileManager.removeItem(at: url)
                removed.append(path)
            }
        }
        return removed
    }

    private func receive(
        _ output: BuildProcessOutput,
        for id: BuildID,
        eventHandler: EventHandler
    ) async {
        guard self.active?.id == id else { return }
        let text = String(decoding: output.bytes, as: UTF8.self)
        let log = BuildLogEntry(sequence: logSequence, channel: output.channel, text: text)
        logSequence += 1
        let parsed = self.active?.parser.consume(output.bytes, channel: output.channel) ?? []
        await eventHandler(.log(log))
        await emit(parsed, for: id, eventHandler: eventHandler)
    }

    private func flushParser(for id: BuildID, eventHandler: EventHandler) async {
        guard self.active?.id == id else { return }
        let parsed = self.active?.parser.finish() ?? []
        await emit(parsed, for: id, eventHandler: eventHandler)
    }

    private func emit(
        _ records: [BuildIssueRecord],
        for id: BuildID,
        eventHandler: EventHandler
    ) async {
        // Mutating self.active in place keeps the issue list and key set
        // copy-on-write unique — a local copy per record made each append
        // O(issues) and a warning-heavy build quadratic.
        for record in records {
            guard self.active?.id == id else { return }
            let key = IssueKey(record)
            guard self.active?.issueKeys.insert(key).inserted == true else { continue }
            self.active?.issues.append(record)
            await eventHandler(.issue(record))
        }
    }

    private func finish(
        id: BuildID,
        lifecycle: BuildLifecycle,
        disposition: BuildArtifactDisposition,
        eventHandler: EventHandler
    ) async -> BuildOutcome {
        let issues = active?.id == id ? active?.issues ?? [] : []
        active = nil
        let outcome = BuildOutcome(
            buildID: id,
            lifecycle: lifecycle,
            issues: issues,
            artifactDisposition: disposition
        )
        mostRecentOutcome = outcome
        await eventHandler(.lifecycle(lifecycle))
        return outcome
    }

    private func ensureNotCancelled(id: BuildID) throws {
        guard let active, active.id == id, !active.cancellationRequested else {
            throw BuildOrchestratorError.cancelled
        }
    }

    private func commandPlan(for tool: BuildToolStage, target: BuildTarget) throws -> DirectCommandPlan {
        switch tool {
        case .xelatex:
            return try DirectCommandPlan(executable: "xelatex", arguments: ["-synctex=1", "-interaction=nonstopmode", "-file-line-error", target.sourcePath])
        case .pdflatex:
            return try DirectCommandPlan(executable: "pdflatex", arguments: ["-synctex=1", "-interaction=nonstopmode", "-file-line-error", target.sourcePath])
        case .lualatex:
            return try DirectCommandPlan(executable: "lualatex", arguments: ["-synctex=1", "--interaction=nonstopmode", "--file-line-error", target.sourcePath])
        case .latexmk, .latexmkXeLaTeX, .latexmkLuaLaTeX:
            let engine = tool == .latexmkXeLaTeX ? "-pdfxe" : (tool == .latexmkLuaLaTeX ? "-pdflua" : "-pdf")
            // latexmk owns bibliography/index dependencies and reruns. -cd
            // keeps relative inputs and output files beside the main source,
            // even when the workspace contains a nested manuscript directory.
            return try DirectCommandPlan(executable: "latexmk", arguments: [engine, "-cd", "-synctex=1", "-interaction=nonstopmode", "-file-line-error", target.sourcePath])
        case .bibtex:
            return try DirectCommandPlan(executable: "bibtex", arguments: [jobName(target.sourcePath)])
        case .makeindex:
            return try DirectCommandPlan(executable: "makeindex", arguments: [jobName(target.sourcePath)])
        }
    }

    private func jobName(_ sourcePath: String) -> String {
        let source = sourcePath as NSString
        let directory = source.deletingLastPathComponent
        let base = source.lastPathComponent as NSString
        let stem = base.deletingPathExtension
        return directory.isEmpty ? stem : directory + "/" + stem
    }

    private func sourceDirectory(for target: BuildTarget) throws -> URL {
        let sourceURL = try target.url(for: target.sourcePath)
        return sourceURL.deletingLastPathComponent()
    }

    private func outputChanged(at url: URL, comparedWith previous: Data?) -> Bool {
        let current = try? Data(contentsOf: url)
        return current != previous
    }

    private func restore(
        _ previous: Data?,
        at outputURL: URL,
        partial: Bool
    ) throws -> BuildArtifactDisposition {
        if partial {
            do {
                if let previous {
                    try previous.write(to: outputURL, options: .atomic)
                } else if fileManager.fileExists(atPath: outputURL.path) {
                    try fileManager.removeItem(at: outputURL)
                }
            } catch {
                throw BuildOrchestratorError.artifactRestorationFailed
            }
        }
        if previous != nil || lastSuccessfulPDFPath == outputURL {
            return .preservedLastSuccessfulPDF(partialOutputWasDiscarded: partial)
        }
        return .noPDF(partialOutputWasDiscarded: partial)
    }

    private func restoreForBuild(
        id: BuildID,
        previous: Data?,
        outputURL: URL,
        partial: Bool,
        eventHandler: EventHandler
    ) async throws -> BuildArtifactDisposition {
        do {
            return try restore(previous, at: outputURL, partial: partial)
        } catch {
            let lifecycle = BuildLifecycle.failed(
                exitCode: nil,
                finishedAtMilliseconds: nowMilliseconds()
            )
            _ = await finish(
                id: id,
                lifecycle: lifecycle,
                disposition: previous == nil
                    ? .noPDF(partialOutputWasDiscarded: partial)
                    : .preservedLastSuccessfulPDF(partialOutputWasDiscarded: partial),
                eventHandler: eventHandler
            )
            throw error
        }
    }

    private func failureExitCode(_ error: BuildOrchestratorError?) -> Int32? {
        if case let .processFailed(_, exitCode) = error { return exitCode }
        return nil
    }

    private func nowMilliseconds() -> UInt64 {
        let milliseconds = Date().timeIntervalSince1970 * 1_000
        return milliseconds > 0 ? UInt64(milliseconds) : 0
    }

    private struct ActiveBuild: Sendable {
        let id: BuildID
        var lifecycle: BuildLifecycle
        var cancellationRequested: Bool
        var parser: BuildLogParser
        var issues: [BuildIssueRecord]
        var issueKeys: Set<IssueKey>
        let eventHandler: EventHandler
    }

    private struct IssueKey: Hashable, Sendable {
        let severity: BuildIssueSeverity
        let message: String
        let file: String?
        let line: Int?
        let column: Int?

        init(_ record: BuildIssueRecord) {
            severity = record.severity
            message = record.message
            file = record.file
            line = record.line
            column = record.column
        }
    }
}
