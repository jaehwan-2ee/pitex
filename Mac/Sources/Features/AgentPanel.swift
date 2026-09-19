import AppKit
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

            composerRow
                .padding(10)
        }
        .accessibilityIdentifier("pitex.assistant")
        .task { coordinator.prepare() }
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
                if coordinator.currentModel == nil {
                    Text("assistant.model_placeholder").tag(Optional<PiModelDescriptor>.none)
                }
                ForEach(coordinator.models) { model in
                    Text(verbatim: modelLabel(model)).tag(Optional(model))
                }
            } label: {
                EmptyView()
            }
            .labelsHidden()
            .frame(maxWidth: 200)
            .disabled(coordinator.models.isEmpty || coordinator.isUpdatingModelSettings)
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
            case .ready, .failed:
                EmptyView()
            }
            Spacer()
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 24)
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

            TextField("assistant.message_placeholder", text: $draft)
                .font(.system(size: settings.aiFontSize))
                .textFieldStyle(.roundedBorder)
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
        guard draft.hasPrefix("/"), draft.rangeOfCharacter(from: .whitespaces) == nil else { return [] }
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

    /// Disambiguates same-named models across providers once more than one
    /// provider is configured (`name · provider`).
    private func modelLabel(_ model: PiModelDescriptor) -> String {
        let providers = Set(coordinator.models.map(\.provider))
        guard providers.count > 1 else { return model.pickerTitle }
        return "\(model.pickerTitle) · \(model.provider)"
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

    private var modelBinding: Binding<PiModelDescriptor?> {
        Binding(
            get: { coordinator.currentModel },
            set: { if let model = $0 { coordinator.selectModel(model) } }
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
                    ForEach(coordinator.transcript) { entry in
                        AgentEntryRow(entry: entry, fontSize: settings.aiFontSize)
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
            DisclosureGroup {
                if !entry.detail.isEmpty {
                    Text(verbatim: entry.detail)
                        .font(.system(size: max(fontSize - 2, 9), design: .monospaced))
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            } label: {
                HStack(spacing: 5) {
                    switch entry.status {
                    case .running:
                        ProgressView().controlSize(.mini)
                    case .done:
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundStyle(.green)
                    case .failed:
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.red)
                    case .streaming:
                        EmptyView()
                    }
                    Text(verbatim: entry.title)
                        .font(captionFont.weight(.semibold))
                    if !entry.detail.isEmpty, entry.status == .running {
                        Text(verbatim: firstLine(entry.detail))
                            .font(captionFont)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }
            }
        case .notice:
            Text(verbatim: entry.text)
                .font(captionFont)
                .foregroundStyle(entry.status == .failed ? Color.red : Color.secondary)
                .frame(maxWidth: .infinity, alignment: .center)
        }
    }

    private func firstLine(_ text: String) -> String {
        text.components(separatedBy: .newlines).first ?? text
    }
}
