import BuildFeature
import Foundation
import XCTest

final class BuildInputsTests: XCTestCase {
    /// Owner report: in an SSH project, Save on notes.md uploaded it and then
    /// started the LaTeX compiler.
    func testSavingNonTeXFilesDoesNotBuild() {
        for plain in ["notes.md", "README.MD", "notes.txt", "data.json"] {
            XCTAssertFalse(BuildInputs.saveStartsBuild(activePath: plain, dirtyPaths: [plain]), plain)
        }
    }

    func testSavingABuildInputBuilds() {
        XCTAssertTrue(BuildInputs.saveStartsBuild(activePath: "main.tex", dirtyPaths: []))
        XCTAssertTrue(BuildInputs.saveStartsBuild(activePath: "chapters/intro.tex", dirtyPaths: ["chapters/intro.tex"]))
        XCTAssertTrue(BuildInputs.saveStartsBuild(activePath: "refs.bib", dirtyPaths: ["refs.bib"]))
        // Save writes every dirty session; an unsaved chapter .tex goes too.
        XCTAssertTrue(BuildInputs.saveStartsBuild(activePath: "notes.md", dirtyPaths: ["chapters/intro.tex", "notes.md"]))
    }

    /// The app's Save and Save All ask the policy; only Build always builds.
    func testRemoteSaveAsksTheBuildInputPolicy() throws {
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { root.deleteLastPathComponent() }
        let app = try String(contentsOf: root.appendingPathComponent("Mac/Sources/AppShell/PitexApp.swift"), encoding: .utf8)
        let build = try String(contentsOf: root.appendingPathComponent("Mac/Sources/Features/BuildSupport.swift"), encoding: .utf8)
        XCTAssertFalse(app.contains("saveRemoteSources(build: true)"))
        XCTAssertEqual(app.components(separatedBy: "saveRemoteSources(build: await saveStartsBuild())").count - 1, 2)
        XCTAssertEqual(build.components(separatedBy: "saveRemoteSources(build: true)").count - 1, 1)
    }
}
