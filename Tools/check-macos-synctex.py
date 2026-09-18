#!/usr/bin/env python3
"""After building Pitex: python3 Tools/check-macos-synctex.py <Build/Products/Debug>

Runs the production PDF/editor coordinators and SyncTeXRunner against a real
three-page document, including /tmp's macOS alias and an included source file.
"""
from pathlib import Path
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
        let binding = try await runner.refreshBinding(projectRoot: root, pdfURL: pdf, buildID: "check")
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
    # Keep the private PDF coordinator in the same compilation unit as its check.
    preview = (features / "Preview.swift").read_text()
    (root / "Check.swift").write_text(
        "import AppKit\nimport PDFKit\nimport SwiftUI\nimport SyncTeXCore\n"
        + preview[preview.index("private struct PDFDocumentView:"):] + check
    )
    modules = ["BuildCore", "SyncTeXCore", "TexDomain", "DocumentSessionCore", "ProjectCore",
               "EditorMacAdapter", "EditorFeature", "AppPorts", "LanguageCore", "AICore"]
    sources = ["SyncTeXSupport.swift", "EditorContainerView.swift", "EditorFolding.swift", "AppearanceTheme.swift"]
    executable = root / "check"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", "arm64-apple-macos15.0", "-module-cache-path", str(root / "cache"),
                    "-I", str(products), str(root / "Check.swift"),
                    *[str(features / name) for name in sources],
                    *[str(products / (name + ".o")) for name in modules],
                    "-o", str(executable)], check=True)
    subprocess.run([str(executable), str(root)], check=True)
