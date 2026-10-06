import AppPorts
import DocumentSessionCore
import SyncTeXCore

public struct PDFPreviewArtifact: Hashable, Codable, Sendable {
    public let path: NormalizedSourcePath
    public let syncRevision: SyncTeXRevision
    public let sourceContentHash: DiskContentHash

    public init(
        path: NormalizedSourcePath,
        syncRevision: SyncTeXRevision,
        sourceContentHash: DiskContentHash
    ) {
        self.path = path
        self.syncRevision = syncRevision
        self.sourceContentHash = sourceContentHash
    }
}

public enum PDFPreviewState: Hashable, Codable, Sendable {
    case unavailable
    case loading(PDFPreviewArtifact)
    case ready(PDFPreviewArtifact, pageCount: Int, visiblePage: Int)
    case failed(message: String)
}

public enum SyncTeXNavigationRequest: Hashable, Codable, Sendable {
    case forward(ForwardSyncQuery)
    case inverse(InverseSyncQuery)

    public var revision: SyncTeXRevision {
        switch self {
        case let .forward(query): query.revision
        case let .inverse(query): query.revision
        }
    }
}

public enum SyncTeXNavigationState: Hashable, Codable, Sendable {
    case idle
    case verifying(SyncTeXNavigationRequest)
    case matched(SyncTeXMatch)
    case stale(requested: SyncTeXRevision, available: [SyncTeXRevision])
    case ambiguous(matchCount: Int)
    case noMatch
    case failed(message: String)
}

public struct PDFFeatureState: Sendable {
    public private(set) var preview: PDFPreviewState
    public private(set) var navigation: SyncTeXNavigationState

    public init() {
        self.preview = .unavailable
        self.navigation = .idle
    }

    public mutating func beginLoading(_ artifact: PDFPreviewArtifact) {
        preview = .loading(artifact)
        navigation = .idle
    }

    public mutating func finishLoading(
        _ artifact: PDFPreviewArtifact,
        pageCount: Int,
        visiblePage: Int
    ) {
        guard pageCount > 0, (1...pageCount).contains(visiblePage) else {
            preview = .failed(message: "The generated PDF has invalid page metadata.")
            return
        }
        preview = .ready(artifact, pageCount: pageCount, visiblePage: visiblePage)
    }

    public mutating func failLoading(message: String) {
        preview = .failed(message: message)
        navigation = .idle
    }

    public mutating func showPage(_ page: Int) {
        guard case let .ready(artifact, pageCount, _) = preview,
              (1...pageCount).contains(page) else { return }
        preview = .ready(artifact, pageCount: pageCount, visiblePage: page)
    }

    @discardableResult
    public mutating func verifyForward(
        _ query: ForwardSyncQuery,
        candidates: [SyncTeXMatch]
    ) -> SyncTeXMatch? {
        navigation = .verifying(.forward(query))
        return verify(queryRevision: query.revision, candidates: candidates) {
            try ExactMatchSelector.forward(candidates, query: query)
        }
    }

    @discardableResult
    public mutating func verifyInverse(
        _ query: InverseSyncQuery,
        candidates: [SyncTeXMatch]
    ) -> SyncTeXMatch? {
        navigation = .verifying(.inverse(query))
        return verify(queryRevision: query.revision, candidates: candidates) {
            try ExactMatchSelector.inverse(candidates, query: query)
        }
    }

    private mutating func verify(
        queryRevision: SyncTeXRevision,
        candidates: [SyncTeXMatch],
        selector: () throws -> SyncTeXMatch
    ) -> SyncTeXMatch? {
        let available = Array(Set(candidates.map(\.revision))).sorted {
            if $0.buildID != $1.buildID { return $0.buildID < $1.buildID }
            return $0.fingerprint < $1.fingerprint
        }
        if !candidates.isEmpty, !candidates.contains(where: { $0.revision == queryRevision }) {
            navigation = .stale(requested: queryRevision, available: available)
            return nil
        }

        do {
            let match = try selector()
            navigation = .matched(match)
            return match
        } catch SyncTeXError.noMatch {
            navigation = .noMatch
        } catch let SyncTeXError.ambiguousMatch(count) {
            navigation = .ambiguous(matchCount: count)
        } catch SyncTeXError.staleResult {
            navigation = .stale(requested: queryRevision, available: available)
        } catch {
            navigation = .failed(message: String(describing: error))
        }
        return nil
    }
}

public struct PDFPointConversionRequest: Hashable, Codable, Sendable {
    public let point: AppPorts.PDFPoint
    public let source: PDFCoordinateSpace
    public let destination: PDFCoordinateSpace

    public init(
        point: AppPorts.PDFPoint,
        source: PDFCoordinateSpace,
        destination: PDFCoordinateSpace
    ) {
        self.point = point
        self.source = source
        self.destination = destination
    }
}
