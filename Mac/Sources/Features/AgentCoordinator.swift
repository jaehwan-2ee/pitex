import AppKit
import AppPorts
import AppShell
import Darwin
import Foundation
import PDFKit

/// Snapshot of the workspace state that is folded into every prompt so the
/// agent always knows which document the user is looking at.
struct AgentContextSnapshot: Sendable {
    var projectRoot: URL?
    var activePath: String?
    var activeText: String?
    var selectionText: String?
    var projectFiles: [String] = []
    var pdfPath: String?
    var pdfData: Data?
}

/// The editor range the user is dragging over, surfaced in the assistant
/// panel as an attachable context chip (VS Code/Cursor-style: the current
/// selection is automatically offered to the agent until dismissed).
struct AgentSelectionAttachment: Equatable {
    var path: String
    var startLine: Int
    var endLine: Int
    var text: String
}

/// One row in the native agent transcript.
struct AgentTranscriptEntry: Identifiable {
    enum Role {
        case user
        case assistant
        case thinking
        case tool
        case notice
    }

    enum Status {
        case streaming
        case running
        case done
        case failed
    }

    let id = UUID()
    var role: Role
    var title: String
    var text: String
    var detail: String = ""
    var status: Status = .done
}

/// A saved conversation for the current project — one pi session file.
struct AgentSessionSummary: Identifiable {
    let url: URL
    let date: Date
    let title: String
    var id: URL { url }
}

/// Drives the assistant panel as a native UI front-end for a `pi --mode rpc`
/// subprocess (the paseo-style embedding: the agent runs headlessly and the
/// app renders its event stream). Providers and credentials are configured
/// through pi itself — the app keeps pi's data in an app-local home
/// (`PiPaths.agentDirectory`) so whichever providers pi supports work here.
@MainActor
final class AgentCoordinator: ObservableObject {
    enum Connection: Equatable {
        case idle
        case connecting
        case piMissing
        case ready
        case failed(String)
    }

    @Published private(set) var connection: Connection = .idle
    /// True once the panel (or a send path) has asked for the agent — pi
    /// spawns lazily on first use, and reconnect hooks must not start it
    /// for a window whose Assistant was never opened.
    private(set) var wantsConnection = false
    /// The history list (`/resume` or the clock button), newest first.
    @Published private(set) var pastSessions: [AgentSessionSummary] = []
    @Published var showingSessionHistory = false
    /// Where pi saves this project's sessions (`--session-dir`).
    private var sessionDirectory: URL?
    @Published private(set) var transcript: [AgentTranscriptEntry] = [] {
        didSet {
            // The settings' chat-history limit trims the oldest entries once
            // the transcript grows past it. Mutating inside didSet does not
            // re-trigger this observer.
            if transcript.count > historyLimit {
                transcript.removeFirst(transcript.count - historyLimit)
                assistantEntryIndex = nil
                thinkingEntryIndex = nil
                toolIndexByCallID = [:]
            }
        }
    }
    /// Preferences applied by the workspace after construction: the model the
    /// picker should select once pi reports its catalogue, and the transcript
    /// cap from the AI tab's history-limit stepper.
    var preferredModelID: String?
    var historyLimit = 50
    @Published private(set) var isRunning = false
    @Published private(set) var models: [PiModelDescriptor] = []
    @Published private(set) var currentModel: PiModelDescriptor?
    /// Session token/cost accounting, refreshed after every agent run.
    @Published private(set) var sessionStats: PiSessionStats?
    /// Slash commands pi advertises (extensions, prompts, skills) — drives
    /// the composer's `/` completion list.
    @Published private(set) var commands: [PiSlashCommand] = []
    /// Both the effective level and the model's choices come from pi.
    @Published private(set) var thinkingLevel = "off"
    @Published private(set) var thinkingLevels: [String] = []
    @Published private(set) var isUpdatingModelSettings = false
    @Published var statusMessage: String?
    /// Whether the active document/selection is folded into each prompt's
    /// context envelope (the "Attach activated document" toggle in the
    /// panel header). Persisted per user, not per project.
    @Published var attachActiveDocument: Bool {
        didSet { UserDefaults.standard.set(attachActiveDocument, forKey: Self.attachDocumentKey) }
    }
    /// One-shot text the composer should adopt (selection sent via the Edit
    /// menu, attached file paths). The panel consumes and clears it.
    @Published var pendingComposerInsertion: String?
    /// The editor selection currently attached as context — updated
    /// automatically as the user drags over text. Shown as a chip above the
    /// composer; folded into the prompt envelope when present.
    @Published private(set) var selectionAttachment: AgentSelectionAttachment?
    /// A manually dismissed attachment — the same range must not re-attach
    /// until the user makes a different selection.
    private var suppressedAttachment: AgentSelectionAttachment?
    private var pdfTextCache: (data: Data, text: String?)?

    /// Called by the workspace whenever the editor selection changes.
    /// A new non-empty range attaches automatically; a collapsed caret or a
    /// range identical to the one the user dismissed leaves the chip alone.
    func updateSelectionAttachment(_ attachment: AgentSelectionAttachment?) {
        if attachment == suppressedAttachment { return }
        suppressedAttachment = nil
        selectionAttachment = attachment
    }

    /// X button on the chip: drops the attachment until the selection
    /// actually changes to something else.
    func clearSelectionAttachment() {
        suppressedAttachment = selectionAttachment
        selectionAttachment = nil
    }

    /// Workspace hooks, wired by WorkspaceModel after project open.
    var contextProvider: () -> AgentContextSnapshot = { AgentContextSnapshot() }
    var persistDirtySessions: () async -> String? = { nil }
    var agentActivityDidFinish: () async -> Void = {}

    private let environment: AppEnvironment
    private var process: PiAgentProcess?
    private var eventTask: Task<Void, Never>?
    private var intentionalStop = false
    private var assistantEntryIndex: Int?
    private var thinkingEntryIndex: Int?
    private var textContentIndex: Int?
    private var thinkingContentIndex: Int?
    private var toolIndexByCallID: [String: Int] = [:]
    /// Token deltas buffer per entry id — transcript indices shift when the
    /// history-limit trim drops rows, so flushes resolve ids at apply time.
    /// One merge per 50 ms window replaces a publish per token.
    private var pendingDeltas: [UUID: String] = [:]
    private var deltaFlushTask: Task<Void, Never>?
    private var prepareTask: Task<Void, Never>?
    private var modelSettingsRequestID: String?
    /// Watches `PiPaths.agentDirectory` so auth/model/settings edits made in
    /// pi's own flows (or by hand) restart the agent without an app relaunch.
    private struct ConfigFileState: Equatable {
        var exists = false
        var mtime: Date?
        var size: UInt64 = 0
    }
    private var configWatcher: DispatchSourceFileSystemObject?
    private var configSnapshot: [String: ConfigFileState] = [:]
    private var pendingConfigRestart: DispatchWorkItem?

    init(environment: AppEnvironment) {
        self.environment = environment
        if UserDefaults.standard.object(forKey: Self.attachDocumentKey) == nil {
            attachActiveDocument = true
        } else {
            attachActiveDocument = UserDefaults.standard.bool(forKey: Self.attachDocumentKey)
        }
    }

    /// Queues text for the composer field (used by Send Selection to AI and
    /// the attach-files button).
    func insertIntoComposer(_ text: String) {
        pendingComposerInsertion = text
    }

    // MARK: - Lifecycle

    /// Idempotent startup: locate the executable and spawn the rpc
    /// subprocess. Called when the panel becomes visible and before the
    /// first prompt is sent. Safe to call again after installing pi or a
    /// spawn failure.
    func prepare() {
        wantsConnection = true
        startConfigWatcher()
        switch connection {
        case .connecting, .ready: return
        case .idle, .piMissing, .failed: break
        }
        guard prepareTask == nil else { return }
        prepareTask = Task { await prepareAgent() }
    }

    func shutdown() {
        intentionalStop = true
        flushDeltas()
        prepareTask?.cancel()
        eventTask?.cancel()
        process?.terminate()
        process = nil
        modelSettingsRequestID = nil
        thinkingLevels = []
        isUpdatingModelSettings = false
    }

    /// Reconnect after provider/model/settings changes: `shutdown()` leaves
    /// `connection` at `.ready`, which would make `prepare()` early-return,
    /// so the state is reset to `.idle` first. A coordinator that never
    /// wanted a connection resets the same way but stays idle — config and
    /// install hooks must not spawn pi for an unopened Assistant.
    func restart() {
        shutdown()
        connection = .idle
        intentionalStop = false
        sessionStats = nil
        commands = []
        if wantsConnection { prepare() }
    }

    /// Install hooks (re)connect only when the panel already asked for a
    /// connection; otherwise the first panel appearance prepares it.
    /// `reconnect` takes the shutdown-and-prepare path for a binary update —
    /// a plain `prepare()` early-returns on `.ready` and can't pick it up.
    func resumeIfWanted(reconnect: Bool = false) {
        guard wantsConnection else { return }
        if reconnect { restart() } else { prepare() }
    }

    // MARK: - Agent config watch

    /// `startWatcher(for:)` in PitexApp.swift, applied to the agent
    /// directory: a `.write` event on the directory fires for any file
    /// create/replace inside it, and a snapshot of the three config files
    /// filters out unrelated writes (sessions, skills, logs).
    private func startConfigWatcher() {
        guard configWatcher == nil else { return }
        let directory = PiPaths.agentDirectory
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let descriptor = Darwin.open(directory.path, O_EVTONLY)
        guard descriptor >= 0 else { return }
        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: descriptor,
            eventMask: .write,
            queue: .main
        )
        source.setEventHandler { [weak self] in
            Task { @MainActor in self?.agentConfigChangedOnDisk() }
        }
        source.setCancelHandler { Darwin.close(descriptor) }
        source.resume()
        configSnapshot = Self.agentConfigSnapshot()
        configWatcher = source
    }

    /// Workspace teardown — cancelling the source closes its O_EVTONLY fd
    /// in the cancel handler. Kept out of shutdown()/restart(): the watcher
    /// is what triggers config-change restarts, so it survives those.
    func stopConfigWatcher() {
        pendingConfigRestart?.cancel()
        pendingConfigRestart = nil
        configWatcher?.cancel()
        configWatcher = nil
    }

    private func agentConfigChangedOnDisk() {
        let snapshot = Self.agentConfigSnapshot()
        guard snapshot != configSnapshot else { return }
        configSnapshot = snapshot
        // Coalesce write bursts (pi rewrites several files on login) into
        // one restart once the directory settles.
        pendingConfigRestart?.cancel()
        let work = DispatchWorkItem { [weak self] in
            Task { @MainActor in self?.restart() }
        }
        pendingConfigRestart = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5, execute: work)
    }

    private static func agentConfigSnapshot() -> [String: ConfigFileState] {
        var snapshot: [String: ConfigFileState] = [:]
        for url in [PiPaths.authFileURL, PiPaths.modelsFileURL, PiPaths.settingsFileURL] {
            let attributes = try? FileManager.default.attributesOfItem(atPath: url.path)
            snapshot[url.lastPathComponent] = ConfigFileState(
                exists: attributes != nil,
                mtime: attributes?[.modificationDate] as? Date,
                size: (attributes?[.size] as? UInt64) ?? 0
            )
        }
        return snapshot
    }

    private func prepareAgent() async {
        defer { prepareTask = nil }
        guard let root = contextProvider().projectRoot else { return }
        let tools = await PiToolchain.discover()
        guard !Task.isCancelled else { return }
        guard let executable = PiExecutableLocator.resolve(environment: tools.environment) else {
            connection = .piMissing
            statusMessage = nil
            return
        }
        await spawn(executable: executable, root: root, tools: tools)
    }

    private func spawn(executable: URL, root: URL, tools: PiToolchain) async {
        intentionalStop = false
        connection = .connecting
        // Every launch starts a new saved session in this project's folder,
        // so past conversations can be resumed after a restart.
        let sessions = Self.sessionDirectory(for: root)
        try? FileManager.default.createDirectory(at: sessions, withIntermediateDirectories: true)
        sessionDirectory = sessions
        let arguments = [
            "--mode", "rpc",
            "--session-dir", sessions.path,
            "--append-system-prompt", Self.systemPromptSupplement,
        ]
        let launch: (executable: URL, arguments: [String])
        do {
            launch = try tools.launch(executable, arguments: arguments)
        } catch {
            connection = .failed(error.localizedDescription)
            return
        }
        let agent = PiAgentProcess(
            executableURL: launch.executable,
            workingDirectory: root,
            arguments: launch.arguments,
            environment: Self.childEnvironment(executable: launch.executable, environment: tools.environment)
        )
        do {
            try agent.start { [weak self, weak agent] in
                Task { @MainActor in self?.processDidExit(agent) }
            }
        } catch {
            connection = .failed(error.localizedDescription)
            return
        }
        process = agent
        connection = .ready
        eventTask = Task { [weak self] in
            for await event in agent.events {
                guard let self else { return }
                self.handle(event)
            }
        }
        sendModelSettings(.getState)
        sendSilently(.getAvailableModels)
        sendSilently(.getSessionStats)
        sendSilently(.getCommands)
        SubscriptionUsageStore.shared.refresh(provider: currentModel?.provider)
    }

    private func processDidExit(_ exited: PiAgentProcess?) {
        guard exited == nil || process === exited else { return }
        flushDeltas()
        process = nil
        isRunning = false
        modelSettingsRequestID = nil
        thinkingLevels = []
        isUpdatingModelSettings = false
        guard !intentionalStop else { return }
        connection = .failed("The agent process exited unexpectedly.")
    }

    // MARK: - Send / stop / session

    func send(prompt: String) {
        let trimmed = prompt.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        switch connection {
        case .piMissing:
            statusMessage = "Pitex Agent is not installed yet — it installs automatically on launch; see Settings → AI."
            return
        default:
            break
        }
        // pi's own /resume is an interactive TUI picker; here it opens the
        // panel's history list instead.
        if trimmed == "/resume" {
            showSessionHistory()
            return
        }
        flushDeltas()
        transcript.append(AgentTranscriptEntry(role: .user, title: "You", text: trimmed))
        statusMessage = nil
        Task {
            if let problem = await persistDirtySessions() {
                flushDeltas()
                transcript.append(AgentTranscriptEntry(
                    role: .notice, title: "Assistant",
                    text: problem, status: .failed
                ))
                return
            }
            if process == nil {
                // Sending counts as wanting the agent. Await an in-flight
                // prepare rather than racing a second spawn; a post-
                // shutdown `.ready` still prepares directly, as before.
                prepare()
                if let prepareTask {
                    await prepareTask.value
                } else {
                    await prepareAgent()
                }
            }
            guard let process, process.isRunning else {
                if connection == .idle { connection = .connecting }
                statusMessage = "The agent is not running."
                return
            }
            // Slash commands go raw so pi expands /skill:name and extension
            // commands itself; the editor-context envelope is for prose.
            let message = trimmed.hasPrefix("/") ? trimmed : await envelope(for: trimmed)
            do {
                let behavior = isRunning ? "followUp" : nil
                try process.send(.prompt(message: message, streamingBehavior: behavior))
            } catch {
                statusMessage = "The prompt could not be sent: \(error.localizedDescription)"
            }
        }
    }

    func stop() {
        guard isRunning, let process else { return }
        try? process.send(.abort)
    }

    func newSession() {
        guard let process, process.isRunning else { return }
        clearTranscript()
        try? process.send(.newSession)
    }

    private func clearTranscript() {
        flushDeltas()
        transcript = []
        toolIndexByCallID = [:]
        assistantEntryIndex = nil
        thinkingEntryIndex = nil
    }

    // MARK: - Conversation history

    /// Each project's conversations live in their own folder, so the history
    /// list only shows this project's chats.
    static func sessionDirectory(for root: URL) -> URL {
        let name = String(root.standardizedFileURL.path.map { $0.isLetter || $0.isNumber ? $0 : "-" })
        return PiPaths.agentDirectory
            .appendingPathComponent("pitex-sessions", isDirectory: true)
            .appendingPathComponent(name, isDirectory: true)
    }

    func showSessionHistory() {
        let files = sessionDirectory.flatMap {
            try? FileManager.default.contentsOfDirectory(
                at: $0, includingPropertiesForKeys: [.contentModificationDateKey]
            )
        } ?? []
        pastSessions = files
            .filter { $0.pathExtension == "jsonl" }
            .compactMap(Self.sessionSummary)
            .sorted { $0.date > $1.date }
        showingSessionHistory = true
    }

    /// Loads a saved session into pi, then redraws the transcript from its
    /// messages (`switch_session` → `get_messages`).
    func resume(_ session: AgentSessionSummary) {
        showingSessionHistory = false
        guard let process, process.isRunning, !isRunning else { return }
        clearTranscript()
        try? process.send(.switchSession(path: session.url.path))
    }

    /// Title = the first prompt as typed; sessions without one are skipped.
    /// Only the head of the file is read — later tool output can be large.
    nonisolated static func sessionSummary(_ url: URL) -> AgentSessionSummary? {
        guard let handle = try? FileHandle(forReadingFrom: url) else { return nil }
        defer { try? handle.close() }
        let head = String(decoding: (try? handle.read(upToCount: 512 * 1024)) ?? Data(), as: UTF8.self)
        for line in head.split(separator: "\n") {
            guard let entry = try? JSONSerialization.jsonObject(with: Data(line.utf8)) as? [String: Any],
                  entry["type"] as? String == "message",
                  let message = entry["message"] as? [String: Any],
                  message["role"] as? String == "user" else { continue }
            let title = userPromptText(message)
            guard !title.isEmpty else { return nil }
            let date = (try? url.resourceValues(forKeys: [.contentModificationDateKey]))?
                .contentModificationDate ?? .distantPast
            return AgentSessionSummary(url: url, date: date, title: String(title.prefix(120)))
        }
        return nil
    }

    /// A user message as typed: prose prompts carry the `<editor-context>`
    /// envelope in front (see `envelope(for:)`).
    nonisolated static func userPromptText(_ message: [String: Any]) -> String {
        let raw = message["content"] as? String
            ?? (message["content"] as? [[String: Any]] ?? [])
                .compactMap { $0["text"] as? String }
                .joined(separator: "\n")
        guard raw.hasPrefix("<editor-context>"),
              let end = raw.range(of: "\n</editor-context>\n") else {
            return raw.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return raw[end.upperBound...].trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// The same row shapes the live event stream produces.
    private func transcriptEntries(from messages: [[String: Any]]) -> [AgentTranscriptEntry] {
        var entries: [AgentTranscriptEntry] = []
        var toolRows: [String: Int] = [:]
        for message in messages {
            switch message["role"] as? String {
            case "user":
                let text = Self.userPromptText(message)
                if !text.isEmpty {
                    entries.append(AgentTranscriptEntry(role: .user, title: "You", text: text))
                }
            case "assistant":
                for block in message["content"] as? [[String: Any]] ?? [] {
                    switch block["type"] as? String {
                    case "thinking":
                        if let text = block["thinking"] as? String, !text.isEmpty {
                            entries.append(AgentTranscriptEntry(role: .thinking, title: "Thinking", text: text))
                        }
                    case "text":
                        if let text = block["text"] as? String, !text.isEmpty {
                            entries.append(AgentTranscriptEntry(role: .assistant, title: assistantTitle, text: text))
                        }
                    case "toolCall":
                        let name = block["name"] as? String ?? "tool"
                        entries.append(AgentTranscriptEntry(
                            role: .tool, title: name, text: "",
                            detail: toolSummary(name: name, args: block["arguments"] as? [String: Any])
                        ))
                        if let id = block["id"] as? String { toolRows[id] = entries.count - 1 }
                    default:
                        break
                    }
                }
            case "toolResult":
                guard let id = message["toolCallId"] as? String, let index = toolRows[id] else { break }
                if message["isError"] as? Bool == true { entries[index].status = .failed }
                if let text = toolResultText(message) {
                    entries[index].detail = bounded(text, limit: 4_000)
                }
            default:
                break
            }
        }
        return entries
    }

    func selectModel(_ model: PiModelDescriptor) {
        guard !isUpdatingModelSettings else { return }
        sendModelSettings(.setModel(provider: model.provider, modelID: model.id))
    }

    func selectThinkingLevel(_ level: String) {
        guard !isUpdatingModelSettings, thinkingLevels.contains(level) else { return }
        sendModelSettings(.setThinkingLevel(level: level))
    }

    /// Read state after each change, then query capabilities for that state.
    /// Request IDs prevent replies from an earlier refresh restoring old choices.
    private func sendModelSettings(_ command: PiRPCCommand, id: String? = nil) {
        guard let process, process.isRunning else { return }
        let requestID = id ?? UUID().uuidString
        modelSettingsRequestID = requestID
        isUpdatingModelSettings = true
        thinkingLevels = []
        do {
            try process.send(command, id: requestID)
        } catch {
            modelSettingsRequestID = nil
            isUpdatingModelSettings = false
            statusMessage = error.localizedDescription
        }
    }

    private func applyPreferredModelIfReady() {
        guard !isUpdatingModelSettings, let preferred = preferredModelID,
              let model = models.first(where: { $0.id == preferred }) else { return }
        preferredModelID = nil
        if currentModel != model { selectModel(model) }
    }

    // MARK: - Event handling

    private func handle(_ event: PiRPCEvent) {
        // Buffered deltas land before anything else rewrites the transcript.
        if event.type != "message_update" { flushDeltas() }
        switch event.type {
        case "response":
            handleResponse(event)
        case "agent_start":
            isRunning = true
        case "agent_end":
            isRunning = false
            finalizeEntries()
            if let failure = lastAssistantError(in: event) {
                transcript.append(AgentTranscriptEntry(
                    role: .notice, title: "Assistant", text: failure, status: .failed
                ))
            }
            sendSilently(.getSessionStats)
            SubscriptionUsageStore.shared.refresh(provider: currentModel?.provider)
            Task { await agentActivityDidFinish() }
        case "message_start":
            handleMessageStart(event)
        case "message_update":
            handleMessageUpdate(event)
        case "message_end":
            handleMessageEnd(event)
        case "turn_end":
            finalizeEntries()
        case "tool_execution_start":
            handleToolStart(event)
        case "tool_execution_update":
            handleToolUpdate(event)
        case "tool_execution_end":
            handleToolEnd(event)
        case "auto_retry_start":
            let attempt = event.object["attempt"] as? Int ?? 1
            let maxAttempts = event.object["maxAttempts"] as? Int ?? 1
            let reason = event.string("errorMessage") ?? "transient error"
            transcript.append(AgentTranscriptEntry(
                role: .notice, title: "Assistant",
                text: "Retrying (\(attempt)/\(maxAttempts)): \(reason)"
            ))
        case "auto_retry_end":
            if !event.bool("success"),
               let finalError = event.string("finalError") {
                transcript.append(AgentTranscriptEntry(
                    role: .notice, title: "Assistant",
                    text: "The request failed after retrying: \(finalError)", status: .failed
                ))
            }
        case "compaction_start":
            transcript.append(AgentTranscriptEntry(
                role: .notice, title: "Assistant", text: "Compacting the conversation…"
            ))
        case "compaction_end":
            if event.bool("aborted") {
                transcript.append(AgentTranscriptEntry(role: .notice, title: "Assistant", text: "Compaction cancelled."))
            } else if let message = event.string("errorMessage") {
                transcript.append(AgentTranscriptEntry(role: .notice, title: "Assistant", text: "Compaction failed: \(message)", status: .failed))
            }
        case "thinking_level_changed":
            if !isUpdatingModelSettings { sendModelSettings(.getState) }
        case "extension_ui_request":
            handleExtensionUI(event)
        case "extension_error":
            let message = event.string("error") ?? "extension error"
            transcript.append(AgentTranscriptEntry(
                role: .notice, title: "Assistant", text: message, status: .failed
            ))
        default:
            break
        }
    }

    private func handleResponse(_ event: PiRPCEvent) {
        let isModelSettingsResponse = [
            "get_state", "get_available_thinking_levels", "set_model", "set_thinking_level",
        ].contains(event.responseCommand ?? "")
        if isModelSettingsResponse {
            guard let id = modelSettingsRequestID, event.string("id") == id else { return }
        }
        guard event.responseSucceeded else {
            if isModelSettingsResponse {
                modelSettingsRequestID = nil
                isUpdatingModelSettings = false
                if event.responseCommand == "set_model" || event.responseCommand == "set_thinking_level" {
                    sendModelSettings(.getState)
                }
            }
            let message = event.responseError ?? "The agent command failed."
            statusMessage = message
            if let command = event.responseCommand, command == "prompt" {
                transcript.append(AgentTranscriptEntry(
                    role: .notice, title: "Assistant", text: message, status: .failed
                ))
            }
            return
        }
        switch event.responseCommand {
        case "get_state":
            currentModel = event.responseData?["model"].flatMap(PiModelDescriptor.init)
            SubscriptionUsageStore.shared.refresh(provider: currentModel?.provider)
            if let level = event.responseData?["thinkingLevel"] as? String {
                thinkingLevel = level
            }
            sendModelSettings(.getAvailableThinkingLevels, id: modelSettingsRequestID)
        case "get_available_thinking_levels":
            modelSettingsRequestID = nil
            isUpdatingModelSettings = false
            guard let levels = event.responseData?["levels"] as? [String],
                  !levels.isEmpty, levels.allSatisfy({ !$0.isEmpty }),
                  Set(levels).count == levels.count, levels.contains(thinkingLevel) else {
                statusMessage = "The agent did not return valid reasoning levels for this model."
                return
            }
            thinkingLevels = currentModel == nil ? [] : levels
            applyPreferredModelIfReady()
        case "get_available_models":
            let raw = event.responseData?["models"] as? [Any] ?? []
            models = raw.compactMap(PiModelDescriptor.init)
            applyPreferredModelIfReady()
        case "get_session_stats":
            if let data = event.responseData {
                sessionStats = PiSessionStats(data)
            }
        case "get_commands":
            let raw = event.responseData?["commands"] as? [Any] ?? []
            // Handled by the panel itself (see `send`), so the composer's
            // completion offers it instead of swapping in another command.
            let resume: [String: Any] = [
                "name": "resume", "description": "Resume a past conversation of this project", "source": "pitex",
            ]
            commands = ([resume] + raw).compactMap(PiSlashCommand.init)
        case "set_model", "set_thinking_level":
            sendModelSettings(.getState, id: modelSettingsRequestID)
        case "new_session":
            sendModelSettings(.getState)
        case "switch_session":
            if event.responseData?["cancelled"] as? Bool != true {
                sendSilently(.getMessages)
            }
            sendModelSettings(.getState)
        case "get_messages":
            transcript = transcriptEntries(from: event.responseData?["messages"] as? [[String: Any]] ?? [])
            sendSilently(.getSessionStats)
        default:
            break
        }
    }

    private func handleMessageStart(_ event: PiRPCEvent) {
        guard let message = event.nested("message"),
              let role = message["role"] as? String else { return }
        // User echoes are rendered optimistically on send; only assistant
        // messages get a live entry here.
        guard role == "assistant" else { return }
        assistantEntryIndex = nil
        thinkingEntryIndex = nil
        textContentIndex = nil
        thinkingContentIndex = nil
    }

    /// Accumulates one token onto an entry's pending text; the first delta
    /// after a flush schedules the next merge.
    private func bufferDelta(_ text: String, for id: UUID) {
        pendingDeltas[id, default: ""] += text
        guard deltaFlushTask == nil else { return }
        deltaFlushTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(50))
            guard let self, !Task.isCancelled else { return }
            self.flushDeltas()
        }
    }

    /// Merges buffered deltas into their transcript rows and drops any
    /// scheduled flush. Anything else that rewrites rows — other events,
    /// sends, clears, shutdown — calls this first so a late flush can never
    /// append after them. Ids no longer in the transcript drop silently.
    private func flushDeltas() {
        deltaFlushTask?.cancel()
        deltaFlushTask = nil
        guard !pendingDeltas.isEmpty else { return }
        let deltas = pendingDeltas
        pendingDeltas = [:]
        for (id, text) in deltas {
            guard let index = transcript.firstIndex(where: { $0.id == id }) else { continue }
            transcript[index].text += text
        }
    }

    private func handleMessageUpdate(_ event: PiRPCEvent) {
        guard let delta = event.nested("assistantMessageEvent"),
              let deltaType = delta["type"] as? String else { return }
        switch deltaType {
        case "text_start":
            flushDeltas()
            textContentIndex = delta["contentIndex"] as? Int
            appendAssistantEntry()
        case "text_delta":
            if delta["contentIndex"] as? Int != textContentIndex {
                flushDeltas()
                textContentIndex = delta["contentIndex"] as? Int
                appendAssistantEntry()
            }
            if let text = delta["delta"] as? String,
               let index = assistantEntryIndex, transcript.indices.contains(index) {
                bufferDelta(text, for: transcript[index].id)
            }
        case "thinking_start":
            flushDeltas()
            thinkingContentIndex = delta["contentIndex"] as? Int
            appendThinkingEntry()
        case "thinking_delta":
            if delta["contentIndex"] as? Int != thinkingContentIndex {
                flushDeltas()
                thinkingContentIndex = delta["contentIndex"] as? Int
                appendThinkingEntry()
            }
            if let text = delta["delta"] as? String,
               let index = thinkingEntryIndex, transcript.indices.contains(index) {
                bufferDelta(text, for: transcript[index].id)
            }
        case "done":
            flushDeltas()
            finalizeEntries()
        case "error":
            flushDeltas()
            let reason = delta["errorMessage"] as? String ?? delta["reason"] as? String ?? "The response failed."
            if let index = assistantEntryIndex, transcript.indices.contains(index) {
                transcript[index].status = .failed
                if transcript[index].text.isEmpty { transcript[index].text = reason }
            } else {
                transcript.append(AgentTranscriptEntry(
                    role: .notice, title: "Assistant", text: reason, status: .failed
                ))
            }
        default:
            break
        }
    }

    private func handleMessageEnd(_ event: PiRPCEvent) {
        guard let message = event.nested("message"),
              let role = message["role"] as? String, role == "assistant",
              let content = message["content"] as? [[String: Any]] else {
            finalizeEntries()
            return
        }
        // Rebuild the rendered text from the final message so the transcript
        // always matches what the model actually produced.
        let text = content
            .filter { ($0["type"] as? String) == "text" }
            .compactMap { $0["text"] as? String }
            .joined(separator: "\n\n")
        if let index = assistantEntryIndex, transcript.indices.contains(index) {
            transcript[index].text = text
            transcript[index].status = (message["stopReason"] as? String) == "error" ? .failed : .done
        } else if !text.isEmpty {
            var entry = AgentTranscriptEntry(
                role: .assistant, title: assistantTitle, text: text,
                status: (message["stopReason"] as? String) == "error" ? .failed : .done
            )
            if let errorMessage = message["errorMessage"] as? String, entry.text.isEmpty {
                entry.text = errorMessage
            }
            transcript.append(entry)
        }
        finalizeEntries()
    }

    private func handleToolStart(_ event: PiRPCEvent) {
        guard let callID = event.string("toolCallId") else { return }
        let name = event.string("toolName") ?? "tool"
        let summary = toolSummary(name: name, args: event.nested("args"))
        transcript.append(AgentTranscriptEntry(
            role: .tool, title: name, text: "", detail: summary, status: .running
        ))
        toolIndexByCallID[callID] = transcript.count - 1
    }

    private func handleToolUpdate(_ event: PiRPCEvent) {
        guard let callID = event.string("toolCallId"),
              let index = toolIndexByCallID[callID],
              transcript.indices.contains(index) else { return }
        if let text = toolResultText(event.nested("partialResult")) {
            transcript[index].detail = bounded(text, limit: 4_000)
        }
    }

    private func handleToolEnd(_ event: PiRPCEvent) {
        guard let callID = event.string("toolCallId"),
              let index = toolIndexByCallID[callID],
              transcript.indices.contains(index) else { return }
        transcript[index].status = event.bool("isError") ? .failed : .done
        if let text = toolResultText(event.nested("result")) {
            transcript[index].detail = bounded(text, limit: 4_000)
        }
    }

    private func handleExtensionUI(_ event: PiRPCEvent) {
        let method = event.string("method") ?? ""
        switch method {
        case "select", "confirm", "input", "editor":
            // Dialog-style requests cannot be rendered inline; cancel them so
            // the agent is never blocked waiting for an answer.
            if let id = event.string("id") {
                try? process?.send(.extensionUICancelled(id: id))
            }
            let title = event.string("title") ?? method
            transcript.append(AgentTranscriptEntry(
                role: .notice, title: "Assistant",
                text: "An interactive prompt (\(title)) is not supported in this panel and was skipped."
            ))
        case "notify":
            if let message = event.string("message") {
                transcript.append(AgentTranscriptEntry(role: .notice, title: "Assistant", text: message))
            }
        case "setStatus":
            statusMessage = event.string("statusText")
        default:
            break
        }
    }

    private func lastAssistantError(in event: PiRPCEvent) -> String? {
        guard let messages = event.object["messages"] as? [[String: Any]] else { return nil }
        for message in messages.reversed() {
            guard (message["role"] as? String) == "assistant" else { continue }
            if (message["stopReason"] as? String) == "error" {
                return message["errorMessage"] as? String ?? "The agent run failed."
            }
            return nil
        }
        return nil
    }

    private func appendAssistantEntry() {
        transcript.append(AgentTranscriptEntry(
            role: .assistant, title: assistantTitle, text: "", status: .streaming
        ))
        assistantEntryIndex = transcript.count - 1
    }

    private func appendThinkingEntry() {
        transcript.append(AgentTranscriptEntry(
            role: .thinking, title: "Thinking", text: "", status: .streaming
        ))
        thinkingEntryIndex = transcript.count - 1
    }

    private func finalizeEntries() {
        if let index = assistantEntryIndex, transcript.indices.contains(index) {
            transcript[index].status = .done
        }
        if let index = thinkingEntryIndex, transcript.indices.contains(index) {
            transcript[index].status = .done
        }
        assistantEntryIndex = nil
        thinkingEntryIndex = nil
        textContentIndex = nil
        thinkingContentIndex = nil
    }

    private var assistantTitle: String {
        currentModel.map { "Pitex Agent · \($0.name)" } ?? "Pitex Agent"
    }

    // MARK: - Prompt envelope

    /// Every prompt carries the live editor context so the agent can act on
    /// the current document without the user having to restate it.
    private func envelope(for prompt: String) async -> String {
        let context = contextProvider()
        var parts: [String] = [
            "<editor-context>",
            "The user is working inside a native macOS LaTeX editor. The project root is the current working directory; use your file tools to inspect or modify project files when asked.",
        ]
        if attachActiveDocument {
            if let path = context.activePath {
                parts.append("Active document (open in the editor right now): \(path)")
                if let text = context.activeText, !text.isEmpty {
                    parts.append("Current content of \(path):\n```\n\(bounded(text, limit: 96_000))\n```")
                }
            }
        }
        // The dragged selection attaches on its own chip — it reaches the
        // agent even when whole-document attach is off.
        if let selection = selectionAttachment, !selection.text.isEmpty {
            parts.append("Text selected in \(selection.path) (lines \(selection.startLine)–\(selection.endLine)):\n```\n\(bounded(selection.text, limit: 16_000))\n```")
        }
        if !context.projectFiles.isEmpty {
            parts.append("Project source files: \(context.projectFiles.joined(separator: ", "))")
        }
        if let pdfData = context.pdfData,
           let pdfText = await cachedPDFText(pdfData),
           !pdfText.isEmpty {
            let label = context.pdfPath ?? "the built PDF preview"
            parts.append("Text extracted from \(label):\n```\n\(bounded(pdfText, limit: 48_000))\n```")
        }
        parts.append("</editor-context>")
        parts.append("")
        parts.append(prompt)
        return parts.joined(separator: "\n")
    }

    private func cachedPDFText(_ data: Data) async -> String? {
        if let cache = pdfTextCache, cache.data == data { return cache.text }
        let text = await Task.detached(priority: .userInitiated) {
            Self.extractPDFText(data)
        }.value
        pdfTextCache = (data, text)
        return text
    }

    nonisolated private static func extractPDFText(_ data: Data) -> String? {
        guard let document = PDFDocument(data: data) else { return nil }
        return document.string
    }

    private func bounded(_ text: String, limit: Int) -> String {
        guard text.utf8.count > limit else { return text }
        let prefix = String(decoding: text.utf8.prefix(limit), as: UTF8.self)
        return prefix + "\n…(truncated — read the file for the full content)"
    }

    // MARK: - Helpers

    private func sendSilently(_ command: PiRPCCommand) {
        try? process?.send(command)
    }

    private func toolSummary(name: String, args: [String: Any]?) -> String {
        guard let args else { return "" }
        switch name {
        case "bash":
            return (args["command"] as? String).map { bounded($0, limit: 2_000) } ?? ""
        case "read":
            return args["path"] as? String ?? ""
        case "edit", "write":
            return args["path"] as? String ?? ""
        case "ls", "find", "grep":
            return args["path"] as? String ?? (args["pattern"] as? String ?? "")
        default:
            guard let data = try? JSONSerialization.data(withJSONObject: args),
                  let text = String(data: data, encoding: .utf8) else { return "" }
            return bounded(text, limit: 500)
        }
    }

    private func toolResultText(_ result: [String: Any]?) -> String? {
        guard let content = result?["content"] as? [[String: Any]] else { return nil }
        let text = content
            .compactMap { $0["text"] as? String }
            .joined(separator: "\n")
        return text.isEmpty ? nil : text
    }

    /// Internal (not private) so `GhostCompletionCoordinator` spawns its
    /// dedicated subprocess with the same PI_CODING_AGENT_DIR/PATH hygiene.
    static func childEnvironment(executable: URL, environment: [String: String]) -> [String: String] {
        var environment = environment
        // The app-local pi home keeps auth/settings/sessions separate from
        // any global ~/.pi the user may have.
        environment["PI_CODING_AGENT_DIR"] = PiPaths.agentDirectory.path
        var search = [
            executable.deletingLastPathComponent().path,
            "/Library/TeX/texbin",
        ]
        if let existing = environment["PATH"] {
            search.append(contentsOf: existing.split(separator: ":").map(String.init))
        }
        search.append(contentsOf: ["/usr/bin", "/bin", "/usr/sbin", "/sbin"])
        var seen = Set<String>()
        environment["PATH"] = search.filter { seen.insert($0).inserted }.joined(separator: ":")
        return environment
    }

    private static let systemPromptSupplement = """
        You are an expert LaTeX writing and research assistant inside Pitex, \
        an AI-accelerated LaTeX editor. When a user shows code, explain or fix \
        it concisely. Default to xelatex semantics. Prefer minimal, idiomatic \
        LaTeX — no unnecessary packages. When you reply with LaTeX, wrap \
        snippets in ```latex code fences so they paste cleanly. Use math mode \
        for equations even in short answers. Use Humanizer when the user asks \
        to write the phrases, sentences, or paragraphs. Use SciSpace when the \
        user asks to search and organize the papers. Use Latex Doctor when the \
        user asks about tex distributions. Use Latex Compile when the user \
        asks o build, render, regenerate, or compile a `.tex` file. Use Texlive \
        Runtime Installer when the user asks to install or repair LaTeX \
        support.
        """

    private static let attachDocumentKey = "ai.attachActiveDocument"
}
