#!/usr/bin/env swift
// Compare native NSTextStorage colors and timing against v1.4.2 (macOS).
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
@MainActor final class EditorMacAdapter {
    let textView = NSTextView()
    var onTextDidChange: (() -> Void)?
}
enum AppearanceColorRole { case bodyText, commands, comments, braces, environments, math, lineNumbers }
@MainActor final class AppearanceSettings {
    static let shared = AppearanceSettings()
    func color(for role: AppearanceColorRole) -> NSColor {
        switch role {
        case .bodyText: .black
        case .commands: .red
        case .comments: .green
        case .braces: .blue
        case .environments: .orange
        case .math: .purple
        case .lineNumbers: .gray
        }
    }
}
@main struct HighlightCheck {
    @MainActor static func main() throws {
        _ = NSApplication.shared
        let source = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
        let before = EditorMacAdapter(), after = EditorMacAdapter()
        before.textView.string = source
        after.textView.string = source
        for editor in [before, after] {
            editor.textView.textStorage!.addAttribute(.backgroundColor, value: NSColor.yellow, range: NSRange(location: 0, length: 10))
        }
        let old = BaselineSyntaxHighlighter(), new = SyntaxHighlighter()
        old.attach(to: before, fileExtension: "tex")
        new.attach(to: after, fileExtension: "tex")
        func checkColors() {
            precondition(before.textView.string == after.textView.string)
            let a = before.textView.textStorage!, b = after.textView.textStorage!
            a.enumerateAttributes(in: NSRange(location: 0, length: a.length)) { attrs, range, _ in
                b.enumerateAttributes(in: range) { other, _, _ in
                    precondition(NSDictionary(dictionary: attrs).isEqual(to: other), "Changed attributes at \(range)")
                }
            }
        }
        checkColors()
        func median(_ action: () -> Void) -> Double {
            var times: [Double] = []
            for _ in 0..<5 {
                let start = DispatchTime.now().uptimeNanoseconds
                action()
                times.append(Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000)
            }
            return times.sorted()[2]
        }
        let baseline = median {
            before.textView.textStorage!.replaceCharacters(in: NSRange(location: 0, length: 0), with: "x")
            old.highlightNow()
        }
        let optimized = median {
            after.textView.textStorage!.replaceCharacters(in: NSRange(location: 0, length: 0), with: "x")
            new.highlightNow()
        }
        checkColors()
        print("{\"operation\":\"macOS highlighting after one-character edits\",\"before_ms\":\(baseline),\"after_ms\":\(optimized),\"speedup\":\(baseline / optimized)}")
        before.textView.textStorage!.replaceCharacters(in: NSRange(location: 5, length: 0), with: "한글 👩🏽‍💻 ")
        after.textView.textStorage!.replaceCharacters(in: NSRange(location: 5, length: 0), with: "한글 👩🏽‍💻 ")
        old.highlightNow(); new.highlightNow(); checkColors()
        // Old colors must disappear when a command becomes plain text; new
        // math/comment/BibTeX colors and Unicode offsets must still match.
        for (text, ext) in [("한글 e\u{301} 👩🏽‍💻 \\section{Title} $x$ % note\n", "tex"),
                            ("plain words after deleting the commands\n", "tex"),
                            ("@article{한글, title={👩🏽‍💻 e\u{301}}, year={2026}}", "bib")] {
            before.textView.string = text; after.textView.string = text
            old.attach(to: before, fileExtension: ext); new.attach(to: after, fileExtension: ext)
            checkColors()
        }
        // Seeded random edits: identical inserts/deletes/replaces on both
        // views, rehighlighting after each batch, attribute state compared
        // over the whole document every round.
        struct SeededRNG: RandomNumberGenerator {
            var state: UInt64
            mutating func next() -> UInt64 {
                state &+= 0x9E3779B97F4A7C15
                var z = state
                z = (z ^ (z >> 30)) &* 0xBF58476D1CE4E5B9
                z = (z ^ (z >> 27)) &* 0x94D049BB133111EB
                return z ^ (z >> 31)
            }
        }
        var rng = SeededRNG(state: 0x5EED)
        func randomEdits(_ rounds: Int, _ snippets: [String]) {
            for _ in 0..<rounds {
                for _ in 0..<Int.random(in: 1...3, using: &rng) {
                    let length = before.textView.textStorage!.length
                    let location = length == 0 ? 0 : Int.random(in: 0...length, using: &rng)
                    let removed = Int.random(in: 0...min(length - location, 24), using: &rng)
                    let insert = snippets[Int.random(in: 0..<snippets.count, using: &rng)]
                    before.textView.textStorage!.replaceCharacters(in: NSRange(location: location, length: removed), with: insert)
                    after.textView.textStorage!.replaceCharacters(in: NSRange(location: location, length: removed), with: insert)
                }
                old.highlightNow(); new.highlightNow()
                checkColors()
            }
        }
        // The dialect loop above left both attached as .bibtex; rebind to LaTeX
        // so the long run exercises the LaTeX highlighting path.
        // A 60k-character excerpt (dozens of sections, verbatim blocks,
        // Unicode) keeps 220 full compare rounds inside the CI timeout —
        // the 2 MB fixture already covers timing and whole-document colors.
        let excerpt = String(source.prefix(60_000))
        before.textView.string = excerpt; after.textView.string = excerpt
        old.attach(to: before, fileExtension: "tex"); new.attach(to: after, fileExtension: "tex")
        checkColors()
        randomEdits(220, ["x", "한", "👩🏽‍💻", "e\u{301}", "\\cmd", "{", "}", "$", "% note\n", "\n", "\\section{T}\n"])
        // Shorter run in the BibTeX dialect on a BibTeX-ish document.
        let bib = String(repeating: "@article{한글, title={👩🏽‍💻 e\u{301}}, year={2026}}\n", count: 300)
        before.textView.string = bib; after.textView.string = bib
        old.attach(to: before, fileExtension: "bib"); new.attach(to: after, fileExtension: "bib")
        checkColors()
        randomEdits(60, ["@book{k,", "title={", "}", ",\n", "x", "한", "👩🏽‍💻", "\n"])
        print("PASS native TextKit: identical colors, Unicode offsets, preserved attributes and edit/rebind behavior")
    }
}

"""#

do {
    let corePath = "Packages/TexCore/Sources/LanguageCore/LanguageCore.swift"
    let core = try read(corePath)
    let oldCore = try read("Fixtures/baselines/v1.4.2/\(corePath).txt")
    let oldLexer = try slice(oldCore, from: "public enum DeterministicTeXLexer", to: "public enum OutlineKind")
        .replacingOccurrences(of: "DeterministicTeXLexer", with: "BaselineTeXLexer")
    let path = "Mac/Sources/Features/SyntaxHighlighting.swift"
    let current = try read(path)
    let prior = try read("Fixtures/baselines/v1.4.2/\(path).txt")
        .replacingOccurrences(of: "SyntaxHighlighter", with: "BaselineSyntaxHighlighter")
        .replacingOccurrences(of: "DeterministicTeXLexer", with: "BaselineTeXLexer")
    try temporary("pitex-highlighting-") { root in
        let source = root.appendingPathComponent("Check.swift")
        let executable = root.appendingPathComponent("check")
        try write(core + oldLexer + stripImports(current) + stripImports(prior) + host, to: source)
        try run(["swiftc", "-swift-version", "6", "-O", "-parse-as-library", source.path, "-o", executable.path])
        try run([executable.path, repoPath + "/Fixtures/projects/large/main.tex"], timeout: 120)
    }
} catch {
    awaitInterruption()
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(1)
}
