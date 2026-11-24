import AppKit
import AppPorts
import BuildCore
import GitCore
import LanguageCore
import SwiftUI

/// Native chat surface for the embedded pi agent, laid out like the host
/// app's own assistant pane: a header row with the model picker, the
/// attach-document toggle and a clear button; a streamed transcript; and a
/// composer row with attach, a rounded field and a circular send button.
/// The agent itself edits project files; this panel only renders its event
/// stream. It lives in the bottom console's Assistant tab.
struct AgentPanel: View {
    @ObservedObject var coordinator: AgentCoordinator
    @ObservedObject var settings: SettingsStore
    @ObservedObject private var subscriptionUsage = SubscriptionUsageStore.shared
    @State private var draft = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("assistant.title")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 10)
                .padding(.top, 8)

            controlRow
                .padding(.horizontal, 10)
                .padding(.vertical, 6)

            // Why an enabled backend lists no models; selectable to copy a command.
            if !coordinator.catalogNotes.isEmpty {
                Text(verbatim: coordinator.catalogNotes.joined(separator: "\n"))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 4)
                    .accessibilityIdentifier("pitex.assistant.catalogNotes")
            }

            transcriptArea

            if let status = coordinator.statusMessage {
                Text(verbatim: status)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 4)
                    .accessibilityIdentifier("pitex.assistant.status")
            }

            if let selection = coordinator.selectionAttachment {
                selectionChip(selection)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 4)
            }

            if let approval = coordinator.pendingApproval {
                AgentApprovalCard(
                    approval: approval, fontSize: settings.aiFontSize, projectRoot: coordinator.projectRoot(),
                    openLink: { coordinator.openFile($0, $1) },
                    answer: { coordinator.answerApproval(id: approval.id, $0) }
                )
                // A new request gets fresh picks and feedback.
                .id(approval.id)
                .padding(.horizontal, 10)
                .padding(.bottom, 4)
            }

            if !coordinator.todos.isEmpty {
                AgentTodoChecklist(items: coordinator.todos, isRunning: coordinator.isRunning, fontSize: settings.aiFontSize)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 4)
            }

            composerRow
                .padding(10)
        }
        .accessibilityIdentifier("pitex.assistant")
        .task {
            coordinator.reconcileEnablement(settings: settings)
            coordinator.prepare()
        }
        .onChange(of: settings.disabledBackends) { _, _ in
            coordinator.reconcileEnablement(settings: settings)
        }
        .onChange(of: coordinator.pendingComposerInsertion) { _, insertion in
            if let insertion {
                draft += insertion
                coordinator.pendingComposerInsertion = nil
            }
        }
    }

    /// Header controls: model popup on the left; the attach-active-document
    /// toggle and clear-conversation button on the right — mirroring the
    /// reference panel's single control row.
    private var controlRow: some View {
        HStack(spacing: 8) {
            Image(systemName: "wand.and.stars")
                .foregroundStyle(.secondary)

            Picker(selection: modelBinding) {
                if coordinator.catalogMenu.selection == nil || coordinator.catalogMenu.rows.isEmpty {
                    Text("assistant.model_placeholder").tag(Optional<AssistantCatalog.RowID>.none)
                }
                ForEach(catalogSections) { section in
                    Section {
                        ForEach(section.rows) { row in
                            Text(verbatim: rowTitle(row, among: section.rows)).tag(Optional(row.id))
                        }
                    } header: {
                        Text(verbatim: section.title)
                    }
                }
            } label: {
                EmptyView()
            }
            .labelsHidden()
            .frame(maxWidth: 320)
            .disabled(!coordinator.catalogMenu.acceptsPicks)
            .accessibilityIdentifier("pitex.assistantModel")

            if coordinator.connection == .ready, !coordinator.thinkingLevels.isEmpty {
                Picker(selection: thinkingBinding) {
                    ForEach(coordinator.thinkingLevels, id: \.self) { level in
                        Text(verbatim: level).tag(level)
                    }
                } label: {
                    EmptyView()
                }
                .labelsHidden()
                .fixedSize()
                .disabled(coordinator.thinkingLevels.count < 2 || coordinator.isUpdatingModelSettings)
                .help(String(localized: "assistant.reasoning_help"))
                .accessibilityIdentifier("pitex.assistantReasoning")
            }

            // Ask before edits / Accept edits / Plan, when the backend reports modes.
            if coordinator.connection == .ready, let mode = coordinator.permissionMode {
                Picker(selection: Binding(get: { mode.id }, set: { coordinator.selectPermissionMode($0) })) {
                    ForEach(mode.options) { option in
                        Text(verbatim: Self.modeTitle(option)).tag(option.id)
                    }
                } label: {
                    EmptyView()
                }
                .labelsHidden()
                .fixedSize()
                // Edits apply without asking: tint the picker so it stays noticed.
                .padding(2)
                .background(mode.autoAppliesEdits ? Color.orange.opacity(0.25) : .clear, in: RoundedRectangle(cornerRadius: 6))
                .help(mode.autoAppliesEdits ? String(localized: "assistant.mode.edits_auto_help") : String(localized: "assistant.mode.help"))
                .accessibilityLabel(Text("assistant.mode.help"))
                .accessibilityIdentifier("pitex.assistantMode")
            }

            Spacer(minLength: 4)

            Text("assistant.attach_document")
                .font(.callout)
                .lineLimit(1)
                .fixedSize()
            Toggle(isOn: $coordinator.attachActiveDocument) {
                EmptyView()
            }
            .labelsHidden()
            .toggleStyle(.switch)
            .controlSize(.small)
            .help("Include the currently active document so the assistant can reference it.")
            .accessibilityIdentifier("pitex.assistantContext")

            Button {
                coordinator.newSession()
            } label: {
                Image(systemName: "trash")
            }
            .buttonStyle(.borderless)
            .disabled(coordinator.connection != .ready)
            .help(String(localized: "assistant.clear"))
            .accessibilityIdentifier("pitex.agent.newSession")

            // Past conversations of this project; `/resume` opens it too.
            Button {
                coordinator.showSessionHistory()
            } label: {
                Image(systemName: "clock.arrow.circlepath")
            }
            .buttonStyle(.borderless)
            .disabled(coordinator.connection != .ready || coordinator.isRunning)
            .help(String(localized: "assistant.history"))
            .popover(isPresented: $coordinator.showingSessionHistory, arrowEdge: .bottom) {
                AgentSessionHistory(coordinator: coordinator)
            }

            if let usage = usageText {
                Text(String(format: String(localized: "assistant.usage"), usage))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .accessibilityIdentifier("pitex.assistant.usage")
            }
        }
    }

    @ViewBuilder
    private var transcriptArea: some View {
        Group {
            if coordinator.transcript.isEmpty {
                emptyState
            } else {
                transcriptView
            }
        }
        .accessibilityIdentifier("pitex.ai")
    }

    private var emptyState: some View {
        VStack(spacing: 10) {
            Spacer()
            Image(systemName: "wand.and.stars")
                .font(.system(size: 30))
                .foregroundStyle(.tertiary)
            Text("assistant.empty_headline")
                .font(.system(size: settings.aiFontSize, weight: .medium))
            switch coordinator.connection {
            case .idle:
                Text("assistant.configure_hint")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                Button("assistant.open_pi_config") {
                    try? PiPaths.openConfiguration()
                }
                .accessibilityIdentifier("pitex.piConfig")
            case .connecting:
                ProgressView()
                    .controlSize(.small)
            case .piMissing:
                Text("assistant.pi_missing")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                installRuntimeButton
            case .ready:
                EmptyView()
            case .failed(let message):
                Text(verbatim: message)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                // Install failures (offline, checksum) land here; retry is one click.
                if !PiExecutableLocator.appLocalRuntimeInstalled { installRuntimeButton }
            }
            Spacer()
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 24)
    }

    private var installRuntimeButton: some View {
        Button("settings.ai.install_runtime") { coordinator.installRuntime() }
            .accessibilityIdentifier("pitex.assistant.installRuntime")
    }

    /// Composer: attach button, rounded text field, circular send/stop.
    private var composerRow: some View {
        VStack(alignment: .leading, spacing: 4) {
            if !slashMatches.isEmpty {
                slashCompletionList
            }
            composerInputRow
        }
    }

    private var composerInputRow: some View {
        HStack(alignment: .bottom, spacing: 8) {
            Button {
                attachFiles()
            } label: {
                Image(systemName: "paperclip")
            }
            .buttonStyle(.borderless)
            .help(String(localized: "assistant.attach_files_help"))
            .accessibilityIdentifier("pitex.assistant.attach")

            TextField("assistant.message_placeholder", text: $draft, axis: .vertical)
                .font(.system(size: settings.aiFontSize))
                .textFieldStyle(.roundedBorder)
                // Grows with content up to 8 visible lines, then scrolls.
                // Return still submits (onSubmit -> send); Option+Return inserts
                // a newline.
                .lineLimit(1...8)
                .onSubmit(sendDraft)
                .accessibilityIdentifier("pitex.assistant.message")

            if coordinator.isRunning {
                Button {
                    coordinator.stop()
                } label: {
                    Image(systemName: "stop.circle.fill")
                        .font(.title2)
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .help(String(localized: "assistant.stop_help"))
                .accessibilityIdentifier("pitex.assistantCancel")
            } else {
                Button {
                    sendDraft()
                } label: {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.title2)
                }
                .buttonStyle(.plain)
                .foregroundStyle(Color.accentColor)
                .disabled(!canSend)
                .help(String(localized: "assistant.send_help"))
                .keyboardShortcut(.return, modifiers: [.command])
                .accessibilityIdentifier("pitex.assistantSend")
            }
        }
    }

    /// `/`-prefixed drafts complete against pi's advertised commands
    /// (extensions, prompts, skills); picking one fills the composer.
    private var slashMatches: [PiSlashCommand] {
        guard draft.hasPrefix("/"), draft.rangeOfCharacter(from: .whitespacesAndNewlines) == nil else { return [] }
        let query = String(draft.dropFirst())
        return coordinator.commands
            .filter { query.isEmpty || $0.name.lowercased().hasPrefix(query.lowercased()) }
            .prefix(8)
            .map { $0 }
    }

    private var slashCompletionList: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(slashMatches) { command in
                Button {
                    draft = "/\(command.name) "
                } label: {
                    HStack(spacing: 6) {
                        Text(verbatim: "/\(command.name)")
                            .font(.callout.monospaced())
                        if let description = command.description, !description.isEmpty {
                            Text(verbatim: description)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                        }
                        Spacer(minLength: 4)
                        Text(verbatim: command.source)
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
            }
        }
        .background(Color.secondary.opacity(0.10), in: RoundedRectangle(cornerRadius: 6))
        .accessibilityIdentifier("pitex.assistant.slashCommands")
    }

    /// `12.3k/200k ctx (6%) · 5h 12% · 7d 34%` for OAuth subscription
    /// providers (Claude, Codex); `… · 45.2k tok · $0.12` for API-token providers.
    private var usageText: String? {
        var parts: [String] = []
        if let stats = coordinator.sessionStats,
           let tokens = stats.contextTokens, let window = stats.contextWindow {
            var context = "\(Self.compactTokens(tokens))/\(Self.compactTokens(window)) ctx"
            if let percent = stats.contextPercent {
                context += " (\(Int(percent.rounded()))%)"
            }
            parts.append(context)
        }
        if let windows = subscriptionUsage.windows, !windows.isEmpty {
            parts += windows.map { "\($0.label) \(Int($0.percent.rounded()))%" }
        } else if let stats = coordinator.sessionStats {
            parts.append("\(Self.compactTokens(stats.totalTokens)) tok")
            parts.append(String(format: "$%.2f", stats.cost))
        }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }

    private static func compactTokens(_ value: Int) -> String {
        if value >= 1_000 {
            return String(format: "%.1fk", Double(value) / 1_000)
        }
        return "\(value)"
    }

    private struct CatalogSection: Identifiable {
        var title: String
        var rows: [AssistantCatalog.Row]
        var id: String { title }
    }

    private var catalogSections: [CatalogSection] {
        var sections: [CatalogSection] = []
        for row in coordinator.catalogMenu.rows {
            if let index = sections.lastIndex(where: { $0.title == row.section }), index == sections.count - 1 {
                sections[index].rows.append(row)
            } else {
                sections.append(CatalogSection(title: row.section, rows: [row]))
            }
        }
        return sections
    }

    /// Inside a backend section the row is the model. Two RPC providers in one
    /// section keep the provider name so identical titles stay distinct.
    private func rowTitle(_ row: AssistantCatalog.Row, among rows: [AssistantCatalog.Row]) -> String {
        let providers = Set(rows.map(\.model.provider))
        guard providers.count > 1 else { return row.model.pickerTitle }
        return "\(row.model.pickerTitle) · \(row.model.provider)"
    }

    /// Chip shown while the user drags over editor text: filename + line
    /// range on the left, dismiss X on the right. Re-appears on the next
    /// distinct selection after a dismiss.
    private func selectionChip(_ selection: AgentSelectionAttachment) -> some View {
        HStack(spacing: 6) {
            Image(systemName: "text.quote")
                .foregroundStyle(.secondary)
            Text(verbatim: selectionLabel(selection))
                .font(.caption)
                .lineLimit(1)
                .truncationMode(.middle)
            Spacer(minLength: 4)
            Button {
                coordinator.clearSelectionAttachment()
            } label: {
                Image(systemName: "xmark")
                    .imageScale(.small)
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
            .accessibilityLabel("assistant.selection_remove")
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color.secondary.opacity(0.14), in: RoundedRectangle(cornerRadius: 6))
        .accessibilityIdentifier("pitex.assistant.selection")
    }

    private func selectionLabel(_ selection: AgentSelectionAttachment) -> String {
        let file = selection.path.split(separator: "/").last.map(String.init) ?? selection.path
        if selection.startLine == selection.endLine {
            return "\(file):\(selection.startLine)"
        }
        return "\(file):\(selection.startLine)–\(selection.endLine)"
    }

    private var canSend: Bool {
        coordinator.connection != .piMissing
            && !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func sendDraft() {
        guard canSend else { return }
        coordinator.send(prompt: draft)
        draft = ""
    }

    private func attachFiles() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = true
        guard panel.runModal() == .OK else { return }
        let paths = panel.urls.map { url -> String in
            if let root = coordinator.contextProvider().projectRoot,
               let relative = try? WorkspaceModel.relativePath(for: url, root: root).rawValue {
                return relative
            }
            return url.path
        }
        coordinator.insertIntoComposer(paths.joined(separator: " ") + " ")
    }

    private var modelBinding: Binding<AssistantCatalog.RowID?> {
        Binding(
            get: { coordinator.catalogMenu.selection },
            set: { selection in
                guard let selection else { return }
                coordinator.applyCatalog(
                    .row(selection),
                    shownGeneration: coordinator.catalogMenu.generation,
                    settings: settings
                )
            }
        )
    }

    static func modeTitle(_ option: AgentPermissionMode.Option) -> String {
        guard let key = AgentPermissionMode.labelKey(id: option.id, label: option.label) else { return option.label }
        return String(localized: String.LocalizationValue(key))
    }

    private var thinkingBinding: Binding<String> {
        Binding(
            get: { coordinator.thinkingLevel },
            set: { coordinator.selectThinkingLevel($0) }
        )
    }

    private var transcriptView: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
                    let root = coordinator.projectRoot()
                    ForEach(coordinator.transcript) { entry in
                        AgentEntryRow(
                            entry: entry, fontSize: settings.aiFontSize, projectRoot: root,
                            openFile: { coordinator.openToolFile($0) },
                            openLink: { coordinator.openFile($0, $1) }
                        )
                        .id(entry.id)
                    }
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
            }
            .onChange(of: coordinator.transcript.count) { _, _ in
                if let last = coordinator.transcript.last {
                    proxy.scrollTo(last.id, anchor: .bottom)
                }
            }
            .onChange(of: coordinator.transcript.last?.text) { _, _ in
                if let last = coordinator.transcript.last {
                    proxy.scrollTo(last.id, anchor: .bottom)
                }
            }
        }
        .accessibilityIdentifier("pitex.assistant.transcript")
    }
}

/// One transcript row: user bubbles on the right, assistant text on the
/// left, thinking (collapsible), tool calls with their running state, and
/// agent notices.
private struct AgentEntryRow: View {
    let entry: AgentTranscriptEntry
    let fontSize: Double
    var projectRoot: URL?
    var openFile: (AgentToolDetail) -> Void = { _ in }
    var openLink: (URL, Int?) -> Void = { _, _ in }
    private var captionFont: Font { .system(size: max(fontSize - 2, 9)) }

    var body: some View {
        switch entry.role {
        case .user:
            HStack {
                Spacer(minLength: 32)
                Text(verbatim: entry.text)
                    .font(.system(size: fontSize))
                    .textSelection(.enabled)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 7)
                    .background(Color.accentColor.opacity(0.16), in: RoundedRectangle(cornerRadius: 10))
            }
        case .assistant:
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 5) {
                    Text(verbatim: entry.title)
                        .font(captionFont.weight(.semibold))
                    if entry.status == .streaming {
                        ProgressView().controlSize(.mini)
                    }
                    if !entry.text.isEmpty {
                        // Copies the raw Markdown of the whole message.
                        Button {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(entry.text, forType: .string)
                        } label: {
                            Image(systemName: "doc.on.doc")
                        }
                        .buttonStyle(.borderless)
                        .font(captionFont)
                        .help("assistant.markdown.copy_message")
                        .accessibilityLabel(Text("assistant.markdown.copy_message"))
                    }
                }
                if !entry.text.isEmpty {
                    if entry.status == .failed {
                        Text(verbatim: entry.text)
                            .font(.system(size: fontSize))
                            .textSelection(.enabled)
                    } else {
                        AgentMarkdownView(text: entry.text, streaming: entry.status == .streaming,
                                          fontSize: fontSize, projectRoot: projectRoot, openLink: openLink)
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        case .thinking:
            DisclosureGroup {
                Text(verbatim: entry.text)
                    .font(captionFont)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            } label: {
                HStack(spacing: 5) {
                    Text(verbatim: entry.title)
                        .font(captionFont.weight(.semibold))
                        .foregroundStyle(.secondary)
                    if entry.status == .streaming {
                        ProgressView().controlSize(.mini)
                    }
                }
            }
        case .tool:
            AgentToolCard(entry: entry, fontSize: fontSize, projectRoot: projectRoot, openFile: openFile)
        case .notice:
            Text(verbatim: entry.text)
                .font(captionFont)
                .foregroundStyle(entry.status == .failed ? Color.red : Color.secondary)
                .frame(maxWidth: .infinity, alignment: .center)
        case .activity:
            HStack(alignment: .firstTextBaseline, spacing: 5) {
                Image(systemName: activitySymbol)
                    .accessibilityHidden(true)
                Text(verbatim: String(format: String(localized: String.LocalizationValue(entry.title)), entry.text))
            }
            .font(captionFont)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var activitySymbol: String {
        switch entry.title {
        case AgentTodoActivity.completed("").key: return "checkmark.circle"
        case AgentTodoActivity.started("").key: return "play.circle"
        default: return "checklist"
        }
    }
}

/// The agent's todo list, pinned above the composer while the chat has one.
/// Collapsed it is one line ("Tasks 2/5", plus " · Fixing the citation" while a
/// turn runs, and a progress bar); expanded it lists every item with its status
/// mark, scrolling past 200 pt.
private struct AgentTodoChecklist: View {
    let items: [AgentTodoItem]
    let isRunning: Bool
    let fontSize: Double
    @State private var expanded = false
    private var captionFont: Font { .system(size: max(fontSize - 2, 9)) }

    var body: some View {
        let progress = AgentTodoList.progress(items)
        DisclosureGroup(isExpanded: $expanded) {
            // A long list scrolls instead of squeezing the transcript (Paseo caps its panel too).
            ViewThatFits(in: .vertical) {
                rows
                ScrollView { rows }
            }
            .frame(maxHeight: 200)
            .padding(.top, 4)
        } label: {
            HStack(spacing: 8) {
                Text(verbatim: summary(progress))
                    .font(captionFont.weight(.semibold))
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: 4)
                ProgressView(value: Double(progress.done), total: Double(max(progress.total, 1)))
                    .frame(width: 64)
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(Text(verbatim: String(format: String(localized: "assistant.todo.a11y"), progress.done, progress.total)))
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 5)
        .background(.quaternary.opacity(0.35), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityIdentifier("pitex.assistant.todos")
    }

    private var rows: some View {
        VStack(alignment: .leading, spacing: 3) {
            ForEach(Array(items.enumerated()), id: \.offset) { _, item in
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    mark(item.status)
                    Text(verbatim: item.text)
                        .strikethrough(item.status == .completed)
                        .foregroundStyle(item.status == .completed ? Color.secondary : Color.primary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .font(captionFont)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(Text(verbatim: "\(statusTitle(item.status)): \(item.text)"))
            }
        }
    }

    private func summary(_ progress: (done: Int, total: Int)) -> String {
        let counts = String(format: String(localized: "assistant.todo.summary"), progress.done, progress.total)
        // After the turn, a present-tense step would claim work that is not running.
        guard isRunning, let current = AgentTodoList.current(items) else { return counts }
        return "\(counts) · \(current)"
    }

    @ViewBuilder
    private func mark(_ status: AgentTodoItem.Status) -> some View {
        switch status {
        case .pending:
            Image(systemName: "circle").foregroundStyle(.secondary)
        case .inProgress:
            Image(systemName: "circle.lefthalf.filled").foregroundStyle(Color.accentColor)
        case .completed:
            Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        }
    }

    private func statusTitle(_ status: AgentTodoItem.Status) -> String {
        switch status {
        case .pending: return String(localized: "assistant.todo.status.pending")
        case .inProgress: return String(localized: "assistant.todo.status.in_progress")
        case .completed: return String(localized: "assistant.todo.status.completed")
        }
    }
}

/// One tool call, collapsed by default: kind icon, status, a one-line title
/// and a badge (exit code or +/- counts). Expanded: the command and output,
/// a colored diff with "Open file", or the call's fields. Rows without a
/// typed `tool` (pi's own provider) show the name and plain detail.
private struct AgentToolCard: View {
    let entry: AgentTranscriptEntry
    let fontSize: Double
    let projectRoot: URL?
    let openFile: (AgentToolDetail) -> Void
    private var captionFont: Font { .system(size: max(fontSize - 2, 9)) }
    private var monoFont: Font { .system(size: max(fontSize - 2, 9), design: .monospaced) }

    var body: some View {
        DisclosureGroup {
            VStack(alignment: .leading, spacing: 4) {
                if let tool = entry.tool {
                    fields(tool)
                }
                if !entry.detail.isEmpty, !showsDiff {
                    Text(verbatim: entry.detail)
                        .font(monoFont)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                if let tool = entry.tool, tool.openableURL(root: projectRoot) != nil {
                    Button("assistant.tool.open_file") { openFile(tool) }
                        .buttonStyle(.link)
                        .font(captionFont)
                }
            }
        } label: {
            HStack(spacing: 5) {
                Image(systemName: kindSymbol)
                    .foregroundStyle(.secondary)
                    .accessibilityHidden(true)
                statusIcon
                Text(verbatim: title)
                    .font(captionFont.weight(.semibold))
                    .lineLimit(1)
                    .truncationMode(.middle)
                if (entry.tool?.kind ?? .other) == .other, !entry.detail.isEmpty, entry.status == .running {
                    Text(verbatim: entry.detail.components(separatedBy: .newlines).first ?? entry.detail)
                        .font(captionFont)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                if let badge = entry.tool?.badge(phrase: Self.phrase) {
                    Text(verbatim: badge)
                        .font(monoFont)
                        .foregroundStyle(entry.tool?.kind == .shell ? Color.red : Color.secondary)
                        .padding(.horizontal, 5)
                        .background(Color.secondary.opacity(0.15), in: Capsule())
                }
            }
        }
    }

    private var title: String {
        entry.tool?.title(fallback: entry.title, root: projectRoot, phrase: Self.phrase) ?? entry.title
    }

    /// The diff stands in for an edit's output, except a failure's error text.
    private var showsDiff: Bool {
        guard let tool = entry.tool, entry.status != .failed else { return false }
        return (tool.kind == .edit || tool.kind == .write) && tool.diff != nil
    }

    @ViewBuilder
    private var statusIcon: some View {
        switch entry.status {
        case .running, .streaming:
            ProgressView().controlSize(.mini)
                .accessibilityLabel(Text("assistant.tool.status_running"))
        case .done:
            Image(systemName: "checkmark.circle.fill")
                .foregroundStyle(.green)
                .accessibilityLabel(Text("assistant.tool.status_completed"))
        case .failed:
            Image(systemName: "xmark.circle.fill")
                .foregroundStyle(.red)
                .accessibilityLabel(Text("assistant.tool.status_failed"))
        case .canceled:
            Image(systemName: "stop.circle.fill")
                .foregroundStyle(.orange)
                .accessibilityLabel(Text("assistant.tool.status_canceled"))
        }
    }

    private var kindSymbol: String {
        switch entry.tool?.kind {
        case .shell: return "terminal"
        case .read: return "doc.text"
        case .edit: return "pencil"
        case .write: return "doc.badge.plus"
        case .search: return "magnifyingglass"
        case .fetch: return "globe"
        case .subAgent: return "person.2"
        case .think: return "brain"
        case .other, nil: return "wrench.and.screwdriver"
        }
    }

    @ViewBuilder
    private func fields(_ tool: AgentToolDetail) -> some View {
        switch tool.kind {
        case .shell:
            if let command = tool.command {
                Text(verbatim: "$ " + command)
                    .font(monoFont)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        case .edit, .write:
            if let diff = tool.diff {
                diffView(diff)
            }
        default:
            let path = tool.path.map { _ in tool.displayPath(root: projectRoot) }
            ForEach([path, tool.query, tool.url, tool.cwd].compactMap { $0 }, id: \.self) { value in
                Text(verbatim: value)
                    .font(monoFont)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        }
    }

    /// The diff as one column of tinted lines, parsed like the Git diff
    /// viewer (`GitSupport.parseFileDiff`, `CommitDiffView` colors).
    private func diffView(_ diff: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(Self.diffLines(diff).enumerated()), id: \.offset) { _, line in
                Text(verbatim: line.text)
                    .font(monoFont)
                    .foregroundStyle(line.color == nil ? Color.secondary : Color.primary)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(line.color ?? .clear)
            }
        }
    }

    static func diffLines(_ diff: String) -> [(text: String, color: Color?)] {
        var lines: [(text: String, color: Color?)] = []
        for row in GitSupport.parseFileDiff(diff) {
            switch row {
            case let .hunk(header):
                lines.append((header, nil))
            case let .meta(text):
                if !text.hasPrefix("---"), !text.hasPrefix("+++") { lines.append((text, nil)) }
            case let .pair(left, right):
                if let left, left.kind == .context {
                    lines.append((" " + left.text, Color.clear))
                    continue
                }
                if let left { lines.append(("-" + left.text, Color.red.opacity(0.16))) }
                if let right { lines.append(("+" + right.text, Color.green.opacity(0.16))) }
            }
        }
        return lines
    }

    private static func phrase(_ phrase: AgentToolDetail.Phrase) -> String {
        switch phrase {
        case .ran: return String(localized: "assistant.tool.ran")
        case .edited: return String(localized: "assistant.tool.edited")
        case .wrote: return String(localized: "assistant.tool.wrote")
        case .read: return String(localized: "assistant.tool.read")
        case .lines: return String(localized: "assistant.tool.lines")
        case .searched: return String(localized: "assistant.tool.searched")
        case .matches: return String(localized: "assistant.tool.matches")
        case .fetched: return String(localized: "assistant.tool.fetched")
        case .subagent: return String(localized: "assistant.tool.subagent")
        case .thought: return String(localized: "assistant.tool.thought")
        case .exit: return String(localized: "assistant.tool.exit")
        }
    }
}

/// The approval card: a tool request with its diff or command, a question
/// form, or a plan to review (M3 Markdown), over the buttons the bridge offered.
private struct AgentApprovalCard: View {
    let approval: AgentApprovalItem
    let fontSize: Double
    let projectRoot: URL?
    let openLink: (URL, Int?) -> Void
    let answer: (AgentApprovalAnswer) -> Void
    /// Single-choice pick per question id (`otherTag` for "Other").
    @State private var single: [String: String] = [:]
    @State private var multi: [String: Set<String>] = [:]
    @State private var otherOn: [String: Bool] = [:]
    @State private var otherText: [String: String] = [:]
    @State private var feedback = ""
    private static let otherTag = "\u{0}other"
    private var captionFont: Font { .system(size: max(fontSize - 2, 9)) }
    private var monoFont: Font { .system(size: max(fontSize - 2, 9), design: .monospaced) }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(verbatim: title)
                .font(.callout.weight(.semibold))
            if approval.kind == "question", !approval.questions.isEmpty {
                ScrollView {
                    VStack(alignment: .leading, spacing: 10) {
                        ForEach(approval.questions, id: \.id) { question in
                            questionView(question)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 360)
                .fixedSize(horizontal: false, vertical: true)
                HStack(spacing: 8) {
                    Button("assistant.question.submit") {
                        answer(AgentApprovalAnswer(decision: "approve", answers: answers))
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(answers.count != approval.questions.count)
                    .accessibilityIdentifier("pitex.assistant.question.submit")
                    Button("assistant.question.dismiss") { answer(AgentApprovalAnswer(decision: "deny")) }
                        .accessibilityIdentifier("pitex.assistant.deny")
                }
            } else {
                if approval.kind == "plan" {
                    ScrollView {
                        AgentMarkdownView(text: approval.plan ?? approval.detail, streaming: false,
                                          fontSize: fontSize, projectRoot: projectRoot, openLink: openLink)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .frame(maxHeight: 320)
                    .fixedSize(horizontal: false, vertical: true)
                    TextField("assistant.plan.feedback_placeholder", text: $feedback)
                        .textFieldStyle(.roundedBorder)
                        .onSubmit {
                            // Return sends the feedback, like Keep planning.
                            guard !feedback.isEmpty, let keep = approval.actions.first(where: { $0.id == "keep_planning" }) else { return }
                            answer(.action(keep, legacy: false, message: feedback))
                        }
                        .accessibilityIdentifier("pitex.assistant.plan.feedback")
                } else {
                    toolBody
                }
                buttons
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(.quaternary.opacity(0.35), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityIdentifier("pitex.assistant.approval")
    }

    private var title: String {
        switch approval.kind {
        case "question": return String(localized: "assistant.question.title")
        case "plan": return String(localized: "assistant.plan.title")
        default: return approval.title
        }
    }

    @ViewBuilder
    private var toolBody: some View {
        if let tool = approval.toolDetail, tool.kind == .shell, let command = tool.command {
            Text(verbatim: "$ " + command)
                .font(monoFont)
                .textSelection(.enabled)
        } else if let tool = approval.toolDetail, tool.kind == .edit || tool.kind == .write, let diff = tool.diff {
            Text(verbatim: tool.displayPath(root: projectRoot))
                .font(monoFont)
                .foregroundStyle(.secondary)
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(AgentToolCard.diffLines(diff).enumerated()), id: \.offset) { _, line in
                        Text(verbatim: line.text)
                            .font(monoFont)
                            .foregroundStyle(line.color == nil ? Color.secondary : Color.primary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(line.color ?? .clear)
                    }
                }
                .textSelection(.enabled)
            }
            .frame(maxHeight: 220)
            .fixedSize(horizontal: false, vertical: true)
        } else if !approval.detail.isEmpty {
            Text(verbatim: approval.detail)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(6)
                .textSelection(.enabled)
        }
    }

    private var buttons: some View {
        let legacy = approval.actions.isEmpty
        return HStack(spacing: 8) {
            ForEach(legacy ? AgentApprovalAction.legacy : approval.actions, id: \.id) { action in
                let button = Button(role: action.variant == "danger" ? .destructive : nil) {
                    answer(.action(action, legacy: legacy, message: approval.kind == "plan" ? feedback : nil))
                } label: {
                    Text(verbatim: action.labelKey.map { String(localized: String.LocalizationValue($0)) } ?? action.label)
                }
                .accessibilityIdentifier(Self.identifier(action))
                if action.variant == "primary" {
                    button.buttonStyle(.borderedProminent)
                } else {
                    button
                }
            }
        }
    }

    private static func identifier(_ action: AgentApprovalAction) -> String {
        switch action.id {
        case "approve", "allow_once": return "pitex.assistant.approve"
        case "deny", "reject_once": return "pitex.assistant.deny"
        case "always_allow", "allow_always": return "pitex.assistant.alwaysAllow"
        default: return "pitex.assistant.action.\(action.id)"
        }
    }

    private func questionView(_ question: AgentApprovalQuestion) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            if !question.header.isEmpty {
                Text(verbatim: question.header)
                    .font(captionFont)
                    .padding(.horizontal, 6)
                    .background(Color.secondary.opacity(0.15), in: Capsule())
            }
            Text(verbatim: question.question)
                .font(.system(size: fontSize))
                .fixedSize(horizontal: false, vertical: true)
            if question.multiSelect {
                ForEach(question.options, id: \.label) { option in
                    Toggle(isOn: Binding(
                        get: { multi[question.id, default: []].contains(option.label) },
                        set: { on in
                            if on { multi[question.id, default: []].insert(option.label) } else { multi[question.id]?.remove(option.label) }
                        }
                    )) {
                        optionLabel(option)
                    }
                    .toggleStyle(.checkbox)
                }
                if question.allowOther {
                    Toggle("assistant.question.other", isOn: Binding(
                        get: { otherOn[question.id] ?? false },
                        set: { otherOn[question.id] = $0 }
                    ))
                    .toggleStyle(.checkbox)
                    otherField(question)
                }
            } else {
                Picker(selection: Binding(
                    get: { single[question.id] },
                    set: { single[question.id] = $0 }
                )) {
                    ForEach(question.options, id: \.label) { option in
                        optionLabel(option).tag(Optional(option.label))
                    }
                    if question.allowOther {
                        Text("assistant.question.other").tag(Optional(Self.otherTag))
                    }
                } label: {
                    EmptyView()
                }
                .labelsHidden()
                .pickerStyle(.radioGroup)
                .accessibilityLabel(Text(verbatim: question.question))
                if question.allowOther {
                    otherField(question)
                }
            }
        }
    }

    private func optionLabel(_ option: AgentApprovalQuestion.Option) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(verbatim: option.label)
            if !option.description.isEmpty {
                Text(verbatim: option.description)
                    .font(captionFont)
                    .foregroundStyle(.secondary)
            }
        }
    }

    /// Typing an answer picks "Other".
    private func otherField(_ question: AgentApprovalQuestion) -> some View {
        TextField("assistant.question.other_placeholder", text: Binding(
            get: { otherText[question.id] ?? "" },
            set: { text in
                otherText[question.id] = text
                guard !text.isEmpty else { return }
                if question.multiSelect { otherOn[question.id] = true } else { single[question.id] = Self.otherTag }
            }
        ))
        .textFieldStyle(.roundedBorder)
        .padding(.leading, 20)
        .onSubmit {
            // Return submits once every question has an answer.
            let picked = answers
            if picked.count == approval.questions.count { answer(AgentApprovalAnswer(decision: "approve", answers: picked)) }
        }
        .accessibilityIdentifier("pitex.assistant.question.other")
    }

    /// Question id → answer, only for questions that have one.
    private var answers: [String: AgentApprovalAnswer.Value] {
        var out: [String: AgentApprovalAnswer.Value] = [:]
        for question in approval.questions {
            let other = (otherText[question.id] ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            if question.multiSelect {
                var labels = question.options.map(\.label).filter { multi[question.id, default: []].contains($0) }
                if otherOn[question.id] == true, !other.isEmpty { labels.append(other) }
                if !labels.isEmpty { out[question.id] = .many(labels) }
            } else if let choice = single[question.id] {
                if choice != Self.otherTag { out[question.id] = .one(choice) } else if !other.isEmpty { out[question.id] = .one(other) }
            }
        }
        return out
    }
}

/// The resumable conversations of the current project, newest first.
private struct AgentSessionHistory: View {
    @ObservedObject var coordinator: AgentCoordinator

    var body: some View {
        Group {
            if coordinator.pastSessions.isEmpty {
                Text("assistant.history_empty")
                    .foregroundStyle(.secondary)
                    .padding(20)
            } else {
                List(coordinator.pastSessions) { session in
                    Button {
                        coordinator.resume(session)
                    } label: {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(verbatim: session.title)
                                .lineLimit(2)
                            Text(session.date, format: .dateTime.year().month().day().hour().minute())
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
                .frame(width: 380, height: 320)
            }
        }
    }
}

// MARK: - Assistant Markdown

/// An assistant message drawn from `AgentMarkdown` blocks: text runs with
/// bold, italic, code and links; code boxes with Copy; tables; and TeX math
/// typeset by the exact equation renderer (the TeX source shows until the
/// picture lands, and for good when TeX is missing).
private struct AgentMarkdownView: View {
    let text: String
    let streaming: Bool
    let fontSize: Double
    let projectRoot: URL?
    let openLink: (URL, Int?) -> Void
    @ObservedObject private var math = AgentMathRenderer.shared

    private enum Piece {
        case text(AttributedString)
        case math(String)
    }

    var body: some View {
        let document = AgentMarkdown.parse(text, streaming: streaming)
        VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(document.blocks.enumerated()), id: \.offset) { item in
                blockView(item.element)
            }
            if document.truncated {
                Text("assistant.markdown.truncated")
                    .font(.system(size: max(fontSize - 2, 9)))
                    .foregroundStyle(.secondary)
            }
        }
        .textSelection(.enabled)
        // Web links go to the browser; project files open in the editor.
        .environment(\.openURL, OpenURLAction { url in
            guard url.isFileURL else { return .systemAction }
            openLink(URL(fileURLWithPath: url.path), url.fragment.flatMap { Int($0.dropFirst()) })
            return .handled
        })
    }

    private func blockView(_ block: AgentMarkdown.Block) -> AnyView {
        switch block {
        case .paragraph(let inlines):
            return AnyView(inlineText(inlines, size: fontSize))
        case .heading(let level, let inlines):
            let scale = level == 1 ? 1.4 : level == 2 ? 1.25 : level == 3 ? 1.1 : 1
            return AnyView(inlineText(inlines, size: fontSize * scale).fontWeight(.semibold))
        case .list(let ordered, let start, let items):
            return AnyView(VStack(alignment: .leading, spacing: 3) {
                ForEach(items.indices, id: \.self) { index in
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Text(verbatim: ordered ? "\(start + index)." : "•")
                            .font(.system(size: fontSize))
                        VStack(alignment: .leading, spacing: 4) {
                            ForEach(items[index].indices, id: \.self) { inner in
                                blockView(items[index][inner])
                            }
                        }
                    }
                }
            })
        case .quote(let blocks):
            return AnyView(HStack(alignment: .top, spacing: 8) {
                RoundedRectangle(cornerRadius: 1.5)
                    .fill(.tertiary)
                    .frame(width: 3)
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(blocks.indices, id: \.self) { index in
                        blockView(blocks[index])
                    }
                }
                .foregroundStyle(.secondary)
            }
            .fixedSize(horizontal: false, vertical: true))
        case .code(let language, let text):
            return AnyView(AgentCodeBlock(language: language, code: text, fontSize: fontSize))
        case .math(let tex):
            return AnyView(mathText(tex, display: true, size: fontSize)
                .frame(maxWidth: .infinity, alignment: .center))
        case .table(let rows):
            return AnyView(ScrollView(.horizontal, showsIndicators: false) {
                Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 4) {
                    ForEach(rows.indices, id: \.self) { row in
                        GridRow {
                            ForEach(rows[row].indices, id: \.self) { column in
                                inlineText(rows[row][column], size: fontSize)
                                    .fontWeight(row == 0 ? .semibold : nil)
                            }
                        }
                        if row == 0 { Divider() }
                    }
                }
            })
        case .rule:
            return AnyView(Divider())
        }
    }

    /// One wrapping `Text`: styled runs, with inline math pictures joined in
    /// by interpolation so they flow with the words.
    private func inlineText(_ inlines: [AgentMarkdown.Inline], size: Double) -> Text {
        var pieces: [Piece] = []
        collect(inlines, intent: [], link: nil, into: &pieces)
        let parts = pieces.map { piece -> Text in
            switch piece {
            case .text(let run): return Text(run)
            case .math(let tex): return mathText(tex, display: false, size: size)
            }
        }
        let joined = parts.dropFirst().reduce(parts.first ?? Text(verbatim: "")) { Text("\($0)\($1)") }
        return joined.font(.system(size: size))
    }

    private func collect(_ inlines: [AgentMarkdown.Inline], intent: InlinePresentationIntent, link: URL?,
                         into pieces: inout [Piece]) {
        for inline in inlines {
            switch inline {
            case .text(let text):
                append(styled(text, intent, link), to: &pieces)
            case .strong(let children):
                collect(children, intent: intent.union(.stronglyEmphasized), link: link, into: &pieces)
            case .emphasis(let children):
                collect(children, intent: intent.union(.emphasized), link: link, into: &pieces)
            case .code(let code):
                var run = styled(code, intent.union(.code), link)
                run[AttributeScopes.SwiftUIAttributes.BackgroundColorAttribute.self] = Color.secondary.opacity(0.15)
                append(run, to: &pieces)
            case .link(let url, let children):
                collect(children, intent: intent, link: linkURL(url) ?? link, into: &pieces)
            case .math(let tex):
                pieces.append(.math(tex))
            }
        }
    }

    private func styled(_ text: String, _ intent: InlinePresentationIntent, _ link: URL?) -> AttributedString {
        var run = AttributedString(text)
        if !intent.isEmpty { run.inlinePresentationIntent = intent }
        if let link { run.link = link }
        return run
    }

    private func append(_ run: AttributedString, to pieces: inout [Piece]) {
        if case .text(let previous)? = pieces.last {
            pieces[pieces.count - 1] = .text(previous + run)
        } else {
            pieces.append(.text(run))
        }
    }

    /// The URL a link run carries: the web address, or the project file
    /// with its line as `#Lnn`. Nil leaves the words unlinked.
    private func linkURL(_ raw: String) -> URL? {
        switch AgentMarkdown.classifyLink(raw, root: projectRoot) {
        case .web(let url):
            return url
        case .file(let url, let line):
            guard let line, var components = URLComponents(url: url, resolvingAgainstBaseURL: false) else { return url }
            components.fragment = "L\(line)"
            return components.url ?? url
        case .unsupported:
            return nil
        }
    }

    /// The typeset picture in the text color, or the TeX source in monospace.
    private func mathText(_ tex: String, display: Bool, size: Double) -> Text {
        if let rendered = math.image(tex, display: display, fontSize: size) {
            // The crop keeps 1pt under the ink: drop it onto the baseline.
            // Labelled with its TeX source, so VoiceOver reads the formula.
            return Text(Image(rendered.cgImage, scale: rendered.scale, label: Text(verbatim: tex)).renderingMode(.template))
                .baselineOffset(display ? 0 : -1)
        }
        return Text(verbatim: display ? tex : "$\(tex)$")
            .font(.system(size: max(size - 1, 9), design: .monospaced))
    }
}

/// A fenced code block: language label, Copy, and sideways-scrolling
/// monospace text; latex/tex/bibtex take the editor's token colors.
private struct AgentCodeBlock: View {
    let language: String
    let code: String
    let fontSize: Double
    @State private var copied = false
    @ObservedObject private var appearance = AppearanceSettings.shared

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                Text(verbatim: language)
                    .foregroundStyle(.secondary)
                Spacer(minLength: 8)
                Button(copied ? LocalizedStringKey("assistant.markdown.copied") : LocalizedStringKey("assistant.markdown.copy")) {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(code, forType: .string)
                    copied = true
                    // Back to "Copy" after 1.5 s (Paseo's COPIED_RESET_MS).
                    Task {
                        try? await Task.sleep(for: .milliseconds(1500))
                        copied = false
                    }
                }
                .buttonStyle(.borderless)
            }
            .font(.system(size: max(fontSize - 2, 9)))
            ScrollView(.horizontal, showsIndicators: false) {
                Text(highlighted)
                    .font(.system(size: max(fontSize - 1, 9), design: .monospaced))
                    .fixedSize(horizontal: true, vertical: false)
            }
        }
        .padding(8)
        .background(Color.secondary.opacity(0.08), in: RoundedRectangle(cornerRadius: 6))
    }

    /// The editor's LaTeX lexer and palette on the block's text.
    private var highlighted: AttributedString {
        var result = AttributedString(code)
        let dialect: TeXDialect
        switch language.lowercased() {
        case "latex", "tex": dialect = .latex
        case "bibtex", "bib": dialect = .bibtex
        default: return result
        }
        let utf8 = code.utf8
        for token in DeterministicTeXLexer.tokenize(code, dialect: dialect) {
            let role: AppearanceColorRole
            switch token.kind {
            case .controlSequence, .bibEntryMarker: role = .commands
            case .comment: role = .comments
            case .leftBrace, .rightBrace, .punctuation: role = .braces
            case .environmentName: role = .environments
            case .math: role = .math
            case .whitespace, .text: continue
            }
            guard let lower = utf8.index(utf8.startIndex, offsetBy: token.range.utf8Offset, limitedBy: utf8.endIndex),
                  let upper = utf8.index(lower, offsetBy: token.range.utf8Length, limitedBy: utf8.endIndex),
                  let range = Range(lower..<upper, in: result) else { continue }
            result[range][AttributeScopes.SwiftUIAttributes.ForegroundColorAttribute.self] = Color(nsColor: appearance.color(for: role))
        }
        return result
    }
}

/// Typesets message math with the exact equation renderer (pdflatex, no
/// shell escape) once per formula and size. Nil until it lands, and for
/// good when TeX is missing or rejects the formula, so the view keeps the source.
@MainActor
final class AgentMathRenderer: ObservableObject {
    struct Rendered {
        let cgImage: CGImage
        let scale: CGFloat
    }

    private struct Key: Hashable {
        let tex: String
        let display: Bool
        let points: Double
    }

    static let shared = AgentMathRenderer()
    // shortcut: one cached picture and one pdflatex job per distinct formula,
    // unbounded and all at once; add an LRU cap and a queue if math-heavy chats spike.
    @Published private var images: [Key: Rendered] = [:]
    private var started: Set<Key> = []

    func image(_ tex: String, display: Bool, fontSize: Double) -> Rendered? {
        let key = Key(tex: tex, display: display, points: fontSize)
        if let rendered = images[key] { return rendered }
        if started.insert(key).inserted {
            Task { await render(key) }
        }
        return nil
    }

    private func render(_ key: Key) async {
        var environment: [String: String] = [:]
        if case let .inherit(overrides) = await StreamingBuildExecutor.buildEnvironment(.inherit(overrides: [:])) {
            environment = overrides
        }
        let token = "assistant"
        let renderer = ExactEquationRenderer(
            profile: ExactEquationProfile(engine: .pdfLaTeX, shellEscape: .disabled),
            // Private per-user directory: nothing in a shared /tmp joins TEXINPUTS.
            mainDirectory: ExactEquationRenderer.workspaceRoot(token: token),
            environment: environment,
            workspaceToken: token
        )
        let region = key.display ? "\\[\(key.tex)\\]" : "$\(key.tex)$"
        let document = ExactEquationDocument.make(preamble: nil, preambleDefinitions: [], bodyDefinitions: [], region: region)
        guard let pdf = try? await renderer.render(document: document) else { return }
        let backing = NSScreen.main?.backingScaleFactor ?? 2
        // TeX sets 10pt text: magnify to the transcript's point size.
        let scale = backing * key.points / 10
        let padding: CGFloat = key.display ? 3 : 1
        let cropped = await Task.detached(priority: .userInitiated) {
            ExactEquationImage.cropped(pdf: pdf, scale: scale, padding: padding).flatMap(inkMask).map(CroppedImage.init)
        }.value
        if let cropped { images[key] = Rendered(cgImage: cropped.image, scale: backing) }
    }
}

/// Black ink on white paper → black ink on transparency, for a template
/// image that takes the text color in light and dark.
private func inkMask(_ image: CGImage) -> CGImage? {
    let width = image.width, height = image.height
    guard let gray = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                               space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue),
          let mask = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                               space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { return nil }
    gray.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
    guard let source = gray.data?.assumingMemoryBound(to: UInt8.self),
          let target = mask.data?.assumingMemoryBound(to: UInt8.self) else { return nil }
    for y in 0..<height {
        for x in 0..<width {
            let pixel = y * mask.bytesPerRow + x * 4
            target[pixel] = 0
            target[pixel + 1] = 0
            target[pixel + 2] = 0
            target[pixel + 3] = 255 - source[y * gray.bytesPerRow + x]
        }
    }
    return mask.makeImage()
}
