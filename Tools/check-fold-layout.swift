#!/usr/bin/env swift
// Differential fold-layout check: production FoldEngine vs v1.8.1 (macOS).
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
func check(_ ok: Bool, _ message: String, line: Int = #line) {
    if !ok {
        FileHandle.standardError.write(Data("line \(line): \(message)\n".utf8))
        fatalError(message)
    }
}
@main struct FoldLayoutCheck {
    @MainActor static func main() throws {
        _ = NSApplication.shared
        // A 60k-character excerpt of the large fixture: dozens of sections
        // and verbatim environments to fold, while the per-glyph compare of
        // every step (two passes) stays well inside the CI timeout.
        let source = String(try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8).prefix(60_000))
        func makeView(_ text: String) -> NSTextView {
            let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 800, height: 600))
            let view = NSTextView(frame: scroll.bounds)
            view.isVerticallyResizable = true
            view.textContainer!.containerSize = NSSize(width: 800, height: CGFloat.greatestFiniteMagnitude)
            view.textContainer!.widthTracksTextView = true
            view.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
            view.string = text
            // Deterministic comparison: no idle-time layout or glyph work
            // may touch one view and not the other during the runloop pumps.
            view.layoutManager!.backgroundLayoutEnabled = false
            scroll.documentView = view
            return view
        }
        let before = makeView(source)
        let after = makeView(source)
        let old = BaselineFoldEngine()
        let new = FoldEngine()
        old.attach(to: before)
        new.attach(to: after)
        func compare(_ label: String) {
            check(before.string == after.string, "\(label): texts diverged")
            let la = before.layoutManager!, lb = after.layoutManager!
            la.ensureLayout(for: before.textContainer!)
            lb.ensureLayout(for: after.textContainer!)
            check(la.numberOfGlyphs == lb.numberOfGlyphs, "\(label): glyph count differs")
            for glyph in 0..<la.numberOfGlyphs {
                check(la.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
                      == lb.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil),
                      "\(label): line fragment rect differs at glyph \(glyph)")
                check(la.notShownAttribute(forGlyphAt: glyph)
                      == lb.notShownAttribute(forGlyphAt: glyph),
                      "\(label): not-shown flag differs at glyph \(glyph)")
            }
            let lines = FoldEngine.computeLineStarts(before.string as NSString).count
            for line in 0..<lines {
                check(old.isLineHidden(line) == new.isLineHidden(line),
                      "\(label): hidden state differs at line \(line)")
            }
        }
        // When set, lay out both views between the edit and recompute — the
        // real app lays out during the 300 ms debounce, with the stale
        // delegate state the incremental invalidation then has to repair.
        var layOutBetweenEdits = false
        func edit(_ location: Int, _ length: Int, _ insert: String, _ label: String) {
            let range = NSRange(location: location, length: length)
            before.textStorage!.replaceCharacters(in: range, with: insert)
            after.textStorage!.replaceCharacters(in: range, with: insert)
            if layOutBetweenEdits {
                before.layoutManager!.ensureLayout(for: before.textContainer!)
                after.layoutManager!.ensureLayout(for: after.textContainer!)
            }
            // Flush queued didProcessEditing observers before recomputing.
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
            old.recompute()
            new.recompute()
            compare(label)
        }
        check(old.regions.map(\.headerLine) == new.regions.map(\.headerLine),
              "regions differ before folding")
        check(!old.regions.isEmpty, "fixture has no foldable regions")
        var headers: [Int] = []
        if let region = old.regions.first(where: { $0.signature.hasPrefix("begin:") }) {
            headers.append(region.headerLine)
        }
        if let region = old.regions.first(where: { $0.signature.hasPrefix("section:") }) {
            headers.append(region.headerLine)
        }
        headers.append(old.regions[old.regions.count / 2].headerLine)
        headers.append(old.regions[old.regions.count - 1].headerLine)
        for line in Set(headers).sorted() {
            old.toggle(atLine: line)
            new.toggle(atLine: line)
        }
        compare("initial folds")
        func length() -> Int { (before.string as NSString).length }
        // First hidden line span of the earliest folded region, in UTF-16
        // offsets — the same math applyFolds uses for hiddenCharRanges.
        func hiddenSpan() -> (Int, Int)? {
            guard let region = new.regions.first(where: { $0.folded }) else { return nil }
            let first = new.lineStart(for: region.hiddenLineRange.lowerBound)
            var end = new.lineStart(for: region.hiddenLineRange.upperBound + 1)
            if end < 0 { end = length() }
            guard first >= 0, end > first else { return nil }
            return (first, end)
        }
        // UTF-16 offset just past the last folded region's hidden lines.
        func afterLastFold() -> Int {
            var tail = 0
            for region in new.regions where region.folded {
                let end = new.lineStart(for: region.hiddenLineRange.upperBound + 1)
                tail = max(tail, end < 0 ? length() : end)
            }
            return tail
        }
        func scenario(_ pass: Int) {
            layOutBetweenEdits = pass == 2
            edit(5, 0, "xyz", "pass \(pass): insert before first fold")
            edit(7, 1, "", "pass \(pass): delete before first fold")
            if let (first, end) = hiddenSpan() {
                let mid = first + (end - first) / 2
                edit(mid, 1, "q", "pass \(pass): same-length replace inside hidden range")
                edit(mid, 1, "q q", "pass \(pass): different-length replace inside hidden range")
                edit(mid, 2, "", "pass \(pass): delete inside hidden range")
            }
            if let header = headers.first {
                let start = new.lineStart(for: header)
                if start >= 0 {
                    edit(start + 1, 0, "X", "pass \(pass): edit on folded header line")
                }
            }
            edit(min(afterLastFold() + 40, length()), 0, " tail", "pass \(pass): edit after last folded region")
            edit(5, 0, "\n", "pass \(pass): newline insert before first region")
            edit(5, 1, "", "pass \(pass): newline delete before first region")
            edit(min(20, length()), 0, "한글 👩🏽‍💻", "pass \(pass): Korean and emoji insert")
        }
        // Pass 2 lays out between each edit and recompute, so the partial
        // invalidation must repair layout produced under the stale delegate.
        scenario(1)
        scenario(2)
        func median(_ action: () -> Void) -> Double {
            var times: [Double] = []
            for _ in 0..<5 {
                let start = DispatchTime.now().uptimeNanoseconds
                action()
                times.append(Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000)
            }
            return times.sorted()[2]
        }
        // Edit deep in the document so the incremental path skips most of
        // the layout work; both engines take the same edits and stay in sync.
        let spot = length() * 3 / 4
        let baseline = median {
            before.textStorage!.replaceCharacters(in: NSRange(location: spot, length: 0), with: "x")
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
            old.recompute()
        }
        let optimized = median {
            after.textStorage!.replaceCharacters(in: NSRange(location: spot, length: 0), with: "x")
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
            new.recompute()
        }
        compare("after timing")
        print("{\"operation\":\"macOS fold recompute after edits\",\"before_ms\":\(baseline),\"after_ms\":\(optimized),\"speedup\":\(baseline / optimized)}")
        print("PASS fold layout: identical text, hidden lines, fragment rects and not-shown flags")
    }
}

"""#

do {
    let folding = "Mac/Sources/Features/EditorFolding.swift"
    let highlighting = "Mac/Sources/Features/SyntaxHighlighting.swift"
    let current = try slice(read(folding), from: "struct FoldRegion", to: "/// Paints the reference editor")
    let prior = try slice(read("Fixtures/baselines/v1.8.1/\(folding).txt"), from: "struct FoldRegion", to: "/// Paints the reference editor")
        .replacingOccurrences(of: "FoldRegion", with: "BaselineFoldRegion")
        .replacingOccurrences(of: "FoldEngine", with: "BaselineFoldEngine")
    let analysis = try slice(read(highlighting), from: "/// A text storage owns", to: "/// Applies deterministic")
    let core = try read("Packages/TexCore/Sources/LanguageCore/LanguageCore.swift")
    try temporary("pitex-fold-") { root in
        let source = root.appendingPathComponent("Check.swift")
        let executable = root.appendingPathComponent("check")
        try write(stripImports(core) + analysis + current + prior + host, to: source)
        try run(["swiftc", "-swift-version", "6", "-O", "-parse-as-library", source.path, "-o", executable.path])
        try run([executable.path, repoPath + "/Fixtures/projects/large/main.tex"], timeout: 180)
    }
} catch {
    awaitInterruption()
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(1)
}
