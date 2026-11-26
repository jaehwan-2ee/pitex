import Foundation
import RemoteCore
import XCTest

final class RemoteSavedSourcesTests: XCTestCase {
    /// R06: a pull deleted a clean open chapter. The next save verifies the
    /// other sources and leaves the deleted chapter out instead of failing.
    func testACleanSourceDeletedByPullDoesNotBlockTheSave() throws {
        let root = try makeRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        try Data("edited".utf8).write(to: root.appendingPathComponent("main.tex"))

        let hashes = try RemoteSyncRules.savedSourceHashes(
            root: root,
            openSources: [("main.tex", true), ("chapter.tex", true)]
        )

        XCTAssertEqual(hashes, ["main.tex": "1fb9f4097256db2d7b1e13aff79cee44339891a31c556b9cf6093885773b3618"])
    }

    /// Unsaved text whose file is gone, and a source that is not a regular
    /// file, still stop the save.
    func testMissingUnsavedSourceAndNonRegularSourceStillFail() throws {
        let root = try makeRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        try FileManager.default.createDirectory(at: root.appendingPathComponent("dir.tex"), withIntermediateDirectories: true)

        XCTAssertThrowsError(try RemoteSyncRules.savedSourceHashes(root: root, openSources: [("chapter.tex", false)]))
        XCTAssertThrowsError(try RemoteSyncRules.savedSourceHashes(root: root, openSources: [("dir.tex", true)]))
    }

    private func makeRoot() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("saved-sources-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }
}
