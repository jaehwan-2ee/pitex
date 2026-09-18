import Foundation
import LanguageCore
import XCTest

final class LargeFixtureTests: XCTestCase {
    private var fixtureURL: URL {
        repositoryRoot.appendingPathComponent("Fixtures/projects/large/main.tex")
    }

    func testLargeFixtureHasDeterministicOriginalStressCharacteristics() throws {
        let bytes = try Data(contentsOf: fixtureURL)
        let source = try XCTUnwrap(String(data: bytes, encoding: .utf8))

        XCTAssertGreaterThanOrEqual(bytes.count, 1_048_576)
        XCTAssertTrue(source.contains("한국어"))
        XCTAssertTrue(source.contains("日本語"))
        XCTAssertTrue(source.contains("👩🏽‍💻"))
        XCTAssertTrue(source.contains("e\u{301}"))
        XCTAssertTrue(source.contains("\\begin{verbatim}"))
        XCTAssertTrue(source.contains("% block 0699"))
        XCTAssertGreaterThanOrEqual(source.components(separatedBy: "\\section{").count - 1, 700)
        XCTAssertGreaterThanOrEqual(source.components(separatedBy: "\\label{").count - 1, 700)
        XCTAssertGreaterThanOrEqual(source.components(separatedBy: "\\ref{").count - 1, 700)
        XCTAssertGreaterThan(
            try XCTUnwrap(source.split(separator: "\n").map { $0.utf8.count }.max()),
            1_000
        )
    }

    func testLexerIndexRevisionReplacementAndCoordinatesAreDeterministicAndBounded() throws {
        let source = try String(contentsOf: fixtureURL, encoding: .utf8)

        let firstTokens = DeterministicTeXLexer.tokenize(source)
        let secondTokens = DeterministicTeXLexer.tokenize(source)
        XCTAssertEqual(firstTokens, secondTokens)
        XCTAssertGreaterThan(firstTokens.count, 5_000)
        XCTAssertLessThan(firstTokens.count, source.utf8.count)
        XCTAssertEqual(firstTokens.last?.range.endUTF8Offset, source.utf8.count)
        XCTAssertTrue(firstTokens.allSatisfy { $0.range.endUTF8Offset <= source.utf8.count })

        let initial = try LanguageFileSnapshot(sourceID: "large/main.tex", revision: 1, source: source)
        XCTAssertGreaterThanOrEqual(initial.outline.count, 1_400)
        XCTAssertGreaterThanOrEqual(initial.references.filter { $0.kind == .label }.count, 1_400)
        XCTAssertGreaterThanOrEqual(initial.references.filter { $0.kind == .reference }.count, 1_400)
        XCTAssertLessThan(initial.tokens.count + initial.outline.count + initial.references.count, source.utf8.count)

        let index = try ProjectLanguageIndex(snapshots: [initial])
        let replacementSource = source.replacingOccurrences(of: "unique-block-0699", with: "unique-block-0699-revised")
        let replacement = try LanguageFileSnapshot(sourceID: "large/main.tex", revision: 2, source: replacementSource)
        let replaced = try index.replacing(replacement)
        XCTAssertEqual(replaced.snapshots.count, 1)
        XCTAssertEqual(replaced.snapshots["large/main.tex"]?.revision.revision, 2)
        XCTAssertTrue(replaced.snapshots["large/main.tex"]?.source.contains("0699-revised") == true)
        XCTAssertThrowsError(try replaced.replacing(initial))

        let sentinel = try XCTUnwrap(source.range(of: "[0699:한:日:👩🏽‍💻:e\u{301}]"))
        let sentinelUTF8 = source[..<sentinel.lowerBound].utf8.count
        let map = UnicodeCoordinateMap(source)
        let position = try map.position(forUTF8Offset: sentinelUTF8)
        XCTAssertEqual(try map.utf8Offset(for: position), sentinelUTF8)
        XCTAssertEqual(try map.utf16Offset(forUTF8Offset: sentinelUTF8), source[..<sentinel.lowerBound].utf16.count)
        XCTAssertEqual(map.utf8Count, source.utf8.count)
        XCTAssertEqual(map.utf16Count, source.utf16.count)
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
