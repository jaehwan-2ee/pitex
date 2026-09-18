import AppPorts
import BuildCore
import DocumentSessionCore

public enum BuildFeatureError: Error, Equatable, Sendable {
    case noCommandSelected
    case customShellAuthorityNotAcknowledged
    case buildAlreadyActive
    case buildIdentityMismatch
}

public struct BuildDocumentRevision: Hashable, Codable, Sendable {
    public let documentID: String
    public let revision: UInt64
    public let contentHash: DiskContentHash

    public init(snapshot: DocumentSessionCore.DocumentSnapshot) {
        self.documentID = snapshot.documentID.rawValue
        self.revision = snapshot.revision
        self.contentHash = snapshot.contentHash
    }
}

public struct PendingCustomShellCommand: Hashable, Codable, Sendable {
    public let shellExecutable: String
    public let command: String
    public let workingDirectory: WorkingDirectoryPolicy
    public let environment: EnvironmentPolicy
    public let source: ShellAuthority.Source
    public let disclosure: String

    public init(
        shellExecutable: String,
        command: String,
        workingDirectory: WorkingDirectoryPolicy = .projectRoot,
        environment: EnvironmentPolicy = .inherit(overrides: [:]),
        source: ShellAuthority.Source,
        disclosure: String
    ) {
        self.shellExecutable = shellExecutable
        self.command = command
        self.workingDirectory = workingDirectory
        self.environment = environment
        self.source = source
        self.disclosure = disclosure
    }

    fileprivate func acknowledgingAuthority() throws -> LoginShellCommandPlan {
        let authority = try ShellAuthority(
            source: source,
            approvedByUser: true,
            disclosure: disclosure
        )
        return try LoginShellCommandPlan(
            shellExecutable: shellExecutable,
            command: command,
            workingDirectory: workingDirectory,
            environment: environment,
            authority: authority
        )
    }
}

public enum BuildCommandSelection: Hashable, Codable, Sendable {
    case builtIn(BuildToolTemplate)
    case direct(DirectCommandPlan)
    case customShellAwaitingAcknowledgement(PendingCustomShellCommand)
}

public enum ResolvedBuildCommand: Hashable, Codable, Sendable {
    case direct(DirectCommandPlan)
    case loginShell(LoginShellCommandPlan)
}

public enum BuildRunState: Hashable, Codable, Sendable {
    case idle
    case active(id: BuildID, lifecycle: BuildLifecycle, cancellation: BuildCancellation?)
}

public struct BuildFeatureState: Sendable {
    public private(set) var selection: BuildCommandSelection?
    public private(set) var authorizedCustomShell: LoginShellCommandPlan?
    public private(set) var run: BuildRunState
    public private(set) var inputRevision: BuildDocumentRevision?
    public private(set) var log: [BuildLogEntry]
    public private(set) var issues: [BuildIssue]

    public init() {
        self.selection = nil
        self.authorizedCustomShell = nil
        self.run = .idle
        self.inputRevision = nil
        self.log = []
        self.issues = []
    }

    public mutating func select(_ selection: BuildCommandSelection) {
        self.selection = selection
        authorizedCustomShell = nil
    }

    public mutating func acknowledgeCustomShellAuthority() throws {
        guard case let .customShellAwaitingAcknowledgement(pending) = selection else {
            throw BuildFeatureError.customShellAuthorityNotAcknowledged
        }
        authorizedCustomShell = try pending.acknowledgingAuthority()
    }

    public func resolvedCommand(input: String) throws -> ResolvedBuildCommand {
        guard let selection else { throw BuildFeatureError.noCommandSelected }
        switch selection {
        case let .builtIn(template):
            return .direct(try template.plan(input: input))
        case let .direct(plan):
            return .direct(plan)
        case .customShellAwaitingAcknowledgement:
            guard let plan = authorizedCustomShell else {
                throw BuildFeatureError.customShellAuthorityNotAcknowledged
            }
            return .loginShell(plan)
        }
    }

    public mutating func queue(buildID: BuildID, input: DocumentSessionCore.DocumentSnapshot) throws {
        guard case .idle = run else { throw BuildFeatureError.buildAlreadyActive }
        _ = try resolvedCommand(input: input.path.rawValue)
        run = .active(id: buildID, lifecycle: .queued, cancellation: nil)
        inputRevision = BuildDocumentRevision(snapshot: input)
        log.removeAll(keepingCapacity: true)
        issues.removeAll(keepingCapacity: true)
    }

    public mutating func transition(buildID: BuildID, to lifecycle: BuildLifecycle) throws {
        guard case let .active(currentID, current, cancellation) = run,
              currentID == buildID else {
            throw BuildFeatureError.buildIdentityMismatch
        }
        run = .active(
            id: currentID,
            lifecycle: try current.transitioning(to: lifecycle),
            cancellation: cancellation
        )
    }

    public mutating func requestCancellation(
        buildID: BuildID,
        cancellation: BuildCancellation
    ) throws {
        guard case let .active(currentID, lifecycle, _) = run,
              currentID == buildID else {
            throw BuildFeatureError.buildIdentityMismatch
        }
        run = .active(
            id: currentID,
            lifecycle: try lifecycle.transitioning(to: .cancelling),
            cancellation: cancellation
        )
    }

    public mutating func recordLog(_ entry: BuildLogEntry) {
        log.removeAll { $0.sequence == entry.sequence }
        log.append(entry)
        log.sort { $0.sequence < $1.sequence }
    }

    public mutating func replaceIssues(_ issues: [BuildIssue]) {
        self.issues = issues.sorted {
            ($0.file ?? "", $0.line ?? 0, $0.message) < ($1.file ?? "", $1.line ?? 0, $1.message)
        }
    }

    public mutating func resetAfterCompletion() {
        guard case let .active(_, lifecycle, _) = run else { return }
        switch lifecycle {
        case .succeeded, .failed, .cancelled:
            run = .idle
        case .queued, .running, .cancelling:
            break
        }
    }
}
