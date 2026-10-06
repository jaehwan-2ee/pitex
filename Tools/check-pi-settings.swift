#!/usr/bin/env swift
// macOS regression check: Tools/check-pi-settings.swift [path/to/pi]
// Uses an isolated agent home and fake credentials; never reads or logs real auth.
import Foundation
import Darwin

struct CheckFailure: Error, CustomStringConvertible {
    let description: String
}

func require(_ condition: Bool, _ message: String) throws {
    if !condition { throw CheckFailure(description: message) }
}

struct Interrupted: Error {}

final class InterruptState: @unchecked Sendable {
    static let shared = InterruptState()
    private let lock = NSLock()
    private var cancelled = false
    private var activeChild: pid_t?
    private let source: DispatchSourceSignal

    private init() {
        // A no-op handler keeps exec'd children on their default SIGINT disposition.
        signal(SIGINT) { _ in }
        source = DispatchSource.makeSignalSource(signal: SIGINT, queue: .global())
        source.setEventHandler { [weak self] in self?.cancel() }
        source.resume()
    }

    var isCancelled: Bool {
        lock.lock()
        defer { lock.unlock() }
        return cancelled
    }

    func check() throws {
        if isCancelled { throw Interrupted() }
    }

    func begin(_ child: pid_t) {
        lock.lock()
        activeChild = child
        let stopping = cancelled
        lock.unlock()
        if stopping { stop(child) }
    }

    func end(_ child: pid_t) {
        lock.lock()
        defer { lock.unlock() }
        if activeChild == child {
            if cancelled { kill(-child, SIGKILL) }
            activeChild = nil
        }
    }

    private func cancel() {
        lock.lock()
        cancelled = true
        let child = activeChild
        lock.unlock()
        if let child { stop(child) }
    }

    private func stop(_ child: pid_t) {
        kill(-child, SIGTERM)
        DispatchQueue.global().asyncAfter(deadline: .now() + 3) { [weak self] in
            guard let self else { return }
            self.lock.lock()
            defer { self.lock.unlock() }
            if self.activeChild == child { kill(-child, SIGKILL) }
        }
    }
}

func run(_ arguments: [String], environment: [String: String]? = nil) throws {
    let child = try spawnChild(arguments, environment: environment ?? ProcessInfo.processInfo.environment)
    defer { InterruptState.shared.end(child) }
    var status: Int32 = 0
    while waitpid(child, &status, 0) < 0 {
        if errno == EINTR { continue }
        throw CheckFailure(description: "Wait for command: \(String(cString: strerror(errno)))")
    }
    try InterruptState.shared.check()
    let exitStatus = status & 0x7f == 0 ? (status >> 8) & 0xff : -(status & 0x7f)
    try require(exitStatus == 0, "Command failed with status \(exitStatus): \(arguments[0])")
}

func posixCheck(_ status: Int32, _ operation: String) throws {
    if status != 0 {
        throw CheckFailure(description: "\(operation): \(String(cString: strerror(status)))")
    }
}

// Resolve the modern entry point when available; older SDKs and systems use _np.
func addWorkingDirectory(_ directory: String, actions: inout posix_spawn_file_actions_t?) throws {
    typealias AddChdir = @convention(c) (UnsafeMutablePointer<posix_spawn_file_actions_t?>?, UnsafePointer<CChar>?) -> Int32
    let symbols = UnsafeMutableRawPointer(bitPattern: -2) // RTLD_DEFAULT
    guard let symbol = dlsym(symbols, "posix_spawn_file_actions_addchdir") ??
            dlsym(symbols, "posix_spawn_file_actions_addchdir_np") else {
        throw CheckFailure(description: "Set logout folder: function unavailable")
    }
    let function = unsafeBitCast(symbol, to: AddChdir.self)
    try posixCheck(directory.withCString { function(&actions, $0) }, "Set logout folder")
}

func spawnChild(_ command: [String], environment: [String: String], root: String? = nil,
                pty: (master: Int32, slave: Int32)? = nil) throws -> pid_t {
    try InterruptState.shared.check()
    var actions: posix_spawn_file_actions_t? = nil
    var attributes: posix_spawnattr_t? = nil
    try posixCheck(posix_spawn_file_actions_init(&actions), "Initialize spawn actions")
    defer { posix_spawn_file_actions_destroy(&actions) }
    try posixCheck(posix_spawnattr_init(&attributes), "Initialize spawn attributes")
    defer { posix_spawnattr_destroy(&attributes) }
    var defaults = sigset_t()
    sigemptyset(&defaults)
    sigaddset(&defaults, SIGINT)
    sigaddset(&defaults, SIGPIPE)
    try posixCheck(posix_spawnattr_setsigdefault(&attributes, &defaults), "Set child signals")
    try posixCheck(posix_spawnattr_setflags(&attributes,
        Int16(POSIX_SPAWN_SETSID | POSIX_SPAWN_CLOEXEC_DEFAULT | POSIX_SPAWN_SETSIGDEF)), "Set spawn flags")
    if let root { try addWorkingDirectory(root, actions: &actions) }
    if let pty {
        for descriptor: Int32 in [0, 1, 2] {
            try posixCheck(posix_spawn_file_actions_adddup2(&actions, pty.slave, descriptor), "Connect PTY")
        }
        try posixCheck(posix_spawn_file_actions_addclose(&actions, pty.master), "Close master in child")
        try posixCheck(posix_spawn_file_actions_addclose(&actions, pty.slave), "Close slave in child")
    } else {
        for descriptor: Int32 in [0, 1, 2] {
            try posixCheck(posix_spawn_file_actions_addinherit_np(&actions, descriptor), "Inherit command stream")
        }
    }
    let argumentStrings = command.map { strdup($0)! }
    let environmentStrings = environment.map { strdup("\($0.key)=\($0.value)")! }
    defer {
        for pointer in argumentStrings + environmentStrings { free(pointer) }
    }
    var arguments: [UnsafeMutablePointer<CChar>?] = argumentStrings.map { $0 } + [nil]
    var variables: [UnsafeMutablePointer<CChar>?] = environmentStrings.map { $0 } + [nil]
    var child: pid_t = 0
    let status = arguments.withUnsafeMutableBufferPointer { arguments in
        variables.withUnsafeMutableBufferPointer { variables in
            if command[0].contains("/") {
                return posix_spawn(&child, command[0], &actions, &attributes,
                                   arguments.baseAddress!, variables.baseAddress!)
            }
            return posix_spawnp(&child, command[0], &actions, &attributes,
                                arguments.baseAddress!, variables.baseAddress!)
        }
    }
    try posixCheck(status, "Start command")
    InterruptState.shared.begin(child)
    return child
}

func writePTY(_ descriptor: Int32, _ data: Data) throws {
    var offset = 0
    try data.withUnsafeBytes { bytes in
        while offset < bytes.count {
            let count = Darwin.write(descriptor, bytes.baseAddress!.advanced(by: offset), bytes.count - offset)
            if count < 0 {
                if errno == EINTR { continue }
                throw CheckFailure(description: "Write PTY: \(String(cString: strerror(errno)))")
            }
            offset += count
        }
    }
}

func waitForChild(_ child: pid_t, timeout: TimeInterval) throws -> Bool {
    let deadline = ProcessInfo.processInfo.systemUptime + timeout
    while true {
        var status: Int32 = 0
        let result = waitpid(child, &status, WNOHANG)
        if result == child { return true }
        if result < 0 {
            if errno == EINTR { continue }
            throw CheckFailure(description: "Wait for logout: \(String(cString: strerror(errno)))")
        }
        if ProcessInfo.processInfo.systemUptime >= deadline { return false }
        usleep(10_000)
    }
}

func checkLogout(master: Int32, child: pid_t, auth: String) throws {
    defer { InterruptState.shared.end(child) }
    var failure: Error?
    do {
        var output = Data()
        var selected = false
        var passed = false
        let deadline = ProcessInfo.processInfo.systemUptime + 30
        while ProcessInfo.processInfo.systemUptime < deadline {
            try InterruptState.shared.check()
            var descriptor = pollfd(fd: master, events: Int16(POLLIN), revents: 0)
            let ready = poll(&descriptor, 1, 100)
            if ready < 0 && errno != EINTR {
                throw CheckFailure(description: "Poll PTY: \(String(cString: strerror(errno)))")
            }
            if ready > 0 {
                var bytes = [UInt8](repeating: 0, count: 65_536)
                let count = Darwin.read(master, &bytes, bytes.count)
                if count < 0 {
                    if errno == EINTR { continue }
                    throw CheckFailure(description: "Read PTY: \(String(cString: strerror(errno)))")
                }
                output.append(contentsOf: bytes.prefix(count))
            }
            if !selected && output.range(of: Data("Select provider to logout:".utf8)) != nil {
                try writePTY(master, Data("openai\r".utf8))
                selected = true
            }
            let credentials = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: auth)))
            guard let object = credentials as? [String: Any] else {
                throw CheckFailure(description: "The credential file must contain a JSON object")
            }
            if selected && object["openai"] == nil {
                let expected: [String: Any] = ["type": "api_key", "key": "pitex-test-only-other"]
                try require((object["anthropic"] as? NSDictionary)?.isEqual(to: expected) == true,
                            "Unselected provider must remain signed in")
                print("PASS generated /logout flow: selected fake provider removed, other provider preserved")
                passed = true
                break
            }
        }
        try require(passed, "The generated logout flow did not remove the selected test credential")
    } catch { failure = error }
    do {
        if !InterruptState.shared.isCancelled { try writePTY(master, Data([3, 3])) }
        if !(try waitForChild(child, timeout: 3)) {
            if kill(-child, SIGTERM) != 0 {
                throw CheckFailure(description: "Stop logout: \(String(cString: strerror(errno)))")
            }
            try require(try waitForChild(child, timeout: 3), "Logout did not stop within 3 seconds")
        }
    } catch { failure = error }
    close(master)
    try InterruptState.shared.check()
    if let failure { throw failure }
}

// Match pathlib's lexical normalization without resolving a relative runtime symlink.
func normalizedPath(_ path: String) -> String {
    let root = path.hasPrefix("//") && !path.hasPrefix("///") ? "//" : (path.hasPrefix("/") ? "/" : "")
    let parts = path.split(separator: "/").filter { $0 != "." }.joined(separator: "/")
    let result = root + parts
    return result.isEmpty ? "." : result
}

func main() throws {
    guard let resolvedTool = realpath(CommandLine.arguments[0], nil) else {
        throw CheckFailure(description: "Resolve tool path: \(String(cString: strerror(errno)))")
    }
    let toolPath = String(cString: resolvedTool)
    free(resolvedTool)
    let repoPath = ((toolPath as NSString).deletingLastPathComponent as NSString).deletingLastPathComponent
    let home = normalizedPath(ProcessInfo.processInfo.environment["HOME"] ?? FileManager.default.homeDirectoryForCurrentUser.path)
    let defaultRuntime = home + (home.hasSuffix("/") ? "" : "/") + "Library/Application Support/Pitex/pi-runtime/bin/pi"
    let runtime = normalizedPath(CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : defaultRuntime)
    var runtimeInfo = stat()
    try require(fstatat(AT_FDCWD, runtime, &runtimeInfo, 0) == 0 && runtimeInfo.st_mode & S_IFMT == S_IFREG,
                "Install the Pitex Agent runtime before running this check")
    var template = Array("/tmp/pitex-settings-XXXXXX".utf8CString)
    guard let path = mkdtemp(&template) else {
        throw CheckFailure(description: "Create temporary folder: \(String(cString: strerror(errno)))")
    }
    let directory = String(cString: path)
    defer { try? FileManager.default.removeItem(atPath: directory) }
    let root = directory + "/space $dollar's folder"
    let agent = root + "/pi"
    try FileManager.default.createDirectory(atPath: agent, withIntermediateDirectories: true)
    let executable = root + "/pi-runtime/bin/pi"
    try FileManager.default.createDirectory(atPath: URL(fileURLWithPath: executable).deletingLastPathComponent().path,
                                            withIntermediateDirectories: true)
    try FileManager.default.createSymbolicLink(atPath: executable, withDestinationPath: runtime)
    var environment = ProcessInfo.processInfo.environment
    environment["PI_CODING_AGENT_DIR"] = agent
    environment["TERM"] = "xterm-256color"
    let source = directory + "/Check.swift"
    let checkSource = "\n" + #"""
import Foundation

@main struct Check {
    static func main() async throws {
        let models = try PiPaths.configurationFile(customProvider: true)
        precondition(models == PiPaths.modelsFileURL)
        let initial = try Data(contentsOf: models)
        let object = try JSONSerialization.jsonObject(with: initial) as! [String: Any]
        precondition((object["providers"] as? [String: Any])?.isEmpty == true)
        let existing = "{\n  \"providers\": {}\n}\n\n"
        try existing.write(to: models, atomically: true, encoding: .utf8)
        _ = try PiPaths.configurationFile(customProvider: true)
        let preserved = try String(contentsOf: models, encoding: .utf8)
        precondition(preserved == existing, "Opening must preserve existing custom configuration")
        let settings = try PiPaths.configurationFile()
        precondition(settings == PiPaths.settingsFileURL && settings != models)
        for logout in [false, true] {
            let script = try await PiRuntimeInstaller.authenticationScript(logout: logout)
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/bin/zsh")
            process.arguments = ["-n", script.path]
            try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
                process.terminationHandler = { _ in continuation.resume() }
                do { try process.run() }
                catch { continuation.resume(throwing: error) }
            }
            process.terminationHandler = nil
            precondition(process.terminationStatus == 0)
            let attributes = try FileManager.default.attributesOfItem(atPath: script.path)
            precondition((attributes[.posixPermissions] as? NSNumber)?.intValue == 0o700)
        }
        print("PASS models.json creation/preservation; settings.json unchanged; auth script syntax/permissions")
    }
}
"""# + "\n"
    try checkSource.write(toFile: source, atomically: false, encoding: .utf8)
    let features = repoPath + "/Mac/Sources/Features"
    let check = directory + "/check"
    let products = repoPath + "/DerivedData/Build/Products/Release"
    try run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-I", products,
             "-module-cache-path", directory + "/cache", source] +
            ["PiAuthStore.swift", "PiAgentProcess.swift", "PiRPC.swift"].map { features + "/" + $0 } +
            ["BuildCore", "TexDomain"].map { products + "/" + $0 + ".o" } + ["-o", check])
    try run([check], environment: environment)

    let auth = agent + "/auth.json"
    try #"{"openai": {"type": "api_key", "key": "pitex-test-only"}, "anthropic": {"type": "api_key", "key": "pitex-test-only-other"}}"#
        .write(toFile: auth, atomically: false, encoding: .utf8)
    var master: Int32 = 0
    var slave: Int32 = 0
    var size = winsize(ws_row: 40, ws_col: 120, ws_xpixel: 0, ws_ypixel: 0)
    try require(openpty(&master, &slave, nil, nil, &size) == 0,
                "Open PTY: \(String(cString: strerror(errno)))")
    let child: pid_t
    do {
        child = try spawnChild(["/bin/zsh", agent + "/Pitex Agent Logout.command"], environment: environment,
                               root: root, pty: (master, slave))
    } catch {
        close(master)
        close(slave)
        throw error
    }
    close(slave)
    try checkLogout(master: master, child: child, auth: auth)
}

_ = InterruptState.shared
do {
    try main()
    try InterruptState.shared.check()
} catch {
    if error is Interrupted || InterruptState.shared.isCancelled { exit(130) }
    FileHandle.standardError.write(Data(("\(error)\n").utf8))
    exit(1)
}
