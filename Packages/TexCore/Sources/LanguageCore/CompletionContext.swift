import Foundation

/// What the caret is completing — drives which candidate list the editor
/// shows. Mirrors `completion::CompletionContextKind` in the Rust
/// `language-core` port.
public enum CompletionContextKind: String, Hashable, Codable, Sendable {
    /// A `\command` prefix — candidates come from the built-in command list.
    case command
    /// Inside a `\cite{…}`-style group — candidates are project .bib keys.
    case citation
    /// Inside a `\ref{…}`-style group — candidates are project \label keys.
    case reference
}

/// A detected completion context at the caret.
public struct CompletionContext: Hashable, Codable, Sendable {
    public let kind: CompletionContextKind
    /// Text the chosen candidate replaces: for `.command` it includes the
    /// leading `\` (`\alp`), for group contexts it is the key fragment
    /// after `{` or the last `,` (`kn` in `\cite{a,kn`).
    public let prefix: String
    /// UTF-16 offset where `prefix` starts — the replacement range is
    /// `prefixUTF16Offset .. caretUTF16Offset`.
    public let prefixUTF16Offset: Int

    public init(kind: CompletionContextKind, prefix: String, prefixUTF16Offset: Int) {
        self.kind = kind
        self.prefix = prefix
        self.prefixUTF16Offset = prefixUTF16Offset
    }
}

/// Shared caret-context detection for command/citation/reference
/// completion — the same scan the Rust `CompletionContextDetector` runs so
/// both platforms trigger identical popups.
public enum CompletionContextDetector {
    /// Commands whose `{…}` argument holds citation keys. Broader than the
    /// outline parser's cite set so natbib/biblatex styles complete too.
    public static let citationCommands: Set<String> = [
        "cite", "citep", "citet", "citealt", "citealp", "citeauthor",
        "citeyear", "citeyearpar", "citepos", "nocite",
        "Citep", "Citet", "Citealt", "Citealp", "Citeauthor",
        "parencite", "Parencite", "textcite", "Textcite", "footcite",
        "autocite", "smartcite", "supercite",
    ]

    /// Commands whose `{…}` argument holds a \label key.
    public static let referenceCommands: Set<String> = [
        "ref", "pageref", "autoref", "eqref", "nameref",
        "cref", "Cref", "vref", "Vref", "cpageref", "labelcref",
    ]

    /// Detects the completion context at `caretUTF16Offset` in `source`.
    /// Returns nil in plain text, inside unrecognised `\cmd{…}` groups, and
    /// after an escaped `\\`. Only looks backwards from the caret, so an
    /// unclosed `\cite{` group still completes.
    public static func context(in source: String, caretUTF16Offset: Int) -> CompletionContext? {
        let units = source as NSString
        let caret = min(max(caretUTF16Offset, 0), units.length)

        // ── group contexts: `\cite{a,pre|` / `\ref{pre|` ──
        // The current key fragment is a run of key characters ending at the
        // caret; the group opener is the nearest `{` reachable over earlier
        // key characters, commas and whitespace.
        var index = caret
        while index > 0, isKeyChar(units.character(at: index - 1)) { index -= 1 }
        let prefixStart = index
        var scan = index
        while scan > 0,
              isKeyChar(units.character(at: scan - 1)) || units.character(at: scan - 1) == 44 || isWhitespace(units.character(at: scan - 1)) {
            scan -= 1
        }
        if scan > 0, units.character(at: scan - 1) == 123, scan < 2 || units.character(at: scan - 2) != 92 {
            if let command = commandName(before: scan - 1, units: units) {
                let kind: CompletionContextKind? =
                    citationCommands.contains(command) ? .citation :
                    referenceCommands.contains(command) ? .reference : nil
                if let kind {
                    let prefix = units.substring(with: NSRange(location: prefixStart, length: caret - prefixStart))
                    return CompletionContext(kind: kind, prefix: prefix, prefixUTF16Offset: prefixStart)
                }
            }
        }

        // ── command context: `\pre|` ──
        index = caret
        while index > 0, isLetter(units.character(at: index - 1)) { index -= 1 }
        if index > 0, units.character(at: index - 1) == 92, !(index > 1 && units.character(at: index - 2) == 92) {
            let prefix = units.substring(with: NSRange(location: index - 1, length: caret - index + 1))
            return CompletionContext(kind: .command, prefix: prefix, prefixUTF16Offset: index - 1)
        }
        return nil
    }

    /// Command name ending just before `brace` (the `{` index), skipping
    /// trailing whitespace, `*`, and `[…]` optional args. Returns nil when
    /// the text before the brace is not a `\name` sequence.
    private static func commandName(before brace: Int, units: NSString) -> String? {
        var index = brace
        while index > 0, isWhitespace(units.character(at: index - 1)) { index -= 1 }
        while index > 0 {
            if units.character(at: index - 1) == 93 {
                // `]` — walk back over the balanced `[…]` optional arg.
                var depth = 1
                var cursor = index - 1
                while cursor > 0 {
                    cursor -= 1
                    if units.character(at: cursor) == 93 { depth += 1 } else if units.character(at: cursor) == 91 {
                        depth -= 1
                        if depth == 0 { break }
                    }
                }
                guard depth == 0 else { break }
                index = cursor
                while index > 0, isWhitespace(units.character(at: index - 1)) { index -= 1 }
                continue
            }
            if units.character(at: index - 1) == 42 {
                index -= 1
                while index > 0, isWhitespace(units.character(at: index - 1)) { index -= 1 }
                continue
            }
            break
        }
        let nameEnd = index
        while index > 0, isLetter(units.character(at: index - 1)) { index -= 1 }
        guard index < nameEnd, index > 0, units.character(at: index - 1) == 92,
              !(index > 1 && units.character(at: index - 2) == 92) else { return nil }
        return units.substring(with: NSRange(location: index, length: nameEnd - index))
    }

    /// Letters, digits and the punctuation a BibTeX/label key may contain.
    private static func isKeyChar(_ unit: UInt16) -> Bool {
        isLetter(unit)
            || (unit >= 48 && unit <= 57)
            || unit == 58 || unit == 95 || unit == 45 || unit == 46 || unit == 47
    }

    private static func isLetter(_ unit: UInt16) -> Bool {
        (unit >= 65 && unit <= 90) || (unit >= 97 && unit <= 122)
    }

    private static func isWhitespace(_ unit: UInt16) -> Bool {
        unit == 32 || unit == 9 || unit == 10 || unit == 13
    }
}

extension LanguageIndex {
    /// Project-wide candidates for a detected context: command contexts
    /// filter the built-in command list; citation/reference contexts filter
    /// the supplied project key sets (every project .bib key / \label).
    /// Deterministically ordered like `ProjectLanguageIndex.completions`.
    public static func completions(
        for context: CompletionContext,
        labels: Set<String>,
        citationKeys: Set<String>
    ) -> [LanguageCompletion] {
        switch context.kind {
        case .command:
            return commandCompletions(prefix: context.prefix)
        case .citation:
            return citationKeys
                .filter { $0.hasPrefix(context.prefix) }
                .sorted()
                .map { LanguageCompletion(text: $0, kind: .citation) }
        case .reference:
            return labels
                .filter { $0.hasPrefix(context.prefix) }
                .sorted()
                .map { LanguageCompletion(text: $0, kind: .label) }
        }
    }
}
