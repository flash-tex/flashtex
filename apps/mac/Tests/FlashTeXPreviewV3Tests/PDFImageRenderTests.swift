import CoreGraphics
import Foundation
import IOSurface
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// An included PDF page (`IMAGE` type `pdf`, protocol §5.2) drawn by
/// `DL3Renderer` exactly as Core Graphics draws the same page of the same
/// file through an independent path (`CGPDFPage.getDrawingTransform`, which
/// applies the page box and `/Rotate` itself). No host is needed: the page
/// file and the display-list page are made here, the page's `cm` as pdfTeX
/// writes it for a Form XObject (`[s 0 0 s X−s·x₁ Y−s·y₁]`: the form's space
/// is the included page's own coordinates).
///
/// Covered: a box away from the origin, a crop box smaller than the media
/// box, a scaled image, `/Rotate` 90/180/270, and the dark appearance (an
/// included page keeps its own pixels).
final class PDFImageRenderTests: XCTestCase {
    /// A one-page PDF written by hand (Core Graphics cannot write /Rotate).
    /// The content covers the whole media box in red, an asymmetric blue bar
    /// inside the crop box, and a green square that the crop box cuts.
    static func pdf(media: [Double], crop: [Double]?, rotate: Int) -> Data {
        let content = "1 0 0 rg -1000 -1000 3000 3000 re f 0 0 1 rg 60 70 20 40 re f 0 0.6 0 rg 150 180 50 50 re f"
        func arr(_ a: [Double]) -> String { a.map { String(format: "%g", $0) }.joined(separator: " ") }
        var page = "<< /Type /Page /Parent 2 0 R /MediaBox [\(arr(media))] /Contents 4 0 R /Resources << >>"
        if let crop { page += " /CropBox [\(arr(crop))]" }
        if rotate != 0 { page += " /Rotate \(rotate)" }
        let objs = ["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [3 0 R] /Count 1 >>", page + " >>",
                    "<< /Length \(content.utf8.count) >>\nstream\n\(content)\nendstream"]
        var out = "%PDF-1.4\n", offsets: [Int] = []
        for (i, o) in objs.enumerated() { offsets.append(out.utf8.count); out += "\(i + 1) 0 obj\n\(o)\nendobj\n" }
        let xref = out.utf8.count
        out += "xref\n0 \(objs.count + 1)\n0000000000 65535 f \n" + offsets.map { String(format: "%010d 00000 n \n", $0) }.joined()
        out += "trailer\n<< /Size \(objs.count + 1) /Root 1 0 R >>\nstartxref\n\(xref)\n%%EOF\n"
        return Data(out.utf8)
    }

    func write(_ data: Data) throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("pdf-image-render-\(UUID().uuidString).pdf")
        try data.write(to: url)
        addTeardownBlock { try? FileManager.default.removeItem(at: url) }
        return url
    }

    struct Case {
        var media: [Double], crop: [Double]?, rotate: Int, pageBox: String, s: Double
    }

    static let pageSize = 300.0, scale = 2.0
    static let at = CGPoint(x: 20, y: 30) // where the turned box's lower-left corner lands on the page

    /// The display-list page (one IMAGE item) and the reference drawing of the same file.
    func render(_ c: Case, appearance: DL3Appearance = .light) throws -> (page: CGImage, reference: CGImage, rect: CGRect) {
        let url = try write(Self.pdf(media: c.media, crop: c.crop, rotate: c.rotate))
        let cgPage = try XCTUnwrap(CGPDFDocument(url as CFURL)?.page(at: 1))
        let kind: CGPDFBox = c.pageBox == "media" ? .mediaBox : .cropBox
        let box = cgPage.getBoxRect(kind) // what pdfTeX's get_pagebox gives: the host's orig_x/orig_y/width/height
        let info: DL3JSON = .object(["id": .int(1), "key": .string(url.lastPathComponent), "type": .string("pdf"), "file": .string(url.path),
                                     "page": .int(1), "page_box": .string(c.pageBox), "rotate": .int(Int64(c.rotate)),
                                     "width": .double(box.width), "height": .double(box.height),
                                     "orig_x": .double(box.minX), "orig_y": .double(box.minY)])
        let image = try DL3RenderImage.load(info).get()
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, Self.pageSize, Self.pageSize]
        page.matrices = [DL3Matrix(a: c.s, b: 0, c: 0, d: c.s, e: Self.at.x - c.s * box.minX, f: Self.at.y - c.s * box.minY)]
        page.items = [.image(id: 1, matrix: 1)]
        let prepared = DL3PreparedPage(page: page, fonts: [:], images: [1: image])
        let mine = try XCTUnwrap(DL3Renderer.rasterizeToSurface(prepared, scale: Self.scale, appearance: appearance).flatMap { DL3Renderer.image(of: $0) })
        let turned = c.rotate % 180 == 0 ? box.size : CGSize(width: box.height, height: box.width)
        let rect = CGRect(x: Self.at.x, y: Self.at.y, width: turned.width * c.s, height: turned.height * c.s)
        let refSurface = DL3Renderer.surface(widthPt: Self.pageSize, heightPt: Self.pageSize, scale: Self.scale, background: appearance.background) { ctx in
            ctx.clip(to: rect)
            ctx.concatenate(cgPage.getDrawingTransform(kind, rect: rect, rotate: 0, preserveAspectRatio: true))
            ctx.drawPDFPage(cgPage)
        }
        let reference = try XCTUnwrap(refSurface.flatMap { DL3Renderer.image(of: $0) })
        return (mine, reference, rect)
    }

    func assertSame(_ c: Case, file: StaticString = #filePath, line: UInt = #line) throws {
        let r = try render(c)
        let d = DL3Parity.diff(DL3Parity.rgba(r.page), DL3Parity.rgba(r.reference))
        XCTAssertEqual(d.pixels, 0, "\(c): \(d.pixels) px differ (max delta \(d.maxDelta))", file: file, line: line)
        // Not trivially equal: the image is on the page (red where it is).
        XCTAssertGreaterThan(redPixels(r.page), Int(r.rect.width * r.rect.height * Self.scale * Self.scale * 0.5), file: file, line: line)
    }

    func redPixels(_ img: CGImage) -> Int {
        let px = DL3Parity.rgba(img)
        var n = 0
        for i in stride(from: 0, to: px.count, by: 4) where px[i] > 200 && px[i + 1] < 60 && px[i + 2] < 60 { n += 1 }
        return n
    }

    func testBoxAwayFromTheOrigin() throws {
        try assertSame(Case(media: [30, 40, 230, 240], crop: nil, rotate: 0, pageBox: "media", s: 1))
    }

    func testCropBoxSmallerThanTheMediaBox() throws {
        try assertSame(Case(media: [30, 40, 230, 240], crop: [50, 60, 170, 200], rotate: 0, pageBox: "crop", s: 1))
        try assertSame(Case(media: [30, 40, 230, 240], crop: [50, 60, 170, 200], rotate: 0, pageBox: "media", s: 1))
    }

    func testScaled() throws {
        try assertSame(Case(media: [30, 40, 230, 240], crop: [50, 60, 170, 200], rotate: 0, pageBox: "crop", s: 0.5))
    }

    func testRotatedPages() throws {
        for r in [90, 180, 270] {
            try assertSame(Case(media: [30, 40, 230, 240], crop: [50, 60, 170, 200], rotate: r, pageBox: "crop", s: 1))
            try assertSame(Case(media: [30, 40, 230, 240], crop: [50, 60, 170, 200], rotate: r, pageBox: "crop", s: 0.5))
        }
    }

    /// The form matrices are pdfTeX's (pdftoepdf.c), checked on one box.
    func testFormMatrixIsPdfTeXs() {
        let b = CGRect(x: 50, y: 60, width: 120, height: 140) // x1 50, y1 60, x2 170, y2 200
        XCTAssertEqual(DL3RenderImage.formMatrix(box: b, rotate: 0), .identity)
        XCTAssertEqual(DL3RenderImage.formMatrix(box: b, rotate: 45), .identity, "pdfTeX writes no matrix")
        XCTAssertEqual(DL3RenderImage.formMatrix(box: b, rotate: 90), CGAffineTransform(a: 0, b: -1, c: 1, d: 0, tx: 50 - 60, ty: 60 + 170))
        XCTAssertEqual(DL3RenderImage.formMatrix(box: b, rotate: 180), CGAffineTransform(a: -1, b: 0, c: 0, d: -1, tx: 50 + 170, ty: 60 + 200))
        XCTAssertEqual(DL3RenderImage.formMatrix(box: b, rotate: 270), CGAffineTransform(a: 0, b: 1, c: -1, d: 0, tx: 50 + 200, ty: 60 - 50))
        XCTAssertEqual(DL3RenderImage.formMatrix(box: b, rotate: -90), DL3RenderImage.formMatrix(box: b, rotate: 270))
        // The turned box keeps the lower-left corner.
        for r in [90, 180, 270] {
            let t = b.applying(DL3RenderImage.formMatrix(box: b, rotate: r))
            XCTAssertEqual(t.minX, b.minX, accuracy: 1e-9); XCTAssertEqual(t.minY, b.minY, accuracy: 1e-9)
        }
    }

    /// The shared cache keeps images least recently used first out, by count
    /// and by bytes (a CGPDFDocument stays mapped while held).
    func testImageCacheIsBounded() throws {
        func info(_ url: URL) -> DL3JSON {
            .object(["id": .int(1), "key": .string(url.lastPathComponent), "type": .string("pdf"), "file": .string(url.path), "page": .int(1),
                     "page_box": .string("crop"), "width": .double(200), "height": .double(200), "orig_x": .double(0), "orig_y": .double(0)])
        }
        let urls = try (0 ..< 3).map { _ in try write(Self.pdf(media: [0, 0, 200, 200], crop: nil, rotate: 0)) }
        let cache = DL3ResourceCache(imageLimit: 2)
        let a = try cache.image(info(urls[0])).get()
        let b = try cache.image(info(urls[1])).get()
        XCTAssertTrue(try cache.image(info(urls[0])).get() === a, "a hit")
        _ = try cache.image(info(urls[2])).get()
        XCTAssertEqual(cache.heldImages.count, 2)
        XCTAssertTrue(try cache.image(info(urls[0])).get() === a, "recently used: kept")
        XCTAssertFalse(try cache.image(info(urls[1])).get() === b, "least recently used: dropped, then read again")
        XCTAssertEqual(cache.heldImages.count, 2)

        let size = try XCTUnwrap(FileManager.default.attributesOfItem(atPath: urls[0].path)[.size] as? Int)
        let small = DL3ResourceCache(imageLimit: 100, imageByteLimit: size * 2)
        for u in urls { _ = small.image(info(u)) }
        XCTAssertEqual(small.heldImages.count, 2, "two files fit the byte bound")
        XCTAssertEqual(small.heldImages.bytes, size * 2)
        let tiny = DL3ResourceCache(imageLimit: 100, imageByteLimit: 1)
        for u in urls { _ = tiny.image(info(u)) }
        XCTAssertEqual(tiny.heldImages.count, 1, "the newest is kept even when it alone is over the bound")
    }

    /// Dark appearance: the included page keeps its own pixels (only the
    /// page's ground and its own items' colours change).
    func testDarkKeepsTheIncludedPagesPixels() throws {
        let c = Case(media: [30, 40, 230, 240], crop: [50, 60, 170, 200], rotate: 90, pageBox: "crop", s: 1)
        let light = try render(c), dark = try render(c, appearance: .dark)
        let lp = DL3Parity.rgba(light.page), dp = DL3Parity.rgba(dark.page)
        let w = light.page.width, h = light.page.height
        let inner = light.rect.insetBy(dx: 1, dy: 1)
        var inside = 0, same = 0, outsideDiffer = 0
        for y in 0 ..< h {
            for x in 0 ..< w {
                let p = CGPoint(x: (Double(x) + 0.5) / Self.scale, y: Double(h - y) / Self.scale - 0.5 / Self.scale)
                let o = (y * w + x) * 4
                let eq = lp[o] == dp[o] && lp[o + 1] == dp[o + 1] && lp[o + 2] == dp[o + 2]
                if inner.contains(p) { inside += 1; if eq { same += 1 } } else if !light.rect.contains(p), !eq { outsideDiffer += 1 }
            }
        }
        XCTAssertGreaterThan(inside, 10_000)
        XCTAssertEqual(same, inside, "the included page's pixels are the same in both appearances")
        XCTAssertGreaterThan(outsideDiffer, 10_000, "the ground around it is dark")
    }
}
