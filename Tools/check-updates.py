#!/usr/bin/env python3
"""Exercise the real update modules using temporary bundles and fake installers.

Run on macOS, Linux or Windows with Python and Rust (Swift also on macOS).
No installed app, Homebrew tap or system package is modified.
"""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
swift_check = r'''
import AppKit

@main struct Check {
    @MainActor static func main() async throws {
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let files = FileManager.default
        let checker = UpdateChecker()
        let app = root.appendingPathComponent("Pitex.app")
        func bundle(_ path: URL, _ version: String) throws {
            try files.createDirectory(at: path.appendingPathComponent("Contents"), withIntermediateDirectories: true)
            let plist = ["CFBundleIdentifier": "app.pitex.desktop", "CFBundleShortVersionString": version]
            try PropertyListSerialization.data(fromPropertyList: plist, format: .xml, options: 0)
                .write(to: path.appendingPathComponent("Contents/Info.plist"))
        }
        func brew(_ body: String) throws -> String {
            let script = root.appendingPathComponent("brew")
            try ("#!/bin/sh\nset -eu\ncd -- \"$(dirname -- \"$0\")\"\nprintf '%s\\n' \"$*\" >> calls\n" + body)
                .write(to: script, atomically: true, encoding: .utf8)
            try files.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
            try? files.removeItem(at: root.appendingPathComponent("calls"))
            return script.path
        }
        try bundle(app, "1.1.3")
        // Prime Foundation's cached bundle data before the on-disk update.
        _ = Bundle(url: app)?.infoDictionary
        let noop = try brew("exit 0\n")
        do {
            try await checker.brewUpgrade(noop, bundle: app, tag: "v1.1.4")
            preconditionFailure("A successful no-op must not request a restart")
        } catch { precondition(error.localizedDescription.contains("was not installed")) }
        let calls = try String(contentsOf: root.appendingPathComponent("calls"), encoding: .utf8)
        precondition(calls == "update\nupgrade --cask pitex\n")
        print("PASS stale Homebrew tap: exit 0 does not mean installed")

        for command in ["update", "upgrade"] {
            let failed = try brew("if [ \"$1\" = \"\(command)\" ]; then echo 'test failure' >&2; exit 9; fi\n")
            do {
                try await checker.brewUpgrade(failed, bundle: app, tag: "v1.1.4")
                preconditionFailure("Failed brew command must not succeed")
            } catch { precondition(error.localizedDescription.contains("test failure")) }
            if command == "update" {
                let calls = try String(contentsOf: root.appendingPathComponent("calls"), encoding: .utf8)
                precondition(calls == "update\n")
            }
        }
        print("PASS brew refresh/upgrade errors are surfaced")

        let success = try brew("if [ \"$1\" = upgrade ]; then /usr/bin/plutil -replace CFBundleShortVersionString -string 1.1.4 Pitex.app/Contents/Info.plist; fi\n")
        try await checker.brewUpgrade(success, bundle: app, tag: "v1.1.4")
        try checker.verifyInstalledBundle(app, tag: "v1.1.4")
        print("PASS installed version is read from disk after upgrade")

        let replacement = root.appendingPathComponent("New.app")
        try bundle(replacement, "1.1.5")
        try "old-only".write(to: app.appendingPathComponent("obsolete"), atomically: true, encoding: .utf8)
        try checker.replaceBundle(at: app, with: replacement)
        try checker.verifyInstalledBundle(app, tag: "v1.1.5")
        precondition(!files.fileExists(atPath: app.appendingPathComponent("obsolete").path))
        do {
            try checker.replaceBundle(at: app, with: root.appendingPathComponent("Missing.app"))
            preconditionFailure("Missing source must fail")
        } catch { try checker.verifyInstalledBundle(app, tag: "v1.1.5") }
        print("PASS bundle replacement and preservation after copy failure")
    }
}
'''

with tempfile.TemporaryDirectory(prefix="pitex updates 한글 ' ") as directory:
    root = Path(directory)
    if sys.platform == 'darwin' and '--rust-only' not in sys.argv:
        source = root / 'Check.swift'
        source.write_text(swift_check)
        executable = root / 'check'
        subprocess.run(['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6',
                        '-module-cache-path', str(root / 'swift-cache'), str(source),
                        str(repo / 'Mac/Sources/Features/UpdateSupport.swift'), '-o', str(executable)], check=True)
        subprocess.run([str(executable), str(root)], check=True, timeout=60)
    if '--swift-only' not in sys.argv:
        # Compile the unchanged Rust updater without pulling in the GTK UI.
        (root / 'Cargo.toml').write_text('''[package]
name = "pitex-update-check"
version = "0.0.0"
edition = "2021"
[dependencies]
serde_json = "1"
dirs = "5"
''')
        (root / 'src').mkdir()
        (root / 'src/lib.rs').write_text('''#![allow(dead_code)]
mod compat {
    pub fn run_in_external_terminal(_: &str, _: Option<&std::path::Path>) -> bool { false }
}
mod model {
    pub fn uuid_v4() -> String {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        format!("{}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}
#[path = ''' + json.dumps(str(repo / 'Linux/crates/pitex-shell/src/update.rs')) + ''']
mod update;
''')
        subprocess.run(['cargo', 'test', '--manifest-path', str(root / 'Cargo.toml')], check=True)
