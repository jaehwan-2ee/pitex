import AICore
import AppPorts
import DocumentSessionCore
import LanguageCore

public enum EditorFeatureError: Error, Equatable, Sendable {
    case invalidRange
    case selectionOutsideDocument
    case staleDecorationRevision(expected: UInt64, received: UInt64)
    case staleMutationRevision(expected: UInt64, received: UInt64)
    case invalidAITransaction
}

public struct EditorTextRange: Hashable, Codable, Sendable {
    public let location: Int
    public let length: Int

    public init(location: Int, length: Int) throws {
        guard location >= 0, length >= 0, location <= Int.max - length else {
            throw EditorFeatureError.invalidRange
        }
        self.location = location
        self.length = length
    }

    public var upperBound: Int { location + length }
}

public struct EditorSelection: Hashable, Codable, Sendable {
    public let ranges: [EditorTextRange]
    public let primaryRangeIndex: Int

    public init(ranges: [EditorTextRange], primaryRangeIndex: Int) throws {
        guard !ranges.isEmpty, ranges.indices.contains(primaryRangeIndex) else {
            throw EditorFeatureError.invalidRange
        }
        self.ranges = ranges
        self.primaryRangeIndex = primaryRangeIndex
    }
}

public struct MarkedTextPresentation: Hashable, Codable, Sendable {
    public let range: EditorTextRange
    public let selectedRange: EditorTextRange

    public init(range: EditorTextRange, selectedRange: EditorTextRange) throws {
        guard selectedRange.upperBound <= range.length else {
            throw EditorFeatureError.invalidRange
        }
        self.range = range
        self.selectedRange = selectedRange
    }
}

/// Immutable UI metadata for a core-owned document revision; it intentionally does not own text.
public struct EditorPresentationSnapshot: Hashable, Codable, Sendable {
    public let documentRevision: UInt64
    public let utf16Length: Int
    public let selection: EditorSelection
    public let markedText: MarkedTextPresentation?

    public init(
        documentRevision: UInt64,
        utf16Length: Int,
        selection: EditorSelection,
        markedText: MarkedTextPresentation?
    ) throws {
        guard utf16Length >= 0,
              selection.ranges.allSatisfy({ $0.upperBound <= utf16Length }),
              markedText.map({ $0.range.upperBound <= utf16Length }) ?? true else {
            throw EditorFeatureError.selectionOutsideDocument
        }
        self.documentRevision = documentRevision
        self.utf16Length = utf16Length
        self.selection = selection
        self.markedText = markedText
    }
}

public struct EditorDecoration: Hashable, Codable, Sendable {
    public let range: SourceRange
    public let tokenKind: LanguageTokenKind

    public init(range: SourceRange, tokenKind: LanguageTokenKind) {
        self.range = range
        self.tokenKind = tokenKind
    }
}

public struct EditorDecorationSnapshot: Hashable, Codable, Sendable {
    public let documentRevision: UInt64
    public let decorations: [EditorDecoration]

    public init(documentRevision: UInt64, decorations: [EditorDecoration]) {
        self.documentRevision = documentRevision
        self.decorations = decorations.sorted {
            if $0.range.utf8Offset != $1.range.utf8Offset {
                return $0.range.utf8Offset < $1.range.utf8Offset
            }
            return $0.range.utf8Length < $1.range.utf8Length
        }
    }

    public func checked(for presentation: EditorPresentationSnapshot) throws -> [EditorDecoration] {
        guard documentRevision == presentation.documentRevision else {
            throw EditorFeatureError.staleDecorationRevision(
                expected: presentation.documentRevision,
                received: documentRevision
            )
        }
        return decorations
    }
}

public enum EditorUndoPolicy: Hashable, Codable, Sendable {
    case register(actionName: String)
    case coalesce(identifier: String)
    case doNotRegister
}

public struct EditorMutationIntent: Hashable, Codable, Sendable {
    public let baseRevision: UInt64
    public let range: EditorTextRange
    public let replacement: String
    public let undoPolicy: EditorUndoPolicy

    public init(
        baseRevision: UInt64,
        range: EditorTextRange,
        replacement: String,
        undoPolicy: EditorUndoPolicy
    ) {
        self.baseRevision = baseRevision
        self.range = range
        self.replacement = replacement
        self.undoPolicy = undoPolicy
    }

    public func checked(for presentation: EditorPresentationSnapshot) throws -> AppPorts.DocumentMutation {
        guard baseRevision == presentation.documentRevision else {
            throw EditorFeatureError.staleMutationRevision(
                expected: presentation.documentRevision,
                received: baseRevision
            )
        }
        guard range.upperBound <= presentation.utf16Length else {
            throw EditorFeatureError.selectionOutsideDocument
        }
        return AppPorts.DocumentMutation(
            baseRevision: baseRevision,
            range: AppPorts.DocumentTextRange(location: range.location, length: range.length),
            replacement: replacement
        )
    }
}

/// An explicit user intent emitted only after a revision-bound AI proposal was accepted and applied.
public struct EditorAIApplyIntent: Hashable, Codable, Sendable {
    public let proposalID: String
    public let baseRevision: UInt64
    public let replacementText: String
    public let undoTransaction: NativeUndoTransactionDescriptor

    public init(application: RevisionBoundEditApplication) throws {
        guard case let .applied(appliedProposalID) =
                application.transaction.state,
              appliedProposalID == application.transaction.proposal.proposalID,
              application.transaction.proposal.proposalID
                == application.undoTransaction.proposalID,
              application.transaction.proposal.baseRevision
                == application.undoTransaction.baseRevision else {
            throw EditorFeatureError.invalidAITransaction
        }
        self.proposalID = application.transaction.proposal.proposalID
        self.baseRevision = application.transaction.proposal.baseRevision
        self.replacementText = application.replacementText
        self.undoTransaction = application.undoTransaction
    }

    public func checked(
        for presentation: EditorPresentationSnapshot
    ) throws -> EditorMutationIntent {
        guard baseRevision == presentation.documentRevision else {
            throw EditorFeatureError.staleMutationRevision(
                expected: presentation.documentRevision,
                received: baseRevision
            )
        }
        return EditorMutationIntent(
            baseRevision: baseRevision,
            range: try EditorTextRange(
                location: 0,
                length: presentation.utf16Length
            ),
            replacement: replacementText,
            undoPolicy: .register(actionName: undoTransaction.actionName)
        )
    }
}

public enum EditorUserIntent: Hashable, Codable, Sendable {
    case mutate(EditorMutationIntent)
    case applyAI(EditorAIApplyIntent)
    case setSelection(EditorSelection)
    case setMarkedText(MarkedTextPresentation?)
    case undo
    case redo
}
