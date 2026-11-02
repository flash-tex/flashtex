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
/// from Core Graphics' are counted inside the links' rectangles (grown by
/// their border width) and outside them; PDFKit's own differences outside
/// links (thin rules, some glyphs) are the baseline. Printed as
/// `link-borders:` lines and written to `FLASHTEX_V3_PARITY_OUT.links.json`.
/// Runs only where the sweep's input is (CI: preview-parity.yml); never
/// fails on what it measures.
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

    func testMeasureWhatPDFKitDrawsForLinkBorders() throws {
        let env = ProcessInfo.processInfo.environment
        let dir = URL(fileURLWithPath: env["FLASHTEX_DL3_FIXTURES"] ?? PreviewParityTests.repoRoot.appendingPathComponent("target/dl3-positions").path)
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: dir.path).sorted() else {
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
                let box = page.bounds(for: .mediaBox)
                for scale in [2.0] {
                    guard let a = DL3Renderer.rasterize(pdfPage: cgPage, scale: scale),
                          let b = PreviewParityTests.pdfKitRender(kit, i, scale) else { continue }
                    let pa = DL3Parity.rgba(a), pb = DL3Parity.rgba(b)
                    guard pa.count == pb.count else { continue }
                    let w = a.width, h = a.height
                    // Link rectangles in device pixels (y down), grown by the border.
                    let rects: [CGRect] = links.map { l in
                        let grow = max(1, l.border?.lineWidth ?? 1) + 1
                        let r = l.bounds.insetBy(dx: -grow, dy: -grow).offsetBy(dx: -box.minX, dy: -box.minY)
                        return CGRect(x: r.minX * scale, y: Double(h) - r.maxY * scale, width: r.width * scale, height: r.height * scale)
                    }
                    var inside = 0, outside = 0, sum = [0.0, 0.0, 0.0]
                    for y in 0 ..< h {
                        for x in 0 ..< w {
                            let k = (y * w + x) * 4
                            guard pa[k] != pb[k] || pa[k + 1] != pb[k + 1] || pa[k + 2] != pb[k + 2] || pa[k + 3] != pb[k + 3] else { continue }
                            let p = CGPoint(x: Double(x) + 0.5, y: Double(y) + 0.5)
                            if rects.contains(where: { $0.contains(p) }) {
                                inside += 1
                                for c in 0 ..< 3 { sum[c] += Double(pb[k + c]) }
                            } else {
                                outside += 1
                            }
                        }
                    }
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
        if let out = env["FLASHTEX_V3_PARITY_OUT"] {
            let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(rows).write(to: URL(fileURLWithPath: out + ".links.json"))
        }
    }
}
