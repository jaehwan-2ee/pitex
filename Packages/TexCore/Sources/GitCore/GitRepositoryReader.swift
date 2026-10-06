import Foundation

/// A Git subprocess result, shared by the portable reader and native shells.
public struct GitCommandResult: Sendable {
    public var code: Int32
    public var stdout: String
    public var stderr: String

    public init(code: Int32, stdout: String, stderr: String = "") {
        self.code = code
        self.stdout = stdout
        self.stderr = stderr
    }

    public var errorText: String {
        (stderr.isEmpty ? stdout : stderr).trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

public struct GitRepositorySnapshot: Sendable {
    public let status: GitStatus
    public let branches: [String]
    public let commits: [GitCommit]
}

public enum GitRepositoryReadResult: Sendable {
    case snapshot(GitRepositorySnapshot)
    case failure(GitCommandResult, detectingRepository: Bool)
    case cancelled
}

/// Read-only panel polling. Ref changes invalidate history immediately;
/// unchanged worktree polls reuse its parsing and history for up to a minute.
/// Repository discovery stays fresh so newly nested/deleted repos are detected.
public actor GitRepositoryReader {
    public typealias Runner = @Sendable ([String], String) async -> GitCommandResult

    private var scope: String?
    private var root: String?
    private var statusText: String?
    private var status: GitStatus?
    private var historyKey: String?
    private var historyDate: Date?
    private var commits: [GitCommit] = []
    private var branches: [String] = []

    public init() {}

    public func read(in directory: String, scope: String, now: Date = Date(), runner: Runner) async -> GitRepositoryReadResult {
        guard !Task.isCancelled else { return .cancelled }
        let top = await runner(GitSupport.topLevelArgs, directory)
        guard !Task.isCancelled else { return .cancelled }
        guard top.code == 0 else { return .failure(top, detectingRepository: true) }
        let detectedRoot = top.stdout.trimmingCharacters(in: .whitespacesAndNewlines)
        async let statusResult = runner(GitSupport.statusArgs, detectedRoot)
        async let refsResult = runner(GitSupport.referencesArgs, detectedRoot)
        let (statusOutput, refsOutput) = await (statusResult, refsResult)
        guard !Task.isCancelled else { return .cancelled }
        guard statusOutput.code == 0 else { return .failure(statusOutput, detectingRepository: false) }
        // show-ref exits 1 without stderr in an unborn repository.
        guard refsOutput.code == 0 || (refsOutput.code == 1 && refsOutput.stdout.isEmpty && refsOutput.stderr.isEmpty) else {
            return .failure(refsOutput, detectingRepository: false)
        }
        let sameRepository = self.scope == scope && root == detectedRoot
        let parsedStatus = sameRepository && statusText == statusOutput.stdout
            ? status! : GitSupport.parseStatus(statusOutput.stdout, root: detectedRoot)
        let refs = GitSupport.parseReferences(refsOutput.stdout)
        // The branch distinguishes switching between branches at the same HEAD.
        let key = refs.fingerprint + "\0" + parsedStatus.branch
        let expired = historyDate.map { now.timeIntervalSince($0) >= 60 || now < $0 } ?? true
        var nextCommits = commits
        let reloadHistory = !sameRepository || historyKey != key || expired
        if reloadHistory {
            if refs.fingerprint.isEmpty {
                nextCommits = []
            } else {
                let log = await runner(GitSupport.logArgs(), detectedRoot)
                guard !Task.isCancelled else { return .cancelled }
                guard log.code == 0 else { return .failure(log, detectingRepository: false) }
                nextCommits = GitSupport.parseLog(log.stdout)
            }
        }
        guard !Task.isCancelled else { return .cancelled }
        self.scope = scope
        root = detectedRoot
        statusText = statusOutput.stdout
        status = parsedStatus
        historyKey = key
        if reloadHistory { historyDate = now }
        commits = nextCommits
        branches = refs.branches
        return .snapshot(GitRepositorySnapshot(status: parsedStatus, branches: branches, commits: commits))
    }
}
