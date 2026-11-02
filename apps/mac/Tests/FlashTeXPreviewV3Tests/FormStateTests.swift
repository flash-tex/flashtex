import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// `form-state` (spec §4.3, lane RENDER-AUDIT): a FORM flagged INHERITS
/// starts from the fill colour where it is drawn, as a PDF form does
/// (pdfTeX writes no colour at a form's start); a FORM without the flag
/// starts black. One offscreen raster of a 10 × 10 bp page, read back at a
/// pixel: no PDF and no screen involved.
final class FormStateTests: XCTestCase {
    /// A page that sets red and draws form 1, a filled 10 × 10 bp rule.
    func centre(formFlags: UInt32) throws -> [UInt8] {
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, 10, 10]
        page.items = [.fillColor([1, 0, 0]), .form(id: 1, matrix: 0)]
        var form = DL3Page(kind: .form, index: 1)
        form.flags = formFlags
        form.box = [0, 0, 10, 10]
        let sp = Int32(10 * 65781.76)
        form.items = [.rule(kind: .fill, x: 0, y: 0, w: sp, h: sp)]
        let forms = [UInt32(1): DL3PreparedPage(page: form, fonts: [:], images: [:])]
        let img = try XCTUnwrap(DL3Renderer.rasterize(DL3PreparedPage(page: page, fonts: [:], images: [:]), forms: forms, scale: 1))
        let px = DL3Parity.rgba(img)
        let i = (5 * img.width + 5) * 4
        return Array(px[i ..< i + 4])
    }

    func testAnInheritingFormDrawsInTheColourItIsDrawnIn() throws {
        XCTAssertEqual(try centre(formFlags: DL3Page.flagInherits), [255, 0, 0, 255])
    }

    func testAFormWithoutTheFlagStartsBlack() throws {
        XCTAssertEqual(try centre(formFlags: 0), [0, 0, 0, 255])
    }
}
