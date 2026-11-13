import XCTest

#if os(macOS)
import AppKit
import EditorMacAdapter

@MainActor
final class SaveChangesCloseGateTests: XCTestCase {
    private final class State: @unchecked Sendable {
        var text = "dirty"
        var response: NSApplication.ModalResponse?
        var alertWindow: NSWindow?
        var windowVisibleDuringAlert = false
        var delivered = false
        var attempts = 0
        var armed = true
    }

    func testCancelClickOnWindowCloseKeepsTheWindowAndBuffer() {
        let outcome = runClose(deliver: .click("취소"))
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertThirdButtonReturn)
        XCTAssertTrue(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "dirty")
    }

    func testEscapeOnWindowCloseKeepsTheWindowAndBuffer() {
        let outcome = runClose(deliver: .escape)
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertThirdButtonReturn)
        XCTAssertTrue(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "dirty")
    }

    func testDontSaveOnWindowCloseDropsTheBufferAndCloses() {
        let outcome = runClose(deliver: .click("저장 안 함"))
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

    private func runClose(deliver: Delivery) -> Outcome {
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
                state.alertWindow = alert.window
                let response = SaveChangesAlert.response(for: alert)
                state.response = response
                state.armed = false
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
        let holder = Holder(delivery: deliver, window: window, state: state)
        Self.schedule(holder)
        Self.watch(state)
        closeButton?.performClick(nil)
        let start = Date()
        while state.armed && Date().timeIntervalSince(start) < 3 {
            RunLoop.main.run(until: Date().addingTimeInterval(0.02))
        }
        state.armed = false
        return Outcome(
            window: window,
            text: state.text,
            response: state.response,
            windowVisibleDuringAlert: state.windowVisibleDuringAlert,
            delivered: state.delivered
        )
    }

    private final class Holder: @unchecked Sendable {
        let delivery: Delivery
        let window: NSWindow
        let state: State
        init(delivery: Delivery, window: NSWindow, state: State) {
            self.delivery = delivery
            self.window = window
            self.state = state
        }
    }

    private static func watch(_ state: State) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            MainActor.assumeIsolated {
                guard state.armed else { return }
                NSApp.stopModal(withCode: .abort)
                if state.armed { watch(state) }
            }
        }
    }

    private static func schedule(_ holder: Holder) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
            MainActor.assumeIsolated {
                let state = holder.state
                guard state.armed else { return }
                guard let modal = state.alertWindow, modal.isVisible, modal !== holder.window,
                      let content = modal.contentView else {
                    if state.armed { schedule(holder) }
                    return
                }
                state.windowVisibleDuringAlert = holder.window.isVisible
                switch holder.delivery {
                case .click(let title):
                    guard let button = button(titled: title, in: content) else {
                        state.attempts += 1
                        if state.attempts < 40 { schedule(holder) }
                        else { NSApp.stopModal(withCode: .abort) }
                        return
                    }
                    state.delivered = true
                    sendClick(to: button)
                case .escape:
                    state.delivered = true
                    sendEscape(to: modal)
                }
            }
        }
    }

    private static func button(titled title: String, in view: NSView) -> NSButton? {
        if let button = view as? NSButton, button.title == title { return button }
        for subview in view.subviews {
            if let button = button(titled: title, in: subview) { return button }
        }
        return nil
    }

    private static func sendClick(to button: NSButton) {
        guard let window = button.window else { return }
        let point = button.convert(NSPoint(x: button.bounds.midX, y: button.bounds.midY), to: nil)
        func event(_ type: NSEvent.EventType) -> NSEvent? {
            NSEvent.mouseEvent(
                with: type,
                location: point,
                modifierFlags: [],
                timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber,
                context: nil,
                eventNumber: 1,
                clickCount: 1,
                pressure: 1
            )
        }
        guard let down = event(.leftMouseDown), let up = event(.leftMouseUp) else { return }
        NSApp.postEvent(down, atStart: false)
        NSApp.postEvent(up, atStart: false)
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
