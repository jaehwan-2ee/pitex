import Darwin
import BuildCore
import CryptoKit
import Foundation
import SyncTeXCore

enum SyncTeXSupportError: LocalizedError {
    case toolUnavailable
    case noBinding
    case noResult
    case staleResult
    case ambiguousResult
    case missingMetadata

    var errorDescription: String? {
        switch self {
        case .toolUnavailable: "The synctex command-line tool is not installed."
        case .noBinding: "SyncTeX metadata has not been captured for the latest build."
        case .noResult: "SyncTeX found no matching location."
        case .staleResult: "SyncTeX results no longer match the current PDF or source."
        case .ambiguousResult: "SyncTeX returned more than one exact match."
        case .missingMetadata: "The build did not produce a .synctex file next to the PDF."
        }
    }
}

struct SyncTeXBinding: Sendable, Equatable {
    let revision: SyncTeXRevision
    let outputHash: String
    let pdfURL: URL
    let projectRoot: URL
    /// Directory recorded relative inputs resolve against — the known
    /// main source's directory, never the (possibly `.pitex-live`) PDF
    /// folder. The synctex CLI runs with this as its working directory.
    let sourceRoot: URL
    let sourcePaths: [String: URL]
}

/// Runs the `synctex` command-line tool and normalizes its raw output into the
/// strict record shape SyncTeXQueryParser validates. Raw CLI output carries
/// extra fields and no binding metadata, so normalization:
///   - drops every field outside the parser's allowed key set,
///   - injects the queried values (Input/Line/Column for view, Page/x/y for
///     edit, Output for both) when a result block omits them,
///   - prepends `SyncTeX Version:` and `SyncTeX Fingerprint:` headers derived
///     from the .synctex file bytes, so any rebuild changes the fingerprint and
///     stale results fail closed.
actor SyncTeXRunner {
    private let runner = ProcessRunner()
    private var toolPath: String?
    private var digestCache: [URL: (stamp: [Int64], digest: String)] = [:]

    /// Fields the normalized record format admits. Everything else in raw
    /// synctex output (Offset, Context, before/offset/kern/glue lines, Post x/y)
    /// is dropped before parsing.
    private static let allowedFields: Set<String> = [
        "Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H",
    ]
    private static let fieldOrder = ["Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H"]

    /// `mainRelativePath` is the project-relative main source the build
    /// ran on — the known anchor the mapper uses instead of the PDF's
    /// folder (a live build's output hides under `.pitex-live`).
    func refreshBinding(projectRoot: URL, pdfURL: URL, buildID: String,
                        mainRelativePath: String) async throws -> SyncTeXBinding {
        _ = try await resolvedTool()
        // synctex reports canonicalized paths (e.g. /private/tmp/... for a
        // project opened as /tmp/...), so the binding stores resolved URLs and
        // every query path is formed against the resolved root.
        let root = projectRoot.resolvingSymlinksInPath().standardizedFileURL
        let pdf = pdfURL.resolvingSymlinksInPath().standardizedFileURL
        digestCache.removeAll(keepingCapacity: true)
        let outputHash = try fileDigest(pdf)
        let fingerprint = try syncTeXFingerprint(pdfURL: pdf)
        let plain = pdf.deletingPathExtension().appendingPathExtension("synctex")
        let gz = plain.appendingPathExtension("gz")
        let metadata: Data
        if FileManager.default.fileExists(atPath: gz.path) {
            let result = try await runner.run(DirectCommandPlan(executable: "/usr/bin/gzip", arguments: ["-cd", gz.path]),
                                              projectRoot: root, timeout: .seconds(15))
            guard result.termination == .exited(code: 0) else { throw SyncTeXSupportError.missingMetadata }
            metadata = result.standardOutput
        } else {
            metadata = try Data(contentsOf: plain)
        }
        // Input tag 1 records the original main source. Its absolute path
        // on the build device minus the known project-relative main
        // reveals the remote project root; a relative recording means the
        // build ran here and inputs are already local anchors.
        let inputs = try SyncTeXTextParser.parse(Self.inputLines(metadata)).inputs
        let recordedRoot = (inputs.first { $0.tag == 1 }?.path.value).flatMap {
            SyncTeXSourceMapper.recordedProjectRoot(originalMain: $0, mainRelative: mainRelativePath)
        }
        var sourcePaths: [String: URL] = [:]
        for input in inputs {
            let path = input.path.value
            guard let relative = SyncTeXSourceMapper.projectRelativePath(
                recordedPath: path,
                projectRoot: root.path,
                mainRelative: mainRelativePath,
                recordedRoot: recordedRoot
            ) else { continue }
            let mapped = root.appendingPathComponent(relative)
                .resolvingSymlinksInPath().standardizedFileURL
            guard mapped.path.hasPrefix(root.path + "/"),
                  FileManager.default.isReadableFile(atPath: mapped.path) else { continue }
            sourcePaths[path] = mapped
        }
        return SyncTeXBinding(
            revision: try SyncTeXRevision(buildID: buildID, fingerprint: fingerprint),
            outputHash: outputHash,
            pdfURL: pdf,
            projectRoot: root,
            // Same anchor the mapper uses: the directory of the known
            // main source (project root when the main sits at top level).
            sourceRoot: root.appendingPathComponent(
                (mainRelativePath as NSString).deletingLastPathComponent
            ).standardizedFileURL,
            sourcePaths: sourcePaths
        )
    }

    func forward(
        binding: SyncTeXBinding,
        sourceURL: URL,
        line: Int,
        column: Int
    ) async throws -> SyncTeXQueryCandidate {
        let synctex = try await resolvedTool()
        let resolvedSource = sourceURL.resolvingSymlinksInPath().standardizedFileURL
        try validate(binding)
        let inputPath = binding.sourcePaths.first { $0.value == resolvedSource }?.key ?? resolvedSource.path
        let result = try await runner.run(
            try DirectCommandPlan(
                executable: synctex,
                arguments: [
                    "view",
                    "-i", "\(line):\(column):\(inputPath)",
                    "-o", binding.pdfURL.path,
                ],
                // Recorded relative inputs resolve against the known
                // source directory, not the PDF's (hidden live) folder.
                workingDirectory: .explicit(binding.sourceRoot.path)
            ),
            projectRoot: binding.projectRoot,
            timeout: .seconds(15)
        )
        try validate(binding)
        let normalized = Self.normalize(
            String(decoding: result.standardOutput, as: UTF8.self),
            queryFields: [
                "Input": resolvedSource.path,
                "Line": "\(line)",
                "Column": "\(column)",
                "Output": binding.pdfURL.path,
            ],
            binding: binding
        )
        let document = try SyncTeXQueryParser.parse(
            normalized,
            projectRoot: binding.projectRoot.path,
            binding: try SyncTeXOutputBinding(
                revision: binding.revision,
                outputHash: binding.outputHash
            )
        )
        let sourcePath = try NormalizedSourcePath(
            String(resolvedSource.path.dropPrefix(binding.projectRoot.path + "/"))
        )
        let pdfPath = try NormalizedSourcePath(
            String(binding.pdfURL.path.dropPrefix(binding.projectRoot.path + "/"))
        )
        return try ExactSyncTeXQuerySelector.forward(
            Array(document.candidates.prefix(1)),
            query: ForwardSyncQuery(
                revision: binding.revision,
                source: SourceLocation(path: sourcePath, line: line, column: column),
                expectedPDF: pdfPath
            ),
            outputHash: binding.outputHash
        )
    }

    func inverse(
        binding: SyncTeXBinding,
        page: Int,
        point: SyncTeXCore.PDFPoint
    ) async throws -> SyncTeXQueryCandidate {
        let synctex = try await resolvedTool()
        try validate(binding)
        let result = try await runner.run(
            try DirectCommandPlan(
                executable: synctex,
                arguments: [
                    "edit",
                    "-o", "\(page):\(point.x):\(point.y):\(binding.pdfURL.path)",
                ],
                workingDirectory: .explicit(binding.sourceRoot.path)
            ),
            projectRoot: binding.projectRoot,
            timeout: .seconds(15)
        )
        try validate(binding)
        let normalized = Self.normalize(
            String(decoding: result.standardOutput, as: UTF8.self),
            queryFields: [
                // `synctex edit` reports Column:-1 (whole line); the normalized
                // format requires a nonnegative column.
                "Column": "0",
                "Page": "\(page)",
                "x": "\(point.x)",
                "y": "\(point.y)",
                "h": "\(point.x)",
                "v": "\(point.y)",
                "W": "0",
                "H": "0",
                "Output": binding.pdfURL.path,
            ],
            binding: binding
        )
        let document = try SyncTeXQueryParser.parse(
            normalized,
            projectRoot: binding.projectRoot.path,
            binding: try SyncTeXOutputBinding(
                revision: binding.revision,
                outputHash: binding.outputHash
            )
        )
        let pdfPath = try NormalizedSourcePath(
            String(binding.pdfURL.path.dropPrefix(binding.projectRoot.path + "/"))
        )
        return try ExactSyncTeXQuerySelector.inverse(
            Array(document.candidates.prefix(1)),
            query: InverseSyncQuery(
                revision: binding.revision,
                pdf: PDFLocation(pdfPath: pdfPath, page: page, point: point)
            ),
            outputHash: binding.outputHash,
            coordinateEpsilon: 2
        )
    }

    /// The `Input:` lines of a decompressed .synctex file, joined by \n —
    /// a byte scan so refreshBinding never decodes or splits the whole
    /// metadata blob (tens of MB for a thesis) just to filter it.
    /// Equivalent to decoding the data, splitting on \n and keeping the
    /// lines starting with "Input:" (the prefix is ASCII, and \n can never
    /// sit inside a multi-byte UTF-8 sequence, so per-line decoding is
    /// identical to decoding the whole file first).
    static func inputLines(_ metadata: Data) -> String {
        var lines: [String] = []
        var start = metadata.startIndex
        while start < metadata.endIndex {
            let end = metadata[start...].firstIndex(of: 0x0A) ?? metadata.endIndex
            if metadata[start...].starts(with: "Input:".utf8) {
                lines.append(String(decoding: metadata[start..<end], as: UTF8.self))
            }
            start = metadata.index(after: end)
        }
        return lines.joined(separator: "\n")
    }

    /// Collapses raw `synctex` output to the normalized record shape. Fields not
    /// in `allowedFields` are dropped; missing fields are filled from the query.
    private static func normalize(
        _ raw: String,
        queryFields: [String: String],
        binding: SyncTeXBinding
    ) -> String {
        var normalized = "SyncTeX Version:1\nSyncTeX Fingerprint:\(binding.revision.fingerprint)\n"
        var inBlock = false
        var fields: [String: String] = [:]

        func flushBlock() {
            guard !fields.isEmpty else { return }
            var merged = queryFields
            for (key, value) in fields { merged[key] = value }
            normalized += "SyncTeX result begin\n"
            for key in Self.fieldOrder {
                if let value = merged[key] { normalized += "\(key):\(value)\n" }
            }
            normalized += "SyncTeX result end\n"
        }

        for rawLine in raw.split(separator: "\n", omittingEmptySubsequences: false) {
            let line = rawLine.hasSuffix("\r") ? String(rawLine.dropLast()) : String(rawLine)
            if line == "SyncTeX result begin" {
                inBlock = true
                fields = [:]
                continue
            }
            if line == "SyncTeX result end" {
                if inBlock { flushBlock() }
                inBlock = false
                continue
            }
            guard inBlock, let separator = line.firstIndex(of: ":") else { continue }
            let key = String(line[..<separator])
            var value = String(line[line.index(after: separator)...])
            // The CLI encloses all ranked hits in one begin/end pair. Each
            // repeated Output starts another hit; keep that ranking instead
            // of overwriting the best hit with the last one.
            if key == "Output", fields["Output"] != nil {
                flushBlock()
                fields = [:]
            }
            // `synctex` emits Column:-1 for whole-line hits; the normalized
            // format requires nonnegative values, so the query fallback wins.
            if key == "Column", value.hasPrefix("-") { continue }
            // Foundation shortens /private/tmp to /tmp even when resolving
            // symlinks. Normalize CLI paths exactly like the binding root.
            // Leave invalid paths intact for the strict parser to reject.
            if (key == "Input" || key == "Output"), !value.contains("\\"), !value.contains("\0") {
                let path = (try? NormalizedSourcePath(value).value) ?? value
                if key == "Input" {
                    // An input the mapper could not anchor stays raw — the
                    // strict selector rejects it rather than following a
                    // fabricated path under the hidden .pitex-live output.
                    if let mapped = binding.sourcePaths[path] { value = mapped.path }
                } else {
                    // Output names the PDF itself; resolving it against its
                    // own directory is correct even under .pitex-live.
                    value = URL(fileURLWithPath: value, relativeTo: binding.pdfURL.deletingLastPathComponent())
                        .resolvingSymlinksInPath().standardizedFileURL.path
                }
            }
            if Self.allowedFields.contains(key) { fields[key] = value }
        }
        return normalized
    }

    private func validate(_ binding: SyncTeXBinding) throws {
        guard try syncTeXFingerprint(pdfURL: binding.pdfURL) == binding.revision.fingerprint,
              try fileDigest(binding.pdfURL) == binding.outputHash
        else { throw SyncTeXQueryError.staleResult }
    }

    /// Fingerprint derived from the .synctex file sitting next to the PDF so
    /// every rebuild invalidates previous bindings deterministically.
    private func fileDigest(_ url: URL) throws -> String {
        let before = try Self.fileStamp(url)
        if let cached = digestCache[url], cached.stamp == before { return cached.digest }
        let data = try Data(contentsOf: url)
        guard !data.isEmpty else { throw SyncTeXSupportError.missingMetadata }
        let digest = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        guard try Self.fileStamp(url) == before else { throw SyncTeXQueryError.staleResult }
        digestCache[url] = (before, digest)
        return digest
    }

    private static func fileStamp(_ url: URL) throws -> [Int64] {
        var info = stat()
        guard stat(url.path, &info) == 0 else { throw SyncTeXSupportError.missingMetadata }
        return [Int64(info.st_dev), Int64(truncatingIfNeeded: info.st_ino), info.st_size,
                Int64(info.st_mtimespec.tv_sec), Int64(info.st_mtimespec.tv_nsec),
                Int64(info.st_ctimespec.tv_sec), Int64(info.st_ctimespec.tv_nsec)]
    }

    private func syncTeXFingerprint(pdfURL: URL) throws -> UInt64 {
        let gz = pdfURL.deletingPathExtension().appendingPathExtension("synctex.gz")
        let plain = pdfURL.deletingPathExtension().appendingPathExtension("synctex")
        let metadataURL = FileManager.default.fileExists(atPath: gz.path) ? gz : plain
        guard let digest = try? fileDigest(metadataURL), let value = UInt64(digest.prefix(16), radix: 16)
        else { throw SyncTeXSupportError.missingMetadata }
        return value
    }

    private func resolvedTool() async throws -> String {
        if let toolPath { return toolPath }
        for candidate in ["/Library/TeX/texbin/synctex", "/usr/local/bin/synctex", "/opt/homebrew/bin/synctex", "/usr/bin/synctex"] {
            if FileManager.default.isExecutableFile(atPath: candidate) {
                toolPath = candidate
                return candidate
            }
        }
        let probe = try await runner.run(
            try DirectCommandPlan(executable: "/usr/bin/env", arguments: ["synctex", "--version"]),
            projectRoot: URL(fileURLWithPath: "/tmp"),
            timeout: .seconds(10)
        )
        if case .exited(let code) = probe.termination, code == 0 {
            toolPath = "synctex"
            return "synctex"
        }
        throw SyncTeXSupportError.toolUnavailable
    }
}

private extension String {
    /// Returns the receiver without `prefix`, or unchanged when the prefix is absent.
    func dropPrefix(_ prefix: String) -> Substring {
        hasPrefix(prefix) ? dropFirst(prefix.count) : self[...]
    }
}
