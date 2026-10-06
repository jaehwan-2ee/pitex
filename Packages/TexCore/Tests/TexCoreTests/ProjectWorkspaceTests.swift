import Foundation
import ProjectCore
import TexDomain
import XCTest

final class ProjectWorkspaceTests: XCTestCase {
    func testMultiFileTabsPreserveOpeningOrderAndDuplicateOpenCoalesces() throws {
        var workspace = ProjectWorkspace(record: try makeRecord(tabs: [mainID], active: mainID))

        try workspace.open(introID)
        try workspace.open(detailsID)
        try workspace.open(introID)

        XCTAssertEqual(workspace.record.tabs, [mainID, introID, detailsID])
        XCTAssertEqual(workspace.record.activeTab, introID)
    }

    func testRecordHasExactDeterministicJSONRoundTrip() throws {
        let record = try makeRecord(
            tabs: [mainID, introID, detailsID],
            active: introID,
            selectedBuildTargetID: "pdf"
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        let first = try encoder.encode(record)
        let restored = try JSONDecoder().decode(ProjectWorkspaceRecord.self, from: first)
        let second = try encoder.encode(restored)

        XCTAssertEqual(restored, record)
        XCTAssertEqual(second, first)
    }

    func testMissingFileOnReopenIsReportedWithoutChangingPersistedRecord() throws {
        let record = try makeRecord(
            tabs: [mainID, introID, detailsID],
            active: detailsID
        )
        let restoration = ProjectWorkspace.restore(
            record,
            availablePaths: [mainPath, introPath]
        )

        XCTAssertEqual(
            restoration.diagnostics,
            [.missingDocument(documentID: detailsID, path: detailsPath)]
        )
        XCTAssertEqual(restoration.persistedRecord, record)
        XCTAssertEqual(restoration.workspace.record.tabs, record.tabs)
        XCTAssertEqual(restoration.workspace.record.activeTab, detailsID)
    }

    func testIncludeCycleAndMissingReferenceRemainInSnapshot() throws {
        let missing = try StableDocumentID(rawValue: "missing")
        let graph = try ProjectGraphSnapshot(
            files: files,
            includeEdges: [
                IncludeEdge(source: mainID, target: introID),
                IncludeEdge(source: introID, target: mainID),
                IncludeEdge(source: mainID, target: missing)
            ]
        )

        XCTAssertEqual(
            try graph.diagnostics(projectID: projectID),
            [
                .missingReference(source: mainID, target: missing, missing: missing),
                .includeCycle(documents: [introID, mainID].sorted())
            ]
        )
        XCTAssertTrue(graph.includeEdges.contains(IncludeEdge(source: mainID, target: missing)))
    }

    func testClosingActiveTabSelectsStableNeighbor() throws {
        var workspace = ProjectWorkspace(
            record: try makeRecord(
                tabs: [mainID, introID, detailsID],
                active: introID
            )
        )

        try workspace.close(introID)
        XCTAssertEqual(workspace.record.tabs, [mainID, detailsID])
        XCTAssertEqual(workspace.record.activeTab, detailsID)

        try workspace.close(detailsID)
        XCTAssertEqual(workspace.record.activeTab, mainID)

        try workspace.close(mainID)
        XCTAssertNil(workspace.record.activeTab)
    }

    func testBuildTargetAndCommandPreferencesPersist() throws {
        var workspace = ProjectWorkspace(
            record: try makeRecord(tabs: [mainID], active: mainID)
        )
        try workspace.selectBuildTarget("pdf")

        let data = try JSONEncoder().encode(workspace.record)
        let restored = try JSONDecoder().decode(ProjectWorkspaceRecord.self, from: data)

        XCTAssertEqual(restored.selectedBuildTargetID, "pdf")
        XCTAssertEqual(restored.buildTargets.map(\.id), ["draft", "pdf"])
        XCTAssertEqual(restored.buildTargets.last?.command.executable, "latexmk")
        XCTAssertEqual(restored.buildTargets.last?.command.arguments, ["-pdf", "main.tex"])
    }

    func testInvalidWorkspaceIdentitiesAndReferencesAreRejected() throws {
        XCTAssertThrowsError(
            try ProjectGraphSnapshot(
                files: files + [ProjectFile(documentID: mainID, path: detailsPath)],
                includeEdges: []
            )
        ) { error in
            XCTAssertEqual(error as? ProjectWorkspaceError, .duplicateDocumentID(self.mainID))
        }
        XCTAssertThrowsError(
            try ProjectGraphSnapshot(
                files: files + [ProjectFile(documentID: try StableDocumentID(rawValue: "other"), path: mainPath)],
                includeEdges: []
            )
        ) { error in
            XCTAssertEqual(error as? ProjectWorkspaceError, .duplicatePath(self.mainPath))
        }
        XCTAssertThrowsError(
            try makeRecord(tabs: [mainID], active: introID)
        ) { error in
            XCTAssertEqual(error as? ProjectWorkspaceError, .invalidActiveTab(self.introID))
        }
        XCTAssertThrowsError(
            try makeRecord(tabs: [mainID], active: mainID, selectedBuildTargetID: "unknown")
        ) { error in
            XCTAssertEqual(error as? ProjectWorkspaceError, .unknownBuildTarget("unknown"))
        }
    }

    func testMalformedAndUnsupportedRestoredStateAreRejected() throws {
        let valid = try JSONEncoder().encode(try makeRecord(tabs: [mainID], active: mainID))
        var object = try XCTUnwrap(JSONSerialization.jsonObject(with: valid) as? [String: Any])
        object["schemaVersion"] = 2
        let unsupported = try JSONSerialization.data(withJSONObject: object)
        XCTAssertThrowsError(try JSONDecoder().decode(ProjectWorkspaceRecord.self, from: unsupported)) {
            XCTAssertEqual(
                $0 as? ProjectWorkspaceError,
                .unsupportedSchemaVersion(found: 2, supported: 1)
            )
        }

        object["schemaVersion"] = 1
        object["tabs"] = ["main", "main"]
        let malformed = try JSONSerialization.data(withJSONObject: object)
        XCTAssertThrowsError(try JSONDecoder().decode(ProjectWorkspaceRecord.self, from: malformed)) {
            XCTAssertEqual($0 as? ProjectWorkspaceError, .duplicateTab(self.mainID))
        }
    }

    private var projectID: StableProjectID { try! StableProjectID(rawValue: "multifile") }
    private var mainID: StableDocumentID { try! StableDocumentID(rawValue: "main") }
    private var introID: StableDocumentID { try! StableDocumentID(rawValue: "intro") }
    private var detailsID: StableDocumentID { try! StableDocumentID(rawValue: "details") }
    private var mainPath: NormalizedRelativePath { try! NormalizedRelativePath(rawValue: "main.tex") }
    private var introPath: NormalizedRelativePath { try! NormalizedRelativePath(rawValue: "sections/intro.tex") }
    private var detailsPath: NormalizedRelativePath { try! NormalizedRelativePath(rawValue: "sections/details.tex") }

    private var files: [ProjectFile] {
        [
            ProjectFile(documentID: mainID, path: mainPath),
            ProjectFile(documentID: introID, path: introPath),
            ProjectFile(documentID: detailsID, path: detailsPath)
        ]
    }

    private func makeRecord(
        tabs: [StableDocumentID],
        active: StableDocumentID?,
        selectedBuildTargetID: String? = nil
    ) throws -> ProjectWorkspaceRecord {
        let targets = [
            try ProjectBuildTarget(
                id: "pdf",
                documentID: mainID,
                command: BuildCommandPreference(
                    executable: "latexmk",
                    arguments: ["-pdf", "main.tex"]
                )
            ),
            try ProjectBuildTarget(
                id: "draft",
                documentID: mainID,
                command: BuildCommandPreference(
                    executable: "pdflatex",
                    arguments: ["-draftmode", "main.tex"]
                )
            )
        ]
        return try ProjectWorkspaceRecord(
            root: ProjectRootIdentity(projectID: projectID),
            graph: ProjectGraphSnapshot(
                files: files,
                includeEdges: [
                    IncludeEdge(source: mainID, target: introID),
                    IncludeEdge(source: mainID, target: detailsID)
                ]
            ),
            tabs: tabs,
            activeTab: active,
            buildTargets: targets,
            selectedBuildTargetID: selectedBuildTargetID,
            externalRevisions: [
                try ExternalRevisionMarker(documentID: introID, revision: "disk-2")
            ]
        )
    }
}
