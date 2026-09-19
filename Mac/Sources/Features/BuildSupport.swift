import BuildCore
import BuildFeature
import DocumentSessionCore
import Foundation
import LanguageCore
import ProjectCore
import SettingsFeature
import TexDomain

/// Streams build output through the portable POSIX ProcessRunner so the log
/// updates live and cancellation terminates the whole spawned process group.
actor StreamingBuildExecutor: BuildProcessExecuting {
    private var running: [BuildID: Task<BuildCore.ProcessResult, Error>] = [:]

    func execute(
        _ request: BuildProcessRequest,
        output: @escaping @Sendable (BuildProcessOutput) async -> Void
    ) async throws -> BuildProcessResult {
        let runner = ProcessRunner()
        let handler: ProcessRunner.OutputHandler = { chunk in
            await output(BuildProcessOutput(
                channel: chunk.channel == .standardOutput ? .standardOutput : .standardError,
                bytes: chunk.bytes
            ))
        }
        let task = Task<BuildCore.ProcessResult, Error> {
            switch request.command {
            case let .direct(plan):
                return try await runner.run(
                    DirectCommandPlan(
                        executable: plan.executable, arguments: plan.arguments,
                        workingDirectory: plan.workingDirectory,
                        environment: await Self.buildEnvironment(plan.environment)
                    ),
                    projectRoot: request.projectRoot,
                    sourceDirectory: request.sourceDirectory,
                    outputHandler: handler
                )
            case let .loginShell(plan):
                return try await runner.runLoginShell(
                    LoginShellCommandPlan(
                        shellExecutable: plan.shellExecutable, command: plan.command,
                        workingDirectory: plan.workingDirectory,
                        environment: await Self.buildEnvironment(plan.environment),
                        authority: plan.authority
                    ),
                    projectRoot: request.projectRoot,
                    sourceDirectory: request.sourceDirectory,
                    outputHandler: handler
                )
            }
        }
        running[request.buildID] = task
        defer { running[request.buildID] = nil }
        let result = try await task.value
        switch result.termination {
        case let .exited(code):
            return BuildProcessResult(exitCode: code)
        case let .signaled(signal):
            return BuildProcessResult(exitCode: 128 + signal)
        }
    }

    func cancel(buildID: BuildID) async {
        running[buildID]?.cancel()
    }

    /// Finder does not load shell profiles. Include the standard macOS TeX
    /// locations for both the engine and subprocesses such as xdvipdfmx/bibtex.
    private static func buildEnvironment(_ policy: EnvironmentPolicy) async -> EnvironmentPolicy {
        guard case var .inherit(overrides) = policy else { return policy }
        let path = overrides["PATH"] ?? ProcessInfo.processInfo.environment["PATH"]
            ?? "/usr/bin:/bin:/usr/sbin:/sbin"
        var buildPath = path + ":/Library/TeX/texbin:/opt/homebrew/bin:/usr/local/bin"
        if let directory = try? await nativeBiberDirectory(in: buildPath) {
            buildPath = directory + ":" + buildPath
        }
        overrides["PATH"] = buildPath
        return .inherit(overrides: overrides)
    }

    /// Some MacTeX Biber launchers invoke a removed lipo option before Perl
    /// starts. Use the installed binary's native slice in our cache; the TeX
    /// installation stays untouched, and an updated binary gets a new cache.
    private static func nativeBiberDirectory(in path: String) async throws -> String? {
        let files = FileManager.default
        guard let source = path.split(separator: ":").map({ URL(fileURLWithPath: String($0)).appendingPathComponent("biber") })
            .first(where: { files.isExecutableFile(atPath: $0.path) }) else { return nil }
        let handle = try FileHandle(forReadingFrom: source)
        defer { try? handle.close() }
        guard let header = try handle.read(upToCount: 4),
              header == Data([0xca, 0xfe, 0xba, 0xbe]) || header == Data([0xca, 0xfe, 0xba, 0xbf]) else { return nil }
        #if arch(arm64)
        let architecture = "arm64"
        #else
        let architecture = "x86_64"
        #endif
        let attributes = try files.attributesOfItem(atPath: source.path)
        let modified = (attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0
        let size = (attributes[.size] as? NSNumber)?.int64Value ?? 0
        let directory = try files.url(for: .cachesDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
            .appendingPathComponent("Pitex/tex-tools/\(architecture)-\(size)-\(modified)", isDirectory: true)
        let destination = directory.appendingPathComponent("biber")
        if !files.isExecutableFile(atPath: destination.path) {
            try files.createDirectory(at: directory, withIntermediateDirectories: true)
            let temporary = directory.appendingPathComponent(UUID().uuidString)
            defer { try? files.removeItem(at: temporary) }
            let result = try await ProcessRunner().run(DirectCommandPlan(executable: "/usr/bin/lipo",
                arguments: [source.path, "-thin", architecture, "-output", temporary.path]),
                projectRoot: directory, timeout: .seconds(15))
            guard result.termination == .exited(code: 0) else { return nil }
            try files.setAttributes([.posixPermissions: 0o755], ofItemAtPath: temporary.path)
            if !files.fileExists(atPath: destination.path) { try files.moveItem(at: temporary, to: destination) }
        }
        return directory.path
    }
}

enum WorkspaceBuildError: LocalizedError {
    case noActiveDocument
    case buildAlreadyRunning
    case customShellNotAcknowledged
    case texToolchainUnavailable

    var errorDescription: String? {
        switch self {
        case .noActiveDocument: "Open a .tex source before building."
        case .buildAlreadyRunning: "A build is already running."
        case .customShellNotAcknowledged: "Approve the custom command's shell authority in Settings before building."
        case .texToolchainUnavailable: "No LaTeX toolchain was found (latexmk/pdflatex/xelatex/lualatex)."
        }
    }
}

/// Resolves ordinary TeX inclusion trees without executing TeX or guessing by
/// basename. The existing language parser excludes comments/verbatim examples.
struct TeXProjectResolver {
    private var snapshots: [URL: LanguageFileSnapshot] = [:]
    var activeText: (url: URL, text: String)?

    private func canonical(_ url: URL) -> URL {
        url.resolvingSymlinksInPath().standardizedFileURL
    }

    private mutating func snapshot(_ url: URL) -> LanguageFileSnapshot? {
        let file = canonical(url)
        if let cached = snapshots[file] { return cached }
        let text = activeText.flatMap { canonical($0.url) == file ? $0.text : nil }
            ?? (try? String(contentsOf: file, encoding: .utf8))
        guard let text, let parsed = try? LanguageFileSnapshot(sourceID: file.path, revision: 0, source: text) else { return nil }
        snapshots[file] = parsed
        return parsed
    }

    private func captures(_ pattern: String, in text: String) -> [String] {
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }
        return regex.matches(in: text, range: NSRange(location: 0, length: text.utf16.count)).map {
            (text as NSString).substring(with: $0.range(at: 1)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    private mutating func rootHint(_ url: URL) -> URL? {
        guard let source = snapshot(url)?.source else { return nil }
        let hints = captures(#"(?im)^\s*%\s*!\s*tex\s+root\s*=\s*(.+)$"#, in: source)
            + captures(#"\\documentclass\s*\[([^\]]+)\]\s*\{subfiles\}"#, in: source)
        guard var hint = hints.first else { return nil }
        if hint.hasPrefix("\""), hint.hasSuffix("\"") { hint = String(hint.dropFirst().dropLast()) }
        if (hint as NSString).pathExtension.isEmpty { hint += ".tex" }
        return canonical(url.deletingLastPathComponent().appendingPathComponent(hint))
    }

    private mutating func declaredRoot(_ url: URL) throws -> URL? {
        var file = canonical(url)
        var seen: Set<URL> = [file]
        var hasHint = false
        while let next = rootHint(file) {
            hasHint = true
            guard seen.insert(next).inserted, FileManager.default.isReadableFile(atPath: next.path) else {
                throw ResolutionError.invalidRoot
            }
            file = next
        }
        return hasHint ? file : nil
    }

    private mutating func isMain(_ url: URL) -> Bool {
        guard url.pathExtension.lowercased() == "tex", let snapshot = snapshot(url) else { return false }
        return snapshot.tokens.contains { $0.kind == .controlSequence("documentclass") }
    }

    /// `direct_links` — include/bibliography targets of one file, resolved
    /// against the main document's directory first, then the including
    /// file's. Only existing files are returned, canonicalized.
    private mutating func directLinks(of file: URL, base: URL) -> [URL] {
        guard let parsed = snapshot(file) else { return [] }
        var links = parsed.includes.map { ($0.target, "tex") }
        // Remove lexer-recognized comments before scanning the resource
        // commands not yet represented by LanguageFileSnapshot.includes.
        var source = parsed.source
        for token in parsed.tokens.reversed() {
            guard case .comment = token.kind else { continue }
            let bytes = source.utf8
            let lower = bytes.index(bytes.startIndex, offsetBy: token.range.utf8Offset)
            let upper = bytes.index(lower, offsetBy: token.range.utf8Length)
            source.removeSubrange(lower..<upper)
        }
        links += captures(#"\\subfile\s*\{([^}]+)\}"#, in: source).map { ($0, "tex") }
        for group in captures(#"\\bibliography\s*\{([^}]+)\}"#, in: source) {
            links += group.split(separator: ",").map { (String($0).trimmingCharacters(in: .whitespaces), "bib") }
        }
        links += captures(#"\\addbibresource(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}"#, in: source).map { ($0, "bib") }
        var targets: [URL] = []
        for (name, ext) in links {
            let path = (name as NSString).pathExtension.isEmpty ? name + "." + ext : name
            // TeX resolves nested \input paths from the main document's
            // working directory. subfiles can additionally use local paths.
            let candidates = [base.appendingPathComponent(path), file.deletingLastPathComponent().appendingPathComponent(path)]
            if let target = candidates.first(where: { FileManager.default.isReadableFile(atPath: $0.path) }) {
                let target = canonical(target)
                if !targets.contains(target) { targets.append(target) }
            }
        }
        return targets
    }

    /// `direct_dependencies` — the main document's own bibliography and
    /// included files, one level only; the sidebar nests these under it.
    mutating func directDependencies(main: URL) -> [URL] {
        let file = canonical(main)
        guard file.pathExtension.lowercased() == "tex" else { return [] }
        return directLinks(of: file, base: file.deletingLastPathComponent())
    }

    private mutating func dependencies(of main: URL) -> Set<URL> {
        let base = main.deletingLastPathComponent()
        var visited = Set<URL>()
        var pending = [canonical(main)]
        while let file = pending.popLast() {
            guard visited.insert(file).inserted, file.pathExtension.lowercased() == "tex" else { continue }
            pending.append(contentsOf: directLinks(of: file, base: base))
        }
        return visited
    }

    mutating func resolve(active: URL?, files: [URL], preferred: URL? = nil) throws -> URL? {
        if let active, let declared = try declaredRoot(active) { return declared }
        if let active, isMain(active) { return canonical(active) }
        let mains = files.filter { isMain($0) }.map(canonical)
        if let active, active.pathExtension.lowercased() == "bbl",
           let main = mains.first(where: { $0.deletingPathExtension() == canonical(active).deletingPathExtension() }) { return main }
        let owners = active.map { file in mains.filter { dependencies(of: $0).contains(canonical(file)) } } ?? []
        if let preferred, owners.contains(canonical(preferred)) { return canonical(preferred) }
        if owners.count == 1 { return owners[0] }
        if owners.isEmpty, mains.count == 1 { return mains[0] }
        if mains.count > 1 { throw ResolutionError.ambiguous }
        return nil
    }

    mutating func initialDocument(in files: [URL]) -> URL? {
        let mains = files.filter { isMain($0) }
        return mains.first { $0.lastPathComponent.lowercased() == "main.tex" }
            ?? mains.first ?? files.first { $0.pathExtension.lowercased() == "tex" } ?? files.first
    }

    /// Opening a chapter directly still opens its owning project. Parent
    /// directories are inspected shallowly and only a proven include edge or
    /// explicit root directive can expand the workspace beyond that directory.
    mutating func projectRoot(for selected: URL) -> URL {
        var main = (try? declaredRoot(selected)) ?? (isMain(selected) ? canonical(selected) : nil)
        var directory = selected.deletingLastPathComponent().standardizedFileURL
        if main == nil {
            while directory.path != "/" {
                let files = (try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil,
                    options: [.skipsHiddenFiles, .skipsPackageDescendants])) ?? []
                let owners = files.filter { isMain($0) && dependencies(of: $0).contains(canonical(selected)) }
                if let owner = owners.first { main = owner; break }
                if directory == FileManager.default.homeDirectoryForCurrentUser
                    || FileManager.default.fileExists(atPath: directory.appendingPathComponent(".git").path) { break }
                directory.deleteLastPathComponent()
            }
        }
        guard let main else { return selected.deletingLastPathComponent().standardizedFileURL }
        var root = canonical(main).deletingLastPathComponent()
        for file in dependencies(of: main).union([canonical(selected)]) {
            while !file.path.hasPrefix(root.path + "/"), root.path != "/" { root.deleteLastPathComponent() }
        }
        return root
    }

    enum ResolutionError: LocalizedError {
        case invalidRoot, ambiguous
        var errorDescription: String? {
            switch self {
            case .invalidRoot: "The TeX root directive points to a missing file or forms a cycle. Check % !TeX root."
            case .ambiguous: "More than one main TeX document uses this file. Open the intended main document and pin it as the build target."
            }
        }
    }
}

extension WorkspaceModel {
    /// Maps a console build-command preset onto the direct engine stage so
    /// standard builds don't need a login shell.
    static func engineStage(forCommandPreset command: String) -> BuildToolStage? {
        switch command {
        case "xelatex -interaction=nonstopmode -synctex=1 {file}": .latexmkXeLaTeX
        case "pdflatex -interaction=nonstopmode -synctex=1 {file}", "latexmk -pdf -interaction=nonstopmode -synctex=1 {file}": .latexmk
        case "lualatex -interaction=nonstopmode -synctex=1 {file}": .latexmkLuaLaTeX
        default: nil
        }
    }

    static let generatedOutputExtensions = [
        "pdf", "aux", "log", "out", "synctex.gz", "fdb_latexmk", "fls",
        "toc", "bbl", "blg", "bcf", "run.xml", "idx", "ind", "ilg", "nav",
        "snm", "vrb", "xdv", "lof", "lot",
    ]

    /// Builds the active document with the configured engine. Dirty open
    /// documents are saved first so the compiler sees what the editor shows.
    func startBuild() async {
        guard !isBuilding else { return }
        guard case .ready = phase else { return }
        // Root discovery must see edits to inactive main/preamble files too.
        if let problem = await persistDirtySessions() {
            buildState = .failed(problem)
            return
        }
        refreshBuildTarget()
        guard case .ready = phase,
              let root = projectURL,
              let sourceURL = buildSourceURL() else {
            let reason = buildTargetMessage ?? WorkspaceBuildError.noActiveDocument.localizedDescription
            buildState = .failed(reason)
            buildLogText = reason
            consoleSection = .log
            bottomPanelVisible = true
            return
        }
        if sourceURL != activeDocumentURL,
           !registeredSessions.contains(where: { session in
               (try? Self.relativePath(for: sourceURL, root: root)) == session.path
           }) {
            // The main file has no session yet; open one so its disk
            // baseline is tracked like every other open document.
            if let text = try? Self.readExactUTF8(sourceURL),
               let relative = try? Self.relativePath(for: sourceURL, root: root),
               let file = try? ProjectFile(
                   documentID: StableDocumentID(rawValue: Self.documentID(for: relative.rawValue)),
                   path: relative
               ),
               let session = try? await registry.open(
                   projectRoot: root, file: file,
                   initialText: text, diskBaselineHash: .hashing(text)
               ) {
                registeredSessions.append(session)
            }
        }
        guard let sourceSession = registeredSessions.first(where: { session in
            (try? Self.relativePath(for: sourceURL, root: root)) == session.path
        }) else {
            buildState = .unavailable(WorkspaceBuildError.noActiveDocument.localizedDescription)
            return
        }
        let snapshot = await sourceSession.snapshot()
        guard !isBuilding else {
            buildState = .unavailable(WorkspaceBuildError.buildAlreadyRunning.localizedDescription)
            return
        }

        let relativeSource = snapshot.path.rawValue
        let stem = (relativeSource as NSString).deletingPathExtension
        var generated = Set<String>()
        for ext in Self.generatedOutputExtensions {
            generated.insert("\(stem).\(ext)")
        }
        let outputPDF = "\(stem).pdf"

        guard let target = try? BuildTarget(
            projectRoot: root,
            sourcePath: relativeSource,
            outputPDFPath: outputPDF,
            declaredGeneratedPaths: generated
        ) else {
            buildState = .failed("The build target could not be constructed for \(relativeSource).")
            return
        }

        let buildPreferences = settings.settings.build
        let pipeline: BuildPipeline
        // The console's build-command field drives the build: known presets
        // map onto the direct engine stages, anything else runs through a
        // login shell (which requires the custom-shell acknowledgement).
        let commandText = buildCommandText.trimmingCharacters(in: .whitespacesAndNewlines)
        if let stage = Self.engineStage(forCommandPreset: commandText) {
            pipeline = .singlePass(stage)
        } else if !commandText.isEmpty {
            guard buildPreferences.customShellAcknowledged else {
                buildState = .failed(WorkspaceBuildError.customShellNotAcknowledged.localizedDescription)
                return
            }
            guard let plan = try? LoginShellCommandPlan(
                shellExecutable: settings.customShellExecutable,
                command: substituteCommandPlaceholders(commandText),
                workingDirectory: .projectRoot,
                authority: try! ShellAuthority(
                    source: .userConfiguration,
                    approvedByUser: true,
                    disclosure: "User-configured build command executed via a login shell."
                )
            ) else {
                buildState = .failed("The configured build command is not valid.")
                return
            }
            pipeline = .custom(plan)
        } else {
            let stage: BuildToolStage = switch buildPreferences.engine {
            case .pdfLaTeX: .latexmk
            case .xeLaTeX: .latexmkXeLaTeX
            case .luaLaTeX: .latexmkLuaLaTeX
            }
            pipeline = .singlePass(stage)
        }

        do {
            try await buildOrchestrator.select(target: target, pipeline: pipeline)
        } catch {
            buildState = .failed("The build could not be configured: \(error.localizedDescription)")
            return
        }

        guard let buildID = try? BuildID(rawValue: "build-\(UUID().uuidString.lowercased())") else {
            buildState = .failed("The build could not be started.")
            return
        }
        activeBuildID = buildID
        invalidateSyncTeXForBuild()
        buildState = .building
        buildLogText = ""
        buildIssues = []
        let orchestrator = buildOrchestrator
        do {
            let outcome = try await orchestrator.build(id: buildID) { [weak self] event in
                await self?.handleBuildEvent(event)
            }
            switch outcome.lifecycle {
            case .succeeded:
                let pdfData = await orchestrator.successfulPDF() ?? Data()
                buildState = .succeeded(pdf: pdfData, log: buildLogText)
                latestBuiltPDFName = outputPDF
                let pdfURL = root.appendingPathComponent(outputPDF).standardizedFileURL
                await refreshSyncTeXBinding(pdfURL: pdfURL)
                if settings.switchToPDFOnBuild {
                    inspectorVisible = true
                }
                if settings.jumpToCursorAfterBuild {
                    await syncForward()
                }
            case .cancelled:
                buildState = .failed("Build cancelled.")
            case let .failed(exitCode, _):
                buildState = .failed("Build failed\(exitCode.map { " (exit \($0))" } ?? "").")
            case .queued, .running, .cancelling:
                buildState = .failed("The build ended in an unexpected state.")
            }
        } catch {
            buildState = .failed(String(describing: error))
        }
        activeBuildID = nil
    }

    func cancelBuild() async {
        try? await buildOrchestrator.cancel()
    }

    var isBuilding: Bool {
        if case .building = buildState { return true }
        return false
    }

    private func handleBuildEvent(_ event: BuildEvent) async {
        switch event {
        case let .log(entry):
            buildLogText += entry.text
        case let .issue(record):
            buildIssues.append(record)
        case .lifecycle, .stageStarted:
            break
        }
    }
}
