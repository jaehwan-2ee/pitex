import AppKit
import BuildCore
import CryptoKit
import Foundation
import RemoteCore

/// Routes each build to this Mac or, while a remote project is open, to
/// that device (`RemoteBuildExecutor`).
actor WorkspaceBuildExecutor: BuildProcessExecuting {
    private let local = StreamingBuildExecutor()
    private var remote: RemoteBuildExecutor?
    private var localBuilds: Set<BuildID> = []

    func use(_ remote: RemoteBuildExecutor?) {
        self.remote = remote
    }

    /// A compatibility preview uses this Mac even while manual builds use SSH.
    /// Keep the decision for every stage/pass belonging to the same build.
    func useLocal(for buildID: BuildID) {
        localBuilds.insert(buildID)
    }

    func releaseLocalBuild(_ buildID: BuildID) {
        localBuilds.remove(buildID)
    }

    func execute(
        _ request: BuildProcessRequest,
        output: @escaping @Sendable (BuildProcessOutput) async -> Void
    ) async throws -> BuildProcessResult {
        if !localBuilds.contains(request.buildID), let remote {
            return try await remote.execute(request, output: output)
        }
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
        /// the next manual save/build or forced close.
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
            let pulled = try await workspace.sync.pull()
            remote?.conflicts = pulled.conflicts
            remote?.lastPull = Date()
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

    /// Uploads the mirror only from an explicit manual save or forced close.
    /// Returns why the upload failed, if it did.
    @discardableResult
    func pushRemote() async -> String? {
        guard let current = remote else { return nil }
        if remote?.status != .syncing { remote?.status = .syncing }
        do {
            let report = try await current.sync.push()
            guard remote?.project == current.project else { return nil }
            remote?.conflicts = report.conflicts
            remote?.status = .synced
            return nil
        } catch {
            if remote?.project == current.project {
                remote?.status = .offline(error.localizedDescription)
            }
            return error.localizedDescription
        }
    }

    /// One manual commit: flush the editor, persist every dirty source, then
    /// upload. Each invocation queues behind the previous one and reads the
    /// latest revisions after that wait, including edits made during upload.
    /// An explicit Save schedules its build only after verification succeeds;
    /// a forced close commits the same sources without starting a build.
    func saveRemoteSources(build: Bool = false) async -> String? {
        guard let current = remote, let root = projectURL else { return nil }
        let previous = manualRemoteSaveTask
        let context = projectGeneration
        remoteSavesInFlight += 1
        syncLiveScheduler()
        defer {
            remoteSavesInFlight -= 1
            syncLiveScheduler()
        }
        let task = Task { @MainActor in
            _ = await previous?.value
            guard self.projectGeneration == context, self.projectURL == root,
                  self.remote?.project == current.project else {
                return "The workspace changed before the remote save completed." as String?
            }
            // Never replace source files while the remote compiler reads them.
            await self.activeBuildTask?.value
            await self.environment?.editor.flushPendingChanges()
            let savedRevision = self.embeddedPreview.ledger.editRevision
            self.refreshBuildTarget()
            let savedTarget = self.buildSourceURL()
            if let problem = await self.persistDirtySessions() { return problem }
            await self.waitForPendingWrites()
            if self.isClosing {
                // An editor acknowledgement may have arrived during a disk
                // write. Preserve it before releasing the closing session.
                for _ in 0..<4 {
                    await self.environment?.editor.flushPendingChanges()
                    if let problem = await self.persistDirtySessions() { return problem }
                    await self.waitForPendingWrites()
                    var dirty = false
                    for session in self.registeredSessions {
                        if await session.snapshot().saveState == .dirty { dirty = true }
                    }
                    if !dirty { break }
                }
                for session in self.registeredSessions {
                    if await session.snapshot().saveState != .clean {
                        return "The source is still changing. Save again before closing the SSH project."
                    }
                }
            }
            let expected: [String: String]
            do { expected = try await self.remoteSavedSourceHashes(root: root) }
            catch { return error.localizedDescription }
            guard self.projectGeneration == context, self.projectURL == root else {
                return "The workspace changed before the remote save completed." as String?
            }
            if let failure = await self.pushRemote() {
                return String(format: String(localized: "remote.build.upload_failed"), failure)
            }
            if let conflicts = self.remote?.conflicts, !conflicts.isEmpty {
                return String(format: String(localized: "remote.build.conflicts"), conflicts.joined(separator: ", "))
            }
            do { try await current.sync.verifyUploaded(expected) }
            catch { return error.localizedDescription }
            self.refreshGit()
            self.remoteSavedEditRevision = savedRevision
            self.remoteSavedBuildTarget = savedTarget
            if build { await self.requestFinalBuild() }
            return nil as String?
        }
        manualRemoteSaveTask = task
        return await task.value
    }

    /// A forced close must never claim that sources excluded from the
    /// mirror protocol reached the device. Validate open documents before
    /// transferring rather than silently dropping them from the scan.
    private func remoteSavedSourceHashes(root: URL) async throws -> [String: String] {
        var hashes: [String: String] = [:]
        for session in registeredSessions {
            let snapshot = await session.snapshot()
            let path = snapshot.path.rawValue
            let file = root.appendingPathComponent(path)
            guard !RemoteSyncRules.isExcludedPath(path),
                  let values = try? file.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
                  values.isRegularFile == true, values.isSymbolicLink != true,
                  let size = values.fileSize, size < RemoteSyncRules.maximumFileSize,
                  let data = try? Data(contentsOf: file) else {
                throw SSHError.remote(status: -1, message: "The file \(path) cannot be saved to the SSH device. Sources must be readable regular files smaller than 50 MB outside excluded folders.")
            }
            hashes[path] = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        }
        return hashes
    }

    /// Keep the workspace/editor alive so a failed save can be retried.
    func showRemoteSaveFailure(_ problem: String) {
        buildState = .failed(problem)
        buildLogText = problem
        consoleSection = .log
        bottomPanelVisible = true
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
            let remaining: [String]
            if keepLocal { remaining = try await current.sync.acceptLocalConflict(path) }
            else { remaining = try await current.sync.resolve(path, keepLocal: false) }
            guard remote?.project == current.project else { return }
            remote?.conflicts = remaining
            remote?.status = .synced
            if !keepLocal { await refreshAfterAgentActivity() }
        } catch {
            guard remote?.project == current.project else { return }
            remote?.status = .offline(error.localizedDescription)
        }
    }

    /// A manual save already uploaded the sources. Configure which outputs
    /// the remote build must return; this preparation never uploads itself.
    func prepareRemoteBuild(outputs: [String], required pdf: String) async -> String? {
        guard let current = remote else { return nil }
        if let conflicts = remote?.conflicts, !conflicts.isEmpty {
            return String(format: String(localized: "remote.build.conflicts"), conflicts.joined(separator: ", "))
        }
        await current.executor.setOutputs(outputs, required: pdf)
        return nil
    }

    /// Teardown happens only after forced manual saving succeeds.
    func endRemoteSession() async {
        manualRemoteSaveTask = nil
        remoteSavedBuildTarget = nil
        remote = nil
        await buildExecutor.use(nil)
    }

    /// Open Recent label: a mirror shows which device it belongs to.
    static func recentTitle(for url: URL) -> String {
        guard let mirror = RemoteMirror.containing(url) else { return url.lastPathComponent }
        return String(format: String(localized: "remote.recent_title"), url.lastPathComponent, mirror.project.connection.name)
    }
}
