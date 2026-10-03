#!/usr/bin/env python3
"""pdf.js candidate vs PDFKit control — pre-registered perf/memory harness.

Addendum A of pdfjs-integration-GATES.md, executable form. Two .app builds of
the SAME checker driver: candidate = working-tree Preview.swift (pdf.js);
control = the pinned accepted pre-integration production Preview.swift
(9b0446fa… — the r3 arms/production bytes; NOT a git ref). Arms run strict
ABAB after one discarded warm-up RUN per arm; identical edit script; fresh
isolated HOME per run. All instrumentation lives in patch-compiled checker
copies — product bytes are untouched.

Timing (ProcessInfo.systemUptime; the harness validates it shares the
time.monotonic epoch by requiring every mark inside the run window, T6):
  t0 candidate = Coordinator.load() gen++/publish instant.
  t0 control   = updateNSView's `view.document = PDFDocument(data:)`.
  cold t0      = makeNSView entry, BOTH arms (includes webview/page/worker
                 init for the candidate, PDFView creation for control).
  t1 candidate = pagerendered{gen==push gen, page==displayed page,
                 error nil-or-NSNull, cssTransform false, detail false}
                 — paired per GENERATION.
  t1 control   = first .beforeWaiting observer pass (order CFIndex.max,
                 i.e. after the CA commit) where view.document is
                 *identical* to the doc that t0's assignment installed and
                 documentView.needsDisplay == false — paired per DOCUMENT.
  Superseded publications (a push that never got its qualifying paint
  before the next push) are counted and reported per arm.

Memory: python-side /bin/ps at 4 Hz. Tracked set = app pid ∪ every
WebContent/GPU/Networking XPC pid created since the pre-launch process
snapshot (WebKit services are launchd-spawned, ppid 1 — NOT app children).
Post-quit every counted pid must be gone or the run is INVALID. Leak:
pid-set growth leak-start→leak-end plus the post-quit sweep.

Statistics: --runs N (default 5) per arm × 30 swaps; swap 1 dropped.
swap median; p95 nearest-rank; cold = median of per-run cold latencies;
plateau = median of per-run RSS30/RSS10; peak = median of per-run max RSS
inside the 100-page window (peak-start → 2 s after that publication's t1,
R2). No outlier removal; raw samples persist.

Gates — all 7 required, any missing = FAIL (G2):
  functional (both arms complete every run), swap_median (k <= c*1.5+100),
  swap_p95 (k <= c_p95*2), cold (k <= c+500), plateau (k <= 1.10),
  peak (k <= c*2), leak (pid growth 0 + clean post-quit).

Usage: check-pdfjs-perf.py <products> <payload> <repo> [--runs N]
       [--arms candidate,control] [--artifacts DIR] [--control PATH]
The aggregator is independently exercisable: `--selfcheck` runs the
fail-closed unit checks (missing arm, partial runs, failed run, leaked
pid, unpaired publication) without launching anything.
"""
import argparse
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import plistlib
import re
MARKER = re.compile(r"/Check\.swift:\d+:\d+: warning: (?:no 'async' operations occur within 'await' expression|no calls to throwing functions occur within 'try' expression)(?: \[#UnnecessaryEffectMarker\])?$", re.M)
import shutil
import statistics
import subprocess
import sys
import tempfile
import threading
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("products", type=Path, nargs="?")
parser.add_argument("payload", type=Path, nargs="?")
parser.add_argument("repo", type=Path, nargs="?")
parser.add_argument("--runs", type=int, default=5)
parser.add_argument("--arms", default="candidate,control")
parser.add_argument("--artifacts", type=Path)
parser.add_argument("--control", type=Path)
parser.add_argument("--selfcheck", action="store_true",
                    help="aggregator fail-closed checks only; no app launch")
parser.add_argument("--compile-only", dest="compile_only", action="store_true",
                    help="patch + swiftc-compile each arm, save evidence; no app launch, no run, no lock")
parser.add_argument("--warmup-only", dest="warmup_only", action="store_true",
                    help="build both arms + run each once (warm-up) then judge validity; no measured runs")
args = parser.parse_args()

CONTROL_PREVIEW_SHA = "9b0446fadb6910fa149360278c567ff989211ea187396446bf9a663d5b737c0e"
SWAPS = 30

# ---------------------------------------------------------------------------
# Aggregator — pure function over per-run results, locally testable.
# A "run" dict: {"ok": bool, "forced": bool, "result": {...}|None,
#                "samples": [(t, rss, frozenset(pids))], "marks": {name: t},
#                "leaked_after_quit": [pid], "launch_mono": t, "exit_mono": t}
# ---------------------------------------------------------------------------

def p95(xs):
    xs = sorted(xs)
    return xs[math.ceil(0.95 * len(xs)) - 1]

def sample_at(samples, mark_t, delay=0.0):
    if not samples or mark_t is None:
        return None
    tgt = mark_t + delay
    return min(samples, key=lambda s: abs(s[0] - tgt))

def compute_verdict(runs_by_arm, expected_runs):
    """Fail-closed verdict. Any missing gate or invalid run → no PASS."""
    verdict = {"arms": {}, "gates": {}, "invalid_runs": []}
    # Every run must supply the hook receipts its arm requires — a run
    # missing any is INVALID (the patch may never have fired).
    NEED_HOOKS = {"candidate": {"mount", "t0", "displayed", "t1"},
                  "control": {"mount", "t0-mount", "t0", "t1"}}
    for arm in ("candidate", "control"):
        rs = runs_by_arm.get(arm, [])
        valid = []
        for r in rs:
            res = r.get("result") or {}
            missing_hooks = NEED_HOOKS[arm] - set(res.get("hooks", []))
            unresolved = res.get("unresolved_keys", [])
            # T6: every mark must lie inside [launch, exit] on the shared
            # monotonic epoch.
            bad_mark = [n for n, t in r.get("marks", {}).items()
                        if not (r.get("launch_mono", -1) <= t <=
                                r.get("exit_mono", float("inf")))]
            # P5: unresolved timed keys or leaked pids or a non-clean run
            # are all INVALID — never silently counted.
            if (bad_mark or r.get("leaked_after_quit") or not r.get("ok")
                    or missing_hooks or unresolved):
                verdict["invalid_runs"].append({
                    "arm": arm, "bad_marks": bad_mark,
                    "leaked": r.get("leaked_after_quit", []),
                    "missing_hooks": sorted(missing_hooks),
                    "unresolved_keys": unresolved,
                    "ok": r.get("ok")})
            else:
                valid.append(r)
        entry = {"runs_ok": len(valid), "runs_total": len(rs),
                 "runs_expected": expected_runs}
        if valid:
            swap_all = [ms for r in valid for ms in
                        r["result"].get("swap_ms", [])[1:]]
            colds = [r["result"]["cold_ms"] for r in valid
                     if r["result"].get("cold_ms") is not None]
            ratios, peaks, leak_growth = [], [], []
            for r in valid:
                m, s = r["marks"], r["samples"]
                if "plateau-10" in m and "plateau-30" in m:
                    a, b = sample_at(s, m["plateau-10"], 1.0), \
                           sample_at(s, m["plateau-30"], 1.0)
                    if a and b and a[1]:
                        ratios.append(b[1] / a[1])
                if "peak-start" in m and "peak-t1" in m:
                    w = [rss for ts, rss, _, _ in s
                         if m["peak-start"] <= ts <= m["peak-t1"] + 2]
                    if w:
                        peaks.append(max(w))
                # P7: leak = COUNT of new-WebContent pids at leak-end vs
                # leak-start, not a set diff (a recycled pid is fine).
                if "leak-start" in m and "leak-end" in m:
                    a, b = sample_at(s, m["leak-start"]), sample_at(s, m["leak-end"])
                    if a and b:
                        leak_growth.append(len(b[2]) - len(a[2]))
            entry.update({
                "swap_median_ms": statistics.median(swap_all) if swap_all else None,
                "swap_p95_ms": p95(swap_all) if swap_all else None,
                "cold_median_ms": statistics.median(colds) if colds else None,
                "plateau_ratio_median": statistics.median(ratios) if ratios else None,
                "peak_median_kb": statistics.median(peaks) if peaks else None,
                # P7 fail-closed: missing marks → None → gate False.
                "leak_pid_growth": (max(leak_growth) if leak_growth else None),
                "superseded": sum(r["result"].get("superseded", 0) for r in valid),
                "raw_swap_ms": [r["result"].get("swap_ms", []) for r in valid],
            })
        verdict["arms"][arm] = entry

    c, k = verdict["arms"]["control"], verdict["arms"]["candidate"]
    g = verdict["gates"]
    # G1: every expected run valid in BOTH arms.
    g["functional"] = (
        c["runs_ok"] == expected_runs == c["runs_total"] and
        k["runs_ok"] == expected_runs == k["runs_total"] and
        not verdict["invalid_runs"])
    # G2: every gate present; missing data ⇒ False, never absent.
    def both(field):
        return c.get(field) is not None and k.get(field) is not None
    g["swap_median"] = (both("swap_median_ms") and
                        k["swap_median_ms"] <= c["swap_median_ms"] * 1.5 + 100)
    g["swap_p95"] = (both("swap_p95_ms") and
                     k["swap_p95_ms"] <= c["swap_p95_ms"] * 2.0)
    g["cold"] = (both("cold_median_ms") and
                 k["cold_median_ms"] <= c["cold_median_ms"] + 500)
    g["plateau"] = (k.get("plateau_ratio_median") is not None and
                    k["plateau_ratio_median"] <= 1.10)
    g["peak"] = (both("peak_median_kb") and
                 k["peak_median_kb"] <= c["peak_median_kb"] * 2.0)
    g["leak"] = (k.get("leak_pid_growth") == 0)
    return verdict

# ---------------------------------------------------------------------------
# Fail-closed self-check — exercised locally, no app launch.
# ---------------------------------------------------------------------------

def selfcheck() -> int:
    """Fail-closed aggregator checks — no app launch. Every scenario must
    refuse PASS except the deliberately-clean baseline."""
    HOOKS_C = ["displayed", "mount", "t0", "t1"]
    HOOKS_K = ["mount", "t0", "t0-mount", "t1"]

    def fake_run(arm, ok=True, swaps=None, cold=100.0, marks=None,
                 samples=None, leaked=None, launch=0.0, exit_=1000.0,
                 superseded=0, hooks=None, unresolved=None, forced=False):
        good_hooks = HOOKS_C if arm == "candidate" else HOOKS_K
        wc = frozenset({1001})
        return {
            "ok": ok, "forced": forced or not ok,
            "result": {"swap_ms": swaps or [100.0] * 30, "cold_ms": cold,
                       "superseded": superseded,
                       "unresolved_keys": unresolved or [],
                       "hooks": hooks if hooks is not None else good_hooks}
                      if ok else None,
            "samples": samples or
                [(t, 100_000, wc, 5_000) for t in
                 (10, 100, 200, 300, 400, 500, 600, 700, 800, 950)],
            "marks": marks if marks is not None else {
                "baseline": 50, "plateau-10": 200, "plateau-30": 400,
                "peak-start": 600, "peak-t1": 700,
                "leak-start": 800, "leak-end": 900},
            "leaked_after_quit": leaked or [],
            "launch_mono": launch, "exit_mono": exit_}

    arms = ("candidate", "control")
    good = {a: [fake_run(a) for _ in range(3)] for a in arms}
    fails = 0

    def expect(name, runs, n, want_pass):
        v = compute_verdict(runs, n)
        passed = all(v["gates"].values()) and len(v["gates"]) == 7
        ok = (passed == want_pass)
        print(f"  {'ok' if ok else 'XX'} {name}: pass={passed} want={want_pass} "
              f"gates={v['gates']}")
        return ok

    # 1. all-good → PASS.
    if not expect("all-good", good, 3, True): fails += 1
    # 2. missing arm → FAIL.
    if not expect("missing-arm", {"candidate": good["candidate"]}, 3, False): fails += 1
    # 3. 1-of-3 runs → FAIL.
    bad = {a: [fake_run(a)] + [fake_run(a, ok=False)] * 2 for a in arms}
    if not expect("1-of-3-runs", bad, 3, False): fails += 1
    # 4. leaked WebContent pid post-quit → FAIL.
    leak = {a: [fake_run(a) for _ in range(3)] for a in arms}
    leak["candidate"][0]["leaked_after_quit"] = [4242]
    if not expect("leaked-pid", leak, 3, False): fails += 1
    # 5. mark outside run window → INVALID → FAIL (T6).
    oob = {a: [fake_run(a) for _ in range(3)] for a in arms}
    oob["candidate"][1]["marks"]["baseline"] = 99999.0
    if not expect("oob-mark", oob, 3, False): fails += 1
    # 6. slow candidate → FAIL.
    slow = {a: [fake_run(a) for _ in range(3)] for a in arms}
    for r in slow["candidate"]:
        r["result"]["swap_ms"] = [500.0] * 30
    if not expect("slow-candidate", slow, 3, False): fails += 1
    # 7. leak marks missing → leak gate False → FAIL (P7).
    nomarks = {a: [fake_run(a) for _ in range(3)] for a in arms}
    for r in nomarks["candidate"]:
        r["marks"] = {"baseline": 50, "plateau-10": 200, "plateau-30": 400,
                      "peak-start": 600, "peak-t1": 700}
    if not expect("missing-leak-marks", nomarks, 3, False): fails += 1
    # 8. replaced WebContent (same count, different pid) → PASS (P7).
    rep = {a: [fake_run(a) for _ in range(3)] for a in arms}
    for r in rep["candidate"]:
        r["samples"] = [(t, 100_000, frozenset({1001 if t < 500 else 2002}), 5_000)
                        for t in (10, 100, 200, 300, 400, 500, 600, 700, 800, 950)]
    if not expect("replaced-webcontent", rep, 3, True): fails += 1
    # 9. missing hook receipt → INVALID → FAIL.
    nohook = {a: [fake_run(a) for _ in range(3)] for a in arms}
    nohook["candidate"][0]["result"]["hooks"] = ["mount", "t0", "t1"]  # no displayed
    if not expect("missing-hook", nohook, 3, False): fails += 1
    # 10. unresolved timed key → INVALID → FAIL (P5).
    unres = {a: [fake_run(a) for _ in range(3)] for a in arms}
    unres["control"][0]["result"]["unresolved_keys"] = [7]
    if not expect("unresolved-key", unres, 3, False): fails += 1
    # 11. forced run → FAIL.
    forced = {a: [fake_run(a) for _ in range(3)] for a in arms}
    forced["control"][0]["forced"] = True
    forced["control"][0]["ok"] = False
    forced["control"][0]["result"] = None
    if not expect("forced-run", forced, 3, False): fails += 1
    # 12. superseded counted, doesn't break PASS.
    sup = {a: [fake_run(a, superseded=2) for _ in range(3)] for a in arms}
    v = compute_verdict(sup, 3)
    ok = all(v["gates"].values()) and v["arms"]["candidate"]["superseded"] == 6
    print(f"  {'ok' if ok else 'XX'} superseded-counted: {v['arms']['candidate'].get('superseded')}")
    if not ok: fails += 1

    print(f"selfcheck: {'PASS' if fails == 0 else f'{fails} FAILURES'}")
    return 0 if fails == 0 else 1

# ---------------------------------------------------------------------------
if args.selfcheck:
    sys.exit(selfcheck())

if not (args.products and args.payload and args.repo):
    sys.exit("products/payload/repo required (or --selfcheck)")
products = args.products.resolve()
payload = args.payload.resolve()
repo = args.repo.resolve()
artifacts = (args.artifacts.resolve() if args.artifacts
             else Path(tempfile.mkdtemp(prefix="pitex-perf-evidence-")))
artifacts.mkdir(parents=True, exist_ok=True)
if args.control is None:
    sys.exit("--control PATH required for a perf run")
CONTROL_PREVIEW = args.control.resolve()
print(f"[diag] perf evidence {artifacts}", flush=True)

native_resolution = {
    "DEVELOPER_DIR": os.environ.get("DEVELOPER_DIR"),
    "inheritedSDKROOT": os.environ.get("SDKROOT"),
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
        "stdout": result.stdout, "stderr": result.stderr})
    native_resolution[key] = result.stdout.strip()
    result.check_returncode()
native_environment = {**os.environ, "SDKROOT": native_resolution["sdk"]}
(artifacts / "native-toolchain.json").write_text(json.dumps(native_resolution, indent=2))

def verify_pdfjs_manifest(pdfjs_dir: Path):
    lines = [l.split(None, 1) for l in
             (pdfjs_dir / "MANIFEST.sha256").read_text().splitlines() if l.strip()]
    assert lines, f"pdfjs manifest empty at {pdfjs_dir}"
    bad = [p for h, p in lines
           if hashlib.sha256((pdfjs_dir / p.lstrip("./")).read_bytes()).hexdigest() != h]
    assert not bad, f"pdfjs payload diverged from MANIFEST.sha256: {bad}"

fixture_main = ("\\documentclass{article}\n\\usepackage{amsmath}\n"
                "\\begin{document}\n\\title{Perf Fixture}\\maketitle\n"
                "\\section{Alpha}\nPerfBaselineMarker\\par\n" +
                "\\par\\medskip\n".join(
                    f"Paragraph {i} with filler $\\int_0^{{{i}}} x^2 dx$."
                    for i in range(1, 40)) +
                "\n\\end{document}\n")
fixture_big = ("\\documentclass{article}\n\\usepackage{amsmath}\n"
               "\\begin{document}\n" +
               "\\newpage\n".join(
                   f"\\section{{Peak {i}}}\nPeakMarker{i}\\par filler\n"
                   for i in range(1, 101)) +
               "\n\\end{document}\n")

PERF_CHECK = r'''
import AppKit
import Darwin
import PDFKit
import SwiftUI
import SyncTeXCore
import WebKit

enum LaunchFlag {
    nonisolated(unsafe) static var didFinishLaunching = false
}

/// Records only bridge ERROR traffic and the worker-ok flag — retaining
/// every post (one per page render) would bias the candidate-only plateau.
/// The compiled Preview.swift copy is patched to call BridgeProbe.log at
/// the pdf.js message-handler entry (the 4 original ports do the same;
/// perf is checker #5).
enum BridgeProbe {
    nonisolated(unsafe) static var errors: [String] = []
    nonisolated(unsafe) static var workerOK = false
    nonisolated(unsafe) static var workerDestroyed = false
    static func log(_ type: String, _ m: [String: Any]) {
        if ["load-error", "destroy-error", "csp", "jserror"].contains(type) {
            errors.append(type)
        }
        if type == "worker-ok" {
            if (m["destroyed"] as? Bool) == true { workerDestroyed = true }
            else { workerOK = true }
        }
    }
    /// "" when clean; error traffic and a missing/destroyed worker-ok fail.
    static func audit() -> String {
        var bad = errors
        if workerDestroyed { bad.append("worker-ok destroyed") }
        if !workerOK { bad.append("no worker-ok post") }
        return bad.isEmpty ? "" : "bridge audit failed: " + bad.joined(separator: ", ")
    }
}


/// Stamps consumed by the patched Preview.swift copy — product bytes are
/// never touched. Pairing is per-PUBLICATION: candidate by generation,
/// control by PDFDocument identity. A push that never gets a qualifying
/// paint before the next push is counted as superseded, not dropped.
enum PerfProbe {
    nonisolated(unsafe) static var nextKey = 0
    nonisolated(unsafe) static var t0s: [Int: Double] = [:]   // key -> t
    nonisolated(unsafe) static var t1s: [Int: Double] = [:]   // key -> t
    nonisolated(unsafe) static var superseded = 0
    /// Candidate: key per gen; the displayed page each gen must paint.
    nonisolated(unsafe) static var genKey: [Int: Int] = [:]
    nonisolated(unsafe) static var displayedPage: [Int: Int] = [:]
    /// Control: doc identity per key; observer pairs when view.document
    /// IS that doc and the view is clean.
    nonisolated(unsafe) static weak var controlView: PDFView?
    nonisolated(unsafe) static var controlDocs: [Int: PDFDocument] = [:]
    nonisolated(unsafe) static var controlSeen = false
    /// The most recent push key — superseded bookkeeping.
    nonisolated(unsafe) static var lastKey = 0
    /// T3: makeNSView entry on BOTH arms — the cold datum covers webview
    /// + page + worker init for the candidate and PDFView creation for
    /// the control.
    nonisolated(unsafe) static var mountT: Double = 0
    /// Fail-closed hook receipts — a run missing any required hook is
    /// INVALID, so a misplaced patch can never fake coverage.
    nonisolated(unsafe) static var hooked: Set<String> = []
    /// A-1: the publication bytes per key, kept only for the live window —
    /// a carrying-check needs the data; a finished window holds nothing.
    nonisolated(unsafe) static var keyData: [Int: Data] = [:]

    static func now() -> Double {
        Double(clock_gettime_nsec_np(CLOCK_UPTIME_RAW)) / 1e9
    }
    static func emit(_ kind: String, _ extra: String = "") {
        print("[perf] {\"event\":\"\(kind)\",\"t\":\(now())\(extra)}")
        fflush(nil)
    }
    static func hook(_ name: String) {
        guard hooked.insert(name).inserted else { return }
        emit("hook", ",\"name\":\"\(name)\"")
    }
    static func mount() { mountT = now(); hook("mount"); emit("mount") }
    /// Document-push stamp. Candidate: gen-bound; carries its publication
    /// bytes. Control: key-bound; carries no data (the doc is keyed instead).
    /// `site` distinguishes the control's mount-time push ("t0-mount") from
    /// updateNSView pushes so every injected hook proves it fired.
    @discardableResult
    static func t0(gen: Int, site: String = "t0", data: Data? = nil) -> Int {
        hook(site)
        // Anything still unpainted when a new push lands is superseded —
        // Q3: drop its bytes AND its control doc immediately.
        if lastKey != 0, t1s[lastKey] == nil {
            t1s[lastKey] = -1; superseded += 1
            keyData[lastKey] = nil; controlDocs[lastKey] = nil
            emit("superseded", ",\"key\":\(lastKey)")
        }
        nextKey += 1
        let k = nextKey
        t0s[k] = now()
        keyData[k] = data
        if gen >= 0 { genKey[gen] = k }
        lastKey = k
        emit("t0", ",\"key\":\(k),\"gen\":\(gen)")
        return k
    }
    /// Control: remember which document this key installed.
    static func t0Doc(_ doc: PDFDocument, key: Int) { controlDocs[key] = doc }
    /// Candidate: the displayed page for a gen (posted at pagesinit).
    static func displayed(gen: Int, page: Int) {
        displayedPage[gen] = page; hook("displayed")
    }
    /// Candidate t1: qualifying pagerendered — gen-matched, displayed
    /// page, `error` KEY PRESENT and NSNull (absent ≠ clean), no
    /// cssTransform, not a detail.
    static func t1(gen: Int, page: Int, error: Any?,
                 hasErrorKey: Bool, cssTransform: Bool, detail: Bool) {
        let errOK = hasErrorKey && error is NSNull
        guard errOK, !cssTransform, !detail,
              let key = genKey[gen], t1s[key] == nil,
              displayedPage[gen] == page else { return }
        t1s[key] = now()
        hook("t1")
        emit("t1", ",\"key\":\(key),\"gen\":\(gen)")
    }
    /// Control t1: beforeWaiting at order CFIndex.max (post-CA-commit),
    /// the observed document IS the one this key installed, view clean.
    /// PDFView members are @MainActor — run inside the assumeIsolated caller.
    @MainActor static func t1Control() {
        guard let v = controlView, let d = v.document,
              let dv = v.documentView, !dv.needsDisplay else { return }
        for (key, doc) in controlDocs where doc === d && t1s[key] == nil {
            t1s[key] = now()
            hook("t1")
            emit("t1", ",\"key\":\(key)")
        }
    }
    static func mark(_ name: String) { emit("mark", ",\"name\":\"\(name)\"") }

    /// A-1/Q1: does this publication's data carry the edit? Data-side
    /// PDFKit read (non-drawing), identical in both arms.
    static func carries(_ key: Int, marker: String) -> Bool {
        if let d = keyData[key] {          // candidate: publication bytes
            return PDFDocument(data: d)?.string?.contains(marker) ?? false
        }
        if let d = controlDocs[key] {      // control: the keyed PDFDocument
            return d.string?.contains(marker) ?? false
        }
        return false
    }
    /// First painted key >= `from` whose publication carries `marker`.
    /// Ascending key order = push order.
    static func firstCarryingPaint(from: Int, marker: String) -> Double? {
        for k in t0s.keys.sorted() where k >= from {
            if (t1s[k] ?? 0) > 0 && carries(k, marker: marker) {
                return t1s[k]!
            }
        }
        return nil
    }
    /// Retire the measurement window: drop publication bytes/docs for keys
    /// below `key` so a finished window holds nothing (Q3).
    static func releaseWindow(below key: Int) {
        for k in keyData.keys where k < key { keyData[k] = nil }
        for k in controlDocs.keys where k < key { controlDocs[k] = nil }
    }
}
@MainActor enum Driver {
    static let root = ProcessInfo.processInfo.environment["PITEX_CHECK_FIXTURE"] ?? ""
    static let arm = ProcessInfo.processInfo.environment["PITEX_PERF_ARM"] ?? "candidate"
    static var workspace: WorkspaceModel!
    static var project: URL { URL(fileURLWithPath: root).standardizedFileURL }
    static var lastText: String?
    static var lastJSError: String?
    /// Last JS eval error — surfaced into marker-missing diagnostics so a
    /// swallowed callAsyncJavaScript failure is attributable, not opaque.

    static func require(_ ok: Bool, _ msg: String) {
        if !ok { print("FAIL \(msg)"); fflush(nil); exit(1) }
    }
    static func until(_ s: Double = 15, _ c: @escaping @MainActor () -> Bool) async -> Bool {
        let deadline = ContinuousClock.now + .milliseconds(Int(s * 1000))
        while ContinuousClock.now < deadline {
            if c() { return true }
            try? await Task.sleep(for: .milliseconds(50))
        }
        return c()
    }
    static func insert(_ text: String) {
        guard let tv = workspace.environment?.editor.textView else {
            require(false, "no text view"); return
        }
        let t = (tv.string as NSString).range(of: "\\end{document}", options: .backwards)
        require(t.location != NSNotFound, "no terminator")
        let r = NSRange(location: t.location, length: 0)
        tv.replaceCharacters(in: r, with: text)
        tv.didChangeText()
    }
    static func findView<T: NSView>(_ type: T.Type, in root: NSView) -> T? {
        for sub in root.subviews.reversed() {
            if let hit = sub as? T { return hit }
            if let hit = findView(type, in: sub) { return hit }
        }
        return nil
    }
    /// The candidate preview is the WKWebView hosting pitex-pdfjs://app —
    /// select by scheme, never by WKWebView class, so auxiliary web views
    /// (settings, about) are never read.
    static func pitexWeb(in root: NSView?) -> WKWebView? {
        guard let root else { return nil }
        if let web = root as? WKWebView,
           web.url?.scheme == "pitex-pdfjs" { return web }
        for sub in root.subviews {
            if let hit = pitexWeb(in: sub) { return hit }
        }
        return nil
    }
    /// Rendered text post-paint (correctness, never timed). Control reads
    /// the DISPLAYED document (G4), not the publication.
    static func refreshText() async {
        if arm == "candidate" {
            guard let wv = pitexWeb(in: workspace.window?.contentView) else {
                lastText = nil
                lastJSError = "no pitex-pdfjs webview"
                return
            }
            lastText = await withCheckedContinuation { cont in
                wv.callAsyncJavaScript(
                    "const g = window.pitex ? window.pitex.state() : -1;"
                    + " return g >= 0 ? window.pitex.text(g) : null;",
                    arguments: [:], in: nil, in: .page) { r in
                        switch r {
                        case let .success(v):
                            lastJSError = nil
                            cont.resume(returning: v as? String)
                        case let .failure(e):
                            lastJSError = String(describing: e)
                            cont.resume(returning: nil)
                        }
                    }
            }
        } else {
            lastText = PerfProbe.controlView?.document?.string
        }
    }
    /// One swap = one unsaved edit → latency from THIS edit's first
    /// observed publication t0 to the first qualifying paint of that key
    /// or ANY successor key carrying the edit (P6: latest-wins preserved,
    /// superseded counted, slow samples never dropped).
    static func swap(_ i: Int, marker: String) async -> Double {
        let keyAtPush = PerfProbe.nextKey + 1   // this edit's first push
        insert("\n\(marker)\\par\n")
        require(await until(30) { PerfProbe.nextKey >= keyAtPush },
                "swap \(i): no document push")
        // Wait until the NEWEST key for this edit has resolved to a paint
        // (>0) — intermediate keys may be superseded (-1) and still count
        // toward the sample via the successor rule below.
        require(await until(60) {
            guard let newest = PerfProbe.t0s.keys.max(), newest >= keyAtPush
            else { return false }
            return (PerfProbe.t1s[newest] ?? 0) > 0
                && workspace.embeddedPreviewStatus == .current
        }, "swap \(i): newest publication never painted")
        // Q1: latency ends at the first painted key ≥ this edit's push whose
        // publication DATA contains the marker (data-side, non-drawing,
        // identical in both arms). A partial intermediate publication that
        // paints without the marker does NOT end the sample.
        let t0 = PerfProbe.t0s[keyAtPush]!
        require(PerfProbe.firstCarryingPaint(from: keyAtPush,
                                             marker: marker) != nil,
                "swap \(i): no painted publication carries the marker")
        let t1 = PerfProbe.firstCarryingPaint(from: keyAtPush,
                                              marker: marker)!
        let ms = (t1 - t0) * 1000.0
        // Q1: release this window's publication bytes/docs before the next
        // measurement so retained data never inflates a later sample.
        PerfProbe.releaseWindow(below: PerfProbe.nextKey + 1)
        await refreshText()
        require(lastText?.contains(marker) == true,
                "swap \(i): marker missing from rendered output "
                + "(lastJSError=\(String(describing: lastJSError)))")
        return ms
    }

    static func run() async {
        // P3: the control paint observer installs FIRST — before any
        // workspace setup — so the cold mount doc can never race it.
        if arm == "control" {
            let obs = CFRunLoopObserverCreateWithHandler(
                nil, CFRunLoopActivity.beforeWaiting.rawValue, true,
                CFIndex.max) { _, _ in
                MainActor.assumeIsolated {
                    if PerfProbe.controlView == nil,
                       let root = WorkspaceWindows.live
                           .first?.window?.contentView {
                        PerfProbe.controlView = findView(PDFView.self, in: root)
                    }
                    PerfProbe.t1Control()
                }
            }
            CFRunLoopAddObserver(CFRunLoopGetMain(), obs, .commonModes)
        }

        require(await until { WorkspaceWindows.live.contains { $0.window != nil } },
                "no window")
        workspace = WorkspaceWindows.live[0]
        SettingsStore.shared.liveCompileEnabled = true
        SettingsStore.shared.livePreviewBackend = "embedded"
        SettingsStore.shared.autoSave = false
        SettingsStore.shared.restoreSession = false
        workspace.syncLiveScheduler()
        WorkspaceWindows.route(project)
        require(await until { workspace.hasProject }, "project never opened")
        require(await until(60) {
            if case .ready = workspace.phase { return true }; return false
        }, "never .ready")

        // Cold: mount → first qualifying paint of the FIRST publication.
        PerfProbe.mark("open")
        insert("\n% cold-trigger\n")
        // H1: the preview mounts lazily on first layout — wait bounded for
        // makeNSView entry (t0 = mount stamp) instead of reading once.
        require(await until(90) { PerfProbe.mountT > 0 },
                "no makeNSView mount stamp")
        let coldT0 = PerfProbe.mountT
        require(await until(90) { PerfProbe.t1s.values.contains { $0 > 0 } },
                "no cold paint (a superseded −1 does not count)")
        let coldT1 = PerfProbe.t1s.values.filter { $0 > 0 }.min()!
        let coldMs = (coldT1 - coldT0) * 1000.0
        require(await until(90) {
            workspace.embeddedPreviewStatus == .current
        }, "initial preview never settled")
        PerfProbe.mark("baseline")
        try? await Task.sleep(for: .seconds(2))

        var swapMs: [Double] = []
        for i in 1...SWAPS_N {
            swapMs.append(await swap(i, marker: "PerfMarker\(i)"))
            if i == 10 || i == 30 {
                PerfProbe.mark("plateau-\(i)")
                try? await Task.sleep(for: .seconds(1))
            }
        }
        PerfProbe.mark("swaps-done")

        // Peak: the 100-page publication's own t1 ends the window (R2).
        WorkspaceWindows.route(project.appendingPathComponent("big.tex"))
        require(await until(20) {
            workspace.activeDocumentURL?.lastPathComponent == "big.tex" },
                "big.tex not active")
        let peakKey = PerfProbe.nextKey + 1
        PerfProbe.mark("peak-start")
        // Peak: ends at the first paint carrying PeakEndMarker (Q2 — a real
        // text run, not a % comment, so the carrying rule works even when an
        // intermediate push supersedes peakKey).
        insert("\nPeakEndMarker\\par\n")
        require(await until(120) {
            PerfProbe.firstCarryingPaint(from: peakKey,
                                         marker: "PeakEndMarker") != nil
        }, "big.tex never painted a publication carrying PeakEndMarker")
        PerfProbe.mark("peak-t1")
        PerfProbe.releaseWindow(below: PerfProbe.nextKey + 1)
        try? await Task.sleep(for: .seconds(2))
        PerfProbe.mark("peak-end")

        // Q4: settle before freezing unresolved keys — the newest push must
        // have painted and the preview reported .current (≤60 s), else an
        // in-flight successor reads as a false unresolved key.
        require(await until(60) {
            guard let newest = PerfProbe.t0s.keys.max() else { return false }
            return (PerfProbe.t1s[newest] ?? 0) > 0
                && workspace.embeddedPreviewStatus == .current
        }, "post-peak settle: newest push never painted/.current")


        // Leak: 5 detach/attach cycles.
        // P5: unresolved TIMED keys freeze here — leak-phase pushes are
        // detach mounts, exempt by definition.
        let unresolved = PerfProbe.t0s.keys.filter { PerfProbe.t1s[$0] == nil }
        PerfProbe.mark("leak-start")
        for _ in 0..<5 {
            workspace.detachPreview()
            require(await until(10) { workspace.detachedPreviewWindow != nil },
                    "detach failed")
            workspace.attachPreview()
            require(await until(10) { workspace.detachedPreviewWindow == nil },
                    "attach failed")
        }
        try? await Task.sleep(for: .seconds(10))
        PerfProbe.mark("leak-end")

        // Bridge audit — candidate arm only (control arm is PDFView, no
        // pdf.js posts). Fails the run on error traffic / destroyed worker.
        if arm == "candidate" {
            let e = BridgeProbe.audit()
            require(e.isEmpty, e)
        }

        let payload: [String: Any] = [
            "arm": arm, "cold_ms": coldMs, "swap_ms": swapMs,
            "superseded": PerfProbe.superseded,
            "unresolved_keys": unresolved.sorted(),
            "hooks": PerfProbe.hooked.sorted(),
            "pushes": PerfProbe.t0s.count, "paints": PerfProbe.t1s.count]
        let data = try! JSONSerialization.data(withJSONObject: payload)
        print("[perf-result] \(String(data: data, encoding: .utf8)!)")
        print("PERF_COMPLETE")
        fflush(nil)
        NSApp.terminate(nil)
    }
}
'''
PERF_CHECK = PERF_CHECK.replace("SWAPS_N", str(SWAPS))

def patch_app(root: Path) -> Path:
    src = (repo / "Mac/Sources/AppShell/PitexApp.swift").read_text()
    # Launch anchor: the function signature line. The real app never sets
    # LaunchFlag.didFinishLaunching — that symbol only exists in the
    # synthetic apps other checkers generate. Driver starts at the top of
    # applicationDidFinishLaunching (whole-line anchor, ends before the '{').
    needle = "    func applicationDidFinishLaunching(_ notification: Notification) {"
    assert src.count(needle) == 1, f"launch anchor not unique: {src.count(needle)}"
    patched = src.replace(
        needle,
        needle + "\n        Task { @MainActor in await Driver.run() }")
    assert "await Driver.run()" in patched
    out = root / "PitexApp.swift"
    out.write_text(patched)
    return out

def patch_preview(root: Path, arm: str) -> Path:
    if arm == "control":
        assert CONTROL_PREVIEW.is_file(), \
            f"control source missing: {CONTROL_PREVIEW}"
        raw = CONTROL_PREVIEW.read_text()
        digest = hashlib.sha256(CONTROL_PREVIEW.read_bytes()).hexdigest()
        assert digest == CONTROL_PREVIEW_SHA, \
            f"control Preview.swift sha {digest} != {CONTROL_PREVIEW_SHA}"
        # makeNSView entry (cold t0) — the function signature line.
        mount_anchor = "    func makeNSView(context: Context) -> PDFView {"
        assert raw.count(mount_anchor) == 1, "control mount anchor missing"
        raw = raw.replace(mount_anchor,
            mount_anchor + "\n        PerfProbe.mount()")
        # P2: the mount-time document is keyed too — cold = mount → its
        # paint, and the big.tex remount is keyed the same way.
        mount_doc = ("        view.displaysPageBreaks = true\n"
                     "        view.document = PDFDocument(data: data)")
        assert raw.count(mount_doc) == 1, "control mount-doc anchor missing"
        raw = raw.replace(mount_doc,
            "        view.displaysPageBreaks = true\n"
            "        let __mk = PerfProbe.t0(gen: -1, site: \"t0-mount\")\n"
            "        view.document = PDFDocument(data: data)\n"
            "        if let __d = view.document { PerfProbe.t0Doc(__d, key: __mk) }")
        # updateNSView's document hand-off — retiredDocument makes it
        # unique vs the makeNSView assignment. Key bound to the doc.
        anchor = ("        context.coordinator.retiredDocument = view.document\n"
                  "        view.document = PDFDocument(data: data)")
        assert raw.count(anchor) == 1, "control patch anchor not unique"
        raw = raw.replace(anchor,
            "        let __pk = PerfProbe.t0(gen: -1, site: \"t0\")\n"
            "        view.document = PDFDocument(data: data)\n"
            "        if let __d = view.document { PerfProbe.t0Doc(__d, key: __pk) }")
    else:
        raw = (repo / "Mac/Sources/Features/Preview.swift").read_text()
        # pdf.js's makeNSView — the markdown view has the same signature,
        # so anchor on the pdf.js coordinator init below it.
        mount_anchor = ("    func makeNSView(context: Context) -> WKWebView {\n"
                        "        let coordinator = context.coordinator")
        assert raw.count(mount_anchor) == 1, "candidate mount anchor missing"
        raw = raw.replace(mount_anchor,
            "    func makeNSView(context: Context) -> WKWebView {\n"
            "        PerfProbe.mount()\n"
            "        let coordinator = context.coordinator")
        push = '            js("return window.pitex.load(gen, mediaBoxes, sameTarget);",'
        assert raw.count(push) == 1, "candidate t0 anchor missing"
        raw = raw.replace(push,
            '            PerfProbe.t0(gen: generation, site: "t0", data: data)\n' + push)
        # Bridge audit: log EVERY pdf.js bridge post at handler entry so a
        # load/destroy/csp/js error or a missing/destroyed worker-ok fails
        # the run (the 4 original ports enforce the same; perf is #5).
        bridge = ('            guard let m = message.body as? [String: Any],\n'
                  '                  let type = m["type"] as? String else { return }')
        assert raw.count(bridge) == 1, "candidate bridge anchor missing"
        raw = raw.replace(bridge,
            bridge + '\n            BridgeProbe.log(type, m)')
        # Record displayed-page INSIDE the product's own case — a second
        # `case "displayed":` would never run (first match wins, P1). The
        # anchor is the `if let g` guard (binds g, unique to this case);
        # hook runs before the guard body so the paint is recorded first.
        # `g` is bound inside the `if let g` body → anchor on its first
        # statement so the inserted hook has g in scope.
        disp = "                    scheme.evictBelow(g)"
        assert raw.count(disp) == 1, "candidate displayed anchor missing"
        raw = raw.replace(disp,
            '                    PerfProbe.displayed(gen: g, page: m["page"] as? Int ?? -1)\n'
            + disp)
        anchor = '            case "inverse":'
        assert raw.count(anchor) == 1, "candidate t1 anchor missing"
        raw = raw.replace(anchor,
            '            case "pagerendered":\n'
            '                PerfProbe.t1(gen: m["gen"] as? Int ?? -2,\n'
            '                             page: m["page"] as? Int ?? -1,\n'
            '                             error: m["error"],\n'
            '                             hasErrorKey: m.keys.contains("error"),\n'
            '                             cssTransform: m["cssTransform"] as? Bool ?? true,\n'
            '                             detail: m["detail"] as? Bool ?? true)\n'
            + anchor)
        # Fail-closed placement proof: the displayed hook must sit textually
        # inside the product's displayed case (between `case` and the next
        # `case`/`default`), and the t1 case before "inverse".
        disp_case = raw.index('case "displayed":')
        next_case = raw.index('case "load-error"')
        assert disp_case < raw.index('PerfProbe.displayed') < next_case, \
            "displayed hook outside the displayed case"
        assert raw.index('case "pagerendered"') < raw.index('case "inverse"'), \
            "pagerendered case not before inverse"
    out = root / "Preview.swift"
    out.write_text(raw)
    return out

def build_app(root: Path, arm: str) -> Path:
    app_main = repo / "Mac/Sources/AppShell/PitexApp.swift"
    preview = repo / "Mac/Sources/Features/Preview.swift"
    app_copy = patch_app(root)
    preview_copy = patch_preview(root, arm)
    (root / "Check.swift").write_text(PERF_CHECK)
    if args.compile_only:
        # Save this arm's patched inputs + generated harness BEFORE compile,
        # so a failing arm's inputs are preserved (R6).
        shutil.copy2(root / "Check.swift", artifacts / f"Check-{arm}.swift")
        shutil.copy2(app_copy, artifacts / f"PitexApp-{arm}.swift")
        shutil.copy2(preview_copy, artifacts / f"Preview-{arm}.swift")
    sources = [p for p in (repo / "Mac/Sources").rglob("*.swift")
               if p not in (app_main, preview)]
    compile_cmd = [native_resolution["swiftc"], "-sdk", native_resolution["sdk"],
                   "-parse-as-library", "-swift-version", "6",
                   "-target", "arm64-apple-macos15.0", "-I", str(products),
                   str(root / "Check.swift"), str(app_copy), str(preview_copy),
                   *[str(p) for p in sources],
                   *[str(p) for p in products.glob("*.o")],
                   "-o", str(root / "check-bin")]
    _comp = subprocess.run(compile_cmd, env=native_environment,
                           capture_output=True, text=True)
    (artifacts / f"compile-{arm}.log").write_text(_comp.stdout + _comp.stderr)
    print(_comp.stdout + _comp.stderr, end="")
    if _comp.returncode != 0:
        sys.exit(f"FAIL: swiftc rc={_comp.returncode}; evidence: {artifacts}")
    _bad = MARKER.findall(_comp.stderr + _comp.stdout)
    if _bad:
        for line in _comp.stderr.splitlines() + _comp.stdout.splitlines():
            if MARKER.search(line):
                print(line)
        sys.exit("FAIL: harness compile defect — generated Check.swift has an "
                 "UnnecessaryEffectMarker warning (async query result discarded); "
                 "no functional verdict")

    if args.compile_only:
        # Inputs already saved above (pre-compile, R6); stop before
        # bundling / codesign / launch.
        return root / "check-bin"

    identifier = f"test.pitex.perf.{arm}.{os.getpid()}"
    app = root / f"PerfCheck-{arm}.app"
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    resources = contents / "Resources"
    resources.mkdir()
    plist = plistlib.loads((repo / "Mac/Config/Info.plist").read_bytes())
    plist.update({"CFBundleExecutable": "check", "CFBundleIdentifier": identifier,
                  "CFBundleName": "PerfCheck", "CFBundleDevelopmentRegion": "en",
                  "CFBundleShortVersionString": "0", "CFBundleVersion": "0",
                  "LSMinimumSystemVersion": "15.0"})
    (contents / "Info.plist").write_bytes(plistlib.dumps(plist))
    for locale in (repo / "Mac/Resources").glob("*.lproj"):
        shutil.copytree(locale, resources / locale.name)
    for name in ("markdown-preview.html", "PitexAgent", "pdfjs"):
        item = repo / "Mac/Resources" / name
        if item.is_dir():
            shutil.copytree(item, resources / name)
        elif item.exists():
            shutil.copy2(item, resources / name)
    verify_pdfjs_manifest(resources / "pdfjs")
    helpers = contents / "Helpers"
    helpers.mkdir()
    payload_src = payload / "PreviewEngine"
    for f in payload_src.rglob("*"):
        if not f.is_file():
            continue
        rel = f.relative_to(payload_src)
        if f.name.startswith("pitex-preview") or f.name.endswith(".dylib"):
            (helpers / "PreviewEngine" / rel.parent).mkdir(parents=True, exist_ok=True)
            shutil.copy2(f, helpers / "PreviewEngine" / rel)
        else:
            (resources / "PreviewEngine" / rel.parent).mkdir(parents=True, exist_ok=True)
            shutil.copy2(f, resources / "PreviewEngine" / rel)
    shutil.copy(root / "check-bin", contents / "MacOS/check")
    for lib in sorted((helpers / "PreviewEngine/lib").glob("*.dylib")):
        subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", str(lib)],
                       check=True, capture_output=True)
    for helper in sorted((helpers / "PreviewEngine").glob("pitex-preview*")):
        if helper.is_file():
            subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                            "--entitlements", str(repo / "Mac/Config/Pitex.entitlements"),
                            str(helper)], check=True, capture_output=True)
    for bundle in resources.glob("*.bundle"):
        subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", str(bundle)],
                       check=True, capture_output=True)
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-",
                    "--entitlements", str(repo / "Mac/Config/Pitex.entitlements"),
                    "--identifier", identifier, str(app)], check=True)
    return app

# --- process / RSS -----------------------------------------------------------
def ps_table():
    out = subprocess.run(["/bin/ps", "-axo", "pid=,ppid=,rss=,comm="],
                         capture_output=True, text=True, timeout=10).stdout
    rows = {}
    for line in out.splitlines():
        f = line.split(None, 3)
        if len(f) == 4:
            try:
                rows[int(f[0])] = (int(f[1]), int(f[2]), f[3])
            except ValueError:
                pass
    return rows

def webkit_service_pids(rows):
    """WebKit XPC services are launchd-spawned (ppid 1), never app
    children — tracked by name, snapshotted pre-launch (R1)."""
    return {pid for pid, (_, _, c) in rows.items()
            if "com.apple.WebKit.WebContent" in c
            or "com.apple.WebKit.Networking" in c
            or "com.apple.WebKit.GPU" in c}

def descendants(app_pid: int, rows):
    owned = {app_pid}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _, _) in rows.items():
            if ppid in owned and pid not in owned:
                owned.add(pid); changed = True
    return owned

def run_once(app: Path, arm: str, run_dir: Path):
    run_dir.mkdir(parents=True, exist_ok=True)
    fixture = run_dir / "fixture"
    fixture.mkdir()
    (fixture / "main.tex").write_text(fixture_main)
    (fixture / "big.tex").write_text(fixture_big)
    home = run_dir / "home"; home.mkdir()
    out_log = run_dir / "check.out.log"
    err_log = run_dir / "check.err.log"
    binary = (app / "Contents/MacOS/check").resolve()

    # R1/P8: pre-launch WebContent snapshot. Gated set = app pid + WebContent
    # pids new since this snapshot — launchd-spawned services, NOT app
    # children. GPU/Networking recorded separately, never gated.
    pre_wc = {p for p, (_, _, c) in ps_table().items()
              if "com.apple.WebKit.WebContent" in c}
    # GPU/Networking likewise scoped new-since-launch (recorded, not gated).
    pre_other = {p for p, (_, _, c) in ps_table().items()
                 if ("com.apple.WebKit.GPU" in c
                     or "com.apple.WebKit.Networking" in c)}

    # H2: marks use clock_gettime_nsec_np(CLOCK_UPTIME_RAW) in Swift (mach
    # absolute). Share the SAME uptime clock for launch/exit/samples so T6 +
    # sample_at share the epoch. No silent fallback (K2).
    _clk = time.CLOCK_UPTIME_RAW
    def _uptime(): return time.clock_gettime(_clk)
    launch_mono = _uptime()
    launcher = subprocess.Popen(
        ["/usr/bin/open", "-W",
         "--stdout", str(out_log), "--stderr", str(err_log),
         "--env", "PI_AGENT_PATH=/usr/bin/false",
         "--env", f"HOME={home}",
         "--env", "PATH=/Library/TeX/texbin:/usr/bin:/bin:/usr/sbin:/sbin",
         "--env", f"PITEX_CHECK_FIXTURE={fixture}",
         "--env", f"PITEX_PERF_ARM={arm}",
         str(app)])

    samples = []          # (mono, gated_rss_kb, wc_pids, other_rss_kb)
    app_pid = {"pid": None}
    stop = threading.Event()

    def sampler():
        while not stop.is_set():
            rows = ps_table()
            if app_pid["pid"] is None:
                for pid, (_, _, c) in rows.items():
                    try:
                        if Path(c).resolve() == binary:
                            app_pid["pid"] = pid; break
                    except OSError:
                        continue
            if app_pid["pid"] is not None:
                wc_new = {p for p, (_, _, c) in rows.items()
                          if "com.apple.WebKit.WebContent" in c} - pre_wc
                other = ({p for p, (_, _, c) in rows.items()
                          if ("com.apple.WebKit.GPU" in c
                              or "com.apple.WebKit.Networking" in c)}
                         - pre_other)
                gated = {app_pid["pid"]} | wc_new
                rss = sum(rows[p][1] for p in gated if p in rows)
                other_rss = sum(rows[p][1] for p in other if p in rows)
                samples.append((_uptime(), rss,
                                frozenset(wc_new), other_rss))
            time.sleep(0.25)

    t = threading.Thread(target=sampler, daemon=True)
    t.start()
    deadline = time.monotonic() + 900
    forced = False
    try:
        while launcher.poll() is None:
            if time.monotonic() >= deadline:
                forced = True
                print(f"FAIL[{arm}]: perf watchdog", flush=True)
                break
            time.sleep(0.2)
    finally:
        stop.set(); t.join(timeout=5)
        exit_mono = _uptime()
        output = out_log.read_text(errors="replace") if out_log.exists() else ""
        (run_dir / "stdout.txt").write_text(output)
        # P4/W1: kill ONLY on a forced timeout. A normal exit gets a ≤10 s
        # reap window; survivors are recorded as leaked BEFORE any cleanup.
        leaked_after_quit = []
        if forced:
            rows = ps_table()
            owned = ({app_pid["pid"]} if app_pid["pid"] else set()) | \
                    ({p for p, (_, _, c) in rows.items()
                      if "com.apple.WebKit.WebContent" in c} - pre_wc)
            for pid in owned:
                try: os.kill(pid, 15)
                except (ProcessLookupError, PermissionError): pass
            time.sleep(1)
            rows = ps_table()
            for pid in owned & set(rows):
                try: os.kill(pid, 9)
                except (ProcessLookupError, PermissionError): pass
        else:
            # wait ≤10 s for the owned set to reap, then record survivors.
            for _ in range(40):
                rows = ps_table()
                ever = set()
                for _, _, wc, _ in samples:
                    ever |= wc
                if app_pid["pid"]:
                    ever.add(app_pid["pid"])
                leaked_after_quit = sorted(p for p in ever if p in rows)
                if not leaked_after_quit:
                    break
                time.sleep(0.25)
            # Recorded; now reap them so survivors never pile up on the VM.
            for pid in leaked_after_quit:
                try: os.kill(pid, 15)
                except (ProcessLookupError, PermissionError): pass
            time.sleep(1)
            rows = ps_table()
            for pid in [p for p in leaked_after_quit if p in rows]:
                try: os.kill(pid, 9)
                except (ProcessLookupError, PermissionError): pass
        if launcher.poll() is None:
            launcher.terminate(); launcher.wait(timeout=10)

    with open(run_dir / "rss.csv", "w") as fh:
        for ts, rss, wc, other in samples:
            fh.write(f"{ts:.3f},{rss},{';'.join(str(p) for p in sorted(wc))},{other}\n")

    marks = {}
    hooks = set()
    for line in output.splitlines():
        m = re.match(r'\[perf\] ({"event":"(mark|hook)".*})', line)
        if m:
            j = json.loads(m.group(1))
            if j["event"] == "mark":
                marks[j["name"]] = j["t"]
            else:
                hooks.add(j["name"])
    result = None
    for line in output.splitlines():
        if line.startswith("[perf-result] "):
            result = json.loads(line[len("[perf-result] "):])
    if result is not None:
        result["hooks"] = sorted(set(result.get("hooks", [])) | hooks)

    ok = (result is not None and "PERF_COMPLETE" in output
          and not forced and not leaked_after_quit)
    if marks:
        _f, _l = min(marks.values()), max(marks.values())
        print(f"[diag] {arm} clock-align first_mark-launch={_f-launch_mono:+.3f}s "
              f"exit-last_mark={exit_mono-_l:+.3f}s", flush=True)
    if not ok:
        print(f"[{arm}] run failed (forced={forced}) — evidence {run_dir}",
              flush=True)
    return {"ok": ok, "forced": forced, "result": result,
            "samples": samples, "marks": marks,
            "leaked_after_quit": leaked_after_quit,
            "launch_mono": launch_mono, "exit_mono": exit_mono}


def main() -> int:
    arms = [a.strip() for a in args.arms.split(",") if a.strip()]
    if args.compile_only:
        # Diagnostic mode: attempt BOTH arms even if one fails (a candidate
        # error must not hide a control error), then stop before any launch.
        # No app launch, no run, no native-verification lock (the runner owns it).
        _rc = 0
        for arm in arms:
            root = Path(tempfile.mkdtemp(prefix=f"perf-{arm}-"))
            try:
                build_app(root, arm)
                print(f"[diag] {arm} compile OK -> {root}/check-bin")
            except SystemExit as e:
                _rc = e.code if isinstance(e.code, int) else 1
                print(f"[diag] {arm} compile FAILED (rc={e.code}): {e}", file=sys.stderr)
            except AssertionError as e:
                _rc = 1
                print(f"[diag] {arm} patch/anchor FAILED: {e}", file=sys.stderr)
            except Exception as e:
                _rc = 1
                print(f"[diag] {arm} UNEXPECTED FAILED: {e!r}", file=sys.stderr)
        return 0 if _rc == 0 else 1

    lock_path = Path.home() / "pitex-verify-scratch/native-verification.lock"
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock_fd = open(lock_path, "a")
    print("[diag] waiting for native-verification lock")
    fcntl.flock(lock_fd, fcntl.LOCK_EX)
    print("[diag] lock acquired")

    builds = {}
    for arm in arms:
        root = Path(tempfile.mkdtemp(prefix=f"perf-{arm}-"))
        builds[arm] = build_app(root, arm)
        print(f"[diag] {arm} app: {builds[arm]}")

    runs = {a: [] for a in arms}
    # G3: one discarded warm-up RUN per arm, then strict ABAB. Warm-up
    # outcomes are recorded — a failed warm-up still precedes real runs
    # (it exercises the code path; it does not gate).
    warmups = {}
    _smoke_runs = {}   # K3: full run objects live here, NOT in warmups
    for arm in arms:
        print(f"[warmup] {arm}", flush=True)
        w = run_once(builds[arm], arm, artifacts / f"warmup-{arm}")
        warmups[arm] = {"ok": w["ok"], "forced": w["forced"]}
        _smoke_runs[arm] = w

    if args.warmup_only:
        # S1: judge each warm-up's VALIDITY with the UNCHANGED compute_verdict
        # (marks/hooks/unresolved/leaks) — no numeric perf, no runs. Both arms
        # must pass for exit 0.
        _smoke_ok = True
        for arm in arms:
            w = _smoke_runs[arm]
            _other = "candidate" if arm == "control" else "control"
            v = compute_verdict({arm: [w], _other: []}, 1)
            inv = [e for e in v["invalid_runs"] if e["arm"] == arm]
            ok = (w["ok"] and not w["forced"] and not w.get("leaked_after_quit")
                  and not inv)
            _smoke_ok = _smoke_ok and ok
            print(f"[smoke] {arm} valid={ok} ok={w['ok']} forced={w['forced']} "
                  f"leaks={w.get('leaked_after_quit')} invalid={inv}", flush=True)
        print("SMOKE_" + ("PASS" if _smoke_ok else "FAIL"), flush=True)
        return 0 if _smoke_ok else 1

    order = []
    for i in range(args.runs):
        order += ["candidate", "control"] if len(arms) == 2 else arms
    for i, arm in enumerate(order):
        print(f"[run] {arm} #{i}", flush=True)
        runs[arm].append(run_once(builds[arm], arm,
                                  artifacts / f"run-{i:02d}-{arm}"))

    verdict = compute_verdict(runs, args.runs)
    verdict["warmups"] = warmups
    (artifacts / "perf-verdict.json").write_text(
        json.dumps(verdict, indent=2, default=str))
    print(json.dumps(verdict["gates"], indent=2))
    ok = all(verdict["gates"].values()) and len(verdict["gates"]) == 7
    print("PERF_" + ("PASS" if ok else "FAIL"))
    return 0 if ok else 1

if __name__ == "__main__":
    sys.exit(main())
