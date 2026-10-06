import LanguageCore
import XCTest

final class LanguageIndexTests: XCTestCase {
    func testCrossFileDefinitionsUsesAndDeterministicCompletions() throws {
        let main = try LanguageFileSnapshot(
            sourceID: "main.tex",
            revision: 1,
            source: "\\section{소개}\\label{sec:intro}\\input{chapter}"
        )
        let chapter = try LanguageFileSnapshot(
            sourceID: "chapter.tex",
            revision: 1,
            source: "\\subsection{Details}\\ref{sec:intro}\\cite{kim2026}"
        )
        let bib = try LanguageFileSnapshot(
            sourceID: "refs.bib",
            revision: 1,
            source: "@article{kim2026, title={제목}}",
            dialect: .bibtex
        )
        let index = try ProjectLanguageIndex(snapshots: [chapter, bib, main])

        XCTAssertTrue(index.diagnostics.isEmpty)
        XCTAssertEqual(index.references.map { $0.record.key }, ["sec:intro", "kim2026", "sec:intro"])
        XCTAssertEqual(index.bibliography.map { $0.entry.key }, ["kim2026"])
        XCTAssertEqual(
            index.completions(prefix: "sec").map { $0.text },
            ["sec:intro"]
        )
        XCTAssertEqual(
            LanguageIndex.commandCompletions(prefix: "\\sub").map { $0.text },
            ["\\subsection", "\\subsubsection"]
        )
        XCTAssertEqual(index.completions(), index.completions())
    }

    func testRecursiveOutlineFollowsIncludesAtTheirSourcePosition() throws {
        let main = try LanguageFileSnapshot(
            sourceID: "main.tex",
            revision: 1,
            source: "\\section{Before}\\input{chapters/a}\\section{After}"
        )
        let a = try LanguageFileSnapshot(
            sourceID: "chapters/a.tex",
            revision: 1,
            source: "\\subsection{A}\\include{b}"
        )
        let b = try LanguageFileSnapshot(
            sourceID: "chapters/b.tex",
            revision: 1,
            source: "\\subsubsection{B}"
        )
        let index = try ProjectLanguageIndex(snapshots: [b, main, a])

        let outline = index.recursiveOutline(from: "main.tex")
        XCTAssertEqual(outline.map { $0.sourceID }, ["main.tex", "chapters/a.tex", "chapters/b.tex", "main.tex"])
        XCTAssertEqual(outline.map { $0.record.title }, ["Before", "A", "B", "After"])
        XCTAssertEqual(outline.map { $0.record.depth }, [0, 1, 2, 0])
    }

    func testBibTeXIndexHandlesNestedGroupsAndIgnoresSpecialEntries() throws {
        let snapshot = try LanguageFileSnapshot(
            sourceID: "library.bib",
            revision: 4,
            source: "% @article{ignored,}\n@string{name = \"Journal\"}\n@article{alpha, title={A {Nested} Title}}\n@book(beta, title=\"B\")",
            dialect: .bibtex
        )

        XCTAssertEqual(snapshot.bibliography.map { $0.type }, ["article", "book"])
        XCTAssertEqual(snapshot.bibliography.map { $0.key }, ["alpha", "beta"])
        XCTAssertEqual(snapshot.minimapRanges.map { $0.kind }, [.bibliography, .bibliography])
        XCTAssertTrue(snapshot.diagnostics.isEmpty)
    }

    func testCommentsVerbatimAndEscapedCommandsAreConservative() throws {
        let source = """
        % \\section{commented}
        \\begin{verbatim}
        \\section{verbatim}
        \\label{fake}
        \\end{verbatim}
        \\verb|\\ref{also-fake}|
        \\% \\section{real}
        """
        let snapshot = try LanguageFileSnapshot(sourceID: "main.tex", revision: 1, source: source)

        XCTAssertEqual(snapshot.outline.map { $0.title }, ["real"])
        XCTAssertTrue(snapshot.references.isEmpty)
    }

    func testMissingDuplicatesCyclesAndMalformedGroupsAreExplicitAndStable() throws {
        let a = try LanguageFileSnapshot(
            sourceID: "a.tex",
            revision: 1,
            source: "\\label{same}\\ref{missing}\\cite{absent}\\input{b}\\section{oops"
        )
        let b = try LanguageFileSnapshot(
            sourceID: "b.tex",
            revision: 1,
            source: "\\label{same}\\input{a}"
        )
        let index = try ProjectLanguageIndex(snapshots: [b, a])
        let diagnostics = index.diagnostics

        XCTAssertEqual(diagnostics, index.diagnostics)
        XCTAssertTrue(diagnostics.contains { if case .malformedGroup(sourceID: "a.tex", command: "section", range: _) = $0 { true } else { false } })
        XCTAssertTrue(diagnostics.contains { if case .duplicateLabel(key: "same", definitions: _) = $0 { true } else { false } })
        XCTAssertTrue(diagnostics.contains { if case .missingLabel(key: "missing", use: _) = $0 { true } else { false } })
        XCTAssertTrue(diagnostics.contains { if case .missingCitation(key: "absent", use: _) = $0 { true } else { false } })
        XCTAssertTrue(diagnostics.contains { if case .includeCycle(["a.tex", "b.tex", "a.tex"]) = $0 { true } else { false } })
    }

    func testStaleSuppressionAndLargeEditRevisionReplacement() throws {
        let old = try LanguageFileSnapshot(sourceID: "main.tex", revision: 8, source: "\\section{Old}")
        let largeSource = String(repeating: "가나다라마바사아자차카타파하\n", count: 10_000) + "\\section{New}"
        let replacement = try LanguageFileSnapshot(sourceID: "main.tex", revision: 9, source: largeSource)
        let stale = try LanguageFileSnapshot(sourceID: "main.tex", revision: 7, source: "stale")

        let replaced = try ProjectLanguageIndex(snapshots: [old]).replacing(replacement)
        XCTAssertEqual(replaced.snapshots["main.tex"]?.outline.map { $0.title }, ["New"])
        XCTAssertEqual(replaced.snapshots["main.tex"]?.revision, replacement.revision)
        XCTAssertThrowsError(try replaced.replacing(stale)) { error in
            XCTAssertEqual(error as? LanguageCoreError, .staleResult(expected: replacement.revision, actual: stale.revision))
        }
        XCTAssertThrowsError(try replaced.snapshot(sourceID: "main.tex", revision: old.revision))
    }

    func testMinimapOrderingIsBySourceRange() throws {
        let snapshot = try LanguageFileSnapshot(
            sourceID: "main.tex",
            revision: 1,
            source: "\\section{A}\n\\label{x}\n\\ref{x}\n\\input{child}"
        )
        XCTAssertEqual(snapshot.minimapRanges.map { $0.kind }, [.outline, .definition, .use, .include])
        XCTAssertEqual(snapshot.minimapRanges.map { $0.range.utf8Offset }, snapshot.minimapRanges.map { $0.range.utf8Offset }.sorted())
    }
}
