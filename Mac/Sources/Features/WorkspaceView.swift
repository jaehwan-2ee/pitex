import AppKit
import LanguageCore
import SwiftUI

struct WorkspaceView: View {
    @ObservedObject var workspace: WorkspaceModel
    /// When true the PDF/AI inspector sits in the left column and the
    /// project sidebar moves right (View → move panel to the left).
    @AppStorage("inspectorOnLeft") private var inspectorOnLeft = false
    /// Observing the settings store directly keeps the editor surface
    /// (minimap flag, appearance colors) live when Preferences change.
    @ObservedObject private var settingsStore: SettingsStore
    @ObservedObject private var appearance = AppearanceSettings.shared
    /// Symbols palette popover anchored to the toolbar button.
    @State private var showingSymbols = false

    init(workspace: WorkspaceModel) {
        self.workspace = workspace
        _settingsStore = ObservedObject(wrappedValue: workspace.settings)
    }

    var body: some View {
        Group {
            switch workspace.phase {
            case .noProject:
                noProject
            case let .loading(url):
                loading(url)
            case let .failed(message):
                error(message)
            case .ready:
                workspaceLayout
            }
        }
        .accessibilityIdentifier("pitex.workspace")
        .toolbar { toolbar }
        // The selected theme colors the whole window chrome, not just the
        // editor: accent comes from the palette's command color, primary
        // text follows the body color (secondary/tertiary derive from it),
        // and the window surface is the palette's editor background.
        .tint(Color(nsColor: appearance.color(for: .commands)))
        .foregroundStyle(Color(nsColor: appearance.color(for: .bodyText)))
        .background(Color(nsColor: appearance.color(for: .editorBackground)))
    }

    private var noProject: some View {
        ContentUnavailableView {
            Label("workspace.no_project", systemImage: "doc.text.magnifyingglass")
        } description: {
            Text("workspace.no_project_detail")
        } actions: {
            Button("workspace.open_project") { workspace.presentOpenPanel() }
                .keyboardShortcut("o")
                .accessibilityIdentifier("pitex.open")
            Button("command.open_via_ssh") { workspace.presentOpenViaSSH() }
                .accessibilityIdentifier("pitex.openViaSSH")
        }
        .frame(minWidth: 560, minHeight: 360)
        .accessibilityIdentifier("pitex.noProject")
    }

    private func loading(_ url: URL) -> some View {
        VStack(spacing: 14) {
            ProgressView()
                .controlSize(.large)
            Text("workspace.open_project")
                .font(.headline)
            Text(verbatim: workspace.fileDisplayName(url))
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityIdentifier("pitex.loading")
    }

    private func error(_ message: String) -> some View {
        ContentUnavailableView {
            Label("workspace.open_project", systemImage: "exclamationmark.triangle")
        } description: {
            Text(verbatim: message)
        } actions: {
            HStack {
                Button("workspace.open") { workspace.presentOpenPanel() }
                    .accessibilityIdentifier("pitex.workspace.openAfterError")
                if workspace.hasProject {
                    Button("workspace.close") { Task { await workspace.close() } }
                }
            }
        }
        .accessibilityIdentifier("pitex.error")
    }

    /// The inspector's own condition, shared by the layout and the
    /// native split so detached content has no inline column.
    /// A commit diff keeps the PDF inspector closed — the pane cannot be
    /// opened while the diff is up. A detached pane lives in its own
    /// window; the column must not render a second copy here.
    private var inspectorInline: Bool {
        workspace.inspectorVisible && workspace.gitDiff == nil && !workspace.previewDetached
    }

    /// The outer split owns geometry independently of the PDF/Markdown
    /// content's fitting size. Its hosting panes survive ordinary updates.
    private var workspaceLayout: some View {
        WorkspaceSplitView(
            workspace: workspace,
            inspectorOnLeft: inspectorOnLeft,
            sidebar: workspace.sidebarVisible ? AnyView(sidebarColumn) : nil,
            center: AnyView(centerColumn),
            inspector: inspectorInline ? AnyView(inspectorColumn) : nil
        )
        .accessibilityIdentifier("pitex.workspace.split")
    }

    private var sidebarColumn: some View {
        ProjectSidebarView(workspace: workspace)
            .frame(minWidth: 170, idealWidth: 170, maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: appearance.color(for: .gutterBackground)))
    }

    private var inspectorColumn: some View {
        Preview(workspace: workspace)
            .frame(minWidth: 300, idealWidth: 300, maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: appearance.color(for: .gutterBackground)))
    }

    // MARK: - Center column

    private var centerColumn: some View {
        VStack(spacing: 0) {
            authorityWarning
            documentTabBar
            Divider()
            editorHeader
            conflictBanner
            remoteConflictBanner
            // VSplitView gives the console's top edge a draggable divider,
            // like the outer native split does for the side panes.
            VSplitView {
                VStack(spacing: 0) {
                    editor
                    editorFooter
                }
                .frame(minHeight: 160, maxHeight: .infinity)
                .layoutPriority(1)
                // An open commit diff covers the editor surface but keeps
                // it mounted underneath, so scroll/undo state survives.
                .overlay {
                    if let diff = workspace.gitDiff {
                        CommitDiffView(workspace: workspace, session: diff)
                    }
                }
                if workspace.bottomPanelVisible {
                    BottomConsoleView(workspace: workspace)
                        .background(Color(nsColor: appearance.color(for: .gutterBackground)))
                }
            }
        }
        .frame(minWidth: 460, maxHeight: .infinity)
        .layoutPriority(1)
    }

    /// Tab strip plus the + / Save / Build / SyncTeX controls on one row,
    /// mirroring the reference editor's top bar.
    private var documentTabBar: some View {
        HStack(spacing: 6) {
            documentTabs

            Button {
                showingSymbols = true
            } label: {
                // Icon-only — the √ glyph reads as the symbol palette.
                Image(systemName: "x.squareroot")
            }
            .buttonStyle(.borderless)
            .fixedSize()
            .help(String(localized: "editor.symbols"))
            .accessibilityLabel(String(localized: "editor.symbols"))
            .disabled(workspace.environment == nil)
            .accessibilityIdentifier("pitex.toolbar.symbols")
            .popover(isPresented: $showingSymbols, arrowEdge: .bottom) {
                SymbolsPaletteView(onInsert: insertSymbol)
            }

            Menu {
                Button("command.new") { Task { await workspace.createDocument() } }
                Button("command.open") { workspace.presentOpenPanel() }
                if !workspace.recentDocuments.isEmpty {
                    Divider()
                    Menu("command.open_recent") {
                        ForEach(workspace.recentDocuments, id: \.self) { url in
                            Button(WorkspaceModel.recentTitle(for: url)) { Task { await workspace.open(url) } }
                        }
                        Divider()
                        Button("command.clear_recents") { workspace.clearRecents() }
                    }
                }
                Button("command.open_via_ssh") { workspace.presentOpenViaSSH() }
            } label: {
                Image(systemName: "plus")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .accessibilityIdentifier("pitex.editor.add")

            Menu {
                Button("editor.save") { Task { await workspace.save() } }
                    .disabled(!workspace.canSave)
                Button("command.save_as") { Task { await workspace.saveAs() } }
                Button("command.save_all") { Task { await workspace.persistDirtySessions() } }
            } label: {
                Image(systemName: "square.and.arrow.down")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .accessibilityIdentifier("pitex.toolbar.save")

            Button {
                Task {
                    if workspace.isBuilding {
                        await workspace.cancelBuild()
                    } else {
                        await workspace.startBuild()
                    }
                }
            } label: {
                Image(systemName: workspace.isBuilding ? "stop.fill" : "play.fill")
            }
            .buttonStyle(.borderless)
            .disabled(workspace.buildUnavailableReason != nil && !workspace.isBuilding)
            .help(workspace.buildUnavailableReason ?? String(localized: "build.start"))
            .accessibilityIdentifier("pitex.build")

            Button {
                settingsStore.liveCompileEnabled.toggle()
            } label: {
                Image(systemName: settingsStore.liveCompileEnabled ? "bolt.fill" : "bolt")
            }
            .buttonStyle(.borderless)
            .help(String(localized: "toolbar.live_compile"))
            .accessibilityLabel(String(localized: "toolbar.live_compile"))
            .accessibilityValue(settingsStore.liveCompileEnabled
                ? String(localized: "accessibility.on")
                : String(localized: "accessibility.off"))
            .accessibilityAddTraits(settingsStore.liveCompileEnabled ? .isSelected : [])
            .accessibilityIdentifier("pitex.toolbar.liveCompile")
        }
        .padding(.horizontal, 6)
        .frame(height: 34)
    }

    /// "Editor" caption row with the disk-change status and reload button on
    /// the right — same position as the reference editor's header line.
    private var editorHeader: some View {
        HStack(spacing: 8) {
            Text("editor.title")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            Spacer()
            diskStatusButton
            Button {
                Task { await workspace.reloadActiveDocumentFromDisk() }
            } label: {
                Image(systemName: "arrow.clockwise")
            }
            .buttonStyle(.borderless)
            .help(String(localized: "editor.reload"))
            .accessibilityIdentifier("pitex.editor.reload")
        }
        .padding(.horizontal, 10)
        .frame(height: 22)
    }

    private var diskStatusButton: some View {
        Button {
            Task { await workspace.reloadActiveDocumentFromDisk() }
        } label: {
            Image(systemName: "plusminus")
        }
        .buttonStyle(.borderless)
        .help(workspace.documentSnapshot?.saveState == .conflicted
              ? String(localized: "editor.disk_changed")
              : String(localized: "editor.disk_unchanged"))
        .accessibilityIdentifier("pitex.editor.diskStatus")
    }

    private var documentTabs: some View {
        ScrollView(.horizontal) {
            HStack(spacing: 4) {
                ForEach(workspace.openDocuments, id: \.self) { url in
                    HStack(spacing: 5) {
                        Button {
                            Task { await workspace.activateDocument(url) }
                        } label: {
                            Text(verbatim: workspace.fileDisplayName(url))
                                .lineLimit(1)
                                .help(url.path)
                        }
                        .buttonStyle(.plain)
                        Button {
                            Task { await workspace.closeDocument(url) }
                        } label: {
                            Image(systemName: "xmark")
                                .imageScale(.small)
                        }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("editor.close")
                    }
                    .padding(.horizontal, 9)
                    .padding(.vertical, 6)
                    .background(
                        url == workspace.activeDocumentURL
                            ? Color.accentColor.opacity(0.16)
                            : Color.clear,
                        in: RoundedRectangle(cornerRadius: 6)
                    )
                    .accessibilityIdentifier("pitex.editor.tab")
                }
            }
            .padding(.horizontal, 4)
        }
        .scrollIndicators(.hidden)
        .accessibilityIdentifier("pitex.tabs")
    }

    @ViewBuilder
    private var conflictBanner: some View {
        if let snapshot = workspace.documentSnapshot, snapshot.saveState == .conflicted {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Image(systemName: "exclamationmark.triangle.fill")
                VStack(alignment: .leading, spacing: 2) {
                    Text("conflict.title")
                        .font(.callout.weight(.semibold))
                    Text(verbatim: String(
                        format: String(localized: "conflict.message"),
                        workspace.activeDocumentURL?.lastPathComponent ?? ""
                    ))
                        .font(.caption)
                }
                Spacer()
                Button("conflict.use_external") {
                    Task { await workspace.resolveConflict(useDiskVersion: true) }
                }
                .accessibilityIdentifier("pitex.conflict.useExternal")
                Button("conflict.keep_mine") {
                    Task { await workspace.resolveConflict(useDiskVersion: false) }
                }
                .accessibilityIdentifier("pitex.conflict.keepMine")
            }
            .foregroundStyle(.orange)
            .padding(9)
            .background(.orange.opacity(0.12))
            .accessibilityIdentifier("pitex.conflict")
        }
    }

    /// Files changed both here and on the SSH device since the last sync;
    /// neither side is overwritten until the user picks one per file.
    @ViewBuilder
    private var remoteConflictBanner: some View {
        if let remote = workspace.remote, !remote.conflicts.isEmpty {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 8) {
                    Image(systemName: "exclamationmark.triangle.fill")
                    Text(verbatim: String(format: String(localized: "remote.conflict.title"), remote.deviceName))
                        .font(.callout.weight(.semibold))
                }
                ForEach(remote.conflicts, id: \.self) { path in
                    HStack(spacing: 8) {
                        Text(verbatim: path)
                            .font(.caption.monospaced())
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Spacer()
                        Button("remote.conflict.take_remote") {
                            Task { await workspace.resolveRemoteConflict(path, keepLocal: false) }
                        }
                        Button("remote.conflict.keep_mine") {
                            Task { await workspace.resolveRemoteConflict(path, keepLocal: true) }
                        }
                    }
                }
            }
            .foregroundStyle(.orange)
            .padding(9)
            .background(.orange.opacity(0.12))
            .accessibilityIdentifier("pitex.remoteConflict")
        }
    }

    @ViewBuilder
    private var editor: some View {
        if let environment = workspace.environment {
            EditorContainerView(
                adapter: environment.editor,
                minimapVisible: settingsStore.minimap,
                foldingEnabled: settingsStore.codeFolding,
                completion: workspace.completion,
                onSyncRequest: { line, column in
                    Task { await workspace.syncForward(line: line, column: column) }
                },
                onBuildRequest: {
                    Task { await workspace.startBuild() }
                },
                scrollSync: workspace.activeDocumentIsMarkdown ? workspace.markdownScrollSync : nil
            )
                .id(ObjectIdentifier(environment.editor))
                .accessibilityIdentifier("pitex.editor")
                // Appearance edits (theme/font/colors) re-apply through
                // updateNSView and a fresh highlight pass.
                .onChange(of: appearance.colorRevision) { workspace.rehighlight() }
                .onChange(of: appearance.fontSize) { workspace.rehighlight() }
        } else {
            ContentUnavailableView("editor.no_document", systemImage: "doc")
                .accessibilityIdentifier("pitex.editor.empty")
        }
    }

    /// Bottom icon row under the editor: panel toggles on the left, word
    /// count on the right — same controls as the reference footer.
    private var editorFooter: some View {
        HStack(spacing: 10) {
            Button { workspace.sidebarVisible.toggle() } label: {
                Image(systemName: "sidebar.left")
            }
            .help(String(localized: workspace.sidebarVisible ? "editor.hide_sidebar" : "editor.show_sidebar"))
            Button { workspace.bottomPanelVisible.toggle() } label: {
                Image(systemName: "rectangle.bottomhalf.filled")
            }
            .help(String(localized: "editor.toggle_bottom"))
            Button { inspectorOnLeft.toggle() } label: {
                Image(systemName: "arrow.left.and.right")
            }
            .help(String(localized: "editor.flip_panels"))
            Button { workspace.toggleInspectorPane() } label: {
                Image(systemName: "sidebar.right")
            }
            .disabled(workspace.gitDiff != nil)
            .help(String(localized: workspace.inspectorVisible && !workspace.previewDetached
                         ? "editor.hide_inspector" : "editor.show_inspector"))
            if let path = workspace.activeDocumentRelativePath {
                Text(verbatim: path)
                    .lineLimit(1)
                    .truncationMode(.head)
            }
            Spacer()
            Text(verbatim: workspace.documentSnapshot == nil
                 ? "— \(String(localized: "editor.words_unit"))"
                 : "\(workspace.wordCount) \(String(localized: "editor.words_unit"))")
        }
        .font(.caption)
        .foregroundStyle(.secondary)
        .buttonStyle(.borderless)
        .padding(.horizontal, 10)
        .frame(height: 26)
        .accessibilityIdentifier("pitex.editor.status")
    }

    @ViewBuilder
    private var authorityWarning: some View {
        if settings_showsShellWarning {
            Label("warning.custom_shell_authority", systemImage: "exclamationmark.shield")
                .font(.caption.weight(.medium))
                .foregroundStyle(.orange)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(.orange.opacity(0.10))
                .accessibilityIdentifier("pitex.shellWarning")
        }
    }

    private var settings_showsShellWarning: Bool {
        if case .custom = workspace.settings.settings.build.shellExecution {
            return !workspace.settings.settings.build.customShellAcknowledged
        }
        return false
    }

    // MARK: - Toolbar

    /// Toolbar layout: a sidebar collapse/expand button at the top-left
    /// (navigation placement) and the five-button cluster at the top-right —
    /// open, PDF pane toggle, assistant wand, settings, find — matching the
    /// reference editor's toolbar.
    @ToolbarContentBuilder
    private var toolbar: some ToolbarContent {
        ToolbarItem(placement: .navigation) {
            Button {
                workspace.sidebarVisible.toggle()
            } label: {
                Image(systemName: "sidebar.left")
            }
            .help(String(localized: workspace.sidebarVisible ? "editor.hide_sidebar" : "editor.show_sidebar"))
            .accessibilityIdentifier("pitex.toolbar.sidebar")
        }
        ToolbarItemGroup {
            Button {
                workspace.presentOpenPanel()
            } label: {
                Label("workspace.open", systemImage: "folder")
            }
            .accessibilityIdentifier("pitex.toolbar.open")

            Button {
                workspace.toggleInspectorPane()
            } label: {
                Label("preview.title", systemImage: "doc.richtext")
            }
            .disabled(workspace.gitDiff != nil)
            .help(String(localized: workspace.inspectorVisible && !workspace.previewDetached
                         ? "editor.hide_inspector" : "editor.show_inspector"))
            .accessibilityIdentifier("pitex.toolbar.pdf")

            Button {
                workspace.toggleAssistant()
            } label: {
                Label("assistant.title", systemImage: "wand.and.stars")
            }
            .help(String(localized: "editor.toggle_assistant"))
            .accessibilityIdentifier("pitex.toolbar.assistant")

            Button {
                WorkspaceWindows.presentSettings(from: workspace)
            } label: {
                Label("command.settings", systemImage: "gearshape")
            }
            .accessibilityIdentifier("pitex.settings")

            Button {
                showFindPanel()
            } label: {
                Label("editor.find", systemImage: "magnifyingglass")
            }
            .disabled(workspace.environment == nil)
            .keyboardShortcut("f")
            .accessibilityIdentifier("pitex.toolbar.find")
        }
    }

    /// Inserts the palette's LaTeX command at the caret through
    /// NSTextView's own insertion path, so undo grouping and the
    /// document-session submit pipeline behave exactly like typed text.
    private func insertSymbol(_ symbol: TexSymbol) {
        guard let textView = workspace.environment?.editor.textView else { return }
        textView.insertText(symbol.command, replacementRange: textView.selectedRange())
        textView.window?.makeFirstResponder(textView)
    }

    private func showFindPanel() {
        guard let textView = workspace.environment?.editor.textView else { return }
        let sender = NSMenuItem(title: "", action: nil, keyEquivalent: "")
        sender.tag = Int(NSFindPanelAction.showFindPanel.rawValue)
        textView.performFindPanelAction(sender)
    }
}

/// Geometry belongs to a workspace/document, not a mounted SwiftUI pane.
/// These preferences are only changed by divider movement; fitting content
/// and a temporarily smaller window must never overwrite them.
@MainActor
final class WorkspacePaneLayoutState {
    struct Widths: Equatable {
        var sidebar: CGFloat = 170
        var inspector: CGFloat = 300
    }

    /// A drag in a constrained window has two results: a new preference
    /// for the moved pane, and exact displayed widths for this viewport.
    /// Re-fitting the preferences immediately would redistribute both sides.
    struct Realization {
        var viewportWidth: CGFloat
        var dividerThickness: CGFloat
        var sidebar: CGFloat?
        var inspector: CGFloat?
    }

    private(set) var widths = Widths()
    private(set) var realization: Realization?
    private(set) var activeDocumentURL: URL?
    private(set) var documentWidths: [URL: Widths] = [:]
    private var documentRealizations: [URL: Realization] = [:]

    func activate(_ url: URL?) {
        let key = url?.standardizedFileURL
        guard key != activeDocumentURL else { return }
        activeDocumentURL = key
        if let key {
            // An unseen tab starts exactly where the outgoing tab left it.
            if let saved = documentWidths[key] {
                widths = saved
                realization = documentRealizations[key]
            }
            documentWidths[key] = widths
            documentRealizations[key] = realization
        }
    }

    func record(sidebar: CGFloat? = nil, inspector: CGFloat? = nil, realization: Realization? = nil) {
        guard sidebar != nil || inspector != nil else { return }
        if let sidebar, sidebar.isFinite, sidebar >= 170 { widths.sidebar = sidebar }
        if let inspector, inspector.isFinite, inspector >= 300 { widths.inspector = inspector }
        self.realization = realization
        if let activeDocumentURL {
            documentWidths[activeDocumentURL] = widths
            documentRealizations[activeDocumentURL] = realization
        }
    }

    func remove(_ url: URL) {
        documentWidths.removeValue(forKey: url.standardizedFileURL)
        documentRealizations.removeValue(forKey: url.standardizedFileURL)
    }

    func reset() {
        activeDocumentURL = nil
        documentWidths.removeAll()
        documentRealizations.removeAll()
        widths = Widths()
        realization = nil
    }
}

private struct WorkspaceSplitView: NSViewRepresentable {
    let workspace: WorkspaceModel
    let inspectorOnLeft: Bool
    let sidebar: AnyView?
    let center: AnyView
    let inspector: AnyView?

    func makeNSView(context: Context) -> WorkspaceNativeSplitView {
        WorkspaceNativeSplitView(state: workspace.paneLayout)
    }

    func updateNSView(_ split: WorkspaceNativeSplitView, context: Context) {
        // Crossing a hosting boundary otherwise loses SwiftUI environment
        // values. Keep window commands bound to this workspace from every pane.
        func hosted(_ view: AnyView) -> AnyView {
            AnyView(view.environment(\.self, context.environment).focusedSceneObject(workspace))
        }
        split.configure(
            sidebar: sidebar.map(hosted), center: hosted(center),
            inspector: inspector.map(hosted), inspectorOnLeft: inspectorOnLeft
        )
    }
}

/// A frame-based outer split: hosted content cannot install intrinsic-size
/// constraints that redistribute the columns when PDFKit becomes WebKit.
/// NSSplitView still owns divider drawing, hit testing, and mouse tracking.
private final class WorkspaceNativeSplitView: NSSplitView, NSSplitViewDelegate {
    private enum Pane: Hashable {
        case sidebar, center, inspector

        var minimum: CGFloat {
            switch self {
            case .sidebar: 170
            case .center: 460
            case .inspector: 300
            }
        }
    }

    private let state: WorkspacePaneLayoutState
    private var hosts: [Pane: NSHostingView<AnyView>] = [:]
    private var panes: [Pane] = []
    private var documentURL: URL?
    private var configuring = false
    private var placing = false
    private var trackingDivider = false

    init(state: WorkspacePaneLayoutState) {
        self.state = state
        super.init(frame: .zero)
        isVertical = true
        dividerStyle = .thin
        delegate = self
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    func configure(sidebar: AnyView?, center: AnyView, inspector: AnyView?, inspectorOnLeft: Bool) {
        configuring = true
        defer { configuring = false }
        documentURL = state.activeDocumentURL
        var contents: [(Pane, AnyView)] = []
        if let sidebar { contents.append((.sidebar, sidebar)) }
        contents.append((.center, center))
        if let inspector { contents.append((.inspector, inspector)) }
        if inspectorOnLeft { contents.reverse() }
        let nextPanes = contents.map(\.0)

        for (pane, content) in contents {
            if let host = hosts[pane] {
                host.rootView = content
            } else {
                let host = NSHostingView(rootView: content)
                host.sizingOptions = []
                host.autoresizingMask = []
                hosts[pane] = host
            }
        }
        // Preserve hosts (and editor/preview identity) across normal updates
        // and mirror changes. Hidden panes unmount just as conditional panes did.
        if nextPanes != panes {
            for pane in panes where !nextPanes.contains(pane) {
                hosts.removeValue(forKey: pane)?.removeFromSuperview()
            }
            panes = nextPanes
            for (index, pane) in panes.enumerated() {
                guard let host = hosts[pane] else { continue }
                if arrangedSubviews.count <= index || arrangedSubviews[index] !== host {
                    host.removeFromSuperview()
                    insertArrangedSubview(host, at: index)
                }
            }
        }
        placePanes()
    }

    override func resizeSubviews(withOldSize oldSize: NSSize) {
        placePanes()
    }

    override func layout() {
        super.layout()
        placePanes()
    }

    private func placePanes() {
        guard !placing, !trackingDivider, !panes.isEmpty,
              arrangedSubviews.count == panes.count, bounds.width > 0 else { return }
        placing = true
        defer { placing = false }
        let available = max(0, bounds.width - CGFloat(panes.count - 1) * dividerThickness)
        // Reuse the exact drag result at the same viewport and visibility.
        // Mirroring does not change this role-based geometry. A different
        // window width fits preferences, recovering an undragged wide pane.
        let realization = state.realization.flatMap { saved in
            saved.viewportWidth == bounds.width && saved.dividerThickness == dividerThickness
                && (saved.sidebar != nil) == panes.contains(.sidebar)
                && (saved.inspector != nil) == panes.contains(.inspector) ? saved : nil
        }
        var widths = panes.map { pane -> CGFloat in
            switch pane {
            case .sidebar: realization?.sidebar ?? state.widths.sidebar
            case .center: pane.minimum
            case .inspector: realization?.inspector ?? state.widths.inspector
            }
        }
        let minimumTotal = panes.reduce(CGFloat.zero) { $0 + $1.minimum }
        let desiredTotal = widths.reduce(0, +)
        if available < minimumTotal {
            // Only possible below the window's normal minimum size.
            widths = panes.map { available * $0.minimum / minimumTotal }
        } else if desiredTotal > available {
            let excess = desiredTotal - minimumTotal
            let scale = (available - minimumTotal) / excess
            widths = zip(panes, widths).map { pane, width in
                pane.minimum + (width - pane.minimum) * scale
            }
        } else if let center = panes.firstIndex(of: .center) {
            widths[center] += available - desiredTotal
        }
        var x = bounds.minX
        for (index, width) in widths.enumerated() {
            let frame = NSRect(x: x, y: bounds.minY, width: width, height: bounds.height)
            if arrangedSubviews[index].frame != frame { arrangedSubviews[index].frame = frame }
            x += width + dividerThickness
        }
    }

    override func mouseDown(with event: NSEvent) {
        let before = visibleWidths()
        trackingDivider = true
        super.mouseDown(with: event)
        trackingDivider = false
        recordChanges(from: before)
    }

    // Explicit native setPosition calls are intentional divider moves too
    // (including accessibility/test clients), unlike frame fitting above.
    override func setPosition(_ position: CGFloat, ofDividerAt dividerIndex: Int) {
        guard panes.indices.contains(dividerIndex), panes.indices.contains(dividerIndex + 1) else { return }
        let before = visibleWidths()
        let minimum = splitView(self, constrainMinCoordinate: 0, ofSubviewAt: dividerIndex)
        let maximum = splitView(self, constrainMaxCoordinate: bounds.maxX, ofSubviewAt: dividerIndex)
        let wasTracking = trackingDivider
        trackingDivider = true
        super.setPosition(max(minimum, min(maximum, position)), ofDividerAt: dividerIndex)
        trackingDivider = wasTracking
        recordChanges(from: before)
    }

    private func visibleWidths() -> [Pane: CGFloat] {
        guard arrangedSubviews.count == panes.count else { return [:] }
        return Dictionary(uniqueKeysWithValues: zip(panes, arrangedSubviews).map { ($0, $1.frame.width) })
    }

    private func recordChanges(from before: [Pane: CGFloat]) {
        guard !placing, !configuring, documentURL == state.activeDocumentURL else { return }
        let after = visibleWidths()
        // Only save the side actually moved. The other side may currently be
        // clamped by a smaller window and must retain its larger preference.
        func changed(_ pane: Pane) -> CGFloat? {
            guard let old = before[pane], let new = after[pane], abs(old - new) > 0.01 else { return nil }
            return new
        }
        state.record(
            sidebar: changed(.sidebar), inspector: changed(.inspector),
            realization: .init(viewportWidth: bounds.width, dividerThickness: dividerThickness,
                               sidebar: after[.sidebar], inspector: after[.inspector])
        )
    }

    func splitView(_ splitView: NSSplitView, canCollapseSubview subview: NSView) -> Bool { false }

    func splitView(_ splitView: NSSplitView, constrainMinCoordinate proposedMinimumPosition: CGFloat,
                   ofSubviewAt dividerIndex: Int) -> CGFloat {
        arrangedSubviews[dividerIndex].frame.minX + panes[dividerIndex].minimum
    }

    func splitView(_ splitView: NSSplitView, constrainMaxCoordinate proposedMaximumPosition: CGFloat,
                   ofSubviewAt dividerIndex: Int) -> CGFloat {
        arrangedSubviews[dividerIndex + 1].frame.maxX - panes[dividerIndex + 1].minimum - dividerThickness
    }
}

/// TeXifier-style symbols palette: a category menu above a glyph grid.
/// Rows come from the shared `TexSymbolCatalogue` so macOS and Linux list
/// identical symbols in identical order.
private struct SymbolsPaletteView: View {
    let onInsert: (TexSymbol) -> Void
    @State private var category = SymbolCategory.greekLetters

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Picker(selection: $category) {
                ForEach(SymbolCategory.allCases, id: \.self) { item in
                    Text(LocalizedStringKey(item.titleKey)).tag(item)
                }
            } label: {
                Text("editor.symbols")
            }
            .labelsHidden()
            .pickerStyle(.menu)
            .accessibilityIdentifier("pitex.symbols.category")

            ScrollView {
                LazyVGrid(
                    columns: [GridItem(.adaptive(minimum: 32), spacing: 2)],
                    spacing: 2
                ) {
                    ForEach(TexSymbolCatalogue.symbols(in: category), id: \.command) { symbol in
                        Button {
                            onInsert(symbol)
                        } label: {
                            Text(symbol.glyph)
                                .font(.system(size: 17))
                                .frame(width: 30, height: 30)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .help(symbol.command)
                    }
                }
            }
        }
        .padding(10)
        .frame(width: 320, height: 320)
        .accessibilityIdentifier("pitex.symbols.palette")
    }
}
