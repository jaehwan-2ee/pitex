import Foundation
import XCTest

final class AgyProviderSourceTests: XCTestCase {
    func testSettingsPickerOmitsTheRetiredBackend() throws {
        let settings = try source(at: "Mac/Sources/Features/SettingsView.swift")
        XCTAssertFalse(settings.contains(".tag(\"agy\")"))
        XCTAssertFalse(settings.contains("case \"agy\""))
        XCTAssertTrue(settings.contains("settings.ai.backend_note"))
        XCTAssertTrue(settings.contains("settings.ai.backend_claude"))
        XCTAssertTrue(settings.contains("settings.ai.backend_grok"))
    }

    func testBridgeLaunchOmitsTheRetiredBackend() throws {
        let coordinator = try source(at: "Mac/Sources/Features/AgentCoordinator.swift")
        XCTAssertFalse(coordinator.contains("case \"codex\", \"claude\", \"grok\", \"agy\""))
        XCTAssertFalse(coordinator.contains("The Antigravity CLI bridge is not installed with this Pitex build."))
        XCTAssertTrue(coordinator.contains("environment[\"PITEX_CLI_ADAPTER\"]"))
        XCTAssertTrue(coordinator.contains("The Claude Code bridge is not installed with this Pitex build."))
        XCTAssertTrue(coordinator.contains("The Grok CLI bridge is not installed with this Pitex build."))
    }

    func testEveryLocaleStillHasTheAntigravityBackendKey() throws {
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
