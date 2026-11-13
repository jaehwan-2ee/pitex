import AppPorts
import XCTest

final class AgentStdoutAssemblerTests: XCTestCase {
    func testCompleteLineAtTheCapIsDelivered() {
        var assembler = AgentStdoutAssembler(maxLineBytes: 4)
        let lines = assembler.append(Data("abcd\n".utf8))
        XCTAssertEqual(lines, [Data("abcd".utf8)])
        XCTAssertTrue(assembler.buffered.isEmpty)
    }

    func testUnterminatedBytesPastTheCapAreDroppedUntilTheNextLine() {
        var assembler = AgentStdoutAssembler(maxLineBytes: 8)
        XCTAssertEqual(assembler.append(Data(repeating: UInt8(ascii: "x"), count: 20)), [])
        XCTAssertTrue(assembler.buffered.isEmpty)

        let lines = assembler.append(Data("tail\n{\"ok\":1}\n".utf8))
        XCTAssertEqual(lines, [Data("{\"ok\":1}".utf8)])
        XCTAssertTrue(assembler.buffered.isEmpty)
    }

    func testACompleteLineLongerThanTheCapIsDroppedAndTheNextLineIsKept() {
        var assembler = AgentStdoutAssembler(maxLineBytes: 4)
        XCTAssertEqual(
            assembler.append(Data("{\"ok\":1}\nok\n".utf8)),
            [Data("ok".utf8)]
        )
    }

    func testOverflowInsideOneChunkDropsThatLineAndKeepsTheNext() {
        var assembler = AgentStdoutAssembler(maxLineBytes: 4)
        let lines = assembler.append(Data("abcde\nok\n".utf8))
        XCTAssertEqual(lines, [Data("ok".utf8)])
    }

    func testCarriageReturnBeforeNewlineIsNotPartOfTheLine() {
        var assembler = AgentStdoutAssembler(maxLineBytes: 8)
        XCTAssertEqual(assembler.append(Data("ab\r\n".utf8)), [Data("ab".utf8)])
    }

    func testProductionCapIsFourMebibytes() {
        XCTAssertEqual(AgentStdoutAssembler.defaultMaxLineBytes, 4 * 1024 * 1024)
        XCTAssertEqual(AgentStdoutAssembler().maxLineBytes, 4 * 1024 * 1024)
    }
}
