import Foundation

/// Expands build-command placeholders against explicit values.
///
/// With `quotePaths` every substitution is a single-quoted POSIX word,
/// so a source named `it's main.tex` or a `.pitex-live` path survives
/// the login shell untouched — the earlier live path substituted
/// `{file}` raw, which broke the stock tectonic preset on any spaced
/// filename. Live builds always use this mode.
///
/// With `quotePaths` off, `{file}`/`{filename}` stay verbatim (the
/// long-standing manual-template behaviour, including a user-quoted
/// `"{file}"`), while `{outdir}` is still emitted quoted — it exists
/// only to name a possibly-spaced directory.
///
/// A placeholder the user already wrapped in matching quotes collapses
/// to the same quoted form rather than nesting — but only when the
/// emitted value is itself quoted: `"{outdir}"` becomes `'a b'`,
/// never `"'a b'"`; a raw-mode `"{file}"` keeps the user's quotes.
/// Other quoting styles (mixed, backslash-escaped) are left alone.
public enum BuildCommandPlaceholders {

    /// Replace `{file}`, `{filename}` and `{outdir}` in `template`.
    ///
    /// One left-to-right pass over the ORIGINAL template: a substituted
    /// value is never rescanned, so a filename literally containing
    /// `{outdir}` or `{filename}` survives as data instead of being
    /// rewritten by a later pass.
    public static func expand(_ template: String, file: String, filename: String, outdir: String,
                              quotePaths: Bool = true) -> String {
        let values = ["{file}": file, "{filename}": filename, "{outdir}": outdir]
        var result = ""
        var index = template.startIndex
        while index < template.endIndex {
            // The nearest token occurrence — tokens are searched in the
            // original template, so substituted values are never input.
            var nearest: (token: String, range: Range<String.Index>)?
            for token in values.keys {
                if let range = template.range(of: token, range: index..<template.endIndex),
                   nearest.map({ range.lowerBound < $0.range.lowerBound }) ?? true {
                    nearest = (token, range)
                }
            }
            guard let found = nearest else {
                result += template[index...]
                break
            }
            // Raw-mode file values emit verbatim, so a user-quoted
            // `"{file}"` keeps its quotes; quoted values eat the
            // user's matching quotes instead of nesting inside them.
            let quoted = quotePaths || found.token == "{outdir}"
            var lower = found.range.lowerBound
            var upper = found.range.upperBound
            if quoted, lower > template.startIndex, upper < template.endIndex {
                let quoteIndex = template.index(before: lower)
                let quote = template[quoteIndex]
                if (quote == "'" || quote == "\""), template[upper] == quote {
                    lower = quoteIndex
                    upper = template.index(after: upper)
                }
            }
            result += template[index..<lower]
            let value = values[found.token] ?? ""
            result += quoted ? "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'" : value
            index = upper
        }
        return result
    }
}
