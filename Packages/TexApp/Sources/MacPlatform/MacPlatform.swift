import AppPorts
import Foundation

#if os(macOS)
import AppKit
import OSLog

public actor MacFileCapabilityBroker: FileCapabilityBroker {
    private var activeURLs: [UUID: URL] = [:]

    public init() {}

    public func issueCapability(for url: URL, access: FileCapabilityAccess) async throws -> FileCapability {
        let fileManager = FileManager.default
        let hasRequestedAccess = switch access {
        case .readOnly:
            fileManager.isReadableFile(atPath: url.path)
        case .readWrite:
            fileManager.isReadableFile(atPath: url.path) && fileManager.isWritableFile(atPath: url.path)
        }
        guard fileManager.fileExists(atPath: url.path), hasRequestedAccess else {
            throw PlatformPortError.accessDenied(url)
        }
        do {
            let bookmark = try url.bookmarkData(
                options: .withSecurityScope,
                includingResourceValuesForKeys: nil,
                relativeTo: nil
            )
            return FileCapability(bookmark: bookmark, access: access)
        } catch {
            throw PlatformPortError.accessDenied(url)
        }
    }

    public func beginAccess(to capability: FileCapability) async throws -> FileAccessLease {
        guard !capability.bookmark.isEmpty else {
            throw PlatformPortError.invalidCapability
        }
        var isStale = false
        let url: URL
        do {
            url = try URL(
                resolvingBookmarkData: capability.bookmark,
                options: [.withSecurityScope, .withoutUI],
                relativeTo: nil,
                bookmarkDataIsStale: &isStale
            )
        } catch {
            throw PlatformPortError.invalidCapability
        }
        guard !isStale else {
            throw PlatformPortError.staleCapability(url)
        }
        guard url.startAccessingSecurityScopedResource() else {
            throw PlatformPortError.accessDenied(url)
        }
        let fileManager = FileManager.default
        let hasRequestedAccess = switch capability.access {
        case .readOnly:
            fileManager.isReadableFile(atPath: url.path)
        case .readWrite:
            fileManager.isReadableFile(atPath: url.path) && fileManager.isWritableFile(atPath: url.path)
        }
        guard hasRequestedAccess else {
            url.stopAccessingSecurityScopedResource()
            throw PlatformPortError.accessDenied(url)
        }
        let lease = FileAccessLease(id: UUID(), url: url, access: capability.access)
        activeURLs[lease.id] = url
        return lease
    }

    public func endAccess(_ lease: FileAccessLease) async throws {
        guard let url = activeURLs.removeValue(forKey: lease.id) else {
            throw PlatformPortError.invalidCapability
        }
        url.stopAccessingSecurityScopedResource()
    }
}

public actor MacProcessExecutor: ProcessExecuting {
    private final class RunningProcess: @unchecked Sendable {
        let process: Process

        init(_ process: Process) {
            self.process = process
        }
    }

    private var running: [UUID: RunningProcess] = [:]

    public init() {}

    public func execute(_ request: ProcessRequest) async throws -> ProcessResult {
        guard running[request.id] == nil else {
            throw PlatformPortError.operationInProgress("process \(request.id)")
        }
        guard FileManager.default.isExecutableFile(atPath: request.executable.path) else {
            throw PlatformPortError.processLaunchFailed(
                executable: request.executable,
                reason: "The executable does not exist or is not executable."
            )
        }

        let temporaryDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("texapp-process-\(request.id.uuidString)", isDirectory: true)
        let outputURL = temporaryDirectory.appendingPathComponent("stdout")
        let errorURL = temporaryDirectory.appendingPathComponent("stderr")
        try FileManager.default.createDirectory(at: temporaryDirectory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: temporaryDirectory) }
        guard FileManager.default.createFile(atPath: outputURL.path, contents: nil),
              FileManager.default.createFile(atPath: errorURL.path, contents: nil) else {
            throw PlatformPortError.processLaunchFailed(
                executable: request.executable,
                reason: "Could not create process output files."
            )
        }

        let outputHandle = try FileHandle(forWritingTo: outputURL)
        let errorHandle = try FileHandle(forWritingTo: errorURL)
        defer {
            try? outputHandle.close()
            try? errorHandle.close()
        }

        let process = Process()
        process.executableURL = request.executable
        process.arguments = request.arguments
        process.environment = request.environment.isEmpty
            ? ProcessInfo.processInfo.environment
            : request.environment
        process.currentDirectoryURL = request.workingDirectory
        process.standardOutput = outputHandle
        process.standardError = errorHandle

        if let input = request.standardInput {
            let pipe = Pipe()
            process.standardInput = pipe
            do {
                try process.run()
                running[request.id] = RunningProcess(process)
                try pipe.fileHandleForWriting.write(contentsOf: input)
                try pipe.fileHandleForWriting.close()
            } catch {
                running.removeValue(forKey: request.id)
                process.terminate()
                throw PlatformPortError.processLaunchFailed(
                    executable: request.executable,
                    reason: error.localizedDescription
                )
            }
        } else {
            do {
                try process.run()
                running[request.id] = RunningProcess(process)
            } catch {
                throw PlatformPortError.processLaunchFailed(
                    executable: request.executable,
                    reason: error.localizedDescription
                )
            }
        }

        let runningProcess = running[request.id]!
        await withTaskCancellationHandler {
            await withCheckedContinuation { continuation in
                DispatchQueue.global(qos: .userInitiated).async {
                    runningProcess.process.waitUntilExit()
                    continuation.resume()
                }
            }
        } onCancel: {
            runningProcess.process.terminate()
        }
        running.removeValue(forKey: request.id)
        try outputHandle.synchronize()
        try errorHandle.synchronize()
        return try ProcessResult(
            terminationStatus: process.terminationStatus,
            standardOutput: Data(contentsOf: outputURL),
            standardError: Data(contentsOf: errorURL)
        )
    }

    public func terminate(id: UUID) async throws {
        guard let process = running[id]?.process else {
            throw PlatformPortError.invalidCapability
        }
        process.terminate()
    }
}

@MainActor
public final class MacWorkspaceOpener: WorkspaceOpening {
    public init() {}

    public func openDocument(at url: URL) throws {
        guard NSWorkspace.shared.open(url) else {
            throw PlatformPortError.accessDenied(url)
        }
    }

    public func revealInFinder(_ urls: [URL]) throws {
        guard !urls.isEmpty else { throw PlatformPortError.invalidCapability }
        NSWorkspace.shared.activateFileViewerSelecting(urls)
    }
}

public struct MacApplicationLogger: ApplicationLogging {
    private let logger: Logger

    public init(subsystem: String, category: String) {
        logger = Logger(subsystem: subsystem, category: category)
    }

    public func log(_ record: LogRecord) throws {
        let metadata = record.metadata
            .sorted { $0.key < $1.key }
            .map { "\($0.key)=\($0.value)" }
            .joined(separator: " ")
        let rendered = metadata.isEmpty ? record.message : "\(record.message) \(metadata)"
        let type: OSLogType = switch record.level {
        case .debug: .debug
        case .info: .info
        case .notice: .default
        case .error: .error
        case .fault: .fault
        }
        logger.log(level: type, "\(rendered, privacy: .public)")
    }
}

#else

public actor MacFileCapabilityBroker: FileCapabilityBroker {
    public init() {}
    public func issueCapability(for url: URL, access: FileCapabilityAccess) async throws -> FileCapability {
        throw PlatformPortError.unavailable(feature: "file capability brokering", platform: "non-macOS")
    }
    public func beginAccess(to capability: FileCapability) async throws -> FileAccessLease {
        throw PlatformPortError.unavailable(feature: "file capability brokering", platform: "non-macOS")
    }
    public func endAccess(_ lease: FileAccessLease) async throws {
        throw PlatformPortError.unavailable(feature: "file capability brokering", platform: "non-macOS")
    }
}

public actor MacProcessExecutor: ProcessExecuting {
    public init() {}
    public func execute(_ request: ProcessRequest) async throws -> ProcessResult {
        throw PlatformPortError.unavailable(feature: "process execution", platform: "non-macOS")
    }
    public func terminate(id: UUID) async throws {
        throw PlatformPortError.unavailable(feature: "process execution", platform: "non-macOS")
    }
}

@MainActor
public final class MacWorkspaceOpener: WorkspaceOpening {
    public init() {}
    public func openDocument(at url: URL) throws {
        throw PlatformPortError.unavailable(feature: "document opening", platform: "non-macOS")
    }
    public func revealInFinder(_ urls: [URL]) throws {
        throw PlatformPortError.unavailable(feature: "Finder reveal", platform: "non-macOS")
    }
}

public struct MacApplicationLogger: ApplicationLogging {
    public init(subsystem: String, category: String) {}
    public func log(_ record: LogRecord) throws {
        throw PlatformPortError.unavailable(feature: "unified application logging", platform: "non-macOS")
    }
}
#endif

public struct MacPDFCoordinateConverter: PDFCoordinateConverting {
    public init() {}

    public func convert(
        _ point: PDFPoint,
        from source: PDFCoordinateSpace,
        to destination: PDFCoordinateSpace
    ) throws -> PDFPoint {
        guard source.page == destination.page else {
            throw PlatformPortError.invalidCapability
        }
        guard source.rectangle.width > 0, source.rectangle.height > 0,
              destination.rectangle.width > 0, destination.rectangle.height > 0 else {
            throw PlatformPortError.invalidCapability
        }
        let normalized = source.toUnitBottomLeft(point)
        return destination.fromUnitBottomLeft(normalized)
    }
}

public struct SystemWallClock: WallClock {
    public init() {}
    public func now() -> Date { Date() }
}

public struct SystemUUIDGenerator: UUIDGenerating {
    public init() {}
    public func makeUUID() -> UUID { UUID() }
}

private extension PDFCoordinateSpace {
    var page: Int {
        switch self {
        case let .pdfBottomLeft(page, _), let .viewTopLeft(page, _): page
        }
    }

    var rectangle: PDFRectangle {
        switch self {
        case let .pdfBottomLeft(_, rectangle), let .viewTopLeft(_, rectangle): rectangle
        }
    }

    func toUnitBottomLeft(_ point: PDFPoint) -> PDFPoint {
        switch self {
        case let .pdfBottomLeft(_, box):
            PDFPoint(
                x: (point.x - box.origin.x) / box.width,
                y: (point.y - box.origin.y) / box.height
            )
        case let .viewTopLeft(_, bounds):
            PDFPoint(
                x: (point.x - bounds.origin.x) / bounds.width,
                y: 1 - ((point.y - bounds.origin.y) / bounds.height)
            )
        }
    }

    func fromUnitBottomLeft(_ point: PDFPoint) -> PDFPoint {
        switch self {
        case let .pdfBottomLeft(_, box):
            PDFPoint(
                x: box.origin.x + point.x * box.width,
                y: box.origin.y + point.y * box.height
            )
        case let .viewTopLeft(_, bounds):
            PDFPoint(
                x: bounds.origin.x + point.x * bounds.width,
                y: bounds.origin.y + (1 - point.y) * bounds.height
            )
        }
    }
}
