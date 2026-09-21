import BuildFeature
import EditorFeature
import ProjectFeature
import SettingsFeature
import XCTest

final class FeatureStateTests: XCTestCase {
    func testSettingsSafeDefaultsAreConservative() {
        let settings = PersistedSettings.safeDefaults

        XCTAssertEqual(settings.editor, .safeDefaults)
        XCTAssertEqual(settings.build.shellExecution, .disabled)
        XCTAssertFalse(settings.build.customShellAcknowledged)
        XCTAssertEqual(settings.diagnosticsConsent.evidenceLogging, .notGranted)
        XCTAssertEqual(settings.diagnosticsConsent.debugLogging, .notGranted)
        XCTAssertFalse(settings.project.restoresLastProject)
        XCTAssertTrue(settings.project.autosavesDocuments)
    }

    func testInvalidPreferencesAreRejected() {
        XCTAssertThrowsError(try EditorPreferences(fontSize: 7)) { error in
            XCTAssertEqual(error as? SettingsValidationError, .editorFontSizeOutOfRange)
        }
        XCTAssertThrowsError(try BuildPreferences(maximumPasses: 0)) { error in
            XCTAssertEqual(error as? SettingsValidationError, .buildPassLimitOutOfRange)
        }
        XCTAssertThrowsError(
            try BuildPreferences(shellExecution: .custom(command: "bad\0command"))
        ) { error in
            XCTAssertEqual(error as? SettingsValidationError, .invalidCustomShell)
        }
    }

    func testChangingCustomShellRequiresFreshAcknowledgement() throws {
        var build = try BuildPreferences().selectingShellExecution(.custom(command: "latexmk main.tex"))
        XCTAssertFalse(build.customShellAcknowledged)

        build = try build.acknowledgingCustomShell()
        XCTAssertTrue(build.customShellAcknowledged)

        build = try build.selectingShellExecution(.custom(command: "xelatex main.tex"))
        XCTAssertFalse(build.customShellAcknowledged)
    }

    func testProjectPermissionFailureCanRecoverByRequestingAgain() {
        var state = ProjectFeatureState()
        state.beginPermissionRequest()
        state.denyPermission(reason: "User declined")

        guard case let .failed(.permissionDenied(reason)) = state.openState else {
            return XCTFail("Expected a permission failure")
        }
        XCTAssertEqual(reason, "User declined")

        state.beginPermissionRequest()
        guard case .awaitingPermission = state.openState else {
            return XCTFail("Expected permission retry to return to awaiting state")
        }
        guard case .requesting = state.permission else {
            return XCTFail("Expected permission retry to be active")
        }
    }

    func testEditorRejectsInvalidPresentationThenAcceptsCorrectedSelection() throws {
        let outside = try EditorSelection(
            ranges: [try EditorTextRange(location: 5, length: 1)],
            primaryRangeIndex: 0
        )
        XCTAssertThrowsError(
            try EditorPresentationSnapshot(
                documentRevision: 1,
                utf16Length: 5,
                selection: outside,
                markedText: nil
            )
        ) { error in
            XCTAssertEqual(error as? EditorFeatureError, .selectionOutsideDocument)
        }

        let corrected = try EditorSelection(
            ranges: [try EditorTextRange(location: 5, length: 0)],
            primaryRangeIndex: 0
        )
        let presentation = try EditorPresentationSnapshot(
            documentRevision: 1,
            utf16Length: 5,
            selection: corrected,
            markedText: nil
        )
        XCTAssertEqual(presentation.selection, corrected)
    }

    func testBuildCustomShellFailureRecoversOnlyAfterExplicitAcknowledgement() throws {
        var state = BuildFeatureState()
        XCTAssertThrowsError(try state.resolvedCommand(input: "main.tex")) { error in
            XCTAssertEqual(error as? BuildFeatureError, .noCommandSelected)
        }

        state.select(.customShellAwaitingAcknowledgement(PendingCustomShellCommand(
            shellExecutable: "/bin/sh",
            command: "latexmk main.tex",
            source: .userConfiguration,
            disclosure: "Runs this command through the configured login shell."
        )))
        XCTAssertThrowsError(try state.resolvedCommand(input: "main.tex")) { error in
            XCTAssertEqual(error as? BuildFeatureError, .customShellAuthorityNotAcknowledged)
        }

        try state.acknowledgeCustomShellAuthority()
        guard case .loginShell = try state.resolvedCommand(input: "main.tex") else {
            return XCTFail("Expected an authorized login-shell command")
        }

        state.select(.customShellAwaitingAcknowledgement(PendingCustomShellCommand(
            shellExecutable: "/bin/sh",
            command: "xelatex main.tex",
            source: .userConfiguration,
            disclosure: "Runs the changed command through the configured login shell."
        )))
        XCTAssertThrowsError(try state.resolvedCommand(input: "main.tex")) { error in
            XCTAssertEqual(error as? BuildFeatureError, .customShellAuthorityNotAcknowledged)
        }
    }

    /// The main document's same-stem .pdf is a build artifact: the sidebar
    /// pins it below the tree, at any depth, while other PDFs stay put.
    func testExtractOutputPDFsPullsMainStemPDFAtAnyDepth() {
        let tree = buildProjectFileTree(relativePaths: [
            "main.tex",
            "main.pdf",
            "figures/diagram.pdf",
            "build/main.pdf",
            "refs.bib",
        ])
        let (filtered, outputs) = extractOutputPDFs(tree, mainStem: "main")
        XCTAssertEqual(outputs.map(\.path), ["build/main.pdf", "main.pdf"].sorted())
        let remaining = filtered.flatMap { flatten($0) }
        XCTAssertFalse(remaining.contains("main.pdf"))
        XCTAssertFalse(remaining.contains("build/main.pdf"))
        XCTAssertTrue(remaining.contains("main.tex"))
        XCTAssertTrue(remaining.contains("figures/diagram.pdf"))
        // A directory emptied by the extraction is pruned, not left hollow.
        XCTAssertNil(filtered.first { $0.name == "build" })
    }

    func testExtractOutputPDFsWithoutMainIsNoop() {
        let tree = buildProjectFileTree(relativePaths: ["main.tex", "main.pdf"])
        let (filtered, outputs) = extractOutputPDFs(tree, mainStem: "")
        XCTAssertEqual(filtered, tree)
        XCTAssertTrue(outputs.isEmpty)
    }

    private func flatten(_ node: ProjectFileNode) -> [String] {
        [node.path] + (node.children ?? []).flatMap(flatten)
    }
}
