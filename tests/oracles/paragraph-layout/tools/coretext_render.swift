// CoreText renderer for a runtime-v1 `compile_result` envelope: draws every text
// item at exactly the compiler-supplied origin (x_pt, baseline_y_pt; PDF points,
// origin top-left) into a PDF, the way the Mac preview/export path is meant to
// place glyphs (rendering-v2: "do not use a platform's measured first baseline
// to reposition the supplied origin"). Links nothing from the app; Apple
// frameworks only. Fonts: `Times-Roman` by PostScript name (macOS Times), so
// glyph *advances* are the platform font's, while every word origin is the
// compiler's.
//
// Usage: swiftc -O coretext_render.swift -o coretext_render
//        coretext_render compile_result.json out.pdf
import Foundation
import CoreGraphics
import CoreText

guard CommandLine.arguments.count == 3 else {
    FileHandle.standardError.write("usage: coretext_render <compile_result.json> <out.pdf>\n".data(using: .utf8)!)
    exit(2)
}
let data = try! Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
let root = try! JSONSerialization.jsonObject(with: data) as! [String: Any]
let payload = root["payload"] as! [String: Any]
let pages = payload["pages"] as! [[String: Any]]

let url = URL(fileURLWithPath: CommandLine.arguments[2]) as CFURL
var mediaBox = CGRect(x: 0, y: 0, width: 612, height: 792)
guard let ctx = CGContext(url, mediaBox: &mediaBox, nil) else { exit(3) }
var fontCache: [Double: CTFont] = [:]
for page in pages {
    let w = page["width_pt"] as! Double
    let h = page["height_pt"] as! Double
    var box = CGRect(x: 0, y: 0, width: w, height: h)
    let info = [kCGPDFContextMediaBox as String: NSData(bytes: &box, length: MemoryLayout<CGRect>.size)] as CFDictionary
    ctx.beginPDFPage(info)
    ctx.textMatrix = .identity
    for item in page["items"] as! [[String: Any]] {
        guard item["kind"] as? String == "text" else { continue }
        let text = item["text"] as! String
        let x = item["x_pt"] as! Double
        let baseline = item["baseline_y_pt"] as! Double
        let size = item["font_size_pt"] as! Double
        let font = fontCache[size] ?? CTFontCreateWithName("Times-Roman" as CFString, CGFloat(size), nil)
        fontCache[size] = font
        // No platform kerning/ligatures: the compiler owns shaping.
        let attrs: [NSAttributedString.Key: Any] = [
            NSAttributedString.Key(kCTFontAttributeName as String): font,
            NSAttributedString.Key(kCTKernAttributeName as String): 0,
            NSAttributedString.Key(kCTLigatureAttributeName as String): 0,
        ]
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: attrs))
        // PDF space is y-up: y_pdf = height - baseline_y.
        ctx.textPosition = CGPoint(x: x, y: h - baseline)
        CTLineDraw(line, ctx)
    }
    ctx.endPDFPage()
}
ctx.closePDF()
