import LanguageCore
import XCTest

/// Ported to `Linux/crates/language-core/tests/completion.rs` — keep both
/// suites aligned for parity.
final class CompletionContextTests: XCTestCase {
    private func detect(_ source: String, _ caret: Int? = nil) -> CompletionContext? {
        CompletionContextDetector.context(
            in: source,
            caretUTF16Offset: caret ?? source.utf16.count
        )
    }

    func testCommandPrefixAtCaretIncludesBackslash() {
        let context = detect("\\sub")
        XCTAssertEqual(context?.kind, .command)
        XCTAssertEqual(context?.prefix, "\\sub")
        XCTAssertEqual(context?.prefixUTF16Offset, 0)
    }

    func testBareBackslashAndPlainText() {
        XCTAssertEqual(detect("\\")?.kind, .command)
        XCTAssertEqual(detect("\\")?.prefix, "\\")
        XCTAssertNil(detect("hello "))
        XCTAssertNil(detect(""))
        XCTAssertNil(detect("3 + 4"))
    }

    func testEscapedBackslashIsNotACommand() {
        XCTAssertNil(detect("\\\\ab"))
        XCTAssertNil(detect("x \\\\"))
    }

    func testCitationContexts() {
        let context = detect("\\cite{kim")
        XCTAssertEqual(context?.kind, .citation)
        XCTAssertEqual(context?.prefix, "kim")
        XCTAssertEqual(context?.prefixUTF16Offset, 6)

        let empty = detect("\\cite{")
        XCTAssertEqual(empty?.kind, .citation)
        XCTAssertEqual(empty?.prefix, "")
        XCTAssertEqual(empty?.prefixUTF16Offset, 6)
    }

    func testCommaSeparatedAndWhitespaceCitationKeys() {
        let second = detect("\\cite{foo,ba")
        XCTAssertEqual(second?.kind, .citation)
        XCTAssertEqual(second?.prefix, "ba")
        XCTAssertEqual(second?.prefixUTF16Offset, 10)

        let spaced = detect("\\cite{foo, ba")
        XCTAssertEqual(spaced?.kind, .citation)
        XCTAssertEqual(spaced?.prefix, "ba")

        let first = detect("\\cite{foo,ba", 8)
        XCTAssertEqual(first?.kind, .citation)
        XCTAssertEqual(first?.prefix, "fo")
    }

    func testOptionalArgumentAndStarredCitationCommands() {
        let optional = detect("\\cite[see]{ki")
        XCTAssertEqual(optional?.kind, .citation)
        XCTAssertEqual(optional?.prefix, "ki")

        let starred = detect("\\citep*{x")
        XCTAssertEqual(starred?.kind, .citation)
        XCTAssertEqual(starred?.prefix, "x")

        let natbib = detect("\\parencite{al")
        XCTAssertEqual(natbib?.kind, .citation)
        XCTAssertEqual(natbib?.prefix, "al")
    }

    func testReferenceContexts() {
        let reference = detect("\\ref{sec:in")
        XCTAssertEqual(reference?.kind, .reference)
        XCTAssertEqual(reference?.prefix, "sec:in")

        XCTAssertEqual(detect("\\pageref{a")?.kind, .reference)
        XCTAssertEqual(detect("\\eqref{eq:")?.kind, .reference)
        XCTAssertEqual(detect("\\cref{x")?.kind, .reference)
        XCTAssertEqual(detect("\\autoref{x")?.kind, .reference)
    }

    func testUnrecognisedGroupsAndEscapedGroupsDoNotComplete() {
        XCTAssertNil(detect("\\section{ti"))
        XCTAssertNil(detect("\\label{x"))
        XCTAssertNil(detect("\\foo{bar"))
        XCTAssertNil(detect("\\cite\\{k"))
        XCTAssertNil(detect("\\\\cite{k"))
    }

    func testWhitespaceBetweenCommandAndBrace() {
        let context = detect("\\cite {ki")
        XCTAssertEqual(context?.kind, .citation)
        XCTAssertEqual(context?.prefix, "ki")
    }

    func testNonBMPTextBeforeCaretKeepsUTF16Offsets() {
        // "한😀" is 3 UTF-16 units; `\cite{k` starts its prefix at 3 + 6.
        let context = detect("한😀\\cite{k")
        XCTAssertEqual(context?.kind, .citation)
        XCTAssertEqual(context?.prefix, "k")
        XCTAssertEqual(context?.prefixUTF16Offset, 9)
    }

    func testContextualCompletionsFilterProjectKeySets() {
        let citations = LanguageIndex.completions(
            for: CompletionContext(kind: .citation, prefix: "k", prefixUTF16Offset: 0),
            labels: ["sec:intro"],
            citationKeys: ["kim2026", "knuth1984", "other"]
        )
        XCTAssertEqual(citations.map { $0.text }, ["kim2026", "knuth1984"])
        XCTAssertEqual(citations.map { $0.kind }, [.citation, .citation])

        let references = LanguageIndex.completions(
            for: CompletionContext(kind: .reference, prefix: "sec", prefixUTF16Offset: 0),
            labels: ["sec:intro", "sec:methods", "fig:x"],
            citationKeys: ["sec:cite"]
        )
        XCTAssertEqual(references.map { $0.text }, ["sec:intro", "sec:methods"])
        XCTAssertTrue(references.allSatisfy { $0.kind == .label })

        let commands = LanguageIndex.completions(
            for: CompletionContext(kind: .command, prefix: "\\sub", prefixUTF16Offset: 0),
            labels: [],
            citationKeys: []
        )
        XCTAssertEqual(commands.map { $0.text }, ["\\subsection", "\\subsubsection"])
        XCTAssertTrue(commands.allSatisfy { $0.kind == .command })
    }
}
