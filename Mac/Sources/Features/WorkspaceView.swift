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
            Text(verbatim: url.lastPathComponent)
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

    /// HSplitView is used instead of NavigationSplitView: its conditional
    /// panes make the sidebar/inspector collapse buttons work reliably —
    /// NavigationSplitViewVisibility cannot express "content only, no
    /// detail", so a get-only binding silently ignored collapse requests.
    private var workspaceLayout: some View {
        HSplitView {
            if inspectorOnLeft {
                // A commit diff keeps the PDF inspector closed — the pane
                // cannot be opened while the diff is up.
                if workspace.inspectorVisible && workspace.gitDiff == nil { inspectorColumn }
                centerColumn
                if workspace.sidebarVisible { sidebarColumn }
            } else {
                if workspace.sidebarVisible { sidebarColumn }
                centerColumn
                if workspace.inspectorVisible && workspace.gitDiff == nil { inspectorColumn }
            }
        }
        .accessibilityIdentifier("pitex.workspace.split")
    }

    private var sidebarColumn: some View {
        ProjectSidebarView(workspace: workspace)
            .frame(minWidth: 170, idealWidth: 200, maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: appearance.color(for: .gutterBackground)))
    }

    private var inspectorColumn: some View {
        Preview(workspace: workspace)
            .frame(minWidth: 300, idealWidth: 400, maxWidth: .infinity, maxHeight: .infinity)
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
                            Button(url.lastPathComponent) { Task { await workspace.open(url) } }
                        }
                        Divider()
                        Button("command.clear_recents") { workspace.clearRecents() }
                    }
                }
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
                            Text(verbatim: url.lastPathComponent)
                                .lineLimit(1)
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
            Button { workspace.inspectorVisible.toggle() } label: {
                Image(systemName: "sidebar.right")
            }
            .disabled(workspace.gitDiff != nil)
            .help(String(localized: workspace.inspectorVisible ? "editor.hide_inspector" : "editor.show_inspector"))
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
                workspace.inspectorVisible.toggle()
            } label: {
                Label("preview.title", systemImage: "doc.richtext")
            }
            .disabled(workspace.gitDiff != nil)
            .help(String(localized: workspace.inspectorVisible ? "editor.hide_inspector" : "editor.show_inspector"))
            .accessibilityIdentifier("pitex.toolbar.pdf")

            Button {
                workspace.toggleAssistant()
            } label: {
                Label("assistant.title", systemImage: "wand.and.stars")
            }
            .help(String(localized: "editor.toggle_assistant"))
            .accessibilityIdentifier("pitex.toolbar.assistant")

            Button {
                workspace.showingSettings = true
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
