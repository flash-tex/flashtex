import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// Settings > "Smooth fonts in preview" at the renderer: off (the default
/// every existing caller gets) is byte-for-byte the parity configuration;
/// on really changes how glyphs are drawn, on every raster entry point.
final class FontSmoothingTests: XCTestCase {
    static let fixtures = PreviewParityTests.repoRoot.appendingPathComponent("apps/mac/Tests/FlashTeXDisplayListV3Tests/Fixtures")

    func document() throws -> DL3Document {
        try DL3Document(frames: Array(try Data(contentsOf: Self.fixtures.appendingPathComponent("beamer-overlays.dl3"))))
    }

    func testDefaultIsOffAndIdenticalToExplicitOff() throws {
        let doc = try document()
        let page = try XCTUnwrap(doc.orderedPages.first)
        let byDefault = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: 2))
        let off = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: 2, smoothFonts: false))
        XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(byDefault), DL3Parity.rgba(off)).pixels, 0)
        let surface = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: 2))
        XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(off), DL3Parity.rgba(try XCTUnwrap(DL3Renderer.image(of: surface)))).pixels, 0)
    }

    func testOnSmoothsGlyphsOnEveryRasterPath() throws {
        let doc = try document()
        let page = try XCTUnwrap(doc.orderedPages.first)
        let off = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: 2)))
        let on = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: 2, smoothFonts: true)))
        XCTAssertGreaterThan(DL3Parity.diff(off, on).pixels, 0, "smoothing on draws text differently")
        // The on-screen IOSurface path draws the same smoothed pixels.
        let surface = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: 2, smoothFonts: true))
        XCTAssertEqual(DL3Parity.diff(on, DL3Parity.rgba(try XCTUnwrap(DL3Renderer.image(of: surface)))).pixels, 0)
        // The PDF fallback path follows the flag too.
        let pdf = try XCTUnwrap(CGPDFDocument(Self.fixtures.appendingPathComponent("beamer-overlays.pdf") as CFURL)?.page(at: 1))
        let pdfOff = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(pdfPage: pdf, scale: 2)))
        let pdfOn = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(pdfPage: pdf, scale: 2, smoothFonts: true)))
        XCTAssertGreaterThan(DL3Parity.diff(pdfOff, pdfOn).pixels, 0)
        let pdfSurface = try XCTUnwrap(DL3Renderer.rasterizeToSurface(pdfPage: pdf, scale: 2, smoothFonts: true))
        XCTAssertEqual(DL3Parity.diff(pdfOn, DL3Parity.rgba(try XCTUnwrap(DL3Renderer.image(of: pdfSurface)))).pixels, 0)
    }
}
