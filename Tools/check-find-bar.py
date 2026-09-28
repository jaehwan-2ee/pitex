#!/usr/bin/env python3
"""check-find-bar.py <Build/Products/Release>

Hosts the production SwiftUI/AppKit editor in a real window and verifies the
native find bar is never covered by the gutter/minimap/fold-chip/ghost
overlays — on show, on the taller Replace layout, after a window resize —
and that the overlays cover the full clip view again after hiding. Clicks on
the bar's controls (counter, query field, its left edge) must land inside
the bar's subtree rather than being intercepted by an overlay.
"""
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
check = r'''
import AppKit
import AppPorts
import EditorMacAdapter
import SwiftUI

private actor Doc: DocumentSessionPort {
    let text: String
    init(_ text: String) { self.text = text }
    func snapshot() async -> DocumentSnapshot { .init(revision: 0, text: text) }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult { .rejected(current: await snapshot()) }
}

@main struct Check {
    @MainActor static func main() {
        // A bundled launch through LaunchServices gives the checker a
        // regular activation policy and a real event loop — direct exec()
        // can leave the process unable to activate.
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            do { try await run(); print("CHECK_COMPLETE"); fflush(nil); exit(0) }
            catch { print("FAIL", error); fflush(nil); exit(1) }
        }
        app.run()
    }

    @MainActor static func run() async throws {
        func require(_ condition: @autoclosure () -> Bool, _ message: String) {
            guard condition() else { print("FAIL", message); fflush(nil); exit(1) }
        }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        let source = "\\begin{itemize}\n"
            + String(repeating: "\\item " + String(repeating: "wrapped text ", count: 20) + "\n", count: 80)
            + "\\end{itemize}\n\nLAST_TARGET\n"
        let adapter = try await EditorMacAdapter.make(session: Doc(source))
        let host = NSHostingView(rootView: EditorContainerView(adapter: adapter).id(ObjectIdentifier(adapter)))
        host.autoresizingMask = [.width, .height]
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 360),
                              styleMask: [.titled, .resizable], backing: .buffered, defer: false)
        window.contentView = host
        window.makeKeyAndOrderFront(nil)
        NSApp.activate()
        defer { window.orderOut(nil) }
        func settle() async throws {
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(200))
            host.layoutSubtreeIfNeeded()
        }
        try await settle()
        let views = descendants(host)
        guard let scroll = views.compactMap({ $0 as? NSScrollView }).first(where: {
                  $0.documentView === adapter.textView }),
              let gutter = views.compactMap({ $0 as? LineNumberGutterView }).first,
              let minimap = views.compactMap({ $0 as? MinimapOverlayView }).first,
              let chips = views.compactMap({ $0 as? FoldChipOverlayView }).first,
              let ghost = views.compactMap({ $0 as? GhostCompletionOverlayView }).first
        else { print("FAIL editor chrome did not mount"); fflush(nil); exit(1) }
        let overlays: [NSView] = [gutter, minimap, chips, ghost]

        func action(_ finderAction: NSTextFinder.Action) -> NSMenuItem {
            let item = NSMenuItem(title: "", action: nil, keyEquivalent: "")
            item.tag = finderAction.rawValue
            return item
        }

        // Every overlay's rect (converted into scroll-view coordinates, the
        // same space they are framed in) must stay clear of the bar's real
        // bounds, and a click on each bar control must resolve inside the
        // bar's subtree — hitTest takes points in the receiver's superview's
        // coordinates, i.e. window coordinates for the content view.
        func requireClearance(_ stage: String) {
            guard let bar = scroll.findBarView, scroll.isFindBarVisible else {
                print("FAIL \(stage): find bar not mounted"); fflush(nil); exit(1)
            }
            let barRect = bar.convert(bar.bounds, to: scroll)
            require(barRect.height > 0, "\(stage): find bar has no height")
            for overlay in overlays {
                let rect = overlay.convert(overlay.bounds, to: scroll)
                require(rect.width > 0 && rect.height > 0,
                        "\(stage): \(type(of: overlay)) lost all size — cannot hide everything")
                require(!rect.intersects(barRect),
                        "\(stage): \(type(of: overlay)) frame \(rect) covers find bar \(barRect)")
            }
            var points: [(String, NSPoint)] = [
                ("bar left edge", bar.convert(NSPoint(x: 4, y: bar.bounds.midY), to: nil)),
            ]
            if let counter = descendants(bar).first(where: {
                $0.accessibilityIdentifier() == "pitex.search.count" }) {
                points.append(("counter", counter.convert(
                    NSPoint(x: counter.bounds.midX, y: counter.bounds.midY), to: nil)))
            }
            if let query = descendants(bar).compactMap({ $0 as? NSTextField })
                .first(where: { $0.isEditable }) {
                points.append(("query field", query.convert(
                    NSPoint(x: query.bounds.midX, y: query.bounds.midY), to: nil)))
            }
            for (name, windowPoint) in points {
                let hit = window.contentView!.hitTest(windowPoint)
                let insideBar = hit.map { descendants(bar).contains($0) } ?? false
                require(insideBar,
                        "\(stage): \(name) click intercepted by "
                        + (hit.map { "\(type(of: $0))" } ?? "nothing"))
            }
        }

        adapter.textView.performTextFinderAction(action(.showFindInterface))
        try await settle()
        requireClearance("find bar open")
        adapter.textView.performTextFinderAction(action(.showReplaceInterface))
        try await settle()
        requireClearance("replace row grown")
        window.setContentSize(NSSize(width: 450, height: 300))
        try await settle()
        requireClearance("window resized while replace row open")
        print("PASS find bar: overlays clear it on show, replace and resize; controls stay clickable")

        adapter.textView.performTextFinderAction(action(.hideFindInterface))
        try await settle()
        require(!scroll.isFindBarVisible, "find bar must hide")
        let content = scroll.contentView.frame
        require(scroll.editorOverlayViewport() == content,
                "hidden find bar must leave the whole clip view to overlays")
        require(gutter.frame.height == content.height && chips.frame == content && ghost.frame == content,
                "gutter/chips/ghost must cover the full viewport again")
        require(abs(minimap.frame.minY - (content.minY + 8)) < 0.5
                && abs(minimap.frame.maxY - (content.maxY - 8)) < 0.5,
                "minimap must cover the full viewport again")
        print("PASS find bar: overlays restored to the full viewport after hiding")
    }
}
'''
with tempfile.TemporaryDirectory(prefix="pitex-find-bar-", dir="/tmp") as directory:
    root = Path(directory)
    bundle = root / "FindBarCheck.app/Contents"
    (bundle / "MacOS").mkdir(parents=True)
    resources = bundle / "Resources"
    resources.mkdir()
    (bundle / "Info.plist").write_bytes(plistlib.dumps({
        "CFBundleExecutable": "check", "CFBundleIdentifier": "test.pitex.findbar",
        "CFBundleDevelopmentRegion": "en", "CFBundlePackageType": "APPL",
    }))
    for locale in (repo / "Mac/Resources").glob("*.lproj"):
        shutil.copytree(locale, resources / locale.name)
    source = root / "Check.swift"
    source.write_text(check)
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    stripped = root / "PitexApp.swift"
    stripped.write_text(app_main.read_text().replace("@main\nstruct PitexApp", "struct PitexApp"))
    executable = bundle / "MacOS/check"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", "arm64-apple-macos15.0", "-I", str(products),
                    str(source), str(stripped),
                    *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift") if p != app_main],
                    *[str(p) for p in products.glob("*.o")], "-o", str(executable)], check=True)
    # Launch through LaunchServices like check-sidebar-ui.py /
    # check-workspace-navigation.py: direct exec() left checkers unable to
    # activate on CI. `open -W` waits but swallows the exit status —
    # CHECK_COMPLETE is authoritative.
    out_log, err_log = root / "check.out.log", root / "check.err.log"
    app = root / "FindBarCheck.app"
    try:
        subprocess.run(["/usr/bin/open", "-n", "-W",
                        "--stdout", str(out_log), "--stderr", str(err_log),
                        "--env", "PI_AGENT_PATH=/usr/bin/false",
                        "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
                        str(app)],
                       check=True, timeout=120)
    finally:
        subprocess.run(["/usr/bin/pkill", "-f", str(app)], check=False)
        for log in (out_log, err_log):
            if log.exists():
                print(log.read_text(errors="replace"), end="")
    output = out_log.read_text(errors="replace") if out_log.exists() else ""
    if "CHECK_COMPLETE" not in output:
        sys.exit("FAIL: completion marker missing from checker log")
