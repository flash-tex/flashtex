import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// A page of six included PDFs (a figure grid), drawn as tiles. Each tile
/// walks the whole page, and each PDF is drawn from the drawing thread's own
/// document (`PDFThreadDocuments`). With a per-thread cache of 4 and no
/// culling, every tile opened every document again (6 per tile, 54-360 ms
/// each on beamer figures). Now a tile draws only the images its rectangle
/// meets, and the cache holds more than one page's PDFs: each document is
/// opened once per thread, and the tiles equal the whole page. No host is
/// needed: the PDFs are made here.
final class PDFImageTileCacheTests: XCTestCase {
    static let side = 100.0 // each included PDF: 100 × 100 bp
    static let pageSize = CGSize(width: 600, height: 400)

    /// A one-page PDF: an axial shading in colour `k` (shadings are what cost a draw), a frame.
    static func pdf(_ k: Int) -> Data {
        let data = NSMutableData()
        var box = CGRect(x: 0, y: 0, width: side, height: side)
        let ctx = CGContext(consumer: CGDataConsumer(data: data as CFMutableData)!, mediaBox: &box, nil)!
        ctx.beginPDFPage(nil)
        let colors = [CGColor(red: Double(k) / 6, green: 0.3, blue: 1 - Double(k) / 6, alpha: 1), CGColor(gray: 1, alpha: 1)] as CFArray
        let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors, locations: [0, 1])!
        ctx.drawLinearGradient(gradient, start: .zero, end: CGPoint(x: side, y: side), options: [])
        ctx.setStrokeColor(CGColor(gray: 0, alpha: 1))
        ctx.stroke(box.insetBy(dx: 2, dy: 2), width: 1.5)
        ctx.endPDFPage()
        ctx.closePDF()
        return data as Data
    }

    /// The page: six PDF images in a 3 × 2 grid, 180 bp each, 10 bp apart.
    static func prepared(dir: URL) throws -> DL3PreparedPage {
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, pageSize.width, pageSize.height]
        var images: [UInt32: DL3RenderImage] = [:]
        for k in 0 ..< 6 {
            let file = dir.appendingPathComponent("fig\(k).pdf")
            try pdf(k).write(to: file)
            let info: DL3JSON = .object(["id": .int(Int64(k + 1)), "key": .string("fig\(k)"), "type": .string("pdf"),
                                         "file": .string(file.path), "page": .int(1), "page_box": .string("media")])
            images[UInt32(k + 1)] = try DL3RenderImage.load(info).get()
            let s = 180 / side
            page.matrices.append(DL3Matrix(a: s, b: 0, c: 0, d: s, e: 15 + Double(k % 3) * 190, f: 15 + Double(k / 3) * 190))
            page.items.append(.image(id: UInt32(k + 1), matrix: UInt32(page.matrices.count)))
        }
        return DL3PreparedPage(page: page, fonts: [:], images: images)
    }

    func tempDir() throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("pdf-tiles-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: dir) }
        return dir
    }

    /// 256 px tiles over the page at `scale`.
    static func tiles(scale: Double) -> [DL3PixelRect] {
        let (w, h) = DL3Renderer.pixelSize(widthPt: pageSize.width, heightPt: pageSize.height, scale: scale)
        var r: [DL3PixelRect] = []
        for y in stride(from: 0, to: h, by: 256) {
            for x in stride(from: 0, to: w, by: 256) { r.append(DL3PixelRect(x: x, y: y, width: min(256, w - x), height: min(256, h - y))) }
        }
        return r
    }

    /// One tile as the tile queue draws it, on this thread.
    static func drawTile(_ p: DL3PreparedPage, scale: Double, _ r: DL3PixelRect) -> CGImage? {
        guard let ctx = CGContext(data: nil, width: r.width, height: r.height, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: DL3Renderer.tileSpace(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        DL3Renderer.drawTile(p, forms: [:], scale: scale, rect: r, in: ctx)
        return ctx.makeImage()
    }

    func testEachPDFIsOpenedOncePerThreadAndATileDrawsOnlyTheImagesItMeets() throws {
        let p = try Self.prepared(dir: tempDir())
        let scale = 4.25
        let cache = PDFThreadDocuments.current
        // A tile inside the first image only (bottom left: pixel rows count from the top).
        let (_, h) = DL3Renderer.pixelSize(widthPt: Self.pageSize.width, heightPt: Self.pageSize.height, scale: scale)
        let inFirst = DL3PixelRect(x: Int(20 * scale), y: h - Int(190 * scale), width: 256, height: 256)
        var before = cache.opened
        XCTAssertNotNil(Self.drawTile(p, scale: scale, inFirst))
        XCTAssertEqual(cache.opened - before, 1, "a tile inside one image opens only that image's document")
        // Every tile, three times over: each of the six documents is opened once.
        let rects = Self.tiles(scale: scale)
        before = cache.opened
        let t0 = Date()
        for _ in 0 ..< 3 { for r in rects { XCTAssertNotNil(Self.drawTile(p, scale: scale, r)) } }
        let ms = Date().timeIntervalSince(t0) * 1000 / Double(3 * rects.count)
        XCTAssertEqual(cache.opened - before, 5, "the other five documents, once each (\(rects.count) tiles x 3)")
        XCTAssertGreaterThanOrEqual(cache.kept.count, 6)
        print("PDFImageTileCacheTests: \(rects.count) tiles at \(scale) px/pt, \(String(format: "%.2f", ms)) ms per tile, \(cache.opened - before) documents opened")
    }

    /// Skipping the images a tile does not meet changes no pixel: every
    /// tile equals the whole page's raster there.
    func testTilesEqualTheWholePage() throws {
        let p = try Self.prepared(dir: tempDir())
        for scale in [2.0, 2.860804497912274, 4.25] {
            let whole = try XCTUnwrap(DL3Renderer.rasterize(p, scale: scale))
            for r in Self.tiles(scale: scale) {
                let tile = try XCTUnwrap(DL3Renderer.rasterizeTile(p, scale: scale, rect: r))
                let crop = try XCTUnwrap(whole.cropping(to: CGRect(x: r.x, y: r.y, width: r.width, height: r.height)))
                let d = DL3Parity.diff(DL3Parity.rgba(tile), DL3Parity.rgba(crop))
                XCTAssertEqual(d.pixels, 0, "tile \(r) at \(scale) px/pt: max delta \(d.maxDelta)")
            }
        }
    }

    /// The cache keeps at most `limit` documents and `byteLimit` bytes of
    /// PDF, the newest always; the limit is above a figure grid's PDFs.
    func testTheCacheIsBoundedByCountAndBytes() throws {
        XCTAssertGreaterThanOrEqual(PDFThreadDocuments.limit, 8)
        let cache = PDFThreadDocuments()
        let small = Self.pdf(0)
        for _ in 0 ..< PDFThreadDocuments.limit + 3 { XCTAssertNotNil(cache.document(for: DL3PDFBytes(small))) }
        XCTAssertEqual(cache.kept.count, PDFThreadDocuments.limit)
        // By bytes: room for two of these PDFs.
        let bounded = PDFThreadDocuments(maxCount: 16, maxBytes: 2 * small.count + small.count / 2)
        for _ in 0 ..< 5 { XCTAssertNotNil(bounded.document(for: DL3PDFBytes(small))) }
        XCTAssertEqual(bounded.kept.count, 2)
        XCTAssertLessThanOrEqual(bounded.kept.bytes, bounded.maxBytes)
        // One PDF over the bound alone stays: the newest.
        let tight = PDFThreadDocuments(maxCount: 16, maxBytes: small.count / 2)
        XCTAssertNotNil(tight.document(for: DL3PDFBytes(small)))
        XCTAssertNotNil(tight.document(for: DL3PDFBytes(small)))
        XCTAssertEqual(tight.kept.count, 1)
    }

    /// A page whose document is gone (the session let go of a fallback
    /// document a DONE replaced, a raster job still holding the page) is
    /// drawn from this thread's own document of its bytes, not from the
    /// shared page, and exactly as the page itself.
    func testAPageWhoseDocumentIsGoneIsDrawnFromItsBytes() throws {
        var page: CGPDFPage?
        autoreleasepool {
            let doc = DL3Renderer.openPDF(data: Self.pdf(3))
            page = doc?.page(at: 1)
            XCTAssertNotNil(page?.document)
        }
        let p = try XCTUnwrap(page)
        XCTAssertNil(p.document, "a CGPDFPage does not keep its document")
        XCTAssertNotNil(DL3PDFBytes.of(p), "the page carries the bytes")
        let opened = PDFThreadDocuments.current.opened
        let drawn = try XCTUnwrap(DL3Renderer.rasterize(pdfPage: p, scale: 2))
        XCTAssertEqual(PDFThreadDocuments.current.opened, opened + 1, "drawn from this thread's own document")
        let reference = try XCTUnwrap(DL3Renderer.rasterize(pdfPage: try XCTUnwrap(CGPDFDocument(CGDataProvider(data: Self.pdf(3) as CFData)!)?.page(at: 1)), scale: 2))
        let d = DL3Parity.diff(DL3Parity.rgba(drawn), DL3Parity.rgba(reference))
        XCTAssertEqual(d.pixels, 0, "max delta \(d.maxDelta)")
        XCTAssertFalse(DL3Parity.rgba(drawn).allSatisfy { $0 == 255 }, "not blank")
    }
}
