#!/usr/bin/env python3
"""Check a rendered workspace without modifying or building its TeX project.

Usage: check-workspace-navigation.py <Build/Products/Release> <main.tex>
       <project-relative-source.tex>:<line> [<source.tex>:<line> ...]
The main PDF and SyncTeX metadata must already exist.
"""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
check = r'''
import AppKit
import Combine
import ObjectiveC
import PDFKit
import SwiftUI
import SyncTeXCore

@MainActor private enum FindIndicatorRecorder {
    static var original: IMP?
    static var calls: [(ObjectIdentifier, NSRange, Bool)] = []
}

@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        func require(_ condition: Bool, _ message: String) {
            guard condition else { print("FAIL", message); fflush(nil); exit(1) }
        }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        let defaults = UserDefaults.standard
        let keys = ["pitex.pref.synctex.inverseHighlight", "pitex.pref.synctex.forwardHighlight"]
        let saved = keys.map { defaults.object(forKey: $0) }
        defer {
            for (key, value) in zip(keys, saved) {
                if let value { defaults.set(value, forKey: key) } else { defaults.removeObject(forKey: key) }
            }
        }
        keys.forEach { defaults.removeObject(forKey: $0) }
        let workspace = WorkspaceModel()
        require(!workspace.settings.inverseSyncHighlight && !workspace.settings.forwardSyncHighlight,
                "Both highlight preferences must default to off")
        for (inverse, forward) in [(true, false), (false, true), (true, true), (false, false)] {
            workspace.settings.inverseSyncHighlight = inverse
            workspace.settings.forwardSyncHighlight = forward
            let restored = SettingsStore()
            require(restored.inverseSyncHighlight == inverse && restored.forwardSyncHighlight == forward,
                    "Highlight preferences must persist independently")
        }
        print("PASS independent highlight preferences: defaults and persistence")

        // Record the real native indicator call, including whether the new
        // editor was mounted before showing it; keep AppKit's rendering intact.
        let method = class_getInstanceMethod(NSTextView.self, #selector(NSTextView.showFindIndicator(for:)))!
        typealias IndicatorIMP = @convention(c) (NSTextView, Selector, NSRange) -> Void
        let record: IndicatorIMP = { view, selector, range in
            MainActor.assumeIsolated {
                FindIndicatorRecorder.calls.append((ObjectIdentifier(view), range, view.window != nil))
                unsafeBitCast(FindIndicatorRecorder.original!, to: IndicatorIMP.self)(view, selector, range)
            }
        }
        FindIndicatorRecorder.original = method_setImplementation(method, unsafeBitCast(record, to: IMP.self))
        defer { method_setImplementation(method, FindIndicatorRecorder.original!) }
        let main = URL(fileURLWithPath: CommandLine.arguments[1]).standardizedFileURL
        await workspace.open(main)
        guard let binding = workspace.syncTeXBinding else { fatalError("Open a built project with SyncTeX metadata") }
        let host = NSHostingView(rootView: WorkspaceView(workspace: workspace))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1400, height: 850),
                              styleMask: [.titled, .resizable], backing: .buffered, defer: false)
        window.contentView = host
        window.makeKeyAndOrderFront(nil)
        defer { window.orderOut(nil) }
        func settle() async throws {
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(200))
            host.layoutSubtreeIfNeeded()
        }
        try await settle()
        let splits = descendants(host).compactMap { $0 as? NSSplitView }
        let split = splits.first { $0.isVertical && $0.arrangedSubviews.count == 3 }!
        split.setPosition(240, ofDividerAt: 0)
        split.setPosition(835, ofDividerAt: 1)
        try await settle()
        let pdf = descendants(host).compactMap { $0 as? PDFView }.first!
        let document = pdf.document!
        let existingNote = PDFAnnotation(bounds: NSRect(x: 20, y: 20, width: 50, height: 10), forType: .highlight, withProperties: nil)
        document.page(at: 0)!.addAnnotation(existingNote)
        let originalAnnotations = (0..<document.pageCount).flatMap { document.page(at: $0)!.annotations }
        pdf.autoScales = false
        pdf.scaleFactor = 1.17
        var phases: [WorkspacePhase] = []
        let observation = workspace.$phase.sink { phases.append($0) }
        defer { observation.cancel() }

        // Visit new tabs, already-open tabs and the same source twice.
        let targets = Array(CommandLine.arguments.dropFirst(2))
        for (index, target) in (targets + targets.reversed()).enumerated() {
            workspace.settings.inverseSyncHighlight = index % 2 == 1
            workspace.settings.forwardSyncHighlight = index % 3 != 0
            let parts = target.split(separator: ":")
            let source = binding.projectRoot.appendingPathComponent(String(parts[0])).standardizedFileURL
            let forward = try await workspace.syncTeXRunner.forward(binding: binding, sourceURL: source,
                                                                    line: Int(parts[1])!, column: 0)
            let expected = try await workspace.syncTeXRunner.inverse(binding: binding, page: forward.pdf.page, point: forward.pdf.point)
            require(expected.source.path.value == String(parts[0]), "Test target must map to the requested source")
            let page = document.page(at: forward.pdf.page - 1)!
            let rect = NSRect(x: forward.h, y: page.bounds(for: .mediaBox).maxY - forward.v,
                              width: max(forward.width, 4), height: max(forward.height, 4))
            pdf.go(to: rect, on: page)
            try await settle()
            let scroll = descendants(pdf).compactMap { $0 as? NSScrollView }.first!
            let bounds = scroll.contentView.bounds
            let frames = split.arrangedSubviews.map(\.frame)
            let scale = pdf.scaleFactor
            phases.removeAll()
            FindIndicatorRecorder.calls.removeAll()
            workspace.environment!.editor.textView.setSelectedRange(NSRange(location: 0, length: 0))
            let pdfPoint = NSPoint(x: forward.pdf.point.x,
                                  y: page.bounds(for: .mediaBox).maxY - forward.pdf.point.y)
            let clickPoint = pdf.convert(pdfPoint, from: page)
            require(pdf.bounds.contains(clickPoint), "Cmd-click target must be visible")
            let event = NSEvent.mouseEvent(with: .leftMouseDown, location: pdf.convert(clickPoint, to: nil),
                modifierFlags: .command, timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil, eventNumber: 1, clickCount: 1, pressure: 1)!
            NSApp.sendEvent(event)
            for _ in 0..<40 {
                try await Task.sleep(for: .milliseconds(50))
                if workspace.activeDocumentURL == source, workspace.environment!.editor.selectedRange.location > 0 { break }
            }
            try await settle()

            require(phases.allSatisfy { if case .ready = $0 { true } else { false } },
                    "Switching a source must not replace the workspace with a loading screen")
            require(descendants(host).contains { $0 === split }, "Split view must survive source navigation")
            require(descendants(host).contains { $0 === pdf } && pdf.document === document,
                    "PDF view and document must survive source navigation")
            require(split.arrangedSubviews.map(\.frame) == frames, "User-adjusted pane widths must stay unchanged")
            require(abs(pdf.scaleFactor - scale) < 0.001 && scroll.contentView.bounds == bounds,
                    "PDF zoom and scroll position must stay unchanged")
            require(workspace.activeDocumentURL == source, "Inverse sync must activate the expected source")
            let text = workspace.environment!.editor.textView
            require(text.window === window && window.firstResponder === text, "New editor must be mounted and focused")
            let selected = text.selectedRange()
            let line = (text.string as NSString).substring(to: selected.location).components(separatedBy: "\n").count
            require(line == expected.source.line, "Caret must land on the SyncTeX source line")
            let layout = text.layoutManager!
            let glyph = layout.glyphIndexForCharacter(at: selected.location)
            let caretLine = layout.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
                .offsetBy(dx: text.textContainerOrigin.x, dy: text.textContainerOrigin.y)
            require(text.visibleRect.intersects(caretLine), "Target source line must be visible after mounting")
            let indicator = FindIndicatorRecorder.calls.last { $0.0 == ObjectIdentifier(text) }
            require(indicator?.2 == true, "Native indicator must run after the editor is mounted")
            require((indicator!.1.length > 0) == workspace.settings.inverseSyncHighlight,
                    "Inverse indicator must follow only the inverse preference")
            require(workspace.buildSourceURL() == main, "Inverse sync must retain the main build target")
            print("PASS \(expected.source.path.value):\(line), PDF page \(forward.pdf.page): stable panes, PDF, caret and focus")

            await workspace.syncForward(line: line, column: expected.source.column)
            let marked = (0..<document.pageCount).flatMap { document.page(at: $0)!.annotations }
            let markers = marked.filter { annotation in !originalAnnotations.contains { $0 === annotation } }
            require(markers.count == (workspace.settings.forwardSyncHighlight ? 1 : 0),
                    "Forward marker must follow only the forward preference")
            require(markers.allSatisfy { !$0.shouldPrint }, "Navigation markers must not appear in printed PDFs")
            if index == 1 {
                try await Task.sleep(for: .milliseconds(1700))
            } else {
                workspace.settings.forwardSyncHighlight = false
                try await settle()
            }
            let cleared = (0..<document.pageCount).flatMap { document.page(at: $0)!.annotations }
            require(cleared.count == originalAnnotations.count && originalAnnotations.allSatisfy { old in cleared.contains { $0 === old } },
                    "Only the temporary sync marker must be removed on timeout or when disabled")
            print("PASS independent inverse/forward highlights and temporary marker cleanup")
            fflush(nil)
        }
        // Ordinary sidebar/tab activation uses the same navigation path.
        let frames = split.arrangedSubviews.map(\.frame)
        await workspace.activateDocument(main)
        try await settle()
        require(descendants(host).contains { $0 === pdf } && split.arrangedSubviews.map(\.frame) == frames,
                "Manual tab changes must also preserve the preview and pane widths")
        print("PASS manual tab activation preserves workspace")
        await workspace.close()
    }
}
'''
with tempfile.TemporaryDirectory(prefix="pitex-navigation-", dir="/tmp") as directory:
    root = Path(directory)
    source = root / "Check.swift"
    source.write_text(check)
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    stripped = root / "PitexApp.swift"
    stripped.write_text(app_main.read_text().replace("@main\nstruct PitexApp", "struct PitexApp"))
    executable = root / "check"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0",
                    "-I", str(products), str(source), str(stripped),
                    *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift") if p != app_main],
                    *[str(p) for p in products.glob("*.o")], "-o", str(executable)], check=True)
    env = {**os.environ, "PI_AGENT_PATH": "/usr/bin/false", "PI_CODING_AGENT_DIR": str(root / "pi")}
    subprocess.run([str(executable), *sys.argv[2:]], env=env, check=True, timeout=90)
