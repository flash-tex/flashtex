import XCTest
@testable import FlashTeXMac

/// The DONE's line index (`EngineV3LineStarts`) gives exactly what
/// `EngineV3Session.lineByteRange` gives, for every line and past the end
/// (APP-EDITOR-INSTANT: one pass per file instead of one walk per diagnostic).
@MainActor
final class EngineV3LineStartsTests: XCTestCase {
    func testTheIndexMatchesTheWalkOnEveryLine() {
        let texts = ["", "a", "a\n", "\n", "\n\n", "one\ntwo\n\nfour", "x\r\ny\n", "é\n€ two\n𝔸\n", String(repeating: "line\n", count: 300) + "tail"]
        for text in texts {
            let ix = EngineV3LineStarts(text)
            let lines = text.utf8.filter { $0 == 0x0A }.count + 1
            for line in -1 ... lines + 2 {
                XCTAssertEqual(ix.range(line: line), EngineV3Session.lineByteRange(text, line: line), "line \(line) of \(text.debugDescription)")
            }
        }
    }

    func testIndexesAreBuiltOncePerFile() {
        var lines = EngineV3LineIndexes()
        XCTAssertEqual(lines.range("a.tex", "x\nyy\n", line: 2), 2 ..< 4)
        // The same file's index is reused (the text is the compile's, fixed per DONE).
        XCTAssertEqual(lines.range("a.tex", "ignored", line: 1), 0 ..< 1)
        XCTAssertEqual(lines.range("b.tex", "ab", line: 1), 0 ..< 2)
    }
}
