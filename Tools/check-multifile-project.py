#!/usr/bin/env python3
"""After Release build: check-multifile-project.py <Build/Products/Release> [project-copy]

Exercises the actual WorkspaceModel and SyncTeX runner. Optional project-copy
is a writable copy of 17_TRB_major_revision for real manuscript verification.
No prompts or model requests; the agent process is disabled.
"""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
check = r'''
import AppKit
import PDFKit
import SyncTeXCore

@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let workspace = WorkspaceModel()
        func require(_ condition: Bool, _ message: String) {
            guard condition else { print("FAIL", message); fflush(nil); exit(1) }
        }
        func build(_ expected: String) async {
            await workspace.startBuild()
            guard case let .succeeded(data, _) = workspace.buildState else {
                print("FAIL build", workspace.buildState, workspace.buildLogText.suffix(5000)); fflush(nil); exit(1)
            }
            require(PDFDocument(data: data)?.string?.localizedCaseInsensitiveContains(expected) == true,
                    "Bibliography content missing from final PDF")
        }
        func roundTrip(file: URL, line: Int) async throws {
            let binding = workspace.syncTeXBinding!
            let runner = SyncTeXRunner()
            let forward = try await runner.forward(binding: binding, sourceURL: file, line: line, column: 0)
            let inverse = try await runner.inverse(binding: binding, page: forward.pdf.page, point: forward.pdf.point)
            require(binding.projectRoot.appendingPathComponent(inverse.source.path.value).standardizedFileURL == file.standardizedFileURL,
                    "Inverse sync must return the same child source, including nested directories")
            await workspace.syncInverse(page: forward.pdf.page, point: forward.pdf.point)
            require(workspace.activeDocumentURL == file.standardizedFileURL, "Inverse sync must activate the child tab")
            require(workspace.buildSourceURL()?.lastPathComponent == binding.pdfURL.deletingPathExtension().appendingPathExtension("tex").lastPathComponent,
                    "Inverse sync must retain the main build target")
        }
        let project = root.appendingPathComponent("project")
        let main = project.appendingPathComponent("paper/main.tex")
        let child = project.appendingPathComponent("paper/sections/intro.tex")
        let deep = project.appendingPathComponent("paper/sections/deep.tex")
        let bib = project.appendingPathComponent("paper/refs/library.bib")
        await workspace.open(project)
        require(workspace.activeDocumentURL == main, "Folder open must choose main, not alphabetically first fragment/bib")
        await workspace.activateDocument(child)
        require(workspace.buildSourceURL() == main, "Child tab must build main")
        await build("BibliographyTarget")
        require(workspace.latestBuiltPDFName == "paper/main.pdf", "Nested outputs belong beside main")
        require(workspace.buildLogText.contains("bibtex"), "First build must run BibTeX")
        try await roundTrip(file: child, line: 2)
        try await roundTrip(file: deep, line: 1)
        await workspace.activateDocument(bib)
        require(workspace.buildSourceURL() == main, "Bibliography tab must retain main")
        print("PASS clean BibTeX build, nested input/include, child and bibliography tabs")
        await workspace.open(deep)
        require(workspace.projectURL == main.deletingLastPathComponent(), "Opening a deep child must discover its parent project")
        require(workspace.latestBuiltPDFName == "main.pdf", "Existing main PDF must load with a child")
        try await roundTrip(file: deep, line: 1)
        await workspace.open(bib)
        require(workspace.buildSourceURL() == main, "Opening a .bib directly must discover its main")
        let oldBib = try String(contentsOf: bib, encoding: .utf8)
        try oldBib.replacingOccurrences(of: "BibliographyTarget", with: "UpdatedReference").write(to: bib, atomically: true, encoding: .utf8)
        await workspace.reloadActiveDocumentFromDisk()
        await build("UpdatedReference")
        print("PASS direct child/bib opening, restored PDF and changed bibliography rebuild")

        let moved = root.appendingPathComponent("moved")
        try FileManager.default.copyItem(at: project, to: moved)
        await workspace.open(moved.appendingPathComponent("paper/sections/intro.tex"))
        try await roundTrip(file: moved.appendingPathComponent("paper/sections/intro.tex"), line: 2)
        print("PASS relocated PDF/SyncTeX paths without rebuilding")
        let binding = workspace.syncTeXBinding!
        let bytes = try Data(contentsOf: binding.pdfURL)
        try (bytes + Data("\n".utf8)).write(to: binding.pdfURL)
        do {
            _ = try await SyncTeXRunner().forward(binding: binding, sourceURL: workspace.activeDocumentURL!, line: 2, column: 0)
            require(false, "Changed PDF must reject old SyncTeX binding")
        } catch SyncTeXQueryError.staleResult { print("PASS stale PDF/metadata rejected") }

        let biber = root.appendingPathComponent("biber/main.tex")
        await workspace.open(biber)
        workspace.buildCommandText = "lualatex -interaction=nonstopmode -synctex=1 {file}"
        await build("BiberTarget")
        require(workspace.buildLogText.contains("biber"), "biblatex first build must run Biber")
        print("PASS LuaLaTeX/biblatex/Biber first build")

        let roots = root.appendingPathComponent("roots")
        let files = try WorkspaceModel.discoverTexFiles(root: roots, selected: roots, isDirectory: true)
        var resolver = TeXProjectResolver()
        let hinted = try resolver.resolve(active: roots.appendingPathComponent("chapters/hinted.tex"), files: files)
        require(hinted == roots.appendingPathComponent("second.tex"), "Explicit root directive must win")
        do {
            _ = try resolver.resolve(active: roots.appendingPathComponent("chapters/shared.tex"), files: files)
            require(false, "Shared fragment must not silently choose between two roots")
        } catch TeXProjectResolver.ResolutionError.ambiguous { print("PASS explicit root and ambiguous shared source") }
        do {
            _ = try resolver.resolve(active: roots.appendingPathComponent("chapters/cycle-a.tex"), files: files)
            require(false, "Cyclic root directives must not loop or select a fragment")
        } catch TeXProjectResolver.ResolutionError.invalidRoot { print("PASS cyclic root directive rejected") }
        await workspace.open(roots.appendingPathComponent("first.tex"))
        workspace.togglePinnedBuildTarget()
        await workspace.activateDocument(roots.appendingPathComponent("chapters/shared.tex"))
        require(workspace.buildSourceURL() == roots.appendingPathComponent("first.tex"), "Pin must override automatic selection")

        if CommandLine.arguments.count > 2 {
            let actual = URL(fileURLWithPath: CommandLine.arguments[2])
            let intro = actual.appendingPathComponent("Body/01-Introduction.tex")
            await workspace.open(intro)
            require(workspace.buildSourceURL()?.lastPathComponent == "icp_manuscript.tex", "Real manuscript ownership")
            await workspace.startBuild()
            guard case let .succeeded(data, _) = workspace.buildState else {
                print("FAIL real manuscript", workspace.buildState, workspace.buildLogText.suffix(5000)); fflush(nil); exit(1)
            }
            require(FileManager.default.fileExists(atPath: actual.appendingPathComponent("icp_manuscript.bbl").path), "Real manuscript missing bbl")
            try await roundTrip(file: intro, line: 8)
            await workspace.activateDocument(actual.appendingPathComponent("library.bib"))
            require(workspace.buildSourceURL()?.lastPathComponent == "icp_manuscript.tex", "Shared bibliography retains selected manuscript")
            print("PASS real manuscript: clean build, bibliography, child round trip; pages", PDFDocument(data: data)!.pageCount)
            let response = actual.appendingPathComponent("response/revised_appendix_a1.tex")
            await workspace.activateDocument(response)
            require(workspace.buildSourceURL()?.lastPathComponent == "response_letter.tex", "Response chapter must use response main")
            await workspace.startBuild()
            guard case let .succeeded(responseData, _) = workspace.buildState else {
                print("FAIL real response", workspace.buildState, workspace.buildLogText.suffix(5000)); fflush(nil); exit(1)
            }
            try await roundTrip(file: response, line: 15)
            print("PASS real response: independent main, bibliography, child round trip; pages", PDFDocument(data: responseData)!.pageCount)
        }
        await workspace.close()
        fflush(nil)
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex multifile 한글 ', dir='/tmp') as directory:
    root = Path(directory)
    def write(name, text):
        p = root / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)
    write('project/paper/main.tex', r'''\documentclass{article}
\begin{document}
MainTarget.
\input{sections/intro}
\include{sections/results}
\bibliographystyle{plain}
\bibliography{refs/library}
\end{document}
''')
    write('project/paper/sections/intro.tex', '\\section{Introduction}\nChildTarget cites \\cite{sample}.\n\\input{sections/deep}\n')
    write('project/paper/sections/deep.tex', 'NestedTarget appears here.\n')
    write('project/paper/sections/results.tex', '\\section{Results}\nResultTarget appears here.\n')
    bib = '@article{sample, author={Doe, Jane}, title={{BibliographyTarget}}, journal={Example}, year={2026}}\n'
    write('project/paper/refs/library.bib', bib)
    write('biber/main.tex', '\\documentclass{article}\n\\usepackage[backend=biber]{biblatex}\n\\addbibresource{refs.bib}\n\\begin{document}\nCitation \\cite{sample}.\n\\printbibliography\n\\end{document}\n')
    write('biber/refs.bib', bib.replace('BibliographyTarget', 'BiberTarget'))
    for name in ['first', 'second']:
        write('roots/' + name + '.tex', '\\documentclass{article}\n\\begin{document}\n\\input{chapters/shared}\n\\end{document}\n')
    write('roots/chapters/shared.tex', 'Shared source.\n')
    write('roots/chapters/hinted.tex', '% !TeX root = "../second"\nExplicit source.\n')
    write('roots/chapters/cycle-a.tex', '% !TeX root = cycle-b.tex\n')
    write('roots/chapters/cycle-b.tex', '% !TeX root = cycle-a.tex\n')
    source = root / 'Check.swift'; source.write_text(check)
    app_main = repo / 'Mac/Sources/AppShell/PitexApp.swift'
    stripped = root / 'PitexApp.swift'; stripped.write_text(app_main.read_text().replace('@main\nstruct PitexApp', 'struct PitexApp'))
    executable = root / 'check'
    subprocess.run(['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6', '-target', 'arm64-apple-macos15.0',
                    '-I', str(products), str(source), str(stripped),
                    *[str(p) for p in (repo / 'Mac/Sources').rglob('*.swift') if p != app_main],
                    *[str(p) for p in products.glob('*.o')], '-o', str(executable)], check=True)
    env = {**os.environ, 'PATH': '/usr/bin:/bin:/usr/sbin:/sbin', 'PI_AGENT_PATH': '/usr/bin/false', 'PI_CODING_AGENT_DIR': str(root / 'pi')}
    subprocess.run([str(executable), str(root), *sys.argv[2:]], env=env, check=True, timeout=240)
