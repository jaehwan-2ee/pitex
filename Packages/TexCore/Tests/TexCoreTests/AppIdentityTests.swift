import Foundation
import TexDomain
import XCTest

final class AppIdentityTests: XCTestCase {
    func testStableDefaults() {
        XCTAssertEqual(AppIdentity.stable.appName, "Pitex")
        XCTAssertEqual(AppIdentity.stable.appFolderName, "Pitex")
        XCTAssertEqual(AppIdentity.stable.tempPrefix, "pitex")
        XCTAssertEqual(AppIdentity.stable.brewCaskName, "pitex")
        XCTAssertFalse(AppIdentity.stable.isNightly)
        XCTAssertEqual(
            AppIdentity.stable.releaseAPI.absoluteString,
            "https://api.github.com/repos/jaehwan-2ee/pitex/releases/latest"
        )
    }

    func testNightlyDefaults() {
        XCTAssertEqual(AppIdentity.nightly.appName, "Pitex Nightly")
        XCTAssertEqual(AppIdentity.nightly.appFolderName, "Pitex Nightly")
        XCTAssertEqual(AppIdentity.nightly.tempPrefix, "pitex-nightly")
        XCTAssertEqual(AppIdentity.nightly.brewCaskName, "pitex@nightly")
        XCTAssertTrue(AppIdentity.nightly.isNightly)
        XCTAssertEqual(
            AppIdentity.nightly.releaseAPI.absoluteString,
            "https://api.github.com/repos/jaehwan-2ee/pitex/releases/tags/nightly"
        )
    }
}
