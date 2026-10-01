// Pixel-parity experiment: Core Graphics rendering of typst-pdf's PDF page vs
// Core Text drawing of the same page's Typst frame (glyph ids + positions,
// original OpenType font files). Throwaway; TYPST-DESIGN-A.
import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
let jsonPath = args[1]
let pdfPath = args[2]
let pageNo = Int(args[3])!  // 1-based
let S = CGFloat(Double(args[4])!)
let outPrefix = args.count > 5 ? args[5] : ""

let d = try! JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: jsonPath))) as! [String: Any]
let w = CGFloat(d["w"] as! Double), h = CGFloat(d["h"] as! Double)
let W = Int((w * S).rounded(.up)), H = Int((h * S).rounded(.up))
let cs = CGColorSpace(name: CGColorSpace.sRGB)!

func makeCtx() -> CGContext {
    let c = CGContext(data: nil, width: W, height: H, bitsPerComponent: 8, bytesPerRow: W * 4, space: cs,
                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    c.setFillColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 1))
    c.fill(CGRect(x: 0, y: 0, width: W, height: H))
    c.setShouldSmoothFonts(false)
    c.setAllowsFontSmoothing(false)
    c.setShouldAntialias(true)
    let env = ProcessInfo.processInfo.environment
    if let v = env["SUBPOS"] { c.setAllowsFontSubpixelPositioning(v == "1"); c.setShouldSubpixelPositionFonts(v == "1") }
    if let v = env["SUBQ"] { c.setAllowsFontSubpixelQuantization(v == "1"); c.setShouldSubpixelQuantizeFonts(v == "1") }
    c.interpolationQuality = .high
    return c
}

// 1. PDF page through Core Graphics
let pdfCtx = makeCtx()
let doc = CGPDFDocument(URL(fileURLWithPath: pdfPath) as CFURL)!
let page = doc.page(at: pageNo)!
pdfCtx.saveGState()
pdfCtx.scaleBy(x: S, y: S)
pdfCtx.drawPDFPage(page)
pdfCtx.restoreGState()

// 2. Display list through Core Text
let dlCtx = makeCtx()
var cgFonts: [CGFont] = []
for f in d["fonts"] as! [[String: Any]] {
    let data = try! Data(contentsOf: URL(fileURLWithPath: f["path"] as! String))
    let idx = f["index"] as! Int
    if idx == 0 {
        cgFonts.append(CGFont(CGDataProvider(data: data as CFData)!)!)
    } else {
        let descs = CTFontManagerCreateFontDescriptorsFromData(data as CFData) as! [CTFontDescriptor]
        let ct = CTFontCreateWithFontDescriptor(descs[idx], 10, nil)
        cgFonts.append(CTFontCopyGraphicsFont(ct, nil))
    }
}
let mbH = ProcessInfo.processInfo.environment["MBH"].flatMap { Double($0) } ?? Double(h)
dlCtx.translateBy(x: 0, y: CGFloat(mbH) * S)
dlCtx.scaleBy(x: S, y: -S)  // top-left, y-down, points

func color(_ a: [Int]) -> CGColor {
    CGColor(srgbRed: CGFloat(a[0]) / 255, green: CGFloat(a[1]) / 255, blue: CGFloat(a[2]) / 255,
            alpha: CGFloat(a[3]) / 255)
}
func affine(_ m: [Double]) -> CGAffineTransform {
    CGAffineTransform(a: m[0], b: m[1], c: m[2], d: m[3], tx: m[4], ty: m[5])
}

for s in d["shapes"] as! [[String: Any]] {
    dlCtx.saveGState()
    dlCtx.concatenate(affine(s["m"] as! [Double]))
    let geom = s["geom"] as! [String: Any]
    let path = CGMutablePath()
    if let l = geom["line"] as? [Double] {
        path.move(to: .zero); path.addLine(to: CGPoint(x: l[0], y: l[1]))
    } else if let r = geom["rect"] as? [Double] {
        path.addRect(CGRect(x: 0, y: 0, width: r[0], height: r[1]))
    } else if let c = geom["curve"] as? [[Any]] {
        for it in c {
            let op = it[0] as! String
            let v = it.dropFirst().map { CGFloat($0 as! Double) }
            switch op {
            case "m": path.move(to: CGPoint(x: v[0], y: v[1]))
            case "l": path.addLine(to: CGPoint(x: v[0], y: v[1]))
            case "c": path.addCurve(to: CGPoint(x: v[4], y: v[5]), control1: CGPoint(x: v[0], y: v[1]), control2: CGPoint(x: v[2], y: v[3]))
            default: path.closeSubpath()
            }
        }
    }
    if let f = s["fill"] as? [Int] {
        dlCtx.addPath(path); dlCtx.setFillColor(color(f))
        dlCtx.fillPath(using: (s["evenodd"] as! Bool) ? .evenOdd : .winding)
    }
    if let st = s["stroke"] as? [Any] {
        dlCtx.addPath(path); dlCtx.setStrokeColor(color(st[0] as! [Int]))
        dlCtx.setLineWidth(CGFloat(st[1] as! Double)); dlCtx.setLineCap(.butt); dlCtx.setLineJoin(.miter)
        dlCtx.strokePath()
    }
    dlCtx.restoreGState()
}

var glyphCount = 0
for r in d["runs"] as! [[String: Any]] {
    let m = r["m"] as! [Double]
    let size = CGFloat(r["size"] as! Double)
    let font = CTFontCreateWithGraphicsFont(cgFonts[r["font"] as! Int], size, nil, nil)
    let g = r["g"] as! [[Double]]
    var glyphs = g.map { CGGlyph($0[0]) }
    var pos = g.map { CGPoint(x: $0[1], y: -$0[2]) }  // y-up inside the flipped run frame
    dlCtx.saveGState()
    if ProcessInfo.processInfo.environment["QUANT"] == "f32" {
        // krilla writes `1 0 0 -1 x y cm` with f32 x, y (PDF space, y up)
        var mm = m
        mm[4] = Double(Float(m[4]))
        mm[5] = Double(h) - Double(Float(Double(h) - m[5]))
        dlCtx.concatenate(affine(mm))
    } else {
        dlCtx.concatenate(affine(m))
    }
    dlCtx.scaleBy(x: 1, y: -1)
    dlCtx.textMatrix = .identity
    dlCtx.setFillColor(color(r["rgba"] as! [Int]))
    if ProcessInfo.processInfo.environment["PATHS"] == "1" {
        for (k, gl) in glyphs.enumerated() {
            var t = CGAffineTransform(translationX: pos[k].x, y: pos[k].y)
            if let p = CTFontCreatePathForGlyph(font, gl, &t) {
                dlCtx.addPath(p)
                dlCtx.fillPath()
            }
        }
    } else {
        CTFontDrawGlyphs(font, &glyphs, &pos, glyphs.count, dlCtx)
    }
    dlCtx.restoreGState()
    glyphCount += glyphs.count
}

// 3. Compare
func bytes(_ c: CGContext) -> UnsafeBufferPointer<UInt8> {
    UnsafeBufferPointer(start: c.data!.assumingMemoryBound(to: UInt8.self), count: W * H * 4)
}
let a = bytes(pdfCtx), b = bytes(dlCtx)
var diffPx = 0, maxd = 0, over16 = 0, inkA = 0
for i in stride(from: 0, to: W * H * 4, by: 4) {
    var m = 0
    for k in 0..<3 { m = max(m, abs(Int(a[i + k]) - Int(b[i + k]))) }
    if a[i] < 250 { inkA += 1 }
    if m > 0 { diffPx += 1 }
    if m > 16 { over16 += 1 }
    maxd = max(maxd, m)
}
print("{\"page\":\(pageNo),\"scale\":\(S),\"W\":\(W),\"H\":\(H),\"glyphs\":\(glyphCount),\"ink_px\":\(inkA),\"diff_px\":\(diffPx),\"diff_gt16_px\":\(over16),\"max_channel_diff\":\(maxd)}")

if !outPrefix.isEmpty {
    let diffCtx = makeCtx()
    let dd = diffCtx.data!.assumingMemoryBound(to: UInt8.self)
    for i in stride(from: 0, to: W * H * 4, by: 4) {
        let da = Int(a[i]) - Int(b[i])  // >0: DL darker
        dd[i] = UInt8(255 - max(0, -da)); dd[i + 1] = UInt8(255 - abs(da)); dd[i + 2] = UInt8(255 - max(0, da))
    }
    for (c, name) in [(pdfCtx, "pdf"), (dlCtx, "dl"), (diffCtx, "diff")] {
        let url = URL(fileURLWithPath: outPrefix + "-\(name).png") as CFURL
        let dst = CGImageDestinationCreateWithURL(url, UTType.png.identifier as CFString, 1, nil)!
        CGImageDestinationAddImage(dst, c.makeImage()!, nil)
        CGImageDestinationFinalize(dst)
    }
}
