import Foundation

/// Sources whose edits can move the compiled output. A Markdown, text,
/// JSON or figure change in the same project never starts a build on its own.
public enum BuildInputs {
    public static let extensions: Set<String> = [
        "tex", "ltx", "dtx", "sty", "cls", "clo", "def", "bib", "bst",
    ]

    public static func contains(_ path: String) -> Bool {
        extensions.contains((path as NSString).pathExtension.lowercased())
    }

    /// An explicit Save builds only when it commits a build input: the
    /// active document, or any other dirty session it writes with it.
    public static func saveStartsBuild(activePath: String?, dirtyPaths: [String]) -> Bool {
        (dirtyPaths + [activePath].compactMap { $0 }).contains(where: contains)
    }
}
