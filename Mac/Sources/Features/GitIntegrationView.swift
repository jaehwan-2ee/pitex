import AppKit
import BuildCore
import GitCore
import RemoteCore
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

    /// The one place deciding whether a command runs locally or on the
    /// device — `remote` non-nil while an SSH workspace is open. For a
    /// remote run `directory` is a mirror path on the first call
    /// (`projectURL`, mapped by `remotePath`) and a device path on
    /// follow-ups (`status.root`, passed through verbatim).
    static func run(_ arguments: [String], in directory: URL, remote: RemoteSync? = nil) async -> Result {
        if let remote {
            do {
                let deviceDir = await remote.remotePath(for: directory) ?? directory.path
                let result = try await remote.client.runGit(arguments, in: deviceDir)
                return Result(code: result.status, stdout: result.stdoutText, stderr: result.stderrText)
            } catch {
                return Result(code: -1, stdout: "", stderr: error.localizedDescription)
            }
        }
        return await Task.detached {
            guard let binary = binaryURL() else {
                return Result(code: -1, stdout: "", stderr: "git is not installed")
            }
            do {
                // Drain both pipes concurrently with the shared runner; a verbose
                // Git hook must not deadlock while stdout is being read first.
                let result = try await ProcessRunner().run(
                    DirectCommandPlan(executable: binary.path, arguments: arguments), projectRoot: directory
                )
                let code: Int32
                switch result.termination {
                case .exited(let value): code = value
                case .signaled(let signal): code = -signal
                }
                return Result(code: code, stdout: String(decoding: result.standardOutput, as: UTF8.self),
                              stderr: String(decoding: result.standardError, as: UTF8.self))
            } catch {
                return Result(code: -1, stdout: "", stderr: error.localizedDescription)
            }
        }.value
    }
}

/// An open diff covering the editor area. `file` nil means the whole
/// change set ("Open Changes"); `sections` is nil while git is in
/// flight and `error` carries a failed run.
struct GitDiffSession {
    /// What the diff is against: a commit's patch (`git show`), or the
    /// working tree — unstaged rows compare index ↔ worktree, staged
    /// rows HEAD ↔ index.
    enum Source: Equatable {
        case commit(GitCommit)
        case workingTree(staged: Bool)
    }
    let source: Source
    let file: GitCommitFile?
    var sections: [GitDiffFileSection]?
    /// Folded render rows for `sections`, computed once when the diff loads
    /// — the view body re-folded every row on each unrelated publish before.
    var displayItems: [[GitDiffItem]]?
    var error: String?
}

extension WorkspaceModel {
    /// Refresh the Git Integration panel: repo detection, status, branches,
    /// and log — all off-actor via `GitRunner`; results land back here.
    func refreshGit() {
        guard let projectRoot = projectURL else {
            gitStatus = nil
            gitCommits = []
            gitBranches = []
            clearGitHistoryState()
            return
        }
        guard bottomPanelVisible, consoleSection == .git, !gitRefreshInFlight else { return }
        gitRefreshInFlight = true
        Task {
            defer {
                gitRefreshInFlight = false
                if projectURL != projectRoot { refreshGit() }
            }
            // Remote: flush local edits first so status/commit read the
            // saved bytes; a failed push lands in `remote.status` (offline).
            if remote != nil { _ = await pushRemote() }
            let top = await GitRunner.run(GitSupport.topLevelArgs, in: projectRoot, remote: remote?.sync)
            guard projectURL == projectRoot else { return }
            guard top.code == 0 else {
                if let remote, top.code == -1 || top.code == 127 {
                    // A transport failure is an error, not "not a
                    // repository" — SSH down, or a 127 from loginExec:
                    // the device shell found no `git`. The last status
                    // stays and the pane shows the error.
                    let text = top.errorText
                    gitError = text.isEmpty && top.code == 127
                        ? "git was not found on \(remote.deviceName)" : text
                    return
                }
                if remote != nil { gitError = nil }
                gitStatus = nil
                gitCommits = []
                gitBranches = []
                clearGitHistoryState()
                return
            }
            if remote != nil { gitError = nil }
            let root = URL(fileURLWithPath: top.stdout.trimmingCharacters(in: .whitespacesAndNewlines))
            async let statusResult = GitRunner.run(GitSupport.statusArgs, in: root, remote: remote?.sync)
            async let branchResult = GitRunner.run(GitSupport.branchArgs, in: root, remote: remote?.sync)
            async let logResult = GitRunner.run(GitSupport.logArgs(), in: root, remote: remote?.sync)
            let (status, branches, log) = await (statusResult, branchResult, logResult)
            guard projectURL == projectRoot else { return }
            let previous = gitStatus
            let parsed = await Task.detached(priority: .utility) {
                let snapshot = status.code == 0 ? GitSupport.parseStatus(status.stdout, root: root.path) : nil
                return (snapshot, snapshot != previous,
                        branches.code == 0 ? GitSupport.parseBranches(branches.stdout) : [],
                        log.code == 0 ? GitSupport.parseLog(log.stdout) : [])
            }.value
            guard projectURL == projectRoot else { return }
            if parsed.1 { gitStatus = parsed.0 }
            if gitBranches != parsed.2 { gitBranches = parsed.2 }
            if gitCommits != parsed.3 { gitCommits = parsed.3 }
        }
    }

    /// Sequential git steps inside the repository; first failure wins.
    func gitOperation(_ steps: [[String]]) {
        let remoteSync = remote?.sync
        runGit { root in
            for args in steps {
                let result = await GitRunner.run(args, in: root, remote: remoteSync)
                if result.code != 0 { return result.errorText }
            }
            return nil
        }
    }

    /// Tries argv candidates in order until one exits 0 (e.g. `restore
    /// --staged` → `reset HEAD` → `rm --cached` on a no-commit repo).
    func gitOperationAny(_ candidates: [[String]]) {
        let remoteSync = remote?.sync
        runGit { root in
            var lastError: String?
            for args in candidates {
                let result = await GitRunner.run(args, in: root, remote: remoteSync)
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
            // Remote ops push the mirror first (saved bytes reach the
            // device) and pull after (device-side rewrites — checkout,
            // switch, pull — land in the mirror and open editors).
            if remote != nil { _ = await pushRemote() }
            gitError = await body(root)
            if remote != nil { await pullRemote() }
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
    /// `at` pins the start point — the graph context menu branches off
    /// the selected commit, like VSCode's "Create Branch…" there.
    func createGitBranch(_ name: String, at commit: GitCommit? = nil) {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { return }
        gitOperation([GitSupport.createBranchArgs(name, at: commit?.fullHash)])
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

    /// `git init` at the project root — `remoteRoot` on the device for a
    /// remote workspace.
    func initGit() {
        guard let projectRoot = projectURL else { return }
        gitBusy = true
        gitError = nil
        Task {
            if remote != nil { _ = await pushRemote() }
            let result = await GitRunner.run(GitSupport.initArgs, in: projectRoot, remote: remote?.sync)
            if remote != nil { await pullRemote() }
            gitBusy = false
            gitError = result.code == 0 ? nil : result.errorText
            refreshGit()
        }
    }

    func openGitChange(_ change: GitChange) {
        guard let root = gitStatus?.root else { return }
        let url: URL
        if let remote {
            // `root` is a device path — the change maps back into the
            // mirror; a repo extending above `remoteRoot` lists changes
            // that have no local copy and simply don't open.
            guard let local = remote.sync.mirror.localURL(toplevel: root, path: change.path) else { return }
            url = local
        } else {
            url = URL(fileURLWithPath: root).appendingPathComponent(change.path)
        }
        Task { await activateDocument(url) }
    }

    /// Expand/collapse a commit row in the graph; the first expansion
    /// fetches `git show --name-status` so the file list fills in.
    func toggleGitCommit(_ commit: GitCommit) {
        if gitExpandedCommits.contains(commit.hash) {
            gitExpandedCommits.remove(commit.hash)
            return
        }
        gitExpandedCommits.insert(commit.hash)
        guard gitCommitFiles[commit.hash] == nil, let status = gitStatus else { return }
        let root = URL(fileURLWithPath: status.root)
        gitCommitFilesBusy.insert(commit.hash)
        Task {
            let result = await GitRunner.run(GitSupport.commitFilesArgs(commit.fullHash), in: root, remote: remote?.sync)
            let files = await Task.detached(priority: .utility) {
                result.code == 0 ? GitSupport.parseCommitFiles(result.stdout) : []
            }.value
            guard gitStatus?.root == root.path else { return }
            gitCommitFiles[commit.hash] = files
            gitCommitFilesBusy.remove(commit.hash)
        }
    }

    /// Clicking a file under a commit opens its diff over the editor area
    /// — the PDF inspector hides itself while `gitDiff` is set.
    func openGitDiff(_ commit: GitCommit, file: GitCommitFile) {
        guard let status = gitStatus else { return }
        let root = URL(fileURLWithPath: status.root)
        gitDiff = GitDiffSession(source: .commit(commit), file: file)
        Task {
            let result = await GitRunner.run(GitSupport.fileDiffArgs(commit.fullHash, path: file.path), in: root, remote: remote?.sync)
            let rendered: (sections: [GitDiffFileSection], items: [[GitDiffItem]])?
                = await Task.detached(priority: .utility) {
                guard result.code == 0 else { return nil }
                let section = GitDiffFileSection(file: file, binary: result.stdout.contains("Binary files"),
                                               rows: GitSupport.parseFileDiff(result.stdout))
                return ([section], [GitSupport.displayItems(section.rows)])
            }.value
            // A second file click or project switch may have replaced the session.
            guard gitStatus?.root == root.path, gitDiff?.source == .commit(commit), gitDiff?.file == file else { return }
            gitDiff = GitDiffSession(
                source: .commit(commit),
                file: file,
                sections: rendered?.sections,
                displayItems: rendered?.items,
                error: result.code == 0 ? nil : result.errorText
            )
        }
    }

    /// Clicking a Changes row opens its working-tree diff the same way —
    /// the row's ↗ button still opens the file itself.
    func openGitWorkingDiff(_ change: GitChange) {
        guard let status = gitStatus else { return }
        let root = URL(fileURLWithPath: status.root)
        let file = GitCommitFile(path: change.path, kind: change.kind)
        let source = GitDiffSession.Source.workingTree(staged: change.staged)
        gitDiff = GitDiffSession(source: source, file: file)
        Task {
            // Push first so the working diff reads the same saved bytes
            // the editor shows.
            if remote != nil { _ = await pushRemote() }
            let result = await GitRunner.run(GitSupport.workingFileDiffArgs(change), in: root, remote: remote?.sync)
            // `git diff --no-index` (untracked files) exits 1 on differences.
            let ok = result.code == 0 || (change.kind == .untracked && result.code == 1)
            let rendered: (sections: [GitDiffFileSection], items: [[GitDiffItem]])?
                = await Task.detached(priority: .utility) {
                guard ok else { return nil }
                let section = GitDiffFileSection(file: file, binary: result.stdout.contains("Binary files"),
                                               rows: GitSupport.parseFileDiff(result.stdout))
                return ([section], [GitSupport.displayItems(section.rows)])
            }.value
            // A second row click or project switch may have replaced the session.
            guard gitStatus?.root == root.path, gitDiff?.source == source, gitDiff?.file == file else { return }
            gitDiff = GitDiffSession(
                source: source,
                file: file,
                sections: rendered?.sections,
                displayItems: rendered?.items,
                error: ok ? nil : result.errorText
            )
        }
    }

    /// The context menu's "Open Changes" — the whole commit as a
    /// multi-file diff, like VSCode's multi-diff editor.
    func openGitDiff(_ commit: GitCommit) {
        guard let status = gitStatus else { return }
        let root = URL(fileURLWithPath: status.root)
        gitDiff = GitDiffSession(source: .commit(commit), file: nil)
        Task {
            async let filesResult = GitRunner.run(GitSupport.commitFilesArgs(commit.fullHash), in: root, remote: remote?.sync)
            async let diffResult = GitRunner.run(GitSupport.commitDiffArgs(commit.fullHash), in: root, remote: remote?.sync)
            let (files, diff) = await (filesResult, diffResult)
            guard gitDiff?.source == .commit(commit), gitDiff?.file == nil else { return }
            guard diff.code == 0 else {
                gitDiff = GitDiffSession(source: .commit(commit), file: nil, error: diff.errorText)
                return
            }
            let rendered: (sections: [GitDiffFileSection], items: [[GitDiffItem]])
                = await Task.detached(priority: .utility) {
                var kinds: [String: GitChangeKind] = [:]
                if files.code == 0 {
                    for f in GitSupport.parseCommitFiles(files.stdout) { kinds[f.path] = f.kind }
                }
                let sections = GitSupport.parseCommitDiff(diff.stdout).map { section in
                    GitDiffFileSection(file: GitCommitFile(path: section.path, kind: kinds[section.path] ?? .modified),
                                       binary: section.binary, rows: section.rows)
                }
                return (sections, sections.map { GitSupport.displayItems($0.rows) })
            }.value
            guard gitStatus?.root == root.path, gitDiff?.source == .commit(commit), gitDiff?.file == nil else { return }
            gitDiff = GitDiffSession(source: .commit(commit), file: nil,
                                     sections: rendered.sections, displayItems: rendered.items)
        }
    }

    /// "Copy Commit Hash" — the full 40-char id, not the display short one.
    func copyGitCommitHash(_ commit: GitCommit) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(commit.fullHash, forType: .string)
    }

    /// "Copy Commit Message" — subject plus body (`%B`), like VSCode.
    func copyGitCommitMessage(_ commit: GitCommit) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(commit.message, forType: .string)
    }

    func closeGitDiff() { gitDiff = nil }

    /// Project closed or repo detection lost — the history overlays refer
    /// to commits that may no longer resolve.
    private func clearGitHistoryState() {
        gitExpandedCommits = []
        gitCommitFiles = [:]
        gitCommitFilesBusy = []
        gitDiff = nil
    }

    /// The "Suggest" half of the commit box: a one-shot `pi --print` run
    /// with the pending diff inline. A dedicated subprocess (like ghost
    /// completion) keeps the chat transcript and any in-flight ask
    /// untouched; failures surface in the panel's error line.
    func suggestCommitMessage() {
        guard let status = gitStatus, !gitSuggestBusy else { return }
        gitSuggestBusy = true
        gitError = nil
        Task {
            defer { gitSuggestBusy = false }
            let root = URL(fileURLWithPath: status.root)
            let staged = !status.staged.isEmpty
            // The diff reads worktree bytes — push first on remote.
            if remote != nil { _ = await pushRemote() }
            let diff = await GitRunner.run(GitSupport.diffArgs(staged: staged), in: root, remote: remote?.sync)
            let tools = await PiToolchain.discover()
            guard let executable = PiExecutableLocator.resolve(environment: tools.environment) else {
                gitError = "Pitex Agent is not installed yet — see Settings → AI."
                return
            }
            let prompt = await Task.detached(priority: .utility) {
                let changes = staged ? status.staged : status.unstaged
                let files = changes.prefix(100).map { "\($0.kind.badge) \($0.path)" }
                return Self.commitMessagePrompt(branch: status.branch, files: files, diff: diff.stdout, totalFiles: changes.count)
            }.value
            do {
                let launch = try tools.launch(
                    executable, arguments: ["--no-session", "--no-tools", "--print", prompt])
                let plan = try DirectCommandPlan(
                    executable: launch.executable.path,
                    arguments: launch.arguments,
                    environment: .inherit(overrides: AgentCoordinator.childEnvironment(
                        executable: launch.executable, environment: tools.environment))
                )
                // pi runs locally — its directory is the mirror, since a
                // remote `status.root` does not exist on this filesystem.
                let workDirectory = remote != nil ? (projectURL ?? root) : root
                let result = try await ProcessRunner().run(plan, projectRoot: workDirectory, timeout: .seconds(90))
                let text = Self.cleanedCommitMessage(String(decoding: result.standardOutput, as: UTF8.self))
                if result.termination == .exited(code: 0), !text.isEmpty {
                    gitCommitMessage = text
                } else {
                    let detail = String(decoding: result.standardError, as: UTF8.self)
                        .trimmingCharacters(in: .whitespacesAndNewlines)
                    gitError = detail.isEmpty
                        ? "Pitex Agent did not return a commit message."
                        : String(detail.suffix(400))
                }
            } catch {
                gitError = error.localizedDescription
            }
        }
    }

    /// Conventional-commit ask: branch + change list + the diff itself.
    /// The diff is capped so a generated/mass-rename changeset cannot blow
    /// past the model's context on a one-shot prompt.
    nonisolated private static func commitMessagePrompt(branch: String, files: [String], diff: String, totalFiles: Int) -> String {
        let limit = 20_000
        let body = String(decoding: diff.utf8.prefix(limit), as: UTF8.self) + (diff.utf8.count > limit ? "\n… [diff truncated]" : "")
        let names = files.joined(separator: "\n")
        let fileList = String(decoding: names.utf8.prefix(limit), as: UTF8.self) + (totalFiles > files.count || names.utf8.count > limit ? "\n… [file list truncated; \(totalFiles) files total]" : "")
        return """
            Write the git commit message for this change set on branch '\(branch)'.
            Changed files:
            \(fileList)

            Diff:
            \(body)

            Reply with ONLY the commit message: one imperative subject line of \
            at most 72 characters, then optionally a blank line and a short \
            body. No quotes, no code fences, no commentary.
            """
    }

    /// `pi --print` should already answer with just the message; this drops
    /// a wrapping code fence or quote pair when a model adds one anyway.
    static func cleanedCommitMessage(_ raw: String) -> String {
        var text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if text.hasPrefix("```") {
            var lines = text.components(separatedBy: "\n").dropFirst()
            if lines.last?.hasPrefix("```") == true { lines = lines.dropLast() }
            text = lines.joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if text.count > 1, text.hasPrefix("\""), text.hasSuffix("\"") {
            text = String(text.dropFirst().dropLast())
        }
        return String(text.prefix(500))
    }
}

/// Git Integration console pane — VSCode Source Control layout adapted to
/// the bottom console: repository header, staged/unstaged change lists with
/// per-file actions, the commit box, and the commit graph.
struct GitIntegrationView: View {
    @ObservedObject var workspace: WorkspaceModel
    /// Observed so Appearance → Editor Font Size resizes the pane live.
    @ObservedObject private var appearance = AppearanceSettings.shared
    @State private var discardTarget: GitChange?
    @State private var showNewBranch = false
    @State private var newBranchName = ""
    /// Set when "New Branch…" came from a commit's context menu — the
    /// branch starts at that commit instead of HEAD.
    @State private var newBranchBase: GitCommit?
    private let refreshTimer = Timer.publish(every: 4, on: .main, in: .common).autoconnect()

    /// Git text follows the editor font size; `delta` keeps the
    /// caption/caption2 hierarchy the fixed styles expressed before.
    private func gitFont(
        _ delta: Double = 0,
        weight: Font.Weight = .regular,
        monospaced: Bool = false
    ) -> Font {
        .system(
            size: max(appearance.fontSize + delta, 8),
            weight: weight,
            design: monospaced ? .monospaced : .default
        )
    }

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
            // A minimized/fully covered window spawns no git subprocesses;
            // reappearing refreshes immediately via the observer below.
            if let window = workspace.window, !window.occlusionState.contains(.visible) { return }
            if !workspace.gitBusy { workspace.refreshGit() }
        }
        .onReceive(NotificationCenter.default.publisher(
            for: NSWindow.didChangeOcclusionStateNotification
        )) { note in
            guard let window = workspace.window,
                  (note.object as? NSWindow) === window,
                  window.occlusionState.contains(.visible),
                  !workspace.gitBusy else { return }
            workspace.refreshGit()
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
            if let error = workspace.gitError {
                Text(verbatim: error)
                    .font(gitFont())
                    .foregroundStyle(.red)
                    .lineLimit(3)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 20)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func repoView(_ status: GitStatus) -> some View {
        VStack(spacing: 0) {
            header(status)
            if let error = workspace.gitError {
                Text(verbatim: error)
                    .font(gitFont())
                    .foregroundStyle(.red)
                    .lineLimit(2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 10)
            }
            Divider()
            // HSplitView: the Changes|Graph divider drags like the outer
            // workspace split — the fixed 340/HStack pairing could not.
            HSplitView {
                changesColumn(status)
                    .frame(minWidth: 240, idealWidth: 340, maxWidth: .infinity, maxHeight: .infinity)
                graphColumn
                    .frame(minWidth: 200, maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }

    private func header(_ status: GitStatus) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "arrow.triangle.branch")
                .foregroundStyle(.secondary)
            Text(verbatim: status.repoName)
                .font(gitFont(1, weight: .bold))
                .lineLimit(1)
            branchMenu(status)
            if status.ahead > 0 || status.behind > 0 {
                Text(verbatim: "↑\(status.ahead) ↓\(status.behind)")
                    .font(gitFont())
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
                .font(gitFont())
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
            Button("git.branch_new") {
                newBranchBase = nil
                showNewBranch = true
            }
            .accessibilityIdentifier("pitex.git.branchNew")
        } label: {
            Text(verbatim: status.branch)
                .font(gitFont(weight: .bold))
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
                            .font(gitFont())
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
                .font(gitFont(weight: .bold))
                .foregroundStyle(.secondary)
            Text(verbatim: "(\(count))")
                .font(gitFont())
                .foregroundStyle(.secondary)
            Spacer()
            Button(action: action) {
                Image(systemName: icon)
                    .font(gitFont())
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
                .font(gitFont(weight: .bold, monospaced: true))
                .foregroundStyle(badgeColor(change.kind))
                .frame(width: 12)
            Button {
                workspace.openGitWorkingDiff(change)
            } label: {
                HStack(spacing: 5) {
                    Text(verbatim: change.path.split(separator: "/").last.map(String.init) ?? change.path)
                        .font(gitFont())
                        .lineLimit(1)
                    if let slash = change.path.lastIndex(of: "/") {
                        Text(verbatim: String(change.path[..<slash]))
                            .font(gitFont(-2))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    Spacer()
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .help(String(localized: "git.open_diff"))
            Button {
                workspace.openGitChange(change)
            } label: {
                Image(systemName: "arrow.up.forward.square")
                    .font(gitFont())
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
                .font(gitFont())
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
                    .font(gitFont())
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
                    .font(gitFont())
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
            // Commit | Suggest — the right half asks Pitex Agent to draft
            // the message into the editor; Commit still requires a message.
            HStack(spacing: 6) {
                Button {
                    workspace.commitGit()
                } label: {
                    Label("git.commit", systemImage: "checkmark")
                        .font(gitFont())
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

                Button {
                    workspace.suggestCommitMessage()
                } label: {
                    Group {
                        if workspace.gitSuggestBusy {
                            ProgressView()
                                .controlSize(.small)
                        } else {
                            Label("git.suggest", systemImage: "wand.and.stars")
                        }
                    }
                    .font(gitFont())
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .disabled(
                    workspace.gitBusy
                        || workspace.gitSuggestBusy
                        || (status.staged.isEmpty && status.unstaged.isEmpty)
                )
                .help(String(localized: "git.suggest_help"))
                .accessibilityIdentifier("pitex.git.suggest")
            }
        }
    }

    // MARK: - Graph

    private var graphColumn: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("git.graph")
                .font(gitFont(weight: .bold))
                .foregroundStyle(.secondary)
            if workspace.gitCommits.isEmpty {
                Text("git.no_commits")
                    .font(gitFont())
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

    /// Commit rows expand in place (VSCode/GitLens-style): the changed
    /// files list fills in under the row and each file opens a fullscreen
    /// diff over the editor area.
    private func commitRow(_ commit: GitCommit) -> some View {
        let expanded = workspace.gitExpandedCommits.contains(commit.hash)
        return VStack(alignment: .leading, spacing: 0) {
            Button {
                workspace.toggleGitCommit(commit)
            } label: {
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: expanded ? "chevron.down" : "chevron.right")
                        .font(gitFont(-2))
                        .foregroundStyle(.secondary)
                        .frame(width: 10)
                        .padding(.top, 5)
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
                                .font(gitFont())
                                .lineLimit(1)
                            ForEach(commit.refs, id: \.self) { ref in
                                Text(verbatim: ref.replacingOccurrences(of: "tag: ", with: ""))
                                    .font(gitFont(-2))
                                    .padding(.horizontal, 5)
                                    .padding(.vertical, 1)
                                    .background(Capsule().fill(Color.accentColor.opacity(0.18)))
                            }
                        }
                        Text(verbatim: "\(commit.author) · \(commit.relativeDate) · \(commit.hash)")
                            .font(gitFont(-2))
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("pitex.git.commit.\(commit.hash)")
            .contextMenu {
                Button("git.open_changes") { workspace.openGitDiff(commit) }
                    .accessibilityIdentifier("pitex.git.commit.openChanges")
                Divider()
                Button("git.copy_hash") { workspace.copyGitCommitHash(commit) }
                    .accessibilityIdentifier("pitex.git.commit.copyHash")
                Button("git.copy_message") { workspace.copyGitCommitMessage(commit) }
                    .accessibilityIdentifier("pitex.git.commit.copyMessage")
                Divider()
                Button("git.branch_new") {
                    newBranchBase = commit
                    showNewBranch = true
                }
                .accessibilityIdentifier("pitex.git.commit.newBranch")
            }
            if expanded {
                commitFileList(commit)
            }
        }
    }

    /// Changed files under an expanded commit; clicking one opens the
    /// side-by-side diff over the editor area.
    private func commitFileList(_ commit: GitCommit) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            if workspace.gitCommitFilesBusy.contains(commit.hash) {
                ProgressView()
                    .controlSize(.small)
                    .padding(.vertical, 2)
            } else {
                ForEach(workspace.gitCommitFiles[commit.hash] ?? [], id: \.path) { file in
                    Button {
                        workspace.openGitDiff(commit, file: file)
                    } label: {
                        HStack(spacing: 6) {
                            Text(verbatim: file.kind.badge)
                                .font(gitFont(weight: .bold, monospaced: true))
                                .foregroundStyle(badgeColor(file.kind))
                                .frame(width: 12)
                            Text(verbatim: file.path.split(separator: "/").last.map(String.init) ?? file.path)
                                .font(gitFont())
                                .lineLimit(1)
                            if let slash = file.path.lastIndex(of: "/") {
                                Text(verbatim: String(file.path[..<slash]))
                                    .font(gitFont(-2))
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                            Spacer()
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .help(String(localized: "git.open_diff"))
                    .accessibilityIdentifier("pitex.git.commitFile")
                }
            }
        }
        .padding(.leading, 28)
        .padding(.vertical, 2)
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
                    workspace.createGitBranch(newBranchName, at: newBranchBase)
                    showNewBranch = false
                    newBranchName = ""
                    newBranchBase = nil
                }
                .buttonStyle(.borderedProminent)
                .disabled(newBranchName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(16)
        .frame(width: 260)
    }
}

// MARK: - Commit diff

/// VSCode-style diff covering the editor surface: the original on the
/// left, the new revision on the right, unchanged regions folded behind
/// an expandable bar, and the changed character range of a modified line
/// shaded darker — like the source-control diff editor. `session.file`
/// nil renders every file of the commit ("Open Changes", multi-diff).
struct CommitDiffView: View {
    @ObservedObject var workspace: WorkspaceModel
    let session: GitDiffSession
    @ObservedObject private var appearance = AppearanceSettings.shared
    /// Expanded fold bars, keyed "path#foldID".
    @State private var expandedFolds: Set<String> = []
    /// Collapsed file sections in multi-file mode.
    @State private var collapsedFiles: Set<String> = []

    private var mono: Font { Font(appearance.editorFont) }
    private var monoSmall: Font { .system(size: max(appearance.fontSize - 1, 8), design: .monospaced) }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            content
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: appearance.color(for: .editorBackground)))
        .accessibilityIdentifier("pitex.git.diff")
    }

    private var header: some View {
        HStack(spacing: 8) {
            if let file = session.file {
                Text(verbatim: file.kind.badge)
                    .font(monoSmall.weight(.bold))
                    .foregroundStyle(badgeColor(file.kind))
                    .frame(width: 12)
                Text(verbatim: file.path)
                    .font(mono)
                    .lineLimit(1)
                    .truncationMode(.middle)
                if let section = session.sections?.first {
                    stats(section)
                }
            } else if let sections = session.sections {
                Text(String(format: String(localized: "git.diff.files"),
                            "\(sections.count)"))
                    .font(monoSmall)
                    .foregroundStyle(.secondary)
            }
            switch session.source {
            case .commit(let commit):
                Text(verbatim: commit.hash)
                    .font(monoSmall)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 1)
                    .background(Capsule().fill(Color.secondary.opacity(0.15)))
                Text(verbatim: commit.subject)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            case .workingTree(let staged):
                Text(staged ? "git.diff.staged" : "git.diff.working_tree")
                    .font(monoSmall)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 1)
                    .background(Capsule().fill(Color.secondary.opacity(0.15)))
            }
            Spacer()
            Button {
                workspace.closeGitDiff()
            } label: {
                Image(systemName: "xmark")
            }
            .buttonStyle(.borderless)
            .help(String(localized: "command.close"))
            .accessibilityIdentifier("pitex.git.diff.close")
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
    }

    /// `+N −N` chip matching the SCM file rows.
    private func stats(_ section: GitDiffFileSection) -> some View {
        HStack(spacing: 4) {
            Text(verbatim: "+\(section.additions)")
                .foregroundStyle(.green)
            Text(verbatim: "−\(section.deletions)")
                .foregroundStyle(.red)
        }
        .font(monoSmall)
    }

    @ViewBuilder
    private var content: some View {
        if let error = session.error {
            Text(verbatim: error)
                .font(mono)
                .foregroundStyle(.red)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .padding()
        } else if let sections = session.sections {
            if sections.isEmpty {
                Text("git.no_changes")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                diffTable(sections)
            }
        } else {
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    /// Columns are always half the viewport — like VSCode's diff editor,
    /// where each side is a fixed-width pane and the row tint always
    /// covers it. Lines wrap inside their cell instead of scrolling, so a
    /// long paragraph can never push the new side off-screen.
    private func diffTable(_ sections: [GitDiffFileSection]) -> some View {
        GeometryReader { geo in
            let columnWidth = geo.size.width / 2 - 1
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(sections.enumerated()), id: \.offset) { index, section in
                        if session.file == nil {
                            fileHeader(section)
                        }
                        if session.file != nil || !collapsedFiles.contains(section.file.path) {
                            fileRows(section, items: session.displayItems?[index] ?? [],
                                     columnWidth: columnWidth)
                        }
                        if index < sections.count - 1 {
                            Divider().padding(.vertical, 4)
                        }
                    }
                }
                .padding(.vertical, 4)
            }
        }
    }

    /// Collapsible file banner between sections (VSCode's multi-diff
    /// editor header): badge, path, and the +/− counts.
    private func fileHeader(_ section: GitDiffFileSection) -> some View {
        Button {
            if collapsedFiles.contains(section.file.path) {
                collapsedFiles.remove(section.file.path)
            } else {
                collapsedFiles.insert(section.file.path)
            }
        } label: {
            HStack(spacing: 8) {
                Image(systemName: collapsedFiles.contains(section.file.path)
                      ? "chevron.right" : "chevron.down")
                    .font(monoSmall)
                    .foregroundStyle(.secondary)
                    .frame(width: 10)
                Text(verbatim: section.file.kind.badge)
                    .font(monoSmall.weight(.bold))
                    .foregroundStyle(badgeColor(section.file.kind))
                    .frame(width: 12)
                Text(verbatim: section.file.path)
                    .font(mono)
                    .lineLimit(1)
                    .truncationMode(.middle)
                stats(section)
                Spacer()
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 10)
        .padding(.vertical, 5)
        .background(Color.secondary.opacity(0.08))
        .accessibilityIdentifier("pitex.git.diff.file.\(section.file.path)")
    }

    /// A Group, not a container: the rows stay direct children of the
    /// outer LazyVStack, so an expanded fold of thousands of lines still
    /// renders lazily.
    @ViewBuilder
    private func fileRows(_ section: GitDiffFileSection, items: [GitDiffItem], columnWidth: CGFloat) -> some View {
        if section.binary {
            noteRow("git.diff.binary")
        }
        ForEach(
            Array(items.enumerated()),
            id: \.offset
        ) { _, item in
            diffItem(item, path: section.file.path, columnWidth: columnWidth)
        }
    }

    @ViewBuilder
    private func diffItem(_ item: GitDiffItem, path: String, columnWidth: CGFloat) -> some View {
        switch item {
        case let .pair(left, right):
            pairRow(left: left, right: right, columnWidth: columnWidth)
        case let .fold(id, pairs):
            let key = "\(path)#\(id)"
            if expandedFolds.contains(key) {
                ForEach(Array(pairs.enumerated()), id: \.offset) { _, p in
                    pairRow(left: p.left, right: p.right, columnWidth: columnWidth)
                }
            } else {
                Button {
                    expandedFolds.insert(key)
                } label: {
                    foldedLabel(pairs.count)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("pitex.git.diff.fold")
            }
        case let .gap(oldLines):
            foldedLabel(oldLines)
        case let .note(text):
            Text(verbatim: text)
                .font(monoSmall)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .padding(.horizontal, 10)
                .padding(.vertical, 1)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    /// The "⋯ N unchanged lines" bar spanning both panes — VSCode's
    /// collapsed-region widget look.
    private func foldedLabel(_ count: Int) -> some View {
        HStack(spacing: 6) {
            Image(systemName: "ellipsis")
            Text(String(format: String(localized: "git.folded_lines"), "\(count)"))
                .lineLimit(1)
        }
        .font(monoSmall)
        .foregroundStyle(.secondary)
        .frame(maxWidth: .infinity)
        .padding(.vertical, 4)
        .background(Color.secondary.opacity(0.10))
    }

    private func noteRow(_ key: LocalizedStringKey) -> some View {
        Text(key)
            .font(monoSmall)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 4)
    }

    /// One aligned row: original line on the left, new line on the right.
    /// A paired removed+added line gets its changed character range
    /// shaded darker — VSCode's intra-line highlight.
    private func pairRow(left: GitDiffLine?, right: GitDiffLine?, columnWidth: CGFloat) -> some View {
        HStack(spacing: 0) {
            diffCell(left, other: right, width: columnWidth)
            Rectangle()
                .fill(Color.secondary.opacity(0.25))
                .frame(width: 1)
            diffCell(right, other: left, width: columnWidth)
        }
    }

    private func diffCell(_ line: GitDiffLine?, other: GitDiffLine?, width: CGFloat) -> some View {
        HStack(alignment: .top, spacing: 0) {
            Text(verbatim: line.map { "\($0.number)" } ?? "")
                .foregroundStyle(.secondary)
                .frame(width: 44, alignment: .trailing)
                .padding(.trailing, 8)
            if let line {
                Text(highlighted(line, other: other))
            }
            Spacer(minLength: 0)
        }
        .font(mono)
        .padding(.vertical, 1)
        .frame(width: width, alignment: .leading)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(diffCellBackground(line))
    }

    /// Line text with the changed character range shaded stronger —
    /// `commonAffixes` trims the shared head/tail; the middle is what
    /// actually changed. NSAttributedString because the range math rides
    /// UTF-16, which `commonAffixes`' Character counts convert into here.
    private func highlighted(_ line: GitDiffLine, other: GitDiffLine?) -> AttributedString {
        let text = NSMutableAttributedString(string: line.text)
        guard let other,
              (line.kind == .removed && other.kind == .added)
              || (line.kind == .added && other.kind == .removed)
        else { return AttributedString(text) }
        let (prefix, suffix) = GitSupport.commonAffixes(line.text, other.text)
        let chars = Array(line.text)
        guard chars.count - prefix - suffix > 0 else { return AttributedString(text) }
        let headLen = String(chars[0 ..< prefix]).utf16.count
        let midLen = String(chars[prefix ..< chars.count - suffix]).utf16.count
        text.addAttribute(
            .backgroundColor,
            value: (line.kind == .removed
                    ? NSColor.systemRed.withAlphaComponent(0.38)
                    : NSColor.systemGreen.withAlphaComponent(0.34)),
            range: NSRange(location: headLen, length: midLen))
        return AttributedString(text)
    }

    private func diffCellBackground(_ line: GitDiffLine?) -> Color {
        guard let line else { return Color.secondary.opacity(0.05) }
        switch line.kind {
        case .removed: return .red.opacity(0.16)
        case .added: return .green.opacity(0.16)
        case .context: return .clear
        }
    }

    private func badgeColor(_ kind: GitChangeKind) -> Color {
        switch kind {
        case .modified, .typeChanged: return .orange
        case .added, .untracked: return .green
        case .deleted, .conflicted: return .red
        case .renamed, .copied: return .cyan
        }
    }
}
