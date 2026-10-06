public enum SettingsValidationError: Error, Equatable, Sendable {
    case editorFontSizeOutOfRange
    case editorTabWidthOutOfRange
    case buildPassLimitOutOfRange
    case emptyCustomShell
    case invalidCustomShell
}

public enum SettingsKey: String, CaseIterable, Codable, Sendable {
    case editor
    case build
    case pdf
    case diagnosticsConsent
    case project
}

public struct EditorPreferences: Codable, Equatable, Sendable {
    public let fontSize: UInt8
    public let tabWidth: UInt8
    public let wrapsLines: Bool
    public let completesDelimiters: Bool

    public init(
        fontSize: UInt8 = 14,
        tabWidth: UInt8 = 4,
        wrapsLines: Bool = true,
        completesDelimiters: Bool = true
    ) throws {
        guard (8...72).contains(fontSize) else {
            throw SettingsValidationError.editorFontSizeOutOfRange
        }
        guard (1...16).contains(tabWidth) else {
            throw SettingsValidationError.editorTabWidthOutOfRange
        }
        self.fontSize = fontSize
        self.tabWidth = tabWidth
        self.wrapsLines = wrapsLines
        self.completesDelimiters = completesDelimiters
    }

    public static var safeDefaults: Self { try! Self() }
}

public enum TeXEnginePreference: String, Codable, CaseIterable, Sendable {
    case pdfLaTeX
    case xeLaTeX
    case luaLaTeX
}

public enum ShellExecutionPreference: Codable, Equatable, Sendable {
    case disabled
    case custom(command: String)
}

public struct BuildPreferences: Codable, Equatable, Sendable {
    public let engine: TeXEnginePreference
    public let maximumPasses: UInt8
    public let stopsAfterFirstError: Bool
    public let shellExecution: ShellExecutionPreference
    public let customShellAcknowledged: Bool

    public init(
        engine: TeXEnginePreference = .pdfLaTeX,
        maximumPasses: UInt8 = 3,
        stopsAfterFirstError: Bool = false,
        shellExecution: ShellExecutionPreference = .disabled,
        customShellAcknowledged: Bool = false
    ) throws {
        guard (1...10).contains(maximumPasses) else {
            throw SettingsValidationError.buildPassLimitOutOfRange
        }
        if case let .custom(command) = shellExecution {
            guard !command.isEmpty else { throw SettingsValidationError.emptyCustomShell }
            guard !command.contains("\0") else { throw SettingsValidationError.invalidCustomShell }
        }
        self.engine = engine
        self.maximumPasses = maximumPasses
        self.stopsAfterFirstError = stopsAfterFirstError
        self.shellExecution = shellExecution
        self.customShellAcknowledged = customShellAcknowledged
    }

    public static var safeDefaults: Self { try! Self() }

    public func selectingShellExecution(_ selection: ShellExecutionPreference) throws -> Self {
        try Self(
            engine: engine,
            maximumPasses: maximumPasses,
            stopsAfterFirstError: stopsAfterFirstError,
            shellExecution: selection,
            customShellAcknowledged: false
        )
    }

    public func acknowledgingCustomShell() throws -> Self {
        guard case .custom = shellExecution else {
            return try Self(
                engine: engine,
                maximumPasses: maximumPasses,
                stopsAfterFirstError: stopsAfterFirstError,
                shellExecution: shellExecution,
                customShellAcknowledged: false
            )
        }
        return try Self(
            engine: engine,
            maximumPasses: maximumPasses,
            stopsAfterFirstError: stopsAfterFirstError,
            shellExecution: shellExecution,
            customShellAcknowledged: true
        )
    }
}

public enum PDFPageLayout: String, Codable, CaseIterable, Sendable {
    case singlePage
    case continuous
    case twoUp
}

public struct PDFPreferences: Codable, Equatable, Sendable {
    public let autoReloadAfterBuild: Bool
    public let highlightsSyncLocation: Bool
    public let pageLayout: PDFPageLayout

    public init(
        autoReloadAfterBuild: Bool = true,
        highlightsSyncLocation: Bool = true,
        pageLayout: PDFPageLayout = .continuous
    ) {
        self.autoReloadAfterBuild = autoReloadAfterBuild
        self.highlightsSyncLocation = highlightsSyncLocation
        self.pageLayout = pageLayout
    }

    public static let safeDefaults = Self()
}

public enum ConsentPreference: String, Codable, CaseIterable, Sendable {
    case notGranted
    case granted
}

public struct DiagnosticsConsentPreferences: Codable, Equatable, Sendable {
    public let evidenceLogging: ConsentPreference
    public let debugLogging: ConsentPreference

    public init(
        evidenceLogging: ConsentPreference = .notGranted,
        debugLogging: ConsentPreference = .notGranted
    ) {
        self.evidenceLogging = evidenceLogging
        self.debugLogging = debugLogging
    }

    public static let safeDefaults = Self()
}

public enum RootFileBehavior: String, Codable, CaseIterable, Sendable {
    case askEveryTime
    case useLastSuccessful
    case discoverFromDocument
}

public struct ProjectPreferences: Codable, Equatable, Sendable {
    public let restoresLastProject: Bool
    public let autosavesDocuments: Bool
    public let rootFileBehavior: RootFileBehavior

    public init(
        restoresLastProject: Bool = false,
        autosavesDocuments: Bool = true,
        rootFileBehavior: RootFileBehavior = .askEveryTime
    ) {
        self.restoresLastProject = restoresLastProject
        self.autosavesDocuments = autosavesDocuments
        self.rootFileBehavior = rootFileBehavior
    }

    public static let safeDefaults = Self()
}

public struct PersistedSettings: Codable, Equatable, Sendable {
    public let editor: EditorPreferences
    public let build: BuildPreferences
    public let pdf: PDFPreferences
    public let diagnosticsConsent: DiagnosticsConsentPreferences
    public let project: ProjectPreferences

    public init(
        editor: EditorPreferences = .safeDefaults,
        build: BuildPreferences = .safeDefaults,
        pdf: PDFPreferences = .safeDefaults,
        diagnosticsConsent: DiagnosticsConsentPreferences = .safeDefaults,
        project: ProjectPreferences = .safeDefaults
    ) {
        self.editor = editor
        self.build = build
        self.pdf = pdf
        self.diagnosticsConsent = diagnosticsConsent
        self.project = project
    }

    public static let safeDefaults = Self()
}

public enum CapabilityAvailability: Equatable, Sendable {
    case available
    case unavailable(reason: String)
}

public struct SettingsRuntimeStatus: Equatable, Sendable {
    public let buildCapability: CapabilityAvailability
    public let pdfCapability: CapabilityAvailability

    public init(
        buildCapability: CapabilityAvailability,
        pdfCapability: CapabilityAvailability
    ) {
        self.buildCapability = buildCapability
        self.pdfCapability = pdfCapability
    }
}

public struct SettingsState: Equatable, Sendable {
    public let preferences: PersistedSettings
    public let runtime: SettingsRuntimeStatus

    public init(preferences: PersistedSettings, runtime: SettingsRuntimeStatus) {
        self.preferences = preferences
        self.runtime = runtime
    }
}
