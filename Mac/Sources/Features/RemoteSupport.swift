import AppKit
import BuildCore
import Foundation
import RemoteCore

/// Routes each build to this Mac or, while a remote project is open, to
/// that device (`RemoteBuildExecutor`).
actor WorkspaceBuildExecutor: BuildProcessExecuting {
    private let local = StreamingBuildExecutor()
    private var remote: RemoteBuildExecutor?

    func use(_ remote: RemoteBuildExecutor?) {
        self.remote = remote
    }

    func execute(
        _ request: BuildProcessRequest,
        output: @escaping @Sendable (BuildProcessOutput) async -> Void
    ) async throws -> BuildProcessResult {
        if let remote { return try await remote.execute(request, output: output) }
        return try await local.execute(request, output: output)
    }

    func cancel(buildID: BuildID) async {
        await remote?.cancel(buildID: buildID)
        await local.cancel(buildID: buildID)
    }
}

/// App side of `MirrorWriteGate`: an editor save holds a ticket while it
/// checks and replaces its file; a sync commit waits for the tickets to
/// drain and holds new saves back until it ends. One for the whole app —
/// commits take milliseconds, and two windows may share a mirror.
@MainActor
final class MirrorWrites: MirrorWriteGate {
    static let shared = MirrorWrites()
    private var writers = 0
    private var committing = false
    private var pendingCommits = 0
    private var waitingWriters: [CheckedContinuation<Void, Never>] = []
    private var waitingCommits: [CheckedContinuation<Void, Never>] = []

    func enter() async {
        // Waiting commits go first, so a stream of saves cannot starve them.
        while committing || pendingCommits > 0 {
            await withCheckedContinuation { waitingWriters.append($0) }
        }
        writers += 1
    }

    func leave() {
        writers -= 1
        if writers == 0 { wake(&waitingCommits) }
    }

    nonisolated func beginSyncCommit() async { await begin() }
    nonisolated func endSyncCommit() async { await end() }

    private func begin() async {
        pendingCommits += 1
        while committing || writers > 0 {
            await withCheckedContinuation { waitingCommits.append($0) }
        }
        pendingCommits -= 1
        committing = true
    }

    private func end() {
        committing = false
        wake(&waitingCommits)
        wake(&waitingWriters)
    }

    private func wake(_ waiting: inout [CheckedContinuation<Void, Never>]) {
        let continuations = waiting
        waiting.removeAll()
        continuations.forEach { $0.resume() }
    }
}

/// A project opened through "Open via SSH": the local mirror the editor
/// works on, its sync engine and the device's build executor.
struct RemoteWorkspace {
    enum Status: Equatable {
        case synced
        case syncing
        /// The last transfer failed; edits stay in the mirror and go up on
        /// the next save, build or open.
        case offline(String)
    }

    let project: RemoteProject
    let sync: RemoteSync
    let executor: RemoteBuildExecutor
    var status: Status = .syncing
    /// Files changed both here and on the device since the last sync.
    var conflicts: [String] = []
    var lastPull = Date.distantPast

    @MainActor
    init(mirror: RemoteMirror) {
        project = mirror.project
        sync = RemoteSync.shared(for: mirror, gate: MirrorWrites.shared)
        executor = RemoteBuildExecutor(sync: sync)
    }

    var deviceName: String { project.connection.name }

    var statusText: String {
        if !conflicts.isEmpty {
            return String(format: String(localized: "remote.status.conflicts"), deviceName, conflicts.count)
        }
        switch status {
        case .synced: return String(format: String(localized: "remote.status.synced"), deviceName)
        case .syncing: return String(format: String(localized: "remote.status.syncing"), deviceName)
        case .offline: return String(format: String(localized: "remote.status.offline"), deviceName)
        }
    }
}

extension WorkspaceModel {
    /// "Open via SSH…" chose a folder: prepare its mirror and open it like
    /// any folder — in this window when it is empty, else a new one.
    func openRemote(_ project: RemoteProject) {
        do {
            let mirror = try RemoteMirror.prepare(for: project)
            WorkspaceWindows.route(mirror.root, from: self)
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    /// Menu/welcome entry point; the sheet needs a window to attach to.
    func presentOpenViaSSH() {
        let host = window != nil ? self : (WorkspaceWindows.live.first { $0.window != nil } ?? self)
        host.showingOpenViaSSH = true
    }

    /// Called by `open(_:)` before it reads any file: a mirror path makes
    /// this a remote workspace and pulls the device's current state. A
    /// failed pull still opens the cached copy (offline) when there is one;
    /// with nothing cached it fails the open.
    func beginRemoteSession(for url: URL) async -> Bool {
        guard let mirror = RemoteMirror.containing(url) else {
            remote = nil
            await buildExecutor.use(nil)
            return true
        }
        let workspace = RemoteWorkspace(mirror: mirror)
        remote = workspace
        await buildExecutor.use(workspace.executor)
        let cached = ((try? FileManager.default.contentsOfDirectory(atPath: mirror.root.path)) ?? []).isEmpty == false
        do {
            try await workspace.sync.pull()
            remote?.lastPull = Date()
            // Edits made while offline (or right before quitting) go up now.
            let pushed = try await workspace.sync.push()
            remote?.conflicts = pushed.conflicts
            remote?.status = .synced
        } catch {
            guard cached else {
                remote = nil
                await buildExecutor.use(nil)
                phase = .failed(String(format: String(localized: "remote.error.open"),
                                       workspace.deviceName, error.localizedDescription))
                return false
            }
            remote?.status = .offline(error.localizedDescription)
        }
        return true
    }

    /// Uploads local edits (after saves, agent runs and before builds).
    /// Returns why the upload failed, if it did.
    @discardableResult
    func pushRemote() async -> String? {
        guard let current = remote else { return nil }
        if remote?.status != .syncing { remote?.status = .syncing }
        do {
            let report = try await current.sync.push()
            guard remote?.project == current.project else { return nil }
            remote?.conflicts = report.conflicts
            if !remotePushPending { remote?.status = .synced }
            return nil
        } catch {
            if remote?.project == current.project {
                remote?.status = .offline(error.localizedDescription)
            }
            return error.localizedDescription
        }
    }

    /// Saving already follows the Editor's Auto Save delay. Upload as soon
    /// as it finishes; saves during a transfer request one follow-up pass.
    func schedulePush() {
        guard remote != nil else { return }
        remotePushPending = true
        guard remotePushTask == nil else { return }
        remotePushTask = Task { @MainActor [weak self] in
            guard let self else { return }
            defer { self.remotePushTask = nil }
            while self.remotePushPending, self.remote != nil {
                self.remotePushPending = false
                await self.pushRemote()
            }
        }
    }

    /// Close/quit: the last upload, bounded so an unreachable device cannot
    /// hold the window or the app (unsent edits stay in the mirror and go
    /// up on the next open).
    func flushRemote(timeout: Duration = .seconds(10)) async {
        guard let current = remote else { return }
        let scheduled = remotePushTask
        let push = Task<String?, Never> {
            await scheduled?.value
            guard !Task.isCancelled, self.remote?.project == current.project else { return nil }
            return await self.pushRemote()
        }
        let once = ResumeOnce()
        await withCheckedContinuation { continuation in
            once.continuation = continuation
            Task { @MainActor in _ = await push.value; once.resume() }
            Task { @MainActor in
                try? await Task.sleep(for: timeout)
                push.cancel()
                once.resume()
            }
        }
    }

    /// Brings down changes made on the device and adopts them in open
    /// documents (clean ones reload, dirty ones flag a conflict).
    func pullRemote() async {
        guard let current = remote else { return }
        remote?.status = .syncing
        do {
            let report = try await current.sync.pull()
            guard remote?.project == current.project else { return }
            remote?.conflicts = report.conflicts
            remote?.lastPull = Date()
            remote?.status = .synced
            if report.changedLocally { await refreshAfterAgentActivity() }
        } catch {
            guard remote?.project == current.project else { return }
            remote?.status = .offline(error.localizedDescription)
        }
    }

    /// Window came forward: pull when the last pull is over a minute old.
    func pullRemoteIfStale() {
        guard let remote, remote.status != .syncing,
              Date().timeIntervalSince(remote.lastPull) > 60 else { return }
        Task { await pullRemote() }
    }

    /// Settles one conflicting file either way.
    func resolveRemoteConflict(_ path: String, keepLocal: Bool) async {
        guard let current = remote else { return }
        remote?.status = .syncing
        do {
            let remaining = try await current.sync.resolve(path, keepLocal: keepLocal)
            guard remote?.project == current.project else { return }
            remote?.conflicts = remaining
            remote?.status = .synced
            if !keepLocal { await refreshAfterAgentActivity() }
        } catch {
            guard remote?.project == current.project else { return }
            remote?.status = .offline(error.localizedDescription)
        }
    }

    /// Before a remote build: upload pending edits and tell the executor
    /// which outputs to bring back. Returns why the build must not start —
    /// the upload failed, or files changed on both sides would have the
    /// device build something other than what the editor shows.
    func prepareRemoteBuild(outputs: [String], required pdf: String) async -> String? {
        guard let current = remote else { return nil }
        if let failure = await pushRemote() {
            return String(format: String(localized: "remote.build.upload_failed"), failure)
        }
        if let conflicts = remote?.conflicts, !conflicts.isEmpty {
            return String(format: String(localized: "remote.build.conflicts"), conflicts.joined(separator: ", "))
        }
        await current.executor.setOutputs(outputs, required: pdf)
        return nil
    }

    /// Workspace teardown: last upload, then back to local builds.
    func endRemoteSession() async {
        await flushRemote()
        remotePushPending = false
        remote = nil
        await buildExecutor.use(nil)
    }

    /// Open Recent label: a mirror shows which device it belongs to.
    static func recentTitle(for url: URL) -> String {
        guard let mirror = RemoteMirror.containing(url) else { return url.lastPathComponent }
        return String(format: String(localized: "remote.recent_title"), url.lastPathComponent, mirror.project.connection.name)
    }
}

/// Resumes a continuation once, whichever of several tasks gets there first.
@MainActor
private final class ResumeOnce {
    var continuation: CheckedContinuation<Void, Never>?

    func resume() {
        continuation?.resume()
        continuation = nil
    }
}
