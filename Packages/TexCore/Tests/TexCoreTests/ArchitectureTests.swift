import DocumentSessionCore
import ProjectCore
import TexDomain
import XCTest

final class ArchitectureTests: XCTestCase {
    func testIdentifiersAndPathsValidateAndNormalize() throws {
        XCTAssertThrowsError(try StableProjectID(rawValue: "")) { error in
            XCTAssertEqual(error as? TexDomainValidationError, .emptyIdentifier)
        }
        XCTAssertThrowsError(try StableDocumentID(rawValue: "chapter one")) { error in
            XCTAssertEqual(error as? TexDomainValidationError, .invalidIdentifierCharacter)
        }
        XCTAssertThrowsError(try NormalizedRelativePath(rawValue: "/tmp/main.tex")) { error in
            XCTAssertEqual(error as? TexDomainValidationError, .absolutePath)
        }
        XCTAssertThrowsError(try NormalizedRelativePath(rawValue: "../main.tex")) { error in
            XCTAssertEqual(error as? TexDomainValidationError, .pathEscapesRoot)
        }

        let path = try NormalizedRelativePath(
            rawValue: "sources//./chapters/../main.tex"
        )
        XCTAssertEqual(path.rawValue, "sources/main.tex")
    }

    func testGraphDiagnosticsAreDeterministic() throws {
        let projectID = try StableProjectID(rawValue: "project-1")
        let a = try StableDocumentID(rawValue: "a")
        let b = try StableDocumentID(rawValue: "b")
        let c = try StableDocumentID(rawValue: "c")
        let missing = try StableDocumentID(rawValue: "missing")

        let graph = try ProjectGraph(
            projectID: projectID,
            files: [
                ProjectFile(
                    documentID: c,
                    path: try NormalizedRelativePath(rawValue: "c.tex")
                ),
                ProjectFile(
                    documentID: b,
                    path: try NormalizedRelativePath(rawValue: "b.tex")
                ),
                ProjectFile(
                    documentID: a,
                    path: try NormalizedRelativePath(rawValue: "a.tex")
                )
            ],
            includeEdges: [
                IncludeEdge(source: b, target: a),
                IncludeEdge(source: a, target: missing),
                IncludeEdge(source: c, target: a),
                IncludeEdge(source: a, target: b)
            ]
        )

        let expected: [ProjectGraphDiagnostic] = [
            .missingReference(source: a, target: missing, missing: missing),
            .includeCycle(documents: [a, b])
        ]
        XCTAssertEqual(graph.diagnostics(), expected)
        XCTAssertEqual(graph.diagnostics(), graph.diagnostics())
        XCTAssertEqual(graph.files.map(\.documentID), [a, b, c])
    }

    func testSessionRevisionsAreMonotonicAndSaveStateTracksContent() async throws {
        let session = try makeSession(initialText: "one")
        let initial = await session.snapshot()
        XCTAssertEqual(initial.revision, 0)
        XCTAssertEqual(initial.saveState, .clean)

        let edited = try await session.apply(
            .replaceText("two"),
            expectedRevision: initial.revision
        )
        XCTAssertEqual(edited.revision, 1)
        XCTAssertEqual(edited.saveState, .dirty)

        let saved = try await session.apply(
            .commitSave(writtenDiskHash: edited.contentHash),
            expectedRevision: edited.revision
        )
        XCTAssertEqual(saved.revision, 2)
        XCTAssertEqual(saved.saveState, .clean)
        XCTAssertEqual(saved.diskBaselineHash, edited.contentHash)
    }

    func testStaleMutationIsRejectedWithoutChangingSnapshot() async throws {
        let session = try makeSession(initialText: "one")
        _ = try await session.apply(.replaceText("two"), expectedRevision: 0)

        do {
            _ = try await session.apply(.replaceText("stale"), expectedRevision: 0)
            XCTFail("Expected a stale revision error")
        } catch {
            XCTAssertEqual(
                error as? DocumentSessionError,
                .staleRevision(expected: 0, actual: 1)
            )
        }

        let snapshot = await session.snapshot()
        XCTAssertEqual(snapshot.revision, 1)
        XCTAssertEqual(snapshot.text, "two")
    }

    func testLocalMutationPreservesRecordedConflict() async throws {
        let session = try makeSession(initialText: "disk text")
        let observedDiskHash = DiskContentHash.hashing("changed elsewhere")
        let conflicted = try await session.apply(
            .recordExternalChange(observedDiskHash: observedDiskHash),
            expectedRevision: 0
        )
        let expectedConflict = DocumentConflict.externalModification(
            baseline: DiskContentHash.hashing("disk text"),
            observedDisk: observedDiskHash
        )
        XCTAssertEqual(conflicted.conflict, expectedConflict)
        XCTAssertEqual(conflicted.saveState, .conflicted)

        let edited = try await session.apply(
            .replaceText("my preserved work"),
            expectedRevision: conflicted.revision
        )
        XCTAssertEqual(edited.text, "my preserved work")
        XCTAssertEqual(edited.conflict, expectedConflict)
        XCTAssertEqual(edited.saveState, .conflicted)
    }

    func testSaveCollisionVariantRetainsLocalContent() async throws {
        let session = try makeSession(initialText: "baseline")
        let edited = try await session.apply(
            .replaceText("local edit"),
            expectedRevision: 0
        )
        let observedDiskHash = DiskContentHash.hashing("remote edit")
        let conflicted = try await session.apply(
            .recordSaveConflict(observedDiskHash: observedDiskHash),
            expectedRevision: edited.revision
        )

        XCTAssertEqual(conflicted.text, "local edit")
        XCTAssertEqual(
            conflicted.conflict,
            .saveCollision(
                baseline: DiskContentHash.hashing("baseline"),
                observedDisk: observedDiskHash
            )
        )
        XCTAssertEqual(conflicted.saveState, .conflicted)
    }

    private func makeSession(initialText: String) throws -> DocumentSession {
        let file = ProjectFile(
            documentID: try StableDocumentID(rawValue: "main"),
            path: try NormalizedRelativePath(rawValue: "main.tex")
        )
        return DocumentSession(file: file, initialText: initialText)
    }
}
