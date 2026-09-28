#!/usr/bin/env python3
r"""check-native-completion.py <Build/Products/Release>

Hosts the real WorkspaceModel + WorkspaceView editor in a real window and
verifies the native completion popup is PASSIVE: an automatic 200ms debounce
trigger may show candidates, but nothing enters the text until the user picks
one. Covers the reported bug (\in auto-becoming \include with the suffix
selected) plus backspace/retype, an unknown prefix, Down+Escape restore and
Down+Return acceptance. The completionSource wrapper counts real candidate
queries; a method swizzle records every NSTextView.complete engagement.

Usage: check-native-completion.py <Build/Products/Release>
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
import EditorMacAdapter
import SwiftUI

/// Counts every real NSTextView.complete(_:) invocation — proof the popup
/// machinery actually engaged, not just that the data source was probed.
@MainActor enum CompleteRecorder {
    static var original: IMP?
    static var calls = 0
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
        func require(_ condition: Bool, _ message: String) {
            guard condition else { print("FAIL", message); fflush(nil); exit(1) }
        }
        func stage(_ message: String) { print("[stage]", message); fflush(nil) }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        func until(_ timeout: Double = 20, _ condition: @MainActor () -> Bool) async -> Bool {
            let deadline = ContinuousClock.now + .seconds(timeout)
            while ContinuousClock.now < deadline {
                if condition() { return true }
                try? await Task.sleep(for: .milliseconds(100))
            }
            return condition()
        }

        let root = ProcessInfo.processInfo.environment["PITEX_COMPLETION_FIXTURE"] ?? ""
        let workspace = WorkspaceModel()
        await workspace.open(URL(fileURLWithPath: root + "/main.tex").standardizedFileURL)
        require(await until { if case .ready = workspace.phase { return true }; return false },
                "Fixture project must reach .ready")
        let host = NSHostingView(rootView: WorkspaceView(workspace: workspace))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 900, height: 600),
                              styleMask: [.titled, .resizable], backing: .buffered, defer: false)
        window.contentView = host
        window.makeKeyAndOrderFront(nil)
        NSApp.activate()
        defer { window.orderOut(nil) }

        func editorParts() async throws -> (NSScrollView, NSTextView) {
            for _ in 0..<50 {
                host.layoutSubtreeIfNeeded()
                if let editor = workspace.environment?.editor,
                   let scroll = descendants(host).compactMap({ $0 as? NSScrollView })
                       .first(where: { $0.documentView === editor.textView }) {
                    return (scroll, editor.textView)
                }
                try await Task.sleep(for: .milliseconds(100))
            }
            print("FAIL editor never mounted: env=\(workspace.environment == nil) "
                + "scrolls=\(descendants(host).compactMap { $0 as? NSScrollView }.count)")
            fflush(nil)
            exit(1)
        }
        let (_, textView) = try await editorParts()
        let editor = workspace.environment!.editor
        require(window.makeFirstResponder(textView), "Editor must become first responder")
        // Caret at end of document on a fresh line — each stage's context
        // scan then starts clean.
        textView.setSelectedRange(NSRange(location: (textView.string as NSString).length, length: 0))

        // Record every candidate query on the REAL provider (attached by
        // activateDocument) without replacing it: nil = no context, [] =
        // in-context but no matches, [x…] = offered candidates.
        var sourceQueries = 0
        var lastCandidates: [String]? = nil
        let innerSource = editor.completionSource
        require(innerSource != nil, "activateDocument must attach the real completion provider")
        editor.completionSource = { text, caret in
            sourceQueries += 1
            let result = innerSource?(text, caret)
            lastCandidates = result?.candidates
            return result
        }
        // Record every popup engagement on the shared NSTextView class.
        let method = class_getInstanceMethod(NSTextView.self, #selector(NSTextView.complete(_:)))!
        typealias CompleteIMP = @convention(c) (NSTextView, Selector, Any?) -> Void
        let record: CompleteIMP = { view, selector, sender in
            MainActor.assumeIsolated {
                CompleteRecorder.calls += 1
                unsafeBitCast(CompleteRecorder.original!, to: CompleteIMP.self)(view, selector, sender)
            }
        }
        CompleteRecorder.original = method_setImplementation(method, unsafeBitCast(record, to: IMP.self))
        defer { method_setImplementation(method, CompleteRecorder.original!) }

        func settle(_ ms: Int = 400) async throws {
            try await Task.sleep(for: .milliseconds(ms))
        }
        func tail() -> String { String(textView.string.suffix(24)) }
        func literal(_ expected: String, _ label: String) {
            require(textView.string.hasSuffix(expected),
                    "\(label): text must stay literal '\(expected)', got tail '\(tail())'")
            require(textView.selectedRange().length == 0,
                    "\(label): no provisional suffix may be selected, selection=\(textView.selectedRange())")
            require(!textView.hasMarkedText(),
                    "\(label): no marked text may linger")
        }
        /// Per-character native input — each character drives didChangeText
        /// and re-arms the real 200ms debounce like typing does.
        func type(_ text: String) {
            for char in text {
                textView.insertText(String(char),
                                    replacementRange: NSRange(location: NSNotFound, length: 0))
            }
        }
        func key(_ code: UInt16, chars: String = "") {
            let event = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [],
                timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                context: nil, characters: chars, charactersIgnoringModifiers: chars,
                isARepeat: false, keyCode: code)!
            NSApp.sendEvent(event)
        }
        let backspace = (code: UInt16(51), chars: "\u{7F}"), escape = (code: UInt16(53), chars: "\u{1B}")
        let down = (code: UInt16(125), chars: "\u{F701}"), ret = (code: UInt16(36), chars: "\r")

        // Fresh line so the context scan for each stage starts clean.
        type("\n")

        // A: bare backslash — popup engages, nothing is inserted.
        CompleteRecorder.calls = 0
        type("\\")
        try await settle()
        require(CompleteRecorder.calls >= 1, "automatic debounce must engage native complete")
        require(sourceQueries > 0, "the real completion source must be queried")
        require(lastCandidates?.contains("\\include") == true,
                "\\ prefix must offer builtin commands, got \(String(describing: lastCandidates))")
        literal("\\", "backslash")
        stage("backslash: popup engaged, text stayed literal")

        // B: continue 'in' — two candidates, still literal.
        type("in")
        try await settle()
        require(lastCandidates?.contains("\\include") == true
                && lastCandidates?.contains("\\input") == true,
                "\\in must offer include+input, got \(String(describing: lastCandidates))")
        literal("\\in", "in-prefix")
        stage("\\in stayed literal with multiple candidates")

        // C: backspace back to '\' (popup may re-engage — text only), then
        // 'x' reaches a zero-match prefix: no popup, no dictionary words.
        key(backspace.code, chars: backspace.chars)
        key(backspace.code, chars: backspace.chars)
        try await settle()
        literal("\\", "after backspacing 'in'")
        let callsBeforeUnknown = CompleteRecorder.calls
        type("x")
        try await settle()
        literal("\\x", "unknown prefix")
        require(lastCandidates == [],
                "\\x is a valid context with zero matches — must not see English words, "
                + "got \(String(describing: lastCandidates))")
        require(CompleteRecorder.calls == callsBeforeUnknown,
                "zero candidates must not open the popup")
        stage("\\x unknown prefix: no popup, no dictionary words")

        // D: single-candidate prefix also stays literal.
        key(backspace.code, chars: backspace.chars)
        type("doc")
        try await settle()
        require(lastCandidates == ["\\documentclass"],
                "\\doc must offer only \\documentclass, got \(String(describing: lastCandidates))")
        literal("\\doc", "single-candidate prefix")
        stage("\\doc single candidate stayed literal")

        // E: Down selects the candidate, Escape restores the original
        // prefix+caret; then wait past the debounce — no reinsertion.
        require(CompleteRecorder.calls >= callsBeforeUnknown + 1, "popup must be open for \\doc")
        key(down.code, chars: down.chars)
        key(escape.code, chars: escape.chars)
        try await settle(300)
        literal("\\doc", "Down+Escape must restore the original prefix")
        try await settle(300)
        literal("\\doc", "post-Escape debounce must not reinsert")
        stage("Down+Escape restored \\doc; no reinsertion after debounce")

        // F: positive opt-in — reopen via the public manual-complete API,
        // then a real Down+Return accepts the candidate.
        editor.requestCompletion()
        try await settle(200)
        key(down.code, chars: down.chars)
        key(ret.code, chars: ret.chars)
        try await settle()
        literal("\\documentclass", "Down+Return must accept")
        require(textView.selectedRange().location == (textView.string as NSString).length,
                "caret must sit after the accepted completion")
        stage("Down+Return accepted \\documentclass")

        // The project provider must have loaded the fixture's labels and
        // .bib keys before the group stages — probe it directly rather
        // than racing the refresh cycle.
        let probeRef = "\\ref{se", probeCite = "\\cite{kn"
        require(await until {
            innerSource?(probeRef, probeRef.utf16.count)?.candidates.contains("sec:intro") == true
                && innerSource?(probeCite, probeCite.utf16.count)?.candidates.contains("knuth84") == true
        }, "provider must load project labels and .bib keys")

        // G: \ref{ — typed prefix stays literal with real label candidates.
        type("\n")
        type("\\ref{se")
        try await settle()
        require(lastCandidates?.contains("sec:intro") == true
                && lastCandidates?.contains("sec:end") == true,
                "\\ref{se must offer both labels, got \(String(describing: lastCandidates))")
        literal("\\ref{se", "ref prefix")
        // Backspace + an unknown custom key stays literal, no word leak.
        key(backspace.code, chars: backspace.chars)
        type("zz")
        try await settle()
        literal("\\ref{szz", "custom ref key")
        require(lastCandidates == [],
                "zero-match ref key must not see dictionary words, "
                + "got \(String(describing: lastCandidates))")
        // Explicit choice of a unique label still works.
        type("\n")
        type("\\ref{sec:i")
        try await settle()
        require(lastCandidates == ["sec:intro"],
                "\\ref{sec:i must offer only sec:intro, got \(String(describing: lastCandidates))")
        literal("\\ref{sec:i", "unique ref prefix")
        editor.requestCompletion()
        try await settle(200)
        key(down.code, chars: down.chars)
        key(ret.code, chars: ret.chars)
        try await settle()
        require(textView.string.hasSuffix("\\ref{sec:intro"),
                "Down+Return must accept sec:intro, tail '\(tail())'")
        stage("\\ref literal, custom key kept, sec:intro accepted")

        // H: cite — the same delete/refuse freedom as commands: \cite{kn
        // stays passive, then backspace + a custom key is retained with
        // no candidates and no word leak.
        type("\n")
        type("\\cite{kn")
        try await settle()
        require(lastCandidates == ["knuth84"],
                "\\cite{kn must offer knuth84, got \(String(describing: lastCandidates))")
        literal("\\cite{kn", "cite prefix")
        key(backspace.code, chars: backspace.chars)
        type("zz")
        try await settle()
        literal("\\cite{kzz", "custom cite key")
        require(lastCandidates == [],
                "zero-match cite key must not see dictionary words, "
                + "got \(String(describing: lastCandidates))")
        // Unicode before the cite — a surrogate pair shifts UTF-16
        // offsets; only the current key token may be replaced.
        type("\n")
        type("😀\\cite{kn")
        try await settle()
        require(lastCandidates == ["knuth84"],
                "\\cite{kn must offer knuth84, got \(String(describing: lastCandidates))")
        literal("😀\\cite{kn", "cite after Unicode")
        editor.requestCompletion()
        try await settle(200)
        key(down.code, chars: down.chars)
        key(ret.code, chars: ret.chars)
        try await settle()
        literal("😀\\cite{knuth84", "cite accept after Unicode")
        // Comma-separated second key: the range covers only the fragment.
        type(",l")
        try await settle()
        require(lastCandidates == ["lamport94"],
                "second cite key must offer lamport94, got \(String(describing: lastCandidates))")
        literal("😀\\cite{knuth84,l", "multi-cite prefix")
        editor.requestCompletion()
        try await settle(200)
        key(down.code, chars: down.chars)
        key(ret.code, chars: ret.chars)
        try await settle()
        literal("😀\\cite{knuth84,lamport94", "multi-cite accept")
        stage("cite: Unicode offsets correct, second key replaced only its token")

        // I: IME marked text must not engage the popup at all.
        type("\n")
        type("\\i")
        try await settle()
        key(escape.code, chars: escape.chars)
        try await settle(300)
        let callsBeforeIME = CompleteRecorder.calls
        textView.setMarkedText("か", selectedRange: NSRange(location: 0, length: 0),
                               replacementRange: NSRange(location: NSNotFound, length: 0))
        try await settle()
        require(textView.hasMarkedText(), "marked text must be active")
        require(CompleteRecorder.calls == callsBeforeIME,
                "no completion popup may engage while IME text is marked")
        textView.unmarkText()
        try await settle(200)
        require(!textView.hasMarkedText() && textView.string.hasSuffix("\\iか"),
                "unmark must commit the literal text, tail '\(tail())'")
        stage("IME marked text suppressed completion; commit stayed literal")

        // Session text must equal what is on screen — the mutation stream
        // saw the user's literal typing plus the one accepted completion.
        // The session submit is async: poll until it lands.
        require(await until { workspace.documentSnapshot?.text == textView.string },
                "session text must match the editor after completion stages")
        print("PASS native completion: passive popup, literal typing, Esc restore, real accept")
    }
}
'''
with tempfile.TemporaryDirectory(prefix="pitex-completion-", dir="/tmp") as directory:
    root = Path(directory)
    fixture = root / "fixture"
    fixture.mkdir()
    (fixture / "main.tex").write_text(
        "\\documentclass{article}\n"
        "\\bibliography{refs}\n"
        "\\begin{document}\n"
        "\\section{Intro}\\label{sec:intro}\n"
        "\\section{End}\\label{sec:end}\n"
        "Body text.\n"
        "\\end{document}\n")
    (fixture / "refs.bib").write_text(
        "@book{knuth84, title={T}, author={Knuth}, year={1984}}\n"
        "@article{lamport94, title={L}, author={Lamport}, year={1994}}\n")
    bundle = root / "CompletionCheck.app/Contents"
    (bundle / "MacOS").mkdir(parents=True)
    resources = bundle / "Resources"
    resources.mkdir()
    (bundle / "Info.plist").write_bytes(plistlib.dumps({
        "CFBundleExecutable": "check", "CFBundleIdentifier": "test.pitex.completion",
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
    # Launch through LaunchServices like the other native checkers: direct
    # exec() can leave the process unable to activate. `open -W` waits but
    # swallows the exit status — CHECK_COMPLETE is authoritative.
    out_log, err_log = root / "check.out.log", root / "check.err.log"
    app = root / "CompletionCheck.app"
    try:
        subprocess.run(["/usr/bin/open", "-n", "-W",
                        "--stdout", str(out_log), "--stderr", str(err_log),
                        "--env", "PI_AGENT_PATH=/usr/bin/false",
                        "--env", f"PI_CODING_AGENT_DIR={root / 'pi'}",
                        "--env", f"PITEX_COMPLETION_FIXTURE={fixture}",
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
