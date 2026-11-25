import Foundation
import XCTest

final class WorkspaceSplitSourceTests: XCTestCase {
    /// SwiftUI's VSplitView hosts its panes in views that feed their content's
    /// sizes into Auto Layout. Dragging the console divider while the assistant
    /// streamed made AppKit throw from _postWindowNeedsUpdateConstraints.
    /// Both splits must be frame-based with hosts that install no size constraints.
    func testWorkspaceSplitsAreFrameBasedWithoutHostSizing() throws {
        let view = try String(
            contentsOf: repositoryRoot.appendingPathComponent("Mac/Sources/Features/WorkspaceView.swift"),
            encoding: .utf8
        )
        XCTAssertFalse(view.contains("VSplitView {"))
        XCTAssertFalse(view.contains("HSplitView {"))
        XCTAssertTrue(view.contains("EditorConsoleSplitView("))
        let hosts = view.components(separatedBy: "NSHostingView(rootView:").count - 1
        let unsized = view.components(separatedBy: "host.sizingOptions = []").count - 1
        XCTAssertEqual(hosts, 2)
        XCTAssertEqual(unsized, hosts, "every pane host must set sizingOptions = []")
    }

    private var repositoryRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
}
