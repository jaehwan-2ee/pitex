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
        // Syntax delimiters are ASCII. Scan UTF-8 directly instead of
        // allocating a scalar array and a byte-offset array for every edit.
        let bytes = Array(source.utf8)
        func token(_ kind: LanguageTokenKind, _ start: Int, _ end: Int) -> LanguageToken {
            LanguageToken(kind: kind, range: .lexerRange(offset: start, length: end - start))
        }
        func string(_ start: Int, _ end: Int) -> String {
            String(decoding: bytes[start..<end], as: UTF8.self)
        }
        func isLetter(_ byte: UInt8) -> Bool {
            (byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122)
        }
        func isWhitespace(_ byte: UInt8) -> Bool {
            byte == 32 || byte == 9 || byte == 13 || byte == 10
        }
        // A failed search proves this closer is absent from the remainder.
        // Repeated unfinished \( / \[ must not rescan the suffix quadratically.
        var missingMathClosers = Set<UInt8>()

        var result: [LanguageToken] = []
        var index = 0
        while index < bytes.count {
            let start = index
            let byte = bytes[index]
            if byte == 37 {
                index += 1
                while index < bytes.count, bytes[index] != 10, bytes[index] != 13 { index += 1 }
                result.append(token(.comment(string(start, index)), start, index))
            } else if byte == 92 {
                index += 1
                if index < bytes.count {
                    if isLetter(bytes[index]) {
                        while index < bytes.count, isLetter(bytes[index]) { index += 1 }
                    } else {
                        index += 1
                        while index < bytes.count, bytes[index] & 0xC0 == 0x80 { index += 1 }
                    }
                }
                let name = string(start + 1, index)
                if name == "(" || name == "[" {
                    // Display math \( … \) / \[ … \]: scan for the matching
                    // closer, skipping escaped characters.
                    let closer: UInt8 = name == "(" ? 41 : 93
                    var scan = missingMathClosers.contains(closer) ? bytes.count : index
                    var closed = false
                    while scan < bytes.count {
                        if bytes[scan] == 92, scan + 1 < bytes.count {
                            if bytes[scan + 1] == closer { closed = true; scan += 2; break }
                            scan += 2
                            while scan < bytes.count, bytes[scan] & 0xC0 == 0x80 { scan += 1 }
                            continue
                        }
                        scan += 1
                    }
                    if closed {
                        result.append(token(.math(string(start, scan)), start, scan))
                        index = scan
                    } else {
                        missingMathClosers.insert(closer)
                        result.append(token(.controlSequence(name), start, index))
                    }
                } else {
                    result.append(token(.controlSequence(name), start, index))
                    if dialect == .latex, name == "begin" || name == "end" {
                        // \begin{env} / \end{env}: the name inside the braces
                        // gets its own token so the highlighter can paint it
                        // with the environment color like the reference editor.
                        var probe = index
                        while probe < bytes.count, bytes[probe] == 32 || bytes[probe] == 9 { probe += 1 }
                        if probe < bytes.count, bytes[probe] == 123 {
                            if probe > index {
                                result.append(token(.whitespace(string(index, probe)), index, probe))
                            }
                            result.append(token(.leftBrace, probe, probe + 1))
                            var cursor = probe + 1
                            while cursor < bytes.count,
                                  bytes[cursor] != 125,
                                  bytes[cursor] != 10,
                                  bytes[cursor] != 13 { cursor += 1 }
                            if cursor > probe + 1 {
                                result.append(token(.environmentName(string(probe + 1, cursor)), probe + 1, cursor))
                            }
                            // The closing brace is emitted by the main loop.
                            index = cursor
                        }
                    }
                }
            } else if byte == 36 {
                // Inline/display math $…$ / $$…$$. An unmatched opening
                // delimiter emits only the delimiter itself as math so the
                // rest of the file keeps its normal coloring.
                let isDouble = index + 1 < bytes.count && bytes[index + 1] == 36
                index += isDouble ? 2 : 1
                var scan = index
                var end = -1
                while scan < bytes.count {
                    if bytes[scan] == 92 {
                        scan += 2
                        while scan < bytes.count, bytes[scan] & 0xC0 == 0x80 { scan += 1 }
                        continue
                    }
                    if bytes[scan] == 36 {
                        if isDouble {
                            if scan + 1 < bytes.count, bytes[scan + 1] == 36 { end = scan + 2; break }
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
            } else if byte == 123 || byte == 91 || (dialect == .latex && byte == 40) {
                index += 1; result.append(token(.leftBrace, start, index))
            } else if byte == 125 || byte == 93 || (dialect == .latex && byte == 41) {
                index += 1; result.append(token(.rightBrace, start, index))
            } else if isWhitespace(byte) {
                index += 1
                while index < bytes.count, isWhitespace(bytes[index]) { index += 1 }
                result.append(token(.whitespace(string(start, index)), start, index))
            } else if dialect == .bibtex, byte == 64 {
                index += 1; result.append(token(.bibEntryMarker, start, index))
            } else if dialect == .bibtex, [44, 61, 40, 41, 35, 34].contains(byte) {
                index += 1; result.append(token(.punctuation(string(start, index)), start, index))
            } else {
                index += 1
                while index < bytes.count {
                    let next = bytes[index]
                    if next == 37 || next == 92 || next == 123 || next == 125 || next == 91 || next == 93 || next == 36 || isWhitespace(next) { break }
                    if dialect == .bibtex, next == 64 || [44, 61, 40, 41, 35, 34].contains(next) { break }
                    if dialect == .latex, next == 40 || next == 41 { break }
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
