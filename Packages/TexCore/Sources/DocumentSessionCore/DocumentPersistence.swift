import Foundation
#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

public struct PersistedDocument: Equatable, Sendable {
    public let text: String
    public let hash: DiskContentHash

    public init(text: String, hash: DiskContentHash? = nil) {
        self.text = text
        self.hash = hash ?? .hashing(text)
    }
}

public struct DocumentPersistenceConflict: Equatable, Sendable {
    public let expectedBaselineHash: DiskContentHash?
    public let local: PersistedDocument
    public let observedDisk: PersistedDocument?

    public init(
        expectedBaselineHash: DiskContentHash?,
        local: PersistedDocument,
        observedDisk: PersistedDocument?
    ) {
        self.expectedBaselineHash = expectedBaselineHash
        self.local = local
        self.observedDisk = observedDisk
    }
}

public enum DocumentLoadOutcome: Equatable, Sendable {
    case loaded(PersistedDocument)
    case externalConflict(DocumentPersistenceConflict)
    case permissionFailure(path: String)
    case interruptedWrite(path: String)
}

public enum DocumentSaveOutcome: Equatable, Sendable {
    case saved(PersistedDocument)
    case staleBaseline(DocumentPersistenceConflict)
    case permissionFailure(path: String)
    case interruptedWrite(path: String)
}

public protocol DocumentPersistenceStore: Sendable {
    func load(
        from url: URL,
        expectedBaselineHash: DiskContentHash?
    ) -> DocumentLoadOutcome

    func save(
        text: String,
        to url: URL,
        expectedBaselineHash: DiskContentHash?
    ) -> DocumentSaveOutcome
}

public struct FoundationAtomicDocumentStore: DocumentPersistenceStore, Sendable {
    public init() {}

    public func load(
        from url: URL,
        expectedBaselineHash: DiskContentHash? = nil
    ) -> DocumentLoadOutcome {
        do {
            let disk = try readDocument(at: url)
            if let expectedBaselineHash, disk.hash != expectedBaselineHash {
                return .externalConflict(
                    DocumentPersistenceConflict(
                        expectedBaselineHash: expectedBaselineHash,
                        local: disk,
                        observedDisk: disk
                    )
                )
            }
            return .loaded(disk)
        } catch {
            return loadFailure(error, path: url.path)
        }
    }

    public func save(
        text: String,
        to url: URL,
        expectedBaselineHash: DiskContentHash?
    ) -> DocumentSaveOutcome {
        let local = PersistedDocument(text: text)
        let parent = url.deletingLastPathComponent()
        let temporaryURL = parent.appendingPathComponent(
            ".\(url.lastPathComponent).texspark-\(UUID().uuidString).tmp",
            isDirectory: false
        )
        var ownsTemporaryFile = false

        defer {
            if ownsTemporaryFile {
                try? FileManager.default.removeItem(at: temporaryURL)
            }
        }

        do {
            try Data().write(to: temporaryURL, options: .withoutOverwriting)
            ownsTemporaryFile = true

            let handle = try FileHandle(forWritingTo: temporaryURL)
            do {
                try handle.write(contentsOf: Data(text.utf8))
                try handle.synchronize()
                try handle.close()
            } catch {
                try? handle.close()
                throw error
            }

            let observedDisk: PersistedDocument?
            if FileManager.default.fileExists(atPath: url.path) {
                observedDisk = try readDocument(at: url)
            } else {
                observedDisk = nil
            }

            guard observedDisk?.hash == expectedBaselineHash,
                  (observedDisk != nil) == (expectedBaselineHash != nil) else {
                return .staleBaseline(
                    DocumentPersistenceConflict(
                        expectedBaselineHash: expectedBaselineHash,
                        local: local,
                        observedDisk: observedDisk
                    )
                )
            }

            try atomicRename(from: temporaryURL, to: url)
            ownsTemporaryFile = false
            try synchronizeDirectory(parent)
            return .saved(local)
        } catch {
            return saveFailure(error, path: url.path)
        }
    }

    private func readDocument(at url: URL) throws -> PersistedDocument {
        let data = try Data(contentsOf: url)
        guard let text = String(data: data, encoding: .utf8) else {
            throw PersistenceInternalError.invalidUTF8
        }
        return PersistedDocument(text: text)
    }

    private func atomicRename(from source: URL, to destination: URL) throws {
        let result = source.path.withCString { sourcePath in
            destination.path.withCString { destinationPath in
                rename(sourcePath, destinationPath)
            }
        }
        guard result == 0 else {
            throw POSIXPersistenceError(code: errno)
        }
    }

    private func synchronizeDirectory(_ directory: URL) throws {
        let descriptor = directory.path.withCString { open($0, O_RDONLY) }
        guard descriptor >= 0 else {
            throw POSIXPersistenceError(code: errno)
        }
        defer { _ = close(descriptor) }

        guard fsync(descriptor) == 0 else {
            let code = errno
            #if canImport(Darwin)
            if code == EINVAL || code == ENOTSUP { return }
            #elseif canImport(Glibc)
            if code == EINVAL || code == EOPNOTSUPP || code == EROFS { return }
            #endif
            throw POSIXPersistenceError(code: code)
        }
    }

    private func loadFailure(_ error: any Error, path: String) -> DocumentLoadOutcome {
        if isPermissionFailure(error) {
            return .permissionFailure(path: path)
        }
        return .interruptedWrite(path: path)
    }

    private func saveFailure(_ error: any Error, path: String) -> DocumentSaveOutcome {
        if isPermissionFailure(error) {
            return .permissionFailure(path: path)
        }
        return .interruptedWrite(path: path)
    }

    private func isPermissionFailure(_ error: any Error) -> Bool {
        let code: Int32
        if let posixError = error as? POSIXPersistenceError {
            code = posixError.code
        } else {
            let cocoaError = error as NSError
            if cocoaError.domain == NSPOSIXErrorDomain {
                code = Int32(cocoaError.code)
            } else if cocoaError.domain == NSCocoaErrorDomain,
                      cocoaError.code == CocoaError.Code.fileReadNoPermission.rawValue
                        || cocoaError.code == CocoaError.Code.fileWriteNoPermission.rawValue {
                return true
            } else {
                return false
            }
        }
        return code == EACCES || code == EPERM || code == EROFS
    }
}

private struct POSIXPersistenceError: Error, Sendable {
    let code: Int32
}

private enum PersistenceInternalError: Error, Sendable {
    case invalidUTF8
}
