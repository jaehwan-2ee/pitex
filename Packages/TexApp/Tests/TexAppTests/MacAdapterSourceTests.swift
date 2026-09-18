import Foundation
import XCTest

final class MacAdapterSourceTests: XCTestCase {
    func testTextKitIsTheAuthoritativeEditingStateOwner() throws {
        let source = try source(at: "Packages/TexApp/Sources/EditorMacAdapter/EditorMacAdapter.swift")

        XCTAssertTrue(source.contains("final class SessionTextView: NSTextView"))
        XCTAssertTrue(source.contains("public let textView: NSTextView"))
        XCTAssertTrue(source.contains("textView.string"))
        XCTAssertTrue(source.contains("textView.selectedRange()"))
        XCTAssertTrue(source.contains("textView.hasMarkedText()"))
        XCTAssertTrue(source.contains("textView.markedRange()"))
        XCTAssertTrue(source.contains("override var undoManager: UndoManager?"))
        XCTAssertTrue(source.contains("nativeUndoManager = nativeView.sessionUndoManager"))
        XCTAssertTrue(source.contains("textView.setSelectedRange"))
        XCTAssertFalse(source.contains("var shadowSelection"))
        XCTAssertFalse(source.contains("var shadowMarkedText"))
    }

    func testAppleFrameworkImportsStayInsideAllowedAdaptersAndAppSources() throws {
        let roots = ["Packages/TexApp/Sources", "Mac/Sources"]
        let files = try swiftFiles(under: roots)
        let frameworkRules: [String: [String]] = [
            "AppKit": ["Packages/TexApp/Sources/EditorMacAdapter/", "Packages/TexApp/Sources/MacPlatform/", "Mac/Sources/"],
            "PDFKit": ["Packages/TexApp/Sources/MacPlatform/", "Mac/Sources/"]
        ]

        for (framework, allowedPrefixes) in frameworkRules {
            let importers = try files.filter { try source(at: $0).contains("import \(framework)") }
            XCTAssertFalse(importers.isEmpty, "Expected an owner for \(framework)")
            for importer in importers {
                XCTAssertTrue(
                    allowedPrefixes.contains { importer.hasPrefix($0) },
                    "\(framework) escaped its adapter/app boundary into \(importer)"
                )
            }
        }
    }

    func testAdaptersKeepStaleConflictAndFailClosedPaths() throws {
        let editor = try source(at: "Packages/TexApp/Sources/EditorMacAdapter/EditorMacAdapter.swift")
        let platform = try source(at: "Packages/TexApp/Sources/MacPlatform/MacPlatform.swift")
        let pdf = try source(at: "Packages/TexApp/Sources/PDFFeature/PDFFeature.swift")
        let app = try sources(under: "Mac/Sources")
        let combined = editor + platform + pdf + app

        XCTAssertTrue(platform.contains("bookmarkDataIsStale: &isStale"))
        XCTAssertTrue(platform.contains("throw PlatformPortError.staleCapability"))
        XCTAssertTrue(editor.contains("case let .rejected(current):"))
        XCTAssertTrue(combined.contains("conflict"))
        XCTAssertTrue(pdf.contains("navigation = .stale"))
        XCTAssertTrue(combined.contains("throw PlatformPortError.unavailable"))
        XCTAssertFalse(combined.contains("TODO"))
        XCTAssertFalse(combined.contains("fake success"))
    }

    func testNonMacOSAdapterBranchesAlwaysFailUnavailableRatherThanPretendSuccess() throws {
        for path in [
            "Packages/TexApp/Sources/EditorMacAdapter/EditorMacAdapter.swift",
            "Packages/TexApp/Sources/MacPlatform/MacPlatform.swift"
        ] {
            let text = try source(at: path)
            let branch = try XCTUnwrap(nonMacBranch(in: text), "Missing non-macOS branch in \(path)")
            XCTAssertTrue(branch.contains("throw PlatformPortError.unavailable"))
            XCTAssertTrue(branch.contains("platform: \"non-macOS\""))
            XCTAssertFalse(branch.contains("return .success"))
            XCTAssertFalse(branch.contains("return true"))
            XCTAssertFalse(branch.contains("return ProcessResult"))
            XCTAssertFalse(branch.contains("return FileCapability"))
        }
    }

    private func nonMacBranch(in text: String) -> String? {
        guard let start = text.range(of: "#else"),
              let end = text.range(of: "#endif", range: start.upperBound..<text.endIndex) else { return nil }
        return String(text[start.upperBound..<end.lowerBound])
    }

    private func swiftFiles(under roots: [String]) throws -> [String] {
        try roots.flatMap { root in
            let url = repositoryRoot.appendingPathComponent(root)
            return try FileManager.default.subpathsOfDirectory(atPath: url.path)
                .filter { $0.hasSuffix(".swift") }
                .map { "\(root)/\($0)" }
        }.sorted()
    }

    private func sources(under root: String) throws -> String {
        try swiftFiles(under: [root]).map { try source(at: $0) }.joined(separator: "\n")
    }

    private func source(at path: String) throws -> String {
        try String(contentsOf: repositoryRoot.appendingPathComponent(path), encoding: .utf8)
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
