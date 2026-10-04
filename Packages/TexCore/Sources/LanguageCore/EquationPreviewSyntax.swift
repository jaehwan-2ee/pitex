/// Equation-preview syntax: complete math regions, preview-visible macro
/// definitions and include edges, found in one forward pass over UTF-8.
///
/// The pass follows TeX's own reading rules rather than pairing the nearest
/// `$`: comments, `\verb`-like arguments and verbatim environments are inert,
/// `\$` never delimits, `$…$` inside `\text{…}` nests instead of closing the
/// outer math, and a paragraph break (blank line or `\par`), `\end{document}`
/// or a mismatched `\end{…}` terminates an unfinished region the way TeX
/// reports "Missing $ inserted". Delimiters are ASCII, so bytes of multi-byte
/// scalars (Korean, emoji, combining marks) never match one.

public enum MathDelimiter: String, Hashable, Codable, Sendable {
    case dollar, doubleDollar, parenthesis, bracket, environment
}

public struct MathRegion: Hashable, Codable, Sendable {
    /// The whole construct, delimiters or `\begin{…}`/`\end{…}` included.
    public let range: SourceRange
    public let contentRange: SourceRange
    public let delimiter: MathDelimiter
    public let environmentName: String?
    public let displayMode: Bool
    /// False when TeX would reject the region before any closer: a
    /// paragraph break, `\end{document}`, a mismatched `\end`, or EOF.
    public let isComplete: Bool
    /// Braces balance and no stray math shift appears inside the body.
    public let isWellFormed: Bool

    public var isRenderable: Bool { isComplete && isWellFormed }

    /// Caret semantics: just after the opener's first byte through just
    /// after the closer — `$x$|` still previews, `|$x$` does not.
    public func containsCaret(_ utf8Offset: Int) -> Bool {
        utf8Offset > range.utf8Offset && utf8Offset <= range.endUTF8Offset
    }

    /// Pointer semantics: the character starting at `utf8Offset`.
    public func containsCharacter(_ utf8Offset: Int) -> Bool {
        utf8Offset >= range.utf8Offset && utf8Offset < range.endUTF8Offset
    }

    /// Environments whose body is handed to MathJax as display/inline math;
    /// every other supported environment is passed whole because MathJax
    /// parses `\begin{align}…\end{align}` itself.
    static let bodyOnlyEnvironments: Set<String> = ["math", "displaymath", "equation", "equation*"]

    /// The TeX handed to the fast renderer.
    public func renderSource(in source: String) -> String {
        if delimiter == .environment, let environmentName,
           !Self.bodyOnlyEnvironments.contains(environmentName) {
            return Self.slice(source, range)
        }
        return Self.slice(source, contentRange)
    }

    /// The region exactly as written, for the exact TeX document.
    public func sourceText(in source: String) -> String {
        Self.slice(source, range)
    }

    static func slice(_ source: String, _ range: SourceRange) -> String {
        let utf8 = source.utf8
        let lower = utf8.index(utf8.startIndex, offsetBy: min(range.utf8Offset, utf8.count))
        let upper = utf8.index(lower, offsetBy: min(range.utf8Length, utf8.distance(from: lower, to: utf8.endIndex)))
        return String(decoding: utf8[lower..<upper], as: UTF8.self)
    }
}

public struct MathDefinition: Hashable, Codable, Sendable {
    public enum Kind: String, Hashable, Codable, Sendable {
        case newCommand, renewCommand, provideCommand, declareMathOperator, newEnvironment, renewEnvironment
    }

    public let kind: Kind
    /// `\R` for commands and operators, the bare name for environments.
    public let name: String
    /// Normalized, MathJax-parsable form (`\providecommand` is emitted as
    /// `\newcommand`; the context decides whether it applies).
    public let mathJaxSource: String
    /// The definition exactly as written, for the exact TeX document.
    public let originalSource: String
    public let range: SourceRange
}

public struct MathInclude: Hashable, Codable, Sendable {
    /// As written; `\usepackage{x}` is recorded as `x.sty` so project-local
    /// packages contribute definitions while system packages never resolve.
    public let target: String
    public let utf8Offset: Int
}

public struct MathSourceScan: Hashable, Sendable {
    /// Top-level regions, sorted and non-overlapping.
    public let regions: [MathRegion]
    public let definitions: [MathDefinition]
    public let includes: [MathInclude]
    /// Offset of `\begin{document}` outside comments/verbatim, if any.
    public let documentBeginOffset: Int?
    /// Source before `\begin{document}`.
    public let preamble: String?

    public func region(atCaret utf8Offset: Int) -> MathRegion? {
        // Last region starting strictly before the caret.
        var lo = 0, hi = regions.count
        while lo < hi {
            let mid = (lo + hi) / 2
            if regions[mid].range.utf8Offset < utf8Offset { lo = mid + 1 } else { hi = mid }
        }
        guard lo > 0 else { return nil }
        let region = regions[lo - 1]
        return region.containsCaret(utf8Offset) ? region : nil
    }

    public func region(atCharacter utf8Offset: Int) -> MathRegion? {
        var lo = 0, hi = regions.count
        while lo < hi {
            let mid = (lo + hi) / 2
            if regions[mid].range.utf8Offset <= utf8Offset { lo = mid + 1 } else { hi = mid }
        }
        guard lo > 0 else { return nil }
        let region = regions[lo - 1]
        return region.containsCharacter(utf8Offset) ? region : nil
    }
}

public enum MathSourceScanner {
    /// Display-level environments (`math` is the one inline member).
    static let outerEnvironments: [String: Bool] = [
        "math": false, "displaymath": true,
        "equation": true, "equation*": true, "align": true, "align*": true,
        "gather": true, "gather*": true, "multline": true, "multline*": true,
        "flalign": true, "flalign*": true, "alignat": true, "alignat*": true,
        "eqnarray": true, "eqnarray*": true,
    ]
    /// Inner math environments: part of an enclosing region, or display
    /// math of their own when written standalone.
    static let innerEnvironments: Set<String> = [
        "aligned", "alignedat", "gathered", "split", "cases", "array",
        "matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix", "smallmatrix",
    ]
    static let verbatimEnvironments: Set<String> = [
        "verbatim", "verbatim*", "Verbatim", "Verbatim*", "BVerbatim", "LVerbatim",
        "lstlisting", "minted", "comment", "filecontents", "filecontents*",
    ]

    public static func scan(_ source: String) -> MathSourceScan {
        var text = source
        return text.withUTF8 { bytes in
            var scanner = Scanner(b: bytes)
            scanner.run()
            let preamble = scanner.documentBegin.map {
                String(decoding: UnsafeBufferPointer(rebasing: bytes[0..<$0]), as: UTF8.self)
            }
            return MathSourceScan(
                regions: scanner.regions,
                definitions: scanner.definitions,
                includes: scanner.includes,
                documentBeginOffset: scanner.documentBegin,
                preamble: preamble
            )
        }
    }
}

private enum Keyword {
    case begin, end, par
    case inlineVerbatim, rawGroup, mintinline
    case newcommand, renewcommand, providecommand, declareMathOperator, newenvironment, renewenvironment
    case input, importFile, subimport, usepackage
    case textArgument
}

private let keywordTable: [[(bytes: [UInt8], keyword: Keyword)]] = {
    let entries: [(String, Keyword)] = [
        ("begin", .begin), ("end", .end), ("par", .par),
        ("verb", .inlineVerbatim), ("Verb", .inlineVerbatim), ("lstinline", .inlineVerbatim),
        ("url", .rawGroup), ("path", .rawGroup), ("href", .rawGroup), ("mintinline", .mintinline),
        ("newcommand", .newcommand), ("renewcommand", .renewcommand), ("providecommand", .providecommand),
        ("DeclareMathOperator", .declareMathOperator),
        ("newenvironment", .newenvironment), ("renewenvironment", .renewenvironment),
        ("input", .input), ("include", .input), ("subfile", .input),
        ("import", .importFile), ("subimport", .subimport),
        ("usepackage", .usepackage), ("RequirePackage", .usepackage),
        ("text", .textArgument), ("textrm", .textArgument), ("textit", .textArgument),
        ("textbf", .textArgument), ("textsf", .textArgument), ("texttt", .textArgument),
        ("textup", .textArgument), ("textnormal", .textArgument), ("textmd", .textArgument),
        ("textsc", .textArgument), ("textsl", .textArgument), ("emph", .textArgument),
        ("mbox", .textArgument), ("hbox", .textArgument), ("fbox", .textArgument),
        ("makebox", .textArgument), ("framebox", .textArgument), ("parbox", .textArgument),
        ("intertext", .textArgument), ("shortintertext", .textArgument), ("tag", .textArgument),
    ]
    var table = Array(repeating: [(bytes: [UInt8], keyword: Keyword)](), count: 20)
    for (name, keyword) in entries { table[name.utf8.count].append((Array(name.utf8), keyword)) }
    return table
}()

private enum MathOutcome {
    case closed(contentEnd: Int, end: Int, wellFormed: Bool)
    /// `end` is where the region stops; scanning resumes in text at `resume`.
    case incomplete(end: Int, resume: Int)
}

private let backslash: UInt8 = 92, dollar: UInt8 = 36, percent: UInt8 = 37
private let lbrace: UInt8 = 123, rbrace: UInt8 = 125, lbracket: UInt8 = 91, rbracket: UInt8 = 93
private let lparen: UInt8 = 40, rparen: UInt8 = 41, newline: UInt8 = 10, carriageReturn: UInt8 = 13
private let space: UInt8 = 32, tab: UInt8 = 9, star: UInt8 = 42, comma: UInt8 = 44

private struct Scanner {
    let b: UnsafeBufferPointer<UInt8>
    var i = 0
    var regions: [MathRegion] = []
    var definitions: [MathDefinition] = []
    var includes: [MathInclude] = []
    var documentBegin: Int?
    /// `\text{$…\text{$…` recursion guard — deeper nesting is read as plain math.
    var nesting = 0

    init(b: UnsafeBufferPointer<UInt8>) { self.b = b }

    var count: Int { b.count }

    static func isLetter(_ byte: UInt8) -> Bool {
        (byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122)
    }

    func string(_ start: Int, _ end: Int) -> String {
        String(decoding: UnsafeBufferPointer(rebasing: b[start..<end]), as: UTF8.self)
    }

    func keyword(_ start: Int, _ end: Int) -> Keyword? {
        let length = end - start
        guard length < keywordTable.count else { return nil }
        outer: for entry in keywordTable[length] where entry.bytes[0] == b[start] {
            for offset in 1..<length where entry.bytes[offset] != b[start + offset] { continue outer }
            return entry.keyword
        }
        return nil
    }

    // MARK: Text mode

    mutating func run() {
        while i < count {
            switch b[i] {
            case percent: skipComment()
            case dollar: openDollar()
            case backslash: textControlSequence()
            default: i += 1
            }
        }
    }

    mutating func skipComment() {
        while i < count, b[i] != newline, b[i] != carriageReturn { i += 1 }
    }

    /// After a backslash's next byte: skip the rest of a control symbol.
    mutating func skipSymbol() {
        i += 1
        while i < count, b[i] & 0xC0 == 0x80 { i += 1 }
    }

    mutating func readLetters() -> (Int, Int) {
        let start = i
        while i < count, Self.isLetter(b[i]) { i += 1 }
        return (start, i)
    }

    mutating func skipSpaces() {
        while i < count, b[i] == space || b[i] == tab { i += 1 }
    }

    /// Spaces plus at most one line break — TeX's tokenization between a
    /// control word and its arguments without crossing a paragraph. A `%`
    /// comment eats the rest of its line (the newline it ends at counts as
    /// the one break); a second line break is still a paragraph boundary.
    mutating func skipInterArgumentSpace() {
        while true {
            skipSpaces()
            if i < count, b[i] == percent { skipComment(); continue }
            if i < count, b[i] == newline || b[i] == carriageReturn {
                if blankLineEnd(at: i) != nil { return }
                if b[i] == carriageReturn, i + 1 < count, b[i + 1] == newline { i += 1 }
                i += 1
                continue
            }
            return
        }
    }

    mutating func textControlSequence() {
        let start = i
        i += 1
        guard i < count else { return }
        if !Self.isLetter(b[i]) {
            switch b[i] {
            case lparen: openRegion(start: start, contentStart: i + 1, delimiter: .parenthesis, environment: nil, display: false)
            case lbracket: openRegion(start: start, contentStart: i + 1, delimiter: .bracket, environment: nil, display: true)
            default: skipSymbol()
            }
            return
        }
        let (nameStart, nameEnd) = readLetters()
        guard let keyword = keyword(nameStart, nameEnd) else { return }
        switch keyword {
        case .begin:
            guard let (envStart, envEnd, after) = groupName() else { return }
            let name = string(envStart, envEnd)
            if name == "document" {
                if documentBegin == nil { documentBegin = start }
                i = after
            } else if MathSourceScanner.verbatimEnvironments.contains(name) {
                skipVerbatimEnvironment(name, from: after)
            } else if let display = MathSourceScanner.outerEnvironments[name] {
                openRegion(start: start, contentStart: after, delimiter: .environment, environment: name, display: display)
            } else if MathSourceScanner.innerEnvironments.contains(name) {
                openRegion(start: start, contentStart: after, delimiter: .environment, environment: name, display: true)
            } else {
                i = after
            }
        case .end:
            guard let (envStart, envEnd, after) = groupName() else { return }
            i = after
            // TeX stops reading at \end{document}.
            if string(envStart, envEnd) == "document" { i = count }
        case .inlineVerbatim: skipInlineVerbatim(mint: false)
        case .mintinline: skipInlineVerbatim(mint: true)
        case .rawGroup: skipRawGroupOrDelimited()
        case .newcommand: parseCommandDefinition(start: start, kind: .newCommand)
        case .renewcommand: parseCommandDefinition(start: start, kind: .renewCommand)
        case .providecommand: parseCommandDefinition(start: start, kind: .provideCommand)
        case .declareMathOperator: parseOperatorDefinition(start: start)
        case .newenvironment: parseEnvironmentDefinition(start: start, kind: .newEnvironment)
        case .renewenvironment: parseEnvironmentDefinition(start: start, kind: .renewEnvironment)
        case .input:
            skipInterArgumentSpace()
            if let (contentStart, contentEnd, end) = balancedGroup() {
                appendInclude(identifier(contentStart, contentEnd), at: start)
                i = end
            }
        case .importFile, .subimport:
            skipInterArgumentSpace()
            guard let (dirStart, dirEnd, dirAfter) = balancedGroup() else { return }
            i = dirAfter
            skipInterArgumentSpace()
            guard let (fileStart, fileEnd, fileAfter) = balancedGroup() else { return }
            var directory = identifier(dirStart, dirEnd)
            if !directory.isEmpty, !directory.hasSuffix("/") { directory += "/" }
            appendInclude(directory + identifier(fileStart, fileEnd), at: start)
            i = fileAfter
        case .usepackage:
            skipInterArgumentSpace()
            while i < count, b[i] == lbracket { guard skipBracketGroup() else { return }; skipInterArgumentSpace() }
            guard let (contentStart, contentEnd, end) = balancedGroup() else { return }
            for name in identifier(contentStart, contentEnd).split(separator: ",") {
                let package = trimmedBytes(Array(name.utf8))
                if !package.isEmpty { appendInclude(package + ".sty", at: start) }
            }
            i = end
        case .par, .textArgument: break
        }
    }

    mutating func appendInclude(_ target: String, at offset: Int) {
        guard !target.isEmpty else { return }
        includes.append(MathInclude(target: target, utf8Offset: offset))
    }

    mutating func openDollar() {
        let start = i
        if i + 1 < count, b[i + 1] == dollar {
            openRegion(start: start, contentStart: i + 2, delimiter: .doubleDollar, environment: nil, display: true)
        } else {
            openRegion(start: start, contentStart: i + 1, delimiter: .dollar, environment: nil, display: false)
        }
    }

    mutating func openRegion(start: Int, contentStart: Int, delimiter: MathDelimiter, environment: String?, display: Bool) {
        i = contentStart
        let outcome = scanMath(closer: delimiter, environment: environment)
        let region: MathRegion
        switch outcome {
        case let .closed(contentEnd, end, wellFormed):
            region = MathRegion(
                range: .lexerRange(offset: start, length: end - start),
                contentRange: .lexerRange(offset: contentStart, length: contentEnd - contentStart),
                delimiter: delimiter, environmentName: environment, displayMode: display,
                isComplete: true, isWellFormed: wellFormed
            )
            i = end
        case let .incomplete(end, resume):
            region = MathRegion(
                range: .lexerRange(offset: start, length: max(end, contentStart) - start),
                contentRange: .lexerRange(offset: contentStart, length: max(end - contentStart, 0)),
                delimiter: delimiter, environmentName: environment, displayMode: display,
                isComplete: false, isWellFormed: false
            )
            i = max(resume, contentStart)
        }
        regions.append(region)
    }

    /// Offset of the second line break when a paragraph break starts at `at`.
    func blankLineEnd(at position: Int) -> Int? {
        var j = position
        if b[j] == carriageReturn, j + 1 < count, b[j + 1] == newline { j += 2 } else { j += 1 }
        while j < count, b[j] == space || b[j] == tab { j += 1 }
        guard j < count, b[j] == newline || b[j] == carriageReturn else { return nil }
        return j
    }

    // MARK: Math mode

    mutating func scanMath(closer: MathDelimiter, environment: String?) -> MathOutcome {
        var braces = 0
        var malformed = false
        var sameNameDepth = 0
        while i < count {
            let byte = b[i]
            switch byte {
            case percent:
                skipComment()
            case newline, carriageReturn:
                if let resume = blankLineEnd(at: i) { return .incomplete(end: i, resume: resume) }
                i += 1
            case lbrace:
                braces += 1; i += 1
            case rbrace:
                braces -= 1; i += 1
            case dollar:
                switch closer {
                case .dollar:
                    let contentEnd = i
                    i += 1
                    return .closed(contentEnd: contentEnd, end: i, wellFormed: braces == 0 && !malformed)
                case .doubleDollar:
                    if i + 1 < count, b[i + 1] == dollar {
                        let contentEnd = i
                        i += 2
                        return .closed(contentEnd: contentEnd, end: i, wellFormed: braces == 0 && !malformed)
                    }
                    // A lone $ inside $$…$$ ends display math with an error.
                    return .incomplete(end: i, resume: i)
                case .parenthesis, .bracket, .environment:
                    malformed = true
                    i += 1
                }
            case backslash:
                let commandStart = i
                i += 1
                guard i < count else { break }
                if !Self.isLetter(b[i]) {
                    let symbol = b[i]
                    if (symbol == rparen && closer == .parenthesis) || (symbol == rbracket && closer == .bracket) {
                        i += 1
                        return .closed(contentEnd: commandStart, end: i, wellFormed: braces == 0 && !malformed)
                    }
                    skipSymbol()
                    continue
                }
                let (nameStart, nameEnd) = readLetters()
                switch keyword(nameStart, nameEnd) {
                case .par:
                    return .incomplete(end: commandStart, resume: nameEnd)
                case .begin:
                    guard let (envStart, envEnd, after) = groupName() else { continue }
                    if let environment, string(envStart, envEnd) == environment { sameNameDepth += 1 }
                    i = after
                case .end:
                    guard let (envStart, envEnd, after) = groupName() else { continue }
                    let name = string(envStart, envEnd)
                    if closer == .environment, name == environment {
                        if sameNameDepth == 0 {
                            i = after
                            return .closed(contentEnd: commandStart, end: after, wellFormed: braces == 0 && !malformed)
                        }
                        sameNameDepth -= 1
                    } else if name == "document" || MathSourceScanner.outerEnvironments[name] != nil {
                        // Closes something this region never opened.
                        return .incomplete(end: commandStart, resume: commandStart)
                    }
                    i = after
                case .textArgument:
                    if let stop = skipTextArgument() { return stop }
                case .inlineVerbatim:
                    skipInlineVerbatim(mint: false)
                case .rawGroup:
                    skipRawGroupOrDelimited()
                default:
                    break
                }
            default:
                i += 1
            }
        }
        return .incomplete(end: count, resume: count)
    }

    /// Text-mode argument inside math: `$…$` there is nested math. Returns
    /// an outcome only when the enclosing region must stop.
    mutating func skipTextArgument() -> MathOutcome? {
        if i < count, b[i] == star { i += 1 }
        skipInterArgumentSpace()
        while i < count, b[i] == lbracket {
            guard skipBracketGroup() else { return nil }
            skipInterArgumentSpace()
        }
        guard i < count, b[i] == lbrace else { return nil }
        i += 1
        var depth = 1
        while i < count {
            switch b[i] {
            case percent:
                skipComment()
            case newline, carriageReturn:
                if let resume = blankLineEnd(at: i) { return .incomplete(end: i, resume: resume) }
                i += 1
            case lbrace:
                depth += 1; i += 1
            case rbrace:
                depth -= 1; i += 1
                if depth == 0 { return nil }
            case dollar where nesting < 8:
                i += 1
                nesting += 1
                let inner = scanMath(closer: .dollar, environment: nil)
                nesting -= 1
                if case let .incomplete(end, resume) = inner { return .incomplete(end: end, resume: resume) }
            case backslash:
                i += 1
                guard i < count else { break }
                if Self.isLetter(b[i]) {
                    let (nameStart, nameEnd) = readLetters()
                    if keyword(nameStart, nameEnd) == .par { return .incomplete(end: nameStart - 1, resume: nameEnd) }
                } else if b[i] == lparen, nesting < 8 {
                    i += 1
                    nesting += 1
                    let inner = scanMath(closer: .parenthesis, environment: nil)
                    nesting -= 1
                    if case let .incomplete(end, resume) = inner { return .incomplete(end: end, resume: resume) }
                } else {
                    skipSymbol()
                }
            default:
                i += 1
            }
        }
        return .incomplete(end: count, resume: count)
    }

    // MARK: Verbatim

    mutating func skipVerbatimEnvironment(_ name: String, from start: Int) {
        let marker = Array(("\\end{" + name + "}").utf8)
        var j = start
        while j + marker.count <= count {
            if b[j] == backslash {
                var matched = true
                for k in 1..<marker.count where b[j + k] != marker[k] { matched = false; break }
                if matched { i = j + marker.count; return }
            }
            j += 1
        }
        i = count
    }

    /// `\verb|…|`, `\lstinline[…]{…}`, `\mintinline{lang}|…|`.
    mutating func skipInlineVerbatim(mint: Bool) {
        if i < count, b[i] == star { i += 1 }
        while i < count, b[i] == lbracket { guard skipBracketGroup() else { return } }
        if mint {
            guard let (_, _, end) = balancedGroup() else { return }
            i = end
        }
        skipRawGroupOrDelimited()
    }

    /// A raw `{…}` (braces nest, nothing else is special) or `<c>…<c>`.
    mutating func skipRawGroupOrDelimited() {
        guard i < count else { return }
        let opener = b[i]
        if opener == lbrace {
            var depth = 0
            while i < count {
                if b[i] == lbrace { depth += 1 }
                if b[i] == rbrace { depth -= 1; if depth == 0 { i += 1; return } }
                if b[i] == newline, blankLineEnd(at: i) != nil { return }
                i += 1
            }
            return
        }
        guard opener != space, opener != newline, opener != carriageReturn, !Self.isLetter(opener) else { return }
        i += 1
        while i < count, b[i] != opener, b[i] != newline, b[i] != carriageReturn { i += 1 }
        if i < count, b[i] == opener { i += 1 }
    }

    // MARK: Groups

    /// `{name}` after optional spaces: (nameStart, nameEnd, after).
    mutating func groupName() -> (Int, Int, Int)? {
        var j = i
        while j < count, b[j] == space || b[j] == tab { j += 1 }
        guard j < count, b[j] == lbrace else { return nil }
        let start = j + 1
        var end = start
        while end < count, b[end] != rbrace, b[end] != newline, b[end] != carriageReturn, b[end] != lbrace { end += 1 }
        guard end < count, b[end] == rbrace else { return nil }
        return (start, end, end + 1)
    }

    /// Balanced `{…}` at `i` (escapes and comments respected): (contentStart, contentEnd, after).
    func balancedGroup() -> (Int, Int, Int)? {
        guard i < count, b[i] == lbrace else { return nil }
        var j = i + 1
        var depth = 1
        while j < count {
            switch b[j] {
            case backslash:
                j += 2
                while j < count, b[j] & 0xC0 == 0x80 { j += 1 }
                continue
            case percent:
                while j < count, b[j] != newline, b[j] != carriageReturn { j += 1 }
                continue
            case lbrace:
                depth += 1
            case rbrace:
                depth -= 1
                if depth == 0 { return (i + 1, j, j + 1) }
            default:
                break
            }
            j += 1
        }
        return nil
    }

    /// `[…]` with braces protecting nested brackets. False when unterminated.
    mutating func skipBracketGroup() -> Bool {
        guard let (_, _, end) = bracketGroup() else { return false }
        i = end
        return true
    }

    func bracketGroup() -> (Int, Int, Int)? {
        guard i < count, b[i] == lbracket else { return nil }
        var j = i + 1
        var depth = 0
        while j < count {
            switch b[j] {
            case backslash:
                j += 2
                continue
            case lbrace: depth += 1
            case rbrace: depth -= 1
            case rbracket where depth == 0: return (i + 1, j, j + 1)
            case newline where blankLineEnd(at: j) != nil: return nil
            default: break
            }
            j += 1
        }
        return nil
    }

    func trimmed(_ start: Int, _ end: Int) -> String {
        var lo = start, hi = end
        while lo < hi, b[lo] == space || b[lo] == tab || b[lo] == newline || b[lo] == carriageReturn { lo += 1 }
        while hi > lo, b[hi - 1] == space || b[hi - 1] == tab || b[hi - 1] == newline || b[hi - 1] == carriageReturn { hi -= 1 }
        return string(lo, hi)
    }

    /// Identifier-style argument text (`\input`, `\usepackage`, dir/file):
    /// `%` comments drop the rest of their line (the break itself stays,
    /// matching TeX), `\%` survives, then edges are trimmed.
    func identifier(_ start: Int, _ end: Int) -> String {
        var bytes: [UInt8] = []
        bytes.reserveCapacity(end - start)
        var j = start
        while j < end {
            switch b[j] {
            case backslash:
                bytes.append(backslash)
                j += 1
                if j < end {
                    bytes.append(b[j]); j += 1
                    while j < end, b[j] & 0xC0 == 0x80 { bytes.append(b[j]); j += 1 }
                }
            case percent:
                while j < end, b[j] != newline, b[j] != carriageReturn { j += 1 }
            default:
                bytes.append(b[j]); j += 1
            }
        }
        return trimmedBytes(bytes)
    }

    /// ASCII-only whitespace trim — byte-identical to Rust's `trimmed()`
    /// (C9): `trimmingCharacters` also strips NBSP and friends, which TeX
    /// treats as printable.
    func trimmedBytes(_ bytes: [UInt8]) -> String {
        var lo = 0, hi = bytes.count
        while lo < hi, bytes[lo] == space || bytes[lo] == tab || bytes[lo] == newline || bytes[lo] == carriageReturn { lo += 1 }
        while hi > lo, bytes[hi - 1] == space || bytes[hi - 1] == tab || bytes[hi - 1] == newline || bytes[hi - 1] == carriageReturn { hi -= 1 }
        return String(decoding: bytes[lo..<hi], as: UTF8.self)
    }
    // MARK: Definitions

    /// `\name` (control word or symbol) at `i`, or nil.
    mutating func controlSequence() -> String? {
        guard i < count, b[i] == backslash, i + 1 < count else { return nil }
        let start = i
        i += 1
        if Self.isLetter(b[i]) { _ = readLetters() } else { skipSymbol() }
        return string(start, i)
    }

    /// `{\name}` or `\name`.
    mutating func definedName() -> String? {
        if i < count, b[i] == lbrace {
            i += 1
            skipInterArgumentSpace()
            guard let name = controlSequence() else { return nil }
            skipInterArgumentSpace()
            guard i < count, b[i] == rbrace else { return nil }
            i += 1
            return name
        }
        return controlSequence()
    }

    /// `[n]` then, when present, `[default]` — as normalized text.
    mutating func argumentSpec() -> String? {
        var spec = ""
        skipInterArgumentSpace()
        guard i < count, b[i] == lbracket else { return spec }
        guard let (countStart, countEnd, countAfter) = bracketGroup() else { return nil }
        let arity = trimmed(countStart, countEnd)
        guard arity.count == 1, let digit = arity.first, ("0"..."9").contains(digit) else { return nil }
        spec += "[" + arity + "]"
        i = countAfter
        skipInterArgumentSpace()
        if i < count, b[i] == lbracket {
            guard let (defaultStart, defaultEnd, defaultAfter) = bracketGroup() else { return nil }
            spec += "[" + string(defaultStart, defaultEnd) + "]"
            i = defaultAfter
        }
        return spec
    }

    /// A `{body}` group after optional space, as written (braces included).
    mutating func bodyGroup() -> String? {
        skipInterArgumentSpace()
        guard let (_, _, end) = balancedGroup() else { return nil }
        let text = string(i, end)
        i = end
        return text
    }

    mutating func parseCommandDefinition(start: Int, kind: MathDefinition.Kind) {
        let resume = i
        if i < count, b[i] == star { i += 1 }
        skipInterArgumentSpace()
        guard let name = definedName(), let spec = argumentSpec(), let body = bodyGroup() else {
            i = resume
            return
        }
        let command = kind == .renewCommand ? "\\renewcommand" : "\\newcommand"
        definitions.append(MathDefinition(
            kind: kind, name: name,
            mathJaxSource: command + "{" + name + "}" + spec + body,
            originalSource: string(start, i),
            range: .lexerRange(offset: start, length: i - start)
        ))
    }

    mutating func parseOperatorDefinition(start: Int) {
        let resume = i
        var starred = false
        if i < count, b[i] == star { starred = true; i += 1 }
        skipInterArgumentSpace()
        guard let name = definedName(), let body = bodyGroup() else {
            i = resume
            return
        }
        definitions.append(MathDefinition(
            kind: .declareMathOperator, name: name,
            mathJaxSource: "\\DeclareMathOperator" + (starred ? "*" : "") + "{" + name + "}" + body,
            originalSource: string(start, i),
            range: .lexerRange(offset: start, length: i - start)
        ))
    }

    mutating func parseEnvironmentDefinition(start: Int, kind: MathDefinition.Kind) {
        let resume = i
        if i < count, b[i] == star { i += 1 }
        skipInterArgumentSpace()
        guard let (nameStart, nameEnd, nameAfter) = balancedGroup() else { i = resume; return }
        let name = trimmed(nameStart, nameEnd)
        i = nameAfter
        guard !name.isEmpty, let spec = argumentSpec(), let begin = bodyGroup(), let end = bodyGroup() else {
            i = resume
            return
        }
        let command = kind == .renewEnvironment ? "\\renewenvironment" : "\\newenvironment"
        definitions.append(MathDefinition(
            kind: kind, name: name,
            mathJaxSource: command + "{" + name + "}" + spec + begin + end,
            originalSource: string(start, i),
            range: .lexerRange(offset: start, length: i - start)
        ))
    }
}

