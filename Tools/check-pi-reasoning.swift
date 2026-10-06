#!/usr/bin/env swift
// After building Pitex: Tools/check-pi-reasoning.swift <Build/Products/Release>.
// Uses local model metadata only; no prompts, network model requests or user credentials.
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

private func objectPaths(in directory: String) -> [String] {
    guard let handle = opendir(directory) else { return [] }
    defer { closedir(handle) }
    var paths: [String] = []
    while let entry = readdir(handle) {
        var name = entry.pointee.d_name
        let text = withUnsafePointer(to: &name) { pointer in
            String(cString: UnsafeRawPointer(pointer).assumingMemoryBound(to: CChar.self))
        }
        if text.hasSuffix(".o") { paths.append(directory + "/" + text) }
    }
    return paths.sorted { $0.utf8.lexicographicallyPrecedes($1.utf8) }
}

private let repository = resolvedURL(#filePath).deletingLastPathComponent().deletingLastPathComponent()
private let files = FileManager.default

private let models = #####"""
{"providers": {"pitex-test": {"api": "openai-completions", "baseUrl": "http://127.0.0.1:1/v1", "apiKey": "local-test-only", "models": [{"id": "ordinary", "reasoning": false}, {"id": "limited", "reasoning": true, "thinkingLevelMap": {"off": null, "minimal": null, "medium": null, "xhigh": null, "max": null}}, {"id": "maximum", "reasoning": true, "thinkingLevelMap": {"minimal": null, "xhigh": null, "max": "max"}}, {"id": "extended", "reasoning": true, "thinkingLevelMap": {"xhigh": "xhigh", "max": "max"}}]}}}
"""#####
private let settings = #####"""
{"defaultProvider": "pitex-test", "defaultModel": "ordinary"}
"""#####

private let check = #####"""

import AppKit
import AppPorts
import AppShell
import Foundation

private actor TestDocument: DocumentSessionPort {
    func snapshot() async -> DocumentSnapshot { .init(revision: 0, text: "") }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult {
        .rejected(current: await snapshot())
    }
}

@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let environment = try await AppShell.make(documentSession: TestDocument())
        let coordinator = AgentCoordinator(environment: environment)
        coordinator.contextProvider = {
            AgentContextSnapshot(projectRoot: URL(fileURLWithPath: CommandLine.arguments[1]))
        }
        coordinator.preferredModelID = "maximum"
        coordinator.prepare()
        defer { coordinator.shutdown() }

        func settled(model: String, level: String? = nil) async throws {
            for _ in 0..<1500 {
                if coordinator.currentModel?.id == model,
                   !coordinator.isUpdatingModelSettings,
                   !coordinator.thinkingLevels.isEmpty,
                   level == nil || coordinator.thinkingLevel == level { return }
                try await Task.sleep(for: .milliseconds(20))
            }
            fatalError("Settings did not settle: \(model), \(coordinator.statusMessage ?? "no error")")
        }

        try await settled(model: "maximum")
        precondition(coordinator.thinkingLevels == ["off", "low", "medium", "high", "max"])
        coordinator.selectThinkingLevel("max")
        try await settled(model: "maximum", level: "max")
        print("PASS runtime-discovered max; unsupported minimal/xhigh excluded")

        let limited = coordinator.models.first { $0.id == "limited" }!
        coordinator.selectModel(limited)
        precondition(coordinator.isUpdatingModelSettings && coordinator.thinkingLevels.isEmpty)
        try await settled(model: "limited", level: "high")
        precondition(coordinator.thinkingLevels == ["low", "high"])
        coordinator.selectThinkingLevel("minimal")
        precondition(!coordinator.isUpdatingModelSettings && coordinator.thinkingLevel == "high")
        print("PASS model switch clamps max to high; invalid level is not sent")

        coordinator.selectModel(coordinator.models.first { $0.id == "ordinary" }!)
        try await settled(model: "ordinary", level: "off")
        precondition(coordinator.thinkingLevels == ["off"])
        print("PASS non-reasoning model only exposes off")

        coordinator.selectModel(coordinator.models.first { $0.id == "extended" }!)
        try await settled(model: "extended")
        precondition(coordinator.thinkingLevels == ["off", "minimal", "low", "medium", "high", "xhigh", "max"])
        coordinator.selectThinkingLevel("xhigh")
        try await settled(model: "extended", level: "xhigh")
        print("PASS xhigh and max are distinct runtime-supported options")

        coordinator.selectModel(PiModelDescriptor(["id": "missing", "provider": "pitex-test"])!)
        try await settled(model: "extended", level: "xhigh")
        precondition(coordinator.statusMessage?.contains("Model not found") == true)
        precondition(coordinator.thinkingLevels.contains("max"))
        print("PASS rejected model change restores actual model capabilities")
    }
}

"""#####

do {
    guard CommandLine.arguments.count > 1 else { try fail("Provide the Release build products path") }
    let products = resolvedURL(CommandLine.arguments[1])
    let home = ProcessInfo.processInfo.environment["HOME"] ?? NSHomeDirectory()
    let runtime = URL(fileURLWithPath: home).appendingPathComponent("Library/Application Support/Pitex/pi-runtime/bin/pi")
    var runtimeInfo = stat()
    guard fstatat(AT_FDCWD, runtime.path, &runtimeInfo, 0) == 0, runtimeInfo.st_mode & S_IFMT == S_IFREG else {
        try fail("Install the Pitex Agent runtime before running this check")
    }
    let root = try temporaryDirectory(prefix: "pitex-reasoning-")
    defer { try? files.removeItem(at: root) }
    let agent = root.appendingPathComponent("pi")
    try files.createDirectory(at: agent, withIntermediateDirectories: false)
    try models.write(to: agent.appendingPathComponent("models.json"), atomically: false, encoding: .utf8)
    try settings.write(to: agent.appendingPathComponent("settings.json"), atomically: false, encoding: .utf8)
    let source = root.appendingPathComponent("Check.swift")
    try check.write(to: source, atomically: false, encoding: .utf8)
    let program = root.appendingPathComponent("check")
    let features = repository.appendingPathComponent("Mac/Sources/Features")
    let objects = objectPaths(in: products.path)
    try run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0", "-module-cache-path", root.appendingPathComponent("cache").path, "-I", products.path, source.path]
        + ["PiRPC.swift", "PiAuthStore.swift", "PiAgentProcess.swift", "AgentCoordinator.swift", "SubscriptionUsage.swift"].map { features.appendingPathComponent($0).path }
        + objects + ["-o", program.path])
    var environment = ProcessInfo.processInfo.environment
    environment["PI_CODING_AGENT_DIR"] = agent.path
    environment["PI_AGENT_PATH"] = runtime.path
    try run([program.path, root.path], directory: root, environment: environment, timeout: 90)
} catch {
    interruption.finishCancellation()
    fputs("\(error)\n", stderr)
    exit(1)
}
