import Foundation

/// Maps SyncTeX-recorded input paths back onto the local project.
///
/// The .synctex Inputs are the paths the compiler opened — absolute on
/// whatever device ran the build (a remote project's paths live under
/// that device's root, not ours), or relative to the compiler's
/// invocation directory. Nothing anchors on the PDF's folder: a live
/// build's output hides under `.pitex-live`, so reattaching inputs to
/// the PDF directory — the previous approach — maps every chapter and
/// include into a hidden build tree that contains no sources.
///
/// Anchoring uses only known facts: the local project root, the
/// project-relative main source the build ran on, and the recorded
/// Input 1 path (the original main) whose directory structure reveals
/// the root the remote build ran under.
public enum SyncTeXSourceMapper {

    /// Derives the project root a build ran under from the recorded
    /// Input 1 (the original main source) and the known project-relative
    /// main path. `/device/project/manuscript/main.tex` recorded for a
    /// local main `manuscript/main.tex` yields `/device/project`.
    ///
    /// Returns nil when the recorded main is relative — a local build,
    /// whose inputs are already anchored here — and when the suffix does
    /// not match exactly: a same-named file elsewhere is not evidence.
    public static func recordedProjectRoot(originalMain: String, mainRelative: String) -> String? {
        // Normalize first — a recorded `/device/project/manuscript/./main.tex`
        // must derive the root exactly like its clean spelling.
        guard let original = try? NormalizedSourcePath(originalMain).value,
              let main = try? NormalizedSourcePath(mainRelative).value,
              original.hasPrefix("/"),
              !main.hasPrefix("/") else { return nil }
        let suffix = "/" + main
        guard original.hasSuffix(suffix) else { return nil }
        let root = String(original.dropLast(suffix.count))
        return root.isEmpty ? "/" : root
    }

    /// Resolves one recorded Input path to a project-relative path, or
    /// nil when it cannot be anchored inside the project.
    ///
    /// - An absolute path under the local root keeps its canonical form.
    /// - An absolute path under `recordedRoot` (a remote build) maps its
    ///   suffix onto the local root — so a remote `../shared` include
    ///   recorded as `<remote>/shared/macros.tex` lands correctly.
    /// - A relative path resolves against the known main's directory —
    ///   the source/invocation directory the build recorded against.
    ///   It is never tried at the project root first: a `-cd`-style run
    ///   inside `manuscript/` records `sections/intro.tex` meaning
    ///   `manuscript/sections/intro.tex`, and a same-named file at the
    ///   project root must not shadow it.
    /// - Anything escaping the project is rejected; basenames alone
    ///   never match.
    public static func projectRelativePath(
        recordedPath: String,
        projectRoot: String,
        mainRelative: String,
        recordedRoot: String?
    ) -> String? {
        let recorded = recordedPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !recorded.isEmpty,
              !recorded.unicodeScalars.contains(where: { $0.value == 0 }) else { return nil }
        if recorded.hasPrefix("/") {
            guard let normalized = try? NormalizedSourcePath(recorded).value else { return nil }
            if let relative = Self.strippingRoot(normalized, root: projectRoot) { return relative }
            if let recordedRoot, let relative = Self.strippingRoot(normalized, root: recordedRoot) {
                return relative
            }
            return nil
        }
        // One anchor, derived from the known main — no root-first guess
        // and no syntactic fallback that could land on a different file.
        let anchor = Self.dirname(mainRelative)
        let joined = anchor.isEmpty ? recorded : "\(anchor)/\(recorded)"
        // Normalization resolves `..` — a path escaping the project
        // throws instead of being emitted.
        guard let resolved = try? NormalizedSourcePath(joined),
              !resolved.value.hasPrefix("/") else { return nil }
        return resolved.value
    }

    /// `path` minus the leading `root/` when it sits beneath root.
    private static func strippingRoot(_ path: String, root: String) -> String? {
        let prefix = root.hasSuffix("/") ? root : root + "/"
        guard path.hasPrefix(prefix), path.count > prefix.count else { return nil }
        return String(path.dropFirst(prefix.count))
    }

    /// Directory part of a project-relative path, or "" at the root.
    private static func dirname(_ relative: String) -> String {
        guard let slash = relative.lastIndex(of: "/") else { return "" }
        return String(relative[..<slash])
    }
}
