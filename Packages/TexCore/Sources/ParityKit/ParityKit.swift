public enum ParityValidationError: Error, Equatable, Sendable {
    case invalidStableID
    case updaterExclusionRequired
    case updaterExclusionForbidden
    case invalidChecksum
    case missingEvidence(String)
    case staleEvidence(String)
    case provisionalCoverage(String)
    case duplicateID(String)
}

public struct ParityStableID: Hashable, Codable, Sendable, Comparable {
    public let rawValue: String
    public init(_ rawValue: String) throws {
        guard !rawValue.isEmpty, rawValue.utf8.count <= 128,
              rawValue.utf8.first.map({ (97...122).contains($0) }) == true,
              rawValue.utf8.allSatisfy({ $0 == 45 || $0 == 46 || $0 == 95 || (48...57).contains($0) || (97...122).contains($0) })
        else { throw ParityValidationError.invalidStableID }
        self.rawValue = rawValue
    }
    public static func < (lhs: Self, rhs: Self) -> Bool { lhs.rawValue < rhs.rawValue }
}

public enum ManifestTaxonomy: String, CaseIterable, Codable, Sendable {
    case applicationShell
    case documentEditing
    case projectManagement
    case typesetting
    case bibliography
    case pdfPreview
    case syncTeX
    case artificialIntelligence
    case preferences
    case accessibility
    case updater
}

public enum ParityDisposition: Hashable, Codable, Sendable {
    case required
    case excluded(reason: ExclusionReason)
}
public enum ExclusionReason: String, Codable, Sendable { case updaterOutOfScope }

public struct ManifestItem: Hashable, Codable, Sendable {
    public let id: ParityStableID
    public let taxonomy: ManifestTaxonomy
    public let title: String
    public let disposition: ParityDisposition

    public init(id: ParityStableID, taxonomy: ManifestTaxonomy, title: String, disposition: ParityDisposition) throws {
        guard !title.isEmpty else { throw ParityValidationError.invalidStableID }
        if taxonomy == .updater {
            guard disposition == .excluded(reason: .updaterOutOfScope) else { throw ParityValidationError.updaterExclusionRequired }
        } else if case .excluded = disposition {
            throw ParityValidationError.updaterExclusionForbidden
        }
        self.id = id; self.taxonomy = taxonomy; self.title = title; self.disposition = disposition
    }
}

public struct EvidenceChecksum: Hashable, Codable, Sendable {
    public let algorithm: Algorithm
    public let lowercaseHex: String
    public enum Algorithm: String, Codable, Sendable { case sha256 }

    public init(algorithm: Algorithm = .sha256, lowercaseHex: String) throws {
        guard lowercaseHex.utf8.count == 64, lowercaseHex.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }) else { throw ParityValidationError.invalidChecksum }
        self.algorithm = algorithm; self.lowercaseHex = lowercaseHex
    }
}

public struct EvidenceArtifact: Hashable, Codable, Sendable {
    public let id: ParityStableID
    public let locator: String
    public let checksum: EvidenceChecksum
    public let capturedAtMilliseconds: UInt64
    public init(id: ParityStableID, locator: String, checksum: EvidenceChecksum, capturedAtMilliseconds: UInt64) throws {
        guard !locator.isEmpty else { throw ParityValidationError.invalidStableID }
        self.id = id; self.locator = locator; self.checksum = checksum; self.capturedAtMilliseconds = capturedAtMilliseconds
    }
}

public struct EvidenceBinding: Hashable, Codable, Sendable {
    public let manifestID: ParityStableID
    public let artifactID: ParityStableID
    public let assertion: String
    public init(manifestID: ParityStableID, artifactID: ParityStableID, assertion: String) throws {
        guard !assertion.isEmpty else { throw ParityValidationError.invalidStableID }
        self.manifestID = manifestID; self.artifactID = artifactID; self.assertion = assertion
    }
}

public struct FreshnessPolicy: Hashable, Codable, Sendable {
    public let maximumAgeMilliseconds: UInt64
    public init(maximumAgeMilliseconds: UInt64) throws {
        guard maximumAgeMilliseconds > 0 else { throw ParityValidationError.staleEvidence("policy") }
        self.maximumAgeMilliseconds = maximumAgeMilliseconds
    }
    public func validate(_ artifact: EvidenceArtifact, nowMilliseconds: UInt64) throws {
        guard artifact.capturedAtMilliseconds <= nowMilliseconds,
              nowMilliseconds - artifact.capturedAtMilliseconds <= maximumAgeMilliseconds
        else { throw ParityValidationError.staleEvidence(artifact.id.rawValue) }
    }
}

public enum CoverageStatus: Hashable, Codable, Sendable {
    case provisional(note: String)
    case verified(evidence: Set<ParityStableID>)
    case excluded(ExclusionReason)
}

public struct CoverageRecord: Hashable, Codable, Sendable {
    public let manifestID: ParityStableID
    public let status: CoverageStatus

    public init(manifestID: ParityStableID, status: CoverageStatus) {
        self.manifestID = manifestID; self.status = status
    }
}

public enum CoverageValidator {
    public static func validateManifest(_ items: [ManifestItem]) throws {
        var ids: Set<ParityStableID> = []
        for item in items where !ids.insert(item.id).inserted { throw ParityValidationError.duplicateID(item.id.rawValue) }
    }

    public static func validateProvisionalCoverage(items: [ManifestItem], coverage: [CoverageRecord]) throws {
        try validateManifest(items)
        let itemsByID = Dictionary(uniqueKeysWithValues: items.map { ($0.id, $0) })
        var covered: Set<ParityStableID> = []
        for record in coverage {
            guard covered.insert(record.manifestID).inserted else { throw ParityValidationError.duplicateID(record.manifestID.rawValue) }
            guard let item = itemsByID[record.manifestID] else { throw ParityValidationError.missingEvidence(record.manifestID.rawValue) }
            switch (item.disposition, record.status) {
            case (.required, .provisional(let note)) where !note.isEmpty: continue
            case (.required, .verified(let evidence)) where !evidence.isEmpty: continue
            case (.excluded(let expected), .excluded(let actual)) where expected == actual: continue
            default: throw ParityValidationError.missingEvidence(record.manifestID.rawValue)
            }
        }
    }

    public static func validateTerminalCoverage(items: [ManifestItem], coverage: [CoverageRecord], artifacts: [EvidenceArtifact], bindings: [EvidenceBinding], freshness: FreshnessPolicy, nowMilliseconds: UInt64) throws {
        try validateManifest(items)
        var artifactByID: [ParityStableID: EvidenceArtifact] = [:]
        for artifact in artifacts {
            guard artifactByID.updateValue(artifact, forKey: artifact.id) == nil else { throw ParityValidationError.duplicateID(artifact.id.rawValue) }
        }
        var coverageByID: [ParityStableID: CoverageStatus] = [:]
        for record in coverage {
            guard coverageByID.updateValue(record.status, forKey: record.manifestID) == nil else { throw ParityValidationError.duplicateID(record.manifestID.rawValue) }
        }
        let bindingsByItem = Dictionary(grouping: bindings, by: \.manifestID)
        for item in items {
            guard let status = coverageByID[item.id] else { throw ParityValidationError.missingEvidence(item.id.rawValue) }
            switch (item.disposition, status) {
            case (.excluded(let expected), .excluded(let actual)) where expected == actual: continue
            case (.required, .verified(let evidenceIDs)):
                guard !evidenceIDs.isEmpty else { throw ParityValidationError.missingEvidence(item.id.rawValue) }
                let boundIDs = Set((bindingsByItem[item.id] ?? []).map(\.artifactID))
                guard evidenceIDs.isSubset(of: boundIDs) else { throw ParityValidationError.missingEvidence(item.id.rawValue) }
                for evidenceID in evidenceIDs {
                    guard let artifact = artifactByID[evidenceID] else { throw ParityValidationError.missingEvidence(evidenceID.rawValue) }
                    try freshness.validate(artifact, nowMilliseconds: nowMilliseconds)
                }
            case (_, .provisional): throw ParityValidationError.provisionalCoverage(item.id.rawValue)
            default: throw ParityValidationError.missingEvidence(item.id.rawValue)
            }
        }
        let itemIDs = Set(items.map(\.id))
        guard Set(coverageByID.keys) == itemIDs else { throw ParityValidationError.missingEvidence("unbound-coverage") }
    }
}
