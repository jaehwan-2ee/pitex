import AppPorts
import XCTest

final class AgentProcessEpochTests: XCTestCase {
    func testAnOldAgentExitCannotFailTheReplacement() {
        var epoch = AgentProcessEpoch()
        XCTAssertFalse(epoch.isCurrent(0))
        let codex = epoch.beginLaunch()
        epoch.retire()
        XCTAssertFalse(epoch.isCurrent(codex))

        let claude = epoch.beginLaunch()
        XCTAssertFalse(epoch.isCurrent(codex))
        XCTAssertTrue(epoch.isCurrent(claude))
        XCTAssertFalse(epoch.isCurrent(0))

        let replacement = epoch.beginLaunch()
        XCTAssertFalse(epoch.isCurrent(claude))
        XCTAssertTrue(epoch.isCurrent(replacement))
    }
}
