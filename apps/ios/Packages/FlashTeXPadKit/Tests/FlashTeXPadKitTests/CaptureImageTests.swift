import Foundation
import ImageIO
import NearbyClient
import UniformTypeIdentifiers
import XCTest
@testable import FlashTeXPadKit

/// A photo picked from the library or shot with the camera becomes a PNG the
/// Mac accepts (≤ 8 MiB, longest side ≤ 4096 px) and stays upright.
final class CaptureImageTests: XCTestCase {
    /// A 12 MP camera frame (4032 × 3024, an iPad's rear camera) of paper
    /// with sensor noise, stored landscape with EXIF orientation 6 (the
    /// camera held upright), as JPEG — the bytes the picker hands over.
    static func cameraJPEG(width: Int = 4032, height: Int = 3024, orientation: Int = 6) throws -> Data {
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        var seed: UInt32 = 0x9E37_79B9
        for y in 0..<height {
            for x in 0..<width {
                seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5
                let noise = Int(seed & 0x1F) - 16 // ±16 levels: sensor noise on paper
                let base = 200 + (x * 40 / width) - (y * 30 / height)
                let i = (y * width + x) * 4
                let v = UInt8(clamping: base + noise)
                pixels[i] = v; pixels[i + 1] = UInt8(clamping: Int(v) - 4); pixels[i + 2] = UInt8(clamping: Int(v) - 10); pixels[i + 3] = 255
            }
        }
        let space = CGColorSpaceCreateDeviceRGB()
        let image = try XCTUnwrap(pixels.withUnsafeMutableBytes { buf -> CGImage? in
            CGContext(data: buf.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4, space: space,
                      bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)?.makeImage()
        })
        let out = NSMutableData()
        let dest = try XCTUnwrap(CGImageDestinationCreateWithData(out as CFMutableData, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(dest, image, [kCGImagePropertyOrientation: orientation, kCGImageDestinationLossyCompressionQuality: 0.92] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(dest))
        return out as Data
    }

    /// What the app sent before: the decoded image PNG-encoded at full size
    /// (`UIImage.pngData()`), orientation dropped.
    static func fullSizePNG(_ data: Data) throws -> Data {
        let source = try XCTUnwrap(CGImageSourceCreateWithData(data as CFData, nil))
        let image = try XCTUnwrap(CGImageSourceCreateImageAtIndex(source, 0, nil))
        return try XCTUnwrap(CaptureImage.encodePNG(image))
    }

    func testCameraPhotoBecomesASendableUprightPNG() throws {
        let jpeg = try Self.cameraJPEG()
        let before = try Self.fullSizePNG(jpeg)
        XCTAssertGreaterThan(before.count, NearbyWire.maxImageBytes, "the full-size PNG is over the Mac's limit")
        XCTAssertNotNil(CaptureQueue.validate(png: before, instructions: "x"), "so the old path refused every camera photo")

        let prepared = try CaptureImage.sendablePNG(from: jpeg).get()
        XCTAssertNil(CaptureQueue.validate(png: prepared.png, instructions: "x"), "\(prepared.png.count) bytes")
        XCTAssertLessThanOrEqual(prepared.png.count, NearbyWire.maxImageBytes)
        XCTAssertLessThanOrEqual(max(prepared.width, prepared.height), CaptureImage.maxPixels)
        XCTAssertGreaterThan(prepared.height, prepared.width, "EXIF orientation 6 applied: the portrait shot stays portrait")
        let out = try XCTUnwrap(CGImageSourceCreateWithData(prepared.png as CFData, nil))
        XCTAssertEqual(CGImageSourceGetType(out) as String?, UTType.png.identifier)
        print("CAPTURE-IMAGE full-size PNG \(before.count) B -> sent \(prepared.png.count) B at \(prepared.width)x\(prepared.height)")
    }

    func testSmallUprightPNGIsSentUnchanged() throws {
        let jpeg = try Self.cameraJPEG(width: 64, height: 48, orientation: 1)
        let png = try Self.fullSizePNG(jpeg)
        let prepared = try CaptureImage.sendablePNG(from: png).get()
        XCTAssertEqual(prepared.png, png)
        XCTAssertEqual(prepared.width, 64)
        XCTAssertEqual(prepared.height, 48)
    }

    func testOversizedPNGIsScaledToThePixelBound() throws {
        let png = try Self.fullSizePNG(try Self.cameraJPEG(width: 5000, height: 1000, orientation: 1))
        let prepared = try CaptureImage.sendablePNG(from: png, maxBytes: .max).get()
        XCTAssertEqual(prepared.width, CaptureImage.maxPixels)
        XCTAssertLessThan(prepared.height, 1000)
    }

    func testNotAnImageIsRefused() {
        XCTAssertEqual(CaptureImage.sendablePNG(from: Data("not an image".utf8)), .failure(.unreadable))
    }
}
