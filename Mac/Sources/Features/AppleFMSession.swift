import AppPorts
import Foundation

#if canImport(FoundationModels)
import FoundationModels
#endif

enum AppleFMRuntime {
    static func availability() -> AppleFMAvailability {
        #if canImport(FoundationModels)
        if #available(macOS 26.0, *) {
            return AppleFMFoundationProbe.availability()
        }
        #endif
        return .unavailable(.unsupportedSystem)
    }
}

/// Event vocabulary matches `PitexAgent/bridge`, adapted from T3 Code v0.1.4 (MIT).
final class AppleFMSession: AgentSession, @unchecked Sendable {
    let events: AsyncStream<PiRPCEvent>
    private let continuation: AsyncStream<PiRPCEvent>.Continuation
    private let root: URL
    private let contextTokens: Int
    private let instructions: String
    private let lock = NSLock()
    private var alive = true
    private var generation = 0
    private var turnToken = 0
    private var turn: Task<Void, Never>?
    private var cursor = AppleFMStreamCursor()
    private var remembered: Set<String> = []
    private var waiters: [String: CheckedContinuation<AppleFMApprovalDecision, Never>] = [:]
    private var command: Process?
    private var estimatedTokens = 0
    private var latestSnapshotBytes = 0
    private let models = AppleFMModelStore()

    init(workingDirectory: URL, contextTokens: Int, instructions: String) {
        self.root = workingDirectory
        self.contextTokens = contextTokens
        self.instructions = instructions
        var captured: AsyncStream<PiRPCEvent>.Continuation!
        events = AsyncStream { captured = $0 }
        continuation = captured
    }

    var isRunning: Bool {
        lock.lock()
        defer { lock.unlock() }
        return alive
    }

    func send(_ command: PiRPCCommand, id: String?) throws {
        guard isRunning else { throw PiAgentProcessError.notRunning }
        switch command {
        case let .prompt(message, _):
            beginPrompt(message, id: id)
        case .abort:
            cancelTurn()
            emit(AppleFMEvents.response(command: "abort", id: id, success: true))
        case let .approvalResponse(approvalID, decision):
            resume(id: approvalID, decision: AppleFMApprovalDecision(wire: decision))
            emit(AppleFMEvents.response(command: "approval_response", id: id, success: true))
        case .getState:
            emit(AppleFMEvents.state(contextTokens: contextTokens, id: id))
        case .getAvailableModels:
            emit(AppleFMEvents.models(contextTokens: contextTokens, id: id))
        case .getAvailableThinkingLevels:
            emit(AppleFMEvents.thinkingLevels(id: id))
        case .getSessionStats:
            emit(AppleFMEvents.sessionStats(totalTokens: tokenEstimate(), contextTokens: contextTokens, id: id))
        case .getCommands:
            emit(AppleFMEvents.commands(id: id))
        case let .setModel(provider, modelID):
            emit(AppleFMEvents.setModel(provider: provider, modelID: modelID, contextTokens: contextTokens, id: id))
        case let .setThinkingLevel(level):
            emit(AppleFMEvents.setThinkingLevel(level, id: id))
        case .newSession:
            cancelTurn()
            models.reset()
            emit(AppleFMEvents.response(command: "new_session", id: id, success: true))
        case .switchSession:
            emit(AppleFMEvents.response(
                command: "switch_session",
                id: id,
                success: true,
                data: .object(["cancelled": .bool(true)])
            ))
        case .getMessages:
            emit(AppleFMEvents.response(
                command: "get_messages",
                id: id,
                success: true,
                data: .object(["messages": .array([])])
            ))
        case .steer, .extensionUICancelled:
            emit(AppleFMEvents.response(command: "extension_ui_response", id: id, success: true))
        }
    }

    func terminate() {
        lock.lock()
        alive = false
        lock.unlock()
        cancelTurn()
        continuation.finish()
    }

    private func beginPrompt(_ message: String, id: String?) {
        if turnIsActive() {
            emit(AppleFMEvents.response(
                command: "prompt",
                id: id,
                success: false,
                error: "Apple Foundation Models are still replying. Wait for this reply to finish, then send the next prompt."
            ))
            return
        }
        switch AppleFMPromptBudget.fit(message, contextTokens: contextTokens) {
        case .rejected(let error):
            emit(AppleFMEvents.response(command: "prompt", id: id, success: false, error: error))
        case .ready(let text):
            emit(AppleFMEvents.response(command: "prompt", id: id, success: true))
            startTurn(text)
        case .trimmed(let text):
            emit(AppleFMEvents.response(command: "prompt", id: id, success: true))
            emit(AppleFMEvents.notify("The on-device context window is small, so part of this prompt was left out."))
            startTurn(text)
        }
    }

    private func startTurn(_ text: String) {
        lock.lock()
        generation += 1
        let token = generation
        turnToken = token
        turn = Task { [weak self] in
            await self?.runTurn(text, token: token)
        }
        lock.unlock()
    }

    private func runTurn(_ text: String, token: Int) async {
        defer { clearTurn(token) }
        #if canImport(FoundationModels)
        if #available(macOS 26.0, *) {
            await AppleFMGeneration.run(
                text: text,
                instructions: instructions,
                root: root,
                store: models,
                host: self,
                token: token
            )
            return
        }
        #endif
        closeTurn(
            stopReason: "error",
            errorMessage: AppleFMUnavailableReason.unsupportedSystem.message,
            token: token
        )
    }

    fileprivate func emit(eventsFrom produced: [AppleFMJSON]) {
        for event in produced {
            emit(event)
        }
    }

    fileprivate func emit(_ json: AppleFMJSON) {
        guard let event = AppleFMPayload.event(json) else { return }
        continuation.yield(event)
    }

    fileprivate func currentGeneration() -> Int {
        lock.lock()
        defer { lock.unlock() }
        return generation
    }

    fileprivate func emitDuringTurn(_ json: AppleFMJSON, token: Int) {
        guard owns(token) else { return }
        emit(json)
    }

    fileprivate func consume(snapshot: String, token: Int) {
        lock.lock()
        let produced: [AppleFMJSON]?
        if generation == token {
            latestSnapshotBytes = snapshot.utf8.count
            produced = cursor.consume(snapshot: snapshot)
        } else {
            produced = nil
        }
        lock.unlock()
        if let produced { emit(eventsFrom: produced) }
    }

    fileprivate func closeTurn(stopReason: String, errorMessage: String?, token: Int) {
        lock.lock()
        let produced: [AppleFMJSON]?
        if generation == token {
            produced = cursor.close(stopReason: stopReason, errorMessage: errorMessage)
        } else {
            produced = nil
        }
        lock.unlock()
        if let produced { emit(eventsFrom: produced) }
    }

    fileprivate func latestBytes() -> Int {
        lock.lock()
        defer { lock.unlock() }
        return latestSnapshotBytes
    }

    fileprivate func addTokens(_ count: Int) {
        lock.lock()
        estimatedTokens += count
        lock.unlock()
    }

    fileprivate func rememberedKinds() -> Set<String> {
        lock.lock()
        defer { lock.unlock() }
        return remembered
    }

    fileprivate func remember(kind: String) {
        lock.lock()
        remembered.insert(kind)
        lock.unlock()
    }

    fileprivate func waitForApproval(id: String) async -> AppleFMApprovalDecision {
        await withCheckedContinuation { continuation in
            lock.lock()
            waiters[id] = continuation
            lock.unlock()
        }
    }

    fileprivate func track(command process: Process?) {
        lock.lock()
        command = process
        lock.unlock()
    }

    private func resume(id: String, decision: AppleFMApprovalDecision) {
        lock.lock()
        let waiter = waiters.removeValue(forKey: id)
        lock.unlock()
        waiter?.resume(returning: decision)
    }

    private func cancelTurn() {
        lock.lock()
        generation += 1
        let task = turn
        let pending = waiters
        waiters = [:]
        let process = command
        command = nil
        let produced = cursor.opened
            ? cursor.close(stopReason: "error", errorMessage: "The turn was cancelled.")
            : []
        lock.unlock()
        task?.cancel()
        process?.terminate()
        for waiter in pending.values {
            waiter.resume(returning: .deny)
        }
        emit(eventsFrom: produced)
    }

    private func turnIsActive() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return turn != nil
    }

    private func clearTurn(_ token: Int) {
        lock.lock()
        if turnToken == token {
            turn = nil
            turnToken = 0
        }
        lock.unlock()
    }

    fileprivate func ownsTurn(_ token: Int) -> Bool {
        owns(token)
    }

    private func owns(_ token: Int) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return generation == token
    }

    private func tokenEstimate() -> Int {
        lock.lock()
        defer { lock.unlock() }
        return estimatedTokens
    }
}

/// Holds the framework session behind a box so this type stays available on macOS 15.
final class AppleFMModelStore: @unchecked Sendable {
    private let lock = NSLock()
    private var needsReset = false
    fileprivate var boxed: AnyObject?

    func reset() {
        lock.lock()
        needsReset = true
        lock.unlock()
    }

    fileprivate func consumeReset() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        let value = needsReset
        needsReset = false
        return value
    }
}

enum AppleFMPayload {
    static func event(_ json: AppleFMJSON) -> PiRPCEvent? {
        guard let dictionary = object(json) as? [String: Any] else { return nil }
        return PiRPCEvent(dictionary)
    }

    static func object(_ json: AppleFMJSON) -> Any {
        switch json {
        case .string(let value):
            return value
        case .int(let value):
            return value
        case .double(let value):
            return value
        case .bool(let value):
            return value
        case .object(let fields):
            let result = NSMutableDictionary()
            for (key, value) in fields {
                result[key] = object(value)
            }
            return result
        case .array(let values):
            let result = NSMutableArray()
            for value in values {
                result.add(object(value))
            }
            return result
        }
    }
}

#if canImport(FoundationModels)
@available(macOS 26.0, *)
enum AppleFMFoundationProbe {
    static func availability() -> AppleFMAvailability {
        let model = SystemLanguageModel.default
        switch model.availability {
        case .available:
            return .available(contextTokens: AppleFMCatalog.defaultContextTokens)
        case .unavailable(.deviceNotEligible):
            return .unavailable(.deviceNotEligible)
        case .unavailable(.appleIntelligenceNotEnabled):
            return .unavailable(.appleIntelligenceNotEnabled)
        case .unavailable(.modelNotReady):
            return .unavailable(.modelNotReady)
        case .unavailable:
            return .unavailable(.unknown)
        }
    }
}

@available(macOS 26.0, *)
enum AppleFMGeneration {
    static func run(
        text: String,
        instructions: String,
        root: URL,
        store: AppleFMModelStore,
        host: AppleFMSession,
        token: Int
    ) async {
        let broker = AppleFMWork(root: root, host: host)
        let box: AppleFMModelBox
        if store.consumeReset() || store.boxed as? AppleFMModelBox == nil {
            box = AppleFMModelBox()
            store.boxed = box
        } else {
            box = store.boxed as! AppleFMModelBox
        }
        if box.session == nil {
            box.session = LanguageModelSession(
                tools: [
                    AppleFMReadTool(broker: broker),
                    AppleFMWriteTool(broker: broker),
                    AppleFMCommandTool(broker: broker),
                ],
                instructions: instructions
            )
        }
        guard let session = box.session else { return }
        do {
            let stream = session.streamResponse(to: Prompt(text))
            for try await partial in stream {
                if Task.isCancelled || !host.ownsTurn(token) { break }
                host.consume(snapshot: partial.content, token: token)
            }
            if Task.isCancelled {
                host.closeTurn(stopReason: "error", errorMessage: "The turn was cancelled.", token: token)
            } else {
                host.addTokens(max((text.utf8.count + host.latestBytes()) / 4, 1))
                host.closeTurn(stopReason: "stop", errorMessage: nil, token: token)
            }
        } catch is CancellationError {
            host.closeTurn(stopReason: "error", errorMessage: "The turn was cancelled.", token: token)
        } catch {
            host.closeTurn(stopReason: "error", errorMessage: error.localizedDescription, token: token)
        }
    }
}

@available(macOS 26.0, *)
private final class AppleFMModelBox: @unchecked Sendable {
    var session: LanguageModelSession?
}

@available(macOS 26.0, *)
final class AppleFMWork: @unchecked Sendable {
    let root: URL
    private weak var host: AppleFMSession?

    init(root: URL, host: AppleFMSession) {
        self.root = root
        self.host = host
    }

    func run(name: String, arguments: [String: String]) async -> String {
        guard let host else { return "The assistant session is gone." }
        let token = host.currentGeneration()
        let callID = UUID().uuidString
        host.emitDuringTurn(AppleFMEvents.toolStart(id: callID, name: name, arguments: arguments), token: token)
        let permission = AppleFMTools.permission(for: name, rememberedKinds: host.rememberedKinds())
        switch permission {
        case .refuse:
            let text = "That tool is not available in Apple Foundation Models."
            host.emitDuringTurn(AppleFMEvents.toolEnd(id: callID, text: text, isError: true), token: token)
            return text
        case .allow:
            return perform(name: name, arguments: arguments, callID: callID, host: host, token: token)
        case .ask(let kind, let title):
            let detail = arguments["command"] ?? arguments["path"] ?? ""
            host.emitDuringTurn(AppleFMEvents.approval(id: callID, kind: kind, title: title, detail: detail), token: token)
            let decision = await host.waitForApproval(id: callID)
            if Task.isCancelled || decision == .deny || !host.ownsTurn(token) {
                let text = "The user denied this action."
                host.emitDuringTurn(AppleFMEvents.toolEnd(id: callID, text: text, isError: true), token: token)
                return text
            }
            if decision == .alwaysAllow {
                host.remember(kind: kind)
            }
            return perform(name: name, arguments: arguments, callID: callID, host: host, token: token)
        }
    }

    private func perform(
        name: String,
        arguments: [String: String],
        callID: String,
        host: AppleFMSession,
        token: Int
    ) -> String {
        let outcome: (String, Bool)
        switch name {
        case AppleFMTools.read:
            outcome = read(arguments["path"] ?? "")
        case AppleFMTools.write:
            outcome = write(arguments["path"] ?? "", contents: arguments["contents"] ?? "")
        case AppleFMTools.run:
            outcome = command(arguments["command"] ?? "", host: host)
        default:
            outcome = ("That tool is not available in Apple Foundation Models.", true)
        }
        host.emitDuringTurn(AppleFMEvents.toolEnd(id: callID, text: outcome.0, isError: outcome.1), token: token)
        return outcome.0
    }

    private func read(_ path: String) -> (String, Bool) {
        guard let relative = AppleFMPaths.relative(path) else {
            return ("The path must stay inside the project.", true)
        }
        let url = root.appendingPathComponent(relative)
        guard let text = try? String(contentsOf: url, encoding: .utf8) else {
            return ("Could not read \(relative) as UTF-8 text.", true)
        }
        let limit = 8_000
        if text.utf8.count <= limit { return (text, false) }
        let prefix = String(decoding: text.utf8.prefix(limit), as: UTF8.self)
        return (prefix + "\n…(truncated)", false)
    }

    private func write(_ path: String, contents: String) -> (String, Bool) {
        guard let relative = AppleFMPaths.relative(path) else {
            return ("The path must stay inside the project.", true)
        }
        if contents.utf8.count > 256_000 {
            return ("The file is larger than Apple Foundation Models will write in one call.", true)
        }
        let url = root.appendingPathComponent(relative)
        do {
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            try contents.write(to: url, atomically: true, encoding: .utf8)
            return ("Wrote \(relative).", false)
        } catch {
            return ("Could not write \(relative).", true)
        }
    }

    private func command(_ line: String, host: AppleFMSession) -> (String, Bool) {
        let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return ("The command was empty.", true) }
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/zsh")
        process.arguments = ["-c", trimmed]
        process.currentDirectoryURL = root
        let output = Pipe()
        process.standardOutput = output
        process.standardError = output
        host.track(command: process)
        defer { host.track(command: nil) }
        do {
            try process.run()
        } catch {
            return ("Could not run the command.", true)
        }
        let deadline = Date().addingTimeInterval(30)
        while process.isRunning, Date() < deadline {
            Thread.sleep(forTimeInterval: 0.05)
            if Task.isCancelled {
                process.terminate()
                return ("The turn was cancelled.", true)
            }
        }
        if process.isRunning {
            process.terminate()
            return ("The command ran longer than 30 seconds and was stopped.", true)
        }
        let data = output.fileHandleForReading.readDataToEndOfFile()
        let text = String(decoding: data.prefix(4_000), as: UTF8.self)
        let status = process.terminationStatus
        if status == 0 { return (text.isEmpty ? "The command finished." : text, false) }
        return (text.isEmpty ? "The command exited with status \(status)." : text, true)
    }
}

@available(macOS 26.0, *)
struct AppleFMReadTool: Tool {
    let broker: AppleFMWork
    let name = "read_file"
    let description = "Read a UTF-8 text file in the project."

    @Generable
    struct Arguments {
        var path: String
    }

    func call(arguments: Arguments) async throws -> String {
        await broker.run(name: name, arguments: ["path": arguments.path])
    }
}

@available(macOS 26.0, *)
struct AppleFMWriteTool: Tool {
    let broker: AppleFMWork
    let name = "write_file"
    let description = "Replace a UTF-8 text file in the project. Ask before writing."

    @Generable
    struct Arguments {
        var path: String
        var contents: String
    }

    func call(arguments: Arguments) async throws -> String {
        await broker.run(name: name, arguments: ["path": arguments.path, "contents": arguments.contents])
    }
}

@available(macOS 26.0, *)
struct AppleFMCommandTool: Tool {
    let broker: AppleFMWork
    let name = "run_command"
    let description = "Run a shell command in the project directory. Ask before running."

    @Generable
    struct Arguments {
        var command: String
    }

    func call(arguments: Arguments) async throws -> String {
        await broker.run(name: name, arguments: ["command": arguments.command])
    }
}
#endif
