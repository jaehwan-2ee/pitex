import ProjectCore
import TexDomain

public struct DiskContentHash: RawRepresentable, Hashable, Codable, Sendable {
    public let rawValue: UInt64

    public init(rawValue: UInt64) {
        self.rawValue = rawValue
    }

    public static func hashing(_ text: String) -> Self {
        var hash: UInt64 = 14_695_981_039_346_656_037
        for byte in text.utf8 {
            hash ^= UInt64(byte)
            hash &*= 1_099_511_628_211
        }
        return Self(rawValue: hash)
    }
}

public enum DocumentConflict: Hashable, Codable, Sendable {
    case externalModification(
        baseline: DiskContentHash,
        observedDisk: DiskContentHash
    )
    case saveCollision(
        baseline: DiskContentHash,
        observedDisk: DiskContentHash
    )
}

public enum DocumentSaveState: String, Codable, Sendable {
    case clean
    case dirty
    case conflicted
}

public struct DocumentSnapshot: Hashable, Codable, Sendable {
    public let documentID: StableDocumentID
    public let path: NormalizedRelativePath
    public let revision: UInt64
    public let text: String
    public let diskBaselineHash: DiskContentHash
    public let contentHash: DiskContentHash
    public let saveState: DocumentSaveState
    public let conflict: DocumentConflict?

    public init(
        documentID: StableDocumentID,
        path: NormalizedRelativePath,
        revision: UInt64,
        text: String,
        diskBaselineHash: DiskContentHash,
        contentHash: DiskContentHash,
        saveState: DocumentSaveState,
        conflict: DocumentConflict?
    ) {
        self.documentID = documentID
        self.path = path
        self.revision = revision
        self.text = text
        self.diskBaselineHash = diskBaselineHash
        self.contentHash = contentHash
        self.saveState = saveState
        self.conflict = conflict
    }
}

public enum DocumentMutation: Hashable, Sendable {
    case replaceText(String)
    case recordExternalChange(observedDiskHash: DiskContentHash)
    case recordSaveConflict(observedDiskHash: DiskContentHash)
    case commitSave(writtenDiskHash: DiskContentHash)
    case resolveConflict(text: String, diskBaselineHash: DiskContentHash)
}

public enum DocumentSessionError: Error, Equatable, Sendable {
    case staleRevision(expected: UInt64, actual: UInt64)
    case savedContentHashMismatch(expected: DiskContentHash, written: DiskContentHash)
    case revisionExhausted
}

public actor DocumentSession {
    public nonisolated let documentID: StableDocumentID
    public nonisolated let path: NormalizedRelativePath

    private var revision: UInt64
    private var text: String
    private var diskBaselineHash: DiskContentHash
    private var conflict: DocumentConflict?

    public init(
        file: ProjectFile,
        initialText: String,
        diskBaselineHash: DiskContentHash? = nil
    ) {
        self.documentID = file.documentID
        self.path = file.path
        self.revision = 0
        self.text = initialText
        self.diskBaselineHash = diskBaselineHash ?? .hashing(initialText)
        self.conflict = nil
    }

    public func snapshot() -> DocumentSnapshot {
        makeSnapshot()
    }

    @discardableResult
    public func apply(
        _ mutation: DocumentMutation,
        expectedRevision: UInt64
    ) throws -> DocumentSnapshot {
        guard expectedRevision == revision else {
            throw DocumentSessionError.staleRevision(
                expected: expectedRevision,
                actual: revision
            )
        }
        guard revision < UInt64.max else {
            throw DocumentSessionError.revisionExhausted
        }

        switch mutation {
        case let .replaceText(replacement):
            text = replacement

        case let .recordExternalChange(observedDiskHash):
            if observedDiskHash != diskBaselineHash && conflict == nil {
                conflict = .externalModification(
                    baseline: diskBaselineHash,
                    observedDisk: observedDiskHash
                )
            }

        case let .recordSaveConflict(observedDiskHash):
            if conflict == nil {
                conflict = .saveCollision(
                    baseline: diskBaselineHash,
                    observedDisk: observedDiskHash
                )
            }

        case let .commitSave(writtenDiskHash):
            let currentHash = DiskContentHash.hashing(text)
            guard writtenDiskHash == currentHash else {
                throw DocumentSessionError.savedContentHashMismatch(
                    expected: currentHash,
                    written: writtenDiskHash
                )
            }
            diskBaselineHash = writtenDiskHash
            conflict = nil

        case let .resolveConflict(resolvedText, newDiskBaselineHash):
            text = resolvedText
            diskBaselineHash = newDiskBaselineHash
            conflict = nil
        }

        revision += 1
        return makeSnapshot()
    }

    private func makeSnapshot() -> DocumentSnapshot {
        let contentHash = DiskContentHash.hashing(text)
        let saveState: DocumentSaveState
        if conflict != nil {
            saveState = .conflicted
        } else if contentHash != diskBaselineHash {
            saveState = .dirty
        } else {
            saveState = .clean
        }
        return DocumentSnapshot(
            documentID: documentID,
            path: path,
            revision: revision,
            text: text,
            diskBaselineHash: diskBaselineHash,
            contentHash: contentHash,
            saveState: saveState,
            conflict: conflict
        )
    }
}
