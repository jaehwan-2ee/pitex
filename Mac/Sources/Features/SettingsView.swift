import AppKit
import CoreServices
import SettingsFeature
import SwiftUI
import UniformTypeIdentifiers

/// Persists validated settings to UserDefaults. Secret material never lands
/// here — only the preference schema owned by SettingsFeature.
@MainActor
final class SettingsStore: ObservableObject {
    /// One store for every window, so a change made in one window's
    /// Settings reaches the others immediately.
    static let shared = SettingsStore()

    @Published private(set) var settings: PersistedSettings {
        didSet { persist() }
    }
    @Published var customShellExecutable: String {
        didSet { UserDefaults.standard.set(customShellExecutable, forKey: "customShellExecutable") }
    }
    @Published var switchToPDFOnBuild: Bool {
        didSet { UserDefaults.standard.set(switchToPDFOnBuild, forKey: "pitex.pref.build.switchToPDFOnBuild") }
    }
    @Published var jumpToCursorAfterBuild: Bool {
        didSet { UserDefaults.standard.set(jumpToCursorAfterBuild, forKey: "pitex.pref.build.jumpToCursorAfterBuild") }
    }
    @Published var restoreSession: Bool {
        didSet { UserDefaults.standard.set(restoreSession, forKey: "pitex.pref.editor.restoreSession") }
    }
    @Published var codeFolding: Bool {
        didSet { UserDefaults.standard.set(codeFolding, forKey: "pitex.pref.editor.codeFolding") }
    }
    @Published var minimap: Bool {
        didSet { UserDefaults.standard.set(minimap, forKey: "pitex.pref.editor.minimap") }
    }
    @Published var inverseSyncHighlight: Bool {
        didSet { UserDefaults.standard.set(inverseSyncHighlight, forKey: "pitex.pref.synctex.inverseHighlight") }
    }
    @Published var forwardSyncHighlight: Bool {
        didSet { UserDefaults.standard.set(forwardSyncHighlight, forKey: "pitex.pref.synctex.forwardHighlight") }
    }
    @Published var autoSave: Bool {
        didSet { UserDefaults.standard.set(autoSave, forKey: "pitex.pref.editor.autoSave") }
    }
    @Published var autoSaveDelay: Int {
        didSet { UserDefaults.standard.set(autoSaveDelay, forKey: "pitex.pref.editor.autoSaveDelay") }
    }
    @Published var confirmOverwrite: Bool {
        didSet { UserDefaults.standard.set(confirmOverwrite, forKey: "pitex.pref.editor.confirmOverwrite") }
    }
    @Published var chatHistoryLimit: Int {
        didSet { UserDefaults.standard.set(chatHistoryLimit, forKey: "ai.chatHistoryLimit") }
    }
    @Published var aiDefaultModel: String {
        didSet { UserDefaults.standard.set(aiDefaultModel, forKey: "ai.defaultModel") }
    }
    @Published var aiAttachDefault: Bool {
        didSet { UserDefaults.standard.set(aiAttachDefault, forKey: "ai.attachDefault") }
    }
    /// `ai.autocompletion` — the Copilot-style inline ghost-text toggle,
    /// default OFF: it spawns a pi subprocess, so opt-in (same key on
    /// Linux/Windows).
    @Published var aiAutocompletion: Bool {
        didSet { UserDefaults.standard.set(aiAutocompletion, forKey: "ai.autocompletion") }
    }
    @Published var aiFontSize: Double {
        didSet { UserDefaults.standard.set(aiFontSize, forKey: "ai.fontSize") }
    }
    @Published var defaultBuildCommand: String {
        didSet { UserDefaults.standard.set(defaultBuildCommand, forKey: "project.defaultBuildCommand") }
    }
    @Published var defaultCustomCommand: String {
        didSet { UserDefaults.standard.set(defaultCustomCommand, forKey: "project.defaultCustomCommand") }
    }
    /// `pitex.pref.update.autoInstall` — check GitHub Releases on launch and
    /// install a newer build without asking (same key on Linux/Windows).
    @Published var autoInstallUpdates: Bool {
        didSet { UserDefaults.standard.set(autoInstallUpdates, forKey: "pitex.pref.update.autoInstall") }
    }
    /// Render the Markdown preview on every edit — off keeps the last render.
    @Published var markdownLivePreview: Bool {
        didSet { UserDefaults.standard.set(markdownLivePreview, forKey: "pitex.pref.markdown.livePreview") }
    }
    /// Keep the editor and the Markdown preview scrolled to the same place.
    @Published var markdownSyncScroll: Bool {
        didSet { UserDefaults.standard.set(markdownSyncScroll, forKey: "pitex.pref.markdown.syncScroll") }
    }
    /// `system` | `light` | `dark` — unknown values resolve to `system`.
    @Published var markdownTheme: String {
        didSet { UserDefaults.standard.set(markdownTheme, forKey: "pitex.pref.markdown.theme") }
    }
    /// Preview root font size, clamped 10…28.
    @Published var markdownFontSize: Double {
        didSet { UserDefaults.standard.set(markdownFontSize, forKey: "pitex.pref.markdown.fontSize") }
    }
    private static let settingsKey = "dev.pitex.settings"

    init() {
        settings = .safeDefaults
        customShellExecutable = "/bin/zsh"
        switchToPDFOnBuild = true
        jumpToCursorAfterBuild = true
        restoreSession = true
        codeFolding = true
        minimap = true
        inverseSyncHighlight = false
        forwardSyncHighlight = false
        autoSave = false
        autoSaveDelay = 5
        confirmOverwrite = true
        chatHistoryLimit = 50
        aiDefaultModel = ""
        aiAttachDefault = true
        aiAutocompletion = false
        aiFontSize = 13
        defaultBuildCommand = "xelatex -interaction=nonstopmode -synctex=1 {file}"
        defaultCustomCommand = ""
        autoInstallUpdates = false
        markdownLivePreview = true
        markdownSyncScroll = true
        markdownTheme = "system"
        markdownFontSize = 16
        reload()
    }

    /// Re-read every key from UserDefaults — used after a settings import
    /// wrote new values behind the store's back.
    func reload() {
        if let data = UserDefaults.standard.data(forKey: Self.settingsKey),
           let decoded = try? JSONDecoder().decode(PersistedSettings.self, from: data) {
            settings = decoded
        }
        let defaults = UserDefaults.standard
        customShellExecutable = defaults.string(forKey: "customShellExecutable") ?? "/bin/zsh"
        switchToPDFOnBuild = defaults.object(forKey: "pitex.pref.build.switchToPDFOnBuild") as? Bool ?? true
        jumpToCursorAfterBuild = defaults.object(forKey: "pitex.pref.build.jumpToCursorAfterBuild") as? Bool ?? true
        restoreSession = defaults.object(forKey: "pitex.pref.editor.restoreSession") as? Bool ?? true
        codeFolding = defaults.object(forKey: "pitex.pref.editor.codeFolding") as? Bool ?? true
        minimap = defaults.object(forKey: "pitex.pref.editor.minimap") as? Bool ?? true
        inverseSyncHighlight = defaults.bool(forKey: "pitex.pref.synctex.inverseHighlight")
        forwardSyncHighlight = defaults.bool(forKey: "pitex.pref.synctex.forwardHighlight")
        autoSave = defaults.object(forKey: "pitex.pref.editor.autoSave") as? Bool ?? false
        autoSaveDelay = defaults.object(forKey: "pitex.pref.editor.autoSaveDelay") as? Int ?? 5
        confirmOverwrite = defaults.object(forKey: "pitex.pref.editor.confirmOverwrite") as? Bool ?? true
        chatHistoryLimit = defaults.object(forKey: "ai.chatHistoryLimit") as? Int ?? 50
        aiDefaultModel = defaults.string(forKey: "ai.defaultModel") ?? ""
        aiAttachDefault = defaults.object(forKey: "ai.attachDefault") as? Bool ?? true
        aiAutocompletion = defaults.object(forKey: "ai.autocompletion") as? Bool ?? false
        aiFontSize = min(max(defaults.object(forKey: "ai.fontSize") as? Double ?? 13, 10), 24)
        defaultBuildCommand = defaults.string(forKey: "project.defaultBuildCommand") ?? "xelatex -interaction=nonstopmode -synctex=1 {file}"
        defaultCustomCommand = defaults.string(forKey: "project.defaultCustomCommand") ?? ""
        autoInstallUpdates = defaults.bool(forKey: "pitex.pref.update.autoInstall")
        markdownLivePreview = defaults.object(forKey: "pitex.pref.markdown.livePreview") as? Bool ?? true
        markdownSyncScroll = defaults.object(forKey: "pitex.pref.markdown.syncScroll") as? Bool ?? true
        markdownTheme = Self.markdownThemeValue(defaults.string(forKey: "pitex.pref.markdown.theme"))
        markdownFontSize = min(max(defaults.object(forKey: "pitex.pref.markdown.fontSize") as? Double ?? 16, 10), 28)
    }

    /// Missing or unknown `pitex.pref.markdown.theme` resolves to `system`.
    static func markdownThemeValue(_ stored: String?) -> String {
        switch stored {
        case "light", "dark": stored!
        default: "system"
        }
    }

    func update(_ transform: (PersistedSettings) throws -> PersistedSettings) rethrows {
        settings = try transform(settings)
    }

    func updateBuild(_ transform: (PersistedSettings) throws -> BuildPreferences) rethrows {
        let current = settings
        settings = PersistedSettings(
            editor: current.editor,
            build: try transform(current),
            pdf: current.pdf,
            diagnosticsConsent: current.diagnosticsConsent,
            project: current.project
        )
    }

    func updateEditor(_ transform: (PersistedSettings) throws -> EditorPreferences) rethrows {
        let current = settings
        settings = PersistedSettings(
            editor: try transform(current),
            build: current.build,
            pdf: current.pdf,
            diagnosticsConsent: current.diagnosticsConsent,
            project: current.project
        )
    }

    private func persist() {
        if let data = try? JSONEncoder().encode(settings) {
            UserDefaults.standard.set(data, forKey: Self.settingsKey)
        }
    }
}

/// Command presets matching the reference editor's Compile preferences —
/// each writes into the build-command field as a runnable shell line.
/// A computed property keeps the non-Sendable LocalizedStringKey out of
/// shared global state (Swift 6 concurrency).
private var buildCommandPresets: [(label: LocalizedStringKey, command: String)] {
    [
        ("preset.pdflatex", "pdflatex -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.xelatex", "xelatex -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.lualatex", "lualatex -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.latexmk", "latexmk -pdf -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.tectonic", "tectonic --synctex {file}"),
    ]
}

/// The four settings panes, mirroring the reference editor's capsule tab
/// strip (icon + label segments in one rounded capsule, centred with
/// comfortable spacing above the content).
private enum SettingsTab: String, CaseIterable, Identifiable {
    case compile
    case markdown
    case editor
    case appearance
    case ai
    case general

    var id: Self { self }

    var titleKey: LocalizedStringKey {
        switch self {
        case .compile: "settings.tab.compile"
        case .markdown: "settings.tab.markdown"
        case .editor: "settings.tab.editor"
        case .appearance: "settings.tab.appearance"
        case .ai: "settings.tab.ai"
        case .general: "settings.tab.general"
        }
    }

    var systemImage: String {
        switch self {
        case .compile: "hammer"
        case .markdown: "doc.richtext"
        case .editor: "textformat"
        case .appearance: "paintpalette"
        case .ai: "wand.and.stars"
        case .general: "gearshape"
        }
    }
}

struct SettingsView: View {
    @ObservedObject var store: SettingsStore
    @ObservedObject var workspace: WorkspaceModel
    @Environment(\.dismiss) private var dismiss
    @StateObject private var appearance = AppearanceSettings.shared
    @StateObject private var updates = UpdateChecker()
    @State private var selectedTab: SettingsTab = .compile
    @State private var piInstallInFlight = false
    @State private var piInstallMessage: String?
    @State private var piInstallIsError = false
    @State private var piSettingsError: String?
    @State private var skills: [PiSkill] = []
    @State private var skillSource = ""
    @State private var skillInstallInFlight = false
    @State private var skillMessage: String?
    @State private var skillIsError = false
    @State private var settingsTransferMessage: String?
    @State private var settingsTransferIsError = false

    var body: some View {
        VStack(spacing: 0) {
            capsuleTabBar
                .padding(.top, 18)
                .padding(.bottom, 10)

            Group {
                switch selectedTab {
                case .compile: compileTab
                case .markdown: markdownTab
                case .editor: editorTab
                case .appearance: appearanceTab
                case .ai: aiTab
                case .general: generalTab
                }
            }

            Divider()
            HStack {
                Spacer()
                Button("settings.done") { dismiss() }
                    .keyboardShortcut(.defaultAction)
            }
            .padding(12)
        }
        // Wide enough for all six tabs on one line in every locale (Russian
        // and Vietnamese titles are the longest).
        .frame(minWidth: 780, minHeight: 560)
    }

    /// Segmented capsule strip matching the reference settings window.
    private var capsuleTabBar: some View {
        HStack(spacing: 2) {
            ForEach(SettingsTab.allCases) { tab in
                Button {
                    selectedTab = tab
                } label: {
                    Label(tab.titleKey, systemImage: tab.systemImage)
                        .font(.callout)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 6)
                        .frame(minWidth: 86)
                        .background(
                            selectedTab == tab
                                ? AnyShapeStyle(.selection)
                                : AnyShapeStyle(.clear),
                            in: Capsule()
                        )
                        .contentShape(Capsule())
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("pitex.settings.tab.\(tab.rawValue)")
            }
        }
        .padding(3)
        .background(.quaternary, in: Capsule())
        // Never squeeze titles onto two lines — the sheet widens instead.
        .fixedSize()
        .frame(maxWidth: .infinity)
    }

    // MARK: - Compile

    private var compileTab: some View {
        Form {
            Section("settings.compile.presets") {
                HStack(spacing: 8) {
                    ForEach(buildCommandPresets, id: \.command) { preset in
                        Button(preset.label) { applyPreset(preset.command) }
                    }
                }
            }
            Section("settings.compile.commands") {
                LabeledContent("settings.compile.build_command") {
                    TextField("xelatex -interaction=nonstopmode -synctex=1 {file}", text: buildCommandTextBinding)
                        .textFieldStyle(.roundedBorder)
                }
                LabeledContent("settings.compile.custom_command") {
                    TextField("make", text: customCommandTextBinding)
                        .textFieldStyle(.roundedBorder)
                }
                Text("settings.compile.commands_note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Section("settings.compile.behavior") {
                Toggle("settings.compile.switch_pdf", isOn: $store.switchToPDFOnBuild)
                Toggle("settings.compile.jump_cursor", isOn: $store.jumpToCursorAfterBuild)
                Toggle("settings.compile.stop_error", isOn: stopOnErrorBinding)
                Stepper(
                    String(format: String(localized: "settings.compile.max_passes"), store.settings.build.maximumPasses),
                    value: passesBinding,
                    in: 1...10
                )
            }
            Section("settings.compile.shell") {
                LabeledContent("settings.compile.shell_path") {
                    TextField("/bin/zsh", text: $store.customShellExecutable)
                        .textFieldStyle(.roundedBorder)
                }
                Toggle("settings.compile.shell_ack", isOn: shellAckBinding)
                TextField("settings.compile.fallback_command", text: customCommandBinding)
                    .textFieldStyle(.roundedBorder)
                    .help("settings.compile.fallback_help")
            }
            Section {
                Button("settings.compile.make_default_editor") {
                    DefaultEditorRegistration.registerAsDefault()
                }
                Text("settings.compile.make_default_note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(8)
    }

    // MARK: - Markdown

    /// Live preview, Sync scrolling, Preview theme, Preview font size —
    /// the same four rows and order as the Linux Markdown page.
    private var markdownTab: some View {
        Form {
            Section {
                Toggle("settings.markdown.live_preview", isOn: $store.markdownLivePreview)
                Text("settings.markdown.live_preview_note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Toggle("settings.markdown.sync_scroll", isOn: $store.markdownSyncScroll)
                Text("settings.markdown.sync_scroll_note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                LabeledContent("settings.markdown.theme") {
                    Picker("", selection: $store.markdownTheme) {
                        Text("settings.markdown.theme.system").tag("system")
                        Text("settings.markdown.theme.light").tag("light")
                        Text("settings.markdown.theme.dark").tag("dark")
                    }
                    .labelsHidden()
                    .fixedSize()
                }
                Stepper(
                    "\(String(localized: "settings.markdown.font_size")): \(Int(store.markdownFontSize))",
                    value: $store.markdownFontSize,
                    in: 10...28
                )
            }
            Section {
                Button("settings.markdown.make_default_editor") {
                    DefaultEditorRegistration.registerAsDefault(markdown: true)
                }
                Text("settings.markdown.make_default_note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(8)
    }

    // MARK: - Editor

    private var editorTab: some View {
        Form {
            Section("settings.editor.autosave") {
                Toggle("settings.editor.autosave", isOn: $store.autoSave)
                LabeledContent("settings.editor.autosave_delay") {
                    Picker("", selection: $store.autoSaveDelay) {
                        Text("settings.editor.delay_2").tag(2)
                        Text("settings.editor.delay_5").tag(5)
                        Text("settings.editor.delay_10").tag(10)
                    }
                    .labelsHidden()
                    .fixedSize()
                    .disabled(!store.autoSave)
                }
            }
            Section("settings.editor.editing") {
                Toggle("settings.editor.confirm_overwrite", isOn: $store.confirmOverwrite)
                Toggle("settings.editor.autocomplete", isOn: completesDelimitersBinding)
                Toggle("settings.editor.folding", isOn: $store.codeFolding)
                Toggle("settings.editor.minimap", isOn: $store.minimap)
            }
            Section("settings.editor.session") {
                Toggle("settings.editor.restore", isOn: $store.restoreSession)
                Stepper(
                    String(format: String(localized: "settings.editor.tab_width"), store.settings.editor.tabWidth),
                    value: tabWidthBinding,
                    in: 1...16
                )
                Toggle("settings.editor.wrap", isOn: wrapBinding)
            }
            Section("SyncTeX") {
                Toggle("settings.synctex.inverse_highlight", isOn: $store.inverseSyncHighlight)
                    .accessibilityIdentifier("pitex.settings.synctex.inverseHighlight")
                Toggle("settings.synctex.forward_highlight", isOn: $store.forwardSyncHighlight)
                    .accessibilityIdentifier("pitex.settings.synctex.forwardHighlight")
                Text("settings.synctex.highlight_note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(8)
    }

    // MARK: - Appearance

    private var appearanceTab: some View {
        Form {
            Section("settings.appearance.theme") {
                LabeledContent("settings.appearance.theme") {
                    Picker("", selection: presetBinding) {
                        ForEach(AppearanceThemePreset.allCases) { preset in
                            Text(preset.title).tag(Optional(preset))
                        }
                    }
                    .labelsHidden()
                    .fixedSize()
                }
                LabeledContent("settings.appearance.language") {
                    Picker("", selection: $appearance.language) {
                        Text("settings.appearance.language_system").tag(AppearanceSettings.AppLanguage.system)
                        Text(verbatim: "English").tag(AppearanceSettings.AppLanguage.en)
                        Text(verbatim: "한국어").tag(AppearanceSettings.AppLanguage.ko)
                        Text(verbatim: "日本語").tag(AppearanceSettings.AppLanguage.ja)
                        Text(verbatim: "Tiếng Việt").tag(AppearanceSettings.AppLanguage.vi)
                        Text(verbatim: "Русский").tag(AppearanceSettings.AppLanguage.ru)
                        Text(verbatim: "简体中文").tag(AppearanceSettings.AppLanguage.zhHans)
                        Text(verbatim: "Español").tag(AppearanceSettings.AppLanguage.es)
                    }
                    .labelsHidden()
                    .fixedSize()
                }
                if appearance.languageRestartPending {
                    Text("settings.appearance.language_restart")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            Section("settings.appearance.font") {
                LabeledContent("settings.appearance.family") {
                    FontFamilyPicker(selection: $appearance.fontFamily)
                }
                LabeledContent("settings.appearance.size") {
                    HStack {
                        Slider(value: $appearance.fontSize, in: 9...24, step: 1)
                        Text("\(Int(appearance.fontSize)) pt")
                            .monospacedDigit()
                            .frame(width: 44, alignment: .trailing)
                    }
                }
                Toggle("settings.appearance.terminal_font_custom", isOn: terminalFontCustomBinding)
                LabeledContent("settings.appearance.terminal_font_family") {
                    FontFamilyPicker(selection: $appearance.terminalFontFamily)
                }
                .disabled(appearance.terminalFontFamily.isEmpty)
                LabeledContent("settings.appearance.terminal_font") {
                    HStack {
                        Slider(value: $appearance.terminalFontSize, in: 9...24, step: 1)
                        Text("\(Int(appearance.terminalFontSize)) pt")
                            .monospacedDigit()
                            .frame(width: 44, alignment: .trailing)
                    }
                }
                .disabled(appearance.terminalFontFamily.isEmpty)
                SyntaxPreview()
            }
            Section("settings.appearance.colors") {
                ForEach(AppearanceColorRole.allCases) { role in
                    LabeledContent(role.titleKey) {
                        HStack {
                            Text(role.detailKey)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Spacer()
                            ColorWellButton(
                                color: appearance.color(for: role),
                                onChange: { appearance.setColor($0, for: role) }
                            )
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
        .padding(8)
    }

    // MARK: - AI

    /// Provider authentication uses pi's own terminal flows; configuration
    /// shortcuts open the app-local settings.json and models.json files.
    private var aiTab: some View {
        Form {
            Section("settings.ai.agent") {
                LabeledContent("settings.ai.location") {
                    Text(PiPaths.runtimeDirectory.path)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                Toggle("settings.ai.attach_default", isOn: $store.aiAttachDefault)
                Toggle("settings.ai.autocompletion", isOn: aiAutocompletionBinding)
                LabeledContent("settings.ai.font_size") {
                    HStack {
                        Slider(value: $store.aiFontSize, in: 10...24, step: 1)
                            .frame(maxWidth: 180)
                            .accessibilityLabel("settings.ai.font_size")
                            .accessibilityIdentifier("pitex.settings.ai.fontSize")
                        Text("\(Int(store.aiFontSize)) pt")
                            .monospacedDigit()
                            .frame(width: 45, alignment: .trailing)
                    }
                }
                Text("settings.ai.font_preview")
                    .font(.system(size: store.aiFontSize))
                    .foregroundStyle(.secondary)
                HStack {
                    Button("settings.ai.setup_providers") {
                        performAgentSettingsAction { try await PiRuntimeInstaller.openAuthenticationInTerminal() }
                    }
                    .disabled(!PiExecutableLocator.appLocalRuntimeInstalled)
                    Button("settings.ai.logout") {
                        performAgentSettingsAction { try await PiRuntimeInstaller.openAuthenticationInTerminal(logout: true) }
                    }
                    .disabled(!PiExecutableLocator.appLocalRuntimeInstalled)
                    .accessibilityIdentifier("pitex.settings.ai.logout")
                    Button("settings.ai.custom_provider") {
                        performAgentSettingsAction { try PiPaths.openConfiguration(customProvider: true) }
                    }
                    .accessibilityIdentifier("pitex.settings.ai.customProvider")
                    Button("settings.ai.update_agent") {
                        installPiRuntime(latest: true)
                    }
                    .disabled(piInstallInFlight || skillInstallInFlight)
                    .accessibilityIdentifier("pitex.settings.ai.updateAgent")
                }
                Button("settings.ai.open_config") {
                    performAgentSettingsAction { try PiPaths.openConfiguration() }
                }
                if let piSettingsError {
                    Text(piSettingsError)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .textSelection(.enabled)
                }
            }
            Section {
                HStack(spacing: 8) {
                    Button(PiExecutableLocator.appLocalRuntimeInstalled
                           ? "settings.ai.reinstall_runtime"
                           : "settings.ai.install_runtime") {
                        installPiRuntime()
                    }
                    .disabled(piInstallInFlight || skillInstallInFlight)
                    if piInstallInFlight {
                        ProgressView()
                            .controlSize(.small)
                    }
                }
                if let piInstallMessage {
                    Text(piInstallMessage)
                        .font(.caption)
                        .foregroundStyle(piInstallIsError ? .red : .secondary)
                        .textSelection(.enabled)
                }
                Text("settings.ai.runtime_note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            skillsSection
        }
        .formStyle(.grouped)
        .padding(8)
        .onAppear { refreshSkills() }
    }

    /// Skills pi discovers under `<agent dir>/skills/<name>/SKILL.md`.
    /// Bundled skills are mirrored from the app bundle on every install and
    /// cannot be removed here; anything else was installed by `pi install`
    /// and gets a remove button plus the install row below.
    private var skillsSection: some View {
        Section("settings.ai.skills") {
            if skills.isEmpty {
                Text("settings.ai.no_skills")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            ForEach(skills) { skill in
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(verbatim: skill.name)
                        if !skill.subtitle.isEmpty {
                            Text(verbatim: skill.subtitle)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .lineLimit(2)
                        }
                    }
                    Spacer()
                    if !skill.bundled {
                        Button("settings.ai.remove") {
                            removeSkill(skill)
                        }
                        .accessibilityIdentifier("pitex.settings.ai.removeSkill.\(skill.name)")
                    }
                }
            }
            HStack(spacing: 8) {
                TextField("settings.ai.install_skill", text: $skillSource)
                    .textFieldStyle(.roundedBorder)
                    .disabled(skillInstallInFlight || piInstallInFlight)
                    .accessibilityIdentifier("pitex.settings.ai.skillSource")
                Button("settings.ai.install") {
                    installSkill()
                }
                .disabled(skillInstallInFlight || piInstallInFlight
                          || skillSource.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                          || !PiExecutableLocator.appLocalRuntimeInstalled)
                .accessibilityIdentifier("pitex.settings.ai.installSkill")
                if skillInstallInFlight {
                    ProgressView()
                        .controlSize(.small)
                }
            }
            if let skillMessage {
                Text(skillMessage)
                    .font(.caption)
                    .foregroundStyle(skillIsError ? .red : .secondary)
                    .textSelection(.enabled)
            }
        }
    }

    private func refreshSkills() {
        skills = Self.loadSkills()
    }

    private func installSkill() {
        let source = skillSource.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !source.isEmpty else { return }
        skillInstallInFlight = true
        skillIsError = false
        skillMessage = String(localized: "settings.ai.skill_installing")
        Task {
            do {
                try await PiRuntimeInstaller.installSkill(source: source)
                skillMessage = String(localized: "settings.ai.skill_installed")
                skillIsError = false
                skillSource = ""
            } catch {
                skillMessage = String(format: String(localized: "settings.ai.skill_failed"), error.localizedDescription)
                skillIsError = true
            }
            skillInstallInFlight = false
            refreshSkills()
        }
    }

    private func removeSkill(_ skill: PiSkill) {
        try? FileManager.default.removeItem(at: skill.directory)
        refreshSkills()
    }

    /// One installed skill directory (`SKILL.md` frontmatter supplies the
    /// display name/description; the directory name is the fallback).
    struct PiSkill: Identifiable {
        var name: String
        var description: String
        var bundled: Bool
        var directory: URL

        var id: String { directory.path }

        var subtitle: String {
            bundled ? String(localized: "settings.ai.builtin") : description
        }
    }

    /// Skill directory names shipped inside the app bundle — the same set
    /// `PiRuntimeInstaller.installBundledSkills` copies (no remove button).
    private static let bundledSkills: Set<String> = [
        "humanizer", "latex-compile", "latex-doctor", "scispace", "texlive-runtime-installer",
    ]

    private static func loadSkills() -> [PiSkill] {
        let fileManager = FileManager.default
        let root = PiPaths.skillsDirectory
        let names = (try? fileManager.contentsOfDirectory(atPath: root.path)) ?? []
        return names.filter { !$0.hasPrefix(".") }.sorted().compactMap { name in
            let directory = root.appendingPathComponent(name, isDirectory: true)
            let manifest = directory.appendingPathComponent("SKILL.md")
            guard fileManager.fileExists(atPath: manifest.path) else { return nil }
            let (skillName, description) = skillFrontmatter(manifest)
            return PiSkill(
                name: skillName ?? name,
                description: description,
                bundled: bundledSkills.contains(name),
                directory: directory
            )
        }
    }

    /// `name:`/`description:` from a `SKILL.md` YAML frontmatter block — a
    /// simple line parse (no YAML dependency): `description: |` block
    /// scalars take their first indented line.
    private static func skillFrontmatter(_ url: URL) -> (name: String?, description: String) {
        guard let text = try? String(contentsOf: url, encoding: .utf8) else { return (nil, "") }
        var name: String?
        var description = ""
        var inside = false
        var blockScalar = false
        for line in text.components(separatedBy: .newlines) {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed == "---" {
                if inside { break }
                inside = true
                continue
            }
            guard inside else { continue }
            if blockScalar {
                if line.hasPrefix(" ") || line.hasPrefix("\t"), !trimmed.isEmpty {
                    description = trimmed
                }
                blockScalar = false
                continue
            }
            if trimmed.hasPrefix("name:") {
                name = trimmed.dropFirst(5).trimmingCharacters(in: .whitespaces)
                    .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
            } else if trimmed.hasPrefix("description:") {
                let value = trimmed.dropFirst(12).trimmingCharacters(in: .whitespaces)
                if value == "|" || value == ">" {
                    blockScalar = true
                } else {
                    description = value.trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
                }
            }
        }
        return (name, description)
    }

    /// General pane — mirrors the GTK settings page: settings backup
    /// (export/import for syncing several Macs/PCs) plus the update
    /// controls: current version, a check button, an install button that
    /// appears when a newer release exists, and the auto-install
    /// preference (same UserDefaults key the Linux and Windows shells read).
    private var generalTab: some View {
        Form {
            Section("settings.general.section") {
                HStack(spacing: 12) {
                    Button("settings.general.export") { exportSettings() }
                        .accessibilityIdentifier("pitex.settings.general.export")
                    Button("settings.general.import") { importSettings() }
                        .accessibilityIdentifier("pitex.settings.general.import")
                }
                Text("settings.general.note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if let settingsTransferMessage {
                    Text(verbatim: settingsTransferMessage)
                        .font(.caption)
                        .foregroundStyle(settingsTransferIsError ? Color.red : Color.secondary)
                        .textSelection(.enabled)
                }
            }
            Section("settings.updates.section") {
                LabeledContent("settings.updates.current_version") {
                    Text(updates.currentVersion)
                        .monospacedDigit()
                }
                HStack {
                    Button("settings.updates.check") {
                        Task { await updates.check() }
                    }
                    .disabled(updates.phase == .checking || updates.phase == .downloading
                              || updates.phase == .installing)
                    .accessibilityIdentifier("pitex.settings.updates.check")
                    if updates.phase == .checking {
                        ProgressView()
                            .controlSize(.small)
                    }
                }
                if let tag = updates.availableTag {
                    Button {
                        Task { await updates.downloadAndInstall() }
                    } label: {
                        Text(String(format: String(localized: "settings.updates.install_version"), tag))
                    }
                    .disabled(updates.phase == .checking || updates.phase == .downloading || updates.phase == .installing)
                    .accessibilityIdentifier("pitex.settings.updates.install")
                }
                if !updates.detail.isEmpty {
                    Text(updates.detail)
                        .font(.caption)
                        .foregroundStyle(updates.phase == .failed ? Color.red : Color.secondary)
                }
                Toggle("settings.updates.auto_install", isOn: $store.autoInstallUpdates)
                    .accessibilityIdentifier("pitex.settings.updates.autoInstall")
            }
        }
        .formStyle(.grouped)
        .padding(8)
    }

    /// "설정보내기" — write every backupable preference to a JSON file.
    private func exportSettings() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "pitex-settings.json"
        panel.allowedContentTypes = [.json]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try SettingsBackup.export().write(to: url)
            settingsTransferMessage = String(localized: "settings.general.exported")
            settingsTransferIsError = false
        } catch {
            settingsTransferMessage = error.localizedDescription
            settingsTransferIsError = true
        }
    }

    /// "설정 불러오기" — merge a backup file into UserDefaults and reload
    /// the live stores so the open window reflects it.
    private func importSettings() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            let applied = try SettingsBackup.importData(Data(contentsOf: url))
            store.reload()
            AppearanceSettings.shared.reload()
            settingsTransferMessage = String(
                format: String(localized: "settings.general.imported"), "\(applied)")
            settingsTransferIsError = false
        } catch {
            settingsTransferMessage = error.localizedDescription
            settingsTransferIsError = true
        }
    }

    private func performAgentSettingsAction(_ action: @escaping @MainActor () async throws -> Void) {
        Task {
            do {
                try await action()
                piSettingsError = nil
            } catch {
                piSettingsError = error.localizedDescription
            }
        }
    }

    private func installPiRuntime(latest: Bool = false) {
        guard !piInstallInFlight else { return }
        if latest && workspace.agent?.isRunning == true {
            piInstallMessage = String(localized: "settings.ai.update_busy")
            piInstallIsError = false
            return
        }
        piInstallInFlight = true
        piInstallMessage = latest ? String(localized: "settings.ai.updating_agent") : nil
        piInstallIsError = false
        if latest { workspace.agent?.shutdown() }
        Task {
            do {
                try await PiRuntimeInstaller.install(latest: latest)
                piInstallMessage = latest
                    ? String(format: String(localized: "settings.ai.agent_updated"), PiRuntimeInstaller.installedVersion() ?? "")
                    : PiPaths.runtimeExecutable.path
                piInstallIsError = false
                refreshSkills()
            } catch {
                piInstallMessage = error.localizedDescription
                piInstallIsError = true
            }
            piInstallInFlight = false
            if latest { workspace.agent?.restart() } else { workspace.agent?.prepare() }
        }
    }

    /// Theme picker binding: shows the preset matching the stored palette, or
    /// "Custom" once any swatch has been changed away from a preset.
    private var presetBinding: Binding<AppearanceThemePreset?> {
        Binding(
            get: { appearance.matchingPreset },
            set: { preset in
                if let preset { appearance.applyPreset(preset) }
            }
        )
    }

    /// Custom-terminal-font switch: on seeds the family from the editor
    /// font, off clears it so the terminal follows the editor again.
    private var terminalFontCustomBinding: Binding<Bool> {
        Binding(
            get: { !appearance.terminalFontFamily.isEmpty },
            set: { on in
                if on {
                    // Seed with the editor font; when the editor uses the
                    // system default (empty family) fall back to the named
                    // system fixed-pitch family — writing "" here would just
                    // read back as "off" and the toggle could never engage.
                    let family = appearance.fontFamily.isEmpty
                        ? (NSFont.userFixedPitchFont(ofSize: appearance.fontSize)?.familyName ?? "Menlo")
                        : appearance.fontFamily
                    appearance.setTerminalFont(family: family, size: appearance.fontSize)
                } else {
                    appearance.terminalFontFamily = ""
                }
            }
        )
    }

    /// Presets write to the open project's command field, falling back to the
    /// default for future projects.
    private func applyPreset(_ command: String) {
        if workspace.hasProject {
            workspace.buildCommandText = command
        } else {
            store.defaultBuildCommand = command
        }
    }

    /// Turning the toggle off hides the visible ghost immediately and stops
    /// further requests (the coordinator re-checks the store at fire time).
    private var aiAutocompletionBinding: Binding<Bool> {
        Binding(
            get: { store.aiAutocompletion },
            set: { newValue in
                store.aiAutocompletion = newValue
                if !newValue { workspace.completion?.dismiss() }
            }
        )
    }

    private var buildCommandTextBinding: Binding<String> {
        Binding(
            get: { workspace.hasProject ? workspace.buildCommandText : store.defaultBuildCommand },
            set: { newValue in
                if workspace.hasProject {
                    workspace.buildCommandText = newValue
                } else {
                    store.defaultBuildCommand = newValue
                }
            }
        )
    }

    private var customCommandTextBinding: Binding<String> {
        Binding(
            get: { workspace.hasProject ? workspace.customCommandText : store.defaultCustomCommand },
            set: { newValue in
                if workspace.hasProject {
                    workspace.customCommandText = newValue
                } else {
                    store.defaultCustomCommand = newValue
                }
            }
        )
    }

    private var completesDelimitersBinding: Binding<Bool> {
        Binding(
            get: { store.settings.editor.completesDelimiters },
            set: { newValue in
                store.updateEditor { try! EditorPreferences(
                    fontSize: $0.editor.fontSize,
                    tabWidth: $0.editor.tabWidth,
                    wrapsLines: $0.editor.wrapsLines,
                    completesDelimiters: newValue
                ) }
            }
        )
    }

    private var engineBinding: Binding<TeXEnginePreference> {
        Binding(
            get: { store.settings.build.engine },
            set: { newValue in
                store.updateBuild { try! BuildPreferences(
                    engine: newValue,
                    maximumPasses: $0.build.maximumPasses,
                    stopsAfterFirstError: $0.build.stopsAfterFirstError,
                    shellExecution: $0.build.shellExecution,
                    customShellAcknowledged: $0.build.customShellAcknowledged
                ) }
            }
        )
    }

    private var passesBinding: Binding<Int> {
        Binding(
            get: { Int(store.settings.build.maximumPasses) },
            set: { newValue in
                store.updateBuild { try! BuildPreferences(
                    engine: $0.build.engine,
                    maximumPasses: UInt8(clamping: newValue),
                    stopsAfterFirstError: $0.build.stopsAfterFirstError,
                    shellExecution: $0.build.shellExecution,
                    customShellAcknowledged: $0.build.customShellAcknowledged
                ) }
            }
        )
    }

    private var stopOnErrorBinding: Binding<Bool> {
        Binding(
            get: { store.settings.build.stopsAfterFirstError },
            set: { newValue in
                store.updateBuild { try! BuildPreferences(
                    engine: $0.build.engine,
                    maximumPasses: $0.build.maximumPasses,
                    stopsAfterFirstError: newValue,
                    shellExecution: $0.build.shellExecution,
                    customShellAcknowledged: $0.build.customShellAcknowledged
                ) }
            }
        )
    }

    private var customCommandBinding: Binding<String> {
        Binding(
            get: {
                if case let .custom(command) = store.settings.build.shellExecution { return command }
                return ""
            },
            set: { newValue in
                store.updateBuild {
                    let execution: ShellExecutionPreference = newValue.isEmpty ? .disabled : .custom(command: newValue)
                    return try! BuildPreferences(
                        engine: $0.build.engine,
                        maximumPasses: $0.build.maximumPasses,
                        stopsAfterFirstError: $0.build.stopsAfterFirstError,
                        shellExecution: execution,
                        customShellAcknowledged: newValue.isEmpty ? false : $0.build.customShellAcknowledged
                    )
                }
            }
        )
    }

    private var shellAckBinding: Binding<Bool> {
        Binding(
            get: { store.settings.build.customShellAcknowledged },
            set: { newValue in
                store.updateBuild {
                    if case .custom = $0.build.shellExecution, newValue {
                        return try! $0.build.acknowledgingCustomShell()
                    }
                    return try! BuildPreferences(
                        engine: $0.build.engine,
                        maximumPasses: $0.build.maximumPasses,
                        stopsAfterFirstError: $0.build.stopsAfterFirstError,
                        shellExecution: $0.build.shellExecution,
                        customShellAcknowledged: false
                    )
                }
            }
        )
    }

    private var fontSizeBinding: Binding<Int> {
        Binding(
            get: { Int(store.settings.editor.fontSize) },
            set: { newValue in
                store.updateEditor { try! EditorPreferences(
                    fontSize: UInt8(clamping: newValue),
                    tabWidth: $0.editor.tabWidth,
                    wrapsLines: $0.editor.wrapsLines,
                    completesDelimiters: $0.editor.completesDelimiters
                ) }
            }
        )
    }

    private var tabWidthBinding: Binding<Int> {
        Binding(
            get: { Int(store.settings.editor.tabWidth) },
            set: { newValue in
                store.updateEditor { try! EditorPreferences(
                    fontSize: $0.editor.fontSize,
                    tabWidth: UInt8(clamping: newValue),
                    wrapsLines: $0.editor.wrapsLines,
                    completesDelimiters: $0.editor.completesDelimiters
                ) }
            }
        )
    }

    private var wrapBinding: Binding<Bool> {
        Binding(
            get: { store.settings.editor.wrapsLines },
            set: { newValue in
                store.updateEditor { try! EditorPreferences(
                    fontSize: $0.editor.fontSize,
                    tabWidth: $0.editor.tabWidth,
                    wrapsLines: newValue,
                    completesDelimiters: $0.editor.completesDelimiters
                ) }
            }
        )
    }

    private var layoutBinding: Binding<PDFPageLayout> {
        Binding(
            get: { store.settings.pdf.pageLayout },
            set: { newValue in
                store.update { _ in PersistedSettings(
                    editor: store.settings.editor,
                    build: store.settings.build,
                    pdf: PDFPreferences(
                        autoReloadAfterBuild: store.settings.pdf.autoReloadAfterBuild,
                        highlightsSyncLocation: store.settings.pdf.highlightsSyncLocation,
                        pageLayout: newValue
                    ),
                    diagnosticsConsent: store.settings.diagnosticsConsent,
                    project: store.settings.project
                ) }
            }
        )
    }

    private var autoReloadBinding: Binding<Bool> {
        Binding(
            get: { store.settings.pdf.autoReloadAfterBuild },
            set: { newValue in
                store.update { _ in PersistedSettings(
                    editor: store.settings.editor,
                    build: store.settings.build,
                    pdf: PDFPreferences(
                        autoReloadAfterBuild: newValue,
                        highlightsSyncLocation: store.settings.pdf.highlightsSyncLocation,
                        pageLayout: store.settings.pdf.pageLayout
                    ),
                    diagnosticsConsent: store.settings.diagnosticsConsent,
                    project: store.settings.project
                ) }
            }
        )
    }

    private var highlightBinding: Binding<Bool> {
        Binding(
            get: { store.settings.pdf.highlightsSyncLocation },
            set: { newValue in
                store.update { _ in PersistedSettings(
                    editor: store.settings.editor,
                    build: store.settings.build,
                    pdf: PDFPreferences(
                        autoReloadAfterBuild: store.settings.pdf.autoReloadAfterBuild,
                        highlightsSyncLocation: newValue,
                        pageLayout: store.settings.pdf.pageLayout
                    ),
                    diagnosticsConsent: store.settings.diagnosticsConsent,
                    project: store.settings.project
                ) }
            }
        )
    }
}

/// Font-family chooser for the Appearance tab: a filterable popup listing the
/// fixed-pitch fonts first, then all installed families.
private struct FontFamilyPicker: View {
    @Binding var selection: String
    @State private var filter = ""

    /// Sorted once per process: the installed family list doesn't change
    /// during a session and sorting per body eval ran on every keystroke of
    /// the filter field.
    private static let sortedFamilies = NSFontManager.shared.availableFontFamilies.sorted()

    private var families: [String] {
        if filter.isEmpty { return Self.sortedFamilies }
        return Self.sortedFamilies.filter { $0.localizedCaseInsensitiveContains(filter) }
    }

    var body: some View {
        Menu {
            TextField("settings.appearance.filter", text: $filter)
                .textFieldStyle(.roundedBorder)
                .padding(.horizontal, 8)
            Divider()
            Button("settings.appearance.system_default") { selection = "" }
            Divider()
            ForEach(families, id: \.self) { family in
                Button(family) { selection = family }
            }
        } label: {
            HStack {
                Text(selection.isEmpty ? String(localized: "settings.appearance.system_default") : selection)
                    .lineLimit(1)
                Image(systemName: "chevron.up.chevron.down")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            .frame(width: 220, alignment: .leading)
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
    }
}

/// Static TeX sample rendered with the live theme colors so the Appearance
/// tab doubles as the reference editor's syntax preview.
private struct SyntaxPreview: View {
    @ObservedObject private var appearance = AppearanceSettings.shared

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            styled("\\documentclass{article}", [.commands, .bodyText])
            styled("% A comment", [.comments])
            styled("\\begin{equation}", [.environments])
            styled("  E = mc^2", [.math])
            styled("\\end{equation}", [.environments])
            styled("Body text with \\textbf{braces}.", [.bodyText, .commands, .braces])
        }
        .font(Font(appearance.editorFont))
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color(appearance.color(for: .editorBackground)))
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .strokeBorder(Color.secondary.opacity(0.3), lineWidth: 1)
        )
    }

    /// Each segment is (text, role); colored runs joined without separators.
    private func styled(_ text: String, _ roles: [AppearanceColorRole]) -> some View {
        let words = text.split(separator: " ", omittingEmptySubsequences: false)
        var runs = Text("")
        for (index, word) in words.enumerated() {
            let role = roles[min(index, roles.count - 1)]
            runs = runs + Text(index == 0 ? String(word) : " \(word)")
                .foregroundColor(Color(appearance.color(for: role)))
        }
        return runs
    }
}

/// NSColorWell wrapped for SwiftUI — AppKit owns the color panel.
private struct ColorWellButton: NSViewRepresentable {
    let color: NSColor
    let onChange: (NSColor) -> Void

    func makeNSView(context: Context) -> NSColorWell {
        let well = NSColorWell()
        well.color = color
        well.target = context.coordinator
        well.action = #selector(Coordinator.colorChanged(_:))
        return well
    }

    func updateNSView(_ well: NSColorWell, context: Context) {
        if well.color != color { well.color = color }
        context.coordinator.onChange = onChange
    }

    func makeCoordinator() -> Coordinator { Coordinator(onChange: onChange) }

    final class Coordinator: NSObject {
        var onChange: (NSColor) -> Void
        init(onChange: @escaping (NSColor) -> Void) { self.onChange = onChange }
        @objc func colorChanged(_ sender: NSColorWell) { onChange(sender.color) }
    }
}

/// Settings export/import — a flat `preferences` map of the same keys the
/// Linux shell stores in `~/.config/pitex/preferences.json`, so a backup
/// file moves between macOS and Linux/Windows unchanged. The Pitex Agent
/// section's keys (`ai.*`) and session state (`pitex.pref.workspace.*`,
/// per-project command paths, internal bookkeeping) stay out: auth and
/// the agent runtime are per-machine, not user preferences.
enum SettingsBackup {
    private static let includedPrefixes = [
        "dev.pitex.", "pitex.pref.", "appearance.", "project.", "customShellExecutable",
    ]
    private static let excludedPrefixes = ["ai.", "pitex.pref.workspace."]

    static func includes(_ key: String) -> Bool {
        includedPrefixes.contains { key.hasPrefix($0) }
            && !excludedPrefixes.contains { key.hasPrefix($0) }
    }

    /// `{format, version, preferences:{key: value}}` — Data values (the
    /// `dev.pitex.settings` blob) become embedded JSON objects so the file
    /// is portable.
    static func export() throws -> Data {
        let identifier = Bundle.main.bundleIdentifier ?? "dev.pitex"
        let domain = UserDefaults.standard.persistentDomain(forName: identifier) ?? [:]
        var preferences: [String: Any] = [:]
        for (key, value) in domain where includes(key) {
            if let data = value as? Data,
               let object = try? JSONSerialization.jsonObject(with: data) {
                preferences[key] = object
            } else {
                preferences[key] = value
            }
        }
        return try JSONSerialization.data(
            withJSONObject: [
                "format": "pitex-settings",
                "version": 1,
                "preferences": preferences,
            ],
            options: [.prettyPrinted, .sortedKeys])
    }

    /// Merge a backup file (the envelope above, or a bare key→value map
    /// like Linux's `preferences.json`) into UserDefaults. Returns the
    /// number of applied keys; unknown/non-backup keys are ignored.
    @discardableResult
    static func importData(_ data: Data) throws -> Int {
        let object = try JSONSerialization.jsonObject(with: data)
        guard let root = object as? [String: Any] else {
            throw NSError(domain: "SettingsBackup", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Not a settings file"])
        }
        let preferences = (root["preferences"] as? [String: Any]) ?? root
        var applied = 0
        let defaults = UserDefaults.standard
        for (key, value) in preferences where includes(key) {
            // JSON null is NSNull — not plist-valid, would raise in set().
            if value is NSNull { continue }
            if key == "dev.pitex.settings", value is [String: Any] {
                // The blob is stored as Data — re-encode the JSON object.
                guard let encoded = try? JSONSerialization.data(withJSONObject: value) else { continue }
                defaults.set(encoded, forKey: key)
            } else {
                defaults.set(value, forKey: key)
            }
            applied += 1
        }
        return applied
    }
}

/// One-click "register as default editor" — sets Launch Services'
/// editor-role handler for the content types the app declares (read from
/// the bundle's own document types so the list never drifts from
/// Info.plist). The TeX Compile tab's button claims everything except
/// Markdown; the Markdown tab's button claims Markdown alone.
enum DefaultEditorRegistration {
    private static let markdownType = "net.daringfireball.markdown"

    static func registerAsDefault(markdown: Bool = false) {
        guard let bundleID = Bundle.main.bundleIdentifier else { return }
        let documentTypes = Bundle.main.object(
            forInfoDictionaryKey: "CFBundleDocumentTypes"
        ) as? [[String: Any]] ?? []
        let contentTypes = documentTypes.flatMap {
            $0["LSItemContentTypes"] as? [String] ?? []
        }.filter { ($0 == markdownType) == markdown }
        for contentType in contentTypes {
            LSSetDefaultRoleHandlerForContentType(
                contentType as CFString, .editor, bundleID as CFString
            )
        }
        if let type = UTType(filenameExtension: markdown ? "md" : "tex") {
            NSWorkspace.shared.setDefaultApplication(
                at: Bundle.main.bundleURL, toOpen: type
            ) { _ in }
        }
    }
}
