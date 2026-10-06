#!/usr/bin/env swift
// Native macOS checks for minimap reuse, edit invalidation and completion parity.
import Foundation
import Darwin

struct CheckError: Error, CustomStringConvertible {
    let description: String
    init(_ description: String) { self.description = description }
}
// Match subprocess.run/TemporaryDirectory cleanup when only this driver receives Ctrl+C.
let interruptionLock = NSLock()
var wasInterrupted = false
private var activeProcess: ChildProcess?
var activeCompletion: DispatchGroup?
var temporaryRoots: [URL] = []
let interruptedWait = DispatchGroup()
interruptedWait.enter()
func awaitInterruption() {
    interruptionLock.lock()
    let interrupted = wasInterrupted
    interruptionLock.unlock()
    if interrupted { interruptedWait.wait() }
}
// A caught signal is reset by exec, so the child keeps its normal SIGINT behavior.
signal(SIGINT) { _ in }
let interruptSource = DispatchSource.makeSignalSource(signal: SIGINT, queue: .global())
interruptSource.setEventHandler {
    interruptionLock.lock()
    wasInterrupted = true
    let process = activeProcess, completion = activeCompletion
    let roots = temporaryRoots
    interruptionLock.unlock()
    process?.stop()
    completion?.wait()
    for root in roots { try? FileManager.default.removeItem(at: root) }
    exit(130)
}
interruptSource.resume()
func removeTemporary(_ root: URL) throws {
    awaitInterruption()
    interruptionLock.lock()
    if wasInterrupted {
        interruptionLock.unlock()
        interruptedWait.wait()
    }
    defer { interruptionLock.unlock() }
    try FileManager.default.removeItem(at: root)
    temporaryRoots.removeAll { $0 == root }
}

// Foundation file URLs decompose Unicode path text; command arguments keep its original UTF-8 bytes.
let repoPath: String = {
    guard let resolved = realpath(#filePath, nil) else { return "" }
    defer { free(resolved) }
    return String(cString: resolved).split(separator: "/", omittingEmptySubsequences: false).dropLast(2).joined(separator: "/")
}()
let repo = URL(fileURLWithPath: repoPath, isDirectory: true)
func read(_ path: String) throws -> String {
    try String(contentsOf: repo.appendingPathComponent(path), encoding: .utf8)
        .replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
}
func write(_ text: String, to url: URL) throws {
    try text.write(to: url, atomically: false, encoding: .utf8)
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
        throw CheckError("Could not set child working directory")
    }
    let function = unsafeBitCast(symbol, to: AddChdir.self)
    let result = directory.withCString { function(&actions, $0) }
    if result != 0 { throw CheckError(String(cString: strerror(result))) }
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

private func spawn(path: String, arguments: [String], environment: [String: String],
               actions: inout posix_spawn_file_actions_t?, attributes: inout posix_spawnattr_t?) throws -> ChildProcess {
        let argv = arguments.map { strdup($0) } + [nil]
        let env = environment.map { strdup($0.key + "=" + $0.value) } + [nil]
        defer {
            for value in argv { free(value) }
            for value in env { free(value) }
        }
        awaitInterruption()
        interruptionLock.lock()
        guard !wasInterrupted else { interruptionLock.unlock(); awaitInterruption(); throw CheckError("Interrupted") }
        defer { interruptionLock.unlock() }
        var pid: pid_t = 0
        let result = argv.withUnsafeBufferPointer { argv in
            env.withUnsafeBufferPointer { env in
                posix_spawn(&pid, path, &actions, &attributes,
                            UnsafeMutablePointer(mutating: argv.baseAddress!),
                            UnsafeMutablePointer(mutating: env.baseAddress!))
            }
        }
        guard result == 0 else { throw CheckError( "\(arguments[0]): \(String(cString: strerror(result)))") }
        let process = ChildProcess(pid: pid)
        activeProcess = process
        activeCompletion = process.exited
        return process
    }

@discardableResult func run(_ arguments: [String], capture: Bool = false,
                           timeout: TimeInterval? = nil, directory: URL? = nil) throws -> String {
    let environment = ProcessInfo.processInfo.environment
    guard let path = executable(arguments[0], environment: environment, directory: directory) else { throw CheckError("No such file or directory: \(arguments[0])") }
    var actions: posix_spawn_file_actions_t?
    var attributes: posix_spawnattr_t?
    func require(_ result: Int32) throws {
        if result != 0 { throw CheckError(String(cString: strerror(result))) }
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
        guard pipe(&descriptors) == 0 else { throw CheckError(String(cString: strerror(errno))) }
        try require(posix_spawn_file_actions_adddup2(&actions, descriptors[1], STDOUT_FILENO))
        try require(posix_spawn_file_actions_addclose(&actions, descriptors[0]))
        try require(posix_spawn_file_actions_addclose(&actions, descriptors[1]))
    }
    let process = try spawn(path: path, arguments: arguments,
        environment: environment, actions: &actions, attributes: &attributes)
    defer {
        interruptionLock.lock()
        if activeProcess === process { activeProcess = nil; activeCompletion = nil }
        interruptionLock.unlock()
    }
    if descriptors[1] >= 0 { close(descriptors[1]); descriptors[1] = -1 }
    let output = capture ? ChildOutput(descriptor: descriptors[0]) : nil
    let result = timeout.map { process.exited.wait(timeout: .now() + $0) } ?? { process.exited.wait(); return .success }()
    if result == .timedOut {
        process.stop()
        output?.finished.wait()
        throw CheckError("Command \(arguments) timed out after \(timeout!) seconds")
    }
    output?.finished.wait()
    awaitInterruption()
    guard process.returnCode == 0 else {
        throw CheckError("Command \(arguments) returned non-zero exit status \(process.returnCode)")
    }
    guard let text = String(data: output?.data ?? Data(), encoding: .utf8) else { throw CheckError("Command output is not valid UTF-8") }
    return text.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
}


func slice(_ source: String, from start: String, to end: String) throws -> String {
    guard let first = source.range(of: start), let last = source.range(of: end), first.lowerBound <= last.lowerBound else {
        throw CheckError("substring not found: \(start) / \(end)")
    }
    return String(source[first.lowerBound..<last.lowerBound])
}
func stripImports(_ text: String) -> String {
    var lines = text.components(separatedBy: "\n")
    if lines.last == "" { lines.removeLast() }
    return lines.filter { !$0.hasPrefix("import ") }.joined(separator: "\n")
}
func temporary(_ prefix: String, _ action: (URL) throws -> Void) throws {
    awaitInterruption()
    interruptionLock.lock()
    if wasInterrupted {
        interruptionLock.unlock()
        interruptedWait.wait()
    }
    var template = Array((NSTemporaryDirectory() + prefix + "XXXXXXXX").utf8CString)
    guard let created = mkdtemp(&template) else {
        let message = String(cString: strerror(errno))
        interruptionLock.unlock()
        throw CheckError("Could not create temporary directory: \(message)")
    }
    let root = URL(fileURLWithPath: String(cString: created), isDirectory: true)
    temporaryRoots.append(root)
    interruptionLock.unlock()
    do { try action(root) } catch {
        try removeTemporary(root)
        throw error
    }
    try removeTemporary(root)
}

let host = #"""

import AppKit
@MainActor final class AppearanceSettings {
    enum Role { case bodyText }
    static let shared = AppearanceSettings()
    func color(for role: Role) -> NSColor { .black }
}
func check(_ value: Bool, _ message: String = "check failed", line: Int = #line) {
    if !value {
        FileHandle.standardError.write(Data("line \(line): \(message)\n".utf8))
        fatalError(message)
    }
}
@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        for text in ["", "한👩🏽‍💻 \\cite{a,ke", "e\u{301} \\ref{sec:한", "\\cite[한][p.2]{x", "\\alpha", "\\\\alpha", "\\cite{a,\n b"] {
            for caret in 0...(text.utf16.count + 2) {
                check(CompletionContextDetector.context(in:text,caretUTF16Offset:caret)
                    == BaselineCompletionContextDetector.context(in:text,caretUTF16Offset:caret))
            }
        }
        let text = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
        let scroll = NSScrollView(frame:NSRect(x:0,y:0,width:800,height:600))
        let view = NSTextView(frame:scroll.bounds)
        view.isVerticallyResizable = true
        view.textContainer!.containerSize = NSSize(width:800,height:CGFloat.greatestFiniteMagnitude)
        view.textContainer!.widthTracksTextView = true
        view.font = NSFont.monospacedSystemFont(ofSize:12,weight:.regular)
        view.string = text
        scroll.documentView = view
        view.layoutManager!.ensureLayout(for:view.textContainer!)
        let old = BaselineMinimapOverlayView(textView:view,scrollView:scroll)
        let new = MinimapOverlayView(textView:view,scrollView:scroll)
        func render(_ map: NSView) -> Data {
            let bitmap = NSBitmapImageRep(bitmapDataPlanes:nil,pixelsWide:80,pixelsHigh:600,
                bitsPerSample:8,samplesPerPixel:4,hasAlpha:true,isPlanar:false,colorSpaceName:.deviceRGB,
                bytesPerRow:0,bitsPerPixel:0)!
            NSGraphicsContext.saveGraphicsState()
            NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep:bitmap)
            NSGraphicsContext.current!.cgContext.clear(NSRect(x:0,y:0,width:80,height:600))
            let transform = NSAffineTransform()
            transform.translateX(by:0,yBy:600); transform.scaleX(by:1,yBy:-1); transform.concat()
            map.draw(map.bounds)
            NSGraphicsContext.restoreGraphicsState()
            return Data(bytes:bitmap.bitmapData!,count:bitmap.bytesPerRow * bitmap.pixelsHigh)
        }
        func median(_ action: () -> Void) -> Double {
            var times:[Double] = []
            for _ in 0..<5 {
                let start = DispatchTime.now().uptimeNanoseconds
                action()
                times.append(Double(DispatchTime.now().uptimeNanoseconds-start)/1_000_000)
            }
            return times.sorted()[2]
        }
        let image = render(new)
        check(image.contains { $0 != 0 })
        let before = median { _ = render(old) }
        let after = median { check(render(new) == image) }
        check(new.rebuilds == 1, "unchanged/scroll redraw rebuilt the document")
        view.string = "\\section{새 문서}\n한글 👩🏽‍💻\n"
        // AppKit can deliver layout/frame notifications after the edit,
        // restarting the debounce. Await the actual rebuild, not one fixed delay.
        var editedImage = image
        for _ in 0..<60 {
            try await Task.sleep(for:.milliseconds(50))
            editedImage = render(new)
            if new.rebuilds > 1 { break }
        }
        check(editedImage != image, "edit did not invalidate the minimap")
        check(new.rebuilds == 2)
        print("{\"operation\":\"minimap redraw\",\"before_ms\":\(before),\"after_ms\":\(after),\"speedup\":\(before/after)}")
        print("PASS minimap cache/edit invalidation and Unicode completion parity")
    }
}

"""#

do {
    var path = "Mac/Sources/Features/EditorContainerView.swift"
    var current = try slice(read(path), from: "final class MinimapOverlayView:", to: "/// Ghost-text layer")
    current = current.replacingOccurrences(of: "final class MinimapOverlayView: NSView {", with: "final class MinimapOverlayView: NSView {\n    var rebuilds = 0")
    current = current.replacingOccurrences(of: "    private func drawDocument() {", with: "    private func drawDocument() {\n        rebuilds += 1")
    var old = try slice(read("Fixtures/baselines/v1.5.0/\(path).txt"), from: "final class MinimapOverlayView:", to: "/// Ghost-text layer")
    old = old.replacingOccurrences(of: "MinimapOverlayView", with: "BaselineMinimapOverlayView")
    path = "Packages/TexCore/Sources/LanguageCore/CompletionContext.swift"
    let completion = try read(path).components(separatedBy: "extension LanguageIndex")[0]
    var prior = try slice(read("Fixtures/baselines/v1.5.0/\(path).txt"), from: "public enum CompletionContextDetector", to: "extension LanguageIndex")
    prior = prior.replacingOccurrences(of: "CompletionContextDetector", with: "BaselineCompletionContextDetector")
    try temporary("pitex-editor-check-") { root in
        let source = root.appendingPathComponent("Check.swift")
        let executable = root.appendingPathComponent("check")
        try write(current + old + completion + prior + host, to: source)
        try run(["swiftc", "-O", "-parse-as-library", source.path, "-o", executable.path])
        try run([executable.path, repoPath + "/Fixtures/projects/large/main.tex"], timeout: 120)
    }
} catch {
    awaitInterruption()
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(1)
}
