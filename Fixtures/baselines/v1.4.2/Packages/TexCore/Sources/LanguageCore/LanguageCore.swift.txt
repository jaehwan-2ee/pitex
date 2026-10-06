public enum LanguageCoreError: Error, Equatable, Sendable {
    case emptySourceIdentifier
    case invalidRange
    case staleResult(expected: SnapshotRevision, actual: SnapshotRevision)
}

public struct SourceRange: Hashable, Codable, Sendable {
    public let utf8Offset: Int
    public let utf8Length: Int

    public init(utf8Offset: Int, utf8Length: Int) throws {
        guard utf8Offset >= 0, utf8Length >= 0 else { throw LanguageCoreError.invalidRange }
        self.utf8Offset = utf8Offset
        self.utf8Length = utf8Length
    }

    private init(validatedUTF8Offset: Int, utf8Length: Int) {
        self.utf8Offset = validatedUTF8Offset
        self.utf8Length = utf8Length
    }

    public var endUTF8Offset: Int { utf8Offset + utf8Length }

    fileprivate static func lexerRange(offset: Int, length: Int) -> Self {
        Self(validatedUTF8Offset: offset, utf8Length: length)
    }
}

public enum LanguageTokenKind: Hashable, Codable, Sendable {
    case controlSequence(String)
    case comment(String)
    case leftBrace
    case rightBrace
    case whitespace(String)
    case text(String)
    case bibEntryMarker
    case punctuation(String)
    /// The environment name inside \begin{…} / \end{…} (e.g. `document`).
    case environmentName(String)
    /// A complete math region: $…$, $$…$$, \(…\), or \[…\], delimiters included.
    case math(String)
}

public struct LanguageToken: Hashable, Codable, Sendable {
    public let kind: LanguageTokenKind
    public let range: SourceRange

    public init(kind: LanguageTokenKind, range: SourceRange) {
        self.kind = kind; self.range = range
    }
}

public enum TeXDialect: String, Codable, Sendable { case latex, bibtex }

public enum DeterministicTeXLexer {
    public static func tokenize(_ source: String, dialect: TeXDialect = .latex) -> [LanguageToken] {
        let scalars = Array(source.unicodeScalars)
        var scalarOffsets: [Int] = []
        scalarOffsets.reserveCapacity(scalars.count + 1)
        var offset = 0
        for scalar in scalars {
            scalarOffsets.append(offset)
            offset += scalar.utf8.count
        }
        scalarOffsets.append(offset)

        func token(_ kind: LanguageTokenKind, _ start: Int, _ end: Int) -> LanguageToken {
            LanguageToken(kind: kind, range: .lexerRange(offset: scalarOffsets[start], length: scalarOffsets[end] - scalarOffsets[start]))
        }
        func string(_ start: Int, _ end: Int) -> String {
            String(String.UnicodeScalarView(scalars[start..<end]))
        }
        func isLetter(_ scalar: Unicode.Scalar) -> Bool {
            (scalar.value >= 65 && scalar.value <= 90) || (scalar.value >= 97 && scalar.value <= 122)
        }
        func isWhitespace(_ scalar: Unicode.Scalar) -> Bool {
            scalar == " " || scalar == "\t" || scalar == "\r" || scalar == "\n"
        }

        var result: [LanguageToken] = []
        var index = 0
        while index < scalars.count {
            let start = index
            let scalar = scalars[index]
            if scalar == "%" {
                index += 1
                while index < scalars.count, scalars[index] != "\n", scalars[index] != "\r" { index += 1 }
                result.append(token(.comment(string(start, index)), start, index))
            } else if scalar == "\\" {
                index += 1
                if index < scalars.count {
                    if isLetter(scalars[index]) {
                        while index < scalars.count, isLetter(scalars[index]) { index += 1 }
                    } else {
                        index += 1
                    }
                }
                let name = string(start + 1, index)
                if name == "(" || name == "[" {
                    // Display math \( … \) / \[ … \]: scan for the matching
                    // closer, skipping escaped characters.
                    let closer: Unicode.Scalar = name == "(" ? ")" : "]"
                    var scan = index
                    var closed = false
                    while scan < scalars.count {
                        if scalars[scan] == "\\", scan + 1 < scalars.count {
                            if scalars[scan + 1] == closer { closed = true; scan += 2; break }
                            scan += 2; continue
                        }
                        scan += 1
                    }
                    if closed {
                        result.append(token(.math(string(start, scan)), start, scan))
                        index = scan
                    } else {
                        result.append(token(.controlSequence(name), start, index))
                    }
                } else {
                    result.append(token(.controlSequence(name), start, index))
                    if dialect == .latex, name == "begin" || name == "end" {
                        // \begin{env} / \end{env}: the name inside the braces
                        // gets its own token so the highlighter can paint it
                        // with the environment color like the reference editor.
                        var probe = index
                        while probe < scalars.count, scalars[probe] == " " || scalars[probe] == "\t" { probe += 1 }
                        if probe < scalars.count, scalars[probe] == "{" {
                            if probe > index {
                                result.append(token(.whitespace(string(index, probe)), index, probe))
                            }
                            result.append(token(.leftBrace, probe, probe + 1))
                            var cursor = probe + 1
                            while cursor < scalars.count,
                                  scalars[cursor] != "}",
                                  scalars[cursor] != "\n",
                                  scalars[cursor] != "\r" { cursor += 1 }
                            if cursor > probe + 1 {
                                result.append(token(.environmentName(string(probe + 1, cursor)), probe + 1, cursor))
                            }
                            // The closing brace is emitted by the main loop.
                            index = cursor
                        }
                    }
                }
            } else if scalar == "$" {
                // Inline/display math $…$ / $$…$$. An unmatched opening
                // delimiter emits only the delimiter itself as math so the
                // rest of the file keeps its normal coloring.
                let isDouble = index + 1 < scalars.count && scalars[index + 1] == "$"
                index += isDouble ? 2 : 1
                var scan = index
                var end = -1
                while scan < scalars.count {
                    if scalars[scan] == "\\" { scan += 2; continue }
                    if scalars[scan] == "$" {
                        if isDouble {
                            if scan + 1 < scalars.count, scalars[scan + 1] == "$" { end = scan + 2; break }
                            scan += 1; continue
                        }
                        end = scan + 1; break
                    }
                    scan += 1
                }
                if end > 0 {
                    result.append(token(.math(string(start, end)), start, end))
                    index = end
                } else {
                    result.append(token(.math(string(start, index)), start, index))
                }
            } else if scalar == "{" || scalar == "[" || (dialect == .latex && scalar == "(") {
                index += 1; result.append(token(.leftBrace, start, index))
            } else if scalar == "}" || scalar == "]" || (dialect == .latex && scalar == ")") {
                index += 1; result.append(token(.rightBrace, start, index))
            } else if isWhitespace(scalar) {
                index += 1
                while index < scalars.count, isWhitespace(scalars[index]) { index += 1 }
                result.append(token(.whitespace(string(start, index)), start, index))
            } else if dialect == .bibtex, scalar == "@" {
                index += 1; result.append(token(.bibEntryMarker, start, index))
            } else if dialect == .bibtex, [",", "=", "(", ")", "#", "\""].contains(scalar) {
                index += 1; result.append(token(.punctuation(String(scalar)), start, index))
            } else {
                index += 1
                while index < scalars.count {
                    let next = scalars[index]
                    if next == "%" || next == "\\" || next == "{" || next == "}" || next == "[" || next == "]" || next == "$" || isWhitespace(next) { break }
                    if dialect == .bibtex, next == "@" || [",", "=", "(", ")", "#", "\""].contains(next) { break }
                    if dialect == .latex, next == "(" || next == ")" { break }
                    index += 1
                }
                result.append(token(.text(string(start, index)), start, index))
            }
        }
        return result
    }
}

public enum OutlineKind: String, Codable, Sendable { case part, chapter, section, subsection, subsubsection, bibliography }

public struct OutlineRecord: Hashable, Codable, Sendable {
    public let kind: OutlineKind
    public let title: String
    public let range: SourceRange
    public let depth: Int

    public init(kind: OutlineKind, title: String, range: SourceRange, depth: Int) throws {
        guard !title.isEmpty, depth >= 0 else { throw LanguageCoreError.invalidRange }
        self.kind = kind; self.title = title; self.range = range; self.depth = depth
    }
}

public enum ReferenceKind: String, Codable, Sendable { case label, reference, citation, bibliographyEntry }

public struct ReferenceRecord: Hashable, Codable, Sendable {
    public let kind: ReferenceKind
    public let key: String
    public let range: SourceRange

    public init(kind: ReferenceKind, key: String, range: SourceRange) throws {
        guard !key.isEmpty else { throw LanguageCoreError.emptySourceIdentifier }
        self.kind = kind; self.key = key; self.range = range
    }
}

public struct SnapshotRevision: Hashable, Codable, Sendable, Comparable {
    public let sourceID: String
    public let revision: UInt64
    public let contentFingerprint: UInt64

    public init(sourceID: String, revision: UInt64, source: String) throws {
        guard !sourceID.isEmpty else { throw LanguageCoreError.emptySourceIdentifier }
        self.sourceID = sourceID; self.revision = revision
        var hash: UInt64 = 14_695_981_039_346_656_037
        for byte in source.utf8 { hash ^= UInt64(byte); hash &*= 1_099_511_628_211 }
        self.contentFingerprint = hash
    }

    public static func < (lhs: Self, rhs: Self) -> Bool {
        (lhs.sourceID, lhs.revision, lhs.contentFingerprint) < (rhs.sourceID, rhs.revision, rhs.contentFingerprint)
    }
}

public struct RevisionBound<Value: Hashable & Codable & Sendable>: Hashable, Codable, Sendable {
    public let revision: SnapshotRevision
    public let value: Value

    public init(revision: SnapshotRevision, value: Value) {
        self.revision = revision; self.value = value
    }

    public func value(for current: SnapshotRevision) throws -> Value {
        guard revision == current else { throw LanguageCoreError.staleResult(expected: current, actual: revision) }
        return value
    }
}
