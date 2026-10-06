#!/usr/bin/env swift
// Check the actual macOS model's manual SSH save boundary.
//
// Usage: Tools/check-remote-autosave.swift <ssh-host>
// PITEX_PREVIEW_HELPER must point to a built embedded helper. The check uses
// only a disposable remote folder, mirror, and isolated application defaults.
// It verifies that SSH Auto Save and local preview never upload, explicit
// saves upload and build, and close preserves changes or refuses failure.
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
let usage = """
Check the actual macOS model's manual SSH save boundary.

Usage: Tools/check-remote-autosave.swift <ssh-host>
PITEX_PREVIEW_HELPER must point to a built embedded helper. The check uses
only a disposable remote folder, mirror, and isolated application defaults.
It verifies that SSH Auto Save and local preview never upload, explicit
saves upload and build, and close preserves changes or refuses failure.
"""
guard CommandLine.arguments.count == 2 else { fail(usage) }
let host = CommandLine.arguments[1]
guard let helperPath = ProcessInfo.processInfo.environment["PITEX_PREVIEW_HELPER"] else { fail("PITEX_PREVIEW_HELPER is required") }
let helper = resolved(helperPath)
guard (try? helper.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true,
      files.isExecutableFile(atPath: helper.path) else { fail(helper.path) }
let sdk = run(["xcrun", "--sdk", "macosx", "--show-sdk-path"], capture: true).trimmingCharacters(in: .whitespacesAndNewlines)
let swift = run(["xcrun", "--sdk", "macosx", "--find", "swift"], capture: true).trimmingCharacters(in: .whitespacesAndNewlines)
let swiftc = run(["xcrun", "--sdk", "macosx", "--find", "swiftc"], capture: true).trimmingCharacters(in: .whitespacesAndNewlines)
let cache = temporary("pitex-remote-module-cache-", in: NSTemporaryDirectory(), cleanup: false)
var environment = ProcessInfo.processInfo.environment
environment["SDKROOT"] = sdk
environment["CLANG_MODULE_CACHE_PATH"] = cache.path
environment["SWIFTPM_MODULECACHE_OVERRIDE"] = cache.path
let check = ##"""

import AppKit
import Combine
import CoreGraphics
import PDFKit
import RemoteCore

@main struct Check {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        setbuf(stdout, nil)
        print("START SSH native check"); fflush(nil)
        Task { @MainActor in
            print("START SSH fixture"); fflush(nil)
            do { try await run(); print("PASS SSH manual-save boundary"); exit(0) }
            catch { print("FAIL: \(error)"); fflush(nil); exit(1) }
        }
        app.run()
    }

    @MainActor static func run() async throws {
        func require(_ condition: Bool, _ message: String) throws {
            if !condition { throw NSError(domain: "RemoteManualSave", code: 1,
                userInfo: [NSLocalizedDescriptionKey: message]) }
        }
        let client = SSHClient(connection: SSHConnection(
            name: "check", destination: CommandLine.arguments[1],
            port: ProcessInfo.processInfo.environment["PITEX_CHECK_SSH_PORT"].flatMap(Int.init),
            identityFile: ProcessInfo.processInfo.environment["PITEX_CHECK_SSH_IDENTITY_FILE"]))
        let created = try await client.runChecked("mktemp -d /tmp/pitex-manual-save.XXXXXXXX")
        let root = created.stdoutText.trimmingCharacters(in: .whitespacesAndNewlines)
        try require(root.hasPrefix("/tmp/pitex-manual-save.") && (root as NSString).deletingLastPathComponent == "/tmp", "Unsafe fixture path")
        let mirror = try RemoteMirror.prepare(for: RemoteProject(connection: client.connection, remoteRoot: root))
        let workspace = WorkspaceModel()
        final class NativeWindowDelegate: NSObject, NSWindowDelegate {}
        let nativeDelegate = NativeWindowDelegate()
        let nativeWindow = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 980, height: 620),
                                    styleMask: [.titled, .closable], backing: .buffered, defer: false)
        nativeWindow.isReleasedWhenClosed = false
        nativeWindow.delegate = nativeDelegate
        workspace.window = nativeWindow
        defer { nativeWindow.orderOut(nil) }
        let tex: (String) -> String = { marker in
            "\\documentclass{article}\n\\begin{document}\n\(marker)\n\\end{document}\n"
        }
        func remote(_ file: String = "main.tex") async throws -> String {
            try await client.runChecked("cat -- \"$1/$2\"", arguments: [root, file]).stdoutText
        }
        func edit(_ text: String) async throws {
            let view = workspace.environment!.editor.textView
            let range = NSRange(location: 0, length: view.string.utf16.count)
            try require(view.shouldChangeText(in: range, replacementString: text), "Editor refused edit")
            view.replaceCharacters(in: range, with: text)
            view.didChangeText()
            try await until("Editor did not publish the edit") { workspace.documentSnapshot?.text == text }
        }
        func until(_ message: String, _ done: () -> Bool) async throws {
            let deadline = ContinuousClock.now + .seconds(60)
            while !done() {
                try require(.now < deadline, message)
                try await Task.sleep(for: .milliseconds(25))
            }
        }
        func finalBuild(_ marker: String) async throws {
            let deadline = ContinuousClock.now + .seconds(60)
            while (try? await remote("saved-by-build.txt")) != tex(marker) {
                if case .failed(let reason) = workspace.buildState {
                    try require(false, "Remote build failed: \(reason)\n\(workspace.buildLogText)")
                }
                try require(.now < deadline, "Remote compiler did not consume the saved revision")
                try await Task.sleep(for: .milliseconds(25))
            }
            try await until("Remote final build did not complete") {
                if case .succeeded = workspace.buildState { return true }
                if case .failed = workspace.buildState { return true }
                return false
            }
            if case .failed(let reason) = workspace.buildState {
                try require(false, "Remote build failed: \(reason)\n\(workspace.buildLogText)")
            }
            try require(try await remote("saved-by-build.txt") == tex(marker), "Remote compiler used a draft instead of the saved revision")
        }
        func cleanup() async {
            workspace.settings.liveCompileEnabled = false
            _ = try? await client.runChecked("chmod 700 \"$1\"", arguments: [root])
            _ = await workspace.close()
            _ = try? await client.runChecked("rm -rf -- \"$1\"", arguments: [root])
            try? FileManager.default.removeItem(at: mirror.directory)
            UserDefaults.standard.removePersistentDomain(forName: Bundle.main.bundleIdentifier!)
        }
        do {
            try require(nativeWindow.delegate === nativeDelegate,
                        "Local windows must retain their native delegate before SSH opens")
            _ = try await client.runChecked("cat > \"$1/main.tex\"", arguments: [root], input: Data(tex("InitialMarker").utf8))
            _ = try await client.runChecked("git -C \"$1\" init -q", arguments: [root])
            let pdfData = NSMutableData()
            var bounds = CGRect(x: 0, y: 0, width: 200, height: 200)
            let consumer = CGDataConsumer(data: pdfData)!
            let context = CGContext(consumer: consumer, mediaBox: &bounds, nil)!
            context.beginPDFPage(nil); context.endPDFPage(); context.closePDF()
            _ = try await client.runChecked("cat > \"$1/fixture.pdf\"", arguments: [root], input: pdfData as Data)
            workspace.settings.aiAutocompletion = false
            workspace.settings.autoSave = true
            workspace.settings.autoSaveDelay = 1
            workspace.settings.liveCompileEnabled = true
            // SSH honors the selected local preview backend.
            workspace.settings.livePreviewBackend = "embedded"
            workspace.settings.updateBuild { try! $0.build.selectingShellExecution(.custom(command: "true")).acknowledgingCustomShell() }
            await workspace.open(mirror.root)
            try require(workspace.remote != nil && workspace.environment != nil, "SSH mirror did not open")
            try require(nativeWindow.delegate != nil && nativeWindow.delegate !== nativeDelegate,
                        "Opening SSH in an existing native window must install the save gate")
            try require(workspace.embeddedPreviewEnabled, "SSH local embedded preview is disabled")
            workspace.buildCommandText = "sleep 1; cat main.tex > saved-by-build.txt; cp fixture.pdf main.pdf"
            try await edit(tex("LocalPreviewMarker"))
            try await until("Local embedded preview did not show the unsaved SSH edit") {
                guard let data = workspace.retainedPDF?.data else { return false }
                return PDFDocument(data: data)?.string?.contains("LocalPreviewMarker") == true
            }
            try await Task.sleep(for: .seconds(3))
            try require(workspace.documentSnapshot?.saveState == .dirty, "SSH Auto Save persisted the draft")
            try require(try await remote() == tex("InitialMarker"), "Editing or live preview uploaded the draft")
            let noBuild = try await client.runChecked("test ! -e \"$1/saved-by-build.txt\"", arguments: [root])
            try require(noBuild.status == 0, "Editing started a remote compiler")
            print("PASS Auto Save disabled on SSH; unsaved local embedded preview; no remote build")

            workspace.buildCommandText = "mkdir -p {outdir}; cat {file} > {outdir}/local-input.txt; cp fixture.pdf {outdir}/{filename}.pdf"
            workspace.settings.livePreviewBackend = "compiler"
            try await edit(tex("LocalCompilerMarker"))
            try await until("SSH compiler preview did not build the local mirror") {
                workspace.retainedPDF?.artifactPath == ".pitex-live/main/main.pdf"
                    && (try? String(contentsOf: mirror.root.appendingPathComponent(".pitex-live/main/local-input.txt"), encoding: .utf8)) == tex("LocalCompilerMarker")
            }
            try require(!workspace.embeddedPreviewEnabled && workspace.embeddedPreview.session == nil,
                        "SSH compiler selection still forced the embedded engine")
            try require(try await remote() == tex("InitialMarker"), "Compiler preview uploaded local edits")
            let noRemoteCompile = try await client.runChecked("test ! -e \"$1/saved-by-build.txt\"", arguments: [root])
            try require(noRemoteCompile.status == 0, "Compiler preview executed remotely")
            print("PASS selected compiler preview builds on this Mac without remote upload or compilation")
            workspace.settings.livePreviewBackend = "embedded"
            workspace.buildCommandText = "sleep 1; cat main.tex > saved-by-build.txt; cp fixture.pdf main.pdf"

            workspace.bottomPanelVisible = true
            workspace.consoleSection = .git
            workspace.refreshGit()
            try await until("Git status did not settle") { !workspace.gitRefreshInFlight }
            workspace.stageAllGit()
            try await until("Git mutation did not settle") { !workspace.gitBusy }
            try require(try await remote() == tex("InitialMarker"), "Git uploaded the draft")
            let localFailure = await workspace.persistDirtySessions()
            try require(localFailure == nil && workspace.documentSnapshot?.saveState == .clean, "Local persistence failed")
            try require(try await remote() == tex("InitialMarker"), "Internal local persistence uploaded changes")
            try "Agent local edit\n".write(to: mirror.root.appendingPathComponent("agent-created.txt"), atomically: true, encoding: .utf8)
            await workspace.refreshAfterAgentActivity()
            try require(workspace.canSave, "Clean SSH document cannot explicitly save local mirror changes")
            await workspace.save()
            try await finalBuild("LocalCompilerMarker")
            try require(try await remote("agent-created.txt") == "Agent local edit\n", "Explicit Save missed the agent-created local file")
            print("PASS Git and internal persistence do not upload; clean-document Save uploads local mirror changes")

            try await edit(tex("SavedRevisionMarker"))
            await workspace.save()
            try await edit(tex("NewDraftMarker"))
            try await finalBuild("SavedRevisionMarker")
            try require(try await remote() == tex("SavedRevisionMarker"), "Draft typed during final compilation reached the remote")
            try require(workspace.documentSnapshot?.saveState == .dirty, "Final compilation saved a later draft")
            print("PASS final remote compiler uses saved revision while later draft remains local")

            workspace.togglePinnedBuildTarget()
            await workspace.createDocument()
            try await edit(tex("BackgroundMarker"))
            await workspace.activateDocument(mirror.root.appendingPathComponent("main.tex"))
            await workspace.saveAll()
            try await finalBuild("NewDraftMarker")
            try require(try await remote("untitled.tex") == tex("BackgroundMarker"), "Save All missed the inactive document")
            print("PASS Save All saves inactive sources and runs the remote build")

            // The compiler backend still uploads only explicit saves. A
            // later draft stays local, then resumes automatic preview after
            // the saved remote build releases the scheduler's manual slot.
            workspace.buildCommandText = "sleep 1; mkdir -p {outdir}; cat {file} > {outdir}/saved-by-build.txt; cp fixture.pdf {outdir}/{filename}.pdf"
            workspace.settings.livePreviewBackend = "compiler"
            try await edit(tex("CompilerSavedMarker"))
            await workspace.save()
            try await until("Compiler selection did not preserve the manual SSH build") {
                workspace.isBuilding && workspace.buildLogText.contains("Building on")
            }
            try await edit(tex("CompilerLaterDraft"))
            try await finalBuild("CompilerSavedMarker")
            try await until("Compiler preview did not resume for the later local draft") {
                (try? String(contentsOf: mirror.root.appendingPathComponent(".pitex-live/main/saved-by-build.txt"), encoding: .utf8)) == tex("CompilerLaterDraft")
            }
            try require(try await remote() == tex("CompilerSavedMarker"),
                        "Resumed local compiler preview uploaded the later draft")
            print("PASS explicit SSH build uses its saved revision; later compiler preview resumes locally")
            workspace.settings.liveCompileEnabled = false

            try await edit(tex("CloseMarker"))
            let closed = await workspace.close()
            try require(closed != nil && workspace.projectURL == nil, "Successful forced Save did not close")
            try require(nativeWindow.delegate === nativeDelegate,
                        "Closing SSH must restore the existing native window delegate")
            try require(try await remote() == tex("CloseMarker"), "Close discarded the final unsaved edit")
            print("PASS project close forces final save and remote upload")

            await workspace.open(mirror.root)
            try await edit(tex("RetryMarker"))
            _ = try await client.runChecked("chmod 500 \"$1\"", arguments: [root])
            let refused = await workspace.close()
            try require(refused == nil && workspace.projectURL == mirror.root && workspace.environment != nil,
                        "Failed upload discarded or closed the project")
            try require(try await remote() == tex("CloseMarker"), "Failed upload changed the remote source")
            let active = workspace.activeDocumentURL!
            await workspace.closeDocument(active)
            try require(workspace.openDocuments.contains(active) && workspace.environment != nil,
                        "Failed forced tab save discarded its session")
            let mayTerminate = await workspace.prepareForTermination()
            try require(!mayTerminate && workspace.projectURL == mirror.root,
                        "Failed forced quit save allowed termination")
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 980, height: 620),
                                  styleMask: [.titled, .closable], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            workspace.window = window
            window.makeKeyAndOrderFront(nil)
            window.performClose(nil)
            try await Task.sleep(for: .milliseconds(250))
            try await until("Failed native close did not settle") { !workspace.isClosing }
            try require(window.isVisible && workspace.projectURL == mirror.root,
                        "Native window closed despite failed remote save")
            print("PASS failed tab, native window and quit saves preserve the workspace")
            _ = try await client.runChecked("chmod 700 \"$1\"", arguments: [root])
            let mayQuitNow = await workspace.prepareForTermination()
            try require(mayQuitNow && workspace.projectURL == mirror.root,
                        "Successful quit preparation discarded a workspace before global approval")
            try require(try await remote() == tex("RetryMarker"), "Quit retry missed the locally clean saved edit")
            try await edit(tex("NativeCloseMarker"))
            window.performClose(nil)
            try await until("Native close did not finish after forced save") { workspace.projectURL == nil && !window.isVisible }
            try require(try await remote() == tex("NativeCloseMarker"), "Native window close lost the final edit")
            print("PASS successful quit preparation and native close force the final remote source")
            await cleanup()
        } catch {
            await cleanup()
            throw error
        }
    }
}

"""##
var products: [URL] = []
for package in ["TexApp", "TexCore"] {
    let path = repo.appendingPathComponent("Packages/" + package)
    run([swift, "build", "--package-path", path.path, "--build-system", "native"], environment: environment)
    let product = run([swift, "build", "--package-path", path.path, "--build-system", "native", "--show-bin-path"], environment: environment, capture: true).trimmingCharacters(in: .whitespacesAndNewlines)
    let productURL = URL(fileURLWithPath: product)
    pathPrefixes.insert((productURL.path, product), at: 0)
    products.append(productURL)
}
var objects: [(String, [String])] = []
for product in products {
    for directory in children(product).filter({ $0.lastPathComponent.hasSuffix(".build") }) {
        let name = directory.lastPathComponent
        if !name.contains("Test") && !name.contains("-tool") {
            let objectFiles = rawChildren(directory.path).filter { $0.hasSuffix(".swift.o") }
            if !objectFiles.isEmpty && !objects.contains(where: { $0.0 == name }) { objects.append((name, objectFiles)) }
        }
    }
}
func validate() {
    let directory = temporary("pitex-remote-check-", in: NSTemporaryDirectory())
    defer { try? files.removeItem(at: directory) }
    let source = directory.appendingPathComponent("Check.swift")
    write(check, to: source)
    let app = repo.appendingPathComponent("Mac/Sources/AppShell/PitexApp.swift")
    let stripped = directory.appendingPathComponent("PitexApp.swift")
    write(read(app).replacingOccurrences(of: "@main\nstruct PitexApp", with: "struct PitexApp"), to: stripped)
    let contents = directory.appendingPathComponent("Check.app/Contents")
    let executable = contents.appendingPathComponent("MacOS/check")
    let resources = contents.appendingPathComponent("Resources")
    do {
        try files.createDirectory(at: executable.deletingLastPathComponent(), withIntermediateDirectories: true)
        try files.createDirectory(at: resources, withIntermediateDirectories: false)
        for localization in children(repo.appendingPathComponent("Mac/Resources")).filter({ $0.lastPathComponent.hasSuffix(".lproj") }) {
            try files.copyItem(at: localization, to: resources.appendingPathComponent(localization.lastPathComponent))
        }
        let plist: [String: String] = ["CFBundleIdentifier": "dev.pitex.remote-save-check-\(getpid())", "CFBundleExecutable": "check",
                                       "CFBundlePackageType": "APPL", "CFBundleDevelopmentRegion": "en"]
        let data = try PropertyListSerialization.data(fromPropertyList: plist, format: .xml, options: 0)
        try data.write(to: contents.appendingPathComponent("Info.plist"))
    } catch { fail("\(contents.path): \(error)") }
    let sourceRoot = repo.appendingPathComponent("Mac/Sources")
    // Path.rglob visits the source tree by directory, followed by its children.
    var sources: [String] = []
    func appendSwiftSources(_ directory: URL) {
        let entries = children(directory)
        sources += entries.filter { $0.pathExtension == "swift" && $0 != app }.map(\.path)
        for entry in entries {
            if let values = try? entry.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey]), values.isDirectory == true, values.isSymbolicLink != true { appendSwiftSources(entry) }
        }
    }
    appendSwiftSources(sourceRoot)
    run([swiftc, "-sdk", sdk, "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0"]
        + products.flatMap { ["-I", $0.appendingPathComponent("Modules").path] }
        + [source.path, stripped.path] + sources + objects.flatMap { $0.1 } + ["-o", executable.path], environment: environment)
    run([executable.path, host, "-AppleLanguages", "(en)"], cwd: directory, environment: environment, timeout: 360)
}
validate()
try? files.removeItem(at: cache)
