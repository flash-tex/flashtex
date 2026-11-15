import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// A page the decoder accepts but whose numbers are out of any sane range
/// (a corrupt frame, a buggy writer) renders nothing or something clipped;
/// it never traps the app. Each case trapped before (Int32 overflow on a
/// rule's far edge, `Int(.infinity)` / `Int(1e300)` sizing the bitmap).
final class MalformedPageTests: XCTestCase {
    private func page(box: [Double], items: [DL3Item] = []) -> DL3PreparedPage {
        var p = DL3Page(kind: .page, index: 0)
        p.box = box
        p.items = items
        return DL3PreparedPage(page: p, fonts: [:], images: [:])
    }

    /// A rule at the edge of the Int32 range (no RULE_GEOMETRY entry): its
    /// far edge is computed in Double, as the tile path already does.
    func testRuleAtTheInt32EdgeDrawsWithoutTrapping() {
        for item: DL3Item in [.rule(kind: .fill, x: .max, y: 0, w: 1, h: 1),
                              .rule(kind: .fill, x: 0, y: .max, w: 1, h: 1),
                              .rule(kind: .strokeH, x: .max, y: .max, w: .max, h: .max),
                              .rule(kind: .fill, x: .min, y: .min, w: -1, h: -1)] {
            let p = page(box: [0, 0, 612, 792], items: [item])
            XCTAssertNotNil(DL3Renderer.rasterize(p, scale: 1), "\(item)")
            XCTAssertNotNil(DL3Renderer.rasterizeToSurface(p, scale: 1), "\(item)")
        }
    }

    /// A non-finite or absurd page box sizes no bitmap.
    func testNonFiniteOrHugePageBoxRastersNothing() {
        for box: [Double] in [[0, 0, .infinity, 792], [0, 0, 612, .nan], [0, 0, 1e300, 792], [-.infinity, 0, 612, 792]] {
            let p = page(box: box)
            XCTAssertNil(DL3Renderer.rasterize(p, scale: 2), "\(box)")
            XCTAssertNil(DL3Renderer.rasterizeToSurface(p, scale: 2), "\(box)")
        }
        XCTAssertNil(DL3Renderer.bitmapContext(widthPt: 612, heightPt: 792, scale: .nan))
        XCTAssertNotNil(DL3Renderer.rasterize(page(box: [0, 0, 612, 792]), scale: 1), "a sane page still rasters")
    }
}
