import Foundation

public enum PlatformPortError: Error, Equatable, Sendable {
    case unavailable(feature: String, platform: String)
    case invalidCapability
    case accessDenied(URL)
    case staleCapability(URL)
    case processLaunchFailed(executable: URL, reason: String)
    case operationInProgress(String)
    case staticCodeChanged
}

extension PlatformPortError: LocalizedError {
    public var errorDescription: String? {
        switch self {
        case let .unavailable(feature, platform):
            "\(feature) is not available on \(platform)."
        case .invalidCapability:
            "Pitex does not have a usable permission for this folder. Choose the folder again with Open."
        case let .accessDenied(url):
            "Pitex cannot access \(url.path). Choose the folder again with Open."
        case let .staleCapability(url):
            "The saved permission for \(url.path) is out of date. Choose the folder again with Open."
        case let .processLaunchFailed(executable, reason):
            "Pitex could not run \(executable.path). \(reason)"
        case let .operationInProgress(name):
            "\(name) is already running."
        case .staticCodeChanged:
            "Pitex was updated or changed on disk while it was running. Quit and reopen Pitex, then open the project again."
        }
    }

    private static let errSecCSStaticCodeChanged = -67_034

    public static func bookmarkAccessFailure(_ error: Error, url: URL) -> PlatformPortError {
        let changed = nsErrorChain(error).contains { nsError in
            nsError.domain == NSOSStatusErrorDomain && nsError.code == Self.errSecCSStaticCodeChanged
        }
        if changed {
            return .staticCodeChanged
        }
        return .accessDenied(url)
    }

    public static func bookmarkFailureLogLine(_ error: Error) -> String {
        nsErrorChain(error)
            .map { "\($0.domain):\($0.code)" }
            .joined(separator: " ")
    }

    public static func projectDirectoryAccessError(at url: URL) -> PlatformPortError? {
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory),
              isDirectory.boolValue else {
            return nil
        }
        if !FileManager.default.isReadableFile(atPath: url.path) {
            return .accessDenied(url)
        }
        let rootPath = url.standardizedFileURL.path
        let denied = RootListingDenied()
        guard let enumerator = FileManager.default.enumerator(
            at: url,
            includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles],
            errorHandler: { failedURL, _ in
                if failedURL.standardizedFileURL.path == rootPath {
                    denied.value = true
                    return false
                }
                return true
            }
        ) else {
            return .accessDenied(url)
        }
        _ = enumerator.nextObject()
        if denied.value {
            return .accessDenied(url)
        }
        return nil
    }

    private static func nsErrorChain(_ error: Error) -> [NSError] {
        var chain: [NSError] = []
        var pending: [Error] = [error]
        var seen = 0
        while let next = pending.popLast(), seen < 8 {
            seen += 1
            let nsError = next as NSError
            chain.append(nsError)
            if let underlying = nsError.userInfo[NSUnderlyingErrorKey] as? Error {
                pending.append(underlying)
            }
            if let many = nsError.userInfo["NSMultipleUnderlyingErrorsKey"] as? [Any] {
                for item in many {
                    if let underlying = item as? Error {
                        pending.append(underlying)
                    }
                }
            }
        }
        return chain
    }
}

private final class RootListingDenied: @unchecked Sendable {
    var value = false
}

public enum FileCapabilityAccess: String, Codable, Sendable {
    case readOnly
    case readWrite
}

public struct FileCapability: Hashable, Codable, Sendable {
    public let bookmark: Data
    public let access: FileCapabilityAccess

    public init(bookmark: Data, access: FileCapabilityAccess) {
        self.bookmark = bookmark
        self.access = access
    }
}

public struct FileAccessLease: Hashable, Sendable {
    public let id: UUID
    public let url: URL
    public let access: FileCapabilityAccess

    public init(id: UUID, url: URL, access: FileCapabilityAccess) {
        self.id = id
        self.url = url
        self.access = access
    }
}

public protocol FileCapabilityBroker: Sendable {
    func issueCapability(for url: URL, access: FileCapabilityAccess) async throws -> FileCapability
    func beginAccess(to capability: FileCapability) async throws -> FileAccessLease
    func endAccess(_ lease: FileAccessLease) async throws
}

public struct ProcessRequest: Sendable {
    public let id: UUID
    public let executable: URL
    public let arguments: [String]
    public let environment: [String: String]
    public let workingDirectory: URL?
    public let standardInput: Data?

    public init(
        id: UUID,
        executable: URL,
        arguments: [String] = [],
        environment: [String: String] = [:],
        workingDirectory: URL? = nil,
        standardInput: Data? = nil
    ) {
        self.id = id
        self.executable = executable
        self.arguments = arguments
        self.environment = environment
        self.workingDirectory = workingDirectory
        self.standardInput = standardInput
    }
}

public struct ProcessResult: Equatable, Sendable {
    public let terminationStatus: Int32
    public let standardOutput: Data
    public let standardError: Data

    public init(terminationStatus: Int32, standardOutput: Data, standardError: Data) {
        self.terminationStatus = terminationStatus
        self.standardOutput = standardOutput
        self.standardError = standardError
    }
}

public protocol ProcessExecuting: Sendable {
    func execute(_ request: ProcessRequest) async throws -> ProcessResult
    func terminate(id: UUID) async throws
}

public struct PDFPoint: Hashable, Codable, Sendable {
    public let x: Double
    public let y: Double

    public init(x: Double, y: Double) {
        self.x = x
        self.y = y
    }
}

public struct PDFRectangle: Hashable, Codable, Sendable {
    public let origin: PDFPoint
    public let width: Double
    public let height: Double

    public init(origin: PDFPoint, width: Double, height: Double) {
        self.origin = origin
        self.width = width
        self.height = height
    }
}

public enum PDFCoordinateSpace: Hashable, Codable, Sendable {
    case pdfBottomLeft(page: Int, mediaBox: PDFRectangle)
    case viewTopLeft(page: Int, bounds: PDFRectangle)
}

public protocol PDFCoordinateConverting: Sendable {
    func convert(_ point: PDFPoint, from source: PDFCoordinateSpace, to destination: PDFCoordinateSpace) throws -> PDFPoint
}

@MainActor
public protocol WorkspaceOpening: Sendable {
    func openDocument(at url: URL) throws
    func revealInFinder(_ urls: [URL]) throws
}

public protocol WallClock: Sendable {
    func now() -> Date
}

public protocol UUIDGenerating: Sendable {
    func makeUUID() -> UUID
}

public enum LogLevel: String, Codable, Sendable {
    case debug
    case info
    case notice
    case error
    case fault
}

public struct LogRecord: Equatable, Sendable {
    public let level: LogLevel
    public let message: String
    public let metadata: [String: String]

    public init(level: LogLevel, message: String, metadata: [String: String] = [:]) {
        self.level = level
        self.message = message
        self.metadata = metadata
    }
}

public protocol ApplicationLogging: Sendable {
    func log(_ record: LogRecord) throws
}

public struct DocumentSnapshot: Equatable, Sendable {
    public let revision: UInt64
    public let text: String

    public init(revision: UInt64, text: String) {
        self.revision = revision
        self.text = text
    }
}

public struct DocumentTextRange: Equatable, Sendable {
    public let location: Int
    public let length: Int

    public init(location: Int, length: Int) {
        self.location = location
        self.length = length
    }
}

public struct DocumentMutation: Equatable, Sendable {
    public let baseRevision: UInt64
    public let range: DocumentTextRange
    public let replacement: String

    public init(baseRevision: UInt64, range: DocumentTextRange, replacement: String) {
        self.baseRevision = baseRevision
        self.range = range
        self.replacement = replacement
    }
}

public enum DocumentMutationResult: Equatable, Sendable {
    case applied(DocumentSnapshot)
    case rejected(current: DocumentSnapshot)
}

public protocol DocumentSessionPort: Sendable {
    func snapshot() async -> DocumentSnapshot
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult
}
