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

        storage.beginEditing()
        storage.removeAttribute(.foregroundColor, range: fullRange)
        storage.addAttribute(
            .foregroundColor,
            value: AppearanceSettings.shared.color(for: .bodyText),
            range: fullRange
        )

        for token in tokens {
            let utf8Start = text.utf8.startIndex
            guard let lower = text.utf8.index(utf8Start, offsetBy: token.range.utf8Offset, limitedBy: text.utf8.endIndex),
                  let upper = text.utf8.index(utf8Start, offsetBy: token.range.endUTF8Offset, limitedBy: text.utf8.endIndex),
                  let lowerIndex = String.Index(lower, within: text),
                  let upperIndex = String.Index(upper, within: text),
                  let lower16 = lowerIndex.samePosition(in: text.utf16),
                  let upper16 = upperIndex.samePosition(in: text.utf16)
            else { continue }
            let start16 = text.utf16.distance(from: text.utf16.startIndex, to: lower16)
            let end16 = text.utf16.distance(from: text.utf16.startIndex, to: upper16)
            guard end16 >= start16 else { continue }
            let range = NSRange(location: start16, length: end16 - start16)
            storage.addAttribute(.foregroundColor, value: color(for: token.kind), range: range)
        }
        storage.endEditing()
    }

    private func color(for kind: LanguageTokenKind) -> NSColor {
        let appearance = AppearanceSettings.shared
        switch kind {
        case .controlSequence: return appearance.color(for: .commands)
        case .comment: return appearance.color(for: .comments)
        case .leftBrace, .rightBrace: return appearance.color(for: .braces)
        case .bibEntryMarker, .environmentName: return appearance.color(for: .environments)
        case .math: return appearance.color(for: .math)
        case .punctuation: return appearance.color(for: .lineNumbers)
        case .whitespace, .text: return appearance.color(for: .bodyText)
        }
    }
}
