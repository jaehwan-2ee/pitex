/// Project-level macro context for the equation preview: the definitions a
/// region can see, in document order, across `\input`/`\include`/`\subfile`/
/// `\import` and project-local `\usepackage` files.
///
/// The host resolves include targets to file IDs (it owns the file system)
/// and supplies each file's ``MathSourceScan``; flattening is pure and cheap,
/// so it reruns after every active-file edit without touching the disk.

public struct MathIncludeKey: Hashable, Sendable {
    public let fromFileID: String
    public let target: String

    public init(fromFileID: String, target: String) {
        self.fromFileID = fromFileID
        self.target = target
    }
}

/// An include the context could not place yet: the host tries the
/// candidates in order and reports the first existing file.
public struct MathIncludeRequest: Hashable, Sendable {
    public let key: MathIncludeKey
    public let candidates: [String]
}

public struct MathPreviewContext: Hashable, Sendable {
    /// MathJax sources, applied in order.
    public let definitions: [String]
    /// Opaque identity of `definitions` (cumulative FNV-1a).
    public let key: String
    /// Bounds dropped later definitions.
    public let truncated: Bool

    public static let empty = MathPreviewContext(definitions: [], key: MathPreviewHash.hex(MathPreviewHash.basis), truncated: false)
}

public enum MathPreviewHash {
    public static let basis: UInt64 = 0xcbf2_9ce4_8422_2325

    public static func combine(_ hash: UInt64, _ text: String) -> UInt64 {
        var value = hash
        for byte in text.utf8 {
            value ^= UInt64(byte)
            value = value &* 0x0000_0100_0000_01b3
        }
        // Unit separator keeps ["ab","c"] and ["a","bc"] apart.
        value ^= 0x1F
        return value &* 0x0000_0100_0000_01b3
    }

    public static func hex(_ value: UInt64) -> String {
        let digits = String(value, radix: 16)
        return String(repeating: "0", count: 16 - digits.count) + digits
    }
}

public struct MathProjectContext: Sendable {
    public struct Entry: Hashable, Sendable {
        public let fileID: String
        public let definition: MathDefinition
    }

    /// Bounds keep a pathological project from turning every render into a
    /// megabyte of definitions.
    public static let maximumDefinitions = 4_096
    public static let maximumDefinitionBytes = 512 * 1024
    public static let maximumFiles = 256

    public private(set) var entries: [Entry] = []
    /// `hashes[k]` identifies the first `k` entries.
    private var hashes: [UInt64] = [MathPreviewHash.basis]
    /// Per visited file: its own definitions/includes as (offset, entry index).
    private var checkpoints: [String: [(offset: Int, index: Int)]] = [:]
    private var segmentEnds: [String: Int] = [:]
    public private(set) var truncated = false
    /// Includes with no known resolution — the host should resolve them.
    public private(set) var unresolved: [MathIncludeRequest] = []
    public let rootFileID: String

    /// Depth-first in document order from `rootFileID`; `activeFileID` is
    /// appended after the root's tree when the root never reaches it (a
    /// chapter opened without its main file resolved, or a standalone file).
    /// `resolutions[key] == .some(nil)` marks a known-missing target.
    /// `activeScan` is the in-memory scan of `activeFileID` — it wins over
    /// a stale on-disk entry in `scans` without cloning that map (C11).
    public init(
        rootFileID: String,
        activeFileID: String?,
        activeScan: MathSourceScan?,
        scans: [String: MathSourceScan],
        resolutions: [MathIncludeKey: String?]
    ) {
        self.rootFileID = rootFileID
        var defined = Set<String>()
        var bytes = 0
        var visited = Set<String>()
        var requested = Set<MathIncludeKey>()

        func visit(_ fileID: String) {
            guard visited.count < Self.maximumFiles, visited.insert(fileID).inserted else { return }
            // The active file's in-memory scan wins over any on-disk copy.
            guard let scan = (fileID == activeFileID ? activeScan : nil) ?? scans[fileID] else { return }
            var marks: [(offset: Int, index: Int)] = []
            var definitionIndex = 0, includeIndex = 0
            while definitionIndex < scan.definitions.count || includeIndex < scan.includes.count {
                let takeDefinition = includeIndex >= scan.includes.count
                    || (definitionIndex < scan.definitions.count
                        && scan.definitions[definitionIndex].range.utf8Offset < scan.includes[includeIndex].utf8Offset)
                if takeDefinition {
                    let definition = scan.definitions[definitionIndex]
                    definitionIndex += 1
                    marks.append((definition.range.utf8Offset, entries.count))
                    append(definition, fileID: fileID)
                } else {
                    let include = scan.includes[includeIndex]
                    includeIndex += 1
                    marks.append((include.utf8Offset, entries.count))
                    let key = MathIncludeKey(fromFileID: fileID, target: include.target)
                    if let resolution = resolutions[key] {
                        if let child = resolution { visit(child) }
                    } else if requested.insert(key).inserted {
                        unresolved.append(MathIncludeRequest(
                            key: key,
                            candidates: MathIncludePaths.candidates(target: include.target, from: fileID, root: rootFileID)
                        ))
                    }
                }
            }
            checkpoints[fileID] = marks
            segmentEnds[fileID] = entries.count
        }

        func append(_ definition: MathDefinition, fileID: String) {
            guard !truncated else { return }
            switch definition.kind {
            case .provideCommand:
                // LaTeX keeps an existing definition; only a first one applies.
                guard defined.insert(definition.name).inserted else { return }
            case .newCommand, .renewCommand, .declareMathOperator:
                defined.insert(definition.name)
            case .newEnvironment, .renewEnvironment:
                break // environments share no namespace with commands
            }
            bytes += definition.mathJaxSource.utf8.count
            guard entries.count < Self.maximumDefinitions, bytes <= Self.maximumDefinitionBytes else {
                truncated = true
                return
            }
            entries.append(Entry(fileID: fileID, definition: definition))
            hashes.append(MathPreviewHash.combine(hashes[hashes.count - 1], definition.mathJaxSource))
        }

        visit(rootFileID)
        if let activeFileID { visit(activeFileID) }
    }

    /// Number of entries TeX has read before `utf8Offset` in `fileID`.
    public func visibleCount(fileID: String, utf8Offset: Int) -> Int {
        guard let marks = checkpoints[fileID] else { return entries.count }
        var lo = 0, hi = marks.count
        while lo < hi {
            let mid = (lo + hi) / 2
            if marks[mid].offset < utf8Offset { lo = mid + 1 } else { hi = mid }
        }
        return lo < marks.count ? marks[lo].index : segmentEnds[fileID] ?? entries.count
    }

    /// The fast-preview context for a region starting at `utf8Offset`.
    public func context(fileID: String, utf8Offset: Int) -> MathPreviewContext {
        let visible = visibleCount(fileID: fileID, utf8Offset: utf8Offset)
        return MathPreviewContext(
            definitions: entries[0..<visible].map(\.definition.mathJaxSource),
            key: MathPreviewHash.hex(hashes[visible]),
            truncated: truncated
        )
    }

    /// Definitions the exact document must restate after `\begin{document}`:
    /// those TeX reads after the root preamble and before the region. The
    /// root's preamble (with its own `\input`s) runs verbatim, so earlier
    /// entries are already in effect there.
    public func bodyDefinitions(fileID: String, utf8Offset: Int, rootDocumentBegin: Int?) -> [String] {
        let visible = visibleCount(fileID: fileID, utf8Offset: utf8Offset)
        let start = rootDocumentBegin.map { visibleCount(fileID: rootFileID, utf8Offset: $0) } ?? 0
        guard start < visible else { return [] }
        return entries[start..<visible].map(\.definition.originalSource)
    }
}

/// Include-target resolution candidates, TeX-style: relative to the main
/// document's directory first (TeX's working directory), then to the
/// including file's; `.tex` is implied for extensionless targets.
public enum MathIncludePaths {
    public static func candidates(target: String, from fileID: String, root rootFileID: String) -> [String] {
        let name = hasExtension(target) ? target : target + ".tex"
        if name.hasPrefix("/") { return [normalize(name)] }
        var result: [String] = []
        for directory in [directoryName(rootFileID), directoryName(fileID)] {
            let path = normalize(directory.isEmpty ? name : directory + "/" + name)
            if !result.contains(path) { result.append(path) }
        }
        return result
    }

    static func hasExtension(_ path: String) -> Bool {
        let last = path.split(separator: "/", omittingEmptySubsequences: false).last ?? ""
        guard let dot = last.lastIndex(of: ".") else { return false }
        return dot != last.startIndex
    }

    static func directoryName(_ path: String) -> String {
        guard let slash = path.lastIndex(of: "/") else { return "" }
        return slash == path.startIndex ? "/" : String(path[..<slash])
    }

    static func normalize(_ path: String) -> String {
        let absolute = path.hasPrefix("/")
        var components: [Substring] = []
        for component in path.split(separator: "/", omittingEmptySubsequences: true) {
            if component == "." { continue }
            if component == "..", let last = components.last, last != ".." {
                components.removeLast()
            } else if component == "..", absolute {
                continue
            } else {
                components.append(component)
            }
        }
        let joined = components.joined(separator: "/")
        return absolute ? "/" + joined : joined
    }
}

/// The minimal document for the optional exact TeX preview: the project's
/// own preamble (or a plain AMS one when the file has none), definitions TeX
/// would have read before the region, then the region exactly as written.
public enum ExactEquationDocument {
    public static let fallbackPreamble = "\\documentclass{article}\n\\usepackage{amsmath,amssymb}\n"

    public static func make(preamble: String?, preambleDefinitions: [String], bodyDefinitions: [String], region: String) -> String {
        var document = preamble ?? fallbackPreamble
        if !document.hasSuffix("\n") { document += "\n" }
        for definition in preambleDefinitions { document += definition + "\n" }
        document += "\\begin{document}\n\\thispagestyle{empty}\n"
        for definition in bodyDefinitions { document += definition + "\n" }
        document += "\\noindent\n" + region + "\n\\end{document}\n"
        return document
    }
}
