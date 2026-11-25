@testable import AppPorts
import Foundation
import XCTest

/// The assistant-message block model `AgentPanel` draws. The cases match
/// `chat_markdown.rs` on Linux.
final class AgentMarkdownTests: XCTestCase {
    private typealias M = AgentMarkdown

    private func blocks(_ text: String) -> [M.Block] { M.parse(text, streaming: false).blocks }

    func testHeadingsParagraphsAndRules() {
        XCTAssertEqual(blocks("# Title\n\nSome *em* and **strong**.\nNext line\n\n---\n### Sub ###"), [
            .heading(1, [.text("Title")]),
            .paragraph([.text("Some "), .emphasis([.text("em")]), .text(" and "), .strong([.text("strong")]), .text(".\nNext line")]),
            .rule,
            .heading(3, [.text("Sub")]),
        ])
        XCTAssertEqual(blocks("#hashtag"), [.paragraph([.text("#hashtag")])])
        XCTAssertEqual(blocks("snake_case_name and 2*3"), [.paragraph([.text("snake_case_name and 2*3")])])
    }

    func testListsNestAndNumber() {
        XCTAssertEqual(blocks("- one\n- two\n  - nested\n\n3. three\n4. four\n\n   more four\n"), [
            .list(ordered: false, start: 0, items: [
                [.paragraph([.text("one")])],
                [.paragraph([.text("two")]), .list(ordered: false, start: 0, items: [[.paragraph([.text("nested")])]])],
            ]),
            .list(ordered: true, start: 3, items: [
                [.paragraph([.text("three")])],
                [.paragraph([.text("four")]), .paragraph([.text("more four")])],
            ]),
        ])
        guard case .list(_, _, let items)? = blocks("1. a\n\n2. b\n").first else { return XCTFail("no list") }
        XCTAssertEqual(items.count, 2)
    }

    func testQuotesAndCodeFences() {
        XCTAssertEqual(blocks("> quoted **text**\n> more\n\n```latex\n\\section{A} % *not* emphasis\n  $x$\n```\n~~~\nplain\n~~~"), [
            .quote([.paragraph([.text("quoted "), .strong([.text("text")]), .text("\nmore")])]),
            .code(language: "latex", text: "\\section{A} % *not* emphasis\n  $x$"),
            .code(language: "", text: "plain"),
        ])
        // An unclosed fence runs to the end (CommonMark), streaming or not.
        XCTAssertEqual(blocks("```py\nx = 1"), [.code(language: "py", text: "x = 1")])
        XCTAssertEqual(M.parse("```py\nx = 1", streaming: true).blocks, [.code(language: "py", text: "x = 1")])
        XCTAssertEqual(blocks("```math\nE=mc^2\n```"), [.math("E=mc^2")])
    }

    func testTables() {
        XCTAssertEqual(blocks("| Cmd | Use |\n|---|:-:|\n| `\\cite` | refs \\| more |\n| $x$ | **b** |\nafter"), [
            .table([
                [[.text("Cmd")], [.text("Use")]],
                [[.code("\\cite")], [.text("refs | more")]],
                [[.math("x")], [.strong([.text("b")])]],
            ]),
            .paragraph([.text("after")]),
        ])
        XCTAssertEqual(blocks("a | b\nnot a table"), [.paragraph([.text("a | b\nnot a table")])])
        // The delimiter row must have as many cells as the header (GFM).
        XCTAssertEqual(blocks("a | b\n---"), [.paragraph([.text("a | b")]), .rule])
    }

    func testMathDelimitersEscapesAndCurrency() {
        XCTAssertEqual(M.parseInlines("Inline $E=mc^2$, \\(a+b\\) and $$x^2$$ here"),
                       [.text("Inline "), .math("E=mc^2"), .text(", "), .math("a+b"), .text(" and "), .math("x^2"), .text(" here")])
        XCTAssertEqual(M.parseInlines("costs $5 and $10 today"), [.text("costs $5 and $10 today")])
        XCTAssertEqual(M.parseInlines("pay \\$x\\$ now"), [.text("pay $x$ now")])
        XCTAssertEqual(M.parseInlines("a $ b $ c"), [.text("a $ b $ c")])
        XCTAssertEqual(M.parseInlines("`$x$` is code"), [.code("$x$"), .text(" is code")])
        XCTAssertEqual(M.parseInlines("$a_1 * b_2$"), [.math("a_1 * b_2")])
        XCTAssertEqual(M.parseInlines("echo $FOO$BAR"), [.text("echo $FOO$BAR")])
        XCTAssertEqual(blocks("$$\n\\int_0^1 x\\,dx\n$$\n\\[ a = b \\]\n$$y$$"), [.math("\\int_0^1 x\\,dx"), .math("a = b"), .math("y")])
        XCTAssertEqual(blocks("$$a$$ and $$b$$"), [.paragraph([.math("a"), .text(" and "), .math("b")])])
        XCTAssertEqual(blocks("$$\nx"), [.paragraph([.text("$$\nx")])])
        // Sentence punctuation after the closer still closes.
        XCTAssertEqual(blocks("$$\na=b\n$$."), [.math("a=b")])
        // Display math never spans a blank line: streaming text after one renders.
        XCTAssertEqual(M.parse("$$5 off\n\nnext **bold**", streaming: true).blocks,
                       [.paragraph([.text("$$5 off")]), .paragraph([.text("next "), .strong([.text("bold")])])])
    }

    func testLinksAndAutolinks() {
        XCTAssertEqual(M.parseInlines("See [the *docs*](https://x.org/a_(b)) or <https://y.org> or https://z.org/p."), [
            .text("See "),
            .link(url: "https://x.org/a_(b)", children: [.text("the "), .emphasis([.text("docs")])]),
            .text(" or "),
            .link(url: "https://y.org", children: [.text("https://y.org")]),
            .text(" or "),
            .link(url: "https://z.org/p", children: [.text("https://z.org/p")]),
            .text("."),
        ])
        XCTAssertEqual(M.parseInlines("[main.tex:12](main.tex:12)"), [.link(url: "main.tex:12", children: [.text("main.tex:12")])])
        XCTAssertEqual(M.parseInlines("[x] todo"), [.text("[x] todo")])
        // Images are not drawn: they stay their source.
        XCTAssertEqual(M.parseInlines("see ![img](x.png)"), [.text("see ![img](x.png)")])
    }

    func testLinkClassification() {
        let root = URL(fileURLWithPath: "/proj")
        func file(_ url: String) -> (String, Int?)? {
            if case let .file(target, line) = M.classifyLink(url, root: root) { return (target.path, line) }
            return nil
        }
        XCTAssertEqual(M.classifyLink("https://example.org/a", root: root), .web(URL(string: "https://example.org/a")!))
        XCTAssertEqual(M.classifyLink("HTTP://example.org", root: nil), .web(URL(string: "HTTP://example.org")!))
        XCTAssertEqual(file("main.tex:12")?.0, "/proj/main.tex")
        XCTAssertEqual(file("main.tex:12")?.1, 12)
        XCTAssertEqual(file("sec/intro.tex:4:2")?.1, 4)
        XCTAssertEqual(file("/proj/main.tex#L7")?.1, 7)
        XCTAssertEqual(file("./a/../main.tex")?.0, "/proj/main.tex")
        XCTAssertNil(file("./a/../main.tex")?.1)
        XCTAssertEqual(M.classifyLink("../outside.tex", root: root), .unsupported)
        XCTAssertEqual(M.classifyLink("/etc/passwd", root: root), .unsupported)
        XCTAssertEqual(M.classifyLink("javascript:alert(1)", root: root), .unsupported)
        XCTAssertEqual(M.classifyLink("main.tex", root: nil), .unsupported)
        XCTAssertEqual(file("README.md#usage")?.0, "/proj/README.md")
        XCTAssertNil(file("README.md#usage")?.1)
    }

    func testStreamingRunsAnOpenCodeFenceToTheEndAndSettles() {
        let text = "# Plan\n\nIntro *text*.\n\n```latex\n\\section{A}\n"
        XCTAssertEqual(M.stablePrefixLength(text), "# Plan\n\nIntro *text*.\n\n".utf8.count)
        XCTAssertEqual(M.parse(text, streaming: true).blocks, [
            .heading(1, [.text("Plan")]),
            .paragraph([.text("Intro "), .emphasis([.text("text")]), .text(".")]),
            .code(language: "latex", text: "\\section{A}"),
        ])
        // An open math fence stays literal: no pdflatex on half a formula.
        XCTAssertEqual(M.parse("```math\nE=", streaming: true).blocks, [.paragraph([.text("```math\nE=")])])
        XCTAssertEqual(M.parse(text + "```\n", streaming: true).blocks.last, .code(language: "latex", text: "\\section{A}"))
        XCTAssertEqual(M.stablePrefixLength("```\na\n\nb\n"), 0)
        XCTAssertEqual(M.parse("$$\n\\int x", streaming: true).blocks, [.paragraph([.text("$$\n\\int x")])])
        // Closed input streams into exactly the blocks of the final parse.
        let done = "# Plan\n\n- a\n- b\n\n$$\nx\n$$\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```latex\n\\section{A}\n```\n\nEnd $y$.\n"
        XCTAssertEqual(M.parse(done, streaming: true), M.parse(done, streaming: false))
    }

    func testDeepNestingAndUnclosedOpenersStayBounded() {
        func depth(_ blocks: [M.Block]) -> Int {
            blocks.map { block -> Int in
                switch block {
                case .quote(let inner): return 1 + depth(inner)
                case .list(_, _, let items): return 1 + (items.map(depth).max() ?? 0)
                default: return 0
                }
            }.max() ?? 0
        }
        func inlineDepth(_ inlines: [M.Inline]) -> Int {
            inlines.map { inline -> Int in
                switch inline {
                case .strong(let c), .emphasis(let c), .link(_, let c): return 1 + inlineDepth(c)
                default: return 0
                }
            }.max() ?? 0
        }
        // Each of these overflowed the stack (or the view tree) before the cap.
        XCTAssertEqual(depth(blocks(String(repeating: ">", count: 20_000))), 16)
        XCTAssertEqual(depth(blocks(String(repeating: "+ ", count: 10_000))), 16)
        XCTAssertEqual(depth(blocks(String(repeating: "1. ", count: 10_000))), 16)
        XCTAssertEqual(depth(M.parse(String(repeating: "> - ", count: 5_000), streaming: true).blocks), 16)
        let links = String(repeating: "[", count: 5_000) + "a" + String(repeating: "](b)", count: 5_000)
        guard case .paragraph(let inlines)? = blocks(links).first else { return XCTFail("no paragraph") }
        XCTAssertLessThanOrEqual(inlineDepth(inlines), 17)
        // Unclosed openers rescan the paragraph each. Without the budget these
        // inputs cost hundreds of millions of steps; with it, at most one scan
        // runs past `scanFuel`. Counted, not timed: a busy CI runner cannot flake it.
        for text in [String(repeating: "[", count: 31_000), String(repeating: "*a ", count: 10_000),
                     String(repeating: "\\(", count: 15_000), String(repeating: "[a](", count: 7_000)] {
            let steps = M.inlineScanSteps(text)
            XCTAssertGreaterThan(steps, 0, String(text.prefix(8)))
            XCTAssertLessThanOrEqual(steps, M.scanFuel + text.utf8.count, String(text.prefix(8)))
            // The block parser on top adds only linear work; a generous wall-clock
            // guard still catches an exponential blow-up there.
            let started = Date()
            _ = M.parse(text, streaming: true)
            XCTAssertLessThan(Date().timeIntervalSince(started), 30, String(text.prefix(8)))
        }
    }

    func testRenderLimitCapsAt32kCharacters() {
        let long = String(repeating: "é", count: M.renderLimit + 5)
        let document = M.parse(long, streaming: false)
        XCTAssertTrue(document.truncated)
        XCTAssertEqual(document.blocks, [.paragraph([.text(String(repeating: "é", count: M.renderLimit))])])
        XCTAssertFalse(M.parse(String(repeating: "a", count: M.renderLimit), streaming: false).truncated)
    }
}
