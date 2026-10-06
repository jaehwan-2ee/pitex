#!/usr/bin/env swift
// After a Release build: check-editor-chrome.swift <Build/Products/Release>
//
// Hosts the production SwiftUI/AppKit editor. Checks clipping and clicks the
// rendered minimap after wrapping, resizing, font changes, folding and tab changes.
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
import EditorMacAdapter
import SwiftUI

private actor Doc: DocumentSessionPort {
    let text: String
    init(_ text: String) { self.text = text }
    func snapshot() async -> DocumentSnapshot { .init(revision: 0, text: text) }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult { .rejected(current: await snapshot()) }
}
private final class Backdrop: NSView {
    override var isFlipped: Bool { true }
    override func draw(_ dirtyRect: NSRect) { NSColor.red.setFill(); bounds.fill() }
}
@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        func content(_ adapter: EditorMacAdapter) -> some View {
            EditorContainerView(adapter: adapter).id(ObjectIdentifier(adapter))
        }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        func bitmap(_ view: NSView) -> NSBitmapImageRep {
            let result = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
            view.cacheDisplay(in: view.bounds, to: result)
            return result
        }
        let source = "\\begin{itemize}\n" + String(repeating: "\\item " + String(repeating: "wrapped text ", count: 20) + "\n", count: 80)
            + "\\end{itemize}\n\n" + String(repeating: "LAST_TARGET\n", count: 32)
        let first = try await EditorMacAdapter.make(session: Doc(source))
        let host = NSHostingView(rootView: content(first))
        let root = Backdrop(frame: NSRect(x: 0, y: 0, width: 600, height: 360))
        root.addSubview(host)
        let window = NSWindow(contentRect: root.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = root
        defer { window.orderOut(nil) }
        window.orderFront(nil)

        func settle() async throws {
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(600))
            root.displayIfNeeded()
        }
        func verify(_ adapter: EditorMacAdapter, name: String) async throws {
            try await settle()
            let views = descendants(host)
            let scroll = views.compactMap { $0 as? NSScrollView }.first!
            let map = views.compactMap { $0 as? MinimapOverlayView }.first!
            let gutter = views.compactMap { $0 as? LineNumberGutterView }.first!
            let chips = views.compactMap { $0 as? FoldChipOverlayView }.first!
            let text = adapter.textView
            precondition(scroll.documentView === text, "Tab must own all its editor chrome")
            precondition(scroll.clipsToBounds && gutter.clipsToBounds && chips.clipsToBounds && map.clipsToBounds)
            precondition(scroll.contentView.frame.contains(gutter.frame) && scroll.contentView.frame.contains(map.frame))
            let targetStart = (text.string as NSString).range(of: "LAST_TARGET").location
            let target = NSRange(location: targetStart, length: (text.string as NSString).length - targetStart)
            text.textStorage!.addAttribute(.foregroundColor, value: NSColor.green, range: target)
            let layout = text.layoutManager!, container = text.textContainer!
            layout.ensureLayout(for: container)
            text.scroll(NSPoint(x: 0, y: 17.5)) // partial row at the top and bottom
            try await settle()
            let image = bitmap(map)
            var green: [NSPoint] = []
            for y in 0..<image.pixelsHigh {
                for x in 0..<image.pixelsWide {
                    let color = image.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
                    if color.greenComponent - color.redComponent > 0.1 && color.greenComponent - color.blueComponent > 0.1 {
                        green.append(NSPoint(x: CGFloat(x) + 0.5, y: CGFloat(y) + 0.5))
                    }
                }
            }
            precondition(!green.isEmpty, "Minimap must render the current document's final target")
            let centre = green.reduce(NSPoint.zero) { NSPoint(x: $0.x + $1.x, y: $0.y + $1.y) }
            let local = NSPoint(x: centre.x / CGFloat(green.count) * map.bounds.width / CGFloat(image.pixelsWide),
                                y: centre.y / CGFloat(green.count) * map.bounds.height / CGFloat(image.pixelsHigh))
            let event = NSEvent.mouseEvent(with: .leftMouseDown, location: map.convert(local, to: nil), modifierFlags: [],
                timestamp: 0, windowNumber: window.windowNumber, context: nil, eventNumber: 1, clickCount: 1, pressure: 1)!
            map.mouseDown(with: event)
            let glyphs = layout.glyphRange(forCharacterRange: target, actualCharacterRange: nil)
            let rect = layout.boundingRect(forGlyphRange: glyphs, in: container)
                .offsetBy(dx: text.textContainerOrigin.x, dy: text.textContainerOrigin.y)
            precondition(text.visibleRect.intersects(rect), "Clicking the painted minimap target must reveal that exact text")

            // Pixel check: scrolling must not paint line numbers into the
            // header/footer outside the editor's AppKit hosting view.
            let all = bitmap(root)
            let backdrop = all.colorAt(x: 0, y: 0)!.usingColorSpace(.deviceRGB)!
            for y in [35, Int(root.bounds.height) - 35] {
                for x in 5..<55 {
                    let color = all.colorAt(x: x * all.pixelsWide / Int(root.bounds.width),
                        y: y * all.pixelsHigh / Int(root.bounds.height))!.usingColorSpace(.deviceRGB)!
                    precondition(abs(color.redComponent - backdrop.redComponent) < 0.01 && abs(color.greenComponent - backdrop.greenComponent) < 0.01 && abs(color.blueComponent - backdrop.blueComponent) < 0.01,
                                 "Editor chrome painted outside its viewport at \(x),\(y): \(color)")
                }
            }
            print("PASS \(name): clipped chrome, final minimap target and click alignment"); fflush(nil)
        }

        for size in [NSSize(width: 600, height: 360), NSSize(width: 350, height: 220)] {
            root.setFrameSize(size)
            host.frame = NSRect(x: 0, y: 40, width: size.width, height: size.height - 80)
            try await verify(first, name: "resize \(size)")
        }
        first.textView.font = .monospacedSystemFont(ofSize: 24, weight: .regular)
        try await verify(first, name: "larger font")
        let gutter = descendants(host).compactMap { $0 as? LineNumberGutterView }.first!
        gutter.foldEngine!.toggle(atLine: 0)
        try await verify(first, name: "folded environment")

        let second = try await EditorMacAdapter.make(session: Doc(String(repeating: "short line\n", count: 35) + "LAST_TARGET\n"))
        host.rootView = content(second)
        try await verify(second, name: "new document, compact minimap scale")
    }
}

"""##
let root = temporary("pitex-editor-chrome-")
defer { try? files.removeItem(at: root) }
let source = root.appendingPathComponent("Check.swift")
write(check, to: source)
let app = repo.appendingPathComponent("Mac/Sources/AppShell/PitexApp.swift")
let stripped = root.appendingPathComponent("PitexApp.swift")
write(read(app).replacingOccurrences(of: "@main\nstruct PitexApp", with: "struct PitexApp"), to: stripped)
func swiftSources(_ directory: URL) -> [URL] {
    let entries = children(directory)
    var result = entries.filter { $0.pathExtension == "swift" && $0 != app }
    for entry in entries {
        if let values = try? entry.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey]), values.isDirectory == true, values.isSymbolicLink != true { result += swiftSources(entry) }
    }
    return result
}
let executable = root.appendingPathComponent("check")
run(["xcrun", "swiftc", "-g", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0",
     "-I", products.path, source.path, stripped.path] + swiftSources(repo.appendingPathComponent("Mac/Sources")).map(\.path)
     + rawChildren(products.path).filter { $0.hasSuffix(".o") } + ["-o", executable.path])
run([executable.path], timeout: 45)
