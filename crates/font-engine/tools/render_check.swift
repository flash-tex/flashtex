// render_check.swift — native rendering check.
//
//   swift tools/render_check.swift <font file> <size pt> <positions file> <text> [out prefix]
//
// <positions file> is the stdout of `cargo run --example pdf_roundtrip`
// (one `gid x y` line per glyph, PDF page points, x0 = 72, y0 = 700).
//
// A: draws exactly those glyph ids at those positions with CoreText
//    (CTFontDrawGlyphs) using the SAME font file (CGFont from the file, so no
//    platform font substitution can occur).
// B: lets CoreText shape and draw the Unicode text itself (CTLine) at the
//    same origin with the same font.
// Prints the ink of each, the number of differing pixels and the ratio
// relative to A's ink, and writes A/B/diff PNGs when an out prefix is given.
// A ratio near zero means the engine's glyph choice and positions reproduce
// CoreText's own rendering of the same text; the number is reported, not
// thresholded, because kerning-table and ligature-policy differences are
// documented per font in README.md.

import CoreText
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = Array(CommandLine.arguments.dropFirst())
guard args.count >= 4 else {
    FileHandle.standardError.write("usage: render_check <font> <size> <positions> <text> [out prefix]\n".data(using: .utf8)!)
    exit(2)
}
let fontPath = args[0]
let size = CGFloat(Double(args[1])!)
let text = args[3]
guard let provider = CGDataProvider(filename: fontPath), let cg = CGFont(provider) else {
    print("FAIL: cannot load \(fontPath)"); exit(1)
}
let font = CTFontCreateWithGraphicsFont(cg, size, nil, nil)

var glyphs: [CGGlyph] = []
var points: [CGPoint] = []
for line in try! String(contentsOfFile: args[2], encoding: .utf8).split(separator: "\n") {
    let f = line.split(separator: " ")
    guard f.count == 3, let g = UInt16(f[0]), let x = Double(f[1]), let y = Double(f[2]) else { continue }
    glyphs.append(CGGlyph(g))
    points.append(CGPoint(x: x, y: y))
}
guard !glyphs.isEmpty else { print("FAIL: no positions"); exit(1) }

let scale: CGFloat = 8
let width = 612, height = 120
let originY: CGFloat = 700 // positions file baseline
func canvas() -> CGContext {
    let ctx = CGContext(data: nil, width: width * Int(scale), height: height * Int(scale), bitsPerComponent: 8,
                        bytesPerRow: width * Int(scale), space: CGColorSpaceCreateDeviceGray(),
                        bitmapInfo: CGImageAlphaInfo.none.rawValue)!
    ctx.setFillColor(gray: 1, alpha: 1)
    ctx.fill(CGRect(x: 0, y: 0, width: width * Int(scale), height: height * Int(scale)))
    ctx.scaleBy(x: scale, y: scale)
    // Map page baseline y=700 to canvas y=40.
    ctx.translateBy(x: 0, y: 40 - originY)
    ctx.setFillColor(gray: 0, alpha: 1)
    ctx.setShouldAntialias(true)
    ctx.setAllowsFontSmoothing(false)
    return ctx
}

// A: engine glyphs and positions.
let a = canvas()
CTFontDrawGlyphs(font, glyphs, points, glyphs.count, a)

// B: CoreText's own shaping of the same text at the same origin.
let b = canvas()
let attrs: [NSAttributedString.Key: Any] = [kCTFontAttributeName as NSAttributedString.Key: font]
let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: attrs))
b.textPosition = CGPoint(x: 72, y: originY)
CTLineDraw(line, b)

func pixels(_ ctx: CGContext) -> [UInt8] {
    let n = ctx.width * ctx.height
    let p = ctx.data!.bindMemory(to: UInt8.self, capacity: n)
    return Array(UnsafeBufferPointer(start: p, count: n))
}
let pa = pixels(a), pb = pixels(b)
var inkA = 0, inkB = 0, diff = 0
for i in 0..<pa.count {
    let da = pa[i] < 128, db = pb[i] < 128
    if da { inkA += 1 }
    if db { inkB += 1 }
    if da != db { diff += 1 }
}
print("glyphs drawn: \(glyphs.count)")
print("ink A (engine positions): \(inkA) px at \(Int(scale))x")
print("ink B (CoreText line):     \(inkB) px")
print("differing pixels: \(diff)  ratio to ink A: \(String(format: "%.4f", inkA > 0 ? Double(diff) / Double(inkA) : 1))")
if args.count >= 5 {
    for (suffix, ctx) in [("A", a), ("B", b)] {
        if let img = ctx.makeImage(),
           let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: args[4] + "-\(suffix).png") as CFURL, UTType.png.identifier as CFString, 1, nil) {
            CGImageDestinationAddImage(dest, img, nil)
            CGImageDestinationFinalize(dest)
        }
    }
}
exit(inkA > 0 ? 0 : 1)
