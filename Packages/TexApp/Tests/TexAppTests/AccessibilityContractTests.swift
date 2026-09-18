import Foundation
import XCTest

final class AccessibilityContractTests: XCTestCase {
    private struct Contract: Decodable {
        let identifiers: [String]
        let locales: [String]
        let localizationKeys: [String]
        let forbiddenUIStrings: [String]
    }

    func testNativeSourcesContainEveryStableIdentifierExactlyOnce() throws {
        let contract = try loadContract()
        XCTAssertEqual(Set(contract.identifiers).count, contract.identifiers.count, "Fixture contains duplicate identifiers")
        XCTAssertTrue(contract.identifiers.allSatisfy { $0.hasPrefix("pitex.") })

        let source = try appSwiftSource()
        for identifier in contract.identifiers {
            XCTAssertEqual(
                source.components(separatedBy: "\"\(identifier)\"").count - 1,
                1,
                "Expected one source declaration of \(identifier)"
            )
        }

        let declared = matches(
            in: source,
            pattern: #"(?:accessibilityIdentifier|setAccessibilityIdentifier)\s*\(\s*"(pitex\.[^"\\]+)"\s*\)"#,
            capture: 1
        )
        XCTAssertEqual(declared.count, Set(declared).count, "Native source declares a duplicate accessibility identifier")
        XCTAssertTrue(Set(contract.identifiers).isSubset(of: Set(declared)))
    }

    func testEveryRequiredLocalizationKeyExistsInEveryLocale() throws {
        let contract = try loadContract()
        XCTAssertEqual(Set(contract.locales).count, contract.locales.count)
        XCTAssertEqual(Set(contract.localizationKeys).count, contract.localizationKeys.count)

        var baseline: Set<String>?
        for locale in contract.locales {
            let url = repositoryRoot
                .appendingPathComponent("Mac/Resources/\(locale).lproj/Localizable.strings")
            let text = try String(contentsOf: url, encoding: .utf8)
            let keys = matches(in: text, pattern: #"(?m)^\s*"([^"\\]+)"\s*="#, capture: 1)
            XCTAssertEqual(keys.count, Set(keys).count, "Duplicate localization key in \(locale)")
            XCTAssertTrue(Set(contract.localizationKeys).isSubset(of: Set(keys)), "Missing required keys in \(locale)")
            if let baseline {
                XCTAssertEqual(Set(keys), baseline, "Locale \(locale) does not have identical key coverage")
            } else {
                baseline = Set(keys)
            }
        }

        let sourceKeys = Set(matches(
            in: try appSwiftSource(),
            pattern: #""((?:app|workspace|editor|build|preview|assistant|command|settings|state|error|conflict|recovery|warning|accessibility)\.[A-Za-z0-9_.]+)""#,
            capture: 1
        ))
        XCTAssertTrue(sourceKeys.isSubset(of: baseline ?? []), "Swift source uses an unlocalized key: \(sourceKeys.subtracting(baseline ?? []))")
    }

    func testNativeUIContainsNoAPIKeyOrUpdaterSurface() throws {
        let contract = try loadContract()
        var visibleText = try appSwiftSource()
        for locale in contract.locales {
            let url = repositoryRoot
                .appendingPathComponent("Mac/Resources/\(locale).lproj/Localizable.strings")
            visibleText += try String(contentsOf: url, encoding: .utf8)
        }
        let folded = visibleText.lowercased()
        for forbidden in contract.forbiddenUIStrings {
            XCTAssertFalse(folded.contains(forbidden.lowercased()), "Forbidden UI string: \(forbidden)")
        }
    }

    private func loadContract() throws -> Contract {
        let url = repositoryRoot.appendingPathComponent("Fixtures/expected/accessibility-identifiers.json")
        return try JSONDecoder().decode(Contract.self, from: Data(contentsOf: url))
    }

    private func appSwiftSource() throws -> String {
        let root = repositoryRoot.appendingPathComponent("Mac/Sources")
        let files = try FileManager.default.subpathsOfDirectory(atPath: root.path)
            .filter { $0.hasSuffix(".swift") }
            .sorted()
        XCTAssertFalse(files.isEmpty)
        return try files.map { path in
            try String(contentsOf: root.appendingPathComponent(path), encoding: .utf8)
        }.joined(separator: "\n")
    }

    private func matches(in text: String, pattern: String, capture: Int) -> [String] {
        guard let expression = try? NSRegularExpression(pattern: pattern) else {
            XCTFail("Invalid test regular expression")
            return []
        }
        let range = NSRange(text.startIndex..., in: text)
        return expression.matches(in: text, range: range).compactMap { match in
            guard let swiftRange = Range(match.range(at: capture), in: text) else { return nil }
            return String(text[swiftRange])
        }
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
