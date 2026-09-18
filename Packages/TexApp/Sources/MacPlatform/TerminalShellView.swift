import AppPorts
import Foundation

#if os(macOS)
import AppKit
import SwiftTerm
import SwiftUI

/// Shared handle the workspace uses to push text and command lines into the
/// embedded terminal. The SwiftUI wrapper binds a live
/// `LocalProcessTerminalView` to it, so model code can feed status output or
/// send user commands without holding a view reference.
@MainActor
public final class TerminalSession: ObservableObject {
    weak var view: LocalProcessTerminalView?

    public init() {}

    /// True while a terminal view is attached and its shell is running.
    public internal(set) var isRunning = false

    /// Displays raw text (ANSI sequences allowed) in the terminal buffer —
    /// used for app-generated notices such as build/clean status lines.
    public func feed(_ text: String) {
        view?.terminal.feed(text: text)
    }

    /// Sends a command line to the shell exactly as if the user typed it
    /// and pressed Return.
    public func send(_ command: String) {
        view?.send(txt: command + "\r")
    }
}

/// Interactive terminal backed by SwiftTerm — the same component the
/// reference editor embeds for its console Terminal tab. Hosts a login shell
/// rooted at the project directory; the app feeds it status text through a
/// `TerminalSession` instead of rendering transcript strings itself.
public struct TerminalShellView: NSViewRepresentable {
    /// Directory the shell starts in; when it changes the running shell
    /// receives a `cd` so the pane follows the open project.
    let workingDirectory: URL?
    let session: TerminalSession
    let font: NSFont
    let foregroundColor: NSColor
    let backgroundColor: NSColor

    public init(
        workingDirectory: URL?,
        session: TerminalSession,
        font: NSFont = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular),
        foregroundColor: NSColor = .textColor,
        backgroundColor: NSColor = .textBackgroundColor
    ) {
        self.workingDirectory = workingDirectory
        self.session = session
        self.font = font
        self.foregroundColor = foregroundColor
        self.backgroundColor = backgroundColor
    }

    public func makeNSView(context: Context) -> LocalProcessTerminalView {
        let view = LocalProcessTerminalView(
            frame: NSRect(x: 0, y: 0, width: 480, height: 160),
            font: font,
            options: TerminalOptions()
        )
        view.nativeForegroundColor = foregroundColor
        view.nativeBackgroundColor = backgroundColor
        view.processDelegate = context.coordinator
        context.coordinator.session = session
        context.coordinator.attach(view: view, directory: workingDirectory)
        return view
    }

    public func updateNSView(_ view: LocalProcessTerminalView, context: Context) {
        if view.nativeForegroundColor != foregroundColor {
            view.nativeForegroundColor = foregroundColor
        }
        if view.nativeBackgroundColor != backgroundColor {
            view.nativeBackgroundColor = backgroundColor
        }
        if view.font != font { view.font = font }
        context.coordinator.session = session
        context.coordinator.syncDirectory(workingDirectory, in: view)
    }

    public static func dismantleNSView(_ nsView: LocalProcessTerminalView, coordinator: Coordinator) {
        coordinator.detach(from: nsView)
    }

    public func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    public final class Coordinator: NSObject, LocalProcessTerminalViewDelegate {
        var session: TerminalSession?
        private weak var view: LocalProcessTerminalView?
        private var currentDirectory: String?
        /// Guards the auto-respawn so a missing/broken shell binary does not
        /// loop forever; resets after a user-visible clean run.
        private var attemptedRespawn = false
        private var shell: String {
            ProcessInfo.processInfo.environment["SHELL"] ?? "/bin/zsh"
        }

        func attach(view: LocalProcessTerminalView, directory: URL?) {
            self.view = view
            session?.view = view
            currentDirectory = directory?.path
            var environment = ProcessInfo.processInfo.environment
            environment["TERM"] = "xterm-256color"
            environment["COLORTERM"] = "truecolor"
            environment["TERM_PROGRAM"] = "Pitex"
            view.startProcess(
                executable: shell,
                args: ["-l"],
                environment: environment.map { "\($0.key)=\($0.value)" },
                currentDirectory: currentDirectory
            )
            session?.isRunning = true
        }

        func detach(from view: LocalProcessTerminalView) {
            if session?.view === view { session?.view = nil }
            self.view = nil
        }

        /// When the project directory changes, move the running shell rather
        /// than respawning it (keeps scrollback, matches user expectation).
        func syncDirectory(_ directory: URL?, in view: LocalProcessTerminalView) {
            guard session?.view === view else { attach(view: view, directory: directory); return }
            let path = directory?.path
            guard path != currentDirectory else { return }
            currentDirectory = path
            guard let path else { return }
            let escaped = path.replacingOccurrences(of: "'", with: "'\\''")
            view.send(txt: "cd -- '\(escaped)'\r")
        }

        // The protocol's requirements are nonisolated; hop to the main actor
        // for anything that touches session or terminal state.

        public nonisolated func sizeChanged(
            source: LocalProcessTerminalView, newCols: Int, newRows: Int
        ) {}

        public nonisolated func setTerminalTitle(
            source: LocalProcessTerminalView, title: String
        ) {}

        public nonisolated func hostCurrentDirectoryUpdate(
            source: TerminalView, directory: String?
        ) {
            Task { @MainActor in self.currentDirectory = directory }
        }

        public nonisolated func processTerminated(source: TerminalView, exitCode: Int32?) {
            Task { @MainActor in
                session?.isRunning = false
                source.terminal.feed(text: "\r\n\u{001B}[90m(shell exited)\u{001B}[0m\r\n")
                // Respawn once so `exit` does not leave a dead pane; if the
                // shell cannot stay up the user sees the notice, not a loop.
                guard !attemptedRespawn,
                      let view = source as? LocalProcessTerminalView else { return }
                attemptedRespawn = true
                attach(view: view, directory: currentDirectory.map { URL(fileURLWithPath: $0) })
            }
        }
    }
}
#endif
