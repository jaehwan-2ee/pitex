import AppKit
import AppPorts
import BuildCore
import CryptoKit
import Foundation

/// Locates the `pi` agent executable. Resolution prefers the app's own
/// installs — a bundled runtime inside Pitex.app, then the app-local runtime
/// installed under Application Support — with a system-wide pi kept only as
/// a development fallback. `PI_AGENT_PATH` still wins for debugging.
enum PiExecutableLocator {
    static func resolve(environment: [String: String]) -> URL? {
        let fileManager = FileManager.default
        if let environment = environment["PI_AGENT_PATH"],
           !environment.isEmpty,
           fileManager.isExecutableFile(atPath: environment) {
            return URL(fileURLWithPath: environment)
        }
        if let bundled = bundledExecutable() { return bundled }
        if fileManager.isExecutableFile(atPath: PiPaths.runtimeExecutable.path) {
            return PiPaths.runtimeExecutable
        }
        return PiToolchain.find("pi", environment: environment)
    }

    /// True when the app-local runtime is already installed.
    static var appLocalRuntimeInstalled: Bool {
        FileManager.default.isExecutableFile(atPath: PiPaths.runtimeExecutable.path)
    }

    /// A pi runtime shipped inside the app bundle (release packaging puts it
    /// under Resources/pi-runtime).
    private static func bundledExecutable() -> URL? {
        let candidates = [
            Bundle.main.resourceURL?.appendingPathComponent("pi-runtime/bin/pi"),
            Bundle.main.resourceURL?.appendingPathComponent("pi/bin/pi"),
        ]
        for url in candidates.compactMap({ $0 }) {
            if FileManager.default.isExecutableFile(atPath: url.path) { return url }
        }
        return nil
    }
}

/// The bundled CLI bridge (`PitexAgent/bridge/cli.ts`). The app runs it with
/// the same Bun or Node runtime it uses for pi.
enum AgentBridge {
    static func scriptURL() -> URL? {
        guard let bundled = Bundle.main.resourceURL?.appendingPathComponent("PitexAgent/bridge/cli.ts"),
              FileManager.default.fileExists(atPath: bundled.path) else { return nil }
        return bundled
    }
}

/// Finder's PATH omits shell/version-manager installs. Discover once per
/// operation, then use the same PATH for installation, RPC and authentication.
struct PiToolchain: Sendable {
    var environment: [String: String]
    var bun: URL?
    var node: URL?
    var npm: URL?

    static func find(_ name: String, environment: [String: String]) -> URL? {
        (environment["PATH"] ?? "").split(separator: ":").map {
            URL(fileURLWithPath: String($0)).appendingPathComponent(name)
        }.first { FileManager.default.isExecutableFile(atPath: $0.path) }
    }

    static func discover(
        environment: [String: String] = ProcessInfo.processInfo.environment,
        home: URL = FileManager.default.homeDirectoryForCurrentUser,
        systemDirectories: [String] = ["/opt/homebrew/bin", "/usr/local/bin", "/run/current-system/sw/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"]
    ) async -> PiToolchain {
        var result = PiToolchain(environment: environment)
        var paths = (environment["PATH"] ?? "").split(separator: ":").map(String.init)
        paths += systemDirectories
        paths += [".bun/bin", ".npm-global/bin", ".local/bin", ".volta/bin", ".asdf/shims",
                  ".local/share/mise/shims", ".nix-profile/bin", "Library/pnpm"].map { home.appendingPathComponent($0).path }
        for key in ["BUN_INSTALL", "NPM_CONFIG_PREFIX", "npm_config_prefix", "HOMEBREW_PREFIX"] {
            if let prefix = environment[key], !prefix.isEmpty {
                paths.append((prefix as NSString).expandingTildeInPath + "/bin")
            }
        }
        func updatePath() {
            var seen = Set<String>()
            result.environment["PATH"] = paths.filter { $0.hasPrefix("/") && seen.insert($0).inserted }.joined(separator: ":")
        }
        updatePath()
        // Interactive startup is needed for .zshrc/.bashrc-managed installs.
        // A timeout prevents a shell startup prompt from hanging the app.
        let shell = environment["SHELL"] ?? "/bin/zsh"
        if let probe = try? await ProcessRunner().run(DirectCommandPlan(executable: shell,
            arguments: ["-ilc", "/usr/bin/printf '\\0'; /usr/bin/printenv PATH"],
            environment: .inherit(overrides: result.environment)), projectRoot: home, timeout: .seconds(5)),
           probe.stopReason == .completed, probe.termination == .exited(code: 0),
           let path = String(decoding: probe.standardOutput, as: UTF8.self).split(separator: "\0").last?.split(separator: "\n").first {
            paths.insert(contentsOf: path.split(separator: ":").map(String.init), at: 0)
        }
        // nvm/fnm may only initialize in a terminal with a TTY. Their installed
        // binaries are still usable directly from Finder.
        for (directory, suffix) in [(".nvm/versions/node", "bin"), (".local/share/fnm/node-versions", "installation/bin")] {
            let root = home.appendingPathComponent(directory)
            let versions = (try? FileManager.default.contentsOfDirectory(atPath: root.path)) ?? []
            paths += versions.sorted { $0.compare($1, options: .numeric) == .orderedDescending }
                .map { root.appendingPathComponent($0).appendingPathComponent(suffix).path }
        }
        // Last, so a user's own Bun or Node wins over the one Pitex installed.
        paths.append(PinnedNode.binDirectory.path)
        updatePath()
        result.node = find("node", environment: result.environment)
        result.npm = find("npm", environment: result.environment)
        // npm's global prefix can be customized without adding it to PATH.
        if find("bun", environment: result.environment) == nil, let command = result.npmCommand(arguments: ["prefix", "--global"]),
           let probe = try? await ProcessRunner().run(DirectCommandPlan(executable: command.executable.path,
                arguments: command.arguments,
                environment: .inherit(overrides: result.environment)), projectRoot: home, timeout: .seconds(5)),
           probe.termination == .exited(code: 0),
           let prefix = String(decoding: probe.standardOutput, as: UTF8.self).split(separator: "\n").last,
           prefix.hasPrefix("/") {
            paths.insert(String(prefix) + "/bin", at: 0)
            updatePath()
        }
        result.bun = find("bun", environment: result.environment)
        // A Node older than pi's minimum fails the install's smoke check:
        // put the pinned Node first instead, or report none so the
        // installer downloads it.
        if result.bun == nil, let node = result.node,
           await !nodeMeetsMinimum(node, environment: result.environment, home: home) {
            paths.insert(PinnedNode.binDirectory.path, at: 0)
            updatePath()
            result.node = find("node", environment: result.environment)
            if let pinned = result.node, await !nodeMeetsMinimum(pinned, environment: result.environment, home: home) {
                result.node = nil
            }
            result.npm = result.node == nil ? nil : find("npm", environment: result.environment)
        }
        return result
    }

    /// pi's `engines.node` (`@earendil-works/pi-coding-agent` 1.5.2 and its
    /// pi-codemode dependency both declare `>=22.19.0`).
    static let minimumNodeVersion = [22, 19, 0]

    /// `node --version` prints `v22.19.0`; an unreadable answer counts as too old.
    static func nodeVersionMeetsMinimum(_ output: String) -> Bool {
        let version = output.trimmingCharacters(in: .whitespacesAndNewlines)
        guard version.hasPrefix("v") else { return false }
        let parts = version.dropFirst().split(separator: ".", omittingEmptySubsequences: false)
            .map { Int($0.prefix { $0.isASCII && $0.isNumber }) }
        guard parts.count == 3, !parts.contains(nil) else { return false }
        return !parts.compactMap { $0 }.lexicographicallyPrecedes(minimumNodeVersion)
    }

    private static func nodeMeetsMinimum(_ node: URL, environment: [String: String], home: URL) async -> Bool {
        guard let probe = try? await ProcessRunner().run(DirectCommandPlan(executable: node.path, arguments: ["--version"],
            environment: .inherit(overrides: environment)), projectRoot: home, timeout: .seconds(5)),
            probe.stopReason == .completed, probe.termination == .exited(code: 0) else { return false }
        return nodeVersionMeetsMinimum(String(decoding: probe.standardOutput, as: UTF8.self))
    }

    func npmCommand(arguments: [String]) -> (executable: URL, arguments: [String])? {
        guard let npm, let node else { return nil }
        let script = npm.resolvingSymlinksInPath()
        // npm can be a JS symlink or an executable version-manager shim.
        return script.pathExtension == "js" ? (node, [script.path] + arguments) : (npm, arguments)
    }

    /// Do not rely on a JavaScript executable's `env node` shebang: Bun-only
    /// machines must use Bun for the agent itself, not just for its installer.
    func launch(_ executable: URL, arguments: [String]) throws -> (executable: URL, arguments: [String]) {
        let script = executable.resolvingSymlinksInPath()
        let handle = try? FileHandle(forReadingFrom: script)
        defer { try? handle?.close() }
        let header = String(decoding: (try? handle?.read(upToCount: 256)) ?? Data(), as: UTF8.self)
            .components(separatedBy: "\n").first ?? ""
        let javaScript = ["js", "mjs", "cjs", "ts", "mts", "cts"].contains(script.pathExtension)
            || (header.hasPrefix("#!") && (header.contains("node") || header.contains("bun")))
        guard javaScript else { return (executable, arguments) }
        if let bun {
            // Pi 0.85's bundled undici imports fail under Bun 1.3. The package
            // also ships an unbundled CLI that avoids those bundled imports.
            let unbundled = script.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("cli.js")
            let entry = script.lastPathComponent == "cli.js" && script.deletingLastPathComponent().lastPathComponent == "bundle"
                && FileManager.default.fileExists(atPath: unbundled.path) ? unbundled : script
            return (bun, ["--bun", entry.path] + arguments)
        }
        if let node { return (node, [script.path] + arguments) }
        throw PiRuntimeInstaller.InstallError.runtimeMissing
    }
}

/// Installs the Pitex Agent runtime into the app-local directory
/// (`~/Library/Application Support/Pitex/pi-runtime`) via Bun or npm, so the agent
/// does not depend on a global pi install.
enum PiRuntimeInstaller {
    enum InstallError: LocalizedError {
        case runtimeMissing
        case installFailed(String)

        var errorDescription: String? {
            switch self {
            case .runtimeMissing:
                "Pitex Agent's JavaScript runtime is not installed yet. Click Install Pitex Agent in the Assistant or in Settings → AI."
            case let .installFailed(output):
                "Pitex Agent install failed: \(output)"
            }
        }
    }

    /// npm package for the agent runtime. The upstream project moved scopes
    /// from @mariozechner to @earendil-works after 0.73.x.
    private static let packageName = "@earendil-works/pi-coding-agent"

    /// Pinned runtime version. 1.5.2 keeps the RPC surface
    /// (`get_state`, `set_thinking_level`, `get_session_stats`,
    /// `get_commands`, `set_model`). RPC types, JSON events, and the JSONL
    /// codec match 1.4.0. The package depends on pi-codemode 1.5.2 exactly.
    /// The Node requirement is unchanged (`>=22.19.0`).
    static let desiredVersion = "1.5.2"

    @MainActor private static var inFlight: Task<Void, Error>?

    /// First-launch path: installs the app-local runtime when missing,
    /// upgrades it when the installed version trails `desiredVersion`, and
    /// always (re)mirrors the bundled skills so reinstalls refresh them too.
    /// Runtime failures are reported in Settings → AI instead of blocking
    /// startup, so this never throws.
    static func ensureInstalled() async {
        do {
            if !PiExecutableLocator.appLocalRuntimeInstalled {
                try await install()
            } else if let installed = installedVersion(),
                      isOlder(installed, than: desiredVersion) {
                try await install()
            }
        } catch {
            NSLog("Pitex Agent auto-install failed: \(error.localizedDescription)")
        }
        try? installBundledSkills()
    }

    /// Read package metadata without executing an `env node` launcher.
    static func installedVersion() -> String? {
        var directory = PiPaths.runtimeExecutable.resolvingSymlinksInPath().deletingLastPathComponent()
        while directory.path != "/" {
            let package = directory.appendingPathComponent("package.json")
            if let data = try? Data(contentsOf: package),
               let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let name = object["name"] as? String,
               [packageName, "@mariozechner/pi-coding-agent"].contains(name),
               let version = object["version"] as? String { return version }
            if directory == PiPaths.runtimeDirectory { break }
            directory.deleteLastPathComponent()
        }
        return nil
    }

    /// Semver-ish comparison: true when `installed` orders before `desired`.
    private static func isOlder(_ installed: String, than desired: String) -> Bool {
        let lhs = installed.split(separator: ".").map { Int($0) ?? 0 }
        let rhs = desired.split(separator: ".").map { Int($0) ?? 0 }
        for index in 0 ..< max(lhs.count, rhs.count) {
            let l = index < lhs.count ? lhs[index] : 0
            let r = index < rhs.count ? rhs[index] : 0
            if l != r { return l < r }
        }
        return false
    }

    /// The installed package's CLI entry. Prefers the unbundled `dist/cli.js`
    /// (see `PiToolchain.launch`); falls back to the declared `bin` bundle.
    private static func installedEntry() -> URL? {
        let dist = PiPaths.runtimeDirectory.appendingPathComponent("node_modules/\(packageName)/dist")
        return ["cli.js", "bundle/cli.js"].lazy.map(dist.appendingPathComponent)
            .first { FileManager.default.fileExists(atPath: $0.path) }
    }

    /// Both package managers install locally; no global packages or provider
    /// credentials are changed. Publish the launcher only after a smoke check.
    /// A pinned install that is already running (the launch-time one) is
    /// joined, so the Install button never fails just because it was early.
    @MainActor
    static func install(latest: Bool = false, toolchain: PiToolchain? = nil) async throws {
        if let inFlight {
            guard !latest else { throw InstallError.installFailed("An agent installation is already in progress.") }
            return try await inFlight.value
        }
        let task = Task { try await performInstall(latest: latest, toolchain: toolchain) }
        inFlight = task
        defer { inFlight = nil }
        try await task.value
    }

    @MainActor
    private static func performInstall(latest: Bool, toolchain: PiToolchain?) async throws {
        var tools = if let toolchain { toolchain } else { await PiToolchain.discover() }
        if tools.bun == nil, tools.npmCommand(arguments: []) == nil {
            try await PinnedNode.install()
            tools = await PiToolchain.discover()
        }
        let files = FileManager.default
        let runtime = PiPaths.runtimeDirectory
        let backup = runtime.deletingLastPathComponent().appendingPathComponent("pi-runtime-backup-\(UUID().uuidString)")
        // Manual and pinned startup updates preserve the working runtime on failure.
        // Auth, settings, sessions and skills live in the separate agent directory.
        let backedUp = files.fileExists(atPath: runtime.path)
        if backedUp { try files.moveItem(at: runtime, to: backup) }
        do {
            try await installPackage(version: latest ? "latest" : desiredVersion, tools: tools)
        } catch {
            if backedUp || latest {
                do {
                    if files.fileExists(atPath: runtime.path) { try files.removeItem(at: runtime) }
                    if backedUp { try files.moveItem(at: backup, to: runtime) }
                } catch let restoreError {
                    throw InstallError.installFailed("\(error.localizedDescription) Previous runtime preserved at \(backup.path); restore failed: \(restoreError.localizedDescription)")
                }
            }
            throw error
        }
        if backedUp {
            _ = await Task.detached { try? FileManager.default.removeItem(at: backup) }.value
        }
    }

    private static func installPackage(version: String, tools: PiToolchain) async throws {
        let spec = "\(packageName)@\(version)"
        // npm is a real fallback even when bun exists: `bun add` can exit 0
        // while laying down nothing (stale tree it won't re-extract, corrupt
        // cache, a `bun` that isn't bun, a registry mirror serving a stub).
        var attempts: [(name: String, executable: URL, arguments: [String])] = []
        if let bun = tools.bun {
            attempts.append(("bun", bun, ["add", "--cwd", PiPaths.runtimeDirectory.path,
                "--force", "--linker", "hoisted", "--no-cache", "--ignore-scripts", "--exact", spec]))
        }
        if let npm = tools.npmCommand(arguments: ["install", "--prefix", PiPaths.runtimeDirectory.path,
                    "--save-exact", "--prefer-online", "--ignore-scripts", "--no-audit", "--no-fund", spec]) {
            attempts.append(("npm", npm.executable, npm.arguments))
            attempts.append(("npm+npmjs", npm.executable, npm.arguments + ["--registry", "https://registry.npmjs.org"]))
        }
        guard !attempts.isEmpty else { throw InstallError.runtimeMissing }
        try FileManager.default.createDirectory(at: PiPaths.runtimeDirectory, withIntermediateDirectories: true)
        /// nil when the installed CLI runs `--version` cleanly.
        func smokeCheck(_ entry: URL) async throws -> String? {
            let command = try tools.launch(entry, arguments: ["--version"])
            let check = try await ProcessRunner().run(DirectCommandPlan(executable: command.executable.path, arguments: command.arguments,
                environment: .inherit(overrides: tools.environment)), projectRoot: PiPaths.runtimeDirectory, timeout: .seconds(15))
            return check.termination == .exited(code: 0) && check.stopReason == .completed
                ? nil : String(String(decoding: check.standardError, as: UTF8.self).suffix(2_000))
        }
        // Every attempt gets a clean slate — including package.json, whose
        // leftover state (e.g. workspaces) can redirect where files land.
        var entry: URL?
        var failures: [String] = []
        for attempt in attempts where entry == nil {
            for name in ["node_modules", "bun.lock", "bun.lockb", "package-lock.json", "package.json"] {
                try? FileManager.default.removeItem(at: PiPaths.runtimeDirectory.appendingPathComponent(name))
            }
            let name = attempt.name
            let result = try await ProcessRunner().run(DirectCommandPlan(executable: attempt.executable.path, arguments: attempt.arguments,
                environment: .inherit(overrides: tools.environment)), projectRoot: PiPaths.runtimeDirectory, timeout: .seconds(300))
            if result.stopReason == .completed, result.termination == .exited(code: 0) {
                if let found = installedEntry() {
                    if let smoke = try await smokeCheck(found) {
                        failures.append("\(name): \(smoke)")
                    } else {
                        entry = found
                    }
                } else {
                    failures.append("\(name): no CLI entry point — output: "
                        + String(decoding: result.standardOutput + result.standardError, as: UTF8.self).suffix(1_000))
                }
            } else {
                failures.append("\(name): " + String(String(decoding: result.standardOutput + result.standardError, as: UTF8.self).suffix(1_000)))
            }
        }
        guard let entry else {
            throw InstallError.installFailed(failures.joined(separator: "; "))
        }
        let fileManager = FileManager.default
        try fileManager.createDirectory(at: PiPaths.runtimeExecutable.deletingLastPathComponent(), withIntermediateDirectories: true)
        // Rename a new symlink over the old one atomically, leaving an existing
        // legacy package's CLI and running agent processes untouched.
        let temporary = PiPaths.runtimeExecutable.deletingLastPathComponent().appendingPathComponent(UUID().uuidString)
        defer { try? fileManager.removeItem(at: temporary) }
        try fileManager.createSymbolicLink(at: temporary, withDestinationURL: entry)
        guard rename(temporary.path, PiPaths.runtimeExecutable.path) == 0 else {
            throw InstallError.installFailed("The agent launcher could not be updated.")
        }
        try installBundledSkills()
    }

    /// Copies the skills bundled inside Pitex.app into the agent's
    /// skills directory, replacing stale copies so upgrades and reinstalls
    /// always ship the current files.
    static func installBundledSkills() throws {
        guard let bundled = PiPaths.bundledSkillsDirectory else { return }
        let fileManager = FileManager.default
        let destination = PiPaths.skillsDirectory
        try fileManager.createDirectory(at: destination, withIntermediateDirectories: true)
        let names = try fileManager.contentsOfDirectory(atPath: bundled.path)
        for name in names where !name.hasPrefix(".") {
            let source = bundled.appendingPathComponent(name, isDirectory: true)
            let target = destination.appendingPathComponent(name, isDirectory: true)
            if fileManager.fileExists(atPath: target.path) {
                try fileManager.removeItem(at: target)
            }
            try fileManager.copyItem(at: source, to: target)
        }
    }

    /// `pi install <source>` — installs a skill or extension into the
    /// app-local agent home. Unlike auth (which needs a pty) this runs
    /// in-process with the same discovered toolchain PATH and
    /// `PI_CODING_AGENT_DIR` the spawned agent uses.
    static func installSkill(source: String) async throws {
        let executable = PiPaths.runtimeExecutable
        guard FileManager.default.isExecutableFile(atPath: executable.path) else {
            throw InstallError.installFailed("Pitex Agent is not installed yet.")
        }
        let tools = await PiToolchain.discover()
        let launch = try tools.launch(executable, arguments: ["install", source])
        var environment = tools.environment
        environment["PI_CODING_AGENT_DIR"] = PiPaths.agentDirectory.path
        let result = try await ProcessRunner().run(DirectCommandPlan(executable: launch.executable.path,
            arguments: launch.arguments,
            environment: .inherit(overrides: environment)), projectRoot: FileManager.default.homeDirectoryForCurrentUser, timeout: .seconds(300))
        guard result.stopReason == .completed, result.termination == .exited(code: 0) else {
            throw InstallError.installFailed(String(String(decoding: result.standardOutput + result.standardError, as: UTF8.self).suffix(2_000)))
        }
    }

    /// pi owns provider selection and credential updates for both actions.
    static func openAuthenticationInTerminal(logout: Bool = false) async throws {
        let url = try await authenticationScript(logout: logout)
        guard NSWorkspace.shared.open(url) else {
            throw InstallError.installFailed("The provider setup terminal could not be opened.")
        }
    }

    static func authenticationScript(logout: Bool, toolchain: PiToolchain? = nil) async throws -> URL {
        let fileManager = FileManager.default
        let executable = PiPaths.runtimeExecutable
        guard fileManager.isExecutableFile(atPath: executable.path) else {
            throw InstallError.installFailed("Pitex Agent is not installed yet.")
        }
        try fileManager.createDirectory(at: PiPaths.agentDirectory, withIntermediateDirectories: true)
        let tools = if let toolchain { toolchain } else { await PiToolchain.discover() }
        let launch = try tools.launch(executable, arguments: [])
        let action = logout ? "Logout" : "Login"
        let command = logout ? "/logout" : "/login"
        let scriptURL = PiPaths.agentDirectory.appendingPathComponent("Pitex Agent \(action).command")
        func quoted(_ value: String) -> String {
            "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
        }
        let script = """
            #!/bin/zsh
            export PI_CODING_AGENT_DIR=\(quoted(PiPaths.agentDirectory.path))
            export PATH=\(quoted(tools.environment["PATH"] ?? "/usr/bin:/bin"))
            echo "Pitex Agent \(action) — pick a provider."
            echo "If the provider list does not open on its own, type \(command) and press Return."
            # script(1) gives pi the pty its TUI needs, but the terminal's own
            # stdin must be noncanonical too — otherwise `cat` line-buffers
            # and arrow keys never reach pi. `stty sane` restores on exit.
            trap 'stty sane' EXIT
            stty -icanon -echo min 1 time 0
            ( sleep 3; printf '\(command)\\r'; cat ) | script -q /dev/null \(([launch.executable.path] + launch.arguments).map(quoted).joined(separator: " "))

            """
        try script.write(to: scriptURL, atomically: true, encoding: .utf8)
        try fileManager.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        return scriptURL
    }
}

protocol AgentSession: AnyObject, Sendable {
    var events: AsyncStream<PiRPCEvent> { get }
    var isRunning: Bool { get }
    func send(_ command: PiRPCCommand, id: String?) throws
    func terminate()
}

extension AgentSession {
    func send(_ command: PiRPCCommand) throws {
        try send(command, id: nil)
    }
}

/// One running `pi --mode rpc` subprocess. Commands are JSON-encoded onto
/// stdin; stdout lines are decoded and republished as an `AsyncStream`.
/// The stream finishes when the process exits.
final class PiAgentProcess: AgentSession, @unchecked Sendable {
    let executableURL: URL
    let workingDirectory: URL

    private let process = Process()
    private let stdinPipe = Pipe()
    private let stdoutPipe = Pipe()
    private let stderrPipe = Pipe()
    private let ioQueue = DispatchQueue(label: "dev.pitex.pi-process")
    private var stdoutAssembler = AgentStdoutAssembler()
    private var stderrTail = Data()
    private let continuation: AsyncStream<PiRPCEvent>.Continuation
    private var didFinish = false
    private var onExitHandler: (@Sendable () -> Void)?
    private var stdinWriter: AgentStdinWriter?

    let events: AsyncStream<PiRPCEvent>

    init(
        executableURL: URL,
        workingDirectory: URL,
        arguments: [String],
        environment: [String: String]
    ) {
        self.executableURL = executableURL
        self.workingDirectory = workingDirectory
        var captured: AsyncStream<PiRPCEvent>.Continuation!
        events = AsyncStream { captured = $0 }
        continuation = captured

        process.executableURL = executableURL
        process.arguments = arguments
        process.currentDirectoryURL = workingDirectory
        process.environment = environment
        process.standardInput = stdinPipe
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe
    }

    func start(onExit: @escaping @Sendable () -> Void) throws {
        onExitHandler = onExit
        process.terminationHandler = { [weak self] _ in
            self?.handleExit()
        }
        stdoutPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            self?.consumeStdout(handle.availableData)
        }
        stderrPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            self?.consumeStderr(handle.availableData)
        }
        // A child that closed stdin cannot take commands: end it so pending sends settle on exit.
        stdinWriter = AgentStdinWriter(stdinPipe.fileHandleForWriting) { [weak self] _ in self?.terminate() }
        try process.run()
    }

    func send(_ command: PiRPCCommand, id: String? = nil) throws {
        guard process.isRunning, let stdinWriter else { throw PiAgentProcessError.notRunning }
        var object = command.object
        if let id { object["id"] = id }
        stdinWriter.write(try PiJSON.encode(object))
    }

    var isRunning: Bool { process.isRunning }

    var stderrText: String {
        ioQueue.sync { String(data: stderrTail, encoding: .utf8) ?? "" }
    }

    func terminate() {
        guard process.isRunning else { return }
        process.terminate()
    }

    private func consumeStdout(_ chunk: Data) {
        ioQueue.async { [self] in
            emitStdoutLines(stdoutAssembler.append(chunk))
        }
    }

    private func emitStdoutLines(_ lines: [Data]) {
        for line in lines {
            guard let object = PiJSON.decode(line), let event = PiRPCEvent(object) else { continue }
            continuation.yield(event)
        }
    }

    private func consumeStderr(_ chunk: Data) {
        ioQueue.async { [self] in
            stderrTail.append(chunk)
            if stderrTail.count > 65_536 {
                stderrTail.removeFirst(stderrTail.count - 65_536)
            }
        }
    }

    private func handleExit() {
        ioQueue.async { [self] in
            guard !didFinish else { return }
            didFinish = true
            stdoutPipe.fileHandleForReading.readabilityHandler = nil
            stderrPipe.fileHandleForReading.readabilityHandler = nil
            let remaining = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
            if !remaining.isEmpty {
                emitStdoutLines(stdoutAssembler.append(remaining))
            }
            continuation.finish()
            onExitHandler?()
        }
    }
}

enum PiAgentProcessError: LocalizedError {
    case notRunning

    var errorDescription: String? { "The pi agent process is not running." }
}

/// Node.js that Pitex installs for itself when neither Bun nor Node is
/// found, next to pi-runtime in Application Support. Keep the version and
/// checksums in step with `PINNED_NODE` in Linux/crates/pitex-shell/src/agent.rs.
enum PinnedNode {
    static let version = "24.22.2"
    /// From nodejs.org/dist/v24.22.2/SHASUMS256.txt.asc (release-key signed).
    private static let sha256 = [
        "darwin-arm64": "aa9549349e47dffaa42dcf95d6e06ade8c4bdafc980979883b88767670df385a",
        "darwin-x64": "d5e336d588c45be0c37dbd52cc29895593009ca4997a77a1fd02815131ed0e52",
    ]

    static var directory: URL {
        PiPaths.runtimeDirectory.deletingLastPathComponent().appendingPathComponent("node", isDirectory: true)
    }

    static var binDirectory: URL { directory.appendingPathComponent("bin", isDirectory: true) }

    /// Download, verify against the pinned SHA-256, unpack, then swap in.
    static func install() async throws {
        #if arch(arm64)
        let platform = "darwin-arm64"
        #else
        let platform = "darwin-x64"
        #endif
        let name = "node-v\(version)-\(platform)"
        let url = URL(string: "https://nodejs.org/dist/v\(version)/\(name).tar.gz")!
        let download: URL
        do {
            let (file, response) = try await URLSession.shared.download(from: url)
            download = file
            guard (response as? HTTPURLResponse)?.statusCode == 200 else {
                try? FileManager.default.removeItem(at: file)
                throw URLError(.badServerResponse)
            }
        } catch {
            throw PiRuntimeInstaller.InstallError.installFailed("Node.js could not be downloaded from nodejs.org: \(error.localizedDescription)")
        }
        defer { try? FileManager.default.removeItem(at: download) }
        let digest = try SHA256.hash(data: Data(contentsOf: download, options: .mappedIfSafe))
            .map { String(format: "%02x", $0) }.joined()
        guard digest == sha256[platform] else {
            throw PiRuntimeInstaller.InstallError.installFailed("The Node.js download did not match its pinned checksum, so it was not installed.")
        }
        let files = FileManager.default
        let staging = directory.deletingLastPathComponent().appendingPathComponent("node-staging-\(UUID().uuidString)")
        try files.createDirectory(at: staging, withIntermediateDirectories: true)
        defer { try? files.removeItem(at: staging) }
        let result = try await ProcessRunner().run(DirectCommandPlan(executable: "/usr/bin/tar",
            arguments: ["-xzf", download.path, "-C", staging.path], environment: .inherit(overrides: [:])),
            projectRoot: staging, timeout: .seconds(300))
        let unpacked = staging.appendingPathComponent(name)
        guard result.termination == .exited(code: 0),
              files.isExecutableFile(atPath: unpacked.appendingPathComponent("bin/node").path) else {
            throw PiRuntimeInstaller.InstallError.installFailed("Node.js could not be unpacked: "
                + String(decoding: result.standardError, as: UTF8.self).suffix(1_000))
        }
        // C headers are only for native addons, which the install skips (--ignore-scripts).
        try? files.removeItem(at: unpacked.appendingPathComponent("include"))
        if files.fileExists(atPath: directory.path) { try files.removeItem(at: directory) }
        try files.moveItem(at: unpacked, to: directory)
    }
}
