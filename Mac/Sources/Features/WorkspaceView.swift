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
    /// The sidebar divider's last real width — remounted panes (hide,
    /// mirror flip, detached preview) restore it; each window starts at
    /// the minimum. In-memory only.
    @StateObject private var sidebarWidth = SidebarWidthState()

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
    /// sidebar anchor so the anchor waits for the same arrangement.
    /// A commit diff keeps the PDF inspector closed — the pane cannot be
    /// opened while the diff is up. A detached pane lives in its own
    /// window; the column must not render a second copy here.
    private var inspectorInline: Bool {
        workspace.inspectorVisible && workspace.gitDiff == nil && !workspace.previewDetached
    }

    /// HSplitView is used instead of NavigationSplitView: its conditional
    /// panes make the sidebar/inspector collapse buttons work reliably —
    /// NavigationSplitViewVisibility cannot express "content only, no
    /// detail", so a get-only binding silently ignored collapse requests.
    private var workspaceLayout: some View {
        HSplitView {
            if inspectorOnLeft {
                if inspectorInline { inspectorColumn }
                centerColumn
                if workspace.sidebarVisible { sidebarColumn }
            } else {
                if workspace.sidebarVisible { sidebarColumn }
                centerColumn
                if inspectorInline { inspectorColumn }
            }
        }
        .accessibilityIdentifier("pitex.workspace.split")
    }

    private var sidebarColumn: some View {
        ProjectSidebarView(workspace: workspace)
            .frame(minWidth: 170, idealWidth: 170, maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: appearance.color(for: .gutterBackground)))
            .background(SidebarDividerAnchor(
                state: sidebarWidth,
                paneCount: inspectorInline ? 3 : 2,
                trailingEdge: inspectorOnLeft
            ))
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
            // like the outer HSplitView does for the side panes.
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

/// Per-window sidebar width memory: NSSplitView forgets a removed
/// arranged subview's width, and SwiftUI's idealWidth only seeds the
/// first layout — so the pane's real divider position is kept here and
/// restored by the anchor whenever the pane remounts.
private final class SidebarWidthState: ObservableObject {
    /// Matches the column's minWidth — real drags never go below it, so
    /// smaller readings mean the pane is leaving the split, not resizing.
    let minimum: CGFloat = 170
    var width: CGFloat = 170
}

/// HSplitView shares spare space across all columns on its first layout,
/// so idealWidth alone leaves the sidebar wider than its minimum. This
/// marker lives inside the sidebar pane; once the pane sits in a windowed
/// split with real frames it moves the divider once to the remembered
/// width, then only records where the user leaves it — no width lock.
private struct SidebarDividerAnchor: NSViewRepresentable {
    let state: SidebarWidthState
    /// How many panes the split shows once settled — placing earlier
    /// (sidebar+center only) would be overwritten when the inspector
    /// column joins.
    let paneCount: Int
    /// Sidebar sits at the split's right edge in the mirrored layout.
    let trailingEdge: Bool

    func makeNSView(context: Context) -> SidebarAnchorView {
        SidebarAnchorView(state: state, paneCount: paneCount, trailingEdge: trailingEdge)
    }

    func updateNSView(_ nsView: SidebarAnchorView, context: Context) {
        nsView.configure(paneCount: paneCount, trailingEdge: trailingEdge)
    }
}

private final class SidebarAnchorView: NSView {
    private let state: SidebarWidthState
    /// The arrangement the split settles into, fed from the layout
    /// condition — a real change re-arms placement, repeats do nothing.
    private var paneCount: Int
    private var trailingEdge: Bool
    /// The split/pane/slot this view last placed — a reparent or index
    /// move that keeps the window (mirror flip, pane reuse) re-places at
    /// the remembered width; leaving the window drops the placement.
    private weak var placedSplit: NSSplitView?
    private weak var placedPane: NSView?
    private var placedIndex = -1
    /// Guards recording while our own setPosition is posting.
    private var placing = false
    private weak var observedSplit: NSSplitView?
    /// nonisolated(unsafe): removed once in deinit; the token is only
    /// touched on the main thread while the view is alive.
    nonisolated(unsafe) private var resizeObserver: NSObjectProtocol?

    init(state: SidebarWidthState, paneCount: Int, trailingEdge: Bool) {
        self.state = state
        self.paneCount = paneCount
        self.trailingEdge = trailingEdge
        super.init(frame: .zero)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    deinit {
        if let resizeObserver { NotificationCenter.default.removeObserver(resizeObserver) }
    }

    /// Covers the whole pane purely as a marker — never take clicks.
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    /// SwiftUI re-feeds the expected arrangement on every body pass; only
    /// a real change re-arms placement so a layout re-applies the width.
    func configure(paneCount: Int, trailingEdge: Bool) {
        guard paneCount != self.paneCount || trailingEdge != self.trailingEdge else { return }
        self.paneCount = paneCount
        self.trailingEdge = trailingEdge
        placedSplit = nil
        placedPane = nil
        placedIndex = -1
        needsLayout = true
        reconcile()
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil {
            placedSplit = nil
            placedPane = nil
            placedIndex = -1
            unobserve()
            return
        }
        // If this attach saw a partial arranged set, guarantee a later
        // layout attempt even when our own bounds never change again.
        needsLayout = true
        reconcile()
    }

    /// First bounds land on the split's initial layout pass — that's the
    /// earliest the real pane width is known, no timer needed.
    override func layout() {
        super.layout()
        reconcile()
    }

    /// The pane (a direct arranged subview) and its index inside the
    /// first vertical NSSplitView above this marker. The sidebar's own
    /// structure/navigator split is skipped because it isn't vertical.
    private func paneContext() -> (split: NSSplitView, pane: NSView, index: Int)? {
        var pane = self as NSView
        while let superview = pane.superview {
            if let split = superview as? NSSplitView, split.isVertical,
               let index = split.arrangedSubviews.firstIndex(of: pane) {
                return (split, pane, index)
            }
            pane = superview
        }
        return nil
    }

    /// Non-nil only when the pane sits in the slot the current config
    /// expects — a transitional arrangement (fewer panes than expected
    /// while a sibling is still arriving, wrong edge mid-mirror) is nil,
    /// so nothing places or records against a layout about to move.
    private func settledContext() -> (split: NSSplitView, pane: NSView, index: Int)? {
        guard let (split, pane, index) = paneContext(),
              split.bounds.width > 0, pane.frame.width > 0,
              split.arrangedSubviews.count == paneCount,
              index == (trailingEdge ? paneCount - 1 : 0) else { return nil }
        return (split, pane, index)
    }

    private func reconcile() {
        // setPosition relayouts subviews — our own layout() re-enters
        // before the placed triple is written, so guard it too.
        guard !placing, window != nil,
              let (split, pane, index) = settledContext() else { return }
        // Same split, same pane, same slot is a routine layout, not a move.
        guard split !== placedSplit || pane !== placedPane || index != placedIndex else { return }
        placing = true
        let width = state.width
        if index == 0 {
            split.setPosition(width, ofDividerAt: 0)
        } else {
            // Trailing column (mirrored layout): the pane's right edge is
            // the split's own bounds, minus the divider strip.
            split.setPosition(split.bounds.width - split.dividerThickness - width,
                              ofDividerAt: index - 1)
        }
        placing = false
        placedSplit = split
        placedPane = pane
        placedIndex = index
        if split !== observedSplit {
            unobserve()
            observedSplit = split
            resizeObserver = NotificationCenter.default.addObserver(
                forName: NSSplitView.didResizeSubviewsNotification,
                object: split, queue: .main
            ) { [weak self] _ in
                MainActor.assumeIsolated { self?.record() }
            }
        }
    }

    private func unobserve() {
        if let resizeObserver {
            NotificationCenter.default.removeObserver(resizeObserver)
            self.resizeObserver = nil
        }
        observedSplit = nil
    }

    /// Records only in the context last placed — transitional layouts (a
    /// pane mid-reparent or before placement) never overwrite the memory.
    /// Redistributed widths (sibling removed, window resize) count too:
    /// remounts restore the latest real width, not just dragged ones.
    private func record() {
        guard !placing,
              let (split, pane, index) = settledContext(),
              split === placedSplit, pane === placedPane, index == placedIndex,
              pane.frame.width >= state.minimum else { return }
        state.width = pane.frame.width
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
