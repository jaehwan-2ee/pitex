import XCTest

#if os(macOS)
import AppKit
import EditorMacAdapter

@MainActor
final class SaveFailureAlertTests: XCTestCase {
    func testReadOnlyDocumentOffersSaveAsFirst() {
        let alert = SaveFailureAlert.make(title: "Not saved", detail: "main.tex is read-only.", saveAsTitle: "Save As…", dismissTitle: "OK")
        XCTAssertEqual(alert.buttons.map(\.title), ["Save As…", "OK"])
        XCTAssertEqual(alert.informativeText, "main.tex is read-only.")
        XCTAssertTrue(SaveFailureAlert.wantsSaveAs(alert, response: .alertFirstButtonReturn))
        XCTAssertFalse(SaveFailureAlert.wantsSaveAs(alert, response: .alertSecondButtonReturn))
    }

    func testOtherFailuresOnlyDismiss() {
        let alert = SaveFailureAlert.make(title: "Not saved", detail: "Disk full.", saveAsTitle: nil, dismissTitle: "OK")
        XCTAssertEqual(alert.buttons.map(\.title), ["OK"])
        XCTAssertFalse(SaveFailureAlert.wantsSaveAs(alert, response: .alertFirstButtonReturn))
    }
}
#endif
