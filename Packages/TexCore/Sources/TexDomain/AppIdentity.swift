import Foundation

/// Build-time identity for the macOS app.
///
/// Values are parsed once from an Info.plist dictionary. The type is
/// `Sendable` so it can be cached on `static let` and used across
/// concurrency domains without carrying the raw `[String: Any]` around.
public struct AppIdentity: Sendable {
    /// Display name (e.g. "Pitex" or "Pitex Nightly").
    public let appName: String
    /// Folder name inside `~/Library/Application Support` and `~/Library/Caches`.
    public let appFolderName: String
    /// Temporary file prefix (e.g. `pitex` or `pitex-nightly`).
    public let tempPrefix: String
    /// `CFBundleIdentifier` from the plist.
    public let bundleIdentifier: String
    /// GitHub release API for the current channel.
    public let releaseAPI: URL
    /// Asset suffix used to build download URLs.
    public let assetSuffix: String
    /// Homebrew cask identifier.
    public let brewCaskName: String
    /// Whether this is the nightly channel.
    public let isNightly: Bool

    public init(info: [String: Any]) {
        let displayName = (info["CFBundleDisplayName"] as? String) ?? "Pitex"
        let nightly = (info["PITEXChannel"] as? String) == "nightly"
        let bundle = (info["CFBundleIdentifier"] as? String) ?? "app.pitex.desktop"
        let apiURL = nightly
            ? "https://api.github.com/repos/jaehwan-2ee/pitex/releases/tags/nightly"
            : "https://api.github.com/repos/jaehwan-2ee/pitex/releases/latest"

        self.appName = displayName
        self.appFolderName = displayName
        self.tempPrefix = nightly ? "pitex-nightly" : "pitex"
        self.bundleIdentifier = bundle
        self.releaseAPI = URL(string: apiURL)!
        self.assetSuffix = "macos-arm64.dmg"
        self.brewCaskName = nightly ? "pitex@nightly" : "pitex"
        self.isNightly = nightly
    }

    /// Identity of the running application.
    public static let current = AppIdentity(info: Bundle.main.infoDictionary ?? [:])

    /// Stable identity for tests.
    public static let stable = AppIdentity(info: [
        "CFBundleDisplayName": "Pitex",
        "CFBundleIdentifier": "app.pitex.desktop",
    ])

    /// Nightly identity for tests.
    public static let nightly = AppIdentity(info: [
        "CFBundleDisplayName": "Pitex Nightly",
        "CFBundleIdentifier": "app.pitex.desktop.nightly",
        "PITEXChannel": "nightly",
    ])
}
