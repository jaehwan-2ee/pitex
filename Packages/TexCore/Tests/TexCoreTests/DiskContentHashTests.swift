import DocumentSessionCore
import XCTest

#if canImport(Darwin)
import Foundation
#endif

/// The hash reads through contiguous UTF-8 storage (falling back to a native
/// copy for bridged strings); these pin byte-for-byte parity with a plain
/// byte loop so the digests on disk never change.
final class DiskContentHashTests: XCTestCase {
    private func referenceHash(_ text: String) -> DiskContentHash {
        var hash: UInt64 = 14_695_981_039_346_656_037
        for byte in text.utf8 {
            hash ^= UInt64(byte)
            hash &*= 1_099_511_628_211
        }
        return DiskContentHash(rawValue: hash)
    }

    private var cases: [String] {
        [
            "",
            "a",
            "plain ascii \\section{intro}\nbody text\n",
            "한글 문서\n\\section{제목}\n내용\n",
            "emoji 👩🏽‍💻 and family 👨‍👩‍👧‍👦 zwj\n",
            "combining e\u{301} cafe\u{301} vs précomposed\n",
            "CRLF\r\nline two\r\n\r\n",
            "line separator\u{2028}and paragraph\u{2029}breaks\n",
            String(repeating: "\\section{long} 한글 👩🏽‍💻 tail\n", count: 20_000),
        ]
    }

    func testHashingMatchesTheReferenceByteLoop() {
        for text in cases {
            XCTAssertEqual(DiskContentHash.hashing(text), referenceHash(text), "mismatch for \(text.prefix(20))")
        }
    }

    func testEmptyStringKeepsTheFNVOffsetBasis() {
        XCTAssertEqual(DiskContentHash.hashing("").rawValue, 14_695_981_039_346_656_037)
    }

    #if canImport(Darwin)
    func testBridgedForeignStringsHashIdentically() {
        for text in cases {
            // NSMutableString bridging keeps foreign storage, exercising the
            // non-contiguous fallback the per-keystroke path now relies on.
            let bridged = NSMutableString(string: text) as String
            XCTAssertEqual(DiskContentHash.hashing(bridged), referenceHash(text), "mismatch for \(text.prefix(20))")
        }
    }
    #endif
}
