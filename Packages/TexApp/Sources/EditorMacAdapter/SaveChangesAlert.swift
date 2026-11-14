#if os(macOS)
import AppKit
import DocumentSessionCore

@MainActor
public enum SaveChangesAlert {
    public static func choose(
        message: String,
        detail: String,
        saveTitle: String,
        dontSaveTitle: String,
        cancelTitle: String
    ) -> LocalCloseChoice {
        choice(for: response(for: make(
            message: message,
            detail: detail,
            saveTitle: saveTitle,
            dontSaveTitle: dontSaveTitle,
            cancelTitle: cancelTitle
        )))
    }

    public static func make(
        message: String,
        detail: String,
        saveTitle: String,
        dontSaveTitle: String,
        cancelTitle: String
    ) -> NSAlert {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = message
        alert.informativeText = detail
        alert.addButton(withTitle: saveTitle)
        alert.addButton(withTitle: dontSaveTitle)
        let cancel = alert.addButton(withTitle: cancelTitle)
        cancel.keyEquivalent = ""
        return alert
    }

    public static func response(for alert: NSAlert) -> NSApplication.ModalResponse {
        let router = AlertKeyRouter(cancel: alert.buttons.count > 2 ? alert.buttons[2] : nil)
        let monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            let keyCode = event.keyCode
            let unmodified = event.modifierFlags
                .intersection(.deviceIndependentFlagsMask)
                .subtracting(.capsLock)
                .isEmpty
            let consumed = MainActor.assumeIsolated {
                router.consume(keyCode: keyCode, unmodified: unmodified)
            }
            return consumed ? nil : event
        }
        defer {
            if let monitor { NSEvent.removeMonitor(monitor) }
        }
        return alert.runModal()
    }

    public static func choice(for response: NSApplication.ModalResponse) -> LocalCloseChoice {
        switch response {
        case .alertFirstButtonReturn:
            return .save
        case .alertSecondButtonReturn:
            return .discard
        default:
            return .cancel
        }
    }
}

private final class AlertKeyRouter: @unchecked Sendable {
    let cancel: NSButton?

    init(cancel: NSButton?) {
        self.cancel = cancel
    }

    @MainActor
    func consume(keyCode: UInt16, unmodified: Bool) -> Bool {
        guard unmodified, keyCode == 53, let cancel else { return false }
        cancel.performClick(nil)
        return true
    }
}
#endif
