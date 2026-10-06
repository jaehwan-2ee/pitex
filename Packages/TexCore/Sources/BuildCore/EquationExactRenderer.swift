import Foundation
import TexDomain

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// The optional exact equation preview: one minimal document compiled with
/// the project's own TeX engine and shell-escape policy, in a private
/// per-request temporary directory. Never a full-project build, never a
/// shell, never on every keystroke (the preview engine decides when).
public enum ExactEquationEngine: String, Hashable, Codable, Sendable {
    case pdfLaTeX = "pdflatex"
    case xeLaTeX = "xelatex"
    case luaLaTeX = "lualatex"
}

public enum ExactEquationShellEscape: String, Hashable, Codable, Sendable {
    /// No flag in the build command — the TeX distribution's default applies
    /// to both the build and the preview.
    case projectDefault
    case enabled
    case restricted
    case disabled
}

public struct ExactEquationProfile: Hashable, Codable, Sendable {
    public let engine: ExactEquationEngine
    public let shellEscape: ExactEquationShellEscape

    public init(engine: ExactEquationEngine, shellEscape: ExactEquationShellEscape) {
        self.engine = engine
        self.shellEscape = shellEscape
    }

    /// Cache/identity string for the preview engine.
    public var identity: String { engine.rawValue + "|" + shellEscape.rawValue }

    /// The engine and shell-escape policy the project's build command uses,
    /// or nil when it is not a plain pdflatex/xelatex/lualatex/latexmk
    /// invocation (custom scripts, tectonic, chained shell commands, a
    /// latexmk whose engine selection depends on user config, or quoting
    /// whitespace-splitting cannot safely tokenize) — the preview never
    /// substitutes a different engine.
    public static func resolve(buildCommand: String) -> ExactEquationProfile? {
        let shellSyntax = ["&&", "||", ";", "|", "`", "$(", ">", "<", "\n"]
        guard !shellSyntax.contains(where: buildCommand.contains) else { return nil }
        let tokens = buildCommand.split(whereSeparator: { $0 == " " || $0 == "\t" }).map(String.init)
        // Quotes mean real shell tokenization that whitespace-splitting
        // would misread (-pdflatex="lualatex -shell-escape" would leak a fake
        // -shell-escape flag); refuse rather than substitute.
        guard !tokens.contains(where: { $0.contains("\"") || $0.contains("'") }) else { return nil }
        guard let first = tokens.first else { return nil }
        let program = String(first.split(separator: "/").last ?? Substring(first))
        let flags = tokens.dropFirst().map { $0.hasPrefix("--") ? String($0.dropFirst()) : $0 }

        let engine: ExactEquationEngine
        switch program {
        case "pdflatex": engine = .pdfLaTeX
        case "xelatex": engine = .xeLaTeX
        case "lualatex": engine = .luaLaTeX
        case "latexmk":
            var selected: ExactEquationEngine? = nil
            for flag in flags {
                let picked: ExactEquationEngine?
                switch flag {
                case "-xelatex", "-pdfxe": picked = .xeLaTeX
                case "-lualatex", "-pdflua": picked = .luaLaTeX
                case "-pdf": picked = .pdfLaTeX // explicit pdf-via-pdflatex
                case _ where flag.hasPrefix("-ps") || flag.hasPrefix("-dvi")
                    || flag.hasPrefix("-pdfps") || flag.hasPrefix("-pdfdvi"):
                    // Non-pdflatex latexmk routes are out of scope.
                    return nil
                case _ where flag.hasPrefix("-pdflatex="):
                    switch String(flag.dropFirst("-pdflatex=".count)).trimmingCharacters(in: .whitespaces) {
                    case "pdflatex": picked = .pdfLaTeX
                    case "xelatex": picked = .xeLaTeX
                    case "lualatex": picked = .luaLaTeX
                    // Anything else is a program string we cannot safely
                    // interpret.
                    default: return nil
                    }
                default: picked = nil
                }
                if let picked {
                    // Conflicting explicit selections are ambiguous.
                    if let selected, selected != picked { return nil }
                    selected = picked
                }
            }
            // Bare latexmk reads its engine from user config ($pdf_mode):
            // never guess it.
            guard let selected else { return nil }
            engine = selected
        default:
            return nil
        }

        var shellEscape = ExactEquationShellEscape.projectDefault
        for flag in flags {
            switch flag {
            case "-shell-escape", "-enable-write18": shellEscape = .enabled
            case "-no-shell-escape", "-disable-write18": shellEscape = .disabled
            case "-shell-restricted": shellEscape = .restricted
            default: break
            }
        }
        return ExactEquationProfile(engine: engine, shellEscape: shellEscape)
    }

    static let jobName = "pitex-equation"

    /// Fixed argv — nothing in it comes from the document text.
    public func arguments(outputDirectory: String) -> [String] {
        let dash = engine == .luaLaTeX ? "--" : "-"
        var arguments = [
            dash + "interaction=batchmode",
            dash + "halt-on-error",
            dash + "output-directory=" + outputDirectory,
            dash + "jobname=" + Self.jobName,
        ]
        switch shellEscape {
        case .projectDefault: break
        case .enabled: arguments.append(dash + "shell-escape")
        case .restricted: arguments.append(dash + "shell-restricted")
        case .disabled: arguments.append(dash + "no-shell-escape")
        }
        arguments.append(Self.jobName + ".tex")
        return arguments
    }
}

public enum ExactEquationRenderError: Error, Equatable, Sendable {
    case compileFailed(log: String)
    case timedOut
    case cancelled
    case outputTooLarge
    case temporaryDirectoryFailed
}

public struct ExactEquationRenderer: Sendable {
    public static let defaultTimeout: Duration = .seconds(20)
    public static let maximumPDFBytes = 4 * 1024 * 1024

    public let profile: ExactEquationProfile
    /// The main document's directory: `\input`/`\usepackage` of project
    /// files in the preamble resolve through TEXINPUTS, never by running
    /// inside the project.
    public let mainDirectory: URL
    /// Host additions (PATH to the TeX distribution).
    public let environment: [String: String]
    /// Opaque per-workspace token keeping concurrent windows apart.
    public let workspaceToken: String
    public let timeout: Duration

    public init(profile: ExactEquationProfile, mainDirectory: URL, environment: [String: String] = [:], workspaceToken: String, timeout: Duration = defaultTimeout) {
        self.profile = profile
        self.mainDirectory = mainDirectory
        self.environment = environment
        self.workspaceToken = workspaceToken
        self.timeout = timeout
    }

    /// `$XDG_RUNTIME_DIR/<temp-prefix>-equation` when the runtime dir is valid and
    /// owned by us, else `$TMPDIR/<temp-prefix>-equation-<uid>` — then `<workspace>`.
    /// The result is only a *candidate*: `render` re-validates the base
    /// before every use (no writes under attacker-owned paths).
    public static func workspaceRoot(token: String) -> URL {
        // XDG itself must pass the bar — the child doesn't exist yet, so
        // checking the child would always fail.
        let prefix = AppIdentity.current.tempPrefix
        let xdg = ProcessInfo.processInfo.environment["XDG_RUNTIME_DIR"]
            .map { URL(fileURLWithPath: $0, isDirectory: true) }
        let parent = (xdg.flatMap { isSafePrivateDirectory($0) ? $0.appendingPathComponent("\(prefix)-equation", isDirectory: true) : nil })
            ?? FileManager.default.temporaryDirectory
                .appendingPathComponent("\(prefix)-equation-\(getuid())", isDirectory: true)
        return parent.appendingPathComponent(token, isDirectory: true)
    }

    /// The directory we drop request directories into must be ours: a real
    /// directory (never a symlink), owned by this uid, no group/other bits.
    /// Checked via lstat so a symlink at the path is not followed.
    static func isSafePrivateDirectory(_ url: URL) -> Bool {
        var info = stat()
        guard url.withUnsafeFileSystemRepresentation({ $0.map { lstat($0, &info) } == 0 }),
              (info.st_mode & S_IFMT) == S_IFDIR,
              info.st_uid == getuid(),
              info.st_mode & 0o077 == 0 else { return false }
        return true
    }

    /// Compiles `document` and returns the PDF bytes. The request directory
    /// is always removed, including on cancellation (the process group is
    /// terminated first by ``ProcessRunner``).
    public func render(document: String) async throws -> [UInt8] {
        let files = FileManager.default
        let base = Self.workspaceRoot(token: workspaceToken)
        let directory = base.appendingPathComponent(UUID().uuidString, isDirectory: true)
        // The base's parent (pitex-equation / pitex-equation-<uid>) is
        // validated BEFORE the base is created under it — otherwise a
        // foreign-owned lax dir would already receive our writes.
        let parent = base.deletingLastPathComponent()
        do {
            try files.createDirectory(at: parent, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            guard Self.isSafePrivateDirectory(parent) else { throw ExactEquationRenderError.temporaryDirectoryFailed }
            // true here is safe: parent is validated, and base is
            // re-validated below — a second render reuses the same
            // workspace root (a fresh UUID request dir lives under it).
            try files.createDirectory(at: base, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            try files.createDirectory(at: directory, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
        } catch {
            throw ExactEquationRenderError.temporaryDirectoryFailed
        }
        // A pre-created base under a shared temp dir may be foreign-owned,
        // lax or a symlink — fail closed rather than trust it. Validate
        // BEFORE the cleanup defer too, so a hostile base never gets a
        // recursive delete rooted inside it.
        guard Self.isSafePrivateDirectory(base), Self.isSafePrivateDirectory(directory) else {
            throw ExactEquationRenderError.temporaryDirectoryFailed
        }
        defer { try? files.removeItem(at: directory) }
        let source = directory.appendingPathComponent(ExactEquationProfile.jobName + ".tex")
        guard files.createFile(atPath: source.path, contents: Data(document.utf8), attributes: [.posixPermissions: 0o600]) else {
            throw ExactEquationRenderError.temporaryDirectoryFailed
        }

        var overrides = environment
        let inherited = overrides["TEXINPUTS"] ?? ProcessInfo.processInfo.environment["TEXINPUTS"]
        // Trailing separator keeps the distribution's default search path.
        overrides["TEXINPUTS"] = mainDirectory.path + ":" + (inherited ?? "")
        // Paranoid writes: \openout stays inside the request directory even
        // if a user texmf.cnf relaxed it.
        overrides["openout_any"] = "p"
        // ponytail: stdout capture is UNBOUNDED in bytes — batchmode
        // silences the engine itself, but a shell-escape child can still
        // write until the 20 s timeout. If that matters, ProcessRunner
        // needs a discard/cap option; the job .log is tailed on failure.
        let plan = try DirectCommandPlan(
            executable: profile.engine.rawValue,
            arguments: profile.arguments(outputDirectory: directory.path),
            workingDirectory: .explicit(directory.path),
            environment: .inherit(overrides: overrides)
        )
        let result = try await ProcessRunner().run(plan, projectRoot: directory, timeout: timeout)
        switch result.stopReason {
        case .timedOut: throw ExactEquationRenderError.timedOut
        case .cancelled: throw ExactEquationRenderError.cancelled
        case .completed: break
        }
        if Task.isCancelled { throw ExactEquationRenderError.cancelled }
        let pdf = directory.appendingPathComponent(ExactEquationProfile.jobName + ".pdf")
        guard result.termination == .exited(code: 0), let data = files.contents(atPath: pdf.path), !data.isEmpty else {
            // batchmode puts the diagnostic stream in <jobname>.log; read
            // only its tail instead of accumulating unbounded stdout.
            let log = Self.logTail(in: directory) ?? String(decoding: result.standardOutput.suffix(4_096), as: UTF8.self)
            throw ExactEquationRenderError.compileFailed(log: log)
        }
        guard data.count <= Self.maximumPDFBytes else { throw ExactEquationRenderError.outputTooLarge }
        return [UInt8](data)
    }

    /// Last 4 KiB of <jobname>.log without loading it whole.
    private static func logTail(in directory: URL) -> String? {
        let path = directory.appendingPathComponent(ExactEquationProfile.jobName + ".log")
        guard let handle = try? FileHandle(forReadingFrom: path) else { return nil }
        defer { try? handle.close() }
        let size = (try? handle.seekToEnd()) ?? 0
        try? handle.seek(toOffset: size > 4_096 ? size - 4_096 : 0)
        guard let data = try? handle.readToEnd() else { return nil }
        return String(decoding: data, as: UTF8.self)
    }

    /// Removes this workspace's leftovers (a crashed or killed app).
    public static func removeWorkspaceArtifacts(token: String) {
        // Never recursively delete inside a foreign-owned/symlinked root:
        // re-validate the parent chain before touching it.
        let root = workspaceRoot(token: token)
        let parent = root.deletingLastPathComponent()
        guard isSafePrivateDirectory(parent), isSafePrivateDirectory(root) else { return }
        try? FileManager.default.removeItem(at: root)
    }
}
