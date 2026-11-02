import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// `/Rotate` (spec §4.7 ROTATE, lane RENDER-AUDIT): a page is shown turned
/// clockwise, as a viewer shows it (pdflscape's landscape pages), in the
/// preview and in every rendering of the PDF page (the parity reference,
/// the fallback), by one transform. Before this the display list carried no
/// rotation and the reference ignored it: the gate could not see it.
final class PageRotateTests: XCTestCase {
    func testTheTurnMapsTheUnturnedPageOntoTheShownOne() {
        let (w, h) = (100.0, 60.0)
        func at(_ q: Int, _ p: CGPoint) -> CGPoint { p.applying(DL3Renderer.turn(width: w, height: h, quarterTurns: q)) }
        let p = CGPoint(x: 10, y: 20)
        XCTAssertEqual(at(0, p), p)
        XCTAssertEqual(at(1, p), CGPoint(x: 20, y: w - 10)) // (y, w - x)
        XCTAssertEqual(at(2, p), CGPoint(x: w - 10, y: h - 20)) // (w - x, h - y)
        XCTAssertEqual(at(3, p), CGPoint(x: h - 20, y: 10)) // (h - y, x)
        XCTAssertEqual(at(-1, p), at(3, p))
        // The unturned box lands on the shown page exactly.
        for q in 0 ..< 4 {
            let r = CGRect(x: 0, y: 0, width: w, height: h).applying(DL3Renderer.turn(width: w, height: h, quarterTurns: q))
            XCTAssertEqual(r, CGRect(x: 0, y: 0, width: q % 2 == 0 ? w : h, height: q % 2 == 0 ? h : w), "q \(q)")
        }
    }

    func testAPageFlaggedRotateIsShownTurned() {
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, 100, 60]
        page.flags = 1 << 4 // ROTATE: one quarter turn
        let prepared = DL3PreparedPage(page: page, fonts: [:], images: [:])
        XCTAssertEqual(page.quarterTurns, 1)
        XCTAssertEqual(prepared.widthPt, 60)
        XCTAssertEqual(prepared.heightPt, 100)
        XCTAssertFalse(DL3Renderer.tilesByTranslation(prepared))
    }

    func testThePDFReferenceIsShownTurned() throws {
        // make_inputs.py's page 5: MediaBox [-20 -10 80 80], /Rotate 90.
        let url = PreviewParityTests.repoRoot.appendingPathComponent("fixtures/preview-probes/pv-pdf-inclusion/pages.pdf")
        let doc = try XCTUnwrap(DL3Renderer.openPDF(url))
        let turned = try XCTUnwrap(DL3Renderer.page(of: doc, at: 5))
        XCTAssertEqual(DL3Renderer.quarterTurns(turned), 1)
        let img = try XCTUnwrap(DL3Renderer.rasterize(pdfPage: turned, scale: 1))
        XCTAssertEqual([img.width, img.height], [90, 100])
        let plain = try XCTUnwrap(DL3Renderer.page(of: doc, at: 1))
        XCTAssertEqual(DL3Renderer.quarterTurns(plain), 0)
    }
}
