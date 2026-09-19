import BuildCore
import MacPlatform
import SwiftUI

extension ConsoleSection {
    var titleKey: LocalizedStringKey {
        switch self {
        case .assistant: "assistant.title"
        case .terminal: "console.terminal"
        case .issues: "build.issues"
        case .log: "build.log"
        }
    }
}

/// Severity filter for the Issues tab, mirroring the reference editor's
/// All (n) | Errors (n) | Warnings (n) chips.
private enum IssueFilter: String, CaseIterable {
    case all
    case errors
    case warnings

    var titleKey: LocalizedStringKey {
        switch self {
        case .all: "issues.filter.all"
        case .errors: "issues.filter.errors"
        case .warnings: "issues.filter.warnings"
        }
    }

    func matches(_ issue: BuildIssueRecord) -> Bool {
        switch self {
        case .all: true
        case .errors: issue.severity == .error
        case .warnings: issue.severity == .warning
        }
    }

    func count(in issues: [BuildIssueRecord]) -> Int {
        issues.filter { matches($0) }.count
    }
}

/// Bottom console under the editor: a Terminal | Issues | Log segmented
/// picker and the selected section's content. The terminal is a real
/// SwiftTerm shell rooted at the project directory — there are no separate
/// command-entry rows, matching the reference editor's console.
struct BottomConsoleView: View {
    @ObservedObject var workspace: WorkspaceModel
    // Observed so terminal font/color changes re-render the pane; the
    // representable's updateNSView applies them to the live terminal.
    @ObservedObject private var appearance = AppearanceSettings.shared
    @State private var issueFilter: IssueFilter = .all

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Picker("console.terminal", selection: $workspace.consoleSection) {
                    ForEach(ConsoleSection.allCases) { item in
                        Text(item.titleKey).tag(item)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(maxWidth: 420)
                .accessibilityIdentifier("pitex.console.section")

                if workspace.consoleSection == .issues {
                    issueFilterChips
                }
                Spacer()
            }
            .padding(.vertical, 6)
            .padding(.horizontal, 10)

            Divider()

            switch workspace.consoleSection {
            case .assistant:
                assistantPane
            case .terminal:
                terminalPane
            case .issues:
                issuesPane
            case .log:
                logPane
            }
        }
        .frame(minHeight: 120, idealHeight: 190)
        .accessibilityIdentifier("pitex.console")
    }

    // MARK: - Assistant

    @ViewBuilder
    private var assistantPane: some View {
        if let coordinator = workspace.agent {
            AgentPanel(coordinator: coordinator, settings: workspace.settings)
        } else {
            ContentUnavailableView("assistant.title", systemImage: "bubble.left.and.text.bubble.right")
                .accessibilityIdentifier("pitex.assistant.empty")
        }
    }

    /// All / Errors / Warnings capsule chips with live counts — only visible
    /// while the Issues tab is selected, as in the reference console.
    private var issueFilterChips: some View {
        HStack(spacing: 6) {
            ForEach(IssueFilter.allCases, id: \.self) { filter in
                let count = filter.count(in: workspace.buildIssues)
                Button {
                    issueFilter = filter
                } label: {
                    (Text(filter.titleKey) + Text(" (\(count))"))
                        .font(.caption)
                        .padding(.horizontal, 9)
                        .padding(.vertical, 3)
                }
                .buttonStyle(.plain)
                .foregroundStyle(issueFilter == filter ? Color.white : Color.primary)
                .background(
                    Capsule().fill(issueFilter == filter
                        ? Color.accentColor
                        : Color.secondary.opacity(0.18))
                )
                .accessibilityIdentifier("pitex.issues.filter.\(filter.rawValue)")
            }
        }
    }

    // MARK: - Terminal

    private var terminalPane: some View {
        TerminalShellView(
            workingDirectory: workspace.projectURL,
            session: workspace.terminalSession,
            font: AppearanceSettings.shared.terminalFont,
            foregroundColor: AppearanceSettings.shared.color(for: .bodyText),
            backgroundColor: AppearanceSettings.shared.color(for: .editorBackground)
        )
        .accessibilityIdentifier("pitex.console.terminal")
    }

    // MARK: - Issues

    private var issuesPane: some View {
        let filtered = workspace.buildIssues.filter { issueFilter.matches($0) }
        return Group {
            if filtered.isEmpty {
                ContentUnavailableView(
                    "console.no_issues",
                    systemImage: "checkmark.circle",
                    description: Text("console.no_issues_detail")
                )
            } else {
                List(filtered, id: \.self) { issue in
                    Button {
                        if let file = issue.file, let root = workspace.projectURL {
                            let url = root.appendingPathComponent(file)
                            Task {
                                await workspace.activateDocument(url)
                                if let line = issue.line {
                                    workspace.jumpTo(line: line, column: issue.column ?? 0)
                                }
                            }
                        }
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: issue.severity == .error ? "xmark.octagon" : "exclamationmark.triangle")
                                .foregroundStyle(issue.severity == .error ? .red : .orange)
                            VStack(alignment: .leading, spacing: 1) {
                                Text(verbatim: issue.message)
                                    .lineLimit(2)
                                if let file = issue.file {
                                    Text(verbatim: "\(file)\(issue.line.map { ":\($0)" } ?? "")")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
                .listStyle(.plain)
            }
        }
        .accessibilityIdentifier("pitex.build.issues")
    }

    // MARK: - Log

    private var logPane: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                ProgressView()
                    .controlSize(.small)
                    .opacity(workspace.isBuilding ? 1 : 0)
                Button("build.cancel") { Task { await workspace.cancelBuild() } }
                    .disabled(!workspace.isBuilding)
                    .controlSize(.small)
                    .accessibilityIdentifier("pitex.build.cancel")
                Spacer()
            }
            .accessibilityIdentifier("pitex.buildStatus")
            Group {
                switch workspace.buildState {
                case let .unavailable(reason):
                    VStack(alignment: .leading, spacing: 6) {
                        Text("build.log_empty")
                            .foregroundStyle(.secondary)
                        Text(verbatim: reason)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                case .building:
                    if workspace.buildLogText.isEmpty {
                        Text("build.running")
                    } else {
                        logText(workspace.buildLogText)
                    }
                case let .succeeded(_, log):
                    logText(log.isEmpty ? workspace.buildLogText : log)
                case let .failed(log):
                    logText(workspace.buildLogText.isEmpty ? log : workspace.buildLogText)
                }
            }
        }
        .padding(.horizontal, 10)
        .padding(.bottom, 8)
        .accessibilityIdentifier("pitex.buildLog")
    }

    private func logText(_ log: String) -> some View {
        ScrollView {
            Text(verbatim: log)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
