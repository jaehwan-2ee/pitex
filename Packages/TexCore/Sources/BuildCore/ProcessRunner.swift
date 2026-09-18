import Foundation

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

public enum ProcessRunnerError: Error, Equatable, Sendable {
    case invalidWorkingDirectory(String)
    case spawnFailed(code: Int32)
    case pipeFailed(code: Int32)
    case waitFailed(code: Int32)
}

public enum ProcessOutputChannel: String, Codable, Sendable {
    case standardOutput
    case standardError
}

public struct ProcessOutputChunk: Sendable {
    public let sequence: UInt64
    public let channel: ProcessOutputChannel
    public let bytes: Data

    public init(sequence: UInt64, channel: ProcessOutputChannel, bytes: Data) {
        self.sequence = sequence
        self.channel = channel
        self.bytes = bytes
    }
}

public enum ProcessTermination: Equatable, Sendable {
    case exited(code: Int32)
    case signaled(signal: Int32)
}

public enum ProcessStopReason: Equatable, Sendable {
    case completed
    case timedOut
    case cancelled
}

public struct ProcessResult: Sendable {
    public let processIdentifier: Int32
    public let termination: ProcessTermination
    public let stopReason: ProcessStopReason
    public let standardOutput: Data
    public let standardError: Data
    public let shellAuthority: ShellAuthority?

    public init(
        processIdentifier: Int32,
        termination: ProcessTermination,
        stopReason: ProcessStopReason,
        standardOutput: Data,
        standardError: Data,
        shellAuthority: ShellAuthority?
    ) {
        self.processIdentifier = processIdentifier
        self.termination = termination
        self.stopReason = stopReason
        self.standardOutput = standardOutput
        self.standardError = standardError
        self.shellAuthority = shellAuthority
    }
}

public struct ProcessRunner: Sendable {
    public typealias OutputHandler = @Sendable (ProcessOutputChunk) async -> Void

    public let terminationGracePeriod: Duration

    public init(terminationGracePeriod: Duration = .milliseconds(500)) {
        self.terminationGracePeriod = terminationGracePeriod
    }

    public func run(
        _ plan: DirectCommandPlan,
        projectRoot: URL,
        sourceDirectory: URL? = nil,
        timeout: Duration? = nil,
        outputHandler: OutputHandler? = nil
    ) async throws -> ProcessResult {
        try await runInvocation(
            executable: plan.executable,
            arguments: plan.arguments,
            workingDirectory: plan.workingDirectory,
            environment: plan.environment,
            projectRoot: projectRoot,
            sourceDirectory: sourceDirectory,
            timeout: timeout,
            shellAuthority: nil,
            outputHandler: outputHandler
        )
    }

    /// This is the only API that interprets command text. The authority is returned with the result.
    public func runLoginShell(
        _ plan: LoginShellCommandPlan,
        projectRoot: URL,
        sourceDirectory: URL? = nil,
        timeout: Duration? = nil,
        outputHandler: OutputHandler? = nil
    ) async throws -> ProcessResult {
        try await runInvocation(
            executable: plan.shellExecutable,
            arguments: plan.invocationArguments,
            workingDirectory: plan.workingDirectory,
            environment: plan.environment,
            projectRoot: projectRoot,
            sourceDirectory: sourceDirectory,
            timeout: timeout,
            shellAuthority: plan.authority,
            outputHandler: outputHandler
        )
    }

    private func runInvocation(
        executable: String,
        arguments: [String],
        workingDirectory: WorkingDirectoryPolicy,
        environment: EnvironmentPolicy,
        projectRoot: URL,
        sourceDirectory: URL?,
        timeout: Duration?,
        shellAuthority: ShellAuthority?,
        outputHandler: OutputHandler?
    ) async throws -> ProcessResult {
        let directory = try resolveWorkingDirectory(
            workingDirectory,
            projectRoot: projectRoot,
            sourceDirectory: sourceDirectory
        )
        let environmentValues = resolvedEnvironment(environment)
        let spawned = try spawn(
            executable: try resolvedExecutable(executable, environment: environmentValues, directory: directory),
            arguments: arguments,
            directory: directory.path,
            environment: environmentValues
        )
        let control = ProcessControl(pid: spawned.pid, gracePeriod: terminationGracePeriod)
        let sequencer = OutputSequencer(handler: outputHandler)
        let standardOutputTask = makeReader(fd: spawned.standardOutput, channel: .standardOutput, sequencer: sequencer)
        let standardErrorTask = makeReader(fd: spawned.standardError, channel: .standardError, sequencer: sequencer)
        let waitTask = Task.detached(priority: nil) { try waitForProcess(spawned.pid) }
        let timeoutTask: Task<Void, Never>? = timeout.map { duration in
            Task.detached {
                do {
                    try await Task.sleep(for: duration)
                    await control.requestStop(.timedOut)
                } catch {}
            }
        }

        return try await withTaskCancellationHandler {
            let status: Int32
            do {
                status = try await waitTask.value
            } catch {
                timeoutTask?.cancel()
                await control.processDidExit()
                _ = await standardOutputTask.value
                _ = await standardErrorTask.value
                throw error
            }
            timeoutTask?.cancel()
            let reason = await control.processDidExit()
            let standardOutput = await standardOutputTask.value
            let standardError = await standardErrorTask.value
            return ProcessResult(
                processIdentifier: spawned.pid,
                termination: decodeWaitStatus(status),
                stopReason: reason,
                standardOutput: standardOutput,
                standardError: standardError,
                shellAuthority: shellAuthority
            )
        } onCancel: {
            Task { await control.requestStop(.cancelled) }
        }
    }
}

private struct SpawnedProcess {
    let pid: pid_t
    let standardOutput: Int32
    let standardError: Int32
}

private func resolveWorkingDirectory(
    _ policy: WorkingDirectoryPolicy,
    projectRoot: URL,
    sourceDirectory: URL?
) throws -> URL {
    let url: URL
    switch policy {
    case .projectRoot:
        url = projectRoot
    case .sourceDirectory:
        guard let sourceDirectory else {
            throw ProcessRunnerError.invalidWorkingDirectory("source directory was not supplied")
        }
        url = sourceDirectory
    case let .explicit(path):
        if path.hasPrefix("/") {
            url = URL(fileURLWithPath: path, isDirectory: true)
        } else {
            url = projectRoot.appendingPathComponent(path, isDirectory: true)
        }
    }

    let normalized = url.standardizedFileURL
    var isDirectory = ObjCBool(false)
    guard FileManager.default.fileExists(atPath: normalized.path, isDirectory: &isDirectory), isDirectory.boolValue else {
        throw ProcessRunnerError.invalidWorkingDirectory(normalized.path)
    }
    return normalized
}

private func resolvedEnvironment(_ policy: EnvironmentPolicy) -> [String: String] {
    switch policy {
    case let .inherit(overrides):
        return ProcessInfo.processInfo.environment.merging(overrides) { _, replacement in replacement }
    case let .replace(values):
        return values
    }
}

/// posix_spawnp searches the parent's PATH, ignoring PATH in its envp. Resolve
/// against the command's environment so GUI launches and overrides agree.
private func resolvedExecutable(_ executable: String, environment: [String: String], directory: URL) throws -> String {
    if executable.contains("/") { return executable }
    var errorCode = ENOENT
    for entry in (environment["PATH"] ?? "/usr/bin:/bin").split(separator: ":", omittingEmptySubsequences: false) {
        let folder = entry.isEmpty ? directory : URL(fileURLWithPath: String(entry), isDirectory: true, relativeTo: directory)
        let candidate = folder.appendingPathComponent(executable).absoluteURL.path
        var isDirectory: ObjCBool = false
        if FileManager.default.fileExists(atPath: candidate, isDirectory: &isDirectory) {
            if !isDirectory.boolValue && FileManager.default.isExecutableFile(atPath: candidate) { return candidate }
            errorCode = EACCES
        }
    }
    throw ProcessRunnerError.spawnFailed(code: errorCode)
}

private func spawn(
    executable: String,
    arguments: [String],
    directory: String,
    environment: [String: String]
) throws -> SpawnedProcess {
    var outputPipe = [Int32](repeating: -1, count: 2)
    var errorPipe = [Int32](repeating: -1, count: 2)
    guard pipe(&outputPipe) == 0 else { throw ProcessRunnerError.pipeFailed(code: errno) }
    guard pipe(&errorPipe) == 0 else {
        let savedError = errno
        close(outputPipe[0]); close(outputPipe[1])
        throw ProcessRunnerError.pipeFailed(code: savedError)
    }

    for descriptor in outputPipe + errorPipe {
        _ = fcntl(descriptor, F_SETFD, FD_CLOEXEC)
    }

#if canImport(Darwin)
    var actions: posix_spawn_file_actions_t?
    var attributes: posix_spawnattr_t?
#else
    var actions = posix_spawn_file_actions_t()
    var attributes = posix_spawnattr_t()
#endif
    posix_spawn_file_actions_init(&actions)
    posix_spawnattr_init(&attributes)
    defer {
        posix_spawn_file_actions_destroy(&actions)
        posix_spawnattr_destroy(&attributes)
    }

    posix_spawn_file_actions_adddup2(&actions, outputPipe[1], STDOUT_FILENO)
    posix_spawn_file_actions_adddup2(&actions, errorPipe[1], STDERR_FILENO)
    for descriptor in outputPipe + errorPipe {
        posix_spawn_file_actions_addclose(&actions, descriptor)
    }
    posix_spawn_file_actions_addchdir_np(&actions, directory)

    var defaultSignals = sigset_t()
    sigemptyset(&defaultSignals)
    for signal in [SIGTERM, SIGINT, SIGQUIT, SIGHUP, SIGPIPE] {
        sigaddset(&defaultSignals, signal)
    }
    var signalMask = sigset_t()
    sigemptyset(&signalMask)

    let flags = Int16(
        POSIX_SPAWN_SETPGROUP
            | POSIX_SPAWN_SETSIGDEF
            | POSIX_SPAWN_SETSIGMASK
    )
    posix_spawnattr_setflags(&attributes, flags)
    posix_spawnattr_setpgroup(&attributes, 0)
    posix_spawnattr_setsigdefault(&attributes, &defaultSignals)
    posix_spawnattr_setsigmask(&attributes, &signalMask)

    let argumentStrings = [executable] + arguments
    let environmentStrings = environment.keys.sorted().map { "\($0)=\(environment[$0]!)" }
    var argv = argumentStrings.map { strdup($0) } + [nil]
    var environmentPointer = environmentStrings.map { strdup($0) } + [nil]
    defer {
        argv.dropLast().forEach { free($0) }
        environmentPointer.dropLast().forEach { free($0) }
    }

    var pid = pid_t()
    let spawnCode = argv.withUnsafeMutableBufferPointer { argvBuffer in
        environmentPointer.withUnsafeMutableBufferPointer { environmentBuffer in
            posix_spawn(
                &pid,
                executable,
                &actions,
                &attributes,
                argvBuffer.baseAddress!,
                environmentBuffer.baseAddress!
            )
        }
    }
    close(outputPipe[1]); close(errorPipe[1])
    guard spawnCode == 0 else {
        close(outputPipe[0]); close(errorPipe[0])
        throw ProcessRunnerError.spawnFailed(code: spawnCode)
    }
    return SpawnedProcess(pid: pid, standardOutput: outputPipe[0], standardError: errorPipe[0])
}

private func makeReader(
    fd: Int32,
    channel: ProcessOutputChannel,
    sequencer: OutputSequencer
) -> Task<Data, Never> {
    Task.detached(priority: nil) {
        defer { close(fd) }
        var collected = Data()
        var buffer = [UInt8](repeating: 0, count: 4096)
        while true {
            let count = buffer.withUnsafeMutableBytes { rawBuffer in
                systemRead(fd, rawBuffer.baseAddress!, rawBuffer.count)
            }
            if count > 0 {
                let bytes = Data(buffer.prefix(count))
                collected.append(bytes)
                await sequencer.emit(channel: channel, bytes: bytes)
            } else if count == 0 {
                return collected
            } else if errno != EINTR {
                return collected
            }
        }
    }
}

private actor OutputSequencer {
    private var sequence: UInt64 = 0
    private let handler: ProcessRunner.OutputHandler?

    init(handler: ProcessRunner.OutputHandler?) {
        self.handler = handler
    }

    func emit(channel: ProcessOutputChannel, bytes: Data) async {
        let chunk = ProcessOutputChunk(sequence: sequence, channel: channel, bytes: bytes)
        sequence += 1
        await handler?(chunk)
    }
}

private actor ProcessControl {
    private let pid: pid_t
    private let gracePeriod: Duration
    private var exited = false
    private var stopReason: ProcessStopReason = .completed
    private var escalation: Task<Void, Never>?

    init(pid: pid_t, gracePeriod: Duration) {
        self.pid = pid
        self.gracePeriod = gracePeriod
    }

    func requestStop(_ reason: ProcessStopReason) {
        guard !exited, stopReason == .completed else { return }
        stopReason = reason
        signalProcessGroup(SIGTERM)
        let pid = pid
        let gracePeriod = gracePeriod
        escalation = Task.detached {
            do { try await Task.sleep(for: gracePeriod) } catch { return }
            await self.forceStopIfRunning(pid: pid)
        }
    }

    func processDidExit() -> ProcessStopReason {
        exited = true
        escalation?.cancel()
        signalProcessGroup(SIGTERM)
        let pid = pid
        let gracePeriod = gracePeriod
        escalation = Task.detached {
            do { try await Task.sleep(for: gracePeriod) } catch { return }
            if processGroupExists(pid) {
                sendSignalToProcessGroup(SIGKILL, pid: pid)
            }
        }
        return stopReason
    }

    private func forceStopIfRunning(pid: pid_t) {
        guard !exited else { return }
        signalProcessGroup(SIGKILL, pid: pid)
    }

    private func signalProcessGroup(_ signal: Int32, pid: pid_t? = nil) {
        sendSignalToProcessGroup(signal, pid: pid ?? self.pid)
    }
}

private func processGroupExists(_ pid: pid_t) -> Bool {
    kill(-pid, 0) == 0 || errno != ESRCH
}

private func sendSignalToProcessGroup(_ signal: Int32, pid: pid_t) {
    _ = kill(-pid, signal)
}

private func waitForProcess(_ pid: pid_t) throws -> Int32 {
    var status: Int32 = 0
    while true {
        let result = waitpid(pid, &status, 0)
        if result == pid { return status }
        if result == -1, errno == EINTR { continue }
        throw ProcessRunnerError.waitFailed(code: errno)
    }
}

private func decodeWaitStatus(_ status: Int32) -> ProcessTermination {
    let signal = status & 0x7f
    if signal == 0 {
        return .exited(code: (status >> 8) & 0xff)
    }
    return .signaled(signal: signal)
}

@inline(__always)
private func systemRead(_ descriptor: Int32, _ buffer: UnsafeMutableRawPointer, _ count: Int) -> Int {
    #if canImport(Darwin)
    return Darwin.read(descriptor, buffer, count)
    #elseif canImport(Glibc)
    return Glibc.read(descriptor, buffer, count)
    #endif
}
