import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
import XCTest
@testable import FlashTeXPadKit

/// The Captures list decodes each capture's PNG once, to a small image.
final class CaptureThumbnailsTests: XCTestCase {
    static func png(width: Int, height: Int) throws -> Data {
        let ctx = try XCTUnwrap(CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                                          space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue))
        ctx.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
        ctx.fill(CGRect(x: 0, y: 0, width: width, height: height))
        ctx.setStrokeColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
        ctx.setLineWidth(12)
        ctx.strokeEllipse(in: CGRect(x: 100, y: 100, width: width - 200, height: height - 200))
        let image = try XCTUnwrap(ctx.makeImage())
        let out = NSMutableData()
        let dest = try XCTUnwrap(CGImageDestinationCreateWithData(out as CFMutableData, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(dest, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(dest))
        return out as Data
    }

    func testAFullSizeCaptureIsDecodedOnceToASmallImage() throws {
        let png = try Self.png(width: 4096, height: 3072)
        let thumbnails = CaptureThumbnails()
        XCTAssertNil(thumbnails.cached("cap-1"))
        let first = try XCTUnwrap(thumbnails.thumbnail(id: "cap-1", png: png))
        XCTAssertLessThanOrEqual(max(first.width, first.height), CaptureThumbnails.maxPixels)
        XCTAssertEqual(first.width, CaptureThumbnails.maxPixels, "aspect kept, longest side at the bound")
        // Every later render (each 2 s poll re-renders the list) reuses it.
        for _ in 0..<20 { XCTAssertTrue(thumbnails.thumbnail(id: "cap-1", png: png) === first) }
        XCTAssertTrue(thumbnails.cached("cap-1") === first)
        XCTAssertEqual(thumbnails.decodes, 1)
    }

    func testEachCaptureHasItsOwnThumbnailAndGarbageIsNil() throws {
        let thumbnails = CaptureThumbnails()
        XCTAssertNotNil(thumbnails.thumbnail(id: "a", png: try Self.png(width: 400, height: 300)))
        XCTAssertNotNil(thumbnails.thumbnail(id: "b", png: try Self.png(width: 300, height: 400)))
        XCTAssertGreaterThan(try XCTUnwrap(thumbnails.cached("b")).height, try XCTUnwrap(thumbnails.cached("b")).width)
        XCTAssertNil(thumbnails.thumbnail(id: "c", png: Data("not a png".utf8)))
        XCTAssertEqual(thumbnails.decodes, 3)
    }
}
