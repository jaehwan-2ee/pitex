public enum CoordinateMapError: Error, Equatable, Sendable {
    case invalidUTF8Offset(Int)
    case invalidUTF16Offset(Int)
    case invalidPosition(TextPosition)
}

public struct TextPosition: Hashable, Codable, Sendable {
    public let line: Int
    public let column: Int

    public init(line: Int, column: Int) {
        self.line = line
        self.column = column
    }
}

public struct UnicodeCoordinateMap: Hashable, Codable, Sendable {
    private struct Boundary: Hashable, Codable, Sendable {
        let utf8: Int
        let utf16: Int
        let position: TextPosition
    }

    public let utf8Count: Int
    public let utf16Count: Int
    private let boundaries: [Boundary]

    public init(_ source: String) {
        var result: [Boundary] = []
        var utf8 = 0
        var utf16 = 0
        var line = 0
        var column = 0
        result.append(Boundary(utf8: 0, utf16: 0, position: TextPosition(line: 0, column: 0)))

        for character in source {
            let text = String(character)
            utf8 += text.utf8.count
            utf16 += text.utf16.count
            if text == "\r\n" {
                line += 1
                column = 0
            } else {
                for scalar in text.unicodeScalars {
                    if scalar == "\n" || scalar == "\r" {
                        line += 1
                        column = 0
                    } else {
                        column += scalar.utf16.count
                    }
                }
            }
            result.append(Boundary(utf8: utf8, utf16: utf16, position: TextPosition(line: line, column: column)))
        }
        self.utf8Count = utf8
        self.utf16Count = utf16
        self.boundaries = result
    }

    public func utf16Offset(forUTF8Offset offset: Int) throws -> Int {
        guard let boundary = boundaries.first(where: { $0.utf8 == offset }) else {
            throw CoordinateMapError.invalidUTF8Offset(offset)
        }
        return boundary.utf16
    }

    public func utf8Offset(forUTF16Offset offset: Int) throws -> Int {
        guard let boundary = boundaries.first(where: { $0.utf16 == offset }) else {
            throw CoordinateMapError.invalidUTF16Offset(offset)
        }
        return boundary.utf8
    }

    public func position(forUTF8Offset offset: Int) throws -> TextPosition {
        guard let boundary = boundaries.first(where: { $0.utf8 == offset }) else {
            throw CoordinateMapError.invalidUTF8Offset(offset)
        }
        return boundary.position
    }

    public func position(forUTF16Offset offset: Int) throws -> TextPosition {
        guard let boundary = boundaries.first(where: { $0.utf16 == offset }) else {
            throw CoordinateMapError.invalidUTF16Offset(offset)
        }
        return boundary.position
    }

    public func utf8Offset(for position: TextPosition) throws -> Int {
        guard position.line >= 0, position.column >= 0,
              let boundary = boundaries.first(where: { $0.position == position }) else {
            throw CoordinateMapError.invalidPosition(position)
        }
        return boundary.utf8
    }

    public func utf16Offset(for position: TextPosition) throws -> Int {
        guard position.line >= 0, position.column >= 0,
              let boundary = boundaries.first(where: { $0.position == position }) else {
            throw CoordinateMapError.invalidPosition(position)
        }
        return boundary.utf16
    }
}

public struct SourceLocation: Hashable, Codable, Sendable {
    public let sourceID: String
    public let range: SourceRange

    public init(sourceID: String, range: SourceRange) {
        self.sourceID = sourceID
        self.range = range
    }
}

public struct IncludeRecord: Hashable, Codable, Sendable {
    public let target: String
    public let range: SourceRange

    public init(target: String, range: SourceRange) throws {
        guard !target.isEmpty else { throw LanguageCoreError.emptySourceIdentifier }
        self.target = target
        self.range = range
    }
}

public struct BibTeXEntry: Hashable, Codable, Sendable {
    public let type: String
    public let key: String
    public let range: SourceRange

    public init(type: String, key: String, range: SourceRange) throws {
        guard !type.isEmpty, !key.isEmpty else { throw LanguageCoreError.emptySourceIdentifier }
        self.type = type
        self.key = key
        self.range = range
    }
}

public enum LanguageDiagnostic: Hashable, Codable, Sendable {
    case malformedGroup(sourceID: String, command: String, range: SourceRange)
    case duplicateLabel(key: String, definitions: [SourceLocation])
    case duplicateCitation(key: String, definitions: [SourceLocation])
    case missingLabel(key: String, use: SourceLocation)
    case missingCitation(key: String, use: SourceLocation)
    case includeCycle([String])
}

public enum MinimapRangeKind: String, Hashable, Codable, Sendable {
    case outline
    case include
    case definition
    case use
    case bibliography
}

public struct MinimapRange: Hashable, Codable, Sendable {
    public let kind: MinimapRangeKind
    public let range: SourceRange

    public init(kind: MinimapRangeKind, range: SourceRange) {
        self.kind = kind
        self.range = range
    }
}

public enum CompletionKind: String, Hashable, Codable, Sendable {
    case command
    case label
    case citation
}

public struct LanguageCompletion: Hashable, Codable, Sendable {
    public let text: String
    public let kind: CompletionKind

    public init(text: String, kind: CompletionKind) {
        self.text = text
        self.kind = kind
    }
}

public struct LanguageFileSnapshot: Hashable, Codable, Sendable {
    public let revision: SnapshotRevision
    public let source: String
    public let dialect: TeXDialect
    public let tokens: [LanguageToken]
    public let outline: [OutlineRecord]
    public let includes: [IncludeRecord]
    public let references: [ReferenceRecord]
    public let bibliography: [BibTeXEntry]
    public let minimapRanges: [MinimapRange]
    public let diagnostics: [LanguageDiagnostic]

    public var sourceID: String { revision.sourceID }
    public var coordinates: UnicodeCoordinateMap { UnicodeCoordinateMap(source) }

    public init(sourceID: String, revision: UInt64, source: String, dialect: TeXDialect = .latex) throws {
        self.revision = try SnapshotRevision(sourceID: sourceID, revision: revision, source: source)
        self.source = source
        self.dialect = dialect
        self.tokens = DeterministicTeXLexer.tokenize(source, dialect: dialect)
        let parsed = try LanguageParser.parse(sourceID: sourceID, source: source, dialect: dialect)
        self.outline = parsed.outline
        self.includes = parsed.includes
        self.references = parsed.references
        self.bibliography = parsed.bibliography
        self.diagnostics = parsed.diagnostics
        self.minimapRanges = parsed.minimap.sorted {
            ($0.range.utf8Offset, $0.range.utf8Length, $0.kind.rawValue) <
            ($1.range.utf8Offset, $1.range.utf8Length, $1.kind.rawValue)
        }
    }
}

public struct ProjectOutlineRecord: Hashable, Codable, Sendable {
    public let sourceID: String
    public let record: OutlineRecord

    public init(sourceID: String, record: OutlineRecord) {
        self.sourceID = sourceID
        self.record = record
    }
}

public struct ProjectLanguageIndex: Hashable, Codable, Sendable {
    public let snapshots: [String: LanguageFileSnapshot]

    public init(snapshots: [LanguageFileSnapshot] = []) throws {
        var result: [String: LanguageFileSnapshot] = [:]
        for snapshot in snapshots.sorted(by: { $0.sourceID < $1.sourceID }) {
            if let current = result[snapshot.sourceID], current.revision != snapshot.revision {
                throw LanguageCoreError.staleResult(expected: current.revision, actual: snapshot.revision)
            }
            result[snapshot.sourceID] = snapshot
        }
        self.snapshots = result
    }

    private init(validated snapshots: [String: LanguageFileSnapshot]) {
        self.snapshots = snapshots
    }

    public func replacing(_ snapshot: LanguageFileSnapshot) throws -> ProjectLanguageIndex {
        if let current = snapshots[snapshot.sourceID] {
            if current.revision == snapshot.revision { return self }
            guard snapshot.revision.revision > current.revision.revision else {
                throw LanguageCoreError.staleResult(expected: current.revision, actual: snapshot.revision)
            }
        }
        var result = snapshots
        result[snapshot.sourceID] = snapshot
        return ProjectLanguageIndex(validated: result)
    }

    public func snapshot(sourceID: String, revision: SnapshotRevision) throws -> LanguageFileSnapshot {
        guard let snapshot = snapshots[sourceID] else {
            throw LanguageCoreError.staleResult(expected: revision, actual: revision)
        }
        guard snapshot.revision == revision else {
            throw LanguageCoreError.staleResult(expected: snapshot.revision, actual: revision)
        }
        return snapshot
    }

    public var references: [(sourceID: String, record: ReferenceRecord)] {
        snapshots.keys.sorted().flatMap { sourceID in
            snapshots[sourceID]!.references.map { (sourceID, $0) }
        }
    }

    public var bibliography: [(sourceID: String, entry: BibTeXEntry)] {
        snapshots.keys.sorted().flatMap { sourceID in
            snapshots[sourceID]!.bibliography.map { (sourceID, $0) }
        }
    }

    public var diagnostics: [LanguageDiagnostic] {
        var result = snapshots.keys.sorted().flatMap { snapshots[$0]!.diagnostics }
        var labels: [String: [SourceLocation]] = [:]
        var citations: [String: [SourceLocation]] = [:]
        var labelUses: [(String, SourceLocation)] = []
        var citationUses: [(String, SourceLocation)] = []

        for sourceID in snapshots.keys.sorted() {
            let snapshot = snapshots[sourceID]!
            for record in snapshot.references {
                let location = SourceLocation(sourceID: sourceID, range: record.range)
                switch record.kind {
                case .label: labels[record.key, default: []].append(location)
                case .reference: labelUses.append((record.key, location))
                case .citation: citationUses.append((record.key, location))
                case .bibliographyEntry: citations[record.key, default: []].append(location)
                }
            }
            for entry in snapshot.bibliography {
                citations[entry.key, default: []].append(SourceLocation(sourceID: sourceID, range: entry.range))
            }
        }
        for key in labels.keys.sorted() where labels[key]!.count > 1 {
            result.append(.duplicateLabel(key: key, definitions: labels[key]!))
        }
        for key in citations.keys.sorted() where citations[key]!.count > 1 {
            result.append(.duplicateCitation(key: key, definitions: citations[key]!))
        }
        for (key, location) in labelUses.sorted(by: LanguageParser.keyLocationOrder) where labels[key] == nil {
            result.append(.missingLabel(key: key, use: location))
        }
        for (key, location) in citationUses.sorted(by: LanguageParser.keyLocationOrder) where citations[key] == nil {
            result.append(.missingCitation(key: key, use: location))
        }
        result.append(contentsOf: includeCycles().map(LanguageDiagnostic.includeCycle))
        return result
    }

    public func completions(prefix: String = "") -> [LanguageCompletion] {
        if prefix.hasPrefix("\\") {
            return LanguageIndex.commandCompletions(prefix: prefix)
        }
        var values: [LanguageCompletion] = []
        let normalized = prefix
        let labels = Set(references.filter { $0.record.kind == .label }.map { $0.record.key })
        let citations = Set(bibliography.map { $0.entry.key })
        values += labels.filter { $0.hasPrefix(normalized) }.map { LanguageCompletion(text: $0, kind: .label) }
        values += citations.filter { $0.hasPrefix(normalized) }.map { LanguageCompletion(text: $0, kind: .citation) }
        return values.sorted { ($0.text, $0.kind.rawValue) < ($1.text, $1.kind.rawValue) }
    }

    public func recursiveOutline(from sourceID: String) -> [ProjectOutlineRecord] {
        var result: [ProjectOutlineRecord] = []
        var active: Set<String> = []
        func visit(_ id: String) {
            guard let snapshot = snapshots[id], active.insert(id).inserted else { return }
            enum Event { case outline(OutlineRecord); case include(IncludeRecord) }
            var events = snapshot.outline.map(Event.outline) + snapshot.includes.map(Event.include)
            events.sort {
                func offset(_ event: Event) -> Int {
                    switch event { case .outline(let value): return value.range.utf8Offset; case .include(let value): return value.range.utf8Offset }
                }
                return offset($0) < offset($1)
            }
            for event in events {
                switch event {
                case .outline(let record): result.append(ProjectOutlineRecord(sourceID: id, record: record))
                case .include(let include): visit(resolve(include.target, from: id))
                }
            }
            active.remove(id)
        }
        visit(sourceID)
        return result
    }

    private func includeCycles() -> [[String]] {
        var cycles: Set<[String]> = []
        var path: [String] = []
        var active: Set<String> = []
        func canonical(_ cycle: [String]) -> [String] {
            let body = Array(cycle.dropLast())
            guard !body.isEmpty else { return cycle }
            let rotations = body.indices.map { index in
                Array(body[index...]) + Array(body[..<index])
            }
            let best = rotations.min { $0.lexicographicallyPrecedes($1) }!
            return best + [best[0]]
        }
        func visit(_ id: String) {
            guard let snapshot = snapshots[id] else { return }
            if let index = path.firstIndex(of: id) {
                cycles.insert(canonical(Array(path[index...]) + [id]))
                return
            }
            guard active.insert(id).inserted else { return }
            path.append(id)
            for include in snapshot.includes.sorted(by: { ($0.range.utf8Offset, $0.target) < ($1.range.utf8Offset, $1.target) }) {
                visit(resolve(include.target, from: id))
            }
            path.removeLast()
            active.remove(id)
        }
        for id in snapshots.keys.sorted() { visit(id) }
        return cycles.sorted { $0.lexicographicallyPrecedes($1) }
    }

    private func resolve(_ target: String, from sourceID: String) -> String {
        var resolvedTarget = target
        if !resolvedTarget.hasSuffix(".tex") { resolvedTarget += ".tex" }
        guard !resolvedTarget.hasPrefix("/") else { return resolvedTarget }
        let directory: String
        if let slash = sourceID.lastIndex(of: "/") {
            directory = String(sourceID[..<sourceID.index(after: slash)])
        } else {
            directory = ""
        }
        var components: [Substring] = []
        for component in (directory + resolvedTarget).split(separator: "/", omittingEmptySubsequences: false) {
            if component.isEmpty || component == "." { continue }
            if component == ".." {
                if !components.isEmpty { components.removeLast() }
            } else {
                components.append(component)
            }
        }
        return components.map(String.init).joined(separator: "/")
    }
}

public enum LanguageIndex {
    private static let commands = [
        "begin", "bibliography", "cite", "documentclass", "emph", "end", "include", "input",
        "item", "label", "paragraph", "ref", "section", "subsection", "subsubsection", "textbf", "textit"
    ]

    public static func commandCompletions(prefix: String = "") -> [LanguageCompletion] {
        let normalized = prefix.hasPrefix("\\") ? String(prefix.dropFirst()) : prefix
        return commands.filter { $0.hasPrefix(normalized) }.sorted().map {
            LanguageCompletion(text: "\\" + $0, kind: .command)
        }
    }
}

private enum LanguageParser {
    struct Parsed {
        var outline: [OutlineRecord] = []
        var includes: [IncludeRecord] = []
        var references: [ReferenceRecord] = []
        var bibliography: [BibTeXEntry] = []
        var minimap: [MinimapRange] = []
        var diagnostics: [LanguageDiagnostic] = []
    }

    static func keyLocationOrder(_ lhs: (String, SourceLocation), _ rhs: (String, SourceLocation)) -> Bool {
        (lhs.0, lhs.1.sourceID, lhs.1.range.utf8Offset) < (rhs.0, rhs.1.sourceID, rhs.1.range.utf8Offset)
    }

    static func parse(sourceID: String, source: String, dialect: TeXDialect) throws -> Parsed {
        dialect == .bibtex ? try parseBibTeX(sourceID: sourceID, source: source) : try parseLaTeX(sourceID: sourceID, source: source)
    }

    private static func parseLaTeX(sourceID: String, source: String) throws -> Parsed {
        let bytes = Array(source.utf8)
        var result = Parsed()
        var index = 0
        let verbatimNames: Set<String> = ["verbatim", "verbatim*", "Verbatim", "lstlisting"]
        while index < bytes.count {
            if bytes[index] == 37 {
                while index < bytes.count, bytes[index] != 10, bytes[index] != 13 { index += 1 }
                continue
            }
            guard bytes[index] == 92 else { index += 1; continue }
            let commandStart = index
            index += 1
            guard index < bytes.count else { continue }
            let nameStart = index
            if isLetter(bytes[index]) {
                while index < bytes.count, isLetter(bytes[index]) { index += 1 }
            } else {
                index += 1
            }
            let command = text(bytes, nameStart, index)
            if command == "verb" {
                if index < bytes.count {
                    let delimiter = bytes[index]
                    index += 1
                    while index < bytes.count, bytes[index] != delimiter, bytes[index] != 10, bytes[index] != 13 { index += 1 }
                    if index < bytes.count, bytes[index] == delimiter { index += 1 }
                }
                continue
            }
            guard ["section", "subsection", "subsubsection", "include", "input", "label", "ref", "pageref", "cite", "citep", "citet", "begin"].contains(command) else { continue }
            if index < bytes.count, bytes[index] == 42 { index += 1 }
            guard let group = group(after: index, bytes: bytes) else {
                let range = try SourceRange(utf8Offset: commandStart, utf8Length: index - commandStart)
                result.diagnostics.append(.malformedGroup(sourceID: sourceID, command: command, range: range))
                continue
            }
            let commandRange = try SourceRange(utf8Offset: commandStart, utf8Length: group.end - commandStart)
            let value = trimmed(groupText(bytes, group.contentStart, group.contentEnd))
            index = group.end
            if value.isEmpty {
                result.diagnostics.append(.malformedGroup(sourceID: sourceID, command: command, range: commandRange))
                continue
            }
            switch command {
            case "section", "subsection", "subsubsection":
                let kind: OutlineKind = command == "section" ? .section : (command == "subsection" ? .subsection : .subsubsection)
                let depth = command == "section" ? 0 : (command == "subsection" ? 1 : 2)
                result.outline.append(try OutlineRecord(kind: kind, title: value, range: commandRange, depth: depth))
                result.minimap.append(MinimapRange(kind: .outline, range: commandRange))
            case "include", "input":
                result.includes.append(try IncludeRecord(target: value, range: commandRange))
                result.minimap.append(MinimapRange(kind: .include, range: commandRange))
            case "label":
                result.references.append(try ReferenceRecord(kind: .label, key: value, range: commandRange))
                result.minimap.append(MinimapRange(kind: .definition, range: commandRange))
            case "ref", "pageref":
                result.references.append(try ReferenceRecord(kind: .reference, key: value, range: commandRange))
                result.minimap.append(MinimapRange(kind: .use, range: commandRange))
            case "cite", "citep", "citet":
                for key in value.split(separator: ",").map({ trimmed(String($0)) }).filter({ !$0.isEmpty }) {
                    result.references.append(try ReferenceRecord(kind: .citation, key: key, range: commandRange))
                }
                result.minimap.append(MinimapRange(kind: .use, range: commandRange))
            case "begin" where verbatimNames.contains(value):
                let marker = Array(("\\end{" + value + "}").utf8)
                if let end = find(marker, in: bytes, from: index) { index = end + marker.count }
            default: break
            }
        }
        return result
    }

    private static func parseBibTeX(sourceID: String, source: String) throws -> Parsed {
        let bytes = Array(source.utf8)
        var result = Parsed()
        var index = 0
        while index < bytes.count {
            if bytes[index] == 37 {
                while index < bytes.count, bytes[index] != 10, bytes[index] != 13 { index += 1 }
                continue
            }
            guard bytes[index] == 64 else { index += 1; continue }
            let start = index
            index += 1
            let typeStart = index
            while index < bytes.count, isLetter(bytes[index]) { index += 1 }
            let type = text(bytes, typeStart, index).lowercased()
            while index < bytes.count, isWhitespace(bytes[index]) { index += 1 }
            guard !type.isEmpty, index < bytes.count, bytes[index] == 123 || bytes[index] == 40 else {
                let range = try SourceRange(utf8Offset: start, utf8Length: index - start)
                result.diagnostics.append(.malformedGroup(sourceID: sourceID, command: "@" + type, range: range))
                continue
            }
            let opening = bytes[index]
            let closing: UInt8 = opening == 123 ? 125 : 41
            index += 1
            if ["comment", "preamble", "string"].contains(type) {
                index = balancedEnd(bytes, from: index, opening: opening, closing: closing) ?? bytes.count
                continue
            }
            let keyStart = index
            while index < bytes.count, bytes[index] != 44, bytes[index] != closing { index += 1 }
            let key = trimmed(text(bytes, keyStart, index))
            guard !key.isEmpty, index < bytes.count, bytes[index] == 44 else {
                let range = try SourceRange(utf8Offset: start, utf8Length: index - start)
                result.diagnostics.append(.malformedGroup(sourceID: sourceID, command: "@" + type, range: range))
                continue
            }
            let end = balancedEnd(bytes, from: index + 1, opening: opening, closing: closing)
            guard let end else {
                let range = try SourceRange(utf8Offset: start, utf8Length: bytes.count - start)
                result.diagnostics.append(.malformedGroup(sourceID: sourceID, command: "@" + type, range: range))
                break
            }
            let range = try SourceRange(utf8Offset: start, utf8Length: end - start)
            result.bibliography.append(try BibTeXEntry(type: type, key: key, range: range))
            result.minimap.append(MinimapRange(kind: .bibliography, range: range))
            index = end
        }
        return result
    }

    private static func group(after start: Int, bytes: [UInt8]) -> (contentStart: Int, contentEnd: Int, end: Int)? {
        var index = start
        while index < bytes.count, isWhitespace(bytes[index]) { index += 1 }
        guard index < bytes.count, bytes[index] == 123 else { return nil }
        let contentStart = index + 1
        var depth = 1
        index += 1
        while index < bytes.count {
            if bytes[index] == 92 { index += min(2, bytes.count - index); continue }
            if bytes[index] == 37 {
                while index < bytes.count, bytes[index] != 10, bytes[index] != 13 { index += 1 }
                continue
            }
            if bytes[index] == 123 { depth += 1 }
            if bytes[index] == 125 {
                depth -= 1
                if depth == 0 { return (contentStart, index, index + 1) }
            }
            index += 1
        }
        return nil
    }

    private static func balancedEnd(_ bytes: [UInt8], from start: Int, opening: UInt8, closing: UInt8) -> Int? {
        var depth = 1
        var quoted = false
        var index = start
        while index < bytes.count {
            if bytes[index] == 92 { index += min(2, bytes.count - index); continue }
            if bytes[index] == 34 { quoted.toggle(); index += 1; continue }
            if !quoted, bytes[index] == 37 {
                while index < bytes.count, bytes[index] != 10, bytes[index] != 13 { index += 1 }
                continue
            }
            if !quoted, bytes[index] == opening { depth += 1 }
            if !quoted, bytes[index] == closing {
                depth -= 1
                if depth == 0 { return index + 1 }
            }
            index += 1
        }
        return nil
    }

    private static func find(_ needle: [UInt8], in bytes: [UInt8], from start: Int) -> Int? {
        guard !needle.isEmpty, start <= bytes.count - min(bytes.count, needle.count) else { return nil }
        if needle.count > bytes.count { return nil }
        for index in start...(bytes.count - needle.count) where Array(bytes[index..<(index + needle.count)]) == needle { return index }
        return nil
    }

    private static func text(_ bytes: [UInt8], _ start: Int, _ end: Int) -> String {
        String(decoding: bytes[start..<end], as: UTF8.self)
    }

    private static func groupText(_ bytes: [UInt8], _ start: Int, _ end: Int) -> String {
        var content: [UInt8] = []
        var index = start
        while index < end {
            if bytes[index] == 92, index + 1 < end {
                content.append(bytes[index])
                content.append(bytes[index + 1])
                index += 2
            } else if bytes[index] == 37 {
                while index < end, bytes[index] != 10, bytes[index] != 13 { index += 1 }
            } else {
                content.append(bytes[index])
                index += 1
            }
        }
        return String(decoding: content, as: UTF8.self)
    }

    private static func trimmed(_ value: String) -> String {
        let leading = value.drop(while: { $0.isWhitespace })
        return String(leading.reversed().drop(while: { $0.isWhitespace }).reversed())
    }

    private static func isLetter(_ byte: UInt8) -> Bool {
        (byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122)
    }

    private static func isWhitespace(_ byte: UInt8) -> Bool {
        byte == 32 || byte == 9 || byte == 10 || byte == 13
    }
}
