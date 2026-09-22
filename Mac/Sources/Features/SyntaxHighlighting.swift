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
    private var cachedTokens: [LanguageToken]?
    private var dialect: TeXDialect?
    private var cachedStarts: [Int]?
    nonisolated(unsafe) private var observer: NSObjectProtocol?

    private init(_ storage: NSTextStorage) {
        self.storage = storage
        observer = NotificationCenter.default.addObserver(forName: NSTextStorage.didProcessEditingNotification,
            object: storage, queue: .main) { [weak self] notification in
            MainActor.assumeIsolated {
                guard let storage = notification.object as? NSTextStorage,
                      storage.editedMask.contains(.editedCharacters) else { return }
                self?.cachedTokens = nil
                self?.cachedStarts = nil
            }
        }
    }
    deinit { if let observer { NotificationCenter.default.removeObserver(observer) } }

    func tokens(for dialect: TeXDialect) -> [LanguageToken] {
        if cachedTokens == nil || self.dialect != dialect {
            cachedTokens = DeterministicTeXLexer.tokenize(storage?.string ?? "", dialect: dialect)
            self.dialect = dialect
        }
        return cachedTokens ?? []
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

    func attach(to adapter: EditorMacAdapter, fileExtension: String) {
        self.adapter = adapter
        dialect = fileExtension.lowercased() == "bib" ? .bibtex : .latex
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
        guard let textView = adapter?.textView, let storage = textView.textStorage else { return }
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

        // TextKit shifts attributes with edits. Keep correct runs intact
        // instead of clearing and repainting the whole document each time.
        func applyColor(_ color: NSColor, from start: Int, to end: Int) {
            guard end > start else { return }
            var changed: [NSRange] = []
            storage.enumerateAttribute(.foregroundColor,
                in: NSRange(location: start, length: end - start),
                options: .longestEffectiveRangeNotRequired) { current, range, _ in
                if current as? NSColor != color { changed.append(range) }
            }
            for range in changed { storage.addAttribute(.foregroundColor, value: color, range: range) }
        }
        storage.beginEditing()
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
            case .whitespace, .text: continue // Coalesced into the gaps below.
            }
            guard let start16 = utf16Offset(forUTF8: token.range.utf8Offset),
                  let end16 = utf16Offset(forUTF8: token.range.endUTF8Offset),
                  end16 >= start16
            else { continue }
            applyColor(bodyColor, from: paintedThrough, to: start16)
            applyColor(color, from: start16, to: end16)
            paintedThrough = end16
        }
        applyColor(bodyColor, from: paintedThrough, to: fullRange.length)
        storage.endEditing()
    }

}
