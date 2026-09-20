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

        // One pass over the scalars maps every UTF-8 boundary to its UTF-16
        // offset; per-token samePosition(in:) walks from the start each time
        // and makes the pass quadratic on large documents.
        var utf8Bounds: [Int] = [0]
        var utf16Bounds: [Int] = [0]
        utf8Bounds.reserveCapacity(text.unicodeScalars.count + 1)
        utf16Bounds.reserveCapacity(text.unicodeScalars.count + 1)
        for scalar in text.unicodeScalars {
            utf8Bounds.append(utf8Bounds[utf8Bounds.count - 1] + scalar.utf8.count)
            utf16Bounds.append(utf16Bounds[utf16Bounds.count - 1] + scalar.utf16.count)
        }
        func utf16Offset(forUTF8 offset: Int) -> Int? {
            var lo = 0, hi = utf8Bounds.count - 1
            while lo < hi {
                let mid = (lo + hi) / 2
                if utf8Bounds[mid] < offset { lo = mid + 1 } else { hi = mid }
            }
            return utf8Bounds[lo] == offset ? utf16Bounds[lo] : nil
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

        storage.beginEditing()
        storage.removeAttribute(.foregroundColor, range: fullRange)
        storage.addAttribute(.foregroundColor, value: bodyColor, range: fullRange)

        for token in tokens {
            guard let start16 = utf16Offset(forUTF8: token.range.utf8Offset),
                  let end16 = utf16Offset(forUTF8: token.range.endUTF8Offset),
                  end16 >= start16
            else { continue }
            let color: NSColor
            switch token.kind {
            case .controlSequence: color = commandColor
            case .comment: color = commentColor
            case .leftBrace, .rightBrace: color = braceColor
            case .bibEntryMarker, .environmentName: color = environmentColor
            case .math: color = mathColor
            case .punctuation: color = punctuationColor
            case .whitespace, .text: color = bodyColor
            }
            storage.addAttribute(
                .foregroundColor,
                value: color,
                range: NSRange(location: start16, length: end16 - start16)
            )
        }
        storage.endEditing()
    }

}
