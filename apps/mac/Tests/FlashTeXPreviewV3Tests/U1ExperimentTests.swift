import CoreGraphics
import Foundation
import ImageIO
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// TEMPORARY (lane PREVIEW-PARITY, U1): a CI-only experiment that renders
/// the preview-probe pages that differ from pdflatex's PDF under renderer
/// variants and prints where the pixels differ ("U1EXP" lines in
/// swift-test.log). Its name starts with the sweep's test name so that
/// preview-parity.yml's --filter runs it. Removed before merge; it never fails.
extension PreviewParityTests {
    struct U1Diff { var pixels = 0, maxDelta = 0, minX = Int.max, minY = Int.max, maxX = -1, maxY = -1; var samples: [String] = [] }

    static func u1Diff(_ a: [UInt8], _ b: [UInt8], width: Int, keep: Int) -> U1Diff {
        var d = U1Diff()
        guard a.count == b.count else { d.pixels = -1; return d }
        var i = 0
        while i < a.count {
            if a[i] != b[i] || a[i + 1] != b[i + 1] || a[i + 2] != b[i + 2] || a[i + 3] != b[i + 3] {
                d.pixels += 1
                for c in 0 ..< 4 { d.maxDelta = max(d.maxDelta, abs(Int(a[i + c]) - Int(b[i + c]))) }
                let p = i / 4, x = p % width, y = p / width
                d.minX = min(d.minX, x); d.maxX = max(d.maxX, x); d.minY = min(d.minY, y); d.maxY = max(d.maxY, y)
                if d.samples.count < keep { d.samples.append("(\(x),\(y)) dl=\(a[i]),\(a[i + 1]),\(a[i + 2]),\(a[i + 3]) pdf=\(b[i]),\(b[i + 1]),\(b[i + 2]),\(b[i + 3])") }
            }
            i += 4
        }
        return d
    }

    static func u1Interpolating(_ p: DL3PreparedPage) -> DL3PreparedPage {
        var images: [UInt32: DL3RenderImage] = [:]
        for (id, img) in p.images {
            guard case .raster(let c) = img.payload, let space = c.colorSpace, let provider = c.dataProvider,
                  let copy = CGImage(width: c.width, height: c.height, bitsPerComponent: c.bitsPerComponent, bitsPerPixel: c.bitsPerPixel,
                                     bytesPerRow: c.bytesPerRow, space: space, bitmapInfo: c.bitmapInfo, provider: provider,
                                     decode: c.decode, shouldInterpolate: true, intent: c.renderingIntent) else { images[id] = img; continue }
            images[id] = DL3RenderImage(key: img.key, payload: .raster(copy))
        }
        return DL3PreparedPage(page: p.page, fonts: p.fonts, images: images, problems: p.problems)
    }

    @objc func testEveryParityFixtureIsPixelIdenticalToThePDF_U1Experiment() throws {
        guard let root = ProcessInfo.processInfo.environment["FLASHTEX_DL3_FIXTURES"] else { throw XCTSkip("no fixtures") }
        defer { DL3Renderer.u1Variant = [] }
        let dir = URL(fileURLWithPath: root)
        let cases: [(String, [Int])] = [("preview-probes__pv-pdf-inclusion", Array(0 ..< 9)),
                                        ("preview-probes__pv-images", [4, 10, 15]),
                                        ("preview-probes__pv-fonts", [0])]
        let variants: [Set<String>] = [[], ["pdf-nolayer"], ["pdf-layer-rect"], ["png-depth-off"], ["png-alpha16-off"],
                                       ["iq-none"], ["iq-low"], ["iq-medium"], ["iq-high"], ["png-depth-off", "iq-none"]]
        for (name, pages) in cases {
            let fx = dir.appendingPathComponent(name)
            guard let bytes = try? Data(contentsOf: fx.appendingPathComponent("display.dl3")),
                  let pdf = CGPDFDocument(fx.appendingPathComponent("oracle.pdf") as CFURL) else {
                print("U1EXP \(name): no input"); continue
            }
            for variant in variants {
                for interpolate in name.hasSuffix("pv-images") ? [false, true] : [false] {
                    DL3Renderer.u1Variant = variant
                    guard let doc = try? DL3Document(frames: Array(bytes), cache: DL3ResourceCache()) else { print("U1EXP \(name): no doc"); continue }
                    let tag = variant.sorted().joined(separator: "+") + (interpolate ? "+interp" : "")
                    for p in pages {
                        guard var prepared = doc.pages[UInt32(p)], let page = pdf.page(at: p + 1) else { continue }
                        if interpolate { prepared = Self.u1Interpolating(prepared) }
                        if variant.isEmpty && !interpolate {
                            for (id, img) in prepared.images.sorted(by: { $0.key < $1.key }) {
                                if case .raster(let c) = img.payload {
                                    print("U1EXP \(name) p\(p + 1) image \(id): bpc \(c.bitsPerComponent) bpp \(c.bitsPerPixel) row \(c.bytesPerRow) alpha \(c.alphaInfo.rawValue) info \(c.bitmapInfo.rawValue) model \(c.colorSpace?.model.rawValue ?? -1) \(c.width)x\(c.height)")
                                }
                            }
                            print("U1EXP \(name) p\(p + 1) incomplete=\(prepared.needsPDFFallback(forms: doc.forms)) unsupported=\(prepared.page.unsupported)")
                        }
                        for scale in [1.0, 2.0, 4.0] {
                            guard let a = DL3Renderer.rasterize(prepared, forms: doc.forms, scale: scale),
                                  let b = DL3Renderer.rasterize(pdfPage: page, scale: scale) else { continue }
                            let full = variant.isEmpty && !interpolate
                            let d = Self.u1Diff(DL3Parity.rgba(a), DL3Parity.rgba(b), width: a.width, keep: full ? 24 : 0)
                            var line = "U1EXP \(name) p\(p + 1) \(scale)x [\(tag)] px=\(d.pixels) maxD=\(d.maxDelta)"
                            if d.pixels > 0 { line += " bbox=\(d.minX),\(d.minY)-\(d.maxX),\(d.maxY)" }
                            print(line)
                            if full && d.pixels > 0 {
                                print("U1EXP   samples \(d.samples.joined(separator: " "))")
                                if name.hasSuffix("pv-fonts") {
                                    // the glyphs nearest each differing sample (bp, y up)
                                    let H = prepared.page.box[3]
                                    var gi = 0
                                    var glyphs: [(Double, Double, UInt16, UInt32)] = []
                                    for item in prepared.page.items {
                                        if case .glyph(let f, let code, let x, let y, _) = item {
                                            let o = gi < prepared.page.origins.count ? (prepared.page.origins[gi].x, prepared.page.origins[gi].y) : (Double(x) / DL3.spPerBp, H - Double(y) / DL3.spPerBp)
                                            glyphs.append((o.0, o.1, f, UInt32(code))); gi += 1
                                        }
                                    }
                                    for s in d.samples.prefix(12) {
                                        let xy = s.dropFirst().prefix { $0 != ")" }.split(separator: ",").compactMap { Double($0) }
                                        guard xy.count == 2 else { continue }
                                        let px = xy[0] / scale, py = H - xy[1] / scale
                                        let near = glyphs.filter { $0.0 <= px + 1 && px - $0.0 < 12 && abs($0.1 - py) < 12 }.suffix(2)
                                        print("U1EXP   at \(px),\(py): \(near.map { "F\($0.2)/\($0.3)@\($0.0),\($0.1)" })")
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // ImageIO's own reading of the probe PNGs.
        for f in ["gray1.png", "rgba16.png", "gray-alpha.png", "pal4.png", "rgb16.png"] {
            let url = dir.appendingPathComponent("preview-probes__pv-images/src/\(f)")
            guard let src = CGImageSourceCreateWithURL(url as CFURL, nil), let c = CGImageSourceCreateImageAtIndex(src, 0, nil) else { print("U1EXP \(f): no image"); continue }
            let props = CGImageSourceCopyPropertiesAtIndex(src, 0, nil) as? [CFString: Any]
            print("U1EXP imageio \(f): bpc \(c.bitsPerComponent) bpp \(c.bitsPerPixel) alpha \(c.alphaInfo.rawValue) info \(c.bitmapInfo.rawValue) model \(c.colorSpace?.model.rawValue ?? -1) depth \(props?[kCGImagePropertyDepth] ?? "?") header \(pngHeader(url).map { "\($0)" } ?? "nil")")
        }
    }
}
