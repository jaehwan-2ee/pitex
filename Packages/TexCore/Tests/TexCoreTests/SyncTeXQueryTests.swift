import Foundation
import SyncTeXCore
import XCTest

final class SyncTeXQueryTests: XCTestCase {
    private let root = "/workspace/multifile"
    private let outputHash = "sha256:8d5c2f1a"

    func testForwardSelectsOnlyExactChildSourceAndPDF() throws {
        let revision = try makeRevision()
        let document = try parseFixture("forward-output.txt", revision: revision)
        XCTAssertEqual(document.metadata, SyncTeXQueryMetadata(version: 1, fingerprint: 7_829_367))
        XCTAssertEqual(document.candidates.count, 3)

        let source = try SourceLocation(
            path: NormalizedSourcePath("sections/details.tex"),
            line: 3,
            column: 2
        )
        let query = ForwardSyncQuery(
            revision: revision,
            source: source,
            expectedPDF: try NormalizedSourcePath("build/main.pdf")
        )
        let selected = try ExactSyncTeXQuerySelector.forward(
            document.candidates,
            query: query,
            outputHash: outputHash
        )
        XCTAssertEqual(selected.source.path.value, "sections/details.tex")
        XCTAssertEqual(selected.pdf.page, 2)
        XCTAssertEqual(selected.pdf.point, try PDFPoint(x: 144.25, y: 221.75))
        XCTAssertEqual(selected.h, 144.25)
        XCTAssertEqual(selected.v, 221.75)
        XCTAssertEqual(selected.width, 275.5)
        XCTAssertEqual(selected.height, 7.25)
    }

    func testInverseReturnsExactChildFileAndLine() throws {
        let revision = try makeRevision()
        let document = try parseFixture("inverse-output.txt", revision: revision)
        let pdf = try PDFLocation(
            pdfPath: NormalizedSourcePath("build/main.pdf"),
            page: 2,
            point: PDFPoint(x: 144.25, y: 221.75)
        )
        let selected = try ExactSyncTeXQuerySelector.inverse(
            document.candidates,
            query: InverseSyncQuery(revision: revision, pdf: pdf),
            outputHash: outputHash
        )
        XCTAssertEqual(selected.source.path.value, "sections/details.tex")
        XCTAssertEqual(selected.source.line, 3)
        XCTAssertEqual(selected.source.column, 2)
    }

    func testInverseEpsilonIdentifiesOnlyAUniqueBlockAndNeverNearestGuesses() throws {
        let revision = try makeRevision()
        let candidates = try parseFixture("inverse-output.txt", revision: revision).candidates
        let pdf = try PDFLocation(
            pdfPath: NormalizedSourcePath("build/main.pdf"),
            page: 2,
            point: PDFPoint(x: 144.25005, y: 221.75005)
        )
        let query = InverseSyncQuery(revision: revision, pdf: pdf)
        let selected = try ExactSyncTeXQuerySelector.inverse(
            candidates,
            query: query,
            outputHash: outputHash,
            coordinateEpsilon: 0.0001
        )
        XCTAssertEqual(selected.source.path.value, "sections/details.tex")
        XCTAssertThrowsError(try ExactSyncTeXQuerySelector.inverse(
            candidates,
            query: query,
            outputHash: outputHash,
            coordinateEpsilon: 0.01
        )) { XCTAssertEqual($0 as? SyncTeXQueryError, .ambiguousMatch(count: 2)) }

        let between = try PDFLocation(
            pdfPath: NormalizedSourcePath("build/main.pdf"),
            page: 2,
            point: PDFPoint(x: 144.2507, y: 221.7507)
        )
        XCTAssertThrowsError(try ExactSyncTeXQuerySelector.inverse(
            candidates,
            query: InverseSyncQuery(revision: revision, pdf: between),
            outputHash: outputHash,
            coordinateEpsilon: 0.0001
        )) { XCTAssertEqual($0 as? SyncTeXQueryError, .noMatch) }
    }

    func testDuplicateExactCandidatesAreAmbiguous() throws {
        let revision = try makeRevision()
        let document = try parseFixture("forward-output.txt", revision: revision)
        let candidate = document.candidates[2]
        let query = ForwardSyncQuery(
            revision: revision,
            source: candidate.source,
            expectedPDF: candidate.pdf.pdfPath
        )
        XCTAssertThrowsError(try ExactSyncTeXQuerySelector.forward(
            document.candidates + [candidate],
            query: query,
            outputHash: outputHash
        )) { XCTAssertEqual($0 as? SyncTeXQueryError, .ambiguousMatch(count: 2)) }
    }

    func testRevisionAndOutputHashMustBothMatch() throws {
        let actualRevision = try makeRevision()
        let document = try parseFixture("forward-output.txt", revision: actualRevision)
        let candidate = document.candidates[0]
        let staleRevision = try SyncTeXRevision(buildID: "build-43", fingerprint: 7_829_367)
        let staleQuery = ForwardSyncQuery(
            revision: staleRevision,
            source: candidate.source,
            expectedPDF: candidate.pdf.pdfPath
        )
        XCTAssertThrowsError(try ExactSyncTeXQuerySelector.forward(
            document.candidates,
            query: staleQuery,
            outputHash: outputHash
        )) { XCTAssertEqual($0 as? SyncTeXQueryError, .staleResult) }

        let currentQuery = ForwardSyncQuery(
            revision: actualRevision,
            source: candidate.source,
            expectedPDF: candidate.pdf.pdfPath
        )
        XCTAssertThrowsError(try ExactSyncTeXQuerySelector.forward(
            document.candidates,
            query: currentQuery,
            outputHash: "sha256:different"
        )) { XCTAssertEqual($0 as? SyncTeXQueryError, .staleResult) }

        let staleMetadata = try fixture("forward-output.txt").replacingOccurrences(
            of: "SyncTeX Fingerprint:7829367",
            with: "SyncTeX Fingerprint:7829368"
        )
        XCTAssertThrowsError(try SyncTeXQueryParser.parse(
            staleMetadata,
            projectRoot: root,
            binding: makeBinding(revision: actualRevision)
        )) { XCTAssertEqual($0 as? SyncTeXQueryError, .staleResult) }
    }

    func testLocaleDecimalAndMalformedNumericFieldsAreRejected() throws {
        let fixture = try fixture("forward-output.txt")
        let binding = try makeBinding(revision: makeRevision())
        for malformed in [
            fixture.replacingOccurrences(of: "x:133.768341", with: "x:133,768341"),
            fixture.replacingOccurrences(of: "W:343.711060", with: "W:nan"),
            fixture.replacingOccurrences(of: "Column:0", with: "Column:-1"),
            fixture.replacingOccurrences(of: "SyncTeX Fingerprint:7829367", with: "SyncTeX Fingerprint:7 829 367")
        ] {
            XCTAssertThrowsError(try SyncTeXQueryParser.parse(
                malformed,
                projectRoot: root,
                binding: binding
            )) { XCTAssertNotNil($0 as? SyncTeXQueryError) }
        }
    }

    func testOutsideRootAndLexicalEscapePathsAreRejected() throws {
        let fixture = try fixture("forward-output.txt")
        let binding = try makeBinding(revision: makeRevision())
        let outside = fixture.replacingOccurrences(
            of: "/workspace/multifile/sections/intro.tex",
            with: "/workspace/other/intro.tex"
        )
        XCTAssertThrowsError(try SyncTeXQueryParser.parse(outside, projectRoot: root, binding: binding)) {
            guard let error = $0 as? SyncTeXQueryError, case .pathOutsideRoot = error else {
                return XCTFail("Expected pathOutsideRoot, got \($0)")
            }
        }
        let lexicalEscape = fixture.replacingOccurrences(
            of: "/workspace/multifile/sections/intro.tex",
            with: "/workspace/multifile/sections/../intro.tex"
        )
        XCTAssertThrowsError(try SyncTeXQueryParser.parse(lexicalEscape, projectRoot: root, binding: binding)) {
            guard let error = $0 as? SyncTeXQueryError, case .pathOutsideRoot = error else {
                return XCTFail("Expected pathOutsideRoot, got \($0)")
            }
        }
    }

    func testLineAndPageBoundariesAreStrict() throws {
        let fixture = try fixture("forward-output.txt")
        let binding = try makeBinding(revision: makeRevision())
        for malformed in [
            fixture.replacingOccurrences(of: "Line:4", with: "Line:0"),
            fixture.replacingOccurrences(of: "Page:1", with: "Page:0"),
            fixture.replacingOccurrences(of: "Line:4", with: "Line:9223372036854775808")
        ] {
            XCTAssertThrowsError(try SyncTeXQueryParser.parse(malformed, projectRoot: root, binding: binding)) {
                guard let error = $0 as? SyncTeXQueryError, case .malformed = error else {
                    return XCTFail("Expected malformed, got \($0)")
                }
            }
        }
    }

    func testNoMatchDoesNotFallBackToBasenamePageOrNearestPoint() throws {
        let revision = try makeRevision()
        let document = try parseFixture("forward-output.txt", revision: revision)
        let source = try SourceLocation(path: NormalizedSourcePath("other/details.tex"), line: 3, column: 2)
        let query = ForwardSyncQuery(
            revision: revision,
            source: source,
            expectedPDF: try NormalizedSourcePath("build/main.pdf")
        )
        XCTAssertThrowsError(try ExactSyncTeXQuerySelector.forward(
            document.candidates,
            query: query,
            outputHash: outputHash
        )) { XCTAssertEqual($0 as? SyncTeXQueryError, .noMatch) }
    }

    func testParsingIsDeterministic() throws {
        let revision = try makeRevision()
        let fixture = try fixture("inverse-output.txt")
        let binding = try makeBinding(revision: revision)
        let first = try SyncTeXQueryParser.parse(fixture, projectRoot: root, binding: binding)
        let second = try SyncTeXQueryParser.parse(fixture, projectRoot: root, binding: binding)
        XCTAssertEqual(first, second)
        XCTAssertEqual(first.candidates.map(\.source.path.value), [
            "sections/intro.tex", "sections/details.tex", "main.tex"
        ])
    }

    // `synctex view` result blocks carry only Output/Page/geometry; the runner
    // injects Input/Line/Column from the query. `synctex edit` blocks carry
    // Input/Line/Column (Column often -1, normalized to the query value) while
    // Page/x/y/h/v/W/H/Output are injected. Input paths may contain `.`
    // components that must canonicalize under the project root.
    func testNormalizedRecordsFromRealCLIShape() throws {
        let revision = try makeRevision()
        let binding = try makeBinding(revision: revision)
        let viewNormalized = """
            SyncTeX Version:1
            SyncTeX Fingerprint:7829367
            SyncTeX result begin
            Input:/workspace/multifile/main.tex
            Line:4
            Column:0
            Output:/workspace/multifile/build/main.pdf
            Page:1
            x:133.768356
            y:167.710464
            h:133.768356
            v:167.710464
            W:343.711060
            H:9.843078
            SyncTeX result end
            """
        let forwardDoc = try SyncTeXQueryParser.parse(viewNormalized, projectRoot: root, binding: binding)
        let forward = try ExactSyncTeXQuerySelector.forward(
            forwardDoc.candidates,
            query: ForwardSyncQuery(
                revision: revision,
                source: SourceLocation(path: NormalizedSourcePath("main.tex"), line: 4, column: 0),
                expectedPDF: NormalizedSourcePath("build/main.pdf")
            ),
            outputHash: outputHash
        )
        XCTAssertEqual(forward.pdf.page, 1)
        XCTAssertEqual(forward.pdf.point, try PDFPoint(x: 133.768356, y: 167.710464))

        let editNormalized = """
            SyncTeX Version:1
            SyncTeX Fingerprint:7829367
            SyncTeX result begin
            Input:/workspace/multifile/./sections/details.tex
            Line:6
            Column:0
            Output:/workspace/multifile/build/main.pdf
            Page:1
            x:150
            y:700
            h:150
            v:700
            W:0
            H:0
            SyncTeX result end
            """
        let inverseDoc = try SyncTeXQueryParser.parse(editNormalized, projectRoot: root, binding: binding)
        let inverse = try ExactSyncTeXQuerySelector.inverse(
            inverseDoc.candidates,
            query: InverseSyncQuery(
                revision: revision,
                pdf: PDFLocation(
                    pdfPath: NormalizedSourcePath("build/main.pdf"),
                    page: 1,
                    point: try PDFPoint(x: 150, y: 700)
                )
            ),
            outputHash: outputHash,
            coordinateEpsilon: 2
        )
        XCTAssertEqual(inverse.source.path.value, "sections/details.tex")
        XCTAssertEqual(inverse.source.line, 6)
    }

    private func makeRevision() throws -> SyncTeXRevision {
        try SyncTeXRevision(buildID: "build-42", fingerprint: 7_829_367)
    }

    private func makeBinding(revision: SyncTeXRevision) throws -> SyncTeXOutputBinding {
        try SyncTeXOutputBinding(revision: revision, outputHash: outputHash)
    }

    private func parseFixture(_ name: String, revision: SyncTeXRevision) throws -> SyncTeXQueryDocument {
        try SyncTeXQueryParser.parse(
            fixture(name),
            projectRoot: root,
            binding: makeBinding(revision: revision)
        )
    }

    private func fixture(_ name: String) throws -> String {
        var repository = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { repository.deleteLastPathComponent() }
        let url = repository.appendingPathComponent("Fixtures/synctex").appendingPathComponent(name)
        return try String(contentsOf: url, encoding: .utf8)
    }
}
