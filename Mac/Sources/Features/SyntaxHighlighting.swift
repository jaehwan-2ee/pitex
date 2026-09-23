import AppKit
import EditorMacAdapter
import LanguageCore

/// A text storage owns one analysis; coloring and folding share its tokens
/// and line offsets until the next character edit. Attribute edits keep it valid.
@MainActor
final class EditorAnalysis {
    private static let entries = NSMapTable<NSTextStorage, EditorAnalysis>(keyOptions: [.weakMemory, .objectPointerPersonality],
                                                                          valueOptions: .strongMemory)
    static func shared(for storage: NSTextStorage) -> EditorAnalysis {
        if let value = entries.object(forKey: storage) { return value }
        let value = EditorAnalysis(storage)
        entries.setObject(value, forKey: storage)
        return value
    }
    private weak var storage: NSTextStorage?
    /// Per dialect: .bib files are lexed as .bibtex by the highlighter and
    /// .latex by FoldEngine — a single-slot cache thrashed between the two.
    private var cachedTokens: [TeXDialect: [LanguageToken]] = [:]
    private var cachedStarts: [Int]?
    nonisolated(unsafe) private var observer: NSObjectProtocol?

    private init(_ storage: NSTextStorage) {
        self.storage = storage
        observer = NotificationCenter.default.addObserver(forName: NSTextStorage.didProcessEditingNotification,
            object: storage, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let storage = self?.storage,
                      storage.editedMask.contains(.editedCharacters) else { return }
                self?.cachedTokens.removeAll()
                self?.cachedStarts = nil
            }
        }
    }
    deinit { if let observer { NotificationCenter.default.removeObserver(observer) } }

    func tokens(for dialect: TeXDialect) -> [LanguageToken] {
        if let tokens = cachedTokens[dialect] { return tokens }
        let tokens = DeterministicTeXLexer.tokenize(storage?.string ?? "", dialect: dialect)
        cachedTokens[dialect] = tokens
        return tokens
    }
    var lineStarts: [Int] {
        if let cachedStarts { return cachedStarts }
        let text = (storage?.string ?? "") as NSString
        var starts = [0]
        for index in 0..<text.length where text.character(at: index) == 10 { starts.append(index + 1) }
        cachedStarts = starts
        return starts
    }
}

/// Applies deterministic LaTeX/BibTeX token coloring to the editor's text
/// storage. Attribute-only mutations never produce textDidChange callbacks, so
/// rehighlighting cannot recurse or desynchronize the canonical session text.
@MainActor
final class SyntaxHighlighter {
    private weak var adapter: EditorMacAdapter?
    private var pendingTask: Task<Void, Never>?
    private var dialect: TeXDialect = .latex
    /// Markdown is rendered by the preview, not the LaTeX lexer — `.md` /
    /// `.markdown` documents keep plain body color in the editor.
    private var enabled = true

    func attach(to adapter: EditorMacAdapter, fileExtension: String) {
        self.adapter = adapter
        let ext = fileExtension.lowercased()
        enabled = ext != "md" && ext != "markdown"
        dialect = ext == "bib" ? .bibtex : .latex
        adapter.onTextDidChange = { [weak self] in
            self?.scheduleHighlight()
        }
        highlightNow()
    }

    func detach() {
        adapter?.onTextDidChange = nil
        pendingTask?.cancel()
        adapter = nil
    }

    private func scheduleHighlight() {
        pendingTask?.cancel()
        pendingTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(120))
            guard !Task.isCancelled else { return }
            self?.highlightNow()
        }
    }

    func highlightNow() {
        guard enabled, let textView = adapter?.textView, let storage = textView.textStorage else { return }
        // NSTextView bridges an NSString: UTF-8 offsets otherwise walk from
        // the beginning for every token, making even small documents quadratic.
        var text = textView.string
        text.makeContiguousUTF8()
        let tokens = EditorAnalysis.shared(for: storage).tokens(for: dialect)
        let fullRange = NSRange(location: 0, length: (text as NSString).length)

        // Lexer ranges are ordered. Advance one scalar cursor for their
        // UTF-16 boundaries instead of allocating two document-sized maps
        // and binary-searching both ends of every token.
        var scalars = text.unicodeScalars.makeIterator()
        var utf8Offset = 0
        var utf16Position = 0
        func utf16Offset(forUTF8 offset: Int) -> Int? {
            while utf8Offset < offset, let scalar = scalars.next() {
                utf8Offset += scalar.utf8.count
                utf16Position += scalar.utf16.count
            }
            return utf8Offset == offset ? utf16Position : nil
        }

        // Resolve the palette once per pass instead of a UserDefaults read +
        // hex parse per token.
        let appearance = AppearanceSettings.shared
        let bodyColor = appearance.color(for: .bodyText)
        let commandColor = appearance.color(for: .commands)
        let commentColor = appearance.color(for: .comments)
        let braceColor = appearance.color(for: .braces)
        let environmentColor = appearance.color(for: .environments)
        let mathColor = appearance.color(for: .math)
        let punctuationColor = appearance.color(for: .lineNumbers)

        // Build the desired color runs in one pass — gaps get body color,
        // tokens their role color. Lexer ranges are ordered and disjoint.
        var desired: [(range: NSRange, color: NSColor)] = []
        var paintedThrough = 0
        for token in tokens {
            let color: NSColor
            switch token.kind {
            case .controlSequence: color = commandColor
            case .comment: color = commentColor
            case .leftBrace, .rightBrace: color = braceColor
            case .bibEntryMarker, .environmentName: color = environmentColor
            case .math: color = mathColor
            case .punctuation: color = punctuationColor
            case .whitespace, .text: continue // Coalesced into the gaps.
            }
            guard let start16 = utf16Offset(forUTF8: token.range.utf8Offset),
                  let end16 = utf16Offset(forUTF8: token.range.endUTF8Offset),
                  end16 >= start16
            else { continue }
            if start16 > paintedThrough {
                desired.append((NSRange(location: paintedThrough, length: start16 - paintedThrough), bodyColor))
            }
            if end16 > start16 {
                desired.append((NSRange(location: start16, length: end16 - start16), color))
            }
            paintedThrough = end16
        }
        if paintedThrough < fullRange.length {
            desired.append((NSRange(location: paintedThrough, length: fullRange.length - paintedThrough), bodyColor))
        }

        // TextKit shifts attributes with edits. Keep correct runs intact:
        // enumerate the current colors once and write only where a desired
        // run disagrees — the same writes per-token enumerations produced,
        // without two AppKit calls per token.
        var current: [(range: NSRange, color: NSColor?)] = []
        storage.enumerateAttribute(.foregroundColor, in: fullRange,
            options: .longestEffectiveRangeNotRequired) { value, range, _ in
            current.append((range, value as? NSColor))
        }
        var changes: [(range: NSRange, color: NSColor)] = []
        var cursor = 0
        for run in desired {
            while cursor < current.count, NSMaxRange(current[cursor].range) <= run.range.location {
                cursor += 1
            }
            var index = cursor
            while index < current.count, current[index].range.location < NSMaxRange(run.range) {
                if current[index].color != run.color {
                    let overlap = NSIntersectionRange(current[index].range, run.range)
                    if overlap.length > 0 { changes.append((overlap, run.color)) }
                }
                index += 1
            }
        }
        storage.beginEditing()
        for change in changes {
            storage.addAttribute(.foregroundColor, value: change.color, range: change.range)
        }
        storage.endEditing()
    }

}
