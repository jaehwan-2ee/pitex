#if os(macOS)
import AppKit

/// Asks before the red close button orders a document window out.
///
/// The window keeps SwiftUI's delegate. Replacing that delegate stops
/// LaunchServices from reusing the window, and refusing `windowShouldClose`
/// during quit deadlocks `NSApp.terminate`.
@MainActor
public final class SaveChangesCloseGate: NSObject {
    public private(set) var isPrompting = false
    private var closeInFlight = false
    private weak var guardedWindow: NSWindow?
    private weak var closeButton: NSButton?
    private weak var originalTarget: AnyObject?
    private var originalAction: Selector?
    private let shouldPrompt: @MainActor () -> Bool
    private let confirm: @MainActor () async -> Bool

    public init(
        shouldPrompt: @escaping @MainActor () -> Bool,
        confirm: @escaping @MainActor () async -> Bool
    ) {
        self.shouldPrompt = shouldPrompt
        self.confirm = confirm
    }

    public func attach(to window: NSWindow) {
        restore()
        guardedWindow = window
        guard let button = window.standardWindowButton(.closeButton) else { return }
        closeButton = button
        originalTarget = button.target
        originalAction = button.action
        button.target = self
        button.action = #selector(closeClicked(_:))
    }

    public func restore() {
        if let closeButton {
            closeButton.target = originalTarget
            closeButton.action = originalAction
        }
        closeButton = nil
        originalTarget = nil
        originalAction = nil
        guardedWindow = nil
    }

    @objc public func closeClicked(_ sender: Any?) {
        guard let window = guardedWindow else { return }
        Task { @MainActor in
            await self.beginClose(of: window, sender: sender)
        }
    }

    @discardableResult
    public func beginClose(of window: NSWindow, sender: Any?) async -> Bool {
        guard shouldPrompt() else {
            window.performClose(sender)
            return true
        }
        guard !closeInFlight else { return false }
        closeInFlight = true
        isPrompting = true
        let accepted = await confirm()
        isPrompting = false
        closeInFlight = false
        guard accepted else { return false }
        window.performClose(sender)
        return true
    }
}
#endif
