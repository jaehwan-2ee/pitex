import BuildCore
import Foundation

/// Runs build stages on the remote device instead of this Mac: each stage's
/// command executes in the matching remote directory through the remote
/// user's login shell (their TeX installation), output streams back live,
/// and after every stage the build outputs are fetched into the mirror so
/// the orchestrator's PDF/SyncTeX handling works on local files as usual.
public actor RemoteBuildExecutor: BuildProcessExecuting {
    public let sync: RemoteSync
    /// Project-relative paths to fetch after each stage (the target's PDF,
    /// SyncTeX data, log…); set before each build.
    private var outputs: [String] = []
    /// The output a successful stage must deliver (the target's PDF).
    private var requiredOutput: String?
    private var running: [BuildID: Task<Int32, Error>] = [:]

    public init(sync: RemoteSync) {
        self.sync = sync
    }

    public func setOutputs(_ paths: [String], required: String? = nil) {
        outputs = paths
        requiredOutput = required
    }

    public func execute(
        _ request: BuildProcessRequest,
        output: @escaping @Sendable (BuildProcessOutput) async -> Void
    ) async throws -> BuildProcessResult {
        let remoteRoot = sync.mirror.project.remoteRoot
        let directory: String
        let command: [String]
        let workingDirectory: WorkingDirectoryPolicy
        let environment: EnvironmentPolicy
        switch request.command {
        case let .direct(plan):
            workingDirectory = plan.workingDirectory
            environment = plan.environment
            command = [plan.executable] + plan.arguments
        case let .loginShell(plan):
            workingDirectory = plan.workingDirectory
            environment = plan.environment
            command = ["/bin/sh", "-c", plan.command]
        }
        switch workingDirectory {
        case .projectRoot:
            directory = remoteRoot
        case .sourceDirectory:
            directory = await sync.remotePath(for: request.sourceDirectory) ?? remoteRoot
        case let .explicit(path):
            guard let mapped = await sync.remotePath(for: URL(fileURLWithPath: path)) else {
                throw BuildProcessExecutorError.launchFailed("\(path) is outside the remote project.")
            }
            directory = mapped
        }
        // Local environment values (this Mac's PATH, TEXINPUTS…) mean
        // nothing on the remote; only explicit overrides travel.
        let overrides: [String]
        switch environment {
        case let .inherit(values):
            overrides = values.filter { $0.key != "PATH" }.sorted { $0.key < $1.key }.map { "\($0.key)=\($0.value)" }
        case let .replace(values):
            overrides = ["-i"] + values.filter { $0.key != "PATH" }.sorted { $0.key < $1.key }.map { "\($0.key)=\($0.value)" }
        }
        let words = overrides.isEmpty ? command : ["env"] + overrides + command
        if request.stageIndex == 0 {
            await output(BuildProcessOutput(channel: .system,
                                            bytes: Data("Building on \(sync.mirror.project.connection.name): \(directory)\n".utf8)))
        }
        let client = sync.client
        let task = Task<Int32, Error> {
            try await client.stream(RemoteScripts.loginExec, arguments: [directory] + words, tty: true) { data, isError in
                await output(BuildProcessOutput(channel: isError ? .standardError : .standardOutput, bytes: data))
            }
        }
        running[request.buildID] = task
        defer { running[request.buildID] = nil }
        let status: Int32
        do {
            status = try await task.value
        } catch SSHError.cancelled {
            return BuildProcessResult(exitCode: 130)
        } catch let error as SSHError {
            throw BuildProcessExecutorError.launchFailed(error.localizedDescription)
        }
        if status == 255 {
            throw BuildProcessExecutorError.launchFailed("The SSH connection to \(sync.mirror.project.connection.name) failed.")
        }
        // Fetch even after a failed stage: a partial PDF and the log are
        // what the user needs to see why.
        var fetched: [String] = []
        var failure: String?
        do {
            fetched = try await sync.fetch(outputs)
        } catch {
            failure = "Could not download the build outputs: \(error.localizedDescription)"
        }
        if let requiredOutput, failure == nil, !fetched.contains(requiredOutput) {
            failure = "The build on \(sync.mirror.project.connection.name) did not produce \(requiredOutput)."
        }
        if let failure {
            // A successful stage without its PDF must not pass: the PDF on
            // screen would still be the previous build's.
            guard status != 0 else { throw BuildProcessExecutorError.launchFailed(failure) }
            await output(BuildProcessOutput(channel: .system, bytes: Data((failure + "\n").utf8)))
        }
        return BuildProcessResult(exitCode: status)
    }

    public func cancel(buildID: BuildID) async {
        running[buildID]?.cancel()
    }
}
