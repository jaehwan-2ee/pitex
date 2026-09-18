import AppPorts
import DocumentSessionCore
import ProjectCore

public enum ProjectPermissionState: Sendable {
    case notRequested
    case requesting
    case granted(FileCapability)
    case denied(reason: String)
    case capabilityExpired
}

public enum ProjectOpenState: Sendable {
    case closed
    case awaitingPermission
    case opening
    case open(ProjectGraph)
    case failed(ProjectOpenError)
}

public enum ProjectOpenError: Error, Equatable, Sendable {
    case permissionDenied(String)
    case invalidCapability
    case projectReadFailed(String)
    case documentNotInProject(ProjectFile)
    case documentRouteMismatch
    case staleDocumentSnapshot(currentRevision: UInt64, receivedRevision: UInt64)
}

/// Presentation-only routing metadata. The durable `DocumentSession` remains owned by the core layer.
public struct DocumentSessionRoute: Sendable {
    public let file: ProjectFile
    public private(set) var presentedRevision: UInt64?
    public private(set) var saveState: DocumentSaveState?

    public init(file: ProjectFile) {
        self.file = file
        self.presentedRevision = nil
        self.saveState = nil
    }

    public mutating func present(_ snapshot: DocumentSessionCore.DocumentSnapshot) throws {
        guard snapshot.documentID == file.documentID, snapshot.path == file.path else {
            throw ProjectOpenError.documentRouteMismatch
        }
        if let current = presentedRevision, snapshot.revision < current {
            throw ProjectOpenError.staleDocumentSnapshot(
                currentRevision: current,
                receivedRevision: snapshot.revision
            )
        }
        presentedRevision = snapshot.revision
        saveState = snapshot.saveState
    }
}

public struct ProjectTab: Sendable {
    public private(set) var route: DocumentSessionRoute

    public init(route: DocumentSessionRoute) {
        self.route = route
    }

    public mutating func present(_ snapshot: DocumentSessionCore.DocumentSnapshot) throws {
        try route.present(snapshot)
    }
}

public enum DocumentOpenDisposition: Equatable, Sendable {
    case opened(index: Int)
    case focusedExisting(index: Int)
}

public struct ProjectFeatureState: Sendable {
    public private(set) var permission: ProjectPermissionState
    public private(set) var openState: ProjectOpenState
    public private(set) var tabs: [ProjectTab]
    public private(set) var selectedTabIndex: Int?

    public init() {
        self.permission = .notRequested
        self.openState = .closed
        self.tabs = []
        self.selectedTabIndex = nil
    }

    public mutating func beginPermissionRequest() {
        permission = .requesting
        openState = .awaitingPermission
    }

    public mutating func receivePermission(_ capability: FileCapability) {
        permission = .granted(capability)
        openState = .opening
    }

    public mutating func denyPermission(reason: String) {
        permission = .denied(reason: reason)
        openState = .failed(.permissionDenied(reason))
    }

    public mutating func expireCapability() {
        permission = .capabilityExpired
        openState = .failed(.invalidCapability)
        tabs.removeAll(keepingCapacity: false)
        selectedTabIndex = nil
    }

    public mutating func finishOpening(_ graph: ProjectGraph) throws {
        guard case .granted = permission else {
            openState = .failed(.invalidCapability)
            throw ProjectOpenError.invalidCapability
        }
        openState = .open(graph)
        tabs.removeAll(keepingCapacity: false)
        selectedTabIndex = nil
    }

    public mutating func failOpening(reason: String) {
        openState = .failed(.projectReadFailed(reason))
    }

    @discardableResult
    public mutating func openDocument(_ file: ProjectFile) throws -> DocumentOpenDisposition {
        guard case let .open(graph) = openState,
              graph.files.contains(file) else {
            throw ProjectOpenError.documentNotInProject(file)
        }

        if let existing = tabs.firstIndex(where: {
            $0.route.file.documentID == file.documentID || $0.route.file.path == file.path
        }) {
            selectedTabIndex = existing
            return .focusedExisting(index: existing)
        }

        tabs.append(ProjectTab(route: DocumentSessionRoute(file: file)))
        let index = tabs.index(before: tabs.endIndex)
        selectedTabIndex = index
        return .opened(index: index)
    }

    public mutating func closeTab(at index: Int) {
        guard tabs.indices.contains(index) else { return }
        tabs.remove(at: index)
        if tabs.isEmpty {
            selectedTabIndex = nil
        } else if let selected = selectedTabIndex {
            selectedTabIndex = min(selected > index ? selected - 1 : selected, tabs.count - 1)
        }
    }

    public mutating func selectTab(at index: Int) {
        guard tabs.indices.contains(index) else { return }
        selectedTabIndex = index
    }

    public mutating func present(
        _ snapshot: DocumentSessionCore.DocumentSnapshot,
        inTabAt index: Int
    ) throws {
        guard tabs.indices.contains(index) else {
            throw ProjectOpenError.documentRouteMismatch
        }
        try tabs[index].present(snapshot)
    }
}
