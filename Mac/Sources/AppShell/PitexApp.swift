import AppKit
import AppPorts
import AppShell
import BuildCore
import Darwin
import DocumentSessionCore
import Foundation
import MacPlatform
import ProjectCore
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

/// Bottom console tabs: Assistant | Terminal | Issues | Build Log. The
/// assistant lives here (not in the inspector) so the PDF preview and the
/// assistant can be visible at the same time.
enum ConsoleSection: String, CaseIterable, Identifiable {
    case assistant
    case terminal
    case issues
    case log
    var id: Self { self }
}

/// Left sidebar sections, mirroring Outline | Labels | BibTeX.
enum SidebarSection: String, CaseIterable, Identifiable {
    case outline
    case labels
    case bibtex
    var id: Self { self }
}

/// One row in the document outline (a sectioning command in the active file).
struct DocumentOutlineItem: Identifiable, Hashable {
    let id = UUID()
    let title: String
    let level: Int
    let line: Int
}

/// One \label{…} entry in the active document.
struct DocumentLabelItem: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let line: Int
}

/// One @entry{key,…} row collected from the project's .bib files.
struct BibliographyItem: Identifiable, Hashable {
    let id = UUID()
    let key: String
    let type: String
    let file: String
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
    @Published private(set) var projectURL: URL?
    @Published private(set) var projectFiles: [URL] = []
    @Published private(set) var openDocuments: [URL] = []
    @Published private(set) var activeDocumentURL: URL?
    @Published private(set) var environment: AppEnvironment? {
        didSet { observeEditorSelection() }
    }
    @Published private(set) var documentSnapshot: DocumentSessionCore.DocumentSnapshot? {
        didSet { refreshStructure(); scheduleAutosave() }
    }
    @Published var buildLogText = ""
    @Published var buildIssues: [BuildIssueRecord] = []
    @Published internal(set) var buildState: WorkspaceBuildState = .unavailable(
        "No build target has been configured for this project."
    )
    @Published private(set) var syncTeXState: WorkspaceSyncTeXState = .unavailable(
        "SyncTeX is unavailable until a successful build produces matching metadata."
    )
    @Published var showingSettings = false

    let registry = DocumentSessionRegistry()
    let settings = SettingsStore()
    let buildOrchestrator = BuildOrchestrator(executor: StreamingBuildExecutor())
    let syncTeXRunner = SyncTeXRunner()
    let highlighter = SyntaxHighlighter()
    private(set) var agent: AgentCoordinator?
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
    @Published var pinnedBuildTarget: URL?
    @Published internal(set) var automaticBuildTarget: URL?
    /// Direct dependencies of the build target, nested under it in the
    /// project sidebar (`project_children`).
    @Published internal(set) var projectChildren: [URL] = []
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
        await close()
        phase = .loading(selectedURL)
        do {
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
            projectURL = root
            projectFiles = files
            registeredSessions = [session]
            loadProjectCommands(root: root)
            let port = NativeDocumentSessionPort(session: session) { [weak self] snapshot in
                self?.documentSnapshot = snapshot
            }
            let appEnvironment = try await AppShell.make(documentSession: port)
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
            capabilityBroker = appEnvironment.files
            capabilityLease = lease
            let verifiedText = try Self.readExactUTF8(initialURL)
            guard verifiedText == initialText else {
                throw WorkspaceOpenError.changedWhileOpening
            }

            environment = appEnvironment
            activeDocumentURL = initialURL
            openDocuments = [initialURL]
            documentSnapshot = await session.snapshot()
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
            startWatcher(for: initialURL)
            coordinator.prepare()
            phase = .ready
            await restoreBuiltPreview()
            recordRecent(selectedURL)
        } catch {
            await close()
            phase = .failed(error.localizedDescription)
        }
    }

    func activateDocument(_ url: URL) async {
        guard url != activeDocumentURL,
              let root = projectURL,
              projectFiles.contains(url) else { return }
        // Keep the workspace mounted when switching sources: a loading phase
        // destroys the split view and PDF view, losing their size and position.
        do {
            let text = try Self.readExactUTF8(url)
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
            if !registeredSessions.contains(where: { $0 === session }) {
                registeredSessions.append(session)
            }
            let port = NativeDocumentSessionPort(session: session) { [weak self] snapshot in
                self?.documentSnapshot = snapshot
            }
            let appEnvironment = try await AppShell.make(documentSession: port)
            let snapshot = await session.snapshot()
            environment = appEnvironment
            activeDocumentURL = url
            if !openDocuments.contains(url) { openDocuments.append(url) }
            documentSnapshot = snapshot
            refreshBuildTarget()
            highlighter.attach(to: appEnvironment.editor, fileExtension: url.pathExtension)
            startWatcher(for: url)
            phase = .ready
            await restoreBuiltPreview()
        } catch {
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

    func close() async {
        for url in fileWatchers.keys { stopWatcher(for: url) }
        for task in pendingDiskChecks.values { task.cancel() }
        pendingDiskChecks.removeAll()
        agent?.shutdown()
        highlighter.detach()
        let brokerToClose = capabilityBroker
        let leaseToClose = capabilityLease
        let root = projectURL
        let sessions = registeredSessions
        environment = nil
        capabilityBroker = nil
        capabilityLease = nil
        registeredSessions.removeAll()
        if let root {
            for session in sessions {
                _ = try? await registry.close(projectRoot: root, session: session)
            }
        }
        if let brokerToClose, let leaseToClose {
            try? await brokerToClose.endAccess(leaseToClose)
        }
        projectURL = nil
        projectFiles = []
        openDocuments = []
        activeDocumentURL = nil
        documentSnapshot = nil
        agent = nil
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
        phase = .noProject
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
        let startLine = nsText.substring(to: range.location).components(separatedBy: "\n").count
        let endLine = nsText.substring(to: range.location + range.length).components(separatedBy: "\n").count
        let path = documentSnapshot?.path.rawValue ?? activeDocumentURL?.lastPathComponent ?? "document"
        agent.updateSelectionAttachment(AgentSelectionAttachment(
            path: path, startLine: startLine, endLine: endLine, text: selected
        ))
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

    /// Rebuilds the Outline / Labels / BibTeX sidebar data from the active
    /// document text and the project's .bib files.
    private func refreshStructure() {
        let text = documentSnapshot?.text ?? ""
        outlineItems = Self.parseOutline(text)
        labelItems = Self.parseLabels(text)
        bibliographyItems = Self.parseBibliography(
            files: projectFiles.filter { $0.pathExtension.lowercased() == "bib" },
            root: projectURL
        )
    }

    /// Re-enumerates the project tree (sidebar rescan button).
    func rescanProject() {
        guard let root = projectURL,
              let discovered = try? Self.discoverTexFiles(root: root, selected: root, isDirectory: true)
        else { return }
        projectFiles = discovered
        refreshBuildTarget()
        refreshStructure()
    }

    static func parseOutline(_ text: String) -> [DocumentOutlineItem] {
        guard let regex = try? NSRegularExpression(
            pattern: #"\\(part|chapter|section|subsection|subsubsection|paragraph)\*?\{([^}]*)\}"#
        ) else { return [] }
        let nsText = text as NSString
        let levels = ["part": 0, "chapter": 1, "section": 2, "subsection": 3, "subsubsection": 4, "paragraph": 5]
        return regex.matches(in: text, range: NSRange(location: 0, length: nsText.length)).map { match in
            let name = nsText.substring(with: match.range(at: 1)).lowercased()
            let title = nsText.substring(with: match.range(at: 2))
            var line = 1
            var location = 0
            while location < match.range.location {
                let next = nsText.range(
                    of: "\n",
                    range: NSRange(location: location, length: nsText.length - location)
                )
                guard next.location != NSNotFound else { break }
                line += 1
                location = next.location + 1
            }
            return DocumentOutlineItem(title: title, level: levels[name] ?? 2, line: line)
        }
    }

    static func parseLabels(_ text: String) -> [DocumentLabelItem] {
        guard let regex = try? NSRegularExpression(pattern: #"\\label\{([^}]*)\}"#) else { return [] }
        let nsText = text as NSString
        return regex.matches(in: text, range: NSRange(location: 0, length: nsText.length)).map { match in
            var line = 1
            var location = 0
            while location < match.range.location {
                let next = nsText.range(
                    of: "\n",
                    range: NSRange(location: location, length: nsText.length - location)
                )
                guard next.location != NSNotFound else { break }
                line += 1
                location = next.location + 1
            }
            return DocumentLabelItem(name: nsText.substring(with: match.range(at: 1)), line: line)
        }
    }

    static func parseBibliography(files: [URL], root: URL?) -> [BibliographyItem] {
        guard let regex = try? NSRegularExpression(
            pattern: #"@([A-Za-z]+)\s*\{\s*([^,\s]+)"#
        ) else { return [] }
        var items: [BibliographyItem] = []
        for file in files {
            guard let text = try? String(contentsOf: file, encoding: .utf8) else { continue }
            let nsText = text as NSString
            let name = (try? relativePath(for: file, root: root ?? file.deletingLastPathComponent()).rawValue) ?? file.lastPathComponent
            for match in regex.matches(in: text, range: NSRange(location: 0, length: nsText.length)) {
                items.append(BibliographyItem(
                    key: nsText.substring(with: match.range(at: 2)),
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

    /// Word count for the editor footer (whitespace-separated tokens).
    var wordCount: Int {
        guard let text = documentSnapshot?.text else { return 0 }
        return text.split { $0 == " " || $0 == "\n" || $0 == "\t" }.count
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

    static func discoverTexFiles(
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
        for case let url as URL in enumerator where ["tex", "bib"].contains(url.pathExtension.lowercased()) {
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

    static func readExactUTF8(_ url: URL) throws -> String {
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
                .disabled(!workspace.hasProject)
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

    func applicationDidFinishLaunching(_ notification: Notification) {
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
