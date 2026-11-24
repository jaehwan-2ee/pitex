import Foundation
import XCTest

final class AgyProviderSourceTests: XCTestCase {
    func testSettingsOffersAntigravityThroughTheBridge() throws {
        let settings = try source(at: "Mac/Sources/Features/SettingsView.swift")
        XCTAssertTrue(settings.contains("case pi, codex, claude, agy, grok"))
        XCTAssertTrue(settings.contains("case .pi, .claude, .agy, .appleFM: true"))
        XCTAssertTrue(settings.contains("case .agy: \"settings.ai.backend_agy\""))
        XCTAssertTrue(settings.contains("case .codex, .claude, .agy, .grok: rawValue"))
        XCTAssertTrue(settings.contains("settings.ai.backend_note"))
    }

    func testBridgeLaunchNamesAntigravity() throws {
        let coordinator = try source(at: "Mac/Sources/Features/AgentCoordinator.swift")
        XCTAssertTrue(coordinator.contains("environment[\"PITEX_CLI_ADAPTER\"]"))
        XCTAssertTrue(coordinator.contains("The Antigravity CLI bridge is not installed with this Pitex build."))
        XCTAssertTrue(coordinator.contains("String(localized: \"settings.ai.backend_agy\")"))
        let bridge = try source(at: "Mac/Resources/PitexAgent/bridge/adapter.ts")
        XCTAssertTrue(bridge.contains("case \"agy\""))
        XCTAssertFalse(bridge.contains("id === \"agy\" ? \"codex\""))
    }

    func testEveryLocaleHasTheAntigravityBackendKey() throws {
        for locale in ["en", "es", "ja", "ko", "ru", "vi", "zh-Hans"] {
            let table = try source(at: "Mac/Resources/\(locale).lproj/Localizable.strings")
            XCTAssertTrue(
                table.contains("\"settings.ai.backend_agy\""),
                "\(locale) is missing settings.ai.backend_agy"
            )
        }
    }

    private func source(at path: String) throws -> String {
        try String(contentsOf: repositoryRoot.appendingPathComponent(path), encoding: .utf8)
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
