//! Exercise the real update modules using temporary bundles and fake installers.
//! No installed app, Homebrew tap, or system package is modified.
use pitex_native_tools::{python_json, repo_root, run_checked, TempDir};
use std::{error::Error, fs, process::Command, time::Duration};

const SWIFT_CHECK: &str = r####"
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
"####;

fn main() {
    pitex_native_tools::exit_on_error(check());
}

fn check() -> Result<(), Box<dyn Error>> {
    let repo = repo_root()?;
    let args: Vec<_> = std::env::args_os().collect();
    let directory = TempDir::new("pitex updates 한글 ' ")?;
    let root = directory.path();
    if cfg!(target_os = "macos") && !args.iter().any(|arg| arg == "--rust-only") {
        let source = root.join("Check.swift");
        fs::write(&source, SWIFT_CHECK)?;
        let update_support = root.join("UpdateSupport.swift");
        let update_src = fs::read_to_string(repo.join("Mac/Sources/Features/UpdateSupport.swift"))?
            .replace("import TexDomain\n", "");
        fs::write(&update_support, update_src)?;
        let executable = root.join("check");
        run_checked(
            Command::new("xcrun")
                .args([
                    "swiftc",
                    "-parse-as-library",
                    "-swift-version",
                    "6",
                    "-module-cache-path",
                ])
                .arg(root.join("swift-cache"))
                .arg(&source)
                .arg(&update_support)
                .arg(repo.join("Packages/TexCore/Sources/TexDomain/AppIdentity.swift"))
                .arg("-o")
                .arg(&executable),
            None,
        )?;
        run_checked(
            Command::new(&executable).arg(root),
            Some(Duration::from_secs(60)),
        )?;
    }
    if !args.iter().any(|arg| arg == "--swift-only") {
        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "pitex-update-check"
version = "0.0.0"
edition = "2021"
[dependencies]
serde_json = "1"
dirs = "5"
"#,
        )?;
        fs::create_dir(root.join("src"))?;
        let source = format!(
            "{}{}{}{}{}",
            r#"#![allow(dead_code)]
#[cfg(test)]
static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
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
#[path = "#,
            python_json(
                &repo
                    .join("Linux/crates/app-ports/src/identity.rs")
                    .to_string_lossy()
                    .into_owned()
                    .into()
            ),
            r#"]
mod identity;
#[path = "#,
            python_json(
                &repo
                    .join("Linux/crates/pitex-shell/src/update.rs")
                    .to_string_lossy()
                    .into_owned()
                    .into()
            ),
            r#"]
mod update;
"#
        );
        fs::write(root.join("src/lib.rs"), source)?;
        run_checked(
            Command::new("cargo")
                .args(["test", "--manifest-path"])
                .arg(root.join("Cargo.toml")),
            None,
        )?;
    }
    directory.close()?;
    Ok(())
}
