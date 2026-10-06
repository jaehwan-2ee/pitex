import TexDomain

public struct ProjectRootIdentity: Hashable, Codable, Sendable {
    public let projectID: StableProjectID

    public init(projectID: StableProjectID) {
        self.projectID = projectID
    }
}

public struct ProjectGraphSnapshot: Hashable, Codable, Sendable {
    public let files: [ProjectFile]
    public let includeEdges: [IncludeEdge]

    public init(files: [ProjectFile], includeEdges: [IncludeEdge]) throws {
        let sortedFiles = files.sorted {
            if $0.documentID != $1.documentID { return $0.documentID < $1.documentID }
            return $0.path < $1.path
        }
        var documentIDs: Set<StableDocumentID> = []
        var paths: Set<NormalizedRelativePath> = []
        for file in sortedFiles {
            guard documentIDs.insert(file.documentID).inserted else {
                throw ProjectWorkspaceError.duplicateDocumentID(file.documentID)
            }
            guard paths.insert(file.path).inserted else {
                throw ProjectWorkspaceError.duplicatePath(file.path)
            }
        }
        self.files = sortedFiles
        self.includeEdges = includeEdges.sorted {
            if $0.source != $1.source { return $0.source < $1.source }
            return $0.target < $1.target
        }
    }

    public init(graph: ProjectGraph) throws {
        try self.init(files: graph.files, includeEdges: graph.includeEdges)
    }

    private enum CodingKeys: String, CodingKey {
        case files, includeEdges
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            files: container.decode([ProjectFile].self, forKey: .files),
            includeEdges: container.decode([IncludeEdge].self, forKey: .includeEdges)
        )
    }

    public func projectGraph(projectID: StableProjectID) throws -> ProjectGraph {
        try ProjectGraph(projectID: projectID, files: files, includeEdges: includeEdges)
    }

    public func diagnostics(projectID: StableProjectID) throws -> [ProjectGraphDiagnostic] {
        try projectGraph(projectID: projectID).diagnostics()
    }
}

public struct BuildCommandPreference: Hashable, Codable, Sendable {
    public let executable: String
    public let arguments: [String]

    public init(executable: String, arguments: [String] = []) throws {
        guard !executable.isEmpty else { throw ProjectWorkspaceError.emptyBuildCommand }
        self.executable = executable
        self.arguments = arguments
    }

    private enum CodingKeys: String, CodingKey {
        case executable, arguments
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            executable: container.decode(String.self, forKey: .executable),
            arguments: container.decode([String].self, forKey: .arguments)
        )
    }
}

public struct ProjectBuildTarget: Hashable, Codable, Sendable {
    public let id: String
    public let documentID: StableDocumentID
    public let command: BuildCommandPreference

    public init(id: String, documentID: StableDocumentID, command: BuildCommandPreference) throws {
        guard !id.isEmpty else { throw ProjectWorkspaceError.emptyBuildTargetID }
        self.id = id
        self.documentID = documentID
        self.command = command
    }

    private enum CodingKeys: String, CodingKey {
        case id, documentID, command
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            id: container.decode(String.self, forKey: .id),
            documentID: container.decode(StableDocumentID.self, forKey: .documentID),
            command: container.decode(BuildCommandPreference.self, forKey: .command)
        )
    }
}

public struct ExternalRevisionMarker: Hashable, Codable, Sendable {
    public let documentID: StableDocumentID
    public let revision: String

    public init(documentID: StableDocumentID, revision: String) throws {
        guard !revision.isEmpty else { throw ProjectWorkspaceError.emptyExternalRevision }
        self.documentID = documentID
        self.revision = revision
    }

    private enum CodingKeys: String, CodingKey {
        case documentID, revision
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            documentID: container.decode(StableDocumentID.self, forKey: .documentID),
            revision: container.decode(String.self, forKey: .revision)
        )
    }
}

public enum ProjectWorkspaceError: Error, Equatable, Sendable {
    case unsupportedSchemaVersion(found: Int, supported: Int)
    case duplicateDocumentID(StableDocumentID)
    case duplicatePath(NormalizedRelativePath)
    case duplicateTab(StableDocumentID)
    case tabOutsideGraph(StableDocumentID)
    case invalidActiveTab(StableDocumentID)
    case duplicateBuildTarget(String)
    case unknownBuildTarget(String)
    case buildTargetOutsideGraph(StableDocumentID)
    case duplicateExternalRevision(StableDocumentID)
    case externalRevisionOutsideGraph(StableDocumentID)
    case emptyBuildTargetID
    case emptyBuildCommand
    case emptyExternalRevision
}

public struct ProjectWorkspaceRecord: Hashable, Codable, Sendable {
    public static let currentSchemaVersion = 1

    public let schemaVersion: Int
    public let root: ProjectRootIdentity
    public let graph: ProjectGraphSnapshot
    public let tabs: [StableDocumentID]
    public let activeTab: StableDocumentID?
    public let buildTargets: [ProjectBuildTarget]
    public let selectedBuildTargetID: String?
    public let externalRevisions: [ExternalRevisionMarker]

    public init(
        root: ProjectRootIdentity,
        graph: ProjectGraphSnapshot,
        tabs: [StableDocumentID],
        activeTab: StableDocumentID?,
        buildTargets: [ProjectBuildTarget] = [],
        selectedBuildTargetID: String? = nil,
        externalRevisions: [ExternalRevisionMarker] = []
    ) throws {
        try self.init(
            schemaVersion: Self.currentSchemaVersion,
            root: root,
            graph: graph,
            tabs: tabs,
            activeTab: activeTab,
            buildTargets: buildTargets,
            selectedBuildTargetID: selectedBuildTargetID,
            externalRevisions: externalRevisions
        )
    }

    private init(
        schemaVersion: Int,
        root: ProjectRootIdentity,
        graph: ProjectGraphSnapshot,
        tabs: [StableDocumentID],
        activeTab: StableDocumentID?,
        buildTargets: [ProjectBuildTarget],
        selectedBuildTargetID: String?,
        externalRevisions: [ExternalRevisionMarker]
    ) throws {
        guard schemaVersion == Self.currentSchemaVersion else {
            throw ProjectWorkspaceError.unsupportedSchemaVersion(
                found: schemaVersion,
                supported: Self.currentSchemaVersion
            )
        }
        let knownDocuments = Set(graph.files.map(\.documentID))
        var tabSet: Set<StableDocumentID> = []
        for tab in tabs {
            guard knownDocuments.contains(tab) else { throw ProjectWorkspaceError.tabOutsideGraph(tab) }
            guard tabSet.insert(tab).inserted else { throw ProjectWorkspaceError.duplicateTab(tab) }
        }
        if let activeTab, !tabSet.contains(activeTab) {
            throw ProjectWorkspaceError.invalidActiveTab(activeTab)
        }

        let sortedTargets = buildTargets.sorted { $0.id < $1.id }
        var targetIDs: Set<String> = []
        for target in sortedTargets {
            guard targetIDs.insert(target.id).inserted else {
                throw ProjectWorkspaceError.duplicateBuildTarget(target.id)
            }
            guard knownDocuments.contains(target.documentID) else {
                throw ProjectWorkspaceError.buildTargetOutsideGraph(target.documentID)
            }
        }
        if let selectedBuildTargetID, !targetIDs.contains(selectedBuildTargetID) {
            throw ProjectWorkspaceError.unknownBuildTarget(selectedBuildTargetID)
        }

        let sortedRevisions = externalRevisions.sorted { $0.documentID < $1.documentID }
        var revisionDocuments: Set<StableDocumentID> = []
        for marker in sortedRevisions {
            guard knownDocuments.contains(marker.documentID) else {
                throw ProjectWorkspaceError.externalRevisionOutsideGraph(marker.documentID)
            }
            guard revisionDocuments.insert(marker.documentID).inserted else {
                throw ProjectWorkspaceError.duplicateExternalRevision(marker.documentID)
            }
        }

        self.schemaVersion = schemaVersion
        self.root = root
        self.graph = graph
        self.tabs = tabs
        self.activeTab = activeTab
        self.buildTargets = sortedTargets
        self.selectedBuildTargetID = selectedBuildTargetID
        self.externalRevisions = sortedRevisions
    }

    private enum CodingKeys: String, CodingKey {
        case schemaVersion, root, graph, tabs, activeTab, buildTargets
        case selectedBuildTargetID, externalRevisions
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            schemaVersion: container.decode(Int.self, forKey: .schemaVersion),
            root: container.decode(ProjectRootIdentity.self, forKey: .root),
            graph: container.decode(ProjectGraphSnapshot.self, forKey: .graph),
            tabs: container.decode([StableDocumentID].self, forKey: .tabs),
            activeTab: container.decodeIfPresent(StableDocumentID.self, forKey: .activeTab),
            buildTargets: container.decode([ProjectBuildTarget].self, forKey: .buildTargets),
            selectedBuildTargetID: container.decodeIfPresent(String.self, forKey: .selectedBuildTargetID),
            externalRevisions: container.decode([ExternalRevisionMarker].self, forKey: .externalRevisions)
        )
    }
}

public enum ProjectWorkspaceRestorationDiagnostic: Hashable, Sendable {
    case missingDocument(documentID: StableDocumentID, path: NormalizedRelativePath)
}

public struct ProjectWorkspaceRestoration: Sendable {
    public let workspace: ProjectWorkspace
    public let persistedRecord: ProjectWorkspaceRecord
    public let diagnostics: [ProjectWorkspaceRestorationDiagnostic]
}

public struct ProjectWorkspace: Hashable, Sendable {
    public private(set) var record: ProjectWorkspaceRecord

    public init(record: ProjectWorkspaceRecord) {
        self.record = record
    }

    public mutating func open(_ documentID: StableDocumentID) throws {
        guard record.graph.files.contains(where: { $0.documentID == documentID }) else {
            throw ProjectWorkspaceError.tabOutsideGraph(documentID)
        }
        var tabs = record.tabs
        if !tabs.contains(documentID) { tabs.append(documentID) }
        try replace(tabs: tabs, activeTab: documentID)
    }

    public mutating func close(_ documentID: StableDocumentID) throws {
        var tabs = record.tabs
        guard let index = tabs.firstIndex(of: documentID) else { return }
        tabs.remove(at: index)
        var active = record.activeTab
        if active == documentID {
            active = tabs.isEmpty ? nil : tabs[min(index, tabs.count - 1)]
        }
        try replace(tabs: tabs, activeTab: active)
    }

    public mutating func activate(_ documentID: StableDocumentID) throws {
        guard record.tabs.contains(documentID) else {
            throw ProjectWorkspaceError.invalidActiveTab(documentID)
        }
        try replace(tabs: record.tabs, activeTab: documentID)
    }

    public mutating func selectBuildTarget(_ id: String) throws {
        guard record.buildTargets.contains(where: { $0.id == id }) else {
            throw ProjectWorkspaceError.unknownBuildTarget(id)
        }
        try replace(selectedBuildTargetID: id)
    }

    public static func restore(
        _ persistedRecord: ProjectWorkspaceRecord,
        availablePaths: Set<NormalizedRelativePath>
    ) -> ProjectWorkspaceRestoration {
        let diagnostics: [ProjectWorkspaceRestorationDiagnostic] =
            persistedRecord.graph.files.compactMap { file in
            availablePaths.contains(file.path) ? nil : .missingDocument(
                documentID: file.documentID,
                path: file.path
            )
        }
        return ProjectWorkspaceRestoration(
            workspace: ProjectWorkspace(record: persistedRecord),
            persistedRecord: persistedRecord,
            diagnostics: diagnostics
        )
    }

    private mutating func replace(
        tabs: [StableDocumentID]? = nil,
        activeTab: StableDocumentID?? = nil,
        selectedBuildTargetID: String?? = nil
    ) throws {
        record = try ProjectWorkspaceRecord(
            root: record.root,
            graph: record.graph,
            tabs: tabs ?? record.tabs,
            activeTab: activeTab ?? record.activeTab,
            buildTargets: record.buildTargets,
            selectedBuildTargetID: selectedBuildTargetID ?? record.selectedBuildTargetID,
            externalRevisions: record.externalRevisions
        )
    }
}
