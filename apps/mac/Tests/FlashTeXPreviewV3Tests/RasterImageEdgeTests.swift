import CoreGraphics
import Foundation
import ImageIO
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// A raster image (PNG/JPEG, `IMAGE` type `png`/`jpeg`) drawn by
/// `DL3Renderer` exactly as Core Graphics draws the same image XObject in a
/// PDF (`q a b c d e f cm /Im Do Q`, as pdfTeX writes it), at the fractional
/// device positions the pane's fit-width scales give. Lane BEAMER-V3: on the
/// beamer `graphics` deck every PNG/JPEG page differed from the PDF along the
/// image's edges (up to 2,852 px, max delta 252 at 2.86 px/pt), because the
/// renderer antialiased the edges and Core Graphics' PDF path does not for an
/// axis-aligned image. No host is needed: the image and the reference PDF are
/// made here.
final class RasterImageEdgeTests: XCTestCase {
    /// 40 × 30 DeviceRGB samples (pdfTeX embeds PNG/JPEG data as
    /// `/DeviceRGB`): a black one-pixel frame around a colour ramp.
    static let image: CGImage = {
        let w = 40, h = 30
        var px = [UInt8](repeating: 0, count: w * h * 4)
        for y in 0 ..< h {
            for x in 0 ..< w where x > 0 && y > 0 && x < w - 1 && y < h - 1 {
                let i = (y * w + x) * 4
                px[i] = UInt8(40 + x * 5); px[i + 1] = UInt8(60 + y * 6); px[i + 2] = 200; px[i + 3] = 255
            }
            for x in [0, w - 1] { px[(y * w + x) * 4 + 3] = 255 }
        }
        for x in 0 ..< w { px[x * 4 + 3] = 255; px[((h - 1) * w + x) * 4 + 3] = 255 }
        let provider = CGDataProvider(data: Data(px) as CFData)!
        return CGImage(width: w, height: h, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: w * 4,
                       space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                       provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)!
    }()

    static let pageSize = CGSize(width: 362.835, height: 272.126) // a 4:3 beamer frame

    /// A one-page PDF drawing the image with `m` as its `cm` (the unit square onto the page).
    static func pdf(_ m: CGAffineTransform) -> CGPDFPage {
        let data = NSMutableData()
        var box = CGRect(origin: .zero, size: pageSize)
        let ctx = CGContext(consumer: CGDataConsumer(data: data as CFMutableData)!, mediaBox: &box, nil)!
        ctx.beginPDFPage(nil)
        ctx.concatenate(m)
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        ctx.endPDFPage()
        ctx.closePDF()
        return CGPDFDocument(CGDataProvider(data: data as CFData)!)!.page(at: 1)!
    }

    /// The display-list page: one IMAGE item with matrix `m`.
    static func prepared(_ m: CGAffineTransform) -> DL3PreparedPage {
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, pageSize.width, pageSize.height]
        page.matrices = [DL3Matrix(a: m.a, b: m.b, c: m.c, d: m.d, e: m.tx, f: m.ty)]
        page.items = [.image(id: 1, matrix: 1)]
        return DL3PreparedPage(page: page, fonts: [:], images: [1: DL3RenderImage(key: "ramp", payload: .raster(deviceSamples(image)))])
    }

    /// The fit-width scales of the pane (2.86 px/pt: a 4:3 frame in a 729 pt
    /// pane at 2×), integers, and tiled scales.
    static let scales = [1.0, 2.0, 2.860804497912274, 3.25, 4.25]

    func assertSame(_ m: CGAffineTransform, _ what: String, file: StaticString = #filePath, line: UInt = #line) throws {
        let page = Self.pdf(m), mine = Self.prepared(m)
        for s in Self.scales {
            let a = try XCTUnwrap(DL3Renderer.rasterize(mine, scale: s), file: file, line: line)
            let b = try XCTUnwrap(DL3Renderer.rasterize(pdfPage: page, scale: s), file: file, line: line)
            let d = DL3Parity.diff(DL3Parity.rgba(a), DL3Parity.rgba(b))
            XCTAssertEqual(d.pixels, 0, "\(what) at \(s) px/pt: \(d.pixels) px differ (max delta \(d.maxDelta))", file: file, line: line)
        }
    }

    func testUprightImageAtFractionalPositions() throws {
        // Where the beamer `graphics` deck's PNG sits (bp), and a smaller one off the grid.
        try assertSame(CGAffineTransform(a: 151.5917, b: 0, c: 0, d: 113.3858, tx: 105.6213, ty: 66.917), "upright")
        try assertSame(CGAffineTransform(a: 37.3, b: 0, c: 0, d: 21.77, tx: 10.13, ty: 200.41), "small")
    }

    func testQuarterTurnsAndFlips() throws {
        try assertSame(CGAffineTransform(a: 0, b: 90.25, c: -120.5, d: 0, tx: 200.3, ty: 40.7), "90°")
        try assertSame(CGAffineTransform(a: -120.5, b: 0, c: 0, d: -90.25, tx: 300.6, ty: 200.2), "180°")
        try assertSame(CGAffineTransform(a: 0, b: -90.25, c: 120.5, d: 0, tx: 60.1, ty: 150.9), "270°")
    }

    /// A JPEG (`IMAGE` type `jpeg`, a DCTDecode stream in the PDF) drawn
    /// well below its resolution: Core Graphics decodes the PDF's stream at
    /// a reduced size, so the renderer must read the file the same way
    /// (lane UNICODE-PREVIEW-QA: mwe's example-image.jpg at 1 px/pt had a
    /// black frame in the preview and a grey one in the PDF, 2,536 px).
    func testJPEGDrawnSmallMatchesThePDFsDCTDecode() throws {
        let w = 400, h = 300
        var px = [UInt8](repeating: 200, count: w * h * 4)
        for y in 0 ..< h { for x in 0 ..< w where x < 2 || y < 2 || x >= w - 2 || y >= h - 2 || x == y {
            px[(y * w + x) * 4] = 0; px[(y * w + x) * 4 + 1] = 0; px[(y * w + x) * 4 + 2] = 0
        } }
        let src = CGImage(width: w, height: h, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: w * 4,
                          space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                          provider: CGDataProvider(data: Data(px) as CFData)!, decode: nil, shouldInterpolate: false, intent: .defaultIntent)!
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("dl3-jpeg-\(ProcessInfo.processInfo.processIdentifier).jpg")
        defer { try? FileManager.default.removeItem(at: url) }
        let dest = try XCTUnwrap(CGImageDestinationCreateWithURL(url as CFURL, "public.jpeg" as CFString, 1, nil))
        CGImageDestinationAddImage(dest, src, [kCGImageDestinationLossyCompressionQuality: 0.9] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(dest))
        // The PDF: the JPEG's own bytes as the image (Core Graphics keeps them as DCTDecode).
        let jpeg = try XCTUnwrap(CGImage(jpegDataProviderSource: CGDataProvider(url: url as CFURL)!, decode: nil,
                                         shouldInterpolate: false, intent: .defaultIntent))
        let m = CGAffineTransform(a: 113.81, b: 0, c: 0, d: 85.36, tx: 72.4, ty: 120.3)
        let data = NSMutableData()
        var box = CGRect(origin: .zero, size: Self.pageSize)
        let ctx = CGContext(consumer: CGDataConsumer(data: data as CFMutableData)!, mediaBox: &box, nil)!
        ctx.beginPDFPage(nil)
        ctx.concatenate(m)
        ctx.draw(jpeg, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        ctx.endPDFPage()
        ctx.closePDF()
        XCTAssertNotNil((data as Data).range(of: Data("DCTDecode".utf8)), "the reference PDF holds the JPEG as DCTDecode")
        let pdfPage = try XCTUnwrap(CGPDFDocument(CGDataProvider(data: data as CFData)!)?.page(at: 1))
        let info = try DL3JSON.parse(Array(#"{"id":1,"key":"j","type":"jpeg","file":"\#(url.path)","width":400,"height":300}"#.utf8))
        let image = try DL3RenderImage.load(info).get()
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, Self.pageSize.width, Self.pageSize.height]
        page.matrices = [DL3Matrix(a: m.a, b: m.b, c: m.c, d: m.d, e: m.tx, f: m.ty)]
        page.items = [.image(id: 1, matrix: 1)]
        let mine = DL3PreparedPage(page: page, fonts: [:], images: [1: image])
        for s in [1.0, 2.0, 3.25] {
            let a = try XCTUnwrap(DL3Renderer.rasterize(mine, scale: s))
            let b = try XCTUnwrap(DL3Renderer.rasterize(pdfPage: pdfPage, scale: s))
            let d = DL3Parity.diff(DL3Parity.rgba(a), DL3Parity.rgba(b))
            XCTAssertEqual(d.pixels, 0, "JPEG at \(s) px/pt: \(d.pixels) px differ (max delta \(d.maxDelta))")
        }
    }

    func testAxisAlignedTransforms() {
        XCTAssertTrue(DL3Renderer.axisAligned(CGAffineTransform(a: 2, b: 0, c: 0, d: -2, tx: 3, ty: 4)))
        XCTAssertTrue(DL3Renderer.axisAligned(CGAffineTransform(a: 0, b: 1, c: -1, d: 0, tx: 0, ty: 0)))
        XCTAssertFalse(DL3Renderer.axisAligned(CGAffineTransform(rotationAngle: 10 * .pi / 180)))
    }
}
