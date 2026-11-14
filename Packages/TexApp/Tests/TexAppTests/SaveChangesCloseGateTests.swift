import XCTest

#if os(macOS)
import AppKit
import EditorMacAdapter

@MainActor
final class SaveChangesCloseGateTests: XCTestCase {
    private final class State {
        var text = "dirty"
        var response: NSApplication.ModalResponse?
        var windowVisibleDuringAlert = false
        var delivered = false
        var wasPrompting = false
        var confirmed = false
    }

    func testCloseButtonCancelKeepsTheWindow() {
        withModalTimeout {
            let state = State()
            let window = makeWindow()
            let ref = GateRef()
            let gate = SaveChangesCloseGate(
                shouldPrompt: { state.text == "dirty" },
                confirm: {
                    state.wasPrompting = ref.gate?.isPrompting == true
                    state.confirmed = true
                    return false
                }
            )
            ref.gate = gate
            window.orderFront(nil)
            gate.attach(to: window)
            let closeButton = window.standardWindowButton(.closeButton)
            XCTAssertTrue(window.isVisible)
            XCTAssertTrue(closeButton?.target === gate)
            XCTAssertEqual(closeButton?.action, #selector(SaveChangesCloseGate.closeClicked(_:)))
            gate.closeClicked(closeButton)
            let deadline = Date().addingTimeInterval(1)
            while !state.confirmed, Date() < deadline {
                RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.05))
            }
            XCTAssertTrue(state.confirmed)
            XCTAssertTrue(state.wasPrompting)
            XCTAssertTrue(window.isVisible)
            XCTAssertEqual(state.text, "dirty")
        }
    }

    func testQuitClosesWithoutWaitingForThePrompt() {
        withModalTimeout {
            var prompted = false
            let window = makeWindow()
            let gate = SaveChangesCloseGate(
                shouldPrompt: { true },
                allowsImmediateClose: { true },
                confirm: {
                    prompted = true
                    return false
                }
            )
            window.orderFront(nil)
            gate.attach(to: window)
            gate.closeClicked(window.standardWindowButton(.closeButton))
            XCTAssertFalse(prompted)
            XCTAssertFalse(window.isVisible)
            XCTAssertFalse(window.standardWindowButton(.closeButton)?.target === gate)
        }
    }

    func testCancelClickOnWindowCloseKeepsTheWindowAndBuffer() {
        let outcome = runClose(deliver: .click(2))
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.wasPrompting)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertThirdButtonReturn)
        XCTAssertTrue(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "dirty")
    }

    func testEscapeOnWindowCloseKeepsTheWindowAndBuffer() {
        let outcome = runClose(deliver: .escape)
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.wasPrompting)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertThirdButtonReturn)
        XCTAssertTrue(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "dirty")
    }

    func testDontSaveOnWindowCloseDropsTheBufferAndCloses() {
        let outcome = runClose(deliver: .click(1))
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.wasPrompting)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertSecondButtonReturn)
        XCTAssertFalse(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "")
    }

    func testSaveOnWindowCloseWritesAndCloses() {
        let outcome = runClose(deliver: .click(0))
        XCTAssertTrue(outcome.delivered)
        XCTAssertTrue(outcome.windowVisibleDuringAlert)
        XCTAssertEqual(outcome.response, .alertFirstButtonReturn)
        XCTAssertFalse(outcome.window.isVisible)
        XCTAssertEqual(outcome.text, "saved")
    }

    private final class GateRef {
        var gate: SaveChangesCloseGate?
    }

    private enum Delivery {
        case click(Int)
        case escape
    }

    private struct Outcome {
        var window: NSWindow
        var text: String
        var response: NSApplication.ModalResponse?
        var windowVisibleDuringAlert: Bool
        var delivered: Bool
        var wasPrompting: Bool
    }

    private final class Prompt: @unchecked Sendable {
        let window: NSWindow
        let alert: NSAlert
        let gate: SaveChangesCloseGate
        let state: State
        let deliver: Delivery
        init(
            window: NSWindow,
            alert: NSAlert,
            gate: SaveChangesCloseGate,
            state: State,
            deliver: Delivery
        ) {
            self.window = window
            self.alert = alert
            self.gate = gate
            self.state = state
            self.deliver = deliver
        }
    }

    private func runClose(deliver: Delivery) -> Outcome {
        withModalTimeout { runCloseBody(deliver: deliver) }
    }

    /// `runModal` on a stuck main thread never reaches `stopModal`. Exit instead of hanging the suite.
    private func withModalTimeout<T>(_ body: () -> T) -> T {
        let flag = TimeoutFlag()
        DispatchQueue.global(qos: .userInitiated).asyncAfter(deadline: .now() + 4) {
            if flag.isArmed { _exit(1) }
        }
        let value = body()
        flag.disarm()
        return value
    }

    private final class TimeoutFlag: @unchecked Sendable {
        private let lock = NSLock()
        private var armed = true
        func disarm() {
            lock.lock()
            armed = false
            lock.unlock()
        }
        var isArmed: Bool {
            lock.lock()
            defer { lock.unlock() }
            return armed
        }
    }

    private func runCloseBody(deliver: Delivery) -> Outcome {
        NSApp.setActivationPolicy(.accessory)
        let state = State()
        let window = makeWindow()
        let gate = SaveChangesCloseGate(
            shouldPrompt: { state.text == "dirty" },
            confirm: { false }
        )
        window.orderFront(nil)
        gate.attach(to: window)
        let closeButton = window.standardWindowButton(.closeButton)
        XCTAssertTrue(closeButton?.target === gate)
        XCTAssertEqual(closeButton?.action, #selector(SaveChangesCloseGate.closeClicked(_:)))
        let alert = SaveChangesAlert.make(
            message: "\"main.tex\"의 변경 내용을 저장할까요?",
            detail: "detail",
            saveTitle: "저장",
            dontSaveTitle: "저장 안 함",
            cancelTitle: "취소"
        )
        let prompt = Prompt(window: window, alert: alert, gate: gate, state: state, deliver: deliver)
        DispatchQueue.main.async {
            MainActor.assumeIsolated {
                prompt.state.windowVisibleDuringAlert = prompt.window.isVisible
                prompt.state.wasPrompting = prompt.gate.isPrompting
                switch prompt.deliver {
                case .click(let index):
                    prompt.state.delivered = true
                    prompt.alert.buttons[index].performClick(nil)
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
        _ = gate.performGuardedClose(of: window, sender: closeButton) {
            let response = SaveChangesAlert.response(for: alert)
            state.response = response
            switch SaveChangesAlert.choice(for: response) {
            case .cancel:
                return false
            case .discard:
                state.text = ""
                gate.restore()
                return true
            case .save:
                state.text = "saved"
                gate.restore()
                return true
            }
        }
        return Outcome(
            window: window,
            text: state.text,
            response: state.response,
            windowVisibleDuringAlert: state.windowVisibleDuringAlert,
            delivered: state.delivered,
            wasPrompting: state.wasPrompting
        )
    }

    private func makeWindow() -> NSWindow {
        let window = NSWindow(
            contentRect: NSRect(x: 40, y: 40, width: 480, height: 320),
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.isReleasedWhenClosed = false
        window.title = "main.tex"
        window.contentView = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 320))
        return window
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
