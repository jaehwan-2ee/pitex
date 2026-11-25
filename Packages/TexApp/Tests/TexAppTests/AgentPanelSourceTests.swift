import Foundation
import XCTest

final class AgentPanelSourceTests: XCTestCase {
    /// A popover left open while a turn streams made AppKit throw from
    /// _postWindowNeedsUpdateConstraints (the context meter, PR 67). The chat
    /// panel keeps one popover, history, and its button is off while a turn runs.
    func testOnlyTheHistoryPopoverAndNotWhileATurnRuns() throws {
        let panel = try String(
            contentsOf: repositoryRoot.appendingPathComponent("Mac/Sources/Features/AgentPanel.swift"),
            encoding: .utf8
        )
        XCTAssertEqual(panel.components(separatedBy: ".popover(").count - 1, 1)
        let history = try XCTUnwrap(panel.range(of: ".popover(isPresented: $coordinator.showingSessionHistory"))
        let before = panel[panel.index(history.lowerBound, offsetBy: -400)..<history.lowerBound]
        XCTAssertTrue(before.contains(".disabled(coordinator.isRunning)"))
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
