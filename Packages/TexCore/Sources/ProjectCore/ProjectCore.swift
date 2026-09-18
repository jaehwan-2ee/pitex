import TexDomain

public struct ProjectFile: Hashable, Codable, Sendable {
    public let documentID: StableDocumentID
    public let path: NormalizedRelativePath

    public init(documentID: StableDocumentID, path: NormalizedRelativePath) {
        self.documentID = documentID
        self.path = path
    }
}

public struct IncludeEdge: Hashable, Codable, Sendable {
    public let source: StableDocumentID
    public let target: StableDocumentID

    public init(source: StableDocumentID, target: StableDocumentID) {
        self.source = source
        self.target = target
    }
}

public enum ProjectGraphConstructionError: Error, Equatable, Sendable {
    case duplicateDocumentID(StableDocumentID)
    case duplicatePath(NormalizedRelativePath)
}

public enum ProjectGraphDiagnostic: Hashable, Sendable {
    case missingReference(
        source: StableDocumentID,
        target: StableDocumentID,
        missing: StableDocumentID
    )
    case includeCycle(documents: [StableDocumentID])
}

public struct ProjectGraph: Sendable {
    public let projectID: StableProjectID
    public let files: [ProjectFile]
    public let includeEdges: [IncludeEdge]

    public init(
        projectID: StableProjectID,
        files: [ProjectFile],
        includeEdges: [IncludeEdge]
    ) throws {
        let sortedFiles = files.sorted {
            if $0.documentID != $1.documentID {
                return $0.documentID < $1.documentID
            }
            return $0.path < $1.path
        }
        var documentIDs: Set<StableDocumentID> = []
        var paths: Set<NormalizedRelativePath> = []
        for file in sortedFiles {
            guard documentIDs.insert(file.documentID).inserted else {
                throw ProjectGraphConstructionError.duplicateDocumentID(file.documentID)
            }
            guard paths.insert(file.path).inserted else {
                throw ProjectGraphConstructionError.duplicatePath(file.path)
            }
        }

        self.projectID = projectID
        self.files = sortedFiles
        self.includeEdges = includeEdges.sorted {
            if $0.source != $1.source {
                return $0.source < $1.source
            }
            return $0.target < $1.target
        }
    }

    public func diagnostics() -> [ProjectGraphDiagnostic] {
        let knownDocuments = Set(files.map(\.documentID))
        var result: [ProjectGraphDiagnostic] = []

        for edge in includeEdges {
            if !knownDocuments.contains(edge.source) {
                result.append(
                    .missingReference(
                        source: edge.source,
                        target: edge.target,
                        missing: edge.source
                    )
                )
            }
            if !knownDocuments.contains(edge.target) {
                result.append(
                    .missingReference(
                        source: edge.source,
                        target: edge.target,
                        missing: edge.target
                    )
                )
            }
        }

        var adjacency: [StableDocumentID: [StableDocumentID]] = [:]
        var reverseAdjacency: [StableDocumentID: [StableDocumentID]] = [:]
        for document in knownDocuments {
            adjacency[document] = []
            reverseAdjacency[document] = []
        }
        for edge in includeEdges
        where knownDocuments.contains(edge.source) && knownDocuments.contains(edge.target) {
            adjacency[edge.source, default: []].append(edge.target)
            reverseAdjacency[edge.target, default: []].append(edge.source)
        }
        for document in knownDocuments {
            adjacency[document]?.sort()
            reverseAdjacency[document]?.sort()
        }

        var visited: Set<StableDocumentID> = []
        var finishOrder: [StableDocumentID] = []
        func visitForward(_ document: StableDocumentID) {
            guard visited.insert(document).inserted else { return }
            for target in adjacency[document, default: []] {
                visitForward(target)
            }
            finishOrder.append(document)
        }
        for document in knownDocuments.sorted() {
            visitForward(document)
        }

        visited.removeAll(keepingCapacity: true)
        func visitReverse(_ document: StableDocumentID, component: inout [StableDocumentID]) {
            guard visited.insert(document).inserted else { return }
            component.append(document)
            for source in reverseAdjacency[document, default: []] {
                visitReverse(source, component: &component)
            }
        }
        for document in finishOrder.reversed() {
            guard !visited.contains(document) else { continue }
            var component: [StableDocumentID] = []
            visitReverse(document, component: &component)
            component.sort()
            let isSelfCycle = component.count == 1
                && adjacency[component[0], default: []].contains(component[0])
            if component.count > 1 || isSelfCycle {
                result.append(.includeCycle(documents: component))
            }
        }

        return result.sorted { diagnosticSortKey($0) < diagnosticSortKey($1) }
    }
}

private func diagnosticSortKey(_ diagnostic: ProjectGraphDiagnostic) -> String {
    switch diagnostic {
    case let .missingReference(source, target, missing):
        return "0|\(source.rawValue)|\(target.rawValue)|\(missing.rawValue)"
    case let .includeCycle(documents):
        return "1|" + documents.map(\.rawValue).joined(separator: "|")
    }
}
