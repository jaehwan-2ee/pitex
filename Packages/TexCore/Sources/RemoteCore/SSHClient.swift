import Foundation
#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// A saved device the user can open folders on — an `~/.ssh/config` alias
/// (ssh resolves HostName/User/Port/keys itself) or a manually entered host.
public struct SSHConnection: Codable, Hashable, Sendable, Identifiable {
    public var id: UUID
    /// Shown in pickers ("Mac mini").
    public var name: String
    /// The ssh destination: a config alias or a host name / address.
    public var destination: String
    public var user: String?
    public var port: Int?
    /// Private key for manually added hosts (config aliases carry their own).
    public var identityFile: String?

    public init(
        id: UUID = UUID(),
        name: String,
        destination: String,
        user: String? = nil,
        port: Int? = nil,
        identityFile: String? = nil
    ) {
        self.id = id
        self.name = name
        self.destination = destination
        self.user = user
        self.port = port
        self.identityFile = identityFile
    }

    public init(configHost entry: SSHHostEntry) {
        self.init(name: entry.alias, destination: entry.alias)
    }

    /// Rejects values that ssh could read as options or that would not
    /// survive argv intact: no leading `-`, no whitespace or control
    /// characters, ports in range.
    public var validationError: String? {
        func unsafe(_ value: String) -> Bool {
            value.hasPrefix("-") || value.unicodeScalars.contains {
                CharacterSet.whitespacesAndNewlines.contains($0) || CharacterSet.controlCharacters.contains($0)
            }
        }
        if destination.isEmpty { return "Enter a host." }
        if unsafe(destination) { return "The host may not start with '-' or contain spaces." }
        if let user, !user.isEmpty, unsafe(user) || user.contains("@") {
            return "The user name may not start with '-' or contain spaces or '@'."
        }
        if let port, !(1...65535).contains(port) { return "The port must be between 1 and 65535." }
        if let identityFile, identityFile.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) {
            return "The key path contains control characters."
        }
        return nil
    }

    /// ssh options this connection adds before the destination.
    var connectionArguments: [String] {
        var arguments: [String] = []
        if let user, !user.isEmpty { arguments += ["-l", user] }
        if let port { arguments += ["-p", String(port)] }
        if let identityFile, !identityFile.isEmpty {
            arguments += ["-i", (identityFile as NSString).expandingTildeInPath, "-o", "IdentitiesOnly=yes"]
        }
        return arguments
    }
}

public enum SSHError: Error, Equatable, LocalizedError, Sendable {
    /// ssh itself failed (exit 255): unreachable, auth, host key…
    case connection(String)
    /// The remote command failed.
    case remote(status: Int32, message: String)
    case invalidConnection(String)
    case cancelled

    public var errorDescription: String? {
        switch self {
        case let .connection(message):
            return "Could not connect over SSH: \(message)"
        case let .remote(status, message):
            return message.isEmpty ? "The remote command failed (exit \(status))." : message
        case let .invalidConnection(message):
            return message
        case .cancelled:
            return "Cancelled."
        }
    }
}

public struct SSHCommandResult: Sendable {
    public let status: Int32
    public let standardOutput: Data
    public let standardError: Data

    public var stdoutText: String { String(decoding: standardOutput, as: UTF8.self) }
    public var stderrText: String { String(decoding: standardError, as: UTF8.self) }
}

/// Runs scripts on one connection through the system `ssh`. Every call is
/// non-interactive (`BatchMode`: key/agent/Keychain auth only, never a
/// password or host-key prompt that would hang the app), keeps strict host
/// key checking, and reuses one multiplexed connection per host.
///
/// Scripts always run as `/bin/sh -c SCRIPT pitex ARGS…` on the remote:
/// paths travel as positional parameters, never spliced into shell text,
/// whatever the user's login shell is.
public struct SSHClient: Sendable {
    public let connection: SSHConnection
    public var sshExecutable: URL
    /// Directory for ControlMaster sockets (nil disables multiplexing).
    public var controlDirectory: URL?
    /// Extra `-o`/`-i` arguments (tests point at a private sshd).
    public var extraArguments: [String]

    public init(
        connection: SSHConnection,
        sshExecutable: URL = URL(fileURLWithPath: "/usr/bin/ssh"),
        controlDirectory: URL? = SSHClient.defaultControlDirectory,
        extraArguments: [String] = []
    ) {
        self.connection = connection
        self.sshExecutable = sshExecutable
        self.controlDirectory = controlDirectory
        self.extraArguments = extraArguments
    }

    /// `/tmp/pitex-ssh-<uid>`. The per-user temporary folder on macOS
    /// (`/var/folders/…/T/`) is too long: a socket path is this directory,
    /// `/`, the 40-hex `%C` and ssh's 17-character temporary suffix, and
    /// macOS caps it at 104 bytes.
    public static var defaultControlDirectory: URL? {
        URL(fileURLWithPath: "/tmp/pitex-ssh-\(getuid())", isDirectory: true)
    }

    /// `directory` when multiplexing through it is safe: short enough for
    /// the socket path, and a real directory owned by this user with no
    /// group or other access — in shared /tmp anything else could let
    /// another user plant a control socket. Created on first use; nil
    /// means plain connections.
    static func usableControlDirectory(_ directory: URL) -> URL? {
        guard directory.path.utf8.count + 1 + 40 + 17 < 104 else { return nil }
        let fileManager = FileManager.default
        try? fileManager.createDirectory(at: directory, withIntermediateDirectories: false,
                                         attributes: [.posixPermissions: 0o700])
        // attributesOfItem describes a symlink itself, not its target.
        guard let attributes = try? fileManager.attributesOfItem(atPath: directory.path),
              attributes[.type] as? FileAttributeType == .typeDirectory,
              (attributes[.ownerAccountID] as? NSNumber)?.uint32Value == getuid(),
              let mode = (attributes[.posixPermissions] as? NSNumber)?.intValue, mode & 0o077 == 0 else { return nil }
        return directory
    }

    /// Full ssh argv for `script` with positional `arguments`.
    func arguments(script: String, arguments scriptArguments: [String], tty: Bool) throws -> [String] {
        if let problem = connection.validationError { throw SSHError.invalidConnection(problem) }
        var argv = ["-o", "BatchMode=yes", "-o", "ConnectTimeout=15", "-o", "ServerAliveInterval=15"]
        if let controlDirectory, let directory = Self.usableControlDirectory(controlDirectory) {
            argv += ["-o", "ControlMaster=auto",
                     "-o", "ControlPath=\(directory.path)/%C",
                     "-o", "ControlPersist=300"]
        }
        argv += tty ? ["-tt"] : ["-T"]
        argv += extraArguments
        argv += connection.connectionArguments
        // The remote login shell parses this one string; everything inside
        // is single-quoted, so only /bin/sh interprets the script.
        let remote = (["/bin/sh", "-c", script, "pitex"] + scriptArguments).map(Self.quote).joined(separator: " ")
        argv += ["--", connection.destination, remote]
        return argv
    }

    /// POSIX single-quoting: `'` → `'\''`.
    public static func quote(_ value: String) -> String {
        "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }

    /// Runs `script` to completion; `input` is written to its stdin, and
    /// stdout goes to `outputFile` instead of memory when given (tar
    /// streams can be large).
    public func run(
        _ script: String,
        arguments: [String] = [],
        input: Data? = nil,
        outputFile: URL? = nil
    ) async throws -> SSHCommandResult {
        let argv = try self.arguments(script: script, arguments: arguments, tty: false)
        let result = try await ProcessPipe.run(executable: sshExecutable, arguments: argv,
                                               input: input, outputFile: outputFile)
        if result.status == 255 {
            throw SSHError.connection(Self.firstLine(result.stderrText) ?? "ssh exited with status 255")
        }
        return result
    }

    /// Like `run`, but a non-zero exit is an error carrying stderr.
    @discardableResult
    public func runChecked(
        _ script: String,
        arguments: [String] = [],
        input: Data? = nil,
        outputFile: URL? = nil
    ) async throws -> SSHCommandResult {
        let result = try await run(script, arguments: arguments, input: input, outputFile: outputFile)
        guard result.status == 0 else {
            throw SSHError.remote(status: result.status, message: Self.firstLine(result.stderrText) ?? "")
        }
        return result
    }

    /// Streams stdout/stderr chunks as they arrive (builds). With `tty`,
    /// the remote runs under a pseudo-terminal so it gets SIGHUP when the
    /// local ssh is terminated — cancelling a build stops it remotely too.
    public func stream(
        _ script: String,
        arguments: [String] = [],
        tty: Bool,
        output: @escaping @Sendable (Data, Bool) async -> Void
    ) async throws -> Int32 {
        let argv = try self.arguments(script: script, arguments: arguments, tty: tty)
        let status = try await ProcessPipe.stream(executable: sshExecutable, arguments: argv, output: output)
        return status
    }

    /// Verifies the connection end to end; returns the remote home path.
    public func check() async throws -> String {
        let result = try await runChecked("cd && pwd -P")
        return result.stdoutText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func firstLine(_ text: String) -> String? {
        text.split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .first { !$0.isEmpty }
    }
}
