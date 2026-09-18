import Foundation
import ProjectCore
import TexDomain

public enum DocumentMutationOrigin: String, Codable, Sendable {
    case textKit
    case externalDisk
    case aiProposal
    case recovery
}

public struct OriginatedDocumentMutation: Sendable {
    public let mutation: DocumentMutation
    public let origin: DocumentMutationOrigin
    public let expectedRevision: UInt64?

    public init(
        mutation: DocumentMutation,
        origin: DocumentMutationOrigin,
        expectedRevision: UInt64? = nil
    ) {
        self.mutation = mutation
        self.origin = origin
        self.expectedRevision = expectedRevision
    }
}

public enum OriginatedDocumentMutationError: Error, Equatable, Sendable {
    case expectedRevisionRequired(origin: DocumentMutationOrigin)
}

public extension DocumentSession {
    @discardableResult
    func apply(_ originatedMutation: OriginatedDocumentMutation) throws -> DocumentSnapshot {
        let revision: UInt64
        if let expectedRevision = originatedMutation.expectedRevision {
            revision = expectedRevision
        } else {
            guard originatedMutation.origin == .textKit else {
                throw OriginatedDocumentMutationError.expectedRevisionRequired(
                    origin: originatedMutation.origin
                )
            }
            revision = snapshot().revision
        }
        return try apply(originatedMutation.mutation, expectedRevision: revision)
    }
}

public enum DocumentSessionRegistryError: Error, Equatable, Sendable {
    case projectRootMustBeFileURL
    case documentIdentityMismatch(
        path: NormalizedRelativePath,
        existing: StableDocumentID,
        requested: StableDocumentID
    )
}

public actor DocumentSessionRegistry {
    private struct Key: Hashable, Sendable {
        let canonicalProjectRoot: String
        let path: NormalizedRelativePath
    }

    private var sessions: [Key: DocumentSession] = [:]

    public init() {}

    public func open(
        projectRoot: URL,
        file: ProjectFile,
        initialText: String,
        diskBaselineHash: DiskContentHash? = nil
    ) throws -> DocumentSession {
        let key = try makeKey(projectRoot: projectRoot, path: file.path)
        if let existing = sessions[key] {
            guard existing.documentID == file.documentID else {
                throw DocumentSessionRegistryError.documentIdentityMismatch(
                    path: file.path,
                    existing: existing.documentID,
                    requested: file.documentID
                )
            }
            return existing
        }

        let session = DocumentSession(
            file: file,
            initialText: initialText,
            diskBaselineHash: diskBaselineHash
        )
        sessions[key] = session
        return session
    }

    @discardableResult
    public func close(projectRoot: URL, session: DocumentSession) throws -> Bool {
        let key = try makeKey(projectRoot: projectRoot, path: session.path)
        guard let registered = sessions[key], registered === session else {
            return false
        }
        sessions.removeValue(forKey: key)
        return true
    }

    public func session(
        projectRoot: URL,
        path: NormalizedRelativePath
    ) throws -> DocumentSession? {
        sessions[try makeKey(projectRoot: projectRoot, path: path)]
    }

    public var count: Int {
        sessions.count
    }

    private func makeKey(
        projectRoot: URL,
        path: NormalizedRelativePath
    ) throws -> Key {
        guard projectRoot.isFileURL else {
            throw DocumentSessionRegistryError.projectRootMustBeFileURL
        }
        let canonicalRoot = projectRoot
            .standardizedFileURL
            .resolvingSymlinksInPath()
            .path
        return Key(canonicalProjectRoot: canonicalRoot, path: path)
    }
}
