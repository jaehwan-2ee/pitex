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
        /// A todo-list change: `title` is its `assistant.todo.*` key and `text` the
        /// key's argument, so the row is localized when drawn.
        case activity
    }

    enum Status {
        case streaming
        case running
        case done
        case failed
        /// A tool call cut off by stop/interrupt (`status: "canceled"`).
        case canceled
    }

    let id = UUID()
    var role: Role
    var title: String
    var text: String
    var detail: String = ""
    var status: Status = .done
    /// Typed tool-call detail from a bridge backend; nil for pi's own
    /// provider, which keeps the plain name + detail rendering.
    var tool: AgentToolDetail?
    /// Images the user sent with this message (thumbnails in the bubble).
    var images: [AgentImage] = []
    /// A thinking block's duration, set when the next non-reasoning event arrives.
    var thinkingSeconds: Int?
    /// A failed turn: its code, the CLI for the wording, and whether Retry is offered.
    var turnError: AgentTurnError?
    /// On a prompt: how long its turn ran (the folded turn's "Worked for").
    var workedSeconds: Int?
    /// A tool row's call id: subagent calls nest under the card with their parent's id.
    var callID: String?
}

/// The rewind confirmation the panel shows for one message.
struct AgentRewindOffer: Equatable {
    /// Whole turns kept (`rewind.keepTurns`).
    var keepTurns: Int
    /// A rewound prompt goes back into the composer.
    var draft: String?
    /// Files a file rewind would restore; nil while the dry run is out.
    var files: [String]?
    /// Why files cannot be restored (no checkpoint, outside the project, unsupported).
    var filesNote: String?
    /// The dry run, then the confirmed rewind, in flight.
    var requestID: String?
    /// The confirmed rewind is out (the dry run, if any, has answered).
    var isRewinding: Bool { files != nil && requestID != nil }
}

/// The ⋯ menu of a prompt or an answer: each action enabled, or the reason it is not.
struct AgentMessageMenu {
    var forkBlocked: String?
    var rewindBlocked: String?
    var fork: () -> Void
    var rewind: () -> Void
}

/// A saved conversation for the current project — one pi session file.
struct AgentSessionSummary: Identifiable {
    let url: URL
    let date: Date
    let title: String
    /// The backend that owns it (`AgentSessionHistory.source`): pi, claude, agy, codex, or grok.
    var source: String { AgentSessionHistory.source(fileName: url.lastPathComponent) }
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
                droppedPrompts += transcript.prefix(transcript.count - historyLimit).filter { $0.role == .user }.count
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
    /// `get_state.capabilities`; pi omits it (see `AgentCapabilities(state:)`).
    @Published private(set) var capabilities = AgentCapabilities()
    /// The rewind confirmation on screen (Rewind to here).
    @Published private(set) var rewindOffer: AgentRewindOffer?
    /// Replaces the composer text once (a forked or rewound prompt, a quoted chat).
    @Published var composerReplacement: String?
    /// The `fork_session` in flight, with the prompt a fork from a user message puts back.
    @Published private var forkRequest: (id: String, draft: String?)?
    /// When the running turn started (`agent_start`); its prompt row gets `workedSeconds`.
    private var turnStarted: Date?
    /// Prompt rows trimmed off the top by `historyLimit`, so turn counts stay right.
    private var droppedPrompts = 0
    /// A fork or rewind notice, shown under the chat once it is reloaded.
    private var noticeAfterReload: String?
    /// Messages typed while a turn runs, sent one per agent_end.
    @Published private(set) var queue = AgentMessageQueue()
    /// Images attached to the draft, shown as chips above the composer.
    @Published var draftImages: [AgentImage] = []
    /// The `prompt` id of the queued message in flight, and that message's queue id.
    private var drainRequest: (id: String, item: Int)?
    @Published private(set) var models: [PiModelDescriptor] = []
    @Published private(set) var currentModel: PiModelDescriptor?
    /// Session token/cost accounting, refreshed after every agent run.
    @Published private(set) var sessionStats: PiSessionStats?
    /// Latest pushed usage (`usage_update`); the meter falls back to `sessionStats` (pi).
    @Published private(set) var usage = AgentUsage()
    /// The model that wrote the current answer (`message_start.message.modelName`).
    private var liveModelName: String?
    /// When the open thinking block got its first delta.
    private var thinkingStarted: Date?
    /// The last message sent as a prompt, with its images: what Retry resends.
    private var lastSent: AgentRetryPayload?
    /// Retry after a crash: the message goes out once the restarted process is up.
    private var pendingRetry = false

    /// Context used of the window: pushed usage, else pi's pulled stats.
    var contextMeter: AgentContextMeter? {
        if let used = usage.contextUsed, let max = usage.contextMax {
            return AgentContextMeter(used: used, max: max)
        }
        guard let used = sessionStats?.contextTokens, let max = sessionStats?.contextWindow else { return nil }
        return AgentContextMeter(used: used, max: max)
    }
    /// Slash commands pi advertises (extensions, prompts, skills) — drives
    /// the composer's `/` completion list.
    @Published private(set) var commands: [PiSlashCommand] = []
    /// Both the effective level and the model's choices come from pi.
    @Published private(set) var thinkingLevel = "off"
    @Published private(set) var thinkingLevels: [String] = []
    /// `get_state` publishes the menu while this is true. Clearing it has to
    /// publish again, or the model popup stays disabled after the reply.
    @Published private(set) var isUpdatingModelSettings = false {
        didSet {
            guard oldValue != isUpdatingModelSettings else { return }
            publishCatalog()
        }
    }
    @Published var statusMessage: String?
    /// The CLI approval card the panel is showing (tool, question, or plan).
    /// Later requests wait in `approvalQueue`. pi never sets this; its
    /// extension prompts stay auto-cancelled.
    @Published private(set) var pendingApproval: AgentApprovalItem?
    /// Bridge backends' permission mode; nil hides the picker (pi, Codex).
    @Published private(set) var permissionMode: AgentPermissionMode?
    /// The conversation's todo list; empty hides the checklist above the composer.
    @Published private(set) var todos: [AgentTodoItem] = []
    /// FIFO keyed by request id. Stop and new-session deny every entry.
    private var approvalQueue = AgentApprovalQueue()
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
    private var process: (any AgentSession)?
    private var eventTask: Task<Void, Never>?
    private var intentionalStop = false
    private var processEpoch = AgentProcessEpoch()
    private var attachedToken: UInt64 = 0
    /// The backend of the live conversation. Settings can move first; this
    /// catches up when the switch shuts the current turn down.
    private var sessionBackend = AgentCoordinator.selectedAgentProvider()
    /// The backend of the running session; history rows of another one say where they open.
    var activeBackend: String { sessionBackend }
    private var catalogSlots: [AssistantCatalog.Slot] = []
    private var catalogGeneration: UInt64 = 1
    private var switchTicket: AssistantCatalog.SwitchTicket?
    private var catalogTools: PiToolchain?
    private var catalogRoot: URL?
    private var probeGeneration: UInt64 = 1
    private var catalogProbes: [CatalogProbe] = []
    /// Why a backend's catalog failed, shown until that backend lists models.
    private var catalogReasons: [String: String] = [:]
    /// The new session listed models while a settings round trip was open.
    private var catalogCommitPending = false
    private var launchIdle = false
    @Published private(set) var catalogMenu = AssistantCatalog.Menu(
        rows: [], selection: nil, acceptsPicks: false, generation: 1
    )
    /// One "<backend>: <reason>" line per enabled backend whose catalog failed.
    @Published private(set) var catalogNotes: [String] = []
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
    private var messagesRequestID: String?
    /// The `refresh_sessions` reply the history list waits for.
    private var refreshRequestID: String?
    /// A history row of another backend: opened once that backend is up.
    private var pendingResume: AgentSessionSummary?
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
        installCatalogSlots()
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
        installCatalogSlots()
        if launchIdle { return }
        let needsLaunch: Bool
        switch connection {
        case .connecting, .ready:
            needsLaunch = false
        case .idle, .piMissing, .failed:
            needsLaunch = true
        }
        if !needsLaunch, catalogTools != nil {
            refreshCatalogProbes()
            return
        }
        guard prepareTask == nil else { return }
        prepareTask = Task { await prepareAgent() }
    }

    func shutdown() {
        abandonDrain()
        intentionalStop = true
        processEpoch.retire()
        isRunning = false
        stopCatalogProbes()
        flushDeltas()
        approvalQueue.clear()
        pendingApproval = nil
        permissionMode = nil
        // The list belonged to that process's session; the next one starts empty.
        todos = []
        // A fork or rewind the process takes with it never answers.
        forkRequest = nil
        rewindOffer = nil
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
        usage = AgentUsage()
        // Another backend may name no model; the picked one titles its answers.
        liveModelName = nil
        commands = []
        if wantsConnection { prepare() }
    }

    /// A cross-backend choice shuts the turn down before the next session starts.
    func applyCatalog(_ intent: AssistantCatalog.SelectionIntent, shownGeneration: UInt64, settings: SettingsStore) {
        let session = AssistantCatalog.SessionSnapshot(
            isRunning: isRunning,
            backend: sessionBackend,
            current: currentModel.map(Self.catalogModel(from:))
        )
        guard let plan = AssistantCatalog.plan(
            intent: intent,
            facts: catalogFacts(),
            shownGeneration: shownGeneration,
            session: session
        ) else { return }
        switch plan {
        case .setModel(let model):
            if var ticket = switchTicket, ticket.backend == sessionBackend {
                ticket.model = model
                switchTicket = ticket
                settings.aiDefaultModel = model.id
                preferredModelID = nil
                currentModel = Self.descriptor(from: model)
                publishCatalog()
                return
            }
            settings.aiDefaultModel = model.id
            preferredModelID = model.id
            if let descriptor = Self.descriptor(from: model) {
                currentModel = descriptor
                publishCatalog()
                selectModel(descriptor)
            }
        case .switchBackend(let backend, let model):
            beginSwitch(backend: backend, model: model, settings: settings)
        }
    }

    private func beginSwitch(backend: String, model: AssistantCatalog.Model?, settings: SettingsStore) {
        // A row picked for the previous backend must not open in this one.
        pendingResume = nil
        switchTicket = AssistantCatalog.SwitchTicket(backend: backend, model: model)
        preferredModelID = nil
        settings.aiDefaultModel = model?.id ?? ""
        models = []
        currentModel = model.flatMap(Self.descriptor(from:))
        catalogGeneration &+= 1
        sessionBackend = backend
        if !transcript.isEmpty {
            transcript.append(AgentTranscriptEntry(
                role: .notice,
                title: "Assistant",
                text: String(localized: "assistant.switched_backend")
            ))
        }
        publishCatalog()
        settings.agentProvider = backend
        restart()
    }

    func reconcileEnablement(settings: SettingsStore) {
        let wasIdle = launchIdle
        let plan = AssistantCatalog.enablementPlan(enablementFacts())
        catalogSlots = plan.slots
        launchIdle = plan.target == .idle
        catalogGeneration &+= 1
        stopProbes(outside: plan.enabled)
        publishCatalog()
        switch plan.session {
        case .keep:
            if wasIdle, case .active(let id) = plan.target, id == sessionBackend, wantsConnection {
                prepare()
            } else {
                refreshCatalogProbes()
            }
        case .switchBackend(let backend):
            beginSwitch(backend: backend, model: nil, settings: settings)
        case .stop:
            restart()
        }
    }

    private func sessionIsOpen() -> Bool {
        isRunning || process != nil || prepareTask != nil || connection == .connecting || connection == .ready
    }

    private func enablementFacts() -> AssistantCatalog.EnablementFacts {
        var seeded: [String: [AssistantCatalog.Model]] = [:]
        if Self.offeredBackends().contains(where: { $0.id == AppleFMCatalog.providerID }) {
            seeded[AppleFMCatalog.providerID] = [AssistantCatalog.appleModel]
        }
        let storedDisabled = UserDefaults.standard.object(forKey: "ai.disabledBackends") == nil
            ? nil
            : UserDefaults.standard.stringArray(forKey: "ai.disabledBackends")
        return AssistantCatalog.EnablementFacts(
            offered: Self.offeredBackends(),
            storedDisabled: storedDisabled,
            normalizedProvider: sessionBackend,
            normalizedIsOnDevice: sessionBackend == AppleFMCatalog.providerID,
            sessionOpen: sessionIsOpen(),
            previousSlots: catalogSlots,
            seeded: seeded
        )
    }

    private static func offeredBackends() -> [AssistantCatalog.OfferedBackend] {
        ShippedAgentProvider.selectableCases.map {
            AssistantCatalog.OfferedBackend(id: $0.rawValue)
        }
    }

    private func stopProbes(outside enabled: [String]) {
        let removed = catalogProbes.filter { !enabled.contains($0.backend) }
        catalogProbes.removeAll { !enabled.contains($0.backend) }
        for probe in removed {
            probe.task.cancel()
            probe.process.terminate()
            try? FileManager.default.removeItem(at: probe.sessions)
        }
    }

    /// Install hooks (re)connect only when the panel already asked for a
    /// connection; otherwise the first panel appearance prepares it.
    /// `reconnect` takes the shutdown-and-prepare path for a binary update —
    /// a plain `prepare()` early-returns on `.ready` and can't pick it up.
    func resumeIfWanted(reconnect: Bool = false) {
        guard wantsConnection else { return }
        if reconnect { restart() } else { prepare() }
    }

    /// The panel's Install button: install (or join the launch-time
    /// install), then connect; a failure stays on screen with the button.
    func installRuntime() {
        connection = .connecting
        Task { @MainActor in
            do {
                try await PiRuntimeInstaller.install()
                connection = .idle
                prepare()
            } catch {
                connection = .failed(error.localizedDescription)
            }
        }
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
        let provider = Self.selectedAgentProvider()
        sessionBackend = provider
        let launching = process == nil
        if launching, provider == AppleFMCatalog.providerID {
            guard !Task.isCancelled else { return }
            spawnAppleFM(root: root)
        }
        let tools = await PiToolchain.discover()
        guard !Task.isCancelled else { return }
        catalogTools = tools
        catalogRoot = root
        refreshCatalogProbes()
        guard launching, provider != AppleFMCatalog.providerID else { return }
        if let adapter = Self.bridgeAdapterID(provider) {
            await spawnBridge(root: root, tools: tools, adapter: adapter)
            return
        }
        guard let executable = PiExecutableLocator.resolve(environment: tools.environment) else {
            connection = .piMissing
            statusMessage = nil
            return
        }
        await spawn(executable: executable, root: root, tools: tools)
    }

    static func selectedAgentProvider() -> String {
        SettingsStore.agentProviderValue(UserDefaults.standard.string(forKey: "ai.agentProvider"))
    }

    /// Adapter id for providers that launch `pitex-agent-bridge`, or nil for pi.
    static func bridgeAdapterID(_ provider: String) -> String? {
        ShippedAgentProvider(rawValue: provider)?.bridgeAdapterID
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
        let environment = Self.childEnvironment(executable: launch.executable, environment: tools.environment)
        startAgent(launch: launch, root: root, environment: environment)
    }

    /// CLI provider: the same Bun/Node runtime, pointed at the bundled
    /// bridge instead of pi. The bridge owns the selected CLI and that
    /// CLI's own login.
    private func spawnBridge(root: URL, tools: PiToolchain, adapter: String) async {
        intentionalStop = false
        connection = .connecting
        let sessions = Self.sessionDirectory(for: root)
        try? FileManager.default.createDirectory(at: sessions, withIntermediateDirectories: true)
        sessionDirectory = sessions
        guard let script = AgentBridge.scriptURL() else {
            connection = .failed(Self.bridgeMissingMessage(adapter))
            return
        }
        let launch: (executable: URL, arguments: [String])
        do {
            launch = try tools.launch(script, arguments: [])
        } catch {
            connection = .failed(error.localizedDescription)
            return
        }
        var environment = Self.childEnvironment(executable: launch.executable, environment: tools.environment)
        environment["PITEX_SESSION_DIR"] = sessions.path
        environment["PITEX_DEVELOPER_INSTRUCTIONS"] = Self.systemPromptSupplement
        environment["PITEX_CLI_ADAPTER"] = adapter
        startAgent(launch: launch, root: root, environment: environment)
    }

    private static func bridgeMissingMessage(_ adapter: String) -> String {
        if adapter == "claude" {
            return "The Claude Code bridge is not installed with this Pitex build."
        }
        if adapter == "grok" {
            return "The Grok CLI bridge is not installed with this Pitex build."
        }
        if adapter == "agy" {
            return "The Antigravity CLI bridge is not installed with this Pitex build."
        }
        return "The Codex bridge is not installed with this Pitex build."
    }

    private func spawnAppleFM(root: URL) {
        intentionalStop = false
        switch AppleFMRuntime.availability() {
        case .unavailable(let reason):
            connection = .failed(reason.message)
            statusMessage = nil
        case .available(let contextTokens):
            connection = .connecting
            let session = AppleFMSession(
                workingDirectory: root,
                contextTokens: contextTokens,
                instructions: Self.systemPromptSupplement
            )
            connect(session)
        }
    }

    private func startAgent(
        launch: (executable: URL, arguments: [String]),
        root: URL,
        environment: [String: String]
    ) {
        let token = processEpoch.beginLaunch()
        attachedToken = token
        let agent = PiAgentProcess(
            executableURL: launch.executable,
            workingDirectory: root,
            arguments: launch.arguments,
            environment: environment
        )
        do {
            try agent.start { [weak self] in
                Task { @MainActor in self?.processDidExit(token) }
            }
        } catch {
            processEpoch.retire()
            connection = .failed(error.localizedDescription)
            return
        }
        connect(agent)
    }

    private func connect(_ session: any AgentSession) {
        if !processEpoch.isCurrent(attachedToken) {
            attachedToken = processEpoch.beginLaunch()
        }
        let token = attachedToken
        process = session
        connection = .ready
        if pendingRetry {
            pendingRetry = false
            if let lastSent { dispatch(lastSent.text, images: lastSent.images, steer: false) }
        }
        eventTask = Task { [weak self] in
            for await event in session.events {
                guard let self, self.processEpoch.isCurrent(token) else { return }
                self.handle(event)
            }
        }
        sendModelSettings(.getState)
        sendSilently(.getAvailableModels)
        sendSilently(.getSessionStats)
        sendSilently(.getCommands)
        SubscriptionUsageStore.shared.refresh(provider: currentModel?.provider)
        if let session = pendingResume {
            pendingResume = nil
            resume(session)
        }
    }

    private func processDidExit(_ token: UInt64) {
        guard processEpoch.isCurrent(token) else { return }
        flushDeltas()
        abandonDrain()
        // Retry only resends a message the exit cut off (a turn running or not yet started).
        let unanswered = isRunning || transcript.last?.role == .user
        process = nil
        isRunning = false
        modelSettingsRequestID = nil
        thinkingLevels = []
        isUpdatingModelSettings = false
        approvalQueue.clear()
        pendingApproval = nil
        permissionMode = nil
        todos = []
        forkRequest = nil
        rewindOffer = nil
        // A message cut off by the exit gets its final (non-streaming) render.
        finalizeEntries()
        guard !intentionalStop else { return }
        connection = .failed("The agent process exited unexpectedly.")
        if !transcript.isEmpty {
            appendTurnError(.crashed, text: "The agent process exited unexpectedly.", retry: unanswered)
        }
    }

    /// Resends the last message (with its images) through the normal send path. After a
    /// crash the process starts again first and the message follows once it is up.
    func retry() {
        guard let lastSent, !isRunning else { return }
        if case .failed = connection {
            // shortcut: the restarted process opens a new session, so the retried message
            // goes without the earlier context; resuming the crashed session would keep it.
            clearRetryOffers()
            pendingRetry = true
            restart()
            return
        }
        dispatch(lastSent.text, images: lastSent.images, steer: false)
    }

    private func appendTurnError(_ code: AgentErrorCode, text: String, retry: Bool = true) {
        clearRetryOffers()
        let canCompact = commands.contains { $0.name == "compact" }
        transcript.append(AgentTranscriptEntry(
            role: .notice, title: "Assistant", text: text, status: .failed,
            turnError: AgentTurnError(code: code, provider: sessionBackend, canCompact: canCompact, retry: retry && lastSent != nil)
        ))
    }

    /// Only the newest failure offers Retry, and none once another message went out.
    private func clearRetryOffers() {
        for index in transcript.indices where transcript[index].turnError?.retry == true {
            transcript[index].turnError?.retry = false
        }
    }

    // MARK: - Send / stop / session

    /// Return (`now` false) or Cmd+Return (`now` true) with the draft's images:
    /// send, steer the running turn, or queue (`AgentComposer.route`).
    func send(prompt: String, now: Bool = false) {
        let trimmed = prompt.trimmingCharacters(in: .whitespacesAndNewlines)
        // Images alone are a message too.
        guard !trimmed.isEmpty || !draftImages.isEmpty, !isBranching else { return }
        switch connection {
        case .piMissing:
            statusMessage = String(localized: "assistant.pi_missing")
            return
        default:
            break
        }
        // pi's own /resume is an interactive TUI picker; here it opens the
        // panel's history list instead. /clear is a new chat, so the panel and
        // the backend's context agree. Either way the attached images stay.
        if trimmed == "/resume" {
            showSessionHistory()
            return
        }
        if trimmed == "/clear" {
            newSession()
            return
        }
        let images = draftImages
        draftImages = []
        queue.paused = false
        // A drained message not yet accepted counts as running: a second plain prompt would be refused.
        let busy = isRunning || drainRequest != nil
        switch AgentComposer.route(running: busy, approvalPending: pendingApproval != nil, now: now, canSteer: capabilities.steer && isRunning) {
        case .send: dispatch(trimmed, images: images, steer: false)
        case .steer: dispatch(trimmed, images: images, steer: true)
        case .queue: queue.enqueue(trimmed, images: images)
        }
    }

    /// "Send now" on a queued row is offered: steer into a running turn, or send
    /// when idle (resumes a queue paused by Stop or a refused send).
    var canSendQueuedNow: Bool {
        pendingApproval == nil && (isRunning ? capabilities.steer : drainRequest == nil)
    }

    func sendQueuedNow(_ id: Int) {
        guard canSendQueuedNow, let item = queue.remove(id) else { return }
        queue.paused = false
        dispatch(item.text, images: item.images, steer: isRunning)
    }

    /// Edit pulls the row out; the panel puts its text back in the composer.
    func takeQueued(_ id: Int) -> AgentMessageQueue.Item? {
        guard let item = queue.remove(id) else { return nil }
        draftImages += item.images.prefix(max(0, AgentComposer.maxImages - draftImages.count))
        return item
    }

    func deleteQueued(_ id: Int) {
        _ = queue.remove(id)
    }

    func clearQueue() {
        queue.clear()
        drainRequest = nil
    }

    /// A pasted, dropped or picked image: a chip, or a status line saying why not.
    func attachImage(name: String, mimeType: String, data: Data) {
        guard capabilities.images else {
            statusMessage = String(localized: "assistant.image.not_supported_backend")
            return
        }
        do {
            draftImages.append(try AgentImage(name: name, mimeType: mimeType, data: data, attached: draftImages.count))
            statusMessage = nil
        } catch {
            let key = (error as? AgentImage.Problem)?.key ?? "assistant.image.unsupported"
            statusMessage = String(localized: String.LocalizationValue(key))
        }
    }

    /// Picked or dropped files: images become chips when the backend takes them;
    /// every other file is inserted as a project-relative path.
    func attach(urls: [URL]) {
        var paths: [String] = []
        let root = contextProvider().projectRoot
        for url in urls {
            if capabilities.images, let mime = AgentComposer.imageMIMEType(path: url.path) {
                // Refuse an oversized file before reading it all on the main actor.
                if let size = try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize, size > AgentComposer.maxImageBytes {
                    statusMessage = String(localized: String.LocalizationValue(AgentImage.Problem.tooLarge.key))
                    continue
                }
                do {
                    attachImage(name: url.lastPathComponent, mimeType: mime, data: try Data(contentsOf: url))
                } catch {
                    statusMessage = error.localizedDescription
                }
            } else if let root, let relative = try? WorkspaceModel.relativePath(for: url, root: root).rawValue {
                paths.append(relative)
            } else {
                paths.append(url.path)
            }
        }
        if !paths.isEmpty { insertIntoComposer(paths.joined(separator: " ") + " ") }
    }

    /// Project files for `@` mentions, relative to the root.
    func mentionFiles() -> [String] {
        contextProvider().projectFiles
    }

    /// One queued message per agent_end. A refused send goes back to the front
    /// (`handleResponse`) and the queue waits for the user.
    private func drainQueue() {
        guard pendingApproval == nil, drainRequest == nil, let item = queue.takeNext() else { return }
        let id = UUID().uuidString
        drainRequest = (id, item.id)
        dispatch(item.text, images: item.images, steer: false, id: id)
    }

    /// Persist dirty sessions first, then envelope + prompt (or steer).
    private func dispatch(_ trimmed: String, images: [AgentImage], steer: Bool, id: String? = nil) {
        switch connection {
        case .piMissing:
            statusMessage = "Pitex Agent is not installed yet — it installs automatically on launch; see Settings → AI."
            refuseDrain(id)
            return
        default:
            break
        }
        // The fork or rewind would close the query this message went to.
        guard !isBranching else {
            refuseDrain(id)
            return
        }
        flushDeltas()
        // A drained message gets its bubble once the backend accepts it (`handleResponse`),
        // so a refused one, back at the front of the queue, does not leave a duplicate.
        if id == nil {
            transcript.append(AgentTranscriptEntry(role: .user, title: "You", text: trimmed, images: images))
        }
        if !steer {
            lastSent = AgentRetryPayload(text: trimmed, images: images)
            // A new message replaces a crash Retry still waiting for the relaunch.
            pendingRetry = false
            clearRetryOffers()
        }
        statusMessage = nil
        Task {
            if let problem = await persistDirtySessions() {
                flushDeltas()
                transcript.append(AgentTranscriptEntry(
                    role: .notice, title: "Assistant",
                    text: problem, status: .failed
                ))
                refuseDrain(id)
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
                if case .failed(let message) = connection, !message.isEmpty {
                    statusMessage = message
                } else {
                    statusMessage = "The agent is not running."
                }
                refuseDrain(id)
                return
            }
            // Slash commands go raw so pi expands /skill:name and extension
            // commands itself; the editor-context envelope is for prose.
            let message = trimmed.hasPrefix("/") ? trimmed : await envelope(for: trimmed)
            do {
                if steer {
                    try process.send(.steer(message: message, images: images))
                } else {
                    // A drained message may meet pi still settling (retry, compaction):
                    // followUp waits for it there, and the bridges ignore it when idle.
                    let behavior = isRunning || id != nil ? "followUp" : nil
                    try process.send(.prompt(message: message, images: images, streamingBehavior: behavior), id: id)
                }
            } catch {
                statusMessage = "The prompt could not be sent: \(error.localizedDescription)"
                refuseDrain(id)
            }
        }
    }

    /// The process went away with a queued message in flight: it goes back to the front.
    private func abandonDrain() {
        guard let drain = drainRequest else { return }
        drainRequest = nil
        queue.failed(drain.item)
    }

    /// A drained message that never went out goes back to the front of the queue.
    private func refuseDrain(_ id: String?) {
        guard let id, let drain = drainRequest, drain.id == id else { return }
        drainRequest = nil
        queue.failed(drain.item)
    }

    /// Stop tools before a forced SSH save takes its final source snapshot.
    /// A stalled request keeps the workspace open instead of racing upload.
    func stopForRemoteClose() async -> String? {
        stop()
        let deadline = ContinuousClock.now.advanced(by: .seconds(10))
        while isRunning {
            if ContinuousClock.now >= deadline {
                return "The Assistant is still changing files. Stop it and save again before closing the SSH project."
            }
            try? await Task.sleep(for: .milliseconds(100))
        }
        // agent_end normally schedules this too; perform it here before
        // the close snapshot rather than depending on that Task's timing.
        await agentActivityDidFinish()
        return nil
    }

    func stop() {
        denyQueuedApprovals()
        // Stop keeps the queue but does not let agent_end send the next one.
        queue.paused = true
        guard isRunning, let process else { return }
        try? process.send(.abort)
    }

    func newSession() {
        guard !isBranching else { return }
        denyQueuedApprovals()
        guard let process, process.isRunning else { return }
        clearTranscript()
        try? process.send(.newSession)
    }

    /// Answers card `id` with a button, question answers, or feedback. A card
    /// the bridge already settled is ignored. The next queued request, if
    /// any, takes its place.
    func answerApproval(id: String, _ answer: AgentApprovalAnswer) {
        guard let pending = approvalQueue.dequeueHead(id: id) else { return }
        showApprovalHead()
        try? process?.send(.approvalResponse(id: pending.id, answer: answer))
    }

    /// Picks a permission mode the bridge listed. The bridge answers with
    /// `mode_changed`; a refused pick re-reads the state.
    func selectPermissionMode(_ id: String) {
        guard var mode = permissionMode, mode.id != id, mode.options.contains(where: { $0.id == id }) else { return }
        mode.id = id
        permissionMode = mode
        try? process?.send(.setMode(modeID: id))
    }

    /// Stop and new-session must unblock every Codex request, including
    /// ones the panel has not shown yet.
    private func denyQueuedApprovals() {
        let queued = approvalQueue.drain()
        pendingApproval = nil
        for item in queued {
            try? process?.send(.approvalResponse(id: item.id, answer: AgentApprovalAnswer(decision: "deny")))
        }
    }

    private func showApprovalHead() {
        guard let head = approvalQueue.head else {
            pendingApproval = nil
            return
        }
        pendingApproval = head
    }

    private func clearTranscript() {
        flushDeltas()
        // A new or another chat: the queue belonged to this one.
        queue.clear()
        drainRequest = nil
        transcript = []
        todos = []
        toolIndexByCallID = [:]
        assistantEntryIndex = nil
        thinkingEntryIndex = nil
        thinkingStarted = nil
        liveModelName = nil
        // Retry belongs to that chat.
        lastSent = nil
        pendingRetry = false
        // The meter belongs to that chat; the next usage_update or stats reply refills it.
        usage = AgentUsage()
        sessionStats = nil
        rewindOffer = nil
        turnStarted = nil
        droppedPrompts = 0
    }

    // MARK: - Turns, fork and rewind

    /// `agent_end`: the turn's prompt row learns how long the turn ran.
    private func noteTurnDuration() {
        guard let started = turnStarted else { return }
        turnStarted = nil
        if let index = transcript.lastIndex(where: { $0.role == .user }) {
            transcript[index].workedSeconds = Int(Date().timeIntervalSince(started))
        }
    }

    /// The transcript as `AgentTurnView` items: subagent calls nested, finished turns folded.
    var turnItems: [AgentTurnView.Item] {
        AgentTurnView.items(transcript.map(Self.turnRow), running: isRunning)
    }

    private static func turnRow(_ entry: AgentTranscriptEntry) -> AgentTurnView.Row {
        let role: AgentTurnView.Row.Role
        switch entry.role {
        case .user: role = .user
        case .assistant: role = .assistant
        case .thinking: role = .thinking
        case .tool: role = .tool
        case .notice: role = .notice
        case .activity: role = .activity
        }
        let kind: AgentToolDetail.Kind? = entry.tool?.kind ?? (entry.role == .tool ? .other : nil)
        return AgentTurnView.Row(role: role, toolKind: kind, path: entry.tool?.path,
                                 callID: entry.callID, parentCallID: entry.tool?.parentCallID,
                                 workedSeconds: entry.workedSeconds)
    }

    /// A fork or a confirmed rewind is out: sending, switching chats and branching again wait.
    var isBranching: Bool { forkRequest != nil || rewindOffer?.isRewinding == true }

    /// Prompts in this chat, counting rows trimmed off the top.
    private var promptCount: Int { droppedPrompts + transcript.filter { $0.role == .user }.count }

    private func turnCut(_ index: Int) -> (keep: Int, draft: String?)? {
        guard let keep = AgentTurnView.turnCut(roles: transcript.map { Self.turnRow($0).role }, index: index, dropped: droppedPrompts) else { return nil }
        return (keep, transcript[index].role == .user ? transcript[index].text : nil)
    }

    /// The ⋯ menu for row `index`, or nil for rows without one.
    func messageMenu(_ index: Int) -> AgentMessageMenu? {
        guard connection == .ready, let cut = turnCut(index) else { return nil }
        let busy = String(localized: "assistant.message.busy")
        let blocked = isRunning || isBranching
        let rewindBlocked: String?
        if blocked {
            rewindBlocked = busy
        } else if !capabilities.rewindConversation {
            rewindBlocked = String(format: String(localized: "assistant.message.rewind_unavailable"), AgentErrorCode.cliName(provider: sessionBackend))
        } else if cut.keep >= promptCount {
            rewindBlocked = String(localized: "assistant.message.rewind_last")
        } else {
            rewindBlocked = nil
        }
        return AgentMessageMenu(
            forkBlocked: blocked ? busy : nil,
            rewindBlocked: rewindBlocked,
            fork: { [weak self] in self?.fork(from: index) },
            rewind: { [weak self] in self?.requestRewind(index) }
        )
    }

    /// "Fork from here": the bridge copies the first turns into a new chat when it can;
    /// otherwise a new chat starts with those turns quoted into the draft.
    func fork(from index: Int) {
        guard !isRunning, !isBranching, let cut = turnCut(index), let process, process.isRunning else { return }
        if cut.keep > 0, capabilities.fork {
            let id = UUID().uuidString
            if (try? process.send(.forkSession(keepTurns: cut.keep, turns: promptCount), id: id)) != nil {
                forkRequest = (id, cut.draft)
            }
            return
        }
        let rows = transcript.map { (role: Self.turnRow($0).role, text: $0.text) }
        let quoted = AgentTurnView.quote(
            rows, keep: max(0, cut.keep - droppedPrompts),
            header: String(localized: "assistant.fork.quote_header"),
            you: String(localized: "assistant.fork.quote_you"),
            assistant: String(localized: "assistant.fork.quote_assistant")
        )
        newSession()
        composerReplacement = [quoted, cut.draft ?? ""].filter { !$0.isEmpty }.joined(separator: "\n\n")
        if cut.keep > 0 {
            transcript.append(AgentTranscriptEntry(
                role: .notice, title: "Assistant",
                text: String(format: String(localized: "assistant.fork.quoted"), AgentErrorCode.cliName(provider: sessionBackend))
            ))
        }
    }

    /// "Rewind to here": opens the confirmation after a dry run lists the files a file rewind would restore.
    func requestRewind(_ index: Int) {
        guard !isRunning, !isBranching, capabilities.rewindConversation, let cut = turnCut(index), cut.keep < promptCount else { return }
        var offer = AgentRewindOffer(keepTurns: cut.keep, draft: cut.draft)
        if capabilities.rewindFiles, let process {
            let id = UUID().uuidString
            if (try? process.send(.rewind(keepTurns: cut.keep, turns: promptCount, conversation: false, files: true, dryRun: true), id: id)) != nil {
                offer.requestID = id
            }
        }
        if offer.requestID == nil {
            offer.files = []
            offer.filesNote = String(localized: "assistant.rewind.files_unsupported")
        }
        rewindOffer = offer
    }

    /// The confirmation's buttons: the conversation, and the files when `files`.
    func confirmRewind(files: Bool) {
        // One request at a time: a reply to the dry run must not be read as the rewind's.
        guard !isRunning, var offer = rewindOffer, offer.requestID == nil, let process else { return }
        let id = UUID().uuidString
        if (try? process.send(.rewind(keepTurns: offer.keepTurns, turns: promptCount, conversation: true, files: files, dryRun: false), id: id)) != nil {
            offer.requestID = id
            rewindOffer = offer
        }
    }

    /// The confirmed rewind cannot be called back: its reply reloads the chat and the files.
    func cancelRewind() {
        guard rewindOffer?.isRewinding != true else { return }
        rewindOffer = nil
    }

    private func handleForkResponse(_ event: PiRPCEvent) {
        guard let request = forkRequest, event.string("id") == request.id else { return }
        forkRequest = nil
        guard event.responseSucceeded else {
            transcript.append(AgentTranscriptEntry(
                role: .notice, title: "Assistant",
                text: String(format: String(localized: "assistant.fork.failed"), event.responseError ?? ""), status: .failed
            ))
            return
        }
        clearTranscript()
        noticeAfterReload = String(localized: "assistant.fork.done")
        requestMessages()
        sendModelSettings(.getState)
        composerReplacement = request.draft
    }

    private func handleRewindResponse(_ event: PiRPCEvent) {
        guard var offer = rewindOffer, offer.requestID != nil, event.string("id") == offer.requestID else { return }
        offer.requestID = nil
        let files = (event.responseData?["filesChanged"] as? [String]) ?? []
        if offer.files == nil {
            // The dry run: the list, or why files cannot be restored.
            if event.responseSucceeded {
                offer.files = files
            } else {
                offer.files = []
                offer.filesNote = String(format: String(localized: "assistant.rewind.files_note"), event.responseError ?? "")
            }
            rewindOffer = offer
            return
        }
        guard event.responseSucceeded else {
            rewindOffer = nil
            transcript.append(AgentTranscriptEntry(
                role: .notice, title: "Assistant",
                text: String(format: String(localized: "assistant.rewind.failed"), event.responseError ?? ""), status: .failed
            ))
            return
        }
        let draft = offer.draft
        clearTranscript()
        let root = projectRoot()
        let shown = files.map { path -> String in
            var file = AgentToolDetail(kind: .other)
            file.path = path
            return file.displayPath(root: root)
        }
        noticeAfterReload = shown.isEmpty
            ? String(localized: "assistant.rewind.done")
            : String(format: String(localized: "assistant.rewind.done_files"), shown.joined(separator: ", "))
        requestMessages()
        sendModelSettings(.getState)
        composerReplacement = draft
        // Restored files: the workspace reloads them like an agent edit.
        Task { await agentActivityDidFinish() }
    }

    /// A new todo snapshot: one activity row per change, then the checklist follows it.
    private func noteTodos(_ items: [AgentTodoItem]) {
        guard items != todos else { return }
        let allPending = items.allSatisfy { $0.status == .pending }
        for activity in AgentTodoList.activities(from: todos, to: items) {
            // A list built one TaskCreate at a time reads as one "created" row.
            if case .added = activity, allPending, let last = transcript.indices.last,
               transcript[last].role == .activity, transcript[last].title == AgentTodoActivity.created(0).key {
                transcript[last].text = String(items.count)
                continue
            }
            transcript.append(AgentTranscriptEntry(role: .activity, title: activity.key, text: activity.argument))
        }
        todos = items
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

    /// Opens the history list. A bridge backend first mirrors sessions the
    /// CLI started elsewhere (`refresh_sessions`); the list opens on its reply.
    func showSessionHistory() {
        guard !isRunning else { return }
        if Self.bridgeAdapterID(sessionBackend) != nil, let process, process.isRunning {
            let id = UUID().uuidString
            refreshRequestID = id
            if (try? process.send(.refreshSessions, id: id)) != nil { return }
        }
        listPastSessions()
    }

    private func listPastSessions() {
        refreshRequestID = nil
        let files = sessionDirectory.flatMap {
            try? FileManager.default.contentsOfDirectory(
                at: $0, includingPropertiesForKeys: [.contentModificationDateKey]
            )
        } ?? []
        pastSessions = files
            .filter { $0.pathExtension == "jsonl" }
            .compactMap(Self.sessionSummary)
            // Mirrors of hidden backends (direct Codex and Grok) cannot be opened from any setting.
            .filter { ShippedAgentProvider(rawValue: $0.source)?.isSelectable ?? true }
            .sorted { $0.date > $1.date }
        showingSessionHistory = true
    }

    /// A history row was picked. Another backend's conversation switches the
    /// backend first and opens once it is up; a turned-off backend says so.
    func open(_ session: AgentSessionSummary, settings: SettingsStore) {
        showingSessionHistory = false
        guard !isRunning, !isBranching else { return }
        if session.source == sessionBackend {
            resume(session)
        } else if catalogSlots.contains(where: { $0.backend == session.source }) {
            beginSwitch(backend: session.source, model: nil, settings: settings)
            // After the switch: the new process connects later, from `prepare`'s task.
            pendingResume = session
        } else {
            transcript.append(AgentTranscriptEntry(
                role: .notice,
                title: "Assistant",
                text: String(format: String(localized: "assistant.history_backend_off"),
                             AgentSessionHistory.sourceLabel(session.source))
            ))
        }
    }

    /// Loads a saved session into pi, then redraws the transcript from its
    /// messages (`switch_session` → `get_messages`).
    func resume(_ session: AgentSessionSummary) {
        showingSessionHistory = false
        guard let process, process.isRunning, !isRunning, !isBranching else { return }
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
                // The model that wrote it, when the backend saved one (Claude), not the picker's alias.
                let title = (message["modelName"] as? String).flatMap { $0.isEmpty ? nil : "Pitex Agent · \($0)" } ?? assistantTitle
                for block in message["content"] as? [[String: Any]] ?? [] {
                    switch block["type"] as? String {
                    case "thinking":
                        if let text = block["thinking"] as? String, !text.isEmpty {
                            entries.append(AgentTranscriptEntry(role: .thinking, title: "Thinking", text: text))
                        }
                    case "text":
                        if let text = block["text"] as? String, !text.isEmpty {
                            entries.append(AgentTranscriptEntry(role: .assistant, title: title, text: text))
                        }
                    case "toolCall":
                        let name = block["name"] as? String ?? "tool"
                        entries.append(AgentTranscriptEntry(
                            role: .tool, title: name, text: "",
                            detail: toolSummary(name: name, args: block["arguments"] as? [String: Any]),
                            callID: block["id"] as? String
                        ))
                        if let id = block["id"] as? String { toolRows[id] = entries.count - 1 }
                    default:
                        break
                    }
                }
            case "toolResult":
                guard let id = message["toolCallId"] as? String, let index = toolRows[id] else { break }
                if message["isError"] as? Bool == true { entries[index].status = .failed }
                // Bridge backends keep the typed card; the args summary goes, as live.
                if let status = Self.entryStatus(AgentToolDetail.status(message["status"] as? String)) {
                    entries[index].status = status
                }
                if var tool = AgentToolDetail(message["detail"] as? [String: Any]) {
                    tool.parentCallID = message["parentToolCallId"] as? String
                    if tool.kind != .other, tool.kind != .subAgent { entries[index].detail = "" }
                    entries[index].tool = tool
                }
                if let text = toolResultText(message) {
                    entries[index].detail = bounded(text, limit: 4_000)
                }
            default:
                break
            }
        }
        return entries
    }

    private struct CatalogProbe {
        var backend: String
        var generation: UInt64
        var process: PiAgentProcess
        var sessions: URL
        var task: Task<Void, Never>
    }

    private func installCatalogSlots() {
        let plan = AssistantCatalog.enablementPlan(enablementFacts())
        catalogSlots = plan.slots
        launchIdle = plan.target == .idle
        publishCatalog()
    }

    private func catalogFacts() -> AssistantCatalog.Facts {
        AssistantCatalog.Facts(
            slots: catalogSlots,
            activeBackend: sessionBackend,
            current: currentModel.map(Self.catalogModel(from:)),
            generation: catalogGeneration,
            acceptsPicks: !isUpdatingModelSettings
        )
    }

    private func publishCatalog() {
        catalogMenu = AssistantCatalog.menu(facts: catalogFacts(), title: backendTitle)
        let notes: [String] = catalogSlots.compactMap { slot in
            if case .ready = slot.state { return nil }
            guard let reason = catalogReasons[slot.backend] else { return nil }
            return "\(backendTitle(slot.backend)): \(reason)"
        }
        if notes != catalogNotes { catalogNotes = notes }
    }

    private func backendTitle(_ backend: String) -> String {
        switch backend {
        case ShippedAgentProvider.pi.rawValue:
            String(localized: "settings.ai.backend_pi")
        case ShippedAgentProvider.claude.rawValue:
            String(localized: "settings.ai.backend_claude")
        case ShippedAgentProvider.agy.rawValue:
            String(localized: "settings.ai.backend_agy")
        case ShippedAgentProvider.appleFM.rawValue:
            String(localized: "settings.ai.backend_apple_fm")
        case ShippedAgentProvider.codex.rawValue:
            String(localized: "settings.ai.backend_codex")
        case ShippedAgentProvider.grok.rawValue:
            String(localized: "settings.ai.backend_grok")
        default:
            backend
        }
    }

    private func noteLiveModels() {
        let listed = models.map(Self.catalogModel(from:))
        let state: AssistantCatalog.SlotState = listed.isEmpty ? .unavailable : .ready(listed)
        if !listed.isEmpty { catalogReasons[sessionBackend] = nil }
        noteCatalog(backend: sessionBackend, state: state)
    }

    /// `reason` is the bridge's text (CLI missing, signed out); nil means it said nothing.
    private func noteCatalogFailure(backend: String, reason: String?) {
        catalogReasons[backend] = reason ?? String(localized: "assistant.backend_unavailable")
        noteCatalog(backend: backend, state: .unavailable)
    }

    private func noteCatalog(backend: String, state: AssistantCatalog.SlotState) {
        AssistantCatalog.note(&catalogSlots, backend: backend, state: state)
        catalogGeneration &+= 1
        publishCatalog()
    }

    private func retryCatalogCommit() {
        if catalogCommitPending { commitSwitchTicket() }
    }

    private func commitSwitchTicket() {
        switch AssistantCatalog.commit(
            ticket: switchTicket,
            activeBackend: sessionBackend,
            models: models.map(Self.catalogModel(from:))
        ) {
        case .hold:
            break
        case .clear:
            catalogCommitPending = false
            switchTicket = nil
            currentModel = nil
            publishCatalog()
            sendModelSettings(.getState)
        case .select(let model):
            if isUpdatingModelSettings {
                catalogCommitPending = true
                return
            }
            catalogCommitPending = false
            switchTicket = nil
            publishCatalog()
            if let descriptor = Self.descriptor(from: model) {
                selectModel(descriptor)
            }
        }
    }

    private func refreshCatalogProbes() {
        guard catalogTools != nil else { return }
        let root = catalogRoot ?? contextProvider().projectRoot
        guard let root else { return }
        catalogRoot = root
        for slot in catalogSlots where slot.backend != sessionBackend && !catalogProbeRunning(slot.backend) {
            if case .pending = slot.state {
                startCatalogProbe(backend: slot.backend, root: root)
            }
        }
    }

    private func catalogProbeRunning(_ backend: String) -> Bool {
        catalogProbes.contains { $0.backend == backend && $0.generation == probeGeneration }
    }

    /// A short-lived process answers `get_available_models` and exits.
    /// Its session directory is disposable. The CLI signs in on its own.
    private func startCatalogProbe(backend: String, root: URL) {
        guard let tools = catalogTools else { return }
        let sessions = FileManager.default.temporaryDirectory
            .appendingPathComponent("pitex-catalog-\(backend)-\(UUID().uuidString)", isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: sessions, withIntermediateDirectories: true)
        } catch {
            noteCatalogFailure(backend: backend, reason: nil)
            return
        }
        guard let launch = catalogProbeLaunch(backend: backend, sessions: sessions, tools: tools) else {
            try? FileManager.default.removeItem(at: sessions)
            noteCatalogFailure(backend: backend, reason: nil)
            return
        }
        let process = PiAgentProcess(
            executableURL: launch.executable,
            workingDirectory: root,
            arguments: launch.arguments,
            environment: launch.environment
        )
        do {
            try process.start(onExit: {})
            try process.send(.getAvailableModels)
        } catch {
            process.terminate()
            try? FileManager.default.removeItem(at: sessions)
            noteCatalogFailure(backend: backend, reason: nil)
            return
        }
        let generation = probeGeneration
        let task = Task { @MainActor [weak self] in
            let timeout = Task { @MainActor in
                do {
                    try await Task.sleep(for: .seconds(15))
                    process.terminate()
                } catch {
                    // The catalog line arrived, or the conversation shut down.
                }
            }
            var found: [AssistantCatalog.Model]?
            var reason: String?
            for await event in process.events {
                if Task.isCancelled {
                    timeout.cancel()
                    return
                }
                if event.type == "extension_error", reason == nil {
                    reason = event.string("error")
                }
                if let models = Self.catalogModels(from: event) {
                    found = models
                    // An empty but successful reply was not refused: a notice
                    // printed before it (agy's permission warning) is not the reason.
                    reason = event.responseSucceeded ? nil : (event.responseError ?? reason)
                    break
                }
            }
            timeout.cancel()
            process.terminate()
            guard !Task.isCancelled else { return }
            self?.completeCatalogProbe(
                backend: backend,
                generation: generation,
                sessions: sessions,
                models: found,
                reason: reason
            )
        }
        catalogProbes.append(CatalogProbe(
            backend: backend,
            generation: generation,
            process: process,
            sessions: sessions,
            task: task
        ))
    }

    private func catalogProbeLaunch(
        backend: String,
        sessions: URL,
        tools: PiToolchain
    ) -> (executable: URL, arguments: [String], environment: [String: String])? {
        if backend == AppleFMCatalog.providerID { return nil }
        if let adapter = Self.bridgeAdapterID(backend) {
            guard let script = AgentBridge.scriptURL(),
                  let launch = try? tools.launch(script, arguments: []) else { return nil }
            var environment = Self.childEnvironment(executable: launch.executable, environment: tools.environment)
            environment["PITEX_SESSION_DIR"] = sessions.path
            environment["PITEX_DEVELOPER_INSTRUCTIONS"] = Self.systemPromptSupplement
            environment["PITEX_CLI_ADAPTER"] = adapter
            environment["PITEX_CATALOG_PROBE"] = "1"
            return (launch.executable, launch.arguments, environment)
        }
        guard let executable = PiExecutableLocator.resolve(environment: tools.environment) else { return nil }
        let arguments = [
            "--mode", "rpc",
            "--session-dir", sessions.path,
            "--append-system-prompt", Self.systemPromptSupplement,
        ]
        guard let launch = try? tools.launch(executable, arguments: arguments) else { return nil }
        var environment = Self.childEnvironment(executable: launch.executable, environment: tools.environment)
        environment["PITEX_CATALOG_PROBE"] = "1"
        return (launch.executable, launch.arguments, environment)
    }

    private func completeCatalogProbe(
        backend: String,
        generation: UInt64,
        sessions: URL,
        models: [AssistantCatalog.Model]?,
        reason: String?
    ) {
        catalogProbes.removeAll { probe in
            guard probe.backend == backend, probe.generation == generation else { return false }
            probe.process.terminate()
            try? FileManager.default.removeItem(at: probe.sessions)
            return true
        }
        guard generation == probeGeneration else {
            try? FileManager.default.removeItem(at: sessions)
            return
        }
        if let models, !models.isEmpty {
            catalogReasons[backend] = nil
            noteCatalog(backend: backend, state: .ready(models))
        } else {
            noteCatalogFailure(backend: backend, reason: reason)
        }
    }

    private func stopCatalogProbes() {
        probeGeneration &+= 1
        let stopped = catalogProbes
        catalogProbes = []
        for probe in stopped {
            probe.task.cancel()
            probe.process.terminate()
            try? FileManager.default.removeItem(at: probe.sessions)
        }
    }

    private static func catalogModels(from event: PiRPCEvent) -> [AssistantCatalog.Model]? {
        guard event.isResponse, event.responseCommand == "get_available_models" else { return nil }
        guard event.responseSucceeded else { return [] }
        let raw = event.responseData?["models"] as? [Any] ?? []
        return raw.compactMap(PiModelDescriptor.init).map(catalogModel(from:))
    }

    private static func catalogModel(from model: PiModelDescriptor) -> AssistantCatalog.Model {
        AssistantCatalog.Model(
            id: model.id,
            provider: model.provider,
            name: model.name,
            contextWindow: model.contextWindow
        )
    }

    private static func descriptor(from model: AssistantCatalog.Model) -> PiModelDescriptor? {
        var object: [String: Any] = [
            "id": model.id,
            "provider": model.provider,
            "name": model.name,
        ]
        if let contextWindow = model.contextWindow {
            object["contextWindow"] = contextWindow
        }
        return PiModelDescriptor(object)
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
            if turnStarted == nil { turnStarted = Date() }
        case "agent_end":
            noteTurnDuration()
            isRunning = false
            finalizeEntries()
            noteTurnEnd(event)
            sendSilently(.getSessionStats)
            SubscriptionUsageStore.shared.refresh(provider: currentModel?.provider)
            Task { await agentActivityDidFinish() }
            drainQueue()
        case "message_start":
            handleMessageStart(event)
        case "usage_update":
            usage.merge(event.object)
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
        case "approval_request":
            approvalQueue.enqueue(AgentApprovalItem(event: event.object))
            showApprovalHead()
        case "approval_resolved":
            // The bridge settled it (abort, turn end, exit): drop the card.
            approvalQueue.remove(id: event.string("id") ?? "")
            showApprovalHead()
        case "mode_changed":
            permissionMode = AgentPermissionMode(event.object["mode"])
        case "todo_update":
            noteTodos(AgentTodoItem.list(event.object["items"]))
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
        if event.responseCommand == "prompt", let drain = drainRequest, event.string("id") == drain.id {
            drainRequest = nil
            if event.responseSucceeded {
                if let item = queue.items.first(where: { $0.id == drain.item }) {
                    flushDeltas()
                    transcript.append(AgentTranscriptEntry(role: .user, title: "You", text: item.text, images: item.images))
                }
                queue.sent(drain.item)
            } else {
                queue.failed(drain.item)
            }
        }
        if event.responseCommand == "fork_session" {
            handleForkResponse(event)
            return
        }
        if event.responseCommand == "rewind" {
            handleRewindResponse(event)
            return
        }
        if event.responseCommand == "refresh_sessions" {
            // Success or not (a signed-out backend refuses), the list shows what is on disk now;
            // a turn started meanwhile wins, since rows cannot be opened during it.
            guard refreshRequestID != nil, event.string("id") == refreshRequestID else { return }
            if isRunning { refreshRequestID = nil } else { listPastSessions() }
            return
        }
        guard event.responseSucceeded else {
            if isModelSettingsResponse {
                modelSettingsRequestID = nil
                isUpdatingModelSettings = false
                if event.responseCommand == "set_model" || event.responseCommand == "set_thinking_level" {
                    sendModelSettings(.getState)
                } else {
                    retryCatalogCommit()
                }
            }
            let message = event.responseError ?? "The agent command failed."
            if event.responseCommand == "get_available_models" {
                noteCatalogFailure(backend: sessionBackend, reason: event.responseError)
            }
            statusMessage = message
            if event.responseCommand == "set_mode" {
                // The picker showed the refused pick; read the mode back.
                sendModelSettings(.getState)
            }
            if let command = event.responseCommand, command == "prompt" {
                transcript.append(AgentTranscriptEntry(
                    role: .notice, title: "Assistant", text: message, status: .failed
                ))
            }
            return
        }
        switch event.responseCommand {
        case "get_state":
            if switchTicket == nil {
                currentModel = event.responseData?["model"].flatMap(PiModelDescriptor.init)
            }
            publishCatalog()
            SubscriptionUsageStore.shared.refresh(provider: currentModel?.provider)
            if let level = event.responseData?["thinkingLevel"] as? String {
                thinkingLevel = level
            }
            permissionMode = AgentPermissionMode(event.responseData?["mode"])
            capabilities = AgentCapabilities(state: event.responseData ?? [:])
            sendModelSettings(.getAvailableThinkingLevels, id: modelSettingsRequestID)
        case "get_available_thinking_levels":
            modelSettingsRequestID = nil
            isUpdatingModelSettings = false
            guard let levels = event.responseData?["levels"] as? [String],
                  !levels.isEmpty, levels.allSatisfy({ !$0.isEmpty }),
                  Set(levels).count == levels.count, levels.contains(thinkingLevel) else {
                statusMessage = "The agent did not return valid reasoning levels for this model."
                retryCatalogCommit()
                return
            }
            thinkingLevels = currentModel == nil ? [] : levels
            applyPreferredModelIfReady()
            retryCatalogCommit()
        case "get_available_models":
            let raw = event.responseData?["models"] as? [Any] ?? []
            models = raw.compactMap(PiModelDescriptor.init)
            noteLiveModels()
            applyPreferredModelIfReady()
            commitSwitchTicket()
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
                requestMessages()
            }
            sendModelSettings(.getState)
        case "get_messages":
            guard event.string("id") == messagesRequestID else { return }
            let messages = event.responseData?["messages"] as? [[String: Any]] ?? []
            droppedPrompts = 0
            transcript = transcriptEntries(from: messages)
            // Prompts the history limit left out still count for Fork and Rewind.
            let prompts = messages.filter { $0["role"] as? String == "user" && !Self.userPromptText($0).isEmpty }.count
            droppedPrompts = max(0, prompts - transcript.filter { $0.role == .user }.count)
            if let notice = noticeAfterReload {
                noticeAfterReload = nil
                transcript.append(AgentTranscriptEntry(role: .notice, title: "Assistant", text: notice))
            }
            // The last list of a resumed chat; no activity rows for it.
            todos = AgentTodoItem.list(event.responseData?["todos"])
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
        if let name = message["modelName"] as? String, !name.isEmpty { liveModelName = name }
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
            settleThinking()
            textContentIndex = delta["contentIndex"] as? Int
            appendAssistantEntry()
        case "text_delta":
            settleThinking()
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
            transcript[index].text = AgentTranscriptText.finalText(streamed: transcript[index].text, messageEnd: text)
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
        settleThinking()
        guard let callID = event.string("toolCallId") else { return }
        let name = event.string("toolName") ?? "tool"
        var tool = AgentToolDetail(event.nested("detail"))
        tool?.parentCallID = event.string("parentToolCallId")
        // A typed card shows its fields in the body; untyped (pi, MCP) and
        // sub-agent cards have no such fields, so they keep the args summary.
        let untyped = tool.map { $0.kind == .other || $0.kind == .subAgent } ?? true
        let summary = untyped ? toolSummary(name: name, args: event.nested("args")) : ""
        transcript.append(AgentTranscriptEntry(
            role: .tool, title: name, text: "", detail: summary,
            status: toolStatus(event) ?? .running, tool: tool, callID: callID
        ))
        toolIndexByCallID[callID] = transcript.count - 1
    }

    private func handleToolUpdate(_ event: PiRPCEvent) {
        guard let callID = event.string("toolCallId"),
              let index = toolIndexByCallID[callID],
              transcript.indices.contains(index) else { return }
        mergeToolDetail(event, at: index)
        if let text = toolResultText(event.nested("partialResult")) {
            transcript[index].detail = bounded(text, limit: 4_000)
        }
    }

    private func handleToolEnd(_ event: PiRPCEvent) {
        guard let callID = event.string("toolCallId"),
              let index = toolIndexByCallID[callID],
              transcript.indices.contains(index) else { return }
        transcript[index].status = toolStatus(event) ?? (event.bool("isError") ? .failed : .done)
        mergeToolDetail(event, at: index)
        if let text = toolResultText(event.nested("result")) {
            transcript[index].detail = bounded(text, limit: 4_000)
        }
    }

    /// A later event's `detail` merges into the card (an untyped one never
    /// replaces a typed kind).
    private func mergeToolDetail(_ event: PiRPCEvent, at index: Int) {
        guard let incoming = AgentToolDetail(event.nested("detail")) else { return }
        if transcript[index].tool == nil {
            transcript[index].tool = incoming
        } else {
            transcript[index].tool?.merge(incoming)
        }
    }

    private func toolStatus(_ event: PiRPCEvent) -> AgentTranscriptEntry.Status? {
        Self.entryStatus(AgentToolDetail.status(event.string("status")))
    }

    /// The bridge's `status` (live events and replayed `toolResult` rows).
    private static func entryStatus(_ status: AgentToolDetail.Status?) -> AgentTranscriptEntry.Status? {
        switch status {
        case .running: return .running
        case .completed: return .done
        case .failed: return .failed
        case .canceled: return .canceled
        case nil: return nil
        }
    }

    /// The editor's open-document path for a tool card's "Open file"
    /// button and a project-file link in a message, with an optional
    /// 1-based line. Set by the workspace; only called for files inside the project.
    var openFile: (URL, Int?) -> Void = { _, _ in }
    /// The project folder, without building a whole context snapshot.
    var projectRoot: () -> URL? = { nil }

    func openToolFile(_ tool: AgentToolDetail) {
        guard let url = tool.openableURL(root: projectRoot()) else { return }
        openFile(url, nil)
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
        // The previous segment of this message is complete once a new one starts.
        if let index = assistantEntryIndex, transcript.indices.contains(index), transcript[index].status == .streaming {
            transcript[index].status = .done
        }
        transcript.append(AgentTranscriptEntry(
            role: .assistant, title: assistantTitle, text: "", status: .streaming
        ))
        assistantEntryIndex = transcript.count - 1
    }

    private func appendThinkingEntry() {
        settleThinking()
        transcript.append(AgentTranscriptEntry(
            role: .thinking, title: "Thinking", text: "", status: .streaming
        ))
        thinkingEntryIndex = transcript.count - 1
        thinkingStarted = Date()
    }

    /// Paseo: a thought is loading until the next event that is not reasoning; then it
    /// closes as "Thought for Ns".
    private func settleThinking() {
        guard let started = thinkingStarted else { return }
        thinkingStarted = nil
        guard let index = thinkingEntryIndex, transcript.indices.contains(index) else { return }
        transcript[index].status = .done
        transcript[index].thinkingSeconds = Int(Date().timeIntervalSince(started))
    }

    /// `agent_end`: a failed turn gets an error row (friendly line, raw detail, Retry);
    /// a stopped one a plain notice. pi sends no `outcome`; its errors read as `other`.
    private func noteTurnEnd(_ event: PiRPCEvent) {
        let fallback = lastAssistantError(in: event)
        if let failure = AgentTurnError.failure(agentEnd: event.object, fallback: fallback) {
            appendTurnError(failure.code, text: failure.text)
        } else if event.string("outcome") == "canceled", let text = event.string("error") ?? fallback {
            transcript.append(AgentTranscriptEntry(role: .notice, title: "Assistant", text: text))
        }
    }

    private func finalizeEntries() {
        settleThinking()
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

    /// The model that answered when the backend says (Claude's "Default" alias hides it),
    /// else the picked one.
    private var assistantTitle: String {
        (liveModelName ?? currentModel?.name).map { "Pitex Agent · \($0)" } ?? "Pitex Agent"
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

    private func requestMessages() {
        let id = UUID().uuidString
        messagesRequestID = id
        try? process?.send(.getMessages, id: id)
    }

    private func toolSummary(name: String, args: [String: Any]?) -> String {
        guard let args else { return "" }
        switch name {
        case "bash", "run_command":
            return (args["command"] as? String).map { bounded($0, limit: 2_000) } ?? ""
        case "read", "read_file":
            return args["path"] as? String ?? ""
        case "edit", "write", "write_file":
            return args["path"] as? String ?? ""
        case "ls", "find", "grep":
            return args["path"] as? String ?? (args["pattern"] as? String ?? "")
        default:
            if args.count == 1, let summary = args["summary"] as? String {
                return bounded(summary, limit: 500)
            }
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
