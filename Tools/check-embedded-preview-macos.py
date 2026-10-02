#!/usr/bin/env python3
"""Native embedded-preview checks against the real PitexApp lifecycle.

Builds a checker .app from the actual Mac sources — production @main intact,
a test-only applicationDidFinishLaunching hook starts the driver — plus the
bundled pitex-preview helper payload staged under Contents/Helpers, then
exercises the embedded editing preview end to end on a real TeX Live
installation:

1. Unsaved edit → Editing-preview PDF carries the new text; disk source and
   final artifacts are untouched; status shows updating while typesetting.
2. Partial (complete:false) publications are accepted with no SyncTeX; the
   newest complete publication reflects the edit.
3. Edit then manual Build before the coalesced send: the final xelatex PDF
   wins; a later edit (one queued during the build included) still drafts.
4. Coherent preview binds SyncTeX; forward and inverse queries run against
   the session artifact; an unsaved diverging edit refuses stale mapping.
5. Open Externally writes a labeled copy of the displayed preview bytes;
   the detached preview renders the same artifact identity.
6. Backend switch to compiler stops the session: helper process group and
   session directory die; SIGSTOP'd helper leaves the UI responsive with a
   bounded writer; resume+stop cleans up.
7. CoreText late-font barrier: a fontspec document that loads a distinct
   system font on page 2 emits FNTB (driver.log font-barrier restart) and
   still produces the complete 2-page PDF whose extracted text matches a
   system-xelatex reference.
8. (B1) Type → publish → save: the next publication after the coalesced
   save still carries the typed marker.
9. (B2, second app run) Project opened through the /tmp spelling
   (/tmp → /private/tmp on Darwin): the helper's --root argv is the
   realpath root, an unsaved edit publishes, and the save regression
   holds under the alias.
Usage: check-embedded-preview-macos.py <Build/Products/Release>
                                     <helper-payload-dir> <source-checkout>
<helper-payload-dir> is the directory produced by
Tools/bundle-preview-engine-macos.sh (contains pitex-preview binaries + lib/).
<source-checkout> is the Pitex worktree path (Mac/Sources, Mac/Resources,
Config are read; nothing inside it is written).
"""
from pathlib import Path
import argparse
import hashlib
import json
import signal
import time
import os
import plistlib
import re
MARKER = re.compile(r"/Check\.swift:\d+:\d+: warning: (?:no 'async' operations occur within 'await' expression|no calls to throwing functions occur within 'try' expression)(?: \[#UnnecessaryEffectMarker\])?$", re.M)
import shutil
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("products", type=Path)
parser.add_argument("payload", type=Path)
parser.add_argument("repo", type=Path)
parser.add_argument("--scenario", choices=("all", "smoke", "full", "tmp-root"), default="all")
parser.add_argument("--artifacts", type=Path,
                    help="Owned evidence directory; captured before cleanup and each next launch.")
args = parser.parse_args()
products = args.products.resolve()
payload = args.payload.resolve()
repo = args.repo.resolve()
artifacts = args.artifacts.resolve() if args.artifacts else Path(tempfile.mkdtemp(prefix="pitex-embedded-evidence-"))
artifacts.mkdir(parents=True, exist_ok=True)
print(f"[diag] evidence directory {artifacts}", flush=True)

# An inherited CLT SDKROOT can disagree with modules built by pinned Xcode.
# Resolve both tools explicitly, then pass the same SDK to every native compile.
native_resolution = {
    "DEVELOPER_DIR": os.environ.get("DEVELOPER_DIR"),
    "inheritedSDKROOT": os.environ.get("SDKROOT"),
    "TOOLCHAINS": os.environ.get("TOOLCHAINS"),
    "commands": [],
}
for key, command in (
    ("swiftc", ["/usr/bin/xcrun", "--sdk", "macosx", "--find", "swiftc"]),
    ("sdk", ["/usr/bin/xcrun", "--sdk", "macosx", "--show-sdk-path"]),
    ("sdkVersion", ["/usr/bin/xcrun", "--sdk", "macosx", "--show-sdk-version"]),
):
    result = subprocess.run(command, capture_output=True, text=True, timeout=30)
    native_resolution["commands"].append({
        "argv": command, "exitCode": result.returncode,
        "stdout": result.stdout, "stderr": result.stderr,
    })
    native_resolution[key] = result.stdout.strip()
    (artifacts / "native-toolchain.json").write_text(json.dumps(native_resolution, indent=2) + "\n")
    result.check_returncode()
assert native_resolution["swiftc"] and Path(native_resolution["swiftc"]).is_file()
assert native_resolution["sdk"] and Path(native_resolution["sdk"]).is_dir()
native_environment = {**os.environ, "SDKROOT": native_resolution["sdk"]}
print(f"[diag] compiler={native_resolution['swiftc']} SDK={native_resolution['sdk']} version={native_resolution['sdkVersion']}", flush=True)

def verify_pdfjs_manifest(pdfjs_dir: Path):
    """Release-parity gate: the bundled payload must match MANIFEST.sha256."""
    lines = [l.split(None, 1) for l in (pdfjs_dir / "MANIFEST.sha256").read_text().splitlines() if l.strip()]
    assert lines, f"pdfjs manifest empty at {pdfjs_dir}"
    bad = [p for h, p in lines
           if hashlib.sha256((pdfjs_dir / p.lstrip("./")).read_bytes()).hexdigest() != h]
    assert not bad, f"pdfjs payload diverged from MANIFEST.sha256: {bad}"

pages = "\\par\\medskip\n".join(
    f"Typesetting filler paragraph {i} with enough content to keep the "
    f"engine busy so intermediate publications arrive before the pass "
    f"finishes. $\\int_0^{{{i}}} x^2\\,dx$ and a table row {i}."
    for i in range(1, 260))

fixture_main = (
    "\\documentclass{article}\n"
    "\\usepackage{amsmath}\n"
    "\\begin{document}\n"
    "\\title{Embedded Preview Fixture}\n\\maketitle\n"
    "\\section{Alpha}\nBaseLineMarker\\par\n" + pages +
    "\n\\input{tail}\n\\end{document}\n")
fixture_tail = "\\section{Omega}\nTailBaseMarker\\par\\newpage\nLastPageText\\par\n"

# fontspec: default Latin Modern (texmf-resolved) on page 1, then a
fixture_fonts = (
    "\\documentclass{article}\n\\usepackage{fontspec}\n"
    "\\begin{document}\n"
    "PageOneDefaultFont\\par\\newpage\n"
    "{\\fontspec{Menlo}PageTwoMenloFont\\par}\n"
    "\\end{document}\n")
# macOS system font loaded only on page 2 — the documented late-platform-font
# trigger for the CoreText barrier (Q_FNTB → driver restart).

check = r'''
import AppKit
import BuildFeature
import Darwin
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

enum LaunchFlag {
    nonisolated(unsafe) static var didFinishLaunching = false
    nonisolated(unsafe) static var finishUserInfo = "nil"
}

@MainActor enum Driver {
    static let root = ProcessInfo.processInfo.environment["PITEX_CHECK_FIXTURE"] ?? ""
    static let scenario = ProcessInfo.processInfo.environment["PITEX_CHECK_SCENARIO"] ?? "full"
    static var workspace: WorkspaceModel!
    static var project: URL { URL(fileURLWithPath: root).standardizedFileURL }
    static var sawPartialDisplay = false
    static var sawStaleRefusal = false
    static var sawFontBarrier = false
    /// B1 watch: after a save, the first publication that replaces the
    /// displayed one must still carry the marker (a coalesced save/close
    /// must not roll the preview back to pre-save text).
    static var watchMarker: String?
    static var watchSeq: UInt64 = 0
    static var watchResult: String?

    static var evidence: URL {
        URL(fileURLWithPath: ProcessInfo.processInfo.environment["PITEX_CHECK_EVIDENCE"]!)
    }
    static var lastEvidence = ContinuousClock.now
    static var evidenceStep = 0
    static var writtenSnapshot: (path: String, hash: UInt64)?
    static var writtenPDF: RetainedPDF?
    static var cachedPDF: RetainedPDF?
    static var cachedPDFSession: UInt64?
    static var cachedPDFSeq: UInt64?
    static var cachedPDFText = ""
    static func require(_ condition: Bool, _ message: String) {
        guard condition else {
            dumpState("FAIL: \(message)")
            print("FAIL", message); fflush(nil); exit(1)
        }
    }
    static func stage(_ m: String) {
        dumpState(m)
        evidenceStep += 1
        let step = evidence.appendingPathComponent("stage-\(evidenceStep)", isDirectory: true)
        try? FileManager.default.createDirectory(at: step, withIntermediateDirectories: true)
        for name in ["state.json", "snapshot.tex", "displayed.pdf"] {
            try? FileManager.default.copyItem(at: evidence.appendingPathComponent(name),
                                             to: step.appendingPathComponent(name))
        }
        print("[stage]", m); fflush(nil)
    }
    static func dumpState(_ reason: String) {
        var state: [String: Any] = ["appPID": getpid(), "reason": reason, "scenario": scenario,
                                  "unixTime": Date().timeIntervalSince1970]
        if let ws = workspace {
            state["windowNumber"] = ws.window?.windowNumber
            let preview = ws.embeddedPreview
            state["status"] = String(describing: ws.embeddedPreviewStatus)
            state["generation"] = preview.ledger.generation
            state["editRevision"] = preview.ledger.editRevision
            state["finalFloor"] = preview.ledger.finalFloor
            state["sources"] = preview.ledger.update(preview.ledger.generation)?.sources
            if let snapshot = ws.documentSnapshot {
                state["snapshotPath"] = snapshot.path.rawValue
                state["snapshotHash"] = snapshot.contentHash.rawValue
                state["saveState"] = String(describing: snapshot.saveState)
                if writtenSnapshot?.path != snapshot.path.rawValue
                    || writtenSnapshot?.hash != snapshot.contentHash.rawValue {
                    do {
                        try Data(snapshot.text.utf8).write(to: evidence.appendingPathComponent("snapshot.tex"), options: .atomic)
                        writtenSnapshot = (snapshot.path.rawValue, snapshot.contentHash.rawValue)
                    } catch { state["snapshotCaptureError"] = error.localizedDescription }
                }
            }
            if let session = preview.session {
                state["helperPID"] = session.checkPID
                state["sessionDirectory"] = session.directory.path
                state["writer"] = session.checkWriterBacklog
            }
            if let displayed = preview.displayed {
                state["displayedSeq"] = displayed.seq
                state["displayedGeneration"] = displayed.generation
                state["displayedComplete"] = displayed.complete
            }
            state["ledgerDisplayed"] = String(describing: preview.ledger.displayed)
            if let pdf = ws.retainedPDF, writtenPDF != pdf {
                do {
                    try pdf.data.write(to: evidence.appendingPathComponent("displayed.pdf"), options: .atomic)
                    writtenPDF = pdf
                } catch { state["pdfCaptureError"] = error.localizedDescription }
            }
        }
        if let bytes = try? JSONSerialization.data(withJSONObject: state, options: [.prettyPrinted, .sortedKeys]) {
            try? bytes.write(to: evidence.appendingPathComponent("state.json"), options: .atomic)
        }
    }
    static func until(_ s: Double = 15, _ c: @escaping @MainActor () -> Bool) async -> Bool {
        let deadline = ContinuousClock.now + .seconds(s)
        while !c() {
            if .now >= deadline { return false }
            try? await Task.sleep(for: .milliseconds(40))
            noteObservations()
        }
        return true
    }
    static func untilAsync(_ s: Double = 15,
                           _ c: @escaping @MainActor () async -> Bool) async -> Bool {
        let deadline = ContinuousClock.now + .seconds(s)
        while !(await c()) {
            if .now >= deadline { return false }
            try? await Task.sleep(for: .milliseconds(40))
            noteObservations()
        }
        return true
    }
    static func noteObservations() {
        if lastEvidence.duration(to: .now) >= .seconds(1) {
            lastEvidence = .now
            dumpState("async wait heartbeat")
        }
        guard let ws = workspace else { return }
        if let d = ws.embeddedPreview.displayed, !d.complete { sawPartialDisplay = true }
        if case let .stale(reason) = ws.syncTeXState, reason.contains("sources changed") {
            sawStaleRefusal = true
        }
        if watchResult == nil, let marker = watchMarker,
           let displayed = ws.embeddedPreview.displayed, displayed.seq > watchSeq,
           ws.retainedPDF?.isEmbeddedPreview == true {
            watchResult = pdfText(ws.retainedPDF!.data).contains(marker) ? "pass" : "fail"
        }
    }
    static func pdfText(_ data: Data) -> String {
        guard let pdf = workspace?.retainedPDF, pdf.data == data else {
            return PDFDocument(data: data)?.string ?? ""
        }
        let session = pdf.isEmbeddedPreview ? workspace.embeddedPreview.session?.id : nil
        let seq = pdf.isEmbeddedPreview ? workspace.embeddedPreview.displayed?.seq : nil
        if cachedPDF != pdf || cachedPDFSession != session || cachedPDFSeq != seq {
            cachedPDFText = PDFDocument(data: data)?.string ?? ""
            cachedPDF = pdf
            cachedPDFSession = session
            cachedPDFSeq = seq
        }
        return cachedPDFText
    }
    /// Inserts inside the fixture body through the production native editor.
    static func insert(_ text: String) {
        guard let tv = workspace.environment?.editor.textView else {
            require(false, "no text view")
            return
        }
        let terminator = (tv.string as NSString).range(of: "\\end{document}", options: .backwards)
        require(terminator.location != NSNotFound, "fixture has no document terminator")
        let body = NSRange(location: terminator.location, length: 0)
        require(tv.shouldChangeText(in: body, replacementString: text), "edit refused")
        tv.replaceCharacters(in: body, with: text)
        tv.didChangeText()
        print("[edit] utf16Offset=\(body.location) text=\(text.trimmingCharacters(in: .whitespacesAndNewlines))")
    }
    static func diskText(_ name: String) -> String {
        (try? String(contentsOf: project.appendingPathComponent(name), encoding: .utf8)) ?? ""
    }
    static func projectNames() -> [String] {
        ((try? FileManager.default.contentsOfDirectory(atPath: project.path)) ?? []).sorted()
    }
    static func sessionDir() -> URL? { workspace.embeddedPreview.session?.directory }
    static func statusIsUpdating() -> Bool {
        workspace.embeddedPreviewStatus == .updating
    }

    /// B1 regression: type a marker, let it publish, save — the very next
    /// publication after the save must still carry the marker. The buggy
    /// path (B1) restores stale fs_data on the coalesced close and shows
    /// pre-save text; a true idle keeps the displayed artifact. Either a
    /// newer publication or a settled unchanged generation is accepted —
    /// a regression is a newer publication WITHOUT the marker.
    static func saveMarkerRegression(_ marker: String) async {
        insert("\n\(marker)\\par\n")
        require(await until(90) {
            workspace.embeddedPreviewStatus == .current
                && workspace.retainedPDF?.isEmbeddedPreview == true
                && workspace.embeddedPreview.displayed?.complete == true
                && pdfText(workspace.retainedPDF!.data).contains(marker)
        }, "\(marker) never previewed")
        watchMarker = marker
        watchSeq = workspace.embeddedPreview.displayed?.seq ?? 0
        watchResult = nil
        let preSaveGeneration = workspace.embeddedPreview.ledger.generation
        await workspace.save()
        require(diskText("main.tex").contains(marker), "save did not write \(marker)")
        // Settled = the flush carrying the save arrived and the helper
        // answered: either the ledger's displayed generation advanced via
        // idle, or a newer publication landed.
        require(await until(45) {
            workspace.embeddedPreview.ledger.generation > preSaveGeneration
                && (watchResult != nil
                    || workspace.embeddedPreview.ledger.displayed?.generation
                        == workspace.embeddedPreview.ledger.generation)
        }, "save update never acknowledged")
        require(watchResult != "fail", "B1 regression: post-save publication dropped \(marker)")
        require(pdfText(workspace.retainedPDF!.data).contains(marker),
                "displayed preview lost \(marker) after save")
        watchMarker = nil
        stage("B1 ok: save did not roll the preview back")
    }

    /// The rendered preview surface is the WKWebView hosting
    /// pitex-pdfjs://app/viewer.html — find it by scheme, never class.
    static func pdfWebView(in view: NSView?) -> WKWebView? {
        guard let view else { return nil }
        if let web = view as? WKWebView, web.url?.scheme == "pitex-pdfjs" { return web }
        return view.subviews.lazy.compactMap { pdfWebView(in: $0) }.first
    }
    /// JS probe in the page world. `state()` reports the displayed
    /// generation; gen-parameterized calls need that real number.
    /// Last js() error — appended to live-poll FAILs instead of being
    /// swallowed by try? (stage-1 REVIEW-2 observability).
    static var lastJSError = ""
    static var lastProbe = "none"
    static func js(_ body: String) async -> Any? {
        lastJSError = ""
        guard let web = pdfWebView(in: workspace.window?.contentView) else {
            lastJSError = "no pitex-pdfjs web view"; return nil
        }
        do {
            return try await web.callAsyncJavaScript(body, arguments: [:], in: nil, contentWorld: .page)
        } catch { lastJSError = "\(error)"; return nil }
    }

    /// Two unsaved native body edits prove both initial startup and incrementality.
    static func runTmpRoot() async {
        let before = projectNames()
        let disk = try! Data(contentsOf: project.appendingPathComponent("main.tex"))
        let initialRevision = workspace.embeddedPreview.ledger.editRevision
        insert("\nStartMarker\\par\n")
        require(await until(15) {
            workspace.documentSnapshot?.saveState == .dirty
                && workspace.documentSnapshot?.text.contains("StartMarker") == true
                && workspace.documentSnapshot?.text == workspace.environment?.editor.textView.string
                && workspace.embeddedPreview.ledger.editRevision > initialRevision
        }, "first native body edit did not reach the dirty snapshot")
        stage("first body edit reached dirty snapshot")
        require(await until(30) { workspace.embeddedPreview.session != nil }, "first edit never launched helper")
        let session = workspace.embeddedPreview.session!
        let argv = await Task.detached { processArguments(session.checkPID) }.value
        func argument(_ flag: String) -> String? {
            guard let argv, let index = argv.firstIndex(of: flag), index + 1 < argv.count else { return nil }
            return argv[index + 1]
        }
        require(argument("--out") == session.directory.path, "argv probe did not identify our session")
        require(argument("--root") == posixCanonicalRoot(), "helper --root differs from POSIX realpath")
        print("[argv] ownedPID=\(session.checkPID) argc=\(argv?.count ?? 0) root=\(argument("--root") ?? "nil")")
        require(await until(90) {
            workspace.embeddedPreviewStatus == .current
                && workspace.retainedPDF?.isEmbeddedPreview == true
                && workspace.embeddedPreview.displayed?.complete == true
                && pdfText(workspace.retainedPDF!.data).contains("StartMarker")
        }, "first body edit never produced a complete StartMarker preview")
        stage("baseline: first unsaved BODY edit rendered StartMarker")
        let firstGeneration = workspace.embeddedPreview.ledger.generation
        let firstSeq = workspace.embeddedPreview.displayed!.seq
        let firstHash = workspace.documentSnapshot!.contentHash
        let firstRevision = workspace.embeddedPreview.ledger.editRevision
        insert("\nEditOneMarker\\par\n")
        require(await until(15) {
            workspace.documentSnapshot?.saveState == .dirty
                && workspace.documentSnapshot?.text.contains("EditOneMarker") == true
                && workspace.documentSnapshot?.contentHash != firstHash
                && workspace.embeddedPreview.ledger.editRevision > firstRevision
                && workspace.documentSnapshot?.text == workspace.environment?.editor.textView.string
        }, "second native body edit did not reach a new dirty snapshot")
        require(await until(30) {
            workspace.embeddedPreview.ledger.generation > firstGeneration
        }, "second edit did not queue a new generation")
        stage("second body edit reached a new dirty snapshot and generation")
        require(await until(90) {
            workspace.embeddedPreviewStatus == .current
                && workspace.retainedPDF?.isEmbeddedPreview == true
                && workspace.embeddedPreview.displayed?.complete == true
                && (workspace.embeddedPreview.displayed?.seq ?? 0) > firstSeq
                && (workspace.embeddedPreview.displayed?.generation ?? 0) > firstGeneration
                && pdfText(workspace.retainedPDF!.data).contains("StartMarker")
                && pdfText(workspace.retainedPDF!.data).contains("EditOneMarker")
        }, "incremental body edit never produced a complete two-marker preview")
        let displayedGeneration = workspace.embeddedPreview.displayed!.generation
        let compiledSources = workspace.embeddedPreview.ledger.update(displayedGeneration)?.sources
        let sourcePath = EmbeddedPreviewPaths.canonical(
            project.appendingPathComponent(workspace.documentSnapshot!.path.rawValue).path)
        require(compiledSources?[sourcePath] == workspace.documentSnapshot!.contentHash.rawValue,
                "displayed generation does not carry the current dirty snapshot hash")
        require((try? Data(contentsOf: project.appendingPathComponent("main.tex"))) == disk,
                "unsaved previews changed main.tex bytes")
        require(projectNames() == before, "unsaved previews changed the project tree")
        require(await untilAsync(15) {
            let text = await self.js("return (async () => { const g = window.pitex.state(); return await window.pitex.text(g); })();") as? String ?? ""
            lastProbe = "len=\(text.count) start=\(text.contains("StartMarker")) "
                + "edit=\(text.contains("EditOneMarker"))"
                + (lastJSError.isEmpty ? "" : " err=\(lastJSError)")
            return text.contains("StartMarker") && text.contains("EditOneMarker")
        }, "pdf.js surface did not render the two-marker document "
            + "(last probe: \(Driver.lastProbe); last js error: \(Driver.lastJSError.isEmpty ? "none" : Driver.lastJSError))")
        // Scroll to the last page through the real viewer surface:
        // setView targets the last .page element, then position() reports
        // the viewer's current page.
        _ = await js("""
            return (() => { const g = window.pitex.state();
              const last = document.querySelectorAll(".page").length;
              return last > 0 && window.pitex.setView(g, "page-width", last); })();
            """)
        _ = await untilAsync(15) {
            await self.js("""
                return (() => { const g = window.pitex.state();
                  const last = document.querySelectorAll(".page").length;
                  const p = window.pitex.position(g);
                  return last > 0 && p && p.page === last; })();
                """) as? Bool == true
        }
        stage("two unsaved BODY edits passed: dirty snapshots, owned canonical helper, new generation, both PDF markers, unchanged disk")
        try? await Task.sleep(for: .seconds(2))
        if scenario != "smoke" { await saveMarkerRegression("TmpRootSaveMarker") }
        do { let e = BridgeProbe.audit(); require(e.isEmpty, e) }
        print("CHECK_COMPLETE")
        fflush(nil)
        exit(0)
    }

    /// Darwin's bounded argv buffer only; never read/log the trailing environment.
    nonisolated static func processArguments(_ pid: pid_t) -> [String]? {
        var mib: [Int32] = [CTL_KERN, KERN_PROCARGS2, pid]
        var size = 0
        guard sysctl(&mib, 3, nil, &size, nil, 0) == 0,
              size >= MemoryLayout<Int32>.size, size <= 1 << 20 else { return nil }
        var data = Data(count: size)
        let result = data.withUnsafeMutableBytes {
            sysctl(&mib, 3, $0.baseAddress, &size, nil, 0)
        }
        guard result == 0, size >= MemoryLayout<Int32>.size else { return nil }
        data.count = size
        let argc = data.withUnsafeBytes { $0.loadUnaligned(as: Int32.self) }
        guard argc > 0, argc <= 1024 else { return nil }
        var cursor = MemoryLayout<Int32>.size
        guard let pathEnd = data[cursor...].firstIndex(of: 0) else { return nil }
        cursor = pathEnd + 1
        while cursor < data.count, data[cursor] == 0 { cursor += 1 }
        var argv: [String] = []
        for _ in 0..<argc {
            guard cursor < data.count, let end = data[cursor...].firstIndex(of: 0) else { return nil }
            argv.append(String(decoding: data[cursor..<end], as: UTF8.self))
            cursor = end + 1
        }
        return argv
    }

    static func posixCanonicalRoot() -> String {
        root.withCString { ptr -> String in
            guard let resolved = realpath(ptr, nil) else { return root }
            defer { free(resolved) }
            return String(cString: resolved)
        }
    }

    static func run() async {
        print("[diag] windows=\(NSApp.windows.count) root=\(root)")
        dumpState("driver launched")
        require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                "no initial window")
        workspace = WorkspaceWindows.live[0]
        SettingsStore.shared.liveCompileEnabled = true
        SettingsStore.shared.livePreviewBackend = "embedded"
        SettingsStore.shared.autoSave = false
        SettingsStore.shared.restoreSession = false
        workspace.syncLiveScheduler()

        WorkspaceWindows.route(project)
        require(await until { workspace.hasProject },
                "project never opened")
        require(await until {
            if case .ready = workspace.phase { return true }; return false
        }, "workspace never reached .ready")
        stage("project open, embedded backend armed")

        if scenario == "tmp-root" || scenario == "smoke" {
            await runTmpRoot()
            return
        }


        // --- S1: unsaved edit → editing preview, disk untouched ---------
        let before = projectNames()
        insert("\nEditOneMarker\\par\n")
        require(await until(20) {
            statusIsUpdating() || workspace.retainedPDF?.isEmbeddedPreview == true
        }, "no preview activity after edit")
        let updatingSeen = statusIsUpdating()
        stage(updatingSeen ? "status showed updating mid-pass" : "status went straight to display (fast pass)")
        require(await until(90) {
            workspace.embeddedPreviewStatus == .current
                && workspace.retainedPDF?.isEmbeddedPreview == true
                && workspace.embeddedPreview.displayed?.complete == true
                && pdfText(workspace.retainedPDF!.data).contains("EditOneMarker")
        }, "edited preview never displayed (baseline publications allowed)")
        let retained1 = workspace.retainedPDF!
        require(retained1.isEmbeddedPreview, "displayed artifact is not an editing preview")
        require(pdfText(retained1.data).contains("EditOneMarker"),
                "preview lacks the edited text")
        require(diskText("main.tex").contains("EditOneMarker") == false,
                "preview wrote into main.tex")
        require(!projectNames().contains("main.pdf") && !projectNames().contains("main.aux"),
                "preview produced final artifacts in the project")
        require(projectNames() == before, "project tree changed by preview")
        stage("S1 ok: unsaved edit previewed, disk + final artifacts untouched")

        // --- B1: coalesced save must not roll the preview back to
        // pre-save text (edit → save → one update → marker stays).
        await saveMarkerRegression("SavePersistMarker")

        // --- S2: partial snapshot path --------------------------------
        require(workspace.embeddedPreview.displayed?.synctex != nil
                || workspace.syncTeXBinding != nil || workspace.embeddedPreview.displayed?.complete == true,
                "coherent publication expected after settled pass")
        insert("\nEditTwoMarker\\par\n")
        require(await until(60) {
            workspace.embeddedPreviewStatus == .current
                && pdfText(workspace.retainedPDF?.data ?? Data()).contains("EditTwoMarker")
        }, "second edit never previewed")
        stage("S2 ok: \(sawPartialDisplay ? "partial publication displayed mid-pass" : "no partial observed (pass too fast)"), newest complete reflects edit")

        // --- S3: manual build floor ------------------------------------
        let beforeBuildRevision = workspace.embeddedPreview.ledger.editRevision
        insert("\nPreBuildEdit\\par\n")          // inside coalesce window
        await workspace.startBuild()
        let joinedRevision = workspace.embeddedPreview.ledger.editRevision
        require(workspace.documentSnapshot?.text.contains("PreBuildEdit") == true
                && workspace.documentSnapshot?.text == workspace.environment?.editor.textView.string
                && joinedRevision > beforeBuildRevision,
                "manual request returned before producer ACK reached workspace snapshot/edit revision")
        require((workspace.embeddedPreview.ledger.update(workspace.embeddedPreview.ledger.generation)?.revision ?? 0)
                    < joinedRevision,
                "immediate-Build regression did not exercise a pre-coalesced edit")
        stage("producer ACK reached workspace snapshot/edit revision before manual scheduling")
        require(await until(120) {
            if case .succeeded = workspace.buildState { return true }; return false
        }, "manual build did not succeed: \(workspace.buildState)")
        require(workspace.embeddedPreview.ledger.finalFloor == joinedRevision,
                "final floor omitted the native edit acknowledged before the build")
        require(diskText("main.tex").contains("PreBuildEdit"),
                "manual build did not save dirty source")
        require(projectNames().contains("main.pdf"), "final main.pdf missing")
        require(workspace.retainedPDF?.isEmbeddedPreview != true,
                "final build did not reclaim the preview")
        require(workspace.retainedPDF?.isFinalOutput == true,
                "retained artifact not marked final")
        require(pdfText(workspace.retainedPDF!.data).contains("PreBuildEdit"),
                "final pdf lacks pre-build edit")
        stage("S3a ok: final PDF replaced the preview, carries pre-build edit")

        // Truly later edit (the build already finished): allowed to draft.
        insert("\nPostBuildDraft\\par\n")
        require(await until(60) {
            workspace.retainedPDF?.isEmbeddedPreview == true
                && pdfText(workspace.retainedPDF!.data).contains("PostBuildDraft")
        }, "post-build edit preview never took over")
        stage("S3b ok: post-build edit drafts over the final PDF")

        // Edit during a long-ish second manual build stays previewed.
        await workspace.startBuild()
        require(await until(15) {
            if case .building = workspace.buildState { return true }; return false
        }, "second manual build never entered its actual build phase")
        let duringBuildFloor = workspace.embeddedPreview.ledger.editRevision
        insert("\nDuringBuildEdit\\par\n")
        require(await until(15) {
            workspace.documentSnapshot?.text.contains("DuringBuildEdit") == true
                && workspace.documentSnapshot?.text == workspace.environment?.editor.textView.string
                && workspace.embeddedPreview.ledger.editRevision > duringBuildFloor
        }, "during-build producer ACK did not reach workspace snapshot/edit revision")
        require(await until(120) {
            if case .succeeded = workspace.buildState { return true }; return false
        }, "second manual build failed")
        require(workspace.embeddedPreview.ledger.finalFloor == duringBuildFloor,
                "final floor swallowed a native edit made during the actual build")
        require(workspace.embeddedPreview.ledger.editRevision > duringBuildFloor,
                "during-build edit is not above the final floor")
        // Ledger: the displayed preview carrying an after-build-start edit
        // stays; otherwise the final is shown. Either is correct per ledger —
        // the wrong outcome is a *stale* preview without the edit.
        let shownAfter = pdfText(workspace.retainedPDF?.data ?? Data())
        require(shownAfter.contains("DuringBuildEdit") || workspace.retainedPDF?.isFinalOutput == true,
                "final display neither shows post-edit preview nor final output")
        require(await until(60) {
            workspace.retainedPDF?.isEmbeddedPreview == true
                && workspace.embeddedPreview.displayed?.complete == true
                && pdfText(workspace.retainedPDF!.data).contains("DuringBuildEdit")
        }, "during-build edit never produced an eligible complete preview")
        stage("S3c ok: edit during manual build resolved per floor")

        // --- S4: SyncTeX on the coherent preview -----------------------
        insert("\nSyncProbeEdit\\par\n")
        require(await until(60) {
            workspace.retainedPDF?.isEmbeddedPreview == true
                && workspace.embeddedPreview.displayed?.complete == true
                && pdfText(workspace.retainedPDF!.data).contains("SyncProbeEdit")
                && workspace.embeddedPreview.displayed?.synctex != nil
        }, "no coherent preview for SyncTeX")
        require(await until(20) { workspace.syncTeXBinding != nil },
                "SyncTeX never bound to the editing preview")
        // Forward: real query against the session artifact.
        workspace.jumpTo(line: 8, column: 0)
        await workspace.syncForward()
        if case .stale = workspace.syncTeXState {
            require(false, "forward sync went stale on a coherent preview")
        }
        // Inverse through the bound runner at page 1 center (PDF coords).
        do {
            let binding = workspace.syncTeXBinding!
            let match = try await workspace.syncTeXRunner.inverse(
                binding: binding, page: 1,
                point: try SyncTeXCore.PDFPoint(x: 150, y: 700))
            print("[diag] inverse -> \(match.source.path.value):\(match.source.line)")
            require(match.source.path.value.hasSuffix(".tex"),
                    "inverse mapped to a non-source path")
        } catch {
            print("[diag] inverse query returned \(error) — acceptable ambiguity")
        }
        // Diverge the buffer: stale mapping must be refused.
        insert("\nStaleProbe\\par\n")
        require(await untilAsync(10) {
            await self.workspace.embeddedPreviewSyncTeXRefusal() != nil
        }, "stale SyncTeX was not refused after diverging edit")
        stage("S4 ok: coherent binding + forward/inverse + stale refusal")

        // --- S5: export + detached preview ------------------------------
        require(await until(60) {
            workspace.retainedPDF?.isEmbeddedPreview == true
                && pdfText(workspace.retainedPDF!.data).contains("StaleProbe")
                && workspace.embeddedPreviewStatus == .current
        }, "preview did not catch up before export")
        let bytes = workspace.retainedPDF!.data
        let exported: URL
        do { exported = try WorkspaceModel.exportEmbeddedPreviewCopy(
                bytes, named: "main (Editing preview).pdf") }
        catch { require(false, "export threw \(error)"); fatalError() }
        require(exported.lastPathComponent.contains("Editing preview"),
                "export is not labeled as an editing preview")
        require((try? Data(contentsOf: exported)) == bytes,
                "exported bytes differ from displayed bytes")
        require(exported.path.contains("Pitex Editing Previews"),
                "export left the owned temp area")
        workspace.detachPreview()
        require(await until { workspace.detachedPreviewWindow != nil },
                "preview did not detach")
        require(workspace.retainedPDF?.data == bytes,
                "detached preview changed artifact identity")
        workspace.attachPreview()
        require(await until { workspace.detachedPreviewWindow == nil },
                "preview did not reattach")
        stage("S5 ok: labeled export + same-artifact detach/attach")

        // --- S6: backend switch + SIGSTOP bounded writer ----------------
        // SIGSTOP the helper: the writer must stay bounded and the main
        // actor responsive.
        let sessionDirURL = sessionDir()
        require(sessionDirURL != nil, "no session directory")
        require(workspace.embeddedPreview.session != nil, "no owned helper")
        let helperPID = workspace.embeddedPreview.session!.checkPID
        kill(helperPID, SIGSTOP)
        let t0 = ContinuousClock.now
        insert("\nStoppedEdit\\par\n")
        let mainActorDelay = t0.duration(to: .now)
        require(mainActorDelay < .milliseconds(500),
                "main actor stalled behind stopped helper: \(mainActorDelay)")
        // A few more edits — bounded outbox (snapshot replacement, not growth).
        for i in 0..<3 { insert("\nStoppedEdit\(i)\\par\n") }
        try? await Task.sleep(for: .milliseconds(500))
        kill(helperPID, SIGCONT)
        // Backend → compiler must kill the group and the session dir.
        SettingsStore.shared.livePreviewBackend = "compiler"
        workspace.syncLiveScheduler()
        require(await until(15) {
            var b: Int32 = 0
            kill(helperPID, 0)   // ESRCH once reaped
            return kill(helperPID, 0) != 0 && errno == ESRCH
        }, "helper group still alive after backend switch")
        require(await until(10) {
            !FileManager.default.fileExists(atPath: sessionDirURL!.path)
        }, "session directory survived backend switch")
        require(workspace.embeddedPreview.session == nil,
                "session object retained")
        stage("S6 ok: SIGSTOP bounded + backend switch killed group/session")

        // --- S7: CoreText late-font barrier -----------------------------
        // The fonts fixture is a second document inside the same project;
        // open it and build an embedded preview for it as the main file.
        workspace.pinnedBuildTarget = nil
        let fontsURL = project.appendingPathComponent("fonts.tex")
        // fonts.tex opens as a document in this workspace; the auto-resolved
        // build target makes it the session's main file. A commented edit
        // triggers the first flush (edits are what arm the preview).
        WorkspaceWindows.route(fontsURL)
        require(await until(20) {
            workspace.activeDocumentURL?.lastPathComponent == "fonts.tex"
        }, "fonts.tex never became the active document")
        SettingsStore.shared.livePreviewBackend = "embedded"
        workspace.syncLiveScheduler()
        insert("\n% trigger-first-preview\n")
        require(await until(30) {
            workspace.activeDocumentURL?.lastPathComponent == "fonts.tex"
                && workspace.embeddedPreview.session?.mainRelative == "fonts.tex"
        }, "fonts document did not start its own preview session")
        require(await until(90) {
            workspace.embeddedPreviewStatus == .current
                && workspace.retainedPDF?.isEmbeddedPreview == true
                && workspace.embeddedPreview.displayed?.complete == true
                && workspace.embeddedPreview.session?.mainRelative == "fonts.tex"
        }, "fonts document never produced a complete preview")
        let fontsBytes = workspace.retainedPDF!.data
        let fontsText = pdfText(fontsBytes)
        require(fontsText.contains("PageOneDefaultFont")
                && fontsText.contains("PageTwoMenloFont"),
                "fonts preview missing expected text")
        let doc = PDFDocument(data: fontsBytes)
        require(doc?.pageCount == 2, "fonts preview is not exactly 2 pages")
        // driver.log evidence: the engine's stderr (font-barrier restart)
        // is captured per session directory.
        if let sessionLog = sessionDir()?.appendingPathComponent("driver.log"),
           let logText = try? String(contentsOf: sessionLog, encoding: .utf8) {
            let barrier = logText.contains("font barrier")
                || logText.contains("FNTB")
            print("[fntb] driver.log barrier evidence: \(barrier)")
            print("[fntb] logtail: \(logText.suffix(400))")
            sawFontBarrier = barrier
        }
        require(sawFontBarrier,
                "no FNTB font-barrier evidence in driver.log for page-2 font load")
        // Reference-text oracle: system xelatex produced the same words.
        let refPath = ProcessInfo.processInfo.environment["PITEX_CHECK_REF"] ?? ""
        if let refData = try? Data(contentsOf: URL(fileURLWithPath: refPath)),
           let refDoc = PDFDocument(data: refData) {
            let refText = refDoc.string ?? ""
            require(refText.contains("PageTwoMenloFont")
                    && refDoc.pageCount == 2,
                    "reference xelatex pdf unexpected")
        }
        stage("S7 ok: FNTB barrier observed, 2-page fontspec preview matches reference")

        print("[winnum] \(workspace.window?.windowNumber ?? -1)")
        do { let e = BridgeProbe.audit(); require(e.isEmpty, e) }
        print("CHECK_COMPLETE")
        fflush(nil)
        exit(0)
    }
}
'''

with tempfile.TemporaryDirectory(prefix="pitex-embedded-check-") as directory:
    # The dedicated VM has one GUI console — serialize actual native runs.
    # real app builds/UI smoke through a user-private flock so parallel runs
    # never fight over key window or skew timings.
    import fcntl
    lock_path = Path.home() / "pitex-verify-scratch/native-verification.lock"
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock_fd = open(lock_path, "a")
    print(f"[diag] waiting for native-verification lock {lock_path}")
    fcntl.flock(lock_fd, fcntl.LOCK_EX)
    print("[diag] lock acquired")
    root = Path(directory)
    fixture = root / "fixture"
    fixture.mkdir()
    (fixture / "main.tex").write_text(fixture_main)
    (fixture / "tail.tex").write_text(fixture_tail)
    (fixture / "fonts.tex").write_text(fixture_fonts)
    (fixture / "notes.md").write_text("# not a tex file\n")

    # The final-compiler reference is only an S7 oracle, never a preview fallback.
    ref_dir = root / "ref"
    ref_dir.mkdir()
    if args.scenario in ("all", "full"):
        shutil.copy2(fixture / "fonts.tex", ref_dir / "fonts.tex")
        subprocess.run(["/Library/TeX/texbin/xelatex", "-interaction=nonstopmode",
                        "-halt-on-error", "fonts.tex"], cwd=ref_dir,
                       env={**os.environ, "HOME": str(root / "home-ref"),
                            "PATH": "/Library/TeX/texbin:/usr/bin:/bin"},
                       check=True, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=240)
        assert (ref_dir / "fonts.pdf").exists(), "reference xelatex produced no pdf"

    # Test-only copy of PitexApp.swift keeping the production @main.
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    patched = app_main.read_text().replace(
        "    func applicationDidFinishLaunching(_ notification: Notification) {",
        "    func applicationDidFinishLaunching(_ notification: Notification) {\n"
        "        LaunchFlag.didFinishLaunching = true\n"
        "        LaunchFlag.finishUserInfo = String(describing: notification.userInfo)\n"
        "        Task { @MainActor in await Driver.run() }")
    assert "await Driver.run()" in patched
    acknowledge = "            await didChange(updated)\n"
    assert patched.count(acknowledge) == 1
    patched = patched.replace(
        acknowledge, acknowledge +
        '            print("[editor-ack] revision=\\(updated.revision) hash=\\(updated.contentHash.rawValue)"); fflush(nil)\n')
    app_copy = root / "PitexApp.swift"
    app_copy.write_text(patched)
    # Read-only accessors exist solely in the compiled checker copy.
    embedded_main = repo / "Mac/Sources/Features/EmbeddedPreview.swift"
    embedded_source = embedded_main.read_text()
    pid_declaration = "    private let pid: pid_t\n"
    assert embedded_source.count(pid_declaration) == 1
    embedded_source = embedded_source.replace(
        pid_declaration, pid_declaration +
        "    var checkPID: pid_t { pid }\n"
        "    var checkWriterBacklog: String { String(describing: writer.backlog()) }\n")
    wire_decode = "                   let event = decodeEvent(wire) {\n"
    assert embedded_source.count(wire_decode) == 1
    embedded_source = embedded_source.replace(
        wire_decode, wire_decode +
        '                    print("[wire] event=\\(wire.event) generation=\\(wire.generation ?? 0) seq=\\(wire.seq ?? 0) complete=\\(wire.complete ?? false)"); fflush(nil)\n')
    update_call = "        writer.update(generation: generation, buffers: buffers)\n"
    assert embedded_source.count(update_call) == 1
    embedded_source = embedded_source.replace(
        update_call, '        print("[queued] generation=\\(generation) overrides=\\(buffers.mapValues { $0.hash })"); fflush(nil)\n' +
        update_call)
    embedded_copy = root / "EmbeddedPreview.swift"
    embedded_copy.write_text(embedded_source)
    # BridgeProbe: the compiled Preview.swift copy records every bridge post.
    preview_main = repo / "Mac/Sources/Features/Preview.swift"
    preview_source = preview_main.read_text()
    bridge_anchor = '            guard let m = message.body as? [String: Any],\n                  let type = m["type"] as? String else { return }'
    assert preview_source.count(bridge_anchor) == 1
    preview_copy = root / "Preview.swift"
    preview_copy.write_text(preview_source.replace(
        bridge_anchor,
        bridge_anchor + '\n            BridgeProbe.log(type, m)'))
    (root / "Check.swift").write_text(check)
    binary = root / "check-bin"
    compile_cmd = [native_resolution["swiftc"], "-sdk", native_resolution["sdk"],
                   "-parse-as-library", "-swift-version", "6",
                   "-target", "arm64-apple-macos15.0", "-I", str(products),
                   str(root / "Check.swift"), str(app_copy), str(embedded_copy),
                   str(preview_copy),
                   *[str(p) for p in (repo / "Mac/Sources").rglob("*.swift")
                     if p not in (app_main, embedded_main, preview_main)],
                   *[str(p) for p in products.glob("*.o")], "-o", str(binary)]
    shutil.copy2(root / "Check.swift", artifacts / "Check.swift")
    shutil.copy2(embedded_copy, artifacts / "EmbeddedPreview.swift")
    native_resolution["compileCommand"] = compile_cmd
    native_resolution["effectiveSDKROOT"] = native_environment["SDKROOT"]
    (artifacts / "native-toolchain.json").write_text(json.dumps(native_resolution, indent=2) + "\n")
    _comp = subprocess.run(compile_cmd, env=native_environment,
                           capture_output=True, text=True)
    (artifacts / "compile.log").write_text(_comp.stdout + _comp.stderr)
    if _comp.returncode != 0:
        print(_comp.stderr[-4000:]); sys.exit(f"FAIL: swiftc rc={_comp.returncode}; evidence: {artifacts}")
    _bad = MARKER.findall(_comp.stderr + _comp.stdout)
    if _bad:
        for line in _comp.stderr.splitlines() + _comp.stdout.splitlines():
            if MARKER.search(line): print(line)
        sys.exit("FAIL: harness compile defect — generated Check.swift has an "
                 "UnnecessaryEffectMarker warning (async query result discarded); "
                 "no functional verdict")

    pi_runtime_bin = root / "pi-runtime/bin"
    pi_runtime_bin.mkdir(parents=True)
    fake_pi = pi_runtime_bin / "pi"
    fake_pi.write_text("#!/bin/sh\nexit 0\n")
    fake_pi.chmod(0o755)

    identifier = f"test.pitex.embedded.{os.getpid()}"
    app = root / "EmbeddedPreviewCheck.app"
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    resources = contents / "Resources"
    resources.mkdir()
    plist = plistlib.loads((repo / "Mac/Config/Info.plist").read_bytes())
    plist.update({
        "CFBundleExecutable": "check",
        "CFBundleIdentifier": identifier,
        "CFBundleName": "EmbeddedPreviewCheck",
        "CFBundleDisplayName": "EmbeddedPreviewCheck",
        "CFBundleDevelopmentRegion": "en",
        "CFBundleShortVersionString": "0",
        "CFBundleVersion": "0",
        "LSMinimumSystemVersion": "15.0",
    })
    (contents / "Info.plist").write_bytes(plistlib.dumps(plist))
    for locale in (repo / "Mac/Resources").glob("*.lproj"):
        shutil.copytree(locale, resources / locale.name)
    for name in ("markdown-preview.html", "PitexAgent", "pdfjs"):
        item = repo / "Mac/Resources" / name
        if item.is_dir():
            shutil.copytree(item, resources / name)
        elif item.exists():
            shutil.copy2(item, resources / name)
    # Release-parity gate: every rendered probe 404s without the payload.
    verify_pdfjs_manifest(resources / "pdfjs")
    for bundle in products.glob("*.bundle"):
        shutil.copytree(bundle, resources / bundle.name)
    # The real bundled helper payload at the production lookup location.
    helpers = contents / "Helpers"
    helpers.mkdir()
    assert (payload / "PreviewEngine/pitex-preview").exists(), \
        f"helper payload missing at {payload}"
    payload_src = payload / "PreviewEngine"
    for f in payload_src.rglob("*"):
        if not f.is_file():
            continue
        if f.name.startswith("pitex-preview") or f.name.endswith(".dylib"):
            rel = f.relative_to(payload_src)
            (helpers / "PreviewEngine" / rel.parent).mkdir(parents=True,
                                                        exist_ok=True)
            shutil.copy2(f, helpers / "PreviewEngine" / rel)
        else:
            # Non-code files cannot live under Contents/Helpers — codesign
            # treats every file there as a signable code object. Licenses
            # and provenance belong in Resources.
            rel = f.relative_to(payload_src)
            (resources / "PreviewEngine" / rel.parent).mkdir(parents=True,
                                                        exist_ok=True)
            shutil.copy2(f, resources / "PreviewEngine" / rel)
    shutil.copy(binary, contents / "MacOS/check")
    # Inside-out: nested helpers/dylibs sign before the bundle (matches
    # Xcode's embedding order; unsigned contents break the outer seal).
    for lib in sorted((helpers / "PreviewEngine/lib").glob("*.dylib")):
        subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                        str(lib)], check=True, capture_output=True)
    for helper in sorted((helpers / "PreviewEngine").glob("pitex-preview*")):
        if helper.is_file() and not helper.is_dir():
            subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                            "--entitlements",
                            str(repo / "Mac/Config/Pitex.entitlements"),
                            str(helper)], check=True, capture_output=True)
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                    "--entitlements",
                    str(repo / "Mac/Config/Pitex.entitlements"),
                    str(contents / "MacOS/check")], check=True, capture_output=True)
    for bundle in resources.glob("*.bundle"):
        subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                        str(bundle)], check=True, capture_output=True)
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                    "--entitlements", str(repo / "Mac/Config/Pitex.entitlements"),
                    "--identifier", identifier, str(app)], check=True)
    describe = subprocess.run(["/usr/bin/codesign", "-d", "--verbose=2", str(app)],
                              check=True, capture_output=True, text=True)
    signed = re.search(r"^Identifier=(.*)$", describe.stderr, re.M)
    print(f"[diag] codesign {app.name}: {signed.group(1) if signed else '?'}")
    assert signed and signed.group(1) == identifier

    for key in ("pitex.pref.editor.restoreSession",
                "pitex.pref.update.autoInstall",
                "pitex.pref.editor.autoSave"):
        subprocess.run(["/usr/bin/defaults", "write", identifier, key,
                        "-bool", "false"], check=True)
    for key, val in (("pitex.pref.build.liveCompileEnabled", True),):
        subprocess.run(["/usr/bin/defaults", "write", identifier, key,
                        "-bool", "true" if val else "false"], check=True)
    subprocess.run(["/usr/bin/defaults", "write", identifier,
                    "pitex.pref.build.livePreviewBackend", "embedded"], check=True)

    home = root / "home"
    home.mkdir()

    # Second fixture addressed through the /tmp spelling — on Darwin /tmp
    # is a symlink to /private/tmp, so this opens the project through a
    # non-canonical root (the personal-Mac failure shape). The scenario
    # asserts the helper's --root is the realpath spelling, an unsaved
    # edit publishes, and the B1 save regression holds.
    tmp_fixture = Path("/tmp") / f"pitex-embedded-check-{os.getpid()}"
    tmp_fixture.mkdir()
    (tmp_fixture / "main.tex").write_text(fixture_main)
    (tmp_fixture / "tail.tex").write_text(fixture_tail)
    (tmp_fixture / "notes.md").write_text("# not a tex file\n")

    def run_app(fixture_dir: Path, scenario: str) -> str:
        run_dir = Path(tempfile.mkdtemp(prefix=f"{scenario}-", dir=artifacts))
        print(f"[diag] {scenario} evidence directory {run_dir}", flush=True)
        shutil.copytree(fixture_dir, run_dir / "fixture-before")
        out_log = run_dir / "check.out.log"
        err_log = run_dir / "check.err.log"
        state_file = run_dir / "state.json"

        def state():
            try:
                return json.loads(state_file.read_text())
            except (OSError, ValueError):
                return {}

        def capture(sample=False):
            current = state()
            # Copy before cleanup AND before the next app's prepareSessionRoot
            # can delete a previous owner's session directory.
            session = current.get("sessionDirectory")
            if session and Path(session).is_dir():
                try:
                    shutil.copytree(session, run_dir / "session", dirs_exist_ok=True)
                except (OSError, shutil.Error) as error:
                    (run_dir / "session-capture-error.log").write_text(str(error) + "\n")
            helper_pid = current.get("helperPID")
            try:
                result = subprocess.run(
                    ["/bin/ps", "-axo", "pid=,ppid=,pgid=,comm="],
                    capture_output=True, text=True, timeout=10, check=True)
            except (OSError, subprocess.SubprocessError) as error:
                (run_dir / "process-capture-error.log").write_text(
                    f"{error}\nProcess ownership/cleanup could not be verified by ps.\n")
                return set()
            processes = []
            for line in result.stdout.splitlines():
                fields = line.split(None, 3)
                try:
                    if len(fields) == 4:
                        processes.append((*(int(value) for value in fields[:3]), fields[3]))
                except ValueError:
                    (run_dir / "process-capture-error.log").write_text("Unparseable ps output: " + line + "\n")
            owned = {pid for pid, _, _, command in processes
                     if Path(command).resolve() == (app / "Contents/MacOS/check").resolve()}
            if helper_pid:
                owned.update(pid for pid, _, group, command in processes
                             if group == helper_pid and "pitex-preview" in command)
            for _ in processes:
                descendants = {pid for pid, parent, _, _ in processes if parent in owned}
                if descendants <= owned:
                    break
                owned.update(descendants)
            (run_dir / "processes.txt").write_text(
                "\n".join(f"{pid} {parent} {group} {command}"
                          for pid, parent, group, command in processes if pid in owned) + "\n")
            if sample:
                for pid in sorted(owned):
                    try:
                        result = subprocess.run(
                            ["/usr/bin/sample", str(pid), "2", "1", "-file",
                             str(run_dir / f"sample-{pid}.txt")],
                            capture_output=True, text=True, timeout=10)
                        (run_dir / f"sample-{pid}.log").write_text(result.stdout + result.stderr)
                    except (OSError, subprocess.SubprocessError) as error:
                        (run_dir / f"sample-{pid}.log").write_text(str(error) + "\n")
            return owned

        command = ["/usr/bin/open", "-W",
                   "--stdout", str(out_log), "--stderr", str(err_log),
                   "--env", "PI_AGENT_PATH=/usr/bin/false",
                   "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
                   "--env", f"HOME={home}",
                   "--env", "PATH=/Library/TeX/texbin:/usr/bin:/bin:/usr/sbin:/sbin",
                   "--env", f"PITEX_CHECK_FIXTURE={fixture_dir}",
                   "--env", f"PITEX_CHECK_SCENARIO={scenario}",
                   "--env", f"PITEX_CHECK_EVIDENCE={run_dir}",
                   "--env", f"PITEX_CHECK_REF={ref_dir / 'fonts.pdf'}",
                   str(app)]
        launcher = subprocess.Popen(command)
        deadline = time.monotonic() + (300 if scenario == "smoke" else 600)
        sampled = False
        started = time.time()
        screenshot = False
        try:
            while launcher.poll() is None:
                heartbeat = state_file.stat().st_mtime if state_file.exists() else started
                current = state()
                if not screenshot and current.get("reason", "").startswith("two unsaved BODY edits passed"):
                    screenshot = True
                    window = current.get("windowNumber")
                    try:
                        if not isinstance(window, int) or window <= 0:
                            raise ValueError("Owned window number unavailable; no visual capture attempted.")
                        subprocess.run(
                            ["/usr/sbin/screencapture", "-x", f"-l{window}", str(run_dir / "two-body-edits.png")],
                            check=True, capture_output=True, timeout=10)
                    except (OSError, subprocess.SubprocessError, ValueError) as error:
                        (run_dir / "visual-capture-error.log").write_text(
                            f"{error}\nVisual proof unavailable; functional assertions are unchanged.\n")
                if not sampled and time.time() - heartbeat > 15:
                    print(f"[watchdog] {scenario}: no MainActor heartbeat; sampling before cleanup", flush=True)
                    capture(sample=True)
                    sampled = True
                if time.monotonic() >= deadline:
                    print(f"FAIL[{scenario}]: external watchdog deadline", flush=True)
                    capture(sample=True)
                    break
                time.sleep(0.2)
        finally:
            output = out_log.read_text(errors="replace") if out_log.exists() else ""
            owned = capture(sample="CHECK_COMPLETE" not in output and not sampled)
            shutil.copytree(fixture_dir, run_dir / "fixture-after")
            for pid in owned:
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            if launcher.poll() is None:
                launcher.terminate()
            launcher.wait(timeout=10)
            for log in (out_log, err_log):
                if log.exists():
                    print(f"--- {log.name} ---")
                    print(log.read_text(errors="replace"), end="")
        return output

    try:
        scenarios = ("full", "tmp-root") if args.scenario == "all" else (args.scenario,)
        outputs = [(scenario, run_app(fixture if scenario == "full" else tmp_fixture, scenario))
                   for scenario in scenarios]
    finally:
        shutil.rmtree(tmp_fixture, ignore_errors=True)
    missing = [name for name, output in outputs if "CHECK_COMPLETE" not in output]
    if missing:
        sys.exit(f"FAIL: no CHECK_COMPLETE in {missing}; evidence: {artifacts}")
