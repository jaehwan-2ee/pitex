import AppPorts
import BuildFeature
import EditorFeature
import EditorMacAdapter
import MacPlatform
import PDFFeature
import ProjectFeature
import SettingsFeature

public enum AppShellError: Error, Equatable, Sendable {
    case platformUnavailable(required: String, detected: String)
}

@MainActor
public struct AppEnvironment {
    public let files: any FileCapabilityBroker
    public let processes: any ProcessExecuting
    public let pdfCoordinates: any PDFCoordinateConverting
    public let workspace: any WorkspaceOpening
    public let clock: any WallClock
    public let uuids: any UUIDGenerating
    public let logger: any ApplicationLogging
    public let editor: EditorMacAdapter

    init(
        files: any FileCapabilityBroker,
        processes: any ProcessExecuting,
        pdfCoordinates: any PDFCoordinateConverting,
        workspace: any WorkspaceOpening,
        clock: any WallClock,
        uuids: any UUIDGenerating,
        logger: any ApplicationLogging,
        editor: EditorMacAdapter
    ) {
        self.files = files
        self.processes = processes
        self.pdfCoordinates = pdfCoordinates
        self.workspace = workspace
        self.clock = clock
        self.uuids = uuids
        self.logger = logger
        self.editor = editor
    }
}

@MainActor
public enum AppShell {
    public static func make(
        documentSession: any DocumentSessionPort,
        loggingSubsystem: String = "app.pitex.desktop"
    ) async throws -> AppEnvironment {
        #if os(macOS)
        let editor = try await EditorMacAdapter.make(session: documentSession)
        return AppEnvironment(
            files: MacFileCapabilityBroker(),
            processes: MacProcessExecutor(),
            pdfCoordinates: MacPDFCoordinateConverter(),
            workspace: MacWorkspaceOpener(),
            clock: SystemWallClock(),
            uuids: SystemUUIDGenerator(),
            logger: MacApplicationLogger(subsystem: loggingSubsystem, category: "application"),
            editor: editor
        )
        #else
        throw AppShellError.platformUnavailable(required: "macOS 15 or later", detected: "non-macOS")
        #endif
    }
}
