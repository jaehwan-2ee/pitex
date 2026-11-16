import AppPorts
import Foundation
import XCTest

final class SecurityScopeLedgerTests: XCTestCase {
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

    func testAnUnstoppedScopeBlocksTheNextStart() {
        let budget = ScopeBudget(cap: 1)
        let leaked = budget.make()
        XCTAssertTrue(leaked.startAccessing())
        budget.make().stopAccessing()
        XCTAssertFalse(budget.make().startAccessing())
    }

    func testOpenCloseReopenManyTimesDoesNotExhaustTheScopeBudget() throws {
        let budget = ScopeBudget(cap: 1)
        let ledger = SecurityScopeLedger()
        for _ in 0..<40 {
            let session = SecurityScopeSession(ledger: ledger)
            let id = try XCTUnwrap(ledger.begin(path: "/Users/me/paper", makeResource: { budget.make() }))
            session.track(id)
            session.relinquish(id)
        }
        XCTAssertEqual(budget.held, 0)
    }

    func testSecondBeginForTheSameFolderReusesTheScope() throws {
        let budget = ScopeBudget(cap: 1)
        let ledger = SecurityScopeLedger()
        let first = try XCTUnwrap(ledger.begin(path: "/Users/me/paper", makeResource: { budget.make() }))
        let second = try XCTUnwrap(ledger.begin(path: "/Users/me/paper", makeResource: { budget.make() }))
        XCTAssertEqual(budget.held, 1)
        ledger.end(first)
        XCTAssertEqual(budget.held, 1)
        ledger.end(second)
        XCTAssertEqual(budget.held, 0)
    }

    func testAHeldFolderDoesNotConsumeTheBudgetOfAnotherOpenAfterRelease() throws {
        let budget = ScopeBudget(cap: 1)
        let ledger = SecurityScopeLedger()
        let first = try XCTUnwrap(ledger.begin(path: "/Users/me/paper", makeResource: { budget.make() }))
        XCTAssertNil(ledger.begin(path: "/Users/me/other", makeResource: { budget.make() }))
        ledger.end(first)
        let next = try XCTUnwrap(ledger.begin(path: "/Users/me/other", makeResource: { budget.make() }))
        ledger.end(next)
        XCTAssertEqual(budget.held, 0)
    }

    func testFailedOpenDropsTheBrokerAndTheNextOpenWorks() throws {
        let budget = ScopeBudget(cap: 1)
        let ledger = SecurityScopeLedger()
        try openAndAbandon(ledger: ledger, budget: budget, path: "/Users/me/paper")
        XCTAssertEqual(budget.held, 0)
        let retry = try XCTUnwrap(ledger.begin(path: "/Users/me/paper", makeResource: { budget.make() }))
        ledger.end(retry)
        XCTAssertEqual(budget.held, 0)
    }

    func testMacBrokerStartsAndStopsTheSameNSURL() throws {
        let source = try String(
            contentsOf: repositoryRoot.appendingPathComponent(
                "Packages/TexApp/Sources/MacPlatform/MacPlatform.swift"
            ),
            encoding: .utf8
        )
        XCTAssertTrue(source.contains("final class MacSecurityScopedURL"))
        XCTAssertTrue(source.contains("self.url = url as NSURL"))
        XCTAssertTrue(source.contains("SecurityScopeLedger.shared"))
        XCTAssertTrue(source.contains("SecurityScopeSession"))
    }

    private func openAndAbandon(ledger: SecurityScopeLedger, budget: ScopeBudget, path: String) throws {
        let session = SecurityScopeSession(ledger: ledger)
        let id = try XCTUnwrap(ledger.begin(path: path, makeResource: { budget.make() }))
        session.track(id)
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}

private final class ScopeBudget: @unchecked Sendable {
    let cap: Int
    private let lock = NSLock()
    private var outstanding = 0

    init(cap: Int) { self.cap = cap }

    var held: Int {
        lock.lock()
        defer { lock.unlock() }
        return outstanding
    }

    func make() -> ScopeToken { ScopeToken(budget: self) }

    fileprivate func acquire() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        if outstanding >= cap { return false }
        outstanding += 1
        return true
    }

    fileprivate func release() {
        lock.lock()
        defer { lock.unlock() }
        if outstanding > 0 { outstanding -= 1 }
    }
}

private final class ScopeToken: SecurityScopedResource, @unchecked Sendable {
    private let budget: ScopeBudget
    private var started = false

    init(budget: ScopeBudget) { self.budget = budget }

    func startAccessing() -> Bool {
        guard budget.acquire() else { return false }
        started = true
        return true
    }

    func stopAccessing() {
        guard started else { return }
        started = false
        budget.release()
    }
}
