#if os(macOS)
import AppKit

/// A save that failed keeps the workspace and its unsaved text on screen and explains why.
/// Save As… is offered only when the active document itself cannot be written.
@MainActor
public enum SaveFailureAlert {
    public static func make(
        title: String,
        detail: String,
        saveAsTitle: String?,
        dismissTitle: String
    ) -> NSAlert {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = title
        alert.informativeText = detail
        if let saveAsTitle { alert.addButton(withTitle: saveAsTitle) }
        alert.addButton(withTitle: dismissTitle)
        return alert
    }

    public static func wantsSaveAs(_ alert: NSAlert, response: NSApplication.ModalResponse) -> Bool {
        alert.buttons.count > 1 && response == .alertFirstButtonReturn
    }
}
#endif
