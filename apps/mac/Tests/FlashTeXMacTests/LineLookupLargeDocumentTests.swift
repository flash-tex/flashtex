import Foundation
import XCTest
@testable import FlashTeXMac

/// REVIEW-APP round 5: the caret's line for caret follow (each follow, and
/// each caret move while following is paused by a manual scroll), the math
/// hover and copied diagnostics was a byte loop from the top of the text:
/// ~30 ms of main thread per lookup at the end of a 3 MB book (debug build).
@MainActor
final class LineLookupLargeDocumentTests: XCTestCase {
    /// The loops these replaced, as the reference.
    nonisolated static func referenceLineAndColumn(byte: Int, in text: String) -> (Int, Int) {
        var line = 1, start = 0, i = 0
        for b in text.utf8 {
            if i >= byte { break }
            if b == 0x0A { line += 1; start = i + 1 }
            i += 1
        }
        return (line, max(0, byte - start))
    }

    nonisolated static func referenceLineNumber(ofByte offset: Int, in text: String) -> Int? {
        guard offset >= 0, offset <= text.utf8.count else { return nil }
        var line = 1
        for b in text.utf8.prefix(offset) where b == 0x0A { line += 1 }
        return line
    }

    func testLookupsAgreeWithTheByteLoopEverywhere() {
        var rng = SystemRandomNumberGenerator()
        let pieces = ["a", "\n", "\r\n", "é", "\u{1F600}", "\\section{x}", "\n\n", " ", "$x$"]
        for _ in 0 ..< 200 {
            var text = ""
            for _ in 0 ..< Int.random(in: 0 ... 40, using: &rng) { text += pieces.randomElement(using: &rng)! }
            let n = text.utf8.count
            for byte in -2 ... n + 2 {
                let a = EngineV3Session.lineAndColumn(byte: byte, in: text)
                let b = Self.referenceLineAndColumn(byte: byte, in: text)
                XCTAssertTrue(a == b, "lineAndColumn(\(byte)) of \(text.debugDescription): \(a) vs \(b)")
                XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: byte, in: text), Self.referenceLineNumber(ofByte: byte, in: text),
                               "lineNumber(\(byte)) of \(text.debugDescription)")
            }
        }
        // A bridged NSString (not native UTF-8) takes the same path.
        let bridged = NSString(string: "ab\ncd\u{e9}\nef") as String
        XCTAssertTrue(EngineV3Session.lineAndColumn(byte: 9, in: bridged) == (3, 1))
        XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: 4, in: bridged), 2)
    }

    func testTheCaretLineAtTheEndOfAThreeMegabyteBookIsCheap() {
        let book = EditorInstantTests.book(sections: 720)
        let bytes = book.utf8.count
        XCTAssertGreaterThan(bytes, 2_700_000)
        let want = Self.referenceLineAndColumn(byte: bytes - 100, in: book)
        var best = Double.infinity
        for _ in 0 ..< 5 {
            let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            let got = EngineV3Session.lineAndColumn(byte: bytes - 100, in: book)
            let line = EditorDiagnostics.lineNumber(ofByte: bytes - 100, in: book)
            let c1 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            XCTAssertTrue(got == want)
            XCTAssertEqual(line, want.0)
            best = min(best, Double(c1 - c0) / 1e6)
        }
        // Before: 34 ms + 30 ms (byte loops); after: ~0.3 ms (memchr).
        TimingBudget.assertWithin(best, 4, "both line lookups at the end of a \(bytes)-byte book (thread CPU)")
    }
}
