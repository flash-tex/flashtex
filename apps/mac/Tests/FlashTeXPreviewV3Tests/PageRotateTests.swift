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

    // MARK: the preview's coordinates (y down): clicks and highlights

    /// `shownTurn` is `turn` in the coordinates links, glyph cells and
    /// sources use (bp from the top-left, y down): a clockwise turn.
    func testTheShownTurnMapsPagePointsAsTheyAreShown() {
        let (w, h) = (100.0, 60.0)
        func at(_ q: Int, _ p: CGPoint) -> CGPoint { p.applying(DL3Renderer.shownTurn(width: w, height: h, quarterTurns: q)) }
        let p = CGPoint(x: 10, y: 20)
        XCTAssertEqual(at(0, p), p)
        XCTAssertEqual(at(1, p), CGPoint(x: h - 20, y: 10)) // the top-left corner goes top-right
        XCTAssertEqual(at(2, p), CGPoint(x: w - 10, y: h - 20))
        XCTAssertEqual(at(3, p), CGPoint(x: 20, y: w - 10)) // the top-left corner goes bottom-left
        XCTAssertEqual(at(-1, p), at(3, p))
    }

    /// A shown point maps back onto the page point shown there, and a page
    /// rectangle onto the shown page, for every quarter turn.
    func testPointsRoundTripAndRectsLandOnTheShownPage() {
        for q in 0 ..< 4 {
            var page = DL3Page(kind: .page, index: 0)
            page.box = [0, 0, 100, 60]
            page.flags = UInt32(q) << 4
            let prepared = DL3PreparedPage(page: page, fonts: [:], images: [:])
            for p in [CGPoint(x: 0, y: 0), CGPoint(x: 10, y: 20), CGPoint(x: 99.5, y: 0.25), CGPoint(x: 100, y: 60)] {
                let shown = p.applying(prepared.shownTurn)
                let back = prepared.pagePoint(shown: shown)
                XCTAssertEqual(back.x, p.x, accuracy: 1e-9, "q \(q)"); XCTAssertEqual(back.y, p.y, accuracy: 1e-9, "q \(q)")
            }
            XCTAssertEqual(prepared.shownRect(CGRect(x: 0, y: 0, width: 100, height: 60)),
                           CGRect(x: 0, y: 0, width: prepared.widthPt, height: prepared.heightPt), "q \(q)")
            XCTAssertTrue(prepared.shownRect(.null).isNull)
        }
    }

    /// The mapping agrees with the image: a filled rule at a page rectangle
    /// is drawn where `shownRect` puts it, and not where it is unturned.
    func testTheImageInksWhereTheShownRectSays() throws {
        let K = 65781.76
        let rule = CGRect(x: 10, y: 5, width: 10, height: 10) // bp, y down
        for q in 0 ..< 4 {
            var page = DL3Page(kind: .page, index: 0)
            page.box = [0, 0, 100, 60]
            page.flags = UInt32(q) << 4
            page.items = [.rule(kind: .fill, x: Int32(rule.minX * K), y: Int32(rule.minY * K), w: Int32(rule.width * K), h: Int32(rule.height * K))]
            let prepared = DL3PreparedPage(page: page, fonts: [:], images: [:])
            let img = try XCTUnwrap(DL3Renderer.rasterize(prepared, scale: 1))
            XCTAssertEqual([img.width, img.height], [Int(prepared.widthPt), Int(prepared.heightPt)], "q \(q)")
            let px = DL3Parity.rgba(img)
            func ink(_ p: CGPoint) -> Bool { px[(Int(p.y) * img.width + Int(p.x)) * 4] < 128 }
            let shown = prepared.shownRect(rule)
            XCTAssertTrue(ink(CGPoint(x: shown.midX, y: shown.midY)), "q \(q): inked at \(shown)")
            if q != 0 { XCTAssertFalse(ink(CGPoint(x: rule.midX, y: rule.midY)), "q \(q): not at the unturned place") }
        }
    }
}
