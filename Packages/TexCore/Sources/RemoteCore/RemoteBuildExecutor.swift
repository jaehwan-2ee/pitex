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
    /// `required`'s directory — the isolated `.pitex-live/<job>` tree to
    /// create on the device before the compiler runs; nil for manual
    /// builds writing beside the sources.
    private var outputDirectory: String?
    private var running: [BuildID: Task<Int32, Error>] = [:]

    public init(sync: RemoteSync) {
        self.sync = sync
    }

    public func setOutputs(_ paths: [String], required: String? = nil) {
        outputs = paths
        requiredOutput = required
        // Live builds only: a manual nested target (manuscript/main.pdf)
        // keeps its in-place layout untouched.
        outputDirectory = required.flatMap { path -> String? in
            guard path.hasPrefix(BuildTarget.liveDirectoryName + "/"),
                  let slash = path.range(of: "/", options: .backwards)
            else { return nil }
            let directory = String(path[..<slash.lowerBound])
            return RemoteSyncRules.isSafeRelativePath(directory) ? directory : nil
        }
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
        let client = sync.client
        let setup = outputSetupDirectories(for: request)
        let task = Task<Int32, Error> {
            if request.stageIndex == 0 {
                await output(BuildProcessOutput(channel: .system,
                                                bytes: Data("Building on \(sync.mirror.project.connection.name): \(directory)\n".utf8)))
            }
            // A callback-triggered cancel lands on this already-registered
            // task; an awaited callback that cancelled must stop remote work.
            if Task.isCancelled { throw SSHError.cancelled }
            // The isolated output tree (never files) is created on the
            // device inside this cancellable task, so a cancel during
            // setup never reaches the compiler. A refused path — a
            // symlink redirecting the live root or a child, even inside
            // the project — fails the stage like a failed launch.
            if let setup {
                let result = try await client.run(
                    RemoteScripts.makeDirectories, arguments: [remoteRoot] + setup)
                if Task.isCancelled { throw SSHError.cancelled }
                guard result.status == 0 else {
                    let refused = result.stdoutText.split(whereSeparator: \.isNewline)
                        .first { $0.hasPrefix("R/") }.map { ": " + $0.dropFirst(2) + " redirected" }
                        ?? ": " + (SSHClient.firstLine(result.stderrText) ?? "mkdir failed")
                    throw SSHError.remote(status: result.status,
                                          message: "The remote build output directory could not be prepared\(refused).")
                }
            }
            if Task.isCancelled { throw SSHError.cancelled }
            return try await client.stream(RemoteScripts.loginExec, arguments: [directory] + words, tty: true) { data, isError in
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

    /// The directory args for `RemoteScripts.makeDirectories`, or nil
    /// when the build writes beside the sources (no output directory).
    /// The orchestrator already prepared `outputDirectory` locally with
    /// the nested include/aux layout, so its directories are replayed
    /// verbatim; a standalone executor without that prepared tree falls
    /// back to the mirror's and the source directory's `.tex` parents.
    /// Every argument is a project-relative path revalidated against the
    /// sync rules.
    private func outputSetupDirectories(for request: BuildProcessRequest) -> [String]? {
        guard let outputDirectory, RemoteSyncRules.isSafeRelativePath(outputDirectory) else { return nil }
        let outputRoot = sync.mirror.root.appendingPathComponent(outputDirectory, isDirectory: true)
        let relative: [String]
        if let prepared = Self.unredirectedDirectory(outputRoot, under: sync.mirror.root) {
            relative = Self.scannedDirectories(under: prepared).all
        } else {
            relative = Self.scannedDirectories(under: sync.mirror.root).texParents
                + (request.sourceDirectory.standardizedFileURL.path
                    == sync.mirror.root.standardizedFileURL.path
                    ? [] : Self.scannedDirectories(under: request.sourceDirectory).texParents)
        }
        var seen: Set<String> = [outputDirectory]
        var directories = [outputDirectory]
        for name in relative {
            let path = outputDirectory + "/" + name
            if RemoteSyncRules.isSafeRelativePath(path), seen.insert(path).inserted {
                directories.append(path)
            }
        }
        return directories
    }

    /// `url` only when every component below `anchor` exists as a real,
    /// non-symlinked directory — a redirected prepared output tree is
    /// never replayed onto the device.
    private static func unredirectedDirectory(_ url: URL, under anchor: URL) -> URL? {
        guard let relative = relativePath(url, under: anchor) else { return nil }
        var current = anchor
        for component in relative.split(separator: "/") {
            current.appendPathComponent(String(component), isDirectory: true)
            guard let values = try? current.resourceValues(forKeys: [.isSymbolicLinkKey, .isDirectoryKey]),
                  values.isDirectory == true, values.isSymbolicLink != true else { return nil }
        }
        return current
    }

    /// One pass below `root`: every real directory (`all`) plus those
    /// containing `.tex` files (`texParents`), `/`-separated relative.
    /// Symlinked directories are neither descended nor listed; hidden
    /// and sync-excluded names skip.
    private static func scannedDirectories(under root: URL) -> (all: [String], texParents: [String]) {
        var all: [String] = [], texParents: [String] = []
        var stack = [root]
        let keys: Set<URLResourceKey> = [.isDirectoryKey, .isSymbolicLinkKey, .isRegularFileKey]
        while let directory = stack.popLast() {
            guard let entries = try? FileManager.default.contentsOfDirectory(
                at: directory, includingPropertiesForKeys: Array(keys)) else { continue }
            var hasTeX = false
            for entry in entries {
                guard let values = try? entry.resourceValues(forKeys: keys) else { continue }
                if values.isDirectory == true {
                    if values.isSymbolicLink == true { continue }
                    let name = entry.lastPathComponent
                    if name.hasPrefix(".") || RemoteSyncRules.isExcluded(name: name) { continue }
                    stack.append(entry)
                } else if values.isRegularFile == true, entry.pathExtension.lowercased() == "tex" {
                    hasTeX = true
                }
            }
            guard let relative = relativePath(directory, under: root) else { continue }
            all.append(relative)
            if hasTeX { texParents.append(relative) }
        }
        return (all, texParents)
    }

    /// `url` under `anchor`, `/`-separated — nil when equal or outside.
    private static func relativePath(_ url: URL, under anchor: URL) -> String? {
        let base = anchor.standardizedFileURL.path
        let path = url.standardizedFileURL.path
        guard path.hasPrefix(base + "/") else { return nil }
        return String(path.dropFirst(base.count + 1))
    }
}
