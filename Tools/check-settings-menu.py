#!/usr/bin/env python3
"""Real App-menu Settings…/⌘, checks against the real PitexApp lifecycle.

Builds a checker .app from the actual Mac sources — production @main
intact, a test-only applicationDidFinishLaunching hook starts the driver
after launch — then exercises the single application-menu Settings item:
menu discovery (one ⌘, item, in the app menu, enabled), a real menu
click and a real key event (virtual key 43) through NSMenu, the visible
attached sheet on an empty and a loaded window, global dedup across
workspaces, and routing while a minimized window's detached preview is
key. Asserts sheet visibility/attachment and window fronting — no forced
isEnabled, no mocked dispatch.

Usage: check-settings-menu.py <Build/Products/Release>
"""
from pathlib import Path
import os
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()

check = r'''
import AppKit
import SwiftUI

/// Set by the injected applicationDidFinishLaunching hook — plain
/// storage so the delegate can write it from any context.
enum LaunchFlag {
    nonisolated(unsafe) static var didFinishLaunching = false
    nonisolated(unsafe) static var finishUserInfo = "nil"
}

@MainActor enum Driver {
    // Environment only — naked argv tokens are parsed by Cocoa as
    // documents to open.
    static let root = ProcessInfo.processInfo.environment["PITEX_CHECK_FIXTURE"] ?? ""

    static func run() async {
        func require(_ condition: Bool, _ message: String) {
            guard condition else { print("FAIL", message); fflush(nil); exit(1) }
        }
        func stage(_ message: String) { print("[stage]", message); fflush(nil) }
        func owning(_ path: String) -> WorkspaceModel? {
            let url = URL(fileURLWithPath: path).standardizedFileURL
            return WorkspaceWindows.live.first { $0.owns(url) }
        }
        func ready(_ workspace: WorkspaceModel) -> Bool {
            if case .ready = workspace.phase { return true }
            return false
        }
        func attachedSheet(_ workspace: WorkspaceModel) -> NSWindow? {
            workspace.window?.attachedSheet
        }

        print("[diag] ", diagnostics())

        // Bare launch: exactly one empty workspace window to start from.
        require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                "App never produced an initial window — \(diagnostics())")
        require(WorkspaceWindows.live.count == 1,
                "a bare launch must produce exactly one window — \(diagnostics())")
        let workspace = WorkspaceWindows.live[0]
        require(!workspace.hasProject, "the launch window must be empty")
        stage("one empty window")

        // Menu structure: exactly one ⌘, item in the whole tree, and it
        // lives in the first (application) menu — not File or Help.
        updateMenus()
        let commas = commaItems()
        for (path, item, _) in commas {
            print("[diag] comma '\(path)' enabled=\(item.isEnabled) action=\(String(describing: item.action))")
        }
        require(commas.count == 1,
                "expected exactly one ⌘, menu item, found \(commas.count) — \(diagnostics())")
        let (commaPath, commaItem, commaMenu) = commas[0]
        let appMenu = NSApp.mainMenu?.items.first?.submenu
        require(commaMenu === appMenu,
                "⌘, must live in the application menu, found at '\(commaPath)'")
        require(commaItem.isEnabled, "the ⌘, item must be enabled — \(diagnostics())")
        stage("one enabled ⌘, item in the application menu")

        // Empty-window menu click: the sheet attaches visibly.
        require(pressSettingsShortcut(via: "menuClick"),
                "menu click could not dispatch the enabled ⌘, item")
        require(await until { attachedSheet(workspace)?.isVisible == true },
                "menu click must show a visible attached settings sheet — \(diagnostics())")
        let firstSheet = attachedSheet(workspace)!
        stage("settings sheet attached to the empty window")

        // Repeat ⌘, as a real key event: same sheet stays, no rebuild.
        require(pressSettingsShortcut(via: "keyEvent"),
                "⌘, key event was not claimed by the menu")
        require(await until { attachedSheet(workspace) === firstSheet },
                "repeat ⌘, must keep the already-open sheet — \(diagnostics())")
        stage("repeat ⌘, kept the same sheet")

        // Dismiss via the flag (setup for the loaded-window case).
        workspace.showingSettings = false
        require(await until { attachedSheet(workspace) == nil },
                "settings sheet must dismiss")

        // Load A through the real route, then ⌘, on its window.
        WorkspaceWindows.route(URL(fileURLWithPath: "\(root)/A/main.tex"))
        require(await until { owning("\(root)/A") != nil },
                "A/main.tex never reached a workspace — \(diagnostics())")
        let workspaceA = owning("\(root)/A")!
        require(await until { ready(workspaceA) && workspaceA.activeDocumentURL?.lastPathComponent == "main.tex" },
                "A must load its requested document")
        let windowA = workspaceA.window!
        require(await until { windowA.isKeyWindow }, "A's window must be key")
        require(pressSettingsShortcut(via: "keyEvent"),
                "⌘, key event on A was not claimed")
        require(await until { attachedSheet(workspaceA)?.isVisible == true },
                "⌘, on a loaded window must show its sheet — \(diagnostics())")
        stage("⌘, opened settings over project A")
        workspaceA.showingSettings = false
        require(await until { attachedSheet(workspaceA) == nil }, "A's sheet must dismiss")

        // Second project B through the real route — two windows.
        WorkspaceWindows.route(URL(fileURLWithPath: "\(root)/B/main.tex"))
        require(await until { owning("\(root)/B") != nil && WorkspaceWindows.live.count == 2 },
                "B must open a second window — \(diagnostics())")
        let workspaceB = owning("\(root)/B")!
        require(await until { ready(workspaceB) && workspaceB.window != nil },
                "B must finish loading with a window")
        let windowB = workspaceB.window!
        require(await until { windowB.isKeyWindow }, "B's window must be key")
        stage("B opened in its own window")

        // No settings open yet: ⌘, from B shows B's own sheet and
        // leaves A's flag untouched.
        require(pressSettingsShortcut(via: "keyEvent"),
                "⌘, key event on B was not claimed")
        require(await until { attachedSheet(workspaceB)?.isVisible == true },
                "⌘, on B must show B's own sheet — \(diagnostics())")
        require(!workspaceA.showingSettings,
                "B's ⌘, must not touch A's settings flag")
        stage("⌘, opened settings over project B")
        workspaceB.showingSettings = false
        require(await until { attachedSheet(workspaceB) == nil }, "B's sheet must dismiss")

        // A's sheet already open: ⌘, from B must front/reuse A — same
        // sheet instance — never stack a second sheet or clobber A's
        // draft. The attached sheet owns key while up, so fronted means
        // A is visible with A or its sheet key.
        workspaceA.showingSettings = true
        require(await until { attachedSheet(workspaceA)?.isVisible == true },
                "setup: A's settings sheet must be attached")
        let reusedSheet = attachedSheet(workspaceA)!
        windowB.makeKeyAndOrderFront(nil)
        require(await until { NSApp.keyWindow === windowB }, "B must be key for its ⌘,")
        require(pressSettingsShortcut(via: "keyEvent"),
                "⌘, key event on B was not claimed")
        require(await until { workspaceA.showingSettings && attachedSheet(workspaceA) === reusedSheet
                              && !workspaceB.showingSettings },
                "B's ⌘, must reuse A's open sheet, not open a second one — \(diagnostics())")
        require(await until {
            windowA.isVisible && (windowA.isKeyWindow || attachedSheet(workspaceA)?.isKeyWindow == true)
        }, "reusing A's sheet must front A's window — \(diagnostics())")
        stage("B's ⌘, fronted A and kept A's sheet")

        // Detach A's preview, minimize A's window, refocus the detached
        // window: ⌘, resolves the owning workspace through the native key
        // window, restores A, and shows the sheet.
        workspaceA.showingSettings = false
        require(await until { attachedSheet(workspaceA) == nil }, "A's sheet must dismiss")
        workspaceA.detachPreview()
        require(await until { workspaceA.detachedPreviewWindow != nil },
                "detaching A's preview must create its window")
        let detached = workspaceA.detachedPreviewWindow!
        windowA.miniaturize(nil)
        require(await until { windowA.isMiniaturized }, "A must miniaturize")
        detached.makeKeyAndOrderFront(nil)
        require(await until { NSApp.keyWindow === detached },
                "the detached preview must be key for its ⌘, — \(diagnostics())")
        require(pressSettingsShortcut(via: "keyEvent"),
                "⌘, key event with the detached preview key was not claimed")
        require(await until { !windowA.isMiniaturized && windowA.isVisible },
                "⌘, from the detached preview must restore A's window — \(diagnostics())")
        require(await until { attachedSheet(workspaceA)?.isVisible == true },
                "⌘, from the detached preview must show A's sheet — \(diagnostics())")
        stage("detached-preview ⌘, restored A and showed its sheet")

        require(workspaceA.hasProject && workspaceB.hasProject,
                "both projects must stay alive")
        let sheetedWindows = NSApp.windows.filter { $0.attachedSheet != nil }
        require(sheetedWindows.count == 1,
                "exactly one settings sheet may exist app-wide — \(diagnostics())")
        print("PASS settings-menu: one ⌘, item, real dispatch, dedup and owner restore")
        print("CHECK_COMPLETE")
        fflush(nil)
        exit(0)
    }

    static func until(_ timeoutSeconds: Double = 10,
                      _ condition: @escaping @MainActor () -> Bool) async -> Bool {
        let deadline = ContinuousClock.now + .seconds(timeoutSeconds)
        while !condition() {
            if .now >= deadline { return false }
            try? await Task.sleep(for: .milliseconds(50))
        }
        return true
    }

    /// Pushes the real menu lifecycle through the whole tree so
    /// item.isEnabled and target/action are resolved before dispatch or
    /// inspection: NSMenu.update() alone only runs NSMenuValidation —
    /// menuNeedsUpdate(_:) is the earlier delegate stage where SwiftUI
    /// populates dynamic items, so fire it first.
    static func updateMenus() {
        func walk(_ menu: NSMenu) {
            menu.delegate?.menuNeedsUpdate?(menu)
            menu.update()
            for item in menu.items {
                if let submenu = item.submenu { walk(submenu) }
            }
        }
        if let menu = NSApp.mainMenu { walk(menu) }
        fflush(nil)
    }

    /// Every ⌘, item in the live menu tree with its path and owning
    /// menu — exactly one must exist, inside the application menu.
    static func commaItems() -> [(path: String, item: NSMenuItem, menu: NSMenu)] {
        let relevant: NSEvent.ModifierFlags = [.command, .shift, .option, .control]
        var found: [(String, NSMenuItem, NSMenu)] = []
        func walk(_ menu: NSMenu, _ path: String) {
            for item in menu.items {
                if let submenu = item.submenu { walk(submenu, path + "/" + item.title); continue }
                guard item.keyEquivalent == ",",
                      item.keyEquivalentModifierMask.intersection(relevant) == [.command]
                else { continue }
                found.append((path + "/" + item.title, item, menu))
            }
        }
        if let menu = NSApp.mainMenu { walk(menu, "") }
        return found
    }

    /// Real ⌘, delivery: "menuClick" fires the enabled item's own action
    /// through performActionForItem; "keyEvent" pushes a physical comma
    /// (virtual key 43) through the menu's key-equivalent path. Returns
    /// false when the item was missing/disabled or unclaimed — the
    /// caller decides the failure, nothing is forced here.
    @discardableResult
    static func pressSettingsShortcut(via modality: String) -> Bool {
        updateMenus()
        if modality == "menuClick" {
            guard let item = commaItems().first?.item, let menu = item.menu,
                  item.isEnabled else { return false }
            menu.performActionForItem(at: menu.index(of: item))
            return true
        }
        let windowNumber = NSApp.keyWindow?.windowNumber
            ?? WorkspaceWindows.live.compactMap { $0.window }.first?.windowNumber ?? 0
        guard let event = NSEvent.keyEvent(with: .keyDown, location: .zero,
            modifierFlags: [.command], timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: windowNumber, context: nil,
            characters: ",", charactersIgnoringModifiers: ",",
            isARepeat: false, keyCode: 43) else { return false }
        let claimed = NSApp.mainMenu?.performKeyEquivalent(with: event) ?? false
        print("[diag] keyEquivalent claimed=\(claimed)")
        return claimed
    }

    static func diagnostics() -> String {
        let app = NSApplication.shared
        let windows = app.windows.map {
            "'\($0.title.isEmpty ? "untitled" : $0.title)' visible=\($0.isVisible) mini=\($0.isMiniaturized) sheet=\($0.attachedSheet != nil)"
        }.joined(separator: "; ")
        return "bundle=\(Bundle.main.bundleIdentifier ?? "nil") "
            + "argv=\(CommandLine.arguments) "
            + "mainThread=\(Thread.isMainThread) running=\(app.isRunning) "
            + "active=\(app.isActive) hidden=\(app.isHidden) "
            + "didFinishLaunching=\(LaunchFlag.didFinishLaunching) "
            + "userInfo=\(LaunchFlag.finishUserInfo) "
            + "live=\(WorkspaceWindows.live.count) windows=\(app.windows.count) [\(windows)]"
    }
}
'''

with tempfile.TemporaryDirectory(prefix="pitex-settings-check-") as directory:
    root = Path(directory)
    fixture = root / "fixture"
    (fixture / "A").mkdir(parents=True)
    (fixture / "A/main.tex").write_text(
        "\\documentclass{article}\n\\begin{document}\nHello A.\n\\end{document}\n")
    (fixture / "B").mkdir()
    (fixture / "B/main.tex").write_text(
        "\\documentclass{article}\n\\begin{document}\nHello B.\n\\end{document}\n")

    # Test-only copy of PitexApp.swift keeping the production @main: an
    # injected finish hook records the launch userInfo and starts the
    # driver once the app has launched. No other source changes.
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    patched = app_main.read_text().replace(
        "    func applicationDidFinishLaunching(_ notification: Notification) {",
        "    func applicationDidFinishLaunching(_ notification: Notification) {\n"
        "        print(\"TRACE didFinishLaunching userInfo=\\(String(describing: notification.userInfo))\")\n"
        "        fflush(nil)\n"
        "        LaunchFlag.didFinishLaunching = true\n"
        "        LaunchFlag.finishUserInfo = String(describing: notification.userInfo)\n"
        "        Task { @MainActor in await Driver.run() }")
    assert "await Driver.run()" in patched
    app_copy = root / "PitexApp.swift"
    app_copy.write_text(patched)
    source = root / "Check.swift"
    source.write_text(check)
    binary = root / "check-bin"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", "arm64-apple-macos15.0", "-I", str(products),
                    str(source), str(app_copy),
                    *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift") if p != app_main],
                    *[str(p) for p in products.glob("*.o")], "-o", str(binary)], check=True)

    # A fake app-local runtime makes PiRuntimeInstaller.ensureInstalled a
    # no-op (no package.json -> installedVersion() nil -> nothing to do).
    pi_runtime_bin = root / "pi-runtime/bin"
    pi_runtime_bin.mkdir(parents=True)
    fake_pi = pi_runtime_bin / "pi"
    fake_pi.write_text("#!/bin/sh\nexit 0\n")
    fake_pi.chmod(0o755)

    identifier = f"test.pitex.settings-menu.{os.getpid()}"
    # Bundle = production Info.plist with a unique identifier, so the
    # LaunchServices registration and defaults domain are fresh.
    app = root / "SettingsMenuCheck.app"
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    resources = contents / "Resources"
    resources.mkdir()
    plist = plistlib.loads((repo / "Mac/Config/Info.plist").read_bytes())
    plist.update({
        "CFBundleExecutable": "check",
        "CFBundleIdentifier": identifier,
        "CFBundleName": "SettingsMenuCheck",
        "CFBundleDisplayName": "SettingsMenuCheck",
        "CFBundleDevelopmentRegion": "en",
        "CFBundleShortVersionString": "0",
        "CFBundleVersion": "0",
        "LSMinimumSystemVersion": "15.0",
    })
    (contents / "Info.plist").write_bytes(plistlib.dumps(plist))
    for locale in (repo / "Mac/Resources").glob("*.lproj"):
        shutil.copytree(locale, resources / locale.name)
    shutil.copy(binary, contents / "MacOS/check")
    # The copied ad-hoc binary still carries the code identity
    # "check-bin" — sign the assembled app under its own bundle
    # identifier so LaunchServices and defaults see a distinct app.
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                    "--identifier", identifier, str(app)], check=True)
    describe = subprocess.run(["/usr/bin/codesign", "-d", "--verbose=2", str(app)],
                              check=True, capture_output=True, text=True)
    signed = re.search(r"^Identifier=(.*)$", describe.stderr, re.M)
    print(f"[diag] codesign {app.name}: {signed.group(1) if signed else 'unknown'}")
    assert signed and signed.group(1) == identifier

    # Pin the isolation keys into this bundle's fresh defaults domain —
    # a bare launch with restore off starts from one empty window.
    for key in ("pitex.pref.editor.restoreSession", "pitex.pref.update.autoInstall"):
        subprocess.run(["/usr/bin/defaults", "write", identifier, key,
                        "-bool", "false"], check=True)

    out_log, err_log = root / "check.out.log", root / "check.err.log"
    # No positional arguments and no --args: Cocoa parses naked argv
    # tokens as documents to open. All config travels through --env.
    command = ["/usr/bin/open", "-W",
               "--stdout", str(out_log), "--stderr", str(err_log),
               "--env", "PI_AGENT_PATH=/usr/bin/false",
               "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
               "--env", f"PITEX_CHECK_FIXTURE={fixture}",
               str(app)]
    try:
        subprocess.run(command, check=True, timeout=180)
    except subprocess.SubprocessError as error:
        print(f"FAIL: launcher raised {error}")
    finally:
        subprocess.run(["/usr/bin/pkill", "-f", str(app)], check=False)
        for log in (out_log, err_log):
            if log.exists():
                print(f"--- {log.name} ---")
                print(log.read_text(errors="replace"), end="")
    output = out_log.read_text(errors="replace") if out_log.exists() else ""
    # `open -W` swallows the exit status — CHECK_COMPLETE is decisive.
    if "CHECK_COMPLETE" not in output:
        sys.exit("FAIL: no CHECK_COMPLETE")
