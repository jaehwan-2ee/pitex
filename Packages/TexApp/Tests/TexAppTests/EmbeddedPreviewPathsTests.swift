import BuildFeature
import Foundation
import XCTest

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// Path identity for the helper boundary: a symlinked root and an override
/// path must spell one canonical identity — including override files that
/// are not on disk (new buffers, or deleted while still open). The oracle
/// is POSIX `realpath(3)`, NOT Foundation: Darwin's URL/NSString resolution
/// can strip the /private prefix realpath keeps, so asserting against it
/// would not prove the identity the driver's vfs_key uses.
final class EmbeddedPreviewPathsTests: XCTestCase {
    /// POSIX realpath of an existing path — the same call the driver makes.
    private func posixRealpath(_ path: String) -> String {
        guard let resolved = path.withCString({ realpath($0, nil) }) else {
            XCTFail("realpath failed for existing path \(path)")
            return path
        }
        defer { free(resolved) }
        return String(cString: resolved)
    }

    private func sandbox() throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("pitex-paths-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        return root
    }

    /// Nested alias chain with a file that does not exist: the deepest
    /// existing ancestor must be realpath'd and the tail re-appended.
    func testMissingLeafUnderSymlinkedAncestorResolvesCanonical() throws {
        let sandbox = try sandbox()
        defer { try? FileManager.default.removeItem(at: sandbox) }
        let real = sandbox.appendingPathComponent("real/deep", isDirectory: true)
        try FileManager.default.createDirectory(at: real, withIntermediateDirectories: true)
        try FileManager.default.createSymbolicLink(
            at: sandbox.appendingPathComponent("alias"),
            withDestinationURL: sandbox.appendingPathComponent("real")
        )
        let spelled = sandbox.appendingPathComponent("alias/deep/new.tex").path
        let expected = posixRealpath(real.path) + "/new.tex"
        XCTAssertEqual(EmbeddedPreviewPaths.canonical(spelled), expected)
    }

    func testExistingFileEqualsRealpath() throws {
        let sandbox = try sandbox()
        defer { try? FileManager.default.removeItem(at: sandbox) }
        try FileManager.default.createDirectory(at: sandbox.appendingPathComponent("a"),
                                                withIntermediateDirectories: false)
        let file = sandbox.appendingPathComponent("a/../b.tex").path
        try "x".write(to: URL(fileURLWithPath: sandbox.appendingPathComponent("b.tex").path),
                      atomically: true, encoding: .utf8)
        XCTAssertEqual(EmbeddedPreviewPaths.canonical(file),
                       posixRealpath(sandbox.appendingPathComponent("b.tex").path))
    }

    /// A missing whole subtree keeps every missing component verbatim —
    /// nothing may fabricate a different path for the helper.
    func testWholeMissingTailIsReappendedVerbatim() throws {
        let sandbox = try sandbox()
        defer { try? FileManager.default.removeItem(at: sandbox) }
        let spelled = sandbox.appendingPathComponent("gone/sub/leaf.tex").path
        XCTAssertEqual(EmbeddedPreviewPaths.canonical(spelled),
                       posixRealpath(sandbox.path) + "/gone/sub/leaf.tex")
    }

    /// The shape that broke the personal Mac: on Darwin /tmp realpaths to
    /// /private/tmp, so an override under a /tmp root must come out
    /// prefixed by the realpath root — even though the file does not
    /// exist. Asserted against realpath, so this is vacuous only on a
    /// platform where /tmp is already canonical.
    func testTmpRootAliasSpelling() throws {
        let root = posixRealpath("/tmp")
        let missing = EmbeddedPreviewPaths.canonical("/tmp/pitex-\(UUID().uuidString).tex")
        XCTAssertEqual(missing, root + missing.dropFirst("/tmp".count))
        XCTAssertTrue(missing.hasPrefix(root + "/"))
    }

    /// "/" realpaths to itself; lexical `.`/`..` must normalize before the
    /// walk so a missing intermediate component cannot poison realpath.
    func testRootAndLexicalNormalization() throws {
        XCTAssertEqual(EmbeddedPreviewPaths.canonical("/"), posixRealpath("/"))
        XCTAssertEqual(EmbeddedPreviewPaths.canonical("/nonexistent-x/./b/../c"),
                       "/nonexistent-x/c")
    }

    /// A symlinked project root: the canonical --root and the canonical
    /// override path must share the realpath prefix the driver computes.
    func testOverridePathSharesCanonicalRoot() throws {
        let sandbox = try sandbox()
        defer { try? FileManager.default.removeItem(at: sandbox) }
        let project = sandbox.appendingPathComponent("proj", isDirectory: true)
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: false)
        let aliasRoot = sandbox.appendingPathComponent("proj-alias")
        try FileManager.default.createSymbolicLink(at: aliasRoot, withDestinationURL: project)
        let canonicalRoot = EmbeddedPreviewPaths.canonical(aliasRoot.path)
        XCTAssertEqual(canonicalRoot, posixRealpath(project.path))
        let overridePath = EmbeddedPreviewPaths.canonical(
            aliasRoot.appendingPathComponent("sub/main.tex").path)
        XCTAssertTrue(overridePath.hasPrefix(canonicalRoot + "/"))
    }
}
