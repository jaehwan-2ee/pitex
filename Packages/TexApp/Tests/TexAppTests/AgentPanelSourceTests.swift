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

    /// The console is a VSplitView pane. Its hosting view must report fixed sizes,
    /// or streamed text re-dirties its constraints until AppKit gives up (PR 67).
    func testConsolePaneSizeDoesNotFollowItsContent() throws {
        let console = try String(
            contentsOf: repositoryRoot.appendingPathComponent("Mac/Sources/Features/BottomConsoleView.swift"),
            encoding: .utf8
        )
        XCTAssertTrue(console.contains(
            ".frame(minWidth: 0, idealWidth: 0, maxWidth: .infinity,\n               minHeight: 300, idealHeight: 360, maxHeight: .infinity)"
        ))
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
