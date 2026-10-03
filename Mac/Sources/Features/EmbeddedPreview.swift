import BuildFeature
import Darwin
import DocumentSessionCore
import Foundation

// Embedded editing preview — the macOS client of the `pitex-preview` helper
// (TeXpresso-derived checkpointing XeTeX engine + Pitex XDV→PDF writer; see
// `PreviewEngine/PROTOCOL.md`, Linux: `pitex-shell/src/embedded_preview.rs`).
//
// The helper runs in its own process group and only writes into an owned
// per-session temporary directory; each publication is a directory it
// deletes once released. Buffer updates stream as soon as they coalesce,
// independent of the final-build slot, and never save source files. Manual
// Build keeps using the project compiler; its PDF is never replaced by a
// preview that lacks a newer edit (`EmbeddedPreviewLedger`).

/// Progress of the editing preview — separate from the final-build status.
enum EmbeddedPreviewStatus: Equatable {
    case off
    /// The newest sent source state is on screen (or the final PDF is).
    case current
    /// A newer state is being typeset, or only a partial snapshot of it
    /// is on screen.
    case updating
    /// The displayed preview has TeX errors (first one attached).
    case errors(String)
    case failed(String)
    case unavailable(String)
    case unsupportedRemote
}

struct EmbeddedPreviewPublication: Sendable {
    let seq: UInt64
    let generation: UInt64
    let complete: Bool
    let errors: UInt64
    let firstError: String?
    let pdf: URL
    /// Present only for coherent publications (every page from this pass)
    /// whose metadata sits where the SyncTeX runner reads it.
    let synctex: URL?
    /// PDF bytes read off the main actor; nil when the artifact is missing,
    /// not a PDF or outside the session directory.
    let bytes: Data?
}

enum EmbeddedPreviewEvent: Sendable {
    case ready
    case published(EmbeddedPreviewPublication)
    case failed(generation: UInt64, message: String)
    case idle(generation: UInt64, seq: UInt64)
    case error(code: String, message: String)
    /// The helper closed its output (exited or crashed).
    case exited
}

enum EmbeddedPreviewError: LocalizedError {
    case helperMissing
    case spawnFailed(Int32)

    var errorDescription: String? {
        switch self {
        case .helperMissing: ""
        case let .spawnFailed(code): String(cString: strerror(code))
        }
    }
}

/// One running `pitex-preview`: own process group, stdin/stdout pipes and a
/// private session directory. Thread-safe; teardown is idempotent.
final class EmbeddedPreviewSession: @unchecked Sendable {
    let id: UInt64
    let root: URL
    let mainRelative: String
    let directory: URL
    private let pid: pid_t
    /// `directory` in canonical spelling — the helper may report artifact
    /// paths through realpath's /private spelling rather than the one
    /// given at launch, and Foundation resolution is not POSIX-canonical.
    private let resolvedDirectory: String
    /// Helper stdin: bounded, non-blocking (`EmbeddedPreviewWriter`) — a
    /// helper that stops reading never blocks the main actor or stop.
    private let writer: EmbeddedPreviewWriter
    private let lock = NSLock()
    private var shutDown = false
    private var reaped = false

    private init(id: UInt64, root: URL, mainRelative: String, directory: URL, pid: pid_t, input: Int32) {
        self.id = id
        self.root = root
        self.mainRelative = mainRelative
        self.directory = directory
        self.pid = pid
        writer = EmbeddedPreviewWriter(fileDescriptor: input)
        resolvedDirectory = EmbeddedPreviewPaths.canonical(directory.path)
    }

    // MARK: Launch

    /// `PITEX_PREVIEW_HELPER`, then the app bundle's helper locations.
    static func helperURL() -> URL? {
        var candidates: [URL] = []
        if let override = ProcessInfo.processInfo.environment["PITEX_PREVIEW_HELPER"], !override.isEmpty {
            candidates.append(URL(fileURLWithPath: override))
        }
        candidates.append(Bundle.main.bundleURL.appendingPathComponent("Contents/Helpers/PreviewEngine/pitex-preview"))
        if let resources = Bundle.main.resourceURL {
            candidates.append(resources.appendingPathComponent("PreviewEngine/pitex-preview"))
        }
        return candidates.first { candidate in
            var isDirectory: ObjCBool = false
            return FileManager.default.fileExists(atPath: candidate.path, isDirectory: &isDirectory)
                && !isDirectory.boolValue
                && FileManager.default.isExecutableFile(atPath: candidate.path)
        }
    }

    /// Owned root of every session directory (per-user temporary directory,
    /// mode 0700). Directories of Pitex processes that no longer run go.
    static func prepareSessionRoot() throws -> URL {
        let files = FileManager.default
        let root = files.temporaryDirectory.appendingPathComponent("pitex-preview", isDirectory: true)
        try files.createDirectory(at: root, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        _ = chmod(root.path, 0o700)
        for name in (try? files.contentsOfDirectory(atPath: root.path)) ?? [] {
            guard let owner = name.split(separator: "-").first.flatMap({ pid_t($0) }),
                  owner != getpid(),
                  kill(owner, 0) != 0, errno == ESRCH else { continue }
            try? files.removeItem(at: root.appendingPathComponent(name))
        }
        return root
    }

    /// Starts the helper. `deliver` receives events in order on the reader
    /// thread; the PDF bytes of a publication are already read.
    static func launch(
        id: UInt64, helper: URL, root: URL, mainRelative: String,
        directory: URL, cache: URL,
        deliver: @escaping @Sendable (EmbeddedPreviewEvent) -> Void
    ) throws -> EmbeddedPreviewSession {
        var toChild: [Int32] = [-1, -1]
        var fromChild: [Int32] = [-1, -1]
        guard pipe(&toChild) == 0 else { throw EmbeddedPreviewError.spawnFailed(errno) }
        guard pipe(&fromChild) == 0 else {
            let code = errno
            close(toChild[0]); close(toChild[1])
            throw EmbeddedPreviewError.spawnFailed(code)
        }
        for descriptor in toChild + fromChild { _ = fcntl(descriptor, F_SETFD, FD_CLOEXEC) }
        // A helper that died must fail the write, not signal the app.
        _ = fcntl(toChild[1], F_SETNOSIGPIPE, 1)

        var actions: posix_spawn_file_actions_t?
        var attributes: posix_spawnattr_t?
        posix_spawn_file_actions_init(&actions)
        posix_spawnattr_init(&attributes)
        defer {
            posix_spawn_file_actions_destroy(&actions)
            posix_spawnattr_destroy(&attributes)
        }
        posix_spawn_file_actions_adddup2(&actions, toChild[0], STDIN_FILENO)
        posix_spawn_file_actions_adddup2(&actions, fromChild[1], STDOUT_FILENO)
        posix_spawn_file_actions_addopen(&actions, STDERR_FILENO, "/dev/null", O_WRONLY, 0)
        var defaultSignals = sigset_t()
        sigemptyset(&defaultSignals)
        for signal in [SIGTERM, SIGINT, SIGQUIT, SIGHUP, SIGPIPE] { sigaddset(&defaultSignals, signal) }
        var signalMask = sigset_t()
        sigemptyset(&signalMask)
        // Own process group: engine checkpoints are forks inside it, and
        // one kill(-pgid) removes all of them. Only stdio is inherited.
        posix_spawnattr_setflags(&attributes, Int16(
            POSIX_SPAWN_SETPGROUP | POSIX_SPAWN_SETSIGDEF | POSIX_SPAWN_SETSIGMASK | POSIX_SPAWN_CLOEXEC_DEFAULT
        ))
        posix_spawnattr_setpgroup(&attributes, 0)
        posix_spawnattr_setsigdefault(&attributes, &defaultSignals)
        posix_spawnattr_setsigmask(&attributes, &signalMask)

        // Same TeX search path as final builds: Finder launches do not
        // load shell profiles.
        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = (environment["PATH"] ?? "/usr/bin:/bin:/usr/sbin:/sbin")
            + ":/Library/TeX/texbin:/opt/homebrew/bin:/usr/local/bin"
        // Canonical root: the driver realpaths it and prefix-matches the
        // (equally canonical) override paths under it — the client must
        // spell the same POSIX identity, not an alias like /tmp for
        // /private/tmp. `root` itself stays the logical root used for
        // session identity.
        let canonicalRoot = EmbeddedPreviewPaths.canonical(root.path)
        let arguments = [helper.path, "--root", canonicalRoot, "--main", mainRelative,
                         "--out", directory.path, "--cache", cache.path]
        var argv = arguments.map { strdup($0) } + [nil]
        var envp = environment.map { strdup("\($0.key)=\($0.value)") } + [nil]
        defer {
            argv.dropLast().forEach { free($0) }
            envp.dropLast().forEach { free($0) }
        }
        var pid = pid_t()
        let code = argv.withUnsafeMutableBufferPointer { argvBuffer in
            envp.withUnsafeMutableBufferPointer { envBuffer in
                posix_spawn(&pid, helper.path, &actions, &attributes, argvBuffer.baseAddress!, envBuffer.baseAddress!)
            }
        }
        close(toChild[0]); close(fromChild[1])
        guard code == 0 else {
            close(toChild[1]); close(fromChild[0])
            throw EmbeddedPreviewError.spawnFailed(code)
        }
        let session = EmbeddedPreviewSession(
            id: id, root: root, mainRelative: mainRelative, directory: directory, pid: pid, input: toChild[1]
        )
        let output = fromChild[0]
        let reader = Thread { session.read(output, deliver: deliver) }
        reader.name = "app.pitex.preview-engine.reader"
        reader.start()
        return session
    }

    // MARK: Requests

    /// Queues the full dirty-buffer snapshot of `generation`; a snapshot
    /// still waiting behind a busy pipe is replaced, never appended.
    func sendUpdate(generation: UInt64, buffers: [String: EmbeddedPreviewOutbox.Buffer]) {
        writer.update(generation: generation, buffers: buffers)
    }

    /// Returns publication `seq` to the helper, which deletes its files.
    func release(_ seq: UInt64) {
        writer.release(seq)
    }

    // MARK: Events

    private struct WireEvent: Decodable {
        let event: String
        var seq: UInt64?
        var generation: UInt64?
        var complete: Bool?
        var coherent: Bool?
        var errors: UInt64?
        var pdf: String?
        var synctex: String?
        var firstError: String?
        var message: String?
        var code: String?
    }

    private func read(_ descriptor: Int32, deliver: @Sendable (EmbeddedPreviewEvent) -> Void) {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        var pending = Data()
        var buffer = [UInt8](repeating: 0, count: 65_536)
        while true {
            let count = buffer.withUnsafeMutableBytes { Darwin.read(descriptor, $0.baseAddress, $0.count) }
            if count < 0, errno == EINTR { continue }
            guard count > 0 else { break }
            pending.append(contentsOf: buffer[0..<count])
            while let newline = pending.firstIndex(of: 0x0A) {
                if let wire = try? decoder.decode(WireEvent.self, from: pending[pending.startIndex..<newline]),
                   let event = decodeEvent(wire) {
                    deliver(event)
                }
                pending.removeSubrange(pending.startIndex...newline)
            }
        }
        close(descriptor)
        shutdown()
        deliver(.exited)
    }

    private func owns(_ url: URL) -> Bool {
        EmbeddedPreviewPaths.canonical(url.path).hasPrefix(resolvedDirectory + "/")
    }

    private func decodeEvent(_ wire: WireEvent) -> EmbeddedPreviewEvent? {
        switch wire.event {
        case "ready":
            return .ready
        case "published":
            guard let seq = wire.seq, let generation = wire.generation else { return nil }
            let pdf = URL(fileURLWithPath: wire.pdf ?? "").standardizedFileURL
            let bytes: Data? = owns(pdf) ? (try? Data(contentsOf: pdf)).flatMap {
                $0.starts(with: Data("%PDF".utf8)) ? $0 : nil
            } : nil
            // The SyncTeX runner reads `<pdf stem>.synctex`; anything else,
            // or a stale-tail (non-coherent) publication, binds nothing.
            let synctex: URL? = wire.synctex.map { URL(fileURLWithPath: $0).standardizedFileURL }.flatMap {
                $0 == pdf.deletingPathExtension().appendingPathExtension("synctex")
                    && wire.coherent == true && owns($0) ? $0 : nil
            }
            return .published(EmbeddedPreviewPublication(
                seq: seq, generation: generation, complete: wire.complete ?? false,
                errors: wire.errors ?? 0, firstError: wire.firstError, pdf: pdf,
                synctex: synctex, bytes: bytes
            ))
        case "failed":
            return .failed(generation: wire.generation ?? 0, message: wire.message ?? "")
        case "idle":
            return .idle(generation: wire.generation ?? 0, seq: wire.seq ?? 0)
        case "error":
            return .error(code: wire.code ?? "", message: wire.message ?? "")
        default:
            return nil // future events
        }
    }

    // MARK: Teardown

    /// Asks the helper to quit, then removes its whole process group and
    /// the session directory. Never blocks the caller, and never waits for
    /// a write stuck behind a helper that stopped reading.
    func shutdown() {
        lock.lock()
        let first = !shutDown
        shutDown = true
        lock.unlock()
        guard first else { return }
        writer.finish()
        DispatchQueue.global(qos: .utility).async { [self] in
            let deadline = Date().addingTimeInterval(1)
            while Date() < deadline, !hasExited() { usleep(40_000) }
            reap()
            try? FileManager.default.removeItem(at: directory)
        }
    }

    /// App quit: nothing asynchronous survives termination.
    func terminateNow() {
        lock.lock()
        shutDown = true
        lock.unlock()
        reap()
        try? FileManager.default.removeItem(at: directory)
    }

    /// Exited but not reaped (WNOWAIT keeps the pid, and so the group id,
    /// reserved for the kill below).
    private func hasExited() -> Bool {
        var info = siginfo_t()
        return waitid(P_PID, id_t(pid), &info, WEXITED | WNOHANG | WNOWAIT) == 0 && info.si_pid == pid
    }

    private func reap() {
        lock.lock()
        defer { lock.unlock() }
        guard !reaped else { return }
        reaped = true
        kill(-pid, SIGKILL)
        var status: Int32 = 0
        while waitpid(pid, &status, 0) < 0, errno == EINTR {}
    }
}

/// Routes helper events to the workspace on the main queue, in order.
private final class EmbeddedPreviewEventSink: @unchecked Sendable {
    /// Set once at init; read on the main queue only.
    private weak var workspace: WorkspaceModel?
    private let session: UInt64

    init(workspace: WorkspaceModel, session: UInt64) {
        self.workspace = workspace
        self.session = session
    }

    func deliver(_ event: EmbeddedPreviewEvent) {
        DispatchQueue.main.async {
            MainActor.assumeIsolated {
                self.workspace?.applyEmbeddedPreviewEvent(event, session: self.session)
            }
        }
    }
}

/// What a flush sends, minus what cannot change the helper's result: the project and main file, the edit revision, and the content hash of every
/// open source TeX reads from the editor (`liveCompileExtensions`). Dirtiness is not part of it (a save leaves the hash alone); the revision is (an edit
/// followed by its undo restores the hash but is two edits). An update with the same key as the last SENT one tells the helper nothing new.
struct EmbeddedFlushKey: Equatable {
    let root: String
    let main: String
    let revision: UInt64
    let sources: [String: UInt64]
}

/// Workspace-owned controller state (extensions cannot hold storage).
@MainActor
final class EmbeddedPreviewState {
    var ledger = EmbeddedPreviewLedger()
    var session: EmbeddedPreviewSession?
    /// Publication on screen; its artifacts stay ours until released.
    var displayed: EmbeddedPreviewPublication?
    /// Coalescing window of the first unsent change.
    var timer: Task<Void, Never>?
    var flushing = false
    var flushAgain = false
    /// The key of the last update SENT in this session; nil before the first one and after the session ends or the helper reports a failure/error.
    var lastFlushKey: EmbeddedFlushKey?
    /// A request that must send even when the key is unchanged (`requestEmbeddedPreviewFlush(force:)`); consumed by the next flush.
    var forceNextFlush = false
    static var sessionCount: UInt64 = 0
}

extension WorkspaceModel {
    /// Coalescing window before a burst of edits is streamed.
    static let embeddedCoalesceMs: UInt64 = 100
    /// Retry interval while an IME composition is open.
    static let embeddedComposingRetryMs: UInt64 = 150

    /// Live compile uses the embedded engine instead of the compiler for a
    /// local project; remote projects keep the compiler path.
    var embeddedPreviewEnabled: Bool {
        settings.liveCompileEnabled && settings.livePreviewBackend == "embedded" && remote == nil
    }

    private func setEmbeddedPreviewStatus(_ status: EmbeddedPreviewStatus) {
        if embeddedPreviewStatus != status { embeddedPreviewStatus = status }
    }

    /// Settings/remote → stop the session when the embedded preview no
    /// longer applies (backend → compiler, live compile off, remote attach).
    func syncEmbeddedPreview() {
        guard !embeddedPreviewEnabled else {
            if embeddedPreviewStatus == .unsupportedRemote { setEmbeddedPreviewStatus(.off) }
            return
        }
        if embeddedPreview.session != nil || embeddedPreview.timer != nil { stopEmbeddedPreview(resetContext: false) }
        let remoteOnly = settings.liveCompileEnabled && settings.livePreviewBackend == "embedded"
        setEmbeddedPreviewStatus(remoteOnly ? .unsupportedRemote : .off)
    }

    /// Ends the session (quit, then kill its group and delete its
    /// directory). Retained preview bytes may stay on screen, but their
    /// SyncTeX binding goes with the artifacts. `resetContext` also drops
    /// the final-build floor (workspace close, build target change).
    func stopEmbeddedPreview(resetContext: Bool) {
        let state = embeddedPreview
        state.timer?.cancel()
        state.timer = nil
        if state.displayed != nil {
            invalidateSyncTeX(reason: "SyncTeX is unavailable for this editing preview; edit or build to refresh it.")
        }
        state.displayed = nil
        state.lastFlushKey = nil
        state.session?.shutdown()
        state.session = nil
        if resetContext { state.ledger.resetContext() } else { state.ledger.resetSession() }
        setEmbeddedPreviewStatus(.off)
    }

    /// App termination: kill every helper group synchronously.
    func terminateEmbeddedPreviewNow() {
        embeddedPreview.session?.terminateNow()
        embeddedPreview.session = nil
    }

    // MARK: Streaming

    /// A real source edit (`noteLiveCompileEdit`): counted even while the
    /// preview is off, so the final-build floor always covers it.
    func noteEmbeddedPreviewEdit() {
        embeddedPreview.ledger.noteEdit()
        requestEmbeddedPreviewFlush()
    }

    /// Something besides an edit needs streaming: arms — never extends —
    /// the coalescing window. Callers: save, document close, disk events of
    /// open files (their watchers), agent completion/remote pull, rescan.
    /// A flush whose key (`EmbeddedFlushKey`) equals the last sent one sends
    /// nothing — unless `force`: what the key cannot see changed (a file the
    /// editor has not open, an open file TeX reads that is not a source: a
    /// disk event on it, agent completion, remote pull, rescan).
    /// Limitation: unopened project files have no watcher on macOS, so an
    /// external change to one (another editor, a script, `git checkout`)
    /// reaches the preview only with the next update of any kind — the
    /// helper re-stats everything TeX read then. Manual builds read the
    /// disk directly and are unaffected.
    func requestEmbeddedPreviewFlush(force: Bool = false) {
        guard embeddedPreviewEnabled, projectURL != nil else { return }
        if force { embeddedPreview.forceNextFlush = true }
        guard embeddedPreview.timer == nil else { return }
        armEmbeddedPreviewTimer(milliseconds: Self.embeddedCoalesceMs)
    }

    private func armEmbeddedPreviewTimer(milliseconds: UInt64) {
        // A change noted while a workspace closes must not flush into the
        // next project opened in this window.
        let context = projectGeneration
        embeddedPreview.timer = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(milliseconds))
            guard let self, !Task.isCancelled else { return }
            self.embeddedPreview.timer = nil
            guard self.projectGeneration == context else { return }
            // Composition keeps the change pending — retry shortly.
            if self.environment?.editor.hasMarkedText == true {
                self.armEmbeddedPreviewTimer(milliseconds: Self.embeddedComposingRetryMs)
                return
            }
            await self.flushEmbeddedPreview()
        }
    }

    /// Streams the newest buffer state, starting the helper if needed.
    /// Never waits for a previous generation to finish typesetting.
    func flushEmbeddedPreview() async {
        let state = embeddedPreview
        guard !state.flushing else {
            state.flushAgain = true
            return
        }
        state.flushing = true
        repeat {
            state.flushAgain = false
            await flushEmbeddedPreviewOnce()
        } while state.flushAgain
        state.flushing = false
    }

    private struct EmbeddedSource {
        let path: String
        let hash: UInt64
        let dirty: Bool
        /// Only dirty buffers carry their text.
        let text: String
    }

    /// Every open TeX-like source (absolute path, content hash, unsaved
    /// text) plus the edit revision those texts correspond to. The active
    /// document comes from `documentSnapshot` — the value whose change
    /// counted the edit; other sessions cannot take edits unless another
    /// document is activated meanwhile, which collects again.
    private func collectEmbeddedSources(root: URL) async -> (revision: UInt64, sources: [EmbeddedSource])? {
        for _ in 0..<4 {
            let active = documentSnapshot
            let revision = embeddedPreview.ledger.editRevision
            var sources: [EmbeddedSource] = []
            for session in registeredSessions
            where Self.liveCompileExtensions.contains((session.path.rawValue as NSString).pathExtension.lowercased()) {
                let snapshot: DocumentSessionCore.DocumentSnapshot
                if let active, active.path == session.path {
                    snapshot = active
                } else {
                    snapshot = await session.snapshot()
                }
                let dirty = snapshot.saveState != .clean
                // Canonical identity: the driver realpaths the project
                // root, and every override path must spell the same POSIX
                // identity for its prefix match to hold — realpath(3),
                // not Foundation (which can strip /private on Darwin). A
                // dirty buffer may name a file that is not on disk, so
                // ancestors are resolved, not the leaf. Two sessions
                // resolving to one canonical path (a source reachable by
                // a symlink alias) are one override — the dirty one wins.
                let path = EmbeddedPreviewPaths.canonical(
                    root.appendingPathComponent(snapshot.path.rawValue).path
                )
                let source = EmbeddedSource(
                    path: path,
                    hash: snapshot.contentHash.rawValue, dirty: dirty,
                    text: dirty ? snapshot.text : ""
                )
                if let index = sources.firstIndex(where: { $0.path == path }) {
                    if dirty && !sources[index].dirty { sources[index] = source }
                } else {
                    sources.append(source)
                }
            }
            guard projectURL == root else { return nil }
            if documentSnapshot?.documentID == active?.documentID { return (revision, sources) }
        }
        return nil
    }

    private func flushEmbeddedPreviewOnce() async {
        guard embeddedPreviewEnabled, case .ready = phase, let root = projectURL else { return }
        // Taken before the first await: a forced request arriving meanwhile belongs to the next flush.
        let force = embeddedPreview.forceNextFlush
        embeddedPreview.forceNextFlush = false
        let context = projectGeneration
        guard let main = buildSourceRelativePath() else {
            setEmbeddedPreviewStatus(.unavailable(
                buildTargetMessage ?? WorkspaceBuildError.noActiveDocument.localizedDescription
            ))
            return
        }
        guard let captured = await collectEmbeddedSources(root: root),
              embeddedPreviewEnabled, projectGeneration == context, projectURL == root,
              buildSourceRelativePath() == main else { return }
        let state = embeddedPreview
        if let session = state.session, session.mainRelative != main || session.root != root {
            stopEmbeddedPreview(resetContext: true)
        }
        if state.session == nil {
            do {
                try startEmbeddedPreviewSession(root: root, main: main)
            } catch {
                setEmbeddedPreviewStatus(.unavailable(error.localizedDescription))
                return
            }
        }
        guard let session = state.session else { return }
        let key = EmbeddedFlushKey(
            root: root.path, main: main, revision: captured.revision,
            sources: Dictionary(captured.sources.map { ($0.path, $0.hash) }, uniquingKeysWith: { first, _ in first })
        )
        if !force, state.lastFlushKey == key {
            // The helper already has exactly this: no generation, no update, no `.updating` (it would answer `idle` and reset the status — a flash
            // with nothing behind it). That `idle` is also what rebinds SyncTeX after a save, so do it here.
            if syncTeXBinding == nil { bindEmbeddedPreviewSyncTeX() }
            return
        }
        let generation = state.ledger.recordUpdate(sources: key.sources, revision: captured.revision)
        // The full dirty set: the writer sends only what the helper does
        // not hold yet and closes what it no longer should.
        session.sendUpdate(generation: generation, buffers: Dictionary(
            captured.sources.filter(\.dirty).map { ($0.path, EmbeddedPreviewOutbox.Buffer(hash: $0.hash, text: $0.text)) },
            uniquingKeysWith: { first, _ in first }
        ))
        state.lastFlushKey = key
        setEmbeddedPreviewStatus(.updating)
    }

    private func startEmbeddedPreviewSession(root: URL, main: String) throws {
        guard let helper = EmbeddedPreviewSession.helperURL() else { throw EmbeddedPreviewError.helperMissing }
        let files = FileManager.default
        let sessionRoot = try EmbeddedPreviewSession.prepareSessionRoot()
        EmbeddedPreviewState.sessionCount += 1
        let id = EmbeddedPreviewState.sessionCount
        let directory = sessionRoot.appendingPathComponent("\(getpid())-\(id)", isDirectory: true)
        try files.createDirectory(at: directory, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
        let cache = (try? files.url(for: .cachesDirectory, in: .userDomainMask, appropriateFor: nil, create: true))
            .map { $0.appendingPathComponent("Pitex/preview-engine", isDirectory: true) }
            ?? files.temporaryDirectory.appendingPathComponent("pitex-preview-engine-cache", isDirectory: true)
        try? files.createDirectory(at: cache, withIntermediateDirectories: true)
        let sink = EmbeddedPreviewEventSink(workspace: self, session: id)
        do {
            embeddedPreview.session = try EmbeddedPreviewSession.launch(
                id: id, helper: helper, root: root, mainRelative: main,
                directory: directory, cache: cache, deliver: { sink.deliver($0) }
            )
        } catch {
            try? files.removeItem(at: directory)
            throw error
        }
        // Generations stay monotonic across respawns (final floor).
        embeddedPreview.ledger.resetSession()
        embeddedPreview.displayed = nil
        embeddedPreview.lastFlushKey = nil
    }

    // MARK: Helper events

    func applyEmbeddedPreviewEvent(_ event: EmbeddedPreviewEvent, session id: UInt64) {
        let state = embeddedPreview
        // A stopped session's late events own nothing: its directory went
        // with it.
        guard let session = state.session, session.id == id else { return }
        switch event {
        case .ready:
            break
        case let .published(publication):
            displayEmbeddedPublication(publication, session: session)
        case let .failed(generation, message):
            state.lastFlushKey = nil   // the next request retries even with an unchanged key
            if generation >= state.ledger.generation { setEmbeddedPreviewStatus(.failed(message)) }
        case let .idle(generation, seq):
            // `seq` is current for `generation` too — its SyncTeX may bind
            // against that generation's sources (e.g. after a save).
            if state.ledger.noteIdle(generation: generation, seq: seq), syncTeXBinding == nil {
                bindEmbeddedPreviewSyncTeX()
            }
            // seq 0: nothing publishable is current for it (a failed or
            // partial pass) — a failure message on screen stays.
            if seq != 0, generation >= state.ledger.generation, state.ledger.displayed?.complete ?? true {
                setEmbeddedPreviewStatus(displayedEmbeddedStatus())
            }
        case let .error(code, message):
            state.lastFlushKey = nil
            if ["no_tex", "no_engine", "usage"].contains(code) {
                stopEmbeddedPreview(resetContext: false)
                setEmbeddedPreviewStatus(.unavailable(message))
            } else {
                setEmbeddedPreviewStatus(.failed(message))
            }
        case .exited:
            // Respawned on the next edit; the last good preview stays.
            var status = EmbeddedPreviewStatus.failed("")
            if case .unavailable = embeddedPreviewStatus { status = embeddedPreviewStatus }
            stopEmbeddedPreview(resetContext: false)
            setEmbeddedPreviewStatus(status)
        }
    }

    private func displayedEmbeddedStatus() -> EmbeddedPreviewStatus {
        guard let displayed = embeddedPreview.displayed,
              let current = embeddedPreview.ledger.displayed else { return .current }
        // `current.generation` includes what `idle` confirmed since.
        if !displayed.complete || current.generation < embeddedPreview.ledger.generation { return .updating }
        if displayed.errors > 0 { return .errors(displayed.firstError ?? "") }
        return .current
    }

    private func displayEmbeddedPublication(_ publication: EmbeddedPreviewPublication, session: EmbeddedPreviewSession) {
        let state = embeddedPreview
        guard let bytes = publication.bytes,
              projectURL == session.root,
              buildSourceRelativePath() == session.mainRelative else {
            session.release(publication.seq)
            return
        }
        switch state.ledger.accept(seq: publication.seq, generation: publication.generation,
                                   complete: publication.complete) {
        case .stale:
            session.release(publication.seq)
        case .belowFinalFloor:
            // The final PDF stays: this carries no edit made after it.
            session.release(publication.seq)
            if publication.generation >= state.ledger.generation { setEmbeddedPreviewStatus(.current) }
        case let .display(releasing):
            state.displayed = publication
            retainedPDF = RetainedPDF(
                data: bytes, artifactPath: publication.pdf.path,
                sourceTarget: session.mainRelative, isEmbeddedPreview: true
            )
            // The binding belonged to the replaced artifact — move it
            // before that artifact's files are released.
            invalidateSyncTeX(reason: publication.synctex == nil
                ? "SyncTeX resumes once the editing preview finishes a coherent pass."
                : "SyncTeX is being refreshed for the editing preview.")
            if let releasing { session.release(releasing) }
            setEmbeddedPreviewStatus(displayedEmbeddedStatus())
            bindEmbeddedPreviewSyncTeX()
        }
    }

    // MARK: SyncTeX

    /// Binds SyncTeX to the displayed coherent publication when the editor
    /// buffers still equal the sources compiled for it.
    private func bindEmbeddedPreviewSyncTeX() {
        let state = embeddedPreview
        guard let publication = state.displayed, publication.synctex != nil,
              let session = state.session else { return }
        let seq = publication.seq
        Task { @MainActor [weak self] in
            guard let self else { return }
            let matches = await self.embeddedPreviewSourcesMatch()
            guard self.embeddedPreview.displayed?.seq == seq, self.embeddedPreview.session === session else { return }
            guard matches else {
                self.invalidateSyncTeX(reason: Self.embeddedSourcesChangedReason)
                return
            }
            await self.refreshSyncTeXBinding(pdfURL: publication.pdf, sourceRelativePath: session.mainRelative)
        }
    }

    private static let embeddedSourcesChangedReason =
        "The sources changed since this editing preview; SyncTeX resumes with the next preview."

    /// Current open sources equal the ones compiled for the displayed
    /// publication (unsaved text included — the disk hash is not the
    /// compiled source).
    private func embeddedPreviewSourcesMatch() async -> Bool {
        guard let displayed = embeddedPreview.ledger.displayed,
              let expected = embeddedPreview.ledger.update(displayed.generation)?.sources,
              let root = projectURL,
              let current = await collectEmbeddedSources(root: root) else { return false }
        return Dictionary(current.sources.map { ($0.path, $0.hash) }, uniquingKeysWith: { first, _ in first }) == expected
    }

    /// Checked before every forward/inverse query on an editing preview:
    /// the reason to refuse it, or nil to proceed.
    func embeddedPreviewSyncTeXRefusal() async -> String? {
        guard retainedPDF?.isEmbeddedPreview == true else { return nil }
        guard let publication = embeddedPreview.displayed else {
            return "SyncTeX is unavailable for this editing preview; edit or build to refresh it."
        }
        guard publication.synctex != nil else {
            return "SyncTeX resumes once the editing preview finishes a coherent pass."
        }
        guard await embeddedPreviewSourcesMatch() else { return Self.embeddedSourcesChangedReason }
        return nil
    }

    // MARK: Final builds

    /// A manual build is about to snapshot its sources (pre-save).
    func noteEmbeddedFinalBuildStarted() {
        embeddedPreview.ledger.noteFinalBuildStarted()
    }

    /// The manual build succeeded. True when the displayed editing preview
    /// already carries an edit made after the build started and stays on
    /// screen; false when the final PDF replaces it (its artifacts are
    /// released after the binding moved off them).
    func keepEmbeddedPreviewOverFinalBuild() -> Bool {
        let state = embeddedPreview
        switch state.ledger.noteFinalBuildPublished() {
        case .keepPreview:
            if syncTeXBinding == nil { bindEmbeddedPreviewSyncTeX() }
            return true
        case let .showFinal(releasing):
            state.displayed = nil
            if let releasing {
                invalidateSyncTeX(reason: "SyncTeX will be refreshed after the build completes.")
                state.session?.release(releasing)
            }
            if embeddedPreviewStatus == .updating, state.ledger.editRevision <= state.ledger.finalFloor ?? 0 {
                setEmbeddedPreviewStatus(.current)
            }
            return false
        }
    }

    /// Open Externally: preview artifacts are released as newer ones
    /// arrive, so the system viewer gets a copy of the bytes on screen
    /// under a name that keeps the editing-preview label. The copy is an
    /// explicit export owned by that viewer, not session state: Pitex
    /// never deletes it (it may still be open there); it stays under
    /// `$TMPDIR/Pitex Editing Previews/` until the system purges it.
    static func exportEmbeddedPreviewCopy(_ data: Data, named name: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("Pitex Editing Previews", isDirectory: true)
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true,
                                                attributes: [.posixPermissions: 0o700])
        let url = directory.appendingPathComponent(name)
        try data.write(to: url, options: .atomic)
        return url
    }
}
