import AppPorts
import XCTest

final class AgentStdoutAssemblerTests: XCTestCase {
    func testCompleteLineAtTheCapIsDelivered() {
        var assembler = AgentStdoutAssembler(maxUnterminatedBytes: 4)
        let lines = assembler.append(Data("abcd\n".utf8))
        XCTAssertEqual(lines, [Data("abcd".utf8)])
        XCTAssertTrue(assembler.buffered.isEmpty)
    }

    func testUnterminatedBytesPastTheCapAreDroppedUntilTheNextLine() {
        var assembler = AgentStdoutAssembler(maxUnterminatedBytes: 4)
        XCTAssertTrue(assembler.append(Data("abcdef".utf8)).isEmpty)
        XCTAssertLessThanOrEqual(assembler.buffered.count, 4)

        let lines = assembler.append(Data("junk\n{\"ok\":1}\n".utf8))
        XCTAssertEqual(lines, [Data("{\"ok\":1}".utf8)])
        XCTAssertTrue(assembler.buffered.isEmpty)
    }

    func testOverflowInsideOneChunkDropsThatLineAndKeepsTheNext() {
        var assembler = AgentStdoutAssembler(maxUnterminatedBytes: 4)
        let lines = assembler.append(Data("abcde\nok\n".utf8))
        XCTAssertEqual(lines, [Data("ok".utf8)])
    }

    func testCarriageReturnBeforeNewlineIsNotPartOfTheLine() {
        var assembler = AgentStdoutAssembler(maxUnterminatedBytes: 8)
        XCTAssertEqual(assembler.append(Data("ab\r\n".utf8)), [Data("ab".utf8)])
    }

    func testProductionCapIsFourMebibytes() {
        XCTAssertEqual(AgentStdoutAssembler.defaultMaxUnterminatedBytes, 4 * 1024 * 1024)
        XCTAssertEqual(AgentStdoutAssembler().maxUnterminatedBytes, 4 * 1024 * 1024)
    }
}
