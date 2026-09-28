#!/usr/bin/env python3
"""Real LaunchServices open events against the real PitexApp lifecycle.

Builds a checker .app from the actual Mac sources (PitexApp @main stripped
and driven via PitexApp.main()), then runs three LaunchServices launches
with configuration only through `--env` (naked argv is parsed by Cocoa as
documents to open): warm (running app gets `/usr/bin/open -a` requests),
cold (the file rides in with the launch), cold-restore (session restore
must not spawn a stray window). It asserts workspace count, the requested
document, window visibility/key and app activation without any Dock click
or mocked activation APIs.

Usage: check-external-open.py <Build/Products/Release>
"""
from pathlib import Path
import os
import plistlib
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()

check = r'''
import AppKit
import SwiftUI

/// Set by the didFinishLaunching observer registered in the wrapper —
/// plain storage so the observer can write it from any context.
enum LaunchFlag { nonisolated(unsafe) static var didFinishLaunching = false }

@main enum CheckMain {
    /// Boots the REAL app scene/delegate stack. Only settings that would
    /// otherwise make the check nondeterministic are pinned first — the
    /// bundle's unique identifier gives it an isolated defaults domain.
    @MainActor static func main() {
        let mode = ProcessInfo.processInfo.environment["PITEX_CHECK_MODE"] ?? "warm"
        // Restore-session stays at the app's default (on) for warm and
        // cold-restore — the bundle is fresh, so warm has no recents —
        // while plain cold keeps it off to isolate the file delivery.
        // cold-restore seeds the recents so an accidental restore window
        // next to the explicitly opened file cannot hide.
        UserDefaults.standard.set(mode != "cold", forKey: "pitex.pref.editor.restoreSession")
        if mode == "cold-restore" {
            let root = ProcessInfo.processInfo.environment["PITEX_CHECK_FIXTURE"] ?? ""
            UserDefaults.standard.set(["\(root)/B/main.tex"],
                                      forKey: "pitex.pref.workspace.recentDocuments")
        }
        UserDefaults.standard.set(false, forKey: "pitex.pref.update.autoInstall")
        _ = NotificationCenter.default.addObserver(
            forName: NSApplication.didFinishLaunchingNotification,
            object: nil, queue: nil
        ) { _ in LaunchFlag.didFinishLaunching = true }
        Task { @MainActor in await Driver.run() }
        PitexApp.main()
    }
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

        // Cold modes launch WITH A/main.tex — the ONLY delivery is the
        // launch event itself; nothing is re-sent, so a dropped event
        // cannot be masked. cold-restore also seeds a B recent: the
        // explicit file beats restore — exactly one A window, never a
        // stray B window.
        if mode != "warm" {
            let label = mode == "cold" ? "cold open" : "cold restore"
            require(await until { owning("\(root)/A") != nil },
                    "\(label): A/main.tex never reached a workspace — \(diagnostics())")
            let workspace = owning("\(root)/A")!
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

        // ⌘W through the real menu chain while A's detached preview is
        // key — but only after focus visited B, so stale SwiftUI focus
        // cannot supply the wrong workspace to the command router.
        workspaceA.detachPreview()
        require(await until { workspaceA.detachedPreviewWindow != nil },
                "Detaching A's preview must create its window")
        let detached = workspaceA.detachedPreviewWindow!
        await openExternal("\(root)/B/main.tex")
        require(await until { windowB.isKeyWindow },
                "B must regain key before the preview is refocused")
        detached.makeKeyAndOrderFront(nil)
        require(await until { NSApp.keyWindow === detached },
                "The detached preview must become key")
        require(await until { NSApp.mainWindow !== detached },
                "The detached preview must be key but never main")
        // Let the focus change settle one turn, then dump every ⌘w menu
        // item (title/enabled/hidden/mods/action/target + resolved target)
        // — performKeyEquivalent returned true yet performCloseCommand
        // never ran, so some other 'w' item is claiming the shortcut.
        try? await Task.sleep(for: .milliseconds(100))
        dumpMenuShortcuts()
        print("[diag] key=\(NSApp.keyWindow?.title ?? "nil") main=\(NSApp.mainWindow?.title ?? "nil")")
        fflush(nil)
        let close = NSEvent.keyEvent(with: .keyDown, location: .zero,
            modifierFlags: .command, timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: detached.windowNumber, context: nil,
            characters: "w", charactersIgnoringModifiers: "w",
            isARepeat: false, keyCode: 13)!
        require(NSApp.mainMenu?.performKeyEquivalent(with: close) == true,
                "Cmd-W must be claimed by the main menu — \(diagnostics())")
        require(await until { !workspaceA.previewDetached },
                "Cmd-W on the detached preview must reattach it")
        require(workspaceA.hasProject && workspaceB.hasProject,
                "Cmd-W on the preview must not close either project")
        require(WorkspaceWindows.live.count == 3,
                "Cmd-W on the preview must not touch any workspace window")
        stage("Cmd-W reattached the detached preview; projects intact")

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

    /// Recursively prints every menu item whose key equivalent is 'w'
    /// — which item claims ⌘W while the detached preview is key.
    static func dumpMenuShortcuts() {
        func walk(_ menu: NSMenu, _ path: String) {
            for item in menu.items {
                if let submenu = item.submenu { walk(submenu, path + "/" + item.title) }
                guard item.keyEquivalent.lowercased() == "w" else { continue }
                let action = item.action.map { "\($0)" } ?? "nil"
                let target = item.target.map { "\(type(of: $0))" } ?? "nil"
                print("TRACE menuItem-w '\(path)/\(item.title)' enabled=\(item.isEnabled) "
                    + "hidden=\(item.isHidden) mods=\(item.keyEquivalentModifierMask.rawValue) "
                    + "action=\(action) target=\(target)")
                if let action = item.action {
                    let resolved = NSApp.target(forAction: action, to: item.target, from: item)
                    print("TRACE menuItem-w resolvedTarget=\(resolved.map { "\(type(of: $0))" } ?? "nil")")
                }
            }
        }
        if let menu = NSApp.mainMenu { walk(menu, "") }
        fflush(nil)
    }

    /// State snapshot for bootstrap failures: thread, app lifecycle, the
    /// AppKit window list, and how many workspaces registered a scene.
    static func diagnostics() -> String {
        let app = NSApplication.shared
        let windows = app.windows.map {
            "'\($0.title.isEmpty ? "untitled" : $0.title)' visible=\($0.isVisible) mini=\($0.isMiniaturized)"
        }.joined(separator: "; ")
        return "mainThread=\(Thread.isMainThread) running=\(app.isRunning) "
            + "active=\(app.isActive) hidden=\(app.isHidden) "
            + "didFinishLaunching=\(LaunchFlag.didFinishLaunching) "
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

    # Test-only copy of PitexApp.swift: @main stripped, trace lines added
    # inside the two real delivery paths (behavior otherwise unchanged) so
    # the CI log shows exactly which entry point each open event took.
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    patched = (app_main.read_text()
        .replace("@main\nstruct PitexApp", "struct PitexApp")
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
            "                    workspace.restoreSessionIfNeeded()",
            "                    print(\"TRACE onAppear->restore claimed=\\(workspace.claimedExternalOpen)\")\n"
            "                    fflush(nil)\n"
            "                    workspace.restoreSessionIfNeeded()")
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
    stripped = root / "PitexApp.swift"
    stripped.write_text(patched)
    source = root / "Check.swift"
    source.write_text(check)
    binary = root / "check-bin"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0",
                    "-I", str(products), str(source), str(stripped),
                    *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift") if p != app_main],
                    *[str(p) for p in products.glob("*.o")], "-o", str(binary)], check=True)

    # A fake app-local runtime makes PiRuntimeInstaller.ensureInstalled a
    # no-op (no package.json -> installedVersion() nil -> nothing to do).
    pi_runtime_bin = root / "pi-runtime/bin"
    pi_runtime_bin.mkdir(parents=True)
    fake_pi = pi_runtime_bin / "pi"
    fake_pi.write_text("#!/bin/sh\nexit 0\n")
    fake_pi.chmod(0o755)

    def make_bundle(mode):
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
            "CFBundleIdentifier": f"test.pitex.external-open.{os.getpid()}.{mode}",
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
        return app

    def run_mode(mode):
        app = make_bundle(mode)
        out_log, err_log = root / f"check-{mode}.out.log", root / f"check-{mode}.err.log"
        # No positional arguments and no --args: Cocoa parses naked argv
        # tokens as documents to open. All config travels through --env.
        # Both cold modes hand the file to LaunchServices with the
        # launch — a real double-click on a stopped app.
        command = ["/usr/bin/open", "-n", "-W",
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
    failures = [mode for mode in ("warm", "cold", "cold-restore") if not run_mode(mode)]
    if failures:
        sys.exit("FAIL: no CHECK_COMPLETE from: " + ", ".join(failures))
