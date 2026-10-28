/// Folder-walk bounds for macOS, Linux, and Windows.
///
/// This file is the only copy of these numbers. macOS reads the static
/// lets. Linux and Windows compile the same literals into
/// `pitex_shell::model` (`MAX_SCAN_ENTRIES`, `MAX_SCAN_DIRECTORIES`,
/// `MAX_LISTED_SOURCES`, `MAX_LISTED_FIGURES`).
///
/// 50,000 names and 2,000 directories stop a Downloads-sized tree before
/// the sidebar builds a row per file. 2,000 sources and 2,000 figures
/// still cover a large book: hundreds of chapters and figures, with aux
/// files counting toward the name cap but not either list. A bigger tree
/// is marked incomplete instead of walked without a limit.
///
/// `folder_scan_limit` reads each `static let <name> = <int>` line. Keep
/// one space on each side of `=` and write the value with digits and
/// underscores only.
enum FolderScanLimits {
    static let maxScanEntries = 50_000
    static let maxScanDirectories = 2_000
    static let maxListedSources = 2_000
    static let maxListedFigures = 2_000
}
