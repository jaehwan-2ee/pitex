import XCTest
@testable import GitCore

final class GitSupportTests: XCTestCase {
    func testStatusParsesStagedUnstagedUntracked() {
        let raw = "## main...origin/main [ahead 1, behind 2]\0 M modified.tex\0M  staged.tex\0?? new.tex\0"
        let status = GitSupport.parseStatus(raw, root: "/tmp/paper")
        XCTAssertEqual(status.branch, "main")
        XCTAssertEqual(status.upstream, "origin/main")
        XCTAssertEqual(status.ahead, 1)
        XCTAssertEqual(status.behind, 2)
        XCTAssertEqual(status.repoName, "paper")
        XCTAssertEqual(status.staged, [
            GitChange(path: "staged.tex", originalPath: nil, kind: .modified, staged: true),
        ])
        XCTAssertEqual(status.unstaged.map(\.kind), [.modified, .untracked])
    }

    func testStatusParsesRenamePairAndConflicts() {
        let status = GitSupport.parseStatus("## feature\0R  new.tex\0old.tex\0UU clash.tex\0", root: "/r")
        XCTAssertEqual(status.branch, "feature")
        XCTAssertEqual(status.staged.first?.kind, .renamed)
        XCTAssertEqual(status.staged.first?.originalPath, "old.tex")
        XCTAssertEqual(status.unstaged.first?.kind, .conflicted)
    }

    func testStatusHandlesNoCommitsAndDetached() {
        XCTAssertEqual(GitSupport.parseStatus("## No commits yet on main\0?? a.tex\0", root: "/r").branch, "main")
        XCTAssertEqual(GitSupport.parseStatus("## HEAD (no branch)\0", root: "/r").branch, "HEAD")
    }

    func testLogParsesRefsAndHead() {
        let raw = "abc1234\u{1f}Jane\u{1f}2 days ago\u{1f}HEAD -> main, origin/main, tag: v1.2.0\u{1f}Release v1.2.0\u{1e}def5678\u{1f}Bob\u{1f}3 days ago\u{1f}\u{1f}Add feature\u{1e}"
        let commits = GitSupport.parseLog(raw)
        XCTAssertEqual(commits.count, 2)
        XCTAssertTrue(commits[0].isHead)
        XCTAssertEqual(commits[0].refs, ["main", "origin/main", "tag: v1.2.0"])
        XCTAssertEqual(commits[0].subject, "Release v1.2.0")
        XCTAssertFalse(commits[1].isHead)
        XCTAssertTrue(commits[1].refs.isEmpty)
    }

    func testBranchesParseLines() {
        XCTAssertEqual(GitSupport.parseBranches("main\nfeature\n"), ["main", "feature"])
    }
}
