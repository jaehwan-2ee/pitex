#!/usr/bin/env swift
// Render the real macOS project-tree view and exercise file/folder clicks.
// Uses Command Line Tools, AppKit and Vision; no Xcode build or user project.
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

@main struct ProjectTreeCheck {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let tree = buildProjectFileTree(relativePaths: [
            "manuscript.tex", "manuscript.bib", "ch1.tex", "ch2.tex", "notes.md",
            "figures/a.pdf", "figures/b.pdf", "other_tex/ch3.tex", "other_tex/ch4.tex",
        ])
        var selected = ""
        let host = NSHostingView(rootView: ProjectTreeRows(nodes: tree) { node in
            Button { selected = node.path } label: {
                HStack(spacing: 7) {
                    Image(systemName: node.name.hasSuffix(".bib") ? "book" : "doc.text")
                    Text(verbatim: node.name)
                    Spacer()
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }.buttonStyle(.plain)
        }.padding(8).frame(width: 340, height: 440, alignment: .topLeading)
            .background(Color(nsColor: .windowBackgroundColor)))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 340, height: 440),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        window.makeKeyAndOrderFront(nil)
        defer { window.orderOut(nil) }
        let names = ["manuscript.tex", "manuscript.bib", "ch1.tex", "ch2.tex",
                     "notes.md", "figures", "a.pdf", "b.pdf", "other_tex", "ch3.tex", "ch4.tex"]
        func snapshot(_ name: String) async throws -> [String: CGRect] {
            try await Task.sleep(for: .milliseconds(300))
            host.layoutSubtreeIfNeeded()
            let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
            host.cacheDisplay(in: host.bounds, to: bitmap)
            try bitmap.representation(using: .png, properties: [:])!.write(
                to: URL(fileURLWithPath: "/tmp/pitex-project-tree-\(name).png"))
            let request = VNRecognizeTextRequest()
            request.recognitionLevel = .accurate
            request.recognitionLanguages = ["en-US"]
            try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
            var rows: [String: CGRect] = [:]
            for observation in request.results ?? [] {
                guard let candidate = observation.topCandidates(1).first else { continue }
                for name in names {
                    if let range = candidate.string.range(of: name),
                       let box = try candidate.boundingBox(for: range) {
                        rows[name] = box.boundingBox
                    }
                }
            }
            return rows
        }
        func click(_ rect: CGRect, disclosure: Bool = false) {
            let local = NSPoint(x: disclosure ? 12 : host.bounds.width * rect.midX,
                y: host.bounds.height * (host.isFlipped ? 1 - rect.midY : rect.midY))
            let point = host.convert(local, to: nil)
            for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
                window.sendEvent(NSEvent.mouseEvent(with: type, location: point, modifierFlags: [],
                    timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                    context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!)
            }
        }
        let sourceNames = ["ch1.tex", "ch2.tex", "manuscript.bib", "manuscript.tex", "notes.md"]
        func checkSources(_ rows: [String: CGRect]) {
            precondition(sourceNames.allSatisfy { rows[$0] != nil }, "Hidden source rows: \(rows)")
            for (parent, child) in zip(sourceNames, sourceNames.dropFirst()) {
                precondition(rows[parent]!.midY > rows[child]!.midY, "Source order changed")
            }
            for child in sourceNames.dropFirst() {
                precondition(abs(rows[child]!.minX - rows["ch1.tex"]!.minX) * 340 < 6,
                             "Sibling files must stay at the same depth: \(rows)")
            }
        }
        let closed = try await snapshot("closed")
        checkSources(closed)
        precondition(closed["a.pdf"] == nil && closed["ch3.tex"] == nil)
        for file in ["manuscript.tex", "manuscript.bib", "notes.md"] {
            click(closed[file]!)
            precondition(selected == file, "File row did not activate")
            let after = try await snapshot(file)
            checkSources(after)
            precondition(after == closed, "Opening a file changed the tree layout")
        }
        click(closed["figures"]!, disclosure: true)
        let figures = try await snapshot("figures")
        checkSources(figures)
        precondition(figures["a.pdf"] != nil && figures["b.pdf"] != nil && figures["ch3.tex"] == nil)
        precondition((figures["a.pdf"]!.minX - figures["figures"]!.minX) * 340 >= 16,
                     "Folder files must be indented: \(figures)")
        precondition((figures["a.pdf"]!.minX - figures["ch1.tex"]!.minX) * 340 >= 20,
                     "PDF must stay inside its folder")
        click(figures["other_tex"]!, disclosure: true)
        let expanded = try await snapshot("expanded")
        checkSources(expanded)
        precondition(expanded["ch3.tex"] != nil && expanded["ch4.tex"] != nil)
        click(expanded["figures"]!, disclosure: true)
        let collapsed = try await snapshot("collapsed")
        checkSources(collapsed)
        precondition(collapsed["a.pdf"] == nil && collapsed["b.pdf"] == nil)
        precondition(collapsed["ch3.tex"] != nil, "Collapsing one folder changed another")
        click(collapsed["figures"]!)
        let byName = try await snapshot("byName")
        precondition(byName["a.pdf"] != nil, "Clicking a folder name must expand it")
        print("PASS: stable folder hierarchy across TeX/Bib/Markdown activation; PDF stays in its folder; independent folder toggles and indentation")
    }
}

"""#####

do {
    let modelSource = try String(contentsOf: repository.appendingPathComponent("Packages/TexApp/Sources/ProjectFeature/ProjectFeature.swift"), encoding: .utf8).replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
    let viewSource = try String(contentsOf: repository.appendingPathComponent("Mac/Sources/Features/ProjectSidebarView.swift"), encoding: .utf8).replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
    guard let modelStart = modelSource.range(of: "public struct ProjectFileNode:"),
          let viewStart = viewSource.range(of: "private struct ProjectTreeRows") else { try fail("Project tree declarations are missing") }
    let model = "public struct ProjectFileNode:" + modelSource[modelStart.upperBound...]
    let view = "private struct ProjectTreeRows" + viewSource[viewStart.upperBound...]
    let root = try temporaryDirectory(prefix: "pitex-project-tree-")
    defer { try? files.removeItem(at: root) }
    let source = root.appendingPathComponent("Check.swift")
    try ("import AppKit\nimport SwiftUI\nimport Vision\n" + model + "\n" + view + check).write(to: source, atomically: false, encoding: .utf8)
    let program = root.appendingPathComponent("check")
    try run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", source.path, "-o", program.path])
    try run([program.path], timeout: 90)
} catch {
    interruption.finishCancellation()
    fputs("\(error)\n", stderr)
    exit(1)
}
