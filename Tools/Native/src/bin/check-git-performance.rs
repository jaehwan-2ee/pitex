//! Release-mode Git parsing benchmark using the production Swift sources.
use pitex_native_tools::{repo_root, run_checked, TempDir};
use std::{error::Error, fs, io, process::Command};

fn main() {
    pitex_native_tools::exit_on_error(check());
}

fn check() -> Result<(), Box<dyn Error>> {
    let root = repo_root()?;
    let view = fs::read_to_string(root.join("Mac/Sources/Features/GitIntegrationView.swift"))?;
    let start = view
        .find("    nonisolated private static func commitMessagePrompt(")
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "commitMessagePrompt not found")
        })?;
    let end = view[start..]
        .find("    /// `pi --print`")
        .map(|offset| start + offset)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "prompt end not found"))?;
    let prompt = view[start..end].replace("nonisolated private static", "static");
    let source = format!(
        "import Foundation\nenum Prompt {{\n{prompt}{}",
        r###"}
@main struct Check {
    static func main() {
        for count in [20_000, 1_012_101] {
            var source = "## main\0"
            for i in 0..<count { source += "?? files/\(i).txt\0" }
            let start = ContinuousClock.now
            let status = GitSupport.parseStatus(source, root: "/tmp/git-check")
            let elapsed = start.duration(to: .now)
            precondition(status.unstaged.count == count)
            precondition(status.unstaged.last?.path == "files/\(count - 1).txt")
            precondition(elapsed < .seconds(2), "Git parsing performance regressed")
            print("Git parse: \(count) files in \(elapsed)")
        }
        // A single grapheme can contain arbitrarily many UTF-8 bytes.
        let longName = "a" + String(repeating: "\u{301}", count: 50_000)
        let prompt = Prompt.commitMessagePrompt(branch: "main", files: Array(repeating: longName, count: 100),
                                                diff: String(repeating: longName, count: 10), totalFiles: 1_012_101)
        precondition(prompt.utf8.count < 50_000)
        precondition(prompt.contains("1012101 files total"))
        precondition(prompt.contains("[diff truncated]"))
        print("Git prompt: bounded to \(prompt.utf8.count) UTF-8 bytes")
    }
}
"###
    );
    let directory = TempDir::new("pitex-git-check-")?;
    let main = directory.path().join("check.swift");
    fs::write(&main, source)?;
    let binary = directory.path().join("check");
    run_checked(
        Command::new("swiftc")
            .args(["-O", "-parse-as-library"])
            .arg(root.join("Packages/TexCore/Sources/GitCore/GitSupport.swift"))
            .arg(&main)
            .arg("-o")
            .arg(&binary),
        None,
    )?;
    run_checked(&mut Command::new(&binary), None)?;
    directory.close()?;
    Ok(())
}
