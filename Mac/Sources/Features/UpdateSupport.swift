import AppKit
import Foundation

/// Self-update — checks GitHub Releases, downloads the macOS DMG, replaces
/// the installed app, and relaunches. Mirrors the Rust `update` module used
/// by the Linux and Windows shells: same release feed, same version compare,
/// same auto-install preference key.
///
/// Install path selection:
/// - Homebrew-cask installs (`/opt/homebrew/Caskroom/pitex` or the Intel
///   prefix) go through `brew upgrade --cask` so brew keeps tracking the
///   version it owns.
/// - Manual/DMG installs replace `/Applications/Pitex.app` directly. When
///   that needs admin rights the mounted DMG is opened for a drag install.
@MainActor
final class UpdateChecker: ObservableObject {
    enum Phase {
        case idle, checking, upToDate, available, downloading, installing
        case installed, handedToBrew, manual, failed
    }

    @Published private(set) var phase: Phase = .idle
    @Published private(set) var detail = ""
    @Published private(set) var availableTag: String?
    private var pendingAsset: ReleaseAsset?

    struct ReleaseAsset {
        let name: String
        let url: URL
    }

    private static let releaseAPI =
        URL(string: "https://api.github.com/repos/jaehwan-2ee/pitex/releases/latest")!
    private static let assetSuffix = "macos-arm64.dmg"

    var currentVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.0.0"
    }

    // MARK: - check

    func check() async {
        phase = .checking
        detail = ""
        do {
            let (tag, asset) = try await latestRelease()
            if isNewer(tag: tag, than: currentVersion) {
                availableTag = tag
                pendingAsset = asset
                phase = .available
                detail = String(format: String(localized: "settings.updates.available"), tag)
            } else {
                availableTag = nil
                pendingAsset = nil
                phase = .upToDate
                detail = String(localized: "settings.updates.up_to_date")
            }
        } catch {
            phase = .failed
            detail = String(format: String(localized: "settings.updates.failed"), error.localizedDescription)
        }
    }

    // MARK: - install

    func downloadAndInstall() async {
        guard let asset = pendingAsset else { return }
        do {
            phase = .downloading
            detail = String(localized: "settings.updates.downloading")
            let dmg = try await download(asset)
            if isBrewManaged {
                phase = .installing
                if await brewUpgrade() {
                    phase = .handedToBrew
                    detail = String(localized: "settings.updates.installed")
                    relaunch()
                } else {
                    try await installFromDMG(dmg)
                }
            } else {
                try await installFromDMG(dmg)
            }
        } catch {
            phase = .failed
            detail = String(format: String(localized: "settings.updates.failed"), error.localizedDescription)
        }
    }

    /// The launch-time auto pass used by `PitexAppDelegate` when
    /// `pitex.pref.update.autoInstall` is on — silent unless something needs
    /// the user (manual drag install, failure is left quiet vs nagging).
    func autoUpdate() async {
        do {
            let (tag, asset) = try await latestRelease()
            guard isNewer(tag: tag, than: currentVersion) else { return }
            let dmg = try await download(asset)
            if isBrewManaged, await brewUpgrade() {
                relaunch()
            } else {
                try await installFromDMG(dmg)
            }
        } catch {
            // Auto-update failures stay silent; the manual check in Settings
            // surfaces the same error on demand.
        }
    }

    // MARK: - release lookup

    private func latestRelease() async throws -> (tag: String, asset: ReleaseAsset) {
        var request = URLRequest(url: Self.releaseAPI, timeoutInterval: 20)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.setValue("pitex-update", forHTTPHeaderField: "User-Agent")
        let (data, response) = try await URLSession.shared.data(for: request)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw NSError(domain: "PitexUpdate", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Release lookup failed"])
        }
        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let tag = json["tag_name"] as? String,
              let assets = json["assets"] as? [[String: Any]]
        else {
            throw NSError(domain: "PitexUpdate", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "Bad release response"])
        }
        for asset in assets {
            if let name = asset["name"] as? String, name.hasSuffix(Self.assetSuffix),
               let raw = asset["browser_download_url"] as? String, let url = URL(string: raw) {
                return (tag, ReleaseAsset(name: name, url: url))
            }
        }
        throw NSError(domain: "PitexUpdate", code: 3,
                      userInfo: [NSLocalizedDescriptionKey: "No macOS asset in the latest release"])
    }

    /// `v1.0.3` vs `1.0.2` — numeric component compare, same rule as the
    /// Rust `version_newer`.
    func isNewer(tag: String, than current: String) -> Bool {
        func parts(_ s: String) -> [Int] {
            s.trimmingCharacters(in: CharacterSet(charactersIn: "vV"))
                .split(separator: ".")
                .map { Int($0.prefix(while: { $0.isNumber })) ?? 0 }
        }
        let (new, old) = (parts(tag), parts(current))
        for i in 0..<max(new.count, old.count) {
            let n = i < new.count ? new[i] : 0
            let o = i < old.count ? old[i] : 0
            if n != o { return n > o }
        }
        return false
    }

    // MARK: - download / install

    private func download(_ asset: ReleaseAsset) async throws -> URL {
        let (tmp, response) = try await URLSession.shared.download(from: asset.url)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw NSError(domain: "PitexUpdate", code: 4,
                          userInfo: [NSLocalizedDescriptionKey: "Download failed"])
        }
        let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("dev.pitex.app/updates", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let dest = dir.appendingPathComponent(asset.name)
        try? FileManager.default.removeItem(at: dest)
        try FileManager.default.moveItem(at: tmp, to: dest)
        return dest
    }

    /// Homebrew owns the app when a Caskroom cellar for `pitex` exists —
    /// `brew upgrade` then does the right thing (and re-tracks the version).
    private var isBrewManaged: Bool {
        ["/opt/homebrew/Caskroom/pitex", "/usr/local/Caskroom/pitex"]
            .contains { FileManager.default.fileExists(atPath: $0) }
    }

    private func brewUpgrade() async -> Bool {
        guard let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
            .first(where: { FileManager.default.isExecutableFile(atPath: $0) })
        else { return false }
        do {
            let status = try await Self.run(brew, ["upgrade", "--cask", "pitex"])
            return status == 0
        } catch {
            return false
        }
    }

    /// Mount the DMG, replace `/Applications/Pitex.app`, detach. Non-admin
    /// installs fall back to opening the DMG for a manual drag.
    private func installFromDMG(_ dmg: URL) async throws {
        phase = .installing
        detail = String(localized: "settings.updates.installing")
        let mount = URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
            .appendingPathComponent("pitex-update-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: mount, withIntermediateDirectories: true)
        defer { _ = try? Self.runSync("/usr/bin/hdiutil", ["detach", "-quiet", mount.path]) }
        let attach = try await Self.run(
            "/usr/bin/hdiutil",
            ["attach", "-nobrowse", "-readonly", "-mountpoint", mount.path, dmg.path])
        guard attach == 0 else {
            throw NSError(domain: "PitexUpdate", code: 6,
                          userInfo: [NSLocalizedDescriptionKey: "Could not mount the downloaded image"])
        }
        let source = mount.appendingPathComponent("Pitex.app", isDirectory: true).path
        guard FileManager.default.fileExists(atPath: source) else {
            throw NSError(domain: "PitexUpdate", code: 5,
                          userInfo: [NSLocalizedDescriptionKey: "The downloaded image has no Pitex.app"])
        }
        let dest = "/Applications/Pitex.app"
        do {
            try? FileManager.default.removeItem(atPath: dest)
            try FileManager.default.copyItem(atPath: source, toPath: dest)
        } catch {
            // /Applications needs admin — hand the mounted image to the user.
            NSWorkspace.shared.open(dmg)
            phase = .manual
            detail = String(localized: "settings.updates.manual")
            return
        }
        phase = .installed
        detail = String(localized: "settings.updates.installed")
        relaunch()
    }

    /// Start the freshly installed copy and leave — the same "exit so the
    /// new build runs" handoff the Windows updater script performs.
    private func relaunch() {
        let config = NSWorkspace.OpenConfiguration()
        config.createsNewApplicationInstance = true
        NSWorkspace.shared.openApplication(
            at: URL(fileURLWithPath: "/Applications/Pitex.app"),
            configuration: config
        ) { _, _ in
            Task { @MainActor in NSApp.terminate(nil) }
        }
    }

    // MARK: - process helpers

    private static func run(_ launchPath: String, _ arguments: [String]) async throws -> Int32 {
        try await withCheckedThrowingContinuation { continuation in
            let process = Process()
            process.executableURL = URL(fileURLWithPath: launchPath)
            process.arguments = arguments
            process.terminationHandler = { p in
                continuation.resume(returning: p.terminationStatus)
            }
            do {
                try process.run()
            } catch {
                continuation.resume(throwing: error)
            }
        }
    }

    private static func runSync(_ launchPath: String, _ arguments: [String]) throws -> Int32 {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: launchPath)
        process.arguments = arguments
        try process.run()
        process.waitUntilExit()
        return process.terminationStatus
    }
}
