#!/usr/bin/env python3
"""After building Pitex: python3 Tools/check-macos-synctex.py <Build/Products/Debug>

Runs the production PDF/editor coordinators and SyncTeXRunner against a real
three-page document, including /tmp's macOS alias and an included source file.
"""
from pathlib import Path
import gzip
import posixpath
import re
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
features = repo / "Mac/Sources/Features"

check = r'''
private final class RecordingPDFView: PDFView {
    var destination: NSRect?
    override func go(to rect: NSRect, on page: PDFPage) {
        destination = rect
        super.go(to: rect, on: page)
    }
}

@main
struct SyncTeXCheck {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
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

        let document = PDFDocument(url: pdf)!
        let view = RecordingPDFView(frame: NSRect(x: 0, y: 0, width: 600, height: 700))
        view.displayMode = .singlePageContinuous
        view.document = document
        let window = NSWindow(contentRect: view.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = view
        let coordinator = PDFDocumentView.Coordinator()
        coordinator.pdfView = view
        var hit: (Int, SyncTeXCore.PDFPoint)?
        coordinator.onInverseSync = { hit = ($0, $1) }

        func click(_ location: NSPoint, in window: NSWindow, command: Bool = true) -> NSEvent {
            NSEvent.mouseEvent(with: .leftMouseDown, location: location,
                modifierFlags: command ? .command : [], timestamp: 0,
                windowNumber: window.windowNumber, context: nil,
                eventNumber: 1, clickCount: 1, pressure: 1)!
        }

        for (word, file, line) in [("RootTarget", "main.tex", 105), ("ChildTarget", "sections/child.tex", 102)] {
            let selection = document.findString(word, fromSelection: nil, withOptions: [])!
            let page = selection.pages[0]
            let rect = selection.bounds(for: page)
            for scale in [0.75, 1.5] {
                view.scaleFactor = scale
                view.go(to: rect, on: page)
                view.layoutSubtreeIfNeeded()
                let point = view.convert(NSPoint(x: rect.midX, y: rect.midY), from: page)
                let location = view.convert(point, to: nil)
                precondition(!coordinator.handleSyncClick(click(location, in: window, command: false)))
                hit = nil
                precondition(coordinator.handleSyncClick(click(location, in: window)))
                let (number, pdfPoint) = hit!
                let inverse = try await runner.inverse(binding: binding, page: number, point: pdfPoint)
                precondition(inverse.source.path.value == file && inverse.source.line == line)

                let forward = try await runner.forward(binding: binding,
                    sourceURL: root.appendingPathComponent(file), line: line, column: 0)
                precondition(forward.pdf.page == number)
                coordinator.highlightRequested(Notification(name: .syncTeXHighlightRequested, userInfo: [
                    "page": number, "x": forward.h, "y": forward.v,
                    "width": forward.width, "height": forward.height,
                ]))
                let destination = view.destination!
                precondition(abs(destination.minY - (page.bounds(for: .mediaBox).maxY - forward.v - 8)) < 0.01)
                precondition(destination.contains(NSPoint(x: rect.midX, y: rect.midY)))
                let visible = view.convert(rect, from: page)
                precondition(view.bounds.intersects(visible), "Forward sync must reveal the actual text")
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
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", "arm64-apple-macos15.0", "-module-cache-path", str(root / "cache"),
                    "-I", str(products), str(root / "Check.swift"),
                    *[str(features / name) for name in sources],
                    *[str(products / (name + ".o")) for name in modules],
                    "-o", str(executable)], check=True)
    subprocess.run([str(executable), str(root)], check=True)
