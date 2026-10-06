#!/usr/bin/env python3
r"""check-equation-preview-macos.py <Build/Products/Release>

Real macOS acceptance harness for the equation hover/caret preview. Builds
a checker .app from the actual production Mac sources (production @main
intact; a test-only applicationDidFinishLaunching hook starts the driver),
loads the REAL bundled MathJax asset tree from Resources/equation-preview,
opens a real temporary TeX project through WorkspaceWindows.route, and
exercises the production EquationPreviewController end to end:

- genuine HID pointer: real Quartz mouseMoved events posted at the HID tap
  move the real cursor, which the window server routes to the real
  NSTrackingArea. The per-run checker app cannot hold post-event access,
  so THIS launcher (run by the operator's trusted automation identity)
  posts them on the driver's request through a private per-run directory.
  A launcher without post-event access is a FAIL, never a synthesized
  fallback; every target is inside the checker's own window, owned by it
  on screen (checked on both sides), and the cursor's arrival is verified;
- caret entry/exit, hover switch, pointer-exit grace, edit invalidation;
- \input macro context + external redefinition refresh on app activation;
- escaped-dollar/comment/verbatim negatives;
- A->B document switch (late results cannot target the wrong file);
- Escape, focus loss, scroll out/re-anchor, the popover size cap (720 pt wide;
  height min(max(360, 55 % of the screen's visible height), 720) pt), delay, placement
  (changed through the real settings sheet, which takes focus; measured
  with finite, current, stable public content/anchor screen rectangles
  in Cocoa y-up coordinates, never CG-list sentinels);
- settings round-trip + imported-garbage normalization on the real store;
- exact preview: TWO distinct exact TeX documents rendered with the real
  project compiler in the same workspace. Without TeX the quiet
  unavailable message is still checked, but the run never completes;
- dark/light readability of the live re-presented page (opaque capture,
  WCAG fg/bg contrast); high-contrast reported when the system toggle is
  off (a global user preference we must not flip);
- cold page, first hover, fresh, cache and exact latency printed
  separately — measured, never assumed.
A wait for a visible preview either starts from a verified hidden state or
names its fresh target (source, body kind, badge): the engine keeps the
previous preview up while a new one renders, so bare visibility would
pass on stale content.
Screenshots capture the preview's own views in-process (public
WKWebView.takeSnapshot for the fast page, cacheDisplay for AppKit bodies) —
no screen-recording permission, no global capture. Waits are async polls
(Task.sleep) so the app's own run loop keeps driving production MainActor
tasks, timers and WebKit callbacks; inputs are injected synchronously and
conditions awaited afterwards. Cloud D1-D4 stages are asserted inline:
key-window/firstResponder on every visible state, real mouseMoved through
the tracking area, Escape precedence, and a forced page hang + guarded
private _killWebContentProcess in the test-only checker exercising
timeout/replacement and B5 budget exhaustion.
Usage: check-equation-preview-macos.py <products-dir> [out-dir]
PITEX_CHECK_COMPILE_ONLY=1 stops after the strict swiftc compile (no UI).
PITEX_CHECK_HID_PREFLIGHT=1 stops after the caret pre-flight and ONE bridged
cursor move inside the checker window (HID_PREFLIGHT_OK on success).
PITEX_CHECK_HOVER_DIAGNOSTIC=1 adds bounded lifecycle tracing to a generated
EquationPreview.swift COPY and stops after the first genuine hover plus
the existing D1 focus/no-churn check. Requires a new output directory;
preserves generated sources, fixture and checker app there; reports
HOVER_DIAGNOSTIC_OK (never CHECK_COMPLETE).
PITEX_CHECK_ESCAPE_DIAGNOSTIC=1 uses the same copied-source trace but follows
the original input sequence through document B and its first Escape,
recording responder identities before/after each transition. Stops there
with ESCAPE_DIAGNOSTIC_OK only if the original assertions pass.
With an output directory, full runs retain the same exact compiler inputs,
checker app and fixture before temporary cleanup. Copy errors are reported
in missing-evidence.json without overriding the functional exit result.
"""
from pathlib import Path
import ctypes
import json
import math
import os
import plistlib
import re
import stat
import shutil
import subprocess
import sys
import tempfile
import time

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
out_dir = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else None
escape_diagnostic = os.environ.get("PITEX_CHECK_ESCAPE_DIAGNOSTIC") == "1"
hover_diagnostic = os.environ.get("PITEX_CHECK_HOVER_DIAGNOSTIC") == "1" or escape_diagnostic
if hover_diagnostic and out_dir is None:
    sys.exit("hover diagnostic requires a round-specific output directory")

# Retention is limited to seven known artifacts: three compiler inputs,
# two logs, the checker bundle and its fixture. Copy failures are evidence
# gaps, not functional failures; record them before temporary cleanup.
evidence_errors = []


def retain_evidence(source, destination):
    try:
        destination.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        if source.is_dir():
            shutil.copytree(source, destination)
        else:
            shutil.copy2(source, destination)
    except (OSError, shutil.Error) as error:
        missing = {"source": str(source), "destination": str(destination),
                   "error": str(error)[:4096]}
        evidence_errors.append(missing)
        print(f"[evidence] MISSING {json.dumps(missing)}", flush=True)
        try:
            (out_dir / "missing-evidence.json").write_text(json.dumps(evidence_errors, indent=2) + "\n")
        except OSError as record_error:
            print(f"[evidence] MISSING record could not be written: {record_error}", flush=True)

check = r'''
import AppKit
import CoreGraphics
import SwiftUI
import WebKit

@MainActor
enum LaunchFlag {
    static var didFinishLaunching = false
    static var finishUserInfo = "nil"
}

/// Diagnostic-only generated sources call this bounded MainActor logger.
/// No instrumentation or private state is added to production sources.
@MainActor enum HoverDiagnostic {
    static let enabled = ProcessInfo.processInfo.environment["PITEX_CHECK_HOVER_DIAGNOSTIC"] == "1"
    static let epoch = ProcessInfo.processInfo.systemUptime
    static var lines = 0
    static func log(_ message: String) {
        guard enabled, lines < 400 else { return }
        lines += 1
        let ms = Int((ProcessInfo.processInfo.systemUptime - epoch) * 1000)
        print("[hovertrace] \(ms)ms \(message)")
        if lines == 400 { print("[hovertrace] trace cap reached (400 lines)") }
        fflush(nil)
    }
}

/// Value-only public geometry; no private window flags or CG-list fallback.
private struct PlacementGeometry: Equatable, Sendable {
    let editorID: ObjectIdentifier
    let contentID: ObjectIdentifier
    let previewWindowID: ObjectIdentifier
    let previewWindowNumber: Int
    let documentID: String
    let source: String
    let setting: String
    let sourceRange: NSRange
    let content: CGRect
    let anchor: CGRect
}

@MainActor enum Driver {
    static let root = ProcessInfo.processInfo.environment["PITEX_CHECK_FIXTURE"] ?? ""
    static let shots = ProcessInfo.processInfo.environment["PITEX_CHECK_OUT"] ?? ""
    static var texAvailable = false
    // MainActor-isolated counters — observer/callback closures enter with
    // MainActor.assumeIsolated; no nonisolated(unsafe) or @unchecked Sendable.
    static var popoverShows = 0
    static var popoverCloses = 0
    static var replaceCount = 0
    /// Pointer events the app's own event stream delivered (local monitor):
    /// separates "HID never reached the app" from "tracking/engine failed".
    static var pointerEvents = 0
    /// HID bridge requests sent (sequence number of the last one).
    static var hidRequests = 0
    static let escapeDiagnostic = ProcessInfo.processInfo.environment["PITEX_CHECK_ESCAPE_DIAGNOSTIC"] == "1"
    typealias PreviewState = (visible: Bool, role: String, source: String, badge: String, win: NSWindow?)

    static func run() async {
        func require(_ condition: Bool, _ message: String) {
            guard condition else { print("FAIL", message); fflush(nil); exit(1) }
        }
        func stage(_ message: String) { print("[stage]", message); fflush(nil) }
        func note(_ message: String) { print("[note]", message); fflush(nil) }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        // Ordered AppKit lifecycle evidence around the first real hover.
        // Only the throwaway diagnostic binary installs these observers.
        if HoverDiagnostic.enabled {
            for name in [NSWindow.didBecomeKeyNotification, NSWindow.didResignKeyNotification,
                         NSPopover.willShowNotification, NSPopover.didShowNotification,
                         NSPopover.willCloseNotification, NSPopover.didCloseNotification] {
                _ = NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { notification in
                    let eventName = notification.name.rawValue
                    MainActor.assumeIsolated {
                        HoverDiagnostic.log("notification \(eventName) appKey=\(NSApp.keyWindow?.windowNumber ?? -1) windows=" + NSApp.windows.map {
                            "\($0.windowNumber):\(type(of: $0)):visible=\($0.isVisible):key=\($0.isKeyWindow):responder=\(String(describing: $0.firstResponder))"
                        }.joined(separator: ";"))
                    }
                }
            }
        }

        // ── Fixture open through the real route ──────────────────────────
        stage("waiting for initial empty window")
        require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                "app never produced an initial window — \(appDiag())")
        WorkspaceWindows.route(URL(fileURLWithPath: root + "/main.tex"))
        var found: WorkspaceModel? = nil
        require(await until {
            found = WorkspaceWindows.live.first { $0.owns(URL(fileURLWithPath: root + "/main.tex")) }
            return found != nil
        }, "fixture project never reached a workspace")
        let workspace = found!
        require(await until { if case .ready = workspace.phase { return true }; return false },
                "project must reach .ready — \(appDiag())")
        require(workspace.equationPreview != nil, "workspace must own an EquationPreviewController")
        stage("project ready")
        let window = workspace.window!
        NSApp.activate(ignoringOtherApps: true)
        // Deterministic geometry: every hover/caret target sits in the
        // fixture's first ~20 lines, which must be on screen (present()
        // hides a scrolled-out anchor), with screen room above the window
        // so "above" placement is not flipped by the screen edge.
        if let frame = window.screen?.visibleFrame {
            let size = NSSize(width: min(1280, frame.width - 40), height: min(860, frame.height - 80))
            window.setFrame(NSRect(x: frame.midX - size.width / 2, y: frame.minY + 20,
                                   width: size.width, height: size.height), display: true)
        }

        // Each document activation creates a new native adapter/view.
        // Fetch it live for interactions; retain the initial view only for
        // startup geometry and diagnostic identity.
        func tv() -> NSTextView? { workspace.environment?.editor.textView }
        require(await until { tv()?.window != nil }, "editor never mounted")
        let editor = tv()!
        require(await until { window.isKeyWindow && window.firstResponder === editor }
                || window.makeFirstResponder(editor), "editor must be first responder")
        editor.layoutManager?.ensureLayout(for: editor.textContainer!)
        stage("editor mounted firstResponder=\(window.firstResponder === editor)")

        // ── Popover probes via the in-process view tree ──────────────────
        func previewElement() -> NSView? {
            for w in NSApp.windows {
                if let found = descendants(w.contentView ?? NSView())
                    .first(where: { $0.accessibilityIdentifier() == "pitex.equationPreview" }) {
                    return found
                }
                for child in w.childWindows ?? [] {
                    if let found = descendants(child.contentView ?? NSView())
                        .first(where: { $0.accessibilityIdentifier() == "pitex.equationPreview" }) {
                        return found
                    }
                }
            }
            return nil
        }
        func previewState() -> PreviewState {
            guard let el = previewElement() else { return (false, "none", "", "", nil) }
            let w = el.window
            let role = (el.accessibilityRole()?.rawValue ?? "?").lowercased()
            let src = (el.accessibilityValue() as? String) ?? ""
            let badge = el.accessibilityHelp() ?? ""
            return (w?.isVisible == true, role, src, badge, w)
        }
        func previewVisible() -> Bool { previewState().visible }
        /// Async poll: the driver suspends between samples so the app's
        /// own run loop keeps running production tasks and timers. With
        /// `want`, `matching` must hold too — the engine keeps the
        /// previous equation's preview up while a new one renders, so
        /// bare visibility would be satisfied by stale content.
        func awaitPreview(_ timeout: Double, want: Bool = true, label: String = "",
                          matching: (PreviewState) -> Bool = { _ in true }) async -> Bool {
            let deadline = ContinuousClock.now + .seconds(timeout)
            var settled = false
            while !settled && ContinuousClock.now < deadline {
                try? await Task.sleep(for: .milliseconds(10))
                let s = previewState()
                settled = want ? s.visible && matching(s) : !s.visible
            }
            if !settled && !label.isEmpty { dumpDiag(label) }
            return settled
        }

        /// Structural dump for a failed preview wait — separates
        /// "popover not found in the view tree" (probe bug) from
        /// "never shown" (engine/render/input path bug).
        func dumpDiag(_ label: String) {
            print("[diag] \(label): appActive=\(NSApp.isActive) key=\(window.isKeyWindow) "
                  + "occlusion=\(window.occlusionState.rawValue) "
                  + "windows=\(NSApp.windows.map { String(describing: type(of: $0)) }.joined(separator: ","))")
            print("[diag] trackingAreas=\(tv()?.trackingAreas.count ?? -1) "
                  + "pointerEventsSeen=\(pointerEvents) cursor=\(NSEvent.mouseLocation) "
                  + "hidBridgeRequests=\(hidRequests) "
                  + "firstResponder=\(String(describing: window.firstResponder))")
            if let el = previewElement() {
                let w = el.window
                print("[diag] previewElement found: window=\(String(describing: w.map { type(of: $0) })) "
                      + "visible=\(w?.isVisible ?? false) role=\(el.accessibilityRole()?.rawValue ?? "?") "
                      + "value=\((el.accessibilityValue() as? String) ?? "") help=\(el.accessibilityHelp() ?? "")")
            } else {
                for w in NSApp.windows { print("[diag]   window \(type(of: w)) children=\(w.childWindows?.count ?? 0)") }
            }
        }
        func escapeFocusSnapshot(_ label: String) {
            guard escapeDiagnostic else { return }
            func identity(_ object: AnyObject?) -> String {
                object.map { "\(type(of: $0))@\(ObjectIdentifier($0))" } ?? "nil"
            }
            let current = tv()
            let responder = window.firstResponder
            let responderView = responder as? NSView
            let preview = previewState()
            HoverDiagnostic.log("escape.snapshot \(label) doc=\(workspace.activeDocumentURL?.lastPathComponent ?? "?")"
                + " appKey=\(identity(NSApp.keyWindow)) initialWindow=\(identity(window)) editorKey=\(window.isKeyWindow)"
                + " workspaceWindow=\(identity(workspace.window)) workspaceResponder=\(identity(workspace.window?.firstResponder))"
                + " initialEditor=\(identity(editor)) currentEditor=\(identity(current)) currentEditorWindow=\(identity(current?.window))"
                + " responder=\(identity(responder)) responderViewWindow=\(identity(responderView?.window))"
                + " responderIsCurrent=\(responder === current) responderIsInitial=\(responder === editor) currentMounted=\(current?.window === window)"
                + " previewVisible=\(preview.visible) previewSource=\(preview.source.prefix(80))")
        }
        func requirePreview(_ timeout: Double, _ label: String, want: Bool = true) async {
            let settled = await awaitPreview(timeout, want: want, label: label)
            if !settled {
                print("FAIL", label)
                fflush(nil); exit(1)
            }
        }
        /// Fixture setup ONLY at the two real document activations, before
        /// any new preview input. Never repair focus after show or Escape.
        func prepareCurrentEditorFocus(_ activation: String) async {
            require(await until(10) { tv()?.window === window },
                    "\(activation): current editor must mount in the workspace window")
            require(!previewVisible(),
                    "\(activation): outgoing preview must already be hidden before focus setup")
            escapeFocusSnapshot("\(activation) before focus setup")
            let current = tv()!
            require(window.makeFirstResponder(current),
                    "\(activation): current editor must accept first responder")
            require(NSApp.keyWindow === window && window.isKeyWindow && window.firstResponder === current,
                    "\(activation): focused current-editor baseline must hold before preview input")
            escapeFocusSnapshot("\(activation) after focus setup")
        }
        /// The same D1 behavior oracle runs after the diagnostic's first
        /// hover and at the full checker's existing D1 stage.
        func requireStablePreviewFocus() async {
            let showTok = NotificationCenter.default.addObserver(
                forName: NSPopover.didShowNotification, object: nil, queue: .main
            ) { _ in MainActor.assumeIsolated { popoverShows += 1 } }
            let closeTok = NotificationCenter.default.addObserver(
                forName: NSPopover.didCloseNotification, object: nil, queue: .main
            ) { _ in MainActor.assumeIsolated { popoverCloses += 1 } }
            defer {
                NotificationCenter.default.removeObserver(showTok)
                NotificationCenter.default.removeObserver(closeTok)
            }
            // The popover was shown just before this call and its show
            // animation may still be running: on macOS 27.0.1 NSPopoverDidShow
            // arrives ~0.5 s after popover.present returned. Absorb exactly
            // THAT pending DidShow before the baselines, without relaxing the
            // oracle: during the wait there must be NO close and at most ONE
            // show (a hide/re-show churn would close first).
            let showsAtEntry = popoverShows
            let closesAtEntry = popoverCloses
            _ = await until(1.0) { popoverShows > showsAtEntry }
            require(popoverCloses == closesAtEntry && popoverShows - showsAtEntry <= 1,
                    "popover must not close or re-show while its show completes: shows=+\(popoverShows - showsAtEntry) closes=+\(popoverCloses - closesAtEntry)")
            // K6: capture BOTH baselines AFTER the popover is visible and its
            // show has completed.
            let showsAtStart = popoverShows
            let closesAtStart = popoverCloses
            for _ in 0..<3 {
                let s = previewState()
                require(s.visible, "popover must stay visible across debounce cycles")
                require(window.isKeyWindow, "editor window must stay key while preview shows")
                require(window.firstResponder === tv()!, "first responder must stay the text view")
                // User-approved public focus oracle: NSPopover's private
                // child proxies key/responder state from its parent.
                require(NSApp.keyWindow === window, "app key window must remain the editor")
                let preview = previewElement()
                require(preview != nil && s.win != nil, "visible preview content and window must be reachable")
                let ownsResponder = (s.win!.firstResponder as? NSView).map {
                    $0 === preview! || $0.isDescendant(of: preview!)
                } ?? false
                require(!ownsResponder, "preview must not own a first responder within its content")
                await pump(0.45)   // > 80ms debounce interval per sample
            }
            require(popoverShows == showsAtStart && popoverCloses == closesAtStart,
                    "no repeated present without input: shows=\(popoverShows) closes=\(popoverCloses)")
            stage("D1: 3 debounce cycles — app key/firstResponder stay editor, preview owns no responder, 0 extra show/close after the show completed")
        }

        // ── Real input through the real loops ────────────────────────────
        func loc(_ needle: String) -> Int {
            guard let tv = tv() else { return NSNotFound }
            return (tv.string as NSString).range(of: needle).location
        }
        func point(atLocation location: Int) -> NSPoint? {
            guard let tv = tv() else { return nil }
            tv.layoutManager?.ensureLayout(for: tv.textContainer!)
            let ns = tv.string as NSString
            guard location >= 0, location < ns.length,
                  let lm = tv.layoutManager, let tc = tv.textContainer else { return nil }
            let glyph = lm.glyphRange(forCharacterRange: NSRange(location: location, length: 1),
                                      actualCharacterRange: nil)
            var r = lm.boundingRect(forGlyphRange: glyph, in: tc)
            r.origin.x += tv.textContainerOrigin.x
            r.origin.y += tv.textContainerOrigin.y
            return NSPoint(x: r.midX, y: r.midY)
        }
        // Pointer input is GENUINE HID: real Quartz mouseMoved events posted
        // at the HID tap move the real cursor, and the window server routes
        // entered/moved events to whichever window owns that screen point,
        // as a physical mouse would. (In the f3 VM run a synthesized
        // NSApp.sendEvent mouseMoved produced no hover preview while the
        // caret pre-flight rendered — the synthetic route is not trusted.)
        // The per-run ad-hoc checker app cannot hold post-event access (b628
        // VM run: denied), so the operator-trusted LAUNCHER posts them: this
        // driver writes one numbered request into the launcher's private
        // per-run directory and waits for its acknowledgement. Scoped: the
        // target must lie inside the editor window and be owned by it on
        // screen here, the launcher re-checks it against this process's
        // on-screen windows, and the real cursor's arrival is verified.
        // Keys keep synchronous NSApp.sendEvent (N5 ordering).
        let bridgeDir = ProcessInfo.processInfo.environment["PITEX_CHECK_HID_BRIDGE"] ?? ""
        func bridgePost(_ at: NSPoint) async {
            require(!bridgeDir.isEmpty, "no HID bridge configured (PITEX_CHECK_HID_BRIDGE): pointer stages cannot run")
            hidRequests += 1
            let name = String(format: "%06d", hidRequests)
            let dir = URL(fileURLWithPath: bridgeDir)
            // Cocoa screen space has its origin at the bottom-left of the
            // primary display (y up); Quartz global space at its top-left
            // (y down). Same points, flipped against the primary height.
            let primaryHeight = NSScreen.screens.first?.frame.height ?? 0
            let request: [String: Any] = [
                "seq": hidRequests, "pid": Int(ProcessInfo.processInfo.processIdentifier),
                "x": Double(at.x), "y": Double(primaryHeight - at.y),
            ]
            do {
                let staged = dir.appendingPathComponent(".req-\(name).tmp")
                try JSONSerialization.data(withJSONObject: request).write(to: staged)
                try FileManager.default.moveItem(at: staged, to: dir.appendingPathComponent("req-\(name).json"))
            } catch {
                require(false, "cannot write HID bridge request \(hidRequests): \(error)")
            }
            let ackURL = dir.appendingPathComponent("ack-\(name).json")
            let deadline = ContinuousClock.now + .seconds(5)
            var ack: [String: Any]?
            while ack == nil {
                if let data = try? Data(contentsOf: ackURL) {
                    ack = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? ["ok": false, "error": "unreadable ack"]
                    try? FileManager.default.removeItem(at: ackURL)
                } else {
                    require(ContinuousClock.now < deadline, "HID bridge never acknowledged request \(hidRequests)")
                    try? await Task.sleep(for: .milliseconds(2))
                }
            }
            require(ack?["ok"] as? Bool == true,
                    "HID bridge refused request \(hidRequests): \(ack?["error"] ?? "?") — no synthesized fallback")
        }
        func movePointer(toWindowPoint p: NSPoint, _ what: String) async {
            let target = window.convertPoint(toScreen: p)
            require(window.frame.contains(target),
                    "pointer target '\(what)' \(target) lies outside the test window \(window.frame)")
            let owner = NSWindow.windowNumber(at: target, belowWindowWithWindowNumber: 0)
            require(owner == window.windowNumber,
                    "pointer target '\(what)' \(target) is covered by window \(owner), not ours \(window.windowNumber)")
            // A zero-distance move may produce no event: step off first.
            if hypot(NSEvent.mouseLocation.x - target.x, NSEvent.mouseLocation.y - target.y) < 1 {
                await bridgePost(NSPoint(x: target.x - 2, y: target.y))
            }
            await bridgePost(target)
            let deadline = ContinuousClock.now + .seconds(3)
            while hypot(NSEvent.mouseLocation.x - target.x, NSEvent.mouseLocation.y - target.y) >= 1 {
                require(ContinuousClock.now < deadline,
                        "real cursor never reached '\(what)': at \(NSEvent.mouseLocation), wanted \(target)")
                try? await Task.sleep(for: .milliseconds(5))
            }
        }
        func hoverChar(_ location: Int) async {
            let p = point(atLocation: location)
            require(p != nil, "hover target \(location) has no glyph on screen")
            let text = tv()!, lm = text.layoutManager!, tc = text.textContainer!
            let wantedGlyphs = lm.glyphRange(forCharacterRange: NSRange(location: location, length: 1),
                                             actualCharacterRange: nil)
            let containerPoint = NSPoint(x: p!.x - text.textContainerOrigin.x,
                                         y: p!.y - text.textContainerOrigin.y)
            let roundtripGlyph = lm.glyphIndex(for: containerPoint, in: tc)
            let windowPoint = text.convert(p!, to: nil)
            let screenPoint = window.convertPoint(toScreen: windowPoint)
            let back = text.convert(window.convertPoint(fromScreen: screenPoint), from: nil)
            let ns = text.string as NSString
            let start = max(0, location - 12)
            let source = ns.substring(with: NSRange(location: start, length: min(60, ns.length - start)))
                .replacingOccurrences(of: "\n", with: "\\n")
            HoverDiagnostic.log("desired hover utf16=\(location) source=\(source) wantedGlyphs=\(wantedGlyphs) roundtripGlyph=\(roundtripGlyph) local=\(p!) window=\(windowPoint) screen=\(screenPoint) roundtripLocal=\(back) inset=\(text.textContainerOrigin) visibleRect=\(text.visibleRect) flipped=\(text.isFlipped)")
            require(NSLocationInRange(roundtripGlyph, wantedGlyphs),
                    "hover glyph roundtrip must reach UTF16 \(location), got glyph \(roundtripGlyph), wanted \(wantedGlyphs)")
            require(hypot(back.x - p!.x, back.y - p!.y) < 0.5,
                    "hover screen/window/local geometry must roundtrip: point=\(p!) back=\(back)")
            await movePointer(toWindowPoint: windowPoint, "char \(location)")
        }
        func hoverPointInText(_ location: Int, dx: CGFloat) async {
            let p = point(atLocation: location)
            require(p != nil, "hover target \(location) has no glyph on screen")
            await movePointer(toWindowPoint: tv()!.convert(NSPoint(x: p!.x + dx, y: p!.y), to: nil),
                              "char \(location) dx \(dx)")
        }
        /// Empty text-view space right of "Intro text line." — reports no
        /// character, and no preview anchors near enough to cover it.
        func restPointer() async {
            let p = point(atLocation: loc("Intro text line"))
            require(p != nil && tv() != nil, "rest line must be on screen")
            await movePointer(toWindowPoint: tv()!.convert(NSPoint(x: tv()!.visibleRect.maxX - 60, y: p!.y), to: nil),
                              "rest (empty text space)")
        }
        func key(_ code: UInt16, chars: String) {
            guard let ev = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [],
                    timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                    context: nil, characters: chars, charactersIgnoringModifiers: chars,
                    isARepeat: false, keyCode: code) else { return }
            NSApp.sendEvent(ev)
        }
        func caret(_ location: Int) { tv()?.setSelectedRange(NSRange(location: location, length: 0)) }
        func pump(_ seconds: Double) async {
            try? await Task.sleep(for: .seconds(seconds))
        }
        /// In-process capture of the preview's own views — no
        /// ScreenCaptureKit, no screen-recording prompt. A fast preview's
        /// pixels live in the WKWebView's web process, where cacheDisplay
        /// sees nothing: snapshot the shown page through the public API.
        /// Exact/message bodies are ordinary AppKit views.
        func rasterize() async -> CGImage? {
            guard let el = previewElement(), el.window != nil else { return nil }
            if let page = descendants(el).compactMap({ $0 as? WKWebView }).first(where: { !$0.isHidden }) {
                let image = try? await page.takeSnapshot(configuration: nil)
                return image?.cgImage(forProposedRect: nil, context: nil, hints: nil)
            }
            guard let rep = el.bitmapImageRepForCachingDisplay(in: el.bounds) else { return nil }
            rep.size = el.bounds.size
            el.cacheDisplay(in: el.bounds, to: rep)
            return rep.cgImage
        }
        func screenshot(_ name: String) async -> String {
            guard let image = await rasterize(),
                  let data = NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:])
            else { return "n/a" }
            let path = "\(shots)/\(name).png"
            try? data.write(to: URL(fileURLWithPath: path))
            return path
        }
        /// Readability of the captured preview. Only opaque pixels count
        /// (translucent ones are excluded and their share reported — a
        /// mostly transparent capture cannot establish contrast).
        /// Background = mean of the dominant luminance bucket; ink =
        /// pixels > 48 levels away from it; foreground = the 90th-percentile
        /// ink by distance (discounts antialiased edges). Contrast is the
        /// WCAG ratio of the two sRGB relative luminances.
        func readability() async -> (bg: Double, fg: Double, contrast: Double, ink: Double, opaque: Double)? {
            guard let image = await rasterize() else { return nil }
            let w = image.width, h = image.height
            // Normalize whatever pixel format the capture used to RGBA8.
            guard w > 4, h > 4,
                  let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                      space: CGColorSpaceCreateDeviceRGB(),
                                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
            else { return nil }
            ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
            guard let px = ctx.data?.bindMemory(to: UInt8.self, capacity: w * h * 4) else { return nil }
            var lums: [Double] = []
            lums.reserveCapacity(w * h)
            var histogram = [Int](repeating: 0, count: 16)
            for i in 0..<(w * h) where px[i * 4 + 3] >= 250 {
                let lum = 0.299 * Double(px[i * 4]) + 0.587 * Double(px[i * 4 + 1]) + 0.114 * Double(px[i * 4 + 2])
                lums.append(lum)
                histogram[min(Int(lum) / 16, 15)] += 1
            }
            guard !lums.isEmpty,
                  let mode = histogram.indices.max(by: { histogram[$0] < histogram[$1] }) else { return nil }
            let bucket = lums.filter { min(Int($0) / 16, 15) == mode }
            let bg = bucket.reduce(0, +) / Double(bucket.count)
            let ink = lums.filter { abs($0 - bg) > 48 }.sorted { abs($0 - bg) < abs($1 - bg) }
            let fg = ink.isEmpty ? bg : ink[Int(Double(ink.count - 1) * 0.9)]
            func linear(_ v: Double) -> Double {
                let c = v / 255
                return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
            }
            let hi = max(linear(bg), linear(fg)), lo = min(linear(bg), linear(fg))
            return (bg, fg, (hi + 0.05) / (lo + 0.05),
                    Double(ink.count) / Double(lums.count), Double(lums.count) / Double(w * h))
        }
        func popoverScreenRect(_ win: NSWindow?) -> CGRect {
            guard let win else { return .null }
            let id = CGWindowID(win.windowNumber)
            let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements],
                                                  kCGNullWindowID) as? [[String: Any]] ?? []
            for entry in list {
                if (entry[kCGWindowNumber as String] as? NSNumber)?.uint32Value == id,
                   let b = entry[kCGWindowBounds as String] as? [String: Any] {
                    return CGRect(x: (b["X"] as? NSNumber)?.doubleValue ?? 0,
                                  y: (b["Y"] as? NSNumber)?.doubleValue ?? 0,
                                  width: (b["Width"] as? NSNumber)?.doubleValue ?? 0,
                                  height: (b["Height"] as? NSNumber)?.doubleValue ?? 0)
                }
            }
            return .null
        }
        func finitePositiveRect(_ rect: CGRect) -> Bool {
            rect.minX.isFinite && rect.minY.isFinite && rect.maxX.isFinite && rect.maxY.isFinite
                && rect.width.isFinite && rect.height.isFinite && rect.width > 0 && rect.height > 0
        }
        /// Current source/setting epoch, measured wholly in Cocoa screen
        /// coordinates (y-up). Invalid observations return no geometry.
        func placementGeometry(_ expected: String) -> (sample: PlacementGeometry?, reason: String) {
            guard let current = tv(), current.window === window,
                  NSApp.keyWindow === window, window.isKeyWindow, window.firstResponder === current else {
                return (nil, "editor/window/key/responder baseline unavailable: editor=\(String(describing: tv().map(ObjectIdentifier.init))) appKey=\(String(describing: NSApp.keyWindow.map(ObjectIdentifier.init))) responder=\(String(describing: window.firstResponder.map(ObjectIdentifier.init)))")
            }
            let setting = SettingsStore.shared.equationPreviewPlacement
            guard setting == expected else { return (nil, "setting epoch is \(setting), expected \(expected)") }
            guard let document = workspace.activeDocumentURL, document.lastPathComponent == "main.tex" else {
                return (nil, "current placement fixture document is not main.tex")
            }
            let state = previewState()
            guard state.visible && fastOf("E = mc")(state),
                  let content = previewElement(), let owner = content.window, owner.isVisible,
                  owner === state.win else {
                return (nil, "current preview unavailable: visible=\(state.visible) role=\(state.role) source=\(state.source) window=\(String(describing: state.win.map(ObjectIdentifier.init)))")
            }
            let sourceRange = (current.string as NSString).range(of: state.source)
            let selection = current.selectedRange()
            guard sourceRange.location != NSNotFound, sourceRange.length > 0, selection.length == 0,
                  NSLocationInRange(selection.location, sourceRange),
                  let lm = current.layoutManager, let tc = current.textContainer else {
                return (nil, "source/caret anchor unavailable: source=\(state.source) range=\(sourceRange) selection=\(selection)")
            }
            lm.ensureLayout(for: tc)
            let glyphs = lm.glyphRange(forCharacterRange: sourceRange, actualCharacterRange: nil)
            var sourceRect = lm.boundingRect(forGlyphRange: glyphs, in: tc)
            sourceRect.origin.x += current.textContainerOrigin.x
            sourceRect.origin.y += current.textContainerOrigin.y
            let visibleAnchor = sourceRect.intersection(current.visibleRect)
            guard finitePositiveRect(content.bounds) && finitePositiveRect(visibleAnchor) else {
                return (nil, "content/visible anchor bounds unavailable: content=\(content.bounds) anchor=\(visibleAnchor)")
            }
            let contentScreen = owner.convertToScreen(content.convert(content.bounds, to: nil))
            let anchorScreen = window.convertToScreen(current.convert(visibleAnchor, to: nil))
            guard finitePositiveRect(contentScreen) && finitePositiveRect(anchorScreen) else {
                return (nil, "finite screen geometry unavailable: content=\(contentScreen) anchor=\(anchorScreen)")
            }
            return (PlacementGeometry(
                editorID: ObjectIdentifier(current), contentID: ObjectIdentifier(content),
                previewWindowID: ObjectIdentifier(owner), previewWindowNumber: owner.windowNumber,
                documentID: document.path, source: state.source, setting: setting, sourceRange: sourceRange,
                content: contentScreen, anchor: anchorScreen
            ), "finite current geometry")
        }
        func awaitPlacementGeometry(_ expected: String) async -> PlacementGeometry {
            let deadline = ContinuousClock.now + .seconds(12)
            var previous: PlacementGeometry?
            var lastObservation = ""
            while ContinuousClock.now < deadline {
                let observed = placementGeometry(expected)
                let description = observed.sample.map { "\($0)" } ?? observed.reason
                if description != lastObservation {
                    note("placement sample \(expected), Cocoa y-up: \(description)")
                    lastObservation = description
                }
                if let sample = observed.sample {
                    let correctSide = expected == "above"
                        ? sample.content.minY >= sample.anchor.midY
                        : sample.content.maxY <= sample.anchor.midY
                    if correctSide && previous == sample {
                        note("placement stable \(expected): two consecutive finite source/setting/window/rect samples, Cocoa y-up: \(sample)")
                        return sample
                    }
                    previous = correctSide ? sample : nil
                } else {
                    previous = nil
                }
                try? await Task.sleep(for: .milliseconds(20))
            }
            dumpDiag("\(expected) placement geometry")
            print("FAIL \(expected) placement needs finite, current, stable geometry on the requested side; last=\(lastObservation)")
            fflush(nil)
            exit(1)
        }
        /// The popover's screen-relative height cap: the SAME formula as
        /// EquationPreviewPopover.maximumHeight(for:), min(max(360, 55 % of
        /// the screen's visible height), 720) pt. ONE value for the whole
        /// file. It is fractional on some displays (577.5 pt at 1050 pt), so
        /// every comparison with native geometry allows < 1 pt (the popover
        /// window rounds, the page's innerWidth/innerHeight are integers).
        func previewHeightCap() -> CGFloat {
            min(max(360, (window.screen?.visibleFrame.height ?? 720) * 0.55), 720)
        }
        /// Public AX-identified content includes the native badge, unlike
        /// the WebKit body. Window chrome is reported separately.
        func previewContentBounds(_ label: String) -> CGRect {
            let element = previewElement()
            require(element != nil && element!.window?.isVisible == true,
                    "\(label): visible preview content must be reachable")
            let bounds = element!.bounds
            let heightCap = previewHeightCap()
            require(bounds.width > 10 && bounds.height > 10 && bounds.width <= 720 && bounds.height < heightCap + 1,
                    "\(label): whole preview content including badge must fit the 720 pt width cap and the screen-relative height cap \(heightCap) pt (+ < 1 pt window rounding), got \(bounds)")
            return bounds
        }
        func visibleFastPage(_ label: String) -> WKWebView {
            let page = previewElement().flatMap {
                descendants($0).compactMap { $0 as? WKWebView }.first { !$0.isHidden && $0.window?.isVisible == true }
            }
            require(page != nil, "\(label): visible MathJax page must be reachable")
            return page!
        }
        /// Measure the actual displayed SVG and live browser viewport,
        /// not production size constants or source-length heuristics.
        func fastPreviewMetrics(_ label: String) async -> (
            svgHeight: Double, displayHeight: Double, viewportWidth: Double,
            viewportHeight: Double, scrollWidth: Double, scrollHeight: Double,
            clientWidth: Double, clientHeight: Double, bodyClientWidth: Double,
            bodyScrollWidth: Double, bodyIsScrollContainer: Double
        ) {
            let page = visibleFastPage(label)
            let deadline = ContinuousClock.now + .seconds(5)
            while true {
                let reply = try? await page.callAsyncJavaScript("""
                    const display = document.getElementById('display');
                    const svg = display && display.querySelector('svg');
                    if (!svg || !document.scrollingElement) throw new Error('rendered SVG missing');
                    const body = document.body, root = document.documentElement;
                    const scrolls = (v) => v === 'auto' || v === 'scroll';
                    const cs = getComputedStyle(body);
                    return JSON.stringify({
                        svgHeight: svg.getBoundingClientRect().height,
                        displayHeight: display.getBoundingClientRect().height,
                        viewportWidth: window.innerWidth, viewportHeight: window.innerHeight,
                        scrollWidth: document.scrollingElement.scrollWidth,
                        scrollHeight: document.scrollingElement.scrollHeight,
                        clientWidth: root.clientWidth, clientHeight: root.clientHeight,
                        bodyClientWidth: body.clientWidth, bodyScrollWidth: body.scrollWidth,
                        bodyIsScrollContainer: (scrolls(cs.overflowX) || scrolls(cs.overflowY)) ? 1 : 0
                    });
                    """, arguments: [:], in: nil, contentWorld: .page) as? String
                let data = reply.map { Data($0.utf8) }
                let metrics = data.flatMap { (try? JSONSerialization.jsonObject(with: $0)) as? [String: NSNumber] }
                let fields = ["svgHeight", "displayHeight", "viewportWidth", "viewportHeight", "scrollWidth", "scrollHeight",
                              "clientWidth", "clientHeight", "bodyClientWidth", "bodyScrollWidth", "bodyIsScrollContainer"]
                require(metrics != nil && fields.allSatisfy { metrics?[$0]?.doubleValue.isFinite == true },
                        "\(label): live SVG/viewport metrics unavailable, reply=\(String(describing: reply))")
                let width = metrics!["viewportWidth"]!.doubleValue
                let height = metrics!["viewportHeight"]!.doubleValue
                // WebKit applies native frame changes asynchronously.
                // Await that actual geometry, not a fixed settling delay.
                // < 1 pt: innerWidth/innerHeight are integers, the native
                // frame can be fractional (the screen-relative height cap).
                if abs(width - Double(page.bounds.width)) < 1 && abs(height - Double(page.bounds.height)) < 1 {
                    return (metrics!["svgHeight"]!.doubleValue, metrics!["displayHeight"]!.doubleValue,
                            width, height, metrics!["scrollWidth"]!.doubleValue, metrics!["scrollHeight"]!.doubleValue,
                            metrics!["clientWidth"]!.doubleValue, metrics!["clientHeight"]!.doubleValue,
                            metrics!["bodyClientWidth"]!.doubleValue, metrics!["bodyScrollWidth"]!.doubleValue,
                            metrics!["bodyIsScrollContainer"]!.doubleValue)
                }
                require(ContinuousClock.now < deadline,
                        "\(label): browser viewport did not settle to native bounds \(page.bounds), reply=\(String(describing: reply))")
                try? await Task.sleep(for: .milliseconds(10))
            }
        }
        func scrollFastPreview(to y: Double, label: String) async -> Double {
            let page = visibleFastPage(label)
            let reply = try? await page.callAsyncJavaScript(
                "window.scrollTo(0, y); return String(window.scrollY);",
                arguments: ["y": y], in: nil, contentWorld: .page
            ) as? String
            let position = reply.flatMap { Double($0) }
            require(position?.isFinite == true, "\(label): browser scroll position unavailable")
            return position!
        }
        func probe(_ name: String) -> Bool {
            // Discover TeX from the actual environment PATH plus the
            // canonical MacTeX symlink dir — never a pinned year/arch path.
            let paths = (ProcessInfo.processInfo.environment["PATH"] ?? "/usr/bin:/bin")
                .split(separator: ":").map(String.init)
                + ["/Library/TeX/texbin"]
            return paths.contains { FileManager.default.isExecutableFile(atPath: "\($0)/\(name)") }
        }
        func fakeBecomeActive() {
            // refreshExternalFiles subscribes on NotificationCenter.default;
            // post to both centers so every production activation observer
            // sees the same event the OS would have sent.
            let name = NSApplication.didBecomeActiveNotification
            NotificationCenter.default.post(name: name, object: NSApp)
            NSWorkspace.shared.notificationCenter.post(name: name, object: NSApp)
        }
        // The preview's badge is the AX help string, i.e. the app's own LOCALIZED text
        // (equation_preview.fast_badge / equation_preview.exact_badge). This checker app bundles
        // the production .lproj tables and runs in the same process as the code under test, so
        // String(localized:) here resolves exactly like the product's present(label:) calls.
        // Never compare against English text: on a Korean host the exact badge reads '정확한 TeX'.
        let fastBadge = String(localized: "equation_preview.fast_badge")
        let exactBadge = String(localized: "equation_preview.exact_badge")
        require(fastBadge != "equation_preview.fast_badge" && exactBadge != "equation_preview.exact_badge"
                && !fastBadge.isEmpty && !exactBadge.isEmpty && fastBadge != exactBadge,
                "badge strings must resolve from the bundled tables and differ: fast='\(fastBadge)' exact='\(exactBadge)'")
        note("badge strings under test: fast='\(fastBadge)' exact='\(exactBadge)'")
        /// A rendered fast (MathJax) body for the equation containing
        /// `needle` — the fresh target, not whatever preview lingers.
        func fastOf(_ needle: String) -> (PreviewState) -> Bool {
            { $0.role.contains("image") && $0.source.contains(needle) && $0.badge == fastBadge }
        }
        func exactOf(_ needle: String) -> (PreviewState) -> Bool {
            { $0.role.contains("image") && $0.source.contains(needle) && $0.badge == exactBadge }
        }
        /// SettingsStore/AppearanceSettings reach the controller through
        /// objectWillChange.receive(on: main) — one FIFO main-queue hop
        /// queued by the setter. Suspending lets that hop run before the
        /// next injected input.
        func settle() async { await pump(0.2) }

        texAvailable = ["xelatex", "pdflatex", "lualatex", "latexmk"].contains { probe($0) }
        note("tex executables present=\(texAvailable) buildCommand=\(workspace.buildCommandText)")
        // Count pointer events the app actually receives (diagnostic only).
        _ = NSEvent.addLocalMonitorForEvents(matching: [.mouseMoved, .mouseEntered, .mouseExited]) { event in
            let kind = event.type.rawValue, number = event.windowNumber
            let point = event.locationInWindow
            let associatedWindow = event.window.map(ObjectIdentifier.init)
            let cgPoint = event.cgEvent?.location
            MainActor.assumeIsolated {
                Driver.pointerEvents += 1
                HoverDiagnostic.log("pointer event type=\(kind) window=\(number) associatedWindow=\(String(describing: associatedWindow)) at=\(point) cgGlobal=\(String(describing: cgPoint)) cursorGlobal=\(NSEvent.mouseLocation) publicWindowCursor=\(window.mouseLocationOutsideOfEventStream) appKey=\(NSApp.keyWindow?.windowNumber ?? -1) editorWindowIdentity=\(ObjectIdentifier(window))")
            }
            return event
        }

        let store = SettingsStore.shared
        store.equationPreviewEnabled = true
        store.equationPreviewWhileTyping = true
        store.equationPreviewPlacement = "above"
        store.equationPreviewRenderer = "fast"
        store.equationPreviewDelayMilliseconds = 80

        // Geometry precondition: every hover target and caret anchor must be
        // on screen, or hover/negative/hang stages would pass vacuously.
        await pump(0.3)
        for needle in ["E = mc^2", "\\frac{1}{3}", "a &= b + c", "\\R + \\C", "\\binom{n}{k}",
                       "\\$ dollars", "% $x^2$ comment", "z = 99", "End plain text line",
                       "\\sum_{k=1}^{n} k", "r_1 &= 1"] {
            let at = loc(needle)
            require(at != NSNotFound, "fixture needle '\(needle)' missing")
            let p = point(atLocation: at)
            require(p.map { editor.visibleRect.contains($0) } ?? false,
                    "fixture target '\(needle)' must be on screen: at=\(String(describing: p)) "
                    + "visible=\(editor.visibleRect) window=\(window.frame)")
        }
        stage("geometry: all fixture targets on screen, window=\(window.frame)")

        // Park the caret in plain text.
        let park = loc("End plain text line") + 5
        caret(park)
        await pump(0.3)
        require(!previewVisible(), "no preview before any trigger")
        stage("baseline: caret parked outside math")

        // ── PRE-FLIGHT: caret path first — no pointer involved. Separates
        // a render/popover pipeline failure from pointer-event delivery.
        let preflightStart = ContinuousClock.now
        caret(loc("a &= b + c") + 2)
        await requirePreview(30, "caret inside align must preview (pre-flight)")
        // Page load + MathJax startup + first typeset: the true cold path.
        let preflightMs = elapsedMs(preflightStart)
        var st = previewState()
        require(st.role.contains("image"),
                "caret preview must render content, got role=\(st.role) src='\(st.source)'")
        require(st.source.contains("align"),
                "caret preview must anchor the align block, got '\(st.source)'")
        stage("pre-flight caret preview on align: \(preflightMs)ms (page load + first render) '\(st.source.prefix(30))'")
        caret(park)
        await requirePreview(8, "caret leaving math must hide", want: false)
        stage("caret exit hides")
        // Pointer stages start only now, so a HID bridge refusal (e.g. no
        // post-event access) still leaves the caret/renderer proof above.
        // First move: a known neutral spot inside our window.
        await restPointer()
        stage("pointer at rest via HID bridge: cursor=\(NSEvent.mouseLocation) "
              + "requests=\(hidRequests) pointerEventsSeen=\(pointerEvents)")
        if ProcessInfo.processInfo.environment["PITEX_CHECK_HID_PREFLIGHT"] == "1" {
            // Preflight success = the real cursor arrived (verified above)
            // AND genuine pointer events reached this app's event stream.
            let delivered = await until(2) { pointerEvents > 0 }
            require(delivered, "HID preflight: the cursor moved but no pointer event reached the checker app")
            print("HID_PREFLIGHT_OK requests=\(hidRequests) pointerEventsSeen=\(pointerEvents) "
                  + "cursor=\(NSEvent.mouseLocation) window=\(window.frame)")
            fflush(nil)
            exit(0)
        }

        // ── COLD: hover $E = mc^2$ — first hover render + show ───────────
        let coldStart = ContinuousClock.now
        await hoverChar(loc("E = mc^2") + 2)
        await requirePreview(30, "hover over inline math must show a preview")
        let coldMs = elapsedMs(coldStart)
        st = previewState()
        require(st.role.contains("image"), "cold preview must be rendered content, got role=\(st.role)")
        require(st.source.contains("mc^2") || st.source.contains("E = mc"),
                "preview AX value must carry the hovered source, got '\(st.source)'")
        require(!st.badge.isEmpty, "fast badge expected on rendered preview")
        require(tv()!.selectedRange().location == park && tv()!.selectedRange().length == 0,
                "hover must not move the caret: \(tv()!.selectedRange())")
        require(window.firstResponder === tv()!, "hover must not steal first responder")
        let coldShot = await screenshot("01-cold-inline")
        stage("cold hover inline: \(coldMs)ms role=\(st.role) badge='\(st.badge)' shot=\(coldShot)")
        if HoverDiagnostic.enabled && !escapeDiagnostic {
            // Targeted proof includes first-hover identity and the same D1
            // focus/no-churn oracle. Remaining full-mode assertions stay below.
            await requireStablePreviewFocus()
            print("HOVER_DIAGNOSTIC_OK")
            fflush(nil)
            exit(0)
        }
        // ── FRESH: hover the display math — a different render ───────────
        let freshStart = ContinuousClock.now
        await hoverChar(loc("\\frac{1}{3}") + 2)
        require(await awaitPreview(15, matching: fastOf("frac{1}{3}")),
                "hover over display math must show a preview")
        let freshMs = elapsedMs(freshStart)
        st = previewState()
        require(st.source.contains("\\int") || st.source.contains("frac"),
                "display preview must carry its own source, got '\(st.source)'")
        stage("fresh hover display: \(freshMs)ms source='\(st.source.prefix(40))'")

        // ── CACHE: back to the first equation — LRU hit path ─────────────
        // With a real cursor the display preview (placed above its line)
        // covers E = mc, so rest the pointer off math and start the cache
        // hit from a hidden state instead of hovering through a popover.
        await restPointer()
        require(await awaitPreview(5, want: false), "resting the pointer off math must hide the display preview")
        let cacheStart = ContinuousClock.now
        await hoverChar(loc("E = mc^2") + 2)
        require(await awaitPreview(10, matching: fastOf("mc^2")),
                "re-hovering the first equation must show again")
        let cacheMs = elapsedMs(cacheStart)
        st = previewState()
        require(st.source.contains("mc^2"), "cache preview must show first source, got '\(st.source)'")
        stage("cache hover inline: \(cacheMs)ms")

        // ── Hover-exit grace: pointer inside the view but outside math ───
        await hoverPointInText(loc("End plain text line") + 2, dx: 60)
        let exitStart = ContinuousClock.now
        require(await awaitPreview(5, want: false),
                "pointer leaving math must hide (caret is outside math)")
        stage("hover exit hide: \(elapsedMs(exitStart))ms (grace <= 250ms)")

        // ── CARET trigger: into the align block ──────────────────────────
        caret(loc("a &= b + c") + 2)
        require(await awaitPreview(15), "caret inside align must preview")
        st = previewState()
        require(st.source.contains("align"),
                "caret preview must anchor the align block, got '\(st.source)'")
        stage("caret preview on align: source='\(st.source.prefix(30))'")
        caret(park)
        require(await awaitPreview(8, want: false), "caret leaving math must hide")
        stage("caret exit hides")

        // ── Negatives: \$, comment, verbatim — hover the would-be math
        // character itself (the geometry precondition proved it on screen).
        for (label, needle, offset) in [
            ("escaped dollar", "\\$ dollars", 1),
            ("comment", "% $x^2$ comment", 3),
            ("verbatim", "z = 99", 0),
        ] {
            let at = loc(needle)
            require(at != NSNotFound, "negative fixture '\(label)' must exist")
            await hoverChar(at + offset)
            await pump(0.5)
            require(!previewVisible(), "\(label) must not produce a preview")
        }
        stage("escaped-dollar/comment/verbatim produce no preview")

        // ── Edit invalidation: type inside the equation ──────────────────
        await hoverChar(loc("E = mc^2") + 2)
        require(await awaitPreview(10, matching: fastOf("mc^2")), "re-hover before edit must show")
        let beforeEditSource = previewState().source
        caret(loc("E = mc^2") + 7)
        tv()!.insertText("3", replacementRange: tv()!.selectedRange())
        await pump(0.4)
        caret(park)
        // Start the post-edit hover from a hidden state: the caret-origin
        // edited preview sits above this same line, and the real cursor
        // must re-enter the equation to hover it again.
        require(await awaitPreview(3, want: false), "parked caret must hide the caret-origin edited preview")
        let edited = loc("E = mc3^2") != NSNotFound ? loc("E = mc3^2") + 2 : loc("E = mc") + 2
        await hoverChar(edited)
        require(await awaitPreview(15, matching: { fastOf("3")($0) && $0.source != beforeEditSource }),
                "editing inside math must re-render, not show stale")
        st = previewState()
        require(st.source != beforeEditSource || st.source.contains("3"),
                "post-edit preview must show the edited source, got '\(st.source)'")
        stage("edit invalidates preview -> '\(st.source.prefix(40))'")

        // ── Macro context: \R + \C defined in \input{macros} ─────────────
        await hoverChar(loc("\\R + \\C") + 1)
        require(await awaitPreview(15, matching: fastOf("\\R + \\C")),
                "macro-defined equation must preview")
        st = previewState()
        require(st.role.contains("image"), "macro preview must render via imported definitions")
        stage("imported macros render: '\(st.source)'")
        _ = await screenshot("02-macro-good")

        // Redefine \R to an unknown command on disk -> activation refresh
        // must invalidate the imported context: the SAME visible preview
        // turns 'unavailable' in place. The real cursor stays on the
        // equation — no new pointer input — so the wait itself observes
        // the refresh landing.
        try? "\\newcommand{\\R}{\\nonexistentmacroxyz}\n\\newcommand{\\C}{\\mathbb{C}}\n"
            .write(toFile: root + "/macros.tex", atomically: true, encoding: .utf8)
        fakeBecomeActive()
        await pump(0.8)
        require(await awaitPreview(15, matching: { $0.role.contains("statictext") && $0.source.contains("\\R + \\C") }),
                "hovered macro preview must re-resolve after redefinition")
        st = previewState()
        require(st.role.contains("statictext"),
                "broken macro redefinition must surface 'unavailable', got role=\(st.role) src='\(st.source)'")
        stage("macro redefinition -> unavailable (real invalidation)")
        try? "\\newcommand{\\R}{\\mathbb{R}}\n\\newcommand{\\C}{\\mathbb{C}}\n"
            .write(toFile: root + "/macros.tex", atomically: true, encoding: .utf8)
        fakeBecomeActive()
        await pump(0.8)
        require(await awaitPreview(15, matching: fastOf("\\R + \\C")), "restored macro must preview again")
        require(previewState().role.contains("image"), "restored macro must render again")
        stage("macro restore -> rendered")

        // ── A -> B document switch: late results target B only ───────────
        escapeFocusSnapshot("before switch")
        await workspace.activateDocument(URL(fileURLWithPath: root + "/other.tex"))
        escapeFocusSnapshot("activation returned")
        require(await until(15) {
            workspace.activeDocumentURL?.lastPathComponent == "other.tex"
                && ((tv()?.string ?? "").contains("x_{B}"))
        }, "document switch to other.tex must complete")
        await pump(0.3)
        await prepareCurrentEditorFocus("other.tex activation")
        escapeFocusSnapshot("B mounted before hover")
        stage("switched to other.tex")
        await hoverChar(loc("x_{B}") + 1)
        require(await awaitPreview(15, matching: fastOf("_{B}")), "hover in B must preview B's equation")
        st = previewState()
        require(st.source.contains("x_{B}") || st.source.contains("_{B}"),
                "preview after A->B must show B's source only, got '\(st.source)'")
        require(!st.source.contains("mc"), "late A results must not appear on B")
        stage("A->B: preview shows only B source '\(st.source)'")
        escapeFocusSnapshot("B preview before Escape")

        // ── Escape hides; focus stays with the editor ────────────────────
        key(53, chars: "\u{1B}")
        escapeFocusSnapshot("immediate post-key")
        require(await awaitPreview(5, want: false), "Escape must hide the preview")
        escapeFocusSnapshot("after preview hidden")
        require(window.firstResponder === tv()!, "Escape must not move first responder")
        stage("Escape hides; editor keeps focus")
        if escapeDiagnostic {
            print("ESCAPE_DIAGNOSTIC_OK")
            fflush(nil)
            exit(0)
        }

        // ── D3: Escape with NO visible preview must not be consumed by the
        // preview layer — the event must reach the editor normally. Assert:
        // typed '!' lands, selection advances exactly 1, no preview appears.
        caret(loc("Other document") + 2)
        let selBefore = tv()!.selectedRange()
        key(53, chars: "\u{1B}")
        await pump(0.5)
        require(!previewVisible(), "no preview during Escape-precedence probe")
        require(window.firstResponder === tv()!, "Escape with no preview must keep first responder")
        tv()?.insertText("!", replacementRange: tv()!.selectedRange())
        await pump(0.2)
        require((tv()?.string ?? "").contains("!"),
                "typed '!' must land in the buffer after Escape")
        require(tv()!.selectedRange().location == selBefore.location + 1,
                "selection must advance exactly one char after typing")
        require(!previewVisible(), "typing after Escape must not conjure a preview")
        stage("D3: Escape un-consumed; '!' inserted, selection +1, no preview")

        // ── Back to main.tex: the pending-Escape and focus stages need
        // offsets (park, targets) that belong to the ACTIVE document.
        await workspace.activateDocument(URL(fileURLWithPath: root + "/main.tex"))
        require(await until(15) {
            workspace.activeDocumentURL?.lastPathComponent == "main.tex"
                && (tv()?.string ?? "").contains("E = mc")
        }, "switch to main.tex for pending-Escape/focus stages")
        await pump(0.3)
        await prepareCurrentEditorFocus("main.tex activation")
        // The cursor still sits where B's equation was; in main.tex that
        // point may lie under a later preview. Park it neutral again.
        await restPointer()

        // ── Pending-Escape (K2, current e8 semantics): a cached CURRENT
        // snapshot makes eligible pending CONSUMED even before the debounce
        // fires. Sequence: render A to prime currentSnapshot, park on
        // non-math without editing, caret into B, then a synchronous
        // Escape in the same main-actor turn — before any fire can run —
        // and the preview must stay hidden.
        caret(loc("E = mc") + 2)
        require(await awaitPreview(12, matching: fastOf("E = mc")),
                "must render A first — primes the current snapshot")
        caret(park)
        require(await awaitPreview(3, want: false), "parked caret must hide the preview first")
        caret(loc("\\frac{1}{3}") + 2)   // inside display math B: arms the debounce
        key(53, chars: "\u{1B}")          // synchronous Escape before the fire
        await pump(0.6)
        require(!previewVisible(),
                "current-snapshot pending must be consumed by Escape — stays hidden")
        stage("D3-pending: Escape before fire consumes eligible pending (snapshot current)")
        // Stale-snapshot pass-through is not asserted: Escape reaching the
        // editor produces no observable buffer change to verify.

        // ── Focus loss: app resigns key; return restores caret preview ───
        // K8: the pending-Escape probe dismissed B and left the caret in
        // it — the same selection cannot re-arm. Move to E = mc instead.
        caret(loc("E = mc") + 2)
        require(await awaitPreview(12, matching: fastOf("E = mc")),
                "caret must re-arm a preview before focus test")
        NSApp.hide(nil)
        _ = await until(5) { !window.isKeyWindow }
        require(await awaitPreview(5, want: false), "focus loss must hide the preview")
        NSApp.unhide(nil)
        NSApp.activate(ignoringOtherApps: true)
        require(await until(10) { window.isKeyWindow }, "unhide must restore key")
        require(await awaitPreview(12, matching: fastOf("E = mc")),
                "caret target must resume preview when focus returns")
        stage("focus loss hides; return restores caret preview")

        // ── Scroll + bounds on the 38-row aligned block in main.tex ──────
        await workspace.activateDocument(URL(fileURLWithPath: root + "/main.tex"))
        require(await until(15) { workspace.activeDocumentURL?.lastPathComponent == "main.tex" },
                "switch back to main.tex")
        await pump(0.3)
        let bigLoc = loc("r_1 &= 1")
        require(bigLoc != NSNotFound, "large align fixture missing")
        caret(bigLoc + 3)
        require(await awaitPreview(15, matching: fastOf("r_1 &= 1")), "large aligned block must preview")
        let pop = previewState()
        require(pop.role.contains("image") && pop.source.contains("\\begin{align}") && pop.source.contains("r_39 &= 39"),
                "large preview must render the complete 39-row align source, got role=\(pop.role) source='\(pop.source)'")
        // The popover's screen-relative cap (fraction of the host screen's
        // visible height, hard-capped): the one shared value defined with
        // previewContentBounds, not a hard-coded constant.
        let maxPreviewHeight = previewHeightCap()
        let badgeH: Double = 16
        let largeMetrics = await fastPreviewMetrics("large align")
        let largeContentBounds = previewContentBounds("large align")
        require(abs(largeContentBounds.height - maxPreviewHeight) < 1,
                "oversized 39-row fixture must clamp the whole content to the screen-relative cap \(maxPreviewHeight) (< 1 pt window rounding), got \(largeContentBounds)")
        require(largeMetrics.svgHeight > (maxPreviewHeight - badgeH)
                && largeMetrics.displayHeight > largeMetrics.viewportHeight
                && largeMetrics.viewportWidth > 0 && largeMetrics.viewportHeight > 0
                && largeMetrics.viewportHeight <= Double(largeContentBounds.height)
                && largeMetrics.scrollHeight > largeMetrics.viewportHeight
                && largeMetrics.scrollWidth <= largeMetrics.clientWidth
                && largeMetrics.bodyScrollWidth <= largeMetrics.bodyClientWidth
                && largeMetrics.bodyIsScrollContainer == 0,
                "large fixture must contain a real oversized SVG in a scrollable clamped viewport (html scrolls vertically; neither html nor body overflows horizontally), got \(largeMetrics)")
        let outerBounds = popoverScreenRect(pop.win)
        stage("large align content=\(largeContentBounds) (badge included); outer CG frame=\(outerBounds); live SVG/viewport=\(largeMetrics)")
        let scrolledY = await scrollFastPreview(to: largeMetrics.scrollHeight, label: "large align scroll")
        require(scrolledY > 0, "large rendered preview must actually scroll within its viewport")
        let largeScrollShot = await screenshot("03-large-scrolled")
        require(largeScrollShot != "n/a", "scrolled large preview must be capturable")
        let restoredY = await scrollFastPreview(to: 0, label: "large align restore")
        require(restoredY == 0, "large preview must restore its top scroll position")
        stage("large preview internal scroll=\(scrolledY) restored=\(restoredY) shot=\(largeScrollShot)")
        // Scroll to the document end so the WHOLE anchor leaves the visible
        // rect (the fixture's filler after the block guarantees the room;
        // the target is clamped explicitly rather than relying on the clip
        // view), then back to the saved origin.
        let editorScroll = descendants(window.contentView!).compactMap { $0 as? NSScrollView }
            .first { $0.documentView === tv() }
        require(editorScroll != nil, "editor scroll view must be reachable for the scroll stage")
        let clip = editorScroll!.contentView
        let savedOrigin = clip.bounds.origin
        let bottom = max(0, (tv()?.frame.height ?? 0) - clip.bounds.height)
        require(bottom > savedOrigin.y + 100, "document must scroll past the large block, bottom=\(bottom)")
        clip.scroll(to: NSPoint(x: savedOrigin.x, y: bottom))
        editorScroll!.reflectScrolledClipView(clip)
        await pump(0.3)
        require(await awaitPreview(6, want: false), "scrolled-out anchor must hide popover")
        clip.scroll(to: savedOrigin)
        editorScroll!.reflectScrolledClipView(clip)
        require(await awaitPreview(12, matching: fastOf("r_1 &= 1")),
                "scrolled-back anchor must re-show the popover")
        stage("scroll out hides, scroll back re-anchors")

        // ── Settings round-trip on the real store ────────────────────────
        store.equationPreviewEnabled = false
        require(await awaitPreview(6, want: false), "disabling must retire any visible preview")
        caret(loc("E = mc") + 2)   // a fresh math target while disabled
        await pump(0.6)
        require(!previewVisible(), "disabled feature must never preview")
        store.equationPreviewEnabled = true
        require(await awaitPreview(12, matching: fastOf("E = mc")),
                "re-enabling must resume the eligible caret preview")
        stage("disable/enable round-trip honored")
        let smallMetrics = await fastPreviewMetrics("small preview after large")
        let smallContentBounds = previewContentBounds("small preview after large")
        require(smallContentBounds.height < largeContentBounds.height,
                "small equation must shrink the previously height-clamped content: large=\(largeContentBounds) small=\(smallContentBounds)")
        require(smallMetrics.svgHeight > 0 && smallMetrics.viewportWidth > 0 && smallMetrics.viewportHeight > 0
                && smallMetrics.viewportHeight <= Double(smallContentBounds.height)
                && smallMetrics.scrollWidth <= smallMetrics.viewportWidth
                && smallMetrics.scrollHeight <= smallMetrics.viewportHeight,
                "small equation must render without retaining large-expression overflow, got \(smallMetrics)")
        stage("large-to-small shrink content=\(smallContentBounds); outer CG frame=\(popoverScreenRect(previewState().win)); live SVG/viewport=\(smallMetrics)")
        // ── C18: many-short-row aligns must preview with no horizontal
        // scroller at html OR body, and with no vertical scroller while they
        // fit under the cap. With classic scrollers the old CSS made <body> a
        // scroll container too: html's vertical scroller narrowed body, body
        // grew a horizontal one, whose height kept html's vertical one on
        // (both scrollbars for a formula that fits). The html-only check
        // above cannot see that latch, so body is asserted explicitly.
        for (rows, prefix) in [(5, "a"), (20, "b"), (60, "c")] {
            let needle = "\(prefix)_\(rows) &= \(rows)"
            let nl = loc(needle)
            require(nl != NSNotFound, "short-row fixture '\(needle)' missing")
            tv()!.scrollRangeToVisible(NSRange(location: nl, length: 0))
            await pump(0.3)
            caret(nl + 3)
            require(await awaitPreview(15, matching: fastOf("\(prefix)_1 &= 1")),
                    "\(rows)-row align must preview")
            let m = await fastPreviewMetrics("align \(rows)-row")
            require(m.bodyIsScrollContainer == 0,
                    "\(rows)-row align: body must not be a scroll container (only html scrolls), got \(m)")
            require(m.scrollWidth <= m.clientWidth && m.bodyScrollWidth <= m.bodyClientWidth,
                    "\(rows)-row align must not overflow horizontally at html or body, got \(m)")
            if m.displayHeight <= maxPreviewHeight - badgeH {
                require(m.scrollHeight <= m.clientHeight,
                        "\(rows)-row align fits under the \(maxPreviewHeight)pt cap and must not scroll vertically, got \(m)")
            }
            stage("align \(rows)-row: html scroll/client=\(m.scrollWidth)/\(m.clientWidth) x \(m.scrollHeight)/\(m.clientHeight) body scroll/client w=\(m.bodyScrollWidth)/\(m.bodyClientWidth) bodyScrolls=\(m.bodyIsScrollContainer)")
        }
        // Later stages hover/caret targets in the first ~20 lines: bring the
        // editor back to the origin saved before the scroll stage.
        clip.scroll(to: savedOrigin)
        editorScroll!.reflectScrolledClipView(clip)
        await pump(0.3)

        // whileTyping=false: typing suppresses; navigation resumes. Park
        // first so nothing visible can satisfy the negative, and let the
        // store hop land before the edit.
        caret(park)
        require(await awaitPreview(3, want: false), "parked caret must hide before the whileTyping probe")
        store.equationPreviewWhileTyping = false
        await settle()
        let wtLoc = loc("E = mc") + 7
        caret(wtLoc)
        tv()!.insertText("7", replacementRange: tv()!.selectedRange())
        await pump(0.5)
        require(!previewVisible(), "typing with whileTyping off must not preview")
        caret(wtLoc + 2)
        caret(wtLoc + 1)
        require(await awaitPreview(12, matching: fastOf("7")),
                "navigating (not typing) with whileTyping off must still preview")
        store.equationPreviewWhileTyping = true
        await settle()
        stage("whileTyping=false suppresses typing previews; navigation still works")

        // delay=150: measured from a hidden state, after the setting landed.
        caret(park)
        require(await awaitPreview(3, want: false), "parked caret must hide before the delay probe")
        store.equationPreviewDelayMilliseconds = 150
        await settle()
        let delayStart = ContinuousClock.now
        await hoverChar(loc("E = mc") + 2)
        require(await awaitPreview(15, matching: fastOf("E = mc")), "delay=150 must still preview")
        let delayMs = elapsedMs(delayStart)
        require(delayMs >= 140, "delay=150 honored (measured \(delayMs)ms)")
        store.equationPreviewDelayMilliseconds = 80
        await settle()
        stage("delay=150 measured \(delayMs)ms")
        // Pointer back onto plain text: later caret stages must not be
        // overridden by a hover target resting on E = mc.
        await hoverPointInText(loc("End plain text line") + 2, dx: 60)
        require(await awaitPreview(5, want: false), "pointer leaving math must hide after the delay probe")
        // Park the real cursor where no later preview (e.g. the sum's,
        // placed above line 19) can appear underneath it.
        await restPointer()

        // Placement below, changed the way a user changes it: through the
        // real settings sheet. The sheet takes key, so the editor loses
        // focus and the preview retires; on return the preview presents
        // afresh on the new edge. (A popover already on screen keeps its
        // edge on Mac — no live re-edge is claimed here.)
        func throughSettingsSheet(_ change: () -> Void) async {
            workspace.showingSettings = true
            require(await until(10) { window.attachedSheet?.isVisible == true }, "settings sheet must attach")
            require(await awaitPreview(5, want: false, label: "settings sheet focus"),
                    "settings sheet takes focus: the preview must retire")
            change()
            await settle()
            workspace.showingSettings = false
            require(await until(10) { window.attachedSheet == nil && window.isKeyWindow },
                    "settings sheet must close and return key to the editor window")
        }
        caret(loc("E = mc") + 2)
        let above = await awaitPlacementGeometry("above")
        require(above.content.minY >= above.anchor.midY,
                "above content must sit above the actual source-line midpoint, got \(above)")
        await throughSettingsSheet { store.equationPreviewPlacement = "below" }
        let below = await awaitPlacementGeometry("below")
        require(below.content.maxY <= below.anchor.midY,
                "below content must sit below the actual source-line midpoint, got \(below)")
        require(below.editorID == above.editorID && below.documentID == above.documentID
                && below.source == above.source && below.sourceRange == above.sourceRange,
                "placement comparison must keep the same current editor/document/source anchor")
        let aboveOffset = above.content.minY - above.anchor.midY
        let belowOffset = below.content.minY - below.anchor.midY
        require(belowOffset < aboveOffset - 4,
                "below content must sit lower relative to its anchor in Cocoa y-up (above=\(aboveOffset) below=\(belowOffset))")
        await throughSettingsSheet { store.equationPreviewPlacement = "above" }
        stage("placement below moves content relative to its source anchor, Cocoa y-up: \(aboveOffset) -> \(belowOffset); above=\(above.content) below=\(below.content)")

        // ── D1: focus invariants over >=3 debounce cycles while visible ──
        // Real NSPopover didShow/didClose counters prove no hide/re-show
        // churn, not just reuse of the same window object.
        caret(loc("E = mc") + 2)
        require(await awaitPreview(12, matching: fastOf("E = mc")), "D1 sampling needs a visible preview")
        await requireStablePreviewFocus()


        // ── Imported-garbage normalization on the real defaults domain ───
        let defaults = UserDefaults.standard
        defaults.set("nonsense", forKey: "pitex.pref.equationPreview.placement")
        defaults.set("bogus", forKey: "pitex.pref.equationPreview.renderer")
        defaults.set(999, forKey: "pitex.pref.equationPreview.delay")
        defaults.set("notabool", forKey: "pitex.pref.equationPreview.enabled")
        store.reload()
        require(store.equationPreviewPlacement == "above", "junk placement must normalize to above")
        require(store.equationPreviewRenderer == "fast", "junk renderer must normalize to fast")
        require(store.equationPreviewDelayMilliseconds == 80,
                "invalid delay 999 must normalize conservatively to 80, got \(store.equationPreviewDelayMilliseconds)")
        require(store.equationPreviewEnabled, "junk enabled must normalize to default true")
        stage("imported garbage values normalize conservatively")

        // ── Settings UI: real public AX server, native press, engine effect ─
        // f295 proved SwiftUI's virtual nodes through the trusted launcher's
        // AXUIElement API; they are not NSAccessibilityProtocol children.
        var settingsRequests = 0
        var settingsNodes: [[String: Any]] = []
        func settingsFailure(_ message: String) -> Never {
            let ids = settingsNodes.compactMap { $0["id"] as? String }
            note("settings AX: sheet=\(window.attachedSheet?.windowNumber ?? -1) visible=\(window.attachedSheet?.isVisible == true)"
                 + " editorWindow=\(window.windowNumber) appKey=\(NSApp.keyWindow?.windowNumber ?? -1)"
                 + " currentMounted=\(tv()?.window === window) responderIsCurrent=\(tv() != nil && window.firstResponder === tv())"
                 + " editorTab=\(ids.contains("pitex.settings.tab.editor"))"
                 + " editorControls=\(ids.contains("pitex.settings.equationPreview.enabled") && ids.contains("pitex.settings.equationPreview.whileTyping"))")
            for node in settingsNodes {
                note("settings AX id=\(node["id"] as? String ?? "<none>")"
                     + " role=\(node["role"] as? String ?? "<none>")"
                     + " subrole=\(node["subrole"] as? String ?? "<none>")"
                     + " booleanObservation=\(node["booleanError"] as? String ?? "available-or-not-whitelisted")")
            }
            print("FAIL", message); fflush(nil); exit(1)
        }
        func settingsAX(_ operation: String = "read", id: String = "") async {
            guard !bridgeDir.isEmpty, let sheet = window.attachedSheet,
                  sheet.isVisible, NSApp.keyWindow === sheet else {
                settingsFailure("public settings AX requires the current visible key sheet and trusted bridge")
            }
            settingsRequests += 1
            let name = String(format: "%06d", settingsRequests)
            let directory = URL(fileURLWithPath: bridgeDir)
            let request: [String: Any] = [
                "seq": settingsRequests, "pid": Int(ProcessInfo.processInfo.processIdentifier),
                "window": sheet.windowNumber, "x": Double(sheet.frame.midX),
                "y": Double((NSScreen.screens.first?.frame.height ?? 0) - sheet.frame.midY),
                "operation": operation, "id": id,
            ]
            do {
                try JSONSerialization.data(withJSONObject: request)
                    .write(to: directory.appendingPathComponent("ax-req-\(name).json"), options: .atomic)
            } catch { settingsFailure("cannot write bounded public settings AX request") }
            let replyURL = directory.appendingPathComponent("ax-ack-\(name).json")
            var reply: [String: Any]?
            let received = await until(15) {
                guard let data = try? Data(contentsOf: replyURL) else { return false }
                reply = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any]
                return reply != nil
            }
            try? FileManager.default.removeItem(at: replyURL)
            guard received, let reply else { settingsFailure("public settings AX bridge reply unavailable") }
            settingsNodes = reply["nodes"] as? [[String: Any]] ?? []
            guard reply["ok"] as? Bool == true else {
                settingsFailure("public settings AX bridge refused: \(reply["error"] as? String ?? "unavailable")")
            }
            guard reply["seq"] as? Int == settingsRequests,
                  reply["pid"] as? Int == Int(ProcessInfo.processInfo.processIdentifier),
                  reply["sheetWindow"] as? Int == sheet.windowNumber,
                  window.attachedSheet === sheet, NSApp.keyWindow === sheet else {
                settingsFailure("public settings AX reply no longer belongs to the current PID/sheet/request")
            }
            if operation == "press" {
                guard reply["pressed"] as? String == id, reply["actionStatus"] as? Int == 0 else {
                    settingsFailure("native public AXPress did not report success for \(id)")
                }
                stage("settings native public AXPress \(id): AXError=0 sheet=\(sheet.windowNumber)")
            }
        }
        func settingsControl(_ id: String) async {
            let deadline = ContinuousClock.now + .seconds(10)
            repeat {
                await settingsAX()
                if settingsNodes.contains(where: { $0["id"] as? String == id }) { return }
                try? await Task.sleep(for: .milliseconds(40))
            } while ContinuousClock.now < deadline
            settingsFailure("public settings AX control unavailable: \(id)")
        }
        // Missing/other persisted values stay nil, never an OFF fallback.
        func settingsBoolean(_ value: Any?) -> Bool? {
            guard let number = value as? NSNumber, number == 0 || number == 1 else { return nil }
            return number.boolValue
        }
        func requireSettingsValues(enabled: Bool) async {
            let deadline = ContinuousClock.now + .seconds(10)
            repeat {
                await settingsAX()
                let enabledValue = settingsNodes.first {
                    $0["id"] as? String == "pitex.settings.equationPreview.enabled"
                }?["boolean"] as? Bool
                let typingValue = settingsNodes.first {
                    $0["id"] as? String == "pitex.settings.equationPreview.whileTyping"
                }?["boolean"] as? Bool
                if enabledValue == enabled && typingValue == store.equationPreviewWhileTyping
                    && store.equationPreviewEnabled == enabled
                    && settingsBoolean(defaults.object(forKey: "pitex.pref.equationPreview.enabled")) == enabled
                    && settingsBoolean(defaults.object(forKey: "pitex.pref.equationPreview.whileTyping")) == store.equationPreviewWhileTyping {
                    stage("settings typed AX/store/persist agree: enabled=\(enabled) whileTyping=\(store.equationPreviewWhileTyping)")
                    return
                }
                try? await Task.sleep(for: .milliseconds(40))
            } while ContinuousClock.now < deadline
            settingsFailure("settings typed boolean AX/store/persisted values must agree (enabled=\(enabled))")
        }
        func openEditorSettings() async {
            workspace.showingSettings = true
            if !(await until(10) {
                window.attachedSheet?.isVisible == true && NSApp.keyWindow === window.attachedSheet
            }) { settingsFailure("settings sheet must attach and become key") }
            await settingsControl("pitex.settings.tab.editor")
            await settingsAX("press", id: "pitex.settings.tab.editor")
            await settingsControl("pitex.settings.equationPreview.enabled")
            await settingsControl("pitex.settings.equationPreview.whileTyping")
        }
        func closeEditorSettings() async {
            workspace.showingSettings = false
            let restored = await until(10) {
                guard let current = tv() else { return false }
                return window.attachedSheet == nil && current.window === window
                    && NSApp.keyWindow === window && window.isKeyWindow
                    && window.firstResponder === current
            }
            if !restored {
                settingsFailure("settings close must restore the current editor/key/responder baseline without repair")
            }
        }
        await openEditorSettings()
        await requireSettingsValues(enabled: true)
        await settingsAX("press", id: "pitex.settings.equationPreview.enabled")
        await requireSettingsValues(enabled: false)
        await closeEditorSettings()
        // Sheet focus already hides the preview: prove OFF only AFTER close.
        caret(park)
        require(await awaitPreview(3, want: false), "parked caret must hide before native OFF probe")
        caret(loc("E = mc") + 2)
        await pump(0.6)
        require(!previewVisible(), "native settings OFF must suppress a fresh eligible caret preview after sheet close")
        await openEditorSettings()
        await requireSettingsValues(enabled: false)
        await settingsAX("press", id: "pitex.settings.equationPreview.enabled")
        await requireSettingsValues(enabled: true)
        await closeEditorSettings()
        require(await awaitPreview(12, matching: fastOf("E = mc")),
                "native settings ON must restore the eligible caret preview after sheet close")
        stage("settings native AX Editor/OFF/ON: typed values match store/persistence; closed-sheet OFF suppresses, ON previews")

        // ── Theme readability: a theme change re-presents the VISIBLE
        // preview (no re-typeset); each wait holds until the captured page
        // actually shows the new background, then judges fg/bg contrast.
        caret(loc("E = mc") + 2)
        require(await awaitPreview(12, matching: fastOf("E = mc")), "theme probe needs a visible fast preview")
        func awaitTheme(dark: Bool) async -> (bg: Double, fg: Double, contrast: Double, ink: Double, opaque: Double)? {
            let deadline = ContinuousClock.now + .seconds(8)
            var last = "no fast E = mc preview visible"
            while ContinuousClock.now < deadline {
                try? await Task.sleep(for: .milliseconds(100))
                guard fastOf("E = mc")(previewState()) else { continue }
                guard let r = await readability() else { last = "capture failed"; continue }
                if (r.bg < 128) == dark { return r }
                last = "\(r)"
            }
            note("theme wait dark=\(dark) timed out; last=\(last) state=\(previewState())")
            return nil
        }
        AppearanceSettings.shared.theme = .dark
        let darkStats = await awaitTheme(dark: true)
        require(darkStats != nil, "dark theme must re-present the visible preview on a dark background")
        _ = await screenshot("03-dark")
        AppearanceSettings.shared.theme = .light
        let lightStats = await awaitTheme(dark: false)
        require(lightStats != nil, "light theme must re-present the visible preview on a light background")
        _ = await screenshot("03-light")
        for (name, r) in [("dark", darkStats!), ("light", lightStats!)] {
            require(r.opaque >= 0.9, "\(name) capture must be opaque to judge contrast (opaque=\(r.opaque))")
            require(r.ink >= 0.005, "\(name) preview must contain rendered ink, not a flat fill (ink=\(r.ink))")
            require(r.contrast >= 4.5, "\(name) fg/bg contrast \(r.contrast) is below 4.5:1 (bg=\(r.bg) fg=\(r.fg))")
        }
        require(darkStats!.bg < lightStats!.bg,
                "dark theme must render darker than light (\(darkStats!.bg) vs \(lightStats!.bg))")
        require(darkStats!.fg > darkStats!.bg && lightStats!.fg < lightStats!.bg,
                "ink must be light on dark and dark on light: dark \(darkStats!) light \(lightStats!)")
        stage("theme: dark bg=\(Int(darkStats!.bg)) fg=\(Int(darkStats!.fg)) \(String(format: "%.1f", darkStats!.contrast)):1; "
              + "light bg=\(Int(lightStats!.bg)) fg=\(Int(lightStats!.fg)) \(String(format: "%.1f", lightStats!.contrast)):1")
        if NSWorkspace.shared.accessibilityDisplayShouldIncreaseContrast {
            note("increase-contrast is ON — high-contrast path exercised live")
        } else {
            note("increase-contrast OFF — global user preference not toggled; that path unverified here")
        }

        // ── Exact preview: TWO distinct documents in the same workspace.
        // Each exact document is keyed by its own source, so the second
        // target cannot be served from the first one's exact cache.
        var exactMs: [Int] = []
        caret(loc("\\R + \\C") + 1)
        require(await awaitPreview(12, matching: fastOf("\\R + \\C")), "caret must re-arm a preview target")
        if texAvailable {
            for (index, needle) in ["\\R + \\C", "\\binom{n}{k}"].enumerated() {
                if index > 0 {
                    caret(loc(needle) + 2)
                    require(await awaitPreview(12, matching: fastOf(needle)),
                            "exact target \(index + 1) must show its fast preview first")
                }
                let start = ContinuousClock.now
                workspace.equationPreview?.requestExact()
                require(await awaitPreview(40, label: "exact \(index + 1)", matching: exactOf(needle)),
                        "exact preview \(index + 1) must complete on this host with the Exact badge")
                exactMs.append(elapsedMs(start))
                st = previewState()
                _ = await screenshot("04-exact-\(index + 1)")
                stage("exact TeX preview \(index + 1): \(exactMs[index])ms badge='\(st.badge)' source='\(st.source)'")
            }
        } else {
            workspace.equationPreview?.requestExact()
            require(await awaitPreview(20, matching: { $0.role.contains("statictext") && $0.source.contains("\\R + \\C") }),
                    "missing TeX must surface a quiet message")
            note("exact preview correctly reports unavailable — no TeX engine on this host")
        }

        // ── D4a: force a hung renderer page (real JS infinite loop on the
        // WKWebView main thread); the next render must hit the 5s
        // renderTimeout. Contract (measured, not assumed): the popover
        // hides for the timeout duration, exactly ONE view replacement
        // happens (B5 budget), and a fresh image renders on the new page.
        // Missing view or missing kill selector is a FAIL, not a skip —
        // D4 is mandatory evidence.
        caret(park)
        await pump(0.2)
        caret(loc("E = mc") + 2)
        require(await awaitPreview(15, matching: fastOf("E = mc")), "D4 needs a live rendered preview before hang")
        let d4Element = previewElement()
        let d4View = d4Element.flatMap { descendants($0).compactMap { $0 as? WKWebView }.first }
        require(d4Element != nil && d4View != nil,
                "D4 FAIL: preview/WKWebView unreachable — cannot exercise hang path")
        let wk = d4View!
        // Chain the internal replacement callback — same module, so
        // onWebViewReplaced is visible; calling the original preserves
        // production behavior. K4: the cast is required, not optional.
        let renderer = wk.navigationDelegate as? MathJaxEquationRenderer
        require(renderer != nil,
                "D4 FAIL: WKWebView.navigationDelegate is not MathJaxEquationRenderer")
        let original = renderer!.onWebViewReplaced
        renderer!.onWebViewReplaced = {
            original?()
            MainActor.assumeIsolated { Driver.replaceCount += 1 }
        }
        // Budget epoch (F1): the renderer's failure count is private, and a
        // silent timeout/crash since the last recover would leave it above
        // replaceCount. With the hook already installed, reset the budget
        // through the NORMAL entrypoint — enabled false→true → recover():
        // failures = 0, the live page is kept (only a dead view is
        // rebuilt). From here every replacement is counted, and none may
        // happen before the deliberate hang.
        store.equationPreviewEnabled = false
        require(await awaitPreview(6, want: false), "D4 budget reset: disabling must retire the preview")
        store.equationPreviewEnabled = true
        require(await awaitPreview(15, matching: fastOf("E = mc")),
                "D4 budget reset: re-enabling must resume the live preview")
        require(Driver.replaceCount == 0,
                "no renderer replacement may occur across hook install, budget reset and warm-up, got \(Driver.replaceCount)")
        require(previewElement().flatMap { descendants($0).compactMap { $0 as? WKWebView }.first } === wk,
                "budget reset must keep the live page, so the hang targets the current view")
        wk.evaluateJavaScript("while (true) {}") { _, _ in }
        await pump(0.3)
        // K9: the E = mc preview may linger visible on a cache miss —
        // park the caret and REQUIRE it hidden before selecting the
        // unrendered target (on screen per the geometry precondition:
        // present() hides an anchor that is scrolled out of view).
        caret(park)
        require(await awaitPreview(3, want: false),
                "parked caret must dismiss any stale popover before hang target")
        caret(loc("\\sum_{k=1}^{n} k") + 2)
        let hideDeadline = ContinuousClock.now + .seconds(4)
        while ContinuousClock.now < hideDeadline {
            try? await Task.sleep(for: .milliseconds(50))
            require(!previewVisible(),
                    "hung page must stay hidden through the render timeout")
        }
        // Then the replacement comes up and a fresh render must land.
        require(await until(15) {
            previewElement().flatMap {
                descendants($0).compactMap { $0 as? WKWebView }.first
            } !== wk
        }, "renderer must replace the hung view — not reuse the dead one")
        require(Driver.replaceCount == 1,
                "exactly one replacement expected after hung timeout, got \(Driver.replaceCount)")
        require(await awaitPreview(20, matching: fastOf("\\sum_{k=1}^{n} k")),
                "replacement page must render a fresh image")
        st = previewState()
        require(st.role.contains("image"),
                "post-hang replacement must render content, got \(st.role)")
        stage("D4a: hung render -> timeout hide -> 1 replacement (counted) -> fresh image")

        // ── D4b: budget exhaustion + honest recovery. Kill the renderer's
        // CURRENT web content process repeatedly (private
        // _killWebContentProcess, test-only — no public API on the macOS 27
        // SDK). Re-fetch the live view each cycle: a killed view is
        // orphaned with nil delegates. After the budget is spent the only
        // sanctioned recovery is the real settings entrypoint —
        // equationPreviewEnabled false→true — never a counter reset.
        let killSel = NSSelectorFromString("_killWebContentProcess")
        let killable = previewElement()
            .flatMap { descendants($0).compactMap { $0 as? WKWebView }.first }
        require(killable?.responds(to: killSel) ?? false,
                "D4b FAIL: _killWebContentProcess unavailable in this WebKit")
        // K5: the criterion is the CONFIGURED budget, not an arbitrary cap.
        // maxRestarts is internal (same module). replaceCount counts every
        // replacement since the D4 budget reset (failures = 0), so it equals
        // the spent budget: D4a's one replacement, asserted above.
        let expectedRemaining = MathJaxEquationRenderer.maxRestarts - Driver.replaceCount
        require(expectedRemaining > 0,
                "D4b FAIL: replacement budget already spent entering kill loop")
        var budgetCycles = 0
        var exhausted = false
        while budgetCycles <= expectedRemaining {   // one extra would prove overshoot
            let live = previewElement()
                .flatMap { descendants($0).compactMap { $0 as? WKWebView }.first }
            require(live?.responds(to: killSel) ?? false,
                    "D4b FAIL: live WKWebView/_killWebContentProcess gone at cycle \(budgetCycles)")
            live!.perform(killSel)
            budgetCycles += 1
            await pump(2.0)   // let webViewWebContentProcessDidTerminate fire + rebuild
            caret(park)
            require(await awaitPreview(3, want: false),
                    "parked caret must hide before probing cycle \(budgetCycles)")
            caret(loc("E = mc") + 2)
            if await awaitPreview(15, matching: fastOf("E = mc")) {
                stage("D4b: cycle \(budgetCycles) — renderer recovered after kill")
            } else {
                exhausted = true
                stage("D4b: budget exhausted after \(budgetCycles) kill(s); preview dead")
                break
            }
        }
        require(exhausted && budgetCycles == expectedRemaining,
                "D4b FAIL: expected exhaustion after \(expectedRemaining) kills, "
                + "got cycles=\(budgetCycles) exhausted=\(exhausted)")
        // Automatic retries must NOT replenish the budget — only the
        // real settings toggle may recover the renderer.
        await pump(1.0)
        caret(park)
        caret(loc("E = mc") + 2)
        let selfHealed = await awaitPreview(8)
        require(!selfHealed,
                "dead budget must not self-heal without explicit enable")

        // P3 must be tested with a SPENT epoch, not a freshly reset budget:
        // Markdown -> TeX must not rebuild/re-arm the exhausted renderer.
        let exhaustedReplacements = Driver.replaceCount
        let dirtyMain = tv()!.string
        func activateTypeFixture(_ name: String, containing needle: String) async {
            await workspace.activateDocument(URL(fileURLWithPath: root + "/" + name))
            require(await until(15) {
                workspace.activeDocumentURL?.lastPathComponent == name
                    && tv()?.window === window && (tv()?.string.contains(needle) == true)
            }, "type fixture \(name) must become the mounted current document")
            require(await awaitPreview(3, want: false),
                    "type activation must retire the outgoing preview before fixture focus setup")
            // A new real document-activation boundary, BEFORE preview input;
            // never repair focus after a shown preview or within an oracle.
            await prepareCurrentEditorFocus("type fixture \(name)")
        }
        await activateTypeFixture("notes.md", containing: "Markdown context")
        require(store.equationPreviewEnabled && Driver.replaceCount == exhaustedReplacements,
                "non-TeX activation must preserve the persisted enabled flag and exhausted epoch")
        await activateTypeFixture("main.tex", containing: "E = mc")
        require(tv()!.string == dirtyMain, "type transitions must preserve the dirty TeX buffer")
        caret(loc("End plain text line") + 5)
        caret(loc("E = mc") + 2)
        require(!(await awaitPreview(8, matching: fastOf("E = mc"))),
                "returning from Markdown must not recover an exhausted renderer")
        require(Driver.replaceCount == exhaustedReplacements,
                "document type changes must not replace/re-arm the exhausted renderer")
        stage("P3: spent epoch survives Markdown -> TeX; persisted enabled stays true, no replacement or preview")
        store.equationPreviewEnabled = false
        await pump(0.4)
        store.equationPreviewEnabled = true
        require(await awaitPreview(15, matching: fastOf("E = mc")),
                "explicit enabled false→true must recover the renderer")
        require(previewState().role.contains("image"),
                "post-recovery preview must render an image")
        stage("D4b: \(budgetCycles) kills -> exhaustion -> explicit enable false→true recovers")

        // P2 separately uses a LIVE renderer, so the non-TeX negative cannot
        // pass merely because the renderer is exhausted or editor unfocused.
        let recoveredReplacements = Driver.replaceCount
        caret(loc("End plain text line") + 5)
        require(await awaitPreview(3, want: false), "park before live non-TeX eligibility probe")
        await activateTypeFixture("notes.md", containing: "Markdown context")
        let markdownMath = loc("E = mc")
        require(markdownMath != NSNotFound, "non-TeX fixture must contain known eligible math syntax")
        caret(markdownMath + 2)
        await pump(0.6)
        require(!previewVisible(), "live enabled renderer must not preview math in a non-TeX document")
        caret(0)
        await activateTypeFixture("main.tex", containing: "E = mc")
        require(tv()!.string == dirtyMain, "live type transitions must preserve the dirty TeX buffer")
        caret(loc("E = mc") + 2)
        require(await awaitPreview(12, matching: fastOf("E = mc")),
                "returning to TeX must restore the actual eligible preview")
        require(Driver.replaceCount == recoveredReplacements,
                "live file-type transitions must not replace the renderer")
        stage("P2: focused live non-TeX math suppressed; TeX return previews with no replacement")


        // ── Temp artifacts cleanup ────────────────────────────────────────
        let tmp = FileManager.default.temporaryDirectory.path
        let leftovers = (try? FileManager.default.contentsOfDirectory(atPath: tmp))?
            .filter { $0.hasPrefix("pitex-equation") } ?? []
        note("exact-temp leftovers under /tmp: \(leftovers)")

        print("[timing] preflight(cold page)=\(preflightMs)ms firstHover=\(coldMs)ms fresh=\(freshMs)ms "
              + "cache=\(cacheMs)ms delay150=\(delayMs)ms exact=\(exactMs)")
        // Two distinct exact renders are an acceptance gate: a host without
        // TeX reports the quiet message above but never completes.
        require(exactMs.count == 2,
                "two distinct exact TeX renders required, got \(exactMs.count) (tex present=\(texAvailable))")
        print("PASS equation-preview: hover/caret/edit/macro/switch/escape/focus/scroll/settings/appearance/exact")
        print("CHECK_COMPLETE")
        fflush(nil)
        exit(0)
    }

    static func elapsedMs(_ start: ContinuousClock.Instant) -> Int {
        let d = ContinuousClock.now - start
        return Int(d.components.seconds * 1000 + d.components.attoseconds / 1_000_000_000_000_000)
    }

    static func until(_ timeoutSeconds: Double = 10,
                      _ condition: @escaping @MainActor () -> Bool) async -> Bool {
        let deadline = ContinuousClock.now + .seconds(timeoutSeconds)
        while !condition() {
            if .now >= deadline { return false }
            try? await Task.sleep(for: .milliseconds(40))
        }
        return true
    }

    static func appDiag() -> String {
        let app = NSApplication.shared
        return "live=\(WorkspaceWindows.live.count) windows=\(app.windows.count) "
            + "active=\(app.isActive) finish=\(LaunchFlag.didFinishLaunching)"
    }
}
'''

# Every hover/caret target lives in the first ~20 lines (on screen at the
# harness window size — the driver asserts it); the large block follows,
# then filler so the whole block can scroll out of view.
FIXTURE_MAIN = r"""\documentclass{article}
\usepackage{amsmath,amssymb}
\input{macros}
\begin{document}
Intro text line.
Inline $E = mc^2$ math here.
Display $$\int_0^1 x^2\,dx = \frac{1}{3}$$ more text.
\begin{align}
a &= b + c \\
d &= e + f
\end{align}
Macro $\R + \C$ usage.
Second exact target $\binom{n}{k}$ here.
Not math: \$ dollars and %% $x^2$ comment.
\begin{verbatim}
$z = 99$
\end{verbatim}
End plain text line for caret parking.
Late $\sum_{k=1}^{n} k$ uncached target for the D4a hang check.

\begin{align}
%(rows)s
\end{align}

\begin{align}
%(rows_a)s
\end{align}

\begin{align}
%(rows_b)s
\end{align}

\begin{align}
%(rows_c)s
\end{align}
%(filler)s
\end{document}
"""

FIXTURE_OTHER = r"""\documentclass{article}
\begin{document}
Other document $x_{B}^{2}+1$ done.
\end{document}
"""

FIXTURE_MACROS = "\\newcommand{\\R}{\\mathbb{R}}\n\\newcommand{\\C}{\\mathbb{C}}\n"


class CGPoint(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]


class CGRect(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double),
                ("width", ctypes.c_double), ("height", ctypes.c_double)]


def settings_ax(bridge, request):
    """Public remote AX, proven by f295; only settings reads/two presses."""
    boolean_ids = {"pitex.settings.equationPreview.enabled", "pitex.settings.equationPreview.whileTyping"}
    nodes, ancestors, owned = [], [], []
    cf = bridge.cf
    ax = ctypes.CDLL("/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices")
    vp = ctypes.c_void_p
    for function, result, arguments in (
        (ax.AXIsProcessTrusted, ctypes.c_bool, []),
        (ax.AXUIElementCreateApplication, vp, [ctypes.c_int]),
        (ax.AXUIElementCopyElementAtPosition, ctypes.c_int, [vp, ctypes.c_float, ctypes.c_float, ctypes.POINTER(vp)]),
        (ax.AXUIElementCopyAttributeValue, ctypes.c_int, [vp, vp, ctypes.POINTER(vp)]),
        (ax.AXUIElementPerformAction, ctypes.c_int, [vp, vp]),
        (ax.AXUIElementGetPid, ctypes.c_int, [vp, ctypes.POINTER(ctypes.c_int)]),
        (ax.AXUIElementGetTypeID, ctypes.c_ulong, []),
        (ax.AXUIElementSetMessagingTimeout, ctypes.c_int, [vp, ctypes.c_float]),
        (cf.CFRetain, vp, [vp]),
        (cf.CFGetTypeID, ctypes.c_ulong, [vp]),
        (cf.CFHash, ctypes.c_ulong, [vp]),
        (cf.CFEqual, ctypes.c_bool, [vp, vp]),
        (cf.CFStringCreateWithCString, vp, [vp, ctypes.c_char_p, ctypes.c_uint32]),
        (cf.CFStringGetTypeID, ctypes.c_ulong, []),
        (cf.CFStringGetCString, ctypes.c_bool, [vp, ctypes.c_char_p, ctypes.c_long, ctypes.c_uint32]),
        (cf.CFArrayGetTypeID, ctypes.c_ulong, []),
        (cf.CFBooleanGetTypeID, ctypes.c_ulong, []),
        (cf.CFBooleanGetValue, ctypes.c_bool, [vp]),
        (cf.CFNumberGetTypeID, ctypes.c_ulong, []),
    ):
        function.restype, function.argtypes = result, arguments
    if not ax.AXIsProcessTrusted():
        return {"ok": False, "error": "launcher AXIsProcessTrusted false", "nodes": nodes}
    deadline, keys = time.monotonic() + 10, {}

    def copy(element, name):
        if time.monotonic() >= deadline:
            raise TimeoutError("public settings AX exceeded 10-second deadline")
        if name not in keys:
            key = cf.CFStringCreateWithCString(None, name.encode(), 0x08000100)
            if not key:
                raise RuntimeError("AX attribute string allocation failed")
            keys[name] = key
            owned.append(key)
        value = vp()
        status = ax.AXUIElementCopyAttributeValue(element, keys[name], ctypes.byref(value))
        if value.value:
            owned.append(value.value)
        if status not in (0, -25205, -25212):
            raise RuntimeError(f"public AX attribute {name} failed: {status}")
        return status, value.value if status == 0 else None

    def text(element, name):
        _, value = copy(element, name)
        if value is None:
            return "<none>"
        if cf.CFGetTypeID(value) != cf.CFStringGetTypeID():
            raise RuntimeError(f"public AX {name} is not a string")
        buffer = ctypes.create_string_buffer(4096)
        if not cf.CFStringGetCString(value, buffer, len(buffer), 0x08000100):
            raise RuntimeError(f"public AX {name} exceeds metadata cap")
        return buffer.value.decode("utf-8")

    def metadata(element):
        pid = ctypes.c_int()
        if ax.AXUIElementGetPid(element, ctypes.byref(pid)) != 0 or pid.value != request["pid"]:
            raise RuntimeError("public AX element is not owned by the checker PID")
        row = {"id": text(element, "AXIdentifier"), "role": text(element, "AXRole"),
               "subrole": text(element, "AXSubrole")}
        if row["id"] in boolean_ids:
            status, value = copy(element, "AXValue")
            if value is None:
                row["booleanError"] = f"AXValue unavailable: {status}"
            elif cf.CFGetTypeID(value) == cf.CFBooleanGetTypeID():
                row["boolean"] = bool(cf.CFBooleanGetValue(value))
            elif cf.CFGetTypeID(value) == cf.CFNumberGetTypeID():
                number = ctypes.c_double()
                if not cf.CFNumberGetValue(value, 13, ctypes.byref(number)) or number.value not in (0, 1):
                    row["booleanError"] = "AXValue is not exactly numeric 0/1"
                else:
                    row["boolean"] = number.value == 1
            else:
                row["booleanError"] = "AXValue has unsupported type"
        return row

    try:
        number_key = vp.in_dll(bridge.cg, "kCGWindowNumber").value
        windows = bridge.cg.CGWindowListCopyWindowInfo(1 | 16, 0)
        if not windows:
            raise RuntimeError("on-screen checker sheet observation unavailable")
        owned.append(windows)
        matched = False
        for index in range(cf.CFArrayGetCount(windows)):
            entry = cf.CFArrayGetValueAtIndex(windows, index)
            pid, number = ctypes.c_int64(), ctypes.c_int64()
            owner = cf.CFDictionaryGetValue(entry, bridge.key_pid)
            window_number = cf.CFDictionaryGetValue(entry, number_key)
            if not owner or not window_number or not cf.CFNumberGetValue(owner, 4, ctypes.byref(pid)) \
                    or not cf.CFNumberGetValue(window_number, 4, ctypes.byref(number)):
                continue
            if pid.value == request["pid"] and number.value == request["window"]:
                bounds, rect = cf.CFDictionaryGetValue(entry, bridge.key_bounds), CGRect()
                matched = bool(bounds and bridge.cg.CGRectMakeWithDictionaryRepresentation(bounds, ctypes.byref(rect))
                               and all(math.isfinite(v) for v in (rect.x, rect.y, rect.width, rect.height))
                               and rect.width > 0 and rect.height > 0
                               and rect.x <= request["x"] < rect.x + rect.width
                               and rect.y <= request["y"] < rect.y + rect.height)
                break
        if not matched:
            raise RuntimeError("requested sheet number/PID/point not present on screen")
        app = ax.AXUIElementCreateApplication(request["pid"])
        if not app:
            raise RuntimeError("AXUIElementCreateApplication failed")
        owned.append(app)
        status = ax.AXUIElementSetMessagingTimeout(app, 0.5)
        if status != 0:
            raise RuntimeError(f"public AX messaging timeout unavailable: {status}")
        hit = vp()
        status = ax.AXUIElementCopyElementAtPosition(app, request["x"], request["y"], ctypes.byref(hit))
        if status != 0 or not hit.value:
            raise RuntimeError(f"public AX checker-sheet hit test failed: {status}")
        owned.append(hit.value)
        element = hit.value
        for _ in range(33):
            row = metadata(element)
            ancestors.append(row)
            if row["role"] == "AXSheet":
                break
            _, parent = copy(element, "AXParent")
            if parent is None or cf.CFGetTypeID(parent) != ax.AXUIElementGetTypeID():
                raise RuntimeError("public AX hit-test ancestry did not reach the owned sheet")
            element = parent
        else:
            raise RuntimeError("public AX sheet ancestry exceeded depth bound")
        pending, seen, controls = [(element, 0)], {}, []
        while pending:
            element, depth = pending.pop()
            bucket = seen.setdefault(cf.CFHash(element), [])
            if any(cf.CFEqual(element, previous) for previous in bucket):
                continue
            if depth > 32 or len(nodes) >= 1024:
                raise RuntimeError("public AX sheet traversal exceeded depth/node bound")
            bucket.append(element)
            row = metadata(element)
            nodes.append(row)
            if request["operation"] == "press" and row["id"] == request["id"]:
                controls.append(element)
            _, children = copy(element, "AXChildren")
            if children is None:
                continue
            if cf.CFGetTypeID(children) != cf.CFArrayGetTypeID():
                raise RuntimeError("public AXChildren is not an array")
            count = cf.CFArrayGetCount(children)
            if count > 1024 or len(pending) + count > 1024:
                raise RuntimeError("public AXChildren exceeds pending-node bound")
            for index in range(count):
                child = cf.CFArrayGetValueAtIndex(children, index)
                if not child or cf.CFGetTypeID(child) != ax.AXUIElementGetTypeID():
                    raise RuntimeError("public AXChildren contains a non-AXUIElement")
                owned.append(cf.CFRetain(child))
                pending.append((child, depth + 1))
        reply = {"ok": True, "seq": request["seq"], "pid": request["pid"],
                 "sheetWindow": request["window"], "nodes": nodes}
        if request["operation"] == "press":
            if len(controls) != 1:
                raise RuntimeError("whitelisted public AXPress target missing or ambiguous")
            if time.monotonic() >= deadline:
                raise TimeoutError("public settings AXPress exceeded deadline")
            action = cf.CFStringCreateWithCString(None, b"AXPress", 0x08000100)
            if not action:
                raise RuntimeError("public AXPress string allocation failed")
            owned.append(action)
            status = ax.AXUIElementPerformAction(controls[0], action)
            if status != 0:
                raise RuntimeError(f"native public AXPress failed: {status}")
            reply.update(pressed=request["id"], actionStatus=status)
        return reply
    except (RuntimeError, TimeoutError) as error:
        return {"ok": False, "error": str(error), "nodes": nodes, "ancestors": ancestors}
    finally:
        for value in reversed(owned):
            cf.CFRelease(value)


class HidBridge:
    """Trusted-operator HID bridge. The per-run ad-hoc checker app can
    never hold post-event access, so THIS launcher process — started by
    the operator's trusted automation identity — posts the real Quartz
    mouseMoved events the driver requests through a private per-run
    directory. Bounded and scoped: strictly sequential numbered requests,
    size and count caps, an exact {seq,pid,x,y} schema, the requester must
    be the checker executable, and the point must lie inside one of its
    on-screen windows. Nothing is evaluated; nothing synthetic is posted."""

    MAX_REQUESTS = 2000
    MAX_BYTES = 512
    NAME = re.compile(r"^req-(\d{6})\.json$")
    AX_NAME = re.compile(r"^ax-req-(\d{6})\.json$")

    def __init__(self, directory, app):
        self.dir, self.app = directory, os.path.realpath(app)
        self.last, self.checker_pid = 0, None
        self.settings_ax_last = 0
        cg = ctypes.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
        cf = ctypes.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
        libproc = ctypes.CDLL("/usr/lib/libproc.dylib")
        vp = ctypes.c_void_p
        for fn, res, args in (
            (cg.CGPreflightPostEventAccess, ctypes.c_bool, []),
            (cg.CGEventSourceCreate, vp, [ctypes.c_int32]),
            (cg.CGEventCreateMouseEvent, vp, [vp, ctypes.c_uint32, CGPoint, ctypes.c_uint32]),
            (cg.CGEventPost, None, [ctypes.c_uint32, vp]),
            (cg.CGEventCreate, vp, [vp]),
            (cg.CGEventGetLocation, CGPoint, [vp]),
            (cg.CGWindowListCopyWindowInfo, vp, [ctypes.c_uint32, ctypes.c_uint32]),
            (cg.CGRectMakeWithDictionaryRepresentation, ctypes.c_bool, [vp, ctypes.POINTER(CGRect)]),
            (cf.CFRelease, None, [vp]),
            (cf.CFArrayGetCount, ctypes.c_long, [vp]),
            (cf.CFArrayGetValueAtIndex, vp, [vp, ctypes.c_long]),
            (cf.CFDictionaryGetValue, vp, [vp, vp]),
            (cf.CFNumberGetValue, ctypes.c_bool, [vp, ctypes.c_long, vp]),
            (libproc.proc_pidpath, ctypes.c_int, [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]),
        ):
            fn.restype, fn.argtypes = res, args
        self.cg, self.cf, self.libproc = cg, cf, libproc
        self.key_pid = vp.in_dll(cg, "kCGWindowOwnerPID").value
        self.key_bounds = vp.in_dll(cg, "kCGWindowBounds").value
        self.access = bool(cg.CGPreflightPostEventAccess())
        self.source = cg.CGEventSourceCreate(1)  # kCGEventSourceStateHIDSystemState

    def cursor(self):
        event = self.cg.CGEventCreate(None)
        point = self.cg.CGEventGetLocation(event)
        self.cf.CFRelease(event)
        return point

    def is_checker(self, pid):
        if pid == self.checker_pid:
            return True
        buffer = ctypes.create_string_buffer(4096)
        if self.libproc.proc_pidpath(pid, buffer, 4096) <= 0:
            return False
        if os.path.realpath(buffer.value.decode(errors="replace")).startswith(self.app + os.sep):
            self.checker_pid = pid
            return True
        return False

    def in_checker_window(self, pid, x, y):
        windows = self.cg.CGWindowListCopyWindowInfo(1 | 16, 0)  # on screen, no desktop elements
        if not windows:
            return False
        try:
            for index in range(self.cf.CFArrayGetCount(windows)):
                entry = self.cf.CFArrayGetValueAtIndex(windows, index)
                owner = ctypes.c_int64(0)
                number = self.cf.CFDictionaryGetValue(entry, self.key_pid)
                if not number or not self.cf.CFNumberGetValue(number, 4, ctypes.byref(owner)) or owner.value != pid:
                    continue  # kCFNumberSInt64Type
                rect, bounds = CGRect(), self.cf.CFDictionaryGetValue(entry, self.key_bounds)
                if bounds and self.cg.CGRectMakeWithDictionaryRepresentation(bounds, ctypes.byref(rect)) \
                        and rect.x <= x < rect.x + rect.width and rect.y <= y < rect.y + rect.height:
                    return True
            return False
        finally:
            self.cf.CFRelease(windows)

    def handle(self, seq, raw):
        def refuse(why):
            return {"seq": seq, "ok": False, "error": why}
        if seq != self.last + 1 or seq > self.MAX_REQUESTS:
            return refuse(f"out-of-order or over-cap request (last served {self.last})")
        self.last = seq
        if raw is None:
            return refuse("request is not a small regular file owned by this user")
        try:
            request = json.loads(raw)
        except ValueError:
            return refuse("request is not JSON")
        if not isinstance(request, dict) or set(request) != {"seq", "pid", "x", "y"} \
                or request["seq"] != seq or type(request["pid"]) is not int \
                or not all(type(request[k]) in (int, float) and math.isfinite(request[k]) for k in ("x", "y")):
            return refuse("request does not match {seq,pid,x,y}")
        if not self.access:
            return refuse("this launcher process has no post-event access (CGPreflightPostEventAccess false)")
        pid, x, y = request["pid"], float(request["x"]), float(request["y"])
        if not self.is_checker(pid):
            return refuse(f"pid {pid} is not the checker executable under {self.app}")
        if not self.in_checker_window(pid, x, y):
            return refuse(f"({x}, {y}) lies outside every on-screen checker window")
        event = self.cg.CGEventCreateMouseEvent(self.source, 5, CGPoint(x, y), 0)  # mouseMoved, left
        if not event:
            return refuse("CGEventCreateMouseEvent failed")
        self.cg.CGEventPost(0, event)  # kCGHIDEventTap
        self.cf.CFRelease(event)
        deadline = time.monotonic() + 1.0
        while True:
            now = self.cursor()
            if math.hypot(now.x - x, now.y - y) < 1:
                return {"seq": seq, "ok": True, "cursor": [now.x, now.y]}
            if time.monotonic() > deadline:
                return refuse(f"posted, but the cursor stayed at ({now.x}, {now.y}): events filtered")
            time.sleep(0.002)

    def service_settings_ax(self):
        for path in sorted(self.dir.iterdir()):
            match = self.AX_NAME.match(path.name)
            if not match:
                continue
            seq = int(match.group(1))
            reply = {"ok": False, "error": "invalid bounded settings AX request", "nodes": []}
            try:
                info = path.lstat()
                valid = stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_size <= self.MAX_BYTES
                request = json.loads(path.read_bytes()) if valid else None
                valid = isinstance(request, dict) and set(request) == {"seq", "pid", "window", "x", "y", "operation", "id"} \
                    and type(request["seq"]) is int and request["seq"] == seq \
                    and type(request["pid"]) is int and type(request["window"]) is int \
                    and all(type(request[k]) in (int, float) and math.isfinite(request[k]) for k in ("x", "y")) \
                    and type(request["operation"]) is str and type(request["id"]) is str \
                    and (request["operation"] == "read" and request["id"] == ""
                         or request["operation"] == "press" and request["id"] in {
                             "pitex.settings.tab.editor", "pitex.settings.equationPreview.enabled"})
                if valid and seq == self.settings_ax_last + 1 and seq <= self.MAX_REQUESTS:
                    self.settings_ax_last = seq
                    if self.is_checker(request["pid"]):
                        reply = settings_ax(self, request)
            except (OSError, ValueError):
                reply = {"ok": False, "error": "settings AX request file/JSON unavailable", "nodes": []}
            finally:
                path.unlink(missing_ok=True)
            staged = self.dir / f".ax-ack-{seq:06d}.tmp"
            staged.write_text(json.dumps(reply))
            os.replace(staged, self.dir / f"ax-ack-{seq:06d}.json")

    def service(self):
        self.service_settings_ax()
        for path in sorted(self.dir.iterdir()):
            match = self.NAME.match(path.name)
            if not match:
                continue
            seq = int(match.group(1))
            try:
                info = path.lstat()
                raw = path.read_bytes() if stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() \
                    and info.st_size <= self.MAX_BYTES else None
            finally:
                path.unlink(missing_ok=True)
            reply = self.handle(seq, raw)
            staged = self.dir / f".ack-{seq:06d}.tmp"
            staged.write_text(json.dumps(reply))
            os.replace(staged, self.dir / f"ack-{seq:06d}.json")
            if not reply["ok"]:
                print(f"[bridge] refused #{seq}: {reply['error']}", flush=True)


def process_ancestry():
    """pid:command chain up to launchd — names the identity whose trust
    (TCC responsibility) the launcher's post-event access comes from."""
    chain, pid = [], os.getpid()
    while pid > 1 and len(chain) < 12:
        line = subprocess.run(["/bin/ps", "-o", "ppid=,comm=", "-p", str(pid)],
                              capture_output=True, text=True).stdout.strip()
        if not line:
            break
        parent, _, command = line.partition(" ")
        chain.append(f"{pid}:{command.strip()}")
        pid = int(parent)
    return " <- ".join(chain)


def traced_equation_source(source):
    """Insert observation only into a throwaway copy; every marker must
    match once. No controller/state-machine behavior is replaced."""
    probes = [
        (
            "        let total = CGSize(width: bodySize.width, height: bodySize.height + badgeHeight)\n",
            r'''        HoverDiagnostic.log("popover.present source=\(source.prefix(100)) total=\(total) rect=\(rect) beforeShown=\(popover.isShown) windowVisible=\(content.window?.isVisible ?? false) editorKey=\(view.window?.isKeyWindow ?? false)")
        defer {
            HoverDiagnostic.log("popover.present.return shown=\(popover.isShown) windowVisible=\(content.window?.isVisible ?? false) frame=\(String(describing: content.window?.frame)) editorKey=\(view.window?.isKeyWindow ?? false)")
        }
''',
        ),
        (
            "    func hide() {\n",
            r'''        HoverDiagnostic.log("popover.hide shown=\(popover.isShown) windowVisible=\(content.window?.isVisible ?? false)")
''',
        ),
        (
            "            apply(engine.pointerInPopover(inside, nowMs: nowMs))\n",
            r'''            HoverDiagnostic.log("popover.pointer inside=\(inside) engineVisible=\(engine.isVisible) deadline=\(String(describing: engine.nextDeadline))")
''',
        ),
        (
            "            apply(engine.hover(location: location, nowMs: nowMs))\n",
            r'''            HoverDiagnostic.log("tracker.hover location=\(String(describing: location)) engineVisible=\(engine.isVisible) deadline=\(String(describing: engine.nextDeadline))")
''',
        ),
        (
            "        let range = textView.selectedRange()\n",
            r'''        HoverDiagnostic.log("selection range=\(range) revision=\(revision)")
''',
        ),
        (
            "        let focused = textView.window?.isKeyWindow == true && textView.window?.firstResponder === textView\n",
            r'''        HoverDiagnostic.log("focus focused=\(focused) editorKey=\(textView.window?.isKeyWindow ?? false) responderIsEditor=\(textView.window?.firstResponder === textView) appKey=\(NSApp.keyWindow?.windowNumber ?? -1) engineVisible=\(engine.isVisible)")
''',
        ),
        (
            "            case .hide:\n",
            r'''                HoverDiagnostic.log("engine.command hide at=\(nowMs) editorKey=\(textView?.window?.isKeyWindow ?? false)")
''',
        ),
        (
            '            apply(engine.poll(nowMs: max(nowMs, deadline), text: text))\n',
            r'''            HoverDiagnostic.log("wake.poll at=\(nowMs) deadline=\(deadline) engineVisible=\(engine.isVisible)")
''',
        ),
        (
            "    private func show(_ presentation: EquationPreviewPresentation) {\n",
            r'''        HoverDiagnostic.log("controller.show generation=\(presentation.generation) trigger=\(presentation.trigger) anchor=\(presentation.anchor) source=\(sourceText(presentation).prefix(100))")
''',
        ),
        (
            "                guard token == showToken else { return }\n",
            r'''                HoverDiagnostic.log("renderer.show reply size=\(String(describing: size)) token=\(token) currentToken=\(showToken)")
''',
        ),
        (
            "    private func present(_ body: EquationPreviewPopover.Body, label: String, background: NSColor) {\n",
            r'''        HoverDiagnostic.log("controller.present body=\(body) presentationExists=\(presentation != nil) textVisibleRect=\(String(describing: textView?.visibleRect))")
''',
        ),
        (
            "        guard let presentation, let textView, let rect = anchorRect(for: presentation) else {\n",
            r'''            HoverDiagnostic.log("controller.present no visible anchor -> hide")
''',
        ),
        (
            "    private func reposition() {\n",
            r'''        HoverDiagnostic.log("reposition presentationExists=\(presentation != nil) shown=\(popover.isShown) textVisibleRect=\(String(describing: textView?.visibleRect))")
''',
        ),
        (
            "        let point = textView.convert(event.locationInWindow, from: nil)\n",
            r'''        let publicWindowCursor = textView.window?.mouseLocationOutsideOfEventStream
        let publicLocalCursor = publicWindowCursor.map { textView.convert($0, from: nil) }
        HoverDiagnostic.log("tracker.geometry type=\(event.type.rawValue) windowNumber=\(event.windowNumber) associatedWindow=\(String(describing: event.window.map(ObjectIdentifier.init))) editorWindow=\(String(describing: textView.window.map(ObjectIdentifier.init))) eventWindowPoint=\(event.locationInWindow) eventLocalPoint=\(point) cgGlobal=\(String(describing: event.cgEvent?.location)) cursorGlobal=\(NSEvent.mouseLocation) publicWindowCursor=\(String(describing: publicWindowCursor)) publicLocalCursor=\(String(describing: publicLocalCursor)) inset=\(textView.textContainerOrigin) visible=\(textView.visibleRect) appKey=\(NSApp.keyWindow?.windowNumber ?? -1)")
''',
        ),
        (
            "    func setDocument(_ url: URL?) {\n",
            r'''        HoverDiagnostic.log("controller.setDocument next=\(String(describing: url?.lastPathComponent)) boundEditor=\(String(describing: textView.map(ObjectIdentifier.init))) boundWindow=\(String(describing: textView?.window.map(ObjectIdentifier.init)))")
''',
        ),
        (
            "    func attach(textView: NSTextView, scrollView: NSScrollView) {\n",
            r'''        HoverDiagnostic.log("controller.attach oldEditor=\(String(describing: self.textView.map(ObjectIdentifier.init))) newEditor=\(ObjectIdentifier(textView)) newWindow=\(String(describing: textView.window.map(ObjectIdentifier.init)))")
''',
        ),
        (
            "    func detach() {\n",
            r'''        HoverDiagnostic.log("controller.detach editor=\(String(describing: textView.map(ObjectIdentifier.init))) window=\(String(describing: textView?.window.map(ObjectIdentifier.init))) shown=\(popover.isShown)")
''',
        ),
        (
            "    private func handleEscape(isEscape: Bool, modifierClear: Bool, windowID: ObjectIdentifier?) -> Bool {\n",
            r'''        HoverDiagnostic.log("controller.escape entry escape=\(isEscape) modifiersClear=\(modifierClear) eventWindowID=\(String(describing: windowID)) editor=\(String(describing: textView.map(ObjectIdentifier.init))) editorWindowID=\(String(describing: textView?.window.map(ObjectIdentifier.init))) marked=\(textView?.hasMarkedText() ?? false) responderIsEditor=\(textView?.window?.firstResponder === textView)")
''',
        ),
        (
            "        let (consumed, commands) = engine.escape(nowMs: nowMs)\n",
            r'''        HoverDiagnostic.log("controller.escape consumed=\(consumed) commandCount=\(commands.count)")
''',
        ),
    ]
    for marker, observation in probes:
        assert source.count(marker) == 1, f"diagnostic marker changed: {marker!r}"
        if "apply(engine.pointerInPopover" in marker or "apply(engine.hover(" in marker \
                or "apply(engine.poll(" in marker or "guard token ==" in marker:
            source = source.replace(marker, observation + marker, 1)
        else:
            source = source.replace(marker, marker + observation, 1)
    return source

with tempfile.TemporaryDirectory(prefix="pitex-eqpreview-", dir="/tmp") as directory:
    root = Path(directory)
    fixture = root / "fixture"
    fixture.mkdir()
    rows = "\n".join(f"r_{i} &= {i} \\\\" for i in range(1, 40))
    filler = "\n".join(f"Filler line {i} keeps the document scrollable." for i in range(1, 91))
    rows_a = "\n".join(f"a_{i} &= {i} \\\\" for i in range(1, 6))
    rows_b = "\n".join(f"b_{i} &= {i} \\\\" for i in range(1, 21))
    rows_c = "\n".join(f"c_{i} &= {i} \\\\" for i in range(1, 61))
    (fixture / "main.tex").write_text(FIXTURE_MAIN % {
        "rows": rows, "rows_a": rows_a, "rows_b": rows_b, "rows_c": rows_c,
        "filler": filler})
    (fixture / "other.tex").write_text(FIXTURE_OTHER)
    (fixture / "macros.tex").write_text(FIXTURE_MACROS)
    (fixture / "notes.md").write_text("Markdown context\n$E = mc^2$\n")

    shots = out_dir or (root / "shots")
    shots.mkdir(parents=True, exist_ok=True)

    # Test-only PitexApp.swift: production @main stays; the finish hook
    # records launch userInfo and starts the driver. Same convention as
    # check-settings-menu.py — the checkout file is never modified.
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    patched = app_main.read_text().replace(
        "    func applicationDidFinishLaunching(_ notification: Notification) {",
        "    func applicationDidFinishLaunching(_ notification: Notification) {\n"
        "        let info = String(describing: notification.userInfo)\n"
        "        Task { @MainActor in\n"
        "            LaunchFlag.didFinishLaunching = true\n"
        "            LaunchFlag.finishUserInfo = info\n"
        "            await Driver.run()\n"
        "        }")
    assert "await Driver.run()" in patched, "PitexApp.swift hook point changed"
    app_copy = root / "PitexApp.swift"
    app_copy.write_text(patched)
    production_sources = [p for p in (repo / "Mac/Sources").rglob("*.swift") if p != app_main]
    equation = repo / "Mac/Sources/Features/EquationPreview.swift"
    if hover_diagnostic:
        equation_copy = root / "EquationPreview.swift"
        equation_copy.write_text(traced_equation_source(equation.read_text()))
        production_sources = [equation_copy if p == equation else p for p in production_sources]
    source = root / "Check.swift"
    source.write_text(check)
    if out_dir:
        generated = out_dir / "generated-sources"
        for compiler_input in (app_copy, equation_copy if hover_diagnostic else equation, source):
            retain_evidence(compiler_input, generated / compiler_input.name)
    binary = root / "check-bin"
    # Production language mode — strict Swift 6, Xcode-default diagnostics.
    # Any sendable-isolation error in product sources is a real blocker the
    # harness must surface; never downgrade language mode or suppress checks.
    arch = subprocess.run(["uname", "-m"], capture_output=True, text=True,
                          check=True).stdout.strip()
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", f"{arch}-apple-macos15.0", "-I", str(products),
                    str(source), str(app_copy),
                    *[str(p) for p in production_sources],
                    *[str(p) for p in products.glob("*.o")], "-o", str(binary)], check=True)
    # Strict-compile gate without a UI slot: the same swiftc invocation,
    # then stop before bundling/launching anything.
    if os.environ.get("PITEX_CHECK_COMPILE_ONLY") == "1":
        print(f"COMPILE_OK strict Swift 6 checker binary {binary.stat().st_size} bytes")
        sys.exit(0)

    # Isolated bundle: production Info.plist + unique id + real resources
    # including the equation-preview MathJax tree.
    identifier = f"test.pitex.eqpreview.{os.getpid()}"
    app = root / "EquationPreviewCheck.app"
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    resources = contents / "Resources"
    resources.mkdir()
    plist = plistlib.loads((repo / "Mac/Config/Info.plist").read_bytes())
    plist.update({
        "CFBundleExecutable": "check",
        "CFBundleIdentifier": identifier,
        "CFBundleName": "EquationPreviewCheck",
        "CFBundleDisplayName": "EquationPreviewCheck",
        "CFBundleDevelopmentRegion": "en",
        "CFBundleShortVersionString": "0",
        "CFBundleVersion": "0",
        "LSMinimumSystemVersion": "15.0",
    })
    (contents / "Info.plist").write_bytes(plistlib.dumps(plist))
    for locale in (repo / "Mac/Resources").glob("*.lproj"):
        shutil.copytree(locale, resources / locale.name)
    shutil.copy2(repo / "Mac/Resources/markdown-preview.html", resources)
    shutil.copytree(repo / "Mac/Resources/PitexAgent", resources / "PitexAgent")
    # Real bundled renderer: the scheme handler serves this exact tree.
    shutil.copytree(repo / "Assets/equation-preview", resources / "equation-preview")
    pi_bin = resources / "pi-runtime/bin"
    pi_bin.mkdir(parents=True)
    fake_pi = pi_bin / "pi"
    fake_pi.write_text("#!/bin/sh\nexit 0\n")
    fake_pi.chmod(0o755)
    shutil.copy(binary, contents / "MacOS/check")
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                    "--identifier", identifier, str(app)], check=True)
    describe = subprocess.run(["/usr/bin/codesign", "-d", "--verbose=2", str(app)],
                              check=True, capture_output=True, text=True)
    signed = re.search(r"^Identifier=(.*)$", describe.stderr, re.M)
    print(f"[diag] codesign {app.name}: {signed.group(1) if signed else 'unknown'}")
    assert signed and signed.group(1) == identifier

    for key in ("pitex.pref.editor.restoreSession", "pitex.pref.update.autoInstall"):
        subprocess.run(["/usr/bin/defaults", "write", identifier, key,
                        "-bool", "false"], check=True)

    # Private per-run bridge directory: inside the 0700 temporary root, 0700
    # itself, owned by us, not a symlink — verified, not assumed.
    bridge_dir = root / "hid-bridge"
    bridge_dir.mkdir(mode=0o700)
    for private in (root, bridge_dir):
        info = private.lstat()
        assert stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid() \
            and stat.S_IMODE(info.st_mode) & 0o077 == 0, f"{private} is not a private directory"
    hid = HidBridge(bridge_dir, app)
    print(f"[bridge] launcher post-event access={hid.access} python={sys.executable} "
          f"ancestry={process_ancestry()}", flush=True)
    # PITEX_CHECK_HID_PREFLIGHT=1: stop right after the caret pre-flight and
    # the first bridged cursor move (arrival + app-event proof).
    preflight = os.environ.get("PITEX_CHECK_HID_PREFLIGHT") == "1"

    out_log, err_log = root / "check.out.log", root / "check.err.log"
    command = ["/usr/bin/open", "-W",
               "--stdout", str(out_log), "--stderr", str(err_log),
               "--env", "PI_AGENT_PATH=/usr/bin/false",
               "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
               "--env", f"PITEX_CHECK_FIXTURE={fixture}",
               "--env", f"PITEX_CHECK_OUT={shots}",
               "--env", f"PITEX_CHECK_HID_BRIDGE={bridge_dir}",
               "--env", f"PITEX_CHECK_HID_PREFLIGHT={'1' if preflight else '0'}",
               "--env", f"PITEX_CHECK_HOVER_DIAGNOSTIC={'1' if hover_diagnostic else '0'}",
               "--env", f"PITEX_CHECK_ESCAPE_DIAGNOSTIC={'1' if escape_diagnostic else '0'}",
               str(app)]
    try:
        # The launcher serves bridge requests while `open -W` waits.
        launched = subprocess.Popen(command)
        deadline = time.monotonic() + 420
        while launched.poll() is None:
            if time.monotonic() > deadline:
                launched.kill()
                raise subprocess.TimeoutExpired(command, 420)
            hid.service()
            time.sleep(0.002)
        if launched.returncode:
            raise subprocess.CalledProcessError(launched.returncode, command)
    except subprocess.SubprocessError as error:
        print(f"FAIL: launcher raised {error}")
    finally:
        subprocess.run(["/usr/bin/pkill", "-f", str(app)], check=False)
        print(f"[bridge] served {hid.last} request(s)")
        for log in (out_log, err_log):
            if log.exists():
                print(f"--- {log.name} ---")
                print(log.read_text(errors="replace"), end="")
    output = out_log.read_text(errors="replace") if out_log.exists() else ""
    if out_dir:
        for log in (out_log, err_log):
            retain_evidence(log, out_dir / log.name)
        # Full and diagnostic runs retain the exact owned bundle, fixture
        # and compiler inputs before TemporaryDirectory cleanup, even when
        # the functional checker fails. Retention errors never alter exit.
        retain_evidence(app, out_dir / app.name)
        retain_evidence(fixture, out_dir / "fixture")
        print(f"[diag] artifacts: {out_dir}; missing copies={len(evidence_errors)}")
    if preflight:
        sys.exit(0 if "HID_PREFLIGHT_OK" in output else "FAIL: no HID_PREFLIGHT_OK")
    if escape_diagnostic:
        sys.exit(0 if "ESCAPE_DIAGNOSTIC_OK" in output else "FAIL: no ESCAPE_DIAGNOSTIC_OK")
    if hover_diagnostic:
        sys.exit(0 if "HOVER_DIAGNOSTIC_OK" in output else "FAIL: no HOVER_DIAGNOSTIC_OK")
    if "CHECK_COMPLETE" not in output:
        sys.exit("FAIL: no CHECK_COMPLETE")
