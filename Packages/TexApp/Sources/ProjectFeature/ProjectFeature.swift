import AppPorts
import DocumentSessionCore
import Foundation
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

/// One node of the hierarchical project file list. `path` is the
/// project-relative path ("/" separated); directories carry their children.
public struct ProjectFileNode: Equatable, Identifiable, Sendable {
    public let path: String
    public let name: String
    public let isDirectory: Bool
    public let children: [ProjectFileNode]?

    public var id: String { path }

    public init(path: String, name: String, isDirectory: Bool, children: [ProjectFileNode]?) {
        self.path = path
        self.name = name
        self.isDirectory = isDirectory
        self.children = children
    }
}

/// Builds the directory tree shown in the project sidebar from flat
/// project-relative paths. Directories sort before files; siblings order
/// lexicographically by name so both platforms render identically.
public func buildProjectFileTree(relativePaths: [String]) -> [ProjectFileNode] {
    var roots: [ProjectFileNode] = []
    for path in relativePaths {
        let components = path.split(separator: "/").map(String.init)
        guard !components.isEmpty else { continue }
        insert(components: components[...], prefix: "", into: &roots)
    }
    return sorted(roots)
}

private func insert(components: ArraySlice<String>, prefix: String, into nodes: inout [ProjectFileNode]) {
    guard let head = components.first else { return }
    let nodePath = prefix.isEmpty ? head : "\(prefix)/\(head)"
    if components.count == 1 {
        nodes.append(ProjectFileNode(path: nodePath, name: head, isDirectory: false, children: nil))
        return
    }
    if let index = nodes.firstIndex(where: { $0.isDirectory && $0.name == head }) {
        var children = nodes[index].children ?? []
        insert(components: components.dropFirst(), prefix: nodePath, into: &children)
        nodes[index] = ProjectFileNode(path: nodePath, name: head, isDirectory: true, children: children)
    } else {
        var children: [ProjectFileNode] = []
        insert(components: components.dropFirst(), prefix: nodePath, into: &children)
        nodes.append(ProjectFileNode(path: nodePath, name: head, isDirectory: true, children: children))
    }
}

private func sorted(_ nodes: [ProjectFileNode]) -> [ProjectFileNode] {
    nodes.sorted { lhs, rhs in
        if lhs.isDirectory != rhs.isDirectory { return lhs.isDirectory }
        return lhs.name.localizedStandardCompare(rhs.name) == .orderedAscending
    }.map { node in
        guard node.isDirectory else { return node }
        return ProjectFileNode(path: node.path, name: node.name, isDirectory: true, children: sorted(node.children ?? []))
    }
}

/// `nest_project_children` — reorders the built tree so the main document leads
/// the root list and its direct dependencies (bibliographies, included
/// chapters) nest one level beneath it. Missing `main` returns the tree
/// unchanged; directories left empty by a move are pruned.
public func nestProjectChildren(_ tree: [ProjectFileNode], main: String, children: [String]) -> [ProjectFileNode] {
    var tree = tree
    guard var mainNode = removeNode(path: main, from: &tree) else { return tree }
    var nested = mainNode.children ?? []
    for child in children {
        if let node = removeNode(path: child, from: &tree) {
            nested.append(node)
        }
    }
    mainNode = ProjectFileNode(path: mainNode.path, name: mainNode.name,
                               isDirectory: mainNode.isDirectory,
                               children: nested.isEmpty ? nil : nested)
    tree.insert(mainNode, at: 0)
    return tree
}

/// Detaches the node with `path` wherever it sits, pruning directory nodes
/// left empty by the removal.
private func removeNode(path: String, from nodes: inout [ProjectFileNode]) -> ProjectFileNode? {
    if let index = nodes.firstIndex(where: { $0.path == path }) {
        return nodes.remove(at: index)
    }
    for index in nodes.indices {
        var children = nodes[index].children ?? []
        guard let found = removeNode(path: path, from: &children) else { continue }
        if children.isEmpty {
            nodes.remove(at: index)
        } else {
            nodes[index] = ProjectFileNode(path: nodes[index].path, name: nodes[index].name,
                                           isDirectory: true, children: children)
        }
        return found
    }
    return nil
}
