import Foundation

/// Where a Markdown preview link goes. The web view's navigation policy
/// evaluates this once per click; in-page navigation past the initial load
/// is never allowed.
public enum PreviewLinkAction: Equatable, Sendable {
    /// http/https/mailto — handed to the OS default handler.
    case openExternal(URL)
    /// An existing, non-executable local file — opened with its default app.
    case openFile(URL)
    /// Executable or refused extension — the UI shows the blocked toast.
    case refuse
    /// Missing file or unsupported scheme (`javascript:`…) — do nothing.
    case ignore
}

public enum MarkdownLinkPolicy {
    /// Extensions a Markdown file must never be able to launch. `.app`
    /// covers bundles (directories), so it is checked before the exec bit.
    static let refusedExtensions: Set<String> = [
        "app", "command", "tool", "terminal", "workflow", "desktop",
        "appimage", "jar", "scpt", "applescript",
        "sh", "bash", "zsh", "fish", "csh", "ksh",
    ]

    public static func action(for url: URL) -> PreviewLinkAction {
        switch url.scheme?.lowercased() {
        case "http", "https", "mailto":
            return .openExternal(url)
        case "file":
            // Percent-decoded path only — query and fragment drop away.
            // Symlinks resolve first: `paper.pdf -> Evil.app` must be judged
            // by its target, not the link's own extension.
            let path = url.path(percentEncoded: false)
            let fileURL = URL(fileURLWithPath: path).resolvingSymlinksInPath()
            var isDirectory: ObjCBool = false
            guard FileManager.default.fileExists(atPath: fileURL.path, isDirectory: &isDirectory)
            else { return .ignore }
            if refusedExtensions.contains(fileURL.pathExtension.lowercased()) { return .refuse }
            // Package bundles without a refused extension (.bundle, .plugin…)
            // still launch code — refuse any directory that is a package.
            if isDirectory.boolValue,
               (try? fileURL.resourceValues(forKeys: [.isPackageKey]))?.isPackage == true {
                return .refuse
            }
            if !isDirectory.boolValue, FileManager.default.isExecutableFile(atPath: fileURL.path) {
                return .refuse
            }
            return .openFile(fileURL)
        default:
            return .ignore
        }
    }

    /// `pitex.pref.markdown.theme` resolver — `system` (and any unknown or
    /// missing value) follows the app's effective appearance; `light` and
    /// `dark` pin the page.
    public static func previewIsDark(theme: String, appIsDark: Bool) -> Bool {
        switch theme {
        case "light": false
        case "dark": true
        default: appIsDark
        }
    }
}
