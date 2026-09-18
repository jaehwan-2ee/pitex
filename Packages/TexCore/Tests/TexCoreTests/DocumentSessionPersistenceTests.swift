import DocumentSessionCore
import Foundation
import ProjectCore
import TexDomain
import XCTest

final class DocumentSessionPersistenceTests: XCTestCase {
    func testCanonicalProjectAndRelativePathAliasesCoalesce() async throws {
        let root = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let nested = root.appendingPathComponent("sources", isDirectory: true)
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)

        let registry = DocumentSessionRegistry()
        let canonicalFile = ProjectFile(
            documentID: try StableDocumentID(rawValue: "main"),
            path: try NormalizedRelativePath(rawValue: "sources/main.tex")
        )
        let aliasedFile = ProjectFile(
            documentID: canonicalFile.documentID,
            path: try NormalizedRelativePath(rawValue: "sources/./draft/../main.tex")
        )

        let first = try await registry.open(
            projectRoot: root,
            file: canonicalFile,
            initialText: "first"
        )
        let second = try await registry.open(
            projectRoot: nested.appendingPathComponent(".."),
            file: aliasedFile,
            initialText: "ignored duplicate"
        )
        XCTAssertTrue(first === second)
        let openCount = await registry.count
        XCTAssertEqual(openCount, 1)
        let didClose = try await registry.close(projectRoot: root, session: first)
        XCTAssertTrue(didClose)

        let reopened = try await registry.open(
            projectRoot: root,
            file: canonicalFile,
            initialText: "reopened"
        )
        XCTAssertFalse(first === reopened)
        let reopenedSnapshot = await reopened.snapshot()
        XCTAssertEqual(reopenedSnapshot.text, "reopened")
        let staleClose = try await registry.close(projectRoot: root, session: first)
        XCTAssertFalse(staleClose)
        let registered = try await registry.session(
            projectRoot: root,
            path: canonicalFile.path
        )
        XCTAssertTrue(registered === reopened)
    }

    func testNonNativeMutationRequiresAndChecksExpectedRevision() async throws {
        let session = try makeSession(initialText: "zero")

        do {
            _ = try await session.apply(
                OriginatedDocumentMutation(
                    mutation: .replaceText("unversioned"),
                    origin: .aiProposal
                )
            )
            XCTFail("Expected revision metadata to be required")
        } catch {
            XCTAssertEqual(
                error as? OriginatedDocumentMutationError,
                .expectedRevisionRequired(origin: .aiProposal)
            )
        }

        _ = try await session.apply(
            OriginatedDocumentMutation(
                mutation: .replaceText("one"),
                origin: .textKit
            )
        )
        do {
            _ = try await session.apply(
                OriginatedDocumentMutation(
                    mutation: .replaceText("stale"),
                    origin: .recovery,
                    expectedRevision: 0
                )
            )
            XCTFail("Expected stale mutation rejection")
        } catch {
            XCTAssertEqual(
                error as? DocumentSessionError,
                .staleRevision(expected: 0, actual: 1)
            )
        }
        let snapshot = await session.snapshot()
        XCTAssertEqual(snapshot.text, "one")
        XCTAssertEqual(snapshot.contentHash, .hashing("one"))
    }

    func testExternalCollisionPreservesLocalAndObservedDiskContent() throws {
        let root = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let fileURL = root.appendingPathComponent("main.tex")
        try "baseline".write(to: fileURL, atomically: true, encoding: .utf8)
        let baselineHash = DiskContentHash.hashing("baseline")
        try "external edit".write(to: fileURL, atomically: true, encoding: .utf8)

        let outcome = FoundationAtomicDocumentStore().save(
            text: "local edit",
            to: fileURL,
            expectedBaselineHash: baselineHash
        )
        guard case let .staleBaseline(conflict) = outcome else {
            return XCTFail("Expected stale baseline, got \(outcome)")
        }
        XCTAssertEqual(conflict.local.text, "local edit")
        XCTAssertEqual(conflict.local.hash, .hashing("local edit"))
        XCTAssertEqual(conflict.observedDisk?.text, "external edit")
        XCTAssertEqual(conflict.observedDisk?.hash, .hashing("external edit"))
        XCTAssertEqual(try String(contentsOf: fileURL, encoding: .utf8), "external edit")
    }

    func testAtomicSuccessfulSaveReloadsExactContentAndHash() throws {
        let root = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let fileURL = root.appendingPathComponent("main.tex")
        try "baseline".write(to: fileURL, atomically: true, encoding: .utf8)
        let store = FoundationAtomicDocumentStore()

        let save = store.save(
            text: "saved π content",
            to: fileURL,
            expectedBaselineHash: .hashing("baseline")
        )
        XCTAssertEqual(
            save,
            .saved(PersistedDocument(text: "saved π content"))
        )
        XCTAssertEqual(
            store.load(from: fileURL, expectedBaselineHash: .hashing("saved π content")),
            .loaded(PersistedDocument(text: "saved π content"))
        )
        XCTAssertEqual(try Data(contentsOf: fileURL), Data("saved π content".utf8))
    }

    func testInterruptedWriteRemovesOnlyItsTemporaryFile() throws {
        let root = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let fileURL = root.appendingPathComponent("main.tex")
        let unrelated = root.appendingPathComponent("keep.tmp")
        try Data([0xFF]).write(to: fileURL)
        try "unrelated".write(to: unrelated, atomically: true, encoding: .utf8)

        let outcome = FoundationAtomicDocumentStore().save(
            text: "local",
            to: fileURL,
            expectedBaselineHash: DiskContentHash(rawValue: 1)
        )
        guard case .interruptedWrite = outcome else {
            return XCTFail("Expected interrupted write, got \(outcome)")
        }

        XCTAssertEqual(try Data(contentsOf: fileURL), Data([0xFF]))
        XCTAssertEqual(try String(contentsOf: unrelated, encoding: .utf8), "unrelated")
        let names = try FileManager.default.contentsOfDirectory(atPath: root.path)
        XCTAssertEqual(Set(names), Set(["main.tex", "keep.tmp"]))
    }

    func testWrongBaselineNeverOverwritesOrLosesEitherVersion() throws {
        let root = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let fileURL = root.appendingPathComponent("chapter.tex")
        try "disk truth".write(to: fileURL, atomically: true, encoding: .utf8)

        let outcome = FoundationAtomicDocumentStore().save(
            text: "valuable local work",
            to: fileURL,
            expectedBaselineHash: .hashing("old baseline")
        )
        guard case let .staleBaseline(conflict) = outcome else {
            return XCTFail("Expected stale baseline, got \(outcome)")
        }
        XCTAssertEqual(conflict.local.text, "valuable local work")
        XCTAssertEqual(conflict.local.hash, .hashing("valuable local work"))
        XCTAssertEqual(conflict.observedDisk?.text, "disk truth")
        XCTAssertEqual(conflict.observedDisk?.hash, .hashing("disk truth"))
        XCTAssertEqual(try String(contentsOf: fileURL, encoding: .utf8), "disk truth")
    }

    private func makeTemporaryDirectory() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("texspark-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func makeSession(initialText: String) throws -> DocumentSession {
        DocumentSession(
            file: ProjectFile(
                documentID: try StableDocumentID(rawValue: "main"),
                path: try NormalizedRelativePath(rawValue: "main.tex")
            ),
            initialText: initialText
        )
    }
}
