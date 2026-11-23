import TexDomain
import XCTest

final class NavigationHistoryTests: XCTestCase {
    func testBackAndForwardRetraceJumps() {
        var history = NavigationHistory<Int>()
        XCTAssertFalse(history.canGoBack)
        XCTAssertNil(history.goBack(from: 1))

        history.jump(from: 1)  // 1 -> 5
        history.jump(from: 5)  // 5 -> 9
        XCTAssertEqual(history.goBack(from: 9), 5)
        XCTAssertEqual(history.goBack(from: 5), 1)
        XCTAssertFalse(history.canGoBack)
        XCTAssertEqual(history.goForward(from: 1), 5)
        XCTAssertEqual(history.goForward(from: 5), 9)
        XCTAssertFalse(history.canGoForward)
        XCTAssertEqual(history.goBack(from: 9), 5)
    }

    func testNewJumpDropsForwardEntries() {
        var history = NavigationHistory<Int>()
        history.jump(from: 1)
        XCTAssertEqual(history.goBack(from: 5), 1)
        XCTAssertTrue(history.canGoForward)
        history.jump(from: 1)
        XCTAssertFalse(history.canGoForward)
        XCTAssertNil(history.goForward(from: 7))
    }

    func testRepeatedJumpFromSamePlaceIsStoredOnce() {
        var history = NavigationHistory<Int>()
        history.jump(from: 3)
        history.jump(from: 3)
        XCTAssertEqual(history.back, [3])
    }
}
