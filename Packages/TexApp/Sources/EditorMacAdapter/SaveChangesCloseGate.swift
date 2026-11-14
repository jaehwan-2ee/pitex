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
    private let allowsImmediateClose: @MainActor () -> Bool
    private let confirm: @MainActor () async -> Bool

    public init(
        shouldPrompt: @escaping @MainActor () -> Bool,
        allowsImmediateClose: @escaping @MainActor () -> Bool = { false },
        confirm: @escaping @MainActor () async -> Bool
    ) {
        self.shouldPrompt = shouldPrompt
        self.allowsImmediateClose = allowsImmediateClose
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
        // Quit closes windows on the main thread inside `reply`. A Task
        // scheduled here cannot run until that reply returns, so the quit waits forever.
        if allowsImmediateClose() {
            restore()
            window.performClose(sender)
            return
        }
        Task { @MainActor in
            await self.beginClose(of: window, sender: sender)
        }
    }

    @discardableResult
    public func beginClose(of window: NSWindow, sender: Any?) async -> Bool {
        switch claim(window, sender: sender) {
        case .closed:
            return true
        case .busy:
            return false
        case .prompt:
            return complete(await confirm(), window: window, sender: sender)
        }
    }

    /// Same close decision as `beginClose`. `decide` runs on the caller's stack.
    @discardableResult
    public func performGuardedClose(
        of window: NSWindow,
        sender: Any?,
        decide: @MainActor () -> Bool
    ) -> Bool {
        switch claim(window, sender: sender) {
        case .closed:
            return true
        case .busy:
            return false
        case .prompt:
            return complete(decide(), window: window, sender: sender)
        }
    }

    private enum Claim {
        case closed
        case busy
        case prompt
    }

    private func claim(_ window: NSWindow, sender: Any?) -> Claim {
        guard shouldPrompt() else {
            window.performClose(sender)
            return .closed
        }
        guard !closeInFlight else { return .busy }
        closeInFlight = true
        isPrompting = true
        return .prompt
    }

    private func complete(_ accepted: Bool, window: NSWindow, sender: Any?) -> Bool {
        isPrompting = false
        closeInFlight = false
        guard accepted else { return false }
        window.performClose(sender)
        return true
    }
}
#endif
