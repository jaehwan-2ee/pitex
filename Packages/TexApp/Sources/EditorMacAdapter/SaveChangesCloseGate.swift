#if os(macOS)
import AppKit

@MainActor
public final class SaveChangesCloseGate: NSObject, NSWindowDelegate {
    public private(set) var isPrompting = false
    private var closeInFlight = false
    private weak var guardedWindow: NSWindow?
    nonisolated(unsafe) private weak var original: (any NSWindowDelegate)?
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
        original = window.delegate
        window.delegate = self
    }

    public func restore() {
        guard let window = guardedWindow, window.delegate === self else {
            guardedWindow = nil
            original = nil
            return
        }
        window.delegate = original
        guardedWindow = nil
        original = nil
    }

    public func windowShouldClose(_ sender: NSWindow) -> Bool {
        if original?.windowShouldClose?(sender) == false { return false }
        guard shouldPrompt() else { return true }
        guard !closeInFlight else { return false }
        closeInFlight = true
        isPrompting = true
        Task { @MainActor in
            let accepted = await self.confirm()
            self.isPrompting = false
            self.closeInFlight = false
            guard accepted else { return }
            sender.performClose(nil)
        }
        return false
    }

    nonisolated override func responds(to selector: Selector!) -> Bool {
        super.responds(to: selector) || (original?.responds(to: selector) ?? false)
    }

    nonisolated override func forwardingTarget(for selector: Selector!) -> Any? {
        original?.responds(to: selector) == true ? original : super.forwardingTarget(for: selector)
    }
}
#endif
