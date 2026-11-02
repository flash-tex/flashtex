import CoreGraphics
import Foundation
import PDFKit
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// A measurement, not a gate (lane RENDER-AUDIT, Commander ruling 3): does
/// PDFKit (what Preview.app shows) draw the borders of link annotations
/// that pdfTeX writes (`\pdfstartlink` without `/Border`: the PDF default
/// `[0 0 1]`; hyperref's `colorlinks=false` boxes, `/Border [0 0 1]` with
/// `/C`)? Core Graphics' `drawPDFPage`, the parity reference, draws no
/// annotations, and neither does the preview. For every page of every
/// sweep fixture with a link, the pixels where PDFKit's rendering differs
/// from Core Graphics' are counted in a band around each link rectangle's
/// edges (the border stroke, `countBorderPixels`) and elsewhere, link
/// interiors included, so a linked glyph's anti-aliasing is not counted as a
/// border; PDFKit's own differences elsewhere (thin rules, some glyphs) are
/// the baseline. Turned pages are skipped. Printed as `link-borders:` lines
/// and written to `FLASHTEX_V3_PARITY_OUT.links.json`. Runs only where the
/// sweep's input is (CI: preview-parity.yml; skipped, and says so, without
/// it); it never fails on what it measures, only when it measured no link.
final class LinkBorderMeasureTests: XCTestCase {
    struct Row: Codable {
        var fixture: String, page: Int, scale: Double
        var links: Int
        /// Each link's /Border width and /C as PDFKit reads them.
        var borders: [Double], colors: [[Double]]
        var insideLinks: Int, outsideLinks: Int
        /// Mean RGB of PDFKit's pixels that differ inside the links.
        var insideMean: [Double]
    }

    /// Counts the pixels where two RGBA bitmaps differ: `inside` in a thin
    /// band along a link rectangle's edges (`band` px each side of the edge,
    /// where a border stroke is drawn; the interior holds the linked text's
    /// glyphs and is excluded), `outside` everywhere else. `sum` adds up
    /// `b`'s RGB over the `inside` pixels.
    static func countBorderPixels(_ a: [UInt8], _ b: [UInt8], width w: Int, height h: Int,
                                  rects: [(rect: CGRect, band: Double)]) -> (inside: Int, outside: Int, sum: [Double]) {
        var inside = 0, outside = 0, sum = [0.0, 0.0, 0.0]
        for y in 0 ..< h {
            for x in 0 ..< w {
                let k = (y * w + x) * 4
                guard a[k] != b[k] || a[k + 1] != b[k + 1] || a[k + 2] != b[k + 2] || a[k + 3] != b[k + 3] else { continue }
                let p = CGPoint(x: Double(x) + 0.5, y: Double(y) + 0.5)
                if rects.contains(where: { $0.rect.insetBy(dx: -$0.band, dy: -$0.band).contains(p) && !$0.rect.insetBy(dx: $0.band, dy: $0.band).contains(p) }) {
                    inside += 1
                    for c in 0 ..< 3 { sum[c] += Double(b[k + c]) }
                } else {
                    outside += 1
                }
            }
        }
        return (inside, outside, sum)
    }

    func testMeasureWhatPDFKitDrawsForLinkBorders() throws {
        let env = ProcessInfo.processInfo.environment
        let dir = URL(fileURLWithPath: env["FLASHTEX_DL3_FIXTURES"] ?? PreviewParityTests.repoRoot.appendingPathComponent("target/dl3-positions").path)
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: dir.path).sorted() else {
            print("link-borders: SKIPPED, no \(dir.path)")
            throw XCTSkip("no \(dir.path): run tools/displaylist/check_positions.py first")
        }
        var rows: [Row] = []
        for name in names {
            guard let pdf = PreviewParityTests.referencePDF(in: dir.appendingPathComponent(name)),
                  let kit = PDFDocument(url: pdf), let cg = CGPDFDocument(pdf as CFURL) else { continue }
            for i in 0 ..< kit.pageCount {
                guard let page = kit.page(at: i), let cgPage = cg.page(at: i + 1) else { continue }
                let links = page.annotations.filter { $0.type == "Link" || $0.action != nil || $0.url != nil }
                guard !links.isEmpty else { continue }
                // DL3Renderer.rasterize turns the page by /Rotate, pdfKitRender does not: not comparable.
                guard page.rotation == 0 else { print("link-borders: \(name) p\(i + 1) skipped, /Rotate \(page.rotation)"); continue }
                let box = page.bounds(for: .mediaBox)
                for scale in [2.0] {
                    guard let a = DL3Renderer.rasterize(pdfPage: cgPage, scale: scale),
                          let b = PreviewParityTests.pdfKitRender(kit, i, scale) else { continue }
                    let pa = DL3Parity.rgba(a), pb = DL3Parity.rgba(b)
                    guard pa.count == pb.count else { continue }
                    let w = a.width, h = a.height
                    // Link rectangles in device pixels (y down), with the band (px) around each edge
                    // that a border stroke of the link's width can reach.
                    let rects: [(rect: CGRect, band: Double)] = links.map { l in
                        let r = l.bounds.offsetBy(dx: -box.minX, dy: -box.minY)
                        let band = (max(1, l.border?.lineWidth ?? 1) + 1) * scale
                        return (CGRect(x: r.minX * scale, y: Double(h) - r.maxY * scale, width: r.width * scale, height: r.height * scale), band)
                    }
                    let (inside, outside, sum) = Self.countBorderPixels(pa, pb, width: w, height: h, rects: rects)
                    let row = Row(fixture: name, page: i + 1, scale: scale, links: links.count,
                                  borders: links.map { Double($0.border?.lineWidth ?? -1) },
                                  colors: links.map { l in
                                      guard let c = l.color.usingColorSpace(.sRGB) else { return [] }
                                      return [Double(c.redComponent), Double(c.greenComponent), Double(c.blueComponent)]
                                  },
                                  insideLinks: inside, outsideLinks: outside,
                                  insideMean: inside > 0 ? sum.map { $0 / Double(inside) } : [])
                    rows.append(row)
                    print("link-borders: \(name) p\(i + 1) at \(scale)x: \(links.count) links (border widths \(row.borders)), PDFKit differs from Core Graphics in \(inside) px inside the links' rectangles and \(outside) px outside; mean differing colour inside \(row.insideMean.map { Int($0.rounded()) })")
                }
            }
        }
        print("link-borders: \(rows.count) page renders with links; \(rows.filter { $0.insideLinks > 0 }.count) where PDFKit draws something in a link rectangle that Core Graphics does not")
        XCTAssertFalse(rows.isEmpty, "no page with a link was measured in \(dir.path)")
        if let out = env["FLASHTEX_V3_PARITY_OUT"] {
            let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(rows).write(to: URL(fileURLWithPath: out + ".links.json"))
        }
    }

    // MARK: the band helper on synthetic bitmaps

    private func bitmaps(_ ink: (Int, Int) -> Bool) -> ([UInt8], [UInt8]) {
        let a = [UInt8](repeating: 255, count: 40 * 40 * 4)
        var b = a
        for y in 0 ..< 40 { for x in 0 ..< 40 where ink(x, y) { for c in 0 ..< 3 { b[(y * 40 + x) * 4 + c] = 0 } } }
        return (a, b)
    }

    private let linkRect: [(rect: CGRect, band: Double)] = [(CGRect(x: 8, y: 8, width: 24, height: 24), 2)]

    func testLinkBorderBandIgnoresInteriorInk() {
        let (a, b) = bitmaps { x, y in (14 ..< 26).contains(x) && (14 ..< 26).contains(y) }
        let r = Self.countBorderPixels(a, b, width: 40, height: 40, rects: linkRect)
        XCTAssertEqual(r.inside, 0)
        XCTAssertEqual(r.outside, 144)
    }

    func testLinkBorderBandCountsOnePixelFrame() {
        let (a, b) = bitmaps { x, y in
            (8 ... 31).contains(x) && (8 ... 31).contains(y) && (x == 8 || x == 31 || y == 8 || y == 31)
        }
        let r = Self.countBorderPixels(a, b, width: 40, height: 40, rects: linkRect)
        XCTAssertEqual(r.inside, 92)
        XCTAssertEqual(r.outside, 0)
    }
}
