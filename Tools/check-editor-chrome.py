#!/usr/bin/env python3
"""After a Release build: check-editor-chrome.py <Build/Products/Release>

Hosts the production SwiftUI/AppKit editor. Checks clipping and clicks the
rendered minimap after wrapping, resizing, font changes, folding and tab changes.
"""
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
check = r'''
import AppKit
import AppPorts
import EditorMacAdapter
import SwiftUI

private actor Doc: DocumentSessionPort {
    let text: String
    init(_ text: String) { self.text = text }
    func snapshot() async -> DocumentSnapshot { .init(revision: 0, text: text) }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult { .rejected(current: await snapshot()) }
}
private final class Backdrop: NSView {
    override var isFlipped: Bool { true }
    override func draw(_ dirtyRect: NSRect) { NSColor.red.setFill(); bounds.fill() }
}
@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        func content(_ adapter: EditorMacAdapter) -> some View {
            EditorContainerView(adapter: adapter).id(ObjectIdentifier(adapter))
        }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        func bitmap(_ view: NSView) -> NSBitmapImageRep {
            let result = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
            view.cacheDisplay(in: view.bounds, to: result)
            return result
        }
        let source = "\\begin{itemize}\n" + String(repeating: "\\item " + String(repeating: "wrapped text ", count: 20) + "\n", count: 80)
            + "\\end{itemize}\n\nLAST_TARGET\n"
        let first = try await EditorMacAdapter.make(session: Doc(source))
        let host = NSHostingView(rootView: content(first))
        let root = Backdrop(frame: NSRect(x: 0, y: 0, width: 600, height: 360))
        root.addSubview(host)
        let window = NSWindow(contentRect: root.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = root
        defer { window.orderOut(nil) }
        window.orderFront(nil)

        func settle() async throws {
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(100))
            root.displayIfNeeded()
        }
        func verify(_ adapter: EditorMacAdapter, name: String) async throws {
            try await settle()
            let views = descendants(host)
            let scroll = views.compactMap { $0 as? NSScrollView }.first!
            let map = views.compactMap { $0 as? MinimapOverlayView }.first!
            let gutter = views.compactMap { $0 as? LineNumberGutterView }.first!
            let chips = views.compactMap { $0 as? FoldChipOverlayView }.first!
            let text = adapter.textView
            precondition(scroll.documentView === text, "Tab must own all its editor chrome")
            precondition(scroll.clipsToBounds && gutter.clipsToBounds && chips.clipsToBounds && map.clipsToBounds)
            precondition(scroll.contentView.frame.contains(gutter.frame) && scroll.contentView.frame.contains(map.frame))
            let target = (text.string as NSString).range(of: "LAST_TARGET")
            text.textStorage!.addAttribute(.foregroundColor, value: NSColor.green, range: target)
            let layout = text.layoutManager!, container = text.textContainer!
            layout.ensureLayout(for: container)
            text.scroll(NSPoint(x: 0, y: 17.5)) // partial row at the top and bottom
            try await settle()
            let image = bitmap(map)
            var green: [NSPoint] = []
            for y in 0..<image.pixelsHigh {
                for x in 0..<image.pixelsWide {
                    let color = image.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
                    if color.greenComponent - color.redComponent > 0.1 && color.greenComponent - color.blueComponent > 0.1 {
                        green.append(NSPoint(x: CGFloat(x) + 0.5, y: CGFloat(y) + 0.5))
                    }
                }
            }
            precondition(!green.isEmpty, "Minimap must render the current document's final target")
            let centre = green.reduce(NSPoint.zero) { NSPoint(x: $0.x + $1.x, y: $0.y + $1.y) }
            let local = NSPoint(x: centre.x / CGFloat(green.count) * map.bounds.width / CGFloat(image.pixelsWide),
                                y: centre.y / CGFloat(green.count) * map.bounds.height / CGFloat(image.pixelsHigh))
            let event = NSEvent.mouseEvent(with: .leftMouseDown, location: map.convert(local, to: nil), modifierFlags: [],
                timestamp: 0, windowNumber: window.windowNumber, context: nil, eventNumber: 1, clickCount: 1, pressure: 1)!
            map.mouseDown(with: event)
            let glyphs = layout.glyphRange(forCharacterRange: target, actualCharacterRange: nil)
            let rect = layout.boundingRect(forGlyphRange: glyphs, in: container)
                .offsetBy(dx: text.textContainerOrigin.x, dy: text.textContainerOrigin.y)
            precondition(text.visibleRect.intersects(rect), "Clicking the painted minimap target must reveal that exact text")

            // Pixel check: scrolling must not paint line numbers into the
            // header/footer outside the editor's AppKit hosting view.
            let all = bitmap(root)
            let backdrop = all.colorAt(x: 0, y: 0)!.usingColorSpace(.deviceRGB)!
            for y in [35, Int(root.bounds.height) - 35] {
                for x in 5..<55 {
                    let color = all.colorAt(x: x * all.pixelsWide / Int(root.bounds.width),
                        y: y * all.pixelsHigh / Int(root.bounds.height))!.usingColorSpace(.deviceRGB)!
                    precondition(abs(color.redComponent - backdrop.redComponent) < 0.01 && abs(color.greenComponent - backdrop.greenComponent) < 0.01 && abs(color.blueComponent - backdrop.blueComponent) < 0.01,
                                 "Editor chrome painted outside its viewport at \(x),\(y): \(color)")
                }
            }
            print("PASS \(name): clipped chrome, final minimap target and click alignment"); fflush(nil)
        }

        for size in [NSSize(width: 600, height: 360), NSSize(width: 350, height: 220)] {
            root.setFrameSize(size)
            host.frame = NSRect(x: 0, y: 40, width: size.width, height: size.height - 80)
            try await verify(first, name: "resize \(size)")
        }
        first.textView.font = .monospacedSystemFont(ofSize: 24, weight: .regular)
        try await verify(first, name: "larger font")
        let gutter = descendants(host).compactMap { $0 as? LineNumberGutterView }.first!
        gutter.foldEngine!.toggle(atLine: 0)
        try await verify(first, name: "folded environment")

        let second = try await EditorMacAdapter.make(session: Doc(String(repeating: "short line\n", count: 35) + "LAST_TARGET\n"))
        host.rootView = content(second)
        try await verify(second, name: "new document, compact minimap scale")
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex-editor-chrome-', dir='/tmp') as directory:
    root = Path(directory)
    source = root / 'Check.swift'
    source.write_text(check)
    executable = root / 'check'
    subprocess.run(['xcrun', 'swiftc', '-g', '-parse-as-library', '-swift-version', '6',
                    '-target', 'arm64-apple-macos15.0', '-I', str(products), str(source),
                    *[str(repo / 'Mac/Sources/Features' / f) for f in ['EditorContainerView.swift', 'EditorFolding.swift', 'AppearanceTheme.swift']],
                    *[str(p) for p in products.glob('*.o')], '-o', str(executable)], check=True)
    subprocess.run([str(executable)], check=True, timeout=45)
