#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// Canonical (realpath) spelling for every path the `pitex-preview`
/// helper is told about: `--root`, `files`, `closed`, and the session
/// directory when it is compared against paths the helper reports. The
/// driver realpaths the project root and prefix-matches override paths,
/// so both sides must spell one POSIX identity — a project opened as
/// `/tmp/x` while the real directory is `/private/tmp/x` otherwise drops
/// every unsaved override.
///
/// Paths go through `realpath(3)` only: Darwin Foundation path methods
/// (`resolvingSymlinksInPath`, `standardizedFileURL`, NSString
/// standardization) can strip the `/private` prefix realpath keeps, so
/// comparing against a Foundation-resolved string does not prove POSIX
/// identity. An override path may name a file that is not on disk (new
/// buffer, or deleted while open): the deepest EXISTING ancestor is
/// realpath'd and the missing tail — which cannot contain a link — is
/// re-appended verbatim with plain string joins.
public enum EmbeddedPreviewPaths {
    /// The canonical spelling of `path`. Lexically normalizes `.`/`..`
    /// first (the kernel answers ENOENT for `a/..` when `a` is missing),
    /// then realpaths the deepest existing prefix.
    public static func canonical(_ path: String) -> String {
        let normalized = normalize(path)
        var tail: [String] = []
        var probe = normalized
        while true {
            if let resolved = realpathString(probe) {
                return append(resolved, tail)
            }
            if probe == "/" { return normalized } // unreachable sanity
            guard let slash = probe.lastIndex(of: "/") else { return normalized }
            if slash == probe.startIndex {
                // probe == "/x": the component is everything after "/".
                tail.insert(String(probe.dropFirst()), at: 0)
                probe = "/"
            } else {
                tail.insert(String(probe[probe.index(after: slash)...]), at: 0)
                probe = String(probe[..<slash])
            }
        }
    }

    /// `realpath(3)` — nil buffer so the result is allocated at any length.
    private static func realpathString(_ path: String) -> String? {
        guard let resolved = path.withCString({ realpath($0, nil) }) else { return nil }
        defer { free(resolved) }
        return String(cString: resolved)
    }

    /// Joins a missing tail onto a canonical base without re-standardizing.
    private static func append(_ base: String, _ tail: [String]) -> String {
        if tail.isEmpty { return base }
        return (base == "/" ? "" : base) + tail.map { "/" + $0 }.joined()
    }

    /// Lexical `.`/`..`/empty-component normalization — what `realpath(3)`
    /// needs before it can see the path at all.
    private static func normalize(_ path: String) -> String {
        let absolute = path.hasPrefix("/")
        var parts: [Substring] = []
        for part in path.split(separator: "/", omittingEmptySubsequences: true) {
            switch part {
            case ".":
                continue
            case "..":
                if !parts.isEmpty, parts.last != ".." { parts.removeLast() }
                else if !absolute { parts.append(part) }
            default:
                parts.append(part)
            }
        }
        let joined = parts.joined(separator: "/")
        if absolute { return "/" + joined } // "/" when every part resolved away
        return joined.isEmpty ? path : joined
    }
}
