#!/usr/bin/env swift
// After a Release build: check-assistant-font.swift <Build/Products/Release>.
// Checks persisted font preference and real SwiftUI sizing for every message role.
// No assistant requests or user account access.
import Foundation
import Darwin

let files = FileManager.default
let lifecycleLock = NSLock()
var cleanupDirectories: [URL] = []
final class RunningChild: @unchecked Sendable {
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

var activeChild: RunningChild?
var interruptionRequested = false
func cleanup() {
    lifecycleLock.lock()
    let directories = cleanupDirectories
    cleanupDirectories.removeAll()
    lifecycleLock.unlock()
    for directory in directories { try? files.removeItem(at: directory) }
}
atexit { cleanup() }
// A handled signal becomes SIG_DFL in an exec child, preserving its normal Ctrl-C behavior.
signal(SIGINT) { _ in }
let interruptSource = DispatchSource.makeSignalSource(signal: SIGINT, queue: .global())
interruptSource.setEventHandler {
    lifecycleLock.lock()
    interruptionRequested = true
    let child = activeChild
    lifecycleLock.unlock()
    child?.stop()
    cleanup()
    exit(130)
}
interruptSource.resume()
func resolvedPath(_ path: String) -> String {
    guard let resolved = realpath(path, nil) else { return URL(fileURLWithPath: path).standardizedFileURL.path }
    defer { free(resolved) }
    return String(cString: resolved)
}
func resolved(_ path: String) -> URL { URL(fileURLWithPath: resolvedPath(path)) }
let repoPath = "/" + resolvedPath(CommandLine.arguments[0]).split(separator: "/").dropLast(2).joined(separator: "/")
let repo = URL(fileURLWithPath: repoPath)
// URL filesystem paths decompose Unicode; retain the original canonical directory bytes in argv.
var pathPrefixes: [(url: String, raw: String)] = [(repo.path, repoPath)]
func nativeArgument(_ value: String) -> String {
    for prefix in pathPrefixes {
        if value == prefix.url { return prefix.raw }
        if value.hasPrefix(prefix.url + "/") { return prefix.raw + value.dropFirst(prefix.url.count) }
    }
    return value
}
func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}
func read(_ url: URL) -> String {
    do { return try String(contentsOf: url, encoding: .utf8).replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n") }
    catch { fail("\(url.path): \(error)") }
}
func write(_ text: String, to url: URL) {
    do { try text.write(to: url, atomically: false, encoding: .utf8) }
    catch { fail("\(url.path): \(error)") }
}
func rawChildren(_ path: String) -> [String] {
    guard let directory = opendir(path) else { return [] }
    defer { closedir(directory) }
    var result: [String] = []
    while let entry = readdir(directory) {
        let name = withUnsafePointer(to: &entry.pointee.d_name) { String(cString: UnsafeRawPointer($0).assumingMemoryBound(to: CChar.self)) }
        if name != "." && name != ".." { result.append(path + "/" + name) }
    }
    return result
}
func children(_ url: URL) -> [URL] { rawChildren(url.path).map { URL(fileURLWithPath: $0) } }
func temporary(_ prefix: String, in directory: String = "/tmp", cleanup: Bool = true) -> URL {
    var pattern = Array(((directory as NSString).appendingPathComponent(prefix + "XXXXXX")).utf8CString)
    lifecycleLock.lock()
    if interruptionRequested { lifecycleLock.unlock(); dispatchMain() }
    guard let path = mkdtemp(&pattern) else {
        let message = String(cString: strerror(errno))
        lifecycleLock.unlock()
        fail("Cannot create temporary directory: \(message)")
    }
    let result = URL(fileURLWithPath: String(cString: path))
    if cleanup { cleanupDirectories.append(result) }
    lifecycleLock.unlock()
    return result
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
        try rawFail("Could not set child working directory")
    }
    let function = unsafeBitCast(symbol, to: AddChdir.self)
    let result = directory.withCString { function(&actions, $0) }
    if result != 0 { try rawFail(String(cString: strerror(result))) }
}

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

@discardableResult private func runRaw(_ arguments: [String], directory: URL? = nil,
                                  environment: [String: String]? = nil,
                                  timeout: TimeInterval? = nil, capture: Bool = false) throws -> String {
    let environment = environment ?? ProcessInfo.processInfo.environment
    guard let path = executable(arguments[0], environment: environment, directory: directory) else { try rawFail("No such file or directory: \(arguments[0])") }
    var actions: posix_spawn_file_actions_t?
    var attributes: posix_spawnattr_t?
    func require(_ result: Int32) throws {
        if result != 0 { try rawFail(String(cString: strerror(result))) }
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
        guard pipe(&descriptors) == 0 else { try rawFail(String(cString: strerror(errno))) }
        try require(posix_spawn_file_actions_adddup2(&actions, descriptors[1], STDOUT_FILENO))
        try require(posix_spawn_file_actions_addclose(&actions, descriptors[0]))
        try require(posix_spawn_file_actions_addclose(&actions, descriptors[1]))
    }
    let process = try spawn(path: path, arguments: arguments,
        environment: environment, actions: &actions, attributes: &attributes)
    defer { clearChild() }
    if descriptors[1] >= 0 { close(descriptors[1]); descriptors[1] = -1 }
    let output = capture ? ChildOutput(descriptor: descriptors[0]) : nil
    let result = timeout.map { process.exited.wait(timeout: .now() + $0) } ?? { process.exited.wait(); return .success }()
    if result == .timedOut {
        process.stop()
        output?.finished.wait()
        try rawFail("Command \(arguments) timed out after \(timeout!) seconds")
    }
    output?.finished.wait()
    finishCancellation()
    guard process.returnCode == 0 else {
        try rawFail("Command \(arguments) returned non-zero exit status \(process.returnCode)")
    }
    guard let text = String(data: output?.data ?? Data(), encoding: .utf8) else { try rawFail("Command output is not valid UTF-8") }
    return text
}


struct ToolFailure: Error, CustomStringConvertible { let description: String }
func rawFail(_ message: String) throws -> Never { throw ToolFailure(description: message) }
func spawn(path: String, arguments: [String], environment: [String: String],
           actions: inout posix_spawn_file_actions_t?, attributes: inout posix_spawnattr_t?) throws -> RunningChild {
    let argv = arguments.map { strdup(nativeArgument($0)) } + [nil]
    let env = environment.map { strdup($0.key + "=" + $0.value) } + [nil]
    defer { for value in argv { free(value) }; for value in env { free(value) } }
    lifecycleLock.lock()
    if interruptionRequested { lifecycleLock.unlock(); dispatchMain() }
    defer { lifecycleLock.unlock() }
    var pid: pid_t = 0
    let result = argv.withUnsafeBufferPointer { argv in
        env.withUnsafeBufferPointer { env in
            posix_spawn(&pid, path, &actions, &attributes,
                        UnsafeMutablePointer(mutating: argv.baseAddress!),
                        UnsafeMutablePointer(mutating: env.baseAddress!))
        }
    }
    if result != 0 { try rawFail("\(arguments[0]): \(String(cString: strerror(result)))") }
    let process = RunningChild(pid: pid)
    activeChild = process
    return process
}
func clearChild() {
    lifecycleLock.lock()
    activeChild = nil
    lifecycleLock.unlock()
}
func finishCancellation() {
    lifecycleLock.lock()
    let interrupted = interruptionRequested
    lifecycleLock.unlock()
    if interrupted { dispatchMain() }
}
@discardableResult
func run(_ arguments: [String], cwd: URL? = nil, environment: [String: String]? = nil,
         timeout: TimeInterval? = nil, capture: Bool = false) -> String {
    do {
        return try runRaw(arguments, directory: cwd, environment: environment, timeout: timeout, capture: capture)
            .replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
    } catch {
        finishCancellation()
        fail(String(describing: error))
    }
}
guard CommandLine.arguments.count > 1 else { fail("Usage: " + CommandLine.arguments[0] + " <Build/Products/Release>") }
let productsPath = resolvedPath(CommandLine.arguments[1])
let products = URL(fileURLWithPath: productsPath)
pathPrefixes.insert((products.path, productsPath), at: 0)
let features = repo.appendingPathComponent("Mac/Sources/Features")
let check = ##"""

@main struct Check {
    @MainActor static func main() throws {
        _ = NSApplication.shared
        let defaults = UserDefaults.standard
        let previous = defaults.object(forKey: "ai.fontSize")
        defer {
            if let previous { defaults.set(previous, forKey: "ai.fontSize") }
            else { defaults.removeObject(forKey: "ai.fontSize") }
        }
        defaults.removeObject(forKey: "ai.fontSize")
        let settings = SettingsStore()
        precondition(settings.aiFontSize == 13)
        settings.aiFontSize = 21
        precondition(SettingsStore().aiFontSize == 21, "Font size must survive a new settings instance")
        print("PASS assistant font preference defaults to 13 pt and persists")
        for role in [AgentTranscriptEntry.Role.user, .assistant, .thinking, .tool, .notice] {
            let entry = AgentTranscriptEntry(role: role, title: "Message title", text: "Conversation text with a readable font.", detail: "Tool result")
            func height(_ size: Double) -> CGFloat {
                let host = NSHostingView(rootView: AgentEntryRow(entry: entry, fontSize: size).frame(width: 300))
                return host.fittingSize.height
            }
            let small = height(10), large = height(24)
            precondition(large > small, "Font change must resize every transcript role")
            print("PASS \(role): rendered height \(small) → \(large)")
        }
    }
}

"""##
let root = temporary("pitex-assistant-font-")
defer { try? files.removeItem(at: root) }
let settings = read(features.appendingPathComponent("SettingsView.swift")).components(separatedBy: "/// Command presets")[0]
let coordinator = read(features.appendingPathComponent("AgentCoordinator.swift"))
guard let entryStart = coordinator.range(of: "struct AgentTranscriptEntry:"),
      let entryEnd = coordinator.range(of: "/// Drives the assistant panel") else { fail("Missing AgentTranscriptEntry source") }
let entry = String(coordinator[entryStart.lowerBound..<entryEnd.lowerBound])
let panel = read(features.appendingPathComponent("AgentPanel.swift"))
guard let rowStart = panel.range(of: "private struct AgentEntryRow:") else { fail("Missing AgentEntryRow source") }
guard let rowEnd = panel.range(of: "/// The resumable conversations of the current project") else { fail("Missing AgentEntryRow end") }
let row = String(panel[rowStart.lowerBound..<rowEnd.lowerBound])
let source = root.appendingPathComponent("Check.swift")
write(settings + entry + row + check, to: source)
let executable = root.appendingPathComponent("check")
run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0",
     "-I", products.path, source.path] + rawChildren(products.path).filter { $0.hasSuffix(".o") } + ["-o", executable.path])
run([executable.path], timeout: 30)
