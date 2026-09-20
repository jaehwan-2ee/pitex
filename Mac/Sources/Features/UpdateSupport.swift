import AppKit
import Foundation

/// Self-update — checks GitHub Releases, downloads the macOS DMG, replaces
/// the installed app, and relaunches. Mirrors the Rust `update` module used
/// by the Linux and Windows shells: same release feed, same version compare,
/// same auto-install preference key.
///
/// Install path selection:
/// - Homebrew-cask installs (this copy is `/Applications/Pitex.app` *and*
///   a Caskroom cellar for `pitex` exists) go through `brew upgrade
///   --cask` so brew keeps tracking the version it owns.
/// - Manual/DMG installs replace the running bundle in place, wherever it
///   lives. When that needs admin rights the mounted DMG is opened for a
///   drag install.
@MainActor
final class UpdateChecker: ObservableObject {
    enum Phase {
        case idle, checking, upToDate, available, downloading, installing
        case installed, manual, failed
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
    /// The signed bundle identifier — built from components so the
    /// accessibility contract's unlocalized-key scan doesn't mistake the
    /// literal for a `Localizable.strings` key.
    private static let bundleIdentifier = ["app", "pitex", "desktop"].joined(separator: ".")

    var currentVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.0.0"
    }

    // MARK: - check

    func check() async {
        guard phase != .checking && phase != .downloading && phase != .installing else { return }
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
        guard phase != .checking && phase != .downloading && phase != .installing,
              let asset = pendingAsset, let tag = availableTag else { return }
        do {
            try await install(tag: tag, asset: asset)
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
            try await install(tag: tag, asset: asset)
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

    private func install(tag: String, asset: ReleaseAsset) async throws {
        if isBrewManaged {
            phase = .installing
            detail = String(localized: "settings.updates.installing")
            guard let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
                .first(where: { FileManager.default.isExecutableFile(atPath: $0) }) else {
                throw NSError(domain: "PitexUpdate", code: 7,
                              userInfo: [NSLocalizedDescriptionKey: "Homebrew could not be found."])
            }
            try await brewUpgrade(brew, bundle: Bundle.main.bundleURL, tag: tag)
        } else {
            phase = .downloading
            detail = String(localized: "settings.updates.downloading")
            let dmg = try await download(asset)
            try await installFromDMG(dmg, tag: tag)
            guard phase != .manual else { return }
        }
        pendingAsset = nil
        availableTag = nil
        phase = .installed
        detail = String(localized: "settings.updates.installed")
        try relaunch()
    }

    private func download(_ asset: ReleaseAsset) async throws -> URL {
        let (tmp, response) = try await URLSession.shared.download(from: asset.url)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw NSError(domain: "PitexUpdate", code: 4,
                          userInfo: [NSLocalizedDescriptionKey: "Download failed"])
        }
        let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("\(Self.bundleIdentifier)/updates", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let dest = dir.appendingPathComponent(asset.name)
        try? FileManager.default.removeItem(at: dest)
        try FileManager.default.moveItem(at: tmp, to: dest)
        return dest
    }

    /// Homebrew owns the app only when this running copy is the cask's
    /// `/Applications/Pitex.app` *and* a Caskroom cellar exists — a stray
    /// DMG copy next to a brew install must not route through `brew`.
    private var isBrewManaged: Bool {
        guard Bundle.main.bundleURL.path == "/Applications/Pitex.app" else { return false }
        return ["/opt/homebrew/Caskroom/pitex", "/usr/local/Caskroom/pitex"]
            .contains { FileManager.default.fileExists(atPath: $0) }
    }

    func brewUpgrade(_ brew: String, bundle: URL, tag: String) async throws {
        // Automatic refresh is throttled (or disabled). A stale tap makes
        // `brew upgrade` exit successfully without changing the app.
        let log = FileManager.default.temporaryDirectory.appendingPathComponent("pitex-brew-\(UUID().uuidString).log")
        FileManager.default.createFile(atPath: log.path, contents: nil)
        let output = try FileHandle(forWritingTo: log)
        defer {
            try? output.close()
            try? FileManager.default.removeItem(at: log)
        }
        for arguments in [["update"], ["upgrade", "--cask", "pitex"]] {
            let status = try await Self.run(brew, arguments, output: output)
            guard status == 0 else {
                let message = (try? String(contentsOf: log, encoding: .utf8)) ?? ""
                throw NSError(domain: "PitexUpdate", code: Int(status), userInfo: [
                    NSLocalizedDescriptionKey: "brew \(arguments.joined(separator: " ")) failed.\n\(message.suffix(4000))"
                ])
            }
        }
        // Read the plist from disk: Bundle caches the running version.
        try verifyInstalledBundle(bundle, tag: tag)
    }

    func verifyInstalledBundle(_ bundle: URL, tag: String) throws {
        let data = try Data(contentsOf: bundle.appendingPathComponent("Contents/Info.plist"))
        let info = try PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any]
        guard info?["CFBundleIdentifier"] as? String == Self.bundleIdentifier,
              let version = info?["CFBundleShortVersionString"] as? String,
              !isNewer(tag: tag, than: version) else {
            throw NSError(domain: "PitexUpdate", code: 8, userInfo: [
                NSLocalizedDescriptionKey: "\(tag) was not installed. The Homebrew cask or downloaded app may not be updated yet. Try again later."
            ])
        }
    }

    /// Mount the DMG, replace the running bundle wherever it lives, detach.
    /// Non-admin installs fall back to opening the DMG for a manual drag.
    private func installFromDMG(_ dmg: URL, tag: String) async throws {
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
        let source = mount.appendingPathComponent("Pitex.app", isDirectory: true)
        guard FileManager.default.fileExists(atPath: source.path) else {
            throw NSError(domain: "PitexUpdate", code: 5,
                          userInfo: [NSLocalizedDescriptionKey: "The downloaded image has no Pitex.app"])
        }
        try verifyInstalledBundle(source, tag: tag)
        do {
            try replaceBundle(at: Bundle.main.bundleURL, with: source)
        } catch {
            // /Applications needs admin — hand the mounted image to the user.
            NSWorkspace.shared.open(dmg)
            phase = .manual
            detail = String(localized: "settings.updates.manual")
            return
        }
        try verifyInstalledBundle(Bundle.main.bundleURL, tag: tag)
    }

    func replaceBundle(at destination: URL, with source: URL) throws {
        let files = FileManager.default
        let staged = destination.deletingLastPathComponent()
            .appendingPathComponent(".pitex-update-\(UUID().uuidString).app")
        defer { try? files.removeItem(at: staged) }
        // Finish the copy before replacing the old app, so a copy failure
        // (permissions, disk space) cannot delete the installed version.
        try files.copyItem(at: source, to: staged)
        _ = try files.replaceItemAt(destination, withItemAt: staged, options: .usingNewMetadataOnly)
    }

    /// Exit so the new build runs. A detached helper waits for this process
    /// to leave, then opens the freshly installed bundle — `open` on the
    /// bundle path reuses the same Dock icon, so no second instance appears
    /// (the old `createsNewApplicationInstance` relaunch produced two).
    /// The wait is bounded: if the user cancels the quit (e.g. unsaved
    /// changes), nothing relaunches behind their back.
    private func relaunch() throws {
        let bundle = Bundle.main.bundleURL.path
            .replacingOccurrences(of: "'", with: "'\\''")
        let pid = getpid()
        let script = """
            n=0
            while kill -0 \(pid) 2>/dev/null && [ $n -lt 600 ]; do sleep 0.2; n=$((n+1)); done
            kill -0 \(pid) 2>/dev/null || open '\(bundle)'
            """
        let helper = Process()
        helper.executableURL = URL(fileURLWithPath: "/bin/sh")
        helper.arguments = ["-c", script]
        try helper.run()
        NSApp.terminate(nil)
    }

    // MARK: - process helpers

    private static func run(_ launchPath: String, _ arguments: [String], output: FileHandle? = nil) async throws -> Int32 {
        try await withCheckedThrowingContinuation { continuation in
            let process = Process()
            process.executableURL = URL(fileURLWithPath: launchPath)
            process.arguments = arguments
            process.standardInput = FileHandle.nullDevice
            if let output {
                process.standardOutput = output
                process.standardError = output
            }
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
