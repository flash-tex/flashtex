// T0 evidence (2026-10-04): Core Graphics rendering of typst-pdf's page vs a
// Core Text drawing of the same page from a display-list-like description,
// drawn the way display-list-v3 spec §4.2/§11.2 says a client draws: in the
// page's stream space (bp, y up), glyph origins at (X, Y). Variants (env):
//   POS    = frame | nearest | cg     glyph origins: Typst's frame (through a
//            y flip, as px/pxdiff.swift), or PDF-derived (pdfpos2.py: double
//            nearest each decimal, or spec §4.2's viewer arithmetic)
//   SHAPES = frame | pdf | none       rules/lines from Typst's frame, from
//            the PDF's own numbers (pdfpos2.py "pdf_shapes"), or not drawn
//   SUBQ   = 0 | 1                    subpixel quantisation of glyphs
// Prints one JSON line; diff pixels are also split into those within 2 px of
// a shape's bounding box ("near_shape") and the rest.
import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
let d = try! JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: args[1]))) as! [String: Any]
let pdfPath = args[2]
let pageNo = Int(args[3])!
let S = CGFloat(Double(args[4])!)
let outPrefix = args.count > 5 ? args[5] : ""
let env = ProcessInfo.processInfo.environment
let POS = env["POS"] ?? "cg", SHAPES = env["SHAPES"] ?? "pdf"
let H = d["pdf_h"] as! Double
let w = d["w"] as! Double
let W = Int((CGFloat(w) * S).rounded(.up)), HH = Int((CGFloat(H) * S).rounded(.up))
let cs = CGColorSpace(name: CGColorSpace.sRGB)!

func makeCtx() -> CGContext {
    let c = CGContext(data: nil, width: W, height: HH, bitsPerComponent: 8, bytesPerRow: W * 4, space: cs,
                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    c.setFillColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 1))
    c.fill(CGRect(x: 0, y: 0, width: W, height: HH))
    c.setShouldSmoothFonts(false)
    c.setAllowsFontSmoothing(false)
    c.setShouldAntialias(true)
    if let v = env["SUBQ"] { c.setAllowsFontSubpixelQuantization(v == "1"); c.setShouldSubpixelQuantizeFonts(v == "1") }
    c.interpolationQuality = .high
    return c
}
func affine(_ m: [Double]) -> CGAffineTransform {
    CGAffineTransform(a: m[0], b: m[1], c: m[2], d: m[3], tx: m[4], ty: m[5])
}
func color(_ a: [Int]) -> CGColor {
    CGColor(srgbRed: CGFloat(a[0]) / 255, green: CGFloat(a[1]) / 255, blue: CGFloat(a[2]) / 255, alpha: CGFloat(a[3]) / 255)
}
func pdfColor(_ v: [Double]) -> CGColor {
    v.count == 3 ? CGColor(srgbRed: v[0], green: v[1], blue: v[2], alpha: 1)
        : CGColor(srgbRed: v[0], green: v[0], blue: v[0], alpha: 1)
}

let pdfCtx = makeCtx()
let doc = CGPDFDocument(URL(fileURLWithPath: pdfPath) as CFURL)!
pdfCtx.saveGState(); pdfCtx.scaleBy(x: S, y: S); pdfCtx.drawPDFPage(doc.page(at: pageNo)!); pdfCtx.restoreGState()

let dl = makeCtx()
dl.scaleBy(x: S, y: S)  // stream space, y up
var fonts: [CGFont] = []
for f in d["fonts"] as! [[String: Any]] {
    let data = try! Data(contentsOf: URL(fileURLWithPath: f["path"] as! String))
    let idx = f["index"] as! Int
    if idx == 0 { fonts.append(CGFont(CGDataProvider(data: data as CFData)!)!) } else {
        let descs = CTFontManagerCreateFontDescriptorsFromData(data as CFData) as! [CTFontDescriptor]
        fonts.append(CTFontCopyGraphicsFont(CTFontCreateWithFontDescriptor(descs[idx], 10, nil), nil))
    }
}
let flip = CGAffineTransform(a: 1, b: 0, c: 0, d: -1, tx: 0, ty: H)
var shapeRects: [CGRect] = []

if SHAPES == "frame" {
    for s in d["shapes"] as! [[String: Any]] {
        dl.saveGState(); dl.concatenate(flip); dl.concatenate(affine(s["m"] as! [Double]))
        let g = s["geom"] as! [String: Any]
        let p = CGMutablePath()
        if let l = g["line"] as? [Double] { p.move(to: .zero); p.addLine(to: CGPoint(x: l[0], y: l[1])) }
        else if let r = g["rect"] as? [Double] { p.addRect(CGRect(x: 0, y: 0, width: r[0], height: r[1])) }
        if let f = s["fill"] as? [Int] { dl.addPath(p); dl.setFillColor(color(f)); dl.fillPath() }
        if let st = s["stroke"] as? [Any] {
            dl.addPath(p); dl.setStrokeColor(color(st[0] as! [Int])); dl.setLineWidth(CGFloat(st[1] as! Double))
            dl.setLineCap(.butt); dl.setLineJoin(.miter); dl.strokePath()
        }
        dl.restoreGState()
    }
}
for s in d["pdf_shapes"] as! [[String: Any]] {
    let ctm = affine(s["ctm"] as! [Double])
    let p = CGMutablePath()
    for seg in s["segs"] as! [[Any]] {
        let op = seg[0] as! String
        let v = seg.dropFirst().map { CGFloat($0 as! Double) }
        switch op {
        case "m": p.move(to: CGPoint(x: v[0], y: v[1]))
        case "l": p.addLine(to: CGPoint(x: v[0], y: v[1]))
        case "c": p.addCurve(to: CGPoint(x: v[4], y: v[5]), control1: CGPoint(x: v[0], y: v[1]), control2: CGPoint(x: v[2], y: v[3]))
        case "re": p.addRect(CGRect(x: v[0], y: v[1], width: v[2], height: v[3]))
        default: p.closeSubpath()
        }
    }
    let lw = CGFloat(s["w"] as! Double)
    let bb = p.boundingBox.insetBy(dx: -lw, dy: -lw).applying(ctm)
    shapeRects.append(CGRect(x: bb.minX * S - 2, y: bb.minY * S - 2, width: bb.width * S + 4, height: bb.height * S + 4))
    if SHAPES != "pdf" { continue }
    dl.saveGState(); dl.concatenate(ctm)
    let paint = s["paint"] as! String
    if paint.hasPrefix("f") || paint.hasPrefix("F") || paint.hasPrefix("B") || paint.hasPrefix("b") {
        dl.addPath(p); dl.setFillColor(pdfColor(s["fill"] as! [Double])); dl.fillPath(using: paint.hasSuffix("*") ? .evenOdd : .winding)
    }
    if paint == "S" || paint == "s" || paint.hasPrefix("B") || paint.hasPrefix("b") {
        dl.addPath(p); dl.setStrokeColor(pdfColor(s["stroke"] as! [Double])); dl.setLineWidth(lw)
        dl.setLineCap(CGLineCap(rawValue: Int32(s["J"] as! Int))!); dl.setLineJoin(CGLineJoin(rawValue: Int32(s["j"] as! Int))!)
        dl.strokePath()
    }
    dl.restoreGState()
}

// COLOR = frame (Typst's rgba as sRGB, as px/pxdiff.swift), device (the
// PDF's components in DeviceGray/RGB/CMYK: a display list without E3) or icc
// (the PDF's components in its own ICCBased space: E3), times the PDF's ca.
let COLOR = env["COLOR"] ?? "frame"
func hexData(_ h: String) -> Data {
    var d = Data(capacity: h.count / 2); var i = h.startIndex
    while i < h.endIndex { let j = h.index(i, offsetBy: 2); d.append(UInt8(h[i..<j], radix: 16)!); i = j }
    return d
}
var iccSpaces: [String: CGColorSpace] = [:]
for (name, v) in (d["colorspaces"] as? [String: [String: Any]]) ?? [:] {
    if let prof = v["profile"] as? String { iccSpaces[name] = CGColorSpace(iccData: hexData(prof) as CFData) }
}
func runColor(_ i: Int) -> CGColor? {
    guard COLOR != "frame", let paints = d["run_paint"] as? [[String: Any]] else { return nil }
    let p = paints[i]
    let comps = (p["comps"] as! [Double]).map { CGFloat($0) }
    let ca = CGFloat(p["ca"] as! Double)
    let csName = p["cs"] as! String
    var space: CGColorSpace
    if COLOR == "icc", let s = iccSpaces[csName] { space = s }
    else if csName == "/DeviceCMYK" || csName == "DeviceCMYK" || comps.count == 4 { space = CGColorSpaceCreateDeviceCMYK() }
    else if comps.count == 3 { space = CGColorSpaceCreateDeviceRGB() }
    else { space = CGColorSpaceCreateDeviceGray() }
    return CGColor(colorSpace: space, components: comps + [ca])
}

var nglyphs = 0
let runs = d["runs"] as! [[String: Any]]
let pdfRuns = POS == "frame" ? nil : (d[POS == "cg" ? "runs_cg" : "runs_nearest"] as! [[[Double]]])
for (i, r) in runs.enumerated() {
    let size = CGFloat(r["size"] as! Double)
    let font = CTFontCreateWithGraphicsFont(fonts[r["font"] as! Int], size, nil, nil)
    dl.saveGState()
    dl.setFillColor(runColor(i) ?? color(r["rgba"] as! [Int]))
    dl.textMatrix = .identity
    var glyphs: [CGGlyph], pos: [CGPoint]
    if let pr = pdfRuns {
        glyphs = pr[i].map { CGGlyph($0[0]) }
        pos = pr[i].map { CGPoint(x: $0[1], y: $0[2]) }
    } else {
        let g = r["g"] as! [[Double]]
        glyphs = g.map { CGGlyph($0[0]) }
        pos = g.map { CGPoint(x: $0[1], y: -$0[2]) }
        dl.concatenate(flip); dl.concatenate(affine(r["m"] as! [Double])); dl.scaleBy(x: 1, y: -1)
    }
    CTFontDrawGlyphs(font, &glyphs, &pos, glyphs.count, dl)
    dl.restoreGState()
    nglyphs += glyphs.count
}

let a = UnsafeBufferPointer(start: pdfCtx.data!.assumingMemoryBound(to: UInt8.self), count: W * HH * 4)
let b = UnsafeBufferPointer(start: dl.data!.assumingMemoryBound(to: UInt8.self), count: W * HH * 4)
var diff = 0, near = 0, maxd = 0, maxNear = 0, maxFar = 0
for y in 0..<HH {
    for x in 0..<W {
        let i = (y * W + x) * 4
        var m = 0
        for k in 0..<3 { m = max(m, abs(Int(a[i + k]) - Int(b[i + k]))) }
        if m == 0 { continue }
        diff += 1; maxd = max(maxd, m)
        // bitmap row 0 is the top: device y-up coordinate is HH - 1 - y
        let p = CGPoint(x: CGFloat(x) + 0.5, y: CGFloat(HH - 1 - y) + 0.5)
        if shapeRects.contains(where: { $0.contains(p) }) { near += 1; maxNear = max(maxNear, m) } else { maxFar = max(maxFar, m) }
    }
}
print("{\"page\":\(pageNo),\"scale\":\(S),\"pos\":\"\(POS)\",\"shapes\":\"\(SHAPES)\",\"color\":\"\(COLOR)\",\"subq\":\"\(env["SUBQ"] ?? "default")\",\"glyphs\":\(nglyphs),\"diff_px\":\(diff),\"diff_near_shape_px\":\(near),\"diff_elsewhere_px\":\(diff - near),\"max_channel_diff\":\(maxd),\"max_near_shape\":\(maxNear),\"max_elsewhere\":\(maxFar)}")
if !outPrefix.isEmpty {
    let dc = makeCtx(); let dd = dc.data!.assumingMemoryBound(to: UInt8.self)
    for i in stride(from: 0, to: W * HH * 4, by: 4) {
        let da = Int(a[i]) - Int(b[i])
        dd[i] = UInt8(255 - max(0, -da)); dd[i + 1] = UInt8(255 - abs(da)); dd[i + 2] = UInt8(255 - max(0, da))
    }
    let url = URL(fileURLWithPath: outPrefix + "-diff.png") as CFURL
    let dst = CGImageDestinationCreateWithURL(url, UTType.png.identifier as CFString, 1, nil)!
    CGImageDestinationAddImage(dst, dc.makeImage()!, nil); CGImageDestinationFinalize(dst)
}
