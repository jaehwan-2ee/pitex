import Foundation
import SyncTeXCore
import XCTest

/// The mapping rules that keep `.pitex-live` outputs from poisoning
/// source lookup: inputs anchor on the local root and the recorded
/// Input 1, never on the hidden PDF directory.
final class SyncTeXSourceMapperTests: XCTestCase {
    private let root = "/Users/ada/Documents/thesis"

    func testRecordedProjectRootStripsKnownMainSuffix() {
        XCTAssertEqual(
            SyncTeXSourceMapper.recordedProjectRoot(
                originalMain: "/device/project/manuscript/main.tex",
                mainRelative: "manuscript/main.tex"
            ),
            "/device/project"
        )
    }

    func testRecordedProjectRootNormalizesBeforeSuffixMatch() {
        XCTAssertEqual(
            SyncTeXSourceMapper.recordedProjectRoot(
                originalMain: "/device/project/manuscript/./main.tex",
                mainRelative: "manuscript/main.tex"
            ),
            "/device/project"
        )
    }

    func testRecordedProjectRootRejectsRelativeAndMismatchedMains() {
        XCTAssertNil(SyncTeXSourceMapper.recordedProjectRoot(
            originalMain: "manuscript/main.tex", mainRelative: "manuscript/main.tex"))
        XCTAssertNil(SyncTeXSourceMapper.recordedProjectRoot(
            originalMain: "/device/other/main.tex", mainRelative: "manuscript/main.tex"))
        // Same basename, different directory — not evidence of the root.
        XCTAssertNil(SyncTeXSourceMapper.recordedProjectRoot(
            originalMain: "/device/project/main.tex", mainRelative: "manuscript/main.tex"))
    }

    func testLocalAbsoluteInputInsideProjectMapsRelative() {
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "/Users/ada/Documents/thesis/chapters/intro.tex",
                projectRoot: root,
                mainRelative: "main.tex",
                recordedRoot: nil
            ),
            "chapters/intro.tex"
        )
    }

    func testLocalAbsoluteInputOutsideProjectIsRejected() {
        XCTAssertNil(SyncTeXSourceMapper.projectRelativePath(
            recordedPath: "/Users/ada/Downloads/other.tex",
            projectRoot: root,
            mainRelative: "main.tex",
            recordedRoot: nil
        ))
    }

    func testRemoteInputsMapThroughRecordedRoot() {
        // Remote run: main recorded on the device, sibling include inside
        // the remote project via a nested main's `../shared` reach.
        let recordedRoot = "/device/project"
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "/device/project/manuscript/main.tex",
                projectRoot: root,
                mainRelative: "manuscript/main.tex",
                recordedRoot: recordedRoot
            ),
            "manuscript/main.tex"
        )
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "/device/project/shared/macros.tex",
                projectRoot: root,
                mainRelative: "manuscript/main.tex",
                recordedRoot: recordedRoot
            ),
            "shared/macros.tex"
        )
        // Outside the remote project entirely — sibling of the root.
        XCTAssertNil(SyncTeXSourceMapper.projectRelativePath(
            recordedPath: "/device/shared/evil.tex",
            projectRoot: root,
            mainRelative: "manuscript/main.tex",
            recordedRoot: recordedRoot
        ))
    }

    func testRelativeInputAnchorsOnKnownMainDirectory() {
        // A `-cd`-style build inside manuscript/ records this exact
        // string for a file that is `manuscript/sections/intro.tex` —
        // a colliding `sections/intro.tex` at the project root must not
        // shadow it.
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "sections/intro.tex",
                projectRoot: root,
                mainRelative: "manuscript/main.tex",
                recordedRoot: nil
            ),
            "manuscript/sections/intro.tex"
        )
        // The main itself records as its own basename under the anchor.
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "main.tex",
                projectRoot: root,
                mainRelative: "manuscript/main.tex",
                recordedRoot: nil
            ),
            "manuscript/main.tex"
        )
    }

    func testRelativeInputWithDotDotSegmentsAnchorsOnMainDirectory() {
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "../shared/macros.tex",
                projectRoot: root,
                mainRelative: "manuscript/main.tex",
                recordedRoot: nil
            ),
            "shared/macros.tex"
        )
        // Dot-dot inside the recorded path still resolves under the
        // anchor — `manuscript/../shared` is `shared`, not the root.
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "manuscript/../shared/macros.tex",
                projectRoot: root,
                mainRelative: "manuscript/main.tex",
                recordedRoot: nil
            ),
            "manuscript/shared/macros.tex"
        )
    }

    func testEscapingRelativeInputIsRejected() {
        XCTAssertNil(SyncTeXSourceMapper.projectRelativePath(
            recordedPath: "../../etc/passwd",
            projectRoot: root,
            mainRelative: "manuscript/main.tex",
            recordedRoot: nil
        ))
    }

    func testIsolatedLivePDFDirectoryIsNeverAnAnchor() {
        // The old mapping reattached inputs beneath the PDF's folder;
        // under .pitex-live that produced .pitex-live/.../chapter.tex.
        // The mapper takes no PDF input at all — its only anchor is the
        // known main's directory, so a recorded relative path resolves
        // inside the source tree, not the hidden output tree.
        let liveMain = "chapters/book/main.tex"
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "intro.tex",
                projectRoot: root,
                mainRelative: liveMain,
                recordedRoot: nil
            ),
            "chapters/book/intro.tex"
        )
    }

    func testRemoteMainEndingAtFilesystemRoot() {
        // Degenerate but consistent: Input 1 directly under `/`.
        XCTAssertEqual(
            SyncTeXSourceMapper.recordedProjectRoot(
                originalMain: "/main.tex", mainRelative: "main.tex"),
            "/"
        )
        XCTAssertEqual(
            SyncTeXSourceMapper.projectRelativePath(
                recordedPath: "/shared/x.tex",
                projectRoot: root,
                mainRelative: "main.tex",
                recordedRoot: "/"
            ),
            "shared/x.tex"
        )
    }
}
