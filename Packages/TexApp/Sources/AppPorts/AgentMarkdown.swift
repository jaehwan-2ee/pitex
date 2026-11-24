import Foundation

/// Assistant-message Markdown for the agent transcript: a small CommonMark
/// subset (GFM tables, fenced code) plus TeX math, parsed into the block
/// model the panel draws. The GTK host has the same model and cases in
/// `Linux/crates/pitex-shell/src/chat_markdown.rs`; keep the two in step.
///
/// The renderer is host-side, so every backend (pi, Claude Code,
/// Antigravity, ...) gets it from plain `text` deltas.
public enum AgentMarkdown {
    /// Characters drawn per message; the rest is cut with a note (Paseo's
    /// assistant-message render limit).
    public static let renderLimit = 32_000

    /// Quotes, lists and inline links or emphasis nest at most this deep;
    /// deeper markers stay text, so `>>>>…` cannot overflow the parser or the view tree.
    static let maxDepth = 16

    /// Inline scan steps per parse: unclosed openers (`[[[…`, `*a *a …`)
    /// rescan the rest of a paragraph each, so pathological text stays
    /// literal past this instead of stalling every streaming update.
    static let scanFuel = 2_000_000

    /// Scan steps left in one parse, and the current inline nesting.
    struct Budget {
        var fuel = scanFuel
        var depth = 0
    }

    public enum Block: Equatable, Sendable {
        case paragraph([Inline])
        case heading(Int, [Inline])
        case list(ordered: Bool, start: Int, items: [[Block]])
        case quote([Block])
        case code(language: String, text: String)
        /// Display math (`$$…$$`, `\[…\]`, a ```math fence), TeX without delimiters.
        case math(String)
        /// Rows of cells; the first row is the header.
        case table([[[Inline]]])
        case rule
    }

    public enum Inline: Equatable, Sendable {
        case text(String)
        case strong([Inline])
        case emphasis([Inline])
        case code(String)
        case link(url: String, children: [Inline])
        /// Inline math (`$…$`, `\(…\)`), TeX without delimiters.
        case math(String)
    }

    public struct Document: Equatable, Sendable {
        public var blocks: [Block]
        /// The message was longer than `renderLimit` and was cut.
        public var truncated: Bool
    }

    /// Where a link in a message goes.
    public enum LinkTarget: Equatable, Sendable {
        case web(URL)
        /// A file inside the project, with an optional 1-based line.
        case file(URL, line: Int?)
        /// Anything else (other schemes, paths outside the project): not clickable.
        case unsupported
    }

    /// The last streaming head and its blocks: closed blocks are parsed once.
    private final class HeadCache: @unchecked Sendable {
        let lock = NSLock()
        var head: String?
        var blocks: [Block] = []
        /// Finished messages by text: SwiftUI re-evaluates every row's body
        /// on each transcript or math update.
        var finished: [String: Document] = [:]
    }

    private static let headCache = HeadCache()

    /// Parses a message. While `streaming`, the closed head is parsed once
    /// and reused; only the open tail is parsed again, and an unclosed fence
    /// or display-math delimiter in it stays literal text until it closes.
    public static func parse(_ text: String, streaming: Bool) -> Document {
        headCache.lock.lock()
        defer { headCache.lock.unlock() }
        if !streaming, let document = headCache.finished[text] { return document }
        let (capped, truncated) = cap(text)
        let bytes = Array(capped.utf8)
        var budget = Budget()
        guard streaming else {
            let document = Document(blocks: parseBlocks(lines(bytes), streaming: false, depth: 0, &budget), truncated: truncated)
            // shortcut: dropped wholesale at 64 messages; an LRU if long chats thrash it.
            if headCache.finished.count >= 64 { headCache.finished.removeAll() }
            headCache.finished[text] = document
            return document
        }
        let split = stablePrefixLength(bytes)
        let head = string(bytes[..<split])
        var blocks: [Block]
        if headCache.head == head {
            blocks = headCache.blocks
        } else {
            blocks = parseBlocks(lines(Array(bytes[..<split])), streaming: false, depth: 0, &budget)
            headCache.head = head
            headCache.blocks = blocks
        }
        blocks += parseBlocks(lines(Array(bytes[split...])), streaming: true, depth: 0, &budget)
        return Document(blocks: blocks, truncated: truncated)
    }

    /// The first `renderLimit` Unicode scalars, and whether anything was cut.
    public static func cap(_ text: String) -> (String, Bool) {
        let scalars = text.unicodeScalars
        guard let end = scalars.index(scalars.startIndex, offsetBy: renderLimit, limitedBy: scalars.endIndex),
              end < scalars.endIndex else { return (text, false) }
        return (String(scalars[..<end]), true)
    }

    /// Byte length of the closed head of a streaming message: everything
    /// before the last top-level block that starts after a blank line,
    /// outside any fence or display math. Only complete lines count (Paseo
    /// `splitStreamingMarkdown`).
    public static func stablePrefixLength(_ text: String) -> Int { stablePrefixLength(Array(text.utf8)) }

    // MARK: - Lines and blocks

    typealias Bytes = [UInt8]
    private static let space: UInt8 = 32, tab: UInt8 = 9, newline: UInt8 = 10, cr: UInt8 = 13
    private static let backtick: UInt8 = 96, tilde: UInt8 = 126, hash: UInt8 = 35, pipe: UInt8 = 124
    private static let dollar: UInt8 = 36, backslash: UInt8 = 92, star: UInt8 = 42, underscore: UInt8 = 95

    static func string<S: Sequence>(_ bytes: S) -> String where S.Element == UInt8 { String(decoding: Array(bytes), as: UTF8.self) }
    private static func isSpace(_ b: UInt8) -> Bool { b == space || b == tab || b == newline || b == cr }
    private static func isBlank(_ b: Bytes) -> Bool { b.allSatisfy(isSpace) }
    private static func trim(_ b: Bytes) -> Bytes { Array(b.drop(while: isSpace).reversed().drop(while: isSpace).reversed()) }
    private static func trimEnd(_ b: Bytes) -> Bytes { Array(b.reversed().drop(while: isSpace).reversed()) }
    private static func hasPrefix(_ b: Bytes, _ p: String) -> Bool { b.starts(with: p.utf8) }
    private static func hasSuffix(_ b: Bytes, _ p: String) -> Bool { b.reversed().starts(with: p.utf8.reversed()) }
    private static func isAlnum(_ b: UInt8) -> Bool { (48...57).contains(b) || (65...90).contains(b) || (97...122).contains(b) }
    private static func isDigit(_ b: UInt8) -> Bool { (48...57).contains(b) }
    private static func isPunctuation(_ b: UInt8) -> Bool {
        (33...47).contains(b) || (58...64).contains(b) || (91...96).contains(b) || (123...126).contains(b)
    }
    private static func run(_ b: Bytes, from i: Int, of c: UInt8) -> Int { b[i...].prefix { $0 == c }.count }
    private static func find(_ b: Bytes, _ needle: String, from start: Int) -> Int? {
        let n = Array(needle.utf8)
        guard start <= b.count - n.count else { return nil }
        return (start...(b.count - n.count)).first { b[$0..<$0 + n.count].elementsEqual(n) }
    }

    /// `find` that spends fuel on what it scanned.
    private static func find(_ b: Bytes, _ needle: String, from start: Int, _ budget: inout Budget) -> Int? {
        guard budget.fuel > 0 else { return nil }
        let found = find(b, needle, from: start)
        budget.fuel -= (found ?? b.count) - start
        return found
    }

    private static func lines(_ text: Bytes) -> [Bytes] {
        guard !text.isEmpty else { return [] }
        let body = text.last == newline ? Array(text.dropLast()) : text
        return body.split(separator: newline, omittingEmptySubsequences: false).map { line in
            line.last == cr ? Array(line.dropLast()) : Array(line)
        }
    }

    static func stablePrefixLength(_ text: Bytes) -> Int {
        let complete = (text.lastIndex(of: newline) ?? -1) + 1
        var boundary = 0, offset = 0, previousBlank = false
        var fence: (UInt8, Int)?
        var mathCloser: String?
        for raw in text[..<complete].split(separator: newline, omittingEmptySubsequences: false).dropLast() {
            let start = offset
            offset += raw.count + 1
            let line = raw.last == cr ? Array(raw.dropLast()) : Array(raw)
            if case let (ch, len)? = fence {
                if isFenceClose(line, ch, len) { fence = nil }
                continue
            }
            let blank = isBlank(line)
            if let closer = mathCloser {
                mathCloser = nil
                // Display math cannot span a blank line (TeX rejects it): it was text.
                if !blank {
                    if !closesMath(line, closer) { mathCloser = closer }
                    continue
                }
            }
            if previousBlank, !blank, line.first != space, line.first != tab { boundary = start }
            previousBlank = blank
            let (indent, rest) = indentation(line)
            guard indent <= 3 else { continue }
            if let open = fenceOpen(rest) {
                fence = (open.0, open.1)
            } else if case let (opener, closer)? = mathDelimiters(rest) {
                if mathBody(rest).count < opener.utf8.count + closer.utf8.count || !closesMath(rest, closer) { mathCloser = closer }
            }
        }
        return boundary
    }

    private static func indentation(_ line: Bytes) -> (Int, Bytes) {
        let count = line.prefix { $0 == space }.count
        return (count, Array(line[count...]))
    }

    /// ``` or ~~~ (3+), its length and the info string's first word.
    private static func fenceOpen(_ rest: Bytes) -> (UInt8, Int, String)? {
        guard let ch = rest.first, ch == backtick || ch == tilde else { return nil }
        let len = run(rest, from: 0, of: ch)
        let info = string(trim(Array(rest[len...])))
        // A backtick fence's info string cannot contain a backtick (that is inline code).
        guard len >= 3, !(ch == backtick && info.contains("`")) else { return nil }
        return (ch, len, info.split(whereSeparator: \.isWhitespace).first.map(String.init) ?? "")
    }

    private static func isFenceClose(_ line: Bytes, _ ch: UInt8, _ len: Int) -> Bool {
        let (indent, rest) = indentation(line)
        let count = run(rest, from: 0, of: ch)
        return indent <= 3 && count >= len && isBlank(Array(rest[count...]))
    }

    /// `$$` or `\[` at the start of a line, with the closer to look for.
    private static func mathDelimiters(_ rest: Bytes) -> (String, String)? {
        if hasPrefix(rest, "$$") { return ("$$", "$$") }
        if hasPrefix(rest, "\\[") { return ("\\[", "\\]") }
        return nil
    }

    /// A display-math line without trailing space or sentence punctuation
    /// (`$$.` closes like `$$`).
    private static func mathBody(_ line: Bytes) -> Bytes {
        trimEnd(Array(trimEnd(line).reversed().drop { [46, 44, 59, 58].contains($0) }.reversed()))
    }

    private static func closesMath(_ line: Bytes, _ closer: String) -> Bool { hasSuffix(mathBody(line), closer) }

    private static func isRule(_ rest: Bytes) -> Bool {
        let marks = rest.filter { !isSpace($0) }
        guard let first = marks.first, first == 45 || first == star || first == underscore else { return false }
        return marks.count >= 3 && marks.allSatisfy { $0 == first }
    }

    private static func heading(_ rest: Bytes) -> (Int, Bytes)? {
        let level = run(rest, from: 0, of: hash)
        guard (1...6).contains(level) else { return nil }
        let body = Array(rest[level...])
        if let first = body.first, first != space, first != tab { return nil }
        let trimmed = trim(body)
        // An optional closing run of #'s after a space.
        let closed = Array(trimmed.reversed().drop { $0 == hash }.reversed())
        return (level, closed.isEmpty || closed.last == space ? trimEnd(closed) : trimmed)
    }

    /// A list marker: (indent, ordered, number, content column).
    private static func listMarker(_ line: Bytes) -> (indent: Int, ordered: Bool, number: Int, column: Int)? {
        let (indent, rest) = indentation(line)
        guard let first = rest.first else { return nil }
        var ordered = false, number = 0, markerLength = 1
        if first == 45 || first == star || first == 43 {
            markerLength = 1
        } else if isDigit(first) {
            let digits = rest.prefix(while: isDigit).count
            guard digits <= 9, digits < rest.count, rest[digits] == 46 || rest[digits] == 41 else { return nil }
            ordered = true
            number = Int(string(rest[..<digits])) ?? 0
            markerLength = digits + 1
        } else {
            return nil
        }
        let after = Array(rest[markerLength...])
        if after.isEmpty { return (indent, ordered, number, indent + markerLength + 1) }
        let spaces = after.prefix { $0 == space }.count
        guard spaces > 0 else { return nil }
        // Five or more spaces: the content starts after one (CommonMark).
        return (indent, ordered, number, indent + markerLength + (spaces > 4 ? 1 : spaces))
    }

    private static func strippedRow(_ line: Bytes) -> Bytes {
        var row = trim(line)
        if row.first == pipe { row.removeFirst() }
        if row.last == pipe, !hasSuffix(row, "\\|") { row.removeLast() }
        return row
    }

    private static func isDelimiterRow(_ line: Bytes) -> Bool {
        let row = strippedRow(line)
        guard !isBlank(row) else { return false }
        return row.split(separator: pipe, omittingEmptySubsequences: false).allSatisfy { cell in
            var core = trim(Array(cell))
            if core.first == 58 { core.removeFirst() }
            if core.last == 58 { core.removeLast() }
            return !core.isEmpty && core.allSatisfy { $0 == 45 }
        }
    }

    /// A table needs a delimiter row with as many cells as its header (GFM).
    private static func startsTable(_ lines: [Bytes], _ i: Int) -> Bool {
        lines[i].contains(pipe) && i + 1 < lines.count && isDelimiterRow(lines[i + 1])
            && splitCells(lines[i + 1]).count == splitCells(lines[i]).count
    }

    private static func tableCells(_ line: Bytes, _ budget: inout Budget) -> [[Inline]] {
        splitCells(line).map { inlines(Array(string(trim($0)).replacingOccurrences(of: "\\|", with: "|").utf8), &budget) }
    }

    private static func splitCells(_ line: Bytes) -> [Bytes] {
        let row = strippedRow(line)
        var cells: [Bytes] = []
        var start = 0, inCode = false, escaped = false
        for (i, b) in row.enumerated() {
            if b == backslash, !escaped {
                escaped = true
                continue
            }
            if b == backtick { inCode.toggle() }
            if b == pipe, !escaped, !inCode {
                cells.append(Array(row[start..<i]))
                start = i + 1
            }
            escaped = false
        }
        cells.append(Array(row[start...]))
        return cells
    }

    /// A line that starts a block other than a paragraph.
    private static func startsBlock(_ line: Bytes) -> Bool {
        let (indent, rest) = indentation(line)
        return indent <= 3 && (fenceOpen(rest) != nil || heading(rest) != nil || rest.first == 62
            || isRule(rest) || mathDelimiters(rest) != nil || listMarker(line) != nil)
    }

    /// Drops up to `count` leading spaces.
    private static func dedent(_ line: Bytes, _ count: Int) -> Bytes {
        Array(line.dropFirst(line.prefix(count).prefix { $0 == space }.count))
    }

    private static func joined(_ lines: ArraySlice<Bytes>, trimming: Bool = false) -> String {
        lines.map { string(trimming ? trim($0) : $0) }.joined(separator: "\n")
    }

    private static func parseBlocks(_ lines: [Bytes], streaming: Bool, depth: Int, _ budget: inout Budget) -> [Block] {
        var blocks: [Block] = []
        var paragraph: [Bytes] = []
        func flush() {
            guard !paragraph.isEmpty else { return }
            blocks.append(.paragraph(inlines(Array(joined(paragraph[...], trimming: true).utf8), &budget)))
            paragraph.removeAll()
        }
        var i = 0
        scan: while i < lines.count {
            let line = lines[i]
            let (indent, rest) = indentation(line)
            if isBlank(rest) {
                flush()
                i += 1
                continue
            }
            if indent > 3 {
                paragraph.append(line)
                i += 1
                continue
            }
            if case let (ch, len, info)? = fenceOpen(rest) {
                let close = ((i + 1)..<lines.count).first(where: { isFenceClose(lines[$0], ch, len) })
                flush()
                if close == nil, streaming, info.lowercased() == "math" {
                    // Still streaming: a math fence stays literal until it closes
                    // (no pdflatex on half a formula); code runs to the end.
                    blocks.append(.paragraph([.text(joined(lines[i...]))]))
                    break scan
                }
                let end = close ?? lines.count
                let text = joined(lines[(i + 1)..<end].map { dedent($0, indent) }[...])
                blocks.append(info.lowercased() == "math"
                    ? .math(text.trimmingCharacters(in: .whitespacesAndNewlines))
                    : .code(language: info, text: text))
                i = end + 1
                continue
            }
            if case let (opener, closer)? = mathDelimiters(rest) {
                let first = mathBody(rest)
                let single = first.count >= opener.utf8.count + closer.utf8.count + 1 && hasSuffix(first, closer)
                // Display math ends at its closer, and never spans a blank line.
                let blockEnd = ((i + 1)..<lines.count).first(where: { isBlank(lines[$0]) }) ?? lines.count
                let close = single ? i : ((i + 1)..<blockEnd).first(where: { closesMath(lines[$0], closer) })
                // `$$a$$ and $$b$$` on one line is inline math in a paragraph.
                let inline = single && find(Array(first[opener.utf8.count..<(first.count - closer.utf8.count)]), opener, from: 0) != nil
                if let end = close, !inline {
                    flush()
                    let body = mathBody(Array(joined(lines[i...end], trimming: true).utf8))
                    let tex = string(body[opener.utf8.count..<(body.count - closer.utf8.count)])
                    blocks.append(.math(tex.trimmingCharacters(in: .whitespacesAndNewlines)))
                    i = end + 1
                    continue
                }
                if close == nil, streaming, blockEnd == lines.count {
                    flush()
                    blocks.append(.paragraph([.text(joined(lines[i...]))]))
                    break scan
                }
            }
            if case let (level, body)? = heading(rest) {
                flush()
                blocks.append(.heading(level, inlines(body, &budget)))
                i += 1
                continue
            }
            if isRule(rest) {
                flush()
                blocks.append(.rule)
                i += 1
                continue
            }
            if rest.first == 62, depth < maxDepth {
                flush()
                var inner: [Bytes] = []
                while i < lines.count {
                    let (indent, rest) = indentation(lines[i])
                    guard indent <= 3, rest.first == 62 else { break }
                    let body = Array(rest.dropFirst())
                    inner.append(body.first == space ? Array(body.dropFirst()) : body)
                    i += 1
                }
                blocks.append(.quote(parseBlocks(inner, streaming: streaming, depth: depth + 1, &budget)))
                continue
            }
            if depth < maxDepth, let marker = listMarker(line) {
                flush()
                var items: [[Block]] = []
                while i < lines.count, let item = listMarker(lines[i]), item.ordered == marker.ordered {
                    let column = item.column
                    var body: [Bytes] = [lines[i].count > column ? Array(lines[i][column...]) : []]
                    i += 1
                    while i < lines.count {
                        let line = lines[i]
                        if isBlank(line) {
                            // A blank line stays in the item only when indented content follows.
                            let next = ((i + 1)..<lines.count).first(where: { !isBlank(lines[$0]) })
                            if let j = next, indentation(lines[j]).0 >= column {
                                body.append([])
                                i += 1
                            } else if let j = next, let m = listMarker(lines[j]), m.ordered == marker.ordered, m.indent < column {
                                i = j
                            } else {
                                break
                            }
                            if i < lines.count, let m = listMarker(lines[i]), m.indent < column { break }
                            continue
                        }
                        if indentation(line).0 >= column {
                            body.append(Array(line[column...]))
                        } else if listMarker(line) != nil || startsBlock(line) || body.last == [] {
                            break
                        } else {
                            // Lazy continuation of the item's paragraph.
                            body.append(Array(line.drop { $0 == space }))
                        }
                        i += 1
                    }
                    items.append(parseBlocks(body, streaming: streaming, depth: depth + 1, &budget))
                    // The list ends at a blank line not followed by a sibling item.
                    if i < lines.count, isBlank(lines[i]) { break }
                }
                blocks.append(.list(ordered: marker.ordered, start: marker.number, items: items))
                continue
            }
            if startsTable(lines, i) {
                flush()
                var rows = [tableCells(line, &budget)]
                i += 2
                while i < lines.count, !isBlank(lines[i]), lines[i].contains(pipe) {
                    rows.append(tableCells(lines[i], &budget))
                    i += 1
                }
                blocks.append(.table(rows))
                continue
            }
            paragraph.append(line)
            i += 1
        }
        flush()
        return blocks
    }

    // MARK: - Inlines

    /// Inline content: emphasis, code spans, links, autolinks and math.
    /// Math is recognised outside code spans only; `\$` and currency such as
    /// `$5 and $10` stay literal (a `$` before a digit or a space opens nothing).
    public static func parseInlines(_ text: String) -> [Inline] {
        var budget = Budget()
        return inlines(Array(text.utf8), &budget)
    }

    /// Inline content of a link label or emphasis; past `maxDepth` it stays text.
    private static func nested(_ b: Bytes, _ budget: inout Budget) -> [Inline] {
        guard budget.depth < maxDepth else { return [.text(string(b))] }
        budget.depth += 1
        defer { budget.depth -= 1 }
        return inlines(b, &budget)
    }

    private static func inlines(_ b: Bytes, _ budget: inout Budget) -> [Inline] {
        var out: [Inline] = []
        var text: Bytes = []
        func pushText() {
            guard !text.isEmpty else { return }
            if case .text(let last)? = out.last {
                out[out.count - 1] = .text(last + string(text))
            } else {
                out.append(.text(string(text)))
            }
            text.removeAll()
        }
        var i = 0
        while i < b.count {
            let c = b[i]
            var parsed: (Inline, Int)?
            switch c {
            case backslash where i + 1 < b.count && b[i + 1] == 40: parsed = delimitedMath(b, i + 2, "\\)", &budget)
            case backslash where i + 1 < b.count && b[i + 1] == 91: parsed = delimitedMath(b, i + 2, "\\]", &budget)
            case backtick: parsed = codeSpan(b, i, &budget)
            case dollar: parsed = dollarMath(b, i, &budget)
            case star, underscore: parsed = emphasis(b, i, &budget)
            case 91: parsed = link(b, i, &budget)
            case 60: parsed = autolink(b, i, &budget)
            case 104 where (i == 0 || !isAlnum(b[i - 1])) && (hasPrefix(Array(b[i...]), "https://") || hasPrefix(Array(b[i...]), "http://")):
                parsed = bareURL(b, i)
            default: break
            }
            if case let (node, next)? = parsed {
                pushText()
                out.append(node)
                i = next
                continue
            }
            // An image (`![alt](src)`) is not drawn: it stays its source text.
            if c == 33, i + 1 < b.count, b[i + 1] == 91, case let (_, next)? = link(b, i + 1, &budget) {
                text += b[i..<next]
                i = next
                continue
            }
            if c == backslash, i + 1 < b.count, isPunctuation(b[i + 1]) {
                text.append(b[i + 1])
                i += 2
            } else if [backtick, star, underscore, dollar].contains(c) {
                // An unmatched run stays literal as a whole.
                let count = run(b, from: i, of: c)
                text += b[i..<i + count]
                i += count
            } else {
                text.append(c)
                i += 1
            }
        }
        pushText()
        return out
    }

    private static func delimitedMath(_ b: Bytes, _ start: Int, _ closer: String, _ budget: inout Budget) -> (Inline, Int)? {
        guard let end = find(b, closer, from: start, &budget) else { return nil }
        let tex = string(trim(Array(b[start..<end])))
        return tex.isEmpty ? nil : (.math(tex), end + closer.utf8.count)
    }

    private static func codeSpan(_ b: Bytes, _ i: Int, _ budget: inout Budget) -> (Inline, Int)? {
        guard budget.fuel > 0 else { return nil }
        let count = run(b, from: i, of: backtick)
        var j = i + count
        while j < b.count {
            guard b[j] == backtick else {
                j += 1
                continue
            }
            let close = run(b, from: j, of: backtick)
            if close == count {
                budget.fuel -= j - i
                var code = string(b[(i + count)..<j]).replacingOccurrences(of: "\n", with: " ")
                if code.count >= 2, code.hasPrefix(" "), code.hasSuffix(" "), !code.trimmingCharacters(in: .whitespaces).isEmpty {
                    code = String(code.dropFirst().dropLast())
                }
                return (.code(code), j + close)
            }
            j += close
        }
        budget.fuel -= j - i
        return nil
    }

    private static func dollarMath(_ b: Bytes, _ i: Int, _ budget: inout Budget) -> (Inline, Int)? {
        if i + 1 < b.count, b[i + 1] == dollar {
            guard let end = find(b, "$$", from: i + 2, &budget) else { return nil }
            let tex = string(trim(Array(b[(i + 2)..<end])))
            return tex.isEmpty ? nil : (.math(tex), end + 2)
        }
        guard i + 1 < b.count, !isSpace(b[i + 1]), !isDigit(b[i + 1]), budget.fuel > 0 else { return nil }
        var j = i + 1
        while j < b.count {
            if b[j] == backslash {
                j += 2
            } else if b[j] == dollar, !isSpace(b[j - 1]), !(j + 1 < b.count && isAlnum(b[j + 1])) {
                // Not before a digit or letter: `$5 and $10`, `$FOO$BAR` stay text (Pandoc).
                budget.fuel -= j - i
                return (.math(string(b[(i + 1)..<j])), j + 1)
            } else {
                j += 1
            }
        }
        budget.fuel -= j - i
        return nil
    }

    private static func emphasis(_ b: Bytes, _ i: Int, _ budget: inout Budget) -> (Inline, Int)? {
        let c = b[i]
        let count = min(run(b, from: i, of: c), 3)
        guard i + count < b.count, !isSpace(b[i + count]), c != underscore || i == 0 || !isAlnum(b[i - 1]),
              budget.fuel > 0 else { return nil }
        var j = i + count + 1
        while j < b.count {
            // Delimiters inside a code span do not close.
            if b[j] == backtick, case let (_, next)? = codeSpan(b, j, &budget) {
                j = next
                continue
            }
            guard b[j] == c else {
                j += 1
                continue
            }
            let close = run(b, from: j, of: c)
            let after = j + close
            if close == count, !isSpace(b[j - 1]), c != underscore || after >= b.count || !isAlnum(b[after]) {
                budget.fuel -= j - i
                let inner = nested(Array(b[(i + count)..<j]), &budget)
                switch count {
                case 1: return (.emphasis(inner), after)
                case 2: return (.strong(inner), after)
                default: return (.strong([.emphasis(inner)]), after)
                }
            }
            j = after
        }
        budget.fuel -= j - i
        return nil
    }

    /// Index of the bracket that closes the one at `start` (`[`/`]` or `(`/`)`).
    private static func matching(_ b: Bytes, _ start: Int, open: UInt8, close: UInt8, stopAtNewline: Bool) -> Int? {
        var depth = 0, j = start
        while j < b.count {
            switch b[j] {
            case backslash: j += 1
            case open: depth += 1
            case close:
                depth -= 1
                if depth == 0 { return j }
            case newline where stopAtNewline: return nil
            default: break
            }
            j += 1
        }
        return nil
    }

    /// `[text](url)` starting at `i` (also the `[alt](src)` of an image).
    private static func link(_ b: Bytes, _ i: Int, _ budget: inout Budget) -> (Inline, Int)? {
        guard budget.fuel > 0 else { return nil }
        let labelEnd = matching(b, i, open: 91, close: 93, stopAtNewline: false)
        let opensDest = labelEnd.map { $0 + 1 < b.count && b[$0 + 1] == 40 } ?? false
        let destEnd = opensDest ? labelEnd.flatMap { matching(b, $0 + 1, open: 40, close: 41, stopAtNewline: true) } : nil
        // A failed scan ran to the end of the text (or of the line, for the destination).
        switch (labelEnd, destEnd) {
        case (let end?, nil) where !opensDest: budget.fuel -= end - i
        case (_, let end?): budget.fuel -= end - i
        default: budget.fuel -= b.count - i
        }
        guard let labelEnd, let destEnd else { return nil }
        let dest = string(trim(Array(b[(labelEnd + 2)..<destEnd])))
        let url: String
        if dest.hasPrefix("<"), let close = dest.firstIndex(of: ">") {
            url = String(dest[dest.index(after: dest.startIndex)..<close])
        } else {
            url = dest.split(whereSeparator: \.isWhitespace).first.map(String.init) ?? ""
        }
        return (.link(url: url, children: nested(Array(b[(i + 1)..<labelEnd]), &budget)), destEnd + 1)
    }

    private static func autolink(_ b: Bytes, _ i: Int, _ budget: inout Budget) -> (Inline, Int)? {
        guard let end = find(b, ">", from: i, &budget) else { return nil }
        let url = string(b[(i + 1)..<end])
        guard url.hasPrefix("https://") || url.hasPrefix("http://"), !url.contains(where: \.isWhitespace) else { return nil }
        return (.link(url: url, children: [.text(url)]), end + 1)
    }

    private static func bareURL(_ b: Bytes, _ i: Int) -> (Inline, Int)? {
        let end = b[i...].firstIndex { isSpace($0) || $0 == 60 } ?? b.count
        var url = Array(b[i..<end])
        // Trailing punctuation ends the sentence, not the URL.
        let trailing: Set<UInt8> = [46, 44, 59, 58, 33, 63, 34, 39, star, underscore]
        while true {
            var trimmed = Array(url.reversed().drop { trailing.contains($0) }.reversed())
            if trimmed.last == 41, trimmed.filter({ $0 == 40 }).count < trimmed.filter({ $0 == 41 }).count {
                trimmed.removeLast()
            }
            if trimmed.count == url.count { break }
            url = trimmed
        }
        guard url.count > "https://".utf8.count else { return nil }
        let text = string(url)
        return (.link(url: text, children: [.text(text)]), i + url.count)
    }

    // MARK: - Links

    /// http(s) opens in the browser; a relative or absolute path inside the
    /// project (optionally `:line`, `:line:col` or `#Lnn`) opens in the editor.
    public static func classifyLink(_ url: String, root: URL?) -> LinkTarget {
        let lower = url.lowercased()
        if lower.hasPrefix("http://") || lower.hasPrefix("https://") {
            return URL(string: url).map(LinkTarget.web) ?? .unsupported
        }
        guard let root else { return .unsupported }
        let raw = lower.hasPrefix("file://") ? String(url.dropFirst(7)) : url
        let (path, line) = splitLineSuffix(raw)
        if path.isEmpty || path.contains("://") || ["mailto:", "javascript:", "data:"].contains(where: { lower.hasPrefix($0) }) {
            return .unsupported
        }
        let file = (path.hasPrefix("/") ? URL(fileURLWithPath: path) : root.appendingPathComponent(path)).standardizedFileURL
        func inside(_ file: URL, _ root: URL) -> Bool {
            let base = root.standardizedFileURL.path
            return file.path.hasPrefix(base.hasSuffix("/") ? base : base + "/")
        }
        guard inside(file, root) || inside(file.resolvingSymlinksInPath(), root.resolvingSymlinksInPath()) else {
            return .unsupported
        }
        return .file(file, line: line)
    }

    /// `main.tex:12`, `main.tex:12:3` and `main.tex#L12` → (`main.tex`, 12);
    /// any other fragment (`README.md#usage`) is dropped.
    static func splitLineSuffix(_ path: String) -> (String, Int?) {
        if let hash = path.lastIndex(of: "#") {
            let anchor = path[path.index(after: hash)...]
            let digits = anchor.hasPrefix("L") ? anchor.dropFirst().prefix(while: { $0.isASCII && $0.isNumber }) : ""
            return (String(path[..<hash]), Int(digits).flatMap { $0 > 0 ? $0 : nil })
        }
        var file = Substring(path)
        var numbers: [Substring] = []
        while numbers.count < 2, let colon = file.lastIndex(of: ":") {
            let tail = file[file.index(after: colon)...]
            guard !tail.isEmpty, tail.allSatisfy({ $0.isASCII && $0.isNumber }) else { break }
            numbers.append(tail)
            file = file[..<colon]
        }
        return (String(file), numbers.last.flatMap { Int($0) }.flatMap { $0 > 0 ? $0 : nil })
    }
}
