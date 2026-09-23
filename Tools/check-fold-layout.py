#!/usr/bin/env python3
"""Differential fold-layout check: production FoldEngine vs v1.8.1 (macOS).

Two identical text views fold the same regions of the large fixture, take the
same deterministic edits, and after every step must agree on text, hidden
lines, and each glyph's line fragment rect and not-shown flag. TextKit layout
is real AppKit layout; no user settings or Xcode project required.
"""
from pathlib import Path
import subprocess
import tempfile

repo = Path(__file__).resolve().parent.parent
folding = 'Mac/Sources/Features/EditorFolding.swift'
highlighting = 'Mac/Sources/Features/SyntaxHighlighting.swift'

def engine(source):
    return source[source.index('struct FoldRegion'):source.index('/// Paints the reference editor')]

current = engine((repo / folding).read_text())
prior = subprocess.check_output(['git', 'show', f'v1.8.1:{folding}'], cwd=repo, text=True)
prior = engine(prior).replace('FoldRegion', 'BaselineFoldRegion').replace('FoldEngine', 'BaselineFoldEngine')
analysis_src = (repo / highlighting).read_text()
analysis = analysis_src[analysis_src.index('/// A text storage owns'):analysis_src.index('/// Applies deterministic')]
core = (repo / 'Packages/TexCore/Sources/LanguageCore/LanguageCore.swift').read_text()

def strip_imports(text):
    return '\n'.join(line for line in text.splitlines() if not line.startswith('import '))

host = r'''
import AppKit
func check(_ ok: Bool, _ message: String, line: Int = #line) {
    if !ok {
        FileHandle.standardError.write(Data("line \(line): \(message)\n".utf8))
        fatalError(message)
    }
}
@main struct FoldLayoutCheck {
    @MainActor static func main() throws {
        _ = NSApplication.shared
        let source = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
        func makeView(_ text: String) -> NSTextView {
            let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 800, height: 600))
            let view = NSTextView(frame: scroll.bounds)
            view.isVerticallyResizable = true
            view.textContainer!.containerSize = NSSize(width: 800, height: CGFloat.greatestFiniteMagnitude)
            view.textContainer!.widthTracksTextView = true
            view.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
            view.string = text
            scroll.documentView = view
            return view
        }
        let before = makeView(source)
        let after = makeView(source)
        let old = BaselineFoldEngine()
        let new = FoldEngine()
        old.attach(to: before)
        new.attach(to: after)
        func compare(_ label: String) {
            check(before.string == after.string, "\(label): texts diverged")
            let la = before.layoutManager!, lb = after.layoutManager!
            la.ensureLayout(for: before.textContainer!)
            lb.ensureLayout(for: after.textContainer!)
            check(la.numberOfGlyphs == lb.numberOfGlyphs, "\(label): glyph count differs")
            for glyph in 0..<la.numberOfGlyphs {
                check(la.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
                      == lb.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil),
                      "\(label): line fragment rect differs at glyph \(glyph)")
                check(la.notShownAttributeForGlyph(at: glyph)
                      == lb.notShownAttributeForGlyph(at: glyph),
                      "\(label): not-shown flag differs at glyph \(glyph)")
            }
            let lines = FoldEngine.computeLineStarts(before.string as NSString).count
            for line in 0..<lines {
                check(old.isLineHidden(line) == new.isLineHidden(line),
                      "\(label): hidden state differs at line \(line)")
            }
        }
        // When set, lay out both views between the edit and recompute — the
        // real app lays out during the 120 ms debounce, with the stale
        // delegate state the incremental invalidation then has to repair.
        var layOutBetweenEdits = false
        func edit(_ location: Int, _ length: Int, _ insert: String, _ label: String) {
            let range = NSRange(location: location, length: length)
            before.textStorage!.replaceCharacters(in: range, with: insert)
            after.textStorage!.replaceCharacters(in: range, with: insert)
            if layOutBetweenEdits {
                before.layoutManager!.ensureLayout(for: before.textContainer!)
                after.layoutManager!.ensureLayout(for: after.textContainer!)
            }
            // Flush queued didProcessEditing observers before recomputing.
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
            old.recompute()
            new.recompute()
            compare(label)
        }
        check(old.regions.map(\.headerLine) == new.regions.map(\.headerLine),
              "regions differ before folding")
        check(!old.regions.isEmpty, "fixture has no foldable regions")
        var headers: [Int] = []
        if let region = old.regions.first(where: { $0.signature.hasPrefix("begin:") }) {
            headers.append(region.headerLine)
        }
        if let region = old.regions.first(where: { $0.signature.hasPrefix("section:") }) {
            headers.append(region.headerLine)
        }
        headers.append(old.regions[old.regions.count / 2].headerLine)
        headers.append(old.regions[old.regions.count - 1].headerLine)
        for line in Set(headers).sorted() {
            old.toggle(atLine: line)
            new.toggle(atLine: line)
        }
        compare("initial folds")
        func length() -> Int { (before.string as NSString).length }
        // First hidden line span of the earliest folded region, in UTF-16
        // offsets — the same math applyFolds uses for hiddenCharRanges.
        func hiddenSpan() -> (Int, Int)? {
            guard let region = new.regions.first(where: { $0.folded }) else { return nil }
            let first = new.lineStart(for: region.hiddenLineRange.lowerBound)
            var end = new.lineStart(for: region.hiddenLineRange.upperBound + 1)
            if end < 0 { end = length() }
            guard first >= 0, end > first else { return nil }
            return (first, end)
        }
        // UTF-16 offset just past the last folded region's hidden lines.
        func afterLastFold() -> Int {
            var tail = 0
            for region in new.regions where region.folded {
                let end = new.lineStart(for: region.hiddenLineRange.upperBound + 1)
                tail = max(tail, end < 0 ? length() : end)
            }
            return tail
        }
        func scenario(_ pass: Int) {
            layOutBetweenEdits = pass == 2
            edit(5, 0, "xyz", "pass \(pass): insert before first fold")
            edit(7, 1, "", "pass \(pass): delete before first fold")
            if let (first, end) = hiddenSpan() {
                let mid = first + (end - first) / 2
                edit(mid, 1, "q", "pass \(pass): same-length replace inside hidden range")
                edit(mid, 1, "q q", "pass \(pass): different-length replace inside hidden range")
                edit(mid, 2, "", "pass \(pass): delete inside hidden range")
            }
            if let header = headers.first {
                let start = new.lineStart(for: header)
                if start >= 0 {
                    edit(start + 1, 0, "X", "pass \(pass): edit on folded header line")
                }
            }
            edit(min(afterLastFold() + 40, length()), 0, " tail", "pass \(pass): edit after last folded region")
            edit(5, 0, "\n", "pass \(pass): newline insert before first region")
            edit(5, 1, "", "pass \(pass): newline delete before first region")
            edit(min(20, length()), 0, "한글 👩🏽‍💻", "pass \(pass): Korean and emoji insert")
        }
        // Pass 2 lays out between each edit and recompute, so the partial
        // invalidation must repair layout produced under the stale delegate.
        scenario(1)
        scenario(2)
        func median(_ action: () -> Void) -> Double {
            var times: [Double] = []
            for _ in 0..<5 {
                let start = DispatchTime.now().uptimeNanoseconds
                action()
                times.append(Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000)
            }
            return times.sorted()[2]
        }
        // Edit deep in the document so the incremental path skips most of
        // the layout work; both engines take the same edits and stay in sync.
        let spot = length() * 3 / 4
        let baseline = median {
            before.textStorage!.replaceCharacters(in: NSRange(location: spot, length: 0), with: "x")
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
            old.recompute()
        }
        let optimized = median {
            after.textStorage!.replaceCharacters(in: NSRange(location: spot, length: 0), with: "x")
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
            new.recompute()
        }
        compare("after timing")
        print("{\"operation\":\"macOS fold recompute after edits\",\"before_ms\":\(baseline),\"after_ms\":\(optimized),\"speedup\":\(baseline / optimized)}")
        print("PASS fold layout: identical text, hidden lines, fragment rects and not-shown flags")
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex-fold-') as directory:
    root = Path(directory)
    source = root / 'Check.swift'
    source.write_text(strip_imports(core) + analysis + current + prior + host)
    executable = root / 'check'
    subprocess.run(['swiftc', '-swift-version', '6', '-O', '-parse-as-library', str(source), '-o', str(executable)], check=True)
    subprocess.run([str(executable), str(repo / 'Fixtures/projects/large/main.tex')], check=True, timeout=180)
