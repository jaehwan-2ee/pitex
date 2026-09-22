#!/usr/bin/env python3
"""Exercise production ghost acceptance code against native AppKit marked text."""
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
source = (root / 'Mac/Sources/Features/EditorContainerView.swift').read_text()
methods = source[source.index('    private func applySuggestion('):source.index('    /// Workspace close — terminate')]
methods = methods.replace('private func applySuggestion', 'func applySuggestion')
check = '''import AppKit
@MainActor final class Overlay {
    func show(_ text: String, anchor: Int) {}
    func clear() {}
}
@MainActor final class Completion {
    var textView: NSTextView?
    var suggestion: String?
    var suggestionAnchor = 0
    var overlay: Overlay? = Overlay()
''' + methods + '''}
@main struct Check {
    @MainActor static func main() {
        _ = NSApplication.shared
        let view = NSTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 200))
        let completion = Completion()
        completion.textView = view
        completion.applySuggestion("wrong")
        view.setMarkedText("한", selectedRange: NSRange(location: 1, length: 0),
                           replacementRange: NSRange(location: 0, length: 0))
        precondition(view.hasMarkedText())
        precondition(!completion.accept())
        precondition(view.string == "한")
        completion.applySuggestion("late response")
        precondition(completion.suggestion == nil)
        view.unmarkText()
        view.setSelectedRange(NSRange(location: view.string.utf16.count, length: 0))
        completion.applySuggestion("1")
        precondition(completion.accept())
        precondition(view.string == "한1")
        print("PASS native IME marked text blocks late completion and acceptance; committed text accepts normally")
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex-ime-completion-') as temporary:
    directory = Path(temporary)
    main = directory / 'check.swift'
    main.write_text(check)
    binary = directory / 'check'
    subprocess.run(['swiftc', '-O', '-parse-as-library', str(main), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=20)
