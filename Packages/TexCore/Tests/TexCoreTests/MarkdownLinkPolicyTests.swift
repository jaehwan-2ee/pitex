import Foundation
import TexDomain
import XCTest

final class MarkdownLinkPolicyTests: XCTestCase {
    private var directory: URL!

    override func setUpWithError() throws {
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("pitex-link-policy-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: directory)
    }

    private func make(_ name: String, executable: Bool = false) throws -> URL {
        let url = directory.appendingPathComponent(name)
        try "x".write(to: url, atomically: true, encoding: .utf8)
        if executable {
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
        }
        return url
    }

    func testExternalSchemesOpenExternally() throws {
        for raw in ["https://example.com/x", "http://example.com", "mailto:a@b.c"] {
            XCTAssertEqual(
                MarkdownLinkPolicy.action(for: URL(string: raw)!),
                .openExternal(URL(string: raw)!)
            )
        }
    }

    func testUnsupportedSchemesAreIgnored() {
        for raw in ["javascript:alert(1)", "data:text/html,x", "ftp://h/f"] {
            XCTAssertEqual(MarkdownLinkPolicy.action(for: URL(string: raw)!), .ignore)
        }
    }

    func testPlainLocalFileOpens() throws {
        let pdf = try make("paper.pdf")
        XCTAssertEqual(
            MarkdownLinkPolicy.action(for: pdf),
            .openFile(pdf.resolvingSymlinksInPath())
        )
    }

    func testExecutableBitIsRefused() throws {
        let script = try make("run", executable: true)
        XCTAssertEqual(MarkdownLinkPolicy.action(for: script), .refuse)
    }

    func testRefusedExtensionIsRefusedWithoutExecBit() throws {
        // `.app` bundles are directories — the extension check runs first.
        let bundle = directory.appendingPathComponent("Tool.app")
        try FileManager.default.createDirectory(at: bundle, withIntermediateDirectories: true)
        XCTAssertEqual(MarkdownLinkPolicy.action(for: bundle), .refuse)
        let shell = try make("build.sh")
        XCTAssertEqual(MarkdownLinkPolicy.action(for: shell), .refuse)
    }

    /// A link's own extension means nothing once symlinks resolve —
    /// `paper.pdf -> run.sh` is judged as the script it points at.
    func testSymlinkToExecutableIsRefused() throws {
        let script = try make("run.sh", executable: true)
        let link = directory.appendingPathComponent("paper.pdf")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: script)
        XCTAssertEqual(MarkdownLinkPolicy.action(for: link), .refuse)
    }

    func testSymlinkToAppBundleIsRefused() throws {
        let bundle = directory.appendingPathComponent("Evil.app")
        try FileManager.default.createDirectory(at: bundle, withIntermediateDirectories: true)
        let link = directory.appendingPathComponent("doc.pdf")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: bundle)
        XCTAssertEqual(MarkdownLinkPolicy.action(for: link), .refuse)
    }

    func testSymlinkToPlainFileOpensTheResolvedTarget() throws {
        let real = try make("real.pdf")
        let link = directory.appendingPathComponent("notes.pdf")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: real)
        XCTAssertEqual(
            MarkdownLinkPolicy.action(for: link),
            .openFile(real.resolvingSymlinksInPath())
        )
    }

    func testMissingFileIsIgnored() {
        let missing = directory.appendingPathComponent("gone.pdf")
        XCTAssertEqual(MarkdownLinkPolicy.action(for: missing), .ignore)
    }

    func testPercentEncodedPathWithSpacesResolves() throws {
        let file = try make("my paper.pdf")
        let encoded = URL(string: "file://\(file.path(percentEncoded: true))")!
        XCTAssertEqual(
            MarkdownLinkPolicy.action(for: encoded),
            .openFile(file.resolvingSymlinksInPath())
        )
    }

    func testQueryAndFragmentDropAway() throws {
        let pdf = try make("paper2.pdf")
        let url = URL(string: "\(pdf.absoluteString)?v=2#frag")!
        XCTAssertEqual(
            MarkdownLinkPolicy.action(for: url),
            .openFile(pdf.resolvingSymlinksInPath())
        )
    }

    func testThemeResolver() {
        XCTAssertFalse(MarkdownLinkPolicy.previewIsDark(theme: "light", appIsDark: true))
        XCTAssertTrue(MarkdownLinkPolicy.previewIsDark(theme: "dark", appIsDark: false))
        XCTAssertTrue(MarkdownLinkPolicy.previewIsDark(theme: "system", appIsDark: true))
        XCTAssertFalse(MarkdownLinkPolicy.previewIsDark(theme: "system", appIsDark: false))
        XCTAssertTrue(MarkdownLinkPolicy.previewIsDark(theme: "bogus", appIsDark: true))
    }
}
