#!/usr/bin/env python3
r"""check-native-completion.py <Build/Products/Release>

Hosts the real WorkspaceModel + WorkspaceView editor in a real window and
verifies the native completion popup is PASSIVE: an automatic 200ms debounce
trigger may show candidates, but nothing enters the text until the user picks
one. Covers the reported bug (\in auto-becoming \include with the suffix
selected) plus backspace/retype, unknown prefixes, Down+Escape restore,
Down+Return acceptance, ref/cite keys, a Unicode prefix, comma-separated
cite tokens and IME marked text.

Input goes through real queued keyDown events (NSApp.postEvent) and the
stage sequence is a chain of one-shot Timers registered in .common AND
.eventTracking — each fire arms the successor BEFORE running its action, so
later steps still deliver on time if an action enters a nested
event-tracking loop. The
completionSource wrapper counts real candidate queries; a method swizzle
records every NSTextView.complete engagement with enter/return text.

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

/// Runs the stage list as chained one-shot timers. Each step's timer fires
/// in .common and .eventTracking modes; on fire the NEXT step is armed
/// before the current action runs, so if an action enters a nested
/// event-tracking loop the following steps still deliver on time.
@MainActor final class CompletionDriver {
    var steps: [(delay: TimeInterval, action: @MainActor () -> Void)] = []
    private var cursor = 0
    var done = false
    func scheduleNext() {
        guard cursor < steps.count else { return }
        let index = cursor
        cursor += 1
        let timer = Timer(timeInterval: steps[index].delay, repeats: false) { _ in
            MainActor.assumeIsolated {
                self.scheduleNext()
                self.steps[index].action()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        RunLoop.main.add(timer, forMode: .eventTracking)
    }
    /// Arms an ad-hoc step outside the linear chain (bounded retries).
    func scheduleAfter(_ delay: TimeInterval, _ action: @escaping @MainActor () -> Void) {
        let timer = Timer(timeInterval: delay, repeats: false) { _ in
            MainActor.assumeIsolated { action() }
        }
        RunLoop.main.add(timer, forMode: .common)
        RunLoop.main.add(timer, forMode: .eventTracking)
    }
}

@main struct Check {
    @MainActor static func main() {
        // A bundled launch through LaunchServices gives the checker a
        // regular activation policy and a real event loop — direct exec()
        // can leave the process unable to activate.
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            do { try await run(); exit(0) }
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
        stage("opening fixture project")
        let workspace = WorkspaceModel()
        await workspace.open(URL(fileURLWithPath: root + "/main.tex").standardizedFileURL)
        require(await until { if case .ready = workspace.phase { return true }; return false },
                "Fixture project must reach .ready")
        stage("project ready; mounting workspace view")
        let host = NSHostingView(rootView: WorkspaceView(workspace: workspace))
        // 1400x850 — the proven window geometry the navigation checker uses.
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1400, height: 850),
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
        stage("editor mounted")
        let editor = workspace.environment!.editor
        require(window.makeFirstResponder(textView), "Editor must become first responder")
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
                print("[trace] complete: enter #\(CompleteRecorder.calls) "
                    + "tail='\(String(view.string.suffix(12)))'")
                fflush(nil)
                unsafeBitCast(CompleteRecorder.original!, to: CompleteIMP.self)(view, selector, sender)
                print("[trace] complete: return tail='\(String(view.string.suffix(12)))'")
                fflush(nil)
            }
        }
        CompleteRecorder.original = method_setImplementation(method, unsafeBitCast(record, to: IMP.self))
        defer { method_setImplementation(method, CompleteRecorder.original!) }

        // The project provider must have loaded the fixture's labels and
        // .bib keys before the chain starts — probe it directly rather
        // than racing the refresh cycle.
        let probeRef = "\\ref{se", probeCite = "\\cite{kn"
        require(await until {
            innerSource?(probeRef, probeRef.utf16.count)?.candidates.contains("sec:intro") == true
                && innerSource?(probeCite, probeCite.utf16.count)?.candidates.contains("knuth84") == true
        }, "provider must load project labels and .bib keys")
        stage("provider ready; starting timer driver")

        func tail() -> String { String(textView.string.suffix(24)) }
        func literal(_ expected: String, _ label: String) {
            require(textView.string.hasSuffix(expected),
                    "\(label): text must stay literal '\(expected)', got tail '\(tail())'")
            require(textView.selectedRange().length == 0,
                    "\(label): no provisional suffix may be selected, selection=\(textView.selectedRange())")
            require(!textView.hasMarkedText(),
                    "\(label): no marked text may linger")
        }
        /// Queued keyDown — the completion popup's nested tracking loop
        /// consumes posted events; a direct sendEvent would bypass it.
        func key(_ code: UInt16, chars: String = "", mods: NSEvent.ModifierFlags = []) {
            let event = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: mods,
                timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                context: nil, characters: chars, charactersIgnoringModifiers: chars,
                isARepeat: false, keyCode: code)!
            NSApp.postEvent(event, atStart: false)
        }
        /// ANSI key codes for the printable characters the stages type.
        /// Non-keyboard text (emoji, IME) uses insertText/setMarkedText.
        func keyCode(for char: Character) -> (code: UInt16, mods: NSEvent.ModifierFlags)? {
            switch char {
            case "\\": return (42, [])
            case "{": return (33, [.shift])
            case ",": return (43, [])
            case ":": return (41, [.shift])
            case "a": return (0, [])
            case "c": return (8, [])
            case "d": return (2, [])
            case "e": return (14, [])
            case "f": return (3, [])
            case "i": return (34, [])
            case "k": return (40, [])
            case "l": return (37, [])
            case "n": return (45, [])
            case "o": return (31, [])
            case "r": return (15, [])
            case "s": return (1, [])
            case "t": return (17, [])
            case "x": return (7, [])
            case "z": return (6, [])
            default: return nil
            }
        }
        /// Real queued keyDown per character — each one drives didChangeText
        /// and re-arms the 200ms debounce like typing; a newline is a
        /// Return key event.
        func type(_ text: String) {
            for char in text {
                if char == "\n" { key(36, chars: "\r"); continue }
                guard let mapped = keyCode(for: char) else {
                    print("FAIL unmapped character \(char)"); fflush(nil); exit(1)
                }
                key(mapped.code, chars: String(char), mods: mapped.mods)
            }
        }
        let backspace = (code: UInt16(51), chars: "\u{7F}"), escape = (code: UInt16(53), chars: "\u{1B}")
        let down = (code: UInt16(125), chars: "\u{F701}"), ret = (code: UInt16(36), chars: "\r")

        var callsBeforeUnknown = 0
        var callsBeforeIME = 0
        var sessionPolls = 0
        let driver = CompletionDriver()
        func pollSession() {
            if workspace.documentSnapshot?.text == textView.string {
                driver.done = true
                print("PASS native completion: passive popup, literal typing, Esc restore, real accept")
                print("CHECK_COMPLETE")
                fflush(nil)
                return
            }
            sessionPolls += 1
            require(sessionPolls <= 60,
                    "session text must match the editor after completion stages")
            driver.scheduleAfter(0.1) { pollSession() }
        }

        driver.steps = [
            // A: bare backslash — popup engages, nothing is inserted.
            (0.1, { type("\\") }),
            (0.4, {
                require(CompleteRecorder.calls >= 1,
                        "automatic debounce must engage native complete")
                require(sourceQueries > 0, "the real completion source must be queried")
                require(lastCandidates?.contains("\\include") == true,
                        "\\ prefix must offer builtin commands, got \(String(describing: lastCandidates))")
                literal("\\", "backslash")
                stage("backslash: popup engaged, text stayed literal")
            }),
            // B: continue 'in' — two candidates, still literal.
            (0.05, { type("in") }),
            (0.4, {
                require(lastCandidates?.contains("\\include") == true
                        && lastCandidates?.contains("\\input") == true,
                        "\\in must offer include+input, got \(String(describing: lastCandidates))")
                literal("\\in", "in-prefix")
                stage("\\in stayed literal with multiple candidates")
            }),
            // C: backspace to '\', then 'x' — a zero-match prefix opens no
            // popup and never sees dictionary words.
            (0.05, { key(backspace.code, chars: backspace.chars)
                     key(backspace.code, chars: backspace.chars) }),
            (0.4, { literal("\\", "after backspacing 'in'") }),
            (0.05, { callsBeforeUnknown = CompleteRecorder.calls; type("x") }),
            (0.4, {
                literal("\\x", "unknown prefix")
                require(lastCandidates == [],
                        "\\x is a valid context with zero matches — must not see English words, "
                        + "got \(String(describing: lastCandidates))")
                require(CompleteRecorder.calls == callsBeforeUnknown,
                        "zero candidates must not open the popup")
                stage("\\x unknown prefix: no popup, no dictionary words")
            }),
            // D: single-candidate prefix also stays literal.
            (0.05, { key(backspace.code, chars: backspace.chars); type("doc") }),
            (0.4, {
                require(lastCandidates == ["\\documentclass"],
                        "\\doc must offer only \\documentclass, got \(String(describing: lastCandidates))")
                literal("\\doc", "single-candidate prefix")
                stage("\\doc single candidate stayed literal")
            }),
            // E: Down selects the candidate, Escape restores the original
            // prefix+caret; then wait past the debounce — no reinsertion.
            (0.05, {
                require(CompleteRecorder.calls >= callsBeforeUnknown + 1,
                        "popup must be open for \\doc")
                key(down.code, chars: down.chars)
            }),
            (0.15, { key(escape.code, chars: escape.chars) }),
            (0.3, { literal("\\doc", "Down+Escape must restore the original prefix") }),
            (0.3, {
                literal("\\doc", "post-Escape debounce must not reinsert")
                stage("Down+Escape restored \\doc; no reinsertion after debounce")
            }),
            // F: positive opt-in — reopen via the public manual-complete
            // API, then queued Down+Return accepts the candidate.
            (0.05, { editor.requestCompletion() }),
            (0.2, { key(down.code, chars: down.chars) }),
            (0.1, { key(ret.code, chars: ret.chars) }),
            (0.4, {
                literal("\\documentclass", "Down+Return must accept")
                require(textView.selectedRange().location == (textView.string as NSString).length,
                        "caret must sit after the accepted completion")
                stage("Down+Return accepted \\documentclass")
            }),
            // G: \ref{ — typed prefix stays literal with real label
            // candidates; a custom key is retained; unique key accepted.
            (0.05, { key(escape.code, chars: escape.chars); type("\n") }),
            (0.1, { type("\\ref{se") }),
            (0.4, {
                require(lastCandidates?.contains("sec:intro") == true
                        && lastCandidates?.contains("sec:end") == true,
                        "\\ref{se must offer both labels, got \(String(describing: lastCandidates))")
                literal("\\ref{se", "ref prefix")
            }),
            (0.05, { key(backspace.code, chars: backspace.chars); type("zz") }),
            (0.4, {
                literal("\\ref{szz", "custom ref key")
                require(lastCandidates == [],
                        "zero-match ref key must not see dictionary words, "
                        + "got \(String(describing: lastCandidates))")
            }),
            (0.05, { key(escape.code, chars: escape.chars); type("\n") }),
            (0.1, { type("\\ref{sec:i") }),
            (0.4, {
                require(lastCandidates == ["sec:intro"],
                        "\\ref{sec:i must offer only sec:intro, got \(String(describing: lastCandidates))")
                literal("\\ref{sec:i", "unique ref prefix")
                editor.requestCompletion()
            }),
            (0.2, { key(down.code, chars: down.chars) }),
            (0.1, { key(ret.code, chars: ret.chars) }),
            (0.4, {
                require(textView.string.hasSuffix("\\ref{sec:intro"),
                        "Down+Return must accept sec:intro, tail '\(tail())'")
                stage("\\ref literal, custom key kept, sec:intro accepted")
            }),
            // H: cite — same delete/refuse freedom: \cite{kn stays passive,
            // backspace + a custom key is retained, no word leak.
            (0.05, { key(escape.code, chars: escape.chars); type("\n") }),
            (0.1, { type("\\cite{kn") }),
            (0.4, {
                require(lastCandidates == ["knuth84"],
                        "\\cite{kn must offer knuth84, got \(String(describing: lastCandidates))")
                literal("\\cite{kn", "cite prefix")
            }),
            (0.05, { key(backspace.code, chars: backspace.chars); type("zz") }),
            (0.4, {
                literal("\\cite{kzz", "custom cite key")
                require(lastCandidates == [],
                        "zero-match cite key must not see dictionary words, "
                        + "got \(String(describing: lastCandidates))")
            }),
            // Unicode before the cite — a surrogate pair shifts UTF-16
            // offsets; only the current key token may be replaced. The
            // queued Escape/newline must be delivered before the emoji's
            // synchronous insertText runs, so they are separate steps.
            (0.05, { key(escape.code, chars: escape.chars); type("\n") }),
            (0.1, {
                textView.insertText("😀", replacementRange: NSRange(location: NSNotFound, length: 0))
                type("\\cite{kn")
            }),
            (0.4, {
                require(lastCandidates == ["knuth84"],
                        "\\cite{kn must offer knuth84, got \(String(describing: lastCandidates))")
                literal("😀\\cite{kn", "cite after Unicode")
                editor.requestCompletion()
            }),
            (0.2, { key(down.code, chars: down.chars) }),
            (0.1, { key(ret.code, chars: ret.chars) }),
            (0.4, { literal("😀\\cite{knuth84", "cite accept after Unicode") }),
            (0.05, { type(",l") }),
            (0.4, {
                require(lastCandidates == ["lamport94"],
                        "second cite key must offer lamport94, got \(String(describing: lastCandidates))")
                literal("😀\\cite{knuth84,l", "multi-cite prefix")
                editor.requestCompletion()
            }),
            (0.2, { key(down.code, chars: down.chars) }),
            (0.1, { key(ret.code, chars: ret.chars) }),
            (0.4, {
                literal("😀\\cite{knuth84,lamport94", "multi-cite accept")
                stage("cite: Unicode offsets correct, second key replaced only its token")
            }),
            // I: IME marked text must not engage the popup at all.
            (0.05, { key(escape.code, chars: escape.chars); type("\n") }),
            (0.1, { type("\\i") }),
            (0.4, { key(escape.code, chars: escape.chars) }),
            (0.3, {
                callsBeforeIME = CompleteRecorder.calls
                textView.setMarkedText("か", selectedRange: NSRange(location: 0, length: 0),
                                       replacementRange: NSRange(location: NSNotFound, length: 0))
            }),
            (0.4, {
                require(textView.hasMarkedText(), "marked text must be active")
                require(CompleteRecorder.calls == callsBeforeIME,
                        "no completion popup may engage while IME text is marked")
            }),
            (0.05, { textView.unmarkText() }),
            (0.2, {
                require(!textView.hasMarkedText() && textView.string.hasSuffix("\\iか"),
                        "unmark must commit the literal text, tail '\(tail())'")
                stage("IME marked text suppressed completion; commit stayed literal")
            }),
            // The session submit is async — poll until it lands or the
            // bound is hit.
            (0.3, { pollSession() }),
        ]
        driver.scheduleNext()
        // run() must not return while timers are pending — its defers would
        // tear down the window and restore the swizzle. The watchdog turns
        // a stalled chain into a diagnosable FAIL instead of a launcher
        // timeout.
        require(await until(120) { driver.done }, "completion driver stalled — see stage output")
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
                       check=True, timeout=180)
    finally:
        subprocess.run(["/usr/bin/pkill", "-f", str(app)], check=False)
        for log in (out_log, err_log):
            if log.exists():
                print(log.read_text(errors="replace"), end="")
    output = out_log.read_text(errors="replace") if out_log.exists() else ""
    if "CHECK_COMPLETE" not in output:
        sys.exit("FAIL: completion marker missing from checker log")
