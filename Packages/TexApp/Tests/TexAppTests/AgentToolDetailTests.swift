import AppPorts
import Foundation
import XCTest

/// The typed tool detail `AgentCoordinator` decodes from bridge events and
/// `AgentPanel` turns into card titles and badges.
final class AgentToolDetailTests: XCTestCase {
    private let root = URL(fileURLWithPath: "/p")

    private func phrase(_ phrase: AgentToolDetail.Phrase) -> String {
        switch phrase {
        case .ran: return "Ran %@"
        case .edited: return "Edited %@"
        case .wrote: return "Wrote %@"
        case .read: return "Read %@"
        case .lines: return "lines %@"
        case .searched: return "Searched %@"
        case .matches: return "%@ matches"
        case .fetched: return "Fetched %@"
        case .subagent: return "Subagent: %@"
        case .thought: return "Thinking"
        case .exit: return "exit %@"
        }
    }

    func testDecodesEveryKindAndBuildsItsTitle() throws {
        let shell = try XCTUnwrap(AgentToolDetail(["kind": "shell", "command": "latexmk -pdf main.tex", "exitCode": 1]))
        XCTAssertEqual(shell.title(fallback: "bash", root: root, phrase: phrase), "Ran latexmk -pdf main.tex")
        XCTAssertEqual(shell.badge(phrase: phrase), "exit 1")

        let edit = try XCTUnwrap(AgentToolDetail([
            "kind": "edit", "path": "/p/chapters/intro.tex", "oldText": "a", "newText": "b",
            "diff": "@@ -1,2 +1,3 @@\n a\n-b\n+B\n+C",
        ]))
        XCTAssertEqual(edit.title(fallback: "edit", root: root, phrase: phrase), "Edited chapters/intro.tex")
        XCTAssertEqual(edit.badge(phrase: phrase), "+2 -1")
        XCTAssertEqual(edit.openableURL(root: root)?.path, "/p/chapters/intro.tex")

        let read = try XCTUnwrap(AgentToolDetail(["kind": "read", "path": "/p/main.tex", "startLine": 10, "endLine": 40]))
        XCTAssertEqual(read.title(fallback: "read", root: root, phrase: phrase), "Read main.tex (lines 10-40)")

        let search = try XCTUnwrap(AgentToolDetail(["kind": "search", "query": "\\cite", "numMatches": 12]))
        XCTAssertEqual(search.title(fallback: "Grep", root: root, phrase: phrase), "Searched \"\\cite\" · 12 matches")

        let fetch = try XCTUnwrap(AgentToolDetail(["kind": "fetch", "url": "https://ctan.org/pkg/biblatex"]))
        XCTAssertEqual(fetch.title(fallback: "WebFetch", root: root, phrase: phrase), "Fetched ctan.org")

        let write = try XCTUnwrap(AgentToolDetail(["kind": "write", "path": "/p/refs.bib", "diff": "@@ -0,0 +1,1 @@\n+@a{b}"]))
        XCTAssertEqual(write.title(fallback: "write", root: root, phrase: phrase), "Wrote refs.bib")

        let other = try XCTUnwrap(AgentToolDetail(["kind": "other", "label": "TodoWrite"]))
        XCTAssertEqual(other.title(fallback: "TodoWrite", root: root, phrase: phrase), "TodoWrite")
        XCTAssertNil(AgentToolDetail(nil))
        XCTAssertNil(AgentToolDetail(["path": "/p/x"]))
    }

    func testOutsideTheProjectShowsTheFullPathAndCannotOpen() throws {
        let edit = try XCTUnwrap(AgentToolDetail(["kind": "edit", "path": "/etc/hosts"]))
        XCTAssertEqual(edit.title(fallback: "edit", root: root, phrase: phrase), "Edited /etc/hosts")
        XCTAssertNil(edit.openableURL(root: root))
        let shell = try XCTUnwrap(AgentToolDetail(["kind": "shell", "command": "ls", "exitCode": 0]))
        XCTAssertNil(shell.openableURL(root: root))
        XCTAssertNil(shell.badge(phrase: phrase))
    }

    func testAnUntypedUpdateNeverReplacesATypedKind() throws {
        var detail = try XCTUnwrap(AgentToolDetail(["kind": "edit", "path": "/p/a.tex"]))
        detail.merge(try XCTUnwrap(AgentToolDetail(["kind": "other", "label": "edit"])))
        XCTAssertEqual(detail.kind, .edit)
        XCTAssertNil(detail.label)
        detail.merge(try XCTUnwrap(AgentToolDetail(["kind": "edit", "diff": "@@ -1,1 +1,1 @@\n-a\n+b"])))
        XCTAssertEqual(detail.path, "/p/a.tex")
        XCTAssertEqual(detail.diff, "@@ -1,1 +1,1 @@\n-a\n+b")
    }

    func testStatusAndDiffCap() throws {
        XCTAssertEqual(AgentToolDetail.status("canceled"), .canceled)
        XCTAssertEqual(AgentToolDetail.status("completed"), .completed)
        XCTAssertNil(AgentToolDetail.status(nil))
        let huge = "@@ -1,1 +1,9000 @@\n" + String(repeating: "+line\n", count: 9_000)
        let capped = try XCTUnwrap(AgentToolDetail(["kind": "write", "diff": huge])?.diff)
        XCTAssertLessThanOrEqual(capped.utf8.count, AgentToolDetail.diffCap + 20)
        XCTAssertTrue(capped.hasSuffix("\\ diff truncated"))
    }
}
