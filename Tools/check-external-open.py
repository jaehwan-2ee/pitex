#!/usr/bin/env python3
"""Real LaunchServices open events against the real PitexApp lifecycle.

Builds a checker .app from the actual Mac sources — production @main
intact, a test-only applicationDidFinishLaunching hook starts the driver —
then runs LaunchServices launches with configuration only through
`--env` (naked argv is parsed by Cocoa as documents to open): warm
(running app gets `/usr/bin/open -a` requests), cold (the file rides in
with the launch), cold-restore (explicit file must beat a seeded recent),
welcome (a bare launch stays empty despite saved restore preferences),
and an actual quit/relaunch after opening two projects. It
asserts workspace count, the requested document, window visibility/key
and app activation without any Dock click or mocked activation APIs.

Usage: check-external-open.py <Build/Products/Release>
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
    // documents to open, which silently diverted the launch args before.
    static let root = ProcessInfo.processInfo.environment["PITEX_CHECK_FIXTURE"] ?? ""
    static let mode = ProcessInfo.processInfo.environment["PITEX_CHECK_MODE"] ?? "warm"

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

        print("[diag] mode=\(mode)", diagnostics())

        if mode == "welcome" || mode == "relaunch" {
            require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                    "\(mode): app must produce the Open window — \(diagnostics())")
            // Let queued launch/restoration work run before asserting.
            try? await Task.sleep(for: .milliseconds(750))
            require(WorkspaceWindows.live.count == 1,
                    "\(mode): launch must show exactly one Open window — \(diagnostics())")
            let workspace = WorkspaceWindows.live[0]
            require(!workspace.hasProject && workspace.activeDocumentURL == nil,
                    "\(mode): saved project must not reopen — \(diagnostics())")
            if case .noProject = workspace.phase {} else {
                require(false, "\(mode): startup must remain in the Open phase")
            }
            require(workspace.settings.restoreSession && workspace.settings.settings.project.restoresLastProject,
                    "\(mode): both saved restore preferences must remain true in this regression")
            require(workspace.recentDocuments.first?.path == "\(root)/B/main.tex",
                    "\(mode): saved recents must remain available")
            require(await until { workspace.window?.isVisible == true && workspace.window?.isKeyWindow == true },
                    "\(mode): Open window must be visible and key")
            require(workspace.window?.isRestorable == false,
                    "\(mode): workspace window must not opt into native restoration")
            if mode == "welcome" {
                // Selecting the retained recent remains an explicit open.
                await workspace.open(workspace.recentDocuments[0])
                require(await until { ready(workspace) && workspace.activeDocumentURL?.path == "\(root)/B/main.tex" },
                        "welcome: explicit recent selection must still open its project")
            }
            print("PASS \(mode): saved restore preferences preserve the Open startup screen")
            print("CHECK_COMPLETE")
            fflush(nil)
            NSApp.terminate(nil)
            return
        }

        if mode == "relaunch-seed" {
            require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                    "relaunch-seed: app must produce its initial window")
            await openExternal("\(root)/A/main.tex")
            require(await until { owning("\(root)/A").map(ready) == true },
                    "relaunch-seed: first project must load")
            // The second project creates a value-based WindowGroup scene,
            // whose encoded request previously reopened after quitting.
            await openExternal("\(root)/B/main.tex")
            require(await until { owning("\(root)/B").map(ready) == true },
                    "relaunch-seed: second project must load")
            require(WorkspaceWindows.live.count == 2,
                    "relaunch-seed: two distinct projects must create two windows")
            print("PASS relaunch-seed: two project windows opened before a native quit")
            print("CHECK_COMPLETE")
            fflush(nil)
            NSApp.terminate(nil)
            return
        }

        // Cold modes launch WITH A/main.tex — the ONLY delivery is the
        // launch event itself; nothing is re-sent, so a dropped event
        // cannot be masked. cold-restore also seeds a B recent: the
        // explicit file beats restore — exactly one A window, never a
        // stray B window, even with both restore preferences stored true.
        if mode.starts(with: "cold") {
            let label = mode
            let expected = "\(root)/A"
            require(await until { owning(expected) != nil },
                    "\(label): expected document never reached a workspace — \(diagnostics())")
            let workspace = owning(expected)!
            require(await until { ready(workspace) && workspace.activeDocumentURL?.lastPathComponent == "main.tex" },
                    "\(label) must load the requested document")
            require(await until { workspace.window?.isVisible == true && workspace.window?.isKeyWindow == true },
                    "\(label) window must be visible and key")
            // Last, so a racing restore window has time to appear.
            require(WorkspaceWindows.live.count == 1,
                    "\(label) must produce exactly one window — \(diagnostics())")
            stage("\(label) delivered main.tex into the only window")
            print("PASS \(label): one fronted window with the requested document")
            print("CHECK_COMPLETE")
            fflush(nil)
            exit(0)
        }

        // The first scene window must exist before firing opens — the
        // empty-window route branch needs a live workspace to attach to.
        require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                "App never produced an initial window — \(diagnostics())")
        let initialCount = WorkspaceWindows.live.count
        stage("initial workspaces=\(initialCount)")

        // A: opens into the already-empty workspace — still one window.
        await openExternal("\(root)/A/main.tex")
        require(await until { owning("\(root)/A") != nil },
                "A/main.tex never reached a workspace")
        let workspaceA = owning("\(root)/A")!
        require(WorkspaceWindows.live.count == 1,
                "Opening A must reuse the empty window, not spawn another")
        require(await until { ready(workspaceA) && workspaceA.activeDocumentURL?.lastPathComponent == "main.tex" },
                "A must open its requested document")
        require(await until { workspaceA.window?.isVisible == true && workspaceA.window?.isKeyWindow == true },
                "The A window must be visible and key after the open")
        stage("A opened in the empty workspace")

        // B: the reported case — the ONLY window is minimized in the Dock
        // AND Finder is frontmost (a real double-click foreground), so
        // Pitex must activate itself from background, no Dock click.
        let windowA = workspaceA.window!
        windowA.miniaturize(nil)
        require(await until { windowA.isMiniaturized }, "Could not minimize A for the background-open case")
        if let finder = NSWorkspace.shared.runningApplications.first(where: {
            $0.bundleIdentifier == "com.apple.finder"
        }) {
            _ = finder.activate()
        } else {
            _ = NSWorkspace.shared.launchApplication("Finder")
        }
        require(await until { !NSApp.isActive && !NSApp.isHidden },
                "Finder must become frontmost while Pitex stays unhidden")
        require(windowA.isMiniaturized, "A must still be miniaturized")
        await openExternal("\(root)/B/main.tex")
        require(await until { owning("\(root)/B") != nil },
                "B/main.tex never reached a workspace")
        require(WorkspaceWindows.live.count == 2,
                "Opening B while A is minimized must create exactly one new window")
        let workspaceB = owning("\(root)/B")!
        require(await until {
            guard let windowB = workspaceB.window else { return false }
            return windowB !== windowA && windowB.isVisible && windowB.isKeyWindow
        }, "The new B window must be fronted without a Dock click")
        require(await until { NSApp.isActive },
                "open -a from a Finder foreground must activate the app")
        require(await until { ready(workspaceB) && workspaceB.activeDocumentURL?.lastPathComponent == "main.tex" },
                "B must open its requested document")
        let windowB = workspaceB.window!
        stage("B opened a new window with A minimized and Finder frontmost")

        // B again, minimized: reuses the same window and restores it.
        windowB.miniaturize(nil)
        require(await until { windowB.isMiniaturized }, "Could not minimize B for the restore case")
        await openExternal("\(root)/B/main.tex")
        require(await until { !windowB.isMiniaturized },
                "Reopening a file of a minimized project must restore its window")
        require(WorkspaceWindows.live.count == 2,
                "Reopening inside B must not create another window")
        require(await until { windowB.isVisible && windowB.isKeyWindow },
                "The restored B window must come forward")
        stage("B reuse restored the minimized window")

        // C: a Markdown file opened while the app is HIDDEN — the new
        // window must appear and reactivate the app without a Dock click.
        NSApp.hide(nil)
        require(await until { !NSApp.isActive }, "App did not hide for the hidden-open case")
        await openExternal("\(root)/C/notes.md")
        require(await until { owning("\(root)/C") != nil },
                "C/notes.md never reached a workspace")
        require(WorkspaceWindows.live.count == 3,
                "Opening C must create a third window")
        let workspaceC = owning("\(root)/C")!
        require(await until { ready(workspaceC) && workspaceC.activeDocumentURL?.lastPathComponent == "notes.md" },
                "C must open its requested document")
        require(await until { workspaceC.window?.isVisible == true && workspaceC.window?.isKeyWindow == true },
                "The C window must be visible and key")
        require(await until { NSApp.isActive },
                "open -a to a hidden app must reactivate it — no Dock click allowed")
        stage("C opened a third window from a hidden app")

        // A again, a different file: routes back to A's (still minimized)
        // window — restores it and activates the requested document.
        await openExternal("\(root)/A/other.tex")
        require(await until { workspaceA.activeDocumentURL?.lastPathComponent == "other.tex" },
                "A/other.tex must activate inside A's existing workspace")
        require(WorkspaceWindows.live.count == 3,
                "Opening another file of A must reuse its window")
        require(await until { !windowA.isMiniaturized && windowA.isVisible && windowA.isKeyWindow },
                "A's window must restore and come forward for its file")
        stage("A/other.tex restored and reused the owning workspace")

        // ⌘W modality matrix — which dispatch path actually delivers a
        // menu command while A's detached preview (key, never main) is
        // focused. Control per row: ⇧⌘Y on the real scene window B must
        // toggle its bottom panel — proving the modality can deliver a
        // native menu action at all — before ⌘W is tried on the preview.
        // Focus visits B once first so stale SwiftUI focus cannot supply
        // the wrong workspace to the command router.
        workspaceA.detachPreview()
        require(await until { workspaceA.detachedPreviewWindow != nil },
                "Detaching A's preview must create its window")
        await openExternal("\(root)/B/main.tex")
        require(await until { windowB.isKeyWindow },
                "B must regain key before the preview is refocused")
        func focusDetached() async -> NSWindow? {
            if workspaceA.detachedPreviewWindow == nil {
                workspaceA.detachPreview()
                guard await until(20, { workspaceA.detachedPreviewWindow != nil }) else { return nil }
            }
            let window = workspaceA.detachedPreviewWindow!
            window.makeKeyAndOrderFront(nil)
            guard await until(20, { NSApp.keyWindow === window }) else { return nil }
            return window
        }
        print("[diag] key=\(NSApp.keyWindow?.title ?? "nil") main=\(NSApp.mainWindow?.title ?? "nil")")
        fflush(nil)
        var rows: [(name: String, control: Bool, close: Bool, healthy: Bool)] = []
        for modality in ["keyEquivalent", "sendEvent", "postEvent", "menuClick"] {
            // Control: ⇧⌘Y on scene window B must toggle its bottom panel.
            windowB.makeKeyAndOrderFront(nil)
            require(await until { NSApp.keyWindow === windowB },
                    "\(modality): B's scene window must be key for the control")
            let panelBefore = workspaceB.bottomPanelVisible
            await dispatch(modality, key: "y", code: 16,
                           mods: [.command, .shift], window: windowB)
            let control = workspaceB.bottomPanelVisible != panelBefore
            // Subject: ⌘W on A's detached preview must reattach it.
            let detached = await focusDetached()
            require(detached != nil,
                    "\(modality): the detached preview must be key — \(diagnostics())")
            require(NSApp.mainWindow !== detached!,
                    "\(modality): the detached preview must be key but never main")
            await dispatch(modality, key: "w", code: 13,
                           mods: [.command], window: detached!)
            let close = !workspaceA.previewDetached
            let healthy = workspaceA.hasProject && workspaceB.hasProject
                && WorkspaceWindows.live.count == 3
            rows.append((modality, control, close, healthy))
            print("[matrix] \(modality): control=\(control) close=\(close) healthy=\(healthy)")
            fflush(nil)
        }
        require(rows.allSatisfy { $0.healthy },
                "A dispatched shortcut must not close a project — \(rows)")
        require(rows.contains { $0.control },
                "No modality delivered ⇧⌘Y to the key scene window — \(diagnostics())")
        require(rows.contains { $0.close },
                "No modality delivered ⌘W to the key detached preview — \(diagnostics())")
        stage("⌘W matrix: \(rows.map { "\($0.name)=\($0.close ? "close" : "no-op")" }.joined(separator: ", "))")

        print("PASS external open: empty reuse, minimized-open new window, minimized restore, hidden third project, owner reuse, Cmd-W detach")
        print("CHECK_COMPLETE")
        fflush(nil)
        exit(0)
    }

    /// Sends a real `open -a` for `path` to THIS running bundle — the
    /// same LaunchServices request Finder Open With produces. No -n: the
    /// event must reach this already-running instance.
    static func openExternal(_ path: String) async {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        process.arguments = ["-a", Bundle.main.bundleURL.path, path]
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        do {
            try process.run()
        } catch {
            print("FAIL open -a \(path): \(error)")
            fflush(nil)
            exit(1)
        }
        while process.isRunning {
            try? await Task.sleep(for: .milliseconds(50))
        }
        if process.terminationStatus != 0 {
            let output = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            print("FAIL open -a \(path) exited \(process.terminationStatus): \(output)")
            fflush(nil)
            exit(1)
        }
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

    /// Recursively prints every menu item whose key equivalent is w, o,
    /// or y — which items claim ⌘W, ⌘O, and the ⇧⌘Y control. `label`
    /// identifies which dispatch target this dump precedes.
    static func dumpMenuShortcuts(_ label: String) {
        print("TRACE menu-dump \(label)")
        func walk(_ menu: NSMenu, _ path: String) {
            let delegate = menu.delegate.map { "\(type(of: $0))" } ?? "nil"
            print("TRACE menu '\(path)' delegate=\(delegate)")
            for item in menu.items {
                if let submenu = item.submenu { walk(submenu, path + "/" + item.title) }
                guard ["w", "o", "y"].contains(item.keyEquivalent.lowercased()) else { continue }
                let action = item.action.map { "\($0)" } ?? "nil"
                let target = item.target.map { "\(type(of: $0))" } ?? "nil"
                print("TRACE menuItem '\(path)/\(item.title)' key=\(item.keyEquivalent) "
                    + "enabled=\(item.isEnabled) hidden=\(item.isHidden) "
                    + "mods=\(item.keyEquivalentModifierMask.rawValue) "
                    + "action=\(action) target=\(target)")
                if let action = item.action {
                    let resolved = NSApp.target(forAction: action, to: item.target, from: item)
                    print("TRACE menuItem resolvedTarget=\(resolved.map { "\(type(of: $0))" } ?? "nil")")
                }
            }
        }
        if let menu = NSApp.mainMenu { walk(menu, "") }
        fflush(nil)
    }

    /// Finds a menu item by exact key equivalent + modifier bits —
    /// an uppercase alphabetic keyEquivalent implies .shift even when
    /// Cocoa stores it without the bit; never matches on localized titles.
    static func menuItem(key: String, mods: NSEvent.ModifierFlags) -> NSMenuItem? {
        let relevant: NSEvent.ModifierFlags = [.command, .shift, .option, .control]
        let wanted = mods.intersection(relevant)
        var found: NSMenuItem?
        func walk(_ menu: NSMenu) {
            for item in menu.items where found == nil {
                if let submenu = item.submenu { walk(submenu); continue }
                var actual = item.keyEquivalentModifierMask.intersection(relevant)
                if let char = item.keyEquivalent.first, char.isLetter, char.isUppercase {
                    actual.insert(.shift)
                }
                guard item.keyEquivalent.lowercased() == key.lowercased(),
                      actual == wanted
                else { continue }
                found = item
            }
        }
        if let menu = NSApp.mainMenu { walk(menu) }
        return found
    }

    /// Delivers one shortcut through the named dispatch path, then lets
    /// the run loop turn so async command routing can land. menuClick
    /// fires the item's real action via performActionForItem — only when
    /// the updated menu reports it enabled; a disabled item is a miss,
    /// not a forced pass.
    static func dispatch(_ modality: String, key: String, code: UInt16,
                         mods: NSEvent.ModifierFlags, window: NSWindow) async {
        // Settle one run-loop tick after the key-window assertion —
        // SwiftUI command availability trails makeKeyAndOrderFront.
        try? await Task.sleep(for: .milliseconds(100))
        updateMenus()
        dumpMenuShortcuts("\(modality) -> '\(window.title.isEmpty ? "untitled" : window.title)'")
        if modality == "menuClick" {
            if let item = menuItem(key: key, mods: mods), let menu = item.menu {
                menu.update()
                let index = menu.index(of: item)
                if item.isEnabled {
                    menu.performActionForItem(at: index)
                } else {
                    print("[matrix] \(modality): '\(item.title)' disabled — not dispatched")
                }
            } else {
                print("[matrix] \(modality): no item for \(mods.rawValue)+\(key)")
            }
        } else {
            let chars = mods.contains(.shift) ? key.uppercased() : key
            guard let event = NSEvent.keyEvent(with: .keyDown, location: .zero,
                modifierFlags: mods, timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil,
                characters: chars, charactersIgnoringModifiers: chars,
                isARepeat: false, keyCode: code) else { return }
            switch modality {
            case "sendEvent": NSApp.sendEvent(event)
            case "postEvent": NSApp.postEvent(event, atStart: false)
            default:
                let claimed = NSApp.mainMenu?.performKeyEquivalent(with: event) ?? false
                print("[matrix] keyEquivalent claimed=\(claimed)")
            }
        }
        try? await Task.sleep(for: .milliseconds(200))
    }

    /// State snapshot for bootstrap failures: thread, app lifecycle, the
    /// AppKit window list, and how many workspaces registered a scene.
    static func diagnostics() -> String {
        let app = NSApplication.shared
        let windows = app.windows.map {
            "'\($0.title.isEmpty ? "untitled" : $0.title)' visible=\($0.isVisible) mini=\($0.isMiniaturized)"
        }.joined(separator: "; ")
        return "bundle=\(Bundle.main.bundleIdentifier ?? "nil") "
            + "argv=\(CommandLine.arguments) "
            + "mainThread=\(Thread.isMainThread) running=\(app.isRunning) "
            + "active=\(app.isActive) hidden=\(app.isHidden) "
            + "didFinishLaunching=\(LaunchFlag.didFinishLaunching) "
            + "userInfo=\(LaunchFlag.finishUserInfo) "
            + "live=\(WorkspaceWindows.live.count) windows=\(app.windows.count) [\(windows)]"
    }

    static func until(_ timeout: Double = 20, _ condition: @MainActor () -> Bool) async -> Bool {
        let deadline = ContinuousClock.now + .seconds(timeout)
        while ContinuousClock.now < deadline {
            if condition() { return true }
            try? await Task.sleep(for: .milliseconds(100))
        }
        return condition()
    }
}
'''

with tempfile.TemporaryDirectory(prefix="pitex-open-", dir="/tmp") as directory:
    root = Path(directory)
    # Fixture: three independent folders so routing decisions differ.
    fixture = root / "fixture"
    (fixture / "A").mkdir(parents=True)
    (fixture / "A/main.tex").write_text(
        "\\documentclass{article}\n\\begin{document}\nHello A.\n\\end{document}\n")
    (fixture / "A/other.tex").write_text("Other document.\n")
    (fixture / "B").mkdir()
    (fixture / "B/main.tex").write_text(
        "\\documentclass{article}\n\\begin{document}\nHello B.\n\\end{document}\n")
    (fixture / "C").mkdir()
    (fixture / "C/notes.md").write_text("# Notes\n\nsome text\n")

    # Test-only copy of PitexApp.swift keeping the production @main:
    # trace lines inside the real delivery paths (behavior otherwise
    # unchanged) so the CI log shows which entry point each open event
    # took, plus a finish-hook that records the launch userInfo and
    # starts the driver after the app has actually launched.
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    patched = (app_main.read_text()
        .replace(
            "        NSApp.activate()\n"
            "        Task { @MainActor in\n"
            "            urls.forEach { WorkspaceWindows.route($0) }",
            "        for url in urls { print(\"TRACE delegate-open\", url.path) }\n"
            "        fflush(nil)\n"
            "        NSApp.activate()\n"
            "        Task { @MainActor in\n"
            "            urls.forEach { WorkspaceWindows.route($0) }")
        .replace(
            "            .onOpenURL { url in",
            "            .onOpenURL { url in\n"
            "                print(\"TRACE onOpenURL\", url.path, \"ws=\\(ObjectIdentifier(workspace))\")\n"
            "                fflush(nil)")
        .replace(
            "    func applicationShouldOpenUntitledFile(_ sender: NSApplication) -> Bool { false }",
            "    func applicationShouldOpenUntitledFile(_ sender: NSApplication) -> Bool {\n"
            "        print(\"TRACE untitled consult live=\\(MainActor.assumeIsolated { WorkspaceWindows.live.count })\")\n"
            "        fflush(nil)\n"
            "        return false\n"
            "    }")
        .replace(
            "        let url = url.standardizedFileURL\n"
            "        let windows = live",
            "        let url = url.standardizedFileURL\n"
            "        let windows = live\n"
            "        print(\"TRACE route\", url.lastPathComponent, \"live=\\(windows.count)\")\n"
            "        fflush(nil)")
        .replace("            owner.reveal(url)",
            "            print(\"TRACE route->owner\")\n            fflush(nil)\n            owner.reveal(url)")
        .replace("            empty.present()\n            Task { await empty.open(url) }",
            "            print(\"TRACE route->empty\")\n            fflush(nil)\n"
            "            empty.present()\n            Task { await empty.open(url) }")
        .replace("            openWindow(value: WindowOpenRequest(url: url))",
            "            print(\"TRACE route->openWindow\")\n            fflush(nil)\n"
            "            openWindow(value: WindowOpenRequest(url: url))")
        .replace("            pending.append(url)",
            "            print(\"TRACE route->pending\")\n            fflush(nil)\n            pending.append(url)")
        .replace(
            "            .onAppear {\n"
            "                WorkspaceWindows.register(workspace, openWindow: openWindow)",
            "            .onAppear {\n"
            "                print(\"TRACE onAppear ws=\\(ObjectIdentifier(workspace)) "
            "initial=\\(String(describing: initialURL)) live=\\(WorkspaceWindows.live.count)\")\n"
            "                fflush(nil)\n"
            "                WorkspaceWindows.register(workspace, openWindow: openWindow)")
        .replace(
            "            Button(\"command.close\") { workspace.performCloseCommand() }",
            "            Button(\"command.close\") { print(\"TRACE close-button-action\"); "
            "fflush(nil); workspace.performCloseCommand() }")
        .replace(
            "    func performCloseCommand() {",
            "    func performCloseCommand() {\n"
            "        print(\"TRACE performClose project=\\(projectURL?.lastPathComponent ?? \"none\") "
            "detached=\\(previewDetached) keyIsDetached=\\(detachedPreviewWindow === NSApp.keyWindow)\")\n"
            "        fflush(nil)")
        .replace(
            "        let key = NSApp.keyWindow\n"
            "        return WorkspaceWindows.workspace(for: key?.sheetParent ?? key)\n"
            "            ?? focusedWorkspace\n"
            "            ?? WorkspaceWindows.unfocused",
            "        let key = NSApp.keyWindow\n"
            "        let byWindow = WorkspaceWindows.workspace(for: key?.sheetParent ?? key)\n"
            "        print(\"TRACE cmd-resolve byWindow=\\(byWindow?.projectURL?.lastPathComponent ?? \"nil\") "
            "focused=\\(focusedWorkspace?.projectURL?.lastPathComponent ?? \"nil\") key=\\(key?.title ?? \"nil\")\")\n"
            "        fflush(nil)\n"
            "        return byWindow\n"
            "            ?? focusedWorkspace\n"
            "            ?? WorkspaceWindows.unfocused"))
    for marker in ("TRACE delegate-open", "TRACE onOpenURL", "TRACE untitled", "TRACE route",
                   "TRACE onAppear", "TRACE performClose", "TRACE cmd-resolve", "TRACE close-button"):
        assert marker in patched, f"trace injection missed: {marker}"

    # Production @main intact — the injected finish hook records the
    # launch userInfo and starts the driver once the app has launched.
    patched = patched.replace(
        "    func applicationDidFinishLaunching(_ notification: Notification) {",
        "    func applicationDidFinishLaunching(_ notification: Notification) {\n"
        "        print(\"TRACE didFinishLaunching userInfo=\\(String(describing: notification.userInfo))\")\n"
        "        fflush(nil)\n"
        "        LaunchFlag.didFinishLaunching = true\n"
        "        LaunchFlag.finishUserInfo = String(describing: notification.userInfo)\n"
        "        Task { @MainActor in await Driver.run() }")
    assert "await Driver.run()" in patched
    # Store the shared schema's old restore flag too, before any workspace
    # creates the SettingsStore singleton. This exercises a real stored true
    # value without changing either production setting or launch behavior.
    patched = patched.replace("import SwiftUI\n", "import SwiftUI\nimport SettingsFeature\n")
    patched = patched.replace(
        "struct PitexApp: App {",
        "struct PitexApp: App {\n"
        "    init() {\n"
        "        let seeded = PersistedSettings(project: ProjectPreferences(restoresLastProject: true))\n"
        "        UserDefaults.standard.set(try! JSONEncoder().encode(seeded), forKey: \"dev.pitex.settings\")\n"
        "    }\n")
    assert "ProjectPreferences(restoresLastProject: true)" in patched
    stripped = root / "PitexApp.swift"
    stripped.write_text(patched)
    source = root / "Check.swift"
    source.write_text(check)
    binary = root / "check-bin"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", "arm64-apple-macos15.0", "-I", str(products),
                    str(source), str(stripped),
                    *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift") if p != app_main],
                    *[str(p) for p in products.glob("*.o")], "-o", str(binary)], check=True)

    # A fake app-local runtime makes PiRuntimeInstaller.ensureInstalled a
    # no-op (no package.json -> installedVersion() nil -> nothing to do).
    pi_runtime_bin = root / "pi-runtime/bin"
    pi_runtime_bin.mkdir(parents=True)
    fake_pi = pi_runtime_bin / "pi"
    fake_pi.write_text("#!/bin/sh\nexit 0\n")
    fake_pi.chmod(0o755)

    def make_bundle(mode, identifier):
        # Bundle = production Info.plist with a per-mode unique identifier,
        # so each LaunchServices registration and defaults domain is fresh.
        app = root / f"ExternalOpenCheck-{mode}.app"
        contents = app / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        resources = contents / "Resources"
        resources.mkdir()
        plist = plistlib.loads((repo / "Mac/Config/Info.plist").read_bytes())
        plist.update({
            "CFBundleExecutable": "check",
            "CFBundleIdentifier": identifier,
            "CFBundleName": "ExternalOpenCheck",
            "CFBundleDisplayName": "ExternalOpenCheck",
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
        # "check-bin" — sign each assembled app under its own bundle
        # identifier so LaunchServices and defaults see a distinct app.
        subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                        "--identifier", identifier, str(app)], check=True)
        describe = subprocess.run(["/usr/bin/codesign", "-d", "--verbose=2", str(app)],
                                  check=True, capture_output=True, text=True)
        signed = re.search(r"^Identifier=(.*)$", describe.stderr, re.M)
        print(f"[diag] codesign {app.name}: {signed.group(1) if signed else 'unknown'}")
        assert signed and signed.group(1) == identifier
        return app

    def run_mode(mode, app=None, identifier=None):
        identifier = identifier or f"test.pitex.external-open.{os.getpid()}.{mode}"
        app = app or make_bundle(mode, identifier)
        # No CheckMain — pin the isolation keys into this bundle's fresh
        # defaults domain before launch. Stored restore=true is deliberately
        # retained; it must have no effect on plain launches or OS opens.
        restore = "false" if mode == "cold" else "true"
        subprocess.run(["/usr/bin/defaults", "write", identifier,
                        "pitex.pref.editor.restoreSession", "-bool", restore], check=True)
        subprocess.run(["/usr/bin/defaults", "write", identifier,
                        "pitex.pref.update.autoInstall", "-bool", "false"], check=True)
        # Request native macOS restoration too. Scene/window opt-outs must
        # prevent saved WindowOpenRequest values from opening old projects.
        subprocess.run(["/usr/bin/defaults", "write", identifier,
                        "NSQuitAlwaysKeepsWindows", "-bool", "true"], check=True)
        if mode in ("cold-restore", "welcome"):
            subprocess.run(["/usr/bin/defaults", "write", identifier,
                            "pitex.pref.workspace.recentDocuments",
                            "-array", str(fixture / "B/main.tex")], check=True)
        out_log, err_log = root / f"check-{mode}.out.log", root / f"check-{mode}.err.log"
        # No positional arguments and no --args: Cocoa parses naked argv
        # tokens as documents to open. All config travels through --env.
        # Cold modes hand the file to LaunchServices with the launch — a
        # real double-click on a stopped app.
        command = ["/usr/bin/open", "-W",
                   "--stdout", str(out_log), "--stderr", str(err_log),
                   "--env", "PI_AGENT_PATH=/usr/bin/false",
                   "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
                   "--env", f"PITEX_CHECK_FIXTURE={fixture}",
                   "--env", f"PITEX_CHECK_MODE={mode}"]
        if mode.startswith("cold"):
            command += ["-a", str(app), str(fixture / "A/main.tex")]
        else:
            command += [str(app)]
        try:
            subprocess.run(command, check=True, timeout=180)
        except subprocess.SubprocessError as error:
            print(f"FAIL: launcher for {mode} raised {error}")
        finally:
            subprocess.run(["/usr/bin/pkill", "-f", str(app)], check=False)
            for log in (out_log, err_log):
                if log.exists():
                    print(f"--- {mode}: {log.name} ---")
                    print(log.read_text(errors="replace"), end="")
        output = out_log.read_text(errors="replace") if out_log.exists() else ""
        # `open -W` swallows the exit status — CHECK_COMPLETE is decisive.
        return "CHECK_COMPLETE" in output

    # warm first (running-app open -a), then the launch-time deliveries —
    # a mode that fails must not hide the evidence from the others.
    modes = ("warm", "cold", "cold-restore", "welcome")
    failures = [mode for mode in modes if not run_mode(mode)]
    # Reuse the exact app bundle and defaults domain across a native quit.
    # No preferences or saved window state are cleared between launches.
    identifier = f"test.pitex.external-open.{os.getpid()}.relaunch"
    app = make_bundle("relaunch", identifier)
    for mode in ("relaunch-seed", "relaunch"):
        if not run_mode(mode, app, identifier):
            failures.append(mode)
    if failures:
        sys.exit("FAIL: no CHECK_COMPLETE from: " + ", ".join(failures))
