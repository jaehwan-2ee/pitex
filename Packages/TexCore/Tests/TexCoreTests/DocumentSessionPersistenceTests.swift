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

    func testCommitSaveAtCurrentRevisionBehavesLikeCommitMutation() async throws {
        let session = try makeSession(initialText: "base")
        let edited = try await session.apply(.replaceText("edit"), expectedRevision: 0)

        let committed = try await session.commitSave(
            writtenDiskHash: .hashing("edit"),
            writtenRevision: edited.revision
        )
        XCTAssertEqual(committed.revision, edited.revision + 1)
        XCTAssertEqual(committed.saveState, .clean)
        XCTAssertEqual(committed.diskBaselineHash, .hashing("edit"))
        XCTAssertNil(committed.conflict)

        do {
            _ = try await session.commitSave(
                writtenDiskHash: .hashing("not the content"),
                writtenRevision: committed.revision
            )
            XCTFail("Expected a saved-content hash mismatch")
        } catch {
            XCTAssertEqual(
                error as? DocumentSessionError,
                .savedContentHashMismatch(
                    expected: .hashing("edit"),
                    written: .hashing("not the content")
                )
            )
        }
    }

    func testCommitSaveAfterInterleavedEditMovesBaselineAndStaysDirty() async throws {
        let session = try makeSession(initialText: "v0")
        // Revision 1's text is what the in-flight write put on disk.
        let written = try await session.apply(.replaceText("v1"), expectedRevision: 0)
        // A keystroke lands while the file is still being written.
        let edited = try await session.apply(
            .replaceText("v2"),
            expectedRevision: written.revision
        )

        let committed = try await session.commitSave(
            writtenDiskHash: .hashing(written.text),
            writtenRevision: written.revision
        )
        XCTAssertEqual(committed.revision, edited.revision + 1)
        XCTAssertEqual(committed.diskBaselineHash, .hashing("v1"))
        XCTAssertEqual(committed.contentHash, .hashing("v2"))
        XCTAssertEqual(committed.text, "v2")
        XCTAssertEqual(committed.saveState, .dirty)
        XCTAssertNil(committed.conflict)
    }

    func testCommitSaveLosesToAConflictRecordedMidWrite() async throws {
        let session = try makeSession(initialText: "base")
        let written = try await session.apply(.replaceText("edit"), expectedRevision: 0)
        let observed = DiskContentHash.hashing("external")
        let conflicted = try await session.apply(
            .recordExternalChange(observedDiskHash: observed),
            expectedRevision: written.revision
        )

        let committed = try await session.commitSave(
            writtenDiskHash: .hashing(written.text),
            writtenRevision: written.revision
        )
        // Everything untouched: no baseline move, no revision bump.
        XCTAssertEqual(committed, conflicted)
        XCTAssertEqual(committed.saveState, .conflicted)
        XCTAssertEqual(committed.diskBaselineHash, .hashing("base"))
    }

    func testCommitSaveWithFutureRevisionThrows() async throws {
        let session = try makeSession(initialText: "base")
        let snapshot = await session.snapshot()
        do {
            _ = try await session.commitSave(
                writtenDiskHash: .hashing("base"),
                writtenRevision: snapshot.revision + 1
            )
            XCTFail("Expected a stale revision error")
        } catch {
            XCTAssertEqual(
                error as? DocumentSessionError,
                .staleRevision(expected: 1, actual: 0)
            )
        }
    }

    func testApplyTextNeutralAppliesConflictsAtTheCurrentRevision() async throws {
        let session = try makeSession(initialText: "base")
        _ = try await session.apply(.replaceText("edit"), expectedRevision: 0)
        // A second edit lands between the caller's snapshot and the
        // conflict record — expectedRevision-based apply would throw here.
        _ = try await session.apply(.replaceText("edit again"), expectedRevision: 1)
        let observed = DiskContentHash.hashing("external")

        let conflicted = try await session.applyTextNeutral(
            .recordExternalChange(observedDiskHash: observed)
        )
        let twin = try makeSession(initialText: "base")
        _ = try await twin.apply(.replaceText("edit"), expectedRevision: 0)
        _ = try await twin.apply(.replaceText("edit again"), expectedRevision: 1)
        let expected = try await twin.apply(
            .recordExternalChange(observedDiskHash: observed),
            expectedRevision: 2
        )
        XCTAssertEqual(conflicted, expected)
        XCTAssertEqual(conflicted.saveState, .conflicted)

        for mutation in [
            DocumentMutation.replaceText("x"),
            .commitSave(writtenDiskHash: .hashing("x")),
            .resolveConflict(text: "x", diskBaselineHash: .hashing("x"))
        ] {
            do {
                _ = try await session.applyTextNeutral(mutation)
                XCTFail("Expected notTextNeutral for \(mutation)")
            } catch {
                XCTAssertEqual(error as? DocumentSessionError, .notTextNeutral)
            }
        }
    }

    /// The race the async save pipeline hits at session level: a dirty
    /// session's revision-R text is written to disk, a keystroke lands
    /// before the commit, and the commit must still move the baseline —
    /// otherwise the next save and the file watcher both invent conflicts.
    func testInterleavedEditDuringWriteKeepsBaselineConsistent() async throws {
        let root = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let fileURL = root.appendingPathComponent("main.tex")
        try "v0".write(to: fileURL, atomically: true, encoding: .utf8)
        let session = try makeSession(initialText: "v0")
        let store = FoundationAtomicDocumentStore()

        let written = try await session.apply(.replaceText("v1"), expectedRevision: 0)
        guard case .saved = store.save(
            text: written.text,
            to: fileURL,
            expectedBaselineHash: written.diskBaselineHash
        ) else {
            return XCTFail("Expected the revision-1 write to succeed")
        }

        _ = try await session.apply(.replaceText("v2"), expectedRevision: written.revision)
        let committed = try await session.commitSave(
            writtenDiskHash: .hashing(written.text),
            writtenRevision: written.revision
        )
        XCTAssertNil(committed.conflict)
        XCTAssertEqual(committed.diskBaselineHash, .hashing("v1"))
        XCTAssertEqual(committed.saveState, .dirty)

        // The follow-up save must see disk == baseline — no false conflict.
        let resave = store.save(
            text: committed.text,
            to: fileURL,
            expectedBaselineHash: committed.diskBaselineHash
        )
        guard case .saved = resave else {
            return XCTFail("Expected a clean re-save, got \(resave)")
        }
    }

    /// The write serializer's shape, exercised without AppKit: each
    /// enqueued write awaits the previous task (commit included), then
    /// re-reads the session before touching disk.
    func testSerializedWritesLeaveNewestTextOnDisk() async throws {
        let root = try makeTemporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let fileURL = root.appendingPathComponent("main.tex")
        try "v0".write(to: fileURL, atomically: true, encoding: .utf8)
        let session = try makeSession(initialText: "v0")
        let store = FoundationAtomicDocumentStore()

        var queue: Task<Void, Never>?
        func enqueue() {
            let previous = queue
            queue = Task {
                await previous?.value
                let snapshot = await session.snapshot()
                guard snapshot.saveState == .dirty else { return }
                if case let .saved(document) = store.save(
                    text: snapshot.text,
                    to: fileURL,
                    expectedBaselineHash: snapshot.diskBaselineHash
                ) {
                    _ = try? await session.commitSave(
                        writtenDiskHash: document.hash,
                        writtenRevision: snapshot.revision
                    )
                }
            }
        }

        _ = try await session.apply(.replaceText("v1"), expectedRevision: 0)
        enqueue()
        // The edit may race the first write's commit; retry on a stale
        // base until it lands (staleRevision is the only possible error).
        var applied = false
        for _ in 0 ..< 100 where !applied {
            let current = await session.snapshot()
            applied = (try? await session.apply(
                .replaceText("v2"),
                expectedRevision: current.revision
            )) != nil
        }
        XCTAssertTrue(applied)
        enqueue()
        await queue?.value

        XCTAssertEqual(try String(contentsOf: fileURL, encoding: .utf8), "v2")
        let snapshot = await session.snapshot()
        XCTAssertEqual(snapshot.diskBaselineHash, .hashing("v2"))
        XCTAssertEqual(snapshot.saveState, .clean)
    }

    /// A keep-mine write that lands after an interleaved edit must still
    /// clear the conflict it was issued to resolve — that conflict is the
    /// session's own, not a different one that should win.
    func testCommitSaveResolvingSameConflictWithInterleavedEdit() async throws {
        let session = try makeSession(initialText: "base")
        _ = try await session.apply(.replaceText("mine"), expectedRevision: 0)
        let conflicted = try await session.apply(
            .recordExternalChange(observedDiskHash: .hashing("external")),
            expectedRevision: 1
        )
        let conflict = try XCTUnwrap(conflicted.conflict)
        // A keystroke lands while the keep-mine write is still in flight.
        _ = try await session.apply(
            .replaceText("mine plus typing"),
            expectedRevision: conflicted.revision
        )

        let committed = try await session.commitSave(
            writtenDiskHash: .hashing("mine"),
            writtenRevision: conflicted.revision,
            resolving: conflict
        )
        XCTAssertNil(committed.conflict)
        XCTAssertEqual(committed.diskBaselineHash, .hashing("mine"))
        XCTAssertEqual(committed.text, "mine plus typing")
        XCTAssertEqual(committed.saveState, .dirty)
        XCTAssertEqual(committed.revision, conflicted.revision + 2)
    }

    /// A conflict recorded after the resolving write started is a
    /// different conflict: it still wins and leaves the session untouched.
    func testCommitSaveResolvingLosesToADifferentConflict() async throws {
        let session = try makeSession(initialText: "base")
        _ = try await session.apply(.replaceText("mine"), expectedRevision: 0)
        let conflicted = try await session.apply(
            .recordExternalChange(observedDiskHash: .hashing("external")),
            expectedRevision: 1
        )
        let conflict = try XCTUnwrap(conflicted.conflict)
        // Mid-write: another resolution lands, then a fresh conflict.
        _ = try await session.apply(
            .resolveConflict(text: "adopted", diskBaselineHash: .hashing("external")),
            expectedRevision: conflicted.revision
        )
        let reconflicted = try await session.apply(
            .recordExternalChange(observedDiskHash: .hashing("external-2")),
            expectedRevision: conflicted.revision + 1
        )

        let committed = try await session.commitSave(
            writtenDiskHash: .hashing("mine"),
            writtenRevision: conflicted.revision,
            resolving: conflict
        )
        XCTAssertEqual(committed, reconflicted)
        XCTAssertEqual(committed.saveState, .conflicted)
        XCTAssertEqual(committed.diskBaselineHash, .hashing("external"))
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
