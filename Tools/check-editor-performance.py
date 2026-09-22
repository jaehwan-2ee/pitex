#!/usr/bin/env python3
"""Native macOS checks for minimap reuse, edit invalidation and completion parity."""
from pathlib import Path
import subprocess
import tempfile

repo = Path(__file__).resolve().parent.parent
path = 'Mac/Sources/Features/EditorContainerView.swift'
def minimap(source):
    return source[source.index('final class MinimapOverlayView:'):source.index('/// Ghost-text layer')]
current = minimap((repo / path).read_text())
current = current.replace('final class MinimapOverlayView: NSView {', 'final class MinimapOverlayView: NSView {\n    var rebuilds = 0')
current = current.replace('    private func drawDocument() {', '    private func drawDocument() {\n        rebuilds += 1')
old = minimap(subprocess.check_output(['git', 'show', f'v1.5.0:{path}'], cwd=repo, text=True))
old = old.replace('MinimapOverlayView', 'BaselineMinimapOverlayView')
path = 'Packages/TexCore/Sources/LanguageCore/CompletionContext.swift'
completion = (repo / path).read_text().split('extension LanguageIndex')[0]
prior = subprocess.check_output(['git', 'show', f'v1.5.0:{path}'], cwd=repo, text=True)
prior = prior[prior.index('public enum CompletionContextDetector'):prior.index('extension LanguageIndex')]
prior = prior.replace('CompletionContextDetector', 'BaselineCompletionContextDetector')
host = r'''
import AppKit
@MainActor final class AppearanceSettings {
    enum Role { case bodyText }
    static let shared = AppearanceSettings()
    func color(for role: Role) -> NSColor { .black }
}
func check(_ value: Bool, _ message: String = "check failed", line: Int = #line) {
    if !value {
        FileHandle.standardError.write(Data("line \(line): \(message)\n".utf8))
        fatalError(message)
    }
}
@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        for text in ["", "한👩🏽‍💻 \\cite{a,ke", "e\u{301} \\ref{sec:한", "\\cite[한][p.2]{x", "\\alpha", "\\\\alpha", "\\cite{a,\n b"] {
            for caret in 0...(text.utf16.count + 2) {
                check(CompletionContextDetector.context(in:text,caretUTF16Offset:caret)
                    == BaselineCompletionContextDetector.context(in:text,caretUTF16Offset:caret))
            }
        }
        let text = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
        let scroll = NSScrollView(frame:NSRect(x:0,y:0,width:800,height:600))
        let view = NSTextView(frame:scroll.bounds)
        view.isVerticallyResizable = true
        view.textContainer!.containerSize = NSSize(width:800,height:CGFloat.greatestFiniteMagnitude)
        view.textContainer!.widthTracksTextView = true
        view.font = NSFont.monospacedSystemFont(ofSize:12,weight:.regular)
        view.string = text
        scroll.documentView = view
        view.layoutManager!.ensureLayout(for:view.textContainer!)
        let old = BaselineMinimapOverlayView(textView:view,scrollView:scroll)
        let new = MinimapOverlayView(textView:view,scrollView:scroll)
        func render(_ map: NSView) -> Data {
            let bitmap = NSBitmapImageRep(bitmapDataPlanes:nil,pixelsWide:80,pixelsHigh:600,
                bitsPerSample:8,samplesPerPixel:4,hasAlpha:true,isPlanar:false,colorSpaceName:.deviceRGB,
                bytesPerRow:0,bitsPerPixel:0)!
            NSGraphicsContext.saveGraphicsState()
            NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep:bitmap)
            NSGraphicsContext.current!.cgContext.clear(NSRect(x:0,y:0,width:80,height:600))
            let transform = NSAffineTransform()
            transform.translateX(by:0,yBy:600); transform.scaleX(by:1,yBy:-1); transform.concat()
            map.draw(map.bounds)
            NSGraphicsContext.restoreGraphicsState()
            return Data(bytes:bitmap.bitmapData!,count:bitmap.bytesPerRow * bitmap.pixelsHigh)
        }
        func median(_ action: () -> Void) -> Double {
            var times:[Double] = []
            for _ in 0..<5 {
                let start = DispatchTime.now().uptimeNanoseconds
                action()
                times.append(Double(DispatchTime.now().uptimeNanoseconds-start)/1_000_000)
            }
            return times.sorted()[2]
        }
        let image = render(new)
        check(image.contains { $0 != 0 })
        let before = median { _ = render(old) }
        let after = median { check(render(new) == image) }
        check(new.rebuilds == 1, "unchanged/scroll redraw rebuilt the document")
        view.string = "\\section{새 문서}\n한글 👩🏽‍💻\n"
        // AppKit can deliver layout/frame notifications after the edit,
        // restarting the debounce. Await the actual rebuild, not one fixed delay.
        var editedImage = image
        for _ in 0..<60 {
            try await Task.sleep(for:.milliseconds(50))
            editedImage = render(new)
            if new.rebuilds > 1 { break }
        }
        check(editedImage != image, "edit did not invalidate the minimap")
        check(new.rebuilds == 2)
        print("{\"operation\":\"minimap redraw\",\"before_ms\":\(before),\"after_ms\":\(after),\"speedup\":\(before/after)}")
        print("PASS minimap cache/edit invalidation and Unicode completion parity")
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex-editor-check-') as directory:
    root = Path(directory)
    source = root / 'Check.swift'
    source.write_text(current + old + completion + prior + host)
    subprocess.run(['swiftc', '-O', '-parse-as-library', str(source), '-o', str(root / 'check')], check=True)
    subprocess.run([str(root / 'check'), str(repo / 'Fixtures/projects/large/main.tex')], check=True, timeout=120)
