import AppKit
import Foundation

/// Filesystem locations for the embedded Pitex Agent. Installing Pitex
/// provides a private pi home under Application Support — independent of any
/// global `~/.pi` setup, so the user's own pi configuration never leaks in or
/// gets touched. `PI_CODING_AGENT_DIR` still wins for debugging.
enum PiPaths {
    /// The app-local agent home (`auth.json`, `settings.json`, sessions and
    /// skills live here). The spawned agent gets this via `PI_CODING_AGENT_DIR`.
    static var agentDirectory: URL {
        if let override = ProcessInfo.processInfo.environment["PI_CODING_AGENT_DIR"],
           !override.isEmpty {
            return URL(fileURLWithPath: (override as NSString).expandingTildeInPath)
        }
        return FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Pitex", isDirectory: true)
            .appendingPathComponent("pi", isDirectory: true)
    }

    static var authFileURL: URL {
        agentDirectory.appendingPathComponent("auth.json")
    }

    /// pi's user configuration file — opened by Settings → AI.
    static var settingsFileURL: URL {
        agentDirectory
            .appendingPathComponent("settings")
            .appendingPathExtension("json")
    }

    static var modelsFileURL: URL {
        agentDirectory.appendingPathComponent("models.json")
    }

    /// Skills pi discovers automatically (`<agent dir>/skills/<name>/SKILL.md`).
    static var skillsDirectory: URL {
        agentDirectory.appendingPathComponent("skills", isDirectory: true)
    }

    /// The skills shipped inside the app bundle and mirrored into
    /// `skillsDirectory` on every install/launch.
    static var bundledSkillsDirectory: URL? {
        Bundle.main.resourceURL?.appendingPathComponent("PitexAgent/skills", isDirectory: true)
    }

    /// Where the app-local Pitex Agent runtime lives (Bun or npm local install).
    static var runtimeDirectory: URL {
        agentDirectory.deletingLastPathComponent()
            .appendingPathComponent("pi-runtime", isDirectory: true)
    }

    /// The app-local `pi` executable after a runtime install.
    static var runtimeExecutable: URL {
        runtimeDirectory.appendingPathComponent("bin/pi")
    }

    /// Existing configuration is never overwritten when opening the editor.
    static func configurationFile(customProvider: Bool = false) throws -> URL {
        let fileManager = FileManager.default
        let url = customProvider ? modelsFileURL : settingsFileURL
        try fileManager.createDirectory(at: agentDirectory, withIntermediateDirectories: true)
        if !fileManager.fileExists(atPath: url.path) {
            let initial = customProvider ? "{\n  \"providers\": {}\n}\n" : "{\n}\n"
            try initial.write(to: url, atomically: true, encoding: .utf8)
        }
        return url
    }

    static func openConfiguration(customProvider: Bool = false) throws {
        let url = try configurationFile(customProvider: customProvider)
        guard NSWorkspace.shared.open(url) else {
            throw CocoaError(.fileReadUnknown, userInfo: [NSFilePathErrorKey: url.path])
        }
    }
}

/// Minimal read access to the app-local pi credential file (`auth.json`).
/// Entries are written by pi itself (whichever providers the user configures
/// — the app does not manage them); this store only reports whether any
/// provider credential exists for the assistant pane's empty-state hint.
enum PiAuthStore {
    /// True when at least one provider credential exists in the app-local
    /// `auth.json`. Token material never leaves the file.
    static var hasAnyCredential: Bool {
        guard let data = try? Data(contentsOf: PiPaths.authFileURL),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return false }
        return !object.isEmpty
    }
}
