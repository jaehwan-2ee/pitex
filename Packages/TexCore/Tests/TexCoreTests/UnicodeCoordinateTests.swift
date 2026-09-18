import LanguageCore
import XCTest

final class UnicodeCoordinateTests: XCTestCase {
    func testKoreanJapaneseEmojiAndUTF16SurrogateCoordinates() throws {
        let map = UnicodeCoordinateMap("한😀\n日")

        XCTAssertEqual(map.utf8Count, 11)
        XCTAssertEqual(map.utf16Count, 5)
        XCTAssertEqual(try map.utf16Offset(forUTF8Offset: 3), 1)
        XCTAssertEqual(try map.utf16Offset(forUTF8Offset: 7), 3)
        XCTAssertEqual(try map.utf8Offset(forUTF16Offset: 3), 7)
        XCTAssertEqual(try map.position(forUTF8Offset: 7), TextPosition(line: 0, column: 3))
        XCTAssertEqual(try map.position(forUTF8Offset: 8), TextPosition(line: 1, column: 0))
        XCTAssertEqual(try map.utf8Offset(for: TextPosition(line: 1, column: 1)), 11)
        XCTAssertEqual(try map.utf16Offset(for: TextPosition(line: 1, column: 1)), 5)
    }

    func testMidScalarAndSurrogateOffsetsAreRejected() throws {
        let map = UnicodeCoordinateMap("😀")

        for invalidUTF8 in 1...3 {
            XCTAssertThrowsError(try map.utf16Offset(forUTF8Offset: invalidUTF8)) { error in
                XCTAssertEqual(error as? CoordinateMapError, .invalidUTF8Offset(invalidUTF8))
            }
        }
        XCTAssertThrowsError(try map.utf8Offset(forUTF16Offset: 1)) { error in
            XCTAssertEqual(error as? CoordinateMapError, .invalidUTF16Offset(1))
        }
        XCTAssertThrowsError(try map.position(forUTF16Offset: 1))
    }

    func testCombiningSequenceScalarBoundaryIsRejectedAsMidGrapheme() throws {
        let source = "e\u{301}"
        let map = UnicodeCoordinateMap(source)

        XCTAssertEqual(source.unicodeScalars.count, 2)
        XCTAssertEqual(map.utf8Count, 3)
        XCTAssertEqual(map.utf16Count, 2)
        XCTAssertThrowsError(try map.utf16Offset(forUTF8Offset: 1))
        XCTAssertThrowsError(try map.utf8Offset(forUTF16Offset: 1))
        XCTAssertThrowsError(try map.utf8Offset(for: TextPosition(line: 0, column: 1)))
        XCTAssertEqual(try map.utf8Offset(for: TextPosition(line: 0, column: 2)), 3)
    }

    func testComplexEmojiGraphemeOnlyExposesOuterBoundaries() throws {
        let emoji = "👩🏽‍💻"
        let map = UnicodeCoordinateMap("A" + emoji + "B")
        let emojiUTF8 = emoji.utf8.count
        let emojiUTF16 = emoji.utf16.count

        XCTAssertEqual(try map.utf16Offset(forUTF8Offset: 1), 1)
        XCTAssertEqual(try map.utf16Offset(forUTF8Offset: 1 + emojiUTF8), 1 + emojiUTF16)
        XCTAssertThrowsError(try map.utf16Offset(forUTF8Offset: 5))
        XCTAssertThrowsError(try map.utf8Offset(forUTF16Offset: 3))
    }

    func testAllValidGraphemeBoundariesRoundTrip() throws {
        let source = "한국어 日本語 👨‍👩‍👧‍👦 café\n끝"
        let map = UnicodeCoordinateMap(source)
        var utf8 = 0

        XCTAssertEqual(try map.utf8Offset(forUTF16Offset: 0), 0)
        for character in source {
            utf8 += String(character).utf8.count
            let utf16 = try map.utf16Offset(forUTF8Offset: utf8)
            XCTAssertEqual(try map.utf8Offset(forUTF16Offset: utf16), utf8)
            let position = try map.position(forUTF8Offset: utf8)
            XCTAssertEqual(try map.utf8Offset(for: position), utf8)
        }
    }

    func testCRLFIsOneLineBreakAndInvalidPositionsFailClosed() throws {
        let map = UnicodeCoordinateMap("a\r\nb")

        XCTAssertEqual(try map.position(forUTF8Offset: 3), TextPosition(line: 1, column: 0))
        XCTAssertEqual(try map.utf8Offset(for: TextPosition(line: 1, column: 1)), 4)
        XCTAssertThrowsError(try map.utf8Offset(for: TextPosition(line: -1, column: 0)))
        XCTAssertThrowsError(try map.utf16Offset(for: TextPosition(line: 0, column: 2)))
        XCTAssertThrowsError(try map.position(forUTF8Offset: 2))
    }
}
