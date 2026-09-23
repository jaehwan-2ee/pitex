#!/usr/bin/env python3
"""Compare native NSTextStorage colors and timing against v1.4.2 (macOS).

Runs production highlighters with a minimal editor/palette host; no user settings,
app session, network or Xcode required. TextKit storage is real AppKit storage.
"""
from pathlib import Path
import subprocess
import tempfile

repo = Path(__file__).resolve().parent.parent
core = (repo / 'Packages/TexCore/Sources/LanguageCore/LanguageCore.swift').read_text()
old_core = subprocess.check_output(['git', 'show', 'v1.4.2:Packages/TexCore/Sources/LanguageCore/LanguageCore.swift'], cwd=repo, text=True)
old_lexer = old_core[old_core.index('public enum DeterministicTeXLexer'):old_core.index('public enum OutlineKind')].replace('DeterministicTeXLexer', 'BaselineTeXLexer')
path = 'Mac/Sources/Features/SyntaxHighlighting.swift'
current = (repo / path).read_text()
prior = subprocess.check_output(['git', 'show', f'v1.4.2:{path}'], cwd=repo, text=True)
prior = prior.replace('SyntaxHighlighter', 'BaselineSyntaxHighlighter').replace('DeterministicTeXLexer', 'BaselineTeXLexer')

def strip_imports(text):
    return '\n'.join(line for line in text.splitlines() if not line.startswith('import '))

host = r'''
import AppKit
@MainActor final class EditorMacAdapter {
    let textView = NSTextView()
    var onTextDidChange: (() -> Void)?
}
enum AppearanceColorRole { case bodyText, commands, comments, braces, environments, math, lineNumbers }
@MainActor final class AppearanceSettings {
    static let shared = AppearanceSettings()
    func color(for role: AppearanceColorRole) -> NSColor {
        switch role {
        case .bodyText: .black
        case .commands: .red
        case .comments: .green
        case .braces: .blue
        case .environments: .orange
        case .math: .purple
        case .lineNumbers: .gray
        }
    }
}
@main struct HighlightCheck {
    @MainActor static func main() throws {
        _ = NSApplication.shared
        let source = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
        let before = EditorMacAdapter(), after = EditorMacAdapter()
        before.textView.string = source
        after.textView.string = source
        for editor in [before, after] {
            editor.textView.textStorage!.addAttribute(.backgroundColor, value: NSColor.yellow, range: NSRange(location: 0, length: 10))
        }
        let old = BaselineSyntaxHighlighter(), new = SyntaxHighlighter()
        old.attach(to: before, fileExtension: "tex")
        new.attach(to: after, fileExtension: "tex")
        func checkColors() {
            precondition(before.textView.string == after.textView.string)
            let a = before.textView.textStorage!, b = after.textView.textStorage!
            a.enumerateAttributes(in: NSRange(location: 0, length: a.length)) { attrs, range, _ in
                b.enumerateAttributes(in: range) { other, _, _ in
                    precondition(NSDictionary(dictionary: attrs).isEqual(to: other), "Changed attributes at \(range)")
                }
            }
        }
        checkColors()
        func median(_ action: () -> Void) -> Double {
            var times: [Double] = []
            for _ in 0..<5 {
                let start = DispatchTime.now().uptimeNanoseconds
                action()
                times.append(Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000)
            }
            return times.sorted()[2]
        }
        let baseline = median {
            before.textView.textStorage!.replaceCharacters(in: NSRange(location: 0, length: 0), with: "x")
            old.highlightNow()
        }
        let optimized = median {
            after.textView.textStorage!.replaceCharacters(in: NSRange(location: 0, length: 0), with: "x")
            new.highlightNow()
        }
        checkColors()
        print("{\"operation\":\"macOS highlighting after one-character edits\",\"before_ms\":\(baseline),\"after_ms\":\(optimized),\"speedup\":\(baseline / optimized)}")
        before.textView.textStorage!.replaceCharacters(in: NSRange(location: 5, length: 0), with: "한글 👩🏽‍💻 ")
        after.textView.textStorage!.replaceCharacters(in: NSRange(location: 5, length: 0), with: "한글 👩🏽‍💻 ")
        old.highlightNow(); new.highlightNow(); checkColors()
        // Old colors must disappear when a command becomes plain text; new
        // math/comment/BibTeX colors and Unicode offsets must still match.
        for (text, ext) in [("한글 e\u{301} 👩🏽‍💻 \\section{Title} $x$ % note\n", "tex"),
                            ("plain words after deleting the commands\n", "tex"),
                            ("@article{한글, title={👩🏽‍💻 e\u{301}}, year={2026}}", "bib")] {
            before.textView.string = text; after.textView.string = text
            old.attach(to: before, fileExtension: ext); new.attach(to: after, fileExtension: ext)
            checkColors()
        }
        // Seeded random edits: identical inserts/deletes/replaces on both
        // views, rehighlighting after each batch, attribute state compared
        // over the whole document every round.
        struct SeededRNG: RandomNumberGenerator {
            var state: UInt64
            mutating func next() -> UInt64 {
                state &+= 0x9E3779B97F4A7C15
                var z = state
                z = (z ^ (z >> 30)) &* 0xBF58476D1CE4E5B9
                z = (z ^ (z >> 27)) &* 0x94D049BB133111EB
                return z ^ (z >> 31)
            }
        }
        var rng = SeededRNG(state: 0x5EED)
        func randomEdits(_ rounds: Int, _ snippets: [String]) {
            for _ in 0..<rounds {
                for _ in 0..<Int.random(in: 1...3, using: &rng) {
                    let length = before.textView.textStorage!.length
                    let location = length == 0 ? 0 : Int.random(in: 0...length, using: &rng)
                    let removed = Int.random(in: 0...min(length - location, 24), using: &rng)
                    let insert = snippets[Int.random(in: 0..<snippets.count, using: &rng)]
                    before.textView.textStorage!.replaceCharacters(in: NSRange(location: location, length: removed), with: insert)
                    after.textView.textStorage!.replaceCharacters(in: NSRange(location: location, length: removed), with: insert)
                }
                old.highlightNow(); new.highlightNow()
                checkColors()
            }
        }
        // The dialect loop above left both attached as .bibtex; rebind to LaTeX
        // so the long run exercises the LaTeX highlighting path.
        before.textView.string = source; after.textView.string = source
        old.attach(to: before, fileExtension: "tex"); new.attach(to: after, fileExtension: "tex")
        checkColors()
        randomEdits(220, ["x", "한", "👩🏽‍💻", "e\u{301}", "\\cmd", "{", "}", "$", "% note\n", "\n", "\\section{T}\n"])
        // Shorter run in the BibTeX dialect on a BibTeX-ish document.
        let bib = String(repeating: "@article{한글, title={👩🏽‍💻 e\u{301}}, year={2026}}\n", count: 300)
        before.textView.string = bib; after.textView.string = bib
        old.attach(to: before, fileExtension: "bib"); new.attach(to: after, fileExtension: "bib")
        checkColors()
        randomEdits(60, ["@book{k,", "title={", "}", ",\n", "x", "한", "👩🏽‍💻", "\n"])
        print("PASS native TextKit: identical colors, Unicode offsets, preserved attributes and edit/rebind behavior")
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex-highlighting-') as directory:
    root = Path(directory)
    source = root / 'Check.swift'
    source.write_text(core + old_lexer + strip_imports(current) + strip_imports(prior) + host)
    executable = root / 'check'
    subprocess.run(['swiftc', '-swift-version', '6', '-O', '-parse-as-library', str(source), '-o', str(executable)], check=True)
    subprocess.run([str(executable), str(repo / 'Fixtures/projects/large/main.tex')], check=True, timeout=120)
