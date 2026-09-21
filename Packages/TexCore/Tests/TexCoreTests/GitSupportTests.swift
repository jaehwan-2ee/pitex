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

    func testCommitFilesParseNameStatus() {
        let files = GitSupport.parseCommitFiles("M\tmain.tex\nA\trefs.bib\nR100\told.tex\tnew.tex\n")
        XCTAssertEqual(files, [
            GitCommitFile(path: "main.tex", kind: .modified),
            GitCommitFile(path: "refs.bib", kind: .added),
            // Rename rows carry the new path last — the diff targets it.
            GitCommitFile(path: "new.tex", kind: .renamed),
        ])
        XCTAssertTrue(GitSupport.parseCommitFiles("").isEmpty)
    }

    func testFileDiffAlignsSideBySide() {
        let raw = """
            diff --git a/main.tex b/main.tex
            index 1111111..2222222 100644
            --- a/main.tex
            +++ b/main.tex
            @@ -2,4 +2,5 @@ context
             keep
            -old line
            -another old
            +new line
             tail
            \\ No newline at end of file
            """
        let rows = GitSupport.parseFileDiff(raw)
        // 4 header metas + hunk + context + 2 change pairs + context + \-meta
        XCTAssertEqual(rows.count, 10)
        guard case .hunk = rows[4] else { return XCTFail("row 4 should be the @@ header") }
        guard case let .pair(l0, r0) = rows[5] else { return XCTFail("row 5") }
        XCTAssertEqual(l0, GitDiffLine(number: 2, text: "keep", kind: .context))
        XCTAssertEqual(r0?.number, 2)
        guard case let .pair(l1, r1) = rows[6] else { return XCTFail("row 6") }
        XCTAssertEqual(l1, GitDiffLine(number: 3, text: "old line", kind: .removed))
        XCTAssertEqual(r1, GitDiffLine(number: 3, text: "new line", kind: .added))
        guard case let .pair(l2, r2) = rows[7] else { return XCTFail("row 7") }
        XCTAssertEqual(l2, GitDiffLine(number: 4, text: "another old", kind: .removed))
        XCTAssertNil(r2)
        guard case let .pair(l3, r3) = rows[8] else { return XCTFail("row 8") }
        XCTAssertEqual(l3?.number, 5)
        XCTAssertEqual(r3?.number, 4)
        guard case .meta = rows[9] else { return XCTFail("row 9 should be the \\ meta") }
    }

    func testFileDiffMetaOnlyForBinary() {
        let rows = GitSupport.parseFileDiff("diff --git a/a.pdf b/a.pdf\nBinary files differ\n")
        XCTAssertEqual(rows, [.meta("diff --git a/a.pdf b/a.pdf"), .meta("Binary files differ")])
    }
}
