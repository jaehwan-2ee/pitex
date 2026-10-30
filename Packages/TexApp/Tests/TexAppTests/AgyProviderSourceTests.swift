import Foundation
import XCTest

/// The Antigravity provider is one more row on the existing backend picker.
/// These checks read the Mac sources the Linux shell copies its strings from.
final class AgyProviderSourceTests: XCTestCase {
    func testSettingsPickerAddsAntigravityWithoutANewPanel() throws {
        let settings = try source(at: "Mac/Sources/Features/SettingsView.swift")
        XCTAssertTrue(settings.contains("case \"agy\":"))
        XCTAssertTrue(settings.contains("return \"agy\""))
        XCTAssertTrue(settings.contains("Text(\"settings.ai.backend_claude\").tag(\"claude\")"))
        XCTAssertTrue(settings.contains("Text(\"settings.ai.backend_grok\").tag(\"grok\")"))
        XCTAssertTrue(settings.contains("Text(\"settings.ai.backend_agy\").tag(\"agy\")"))
        XCTAssertTrue(settings.contains("Text(\"settings.ai.backend_note\")"))
    }

    func testBridgeLaunchUsesTheSelectedProvider() throws {
        let coordinator = try source(at: "Mac/Sources/Features/AgentCoordinator.swift")
        XCTAssertTrue(coordinator.contains("case \"codex\", \"claude\", \"grok\", \"agy\":"))
        XCTAssertTrue(coordinator.contains("environment[\"PITEX_CLI_ADAPTER\"] = adapter"))
        XCTAssertTrue(coordinator.contains("The Antigravity CLI bridge is not installed with this Pitex build."))
        XCTAssertTrue(coordinator.contains("The Claude Code bridge is not installed with this Pitex build."))
        XCTAssertTrue(coordinator.contains("The Grok CLI bridge is not installed with this Pitex build."))
    }

    func testEveryLocaleNamesTheAntigravityBackend() throws {
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
