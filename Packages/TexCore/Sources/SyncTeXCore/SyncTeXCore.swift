import Foundation

public enum SyncTeXError: Error, Equatable, Sendable {
    case emptyPath
    case invalidCoordinate
    case invalidLine
    case invalidUTF8
    case malformedInput(line: Int)
    case staleResult
    case noMatch
    case ambiguousMatch(count: Int)
}

public struct SyncTeXRevision: Hashable, Codable, Sendable {
    public let buildID: String
    public let fingerprint: UInt64
    public init(buildID: String, fingerprint: UInt64) throws {
        guard !buildID.isEmpty else { throw SyncTeXError.emptyPath }
        self.buildID = buildID; self.fingerprint = fingerprint
    }
}

public struct NormalizedSourcePath: Hashable, Codable, Sendable, Comparable {
    public let value: String
    public init(_ path: String) throws {
        guard !path.isEmpty, !path.unicodeScalars.contains(where: { $0.value == 0 }) else { throw SyncTeXError.emptyPath }
        let absolute = path.hasPrefix("/")
        var components: [Substring] = []
        for component in path.split(separator: "/", omittingEmptySubsequences: true) {
            if component == "." { continue }
            if component == ".." {
                guard !components.isEmpty, components.last != ".." else { throw SyncTeXError.emptyPath }
                components.removeLast()
            } else { components.append(component) }
        }
        guard !components.isEmpty else { throw SyncTeXError.emptyPath }
        self.value = (absolute ? "/" : "") + components.joined(separator: "/")
    }
    public static func < (lhs: Self, rhs: Self) -> Bool { lhs.value < rhs.value }
}

public struct PDFPoint: Hashable, Codable, Sendable {
    public let x: Double
    public let y: Double
    public init(x: Double, y: Double) throws {
        guard x.isFinite, y.isFinite else { throw SyncTeXError.invalidCoordinate }
        self.x = x; self.y = y
    }
}

public struct SourceLocation: Hashable, Codable, Sendable {
    public let path: NormalizedSourcePath
    public let line: Int
    public let column: Int
    public init(path: NormalizedSourcePath, line: Int, column: Int = 0) throws {
        guard line > 0, column >= 0 else { throw SyncTeXError.invalidLine }
        self.path = path; self.line = line; self.column = column
    }
}

public struct PDFLocation: Hashable, Codable, Sendable {
    public let pdfPath: NormalizedSourcePath
    public let page: Int
    public let point: PDFPoint
    public init(pdfPath: NormalizedSourcePath, page: Int, point: PDFPoint) throws {
        guard page > 0 else { throw SyncTeXError.invalidLine }
        self.pdfPath = pdfPath; self.page = page; self.point = point
    }
}

public struct ForwardSyncQuery: Hashable, Codable, Sendable {
    public let revision: SyncTeXRevision
    public let source: SourceLocation
    public let expectedPDF: NormalizedSourcePath

    public init(revision: SyncTeXRevision, source: SourceLocation, expectedPDF: NormalizedSourcePath) {
        self.revision = revision; self.source = source; self.expectedPDF = expectedPDF
    }
}
public struct InverseSyncQuery: Hashable, Codable, Sendable {
    public let revision: SyncTeXRevision
    public let pdf: PDFLocation

    public init(revision: SyncTeXRevision, pdf: PDFLocation) {
        self.revision = revision; self.pdf = pdf
    }
}

public struct SyncTeXMatch: Hashable, Codable, Sendable {
    public let revision: SyncTeXRevision
    public let source: SourceLocation
    public let pdf: PDFLocation
    public let exact: Bool

    public init(revision: SyncTeXRevision, source: SourceLocation, pdf: PDFLocation, exact: Bool) {
        self.revision = revision; self.source = source; self.pdf = pdf; self.exact = exact
    }
}

public enum ExactMatchSelector {
    public static func forward(_ candidates: [SyncTeXMatch], query: ForwardSyncQuery) throws -> SyncTeXMatch {
        try one(candidates.filter { $0.source == query.source && $0.pdf.pdfPath == query.expectedPDF }, revision: query.revision)
    }
    public static func inverse(_ candidates: [SyncTeXMatch], query: InverseSyncQuery) throws -> SyncTeXMatch {
        try one(candidates.filter { $0.pdf == query.pdf }, revision: query.revision)
    }
    private static func one(_ matches: [SyncTeXMatch], revision: SyncTeXRevision) throws -> SyncTeXMatch {
        if matches.isEmpty { throw SyncTeXError.noMatch }
        let current = matches.filter { $0.revision == revision }
        guard !current.isEmpty else { throw SyncTeXError.staleResult }
        let exact = current.filter(\.exact)
        guard !exact.isEmpty else { throw SyncTeXError.noMatch }
        guard exact.count == 1 else { throw SyncTeXError.ambiguousMatch(count: exact.count) }
        return exact[0]
    }
}

public struct SyncTeXInput: Hashable, Codable, Sendable {
    public let tag: Int
    public let path: NormalizedSourcePath
    public init(tag: Int, path: NormalizedSourcePath) throws {
        guard tag >= 0 else { throw SyncTeXError.malformedInput(line: 0) }
        self.tag = tag; self.path = path
    }
}

public struct SyncTeXTextDocument: Hashable, Codable, Sendable {
    public let inputs: [SyncTeXInput]
    public let contentLines: [String]
}

public enum SyncTeXTextParser {
    /// Accepts the textual content emitted by SyncTeX, after any gzip decoding.
    public static func parse(decodedUTF8 bytes: [UInt8]) throws -> SyncTeXTextDocument {
        guard let text = String(bytes: bytes, encoding: .utf8) else { throw SyncTeXError.invalidUTF8 }
        return try parse(text)
    }

    public static func parse(_ text: String) throws -> SyncTeXTextDocument {
        var inputs: [SyncTeXInput] = []
        var seenTags: Set<Int> = []
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false)
        for (index, rawLine) in lines.enumerated() where rawLine.hasPrefix("Input:") {
            let body = rawLine.dropFirst("Input:".count)
            guard let separator = body.firstIndex(of: ":"), let tag = Int(body[..<separator]), seenTags.insert(tag).inserted else {
                throw SyncTeXError.malformedInput(line: index + 1)
            }
            let rawPath = String(body[body.index(after: separator)...]).trimmingCharacters(in: .whitespacesAndNewlines)
            do { inputs.append(try SyncTeXInput(tag: tag, path: NormalizedSourcePath(rawPath))) }
            catch { throw SyncTeXError.malformedInput(line: index + 1) }
        }
        guard !inputs.isEmpty else { throw SyncTeXError.malformedInput(line: 1) }
        return SyncTeXTextDocument(inputs: inputs.sorted { $0.tag < $1.tag }, contentLines: lines.map(String.init))
    }
}
