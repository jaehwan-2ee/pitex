#!/usr/bin/env python3
"""After a Release build: check-macos-release.py <Build/Products/Release>

Checks editor highlighting on real NSTextView storage and the app's build
executor under Finder's minimal PATH. Requires MacTeX; uses temporary files.
"""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
features = repo / 'Mac/Sources/Features'
check = r'''
import AppKit
import AppPorts
import AppShell
import BuildCore

private actor Doc: DocumentSessionPort {
    let text: String
    init(_ text: String) { self.text = text }
    func snapshot() async -> DocumentSnapshot { .init(revision: 0, text: text) }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult {
        fatalError("Highlighting must never edit the document")
    }
}

@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        for (ext, line, needle, role) in [
            ("tex", "한글 e\u{301} 👩‍💻 \\section{Title} $x^2$ % note\n", "\\section", AppearanceColorRole.commands),
            ("bib", "@article{ref, title={한글 e\u{301} 👩‍💻}, year={2026}}\n", "@", AppearanceColorRole.environments),
        ] {
            let source = String(repeating: line, count: 1000)
            let environment = try await AppShell.make(documentSession: Doc(source))
            let highlighter = SyntaxHighlighter()
            let start = ContinuousClock.now
            highlighter.attach(to: environment.editor, fileExtension: ext)
            let elapsed = start.duration(to: .now)
            precondition(elapsed < .seconds(3), "Highlighting regressed to repeated full-string scans")
            let view = environment.editor.textView
            precondition(view.string == source)
            let index = (source as NSString).range(of: needle, options: .backwards).location
            let color = view.textStorage!.attribute(.foregroundColor, at: index, effectiveRange: nil) as! NSColor
            precondition(color == AppearanceSettings.shared.color(for: role), "Wrong UTF-16 range after Unicode text")
            print("PASS \(ext) highlighting, Unicode offsets and unchanged source: \(elapsed)")
            highlighter.detach()
        }

        precondition(ProcessInfo.processInfo.environment["PATH"] == "/usr/bin:/bin:/usr/sbin:/sbin")
        let source = root.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\begin{document}\nFinder build.\\end{document}\n".write(to: source, atomically: true, encoding: .utf8)
        let executor = StreamingBuildExecutor()
        for engine in ["xelatex", "latexmk"] {
            for suffix in ["pdf", "synctex.gz"] {
                try? FileManager.default.removeItem(at: root.appendingPathComponent("main." + suffix))
            }
            let arguments = (engine == "latexmk" ? ["-pdf"] : []) + ["-synctex=1", "-interaction=nonstopmode", "-halt-on-error", "main.tex"]
            let request = BuildProcessRequest(buildID: try BuildID(rawValue: engine), stageIndex: 0,
                command: .direct(try DirectCommandPlan(executable: engine, arguments: arguments)),
                projectRoot: root, sourceDirectory: root)
            let result = try await executor.execute(request) { _ in }
            precondition(result.exitCode == 0, "\(engine) failed under Finder PATH")
            let pdf = try Data(contentsOf: root.appendingPathComponent("main.pdf"))
            precondition(pdf.starts(with: Data("%PDF".utf8)))
            precondition(FileManager.default.fileExists(atPath: root.appendingPathComponent("main.synctex.gz").path))
            print("PASS Finder PATH: \(engine) creates PDF and SyncTeX, including child tools")
        }
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex release 한글 ', dir='/tmp') as directory:
    root = Path(directory)
    executor = root / 'Executor.swift'
    executor.write_text((features / 'BuildSupport.swift').read_text().split('enum WorkspaceBuildError:')[0])
    source = root / 'Check.swift'
    source.write_text(check)
    executable = root / 'check'
    subprocess.run(['xcrun', 'swiftc', '-O', '-parse-as-library', '-swift-version', '6',
                    '-target', 'arm64-apple-macos15.0', '-I', str(products), str(source), str(executor),
                    str(features / 'SyntaxHighlighting.swift'), str(features / 'AppearanceTheme.swift'),
                    *[str(p) for p in products.glob('*.o')], '-o', str(executable)], check=True)
    subprocess.run([str(executable), str(root)], env={**os.environ, 'PATH': '/usr/bin:/bin:/usr/sbin:/sbin'},
                   cwd=root, check=True, timeout=60)
