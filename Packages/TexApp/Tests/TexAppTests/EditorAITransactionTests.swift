import AICore
import DocumentSessionCore
import EditorFeature
import TexDomain
import XCTest

final class EditorAITransactionTests: XCTestCase {
    func testExplicitAIApplyIntentCarriesOneUndoTransaction() throws {
        let snapshot = try makeSnapshot(text: "before", revision: 4)
        let proposal = try RevisionBoundEditProposal(
            proposalID: "proposal-editor-1",
            snapshot: snapshot,
            edits: [
                try TextEdit(
                    utf8Offset: 0,
                    utf8Length: 6,
                    replacement: "after"
                )
            ],
            disclosureScope: .wholeDocument(userApproved: true)
        )
        let application = try RevisionBoundEditTransaction(proposal: proposal)
            .accepting()
            .applying(to: snapshot)
        let applyIntent = try EditorAIApplyIntent(application: application)
        let userIntent = EditorUserIntent.applyAI(applyIntent)

        guard case let .applyAI(exposedIntent) = userIntent else {
            return XCTFail("Expected an explicit AI apply intent")
        }
        XCTAssertEqual(exposedIntent.proposalID, "proposal-editor-1")
        XCTAssertEqual(
            exposedIntent.undoTransaction,
            application.undoTransaction
        )

        let presentation = try makePresentation(revision: 4, text: "before")
        let mutation = try exposedIntent.checked(for: presentation)
        XCTAssertEqual(mutation.baseRevision, 4)
        XCTAssertEqual(mutation.range, try EditorTextRange(location: 0, length: 6))
        XCTAssertEqual(mutation.replacement, "after")
        XCTAssertEqual(
            mutation.undoPolicy,
            .register(actionName: "Apply AI Edit")
        )
    }

    func testAIApplyIntentRejectsStalePresentation() throws {
        let snapshot = try makeSnapshot(text: "before", revision: 4)
        let proposal = try RevisionBoundEditProposal(
            proposalID: "proposal-editor-1",
            snapshot: snapshot,
            edits: [
                try TextEdit(
                    utf8Offset: 0,
                    utf8Length: 6,
                    replacement: "after"
                )
            ],
            disclosureScope: .wholeDocument(userApproved: true)
        )
        let application = try RevisionBoundEditTransaction(proposal: proposal)
            .accepting()
            .applying(to: snapshot)
        let intent = try EditorAIApplyIntent(application: application)
        let stalePresentation = try makePresentation(
            revision: 5,
            text: "newer editor text"
        )

        XCTAssertThrowsError(
            try intent.checked(for: stalePresentation)
        ) { error in
            XCTAssertEqual(
                error as? EditorFeatureError,
                .staleMutationRevision(expected: 5, received: 4)
            )
        }
    }

    private func makePresentation(
        revision: UInt64,
        text: String
    ) throws -> EditorPresentationSnapshot {
        let selection = try EditorSelection(
            ranges: [EditorTextRange(location: 0, length: 0)],
            primaryRangeIndex: 0
        )
        return try EditorPresentationSnapshot(
            documentRevision: revision,
            utf16Length: text.utf16.count,
            selection: selection,
            markedText: nil
        )
    }

    private func makeSnapshot(
        text: String,
        revision: UInt64
    ) throws -> DocumentSnapshot {
        let hash = DiskContentHash.hashing(text)
        return DocumentSnapshot(
            documentID: try StableDocumentID(rawValue: "document-editor-1"),
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
