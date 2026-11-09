import Foundation
import XCTest
@testable import FlashTeXPreviewV3

/// The session opens the compile's PDF for fallback pages off main, while
/// the next compile may already be rewriting it in place (REVIEW-B on
/// #1971). Bytes read then do not end in `%%EOF`: a cold run truncated the
/// file and is writing it from the start, or a resident run blanked the old
/// run's last 1,024 bytes before writing over it. `endsWhole` tells those
/// from a finished file, which CGPDF alone does not: it repairs a torn file
/// and can open it.
final class PDFEndsWholeTests: XCTestCase {
    func testOnlyAFileEndingInEOFIsWhole() throws {
        let whole = try Data(contentsOf: PDFConcurrentDrawTests.madrid) // pdflatex's: ends "%%EOF\n"
        XCTAssertTrue(DL3Renderer.endsWhole(whole))
        XCTAssertTrue(DL3Renderer.endsWhole(whole.dropLast()), "%%EOF with no end of line")
        var crlf = whole.dropLast()
        crlf.append(contentsOf: Array("\r\n".utf8))
        XCTAssertTrue(DL3Renderer.endsWhole(crlf))

        // a cold run part-way: the file from the start, cut short
        XCTAssertFalse(DL3Renderer.endsWhole(whole.prefix(whole.count / 2)))
        XCTAssertFalse(DL3Renderer.endsWhole(whole.dropLast(3)))
        XCTAssertFalse(DL3Renderer.endsWhole(Data()))
        // a resident run part-way: the old run's last 1,024 bytes blanked (spaces)
        var blanked = whole
        blanked.replaceSubrange((whole.count - 1024) ..< whole.count, with: Data(repeating: 0x20, count: 1024))
        XCTAssertFalse(DL3Renderer.endsWhole(blanked))
        // ... and a blank after %%EOF is one too
        var spaced = whole
        spaced.append(0x20)
        XCTAssertFalse(DL3Renderer.endsWhole(spaced))
        // a slice whose indices do not start at 0
        let tail = whole.suffix(10)
        XCTAssertTrue(DL3Renderer.endsWhole(tail))
    }
}
