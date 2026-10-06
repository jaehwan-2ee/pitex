//! Exercise production ghost acceptance code against native AppKit marked text.
use pitex_native_tools::{repo_root, run_checked, TempDir};
use std::{error::Error, fs, io, process::Command, time::Duration};

fn main() {
    pitex_native_tools::exit_on_error(check());
}

fn check() -> Result<(), Box<dyn Error>> {
    let root = repo_root()?;
    let source = fs::read_to_string(root.join("Mac/Sources/Features/EditorContainerView.swift"))?;
    let start = source
        .find("    private func applySuggestion(")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "applySuggestion not found"))?;
    let end = source
        .find("    /// Workspace close — terminate")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "completion methods end not found",
            )
        })?;
    let methods =
        source[start..end].replace("private func applySuggestion", "func applySuggestion");
    let check = format!(
        "{}{methods}{}",
        r#"import AppKit
@MainActor final class Overlay {
    func show(_ text: String, anchor: Int) {}
    func clear() {}
}
@MainActor final class Completion {
    var textView: NSTextView?
    var suggestion: String?
    var suggestionAnchor = 0
    var overlay: Overlay? = Overlay()
"#,
        r#"}
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
"#
    );
    let directory = TempDir::new("pitex-ime-completion-")?;
    let main = directory.path().join("check.swift");
    fs::write(&main, check)?;
    let binary = directory.path().join("check");
    run_checked(
        Command::new("swiftc")
            .args(["-O", "-parse-as-library"])
            .arg(&main)
            .arg("-o")
            .arg(&binary),
        None,
    )?;
    run_checked(&mut Command::new(&binary), Some(Duration::from_secs(20)))?;
    directory.close()?;
    Ok(())
}
