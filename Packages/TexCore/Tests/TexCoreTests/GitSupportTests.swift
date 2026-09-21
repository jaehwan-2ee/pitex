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
        // %h %an %ar %D %s %H %B — the subject is repeated inside %B, so the
        // full message starts with it when there is a body.
        let raw = "abc1234\u{1f}Jane\u{1f}2 days ago\u{1f}HEAD -> main, origin/main, tag: v1.2.0\u{1f}Release v1.2.0\u{1f}abc1234fullhash00\u{1f}Release v1.2.0\n\nBody line one.\nBody line two.\n\u{1e}def5678\u{1f}Bob\u{1f}3 days ago\u{1f}\u{1f}Add feature\u{1f}def5678fullhash00\u{1f}Add feature\n\u{1e}"
        let commits = GitSupport.parseLog(raw)
        XCTAssertEqual(commits.count, 2)
        XCTAssertTrue(commits[0].isHead)
        XCTAssertEqual(commits[0].refs, ["main", "origin/main", "tag: v1.2.0"])
        XCTAssertEqual(commits[0].subject, "Release v1.2.0")
        XCTAssertEqual(commits[0].fullHash, "abc1234fullhash00")
        XCTAssertEqual(commits[0].message, "Release v1.2.0\n\nBody line one.\nBody line two.")
        XCTAssertFalse(commits[1].isHead)
        XCTAssertTrue(commits[1].refs.isEmpty)
        XCTAssertEqual(commits[1].message, "Add feature")
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

    func testCommitDiffSplitsIntoFileSections() {
        let raw = """
            diff --git a/a.tex b/a.tex
            index 1111111..2222222 100644
            --- a/a.tex
            +++ b/a.tex
            @@ -1,2 +1,2 @@
            -old a
            +new a
             same
            diff --git a/b.tex b/b.tex
            index 3333333..4444444 100644
            --- a/b.tex
            +++ b/b.tex
            @@ -1,1 +1,2 @@
             keep b
            +added b
            diff --git a/pic.pdf b/pic.pdf
            Binary files differ
            """
        let sections = GitSupport.parseCommitDiff(raw)
        XCTAssertEqual(sections.count, 3)
        XCTAssertEqual(sections[0].path, "a.tex")
        XCTAssertFalse(sections[0].binary)
        // One removal+one addition aligned into a single pair row.
        let section0 = GitDiffFileSection(file: GitCommitFile(path: "a.tex", kind: .modified),
                                          binary: sections[0].binary, rows: sections[0].rows)
        XCTAssertEqual(section0.additions, 1)
        XCTAssertEqual(section0.deletions, 1)
        XCTAssertEqual(sections[1].path, "b.tex")
        XCTAssertTrue(sections[2].binary)
        XCTAssertEqual(sections[2].path, "pic.pdf")
    }

    func testCommitDiffDetectsBinaryByMarkerAlone() {
        let raw = "diff --git a/x.bin b/x.bin\nindex a..b 100644\nBinary files a/x.bin and b/x.bin differ\n"
        let sections = GitSupport.parseCommitDiff(raw)
        XCTAssertEqual(sections.count, 1)
        XCTAssertTrue(sections[0].binary)
        XCTAssertEqual(sections[0].path, "x.bin")
    }

    func testCommitDiffDeletedFileFallsBackToOldPath() {
        // Deleted files have `+++ /dev/null`, so the path comes from the
        // `diff --git` header's `b/` side instead.
        let raw = """
            diff --git a/gone.tex b/gone.tex
            deleted file mode 100644
            index 1111111..0000000
            --- a/gone.tex
            +++ /dev/null
            @@ -1,1 +0,0 @@
            -bye
            """
        let sections = GitSupport.parseCommitDiff(raw)
        XCTAssertEqual(sections.count, 1)
        XCTAssertEqual(sections[0].path, "gone.tex")
    }

    func testCommonAffixesFindsChangeRegion() {
        let (prefix, suffix) = GitSupport.commonAffixes("let x = old", "let x = new")
        XCTAssertEqual(prefix, 8)   // "let x = "
        XCTAssertEqual(suffix, 0)
        let (p2, s2) = GitSupport.commonAffixes("abc123xyz", "abc456xyz")
        XCTAssertEqual(p2, 3)
        XCTAssertEqual(s2, 3)
        // Full overlap must not double-count shared characters.
        let (p3, s3) = GitSupport.commonAffixes("aaa", "aaaa")
        XCTAssertEqual(p3 + s3, 3)
    }

    func testDisplayItemsFoldLongContext() {
        // 20 unchanged context lines: VSCode folds the middle run,
        // keeping edgeContext lines at each end of the fold.
        let ctx = (1...20).map {
            GitDiffLine(number: $0, text: "c\($0)", kind: .context)
        }
        let rows: [GitDiffRow] = ctx.map { .pair(left: $0, right: $0) } + [
            .pair(left: GitDiffLine(number: 21, text: "old", kind: .removed),
                  right: GitDiffLine(number: 21, text: "new", kind: .added)),
        ]
        let items = GitSupport.displayItems(rows, edgeContext: 3)
        // 3 context + fold(14) + 3 context + 1 change pair = 8 items.
        XCTAssertEqual(items.count, 8)
        guard case let .fold(_, pairs) = items[3] else {
            return XCTFail("middle item should be a fold")
        }
        XCTAssertEqual(pairs.count, 14)
        guard case let .pair(l, r) = items[7] else {
            return XCTFail("last item should be the change")
        }
        XCTAssertEqual(l?.kind, .removed)
        XCTAssertEqual(r?.kind, .added)
    }

    func testDisplayItemsShortContextStaysFlat() {
        let ctx = (1...6).map {
            GitDiffLine(number: $0, text: "c\($0)", kind: .context)
        }
        let rows: [GitDiffRow] = ctx.map { .pair(left: $0, right: $0) }
        let items = GitSupport.displayItems(rows, edgeContext: 3)
        XCTAssertEqual(items.count, 6)
        XCTAssertTrue(items.allSatisfy { if case .pair = $0 { true } else { false } })
    }

    func testDisplayItemsHunkBecomesGap() {
        // A `@@` boundary (file longer than diffContextLines) turns into a
        // non-expandable gap sized by the hidden old-side lines.
        let rows: [GitDiffRow] = [
            .pair(left: GitDiffLine(number: 10, text: "c", kind: .context),
                  right: GitDiffLine(number: 10, text: "c", kind: .context)),
            .hunk("@@ -110,3 +110,3 @@"),
            .pair(left: GitDiffLine(number: 110, text: "x", kind: .removed),
                  right: GitDiffLine(number: 110, text: "y", kind: .added)),
        ]
        let items = GitSupport.displayItems(rows)
        XCTAssertEqual(items.count, 3)
        guard case let .gap(hidden) = items[1] else {
            return XCTFail("hunk header should become a gap")
        }
        XCTAssertEqual(hidden, 99) // old lines 11...109 hidden
    }
}
