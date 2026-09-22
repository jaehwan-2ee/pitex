#!/usr/bin/env python3
"""Release-mode Git parsing benchmark using the production Swift sources."""
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
view = (root / "Mac/Sources/Features/GitIntegrationView.swift").read_text()
start = view.index("    nonisolated private static func commitMessagePrompt(")
end = view.index('    /// `pi --print`', start)
prompt = view[start:end].replace("nonisolated private static", "static")
source = '''import Foundation
enum Prompt {
''' + prompt + '''}
@main struct Check {
    static func main() {
        for count in [20_000, 1_012_101] {
            var source = "## main\\0"
            for i in 0..<count { source += "?? files/\\(i).txt\\0" }
            let start = ContinuousClock.now
            let status = GitSupport.parseStatus(source, root: "/tmp/git-check")
            let elapsed = start.duration(to: .now)
            precondition(status.unstaged.count == count)
            precondition(status.unstaged.last?.path == "files/\\(count - 1).txt")
            precondition(elapsed < .seconds(2), "Git parsing performance regressed")
            print("Git parse: \\(count) files in \\(elapsed)")
        }
        // A single grapheme can contain arbitrarily many UTF-8 bytes.
        let longName = "a" + String(repeating: "\\u{301}", count: 50_000)
        let prompt = Prompt.commitMessagePrompt(branch: "main", files: Array(repeating: longName, count: 100),
                                                diff: String(repeating: longName, count: 10), totalFiles: 1_012_101)
        precondition(prompt.utf8.count < 50_000)
        precondition(prompt.contains("1012101 files total"))
        precondition(prompt.contains("[diff truncated]"))
        print("Git prompt: bounded to \\(prompt.utf8.count) UTF-8 bytes")
    }
}
'''
with tempfile.TemporaryDirectory(prefix="pitex-git-check-") as temporary:
    directory = Path(temporary)
    main = directory / "check.swift"
    main.write_text(source)
    binary = directory / "check"
    subprocess.run(["swiftc", "-O", "-parse-as-library", str(root / "Packages/TexCore/Sources/GitCore/GitSupport.swift"),
                    str(main), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
