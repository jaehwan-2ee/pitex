import AICore
import DocumentSessionCore
import ProjectCore
import TexDomain
import XCTest

final class RevisionBoundEditTests: XCTestCase {
    func testMultipleOrderedEditsProduceOneDeterministicReplacementAndUndoDescriptor() throws {
        let snapshot = try makeSnapshot(text: "alpha beta gamma", revision: 7)
        let proposal = try makeProposal(
            snapshot: snapshot,
            edits: [
                edit(0, 5, "A"),
                edit(11, 5, "G")
            ]
        )
        let accepted = try RevisionBoundEditTransaction(proposal: proposal).accepting()

        let first = try accepted.applying(to: snapshot)
        let second = try accepted.applying(to: snapshot)

        XCTAssertEqual(first, second)
        XCTAssertEqual(first.replacementText, "A beta G")
        guard case .applied("proposal-1") = first.transaction.state else {
            return XCTFail("Expected applied proposal state")
        }
        XCTAssertEqual(first.undoTransaction.proposalID, "proposal-1")
        XCTAssertEqual(first.undoTransaction.baseRevision, 7)
        XCTAssertEqual(first.undoTransaction.actionName, "Apply AI Edit")
    }

    func testUnicodeRangesUseUTF8AndRejectMidScalarBoundaries() throws {
        let snapshot = try makeSnapshot(text: "a😀éz")
        let proposal = try makeProposal(
            snapshot: snapshot,
            edits: [edit(1, 4, "🙂"), edit(5, 2, "e")]
        )
        let application = try RevisionBoundEditTransaction(proposal: proposal)
            .accepting()
            .applying(to: snapshot)
        XCTAssertEqual(application.replacementText, "a🙂ez")

        XCTAssertThrowsError(
            try makeProposal(snapshot: snapshot, edits: [edit(2, 1, "x")])
        ) { error in
            XCTAssertEqual(error as? RevisionBoundEditError, .midScalarBoundary)
        }
    }

    func testEmptyOverlappingUnorderedAndOutOfBoundsEditsAreRejected() throws {
        let snapshot = try makeSnapshot(text: "abcdef")

        XCTAssertThrowsError(
            try makeProposal(snapshot: snapshot, edits: [])
        ) { error in
            XCTAssertEqual(error as? RevisionBoundEditError, .emptyEdits)
        }
        XCTAssertThrowsError(
            try makeProposal(snapshot: snapshot, edits: [edit(1, 0, "")])
        ) { error in
            XCTAssertEqual(error as? RevisionBoundEditError, .emptyEdits)
        }
        XCTAssertThrowsError(
            try makeProposal(
                snapshot: snapshot,
                edits: [edit(1, 3, "x"), edit(3, 1, "y")]
            )
        ) { error in
            XCTAssertEqual(
                error as? RevisionBoundEditError,
                .unorderedOrOverlappingEdits
            )
        }
        XCTAssertThrowsError(
            try makeProposal(
                snapshot: snapshot,
                edits: [edit(4, 1, "x"), edit(1, 1, "y")]
            )
        ) { error in
            XCTAssertEqual(
                error as? RevisionBoundEditError,
                .unorderedOrOverlappingEdits
            )
        }
        XCTAssertThrowsError(
            try makeProposal(snapshot: snapshot, edits: [edit(6, 1, "x")])
        ) { error in
            XCTAssertEqual(error as? RevisionBoundEditError, .outOfBounds)
        }
    }

    func testApplicationRejectsStaleRevisionAndHashMismatch() throws {
        let base = try makeSnapshot(text: "abcdef", revision: 2)
        let accepted = try RevisionBoundEditTransaction(
            proposal: makeProposal(snapshot: base, edits: [edit(1, 1, "X")])
        ).accepting()

        let stale = try makeSnapshot(text: "abcdef", revision: 3)
        XCTAssertThrowsError(try accepted.applying(to: stale)) { error in
            XCTAssertEqual(
                error as? RevisionBoundEditError,
                .staleRevision(expected: 2, actual: 3)
            )
        }

        let changed = try makeSnapshot(text: "abcxef", revision: 2)
        XCTAssertThrowsError(try accepted.applying(to: changed)) { error in
            XCTAssertEqual(
                error as? RevisionBoundEditError,
                .contentHashMismatch(
                    expected: DiskContentHash.hashing("abcdef"),
                    actual: DiskContentHash.hashing("abcxef")
                )
            )
        }
    }

    func testDisclosureRequiresExplicitWholeDocumentConsent() throws {
        let snapshot = try makeSnapshot(text: "abcdef")
        let edits = [try edit(1, 1, "X")]

        XCTAssertThrowsError(
            try RevisionBoundEditProposal(
                proposalID: "proposal-1",
                snapshot: snapshot,
                edits: edits,
                disclosureScope: .wholeDocument(userApproved: false)
            )
        ) { error in
            XCTAssertEqual(
                error as? RevisionBoundEditError,
                .disclosureNotApproved
            )
        }
        XCTAssertThrowsError(
            try RevisionBoundEditProposal(
                proposalID: "proposal-1",
                snapshot: snapshot,
                edits: edits,
                disclosureScope: .selectedRanges([
                    try UTF8TextRange(offset: 0, length: 3),
                    try UTF8TextRange(offset: 3, length: 3)
                ])
            )
        ) { error in
            XCTAssertEqual(
                error as? RevisionBoundEditError,
                .implicitWholeDocumentDisclosure
            )
        }

        XCTAssertNoThrow(
            try RevisionBoundEditProposal(
                proposalID: "proposal-1",
                snapshot: snapshot,
                edits: edits,
                disclosureScope: .wholeDocument(userApproved: true)
            )
        )
    }

    func testAcceptRejectAndApplyTransitionsAreExplicit() throws {
        let snapshot = try makeSnapshot(text: "abcdef")
        let proposed = RevisionBoundEditTransaction(
            proposal: try makeProposal(
                snapshot: snapshot,
                edits: [edit(1, 1, "X")]
            )
        )

        XCTAssertThrowsError(try proposed.applying(to: snapshot)) { error in
            XCTAssertEqual(
                error as? RevisionBoundEditError,
                .applyRequiresAcceptedProposal
            )
        }
        let rejected = try proposed.rejecting()
        guard case .rejected("proposal-1") = rejected.state else {
            return XCTFail("Expected rejected proposal state")
        }
        XCTAssertThrowsError(try rejected.accepting())
        XCTAssertThrowsError(try rejected.applying(to: snapshot))

        let accepted = try proposed.accepting()
        guard case .accepted("proposal-1", _) = accepted.state else {
            return XCTFail("Expected accepted proposal state")
        }
        XCTAssertThrowsError(try accepted.rejecting())
        let applied = try accepted.applying(to: snapshot).transaction
        guard case .applied("proposal-1") = applied.state else {
            return XCTFail("Expected applied proposal state")
        }
    }

    private func makeProposal(
        snapshot: DocumentSnapshot,
        edits: [TextEdit]
    ) throws -> RevisionBoundEditProposal {
        try RevisionBoundEditProposal(
            proposalID: "proposal-1",
            snapshot: snapshot,
            edits: edits,
            disclosureScope: .selectedRanges([
                UTF8TextRange(offset: 0, length: 1)
            ])
        )
    }

    private func edit(
        _ offset: Int,
        _ length: Int,
        _ replacement: String
    ) throws -> TextEdit {
        try TextEdit(
            utf8Offset: offset,
            utf8Length: length,
            replacement: replacement
        )
    }

    private func makeSnapshot(
        text: String,
        revision: UInt64 = 0
    ) throws -> DocumentSnapshot {
        let hash = DiskContentHash.hashing(text)
        return DocumentSnapshot(
            documentID: try StableDocumentID(rawValue: "document-1"),
            path: try NormalizedRelativePath(rawValue: "main.tex"),
            revision: revision,
            text: text,
            diskBaselineHash: hash,
            contentHash: hash,
            saveState: .clean,
            conflict: nil
        )
    }
}
