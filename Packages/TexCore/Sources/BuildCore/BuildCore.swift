public enum BuildCoreError: Error, Equatable, Sendable {
    case emptyExecutable
    case invalidExecutable
    case invalidArgument(index: Int)
    case invalidEnvironmentKey(String)
    case invalidEnvironmentValue(key: String)
    case invalidWorkingDirectory
    case invalidTransition
    case customShellRequiresAuthority
}

private func validateCommandField(_ value: String, emptyAllowed: Bool) -> Bool {
    (emptyAllowed || !value.isEmpty) && !value.unicodeScalars.contains(where: { $0.value == 0 })
}

public enum WorkingDirectoryPolicy: Hashable, Codable, Sendable {
    case projectRoot
    case sourceDirectory
    case explicit(String)

    public func validated() throws -> Self {
        if case let .explicit(path) = self {
            guard !path.isEmpty, !path.unicodeScalars.contains(where: { $0.value == 0 }) else { throw BuildCoreError.invalidWorkingDirectory }
        }
        return self
    }
}

public enum EnvironmentPolicy: Hashable, Codable, Sendable {
    case inherit(overrides: [String: String])
    case replace([String: String])

    public func validated() throws -> Self {
        let values: [String: String]
        switch self { case let .inherit(v), let .replace(v): values = v }
        for (key, value) in values {
            guard !key.isEmpty, !key.contains("="), !key.unicodeScalars.contains(where: { $0.value == 0 }) else { throw BuildCoreError.invalidEnvironmentKey(key) }
            guard !value.unicodeScalars.contains(where: { $0.value == 0 }) else { throw BuildCoreError.invalidEnvironmentValue(key: key) }
        }
        return self
    }
}

public struct DirectCommandPlan: Hashable, Codable, Sendable {
    public let executable: String
    public let arguments: [String]
    public let workingDirectory: WorkingDirectoryPolicy
    public let environment: EnvironmentPolicy

    public init(executable: String, arguments: [String], workingDirectory: WorkingDirectoryPolicy = .projectRoot, environment: EnvironmentPolicy = .inherit(overrides: [:])) throws {
        guard !executable.isEmpty else { throw BuildCoreError.emptyExecutable }
        guard validateCommandField(executable, emptyAllowed: false) else { throw BuildCoreError.invalidExecutable }
        for (index, argument) in arguments.enumerated() where !validateCommandField(argument, emptyAllowed: true) { throw BuildCoreError.invalidArgument(index: index) }
        self.executable = executable; self.arguments = arguments
        self.workingDirectory = try workingDirectory.validated()
        self.environment = try environment.validated()
    }
}

public struct ShellAuthority: Hashable, Codable, Sendable {
    public let source: Source
    public let approvedByUser: Bool
    public let disclosure: String

    public enum Source: String, Codable, Sendable { case builtIn, projectManifest, userConfiguration }

    public init(source: Source, approvedByUser: Bool, disclosure: String) throws {
        guard source == .builtIn || approvedByUser, !disclosure.isEmpty else { throw BuildCoreError.customShellRequiresAuthority }
        self.source = source; self.approvedByUser = approvedByUser; self.disclosure = disclosure
    }
}

public struct LoginShellCommandPlan: Hashable, Codable, Sendable {
    public let shellExecutable: String
    public let command: String
    public let workingDirectory: WorkingDirectoryPolicy
    public let environment: EnvironmentPolicy
    public let authority: ShellAuthority

    public init(shellExecutable: String, command: String, workingDirectory: WorkingDirectoryPolicy = .projectRoot, environment: EnvironmentPolicy = .inherit(overrides: [:]), authority: ShellAuthority) throws {
        guard validateCommandField(shellExecutable, emptyAllowed: false) else { throw BuildCoreError.invalidExecutable }
        guard validateCommandField(command, emptyAllowed: false) else { throw BuildCoreError.invalidArgument(index: 0) }
        self.shellExecutable = shellExecutable; self.command = command
        self.workingDirectory = try workingDirectory.validated(); self.environment = try environment.validated(); self.authority = authority
    }

    public var invocationArguments: [String] { ["-l", "-c", command] }
}

public enum BuildToolTemplate: String, CaseIterable, Codable, Sendable {
    case latexmk, pdfLaTeX, xeLaTeX, luaLaTeX, bibTeX, biber

    public func plan(input: String) throws -> DirectCommandPlan {
        guard !input.isEmpty, !input.hasPrefix("-"), validateCommandField(input, emptyAllowed: false) else { throw BuildCoreError.invalidArgument(index: 0) }
        switch self {
        case .latexmk: return try DirectCommandPlan(executable: "latexmk", arguments: ["-pdf", "-synctex=1", "-interaction=nonstopmode", "-file-line-error", input])
        case .pdfLaTeX: return try DirectCommandPlan(executable: "pdflatex", arguments: ["-synctex=1", "-interaction=nonstopmode", "-file-line-error", input])
        case .xeLaTeX: return try DirectCommandPlan(executable: "xelatex", arguments: ["-synctex=1", "-interaction=nonstopmode", "-file-line-error", input])
        case .luaLaTeX: return try DirectCommandPlan(executable: "lualatex", arguments: ["-synctex=1", "--interaction=nonstopmode", "--file-line-error", input])
        case .bibTeX: return try DirectCommandPlan(executable: "bibtex", arguments: [input])
        case .biber: return try DirectCommandPlan(executable: "biber", arguments: [input])
        }
    }
}

public struct BuildID: Hashable, Codable, Sendable {
    public let rawValue: String
    public init(rawValue: String) throws {
        guard !rawValue.isEmpty, rawValue.utf8.count <= 128, rawValue.utf8.allSatisfy({ $0 == 45 || $0 == 95 || (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) }) else { throw BuildCoreError.invalidExecutable }
        self.rawValue = rawValue
    }
}

public enum BuildLifecycle: Hashable, Codable, Sendable {
    case queued
    case running(startedAtMilliseconds: UInt64)
    case cancelling
    case succeeded(exitCode: Int32, finishedAtMilliseconds: UInt64)
    case failed(exitCode: Int32?, finishedAtMilliseconds: UInt64)
    case cancelled(finishedAtMilliseconds: UInt64)

    public func transitioning(to next: Self) throws -> Self {
        let allowed: Bool
        switch (self, next) {
        case (.queued, .running), (.queued, .cancelled), (.running, .cancelling), (.running, .succeeded), (.running, .failed), (.cancelling, .cancelled), (.cancelling, .failed): allowed = true
        default: allowed = false
        }
        guard allowed else { throw BuildCoreError.invalidTransition }
        return next
    }
}

public enum BuildLogChannel: String, Codable, Sendable { case standardOutput, standardError, system }
public struct BuildLogEntry: Hashable, Codable, Sendable {
    public let sequence: UInt64
    public let channel: BuildLogChannel
    public let text: String
    public init(sequence: UInt64, channel: BuildLogChannel, text: String) { self.sequence = sequence; self.channel = channel; self.text = text }
}

public enum BuildIssueSeverity: String, Codable, Sendable { case warning, error }
public struct BuildIssue: Hashable, Codable, Sendable {
    public let severity: BuildIssueSeverity
    public let message: String
    public let file: String?
    public let line: Int?
    public init(severity: BuildIssueSeverity, message: String, file: String? = nil, line: Int? = nil) throws {
        guard !message.isEmpty, line.map({ $0 > 0 }) ?? true else { throw BuildCoreError.invalidArgument(index: 0) }
        self.severity = severity; self.message = message; self.file = file; self.line = line
    }
}

public struct BuildCancellation: Hashable, Codable, Sendable {
    public let requestedAtMilliseconds: UInt64
    public let gracePeriodMilliseconds: UInt64
    public init(requestedAtMilliseconds: UInt64, gracePeriodMilliseconds: UInt64) throws {
        guard !requestedAtMilliseconds.addingReportingOverflow(gracePeriodMilliseconds).overflow else { throw BuildCoreError.invalidArgument(index: 0) }
        self.requestedAtMilliseconds = requestedAtMilliseconds
        self.gracePeriodMilliseconds = gracePeriodMilliseconds
    }
    public var forceTerminationAtMilliseconds: UInt64 { requestedAtMilliseconds + gracePeriodMilliseconds }
}

public enum CleanupPolicy: Hashable, Codable, Sendable {
    case preserveAll
    case removeKnownAuxiliaryFiles(Set<String>)
    case removeBuildDirectory
}
