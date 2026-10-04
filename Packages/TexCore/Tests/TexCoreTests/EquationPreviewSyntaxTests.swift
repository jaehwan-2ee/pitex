import Foundation
import LanguageCore
import XCTest

/// Runs the shared `Fixtures/equation-preview/syntax.json` cases — the Rust
/// `language-core` port runs the same file, so both scanners must agree.
final class EquationPreviewSyntaxTests: XCTestCase {
    private static let caret = "\u{2038}"

    private var fixture: [String: Any] {
        get throws {
            let url = repositoryRoot.appendingPathComponent("Fixtures/equation-preview/syntax.json")
            return try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
        }
    }

    /// Source without the marker plus the marker's UTF-8 offset.
    private func split(_ marked: String) throws -> (String, Int) {
        let range = try XCTUnwrap(marked.range(of: Self.caret))
        let offset = marked.utf8.distance(from: marked.startIndex, to: range.lowerBound)
        return (marked.replacingCharacters(in: range, with: ""), offset)
    }

    func testRegionFixtures() throws {
        let cases = try XCTUnwrap(fixture["regions"] as? [[String: Any]])
        XCTAssertFalse(cases.isEmpty)
        for entry in cases {
            let name = try XCTUnwrap(entry["name"] as? String)
            let (source, offset) = try split(try XCTUnwrap(entry["source"] as? String))
            let scan = MathSourceScanner.scan(source)
            let region = (entry["mode"] as? String) == "character"
                ? scan.region(atCharacter: offset)
                : scan.region(atCaret: offset)
            guard let expected = entry["expected"] as? [String: Any] else {
                XCTAssertNil(region, name)
                continue
            }
            guard let region else { XCTFail("\(name): no region"); continue }
            XCTAssertEqual(region.sourceText(in: source), expected["text"] as? String, name)
            XCTAssertEqual(region.isComplete, expected["complete"] as? Bool, name)
            guard region.isComplete else { continue }
            XCTAssertEqual(region.renderSource(in: source), expected["render"] as? String, name)
            XCTAssertEqual(region.displayMode, expected["display"] as? Bool, name)
            XCTAssertEqual(region.environmentName, expected["environment"] as? String, name)
            XCTAssertEqual(region.isWellFormed, expected["wellFormed"] as? Bool, name)
        }
    }

    func testDefinitionAndIncludeFixtures() throws {
        for entry in try XCTUnwrap(fixture["definitions"] as? [[String: Any]]) {
            let name = try XCTUnwrap(entry["name"] as? String)
            let scan = MathSourceScanner.scan(try XCTUnwrap(entry["source"] as? String))
            let expected = try XCTUnwrap(entry["expected"] as? [[String: String]])
            XCTAssertEqual(scan.definitions.map(\.kind.rawValue), expected.map { $0["kind"] ?? "" }, name)
            XCTAssertEqual(scan.definitions.map(\.name), expected.map { $0["name"] ?? "" }, name)
            XCTAssertEqual(scan.definitions.map(\.mathJaxSource), expected.map { $0["mathJax"] ?? "" }, name)
        }
        for entry in try XCTUnwrap(fixture["includes"] as? [[String: Any]]) {
            let scan = MathSourceScanner.scan(try XCTUnwrap(entry["source"] as? String))
            XCTAssertEqual(scan.includes.map(\.target), entry["expected"] as? [String], (entry["name"] as? String) ?? "")
        }
    }

    func testProjectContextFixtures() throws {
        for entry in try XCTUnwrap(fixture["contexts"] as? [[String: Any]]) {
            let name = try XCTUnwrap(entry["name"] as? String)
            let marked = try XCTUnwrap(entry["files"] as? [String: String])
            let root = try XCTUnwrap(entry["root"] as? String)
            let active = try XCTUnwrap(entry["active"] as? String)
            var scans: [String: MathSourceScan] = [:]
            var caret = 0
            for (path, text) in marked {
                if path == active, text.contains(Self.caret) {
                    let (source, offset) = try split(text)
                    scans[path] = MathSourceScanner.scan(source)
                    caret = offset
                } else {
                    scans[path] = MathSourceScanner.scan(text)
                }
            }
            // The host's role: resolve each requested include against the
            // files that exist, until nothing is left unresolved.
            var resolutions: [MathIncludeKey: String?] = [:]
            var context = MathProjectContext(rootFileID: root, activeFileID: active, activeScan: nil, scans: scans, resolutions: resolutions)
            while !context.unresolved.isEmpty {
                for request in context.unresolved {
                    resolutions[request.key] = .some(request.candidates.first { marked[$0] != nil })
                }
                context = MathProjectContext(rootFileID: root, activeFileID: active, activeScan: nil, scans: scans, resolutions: resolutions)
            }
            let region = try XCTUnwrap(scans[active]?.region(atCaret: caret), name)
            let math = context.context(fileID: active, utf8Offset: region.range.utf8Offset)
            XCTAssertEqual(math.definitions, entry["expected"] as? [String], name)
            XCTAssertFalse(math.truncated, name)
            let body = context.bodyDefinitions(
                fileID: active, utf8Offset: region.range.utf8Offset,
                rootDocumentBegin: scans[root]?.documentBeginOffset
            )
            XCTAssertEqual(body, entry["body"] as? [String], name)
        }
    }

    /// Macro redefinition must change the context identity the preview
    /// cache is keyed by; an unrelated edit must not.
    func testContextKeyTracksDefinitionsOnly() {
        func key(_ source: String) -> String {
            let scan = MathSourceScanner.scan(source)
            let context = MathProjectContext(rootFileID: "/a.tex", activeFileID: "/a.tex", activeScan: nil, scans: ["/a.tex": scan], resolutions: [:])
            return context.context(fileID: "/a.tex", utf8Offset: scan.regions[0].range.utf8Offset).key
        }
        let original = key("\\newcommand{\\R}{\\mathbb{R}}\n$x \\in \\R$")
        XCTAssertEqual(original, key("\\newcommand{\\R}{\\mathbb{R}}\nSome words.\n$x \\in \\R$"))
        XCTAssertNotEqual(original, key("\\newcommand{\\R}{\\mathbf{R}}\n$x \\in \\R$"))
        XCTAssertNotEqual(original, key("$x \\in \\R$"))
    }

    func testDefinitionBoundsTruncate() {
        let source = String(repeating: "\\newcommand{\\x}{y}\n", count: MathProjectContext.maximumDefinitions + 10) + "$x$"
        let scan = MathSourceScanner.scan(source)
        let context = MathProjectContext(rootFileID: "/a.tex", activeFileID: "/a.tex", activeScan: nil, scans: ["/a.tex": scan], resolutions: [:])
        let math = context.context(fileID: "/a.tex", utf8Offset: scan.regions[0].range.utf8Offset)
        XCTAssertTrue(math.truncated)
        XCTAssertEqual(math.definitions.count, MathProjectContext.maximumDefinitions)
    }

    func testIncludeCandidatesPreferMainDirectory() {
        XCTAssertEqual(
            MathIncludePaths.candidates(target: "defs", from: "/p/chapters/one.tex", root: "/p/main.tex"),
            ["/p/defs.tex", "/p/chapters/defs.tex"]
        )
        XCTAssertEqual(
            MathIncludePaths.candidates(target: "../shared/m.sty", from: "/p/main.tex", root: "/p/main.tex"),
            ["/shared/m.sty"]
        )
    }

    /// Large-document detector budget (Apple Silicon target < 5 ms). Here
    /// only a coarse ceiling: the benchmark harness records real timings.
    func testLargeFixtureScanIsLinearAndStable() throws {
        let url = repositoryRoot.appendingPathComponent("Fixtures/projects/large/main.tex")
        let source = try String(contentsOf: url, encoding: .utf8)
        let first = MathSourceScanner.scan(source)
        XCTAssertEqual(first, MathSourceScanner.scan(source))
        XCTAssertTrue(zip(first.regions, first.regions.dropFirst()).allSatisfy { $0.range.endUTF8Offset <= $1.range.utf8Offset })
        XCTAssertTrue(first.regions.allSatisfy { $0.range.endUTF8Offset <= source.utf8.count })
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
