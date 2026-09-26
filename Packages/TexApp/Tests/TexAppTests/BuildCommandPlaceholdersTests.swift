import BuildFeature
import Foundation
import XCTest

/// Live command substitution must produce shell-safe words for hostile
/// filenames and collapse already-quoted placeholders instead of nesting.
final class BuildCommandPlaceholdersTests: XCTestCase {

    func testTectonicLiveTemplateWithPlainNames() {
        XCTAssertEqual(
            BuildCommandPlaceholders.expand(
                "tectonic --synctex --outdir {outdir} {file}",
                file: "manuscript/main.tex",
                filename: "manuscript/main",
                outdir: ".pitex-live/manuscript/main"
            ),
            "tectonic --synctex --outdir '.pitex-live/manuscript/main' 'manuscript/main.tex'"
        )
    }

    func testSpacesAndApostrophesAreShellSafe() {
        let command = BuildCommandPlaceholders.expand(
            "latexmk -outdir={outdir} {file}",
            file: "it's main/rough draft.tex",
            filename: "it's main/rough draft",
            outdir: ".pitex-live/it's main/rough draft"
        )
        // A POSIX single-quoted word per placeholder, apostrophes escaped
        // as '\'' — the shell sees exactly one argument each.
        XCTAssertEqual(
            command,
            "latexmk -outdir='.pitex-live/it'\\''s main/rough draft' 'it'\\''s main/rough draft.tex'"
        )
    }

    func testQuotedPlaceholdersCollapseInsteadOfNesting() {
        XCTAssertEqual(
            BuildCommandPlaceholders.expand(
                "run \"{file}\" '{outdir}' \"{filename}\"",
                file: "a b.tex", filename: "a b", outdir: "o dir"
            ),
            "run 'a b.tex' 'o dir' 'a b'"
        )
    }

    func testRepeatedPlaceholdersAllExpand() {
        XCTAssertEqual(
            BuildCommandPlaceholders.expand(
                "{file} {file} {outdir}",
                file: "m.tex", filename: "m", outdir: "o"
            ),
            "'m.tex' 'm.tex' 'o'"
        )
    }

    func testTemplateWithoutPlaceholdersIsUnchanged() {
        XCTAssertEqual(
            BuildCommandPlaceholders.expand("latexmk -pdf main.tex", file: "x", filename: "y", outdir: "z"),
            "latexmk -pdf main.tex"
        )
    }

    func testValuesContainingPlaceholderTextAreDataNotInput() {
        // A filename literally carrying `{outdir}`/`{filename}` must not
        // be rewritten — only the template's tokens expand.
        XCTAssertEqual(
            BuildCommandPlaceholders.expand(
                "{file} {outdir}",
                file: "my {outdir} notes.tex",
                filename: "x",
                outdir: "out {file} dir"
            ),
            "'my {outdir} notes.tex' 'out {file} dir'"
        )
    }

    /// Manual mode: {file}/{filename} emit verbatim (compat), {outdir}
    /// stays quoted, a user-quoted "{file}" keeps its quotes, and the
    /// template is still scanned in one pass — placeholder text inside
    /// a substituted value is data, never input.
    func testManualModeKeepsRawFileNamesAndUserQuotes() {
        XCTAssertEqual(
            BuildCommandPlaceholders.expand(
                "xelatex -synctex=1 \"{file}\" -output-directory={outdir} {filename}",
                file: "dir/{outdir} main.tex", filename: "dir/{outdir} main",
                outdir: "out dir", quotePaths: false
            ),
            "xelatex -synctex=1 \"dir/{outdir} main.tex\" -output-directory='out dir' dir/{outdir} main"
        )
    }

    /// The expanded words are fed to a real POSIX shell — each placeholder
    /// value must arrive as exactly one argv entry, quotes and literal
    /// braces intact.
    func testExpandedWordsAreSingleShellArguments() throws {
        guard FileManager.default.isExecutableFile(atPath: "/bin/sh") else {
            throw XCTSkip("no /bin/sh on this host")
        }
        let expanded = BuildCommandPlaceholders.expand(
            "{outdir} {file}",
            file: "it's {outdir} draft.tex",
            filename: "x",
            outdir: "o dir/{file}"
        )
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/sh")
        process.arguments = ["-c", "set -- \(expanded); for a in \"$@\"; do printf '%s\\n' \"$a\"; done"]
        let pipe = Pipe()
        process.standardOutput = pipe
        try process.run()
        process.waitUntilExit()
        let output = String(decoding: pipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
        XCTAssertEqual(process.terminationStatus, 0)
        XCTAssertEqual(output, "o dir/{file}\nit's {outdir} draft.tex\n")
    }
}
