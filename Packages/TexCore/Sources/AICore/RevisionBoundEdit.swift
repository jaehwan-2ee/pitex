import DocumentSessionCore
import TexDomain

public enum RevisionBoundEditError: Error, Equatable, Sendable {
    case emptyProposalID
    case emptyEdits
    case invalidRange
    case unorderedOrOverlappingEdits
    case outOfBounds
    case midScalarBoundary
    case disclosureNotApproved
    case emptyDisclosure
    case implicitWholeDocumentDisclosure
    case documentMismatch(expected: StableDocumentID, actual: StableDocumentID)
    case staleRevision(expected: UInt64, actual: UInt64)
    case contentHashMismatch(expected: DiskContentHash, actual: DiskContentHash)
    case applyRequiresAcceptedProposal
    case invalidTransition
}

public struct UTF8TextRange: Hashable, Codable, Sendable {
    public let offset: Int
    public let length: Int

    public init(offset: Int, length: Int) throws {
        guard offset >= 0, length >= 0, offset <= Int.max - length else {
            throw RevisionBoundEditError.invalidRange
        }
        self.offset = offset
        self.length = length
    }

    public var upperBound: Int { offset + length }
}

/// Describes exactly which source text the user permits an AI provider to receive.
public enum AIContextDisclosureScope: Hashable, Codable, Sendable {
    case selectedRanges([UTF8TextRange])
    case wholeDocument(userApproved: Bool)
}

public struct RevisionBoundEditProposal: Hashable, Sendable {
    public let proposalID: String
    public let documentID: StableDocumentID
    public let baseRevision: UInt64
    public let baseContentHash: DiskContentHash
    public let edits: [TextEdit]
    public let disclosureScope: AIContextDisclosureScope

    public init(
        proposalID: String,
        snapshot: DocumentSnapshot,
        edits: [TextEdit],
        disclosureScope: AIContextDisclosureScope
    ) throws {
        guard !proposalID.isEmpty,
              !proposalID.unicodeScalars.contains(where: { $0.value == 0 }) else {
            throw RevisionBoundEditError.emptyProposalID
        }
        guard !edits.isEmpty,
              edits.allSatisfy({
                  $0.utf8Length > 0 || !$0.replacement.isEmpty
              }) else {
            throw RevisionBoundEditError.emptyEdits
        }
        let actualContentHash = DiskContentHash.hashing(snapshot.text)
        guard snapshot.contentHash == actualContentHash else {
            throw RevisionBoundEditError.contentHashMismatch(
                expected: snapshot.contentHash,
                actual: actualContentHash
            )
        }
        let editRanges = try edits.map {
            try UTF8TextRange(
                offset: $0.utf8Offset,
                length: $0.utf8Length
            )
        }
        try Self.validateOrderedRanges(editRanges, in: snapshot.text)
        try Self.validateDisclosure(disclosureScope, in: snapshot.text)

        self.proposalID = proposalID
        self.documentID = snapshot.documentID
        self.baseRevision = snapshot.revision
        self.baseContentHash = snapshot.contentHash
        self.edits = edits
        self.disclosureScope = disclosureScope
    }

    fileprivate func validateBase(_ snapshot: DocumentSnapshot) throws {
        guard snapshot.documentID == documentID else {
            throw RevisionBoundEditError.documentMismatch(
                expected: documentID,
                actual: snapshot.documentID
            )
        }
        guard snapshot.revision == baseRevision else {
            throw RevisionBoundEditError.staleRevision(
                expected: baseRevision,
                actual: snapshot.revision
            )
        }
        guard snapshot.contentHash == baseContentHash,
              DiskContentHash.hashing(snapshot.text) == baseContentHash else {
            throw RevisionBoundEditError.contentHashMismatch(
                expected: baseContentHash,
                actual: DiskContentHash.hashing(snapshot.text)
            )
        }
    }

    private static func validateDisclosure(
        _ scope: AIContextDisclosureScope,
        in text: String
    ) throws {
        switch scope {
        case let .wholeDocument(userApproved):
            guard userApproved else {
                throw RevisionBoundEditError.disclosureNotApproved
            }
        case let .selectedRanges(ranges):
            guard !ranges.isEmpty,
                  ranges.contains(where: { $0.length > 0 }) else {
                throw RevisionBoundEditError.emptyDisclosure
            }
            try validateOrderedRanges(ranges, in: text)
            if ranges.first?.offset == 0,
               ranges.last?.upperBound == text.utf8.count,
               zip(ranges, ranges.dropFirst()).allSatisfy({
                   $0.0.upperBound == $0.1.offset
               }) {
                throw RevisionBoundEditError.implicitWholeDocumentDisclosure
            }
        }
    }

    private static func validateOrderedRanges(
        _ ranges: [UTF8TextRange],
        in text: String
    ) throws {
        var previousUpperBound = 0
        for (index, range) in ranges.enumerated() {
            guard range.upperBound <= text.utf8.count else {
                throw RevisionBoundEditError.outOfBounds
            }
            guard index == 0 || range.offset >= previousUpperBound else {
                throw RevisionBoundEditError.unorderedOrOverlappingEdits
            }
            guard isScalarBoundary(range.offset, in: text),
                  isScalarBoundary(range.upperBound, in: text) else {
                throw RevisionBoundEditError.midScalarBoundary
            }
            previousUpperBound = range.upperBound
        }
    }

    private static func isScalarBoundary(
        _ offset: Int,
        in text: String
    ) -> Bool {
        guard offset <= text.utf8.count else { return false }
        guard offset < text.utf8.count else { return true }
        let index = text.utf8.index(text.utf8.startIndex, offsetBy: offset)
        return text.utf8[index] & 0b1100_0000 != 0b1000_0000
    }
}

public struct NativeUndoTransactionDescriptor: Hashable, Codable, Sendable {
    public let proposalID: String
    public let documentID: StableDocumentID
    public let baseRevision: UInt64
    public let actionName: String

    public init(
        proposalID: String,
        documentID: StableDocumentID,
        baseRevision: UInt64,
        actionName: String
    ) {
        self.proposalID = proposalID
        self.documentID = documentID
        self.baseRevision = baseRevision
        self.actionName = actionName
    }
}

public struct RevisionBoundEditApplication: Hashable, Sendable {
    public let transaction: RevisionBoundEditTransaction
    public let replacementText: String
    public let undoTransaction: NativeUndoTransactionDescriptor

    fileprivate init(
        transaction: RevisionBoundEditTransaction,
        replacementText: String,
        undoTransaction: NativeUndoTransactionDescriptor
    ) {
        self.transaction = transaction
        self.replacementText = replacementText
        self.undoTransaction = undoTransaction
    }
}

public struct RevisionBoundEditTransaction: Hashable, Sendable {
    public let proposal: RevisionBoundEditProposal
    public let state: EditProposalState

    public init(proposal: RevisionBoundEditProposal) {
        self.proposal = proposal
        self.state = .proposed(
            id: proposal.proposalID,
            edits: proposal.edits
        )
    }

    private init(proposal: RevisionBoundEditProposal, state: EditProposalState) {
        self.proposal = proposal
        self.state = state
    }

    public func accepting() throws -> Self {
        guard case let .proposed(id, edits) = state,
              id == proposal.proposalID,
              edits == proposal.edits else {
            throw RevisionBoundEditError.invalidTransition
        }
        return Self(proposal: proposal, state: try state.accepting())
    }

    public func rejecting() throws -> Self {
        guard case let .proposed(id, edits) = state,
              id == proposal.proposalID,
              edits == proposal.edits else {
            throw RevisionBoundEditError.invalidTransition
        }
        return Self(proposal: proposal, state: try state.rejecting())
    }

    public func applying(to snapshot: DocumentSnapshot) throws -> RevisionBoundEditApplication {
        guard case let .accepted(id, edits) = state,
              id == proposal.proposalID,
              edits == proposal.edits else {
            throw RevisionBoundEditError.applyRequiresAcceptedProposal
        }
        try proposal.validateBase(snapshot)

        let source = Array(snapshot.text.utf8)
        var replacement: [UInt8] = []
        replacement.reserveCapacity(source.count)
        var cursor = 0
        for edit in proposal.edits {
            let upperBound = edit.utf8Offset + edit.utf8Length
            replacement.append(contentsOf: source[cursor..<edit.utf8Offset])
            replacement.append(contentsOf: edit.replacement.utf8)
            cursor = upperBound
        }
        replacement.append(contentsOf: source[cursor...])
        let replacementText = String(decoding: replacement, as: UTF8.self)

        let applied = Self(proposal: proposal, state: try state.applied())
        let undo = NativeUndoTransactionDescriptor(
            proposalID: proposal.proposalID,
            documentID: proposal.documentID,
            baseRevision: proposal.baseRevision,
            actionName: "Apply AI Edit"
        )
        return RevisionBoundEditApplication(
            transaction: applied,
            replacementText: replacementText,
            undoTransaction: undo
        )
    }
}
