import Foundation

public enum SyncTeXQueryError: Error, Equatable, Sendable {
    case malformed(line: Int)
    case pathOutsideRoot(line: Int)
    case staleResult
    case noMatch
    case ambiguousMatch(count: Int)
}

public struct SyncTeXOutputBinding: Hashable, Codable, Sendable {
    public let revision: SyncTeXRevision
    public let outputHash: String

    public init(revision: SyncTeXRevision, outputHash: String) throws {
        guard !outputHash.isEmpty else { throw SyncTeXQueryError.malformed(line: 0) }
        self.revision = revision
        self.outputHash = outputHash
    }
}

public struct SyncTeXQueryMetadata: Hashable, Codable, Sendable {
    public let version: Int
    public let fingerprint: UInt64

    public init(version: Int, fingerprint: UInt64) {
        self.version = version
        self.fingerprint = fingerprint
    }
}

public struct SyncTeXQueryCandidate: Hashable, Codable, Sendable {
    public let binding: SyncTeXOutputBinding
    public let metadata: SyncTeXQueryMetadata
    public let source: SourceLocation
    public let pdf: PDFLocation
    public let h: Double
    public let v: Double
    public let width: Double
    public let height: Double

    public var match: SyncTeXMatch {
        SyncTeXMatch(revision: binding.revision, source: source, pdf: pdf, exact: true)
    }
}

public struct SyncTeXQueryDocument: Hashable, Codable, Sendable {
    public let metadata: SyncTeXQueryMetadata
    public let candidates: [SyncTeXQueryCandidate]
}

/// Parser for the normalized, line-oriented output of `synctex view` and
/// `synctex edit`. Normalization adds the queried Input/Line/Column and
/// Page/x/y values to each result block, but does not change SyncTeX values.
public enum SyncTeXQueryParser {
    public static func parse(
        _ text: String,
        projectRoot: String,
        binding: SyncTeXOutputBinding
    ) throws -> SyncTeXQueryDocument {
        let root = try canonicalRoot(projectRoot)
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false).map {
            $0.last == "\r" ? String($0.dropLast()) : String($0)
        }
        var version: Int?
        var fingerprint: UInt64?
        var blocks: [(start: Int, fields: [String: (String, Int)])] = []
        var blockStart: Int?
        var fields: [String: (String, Int)] = [:]

        for (offset, line) in lines.enumerated() {
            let number = offset + 1
            if line == "SyncTeX result begin" {
                guard blockStart == nil else { throw SyncTeXQueryError.malformed(line: number) }
                blockStart = number
                fields = [:]
                continue
            }
            if line == "SyncTeX result end" {
                guard let start = blockStart else { throw SyncTeXQueryError.malformed(line: number) }
                blocks.append((start, fields))
                blockStart = nil
                fields = [:]
                continue
            }
            if blockStart != nil {
                guard !line.isEmpty, let separator = line.firstIndex(of: ":") else {
                    throw SyncTeXQueryError.malformed(line: number)
                }
                let key = String(line[..<separator])
                let value = String(line[line.index(after: separator)...])
                let allowed = Set(["Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H"])
                guard allowed.contains(key), fields[key] == nil else {
                    throw SyncTeXQueryError.malformed(line: number)
                }
                fields[key] = (value, number)
            } else if line.hasPrefix("SyncTeX Version:") {
                guard version == nil,
                      let parsed = strictPositiveInt(String(line.dropFirst("SyncTeX Version:".count))) else {
                    throw SyncTeXQueryError.malformed(line: number)
                }
                version = parsed
            } else if line.hasPrefix("SyncTeX Fingerprint:") {
                let value = String(line.dropFirst("SyncTeX Fingerprint:".count))
                guard fingerprint == nil, isASCIIDigits(value), let parsed = UInt64(value) else {
                    throw SyncTeXQueryError.malformed(line: number)
                }
                fingerprint = parsed
            } else if !line.isEmpty {
                throw SyncTeXQueryError.malformed(line: number)
            }
        }
        guard blockStart == nil else { throw SyncTeXQueryError.malformed(line: blockStart!) }
        guard let version, let fingerprint else { throw SyncTeXQueryError.malformed(line: 1) }
        guard fingerprint == binding.revision.fingerprint else {
            throw SyncTeXQueryError.staleResult
        }
        guard !blocks.isEmpty else { throw SyncTeXQueryError.noMatch }

        let metadata = SyncTeXQueryMetadata(version: version, fingerprint: fingerprint)
        var candidates: [SyncTeXQueryCandidate] = []
        candidates.reserveCapacity(blocks.count)
        for block in blocks {
            let required = ["Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H"]
            guard required.allSatisfy({ block.fields[$0] != nil }) else {
                throw SyncTeXQueryError.malformed(line: block.start)
            }
            let input = block.fields["Input"]!
            let output = block.fields["Output"]!
            let sourcePath = try canonicalProjectPath(input.0, root: root, line: input.1)
            let pdfPath = try canonicalProjectPath(output.0, root: root, line: output.1)
            guard let line = strictPositiveInt(block.fields["Line"]!.0),
                  let column = strictNonnegativeInt(block.fields["Column"]!.0),
                  let page = strictPositiveInt(block.fields["Page"]!.0),
                  let x = strictDouble(block.fields["x"]!.0),
                  let y = strictDouble(block.fields["y"]!.0),
                  let h = strictDouble(block.fields["h"]!.0),
                  let v = strictDouble(block.fields["v"]!.0),
                  let width = strictDouble(block.fields["W"]!.0), width >= 0,
                  let height = strictDouble(block.fields["H"]!.0), height >= 0 else {
                throw SyncTeXQueryError.malformed(line: block.start)
            }
            do {
                let source = try SourceLocation(path: sourcePath, line: line, column: column)
                let pdf = try PDFLocation(pdfPath: pdfPath, page: page, point: PDFPoint(x: x, y: y))
                candidates.append(SyncTeXQueryCandidate(
                    binding: binding,
                    metadata: metadata,
                    source: source,
                    pdf: pdf,
                    h: h,
                    v: v,
                    width: width,
                    height: height
                ))
            } catch {
                throw SyncTeXQueryError.malformed(line: block.start)
            }
        }
        return SyncTeXQueryDocument(metadata: metadata, candidates: candidates)
    }

    private static func canonicalRoot(_ raw: String) throws -> [String] {
        guard raw.hasPrefix("/"), !raw.contains("\\"), !raw.contains("\0") else {
            throw SyncTeXQueryError.pathOutsideRoot(line: 0)
        }
        let components = raw.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
        guard !components.isEmpty, !components.contains("..") else {
            throw SyncTeXQueryError.pathOutsideRoot(line: 0)
        }
        return components.filter { $0 != "." }
    }

    private static func canonicalProjectPath(
        _ raw: String,
        root: [String],
        line: Int
    ) throws -> NormalizedSourcePath {
        guard !raw.isEmpty, !raw.contains("\\"), !raw.contains("\0") else {
            throw SyncTeXQueryError.pathOutsideRoot(line: line)
        }
        let components = raw.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
        guard !components.contains("..") else { throw SyncTeXQueryError.pathOutsideRoot(line: line) }
        let clean = components.filter { $0 != "." }
        let relative: ArraySlice<String>
        if raw.hasPrefix("/") {
            guard clean.count > root.count, clean.prefix(root.count).elementsEqual(root) else {
                throw SyncTeXQueryError.pathOutsideRoot(line: line)
            }
            relative = clean.dropFirst(root.count)
        } else {
            guard !clean.isEmpty else { throw SyncTeXQueryError.pathOutsideRoot(line: line) }
            relative = clean[...]
        }
        do { return try NormalizedSourcePath(relative.joined(separator: "/")) }
        catch { throw SyncTeXQueryError.pathOutsideRoot(line: line) }
    }

    private static func isASCIIDigits(_ value: String) -> Bool {
        !value.isEmpty && value.utf8.allSatisfy { $0 >= 48 && $0 <= 57 }
    }

    private static func strictPositiveInt(_ value: String) -> Int? {
        guard isASCIIDigits(value), let result = Int(value), result > 0 else { return nil }
        return result
    }

    private static func strictNonnegativeInt(_ value: String) -> Int? {
        guard isASCIIDigits(value) else { return nil }
        return Int(value)
    }

    private static func strictDouble(_ value: String) -> Double? {
        guard !value.isEmpty,
              value.utf8.allSatisfy({ ($0 >= 48 && $0 <= 57) || $0 == 43 || $0 == 45 || $0 == 46 || $0 == 69 || $0 == 101 }),
              let result = Double(value), result.isFinite else { return nil }
        return result
    }
}

public enum ExactSyncTeXQuerySelector {
    public static func forward(
        _ candidates: [SyncTeXQueryCandidate],
        query: ForwardSyncQuery,
        outputHash: String
    ) throws -> SyncTeXQueryCandidate {
        let pathMatches = candidates.filter {
            $0.source == query.source && $0.pdf.pdfPath == query.expectedPDF
        }
        return try selectBound(pathMatches, revision: query.revision, outputHash: outputHash)
    }

    public static func inverse(
        _ candidates: [SyncTeXQueryCandidate],
        query: InverseSyncQuery,
        outputHash: String,
        coordinateEpsilon: Double = 0
    ) throws -> SyncTeXQueryCandidate {
        guard coordinateEpsilon.isFinite, coordinateEpsilon >= 0 else {
            throw SyncTeXQueryError.malformed(line: 0)
        }
        let pageMatches = candidates.filter {
            $0.pdf.pdfPath == query.pdf.pdfPath && $0.pdf.page == query.pdf.page
        }
        guard !pageMatches.isEmpty else { throw SyncTeXQueryError.noMatch }
        let bound = pageMatches.filter {
            $0.binding.revision == query.revision && $0.binding.outputHash == outputHash
        }
        guard !bound.isEmpty else { throw SyncTeXQueryError.staleResult }
        let coordinateMatches = bound.filter {
            abs($0.pdf.point.x - query.pdf.point.x) <= coordinateEpsilon &&
            abs($0.pdf.point.y - query.pdf.point.y) <= coordinateEpsilon
        }
        guard !coordinateMatches.isEmpty else { throw SyncTeXQueryError.noMatch }
        guard coordinateMatches.count == 1 else {
            throw SyncTeXQueryError.ambiguousMatch(count: coordinateMatches.count)
        }
        return coordinateMatches[0]
    }

    private static func selectBound(
        _ candidates: [SyncTeXQueryCandidate],
        revision: SyncTeXRevision,
        outputHash: String
    ) throws -> SyncTeXQueryCandidate {
        guard !candidates.isEmpty else { throw SyncTeXQueryError.noMatch }
        let bound = candidates.filter {
            $0.binding.revision == revision && $0.binding.outputHash == outputHash
        }
        guard !bound.isEmpty else { throw SyncTeXQueryError.staleResult }
        guard bound.count == 1 else { throw SyncTeXQueryError.ambiguousMatch(count: bound.count) }
        return bound[0]
    }
}
