import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// A palette PNG's samples are drawn in the space pdfTeX writes for it
/// (writepng.c: `/Indexed /DeviceRGB` with the PLTE bytes, whatever gAMA,
/// cHRM, sRGB or iCCP chunk the file has). ImageIO bases the palette on
/// sRGB, and on *linear* sRGB for a PNG with gAMA 1.0: drawn so, the
/// preview's palette colours were not the PDF's. Checked on the decoded
/// data alone (no drawing): the image's space, base and table.
/// Inputs: fixtures/preview-probes/pv-images (lane RENDER-AUDIT), which the
/// preview-parity sweep also renders against pdflatex's PDF.
final class PaletteImageSpaceTests: XCTestCase {
    static let images = PreviewParityTests.repoRoot.appendingPathComponent("fixtures/preview-probes/pv-images")
    /// make_inputs.py's palette: six colours.
    static let palette: [UInt8] = [255, 0, 0, 0, 160, 0, 0, 0, 255, 255, 200, 0, 40, 40, 40, 200, 120, 255]

    func samples(_ name: String) throws -> CGImage {
        let info: DL3JSON = .object(["id": .int(1), "key": .string(UUID().uuidString), "type": .string("png"),
                                     "file": .string(Self.images.appendingPathComponent(name).path)])
        guard case .raster(let img) = try DL3RenderImage.load(info).get().payload else { throw XCTSkip("not a raster") }
        return img
    }

    func testPaletteImagesAreIndexedOverDeviceRGB() throws {
        for name in ["pal8.png", "pal8-gamma.png"] {
            let img = try samples(name)
            let space = try XCTUnwrap(img.colorSpace, name)
            XCTAssertEqual(space.model, .indexed, name)
            XCTAssertEqual(space.baseColorSpace, CGColorSpaceCreateDeviceRGB(), "\(name): the palette's base is the PDF's /DeviceRGB")
            XCTAssertEqual(space.colorTable, Self.palette, "\(name): the PLTE bytes, unconverted")
        }
    }

    func testDirectColourImagesStayInTheirDeviceSpace() throws {
        XCTAssertEqual(try samples("rgb8-gamma.png").colorSpace, CGColorSpaceCreateDeviceRGB())
        XCTAssertEqual(try samples("gray8.png").colorSpace, CGColorSpaceCreateDeviceGray())
    }
}
