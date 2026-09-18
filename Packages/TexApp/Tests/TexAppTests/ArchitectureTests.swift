import AppPorts
import AppShell
import BuildFeature
import EditorFeature
import PDFFeature
import ProjectFeature
import SettingsFeature
import XCTest

final class ArchitectureTests: XCTestCase {
    private func assertPortable<T: Sendable>(_: T.Type) {}

    func testFeatureStateSurfacesAreSendableAndPortable() {
        assertPortable(ProjectFeatureState.self)
        assertPortable(EditorPresentationSnapshot.self)
        assertPortable(BuildFeatureState.self)
        assertPortable(PDFFeatureState.self)
        assertPortable(SettingsState.self)
    }

    func testPersistedSettingsKeysExcludeSecretUpdaterAndDistributionSemantics() {
        let prohibitedFragments = [
            "apikey", "api_key", "secret", "updater", "updatechannel",
            "distribution", "releasechannel",
        ]

        for key in SettingsKey.allCases {
            let normalized = key.rawValue.lowercased()
            XCTAssertFalse(
                prohibitedFragments.contains(where: normalized.contains),
                "Persisted settings key must remain a non-secret, portable preference: \(key.rawValue)"
            )
        }
    }

    #if os(Linux)
    @MainActor
    func testAppShellConstructionFailsWhenItsPlatformIsUnavailable() async {
        do {
            _ = try await AppShell.make(documentSession: ArchitectureDocumentSession())
            XCTFail("AppShell must not manufacture a fake platform environment")
        } catch let error as AppShellError {
            XCTAssertEqual(
                error,
                .platformUnavailable(required: "macOS 15 or later", detected: "non-macOS")
            )
        } catch {
            XCTFail("Unexpected construction error: \(error)")
        }
    }
    #endif
}

private actor ArchitectureDocumentSession: DocumentSessionPort {
    func snapshot() async -> AppPorts.DocumentSnapshot {
        AppPorts.DocumentSnapshot(revision: 0, text: "")
    }

    func submit(_ mutation: AppPorts.DocumentMutation) async throws -> AppPorts.DocumentMutationResult {
        .rejected(current: await snapshot())
    }
}
