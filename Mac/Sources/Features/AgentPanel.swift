import AppKit
import AppPorts
import BuildCore
import GitCore
import LanguageCore
import SwiftUI
import UniformTypeIdentifiers

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
    /// The highlighted row of the `/` or `@` list.
    @State private var completionIndex = 0
    @State private var showAllQueued = false
    @FocusState private var composerFocused: Bool
    /// Cmd+V with only an image on the pasteboard: caught before the field editor's paste.
    @State private var pasteMonitor: Any?
    /// Follow new transcript content only while the view is at the bottom.
    @State private var scrollPin = AgentScrollPin()
    @State private var showingContextDetails = false

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

            if !coordinator.queue.items.isEmpty {
                queueList
                    .padding(.horizontal, 10)
                    .padding(.bottom, 4)
            }

            if !coordinator.draftImages.isEmpty {
                imageChips
                    .padding(.horizontal, 10)
                    .padding(.bottom, 2)
            }

            if showingContextDetails, let meter = coordinator.contextMeter {
                contextDetails(meter)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 4)
            }

            composerRow
                .padding(10)
        }
        .accessibilityIdentifier("pitex.assistant")
        // Only while the composer has focus, so the monitor needs no view state.
        .onChange(of: composerFocused) { _, focused in
            if focused { installPasteMonitor() } else { removePasteMonitor() }
        }
        .onDisappear(perform: removePasteMonitor)
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
            // Open while the backend is down too: its rows can switch to another backend.
            .disabled(coordinator.isRunning)
            .help(String(localized: "assistant.history"))
            .popover(isPresented: $coordinator.showingSessionHistory, arrowEdge: .bottom) {
                AgentSessionHistoryList(coordinator: coordinator, settings: settings)
            }

            if let usage = usageText {
                Text(String(format: String(localized: "assistant.usage"), usage))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.tail)
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
            } else if !mentionMatches.isEmpty {
                mentionCompletionList
            }
            composerInputRow
        }
        .onDrop(of: [.fileURL, .image], isTargeted: nil, perform: dropItems)
    }

    private var composerInputRow: some View {
        HStack(alignment: .bottom, spacing: 8) {
            Button {
                attachFiles()
            } label: {
                Image(systemName: "paperclip")
            }
            .buttonStyle(.borderless)
            .help(String(localized: String.LocalizationValue(
                coordinator.capabilities.images ? "assistant.attach_help" : "assistant.image.not_supported_backend"
            )))
            .accessibilityLabel(Text("assistant.attach_help"))
            .accessibilityIdentifier("pitex.assistant.attach")

            TextField("assistant.message_placeholder", text: $draft, axis: .vertical)
                .font(.system(size: settings.aiFontSize))
                .textFieldStyle(.roundedBorder)
                // Grows with content up to 8 visible lines, then scrolls.
                // Return submits (queues while a turn runs); Option+Return inserts
                // a newline; Cmd+Return sends now (steer) where the backend can.
                .lineLimit(1...8)
                .focused($composerFocused)
                .onSubmit { sendDraft() }
                .onKeyPress(.upArrow) { moveCompletion(-1) }
                .onKeyPress(.downArrow) { moveCompletion(1) }
                .onKeyPress(.tab) { applyCompletion() }
                .onKeyPress(.escape) {
                    guard completionCount > 0 else { return .ignored }
                    dismissCompletion()
                    return .handled
                }
                .onKeyPress(keys: [.return]) { press in
                    if completionCount > 0 { return applyCompletion() }
                    guard press.modifiers.contains(.command) else { return .ignored }
                    sendDraft(now: true)
                    return .handled
                }
                .onChange(of: draft) { _, _ in completionIndex = 0 }
                .accessibilityIdentifier("pitex.assistant.message")

            if let meter = coordinator.contextMeter {
                contextMeter(meter)
            }

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
            }
            // Stays beside Stop while a turn runs: a click then queues the draft,
            // and its ⌘Return shortcut sends now (steers) where the backend can.
            Button {
                sendDraft(now: NSApp.currentEvent?.type == .keyDown)
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

    /// `/`-prefixed drafts complete against pi's advertised commands
    /// (extensions, prompts, skills); picking one fills the composer.
    private var slashMatches: [PiSlashCommand] {
        guard draft.hasPrefix("/"), draft.rangeOfCharacter(from: .whitespacesAndNewlines) == nil,
              draft != dismissedDraft else { return [] }
        let query = String(draft.dropFirst())
        return coordinator.commands
            .filter { query.isEmpty || $0.name.lowercased().hasPrefix(query.lowercased()) }
            .prefix(8)
            .map { $0 }
    }

    private var slashCompletionList: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(slashMatches.enumerated()), id: \.element.id) { index, command in
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
                        Text(verbatim: command.argumentHint ?? command.source)
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                            .lineLimit(1)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(index == completionIndex ? Color.accentColor.opacity(0.18) : .clear, in: RoundedRectangle(cornerRadius: 4))
                .accessibilityLabel(Text(verbatim: "/\(command.name) \(command.description ?? "")"))
                .accessibilityAddTraits(index == completionIndex ? .isSelected : [])
            }
        }
        .background(Color.secondary.opacity(0.10), in: RoundedRectangle(cornerRadius: 6))
        .accessibilityIdentifier("pitex.assistant.slashCommands")
    }

    /// A list the user closed with Escape stays closed until the draft changes.
    @State private var dismissedDraft: String?

    /// `@` nearest the end of the draft.
    // shortcut: SwiftUI's TextField hides the caret, so the mention must end the draft; an NSTextView wrapper would allow mid-text mentions.
    private var activeMention: AgentFileMention? {
        guard draft != dismissedDraft else { return nil }
        return AgentFileMention.active(in: draft, cursor: draft.count)
    }

    private var mentionMatches: [String] {
        guard let mention = activeMention else { return [] }
        return AgentFileMention.matches(coordinator.mentionFiles(), query: mention.query)
    }

    private var mentionCompletionList: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(mentionMatches.enumerated()), id: \.element) { index, path in
                Button {
                    insertMention(path)
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "doc.text")
                            .foregroundStyle(.secondary)
                            .accessibilityHidden(true)
                        Text(verbatim: (path as NSString).lastPathComponent)
                            .font(.callout)
                        Text(verbatim: path)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Spacer(minLength: 4)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(index == completionIndex ? Color.accentColor.opacity(0.18) : .clear, in: RoundedRectangle(cornerRadius: 4))
                .accessibilityLabel(Text(verbatim: path))
                .accessibilityAddTraits(index == completionIndex ? .isSelected : [])
            }
        }
        .background(Color.secondary.opacity(0.10), in: RoundedRectangle(cornerRadius: 6))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text("assistant.mention.label"))
        .accessibilityIdentifier("pitex.assistant.completions")
    }

    private var completionCount: Int {
        slashMatches.isEmpty ? mentionMatches.count : slashMatches.count
    }

    private func moveCompletion(_ step: Int) -> KeyPress.Result {
        let count = completionCount
        guard count > 0 else { return .ignored }
        completionIndex = min(max(completionIndex + step, 0), count - 1)
        return .handled
    }

    private func applyCompletion() -> KeyPress.Result {
        let commands = slashMatches
        if !commands.isEmpty {
            draft = "/\(commands[min(completionIndex, commands.count - 1)].name) "
            return .handled
        }
        let paths = mentionMatches
        guard !paths.isEmpty else { return .ignored }
        insertMention(paths[min(completionIndex, paths.count - 1)])
        return .handled
    }

    private func dismissCompletion() {
        dismissedDraft = draft
    }

    private func insertMention(_ path: String) {
        guard let mention = activeMention else { return }
        draft = mention.apply(to: draft, path: path).text
        composerFocused = true
    }

    /// Queued messages: text, then Edit, Send now (when the backend can steer) and
    /// Delete; more than three fold behind "Show all N queued".
    private var queueList: some View {
        let items = coordinator.queue.items
        let canSendNow = coordinator.canSendQueuedNow
        return VStack(alignment: .leading, spacing: 2) {
            HStack {
                Text(String(format: String(localized: "assistant.queue.title"), String(items.count)))
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Button("assistant.queue.clear") { coordinator.clearQueue() }
                    .buttonStyle(.borderless)
                    .font(.caption)
                    .accessibilityIdentifier("pitex.assistant.queue.clear")
            }
            ForEach(showAllQueued ? items : Array(items.prefix(3))) { item in
                HStack(spacing: 6) {
                    Text(verbatim: item.text.split(separator: "\n", omittingEmptySubsequences: false).first.map(String.init) ?? "")
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .help(item.text)
                    if !item.images.isEmpty {
                        Text(String(format: String(localized: "assistant.queue.images"), String(item.images.count)))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    Spacer(minLength: 4)
                    if item.state == .sending {
                        Text("assistant.queue.sending")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    queueButton("pencil", "assistant.queue.edit", item) {
                        if let taken = coordinator.takeQueued(item.id) {
                            let rest = draft.trimmingCharacters(in: .whitespacesAndNewlines)
                            draft = rest.isEmpty ? taken.text : taken.text + "\n" + draft
                            composerFocused = true
                        }
                    }
                    if canSendNow {
                        queueButton("arrow.up.circle", "assistant.queue.send_now", item) { coordinator.sendQueuedNow(item.id) }
                    }
                    queueButton("trash", "assistant.queue.delete", item) { coordinator.deleteQueued(item.id) }
                }
                .font(.callout)
            }
            if items.count > 3 {
                Button {
                    showAllQueued.toggle()
                } label: {
                    Text(showAllQueued
                         ? String(localized: "assistant.queue.show_less")
                         : String(format: String(localized: "assistant.queue.show_all"), String(items.count)))
                }
                .buttonStyle(.borderless)
                .font(.caption)
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 5)
        .background(Color.secondary.opacity(0.10), in: RoundedRectangle(cornerRadius: 6))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text("assistant.queue.label"))
        .accessibilityIdentifier("pitex.assistant.queue")
    }

    private func queueButton(_ symbol: String, _ key: String, _ item: AgentMessageQueue.Item, action: @escaping () -> Void) -> some View {
        let title = String(localized: String.LocalizationValue(key))
        return Button(action: action) {
            Image(systemName: symbol)
        }
        .buttonStyle(.borderless)
        .disabled(item.state == .sending)
        .help(title)
        .accessibilityLabel(Text(verbatim: "\(title) — \(item.text)"))
    }

    /// The draft's images: thumbnail, name and size, and a remove button.
    private var imageChips: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(coordinator.draftImages) { image in
                    let size = AgentComposer.byteSize(image.data.count)
                    HStack(spacing: 4) {
                        if let thumbnail = NSImage(data: image.data) {
                            Image(nsImage: thumbnail)
                                .resizable()
                                .aspectRatio(contentMode: .fill)
                                .frame(width: 28, height: 28)
                                .clipShape(RoundedRectangle(cornerRadius: 4))
                                .accessibilityHidden(true)
                        }
                        Text(verbatim: "\(image.name) · \(size)")
                            .font(.caption)
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Button {
                            coordinator.draftImages.removeAll { $0.id == image.id }
                        } label: {
                            Image(systemName: "xmark").imageScale(.small)
                        }
                        .buttonStyle(.plain)
                        .foregroundStyle(.secondary)
                        .help(String(localized: "assistant.image.remove"))
                        .accessibilityLabel(Text(verbatim: "\(String(localized: "assistant.image.remove")) — \(image.name)"))
                    }
                    .padding(.horizontal, 4)
                    .padding(.vertical, 2)
                    .background(Color.secondary.opacity(0.14), in: RoundedRectangle(cornerRadius: 6))
                    .accessibilityElement(children: .contain)
                    .accessibilityLabel(Text(verbatim: "\(image.name), \(size)"))
                }
            }
        }
        .accessibilityLabel(Text("assistant.image.label"))
        .accessibilityIdentifier("pitex.assistant.images")
    }

    private func installPasteMonitor() {
        guard pasteMonitor == nil else { return }
        let coordinator = coordinator
        pasteMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            guard event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command,
                  event.charactersIgnoringModifiers == "v" else { return event }
            // The monitor's closure is not main-actor isolated; local monitors run on the main thread.
            let consumed = MainActor.assumeIsolated { () -> Bool in
                let pasteboard = NSPasteboard.general
                // Text (and a copied file's name) pastes as usual.
                guard pasteboard.string(forType: .string) == nil,
                      let image = NSImage(pasteboard: pasteboard),
                      let tiff = image.tiffRepresentation,
                      let png = NSBitmapImageRep(data: tiff)?.representation(using: .png, properties: [:]) else { return false }
                coordinator.attachImage(name: "pasted.png", mimeType: "image/png", data: png)
                return true
            }
            return consumed ? nil : event
        }
    }

    private func removePasteMonitor() {
        if let pasteMonitor { NSEvent.removeMonitor(pasteMonitor) }
        pasteMonitor = nil
    }

    /// Dropped image files become chips; other files go in as paths.
    private func dropItems(_ providers: [NSItemProvider]) -> Bool {
        var handled = false
        for provider in providers {
            let coordinator = coordinator
            if provider.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) {
                handled = true
                _ = provider.loadObject(ofClass: URL.self) { url, _ in
                    guard let url else { return }
                    Task { @MainActor in coordinator.attach(urls: [url]) }
                }
            } else if provider.hasItemConformingToTypeIdentifier(UTType.image.identifier) {
                handled = true
                provider.loadDataRepresentation(forTypeIdentifier: UTType.image.identifier) { data, _ in
                    guard let data, let rep = NSBitmapImageRep(data: data),
                          let png = rep.representation(using: .png, properties: [:]) else { return }
                    Task { @MainActor in coordinator.attachImage(name: "dropped.png", mimeType: "image/png", data: png) }
                }
            }
        }
        return handled
    }

    /// `5h 12% · 7d 34%` for OAuth subscription providers (Claude, Codex). The context
    /// meter in the composer row shows the window, the tokens and the cost.
    private var usageText: String? {
        guard let windows = subscriptionUsage.windows, !windows.isEmpty else { return nil }
        return windows.map { "\($0.label) \(Int($0.percent.rounded()))%" }.joined(separator: " · ")
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
            && (!draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !coordinator.draftImages.isEmpty)
    }

    private func sendDraft(now: Bool = false) {
        guard canSend else { return }
        coordinator.send(prompt: draft, now: now)
        draft = ""
        // Sending follows the transcript again.
        scrollPin.jump()
    }

    /// Ring and "38% · 76k/200k"; amber from 70 %, red above 90 %. A click shows the
    /// tokens and the session cost.
    private func contextMeter(_ meter: AgentContextMeter) -> some View {
        let tint: Color = switch meter.band {
        case .normal: .secondary
        case .warning: .orange
        case .danger: .red
        }
        let spoken = String(format: String(localized: "assistant.context.a11y"), "\(meter.roundedPercent)")
        return Button {
            showingContextDetails.toggle()
        } label: {
            HStack(spacing: 4) {
                ZStack {
                    Circle().stroke(Color.secondary.opacity(0.25), lineWidth: 2.5)
                    Circle()
                        .trim(from: 0, to: meter.percent / 100)
                        .stroke(tint, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                }
                .frame(width: 14, height: 14)
                Text(verbatim: meter.label)
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(tint)
                    .lineLimit(1)
            }
        }
        .buttonStyle(.plain)
        .padding(.bottom, 4)
        .help(spoken)
        .accessibilityLabel(Text(verbatim: spoken))
        .accessibilityIdentifier("pitex.assistant.contextMeter")
        .accessibilityAddTraits(showingContextDetails ? .isSelected : [])
    }

    /// Inline, not a popover: a popover open while the turn streams crashed AppKit's layout.
    private func contextDetails(_ meter: AgentContextMeter) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text("assistant.context.title").font(.caption.weight(.semibold))
            ForEach(contextDetailLines(meter), id: \.self) { line in
                Text(verbatim: line).font(.caption.monospacedDigit())
            }
        }
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.secondary.opacity(0.10), in: RoundedRectangle(cornerRadius: 6))
        .textSelection(.enabled)
        .accessibilityIdentifier("pitex.assistant.contextDetails")
    }

    /// The details: context, the token counts the backend reported, and the cost.
    private func contextDetailLines(_ meter: AgentContextMeter) -> [String] {
        let usage = coordinator.usage
        let compact = AgentContextMeter.compactTokens
        var lines = [String(
            format: String(localized: "assistant.context.used"),
            compact(meter.used), compact(meter.max), "\(meter.roundedPercent)"
        )]
        let counts: [(String, Int?)] = [
            ("assistant.context.input", usage.inputTokens),
            ("assistant.context.cached", usage.cachedInputTokens),
            ("assistant.context.output", usage.outputTokens),
            ("assistant.context.thinking", usage.thinkingTokens),
        ]
        for (key, value) in counts {
            if let value { lines.append(String(format: String(localized: String.LocalizationValue(key)), compact(value))) }
        }
        // pi pushes nothing: its pulled totals stand in.
        if usage.inputTokens == nil, let total = coordinator.sessionStats?.totalTokens, total > 0 {
            lines.append(String(format: String(localized: "assistant.context.total"), compact(total)))
        }
        if let cost = usage.costUsd ?? coordinator.sessionStats?.cost, cost > 0 {
            let text = cost < 0.01 ? String(format: "$%.4f", cost) : String(format: "$%.2f", cost)
            lines.append(String(format: String(localized: "assistant.context.cost"), text))
        }
        return lines
    }

    private func attachFiles() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = true
        guard panel.runModal() == .OK else { return }
        coordinator.attach(urls: panel.urls)
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

    /// Offset and distance from the bottom; only an offset change is a scroll.
    private struct ScrollSpot: Equatable {
        var offset: Double
        var distanceFromBottom: Double
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
                            openLink: { coordinator.openFile($0, $1) },
                            retry: {
                                scrollPin.jump()
                                coordinator.retry()
                            }
                        )
                        .id(entry.id)
                    }
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
            }
            .onScrollGeometryChange(for: ScrollSpot.self) { geometry in
                ScrollSpot(
                    offset: geometry.contentOffset.y,
                    distanceFromBottom: geometry.contentSize.height - geometry.contentOffset.y - geometry.containerSize.height
                )
            } action: { old, new in
                // Content growth moves only the distance; the user (or scrollTo) moves the offset.
                if old.offset != new.offset { scrollPin.scrolled(distanceFromBottom: new.distanceFromBottom) }
            }
            .onChange(of: coordinator.transcript.count) { _, _ in followLatest(proxy) }
            .onChange(of: coordinator.transcript.last?.text) { _, _ in followLatest(proxy) }
            .overlay(alignment: .bottom) {
                if scrollPin.unseen {
                    Button {
                        scrollPin.jump()
                        if let last = coordinator.transcript.last { proxy.scrollTo(last.id, anchor: .bottom) }
                    } label: {
                        Label("assistant.jump_latest", systemImage: "arrow.down")
                            .font(.caption.weight(.medium))
                            .padding(.horizontal, 10)
                            .padding(.vertical, 5)
                            .background(.regularMaterial, in: Capsule())
                    }
                    .buttonStyle(.plain)
                    .padding(.bottom, 8)
                    .accessibilityIdentifier("pitex.assistant.jumpLatest")
                }
            }
        }
        .accessibilityIdentifier("pitex.assistant.transcript")
    }

    private func followLatest(_ proxy: ScrollViewProxy) {
        guard scrollPin.contentAdded(), let last = coordinator.transcript.last else { return }
        proxy.scrollTo(last.id, anchor: .bottom)
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
    var retry: () -> Void = {}
    /// The user opened or closed this thinking block; nil follows the streaming state.
    @State private var thinkingChoice: Bool?
    private var captionFont: Font { .system(size: max(fontSize - 2, 9)) }

    var body: some View {
        switch entry.role {
        case .user:
            HStack {
                Spacer(minLength: 32)
                VStack(alignment: .trailing, spacing: 6) {
                    Text(verbatim: entry.text)
                        .font(.system(size: fontSize))
                        .textSelection(.enabled)
                    if !entry.images.isEmpty {
                        HStack(spacing: 6) {
                            ForEach(entry.images) { image in
                                if let thumbnail = NSImage(data: image.data) {
                                    Image(nsImage: thumbnail)
                                        .resizable()
                                        .aspectRatio(contentMode: .fit)
                                        .frame(maxWidth: 96, maxHeight: 96)
                                        .clipShape(RoundedRectangle(cornerRadius: 6))
                                        .accessibilityLabel(Text(verbatim: image.name))
                                }
                            }
                        }
                    }
                }
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
            // Open with a spinner while it streams, then closed as "Thought for Ns".
            DisclosureGroup(isExpanded: Binding(
                get: { AgentThinking.isExpanded(streaming: entry.status == .streaming, userChoice: thinkingChoice) },
                set: { thinkingChoice = $0 }
            )) {
                Text(verbatim: entry.text)
                    .font(captionFont)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            } label: {
                HStack(spacing: 5) {
                    if entry.status != .streaming, let seconds = entry.thinkingSeconds {
                        // Under a second reads "<1s", not "0s".
                        Text(verbatim: String(format: String(localized: "assistant.thinking.done"), seconds > 0 ? "\(seconds)" : "<1"))
                            .font(captionFont.weight(.semibold))
                            .foregroundStyle(.secondary)
                    } else {
                        // A row reloaded from history has no duration but is finished.
                        Text(LocalizedStringKey(entry.status == .streaming ? "assistant.thinking.title" : "assistant.thinking.finished"))
                            .font(captionFont.weight(.semibold))
                            .foregroundStyle(.secondary)
                    }
                    if entry.status == .streaming {
                        ProgressView().controlSize(.mini)
                    }
                }
            }
            .accessibilityIdentifier("pitex.assistant.thinking")
        case .tool:
            AgentToolCard(entry: entry, fontSize: fontSize, projectRoot: projectRoot, openFile: openFile)
        case .notice:
            if let failure = entry.turnError {
                turnErrorRow(failure)
            } else {
                Text(verbatim: entry.text)
                    .font(captionFont)
                    .foregroundStyle(entry.status == .failed ? Color.red : Color.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
            }
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

    /// A failed turn: a short line for its code, the raw error under Details, and Retry.
    private func turnErrorRow(_ failure: AgentTurnError) -> some View {
        let line = String(
            format: String(localized: String.LocalizationValue(failure.code.messageKey(canCompact: failure.canCompact))),
            AgentErrorCode.cliName(provider: failure.provider)
        )
        return VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.red)
                    .accessibilityHidden(true)
                Text(verbatim: line)
                    .font(.system(size: fontSize))
                    .fixedSize(horizontal: false, vertical: true)
            }
            if !entry.text.isEmpty, entry.text != line {
                DisclosureGroup("assistant.error.details") {
                    Text(verbatim: entry.text)
                        .font(.system(size: max(fontSize - 2, 9), design: .monospaced))
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .font(captionFont)
            }
            if failure.retry {
                Button("assistant.error.retry", action: retry)
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
                    .accessibilityIdentifier("pitex.assistant.retry")
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.red.opacity(0.06), in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(Color.red.opacity(0.45)))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("pitex.assistant.turnError")
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

/// The resumable conversations of the current project, newest first, with a
/// title search. Arrows move the highlight from the search field, Return
/// opens it. Each row shows its age and owning backend; another backend's
/// rows say where they open. `List` is lazy, so 500 rows stay fast.
private struct AgentSessionHistoryList: View {
    @ObservedObject var coordinator: AgentCoordinator
    @ObservedObject var settings: SettingsStore
    @State private var query = ""
    @State private var selection: URL?
    @FocusState private var searchFocused: Bool

    private var shown: [AgentSessionSummary] {
        coordinator.pastSessions.filter { AgentSessionHistory.matches(title: $0.title, query: query) }
    }

    var body: some View {
        let shown = shown
        let now = Date()
        VStack(spacing: 6) {
            TextField(String(localized: "assistant.history_search"), text: $query)
                .textFieldStyle(.roundedBorder)
                .focused($searchFocused)
                .onSubmit { _ = openSelection(in: shown) }
                .onKeyPress(.downArrow) { move(1, in: shown) }
                .onKeyPress(.upArrow) { move(-1, in: shown) }
                .accessibilityIdentifier("pitex.assistant.historySearch")
            if shown.isEmpty {
                Text(LocalizedStringKey(coordinator.pastSessions.isEmpty ? "assistant.history_empty" : "assistant.history_no_match"))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollViewReader { proxy in
                    List(shown, selection: $selection) { session in
                        Button {
                            coordinator.open(session, settings: settings)
                        } label: {
                            AgentSessionHistoryRow(
                                session: session,
                                foreign: session.source != coordinator.activeBackend,
                                now: now
                            )
                        }
                        .buttonStyle(.plain)
                        .tag(session.url)
                    }
                    .onKeyPress(.return) { openSelection(in: shown) }
                    .onChange(of: selection) { _, url in
                        if let url { proxy.scrollTo(url) }
                    }
                    .accessibilityLabel(Text("assistant.history"))
                    .accessibilityIdentifier("pitex.assistant.historyList")
                }
            }
        }
        .padding(8)
        .frame(width: 400, height: 360)
        .onAppear {
            searchFocused = true
            selection = shown.first?.url
        }
        .onChange(of: query) { selection = self.shown.first?.url }
    }

    private func move(_ step: Int, in shown: [AgentSessionSummary]) -> KeyPress.Result {
        guard !shown.isEmpty else { return .ignored }
        let current = shown.firstIndex { $0.url == selection } ?? (step > 0 ? -1 : shown.count)
        selection = shown[min(max(current + step, 0), shown.count - 1)].url
        return .handled
    }

    private func openSelection(in shown: [AgentSessionSummary]) -> KeyPress.Result {
        guard let session = shown.first(where: { $0.url == selection }) ?? shown.first else { return .ignored }
        coordinator.open(session, settings: settings)
        return .handled
    }
}

/// One history row: the title on one line; age and backend badge below.
private struct AgentSessionHistoryRow: View {
    let session: AgentSessionSummary
    let foreign: Bool
    let now: Date

    var body: some View {
        let label = AgentSessionHistory.sourceLabel(session.source)
        let opens = String(format: String(localized: "assistant.history_other_backend"), label)
        VStack(alignment: .leading, spacing: 2) {
            Text(verbatim: session.title)
                .lineLimit(1)
                .truncationMode(.tail)
            HStack(spacing: 6) {
                Text(verbatim: age)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(verbatim: label)
                    .font(.caption)
                    .padding(.horizontal, 6)
                    .background(Capsule().fill(foreign ? Color.accentColor.opacity(0.18) : Color.secondary.opacity(0.12)))
                if foreign {
                    // Another backend's chat: picking it switches the backend first.
                    Text(verbatim: opens)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .help(foreign ? opens : "")
        .accessibilityElement(children: .combine)
    }

    private var age: String {
        switch AgentSessionHistory.age(of: session.date, now: now) {
        case .justNow: String(localized: "assistant.time_now")
        case .minutes(let n): String(format: String(localized: "assistant.time_minutes"), n)
        case .hours(let n): String(format: String(localized: "assistant.time_hours"), n)
        case .yesterday: String(localized: "assistant.time_yesterday")
        case .days(let n): String(format: String(localized: "assistant.time_days"), n)
        case .date: session.date.formatted(date: .numeric, time: .omitted)
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
