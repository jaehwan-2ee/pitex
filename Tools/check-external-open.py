#!/usr/bin/env python3
"""Real LaunchServices open events against the real PitexApp lifecycle.

Builds a checker .app from the actual Mac sources (PitexApp @main stripped
and driven via PitexApp.main()), launches it hidden through `open`, then
has an in-app driver send itself /usr/bin/open -a requests — the same
events Finder `Open With`/`open -a` produce. It asserts workspace count,
the requested document, window visibility/key and app activation without
any Dock click or mocked activation APIs.

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

@main enum CheckMain {
    /// Boots the REAL app scene/delegate stack. Only settings that would
    /// otherwise make the check nondeterministic are pinned first — the
    /// bundle's unique identifier gives it an isolated defaults domain.
    static func main() async {
        UserDefaults.standard.set(false, forKey: "pitex.pref.editor.restoreSession")
        UserDefaults.standard.set(false, forKey: "pitex.pref.update.autoInstall")
        Task { @MainActor in await Driver.run() }
        await PitexApp.main()
    }
}

@MainActor enum Driver {
    static let root = CommandLine.arguments[1]

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

        // The first scene window must exist before firing opens — the
        // empty-window route branch needs a live workspace to attach to.
        require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                "App never produced an initial window")
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
        // (app not hidden). Opening another folder must create exactly
        // one new window, fronted, with no Dock click.
        let windowA = workspaceA.window!
        windowA.miniaturize(nil)
        require(await until { windowA.isMiniaturized }, "Could not minimize A for the background-open case")
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
                "open -a while A is minimized must keep the app active")
        require(await until { ready(workspaceB) && workspaceB.activeDocumentURL?.lastPathComponent == "main.tex" },
                "B must open its requested document")
        let windowB = workspaceB.window!
        stage("B opened a new window while A was minimized")

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
        let close = NSEvent.keyEvent(with: .keyDown, location: .zero,
            modifierFlags: .command, timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: detached.windowNumber, context: nil,
            characters: "w", charactersIgnoringModifiers: "w",
            isARepeat: false, keyCode: 13)!
        NSApp.sendEvent(close)
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

    # Bundle = production Info.plist with a unique test identifier, so
    # LaunchServices registers and routes document opens for real.
    bundle = root / "ExternalOpenCheck.app/Contents"
    (bundle / "MacOS").mkdir(parents=True)
    resources = bundle / "Resources"
    resources.mkdir()
    plist = plistlib.loads((repo / "Mac/Config/Info.plist").read_bytes())
    plist.update({
        "CFBundleExecutable": "check",
        "CFBundleIdentifier": f"test.pitex.external-open.{os.getpid()}",
        "CFBundleName": "ExternalOpenCheck",
        "CFBundleDisplayName": "ExternalOpenCheck",
        "CFBundleDevelopmentRegion": "en",
        "CFBundleShortVersionString": "0",
        "CFBundleVersion": "0",
        "LSMinimumSystemVersion": "15.0",
    })
    (bundle / "Info.plist").write_bytes(plistlib.dumps(plist))
    for locale in (repo / "Mac/Resources").glob("*.lproj"):
        shutil.copytree(locale, resources / locale.name)

    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    stripped = root / "PitexApp.swift"
    stripped.write_text(app_main.read_text().replace("@main\nstruct PitexApp", "struct PitexApp"))
    source = root / "Check.swift"
    source.write_text(check)
    executable = bundle / "MacOS/check"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0",
                    "-I", str(products), str(source), str(stripped),
                    *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift") if p != app_main],
                    *[str(p) for p in products.glob("*.o")], "-o", str(executable)], check=True)

    # A fake app-local runtime makes PiRuntimeInstaller.ensureInstalled a
    # no-op (no package.json -> installedVersion() nil -> nothing to do).
    pi_runtime_bin = root / "pi-runtime/bin"
    pi_runtime_bin.mkdir(parents=True)
    fake_pi = pi_runtime_bin / "pi"
    fake_pi.write_text("#!/bin/sh\nexit 0\n")
    fake_pi.chmod(0o755)

    # Launch through LaunchServices so the bundle registers; the driver
    # minimizes/hides the app itself for the background-open stages.
    # `open -W` swallows the exit status — CHECK_COMPLETE is authoritative.
    out_log, err_log = root / "check.out.log", root / "check.err.log"
    app = root / "ExternalOpenCheck.app"
    try:
        subprocess.run(["/usr/bin/open", "-n", "-W",
                        "--stdout", str(out_log), "--stderr", str(err_log),
                        "--env", "PI_AGENT_PATH=/usr/bin/false",
                        "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
                        str(app), "--args", str(fixture)],
                       check=True, timeout=180)
    finally:
        subprocess.run(["/usr/bin/pkill", "-f", str(app)], check=False)
        for log in (out_log, err_log):
            if log.exists():
                print(log.read_text(errors="replace"), end="")
    output = out_log.read_text(errors="replace") if out_log.exists() else ""
    if "CHECK_COMPLETE" not in output:
        sys.exit("FAIL: completion marker missing from checker log")
