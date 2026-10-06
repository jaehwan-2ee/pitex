#!/usr/bin/env swift
// After a Release build: check-macos-release.swift <Build/Products/Release>
//
// Checks editor highlighting on real NSTextView storage and the app's build
// executor under Finder's minimal PATH. Requires MacTeX; uses temporary files.
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

import AppKit
import AppPorts
import AppShell
import BuildCore

private actor Doc: DocumentSessionPort {
    let text: String
    init(_ text: String) { self.text = text }
    func snapshot() async -> DocumentSnapshot { .init(revision: 0, text: text) }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult {
        fatalError("Highlighting must never edit the document")
    }
}

@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        for (ext, line, needle, role) in [
            ("tex", "한글 e\u{301} 👩‍💻 \\section{Title} $x^2$ % note\n", "\\section", AppearanceColorRole.commands),
            ("bib", "@article{ref, title={한글 e\u{301} 👩‍💻}, year={2026}}\n", "@", AppearanceColorRole.environments),
        ] {
            let source = String(repeating: line, count: 1000)
            let environment = try await AppShell.make(documentSession: Doc(source))
            let highlighter = SyntaxHighlighter()
            let start = ContinuousClock.now
            highlighter.attach(to: environment.editor, fileExtension: ext)
            let elapsed = start.duration(to: .now)
            precondition(elapsed < .seconds(3), "Highlighting regressed to repeated full-string scans")
            let view = environment.editor.textView
            precondition(view.string == source)
            let index = (source as NSString).range(of: needle, options: .backwards).location
            let color = view.textStorage!.attribute(.foregroundColor, at: index, effectiveRange: nil) as! NSColor
            precondition(color == AppearanceSettings.shared.color(for: role), "Wrong UTF-16 range after Unicode text")
            print("PASS \(ext) highlighting, Unicode offsets and unchanged source: \(elapsed)")
            highlighter.detach()
        }

        precondition(ProcessInfo.processInfo.environment["PATH"] == "/usr/bin:/bin:/usr/sbin:/sbin")
        let source = root.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\begin{document}\nFinder build.\\end{document}\n".write(to: source, atomically: true, encoding: .utf8)
        let executor = StreamingBuildExecutor()
        for engine in ["xelatex", "latexmk"] {
            for suffix in ["pdf", "synctex.gz"] {
                try? FileManager.default.removeItem(at: root.appendingPathComponent("main." + suffix))
            }
            let arguments = (engine == "latexmk" ? ["-pdf"] : []) + ["-synctex=1", "-interaction=nonstopmode", "-halt-on-error", "main.tex"]
            let request = BuildProcessRequest(buildID: try BuildID(rawValue: engine), stageIndex: 0,
                command: .direct(try DirectCommandPlan(executable: engine, arguments: arguments)),
                projectRoot: root, sourceDirectory: root)
            let result = try await executor.execute(request) { _ in }
            precondition(result.exitCode == 0, "\(engine) failed under Finder PATH")
            let pdf = try Data(contentsOf: root.appendingPathComponent("main.pdf"))
            precondition(pdf.starts(with: Data("%PDF".utf8)))
            precondition(FileManager.default.fileExists(atPath: root.appendingPathComponent("main.synctex.gz").path))
            print("PASS Finder PATH: \(engine) creates PDF and SyncTeX, including child tools")
        }
    }
}

"""##
let root = temporary("pitex release 한글 ")
defer { try? files.removeItem(at: root) }
let executor = root.appendingPathComponent("Executor.swift")
write(read(features.appendingPathComponent("BuildSupport.swift")).components(separatedBy: "enum WorkspaceBuildError:")[0], to: executor)
let source = root.appendingPathComponent("Check.swift")
write(check, to: source)
let executable = root.appendingPathComponent("check")
run(["xcrun", "swiftc", "-O", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0", "-I", products.path,
     source.path, executor.path, features.appendingPathComponent("SyntaxHighlighting.swift").path, features.appendingPathComponent("AppearanceTheme.swift").path]
     + rawChildren(products.path).filter { $0.hasSuffix(".o") } + ["-o", executable.path])
var environment = ProcessInfo.processInfo.environment
environment["PATH"] = "/usr/bin:/bin:/usr/sbin:/sbin"
run([executable.path, root.path], cwd: root, environment: environment, timeout: 60)
