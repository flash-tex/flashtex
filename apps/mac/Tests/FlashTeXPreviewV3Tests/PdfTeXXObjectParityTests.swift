import CoreGraphics
import Foundation
import ImageIO
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// XObjects as pdfTeX writes them, drawn by `DL3Renderer` exactly as Core
/// Graphics draws pdflatex's PDF (#2214, preview parity gap U1; the cases of
/// fixtures/preview-probes pv-pdf-inclusion and pv-images, at the positions
/// pdflatex gave them there). No host is needed: the reference PDF is written
/// here by hand, with the objects pdfTeX writes, and the display-list page
/// draws the same file.
final class PdfTeXXObjectParityTests: XCTestCase {
    /// A one-page Letter PDF written by hand: `xobject` is `/X1`, drawn by
    /// `content`. `stream` objects are (dictionary entries, data).
    static func pdf(content: String, xobject: (String, Data)) -> Data {
        let objects: [(String, Data?)] = [
            ("<< /Type /Catalog /Pages 2 0 R >>", nil),
            ("<< /Type /Pages /Kids [3 0 R] /Count 1 >>", nil),
            ("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /X1 5 0 R >> >> >>", nil),
            ("", Data(content.utf8)),
            (xobject.0, xobject.1),
        ]
        var out = Data("%PDF-1.5\n".utf8), offsets: [Int] = []
        for (i, o) in objects.enumerated() {
            offsets.append(out.count)
            out += Data("\(i + 1) 0 obj\n".utf8)
            if let body = o.1 {
                out += Data("<< \(o.0) /Length \(body.count) >>\nstream\n".utf8) + body + Data("\nendstream\nendobj\n".utf8)
            } else {
                out += Data("\(o.0)\nendobj\n".utf8)
            }
        }
        let xref = out.count
        out += Data("xref\n0 \(objects.count + 1)\n0000000000 65535 f \n".utf8)
        for o in offsets { out += Data(String(format: "%010d 00000 n \n", o).utf8) }
        out += Data("trailer\n<< /Size \(objects.count + 1) /Root 1 0 R >>\nstartxref\n\(xref)\n%%EOF\n".utf8)
        return out
    }

    static func write(_ data: Data, _ ext: String) throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("xobject-parity-\(UUID().uuidString).\(ext)")
        try data.write(to: url)
        return url
    }

    /// The `cm` before each `/X1 Do`, as pdflatex wrote them for
    /// `\includegraphics` at its natural size, at width 1.5cm, and at
    /// angle 30 width 2cm (pv-pdf-inclusion p1), as `q … cm /X1 Do Q`
    /// operators and as the display list's matrices (the CTM at the Do).
    static let pagePlacements: [[CGAffineTransform]] = [
        [CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 148.712, ty: 567.771)],
        [CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 238.675, ty: 567.771), CGAffineTransform(a: 0.53151, b: 0, c: 0, d: 0.53151, tx: 0, ty: 0)],
        [CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 238.675, ty: 567.771), CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 52.482, ty: 0),
         CGAffineTransform(a: 0.51878, b: 0, c: 0, d: 0.51878, tx: 0, ty: 0), CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 39.999, ty: 0),
         CGAffineTransform(a: 0.86601, b: 0.49998, c: -0.49998, d: 0.86601, tx: 0, ty: 0)],
    ]

    /// pv-images p5: `\includegraphics[width=4cm]`, `1.1cm` and `0.35cm`
    /// of a 16 × 16 image.
    static let imagePlacements: [[CGAffineTransform]] = [
        [CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 148.712, ty: 534.348), CGAffineTransform(a: 7.08896, b: 0, c: 0, d: 7.08896, tx: 0, ty: 0),
         CGAffineTransform(a: 16, b: 0, c: 0, d: 16, tx: 0, ty: 0)],
        [CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 272.061, ty: 534.348), CGAffineTransform(a: 1.94899, b: 0, c: 0, d: 1.94899, tx: 0, ty: 0),
         CGAffineTransform(a: 16, b: 0, c: 0, d: 16, tx: 0, ty: 0)],
        [CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: 313.205, ty: 534.348), CGAffineTransform(a: 0.6201, b: 0, c: 0, d: 0.6201, tx: 0, ty: 0),
         CGAffineTransform(a: 16, b: 0, c: 0, d: 16, tx: 0, ty: 0)],
    ]

    static func ops(_ placements: [[CGAffineTransform]]) -> String {
        placements.map { cms in
            "q " + cms.map { m in [m.a, m.b, m.c, m.d, m.tx, m.ty].map { String(format: "%g", $0) }.joined(separator: " ") + " cm " }.joined() + "/X1 Do Q"
        }.joined(separator: "\n")
    }

    /// The CTM after `cms` (each `cm` premultiplies the CTM).
    static func ctm(_ cms: [CGAffineTransform]) -> CGAffineTransform {
        cms.reduce(CGAffineTransform.identity) { $1.concatenating($0) }
    }

    /// The display-list page: one IMAGE item per placement.
    static func prepared(_ image: DL3RenderImage, _ placements: [[CGAffineTransform]]) -> DL3PreparedPage {
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, 612, 792]
        page.matrices = placements.map { let m = ctm($0); return DL3Matrix(a: m.a, b: m.b, c: m.c, d: m.d, e: m.tx, f: m.ty) }
        page.items = placements.indices.map { .image(id: 1, matrix: UInt32($0 + 1)) }
        return DL3PreparedPage(page: page, fonts: [:], images: [1: image])
    }

    /// Pixels apart (and the largest channel difference) at 1x, 2x and 4x.
    static func diffs(_ mine: DL3PreparedPage, _ reference: Data) -> [(scale: Double, pixels: Int, maxDelta: Int)] {
        guard let page = CGPDFDocument(CGDataProvider(data: reference as CFData)!)?.page(at: 1) else { return [] }
        return [1.0, 2.0, 4.0].compactMap { s in
            guard let a = DL3Renderer.rasterize(mine, scale: s), let b = DL3Renderer.rasterize(pdfPage: page, scale: s) else { return nil }
            let d = DL3Parity.diff(DL3Parity.rgba(a), DL3Parity.rgba(b))
            return (s, d.pixels, d.maxDelta)
        }
    }

    /// pages.pdf of pv-pdf-inclusion: red and blue bars and a green stroke,
    /// the blue bar and the stroke crossing the top of a 60 bp crop box.
    static let includedContent = "0.9 0.2 0.1 rg 10 10 60 20 re f 0.1 0.3 0.8 rg 10 40 20 30 re f 0 0.6 0.2 RG 3 w 40 45 m 70 70 l S"

    /// An included PDF page with `media`, `crop` and `rotate`, drawn as
    /// pdfTeX's Form XObject with `bbox` and `matrix` and pdftex.def's
    /// `/Group <</S/Transparency/K false/I false>>`.
    static func groupDiffs(media: String, crop: String?, rotate: Int, bbox: [Double], matrix: String) throws -> [(scale: Double, pixels: Int, maxDelta: Int)] {
        var page = "<< /Type /Page /Parent 2 0 R /MediaBox [\(media)] /Contents 4 0 R /Resources << >>"
        if let crop { page += " /CropBox [\(crop)]" }
        if rotate != 0 { page += " /Rotate \(rotate)" }
        var included = "%PDF-1.4\n", offsets: [Int] = []
        let objs = ["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [3 0 R] /Count 1 >>", page + " >>",
                    "<< /Length \(includedContent.utf8.count) >>\nstream\n\(includedContent)\nendstream"]
        for (i, o) in objs.enumerated() { offsets.append(included.utf8.count); included += "\(i + 1) 0 obj\n\(o)\nendobj\n" }
        let xref = included.utf8.count
        included += "xref\n0 \(objs.count + 1)\n0000000000 65535 f \n" + offsets.map { String(format: "%010d 00000 n \n", $0) }.joined()
        included += "trailer\n<< /Size \(objs.count + 1) /Root 1 0 R >>\nstartxref\n\(xref)\n%%EOF\n"
        let url = try write(Data(included.utf8), "pdf")
        defer { try? FileManager.default.removeItem(at: url) }
        let box = bbox.map { String(format: "%g", $0) }.joined(separator: " ")
        let form = "/Type /XObject /Subtype /Form /FormType 1 /BBox [\(box)] /Matrix [\(matrix)] "
            + "/Group << /S /Transparency /K false /I false >> /Resources << >>"
        let reference = pdf(content: ops(pagePlacements), xobject: (form, Data(includedContent.utf8)))
        let info: DL3JSON = .object(["id": .int(1), "key": .string(url.lastPathComponent), "type": .string("pdf"), "file": .string(url.path),
                                     "page": .int(1), "page_box": .string("crop"), "rotate": .int(Int64(rotate)),
                                     "width": .double(bbox[2] - bbox[0]), "height": .double(bbox[3] - bbox[1]),
                                     "orig_x": .double(bbox[0]), "orig_y": .double(bbox[1]), "group": .bool(true)])
        let image = try DL3RenderImage.load(info).get()
        return diffs(prepared(image, pagePlacements), reference)
    }

    /// pv-pdf-inclusion p1: an included page drawn through pdftex.def's
    /// transparency group (drawn straight, edge pixels were 1 level off at
    /// 2x and 4x).
    static func groupedPageDiffs() throws -> [(scale: Double, pixels: Int, maxDelta: Int)] {
        try groupDiffs(media: "0 0 80 80", crop: nil, rotate: 0, bbox: [0, 0, 80, 80], matrix: "1 0 0 1 0 0")
    }

    /// pv-pdf-inclusion p5: a crop box inside the media box, /Rotate 90; the
    /// blue bar and the stroke are cut by the form's /BBox.
    static func groupedCroppedPageDiffs() throws -> [(scale: Double, pixels: Int, maxDelta: Int)] {
        try groupDiffs(media: "-20 -10 80 80", crop: "0 0 75 60", rotate: 90, bbox: [0, 0, 75, 60], matrix: "0 -1 1 0 0 75")
    }

    /// Without the layer: 0, 4 and 4 px apart at 1x, 2x and 4x; with the
    /// box clipped before the layer, the cropped page: 56, 107 and 204 px
    /// (run 43750699156, U1 experiment).
    func testAnIncludedPageWithATransparencyGroup() throws {
        for d in try Self.groupedPageDiffs() {
            XCTAssertEqual(d.pixels, 0, "page 1 at \(d.scale)x: \(d.pixels) px differ (max Δ \(d.maxDelta))")
        }
    }

    func testAnIncludedCroppedTurnedPageWithATransparencyGroup() throws {
        for d in try Self.groupedCroppedPageDiffs() {
            XCTAssertEqual(d.pixels, 0, "page 5 at \(d.scale)x: \(d.pixels) px differ (max Δ \(d.maxDelta))")
        }
    }

    /// The repository's fixtures/preview-probes/pv-images.
    static func probe(_ name: String) -> URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("fixtures/preview-probes/pv-images/\(name)")
    }

    /// pv-images p5: a 1-bit grey PNG, which pdfTeX copies as a
    /// `/BitsPerComponent 1` `/DeviceGray` image.
    static func oneBitGreyImage() throws -> DL3RenderImage {
        let url = probe("gray1.png")
        let info: DL3JSON = .object(["id": .int(1), "key": .string("gray1"), "type": .string("png"), "file": .string(url.path),
                                     "width": .int(16), "height": .int(16)])
        return try DL3RenderImage.load(info).get()
    }

    static func oneBitGreyDiffs(_ placements: [[CGAffineTransform]]) throws -> [(scale: Double, pixels: Int, maxDelta: Int)] {
        let url = probe("gray1.png")
        let src = try XCTUnwrap(CGImageSourceCreateWithURL(url as CFURL, nil))
        let decoded = try XCTUnwrap(CGImageSourceCreateImageAtIndex(src, 0, nil))
        let w = decoded.width, h = decoded.height, row = (w + 7) / 8
        // The samples as pdfTeX writes them: one bit each, rows padded to a byte.
        var grey = [UInt8](repeating: 0, count: w * h)
        let ctx = try XCTUnwrap(CGContext(data: &grey, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w,
                                          space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue))
        ctx.draw(decoded, in: CGRect(x: 0, y: 0, width: w, height: h))
        var bits = [UInt8](repeating: 0, count: row * h)
        for y in 0 ..< h { for x in 0 ..< w where grey[y * w + x] > 127 { bits[y * row + x / 8] |= UInt8(0x80 >> (x % 8)) } }
        let image = "/Type /XObject /Subtype /Image /Width \(w) /Height \(h) /ColorSpace /DeviceGray /BitsPerComponent 1"
        let reference = pdf(content: ops(placements), xobject: (image, Data(bits)))
        let mine = try oneBitGreyImage()
        if case .raster(let c) = mine.payload { XCTAssertEqual(c.bitsPerComponent, 1, "drawn at the bit depth pdfTeX copies") }
        return diffs(prepared(mine, placements), reference)
    }

    /// The 4cm and 1.1cm copies (7.1 and 1.95 bp per sample): drawn as the
    /// PDF draws them, edges antialiased (with hard edges: up to 1,703 px
    /// apart, Δ 252; run 43750699156).
    func testAOneBitGreyImage() throws {
        for d in try Self.oneBitGreyDiffs(Array(Self.imagePlacements.prefix(2))) {
            XCTAssertEqual(d.pixels, 0, "gray1.png at \(d.scale)x: \(d.pixels) px differ (max Δ \(d.maxDelta))")
        }
    }

    /// The 0.35cm copy (0.62 bp per sample) is drawn minified at 1x, where
    /// Core Graphics' PDF path differs (117 px): the page falls back to the
    /// PDF; the larger copies do not make it.
    func testAOneBitImageDrawnSmallFallsBack() throws {
        let image = try Self.oneBitGreyImage()
        let m = Self.imagePlacements.map { let t = Self.ctm($0); return DL3Matrix(a: t.a, b: t.b, c: t.c, d: t.d, e: t.tx, f: t.ty) }
        XCTAssertFalse(DL3Renderer.oneBitDrawnTooSmall(image, m[0]))
        XCTAssertFalse(DL3Renderer.oneBitDrawnTooSmall(image, m[1]))
        XCTAssertTrue(DL3Renderer.oneBitDrawnTooSmall(image, m[2]))
        var bindings = DL3Bindings()
        bindings.images[1] = .success(image)
        var page = DL3Page(kind: .page, index: 0)
        page.matrices = [m[1]]
        page.items = [.image(id: 1, matrix: 1)]
        XCTAssertFalse(bindings.prepare(page).needsPDFFallback)
        page.matrices = [m[2]]
        XCTAssertTrue(bindings.prepare(page).needsPDFFallback)
    }

    /// pv-images p11: a 16-bit RGBA PNG's SMask holds only the alpha's high
    /// byte (writepng.c), so the alpha the preview draws is hi·257.
    func testSixteenBitAlphaIsTheSMasksHighByte() throws {
        let url = Self.probe("rgba16.png")
        let src = try XCTUnwrap(CGImageSourceCreateWithURL(url as CFURL, nil))
        let decoded = try XCTUnwrap(CGImageSourceCreateImageAtIndex(src, 0, nil))
        let header = try XCTUnwrap(pngHeader(url))
        XCTAssertEqual(header.depth, 16)
        let out = pngSamplesAsPDFTeXWrites(decoded, header)
        XCTAssertEqual(out.bitsPerComponent, 16)
        let data = try XCTUnwrap(out.dataProvider?.data), base = try XCTUnwrap(CFDataGetBytePtr(data))
        let n = out.bitsPerPixel / 16, a = out.alphaInfo == .last || out.alphaInfo == .premultipliedLast ? n - 1 : 0
        var lowDiffered = false
        let original = try XCTUnwrap(decoded.dataProvider?.data), obase = try XCTUnwrap(CFDataGetBytePtr(original))
        for y in 0 ..< out.height {
            for x in 0 ..< out.width {
                let o = y * out.bytesPerRow + (x * n + a) * 2
                XCTAssertEqual(base[o], base[o + 1], "alpha at \(x),\(y) is not hi·257")
                if obase[o] != obase[o + 1] { lowDiffered = true }
            }
        }
        XCTAssertTrue(lowDiffered, "the probe's alpha has low bytes of its own (else this proves nothing)")
    }

    /// A slanted Type 1 font (pv-fonts p1, SlantFont .167): the program gets
    /// `font_matrix` as its /FontMatrix, as writet1.c writes it, and the
    /// .pfb's first segment length follows.
    func testTheFontMatrixIsWrittenIntoTheProgram() throws {
        let clear = Array("%!PS-AdobeFont-1.0: Test\n/FontMatrix [0.001 0 0 0.001 0 0] readonly def\ncurrentfile eexec\n".utf8)
        let n = clear.count
        let pfb = [0x80, 1, UInt8(n & 0xFF), UInt8(n >> 8 & 0xFF), 0, 0] + clear + [0x80, 2, 4, 0, 0, 0, 1, 2, 3, 4, 0x80, 3]
        let out = try XCTUnwrap(Type1Encoding.withFontMatrix(pfb, [0.001, 0, 0.000167, 0.001, 0, 0]))
        let m = Int(out[2]) | Int(out[3]) << 8 | Int(out[4]) << 16 | Int(out[5]) << 24
        let text = String(decoding: out[6 ..< 6 + m], as: UTF8.self)
        XCTAssertTrue(text.contains("/FontMatrix [0.001 0.0 0.000167 0.001 0.0 0.0] readonly def"), text)
        XCTAssertTrue(text.hasSuffix("currentfile eexec\n"), "the segment length is the new text's")
        XCTAssertEqual(Array(out[(6 + m)...]), [0x80, 2, 4, 0, 0, 0, 1, 2, 3, 4, 0x80, 3])
        XCTAssertNil(Type1Encoding.withFontMatrix(Array("no matrix here".utf8), [1, 0, 0, 1, 0, 0]))
    }
}
