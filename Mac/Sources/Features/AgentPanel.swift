import AppKit
import AppPorts
import GitCore
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
                approvalRow(approval)
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

    private func approvalRow(_ approval: AgentApprovalRequest) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(verbatim: approval.title)
                .font(.callout.weight(.semibold))
            if !approval.detail.isEmpty {
                Text(verbatim: approval.detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(6)
                    .textSelection(.enabled)
            }
            HStack(spacing: 8) {
                Button("assistant.approve") {
                    coordinator.respondToApproval("approve")
                }
                .accessibilityIdentifier("pitex.assistant.approve")
                Button("assistant.deny") {
                    coordinator.respondToApproval("deny")
                }
                .accessibilityIdentifier("pitex.assistant.deny")
                Button("assistant.always_allow") {
                    coordinator.respondToApproval("always_allow")
                }
                .accessibilityIdentifier("pitex.assistant.alwaysAllow")
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(.quaternary.opacity(0.35), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityIdentifier("pitex.assistant.approval")
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
                            openFile: { coordinator.openToolFile($0) }
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
                }
                if !entry.text.isEmpty {
                    Text(verbatim: entry.text)
                        .font(.system(size: fontSize))
                        .textSelection(.enabled)
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

    private static func diffLines(_ diff: String) -> [(text: String, color: Color?)] {
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
