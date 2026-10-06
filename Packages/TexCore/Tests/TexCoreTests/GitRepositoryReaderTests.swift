import BuildCore
import Foundation
import GitCore
import XCTest

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

private actor GitTestCommands {
    private var calls: [[String]] = []
    var failNextLog = false

    func run(_ args: [String], in directory: String) async -> GitCommandResult {
        calls.append(args)
        if failNextLog, args.first == "log" {
            failNextLog = false
            return GitCommandResult(code: 128, stdout: "", stderr: "injected log failure")
        }
        do {
            let result = try await ProcessRunner().run(
                DirectCommandPlan(executable: "/usr/bin/git", arguments: args,
                                  environment: .replace(["PATH": "/usr/bin:/bin", "GIT_CONFIG_GLOBAL": "/dev/null",
                                                         "GIT_CONFIG_NOSYSTEM": "1", "GIT_TERMINAL_PROMPT": "0"])),
                projectRoot: URL(fileURLWithPath: directory), timeout: .seconds(15))
            let code: Int32
            switch result.termination {
            case let .exited(value): code = value
            case let .signaled(signal): code = -signal
            }
            return GitCommandResult(code: code, stdout: String(decoding: result.standardOutput, as: UTF8.self),
                                    stderr: String(decoding: result.standardError, as: UTF8.self))
        } catch {
            return GitCommandResult(code: -1, stdout: "", stderr: error.localizedDescription)
        }
    }

    func checked(_ args: [String], in root: URL) async throws {
        let result = await run(args, in: root.path)
        guard result.code == 0 else { throw NSError(domain: "GitTest", code: Int(result.code), userInfo: [NSLocalizedDescriptionKey: result.errorText]) }
    }
    func clear() { calls.removeAll() }
    func recorded() -> [[String]] { calls }
    func injectLogFailure() { failNextLog = true }
}

private actor GitReadGate {
    var started = false
    private var continuation: CheckedContinuation<Void, Never>?
    func hold() async {
        started = true
        await withCheckedContinuation { continuation = $0 }
    }
    func release() { continuation?.resume(); continuation = nil }
}

final class GitRepositoryReaderTests: XCTestCase {
    private let epoch = Date(timeIntervalSince1970: 1_000)

    private func temporaryDirectory() throws -> URL {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pitex-git-reader-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
        return root
    }
    private func physicalPath(_ path: String) throws -> String {
        // Compare physical paths as strings: Foundation can present /var
        // again after resolving macOS's /private/var temporary-directory alias.
        guard let resolved = realpath(path, nil) else {
            throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EINVAL)
        }
        defer { free(resolved) }
        return String(cString: resolved)
    }
    private func initialize(_ commands: GitTestCommands, root: URL, commit: Bool = true) async throws {
        try await commands.checked(["init", "-b", "main"], in: root)
        try await commands.checked(["config", "user.name", "Pitex Test"], in: root)
        try await commands.checked(["config", "user.email", "test@example.invalid"], in: root)
        if commit { try await commands.checked(["commit", "--allow-empty", "-m", "Initial"], in: root) }
        await commands.clear()
    }
    private func read(_ reader: GitRepositoryReader, commands: GitTestCommands, root: URL,
                      scope: String = "project", elapsed: TimeInterval = 0) async throws -> GitRepositorySnapshot {
        let result = await reader.read(in: root.path, scope: scope, now: epoch.addingTimeInterval(elapsed)) { args, path in
            await commands.run(args, in: path)
        }
        guard case let .snapshot(snapshot) = result else {
            if case let .failure(output, _) = result { throw NSError(domain: "GitRead", code: Int(output.code), userInfo: [NSLocalizedDescriptionKey: output.errorText]) }
            throw NSError(domain: "GitRead", code: -1)
        }
        return snapshot
    }

    func testNonRepositoryPollingNeverInitializesOrMutatesFiles() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        try "draft".write(to: root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let commands = GitTestCommands()
        let reader = GitRepositoryReader()
        for _ in 0..<3 {
            let result = await reader.read(in: root.path, scope: "nonrepo") { args, path in await commands.run(args, in: path) }
            guard case let .failure(output, detectingRepository) = result else { return XCTFail("nonrepo must remain nonrepo") }
            XCTAssertNotEqual(output.code, 0)
            XCTAssertTrue(detectingRepository)
        }
        let calls = await commands.recorded()
        XCTAssertEqual(calls, Array(repeating: GitSupport.topLevelArgs, count: 3))
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: root.path), ["main.tex"])
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("main.tex"), encoding: .utf8), "draft")
    }

    func testWarmRefreshUsesThreeCommandsAndOnlyRefsReloadHistory() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let commands = GitTestCommands()
        try await initialize(commands, root: root)
        let reader = GitRepositoryReader()
        let cold = try await read(reader, commands: commands, root: root)
        var calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
        XCTAssertEqual(cold.branches, ["main"])
        await commands.clear()
        try "new draft".write(to: root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let warm = try await read(reader, commands: commands, root: root, elapsed: 4)
        calls = await commands.recorded()
        XCTAssertEqual(calls.count, 3)
        XCTAssertFalse(calls.contains { $0.first == "log" })
        XCTAssertEqual(warm.commits, cold.commits)
        XCTAssertEqual(warm.status.unstaged.map(\.path), ["main.tex"])
        try await commands.checked(["add", "--all"], in: root)
        try await commands.checked(["commit", "-m", "Saved"], in: root)
        await commands.clear()
        let changed = try await read(reader, commands: commands, root: root, elapsed: 8)
        calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
        XCTAssertEqual(changed.commits.first?.subject, "Saved")
        XCTAssertTrue(changed.status.unstaged.isEmpty)
    }

    func testBranchSwitchAtSameHEADAndDetachedHEADInvalidateHistory() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let commands = GitTestCommands()
        try await initialize(commands, root: root)
        try await commands.checked(["branch", "topic"], in: root)
        let reader = GitRepositoryReader()
        _ = try await read(reader, commands: commands, root: root)
        try await commands.checked(["switch", "topic"], in: root)
        await commands.clear()
        let switched = try await read(reader, commands: commands, root: root, elapsed: 1)
        var calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
        XCTAssertEqual(switched.status.branch, "topic")
        XCTAssertTrue(switched.commits.first?.refs.contains("topic") == true)
        try await commands.checked(["commit", "--allow-empty", "-m", "Detached candidate"], in: root)
        try await commands.checked(["switch", "--detach", "HEAD~1"], in: root)
        _ = try await read(reader, commands: commands, root: root, elapsed: 2)
        try await commands.checked(["switch", "--detach", "topic"], in: root)
        await commands.clear()
        let detached = try await read(reader, commands: commands, root: root, elapsed: 3)
        calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
        XCTAssertEqual(detached.status.branch, "HEAD")
        XCTAssertEqual(detached.commits.first?.subject, "Detached candidate")
        XCTAssertTrue(detached.commits.first?.isHead == true)
    }

    func testNotesDoNotReloadHistoryButTagsAndRelativeDateExpiryDo() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let commands = GitTestCommands()
        try await initialize(commands, root: root)
        let reader = GitRepositoryReader()
        _ = try await read(reader, commands: commands, root: root)
        try await commands.checked(["notes", "--ref=ai", "add", "-m", "generated attribution"], in: root)
        await commands.clear()
        _ = try await read(reader, commands: commands, root: root, elapsed: 4)
        var calls = await commands.recorded()
        XCTAssertEqual(calls.count, 3)
        try await commands.checked(["tag", "v1"], in: root)
        await commands.clear()
        let tagged = try await read(reader, commands: commands, root: root, elapsed: 8)
        calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
        XCTAssertTrue(tagged.commits.first?.refs.contains("tag: v1") == true)
        await commands.clear()
        _ = try await read(reader, commands: commands, root: root, elapsed: 68)
        calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
    }

    func testUnbornRepositoryAndExplicitInitializationStayAtProjectRoot() async throws {
        let outer = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: outer) }
        let root = outer.appendingPathComponent("project", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
        let commands = GitTestCommands()
        let reader = GitRepositoryReader()
        let before = await reader.read(in: root.path, scope: "project") { args, path in await commands.run(args, in: path) }
        guard case .failure = before else { return XCTFail("must detect nonrepo before explicit init") }
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent(".git").path))
        try await initialize(commands, root: root, commit: false)
        let unborn = try await read(reader, commands: commands, root: root)
        let calls = await commands.recorded()
        XCTAssertEqual(calls.count, 3)
        XCTAssertEqual(try physicalPath(unborn.status.root), try physicalPath(root.path))
        XCTAssertEqual(unborn.status.branch, "main")
        XCTAssertTrue(unborn.commits.isEmpty)
        XCTAssertFalse(FileManager.default.fileExists(atPath: outer.appendingPathComponent(".git").path))
    }

    func testAncestorNestedCreationRemovalAndSamePathReopenAreFresh() async throws {
        let outer = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: outer) }
        let project = outer.appendingPathComponent("nested", isDirectory: true)
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: false)
        let commands = GitTestCommands()
        try await initialize(commands, root: outer)
        let reader = GitRepositoryReader()
        let ancestor = try await read(reader, commands: commands, root: project)
        XCTAssertEqual(try physicalPath(ancestor.status.root), try physicalPath(outer.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: project.appendingPathComponent(".git").path))
        try await initialize(commands, root: project)
        let nested = try await read(reader, commands: commands, root: project, elapsed: 1)
        XCTAssertEqual(try physicalPath(nested.status.root), try physicalPath(project.path))
        try FileManager.default.removeItem(at: project.appendingPathComponent(".git"))
        let parentAgain = try await read(reader, commands: commands, root: project, elapsed: 2)
        XCTAssertEqual(try physicalPath(parentAgain.status.root), try physicalPath(outer.path))
        await commands.clear()
        _ = try await read(reader, commands: commands, root: project, scope: "reopened", elapsed: 3)
        let calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
        try FileManager.default.removeItem(at: outer.appendingPathComponent(".git"))
        let removed = await reader.read(in: project.path, scope: "reopened") { args, path in await commands.run(args, in: path) }
        guard case .failure = removed else { return XCTFail("removed repo must not return cached status") }
    }

    func testUnbornOrphanBranchStillReadsOtherRefsHistory() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let commands = GitTestCommands()
        try await initialize(commands, root: root)
        try await commands.checked(["switch", "--orphan", "draft"], in: root)
        await commands.clear()
        let snapshot = try await read(GitRepositoryReader(), commands: commands, root: root)
        XCTAssertEqual(snapshot.status.branch, "draft")
        XCTAssertEqual(snapshot.branches, ["main"])
        XCTAssertEqual(snapshot.commits.map(\.subject), ["Initial"])
        XCTAssertFalse(snapshot.commits.first?.isHead == true)
    }

    func testLinkedWorktreeDiscoveryAndControlCharacterCommitPaths() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let repo = root.appendingPathComponent("repo", isDirectory: true)
        let worktree = root.appendingPathComponent("worktree", isDirectory: true)
        try FileManager.default.createDirectory(at: repo, withIntermediateDirectories: false)
        let commands = GitTestCommands()
        try await initialize(commands, root: repo)
        try await commands.checked(["config", "core.quotePath", "true"], in: repo)
        try await commands.checked(["worktree", "add", "-b", "other", worktree.path], in: repo)
        let path = "한글\tname\n.tex"
        let spacedPath = "한글 name.tex"
        for name in [path, spacedPath] {
            try "contents\n".write(to: worktree.appendingPathComponent(name), atomically: true, encoding: .utf8)
        }
        try await commands.checked(["add", "--all"], in: worktree)
        try await commands.checked(["commit", "-m", "Unusual path"], in: worktree)
        let reader = GitRepositoryReader()
        let snapshot = try await read(reader, commands: commands, root: worktree)
        XCTAssertEqual(try physicalPath(snapshot.status.root), try physicalPath(worktree.path))
        XCTAssertEqual(snapshot.status.branch, "other")
        let hash = try XCTUnwrap(snapshot.commits.first?.fullHash)
        let names = await commands.run(GitSupport.commitFilesArgs(hash), in: worktree.path)
        let expectedFiles = [path, spacedPath].sorted().map { GitCommitFile(path: $0, kind: .added) }
        XCTAssertEqual(GitSupport.parseCommitFiles(names.stdout), expectedFiles)
        let patch = await commands.run(GitSupport.commitDiffArgs(hash), in: worktree.path)
        XCTAssertEqual(GitSupport.parseCommitDiffSections(patch.stdout).map(\.file), expectedFiles)
    }

    func testFailedHistoryIsRetriedAndCancellationCannotPopulateCache() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let commands = GitTestCommands()
        try await initialize(commands, root: root)
        let reader = GitRepositoryReader()
        await commands.injectLogFailure()
        let failure = await reader.read(in: root.path, scope: "project") { args, path in await commands.run(args, in: path) }
        guard case let .failure(output, detectingRepository) = failure else { return XCTFail("log failure must surface") }
        XCTAssertFalse(detectingRepository)
        XCTAssertEqual(output.errorText, "injected log failure")
        await commands.clear()
        _ = try await read(reader, commands: commands, root: root)
        var calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
        let gate = GitReadGate()
        let directory = root.path
        let task = Task {
            await reader.read(in: directory, scope: "new-project") { args, path in
                if args == GitSupport.topLevelArgs { await gate.hold() }
                return await commands.run(args, in: path)
            }
        }
        while !(await gate.started) { try await Task.sleep(for: .milliseconds(1)) }
        task.cancel()
        await gate.release()
        let cancelled = await task.value
        guard case .cancelled = cancelled else { return XCTFail("cancelled result must be discarded") }
        await commands.clear()
        _ = try await read(reader, commands: commands, root: root, scope: "new-project")
        calls = await commands.recorded()
        XCTAssertEqual(calls.count, 4)
    }

    /// Runtime is reported rather than asserted: filesystem/process timing
    /// varies by host. The 25% warm process reduction is deterministic.
    func testBenchmarkRealRepositoryPollingCommandCountAndRuntime() async throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let commands = GitTestCommands()
        try await initialize(commands, root: root)
        for index in 0..<250 {
            try String(repeating: "fixture line\n", count: 40).write(to: root.appendingPathComponent("file-\(index).tex"), atomically: true, encoding: .utf8)
        }
        try await commands.checked(["add", "--all"], in: root)
        try await commands.checked(["commit", "-m", "Fixture"], in: root)
        for index in 0..<20 { try await commands.checked(["commit", "--allow-empty", "-m", "History \(index)"], in: root) }
        let reader = GitRepositoryReader()
        _ = try await read(reader, commands: commands, root: root)
        let polls = 8
        await commands.clear()
        let baselineStart = ContinuousClock.now
        for _ in 0..<polls {
            let top = await commands.run(GitSupport.topLevelArgs, in: root.path)
            XCTAssertEqual(top.code, 0)
            async let status = commands.run(GitSupport.statusArgs, in: root.path)
            async let branches = commands.run(GitSupport.branchArgs, in: root.path)
            async let log = commands.run(GitSupport.logArgs(), in: root.path)
            let outputs = await (status, branches, log)
            XCTAssertEqual(outputs.0.code, 0)
            XCTAssertEqual(outputs.1.code, 0)
            XCTAssertEqual(outputs.2.code, 0)
            _ = GitSupport.parseStatus(outputs.0.stdout, root: root.path)
            _ = GitSupport.parseBranches(outputs.1.stdout)
            _ = GitSupport.parseLog(outputs.2.stdout)
        }
        let baselineTime = baselineStart.duration(to: .now)
        let baselineCalls = await commands.recorded().count
        await commands.clear()
        let optimizedStart = ContinuousClock.now
        for index in 0..<polls { _ = try await read(reader, commands: commands, root: root, elapsed: Double(index + 1)) }
        let optimizedTime = optimizedStart.duration(to: .now)
        let optimizedCalls = await commands.recorded().count
        XCTAssertEqual(baselineCalls, polls * 4)
        XCTAssertEqual(optimizedCalls, polls * 3)
        print("GIT_REFRESH_BENCHMARK polls=\(polls) files=250 commits=22 baseline_commands=\(baselineCalls) optimized_commands=\(optimizedCalls) baseline=\(baselineTime) optimized=\(optimizedTime)")
    }
}
