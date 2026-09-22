import AppKit
import AppPorts
import AppShell
import BuildCore
import Darwin
import DocumentSessionCore
import EditorMacAdapter
import Foundation
import GitCore
import LanguageCore
import MacPlatform
import ProjectCore
import ProjectFeature
import SyncTeXCore
import SwiftUI
import TexDomain
import UniformTypeIdentifiers

enum WorkspacePhase {
    case noProject
    case loading(URL)
    case ready
    case failed(String)
}

enum WorkspaceBuildState: Equatable {
    case unavailable(String)
    case building
    case succeeded(pdf: Data, log: String)
    case failed(String)
}

enum WorkspaceSyncTeXState: Equatable {
    case unavailable(String)
    case current
    case stale(String)
    case ambiguous(String)
}

/// Bottom console tabs: Assistant | Git Integration | Issues | Terminal |
/// Build Log. The assistant lives here (not in the inspector) so the PDF
/// preview and the assistant can be visible at the same time.
enum ConsoleSection: String, CaseIterable, Identifiable {
    case assistant
    case git
    case issues
    case terminal
    case log
    var id: Self { self }
}

/// Upper sidebar sections: Outline | Labels | BibTeX.
enum SidebarSection: String, CaseIterable, Identifiable {
    case outline
    case labels
    case bibtex
    var id: Self { self }
}

/// One row in the document outline (a sectioning command in the active file).
struct DocumentOutlineItem: Identifiable, Hashable, Sendable {
    let id: Int
    let title: String
    let level: Int
    let line: Int
}

/// One \label{…} entry in the active document.
struct DocumentLabelItem: Identifiable, Hashable, Sendable {
    let id: Int
    let name: String
    let line: Int
}

/// One @entry{key,…} row collected from the project's .bib files.
struct BibliographyItem: Identifiable, Hashable, Sendable {
    let id: String
    let key: String
    let type: String
    let file: String
}

/// One `% TODO:`/`% DONE:` comment collected from the project's .tex files.
struct DocumentTodoItem: Identifiable, Hashable, Sendable {
    var id: String { "\(url.path)#\(line)" }
    /// Project-relative path (like `BibliographyItem.file`).
    let file: String
    let url: URL
    /// 1-based line of the comment.
    let line: Int
    let text: String
    let done: Bool
}

actor NativeDocumentSessionPort: AppPorts.DocumentSessionPort {
    let session: DocumentSessionCore.DocumentSession
    private let didChange: @MainActor @Sendable (DocumentSessionCore.DocumentSnapshot) -> Void

    init(
        session: DocumentSessionCore.DocumentSession,
        didChange: @escaping @MainActor @Sendable (DocumentSessionCore.DocumentSnapshot) -> Void
    ) {
        self.session = session
        self.didChange = didChange
    }

    func snapshot() async -> AppPorts.DocumentSnapshot {
        let snapshot = await session.snapshot()
        return AppPorts.DocumentSnapshot(revision: snapshot.revision, text: snapshot.text)
    }

    func submit(_ mutation: AppPorts.DocumentMutation) async throws -> AppPorts.DocumentMutationResult {
        let current = await session.snapshot()
        guard mutation.baseRevision == current.revision else {
            return .rejected(current: .init(revision: current.revision, text: current.text))
        }
        let source = current.text as NSString
        let range = NSRange(location: mutation.range.location, length: mutation.range.length)
        guard range.location >= 0,
              range.length >= 0,
              range.location <= source.length,
              range.length <= source.length - range.location else {
            return .rejected(current: .init(revision: current.revision, text: current.text))
        }
        let replacement = source.replacingCharacters(in: range, with: mutation.replacement)
        do {
            let updated = try await session.apply(
                .replaceText(replacement),
                expectedRevision: mutation.baseRevision
            )
            await didChange(updated)
            return .applied(.init(revision: updated.revision, text: updated.text))
        } catch let error as DocumentSessionError {
            switch error {
            case .staleRevision:
                let latest = await session.snapshot()
                return .rejected(current: .init(revision: latest.revision, text: latest.text))
            default:
                throw error
            }
        }
    }
}

@MainActor
final class WorkspaceModel: ObservableObject {
    @Published private(set) var phase: WorkspacePhase = .noProject
    @Published private(set) var projectURL: URL? {
        didSet { rebuildProjectTree() }
    }
    @Published private(set) var projectFiles: [URL] = [] {
        didSet { rebuildProjectTree() }
    }
    @Published private(set) var openDocuments: [URL] = []
    @Published private(set) var activeDocumentURL: URL?
    @Published private(set) var environment: AppEnvironment? {
        didSet { observeEditorSelection() }
    }
    @Published private(set) var documentSnapshot: DocumentSessionCore.DocumentSnapshot? {
        didSet { scheduleStructureRefresh(); scheduleAutosave() }
    }
    @Published var buildLogText = ""
    /// Buffered build-log chunks; flushed into buildLogText once per runloop
    /// turn by BuildSupport.scheduleLogFlush.
    var pendingLogText = ""
    var logFlushScheduled = false
    @Published var buildIssues: [BuildIssueRecord] = []
    @Published internal(set) var buildState: WorkspaceBuildState = .unavailable(
        "No build target has been configured for this project."
    )
    @Published private(set) var syncTeXState: WorkspaceSyncTeXState = .unavailable(
        "SyncTeX is unavailable until a successful build produces matching metadata."
    )
    @Published var showingSettings = false

    private var documentLoadGeneration = UUID()
    var gitRefreshInFlight = false
    @Published private(set) var wordCount = 0
    let registry = DocumentSessionRegistry()
    let settings = SettingsStore()
    let buildOrchestrator = BuildOrchestrator(executor: StreamingBuildExecutor())
    let syncTeXRunner = SyncTeXRunner()
    let highlighter = SyntaxHighlighter()
    private(set) var agent: AgentCoordinator?
    /// The Copilot-style inline completion session — a dedicated pi
    /// subprocess that never touches the chat transcript. Created per
    /// project like `agent` and shut down in `close()`.
    private(set) var completion: GhostCompletionCoordinator?
    private(set) var syncTeXBinding: SyncTeXBinding?
    var activeBuildID: BuildID?
    /// Name (relative to the project root) of the PDF produced by the most
    /// recent successful build; surfaced to the agent as preview context.
    internal(set) var latestBuiltPDFName: String?
    /// Bottom console tab (Assistant / Terminal / Issues / Build Log).
    @Published var consoleSection: ConsoleSection = .assistant
    /// Left sidebar tab (Outline / Labels / BibTeX).
    @Published var sidebarSection: SidebarSection = .outline
    @Published var sidebarVisible = true
    @Published var bottomPanelVisible = false
    @Published var inspectorVisible = true
    /// File pinned as the build target; nil means "build the active document".
    @Published var pinnedBuildTarget: URL? {
        didSet { rebuildProjectTree() }
    }
    @Published internal(set) var automaticBuildTarget: URL? {
        didSet { rebuildProjectTree() }
    }
    /// Direct dependencies of the build target, nested under it in the
    /// project sidebar (`project_children`).
    @Published internal(set) var projectChildren: [URL] = [] {
        didSet { rebuildProjectTree() }
    }
    /// Sidebar file tree, rebuilt only when its inputs change — computing it
    /// in the view body re-sorted the whole tree on every keystroke.
    @Published private(set) var projectTree: [ProjectFileNode] = []
    @Published internal(set) var buildTargetMessage: String?
    /// Recently opened documents/projects, shown in the tab bar's + menu.
    @Published private(set) var recentDocuments: [URL] = []
    /// Build command shown in the console bar, remembered per project.
    @Published var buildCommandText = "xelatex -interaction=nonstopmode -synctex=1 {file}"
    /// Free-form terminal command run with ⌃⌘B, remembered per project.
    @Published var customCommandText = ""
    /// Accumulated terminal output shown in the console's Terminal tab.
    /// Live handle to the embedded SwiftTerm shell in the console's Terminal
    /// tab; status notices and saved commands are pushed into it.
    let terminalSession = TerminalSession()
    /// Document-structure data for the sidebar tabs, recomputed on each
    /// snapshot/file-list change.
    @Published private(set) var outlineItems: [DocumentOutlineItem] = []
    @Published private(set) var labelItems: [DocumentLabelItem] = []
    @Published private(set) var bibliographyItems: [BibliographyItem] = []
    @Published private(set) var todoItems: [DocumentTodoItem] = []
    /// Git Integration panel — driven by `WorkspaceModel+Git.swift`; nil
    /// until the first refresh reports whether the project is a repository.
    @Published internal(set) var gitStatus: GitStatus?
    @Published internal(set) var gitCommits: [GitCommit] = []
    @Published internal(set) var gitBranches: [String] = []
    @Published var gitCommitMessage = ""
    @Published internal(set) var gitBusy = false
    /// Separate spinner for `suggestCommitMessage` — a pi subprocess, not
    /// a git op, so pull/push stay enabled while it runs.
    @Published internal(set) var gitSuggestBusy = false
    @Published internal(set) var gitError: String?
    /// Commit rows expanded in the graph → their `git show --name-status`
    /// file lists, fetched lazily and cached per hash.
    @Published internal(set) var gitExpandedCommits: Set<String> = []
    @Published internal(set) var gitCommitFiles: [String: [GitCommitFile]] = [:]
    @Published internal(set) var gitCommitFilesBusy: Set<String> = []
    /// Commit diff covering the editor area (VSCode Source Control style).
    /// While set, the PDF inspector is hidden and its toggles disabled.
    @Published internal(set) var gitDiff: GitDiffSession?
    /// Project-wide \label keys and .bib citation keys feeding the editor's
    /// native completion. Rebuilt on the structure-refresh cadence through
    /// mtime-keyed caches — never reparsed per keystroke.
    @Published private(set) var projectLabels: Set<String> = []
    @Published private(set) var citationKeys: Set<String> = []

    var capabilityBroker: (any FileCapabilityBroker)?
    var capabilityLease: FileAccessLease?
    var registeredSessions: [DocumentSessionCore.DocumentSession] = []
    private var fileWatchers: [URL: DispatchSourceFileSystemObject] = [:]
    private var pendingDiskChecks: [URL: Task<Void, Never>] = [:]
    private var selectionObserver: NSObjectProtocol?

    init() { loadRecents() }

    var hasProject: Bool { projectURL != nil }
    private var didRestoreSession = false
    var canSave: Bool {
        documentSnapshot?.saveState == .dirty && capabilityLease?.access == .readWrite
    }
    var saveUnavailableReason: String? {
        guard let snapshot = documentSnapshot else { return "No document is open." }
        if snapshot.saveState == .conflicted {
            return "The file changed on disk. Resolve the conflict before saving."
        }
        if snapshot.saveState == .clean { return "The document has no unsaved changes." }
        if capabilityLease?.access != .readWrite { return "This project was opened read-only." }
        return nil
    }
    var buildUnavailableReason: String? {
        if isBuilding || canBuild { return nil }
        if let buildTargetMessage { return buildTargetMessage }
        if case let .unavailable(reason) = buildState { return reason }
        return "Open a project with a .tex source before building."
    }
    var canBuild: Bool {
        // A newly edited root directive is resolved again when Build is
        // pressed, so an unresolved target must not leave this button stuck.
        if case .ready = phase { return true }
        return false
    }

    func presentOpenPanel() {
        let panel = NSOpenPanel()
        panel.title = String(localized: "workspace.open_panel_title")
        panel.prompt = String(localized: "workspace.open")
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        panel.allowedContentTypes = [UTType(filenameExtension: "tex"), UTType(filenameExtension: "bib")].compactMap { $0 }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task { await open(url) }
    }

    // MARK: - Recents & file operations

    private static let recentsKey = "pitex.pref.workspace.recentDocuments"

    private func recordRecent(_ url: URL) {
        recentDocuments.removeAll { $0 == url }
        recentDocuments.insert(url, at: 0)
        if recentDocuments.count > 10 { recentDocuments = Array(recentDocuments.prefix(10)) }
        UserDefaults.standard.set(recentDocuments.map(\.path), forKey: Self.recentsKey)
    }

    func loadRecents() {
        recentDocuments = (UserDefaults.standard.stringArray(forKey: Self.recentsKey) ?? [])
            .map { URL(fileURLWithPath: $0) }
    }

    func clearRecents() {
        recentDocuments = []
        UserDefaults.standard.removeObject(forKey: Self.recentsKey)
    }

    /// File → New: creates an empty .tex file inside the open project and
    /// opens it. Without a project this is a no-op.
    func createDocument() async {
        guard let root = projectURL else { return }
        var index = 1
        var url = root.appendingPathComponent("untitled.tex")
        while FileManager.default.fileExists(atPath: url.path) {
            index += 1
            url = root.appendingPathComponent("untitled-\(index).tex")
        }
        do {
            try "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n"
                .write(to: url, atomically: true, encoding: .utf8)
            if !projectFiles.contains(url) {
                projectFiles.append(url)
                projectFiles.sort { $0.path.localizedStandardCompare($1.path) == .orderedAscending }
            }
            await activateDocument(url)
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    /// File → Save As…: writes the active document to a chosen path. When the
    /// destination lives inside the project it becomes the active document.
    func saveAs() async {
        guard let snapshot = documentSnapshot else { return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = activeDocumentURL?.lastPathComponent ?? "document.tex"
        panel.allowedContentTypes = [UTType(filenameExtension: "tex")].compactMap { $0 }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try snapshot.text.write(to: url, atomically: true, encoding: .utf8)
            if let root = projectURL,
               (try? Self.relativePath(for: url, root: root)) != nil {
                if !projectFiles.contains(url) {
                    projectFiles.append(url)
                    projectFiles.sort { $0.path.localizedStandardCompare($1.path) == .orderedAscending }
                }
                await activateDocument(url)
            }
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    /// Edit → Toggle Line Comment: inserts or removes a leading % on every
    /// line covered by the selection.
    func toggleLineComment() {
        guard let textView = environment?.editor.textView,
              let text = documentSnapshot?.text else { return }
        let nsText = text as NSString
        let range = textView.selectedRange()
        guard range.location + range.length <= nsText.length else { return }
        let lineRange = nsText.lineRange(for: range)
        let block = nsText.substring(with: lineRange)
        let lines = block.components(separatedBy: "\n")
        let contentLines = lines.filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
        let allCommented = !contentLines.isEmpty && contentLines.allSatisfy {
            $0.trimmingCharacters(in: .whitespaces).hasPrefix("%")
        }
        let transformed = lines.map { line -> String in
            guard !line.trimmingCharacters(in: .whitespaces).isEmpty else { return line }
            if allCommented {
                guard let percentIndex = line.firstIndex(of: "%") else { return line }
                var result = line
                result.remove(at: percentIndex)
                if result.first == " " { result.removeFirst() }
                return result
            }
            return "%" + line
        }.joined(separator: "\n")
        guard textView.shouldChangeText(in: lineRange, replacementString: transformed) else { return }
        textView.replaceCharacters(in: lineRange, with: transformed)
        textView.didChangeText()
    }

    func open(_ selectedURL: URL) async {
        let generation = await close()
        guard documentLoadGeneration == generation else { return }
        phase = .loading(selectedURL)
        do {
            let (root, files, initialURL, initialText) = try await Task.detached(priority: .userInitiated) {
                let values = try selectedURL.resourceValues(forKeys: [.isDirectoryKey])
                let isDirectory = values.isDirectory == true
                // Standardize so symlinked roots (/tmp → /private/tmp) canonicalize
                // identically to document paths in SyncTeX/path containment checks.
                var resolver = TeXProjectResolver()
                let root = (isDirectory ? selectedURL : resolver.projectRoot(for: selectedURL)).standardizedFileURL
                let files = try Self.discoverTexFiles(root: root, selected: selectedURL, isDirectory: isDirectory)
                let initialURL: URL
                if isDirectory {
                    guard let first = resolver.initialDocument(in: files) else { throw WorkspaceOpenError.noTexSources }
                    initialURL = first
                } else {
                    initialURL = selectedURL.standardizedFileURL
                }

                // NSOpenPanel grants immediate access. AppShell's broker then owns the durable lease.
                let initialText = try Self.readExactUTF8(initialURL)
                return (root, files, initialURL, initialText)
            }.value
            guard documentLoadGeneration == generation else { return }
            let relativePath = try Self.relativePath(for: initialURL, root: root)
            let file = ProjectFile(
                documentID: try StableDocumentID(rawValue: Self.documentID(for: relativePath.rawValue)),
                path: relativePath
            )
            let session = try await registry.open(
                projectRoot: root,
                file: file,
                initialText: initialText,
                diskBaselineHash: .hashing(initialText)
            )
            guard documentLoadGeneration == generation else { return }
            projectURL = root
            projectFiles = files
            registeredSessions = [session]
            loadProjectCommands(root: root)
            refreshGit()
            let port = NativeDocumentSessionPort(session: session) { [weak self] snapshot in
                guard let self, activeDocumentURL == initialURL else { return }
                documentSnapshot = snapshot
            }
            let appEnvironment = try await AppShell.make(documentSession: port)
            guard documentLoadGeneration == generation else { return }
            let capability: FileCapability
            do {
                capability = try await appEnvironment.files.issueCapability(
                    for: selectedURL,
                    access: .readWrite
                )
            } catch {
                capability = try await appEnvironment.files.issueCapability(
                    for: selectedURL,
                    access: .readOnly
                )
            }
            let lease = try await appEnvironment.files.beginAccess(to: capability)
            guard documentLoadGeneration == generation else {
                try? await appEnvironment.files.endAccess(lease)
                return
            }
            capabilityBroker = appEnvironment.files
            capabilityLease = lease
            let verifiedText = try await Task.detached(priority: .userInitiated) {
                try Self.readExactUTF8(initialURL)
            }.value
            let snapshot = await session.snapshot()
            guard documentLoadGeneration == generation else { return }
            guard verifiedText == initialText else {
                throw WorkspaceOpenError.changedWhileOpening
            }

            // Created before `environment` publishes so no body re-evaluation
            // can build the editor with a permanently-nil `completion:` — a
            // struct input evaluated once per makeNSView pass.
            let completion = GhostCompletionCoordinator()
            completion.contextProvider = { [weak self] in
                self?.completionContext() ?? GhostCompletionCoordinator.Context()
            }
            completion.attach(to: appEnvironment.editor)
            self.completion = completion

            environment = appEnvironment
            activeDocumentURL = initialURL
            openDocuments = [initialURL]
            documentSnapshot = snapshot
            refreshBuildTarget()
            buildState = .unavailable("No build has run yet for this project.")
            syncTeXState = .unavailable(
                "SyncTeX is unavailable until a successful build produces matching metadata."
            )

            let coordinator = AgentCoordinator(environment: appEnvironment)
            coordinator.contextProvider = { [weak self] in self?.agentContext() ?? AgentContextSnapshot() }
            coordinator.persistDirtySessions = { [weak self] in
                guard let self else { return "The workspace is not available." }
                return await self.persistDirtySessions()
            }
            coordinator.agentActivityDidFinish = { [weak self] in
                await self?.refreshAfterAgentActivity()
            }
            coordinator.attachActiveDocument = settings.aiAttachDefault
            coordinator.preferredModelID = settings.aiDefaultModel
            coordinator.historyLimit = settings.chatHistoryLimit
            agent = coordinator
            highlighter.attach(to: appEnvironment.editor, fileExtension: initialURL.pathExtension)
            attachCompletion(to: appEnvironment.editor)
            startWatcher(for: initialURL)
            coordinator.prepare()
            phase = .ready
            await restoreBuiltPreview()
            recordRecent(selectedURL)
        } catch {
            guard documentLoadGeneration == generation else { return }
            let closedGeneration = await close()
            guard documentLoadGeneration == closedGeneration else { return }
            phase = .failed(error.localizedDescription)
        }
    }

    func activateDocument(_ url: URL) async {
        guard url != activeDocumentURL,
              let root = projectURL,
              projectFiles.contains(url) else { return }
        // Activating a real document dismisses a commit-diff overlay —
        // the user asked to see the document, not the diff.
        gitDiff = nil
        // Keep the workspace mounted when switching sources: a loading phase
        // destroys the split view and PDF view, losing their size and position.
        let generation = UUID()
        documentLoadGeneration = generation
        do {
            let text = try await Task.detached(priority: .userInitiated) {
                try Self.readExactUTF8(url)
            }.value
            guard documentLoadGeneration == generation, projectURL == root else { return }
            let relativePath = try Self.relativePath(for: url, root: root)
            let file = ProjectFile(
                documentID: try StableDocumentID(rawValue: Self.documentID(for: relativePath.rawValue)),
                path: relativePath
            )
            let session = try await registry.open(
                projectRoot: root,
                file: file,
                initialText: text,
                diskBaselineHash: .hashing(text)
            )
            guard documentLoadGeneration == generation, projectURL == root else { return }
            if !registeredSessions.contains(where: { $0 === session }) {
                registeredSessions.append(session)
            }
            let port = NativeDocumentSessionPort(session: session) { [weak self] snapshot in
                guard let self, activeDocumentURL == url else { return }
                documentSnapshot = snapshot
            }
            let appEnvironment = try await AppShell.make(documentSession: port)
            let snapshot = await session.snapshot()
            guard documentLoadGeneration == generation, projectURL == root else { return }
            environment = appEnvironment
            activeDocumentURL = url
            if !openDocuments.contains(url) { openDocuments.append(url) }
            documentSnapshot = snapshot
            refreshBuildTarget()
            completion?.attach(to: appEnvironment.editor)
            highlighter.attach(to: appEnvironment.editor, fileExtension: url.pathExtension)
            attachCompletion(to: appEnvironment.editor)
            startWatcher(for: url)
            phase = .ready
            await restoreBuiltPreview()
        } catch {
            guard documentLoadGeneration == generation, projectURL == root else { return }
            phase = .failed(error.localizedDescription)
        }
    }

    /// Debounced auto-save: when enabled in Settings → Editor, a dirty
    /// document is written to disk after the configured idle delay. Each new
    /// snapshot resets the timer so saves never interrupt typing.
    private var autosaveTask: Task<Void, Never>?

    private func scheduleAutosave() {
        autosaveTask?.cancel()
        autosaveTask = nil
        guard settings.autoSave,
              documentSnapshot?.saveState == .dirty,
              capabilityLease?.access == .readWrite else { return }
        let delay = max(settings.autoSaveDelay, 1)
        autosaveTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .seconds(delay))
            guard !Task.isCancelled else { return }
            await self?.save()
        }
    }

    func save() async {
        guard canSave,
              let url = activeDocumentURL,
              let snapshot = documentSnapshot,
              let session = registeredSessions.first(where: { $0.path == snapshot.path }) else { return }
        do {
            let outcome = FoundationAtomicDocumentStore().save(
                text: snapshot.text,
                to: url,
                expectedBaselineHash: snapshot.diskBaselineHash
            )
            switch outcome {
            case let .saved(document):
                let saved = try await session.apply(
                    .commitSave(writtenDiskHash: document.hash),
                    expectedRevision: snapshot.revision
                )
                documentSnapshot = saved
                refreshGit()
            case let .staleBaseline(conflict):
                let observedHash = conflict.observedDisk?.hash ?? .hashing("")
                let conflicted = try await session.apply(
                    .recordSaveConflict(observedDiskHash: observedHash),
                    expectedRevision: snapshot.revision
                )
                documentSnapshot = conflicted
                syncTeXState = .stale("The source changed on disk; SyncTeX locations may be stale.")
            case let .permissionFailure(path):
                throw WorkspaceOpenError.savePermissionDenied(path)
            case let .interruptedWrite(path):
                throw WorkspaceOpenError.saveInterrupted(path)
            }
        } catch let error as DocumentSessionError {
            switch error {
            case .staleRevision:
                documentSnapshot = await session.snapshot()
            default:
                phase = .failed(error.localizedDescription)
            }
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    /// Resolves an external-change conflict by adopting either the disk content
    /// or the in-memory content. Both versions are preserved through the
    /// session's revision history; nothing is silently dropped.
    func resolveConflict(useDiskVersion: Bool) async {
        guard let snapshot = documentSnapshot, snapshot.saveState == .conflicted,
              let url = activeDocumentURL,
              let session = registeredSessions.first(where: { $0.path == snapshot.path }) else { return }
        do {
            if useDiskVersion {
                let diskText = try Self.readExactUTF8(url)
                let updated = try await session.apply(
                    .resolveConflict(text: diskText, diskBaselineHash: .hashing(diskText)),
                    expectedRevision: snapshot.revision
                )
                documentSnapshot = updated
                await environment?.editor.refreshFromSession()
            } else {
                let outcome = FoundationAtomicDocumentStore().save(
                    text: snapshot.text,
                    to: url,
                    expectedBaselineHash: snapshot.conflict.flatMap { conflict in
                        if case let .externalModification(_, observed) = conflict { return observed }
                        if case let .saveCollision(_, observed) = conflict { return observed }
                        return nil
                    } ?? snapshot.diskBaselineHash
                )
                guard case let .saved(document) = outcome else {
                    phase = .failed("The conflicted file could not be written to disk.")
                    return
                }
                let updated = try await session.apply(
                    .commitSave(writtenDiskHash: document.hash),
                    expectedRevision: snapshot.revision
                )
                documentSnapshot = updated
            }
        } catch {
            phase = .failed("The conflict could not be resolved: \(error.localizedDescription)")
        }
    }

    func closeDocument(_ url: URL) async {
        guard let root = projectURL else { return }
        stopWatcher(for: url)
        if let index = registeredSessions.firstIndex(where: { session in
            guard let path = try? Self.relativePath(for: url, root: root) else { return false }
            return session.path == path
        }) {
            let session = registeredSessions.remove(at: index)
            _ = try? await registry.close(projectRoot: root, session: session)
        }
        openDocuments.removeAll { $0 == url }
        if activeDocumentURL == url {
            if let next = openDocuments.first {
                activeDocumentURL = nil
                await activateDocument(next)
            } else {
                environment = nil
                activeDocumentURL = nil
                documentSnapshot = nil
            }
        }
    }

    @discardableResult
    func close() async -> UUID {
        let generation = UUID()
        documentLoadGeneration = generation
        wordCount = 0
        for url in fileWatchers.keys { stopWatcher(for: url) }
        for task in pendingDiskChecks.values { task.cancel() }
        pendingDiskChecks.removeAll()
        agent?.shutdown()
        completion?.shutdown()
        highlighter.detach()
        let brokerToClose = capabilityBroker
        let leaseToClose = capabilityLease
        let root = projectURL
        let sessions = registeredSessions
        environment = nil
        capabilityBroker = nil
        capabilityLease = nil
        registeredSessions.removeAll()
        projectURL = nil
        projectFiles = []
        openDocuments = []
        activeDocumentURL = nil
        documentSnapshot = nil
        agent = nil
        completion = nil
        latestBuiltPDFName = nil
        syncTeXBinding = nil
        pinnedBuildTarget = nil
        automaticBuildTarget = nil
        projectChildren = []
        buildTargetMessage = nil
        consoleSection = .assistant
        outlineItems = []
        labelItems = []
        bibliographyItems = []
        todoItems = []
        gitStatus = nil
        gitCommits = []
        gitBranches = []
        gitCommitMessage = ""
        gitSuggestBusy = false
        gitError = nil
        gitExpandedCommits = []
        gitCommitFiles = [:]
        gitCommitFilesBusy = []
        gitDiff = nil
        bibliographyCache = nil
        todoCache.removeAll()
        projectLabels = []
        citationKeys = []
        projectLabelCache = nil
        structureTask?.cancel()
        structureTask = nil
        phase = .noProject
        if let root {
            for session in sessions {
                _ = try? await registry.close(projectRoot: root, session: session)
            }
        }
        if let brokerToClose, let leaseToClose {
            try? await brokerToClose.endAccess(leaseToClose)
        }
        return generation
    }

    /// Reveals the assistant tab in the bottom console; used by the toolbar
    /// wand button and Edit → Send Selection to AI Assistant.
    func revealAssistant() {
        bottomPanelVisible = true
        consoleSection = .assistant
    }

    /// Wand-button toggle: hides the console when the assistant tab is the
    /// one already showing, otherwise reveals it on the assistant tab.
    func toggleAssistant() {
        if bottomPanelVisible && consoleSection == .assistant {
            bottomPanelVisible = false
        } else {
            revealAssistant()
        }
    }

    /// Sends the current editor selection to the assistant composer so the
    /// user can ask about it directly (Edit → Send Selection to AI Assistant).
    func sendSelectionToAssistant() {
        guard let range = environment?.editor.selectedRange, range.length > 0,
              let text = documentSnapshot?.text,
              range.location + range.length <= (text as NSString).length else { return }
        let selection = (text as NSString).substring(with: range)
        revealAssistant()
        agent?.insertIntoComposer("```\n\(selection)\n```\n")
    }

    /// Watches the editor's selection changes and mirrors the dragged range
    /// into the assistant's selection-attachment chip — the agent always
    /// knows what the user is pointing at, like the reference IDE's
    /// selection-to-chat flow.
    private func observeEditorSelection() {
        if let selectionObserver { NotificationCenter.default.removeObserver(selectionObserver) }
        selectionObserver = nil
        agent?.updateSelectionAttachment(nil)
        guard let textView = environment?.editor.textView else { return }
        selectionObserver = NotificationCenter.default.addObserver(
            forName: NSTextView.didChangeSelectionNotification,
            object: textView,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor [weak self] in self?.syncSelectionAttachment() }
        }
    }

    private func syncSelectionAttachment() {
        guard let agent,
              let range = environment?.editor.selectedRange,
              range.length > 0,
              let text = documentSnapshot?.text,
              range.location + range.length <= (text as NSString).length else {
            agent?.updateSelectionAttachment(nil)
            return
        }
        let nsText = text as NSString
        let selected = nsText.substring(with: range)
        // One scan to the selection end: the previous substring+split pair
        // allocated two document-sized arrays per selection change.
        var startLine = 1
        var endLine = 1
        let end = range.location + range.length
        for index in 0..<end where nsText.character(at: index) == 0x0A {
            endLine += 1
            if index < range.location { startLine += 1 }
        }
        let path = documentSnapshot?.path.rawValue ?? activeDocumentURL?.lastPathComponent ?? "document"
        agent.updateSelectionAttachment(AgentSelectionAttachment(
            path: path, startLine: startLine, endLine: endLine, text: selected
        ))
    }

    /// The fire-time gates for inline completion — setting toggle, .tex
    /// extension, project root for the subprocess cwd, and the file name
    /// that lands in the prompt header.
    private func completionContext() -> GhostCompletionCoordinator.Context {
        var context = GhostCompletionCoordinator.Context()
        context.projectRoot = projectURL
        context.enabled = settings.aiAutocompletion
        context.isTeX = activeDocumentURL?.pathExtension.lowercased() == "tex"
        context.fileName = documentSnapshot?.path.rawValue
            ?? activeDocumentURL?.lastPathComponent
            ?? "document.tex"
        return context
    }

    /// The live context folded into every agent prompt: which document is
    /// open, its text, the selection, the project file list, and the most
    /// recently built PDF preview.
    private func agentContext() -> AgentContextSnapshot {
        var context = AgentContextSnapshot()
        context.projectRoot = projectURL
        context.activePath = documentSnapshot?.path.rawValue
        context.activeText = documentSnapshot?.text
        if let text = documentSnapshot?.text,
           let range = environment?.editor.selectedRange,
           range.length > 0,
           range.location + range.length <= (text as NSString).length {
            context.selectionText = (text as NSString).substring(with: range)
        }
        if let root = projectURL {
            context.projectFiles = projectFiles.compactMap {
                try? Self.relativePath(for: $0, root: root).rawValue
            }
        }
        if case let .succeeded(pdf, _) = buildState {
            context.pdfData = pdf
            context.pdfPath = latestBuiltPDFName
        }
        return context
    }

    // MARK: - SyncTeX

    func refreshSyncTeXBinding(pdfURL: URL) async {
        guard let root = projectURL else {
            syncTeXState = .unavailable("SyncTeX is unavailable until a successful build produces matching metadata.")
            return
        }
        do {
            syncTeXBinding = try await syncTeXRunner.refreshBinding(
                projectRoot: root,
                pdfURL: pdfURL,
                // A restored/existing PDF has no build identifier; the
                // .synctex fingerprint still keeps stale results failing closed.
                buildID: activeBuildID?.rawValue ?? "existing-pdf"
            )
            syncTeXState = .current
        } catch {
            syncTeXBinding = nil
            syncTeXState = .unavailable("SyncTeX metadata could not be loaded for this build.")
        }
    }

    /// Lazily binds SyncTeX to the PDF on disk so Cmd-click navigation works
    /// for documents rendered before this session as well as fresh builds.
    private func ensureSyncTeXBinding() async {
        guard !isBuilding else { return }
        await restoreBuiltPreview()
    }

    /// Reopen the main document's PDF, including when a chapter or .bib file
    /// was opened first. Switching within that document keeps its preview.
    private func restoreBuiltPreview() async {
        guard !isBuilding, let root = projectURL, let source = buildSourceURL(),
              let relative = try? Self.relativePath(for: source, root: root).rawValue else { return }
        let name = (relative as NSString).deletingPathExtension + ".pdf"
        if latestBuiltPDFName == name, syncTeXBinding != nil { return }
        let pdf = root.appendingPathComponent(name)
        guard let data = try? Data(contentsOf: pdf), data.starts(with: Data("%PDF".utf8)) else {
            if latestBuiltPDFName != name {
                latestBuiltPDFName = nil
                syncTeXBinding = nil
                buildState = .unavailable("Build the main document to create its PDF preview.")
                syncTeXState = .unavailable("SyncTeX requires a completed build of the main document.")
            }
            return
        }
        latestBuiltPDFName = name
        buildState = .succeeded(pdf: data, log: buildLogText)
        await refreshSyncTeXBinding(pdfURL: pdf)
    }

    func invalidateSyncTeXForBuild() {
        syncTeXBinding = nil
        syncTeXState = .unavailable("SyncTeX will be refreshed after the build completes.")
    }

    /// Forward sync: editor selection → PDF highlight.
    func syncForward() async {
        guard let text = documentSnapshot?.text,
              let editor = environment?.editor else {
            syncTeXState = .unavailable("SyncTeX requires a completed build.")
            return
        }
        let cursor = min(editor.selectedRange.location, (text as NSString).length)
        let nsText = text as NSString
        let line = nsText.substring(to: cursor).components(separatedBy: "\n").count
        let lineStart = nsText.range(of: "\n", options: .backwards, range: NSRange(location: 0, length: cursor))
        let column = lineStart.location == NSNotFound ? cursor : cursor - lineStart.location - 1
        await syncForward(line: line, column: max(column, 0))
    }

    /// Forward sync at an explicit position — used by the editor's
    /// Cmd-click gesture (the reference editor's Ctrl-click equivalent).
    func syncForward(line: Int, column: Int) async {
        await ensureSyncTeXBinding()
        guard let binding = syncTeXBinding,
              let url = activeDocumentURL else {
            syncTeXState = .unavailable("SyncTeX requires a completed build.")
            return
        }
        do {
            let match = try await syncTeXRunner.forward(
                binding: binding,
                sourceURL: url,
                line: line,
                column: column
            )
            NotificationCenter.default.post(
                name: .syncTeXHighlightRequested,
                object: nil,
                userInfo: [
                    "page": match.pdf.page,
                    "x": match.h,
                    "y": match.v,
                    "width": match.width,
                    "height": match.height,
                ]
            )
        } catch let error as SyncTeXQueryError {
            switch error {
            case .staleResult: syncTeXState = .stale("SyncTeX results no longer match the current PDF.")
            case .ambiguousMatch(let count): syncTeXState = .ambiguous("\(count) matches; the source is ambiguous.")
            case .noMatch: syncTeXState = .stale("No SyncTeX location matched the cursor position.")
            default: syncTeXState = .stale("SyncTeX output could not be parsed.")
            }
        } catch {
            syncTeXState = .stale("SyncTeX lookup failed.")
        }
    }

    /// Inverse sync: PDF click → editor position.
    func syncInverse(page: Int, point: SyncTeXCore.PDFPoint) async {
        await ensureSyncTeXBinding()
        guard let binding = syncTeXBinding, let root = projectURL else {
            NSLog("[SyncTeX] inverse dropped: no binding/root")
            return
        }
        do {
            let match = try await syncTeXRunner.inverse(binding: binding, page: page, point: point)
            NSLog("[SyncTeX] inverse page=\(page) x=\(point.x) y=\(point.y) -> \(match.source.path.value):\(match.source.line)")
            let fileURL = root.appendingPathComponent(match.source.path.value).standardizedFileURL
            let resolved = fileURL.resolvingSymlinksInPath()
            if !projectFiles.contains(where: { $0.resolvingSymlinksInPath() == resolved }) {
                projectFiles.append(fileURL)
                projectFiles.sort { $0.path.localizedStandardCompare($1.path) == .orderedAscending }
            }
            if resolved != activeDocumentURL?.resolvingSymlinksInPath() {
                await activateDocument(fileURL)
            }
            guard resolved == activeDocumentURL?.resolvingSymlinksInPath() else { return }
            jumpTo(line: match.source.line, column: match.source.column, highlight: settings.inverseSyncHighlight)
        } catch let error as SyncTeXQueryError {
            NSLog("[SyncTeX] inverse error: \(error)")
            switch error {
            case .staleResult: syncTeXState = .stale("SyncTeX results no longer match the current PDF.")
            case .ambiguousMatch(let count): syncTeXState = .ambiguous("\(count) matches; the PDF position is ambiguous.")
            case .noMatch: break
            default: break
            }
        } catch {
            NSLog("[SyncTeX] inverse error: \(error)")
        }
    }

    func jumpTo(line: Int, column: Int = 0, highlight: Bool = false) {
        guard let editor = environment?.editor else { return }
        // The live text view is authoritative — documentSnapshot trails edits
        // made since the last flush, which would misplace the caret.
        let nsText = editor.textView.string as NSString
        var location = 0
        var currentLine = 1
        while currentLine < line {
            let nextRange = nsText.range(of: "\n", options: [], range: NSRange(location: location, length: nsText.length - location))
            guard nextRange.location != NSNotFound else { break }
            location = nextRange.location + 1
            currentLine += 1
        }
        var contentsEnd = location
        nsText.getLineStart(nil, end: nil, contentsEnd: &contentsEnd, for: NSRange(location: location, length: 0))
        let target = location + min(max(column, 0), contentsEnd - location)
        if let folds = editor.textView.layoutManager?.delegate as? FoldEngine {
            for region in folds.regions where region.folded && region.hiddenLineRange.contains(currentLine - 1) {
                folds.unfold(atLine: region.headerLine)
            }
        }
        editor.revealSelection(NSRange(location: target, length: 0), highlight: highlight)
    }

    // MARK: - External change monitoring

    private func startWatcher(for url: URL) {
        stopWatcher(for: url)
        let descriptor = Darwin.open(url.path, O_EVTONLY)
        guard descriptor >= 0 else { return }
        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: descriptor,
            eventMask: [.write, .delete, .rename, .attrib],
            queue: .main
        )
        source.setEventHandler { [weak self] in
            Task { @MainActor in self?.fileChangedOnDisk(url) }
        }
        source.setCancelHandler { Darwin.close(descriptor) }
        source.resume()
        fileWatchers[url] = source
    }

    private func stopWatcher(for url: URL) {
        fileWatchers.removeValue(forKey: url)?.cancel()
    }

    private func fileChangedOnDisk(_ url: URL) {
        // Coalesce write bursts (the agent's edit tool may write several
        // times in quick succession) so the session compares against the
        // final on-disk content once, not every intermediate state.
        pendingDiskChecks[url]?.cancel()
        pendingDiskChecks[url] = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(120))
            guard !Task.isCancelled else { return }
            await processDiskChange(url)
            pendingDiskChecks[url] = nil
            refreshGit()
        }
        // The watcher fd is invalidated by rename/delete; re-arm it.
        if FileManager.default.fileExists(atPath: url.path) {
            startWatcher(for: url)
        }
    }

    /// Applies the on-disk state of `url` to its document session. A clean
    /// session (no unsaved edits) adopts the disk content directly so
    /// agent-made edits surface in the editor without a conflict round-trip;
    /// a dirty session keeps its edits and flags a conflict instead.
    private func processDiskChange(_ url: URL) async {
        guard let root = projectURL,
              let session = registeredSessions.first(where: { session in
                  guard let path = try? Self.relativePath(for: url, root: root) else { return false }
                  return session.path == path
              }) else { return }
        guard let diskText = try? Self.readExactUTF8(url) else {
            // Deleted or unreadable: never replace content with nothing. Flag
            // a conflict only when unsaved in-memory edits are at stake.
            let snapshot = await session.snapshot()
            if snapshot.saveState != .clean, snapshot.conflict == nil,
               let updated = try? await session.apply(
                   .recordExternalChange(observedDiskHash: .hashing("")),
                   expectedRevision: snapshot.revision
               ), snapshot.path == documentSnapshot?.path {
                documentSnapshot = updated
            }
            return
        }
        let observedHash = DiskContentHash.hashing(diskText)
        let snapshot = await session.snapshot()
        guard observedHash != snapshot.diskBaselineHash, snapshot.conflict == nil else { return }
        // "Confirm before overwriting external changes" off → adopt the disk
        // version even over unsaved in-memory edits; on → flag a conflict.
        let adoptDisk = snapshot.saveState == .clean || !settings.confirmOverwrite
        if adoptDisk {
            if let updated = try? await session.apply(
                .resolveConflict(text: diskText, diskBaselineHash: observedHash),
                expectedRevision: snapshot.revision
            ), updated.path == documentSnapshot?.path {
                documentSnapshot = updated
                await environment?.editor.refreshFromSession()
            }
        } else if let updated = try? await session.apply(
            .recordExternalChange(observedDiskHash: observedHash),
            expectedRevision: snapshot.revision
        ), snapshot.path == documentSnapshot?.path {
            documentSnapshot = updated
        }
        // A watched non-active file's labels/citations changed on disk too —
        // the mtime key in the structure caches picks it up on this pass.
        scheduleStructureRefresh()
    }

    // MARK: - Agent integration

    /// Persists every dirty open document so the agent's file tools observe
    /// the same content the editor shows. Returns a user-facing problem when
    /// a document cannot be persisted (e.g. unresolved conflict).
    @discardableResult
    func persistDirtySessions() async -> String? {
        guard let root = projectURL else { return nil }
        for session in registeredSessions {
            let sessionSnapshot = await session.snapshot()
            if sessionSnapshot.saveState == .conflicted {
                return "The file \(sessionSnapshot.path.rawValue) has an unresolved external-change conflict. Resolve it before using the agent."
            }
            guard sessionSnapshot.saveState == .dirty else { continue }
            let fileURL = root.appendingPathComponent(sessionSnapshot.path.rawValue)
            let outcome = FoundationAtomicDocumentStore().save(
                text: sessionSnapshot.text,
                to: fileURL,
                expectedBaselineHash: sessionSnapshot.diskBaselineHash
            )
            switch outcome {
            case let .saved(document):
                _ = try? await session.apply(
                    .commitSave(writtenDiskHash: document.hash),
                    expectedRevision: sessionSnapshot.revision
                )
            case .staleBaseline:
                _ = try? await session.apply(
                    .recordSaveConflict(observedDiskHash: .hashing((try? String(contentsOf: fileURL, encoding: .utf8)) ?? "")),
                    expectedRevision: sessionSnapshot.revision
                )
                return "The file \(sessionSnapshot.path.rawValue) changed on disk while preparing the agent. Resolve the conflict first."
            case .permissionFailure, .interruptedWrite:
                return "The file \(sessionSnapshot.path.rawValue) could not be saved before running the agent."
            }
        }
        if let snapshot = documentSnapshot {
            documentSnapshot = await registeredSessions
                .first(where: { $0.path == snapshot.path })?.snapshot() ?? snapshot
        }
        return nil
    }

    /// After an agent run completes: adopt its edits into any session whose
    /// disk file changed while clean, and pick up source files the agent
    /// created so they appear in the project outline.
    private func refreshAfterAgentActivity() async {
        guard let root = projectURL else { return }
        for session in registeredSessions {
            let url = root.appendingPathComponent(session.path.rawValue)
            await processDiskChange(url)
        }
        if let discovered = try? Self.discoverTexFiles(
            root: root, selected: root, isDirectory: true
        ) {
            let known = Set(projectFiles)
            let additions = discovered.filter { !known.contains($0) }
            if !additions.isEmpty {
                projectFiles.append(contentsOf: additions)
                projectFiles.sort { $0.path.localizedStandardCompare($1.path) == .orderedAscending }
            }
        }
    }

    // MARK: - Sidebar structure

    /// Debounced like the syntax highlighter: parsing outline/labels is
    /// O(document) and bibliography reads hit disk, so neither belongs on
    /// the per-keystroke path.
    private var structureTask: Task<Void, Never>?

    private func scheduleStructureRefresh() {
        structureTask?.cancel()
        structureTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(120))
            guard !Task.isCancelled else { return }
            self?.refreshStructure()
        }
    }

    /// Rebuilds the Outline / Labels / BibTeX / TODOs sidebar data from the
    /// active document text, the project's .bib files, and its .tex files.
    private func refreshStructure() {
        let snapshot = documentSnapshot
        let root = projectURL
        let text = snapshot?.text ?? ""
        Task { @MainActor [weak self] in
            let (outline, labels, count) = await Task.detached(priority: .userInitiated) {
                let starts = Self.lineStartOffsets(text as NSString)
                return (Self.parseOutline(text, lineStarts: starts),
                        Self.parseLabels(text, lineStarts: starts),
                        text.split { $0 == " " || $0 == "\n" || $0 == "\t" }.count)
            }.value
            guard let self, projectURL == root,
                  documentSnapshot?.documentID == snapshot?.documentID,
                  documentSnapshot?.revision == snapshot?.revision else { return }
            if outlineItems != outline { outlineItems = outline }
            if labelItems != labels { labelItems = labels }
            if wordCount != count { wordCount = count }
            refreshBibliography()
            refreshTodos()
            refreshCompletionKeys(text: text)
        }
    }

    /// Completion key sets: labels merge every project .tex file
    /// (mtime-cached disk reads) with the active buffer, which may hold
    /// unsaved edits; citations merge every project .bib file via
    /// `bibliographyItems` with the active buffer when a .bib is open.
    private func refreshCompletionKeys(text: String) {
        let activeIsTex = activeDocumentURL?.pathExtension.lowercased() == "tex"
        var labels = cachedProjectLabels()
        if activeIsTex {
            labels.formUnion(labelItems.map(\.name))
        }
        if projectLabels != labels { projectLabels = labels }

        var citations = Set(bibliographyItems.map(\.key))
        if activeDocumentURL?.pathExtension.lowercased() == "bib" {
            citations.formUnion(Self.parseBibliographyKeys(text))
        }
        if citationKeys != citations { citationKeys = citations }
    }

    /// mtime-keyed cache on the same contract as `bibliographyCache` —
    /// project .tex files are only re-read when the file list or a
    /// modification date changes. The active document is excluded: its
    /// in-memory buffer is the authority for its labels.
    private var projectLabelCache: (key: [String], labels: Set<String>)?

    private func cachedProjectLabels() -> Set<String> {
        let activePath = activeDocumentURL?.standardizedFileURL.path
        let files = projectFiles.filter {
            $0.pathExtension.lowercased() == "tex"
                && $0.standardizedFileURL.path != activePath
        }
        let key = files.map { file in
            let modified = (try? file.resourceValues(forKeys: [.contentModificationDateKey])
                .contentModificationDate?.timeIntervalSince1970) ?? 0
            return "\(file.path)#\(modified)"
        }
        if let cache = projectLabelCache, cache.key == key {
            return cache.labels
        }
        var labels = Set<String>()
        for file in files {
            guard let fileText = try? Self.readExactUTF8(file) else { continue }
            labels.formUnion(Self.parseLabels(fileText).map(\.name))
        }
        projectLabelCache = (key, labels)
        return labels
    }

    /// The editor's completion source: shared context detection plus the
    /// project-wide key sets. Reattached per adapter because each document
    /// activation builds a new EditorMacAdapter.
    private func attachCompletion(to editor: EditorMacAdapter) {
        editor.completionSource = { [weak self] text, caretUTF16Offset in
            self?.editorCompletions(in: text, caretUTF16Offset: caretUTF16Offset)
        }
    }

    /// Detects the caret's completion context in shared LanguageCore and
    /// maps the project key sets into native candidates. The returned
    /// UTF-16 range covers only the current prefix/token, so the popup
    /// inserts the missing tail rather than re-typing the whole command.
    func editorCompletions(
        in text: String,
        caretUTF16Offset: Int
    ) -> (range: NSRange, candidates: [String])? {
        guard let context = CompletionContextDetector.context(
            in: text,
            caretUTF16Offset: caretUTF16Offset
        ) else { return nil }
        let items = LanguageIndex.completions(
            for: context,
            labels: projectLabels,
            citationKeys: citationKeys
        )
        guard !items.isEmpty else { return nil }
        return (
            NSRange(
                location: context.prefixUTF16Offset,
                length: max(0, caretUTF16Offset - context.prefixUTF16Offset)
            ),
            items.map(\.text)
        )
    }

    /// .bib items come from disk, not the editor buffer: re-parse only when
    /// the file list or a .bib modification date changes instead of reading
    /// every .bib on each document snapshot.
    private var bibliographyCache: (key: [String], items: [BibliographyItem])?

    private func refreshBibliography() {
        let files = projectFiles.filter { $0.pathExtension.lowercased() == "bib" }
        let key = files.map { file in
            let modified = (try? file.resourceValues(forKeys: [.contentModificationDateKey])
                .contentModificationDate?.timeIntervalSince1970) ?? 0
            return "\(file.path)#\(modified)"
        }
        if let cache = bibliographyCache, cache.key == key {
            if bibliographyItems != cache.items { bibliographyItems = cache.items }
            return
        }
        let items = Self.parseBibliography(files: files, root: projectURL)
        bibliographyCache = (key, items)
        if bibliographyItems != items { bibliographyItems = items }
    }

    /// The active file's todos parse from the live snapshot (keyed by
    /// revision) while the rest keep mtime-keyed disk parses, so a typing
    /// refresh re-reads only the file that changed.
    private var todoCache: [URL: (key: String, items: [DocumentTodoItem])] = [:]

    private func refreshTodos() {
        let active = activeDocumentURL
        let revision = documentSnapshot?.revision ?? 0
        let texFiles = projectFiles.filter { $0.pathExtension.lowercased() == "tex" }
        var items: [DocumentTodoItem] = []
        for file in texFiles {
            let isActive = file == active
            let key: String
            if isActive {
                key = "r\(revision)"
            } else {
                let modified = (try? file.resourceValues(forKeys: [.contentModificationDateKey])
                    .contentModificationDate?.timeIntervalSince1970) ?? 0
                key = "m\(modified)"
            }
            if let entry = todoCache[file], entry.key == key {
                items.append(contentsOf: entry.items)
                continue
            }
            let name = (try? Self.relativePath(for: file, root: projectURL ?? file.deletingLastPathComponent()).rawValue)
                ?? file.lastPathComponent
            let text = isActive ? documentSnapshot?.text : try? Self.readExactUTF8(file)
            let parsed = text.map { Self.parseTodos($0, file: name, url: file) } ?? []
            todoCache[file] = (key, parsed)
            items.append(contentsOf: parsed)
        }
        let alive = Set(texFiles)
        todoCache = todoCache.filter { alive.contains($0.key) }
        if todoItems != items { todoItems = items }
    }

    // MARK: - TODO sidebar

    /// Sidebar `+` — inserts `% TODO: ` at the caret of the active .tex
    /// document (through the text view so undo and the session see it), or
    /// appends it to the build target when the active file isn't a source.
    var canAddTodo: Bool {
        if activeDocumentURL?.pathExtension.lowercased() == "tex" { return true }
        return buildSourceURL()?.pathExtension.lowercased() == "tex"
    }

    func addTodo() {
        if let textView = environment?.editor.textView,
           activeDocumentURL?.pathExtension.lowercased() == "tex" {
            let range = textView.selectedRange()
            guard textView.shouldChangeText(in: range, replacementString: "% TODO: ") else { return }
            textView.replaceCharacters(in: range, with: "% TODO: ")
            textView.didChangeText()
            return
        }
        guard let target = buildSourceURL(),
              target.pathExtension.lowercased() == "tex" else { return }
        Task { await appendTodoComment(to: target) }
    }

    /// Click-to-jump: activate the item's file when needed, then land the
    /// caret on the comment's line (the inverse-SyncTeX two-step).
    func openTodo(_ item: DocumentTodoItem) async {
        if item.url != activeDocumentURL {
            await activateDocument(item.url)
        }
        guard item.url == activeDocumentURL else { return }
        jumpTo(line: item.line, column: 0)
    }

    func toggleTodo(_ item: DocumentTodoItem) { editTodo(item, .toggleDone) }
    func renameTodo(_ item: DocumentTodoItem, to text: String) { editTodo(item, .rename(text)) }
    func removeTodo(_ item: DocumentTodoItem) { editTodo(item, .delete) }

    /// What a todo-row mutation does to its comment line.
    private enum TodoLineEdit {
        case toggleDone
        case rename(String)
        case delete
    }

    /// The active document is edited through the live text view so revision
    /// tracking stays consistent; every other file goes through the disk
    /// path, which refuses while a registered session is dirty for it.
    private func editTodo(_ item: DocumentTodoItem, _ edit: TodoLineEdit) {
        if item.url == activeDocumentURL {
            guard let textView = environment?.editor.textView,
                  let bounds = Self.todoLineBounds(textView.string as NSString, line: item.line),
                  let replacement = Self.applyTodoEdit(edit, content: bounds.content, terminator: bounds.terminator),
                  textView.shouldChangeText(in: bounds.range, replacementString: replacement) else { return }
            textView.replaceCharacters(in: bounds.range, with: replacement)
            textView.didChangeText()
            return
        }
        Task { await editTodoOnDisk(item.url, line: item.line, edit: edit) }
    }

    /// Rewrites a comment line in a non-active file. A registered session
    /// must be clean — the new bytes are then adopted through
    /// `processDiskChange` exactly like an external edit; a dirty session
    /// (or none matching) turns the mutation into a no-op rather than
    /// clobbering unsaved work.
    private func editTodoOnDisk(_ url: URL, line: Int, edit: TodoLineEdit) async {
        guard let root = projectURL,
              let relative = try? Self.relativePath(for: url, root: root) else { return }
        if await !todoSessionIsClean(relative: relative) { return }
        guard let diskText = try? Self.readExactUTF8(url) else { return }
        let nsText = diskText as NSString
        guard let bounds = Self.todoLineBounds(nsText, line: line),
              let replacement = Self.applyTodoEdit(edit, content: bounds.content, terminator: bounds.terminator) else { return }
        let updated = nsText.replacingCharacters(in: bounds.range, with: replacement)
        let outcome = FoundationAtomicDocumentStore().save(
            text: updated,
            to: url,
            expectedBaselineHash: .hashing(diskText)
        )
        guard case .saved = outcome else { return }
        await processDiskChange(url)
        refreshTodos()
    }

    /// `addTodo`'s fallback — appends `% TODO: ` at the end of the build
    /// target under the same session-clean rules as `editTodoOnDisk`.
    private func appendTodoComment(to url: URL) async {
        guard let root = projectURL,
              let relative = try? Self.relativePath(for: url, root: root) else { return }
        if await !todoSessionIsClean(relative: relative) { return }
        guard let diskText = try? Self.readExactUTF8(url) else { return }
        let separator = diskText.isEmpty || diskText.hasSuffix("\n") ? "" : "\n"
        let outcome = FoundationAtomicDocumentStore().save(
            text: diskText + separator + "% TODO: \n",
            to: url,
            expectedBaselineHash: .hashing(diskText)
        )
        guard case .saved = outcome else { return }
        await processDiskChange(url)
        refreshTodos()
    }

    /// True when no registered session holds unsaved edits for `relative`.
    private func todoSessionIsClean(relative: NormalizedRelativePath) async -> Bool {
        guard let session = registeredSessions.first(where: { $0.path == relative }) else { return true }
        return (await session.snapshot()).saveState == .clean
    }

    /// Locates a `% TODO:`/`% DONE:` marker inside `line`: the done flag,
    /// the keyword range (incl. colon), and the text tail after it. Any `%`
    /// can introduce the comment, so trailing `code % TODO: x` lines count.
    private static func todoMarker(in line: String) -> (done: Bool, keyword: Range<String.Index>, tail: Range<String.Index>)? {
        var index = line.startIndex
        while index < line.endIndex, let percent = line[index...].firstIndex(of: "%") {
            var cursor = line.index(after: percent)
            while cursor < line.endIndex, line[cursor] == " " || line[cursor] == "\t" {
                cursor = line.index(after: cursor)
            }
            for (word, done) in [("TODO:", false), ("DONE:", true)] {
                if line[cursor...].hasPrefix(word) {
                    let keyEnd = line.index(cursor, offsetBy: word.count)
                    return (done, cursor..<keyEnd, keyEnd..<line.endIndex)
                }
            }
            index = line.index(after: percent)
        }
        return nil
    }

    /// UTF-16 range of the 1-based `line` including its terminator, split
    /// into content/terminator so a rewrite can delete or preserve it.
    private static func todoLineBounds(
        _ text: NSString, line: Int
    ) -> (range: NSRange, content: String, terminator: String)? {
        guard line >= 1 else { return nil }
        var start = 0
        var current = 1
        while current < line {
            let next = text.range(of: "\n", options: [], range: NSRange(location: start, length: text.length - start))
            guard next.location != NSNotFound else { return nil }
            start = next.location + 1
            current += 1
        }
        var lineEnd = 0, contentsEnd = 0
        text.getLineStart(nil, end: &lineEnd, contentsEnd: &contentsEnd, for: NSRange(location: start, length: 0))
        return (
            NSRange(location: start, length: lineEnd - start),
            text.substring(with: NSRange(location: start, length: contentsEnd - start)),
            text.substring(with: NSRange(location: contentsEnd, length: lineEnd - contentsEnd))
        )
    }

    /// Full-line replacement for an edit (content + terminator); `nil`
    /// keeps the file untouched when the marker moved since the scan.
    private static func applyTodoEdit(_ edit: TodoLineEdit, content: String, terminator: String) -> String? {
        switch edit {
        case .delete:
            return ""
        case .toggleDone:
            guard let marker = todoMarker(in: content) else { return nil }
            var line = content
            line.replaceSubrange(marker.keyword, with: marker.done ? "TODO:" : "DONE:")
            return line + terminator
        case .rename(let text):
            guard let marker = todoMarker(in: content) else { return nil }
            var line = content
            line.replaceSubrange(marker.tail, with: text.isEmpty ? "" : " " + text)
            return line + terminator
        }
    }
    private func rebuildProjectTree() {
        func relative(_ url: URL) -> String {
            guard let root = projectURL else { return url.lastPathComponent }
            let rootPath = root.standardizedFileURL.path
            let path = url.standardizedFileURL.path
            let prefix = rootPath == "/" ? "/" : rootPath + "/"
            guard path.hasPrefix(prefix) else { return url.lastPathComponent }
            return String(path.dropFirst(prefix.count))
        }
        projectTree = nestProjectChildren(
            buildProjectFileTree(relativePaths: projectFiles.map(relative)),
            main: buildSourceURL().map(relative) ?? "",
            children: projectChildren.map(relative)
        )
    }

    /// Re-enumerates the project tree (sidebar rescan button).
    func rescanProject() {
        guard let root = projectURL else { return }
        Task { @MainActor [weak self] in
            let discovered = try? await Task.detached(priority: .utility) {
                try Self.discoverTexFiles(root: root, selected: root, isDirectory: true)
            }.value
            guard let self, projectURL == root, let discovered else { return }
            projectFiles = discovered
            refreshBuildTarget()
            refreshStructure()
        }
    }

    nonisolated private static let outlineRegex = try? NSRegularExpression(
        pattern: #"\\(part|chapter|section|subsection|subsubsection|paragraph)\*?\{([^}]*)\}"#
    )
    nonisolated private static let labelRegex = try? NSRegularExpression(pattern: #"\\label\{([^}]*)\}"#)
    private static let bibliographyRegex = try? NSRegularExpression(
        pattern: #"@([A-Za-z]+)\s*\{\s*([^,\s]+)"#
    )

    /// UTF-16 offsets of every line start, built in one pass. Line numbers
    /// are then a binary search instead of a rescan from offset 0 per match.
    nonisolated private static func lineStartOffsets(_ text: NSString) -> [Int] {
        var starts = [0]
        for index in 0..<text.length where text.character(at: index) == 0x0A {
            starts.append(index + 1)
        }
        return starts
    }

    /// 1-based line containing `location` — the count of '\n' strictly
    /// before it, plus one.
    nonisolated private static func lineNumber(at location: Int, lineStarts: [Int]) -> Int {
        var lo = 0, hi = lineStarts.count - 1
        while lo < hi {
            let mid = (lo + hi + 1) / 2
            if lineStarts[mid] <= location { lo = mid } else { hi = mid - 1 }
        }
        return lo + 1
    }

    nonisolated static func parseOutline(_ text: String, lineStarts suppliedStarts: [Int]? = nil) -> [DocumentOutlineItem] {
        guard let regex = outlineRegex else { return [] }
        let nsText = text as NSString
        let lineStarts = suppliedStarts ?? lineStartOffsets(nsText)
        let levels = ["part": 0, "chapter": 1, "section": 2, "subsection": 3, "subsubsection": 4, "paragraph": 5]
        return regex.matches(in: text, range: NSRange(location: 0, length: nsText.length)).map { match in
            let name = nsText.substring(with: match.range(at: 1)).lowercased()
            let title = nsText.substring(with: match.range(at: 2))
            return DocumentOutlineItem(
                id: match.range.location, title: title,
                level: levels[name] ?? 2,
                line: lineNumber(at: match.range.location, lineStarts: lineStarts)
            )
        }
    }

    nonisolated static func parseLabels(_ text: String, lineStarts suppliedStarts: [Int]? = nil) -> [DocumentLabelItem] {
        guard let regex = labelRegex else { return [] }
        let nsText = text as NSString
        let lineStarts = suppliedStarts ?? lineStartOffsets(nsText)
        return regex.matches(in: text, range: NSRange(location: 0, length: nsText.length)).map { match in
            DocumentLabelItem(
                id: match.range.location, name: nsText.substring(with: match.range(at: 1)),
                line: lineNumber(at: match.range.location, lineStarts: lineStarts)
            )
        }
    }

    /// `%[ \t]*(TODO|DONE):[ \t]*(…)` — one item per comment line, in file
    /// order like the reference editor's checklist.
    static func parseTodos(_ text: String, file: String, url: URL) -> [DocumentTodoItem] {
        var items: [DocumentTodoItem] = []
        var line = 1
        for rawLine in (text as NSString).components(separatedBy: "\n") {
            if let marker = todoMarker(in: rawLine) {
                let body = rawLine[marker.tail].drop(while: { $0 == " " || $0 == "\t" })
                items.append(DocumentTodoItem(
                    file: file, url: url, line: line, text: String(body), done: marker.done
                ))
            }
            line += 1
        }
        return items
    }

    /// Every @entry key in a .bib text — used for the in-memory buffer's
    /// contribution to the project citation set.
    static func parseBibliographyKeys(_ text: String) -> Set<String> {
        guard let regex = bibliographyRegex else { return [] }
        let nsText = text as NSString
        return Set(regex
            .matches(in: text, range: NSRange(location: 0, length: nsText.length))
            .map { nsText.substring(with: $0.range(at: 2)) })
    }

    static func parseBibliography(files: [URL], root: URL?) -> [BibliographyItem] {
        guard let regex = bibliographyRegex else { return [] }
        var items: [BibliographyItem] = []
        for file in files {
            guard let text = try? String(contentsOf: file, encoding: .utf8) else { continue }
            let nsText = text as NSString
            let name = (try? relativePath(for: file, root: root ?? file.deletingLastPathComponent()).rawValue) ?? file.lastPathComponent
            for match in regex.matches(in: text, range: NSRange(location: 0, length: nsText.length)) {
                items.append(BibliographyItem(
                    id: "\(file.path)#\(match.range.location)", key: nsText.substring(with: match.range(at: 2)),
                    type: nsText.substring(with: match.range(at: 1)).lowercased(),
                    file: name
                ))
            }
        }
        return items
    }

    // MARK: - Build target & console commands

    /// Toggles the build-target pin on the active document (File → Pin/Unpin).
    func togglePinnedBuildTarget() {
        guard let url = activeDocumentURL, url.pathExtension.lowercased() == "tex" else { return }
        pinnedBuildTarget = pinnedBuildTarget == url ? nil : url
        refreshBuildTarget()
        Task { await restoreBuiltPreview() }
    }

    /// Persists the console command fields per project root so each project
    /// keeps its own build/custom commands like texspark does.
    private func loadProjectCommands(root: URL) {
        let key = "commands.\(root.standardizedFileURL.path)"
        let stored = UserDefaults.standard.dictionary(forKey: key)
        buildCommandText = stored?["build"] as? String ?? settings.defaultBuildCommand
        customCommandText = stored?["custom"] as? String ?? settings.defaultCustomCommand
    }

    /// Restores the most recently opened project on launch when the Editor
    /// preference allows it. Called once from the root view's onAppear.
    func restoreSessionIfNeeded() {
        guard !didRestoreSession, !hasProject, settings.restoreSession,
              let recent = recentDocuments.first,
              FileManager.default.fileExists(atPath: recent.path) else { return }
        didRestoreSession = true
        Task { await open(recent) }
    }

    func persistCommands() {
        guard let root = projectURL else { return }
        UserDefaults.standard.set(
            ["build": buildCommandText, "custom": customCommandText],
            forKey: "commands.\(root.standardizedFileURL.path)"
        )
    }

    /// Re-reads the active document from disk (editor toolbar reload button).
    /// Clean sessions adopt the disk bytes; dirty sessions surface a conflict.
    func reloadActiveDocumentFromDisk() async {
        guard let url = activeDocumentURL else { return }
        await processDiskChange(url)
    }

    /// Footer path label: the active document's project-relative path, like
    /// the reference editor's bottom-left path chip.
    var activeDocumentRelativePath: String? {
        guard let url = activeDocumentURL, let root = projectURL else { return nil }
        return try? Self.relativePath(for: url, root: root).rawValue
    }

    /// Re-runs the syntax highlighter after appearance changes.
    func rehighlight() {
        highlighter.highlightNow()
    }

    /// Runs the configured custom command (⌃⌘B) in the console's interactive
    /// terminal so the user sees live output — the reference editor's
    /// behaviour of running saved commands in the embedded shell.
    func runCustomCommand() async {
        let template = customCommandText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !template.isEmpty else { return }
        guard settings.settings.build.customShellAcknowledged else {
            terminalSession.feed("\u{001B}[33mEnable shell commands in Settings → Compile first.\u{001B}[0m\r\n")
            consoleSection = .terminal
            bottomPanelVisible = true
            return
        }
        terminalSession.send(substituteCommandPlaceholders(template))
        consoleSection = .terminal
        bottomPanelVisible = true
    }

    /// Sends a line to the interactive terminal — kept for programmatic
    /// callers; the terminal itself already handles direct typing.
    func runTerminalInput(_ command: String) {
        let trimmed = command.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        terminalSession.send(trimmed)
        consoleSection = .terminal
        bottomPanelVisible = true
    }

    /// Expands {file} / {filename} against the current build source.
    func substituteCommandPlaceholders(_ template: String) -> String {
        let relative = buildSourceRelativePath() ?? "main.tex"
        let stem = (relative as NSString).deletingPathExtension
        return template
            .replacingOccurrences(of: "{file}", with: relative)
            .replacingOccurrences(of: "{filename}", with: stem)
    }

    /// Chapters and bibliography files share their owning main document.
    func buildSourceURL() -> URL? { pinnedBuildTarget ?? automaticBuildTarget }

    func refreshBuildTarget() {
        var resolver = TeXProjectResolver()
        if let url = activeDocumentURL, let text = documentSnapshot?.text {
            resolver.activeText = (url, text)
        }
        do {
            automaticBuildTarget = try resolver.resolve(active: activeDocumentURL, files: projectFiles,
                                                        preferred: automaticBuildTarget)
            buildTargetMessage = automaticBuildTarget == nil
                ? "No main TeX document was found. Open the project folder or set % !TeX root in the chapter." : nil
        } catch {
            automaticBuildTarget = nil
            buildTargetMessage = error.localizedDescription
        }
        projectChildren = buildSourceURL().map { resolver.directDependencies(main: $0) } ?? []
    }

    func buildSourceRelativePath() -> String? {
        guard let root = projectURL, let url = buildSourceURL() else { return nil }
        return try? Self.relativePath(for: url, root: root).rawValue
    }

    /// Build → Clean: removes generated artifacts for the current target.
    func cleanBuildArtifacts() {
        guard let root = projectURL, let relative = buildSourceRelativePath() else { return }
        let stem = (relative as NSString).deletingPathExtension
        for ext in Self.generatedOutputExtensions {
            try? FileManager.default.removeItem(at: root.appendingPathComponent("\(stem).\(ext)"))
        }
        latestBuiltPDFName = nil
        terminalSession.feed("\u{001B}[90mcleaned generated files for \(stem)\u{001B}[0m\r\n")
        consoleSection = .terminal
        bottomPanelVisible = true
    }

    // MARK: - Helpers

    /// `discoverTexFiles`' whitelist — sources plus the figure formats the
    /// project tree lists. Build artifacts (aux/log/out/…/synctex.gz) stay
    /// hidden by omission; .pdf stays in because papers use PDF figures.
    nonisolated static let projectFileExtensions: Set<String> = [
        "tex", "bib",
        "png", "jpg", "jpeg", "pdf", "eps", "svg", "gif", "tif", "tiff", "bmp", "webp",
    ]

    /// Text files the editor can activate — figure rows open externally.
    nonisolated static func isSourceFile(_ url: URL) -> Bool {
        ["tex", "bib"].contains(url.pathExtension.lowercased())
    }

    nonisolated static func discoverTexFiles(
        root: URL,
        selected: URL,
        isDirectory: Bool
    ) throws -> [URL] {
        // A single-file open treats the file's directory as the project so
        // sibling .tex/.bib sources appear alongside it (matching how the
        // reference editor lists project files).
        let scanRoot = root
        guard let enumerator = FileManager.default.enumerator(
            at: scanRoot,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles, .skipsPackageDescendants]
        ) else { throw WorkspaceOpenError.unreadableProject }
        var files: [URL] = []
        for case let url as URL in enumerator where Self.projectFileExtensions.contains(url.pathExtension.lowercased()) {
            files.append(url.standardizedFileURL)
        }
        if !isDirectory, !files.contains(selected.standardizedFileURL) {
            files.append(selected.standardizedFileURL)
        }
        return files.sorted { $0.path.localizedStandardCompare($1.path) == .orderedAscending }
    }

    static func relativePath(for url: URL, root: URL) throws -> NormalizedRelativePath {
        let rootPath = root.standardizedFileURL.path
        let filePath = url.standardizedFileURL.path
        let prefix = rootPath == "/" ? "/" : rootPath + "/"
        guard filePath.hasPrefix(prefix) else { throw WorkspaceOpenError.outsideProject }
        return try NormalizedRelativePath(rawValue: String(filePath.dropFirst(prefix.count)))
    }

    nonisolated static func readExactUTF8(_ url: URL) throws -> String {
        let data = try Data(contentsOf: url, options: .mappedIfSafe)
        guard let text = String(data: data, encoding: .utf8) else {
            throw WorkspaceOpenError.invalidUTF8
        }
        return text
    }

    static func documentID(for path: String) -> String {
        var hash: UInt64 = 14_695_981_039_346_656_037
        for byte in path.utf8 {
            hash ^= UInt64(byte)
            hash &*= 1_099_511_628_211
        }
        return "document-" + String(hash, radix: 16)
    }
}

extension Notification.Name {
    // syncTeXHighlightRequested lives in Preview.swift next
    // to its observer.
    static let syncTeXInverseRequested = Notification.Name("pitex.syncTeXInverse")
}

private enum WorkspaceOpenError: LocalizedError {
    case noTexSources
    case unreadableProject
    case invalidUTF8
    case outsideProject
    case changedWhileOpening
    case savePermissionDenied(String)
    case saveInterrupted(String)

    var errorDescription: String? {
        switch self {
        case .noTexSources: "The selected project contains no .tex source files."
        case .unreadableProject: "The selected project could not be enumerated."
        case .invalidUTF8: "The selected source is not valid UTF-8."
        case .outsideProject: "The selected source is outside the project root."
        case .changedWhileOpening: "The source changed while it was opening. Open it again to avoid losing changes."
        case let .savePermissionDenied(path): "The source could not be saved because access was denied: \(path)"
        case let .saveInterrupted(path): "The atomic save did not complete: \(path)"
        }
    }
}

struct AppCommands: Commands {
    @ObservedObject var workspace: WorkspaceModel

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("command.new") { Task { await workspace.createDocument() } }
                .keyboardShortcut("n")
                .disabled(!workspace.hasProject)
            Button("command.open") { workspace.presentOpenPanel() }
                .keyboardShortcut("o")
                .accessibilityIdentifier("pitex.command.open")
            if !workspace.recentDocuments.isEmpty {
                Menu("command.open_recent") {
                    ForEach(workspace.recentDocuments, id: \.self) { url in
                        Button(url.lastPathComponent) { Task { await workspace.open(url) } }
                    }
                    Divider()
                    Button("command.clear_recents") { workspace.clearRecents() }
                }
            }
            Divider()
            Button("command.pin_build_target") { workspace.togglePinnedBuildTarget() }
                .keyboardShortcut("p", modifiers: [.command])
                .disabled(workspace.activeDocumentURL == nil)
            Divider()
            Button("command.close") { Task { await workspace.close() } }
                .keyboardShortcut("w")
                .disabled(!workspace.hasProject)
            Button("command.clear_session") { Task { await workspace.close() } }
                .disabled(!workspace.hasProject)
        }
        CommandGroup(replacing: .saveItem) {
            Button("editor.save") { Task { await workspace.save() } }
                .keyboardShortcut("s")
                .disabled(!workspace.canSave)
            Button("command.save_as") { Task { await workspace.saveAs() } }
                .keyboardShortcut("s", modifiers: [.command, .shift])
                .disabled(workspace.documentSnapshot == nil)
            Button("command.save_all") { Task { await workspace.persistDirtySessions() } }
                .keyboardShortcut("s", modifiers: [.command, .option])
                .disabled(!workspace.hasProject)
        }
        CommandGroup(after: .textEditing) {
            Divider()
            Button("command.toggle_comment") { workspace.toggleLineComment() }
                .keyboardShortcut("/", modifiers: [.command])
                .disabled(workspace.documentSnapshot == nil)
            Divider()
            Button("command.send_selection_ai") { workspace.sendSelectionToAssistant() }
                .keyboardShortcut("a", modifiers: [.command, .shift])
                .disabled(workspace.environment == nil || workspace.agent == nil)
        }
        CommandMenu("command.build_menu") {
            Button("command.build") { Task { await workspace.startBuild() } }
                .keyboardShortcut("b")
                .disabled(!workspace.canBuild || workspace.isBuilding)
            Button("command.cancel_build") { Task { await workspace.cancelBuild() } }
                .keyboardShortcut(".", modifiers: [.command])
                .disabled(!workspace.isBuilding)
            Button("command.clean") { workspace.cleanBuildArtifacts() }
                .disabled(!workspace.hasProject)
            Divider()
            Button("command.run_custom") { Task { await workspace.runCustomCommand() } }
                .keyboardShortcut("b", modifiers: [.command, .control])
                .disabled(!workspace.hasProject)
            Divider()
            Button("command.sync_forward") { Task { await workspace.syncForward() } }
                .keyboardShortcut("j", modifiers: [.command, .shift])
                .disabled(workspace.syncTeXBinding == nil)
        }
        CommandMenu("command.view") {
            Button("command.toggle_assistant") { workspace.toggleAssistant() }
                .keyboardShortcut("t", modifiers: [.command])
                .disabled(!workspace.hasProject)
            Button("command.toggle_inspector") { workspace.inspectorVisible.toggle() }
                .keyboardShortcut("p", modifiers: [.command, .option])
                .disabled(!workspace.hasProject || workspace.gitDiff != nil)
            Button("command.toggle_bottom") { workspace.bottomPanelVisible.toggle() }
                .keyboardShortcut("y", modifiers: [.command, .shift])
                .disabled(!workspace.hasProject)
            Button("command.toggle_sidebar") { workspace.sidebarVisible.toggle() }
                .disabled(!workspace.hasProject)
        }
        CommandGroup(replacing: .help) {
            Button("command.settings") { workspace.showingSettings = true }
        }
    }
}

final class PitexAppDelegate: NSObject, NSApplicationDelegate {
    weak var workspace: WorkspaceModel?

    /// LaunchServices caches the Dock/Finder icon keyed by the app bundle's
    /// modification date — a drag-copied update preserves it, so a new icon
    /// can stay invisible. Bump the bundle's mtime once per app version.
    /// Let macOS render the bundle icon so its Dock appearance stays consistent.
    private func refreshBundleIconCache() {
        let defaults = UserDefaults.standard
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? ""
        guard defaults.string(forKey: "iconRefreshVersion") != version else { return }
        let now = Date()
        for url in [Bundle.main.bundleURL, Bundle.main.bundleURL.appendingPathComponent("Contents")] {
            try? FileManager.default.setAttributes([.modificationDate: now], ofItemAtPath: url.path)
        }
        defaults.set(version, forKey: "iconRefreshVersion")
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        refreshBundleIconCache()
        // The Pitex Agent installs itself into the app's own support folder
        // on first launch (and refreshes its bundled skills on every launch)
        // — no install button required.
        Task { [weak self] in
            await PiRuntimeInstaller.ensureInstalled()
            await MainActor.run { self?.workspace?.agent?.prepare() }
        }
        // Auto-update: check+install on launch when the preference allows
        // it — same pref key the Linux/Windows shells read.
        Task { @MainActor [weak self] in
            if self?.workspace?.settings.autoInstallUpdates == true {
                await UpdateChecker().autoUpdate()
            }
        }
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        guard let url = urls.first else { return }
        Task { @MainActor in
            await workspace?.open(url)
        }
    }

    func applicationShouldOpenUntitledFile(_ sender: NSApplication) -> Bool { false }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        guard let workspace else { return }
        Task { @MainActor in
            await workspace.close()
        }
    }
}

@main
struct PitexApp: App {
    @NSApplicationDelegateAdaptor(PitexAppDelegate.self) private var appDelegate
    @StateObject private var workspace = WorkspaceModel()

    var body: some Scene {
        // A single window: every window hosts the same shared
        // adapter.textView, and an NSTextView can only live in one
        // scrollView — extra windows would steal it and leave blank editors.
        Window("Pitex", id: "main") {
            WorkspaceView(workspace: workspace)
                .frame(minWidth: 980, minHeight: 620)
                .onAppear { appDelegate.workspace = workspace }
                .sheet(isPresented: $workspace.showingSettings) {
                    SettingsView(store: workspace.settings, workspace: workspace)
                }
                .onAppear { workspace.restoreSessionIfNeeded() }
        }
        .commands { AppCommands(workspace: workspace) }
    }
}
