import XCTest

#if os(macOS)
import AppKit
import DocumentSessionCore
import EditorMacAdapter

@MainActor
final class SaveChangesAlertTests: XCTestCase {
    private final class Box: @unchecked Sendable {
        let alert: NSAlert
        var armed = true
        init(_ alert: NSAlert) { self.alert = alert }
    }

    private enum Action: Sendable {
        case click(Int)
        case key(UInt16, String)
    }

    func testCancelClickDismissesWithNoDocumentWindow() {
        let alert = sample(cancelTitle: "Cancel")
        XCTAssertEqual(alert.buttons[2].keyEquivalent, "")
        XCTAssertEqual(finish(alert, .click(2)), .alertThirdButtonReturn)
    }

    func testEscapeDismissesWithNoDocumentWindow() {
        let alert = sample(cancelTitle: "취소")
        XCTAssertEqual(finish(alert, .key(53, "\u{1b}")), .alertThirdButtonReturn)
    }

    func testDontSaveClickDismisses() {
        let alert = sample(cancelTitle: "취소")
        XCTAssertEqual(finish(alert, .click(1)), .alertSecondButtonReturn)
    }

    func testSaveClickDismisses() {
        let alert = sample(cancelTitle: "취소")
        XCTAssertEqual(finish(alert, .click(0)), .alertFirstButtonReturn)
    }

    func testChoicesMatchTheButtons() {
        XCTAssertEqual(SaveChangesAlert.choice(for: .alertFirstButtonReturn), .save)
        XCTAssertEqual(SaveChangesAlert.choice(for: .alertSecondButtonReturn), .discard)
        XCTAssertEqual(SaveChangesAlert.choice(for: .alertThirdButtonReturn), .cancel)
    }

    private func sample(cancelTitle: String) -> NSAlert {
        NSApp.setActivationPolicy(.accessory)
        return SaveChangesAlert.make(
            message: "Save changes?",
            detail: "detail",
            saveTitle: "저장",
            dontSaveTitle: "저장 안 함",
            cancelTitle: cancelTitle
        )
    }

    private func finish(_ alert: NSAlert, _ action: Action) -> NSApplication.ModalResponse {
        let box = Box(alert)
        DispatchQueue.main.async {
            MainActor.assumeIsolated {
                switch action {
                case .click(let index):
                    box.alert.buttons[index].performClick(nil)
                case .key(let code, let characters):
                    guard let event = NSEvent.keyEvent(
                        with: .keyDown,
                        location: .zero,
                        modifierFlags: [],
                        timestamp: ProcessInfo.processInfo.systemUptime,
                        windowNumber: box.alert.window.windowNumber,
                        context: nil,
                        characters: characters,
                        charactersIgnoringModifiers: characters,
                        isARepeat: false,
                        keyCode: code
                    ) else { return }
                    NSApp.postEvent(event, atStart: true)
                }
            }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            MainActor.assumeIsolated {
                guard box.armed, NSApp.modalWindow != nil else { return }
                NSApp.stopModal(withCode: .abort)
            }
        }
        let response = SaveChangesAlert.response(for: alert)
        box.armed = false
        return response
    }
}
#endif
