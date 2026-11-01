// preview_positions_check.swift — the preview-side check over the corpus.
//
//   swift tools/preview_positions_check.swift <face_metrics.json> [out prefix]
//
// Reads the engine's export (adapters::preview::face_metrics_json): the font
// file path, size, and for every run the glyph ids with pen-relative
// positions in points. For each run it
//   A. draws exactly those glyph ids at those positions with CoreText from
//      the SAME font file (what the Swift preview will do), and
//   B. lets CoreText shape and draw the run's Unicode text itself,
// then reports CoreText's typographic width vs the engine's width_pt and the
// number of differing pixels at 8x. This is the corpus-wide generalisation of
// render_check.swift; per-run deltas are quoted in docs/consumers.md.

import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = Array(CommandLine.arguments.dropFirst())
guard args.count >= 1 else {
    FileHandle.standardError.write("usage: preview_positions_check <face_metrics.json> [out prefix]\n".data(using: .utf8)!)
    exit(2)
}
let data = try! Data(contentsOf: URL(fileURLWithPath: args[0]))
let root = try! JSONSerialization.jsonObject(with: data) as! [String: Any]
let fontInfo = root["font"] as! [String: Any]
let fontPath = fontInfo["source_path"] as! String
let size = CGFloat(root["size_pt"] as! Double)
guard let provider = CGDataProvider(filename: fontPath), let cg = CGFont(provider) else {
    print("FAIL: cannot load \(fontPath)"); exit(1)
}
let font = CTFontCreateWithGraphicsFont(cg, size, nil, nil)
print("font: \(fontInfo["postscript_name"] as! String) (\(fontPath)) at \(size) pt")
print("| run | engine width pt | CoreText width pt | Δ pt | ink A | ink B | differing px (8x) |")
print("| --- | --- | --- | --- | --- | --- | --- |")

let scale: CGFloat = 8
let width = 700, height = 60
func canvas() -> CGContext {
    let ctx = CGContext(data: nil, width: width * Int(scale), height: height * Int(scale), bitsPerComponent: 8,
                        bytesPerRow: width * Int(scale), space: CGColorSpaceCreateDeviceGray(),
                        bitmapInfo: CGImageAlphaInfo.none.rawValue)!
    ctx.setFillColor(gray: 1, alpha: 1)
    ctx.fill(CGRect(x: 0, y: 0, width: width * Int(scale), height: height * Int(scale)))
    ctx.scaleBy(x: scale, y: scale)
    ctx.setFillColor(gray: 0, alpha: 1)
    ctx.setAllowsFontSmoothing(false)
    return ctx
}
func pixels(_ ctx: CGContext) -> [UInt8] {
    let n = ctx.width * ctx.height
    let p = ctx.data!.bindMemory(to: UInt8.self, capacity: n)
    return Array(UnsafeBufferPointer(start: p, count: n))
}

var worst = 0.0
var runIndex = 0
for run in root["runs"] as! [[String: Any]] {
    let text = run["text"] as! String
    let engineWidth = run["width_pt"] as! Double
    var glyphs: [CGGlyph] = []
    var points: [CGPoint] = []
    for cluster in run["clusters"] as! [[String: Any]] {
        for g in cluster["glyphs"] as! [[String: Any]] {
            glyphs.append(CGGlyph(g["gid"] as! Int))
            points.append(CGPoint(x: 10 + (g["x_pt"] as! Double), y: 20 + (g["y_pt"] as! Double)))
        }
    }
    let a = canvas()
    CTFontDrawGlyphs(font, glyphs, points, glyphs.count, a)
    let b = canvas()
    let attrs: [NSAttributedString.Key: Any] = [kCTFontAttributeName as NSAttributedString.Key: font]
    let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: attrs))
    b.textPosition = CGPoint(x: 10, y: 20)
    CTLineDraw(line, b)
    let ctWidth = CTLineGetTypographicBounds(line, nil, nil, nil)
    let pa = pixels(a), pb = pixels(b)
    var inkA = 0, inkB = 0, diff = 0
    for i in 0..<pa.count {
        let da = pa[i] < 128, db = pb[i] < 128
        if da { inkA += 1 }
        if db { inkB += 1 }
        if da != db { diff += 1 }
    }
    let delta = engineWidth - ctWidth
    worst = max(worst, abs(delta))
    let short = text.count > 40 ? String(text.prefix(40)) + "…" : text
    print("| \(short) | \(String(format: "%.4f", engineWidth)) | \(String(format: "%.4f", ctWidth)) | \(String(format: "%+.4f", delta)) | \(inkA) | \(inkB) | \(diff) |")
    if args.count >= 2 {
        for (suffix, ctx) in [("A", a), ("B", b)] {
            if let img = ctx.makeImage(),
               let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: "\(args[1])-\(runIndex)-\(suffix).png") as CFURL, UTType.png.identifier as CFString, 1, nil) {
                CGImageDestinationAddImage(dest, img, nil)
                CGImageDestinationFinalize(dest)
            }
        }
    }
    runIndex += 1
}
print("largest |Δ|: \(String(format: "%.4f", worst)) pt")
