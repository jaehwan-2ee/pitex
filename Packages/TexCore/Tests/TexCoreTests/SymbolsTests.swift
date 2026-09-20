import LanguageCore
import XCTest

/// Ported to `Linux/crates/language-core/tests/completion.rs` — keep both
/// suites aligned.
final class SymbolsTests: XCTestCase {
    func testEveryCategoryHasSymbolsInCatalogueOrder() {
        for category in SymbolCategory.allCases {
            let rows = TexSymbolCatalogue.symbols(in: category)
            XCTAssertFalse(rows.isEmpty, "\(category) must not be empty")
            XCTAssertTrue(rows.allSatisfy { $0.category == category })
            XCTAssertTrue(rows.allSatisfy { !$0.glyph.isEmpty })
            XCTAssertTrue(rows.allSatisfy { !$0.command.isEmpty })
        }
    }

    func testCatalogueContainsRequiredGroupsAndCommands() {
        let commands = Set(TexSymbolCatalogue.symbols.map { $0.command })
        for required in [
            "\\alpha", "\\Omega", "\\leq", "\\in", "\\times", "\\sum", "\\int",
            "\\rightarrow", "\\Rightarrow", "\\langle", "\\infty", "\\partial",
            "\\S", "\\ulcorner", "\\digamma", "\\boxplus", "\\leqslant",
            "\\dashrightarrow",
        ] {
            XCTAssertTrue(commands.contains(required), "missing \(required)")
        }
    }

    func testSymbolCategoryTitleKeysAreStable() {
        XCTAssertEqual(SymbolCategory.greekLetters.titleKey, "editor.symbol_cat_greek")
        XCTAssertEqual(SymbolCategory.amsArrows.titleKey, "editor.symbol_cat_ams_arrows")
        XCTAssertEqual(SymbolCategory.allCases.count, 14)
    }
}
