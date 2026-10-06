#!/usr/bin/env swift
// After Release build: Tools/check-pi-runtime.swift <Build/Products/Release> [--latest].
// Checks Finder PATH discovery, real Bun and Node/npm installs, RPC and isolated auth; --latest also checks update rollback.
import Foundation
import Darwin

private struct CheckFailure: Error, CustomStringConvertible {
    let description: String
}

private func fail(_ message: String) throws -> Never {
    throw CheckFailure(description: message)
}

private final class ChildProcess: @unchecked Sendable {
    let pid: pid_t
    let exited = DispatchGroup()
    var status: Int32 = 0

    init(pid: pid_t) {
        self.pid = pid
        exited.enter()
        DispatchQueue.global().async { [self] in
            while waitpid(pid, &status, 0) < 0 {
                if errno != EINTR { status = 1 << 8; break }
            }
            exited.leave()
        }
    }

    func stop() {
        if exited.wait(timeout: .now()) == .timedOut { _ = kill(pid, SIGKILL) }
        exited.wait()
    }

    var returnCode: Int32 {
        status & 0x7f == 0 ? (status >> 8) & 0xff : -(status & 0x7f)
    }
}

private final class CheckInterruption: @unchecked Sendable {
    private let lock = NSLock()
    private var child: ChildProcess?
    private var directories: [URL] = []
    private var cancelled = false
    private let source = DispatchSource.makeSignalSource(signal: SIGINT, queue: .global())

    init() {
        signal(SIGINT) { _ in }
        source.setEventHandler { [self] in
            lock.lock()
            cancelled = true
            let running = child
            lock.unlock()
            running?.stop()
            lock.lock()
            let paths = directories
            lock.unlock()
            for path in paths { try? FileManager.default.removeItem(at: path) }
            exit(130)
        }
        source.resume()
    }

    func spawn(path: String, arguments: [String], environment: [String: String],
               actions: inout posix_spawn_file_actions_t?, attributes: inout posix_spawnattr_t?) throws -> ChildProcess {
        let argv = arguments.map { strdup($0) } + [nil]
        let env = environment.map { strdup($0.key + "=" + $0.value) } + [nil]
        defer {
            for value in argv { free(value) }
            for value in env { free(value) }
        }
        lock.lock()
        guard !cancelled else { lock.unlock(); finishCancellation(); throw CheckFailure(description: "Interrupted") }
        defer { lock.unlock() }
        var pid: pid_t = 0
        let result = argv.withUnsafeBufferPointer { argv in
            env.withUnsafeBufferPointer { env in
                posix_spawn(&pid, path, &actions, &attributes,
                            UnsafeMutablePointer(mutating: argv.baseAddress!),
                            UnsafeMutablePointer(mutating: env.baseAddress!))
            }
        }
        guard result == 0 else { throw CheckFailure(description: "\(arguments[0]): \(String(cString: strerror(result)))") }
        let process = ChildProcess(pid: pid)
        child = process
        return process
    }

    func clearChild() {
        lock.lock()
        child = nil
        lock.unlock()
    }

    func finishCancellation() {
        lock.lock()
        let stopping = cancelled
        lock.unlock()
        if stopping { dispatchMain() }
    }

    func temporaryDirectory(prefix: String) throws -> URL {
        lock.lock()
        guard !cancelled else { lock.unlock(); finishCancellation(); throw CheckFailure(description: "Interrupted") }
        defer { lock.unlock() }
        var template = Array(("/tmp/" + prefix + "XXXXXX").utf8CString)
        guard mkdtemp(&template) != nil else { throw CheckFailure(description: "Could not create temporary directory: \(String(cString: strerror(errno)))") }
        let directory = URL(fileURLWithPath: String(cString: template), isDirectory: true)
        directories.append(directory)
        return directory
    }
}

private final class ChildOutput: @unchecked Sendable {
    let finished = DispatchGroup()
    var data = Data()

    init(descriptor: Int32) {
        finished.enter()
        DispatchQueue.global().async { [self] in
            data = FileHandle(fileDescriptor: descriptor, closeOnDealloc: false).readDataToEndOfFile()
            finished.leave()
        }
    }
}

private func addWorkingDirectory(_ directory: String, actions: inout posix_spawn_file_actions_t?) throws {
    typealias AddChdir = @convention(c) (UnsafeMutablePointer<posix_spawn_file_actions_t?>?, UnsafePointer<CChar>?) -> Int32
    let symbols = UnsafeMutableRawPointer(bitPattern: -2) // RTLD_DEFAULT
    guard let symbol = dlsym(symbols, "posix_spawn_file_actions_addchdir") ?? dlsym(symbols, "posix_spawn_file_actions_addchdir_np") else {
        try fail("Could not set child working directory")
    }
    let function = unsafeBitCast(symbol, to: AddChdir.self)
    let result = directory.withCString { function(&actions, $0) }
    if result != 0 { try fail(String(cString: strerror(result))) }
}

private let interruption = CheckInterruption()

private func executable(_ name: String, environment: [String: String] = ProcessInfo.processInfo.environment, directory: URL? = nil) -> String? {
    let workingDirectory = directory?.path ?? FileManager.default.currentDirectoryPath
    func absolute(_ path: String) -> String { path.hasPrefix("/") ? path : workingDirectory + "/" + path }
    if name.contains("/") { let path = absolute(name); return access(path, X_OK) == 0 ? path : nil }
    for part in (environment["PATH"] ?? "/usr/bin:/bin").split(separator: ":", omittingEmptySubsequences: false) {
        let path = absolute(part.isEmpty ? name : String(part) + "/" + name)
        var isDirectory: ObjCBool = false
        if access(path, X_OK) == 0 && FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory) && !isDirectory.boolValue { return path }
    }
    return nil
}

@discardableResult private func run(_ arguments: [String], directory: URL? = nil,
                                  environment: [String: String]? = nil,
                                  timeout: TimeInterval? = nil, capture: Bool = false) throws -> String {
    let environment = environment ?? ProcessInfo.processInfo.environment
    guard let path = executable(arguments[0], environment: environment, directory: directory) else { try fail("No such file or directory: \(arguments[0])") }
    var actions: posix_spawn_file_actions_t?
    var attributes: posix_spawnattr_t?
    func require(_ result: Int32) throws {
        if result != 0 { try fail(String(cString: strerror(result))) }
    }
    try require(posix_spawn_file_actions_init(&actions))
    defer { posix_spawn_file_actions_destroy(&actions) }
    try require(posix_spawnattr_init(&attributes))
    defer { posix_spawnattr_destroy(&attributes) }
    try require(posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_CLOEXEC_DEFAULT)))
    for descriptor in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] where fcntl(descriptor, F_GETFD) >= 0 {
        try require(posix_spawn_file_actions_addinherit_np(&actions, descriptor))
    }
    if let directory { try addWorkingDirectory(directory.path, actions: &actions) }
    var descriptors: [Int32] = [-1, -1]
    defer { for descriptor in descriptors where descriptor >= 0 { close(descriptor) } }
    if capture {
        guard pipe(&descriptors) == 0 else { try fail(String(cString: strerror(errno))) }
        try require(posix_spawn_file_actions_adddup2(&actions, descriptors[1], STDOUT_FILENO))
        try require(posix_spawn_file_actions_addclose(&actions, descriptors[0]))
        try require(posix_spawn_file_actions_addclose(&actions, descriptors[1]))
    }
    let process = try interruption.spawn(path: path, arguments: arguments,
        environment: environment, actions: &actions, attributes: &attributes)
    defer { interruption.clearChild() }
    if descriptors[1] >= 0 { close(descriptors[1]); descriptors[1] = -1 }
    let output = capture ? ChildOutput(descriptor: descriptors[0]) : nil
    let result = timeout.map { process.exited.wait(timeout: .now() + $0) } ?? { process.exited.wait(); return .success }()
    if result == .timedOut {
        process.stop()
        output?.finished.wait()
        try fail("Command \(arguments) timed out after \(timeout!) seconds")
    }
    output?.finished.wait()
    interruption.finishCancellation()
    guard process.returnCode == 0 else {
        try fail("Command \(arguments) returned non-zero exit status \(process.returnCode)")
    }
    guard let text = String(data: output?.data ?? Data(), encoding: .utf8) else { try fail("Command output is not valid UTF-8") }
    return text
}

private func temporaryDirectory(prefix: String) throws -> URL {
    try interruption.temporaryDirectory(prefix: prefix)
}

private func resolvedURL(_ path: String) -> URL {
    let url = URL(fileURLWithPath: path)
    if let resolved = realpath(url.path, nil) {
        defer { free(resolved) }
        return URL(fileURLWithPath: String(cString: resolved))
    }
    guard url.path != "/" else { return url }
    return resolvedURL(url.deletingLastPathComponent().path).appendingPathComponent(url.lastPathComponent)
}

private let repository = resolvedURL(#filePath).deletingLastPathComponent().deletingLastPathComponent()
private let files = FileManager.default

private let check = #####"""

import Foundation
import BuildCore

@main struct Check {
    static func main() async throws {
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let mode = CommandLine.arguments[2]
        let realBun = URL(fileURLWithPath: CommandLine.arguments[3])
        let realNode = URL(fileURLWithPath: CommandLine.arguments[4])
        let realNpm = URL(fileURLWithPath: CommandLine.arguments[5])
        let files = FileManager.default
        func link(_ target: URL, _ path: URL) throws {
            try files.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
            try files.createSymbolicLink(at: path, withDestinationURL: target)
        }
        func script(_ path: URL, _ text: String) throws {
            try files.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
            try text.write(to: path, atomically: true, encoding: .utf8)
            try files.setAttributes([.posixPermissions: 0o755], ofItemAtPath: path.path)
        }
        let minimal = "/usr/bin:/bin:/usr/sbin:/sbin"
        let base = ["PATH": minimal, "SHELL": "/usr/bin/false", "PI_CODING_AGENT_DIR": PiPaths.agentDirectory.path]
        var selected: PiToolchain?
        if mode == "bun" {
            for layout in ["opt/homebrew/bin", "usr/local/bin", ".bun/bin", ".npm-global/bin"] {
                let home = root.appendingPathComponent(layout.replacingOccurrences(of: "/", with: "-"))
                let binary = home.appendingPathComponent(layout + "/bun")
                try link(realBun, binary)
                let system = layout.hasPrefix(".") ? [] : [binary.deletingLastPathComponent().path]
                let tools = await PiToolchain.discover(environment: base, home: home, systemDirectories: system)
                precondition(tools.bun == binary && tools.node == nil && tools.npm == nil)
                print("PASS Finder discovery: \(layout), no Node/npm"); fflush(nil)
                selected = tools
            }
            let home = root.appendingPathComponent("custom")
            let binary = home.appendingPathComponent("bun prefix/bin/bun")
            try link(realBun, binary)
            let custom = await PiToolchain.discover(environment: base.merging(["BUN_INSTALL": home.appendingPathComponent("bun prefix").path]) { _, b in b }, home: home, systemDirectories: [])
            precondition(custom.bun == binary)
            print("PASS custom BUN_INSTALL directory"); fflush(nil)

            let shellHome = root.appendingPathComponent("shell")
            try files.createDirectory(at: shellHome, withIntermediateDirectories: true)
            let zshrc = shellHome.appendingPathComponent(".zshrc")
            let quotedBin = binary.deletingLastPathComponent().path.replacingOccurrences(of: "'", with: "'\\''")
            try "export PATH='\(quotedBin)':$PATH\necho profile-noise\n".write(to: zshrc, atomically: true, encoding: .utf8)
            let shell = await PiToolchain.discover(environment: base.merging(["SHELL": "/bin/zsh", "ZDOTDIR": shellHome.path]) { _, b in b }, home: shellHome, systemDirectories: [])
            precondition(shell.bun == binary)
            print("PASS interactive .zshrc-only install with startup output"); fflush(nil)

            let prefixHome = root.appendingPathComponent("npm-prefix")
            let bin = prefixHome.appendingPathComponent("tools")
            try link(realNode, bin.appendingPathComponent("node"))
            try link(realNpm, bin.appendingPathComponent("npm"))
            let prefix = prefixHome.appendingPathComponent("global packages")
            try link(realBun, prefix.appendingPathComponent("bin/bun"))
            let config = prefixHome.appendingPathComponent("npmrc")
            try "prefix=\(prefix.path)\n".write(to: config, atomically: true, encoding: .utf8)
            let npmTools = await PiToolchain.discover(environment: base.merging(["PATH": bin.path + ":" + minimal,
                "NPM_CONFIG_USERCONFIG": config.path]) { _, b in b }, home: prefixHome, systemDirectories: [])
            precondition(npmTools.bun == prefix.appendingPathComponent("bin/bun"))
            print("PASS npm custom global prefix absent from PATH"); fflush(nil)

            let hung = root.appendingPathComponent("hung-shell")
            try script(hung, "#!/bin/sh\nexec /bin/sleep 30\n")
            let start = Date()
            let bounded = await PiToolchain.discover(environment: base.merging(["SHELL": hung.path,
                "BUN_INSTALL": home.appendingPathComponent("bun prefix").path]) { _, b in b }, home: home, systemDirectories: [])
            precondition(bounded.bun == binary && Date().timeIntervalSince(start) < 9)
            print("PASS shell startup timeout still discovers installed Bun"); fflush(nil)
        } else {
            let home = root.appendingPathComponent("nvm")
            let bin = home.appendingPathComponent(".nvm/versions/node/v25.0.0/bin")
            try link(realNode, bin.appendingPathComponent("node"))
            try link(realNpm, bin.appendingPathComponent("npm"))
            let config = home.appendingPathComponent("npmrc")
            try "prefix=\(home.appendingPathComponent("empty-prefix").path)\n".write(to: config, atomically: true, encoding: .utf8)
            selected = await PiToolchain.discover(environment: base.merging(["NPM_CONFIG_USERCONFIG": config.path]) { _, b in b }, home: home, systemDirectories: [])
            precondition(selected!.node == bin.appendingPathComponent("node") && selected!.npm == bin.appendingPathComponent("npm") && selected!.bun == nil)
            print("PASS nvm Node/npm discovery without shell initialization"); fflush(nil)
        }
        let tools = selected!
        try files.createDirectory(at: PiPaths.agentDirectory, withIntermediateDirectories: true)
        let credentials = Data("{\"fake-only\":{\"type\":\"api_key\",\"key\":\"local-test-only\"}}".utf8)
        try credentials.write(to: PiPaths.authFileURL)
        let legacyPackage = PiPaths.runtimeDirectory.appendingPathComponent("lib/node_modules/@mariozechner/pi-coding-agent")
        let legacy = legacyPackage.appendingPathComponent("dist/cli.js")
        try script(legacy, "#!/usr/bin/env node\nconsole.log('legacy');\n")
        try "{\"name\":\"@mariozechner/pi-coding-agent\",\"version\":\"0.73.0\"}".write(
            to: legacyPackage.appendingPathComponent("package.json"), atomically: true, encoding: .utf8)
        try link(legacy, PiPaths.runtimeExecutable)
        precondition(PiRuntimeInstaller.installedVersion() == "0.73.0", "Version must follow the installed launcher")
        try await PiRuntimeInstaller.install(toolchain: tools)
        let preservedCredentials = try Data(contentsOf: PiPaths.authFileURL)
        precondition(preservedCredentials == credentials)
        precondition(!files.fileExists(atPath: legacy.path), "The obsolete package tree must retire after a successful upgrade")
        precondition(PiRuntimeInstaller.installedVersion() == PiRuntimeInstaller.desiredVersion)
        precondition(PiExecutableLocator.resolve(environment: tools.environment) == PiPaths.runtimeExecutable)
        let unavailable = PiToolchain(environment: tools.environment, bun: URL(fileURLWithPath: "/usr/bin/false"))
        do {
            try await PiRuntimeInstaller.install(toolchain: unavailable)
            preconditionFailure("A failed pinned reinstall must not report success")
        } catch {
            precondition(PiRuntimeInstaller.installedVersion() == PiRuntimeInstaller.desiredVersion,
                         "Pinned startup updates must preserve the working runtime")
        }
        print("PASS \(mode) real install and version check; credentials preserved and legacy layout migrated"); fflush(nil)
        let latest = CommandLine.arguments[6]
        if !latest.isEmpty {
            let settings = Data("{\"test-setting\":true}".utf8)
            let models = Data("{\"providers\":{}}".utf8)
            try settings.write(to: PiPaths.settingsFileURL)
            try models.write(to: PiPaths.modelsFileURL)
            try await PiRuntimeInstaller.install(latest: true, toolchain: tools)
            precondition(PiRuntimeInstaller.installedVersion() == latest)
            let bad = PiToolchain(environment: tools.environment, bun: URL(fileURLWithPath: "/usr/bin/false"))
            do {
                try await PiRuntimeInstaller.install(latest: true, toolchain: bad)
                preconditionFailure("A failed package manager must not report a successful update")
            } catch {
                precondition(PiRuntimeInstaller.installedVersion() == latest, "Failed update must restore the working runtime")
            }
            let actualCredentials = try Data(contentsOf: PiPaths.authFileURL)
            let actualSettings = try Data(contentsOf: PiPaths.settingsFileURL)
            let actualModels = try Data(contentsOf: PiPaths.modelsFileURL)
            precondition(actualCredentials == credentials && actualSettings == settings && actualModels == models)
            let remaining = try files.contentsOfDirectory(atPath: root.path)
            precondition(!remaining.contains { $0.hasPrefix("pi-runtime-backup-") })
            print("PASS \(mode) latest update to \(latest), failure rollback and auth/settings preservation"); fflush(nil)
        }
        let launch = try tools.launch(PiPaths.runtimeExecutable, arguments: ["--mode", "rpc", "--no-session"])
        precondition(launch.executable == (mode == "bun" ? tools.bun! : tools.node!))
        let process = PiAgentProcess(executableURL: launch.executable, workingDirectory: root,
                                     arguments: launch.arguments, environment: tools.environment)
        try process.start(onExit: {})
        defer { process.terminate() }
        try process.send(.getState, id: "runtime-check")
        var receivedState = false
        for await event in process.events {
            if event.string("id") == "runtime-check" {
                precondition(event.object["success"] as? Bool == true, process.stderrText)
                receivedState = true
                print("PASS \(mode) live RPC get_state, no model request"); fflush(nil)
                break
            }
        }
        precondition(receivedState, "RPC exited without a state response: \(process.stderrText)")
        for logout in [false, true] {
            let script = try await PiRuntimeInstaller.authenticationScript(logout: logout, toolchain: tools)
            let result = try await ProcessRunner().run(DirectCommandPlan(executable: "/bin/zsh", arguments: ["-n", script.path]),
                                                      projectRoot: root, timeout: .seconds(5))
            precondition(result.termination == .exited(code: 0))
            let text = try String(contentsOf: script, encoding: .utf8)
            precondition(text.contains(launch.executable.path.replacingOccurrences(of: "'", with: "'\\''")) && text.contains("export PATH="))
        }
        print("PASS \(mode) login/logout launch scripts use the discovered runtime and PATH"); fflush(nil)
    }
}

"""#####

do {
    guard CommandLine.arguments.count > 1 else { try fail("Provide the Release build products path") }
    let products = resolvedURL(CommandLine.arguments[1])
    guard ProcessInfo.processInfo.environment["PATH"] != "", let bun = executable("bun"), let node = executable("node"), let npm = executable("npm") else {
        try fail("Install Bun and Node/npm to exercise both runtime routes")
    }
    let latest = CommandLine.arguments.contains("--latest")
        ? try run(["npm", "view", "@earendil-works/pi-coding-agent@latest", "version"], capture: true).trimmingCharacters(in: .whitespacesAndNewlines) : ""
    let root = try temporaryDirectory(prefix: "pitex runtime's ")
    defer { try? files.removeItem(at: root) }
    try files.createDirectory(at: root.appendingPathComponent("PitexAgent/skills"), withIntermediateDirectories: true)
    let source = root.appendingPathComponent("Check.swift")
    try check.write(to: source, atomically: false, encoding: .utf8)
    let program = root.appendingPathComponent("check")
    let features = repository.appendingPathComponent("Mac/Sources/Features")
    try run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-I", products.path, source.path]
        + ["PiAgentProcess.swift", "PiAuthStore.swift", "PiRPC.swift"].map { features.appendingPathComponent($0).path }
        + ["BuildCore", "TexDomain"].map { products.appendingPathComponent($0 + ".o").path } + ["-o", program.path])
    for mode in ["bun", "node"] {
        let work = root.appendingPathComponent(mode)
        try files.createDirectory(at: work, withIntermediateDirectories: false)
        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = "/usr/bin:/bin:/usr/sbin:/sbin"
        environment["PI_CODING_AGENT_DIR"] = work.appendingPathComponent("pi").path
        try run([program.path, work.path, mode, bun, node, npm, latest], environment: environment, timeout: 480)
    }
} catch {
    interruption.finishCancellation()
    fputs("\(error)\n", stderr)
    exit(1)
}
