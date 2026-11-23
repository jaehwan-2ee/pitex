import Foundation

public struct LineCommentEdit: Equatable {
    public var range: NSRange
    public var replacement: String
}

public struct LineIndentEdit: Equatable {
    public var range: NSRange
    public var replacement: String
    public var selection: NSRange
}

enum EditorText {
    static func lineCommentEdit(text: String, selection: NSRange) -> LineCommentEdit {
        let nsText = text as NSString
        let location = min(max(selection.location, 0), nsText.length)
        let length = min(max(selection.length, 0), nsText.length - location)
        let lineRange = nsText.lineRange(for: NSRange(location: location, length: length))
        let block = nsText.substring(with: lineRange)
        let lines = block.components(separatedBy: "\n")
        let contentLines = lines.filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
        let allCommented = !contentLines.isEmpty && contentLines.allSatisfy {
            $0.trimmingCharacters(in: .whitespaces).hasPrefix("%")
        }
        let transformed = lines.map { line -> String in
            guard !line.trimmingCharacters(in: .whitespaces).isEmpty else { return line }
            if allCommented {
                guard let percentIndex = line.firstIndex(of: "%") else { return line }
                var result = line
                result.remove(at: percentIndex)
                if result.first == " " { result.removeFirst() }
                return result
            }
            return "%" + line
        }.joined(separator: "\n")
        return LineCommentEdit(range: lineRange, replacement: transformed)
    }

    /// Indents (or outdents) every line touched by `selection` by one tab.
    /// Outdent removes one leading tab, else up to four leading spaces.
    static func lineIndentEdit(text: String, selection: NSRange, outdent: Bool) -> LineIndentEdit {
        let nsText = text as NSString
        let location = min(max(selection.location, 0), nsText.length)
        let length = min(max(selection.length, 0), nsText.length - location)
        let clamped = NSRange(location: location, length: length)
        let lineRange = nsText.lineRange(for: clamped)
        let replacement = nsText.substring(with: lineRange).components(separatedBy: "\n").map { line -> String in
            guard !line.trimmingCharacters(in: .newlines).isEmpty else { return line }
            guard outdent else { return "\t" + line }
            if line.hasPrefix("\t") { return String(line.dropFirst()) }
            // shortcut: 4 is the default tab width; the tabWidth setting is not wired into the editor yet.
            return String(line.dropFirst(line.prefix(4).prefix { $0 == " " }.count))
        }.joined(separator: "\n")
        let newLength = (replacement as NSString).length
        // A multi-line selection becomes the whole shifted block so Tab can repeat.
        if nsText.rangeOfCharacter(from: .newlines, options: [], range: clamped).location != NSNotFound {
            return LineIndentEdit(range: lineRange, replacement: replacement,
                                  selection: NSRange(location: lineRange.location, length: newLength))
        }
        let delta = newLength - lineRange.length
        let start = max(lineRange.location, location + delta)
        let end = max(lineRange.location, NSMaxRange(clamped) + delta)
        return LineIndentEdit(range: lineRange, replacement: replacement,
                              selection: NSRange(location: start, length: end - start))
    }

    static func clampedCharacterRange(start: Int, end: Int, limit: Int) -> NSRange? {
        guard limit >= 0 else { return nil }
        let lower = min(max(start, 0), limit)
        let upper = min(max(end, 0), limit)
        guard upper > lower else { return nil }
        return NSRange(location: lower, length: upper - lower)
    }
}
