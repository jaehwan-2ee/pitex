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

struct SyncTeXBinding: Sendable {
    let revision: SyncTeXRevision
    let outputHash: String
    let pdfURL: URL
    let projectRoot: URL
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

    /// Fields the normalized record format admits. Everything else in raw
    /// synctex output (Offset, Context, before/offset/kern/glue lines, Post x/y)
    /// is dropped before parsing.
    private static let allowedFields: Set<String> = [
        "Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H",
    ]
    private static let fieldOrder = ["Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H"]

    func refreshBinding(projectRoot: URL, pdfURL: URL, buildID: String) async throws -> SyncTeXBinding {
        _ = try await resolvedTool()
        // synctex reports canonicalized paths (e.g. /private/tmp/... for a
        // project opened as /tmp/...), so the binding stores resolved URLs and
        // every query path is formed against the resolved root.
        let root = projectRoot.resolvingSymlinksInPath().standardizedFileURL
        let pdf = pdfURL.resolvingSymlinksInPath().standardizedFileURL
        let pdfData = try Data(contentsOf: pdf)
        let outputHash = SHA256.hash(data: pdfData).map { String(format: "%02x", $0) }.joined()
        let fingerprint = try Self.syncTeXFingerprint(pdfURL: pdf)
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
        // Input tag 1 records the original main source. Preserve relative
        // paths beneath that directory when a downloaded project is moved;
        // never guess using just a chapter's filename.
        let inputs = try SyncTeXTextParser.parse(String(decoding: metadata, as: UTF8.self)
            .split(separator: "\n").filter { $0.hasPrefix("Input:") }.joined(separator: "\n")).inputs
        let originalMain = inputs.first { $0.tag == 1 }?.path.value
        let originalDirectory = originalMain.map { ($0 as NSString).deletingLastPathComponent }
        var sourcePaths: [String: URL] = [:]
        for input in inputs {
            let path = input.path.value
            var relative = path
            if path.hasPrefix("/"), let originalDirectory, path.hasPrefix(originalDirectory + "/") {
                relative = String(path.dropFirst(originalDirectory.count + 1))
            }
            let mapped = URL(fileURLWithPath: relative, relativeTo: pdf.deletingLastPathComponent())
                .resolvingSymlinksInPath().standardizedFileURL
            guard mapped.path.hasPrefix(root.path + "/"), FileManager.default.isReadableFile(atPath: mapped.path) else { continue }
            sourcePaths[path] = mapped
        }
        return SyncTeXBinding(
            revision: try SyncTeXRevision(buildID: buildID, fingerprint: fingerprint),
            outputHash: outputHash,
            pdfURL: pdf,
            projectRoot: root,
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
        try Self.validate(binding)
        let inputPath = binding.sourcePaths.first { $0.value == resolvedSource }?.key ?? resolvedSource.path
        let result = try await runner.run(
            try DirectCommandPlan(
                executable: synctex,
                arguments: [
                    "view",
                    "-i", "\(line):\(column):\(inputPath)",
                    "-o", binding.pdfURL.path,
                ],
                workingDirectory: .explicit(binding.pdfURL.deletingLastPathComponent().path)
            ),
            projectRoot: binding.projectRoot,
            timeout: .seconds(15)
        )
        try Self.validate(binding)
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
        try Self.validate(binding)
        let result = try await runner.run(
            try DirectCommandPlan(
                executable: synctex,
                arguments: [
                    "edit",
                    "-o", "\(page):\(point.x):\(point.y):\(binding.pdfURL.path)",
                ],
                workingDirectory: .explicit(binding.pdfURL.deletingLastPathComponent().path)
            ),
            projectRoot: binding.projectRoot,
            timeout: .seconds(15)
        )
        try Self.validate(binding)
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
                let mapped = key == "Input" ? binding.sourcePaths[path] : nil
                value = (mapped ?? URL(fileURLWithPath: value, relativeTo: binding.pdfURL.deletingLastPathComponent()))
                    .resolvingSymlinksInPath().standardizedFileURL.path
            }
            if Self.allowedFields.contains(key) { fields[key] = value }
        }
        return normalized
    }

    private static func validate(_ binding: SyncTeXBinding) throws {
        guard try syncTeXFingerprint(pdfURL: binding.pdfURL) == binding.revision.fingerprint,
              SHA256.hash(data: try Data(contentsOf: binding.pdfURL)).map({ String(format: "%02x", $0) }).joined() == binding.outputHash
        else { throw SyncTeXQueryError.staleResult }
    }

    /// Fingerprint derived from the .synctex file sitting next to the PDF so
    /// every rebuild invalidates previous bindings deterministically.
    private static func syncTeXFingerprint(pdfURL: URL) throws -> UInt64 {
        let gz = pdfURL.deletingPathExtension().appendingPathExtension("synctex.gz")
        let plain = pdfURL.deletingPathExtension().appendingPathExtension("synctex")
        let metadataURL = FileManager.default.fileExists(atPath: gz.path) ? gz : plain
        guard let data = try? Data(contentsOf: metadataURL), !data.isEmpty else {
            throw SyncTeXSupportError.missingMetadata
        }
        let digest = SHA256.hash(data: data)
        return digest.prefix(8).reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
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
