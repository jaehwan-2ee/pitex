import XCTest

#if os(macOS)
import AppKit
import EditorMacAdapter

@MainActor
final class SaveChangesCloseGateTests: XCTestCase {
    private final class State: @unchecked Sendable {
        var text = "dirty"
        var response: NSApplication.ModalResponse?
        var windowVisibleDuringAlert = false
        var delivered = false
    }

    func testCancelClickOnWindowCloseKeepsTheWindowAndBuffer() async {
        let outcome = await runClose(deliver: .click("취소"))
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertThirdButtonReturn)
        XCTAssertTrue(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "dirty")
    }

    func testEscapeOnWindowCloseKeepsTheWindowAndBuffer() async {
        let outcome = await runClose(deliver: .escape)
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertThirdButtonReturn)
        XCTAssertTrue(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "dirty")
    }

    func testDontSaveOnWindowCloseDropsTheBufferAndCloses() async {
        let outcome = await runClose(deliver: .click("저장 안 함"))
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertSecondButtonReturn)
        XCTAssertFalse(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "")
    }

    private enum Delivery: Sendable {
        case click(String)
        case escape
    }

    private struct Outcome {
        var window: NSWindow
        var text: String
        var response: NSApplication.ModalResponse?
        var windowVisibleDuringAlert: Bool
        var delivered: Bool
    }

    private final class Prompt: @unchecked Sendable {
        let window: NSWindow
        let alert: NSAlert
        let state: State
        let deliver: Delivery
        init(window: NSWindow, alert: NSAlert, state: State, deliver: Delivery) {
            self.window = window
            self.alert = alert
            self.state = state
            self.deliver = deliver
        }
    }

    private func runClose(deliver: Delivery) async -> Outcome {
        NSApp.setActivationPolicy(.accessory)
        let state = State()
        let window = NSWindow(
            contentRect: NSRect(x: 40, y: 40, width: 480, height: 320),
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.isReleasedWhenClosed = false
        window.title = "main.tex"
        window.contentView = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 320))
        let gate = SaveChangesCloseGate(
            shouldPrompt: { state.text == "dirty" },
            confirm: {
                let alert = SaveChangesAlert.make(
                    message: "\"main.tex\"의 변경 내용을 저장할까요?",
                    detail: "detail",
                    saveTitle: "저장",
                    dontSaveTitle: "저장 안 함",
                    cancelTitle: "취소"
                )
                let prompt = Prompt(window: window, alert: alert, state: state, deliver: deliver)
                DispatchQueue.main.async {
                    MainActor.assumeIsolated {
                        prompt.state.windowVisibleDuringAlert = prompt.window.isVisible
                        switch prompt.deliver {
                        case .click(let title):
                            guard let content = prompt.alert.window.contentView,
                                  let button = Self.button(titled: title, in: content) else { return }
                            prompt.state.delivered = true
                            button.performClick(nil)
                        case .escape:
                            prompt.state.delivered = true
                            Self.sendEscape(to: prompt.alert.window)
                        }
                    }
                }
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                    MainActor.assumeIsolated {
                        guard prompt.state.response == nil else { return }
                        NSApp.stopModal(withCode: .abort)
                    }
                }
                let response = SaveChangesAlert.response(for: alert)
                state.response = response
                switch SaveChangesAlert.choice(for: response) {
                case .cancel:
                    return false
                case .discard:
                    state.text = ""
                    return true
                case .save:
                    state.text = "saved"
                    return true
                }
            }
        )
        window.makeKeyAndOrderFront(nil)
        gate.attach(to: window)
        let closeButton = window.standardWindowButton(.closeButton)
        XCTAssertNotNil(closeButton)
        XCTAssertTrue(closeButton?.target === gate)
        XCTAssertEqual(closeButton?.action, #selector(SaveChangesCloseGate.closeClicked(_:)))
        await gate.beginClose(of: window, sender: closeButton)
        return Outcome(
            window: window,
            text: state.text,
            response: state.response,
            windowVisibleDuringAlert: state.windowVisibleDuringAlert,
            delivered: state.delivered
        )
    }

    private static func button(titled title: String, in view: NSView) -> NSButton? {
        if let button = view as? NSButton, button.title == title { return button }
        for subview in view.subviews {
            if let button = button(titled: title, in: subview) { return button }
        }
        return nil
    }

    private static func sendEscape(to window: NSWindow) {
        guard let event = NSEvent.keyEvent(
            with: .keyDown,
            location: .zero,
            modifierFlags: [],
            timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: window.windowNumber,
            context: nil,
            characters: "\u{1b}",
            charactersIgnoringModifiers: "\u{1b}",
            isARepeat: false,
            keyCode: 53
        ) else { return }
        NSApp.postEvent(event, atStart: true)
    }
}
#endif
