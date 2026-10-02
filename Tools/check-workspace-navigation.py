#!/usr/bin/env python3
"""Check a rendered workspace without modifying or building its TeX project.

Usage: check-workspace-navigation.py <Build/Products/Release> [<main.tex>
       <project-relative-source.tex>:<line> [<source.tex>:<line> ...]]
With no fixture arguments the script builds its own two-file project with
xelatex (BasicTeX at /Library/TeX/texbin) and checks it end-to-end,
including the detached preview window lifecycle.
"""
from pathlib import Path
import json
import plistlib
import re
MARKER = re.compile(r"/Check\.swift:\d+:\d+: warning: (?:no 'async' operations occur within 'await' expression|no calls to throwing functions occur within 'try' expression)(?: \[#UnnecessaryEffectMarker\])?$", re.M)
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
check = r'''
import AppKit
import Combine
import CryptoKit
import EditorMacAdapter
import ObjectiveC
import PDFKit
import SwiftUI
import SyncTeXCore
import WebKit
/// Records every bridge post so the checker can FAIL on error traffic
/// product code only logs. The compiled Preview.swift copy is patched to
/// call BridgeProbe.log at the message-handler entry.
enum BridgeProbe {
    nonisolated(unsafe) static var posts: [[String: Any]] = []
    static func log(_ type: String, _ m: [String: Any]) {
        posts.append(["type": type, "m": m])
    }
    /// "" when clean; error traffic and a missing/destroyed worker-ok fail.
    static func audit() -> String {
        var bad: [String] = []
        var workerOK = false
        for p in posts {
            let t = p["type"] as? String ?? ""
            if ["load-error", "destroy-error", "csp", "jserror"].contains(t) {
                bad.append(t)
            }
            if t == "worker-ok" {
                let destroyed = (p["m"] as? [String: Any])?["destroyed"] as? Bool ?? false
                if destroyed { bad.append("worker-ok destroyed") } else { workerOK = true }
            }
        }
        if !workerOK { bad.append("no worker-ok post") }
        return bad.isEmpty ? "" : "bridge audit failed: " + bad.joined(separator: ", ")
    }
}


@MainActor private enum FindIndicatorRecorder {
    static var original: IMP?
    static var calls: [(ObjectIdentifier, NSRange, Bool)] = []
}

@main struct Check {
    @MainActor static func main() {
        // A bundled launch through LaunchServices gives the checker a
        // regular activation policy and a real event loop — direct exec()
        // can leave the process unable to activate, which would make the
        // isKeyWindow/miniaturize assertions fail for the wrong reason.
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            do { try await run()
                let __e = BridgeProbe.audit()
                precondition(__e.isEmpty, __e)
                print("CHECK_COMPLETE"); fflush(nil); exit(0) }
            catch { print("FAIL", error); fflush(nil); exit(1) }
        }
        app.run()
    }

    @MainActor static func run() async throws {
        func require(_ condition: Bool, _ message: String) {
            guard condition else { print("FAIL", message); fflush(nil); exit(1) }
        }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        /// The rendered preview surface: the WKWebView hosting the offline
        /// pdf.js viewer — identified by its app scheme, never class.
        func pitexWeb(_ host: NSView) -> WKWebView? {
            descendants(host).compactMap { $0 as? WKWebView }
                .first { $0.url?.scheme == "pitex-pdfjs" }
        }
        /// Page-world JS probe. `pitex.state()` yields the displayed
        /// generation; gen-parameterized calls need that number.
        func js(_ web: WKWebView, _ body: String) async throws -> Any? {
            try await web.callAsyncJavaScript(body, arguments: [:], in: nil, contentWorld: .page)
        }
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
        // Arguments arrive as JSON in the environment — Cocoa parses
        // naked argv tokens as documents to open.
        let launchArgs = (try? JSONSerialization.jsonObject(with: Data(
            (ProcessInfo.processInfo.environment["PITEX_NAVIGATION_ARGS"] ?? "[]").utf8))) as? [String] ?? []
        guard !launchArgs.isEmpty else { fatalError("PITEX_NAVIGATION_ARGS missing or empty") }
        let main = URL(fileURLWithPath: launchArgs[0]).standardizedFileURL
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
        // Data-side PDFKit oracle stays: MediaBoxes + the string payload.
        let document = PDFDocument(data: workspace.retainedPDF!.data)!
        let pdf = pitexWeb(host)!
        // Wait for the viewer to load and render the retained document —
        // then snapshot the annotation-layer element count, the extracted
        // text and the displayed generation as the identity oracles.
        var docText = ""
        var annCount = -1
        var pdfGen = -1
        var lastProbe = "none"
        for _ in 0..<100 {
            do {
                pdfGen = try await js(pdf, "return window.pitex ? window.pitex.state() : -1;") as? Int ?? -1
                lastProbe = "g=\(pdfGen)"
            } catch { pdfGen = -1; lastProbe = "error: \(error)" }
            if pdfGen >= 0 {
                docText = ""
                annCount = -1
                do {
                    docText = try await js(pdf, """
                        return (async () => { const g = window.pitex.state();
                          return await window.pitex.text(g); })();
                        """) as? String ?? ""
                    annCount = try await js(pdf, """
                        return (() => { const g = window.pitex.state();
                          let n = 0;
                          document.querySelectorAll('.annotationLayer')
                            .forEach(l => { n += l.childElementCount; });
                          return n; })();
                        """) as? Int ?? -1
                    lastProbe = "g=\(pdfGen) ann=\(annCount) text=\(docText.isEmpty ? "empty" : "ok")"
                } catch { docText = ""; annCount = -1; lastProbe = "inner error: \(error)" }
                if !docText.isEmpty && annCount >= 0 { break }
            }
            try await Task.sleep(for: .milliseconds(50))
        }
        require(pdfGen >= 0 && annCount >= 0,
                "pdf.js viewer never rendered the retained document "
                + "(last probe: \(lastProbe))")
        // PDFView scaleFactor=1.17 → set the same absolute zoom through the
        // product's own non-compounding zoom entry point.
        _ = try await js(pdf, """
            return (() => { const g = window.pitex.state();
              return window.pitex.zoomBy(1.17 / window.pitex.position(g).scale); })();
            """)
        var phases: [WorkspacePhase] = []
        let observation = workspace.$phase.sink { phases.append($0) }
        defer { observation.cancel() }

        // Visit new tabs, already-open tabs and the same source twice.
        let targets = Array(launchArgs.dropFirst())
        for (index, target) in (targets + targets.reversed()).enumerated() {
            workspace.settings.inverseSyncHighlight = index % 2 == 1
            workspace.settings.forwardSyncHighlight = index % 3 != 0
            let parts = target.split(separator: ":")
            let source = binding.projectRoot.appendingPathComponent(String(parts[0])).standardizedFileURL
            let forward = try await workspace.syncTeXRunner.forward(binding: binding, sourceURL: source,
                                                                    line: Int(parts[1])!, column: 0)
            let expected = try await workspace.syncTeXRunner.inverse(binding: binding, page: forward.pdf.page, point: forward.pdf.point)
            require(expected.source.path.value == String(parts[0]), "Test target must map to the requested source")
            let pageBox = document.page(at: forward.pdf.page - 1)!.bounds(for: .mediaBox)
            // Scroll the SyncTeX box through the real coordinator path —
            // goto() is the production forward-sync entry: scroll always,
            // overlay gated by the pref (which the checker just set).
            _ = try await js(pdf, """
                return (() => { const g = window.pitex.state();
                  return window.pitex.goto(g, \(forward.pdf.page), \(forward.h),
                                           \(forward.v), \(forward.width), \(forward.height)); })();
                """)
            try await settle()
            // Scroll/zoom oracle: the viewer's reported position (page,
            // scale, scrollTop) replaces the NSScrollView bounds snapshot.
            let posJSON = (try? await js(pdf, """
                return (() => { const g = window.pitex.state();
                  const p = window.pitex.position(g);
                  const c = document.getElementById("viewerContainer");
                  return JSON.stringify({p, scrollLeft: c.scrollLeft,
                    cw: c.clientWidth, ch: c.clientHeight}); })();
                """)) as? String ?? ""
            let frames = split.arrangedSubviews.map(\.frame)
            let scale = ((try? await js(pdf, """
                return (() => { const g = window.pitex.state();
                  return window.pitex.position(g).scale; })();
                """)) as? Double) ?? 0
            require(abs(scale - 1.17) < 0.001, "PDF zoom must hold the staged 1.17")
            phases.removeAll()
            FindIndicatorRecorder.calls.removeAll()
            workspace.environment!.editor.textView.setSelectedRange(NSRange(location: 0, length: 0))
            // Cmd-click client point: checker-side projection from the
            // SyncTeX point through the page element's DOM rect — no
            // product geometry oracle.
            let probe = (try? await js(pdf, """
                return (() => {
                  const g = window.pitex.state();
                  const pageEl = document.querySelector(
                    '.page[data-page-number="\(forward.pdf.page)"]');
                  if (!pageEl) return null;
                  const r = pageEl.querySelector(".canvasWrapper").getBoundingClientRect();
                  const c = document.getElementById("viewerContainer").getBoundingClientRect();
                  const s = r.height / \(pageBox.maxY);
                  const x = r.left + \(forward.pdf.point.x) * s;
                  const y = r.top + \(forward.pdf.point.y) * s;
                  return {x, y, inside: x > c.left && x < c.right && y > c.top && y < c.bottom}; })();
                """)) as? [String: Any]
            require(probe?["inside"] as? Bool == true, "Cmd-click target must be visible")
            // Freeze fix 1: WKWebView.isFlipped is YES on macOS, so the
            // client y (top-down) is already view-space. Mirror only when the
            // view is NOT flipped — the runtime value, not an assumption.
            let clientY = CGFloat(probe!["y"] as! Double)
            let clickPoint = NSPoint(x: probe!["x"] as! Double,
                                     y: pdf.isFlipped ? clientY
                                                      : pdf.bounds.height - clientY)
            require(pdf.bounds.contains(clickPoint), "Cmd-click target must be visible")
            // Delivered-point proof: a capture-phase mousedown listener (once,
            // passive, no preventDefault) records the client point the page
            // actually received; capture runs before the product's handler.
            _ = try await js(pdf, """
                window.__probeDown = null;
                document.addEventListener("mousedown", e => {
                  window.__probeDown = {x: e.clientX, y: e.clientY, meta: e.metaKey};
                }, {capture: true, once: true});
                return "armed";
                """)
            let event = NSEvent.mouseEvent(with: .leftMouseDown, location: pdf.convert(clickPoint, to: nil),
                modifierFlags: .command, timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil, eventNumber: 1, clickCount: 1, pressure: 1)!
            let postsBefore = BridgeProbe.posts.count
            NSApp.sendEvent(event)
            for _ in 0..<40 {
                try await Task.sleep(for: .milliseconds(50))
                if workspace.activeDocumentURL == source, workspace.environment!.editor.selectedRange.location > 0 { break }
            }
            try await settle()
            // X6: read __probeDown only after the existing wait — a null read
            // immediately after sendEvent is a queue race, not a miss.
            let delivered = (try? await js(pdf, "return window.__probeDown || null;"))
                as? [String: Any]

            require(phases.allSatisfy { if case .ready = $0 { true } else { false } },
                    "Switching a source must not replace the workspace with a loading screen")
            require(descendants(host).contains { $0 === split }, "Split view must survive source navigation")
            let stillSame = ((try? await js(pdf, """
                return (async () => { const g = window.pitex.state();
                  return {gen: g, same: g === \(pdfGen),
                          text: await window.pitex.text(g)}; })();
                """)) as? [String: Any])
            require(descendants(host).contains { $0 === pdf }
                    && (stillSame?["same"] as? Bool) == true
                    && (stillSame?["text"] as? String) == docText,
                    "PDF view and document must survive source navigation")
            require(split.arrangedSubviews.map(\.frame) == frames, "User-adjusted pane widths must stay unchanged")
            let posAfter = ((try? await js(pdf, """
                return (() => { const g = window.pitex.state();
                  const p = window.pitex.position(g);
                  const c = document.getElementById("viewerContainer");
                  return JSON.stringify({p, scrollLeft: c.scrollLeft,
                    cw: c.clientWidth, ch: c.clientHeight}); })();
                """)) as? String) ?? "nil"
            require(posAfter == posJSON, "PDF zoom and scroll position must stay unchanged")
            require(workspace.activeDocumentURL == source, "Inverse sync must activate the expected source")
            let text = workspace.environment!.editor.textView
            require(text.window === window && window.firstResponder === text, "New editor must be mounted and focused")
            let selected = text.selectedRange()
            let line = (text.string as NSString).substring(to: selected.location).components(separatedBy: "\n").count
            // Freeze fix 2: record isFlipped + client/view/window coords so a
            // wrong line is attributable, not opaque.
            let clickWin = pdf.convert(clickPoint, to: nil)
            let lastInv = BridgeProbe.posts.dropFirst(postsBefore).last {
                ($0["type"] as? String) == "inverse" }.map { "\($0)" } ?? "none"
            require(line == expected.source.line,
                "Caret must land on the SyncTeX source line "
                + "(expected=\(expected.source.line) got=\(line) "
                + "isFlipped=\(pdf.isFlipped) clientY=\(probe!["y"] as! Double) "
                + "viewPt=\(clickPoint) winPt=\(clickWin) delivered=\(String(describing: delivered)) "
                + "sel=\(text.selectedRange().location) lastInverse=\(lastInv))")
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

            let wrapBefore = ((try? await js(pdf, """
                return (() => {
                  const pageEl = document.querySelector(
                    '.page[data-page-number="\(forward.pdf.page)"]');
                  const w = pageEl?.querySelector(".canvasWrapper");
                  // B4: exclude .pitex-hl — the positioning goto's overlay
                  // (1.5 s timer) may still be mounted at this instant.
                  return w ? [...w.children]
                    .filter(e => !e.classList.contains("pitex-hl")).length : -1; })();
                """)) as? Int) ?? -1
            require(wrapBefore >= 0, "Forward page must be mounted for the marker check")
            // N2: the marker must never reach the printed bytes — hash the
            // PDF shown THIS iteration (navigation may have swapped it).
            let retainedSHA = SHA256.hash(data: workspace.retainedPDF!.data)
                .map { String(format: "%02x", $0) }.joined()
            await workspace.syncForward(line: line, column: expected.source.column)
            // The forward marker is a .pitex-hl overlay in the page's
            // canvasWrapper — DOM-only, never part of the printed page.
            var overlayCount = -1
            for _ in 0..<100 where overlayCount != (workspace.settings.forwardSyncHighlight ? 1 : 0) {
                overlayCount = ((try? await js(pdf, "return document.querySelectorAll('.pitex-hl').length;")) as? Int) ?? -1
                if overlayCount == (workspace.settings.forwardSyncHighlight ? 1 : 0) { break }
                try await Task.sleep(for: .milliseconds(50))
            }
            require(overlayCount == (workspace.settings.forwardSyncHighlight ? 1 : 0),
                    "Forward marker must follow only the forward preference")
            if index == 1 {
                try await Task.sleep(for: .milliseconds(1700))
            } else {
                workspace.settings.forwardSyncHighlight = false
                try await settle()
            }
            var cleared = -1
            for _ in 0..<100 where cleared != 0 {
                cleared = ((try? await js(pdf, "return document.querySelectorAll('.pitex-hl').length;")) as? Int) ?? -1
                if cleared == 0 { break }
                try await Task.sleep(for: .milliseconds(50))
            }
            let annAfter = ((try? await js(pdf, """
                return (() => { let n = 0;
                  document.querySelectorAll('.annotationLayer')
                    .forEach(l => { n += l.childElementCount; });
                  return n; })();
                """)) as? Int) ?? -1
            // N2: the printed source bytes must be untouched by the marker.
            let retainedNow = SHA256.hash(data: workspace.retainedPDF!.data)
                .map { String(format: "%02x", $0) }.joined()
            require(retainedNow == retainedSHA,
                    "Forward highlight must never mutate the printed PDF bytes")
            // N3: the forward page's canvasWrapper must return to its exact
            // pre-forward childElementCount — annCount can be 0 (vacuous),
            // this catches a real DOM node the cleanup would miss.
            let wrapAfter = ((try? await js(pdf, """
                return (() => {
                  const pageEl = document.querySelector(
                    '.page[data-page-number="\(forward.pdf.page)"]');
                  const w = pageEl?.querySelector(".canvasWrapper");
                  return w ? [...w.children]
                    .filter(e => !e.classList.contains("pitex-hl")).length : -1; })();
                """)) as? Int) ?? -1
            require(cleared == 0 && annAfter == annCount && wrapAfter == wrapBefore,
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

        // ── Per-document editor view state ───────────────────────────
        // Each activation rebuilds the editor adapter; the model keeps a
        // caret + scroll snapshot per document. A caret far from the
        // viewport proves the restore is not just scrollRangeToVisible.
        // editorParts waits for the CURRENT adapter's textView to actually
        // mount — SwiftUI commits the adapter swap on a later layout pass,
        // so right after activateDocument the hosted scroll view can still
        // own the previous adapter's document view.
        func editorParts(_ stage: String) async throws -> (NSScrollView, NSTextView) {
            let text = workspace.environment!.editor.textView
            for _ in 0..<100 {
                if let scroll = descendants(host).compactMap({ $0 as? NSScrollView })
                    .first(where: { $0.documentView === text }) {
                    return (scroll, text)
                }
                host.layoutSubtreeIfNeeded()
                try await Task.sleep(for: .milliseconds(50))
            }
            let scrolls = descendants(host).compactMap { $0 as? NSScrollView }
                .map { "docView=\($0.documentView.map { "\(Unmanaged.passUnretained($0).toOpaque())" } ?? "nil")" }
                .joined(separator: ", ")
            print("FAIL \(stage): adapter textView \(Unmanaged.passUnretained(text).toOpaque()) "
                + "window=\(String(describing: text.window)) "
                + "active=\(workspace.activeDocumentURL?.lastPathComponent ?? "nil") "
                + "never mounted under host; scrolls=[\(scrolls)]")
            fflush(nil); exit(1)
        }
        func setEditorState(_ stage: String, selection: NSRange, scrollY: CGFloat) async throws {
            let (scroll, text) = try await editorParts(stage)
            text.setSelectedRange(selection)
            text.scroll(NSPoint(x: 0, y: scrollY))
            scroll.reflectScrolledClipView(scroll.contentView)
        }
        // The staged viewport restore lands on a layout pass after mount —
        // poll for the restored values instead of asserting on one frame.
        func requireState(_ stage: String, expected: EditorMacAdapter.ViewState) async throws {
            var last = EditorMacAdapter.ViewState(selection: NSRange(), scrollOrigin: .zero)
            for _ in 0..<100 {
                let (_, _) = try await editorParts(stage)
                last = workspace.environment!.editor.viewState
                if last.selection == expected.selection,
                   abs(last.scrollOrigin.y - expected.scrollOrigin.y) < 1 { return }
                host.layoutSubtreeIfNeeded()
                try await Task.sleep(for: .milliseconds(50))
            }
            require(false,
                    "\(stage): restored selection \(last.selection) != \(expected.selection) "
                    + "or viewport y \(last.scrollOrigin.y) != \(expected.scrollOrigin.y)")
        }
        let canon: (URL) -> URL = { $0.resolvingSymlinksInPath().standardizedFileURL }
        // activateDocument is done by the time it returns — a failed load
        // (e.g. a figure row) leaves phase failed and swaps the editor away.
        func requireActivated(_ stage: String, _ url: URL) {
            var ready = false
            if case .ready = workspace.phase { ready = true }
            require(ready && workspace.activeDocumentURL.map(canon) == canon(url),
                    "\(stage): \(url.lastPathComponent) must be active+ready "
                    + "(phase=\(workspace.phase), active=\(workspace.activeDocumentURL?.path ?? "nil"))")
        }
        let docA = workspace.activeDocumentURL!
        // projectFiles also lists figures/PDF artifacts — they sort before
        // same-stem sources and cannot be edited. Pick a real source.
        guard let docB = workspace.projectFiles.first(where: {
            WorkspaceModel.isSourceFile($0) && canon($0) != canon(docA)
        }) else {
            print("FAIL view-state check needs a second source file; projectFiles="
                + "\(workspace.projectFiles.map(\.path))"); fflush(nil); exit(1)
        }
        // Hand the outgoing position back at the end — the detached-preview
        // checks below SyncTeX from the caret the navigation loop left.
        let preBlockState = workspace.environment!.editor.viewState
        let (scrollA, textA) = try await editorParts("docA mount")
        let lengthA = (textA.string as NSString).length
        let maxYA = max(textA.bounds.height - scrollA.contentView.bounds.height, 0)
        require(maxYA > 50, "view-state fixture must be scrollable")
        // A: caret near the top, viewport at the very bottom.
        try await setEditorState("docA state",
            selection: NSRange(location: min(4, lengthA), length: 0), scrollY: maxYA)
        try await settle()
        let expectedA = workspace.environment!.editor.viewState
        require(expectedA.scrollOrigin.y > scrollA.contentView.bounds.height / 2,
                "A's viewport must sit far from its caret")
        await workspace.activateDocument(docB)
        requireActivated("docB", docB)
        try await settle()
        let (scrollB, textB) = try await editorParts("docB mount")
        let lengthB = (textB.string as NSString).length
        let maxYB = max(textB.bounds.height - scrollB.contentView.bounds.height, 0)
        require(maxYB > 50, "view-state fixture must be scrollable")
        // B: a real selection plus a mid-document viewport.
        try await setEditorState("docB state",
            selection: NSRange(location: min(20, lengthB), length: min(8, lengthB - min(20, lengthB))),
            scrollY: maxYB / 2)
        try await settle()
        let expectedB = workspace.environment!.editor.viewState

        await workspace.activateDocument(docA)
        requireActivated("A→B→A", docA)
        try await requireState("A→B→A", expected: expectedA)
        await workspace.activateDocument(docB)
        requireActivated("back on B", docB)
        try await requireState("back on B", expected: expectedB)
        print("PASS per-document caret and viewport restored across tab switches")

        // Explicit navigation issued before the restore layout settles
        // must win over the cached viewport.
        await workspace.activateDocument(docA)
        requireActivated("pre-jump", docA)
        workspace.jumpTo(line: 2, column: 0)
        try await settle()
        let nsTextA = workspace.environment!.editor.textView.string as NSString
        let firstBreak = nsTextA.range(of: "\n")
        require(firstBreak.location != NSNotFound, "fixture needs multiple lines")
        let (_, jumpText) = try await editorParts("post-jump mount")
        require(jumpText.selectedRange().location == firstBreak.location + 1,
                "jumpTo must win over the staged viewport restore")
        let jumpLayout = jumpText.layoutManager!
        let jumpGlyph = jumpLayout.glyphIndexForCharacter(at: jumpText.selectedRange().location)
        let jumpRect = jumpLayout.lineFragmentRect(forGlyphAt: jumpGlyph, effectiveRange: nil)
            .offsetBy(dx: jumpText.textContainerOrigin.x, dy: jumpText.textContainerOrigin.y)
        require(jumpText.visibleRect.intersects(jumpRect),
                "explicit jump must leave the target visible, not the cached viewport")
        print("PASS explicit navigation overrides restored view state")
        workspace.environment!.editor.applyViewState(preBlockState)
        try await settle()

        // ── Detached preview window ──────────────────────────────────
        // The real app's WindowReader performs this same assignment when
        // SwiftUI mounts the scene — needed for present()/inverse window
        // fronting since the harness hosts the view by hand.
        workspace.window = window
        WorkspaceWindows.register(workspace, openWindow: nil)
        let liveCount = WorkspaceWindows.live.count
        func waitFor(_ condition: @autoclosure () -> Bool, _ message: String) async throws {
            for _ in 0..<100 where !condition() {
                try await Task.sleep(for: .milliseconds(50))
            }
            require(condition(), message)
        }

        workspace.detachPreview()
        try await settle()
        require(workspace.previewDetached, "detachPreview must mark the pane detached")
        let detached = workspace.detachedPreviewWindow!
        require(WorkspaceWindows.live.count == liveCount,
                "The detached preview must not register as a workspace")
        require(pitexWeb(host) == nil,
                "The inline preview must unmount while detached")
        // The detached window mounts its own pdf.js view — its own web
        // view, own generation sequence, same retained bytes.
        var detachedPDF: WKWebView?
        var detachedSame = false
        for _ in 0..<100 where !detachedSame {
            if let web = pitexWeb(detached.contentView!), web !== pdf {
                detachedPDF = web
                let text = (try? await js(web, """
                    return (async () => { const g = window.pitex.state();
                      return g < 0 ? "" : await window.pitex.text(g); })();
                    """)) as? String
                if text == docText { detachedSame = true }
            }
            if !detachedSame { try await Task.sleep(for: .milliseconds(50)) }
        }
        require(detachedPDF != nil && detachedPDF !== pdf && detachedSame,
                "The detached window must host its own pdf.js view over the same document")

        // Forward sync restores a minimized detached window and lands on it.
        workspace.settings.forwardSyncHighlight = true
        detached.miniaturize(nil)
        try await settle()
        require(detached.isMiniaturized, "Fixture must be able to minimize the detached window")
        let detachedAnnBefore = ((try? await js(detachedPDF!, """
            return (() => { let n = 0;
              document.querySelectorAll('.annotationLayer')
                .forEach(l => { n += l.childElementCount; });
              return n; })();
            """)) as? Int) ?? -1
        await workspace.syncForward()
        try await settle()
        require(!detached.isMiniaturized && detached.isVisible,
                "Forward sync must restore and present the detached preview")
        var detachedOverlays = -1
        for _ in 0..<100 where detachedOverlays != 1 {
            detachedOverlays = ((try? await js(detachedPDF!, "return document.querySelectorAll('.pitex-hl').length;")) as? Int) ?? -1
            if detachedOverlays == 1 { break }
            try await Task.sleep(for: .milliseconds(50))
        }
        let detachedAnnAfter = ((try? await js(detachedPDF!, """
            return (() => { let n = 0;
              document.querySelectorAll('.annotationLayer')
                .forEach(l => { n += l.childElementCount; });
              return n; })();
            """)) as? Int) ?? -1
        require(detachedOverlays == 1 && detachedAnnAfter == detachedAnnBefore,
                "Forward marker must land on the detached PDF: overlays=\(detachedOverlays) "
                + "ann=\(detachedAnnBefore)→\(detachedAnnAfter), "
                + "caret=\(workspace.environment!.editor.selectedRange), "
                + "sync=\(workspace.syncTeXState)")

        // Inverse Cmd-click in the detached window lands in the source
        // editor and fronts its window — including cross-file targets.
        let inverseParts = (targets.last ?? "main.tex:105").split(separator: ":")
        let inverseSource = binding.projectRoot
            .appendingPathComponent(String(inverseParts[0])).standardizedFileURL
        let forward = try await workspace.syncTeXRunner.forward(
            binding: binding, sourceURL: inverseSource,
            line: Int(inverseParts[1])!, column: 0)
        let expected = try await workspace.syncTeXRunner.inverse(
            binding: binding, page: forward.pdf.page, point: forward.pdf.point)
        require(expected.source.path.value == String(inverseParts[0]),
                "The detached-sync target must map back to the requested source")
        let pageBox = document.page(at: forward.pdf.page - 1)!.bounds(for: .mediaBox)
        // Same point math as the inline-navigation loop above: scroll the
        // SyncTeX box into view first, then click the precise point the
        // expected inverse was computed from. goto() routes through the
        // detached coordinator's own generation.
        _ = try await js(detachedPDF!, """
            return (() => { const g = window.pitex.state();
              return window.pitex.goto(g, \(forward.pdf.page), \(forward.h),
                                       \(forward.v), \(forward.width), \(forward.height)); })();
            """)
        try await settle()
        let probe = (try? await js(detachedPDF!, """
            return (() => {
              const pageEl = document.querySelector(
                '.page[data-page-number="\(forward.pdf.page)"]');
              if (!pageEl) return null;
              const r = pageEl.querySelector(".canvasWrapper").getBoundingClientRect();
              const c = document.getElementById("viewerContainer").getBoundingClientRect();
              const s = r.height / \(pageBox.maxY);
              const x = r.left + \(forward.pdf.point.x) * s;
              const y = r.top + \(forward.pdf.point.y) * s;
              return {x, y, inside: x > c.left && x < c.right && y > c.top && y < c.bottom}; })();
            """)) as? [String: Any]
        require(probe?["inside"] as? Bool == true, "Cmd-click target must be visible")
        // Freeze fix 1: runtime isFlipped — WKWebView reports YES on macOS,
        // so client y is already top-down view space.
        let clientY = CGFloat(probe!["y"] as! Double)
        let clickPoint = NSPoint(x: probe!["x"] as! Double,
                                 y: detachedPDF!.isFlipped ? clientY
                                                           : detachedPDF!.bounds.height - clientY)
        require(detachedPDF!.bounds.contains(clickPoint), "Cmd-click target must be visible")
        _ = try await js(detachedPDF!, """
            window.__probeDown = null;
            document.addEventListener("mousedown", e => {
              window.__probeDown = {x: e.clientX, y: e.clientY, meta: e.metaKey};
            }, {capture: true, once: true});
            return "armed";
            """)
        let event = NSEvent.mouseEvent(with: .leftMouseDown,
            location: detachedPDF!.convert(clickPoint, to: nil),
            modifierFlags: .command, timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: detached.windowNumber, context: nil,
            eventNumber: 1, clickCount: 1, pressure: 1)!
        let postsBeforeD = BridgeProbe.posts.count
        NSApp.sendEvent(event)
        try await waitFor(workspace.activeDocumentURL == inverseSource,
                        "Inverse sync from the detached preview must activate the expected source")
        try await settle()
        let deliveredD = (try? await js(detachedPDF!, "return window.__probeDown || null;"))
            as? [String: Any]
        let text = workspace.environment!.editor.textView
        let line = (text.string as NSString).substring(to: text.selectedRange().location)
            .components(separatedBy: "\n").count
        let clickWinD = detachedPDF!.convert(clickPoint, to: nil)
        let lastInvD = BridgeProbe.posts.dropFirst(postsBeforeD).last {
            ($0["type"] as? String) == "inverse" }.map { "\($0)" } ?? "none"
        require(line == expected.source.line,
            "Caret must land on the inverse-sync line "
            + "(expected=\(expected.source.line) got=\(line) "
            + "isFlipped=\(detachedPDF!.isFlipped) clientY=\(probe!["y"] as! Double) "
            + "viewPt=\(clickPoint) winPt=\(clickWinD) delivered=\(String(describing: deliveredD)) "
            + "sel=\(text.selectedRange().location) lastInverse=\(lastInvD))")
        require(window.isKeyWindow,
                "Inverse sync from the detached preview must front the editor window")
        require(detached.isVisible, "The detached window must stay open after inverse sync")

        // ⌘W while the detached window is key reattaches only the pane —
        // the project must survive.
        detached.makeKeyAndOrderFront(nil)
        try await settle()
        require(NSApp.keyWindow === detached, "Fixture must make the detached window key")
        workspace.performCloseCommand()
        try await settle()
        require(!workspace.previewDetached && workspace.detachedPreviewWindow == nil,
                "Cmd-W on the detached preview must reattach it, not close the project")
        require(workspace.hasProject, "Cmd-W on the detached preview must keep the project alive")
        require(window.isVisible, "The workspace window must stay open")
        require(detached.contentView == nil, "The reattached window must drop its hosted view")
        require(pitexWeb(host) != nil,
                "The inline preview must remount after reattach")

        // The window's own close button reattaches the same way.
        workspace.detachPreview()
        try await settle()
        let closed = workspace.detachedPreviewWindow!
        closed.close()
        try await settle()
        require(!workspace.previewDetached && workspace.detachedPreviewWindow == nil,
                "Closing the detached window must reattach the pane")
        require(closed.contentView == nil && closed.delegate == nil,
                "A closed detached window must drop its view and delegate")
        require(pitexWeb(host) != nil,
                "The inline preview must remount after the detached window closes")

        // Closing the owning workspace closes and releases the window.
        workspace.detachPreview()
        try await settle()
        let orphan = workspace.detachedPreviewWindow!
        await workspace.close()
        require(workspace.detachedPreviewWindow == nil && !workspace.previewDetached,
                "Workspace close must tear down the detached preview")
        require(orphan.contentView == nil && !orphan.isVisible,
                "The detached window must close with its workspace")
        print("PASS detached preview: detach/reattach, minimized forward sync, inverse fronting, Cmd-W and teardown")

        await workspace.close()
    }
}
'''
with tempfile.TemporaryDirectory(prefix="pitex-navigation-", dir="/tmp") as directory:
    root = Path(directory)
    bundle = root / "NavigationCheck.app/Contents"
    (bundle / "MacOS").mkdir(parents=True)
    resources = bundle / "Resources"
    resources.mkdir()
    (bundle / "Info.plist").write_bytes(plistlib.dumps({
        "CFBundleExecutable": "check", "CFBundleIdentifier": "test.pitex.navigation",
        "CFBundleDevelopmentRegion": "en", "CFBundlePackageType": "APPL",
    }))
    for locale in (repo / "Mac/Resources").glob("*.lproj"):
        shutil.copytree(locale, resources / locale.name)
    # The pdf.js viewer payload — every rendered probe 404s without it.
    # Verified against MANIFEST.sha256 (Release-parity gate).
    import hashlib
    pdfjs_dest = resources / "pdfjs"
    shutil.copytree(repo / "Mac/Resources/pdfjs", pdfjs_dest)
    manifest = [l.split(None, 1) for l in (pdfjs_dest / "MANIFEST.sha256").read_text().splitlines() if l.strip()]
    bad = [p for h, p in manifest
           if hashlib.sha256((pdfjs_dest / p.lstrip("./")).read_bytes()).hexdigest() != h]
    assert not bad, f"pdfjs payload diverged from MANIFEST.sha256: {bad}"
    source = root / "Check.swift"
    source.write_text(check)
    preview_main = repo / "Mac/Sources/Features/Preview.swift"
    _prev = preview_main.read_text()
    _anchor = ('            guard let m = message.body as? [String: Any],\n'
               '                  let type = m["type"] as? String else { return }')
    assert _prev.count(_anchor) == 1
    preview_copy = root / "Preview.swift"
    preview_copy.write_text(_prev.replace(
        _anchor, _anchor + '\n            BridgeProbe.log(type, m)'))
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    stripped = root / "PitexApp.swift"
    stripped.write_text(app_main.read_text().replace("@main\nstruct PitexApp", "struct PitexApp"))
    executable = bundle / "MacOS/check"
    _cc = subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-target", "arm64-apple-macos15.0",
                    "-I", str(products), str(source), str(stripped),
                    *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift")
                     if p not in (app_main, preview_main)],
                     str(preview_copy),
                    *[str(p) for p in products.glob("*.o")], "-o", str(executable)],
                   check=False, capture_output=True, text=True)
    print(_cc.stdout + _cc.stderr, end="")
    if _cc.returncode != 0:
        sys.exit(f"FAIL: swiftc rc={_cc.returncode} — no functional verdict")
    _bad = MARKER.findall(_cc.stderr + _cc.stdout)
    if _bad:
        for line in _cc.stderr.splitlines() + _cc.stdout.splitlines():
            if MARKER.search(line): print(line)
        sys.exit("FAIL: harness compile defect — UnnecessaryEffectMarker in "
                 "generated Check.swift (discarded async query); no functional verdict")
    if len(sys.argv) > 2:
        # Caller-supplied fixture: a built project + source:line targets.
        fixture_args = sys.argv[2:]
    else:
        # Self-contained fixture (same shape as check-macos-synctex.py):
        # a two-file project the navigation loop can round-trip through
        # SyncTeX. Needs BasicTeX's xelatex at the harness path.
        fixture = root / "fixture"
        (fixture / "sections").mkdir(parents=True)
        (fixture / "main.tex").write_text(
            "\\documentclass{article}\n\\begin{document}\nFirst page.\\par\n\\newpage\n"
            + "\n" * 100 + "RootTarget\\par\n\\newpage\n\\input{sections/child}\n\\end{document}\n"
        )
        (fixture / "sections/child.tex").write_text("% child\n" + "\n" * 100 + "ChildTarget\\par\n")
        subprocess.run(["/Library/TeX/texbin/xelatex", "-synctex=1", "-interaction=nonstopmode",
                        "-halt-on-error", "main.tex"], cwd=fixture, check=True, stdout=subprocess.DEVNULL)
        fixture_args = [str(fixture / "main.tex"), "main.tex:105", "sections/child.tex:102"]
    # Launch through LaunchServices like check-sidebar-ui.py: directly
    # exec()ing the Mach-O left the checker unable to activate on CI, and
    # the isKeyWindow assertions need a real active app. `open -W` waits
    # but swallows the exit status — CHECK_COMPLETE is authoritative.
    out_log, err_log = root / "check.out.log", root / "check.err.log"
    app = root / "NavigationCheck.app"
    try:
        subprocess.run(["/usr/bin/open", "-n", "-W",
                        "--stdout", str(out_log), "--stderr", str(err_log),
                        "--env", "PI_AGENT_PATH=/usr/bin/false",
                        "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
                        "--env", f"PITEX_NAVIGATION_ARGS={json.dumps(fixture_args)}",
                        str(app)],
                       check=True, timeout=180)
    finally:
        subprocess.run(["/usr/bin/pkill", "-f", str(app)], check=False)
        for log in (out_log, err_log):
            if log.exists():
                print(log.read_text(errors="replace"), end="")
    output = out_log.read_text(errors="replace") if out_log.exists() else ""
    if "CHECK_COMPLETE" not in output:
        sys.exit("FAIL: completion marker missing from checker log")
