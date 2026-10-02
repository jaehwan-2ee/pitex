#!/usr/bin/env python3
"""After building Pitex: python3 Tools/check-macos-synctex.py <Build/Products/Debug>

Runs the production PDF/editor coordinators and SyncTeXRunner against a real
three-page document, including /tmp's macOS alias and an included source file.
"""
from pathlib import Path
import gzip
import hashlib
import posixpath
import re
MARKER = re.compile(r"/Check\.swift:\d+:\d+: warning: (?:no 'async' operations occur within 'await' expression|no calls to throwing functions occur within 'try' expression)(?: \[#UnnecessaryEffectMarker\])?$", re.M)
import subprocess
import shutil
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
features = repo / "Mac/Sources/Features"

# Swift: records every bridge post so the run can FAIL on error traffic
# product code only logs; the compiled Preview.swift copy is patched to
# call BridgeProbe.log at the message-handler entry.
BRIDGE_ENUM = '''
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
'''

check = r'''
@main
struct SyncTeXCheck {
    @MainActor static func main() {
        // The WKWebView needs a real app object and run loop for its
        // rendering/IPC; app.run() services both while run() awaits.
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            do {
                try await run()
                let __e = BridgeProbe.audit()
                precondition(__e.isEmpty, __e)
                fflush(nil)
                exit(0)
            } catch {
                print("FAIL", error); fflush(nil); exit(1)
            }
        }
        app.run()
    }

    @MainActor static func run() async throws {
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let pdf = root.appendingPathComponent("main.pdf")
        let runner = SyncTeXRunner()
        let binding = try await runner.refreshBinding(projectRoot: root, pdfURL: pdf, buildID: "check",
                                                      mainRelativePath: "main.tex")

        // The byte-level Input scan must produce exactly what decoding the
        // whole .synctex and filtering the lines produced before.
        let metadata = try Data(contentsOf: root.appendingPathComponent("inputs-check.bin"))
        let scanned = SyncTeXRunner.inputLines(metadata)
        let reference = String(decoding: metadata, as: UTF8.self)
            .split(separator: "\n").filter { $0.hasPrefix("Input:") }.joined(separator: "\n")
        precondition(scanned == reference, "Input scan diverged from the decoded filter")
        print("PASS Input scan: \(metadata.count) metadata bytes -> \(scanned.utf8.count) filtered bytes")

        // Real production view stack: PDFDocumentView's own makeNSView wires
        // the pdf.js WKWebView, scheme handler, coordinator and JS bridge —
        // the same code that runs in the app (concatenated into this file).
        // The payload resolves through Bundle.main/resourceURL/pdfjs, which
        // the harness copies beside this executable.
        let document = PDFDocument(url: pdf)!
        let docData = try Data(contentsOf: pdf)
        // Y1: no public NSViewRepresentableContext init — host the real
        // representable in an NSHostingView and drive it through its normal
        // inputs: rootView reassignment toggles highlightSync (updateNSView's
        // same-target fast path still sets coordinator.highlightSync → didSet
        // → push); forward sync rides the real notification, filtered by the
        // workspace object identity like the production observer.
        let workspaceToken = NSObject()
        var hit: (Int, SyncTeXCore.PDFPoint)?
        func makeView(_ hl: Bool) -> PDFDocumentView {
            PDFDocumentView(data: docData, target: "synctex-check",
                            workspace: workspaceToken, highlightSync: hl,
                            onInverseSync: { p, pt in hit = (p, pt) })
        }
        let hosting = NSHostingView(rootView: makeView(false))
        hosting.frame = NSRect(x: 0, y: 0, width: 600, height: 700)
        let window = NSWindow(contentRect: hosting.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = hosting
        window.makeKeyAndOrderFront(nil)
        NSApp.activate()
        hosting.layoutSubtreeIfNeeded()
        func descendants(_ v: NSView) -> [NSView] { [v] + v.subviews.flatMap(descendants) }
        var view: WKWebView?
        for _ in 0..<100 where view == nil {
            view = descendants(hosting).compactMap { $0 as? WKWebView }
                .first { $0.url?.scheme == "pitex-pdfjs" }
            if view == nil { try await Task.sleep(for: .milliseconds(50)) }
        }
        precondition(view != nil, "pdf.js web view never mounted")

        func js(_ body: String, arguments: [String: Any] = [:]) async throws -> Any? {
            try await view!.callAsyncJavaScript(body, arguments: arguments, in: nil, contentWorld: .page)
        }
        /// Viewer ready + this document loaded and its first page rendered.
        /// All probes go through window.pitex — the only product surface.
        var ready = false
        var lastProbe = "none"
        let readyDeadline = ContinuousClock.now + .seconds(20)
        while !ready && .now < readyDeadline {
            do {
                let r = try await js("""
                    return (() => {
                      if (!window.pitex) return {e:"no-pitex"};
                      const g = window.pitex.state();
                      const p = g >= 0 ? window.pitex.position(g) : null;
                      return {g, page: p?.page ?? -1,
                              canvases: document.querySelectorAll(".page canvas").length}; })();
                    """) as? [String: Any]
                lastProbe = String(describing: r ?? ["none": 1])
                ready = (r?["g"] as? Int ?? -1) >= 0
                    && (r?["page"] as? Int ?? -1) >= 1
                    && (r?["canvases"] as? Int ?? -1) >= 1
            } catch { lastProbe = "error: \(error)" }
            if !ready { try await Task.sleep(for: .milliseconds(50)) }
        }
        precondition(ready, "pdf.js viewer never rendered the fixture document "
                     + "(last probe: \(lastProbe))")

        func click(_ location: NSPoint, in window: NSWindow, command: Bool = true) -> NSEvent {
            NSEvent.mouseEvent(with: .leftMouseDown, location: location,
                modifierFlags: command ? .command : [], timestamp: 0,
                windowNumber: window.windowNumber, context: nil,
                eventNumber: 1, clickCount: 1, pressure: 1)!
        }

        /// Map SyncTeX (x, v-from-top) → the viewer's client point via the
        /// overlay-free path: pdf.js scrollPageIntoView accepts user-space
        // dest coords; the word's own position comes back from the DOM.
        for (word, file, line) in [("RootTarget", "main.tex", 105), ("ChildTarget", "sections/child.tex", 102)] {
            let selection = document.findString(word, fromSelection: nil, withOptions: [])!
            let page = selection.pages[0]
            let pageNumber = document.index(for: page) + 1
            let rect = selection.bounds(for: page)  // user space, y-up
            for scale in [0.75, 1.5] {
                // Absolute scale via the product's own zoom entry point.
                let applied = try await js("""
                    return (() => { const g = window.pitex.state();
                      const cur = window.pitex.position(g).scale;
                      return window.pitex.zoomBy(\(scale) / cur); })();
                    """) as? Double
                precondition(abs((applied ?? 0) - scale) < 0.001, "zoomBy did not reach \(scale)")
                // Y3: setView lands on the PAGE top; at 1.5x the word can be
                // below the 700px viewport → scroll the WORD into view
                // checker-locally (container.scrollTop from the word's
                // projected client rect), clamping at the scroll bounds.
                _ = try await js("""
                    return (() => { const g = window.pitex.state();
                      return window.pitex.setView(g, \(scale), \(pageNumber)); })();
                    """)
                try await Task.sleep(for: .milliseconds(200))
                _ = try await js("""
                    return (() => {
                      const pageEl = document.querySelector(
                        '.page[data-page-number="\(pageNumber)"]');
                      if (!pageEl) return false;
                      const r = pageEl.querySelector(".canvasWrapper").getBoundingClientRect();
                      const c = document.getElementById("viewerContainer");
                      const s = r.height / \(page.bounds(for: .mediaBox).maxY);
                      // Click mappings assume MediaBox origin 0 (valid for
                      // these TeX fixtures; flagged by review).
                      const wy = r.top + (\(page.bounds(for: .mediaBox).maxY)
                                          - \(rect.midY)) * s;
                      const wx = r.left + \(rect.midX) * s;
                      const cR = c.getBoundingClientRect();
                      c.scrollTop += wy - (cR.top + cR.height / 2);
                      c.scrollLeft += wx - (cR.left + cR.width / 2);
                      return true; })();
                    """)
                try await Task.sleep(for: .milliseconds(200))
                let probe = try await js("""
                    return (() => {
                      const g = window.pitex.state();
                      const pageEl = document.querySelector('.page[data-page-number="\(pageNumber)"]');
                      const wrap = pageEl.querySelector(".canvasWrapper");
                      const r = wrap.getBoundingClientRect();
                      const c = document.getElementById("viewerContainer").getBoundingClientRect();
                      // MediaBox top in user units == page pixel height /
                      // scale: derive scale from the canvas vs viewBox.
                      const canvas = wrap.querySelector("canvas");
                      const pageH = \(page.bounds(for: .mediaBox).maxY);
                      const scale = r.height / pageH;
                      const x = r.left + \(rect.midX) * scale;
                      const y = r.top + (pageH - \(rect.midY)) * scale;
                      return {x, y, inside: y > c.top && y < c.bottom
                              && x > c.left && x < c.right}; })();
                    """) as? [String: Any]
                precondition(probe?["inside"] as? Bool == true,
                             "checker scroll left the target off-screen")
                // Freeze fix 1: WKWebView.isFlipped is YES on macOS, so the
                // client y (top-down) is already view-space. Mirror only when
                // the view is NOT flipped — the runtime value, not an assume.
                let clientY = CGFloat(probe!["y"] as! Double)
                let viewPoint = NSPoint(x: probe!["x"] as! Double,
                                        y: view!.isFlipped ? clientY
                                                           : view!.bounds.height - clientY)
                precondition(view!.bounds.contains(viewPoint), "Cmd-click target must be visible")
                // Sanity: the checker-side inverse at that client point
                // (elementFromPoint + linear user-space map — same math
                // the page's mousedown path performs) must land on the
                // word's page before we post the event.
                let inv = try await js("""
                    return (() => {
                      const el = document.elementFromPoint(\(probe!["x"] as! Double),
                                                           \(probe!["y"] as! Double));
                      const pageEl = el && el.closest && el.closest(".page");
                      if (!pageEl) return null;
                      const r = pageEl.querySelector(".canvasWrapper").getBoundingClientRect();
                      const s = r.height / \(page.bounds(for: .mediaBox).maxY);
                      const cx = \(probe!["x"] as! Double), cy = \(probe!["y"] as! Double);
                      return {page: parseInt(pageEl.dataset.pageNumber, 10),
                              x: (cx - r.left) / s,
                              y: \(page.bounds(for: .mediaBox).maxY) - (cy - r.top) / s}; })();
                    """) as? [String: Any]
                precondition((inv?["page"] as? Int) == pageNumber,
                             "projected client point does not resolve on the word's page")
                let location = view!.convert(viewPoint, to: nil)
                // Plain click must not sync; Cmd-click must.
                hit = nil
                NSApp.sendEvent(click(location, in: window, command: false))
                try await Task.sleep(for: .milliseconds(150))
                precondition(hit == nil, "inverse fired without the Command modifier")
                // X4: delivered-point probe — capture-phase once listener
                // records the client point the page actually received.
                _ = try await js("""
                    window.__probeDown = null;
                    document.addEventListener("mousedown", e => {
                      window.__probeDown = {x: e.clientX, y: e.clientY, meta: e.metaKey};
                    }, {capture: true, once: true});
                    return "armed";
                    """)
                NSApp.sendEvent(click(location, in: window))
                let clickDeadline = ContinuousClock.now + .seconds(5)
                while hit == nil && .now < clickDeadline {
                    try await Task.sleep(for: .milliseconds(50))
                }
                let delivered = (try? await js("return window.__probeDown || null;"))
                    as? [String: Any]
                precondition(hit != nil,
                    "inverse never fired on Cmd-click; isFlipped=\(view!.isFlipped) "
                    + "intendedClient=(\(probe!["x"] as! Double),\(probe!["y"] as! Double)) "
                    + "delivered=\(String(describing: delivered))")
                let (number, pdfPoint) = hit!
                let inverse = try await runner.inverse(binding: binding, page: number, point: pdfPoint)
                precondition(inverse.source.path.value == file && inverse.source.line == line,
                    "inverse round trip mismatched: got \(inverse.source.path.value):\(inverse.source.line) "
                    + "want \(file):\(line) | isFlipped=\(view!.isFlipped) "
                    + "intendedClient=(\(probe!["x"] as! Double),\(probe!["y"] as! Double)) "
                    + "delivered=\(String(describing: delivered))")

                let forward = try await runner.forward(binding: binding,
                    sourceURL: root.appendingPathComponent(file), line: line, column: 0)
                precondition(forward.pdf.page == number)
                // Y2a — data side (unchanged contract): the forward box,
                // inset by −8pt (the F2 padding), contains the word's mid.
                let boxMinY = page.bounds(for: .mediaBox).maxY - forward.v
                let fwBox = NSRect(x: forward.h - 8,
                                   y: boxMinY - 8,
                                   width: max(forward.width, 4) + 16,
                                   height: max(forward.height, 4) + 16)
                precondition(fwBox.contains(NSPoint(x: rect.midX, y: rect.midY)),
                             "forward box must cover the inverse word (padded)")
                // Forward-sync highlight: post the REAL notification — the
                // production observer filters on `object: workspace` identity,
                // so the workspace token is the selector. Scroll runs on
                // every post; the overlay is gated by highlightSync.
                NotificationCenter.default.post(
                    name: .syncTeXHighlightRequested, object: workspaceToken,
                    userInfo: ["page": number, "x": forward.h, "y": forward.v,
                               "width": forward.width, "height": forward.height])
                try await Task.sleep(for: .milliseconds(200))
                // Scroll happens regardless of the highlight preference …
                let landed = try await js("""
                    return (() => { const g = window.pitex.state();
                      return window.pitex.position(g).page; })();
                    """) as? Int
                precondition(landed == number, "forward sync did not land on the target page")
                // Y2b — the F2 scroll contract: pdf.js XYZ aligns the
                // destination point with the container's TOP edge; the F2
                // destination is the box top + 8pt → container top must map
                // to user-Y maxY − v + max(h,4) + 8. Tolerance 1/s + 0.5 pt
                // (CSS-pixel quantization); a clamp is accepted ONLY when
                // scrollTop sits at 0/max (below-content page) and recorded.
                let scrollOK = try await js("""
                    return (() => {
                      const pageEl = document.querySelector(
                        '.page[data-page-number="\(number)"]');
                      if (!pageEl) return {ok:false};
                      const r = pageEl.querySelector(".canvasWrapper").getBoundingClientRect();
                      const c = document.getElementById("viewerContainer");
                      const cR = c.getBoundingClientRect();
                      const s = r.height / \(page.bounds(for: .mediaBox).maxY);
                      const top = \(page.bounds(for: .mediaBox).maxY) - (cR.top - r.top) / s;
                      const want = \(page.bounds(for: .mediaBox).maxY)
                                   - \(forward.v) + Math.max(\(forward.height), 4) + 8;
                      const ok = Math.abs(top - want) <= 1 / s + 0.5;
                      const clamped = c.scrollTop <= 0
                                   || c.scrollTop >= c.scrollHeight - c.clientHeight - 0.5;
                      return {ok: ok || clamped, exact: ok, clamped,
                              top, want, scrollTop: c.scrollTop}; })();
                    """) as? [String: Any]
                if (scrollOK?["exact"] as? Bool) != true {
                    print("NOTE forward scroll clamped:", scrollOK ?? [:])
                }
                precondition(scrollOK?["ok"] as? Bool == true,
                             "forward scroll target diverged from F2 (top≠box top+8)")
                // … but the overlay only exists when highlightSync is on.
                let overlayHidden = try await js(
                    "return document.querySelectorAll('.pitex-hl').length;") as? Int ?? -1
                precondition(overlayHidden == 0, "highlight must follow only the preference")
                // Toggle via the real SwiftUI input — updateNSView pushes
                // highlightSync into the coordinator (didSet → JS).
                // Settled barrier: after every rootView reassignment, force
                // updateNSView and poll the Coordinator's own highlightSync
                // until it equals the requested value (<=2s, fail-closed),
                // then a JS round-trip orders the didSet's setHighlightSync
                // push ahead of the post.
                hosting.rootView = makeView(true)
                var ok_hl = false
                for _ in 0..<40 {
                    hosting.layoutSubtreeIfNeeded()
                    if (view!.navigationDelegate as? PDFDocumentView.Coordinator)?
                        .highlightSync == true { ok_hl = true; break }
                    try? await Task.sleep(for: .milliseconds(50))
                }
                precondition(ok_hl,
                    "SwiftUI update barrier not observed "
                    + "(coordinator.highlightSync="
                    + "\(String(describing: (view!.navigationDelegate as? PDFDocumentView.Coordinator)?.highlightSync)))")
                _ = try await js("return 1;")
                NotificationCenter.default.post(
                    name: .syncTeXHighlightRequested, object: workspaceToken,
                    userInfo: ["page": number, "x": forward.h, "y": forward.v,
                               "width": forward.width, "height": forward.height])
                // Y2c — the overlay client rect must equal the viewport
                // mapping of the (unpadded) box corners ±1 px, including the
                // product's 4-CSS-px minimum-size clamp — centre checks are
                // too loose (r5's scrolled-out regression).
                let overlayDeadline = ContinuousClock.now + .seconds(5)
                var overlayOK = false
                var lastObs = "no-observation"
                var lastOverlayErr = "none"
                while !overlayOK && .now < overlayDeadline {
                    do {
                        let obs = try await js("""
                            return (() => {
                              const marks = document.querySelectorAll(".pitex-hl");
                              const c = document.getElementById("viewerContainer")
                                            .getBoundingClientRect();
                              const r0 = document.querySelector(
                                '.page[data-page-number="\(number)"]');
                              const r = r0 ? r0.querySelector(".canvasWrapper")
                                           .getBoundingClientRect() : null;
                              const s = r ? r.height / \(page.bounds(for: .mediaBox).maxY) : 0;
                              const L = r ? r.left + \(forward.h) * s : 0;
                              const T = r ? r.top + (\(forward.v) - \(forward.height)) * s : 0;
                              const R = L + Math.max(\(forward.width) * s, 4);
                              const B = T + Math.max(\(forward.height) * s, 4);
                              let e = null, pg = null;
                              if (marks.length === 1) {
                                const el = marks[0].getBoundingClientRect();
                                e = {l:el.left,t:el.top,r:el.right,b:el.bottom};
                                const pe = marks[0].closest(".page");
                                pg = pe ? parseInt(pe.dataset.pageNumber,10) : null;
                              }
                              return {marks:marks.length, page:pg, e, L,T,R,B,s,
                                      cL:c.left,cR:c.right,cT:c.top,cB:c.bottom};
                            })();
                            """) as? [String: Any]
                        lastObs = obs.map { String(describing: $0) } ?? "nil"
                        lastOverlayErr = "none"
                        if let o = obs {
                            let marks = o["marks"] as? Int ?? -1
                            let pg = o["page"] as? Int
                            let e = o["e"] as? [String: Double]
                            let L = o["L"] as? Double ?? 0, T = o["T"] as? Double ?? 0
                            let R = o["R"] as? Double ?? 0, B = o["B"] as? Double ?? 0
                            let cT = o["cT"] as? Double ?? 0, cB = o["cB"] as? Double ?? 0
                            let cL = o["cL"] as? Double ?? 0, cR = o["cR"] as? Double ?? 0
                            // identical ±1 px predicate, computed in Swift.
                            overlayOK = marks == 1 && pg == number
                                && e != nil
                                && e!["b"]! > cT && e!["t"]! < cB
                                && e!["r"]! > cL && e!["l"]! < cR
                                && abs(e!["l"]! - L) <= 1 && abs(e!["t"]! - T) <= 1
                                && abs(e!["r"]! - R) <= 1 && abs(e!["b"]! - B) <= 1
                        }
                    } catch {
                        lastOverlayErr = "\(error)"
                    }
                    if !overlayOK { try await Task.sleep(for: .milliseconds(50)) }
                }
                precondition(overlayOK,
                    "forward marker rect must match the SyncTeX box in the viewport; "
                    + "lastObs=\(lastObs) lastErr=\(lastOverlayErr)")
                hosting.rootView = makeView(false)
                var ok_hl_off = false
                for _ in 0..<40 {
                    hosting.layoutSubtreeIfNeeded()
                    if (view!.navigationDelegate as? PDFDocumentView.Coordinator)?
                        .highlightSync == false { ok_hl_off = true; break }
                    try? await Task.sleep(for: .milliseconds(50))
                }
                precondition(ok_hl_off,
                    "SwiftUI update barrier not observed (off) "
                    + "(coordinator.highlightSync="
                    + "\(String(describing: (view!.navigationDelegate as? PDFDocumentView.Coordinator)?.highlightSync)))")
                _ = try await js("return 1;")
                print("PASS PDF/editor round trip: \(file):\(line), zoom \(scale)")
            }
        }

        let text = NSTextView()
        text.string = try String(contentsOf: root.appendingPathComponent("main.tex"), encoding: .utf8)
        text.font = .monospacedSystemFont(ofSize: 13, weight: .regular)
        text.textContainerInset = NSSize(width: 66, height: 12)
        text.isVerticallyResizable = true
        text.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        text.autoresizingMask = [.width]
        text.textContainer?.widthTracksTextView = true
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 400, height: 140))
        scroll.documentView = text
        window.contentView = scroll
        scroll.layoutSubtreeIfNeeded()
        let editor = EditorContainerView.Coordinator()
        editor.textView = text
        var sourceHit: (Int, Int)?
        editor.onSyncRequest = { sourceHit = ($0, $1) }
        let range = (text.string as NSString).range(of: "RootTarget")
        text.layoutManager!.ensureLayout(for: text.textContainer!)
        text.scrollRangeToVisible(range)
        let glyphs = text.layoutManager!.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
        let bounds = text.layoutManager!.boundingRect(forGlyphRange: glyphs, in: text.textContainer!)
            .offsetBy(dx: text.textContainerOrigin.x, dy: text.textContainerOrigin.y)
        for fraction in [0.2, 0.8] {
            let location = text.convert(NSPoint(x: bounds.midX, y: bounds.minY + bounds.height * fraction), to: nil)
            precondition(!editor.handleSyncClick(click(location, in: window, command: false)))
            precondition(editor.handleSyncClick(click(location, in: window)))
            precondition(sourceHit?.0 == 105, "Editor inset must not shift the clicked source line")
        }
        print("PASS editor Cmd-click: scrolled text, gutter inset, upper/lower glyph halves")

        // A live build's PDF hides under .pitex-live — the binding must
        // still anchor recorded inputs on the source tree, not the PDF's
        // hidden directory. The fixture was compiled with -output-directory
        // so its .synctex sits beside the isolated PDF.
        let livePdf = root.appendingPathComponent(".pitex-live/main/main.pdf")
        let liveBinding = try await runner.refreshBinding(
            projectRoot: root, pdfURL: livePdf, buildID: "live",
            mainRelativePath: "main.tex")
        precondition(liveBinding.sourcePaths.values.contains {
            $0 == root.appendingPathComponent("main.tex").resolvingSymlinksInPath().standardizedFileURL
        }, "live binding must map the main source, not the hidden output dir")
        precondition(liveBinding.sourcePaths.values.contains {
            $0 == root.appendingPathComponent("sections/child.tex").resolvingSymlinksInPath().standardizedFileURL
        }, "live binding must map the included child source")
        let liveForward = try await runner.forward(
            binding: liveBinding, sourceURL: root.appendingPathComponent("sections/child.tex"),
            line: 102, column: 0)
        precondition(liveForward.pdf.page >= 1, "live forward sync must resolve a page")
        print("PASS live binding: .pitex-live output maps inputs to the source tree")

        // Synthetic relative-path variant: the fixture's absolute Input
        // records were rewritten to ./-relative form (see the Python
        // fixture below — actual xelatex here records absolute paths, so
        // this exercises the relative-anchoring path deliberately). The
        // synctex CLI must resolve them against the binding's sourceRoot
        // (the project root for a top-level main), never the PDF's
        // .pitex-live directory.
        let relPdf = livePdf.deletingLastPathComponent().appendingPathComponent("relmain.pdf")
        let relBinding = try await runner.refreshBinding(
            projectRoot: root, pdfURL: relPdf, buildID: "rel",
            mainRelativePath: "main.tex")
        precondition(relBinding.sourceRoot.standardizedFileURL
            == root.resolvingSymlinksInPath().standardizedFileURL,
            "top-level main anchors sourceRoot at the project root")
        // The rewrite must have produced relative recorded keys — if the
        // transformation silently did nothing this check cannot pass.
        precondition(relBinding.sourcePaths.keys.filter { !$0.hasPrefix("/") }.count >= 2,
            "synthetic fixture must surface relative source keys for main and child")
        let relForward = try await runner.forward(
            binding: relBinding, sourceURL: root.appendingPathComponent("sections/child.tex"),
            line: 102, column: 0)
        precondition(relForward.pdf.page >= 1,
            "relative inputs must resolve against the source directory, not the PDF dir")
        let relInverse = try await runner.inverse(
            binding: relBinding, page: relForward.pdf.page,
            point: SyncTeXCore.PDFPoint(x: relForward.h, y: relForward.v))
        precondition(relInverse.source.path.value == "sections/child.tex",
            "relative inverse input must map back under the project root")
        print("PASS relative inputs: forward+inverse anchored on sourceRoot, not PDF dir")
        window.orderOut(nil)
    }
}
'''

with tempfile.TemporaryDirectory(prefix="pitex-synctex-", dir="/tmp") as directory:
    root = Path(directory)
    (root / "sections").mkdir()
    (root / "main.tex").write_text(
        "\\documentclass{article}\n\\begin{document}\nFirst page.\\par\n\\newpage\n"
        + "\n" * 100 + "RootTarget\\par\n\\newpage\n\\input{sections/child}\n\\end{document}\n"
    )
    (root / "sections/child.tex").write_text("% child\n" + "\n" * 100 + "ChildTarget\\par\n")
    subprocess.run(["/Library/TeX/texbin/xelatex", "-synctex=1", "-interaction=nonstopmode",
                    "-halt-on-error", "main.tex"], cwd=root, check=True, stdout=subprocess.DEVNULL)
    # The same project compiled into the isolated live output dir — the
    # runner must map its .synctex inputs to the real source tree.
    live_dir = root / ".pitex-live" / "main"
    live_dir.mkdir(parents=True)
    subprocess.run(["/Library/TeX/texbin/xelatex", "-synctex=1", "-interaction=nonstopmode",
                    "-halt-on-error", "-output-directory", str(live_dir), "main.tex"],
                   cwd=root, check=True, stdout=subprocess.DEVNULL)
    # Synthetic relative-path variant: rewrite the live build's absolute
    # Input records to ./-relative form so the runner's sourceRoot anchor
    # is exercised end-to-end. This is a fabricated fixture — the actual
    # xelatex on this toolchain records absolute cwd-prefixed Inputs —
    # but relative records are the failure mode the mapper/runner must
    # still handle (older engines, relocated remote metadata).
    live_meta = gzip.decompress((live_dir / "main.synctex.gz").read_bytes()).decode()
    # xelatex canonicalizes /tmp to /private/tmp on macOS — accept the
    # recorded path under either the lexical or the resolved root.
    # Recorded suffixes may carry "/./" (invocation-dir marker), so the
    # stripped remainder is normalized before it becomes a ./-relative
    # record; a suffix that escapes (../) is left absolute rather than
    # fabricating a path that means something else.
    rel_meta = live_meta
    total = 0

    def relativize(match):
        global total
        suffix = posixpath.normpath(match.group(2))
        if suffix.startswith("../") or suffix == ".." or suffix.startswith("/"):
            return match.group(0)
        total += 1
        return f"Input:{match.group(1)}:./{suffix}"

    for recorded_root in {str(root), str(root.resolve())}:
        rel_meta = re.sub(r"^Input:(\d+):" + re.escape(recorded_root) + r"/(.+)",
                          relativize, rel_meta, flags=re.M)
    rel_inputs = {l.split(":", 2)[2] for l in rel_meta.splitlines() if l.startswith("Input:")}
    assert total >= 2 and "./main.tex" in rel_inputs and "./sections/child.tex" in rel_inputs, (
        f"expected main+child rewritten to ./-relative, got {total} subs: {sorted(rel_inputs)}")
    (live_dir / "relmain.synctex.gz").write_bytes(gzip.compress(rel_meta.encode()))
    # The PDF must be THIS compile's output — the root build's main.pdf
    # is a different run's bytes.
    (live_dir / "relmain.pdf").write_bytes((live_dir / "main.pdf").read_bytes())
    # A decompressed copy lets the check compare the byte-level Input scan
    # against the old decode-then-filter expression. The name must not be
    # <name>.synctex — the runner and the synctex CLI pick that over the .gz.
    (root / "inputs-check.bin").write_bytes(gzip.decompress((root / "main.synctex.gz").read_bytes()))
    # Keep the private coordinators in the same compilation unit as the
    # check. Preview.swift contributes everything from PDFDocumentView on
    # (incl. MarkdownPreviewView -> needs WebKit). EditorContainerView is
    # sliced before its agent-runtime tail — GhostCompletionCoordinator
    # pulls PiAgentProcess/AgentCoordinator (and thereby WorkspaceModel);
    # the harness only exercises Coordinator.handleSyncClick, so a stub
    # covers the four members the editor Coordinator touches.
    preview = (features / "Preview.swift").read_text()
    _bridge_anchor = ('            guard let m = message.body as? [String: Any],\n'
                      '                  let type = m["type"] as? String else { return }')
    assert preview.count(_bridge_anchor) == 1
    preview = preview.replace(_bridge_anchor,
        _bridge_anchor + '\n            BridgeProbe.log(type, m)')
    editor = (features / "EditorContainerView.swift").read_text()
    editor = editor[:editor.index("/// Copilot-style inline LaTeX completion")]
    ghost_stub = '''
/// Harness stand-in for the sliced-out agent coordinator — only the
/// members EditorContainerView.Coordinator reads exist here.
@MainActor
final class GhostCompletionCoordinator {
    weak var overlay: GhostCompletionOverlayView?
    private(set) var suggestion: String?
    func accept() -> Bool { false }
    func dismiss() {}
}
'''
    (root / "Check.swift").write_text(
        "import AppKit\nimport EditorMacAdapter\nimport LanguageCore\nimport PDFKit\n"
        + "import SwiftUI\nimport SyncTeXCore\nimport TexDomain\nimport WebKit\n"
        + BRIDGE_ENUM
        + preview[preview.index("private struct PDFDocumentView:"):]
        + editor + ghost_stub + check
    )
    modules = ["BuildCore", "SyncTeXCore", "TexDomain", "DocumentSessionCore", "ProjectCore",
               "EditorMacAdapter", "EditorFeature", "AppPorts", "LanguageCore", "AICore"]
    # SyntaxHighlighting.swift provides the real EditorAnalysis the
    # editor Coordinator uses for line math — production source, not a
    # stub. Its AppearanceSettings dependency comes from
    # AppearanceTheme.swift.
    sources = ["SyncTeXSupport.swift", "EditorFolding.swift",
               "AppearanceTheme.swift", "SyntaxHighlighting.swift"]
    executable = root / "check"
    _cc = subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", "arm64-apple-macos15.0", "-module-cache-path", str(root / "cache"),
                    "-I", str(products), str(root / "Check.swift"),
                    *[str(features / name) for name in sources],
                    *[str(products / (name + ".o")) for name in modules],
                    "-o", str(executable)], check=False, capture_output=True, text=True)
    print(_cc.stdout + _cc.stderr, end="")
    if _cc.returncode != 0:
        sys.exit(f"FAIL: swiftc rc={_cc.returncode} — no functional verdict")
    _bad = MARKER.findall(_cc.stderr + _cc.stdout)
    if _bad:
        for line in _cc.stderr.splitlines() + _cc.stdout.splitlines():
            if MARKER.search(line): print(line)
        sys.exit("FAIL: harness compile defect — UnnecessaryEffectMarker in "
                 "generated Check.swift (discarded async query); no functional verdict")
    # The pdf.js payload must sit beside the executable — an unbundled
    # Bundle.main.resourceURL is the executable's directory, which is where
    # PDFJSSchemeHandler resolves `pdfjs/`. Verified against the manifest
    # so a missing/stale payload fails loudly, not as a blank preview.
    pdfjs_dest = root / "pdfjs"
    shutil.copytree(repo / "Mac/Resources/pdfjs", pdfjs_dest)
    manifest = [l.split(None, 1) for l in (pdfjs_dest / "MANIFEST.sha256").read_text().splitlines() if l.strip()]
    assert manifest, "pdfjs MANIFEST.sha256 empty"
    bad = [p for h, p in manifest
           if hashlib.sha256((pdfjs_dest / p.lstrip("./")).read_bytes()).hexdigest() != h]
    assert not bad, f"pdfjs payload diverged from MANIFEST.sha256: {bad}"
    subprocess.run([str(executable), str(root)], check=True)
