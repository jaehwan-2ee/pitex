import Foundation

public struct LineCommentEdit: Equatable {
    public var range: NSRange
    public var replacement: String
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

    static func clampedCharacterRange(start: Int, end: Int, limit: Int) -> NSRange? {
        guard limit >= 0 else { return nil }
        let lower = min(max(start, 0), limit)
        let upper = min(max(end, 0), limit)
        guard upper > lower else { return nil }
        return NSRange(location: lower, length: upper - lower)
    }
}
