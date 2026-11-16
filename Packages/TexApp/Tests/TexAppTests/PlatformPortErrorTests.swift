import AppPorts
import Foundation
import XCTest

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

final class PlatformPortErrorTests: XCTestCase {
    private let reopenMessage = "Pitex was updated or changed on disk while it was running. Quit and reopen Pitex, then open the project again."

    func testAccessDeniedBridgesAsErrorCodeOne() {
        let url = URL(fileURLWithPath: "/Users/me/paper")
        let denied = PlatformPortError.accessDenied(url) as NSError
        let unavailable = PlatformPortError.unavailable(feature: "preview", platform: "test") as NSError
        let invalid = PlatformPortError.invalidCapability as NSError
        XCTAssertEqual(denied.code, 1, "domain \(denied.domain) code \(denied.code)")
        XCTAssertTrue(denied.domain.contains("PlatformPortError"), denied.domain)
        XCTAssertEqual(unavailable.code, 0)
        XCTAssertNotEqual(invalid.code, 1)
    }

    func testAccessDeniedNamesTheFolderAndTheNextStep() {
        let url = URL(fileURLWithPath: "/Users/me/paper")
        let message = PlatformPortError.accessDenied(url).localizedDescription
        XCTAssertTrue(message.contains(url.path), message)
        XCTAssertTrue(message.contains("Open"), message)
        XCTAssertFalse(message.contains("error 1"), message)
        XCTAssertFalse(message.contains("PlatformPortError"), message)
    }

    func testStaticCodeChangedMapsFromOSStatus() {
        let url = URL(fileURLWithPath: "/Users/me/paper")
        let error = NSError(domain: NSOSStatusErrorDomain, code: -67_034)
        let mapped = PlatformPortError.bookmarkAccessFailure(error, url: url)
        XCTAssertEqual(mapped, .staticCodeChanged)
        XCTAssertEqual(mapped.localizedDescription, reopenMessage)
        XCTAssertFalse(mapped.localizedDescription.contains(url.path), mapped.localizedDescription)
        XCTAssertFalse(mapped.localizedDescription.contains("permission"), mapped.localizedDescription)
    }

    func testStaticCodeChangedMapsFromTheUnderlyingChain() {
        let url = URL(fileURLWithPath: "/Users/me/paper")
        let status = NSError(domain: NSOSStatusErrorDomain, code: -67_034)
        let wrapped = NSError(
            domain: NSCocoaErrorDomain,
            code: 256,
            userInfo: [NSUnderlyingErrorKey: status]
        )
        XCTAssertEqual(PlatformPortError.bookmarkAccessFailure(wrapped, url: url), .staticCodeChanged)

        let nested = NSError(
            domain: NSCocoaErrorDomain,
            code: 256,
            userInfo: [NSUnderlyingErrorKey: wrapped]
        )
        XCTAssertEqual(PlatformPortError.bookmarkAccessFailure(nested, url: url), .staticCodeChanged)

        let sibling = NSError(
            domain: NSCocoaErrorDomain,
            code: 260,
            userInfo: ["NSMultipleUnderlyingErrorsKey": [status]]
        )
        XCTAssertEqual(PlatformPortError.bookmarkAccessFailure(sibling, url: url), .staticCodeChanged)
    }

    func testOtherBookmarkFailuresStayAccessDenied() {
        let url = URL(fileURLWithPath: "/Users/me/paper")
        let error = NSError(domain: NSCocoaErrorDomain, code: 257)
        let mapped = PlatformPortError.bookmarkAccessFailure(error, url: url)
        XCTAssertEqual(mapped, .accessDenied(url))
        let message = mapped.localizedDescription
        XCTAssertTrue(message.contains(url.path), message)
        XCTAssertTrue(message.contains("Open"), message)
        XCTAssertNotEqual(message, reopenMessage)
    }

    func testBookmarkFailureLogLineIncludesDomainAndCode() {
        let status = NSError(domain: NSOSStatusErrorDomain, code: -67_034)
        let wrapped = NSError(
            domain: NSCocoaErrorDomain,
            code: 256,
            userInfo: [NSUnderlyingErrorKey: status]
        )
        let line = PlatformPortError.bookmarkFailureLogLine(wrapped)
        XCTAssertTrue(line.contains("NSCocoaErrorDomain:256"), line)
        XCTAssertTrue(line.contains("\(NSOSStatusErrorDomain):-67034"), line)
    }

    func testUnreadableProjectDirectoryNamesThePath() throws {
        try skipWhenRoot()
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("pitex-locked-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer {
            try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: root.path)
            try? FileManager.default.removeItem(at: root)
        }
        try "\\documentclass{article}".write(
            to: root.appendingPathComponent("main.tex"),
            atomically: true,
            encoding: .utf8
        )
        try FileManager.default.setAttributes([.posixPermissions: 0], ofItemAtPath: root.path)

        let denied = try XCTUnwrap(PlatformPortError.projectDirectoryAccessError(at: root))
        XCTAssertEqual(denied, .accessDenied(root))
        let message = denied.localizedDescription
        XCTAssertTrue(message.contains(root.path), message)
        XCTAssertFalse(message.contains("no .tex"), message)
    }

    func testReadableDirectoryIsNotAnAccessError() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("pitex-open-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        try "x".write(to: root.appendingPathComponent("notes.txt"), atomically: true, encoding: .utf8)
        XCTAssertNil(PlatformPortError.projectDirectoryAccessError(at: root))
    }

    func testMacBrokerKeepsTheStartedURLAndLogsTheBookmarkError() throws {
        let source = try String(
            contentsOf: repositoryRoot.appendingPathComponent(
                "Packages/TexApp/Sources/MacPlatform/MacPlatform.swift"
            ),
            encoding: .utf8
        )
        XCTAssertTrue(source.contains("url as NSURL"))
        XCTAssertTrue(source.contains("startAccessingSecurityScopedResource()"))
        XCTAssertTrue(source.contains("stopAccessingSecurityScopedResource()"))
        XCTAssertTrue(source.contains("bookmarkAccessFailure"))
        XCTAssertTrue(source.contains("bookmarkFailureLogLine"))
        XCTAssertTrue(source.contains("app.pitex.desktop"))
        XCTAssertFalse(source.contains("SecurityScopeLedger"))
    }

    private func skipWhenRoot() throws {
        if geteuid() == 0 {
            throw XCTSkip("root can read a mode 000 directory")
        }
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
