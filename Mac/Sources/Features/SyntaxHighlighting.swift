import AppKit
import EditorMacAdapter
import LanguageCore

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
        let tokens = DeterministicTeXLexer.tokenize(text, dialect: dialect)
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
