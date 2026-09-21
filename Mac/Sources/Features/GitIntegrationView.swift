import GitCore
import SwiftUI

/// Process-level `git` runner — argv comes from `GitSupport` so the Linux
/// shell runs identical commands (`pitex-shell/git.rs` is the port).
enum GitRunner {
    struct Result: Sendable {
        var code: Int32
        var stdout: String
        var stderr: String
        var errorText: String {
            (stderr.isEmpty ? stdout : stderr).trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    /// GUI apps get a minimal PATH — check the usual git locations.
    static func binaryURL() -> URL? {
        for path in ["/usr/bin/git", "/opt/homebrew/bin/git", "/usr/local/bin/git"]
        where FileManager.default.isExecutableFile(atPath: path) {
            return URL(fileURLWithPath: path)
        }
        return nil
    }

    static func run(_ arguments: [String], in directory: URL) async -> Result {
        await Task.detached {
            guard let binary = binaryURL() else {
                return Result(code: -1, stdout: "", stderr: "git is not installed")
            }
            let process = Process()
            process.executableURL = binary
            process.arguments = arguments
            process.currentDirectoryURL = directory
            let out = Pipe()
            let err = Pipe()
            process.standardOutput = out
            process.standardError = err
            do {
                try process.run()
            } catch {
                return Result(code: -1, stdout: "", stderr: error.localizedDescription)
            }
            let outData = out.fileHandleForReading.readDataToEndOfFile()
            let errData = err.fileHandleForReading.readDataToEndOfFile()
            process.waitUntilExit()
            return Result(
                code: process.terminationStatus,
                stdout: String(decoding: outData, as: UTF8.self),
                stderr: String(decoding: errData, as: UTF8.self)
            )
        }.value
    }
}

extension WorkspaceModel {
    /// Refresh the Git Integration panel: repo detection, status, branches,
    /// and log — all off-actor via `GitRunner`; results land back here.
    func refreshGit() {
        guard let projectRoot = projectURL else {
            gitStatus = nil
            gitCommits = []
            gitBranches = []
            return
        }
        Task {
            let top = await GitRunner.run(GitSupport.topLevelArgs, in: projectRoot)
            guard top.code == 0 else {
                gitStatus = nil
                gitCommits = []
                gitBranches = []
                return
            }
            let root = URL(fileURLWithPath: top.stdout.trimmingCharacters(in: .whitespacesAndNewlines))
            async let statusResult = GitRunner.run(GitSupport.statusArgs, in: root)
            async let branchResult = GitRunner.run(GitSupport.branchArgs, in: root)
            async let logResult = GitRunner.run(GitSupport.logArgs(), in: root)
            let (status, branches, log) = await (statusResult, branchResult, logResult)
            gitStatus = status.code == 0
                ? GitSupport.parseStatus(status.stdout, root: root.path)
                : nil
            gitBranches = branches.code == 0 ? GitSupport.parseBranches(branches.stdout) : []
            gitCommits = log.code == 0 ? GitSupport.parseLog(log.stdout) : []
        }
    }

    /// Sequential git steps inside the repository; first failure wins.
    func gitOperation(_ steps: [[String]]) {
        runGit { root in
            for args in steps {
                let result = await GitRunner.run(args, in: root)
                if result.code != 0 { return result.errorText }
            }
            return nil
        }
    }

    /// Tries argv candidates in order until one exits 0 (e.g. `restore
    /// --staged` → `reset HEAD` → `rm --cached` on a no-commit repo).
    func gitOperationAny(_ candidates: [[String]]) {
        runGit { root in
            var lastError: String?
            for args in candidates {
                let result = await GitRunner.run(args, in: root)
                if result.code == 0 { return nil }
                lastError = result.errorText
            }
            return lastError
        }
    }

    private func runGit(_ body: @Sendable @escaping (URL) async -> String?) {
        guard let status = gitStatus else { return }
        let root = URL(fileURLWithPath: status.root)
        gitBusy = true
        gitError = nil
        Task {
            gitError = await body(root)
            gitBusy = false
            refreshGit()
        }
    }

    func stageGit(_ change: GitChange) {
        gitOperation([GitSupport.stageArgs(change.path)])
    }

    func unstageGit(_ change: GitChange) {
        gitOperationAny([
            GitSupport.unstageArgs(change.path),
            GitSupport.unstageFallbackArgs(change.path),
            GitSupport.unstageNoHeadArgs(change.path),
        ])
    }

    func stageAllGit() {
        gitOperation([GitSupport.stageAllArgs()])
    }

    func unstageAllGit() {
        gitOperationAny([GitSupport.unstageAllArgs(), GitSupport.unstageAllNoHeadArgs()])
    }

    /// VSCode "Commit": staged changes only; when nothing is staged the
    /// button commits everything (stage-all first, like the VSCode prompt).
    func commitGit() {
        guard let status = gitStatus else { return }
        let message = gitCommitMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !message.isEmpty, !status.staged.isEmpty || !status.unstaged.isEmpty else { return }
        var steps: [[String]] = []
        if status.staged.isEmpty { steps.append(GitSupport.stageAllArgs()) }
        steps.append(GitSupport.commitArgs(message, all: false))
        gitOperation(steps)
        gitCommitMessage = ""
    }

    func fetchGit() { gitOperation([GitSupport.fetchArgs]) }
    func pullGit() { gitOperation([GitSupport.pullArgs]) }
    func pushGit() { gitOperation([GitSupport.pushArgs]) }
    func syncGit() { gitOperation([GitSupport.pullArgs, GitSupport.pushArgs]) }
    func switchGit(_ branch: String) { gitOperation([GitSupport.switchArgs(branch)]) }
    func createGitBranch(_ name: String) {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { return }
        gitOperation([GitSupport.createBranchArgs(name)])
    }

    /// Discard like VSCode: untracked files are deleted, staged-new files
    /// are `git rm -f`ed, everything else checks out HEAD (staged) or the
    /// index (unstaged).
    func discardGit(_ change: GitChange) {
        if change.kind == .untracked {
            gitOperation([GitSupport.cleanArgs(change.path)])
        } else if change.staged {
            gitOperation([
                change.kind == .added
                    ? GitSupport.removeFileArgs(change.path)
                    : GitSupport.discardStagedArgs(change.path),
            ])
        } else {
            gitOperation([GitSupport.discardArgs(change.path)])
        }
    }

    /// `git init` at the project root — the panel's only op when the
    /// project is not yet a repository.
    func initGit() {
        guard let projectRoot = projectURL else { return }
        gitBusy = true
        gitError = nil
        Task {
            let result = await GitRunner.run(GitSupport.initArgs, in: projectRoot)
            gitBusy = false
            gitError = result.code == 0 ? nil : result.errorText
            refreshGit()
        }
    }

    func openGitChange(_ change: GitChange) {
        guard let root = gitStatus?.root else { return }
        let url = URL(fileURLWithPath: root).appendingPathComponent(change.path)
        Task { await activateDocument(url) }
    }
}

/// Git Integration console pane — VSCode Source Control layout adapted to
/// the bottom console: repository header, staged/unstaged change lists with
/// per-file actions, the commit box, and the commit graph.
struct GitIntegrationView: View {
    @ObservedObject var workspace: WorkspaceModel
    @State private var discardTarget: GitChange?
    @State private var showNewBranch = false
    @State private var newBranchName = ""
    private let refreshTimer = Timer.publish(every: 4, on: .main, in: .common).autoconnect()

    var body: some View {
        Group {
            if let status = workspace.gitStatus {
                repoView(status)
            } else {
                noRepoView
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityIdentifier("pitex.git")
        .onAppear { workspace.refreshGit() }
        .onReceive(refreshTimer) { _ in
            if !workspace.gitBusy { workspace.refreshGit() }
        }
        .confirmationDialog(
            "git.discard",
            isPresented: Binding(
                get: { discardTarget != nil },
                set: { if !$0 { discardTarget = nil } }
            )
        ) {
            Button("git.discard", role: .destructive) {
                if let target = discardTarget { workspace.discardGit(target) }
            }
            Button("git.cancel", role: .cancel) {}
        } message: {
            if let target = discardTarget {
                Text(String(
                    format: String(localized: target.kind == .untracked
                        ? "git.discard_confirm_untracked"
                        : "git.discard_confirm"),
                    target.path
                ))
            }
        }
        .sheet(isPresented: $showNewBranch) { newBranchSheet }
    }

    // MARK: - Repository

    private var noRepoView: some View {
        VStack(spacing: 10) {
            Image(systemName: "arrow.triangle.branch")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("git.no_repo")
                .foregroundStyle(.secondary)
            if workspace.projectURL != nil {
                Button("git.init") { workspace.initGit() }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
                    .accessibilityIdentifier("pitex.git.init")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func repoView(_ status: GitStatus) -> some View {
        VStack(spacing: 0) {
            header(status)
            if let error = workspace.gitError {
                Text(verbatim: error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .lineLimit(2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 10)
            }
            Divider()
            HStack(spacing: 0) {
                changesColumn(status)
                    .frame(width: 340)
                Divider()
                graphColumn
                    .frame(maxWidth: .infinity)
            }
        }
    }

    private func header(_ status: GitStatus) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "arrow.triangle.branch")
                .foregroundStyle(.secondary)
            Text(verbatim: status.repoName)
                .font(.callout.bold())
                .lineLimit(1)
            branchMenu(status)
            if status.ahead > 0 || status.behind > 0 {
                Text(verbatim: "↑\(status.ahead) ↓\(status.behind)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if workspace.gitBusy {
                ProgressView()
                    .controlSize(.small)
            }
            toolbarButton("arrow.down.to.line", help: "git.pull") {
                workspace.pullGit()
            }
            .accessibilityIdentifier("pitex.git.pull")
            toolbarButton("arrow.up.to.line", help: "git.push") {
                workspace.pushGit()
            }
            .accessibilityIdentifier("pitex.git.push")
            toolbarButton("arrow.triangle.2.circlepath", help: "git.sync") {
                workspace.syncGit()
            }
            .accessibilityIdentifier("pitex.git.sync")
            toolbarButton("arrow.clockwise", help: "git.refresh") {
                workspace.refreshGit()
            }
            .accessibilityIdentifier("pitex.git.refresh")
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
    }

    private func toolbarButton(
        _ icon: String,
        help: LocalizedStringKey,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.caption)
        }
        .buttonStyle(.borderless)
        .help(help)
        .disabled(workspace.gitBusy)
    }

    private func branchMenu(_ status: GitStatus) -> some View {
        Menu {
            ForEach(workspace.gitBranches, id: \.self) { branch in
                Button(branch) { workspace.switchGit(branch) }
                    .disabled(branch == status.branch)
            }
            if !workspace.gitBranches.isEmpty { Divider() }
            Button("git.branch_new") { showNewBranch = true }
                .accessibilityIdentifier("pitex.git.branchNew")
        } label: {
            HStack(spacing: 3) {
                Text(verbatim: status.branch)
                    .font(.caption.bold())
                Image(systemName: "chevron.down")
                    .font(.caption2)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(Capsule().fill(Color.secondary.opacity(0.15)))
        }
        .menuStyle(.borderlessButton)
        .accessibilityIdentifier("pitex.git.branch")
    }

    // MARK: - Changes

    private func changesColumn(_ status: GitStatus) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 2) {
                    if !status.staged.isEmpty {
                        sectionHeader(
                            "git.staged",
                            count: status.staged.count,
                            icon: "minus",
                            help: "git.unstage_all",
                            action: { workspace.unstageAllGit() }
                        )
                        ForEach(status.staged, id: \.path) { changeRow($0) }
                    }
                    sectionHeader(
                        "git.changes",
                        count: status.unstaged.count,
                        icon: "plus",
                        help: "git.stage_all",
                        action: { workspace.stageAllGit() }
                    )
                    if status.staged.isEmpty && status.unstaged.isEmpty {
                        Text("git.no_changes")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .padding(.vertical, 4)
                    }
                    ForEach(status.unstaged, id: \.path) { changeRow($0) }
                }
            }
            .accessibilityIdentifier("pitex.git.changes")
            commitBox(status)
        }
        .padding(.horizontal, 8)
        .padding(.bottom, 8)
    }

    private func sectionHeader(
        _ title: LocalizedStringKey,
        count: Int,
        icon: String,
        help: LocalizedStringKey,
        action: @escaping () -> Void
    ) -> some View {
        HStack {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            Text(verbatim: "(\(count))")
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
            Button(action: action) {
                Image(systemName: icon)
                    .font(.caption)
            }
            .buttonStyle(.borderless)
            .help(help)
            .disabled(workspace.gitBusy)
        }
        .padding(.top, 2)
    }

    private func changeRow(_ change: GitChange) -> some View {
        HStack(spacing: 6) {
            Text(verbatim: change.kind.badge)
                .font(.caption.bold().monospaced())
                .foregroundStyle(badgeColor(change.kind))
                .frame(width: 12)
            Button {
                workspace.openGitChange(change)
            } label: {
                HStack(spacing: 5) {
                    Text(verbatim: change.path.split(separator: "/").last.map(String.init) ?? change.path)
                        .font(.caption)
                        .lineLimit(1)
                    if let slash = change.path.lastIndex(of: "/") {
                        Text(verbatim: String(change.path[..<slash]))
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    Spacer()
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .help(String(localized: "git.open_file"))
            Button {
                workspace.openGitChange(change)
            } label: {
                Image(systemName: "arrow.up.forward.square")
                    .font(.caption)
            }
            .buttonStyle(.borderless)
            .help(String(localized: "git.open_file"))
            if change.staged {
                actionButton("minus", help: "git.unstage") { workspace.unstageGit(change) }
            } else {
                actionButton("plus", help: "git.stage") { workspace.stageGit(change) }
            }
            actionButton("arrow.uturn.backward", help: "git.discard") { discardTarget = change }
        }
    }

    private func actionButton(
        _ icon: String,
        help: LocalizedStringKey,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.caption)
        }
        .buttonStyle(.borderless)
        .help(help)
        .disabled(workspace.gitBusy)
    }

    private func badgeColor(_ kind: GitChangeKind) -> Color {
        switch kind {
        case .modified, .typeChanged: return .orange
        case .added, .untracked: return .green
        case .deleted, .conflicted: return .red
        case .renamed, .copied: return .cyan
        }
    }

    // MARK: - Commit

    private func commitBox(_ status: GitStatus) -> some View {
        VStack(spacing: 6) {
            ZStack(alignment: .topLeading) {
                TextEditor(text: $workspace.gitCommitMessage)
                    .font(.caption)
                    .frame(height: 42)
                    .onKeyPress(keys: [.return]) { press in
                        guard press.modifiers == .command else { return .ignored }
                        workspace.commitGit()
                        return .handled
                    }
                if workspace.gitCommitMessage.isEmpty {
                    Text(String(
                        format: String(localized: "git.commit_placeholder"),
                        status.branch
                    ))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.top, 8)
                    .padding(.leading, 5)
                    .allowsHitTesting(false)
                }
            }
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .stroke(Color.secondary.opacity(0.4))
            )
            Button {
                workspace.commitGit()
            } label: {
                Label("git.commit", systemImage: "checkmark")
                    .font(.caption)
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.small)
            .disabled(
                workspace.gitCommitMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                    || workspace.gitBusy
                    || (status.staged.isEmpty && status.unstaged.isEmpty)
            )
            .accessibilityIdentifier("pitex.git.commit")
        }
    }

    // MARK: - Graph

    private var graphColumn: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("git.graph")
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if workspace.gitCommits.isEmpty {
                Text("git.no_commits")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.vertical, 4)
            }
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(workspace.gitCommits, id: \.hash) { commitRow($0) }
                }
            }
            .accessibilityIdentifier("pitex.git.graph")
        }
        .padding(.horizontal, 8)
        .padding(.top, 4)
    }

    private func commitRow(_ commit: GitCommit) -> some View {
        HStack(alignment: .top, spacing: 8) {
            VStack(spacing: 0) {
                Circle()
                    .fill(commit.isHead ? Color.accentColor : Color.secondary)
                    .frame(width: 7, height: 7)
                    .padding(.top, 4)
                Rectangle()
                    .fill(Color.secondary.opacity(0.3))
                    .frame(width: 1.5)
            }
            .frame(width: 10)
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(verbatim: commit.subject)
                        .font(.caption)
                        .lineLimit(1)
                    ForEach(commit.refs, id: \.self) { ref in
                        Text(verbatim: ref.replacingOccurrences(of: "tag: ", with: ""))
                            .font(.caption2)
                            .padding(.horizontal, 5)
                            .padding(.vertical, 1)
                            .background(Capsule().fill(Color.accentColor.opacity(0.18)))
                    }
                }
                Text(verbatim: "\(commit.author) · \(commit.relativeDate) · \(commit.hash)")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
    }

    // MARK: - New branch

    private var newBranchSheet: some View {
        VStack(spacing: 12) {
            Text("git.branch_new")
                .font(.headline)
            TextField("git.branch_name", text: $newBranchName)
                .textFieldStyle(.roundedBorder)
            HStack {
                Button("git.cancel") { showNewBranch = false }
                Spacer()
                Button("git.create") {
                    workspace.createGitBranch(newBranchName)
                    showNewBranch = false
                    newBranchName = ""
                }
                .buttonStyle(.borderedProminent)
                .disabled(newBranchName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(16)
        .frame(width: 260)
    }
}
