import BuildCore
import Foundation
import LanguageCore
import XCTest

final class EquationExactRendererTests: XCTestCase {
    func testProfileFollowsTheProjectBuildCommand() {
        let cases: [(String, ExactEquationProfile?)] = [
            ("xelatex -interaction=nonstopmode -synctex=1 {file}", .init(engine: .xeLaTeX, shellEscape: .projectDefault)),
            ("pdflatex -shell-escape {file}", .init(engine: .pdfLaTeX, shellEscape: .enabled)),
            ("/Library/TeX/texbin/lualatex --no-shell-escape {file}", .init(engine: .luaLaTeX, shellEscape: .disabled)),
            ("latexmk -pdf -interaction=nonstopmode -synctex=1 {file}", .init(engine: .pdfLaTeX, shellEscape: .projectDefault)),
            ("latexmk -xelatex -shell-restricted {file}", .init(engine: .xeLaTeX, shellEscape: .restricted)),
            ("latexmk -pdflua {file}", .init(engine: .luaLaTeX, shellEscape: .projectDefault)),
            ("tectonic --synctex {file}", nil),
            ("make pdf", nil),
            ("pdflatex {file} && bibtex main", nil),
            ("", nil),
        ]
        for (command, expected) in cases {
            XCTAssertEqual(ExactEquationProfile.resolve(buildCommand: command), expected, command)
        }
    }

    func testArgumentsCarryTheShellEscapePolicyAndNothingFromTheDocument() {
        let disabled = ExactEquationProfile(engine: .pdfLaTeX, shellEscape: .disabled).arguments(outputDirectory: "/tmp/x")
        XCTAssertEqual(disabled, ["-interaction=batchmode", "-halt-on-error", "-output-directory=/tmp/x",
                                  "-jobname=pitex-equation", "-no-shell-escape", "pitex-equation.tex"])
        let lua = ExactEquationProfile(engine: .luaLaTeX, shellEscape: .projectDefault).arguments(outputDirectory: "/o")
        XCTAssertFalse(lua.contains { $0.contains("shell") })
        XCTAssertTrue(lua.allSatisfy { $0.hasPrefix("--") || $0 == "pitex-equation.tex" })
    }

    private func requireEngine(_ name: String) throws {
        let path = ProcessInfo.processInfo.environment["PATH"] ?? ""
        let found = path.split(separator: ":").contains {
            FileManager.default.isExecutableFile(atPath: String($0) + "/" + name)
        }
        try XCTSkipUnless(found, "\(name) is not installed")
    }

    /// A real minimal document through the project's engine: PDF bytes come
    /// back, the project directory resolves preamble inputs, and nothing is
    /// left in the temporary tree.
    func testRendersWithProjectPreambleAndCleansUp() async throws {
        try requireEngine("pdflatex")
        let project = FileManager.default.temporaryDirectory.appendingPathComponent("pitex-exact-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: project) }
        try "\\newcommand{\\R}{\\mathbb{R}}\n".write(to: project.appendingPathComponent("commands.tex"), atomically: true, encoding: .utf8)
        let main = "\\documentclass{article}\n\\usepackage{amssymb}\n\\input{commands}\n\\begin{document}\n$x \\in \\R$\n\\end{document}\n"
        let scan = MathSourceScanner.scan(main)
        let document = ExactEquationDocument.make(
            preamble: scan.preamble, preambleDefinitions: [], bodyDefinitions: [],
            region: scan.regions[0].sourceText(in: main)
        )
        let token = "test-\(UUID().uuidString)"
        let renderer = ExactEquationRenderer(
            profile: ExactEquationProfile(engine: .pdfLaTeX, shellEscape: .disabled),
            mainDirectory: project, workspaceToken: token
        )
        let pdf = try await renderer.render(document: document)
        XCTAssertEqual(Array(pdf.prefix(5)), Array("%PDF-".utf8))
        // Second render through the SAME workspaceToken: the base directory
        // must be reused, not re-created (a real bug made this fail).
        let main2 = "\\documentclass{article}\n\\usepackage{amssymb}\n\\input{commands}\n\\begin{document}\n$y \\notin \\R$\n\\end{document}\n"
        let scan2 = MathSourceScanner.scan(main2)
        let document2 = ExactEquationDocument.make(
            preamble: scan2.preamble, preambleDefinitions: [], bodyDefinitions: [],
            region: scan2.regions[0].sourceText(in: main2)
        )
        let pdf2 = try await renderer.render(document: document2)
        XCTAssertEqual(Array(pdf2.prefix(5)), Array("%PDF-".utf8))
        XCTAssertNotEqual(pdf, pdf2)
        let leftovers = try FileManager.default.contentsOfDirectory(atPath: ExactEquationRenderer.workspaceRoot(token: token).path)
        XCTAssertEqual(leftovers, [])
        ExactEquationRenderer.removeWorkspaceArtifacts(token: token)
        XCTAssertFalse(FileManager.default.fileExists(atPath: ExactEquationRenderer.workspaceRoot(token: token).path))
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: project.path), ["commands.tex"])
    }

    func testUnknownMacroFailsWithoutOutput() async throws {
        try requireEngine("pdflatex")
        let token = "test-\(UUID().uuidString)"
        defer { ExactEquationRenderer.removeWorkspaceArtifacts(token: token) }
        let renderer = ExactEquationRenderer(
            profile: ExactEquationProfile(engine: .pdfLaTeX, shellEscape: .disabled),
            mainDirectory: FileManager.default.temporaryDirectory, workspaceToken: token
        )
        let document = ExactEquationDocument.make(preamble: nil, preambleDefinitions: [], bodyDefinitions: [], region: "$\\undefinedmacro$")
        do {
            _ = try await renderer.render(document: document)
            XCTFail("expected a compile failure")
        } catch let ExactEquationRenderError.compileFailed(log) {
            XCTAssertTrue(log.contains("Undefined control sequence"))
        }
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: ExactEquationRenderer.workspaceRoot(token: token).path), [])
    }

    /// Shell escape stays off when the project has it off, even if the
    /// document asks for \write18.
    func testDisabledShellEscapeIsEnforced() async throws {
        try requireEngine("pdflatex")
        let token = "test-\(UUID().uuidString)"
        defer { ExactEquationRenderer.removeWorkspaceArtifacts(token: token) }
        let marker = FileManager.default.temporaryDirectory.appendingPathComponent("pitex-write18-\(UUID().uuidString)")
        let renderer = ExactEquationRenderer(
            profile: ExactEquationProfile(engine: .pdfLaTeX, shellEscape: .disabled),
            mainDirectory: FileManager.default.temporaryDirectory, workspaceToken: token
        )
        let document = ExactEquationDocument.make(
            preamble: "\\documentclass{article}\n\\immediate\\write18{touch \(marker.path)}\n",
            preambleDefinitions: [], bodyDefinitions: [], region: "$x$"
        )
        _ = try await renderer.render(document: document)
        XCTAssertFalse(FileManager.default.fileExists(atPath: marker.path))
    }

    func testCancellationStopsTheEngineAndCleansUp() async throws {
        try requireEngine("pdflatex")
        let token = "test-\(UUID().uuidString)"
        defer { ExactEquationRenderer.removeWorkspaceArtifacts(token: token) }
        let renderer = ExactEquationRenderer(
            profile: ExactEquationProfile(engine: .pdfLaTeX, shellEscape: .disabled),
            mainDirectory: FileManager.default.temporaryDirectory, workspaceToken: token
        )
        // An endless loop keeps the engine busy until cancellation.
        let document = "\\documentclass{article}\n\\begin{document}\n\\def\\loop{\\loop}\\loop\n\\end{document}\n"
        let task = Task { try await renderer.render(document: document) }
        try await Task.sleep(for: .milliseconds(300))
        task.cancel()
        do {
            _ = try await task.value
            XCTFail("expected cancellation")
        } catch let error as ExactEquationRenderError {
            XCTAssertEqual(error, .cancelled)
        }
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: ExactEquationRenderer.workspaceRoot(token: token).path), [])
    }
}
