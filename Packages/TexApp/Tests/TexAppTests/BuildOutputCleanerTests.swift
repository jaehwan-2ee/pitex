import BuildFeature
import Foundation
import XCTest

final class BuildOutputCleanerTests: XCTestCase {
    /// R04: a directory or link that merely carries an output name is the
    /// user's. Clean removes only regular output files.
    func testCleanRemovesOutputFilesButNeverADirectoryOrLink() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("clean-\(UUID().uuidString)")
        try fm.createDirectory(at: root.appendingPathComponent("paper.out"), withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }
        let notes = root.appendingPathComponent("paper.out/notes.txt")
        try Data("hand-written".utf8).write(to: notes)
        try Data("aux".utf8).write(to: root.appendingPathComponent("paper.aux"))
        let figure = root.appendingPathComponent("figure.pdf")
        try Data("figure".utf8).write(to: figure)
        try fm.createSymbolicLink(at: root.appendingPathComponent("paper.pdf"), withDestinationURL: figure)

        BuildOutputCleaner.clean(stem: "paper", extensions: ["aux", "out", "pdf", "log"], in: root)

        XCTAssertFalse(fm.fileExists(atPath: root.appendingPathComponent("paper.aux").path))
        XCTAssertEqual(try String(contentsOf: notes, encoding: .utf8), "hand-written")
        XCTAssertNotNil(try? fm.destinationOfSymbolicLink(atPath: root.appendingPathComponent("paper.pdf").path))
        XCTAssertEqual(try String(contentsOf: figure, encoding: .utf8), "figure")
    }
}
