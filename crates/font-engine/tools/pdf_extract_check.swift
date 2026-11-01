// pdf_extract_check.swift — round-trip check for the crate's embedding output.
//
//   swift tools/pdf_extract_check.swift <file.pdf> <expected text> [render.png]
//
// 1. Opens the PDF with PDFKit and extracts the page text (`PDFPage.string`),
//    which honours /ToUnicode and /ActualText; compares it with the expected
//    Unicode after trimming trailing whitespace. Exit 1 on mismatch.
// 2. Renders the page with CoreGraphics into a bitmap and reports how many
//    pixels are inked, so a font that fails to load (blank page) is caught;
//    optionally writes the PNG.

import Foundation
import PDFKit
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

let args = Array(CommandLine.arguments.dropFirst())
guard args.count >= 2 else {
    FileHandle.standardError.write("usage: pdf_extract_check <file.pdf> <expected text> [render.png]\n".data(using: .utf8)!)
    exit(2)
}
let url = URL(fileURLWithPath: args[0])
let expected = args[1]
guard let doc = PDFDocument(url: url), let page = doc.page(at: 0) else {
    print("FAIL: PDFKit could not open \(args[0])")
    exit(1)
}
let extracted = (page.string ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
let want = expected.trimmingCharacters(in: .whitespacesAndNewlines)
let textOK = extracted == want
print("extracted: \(extracted.debugDescription)")
print("expected:  \(want.debugDescription)")
print(textOK ? "TEXT PASS" : "TEXT FAIL")
if !textOK {
    // Show the first differing scalar for diagnosis.
    let a = Array(extracted.unicodeScalars), b = Array(want.unicodeScalars)
    for i in 0..<max(a.count, b.count) {
        let x = i < a.count ? String(format: "U+%04X", a[i].value) : "end"
        let y = i < b.count ? String(format: "U+%04X", b[i].value) : "end"
        if x != y { print("first difference at scalar \(i): extracted \(x) expected \(y)"); break }
    }
}

// Render at 4x for an ink count.
let scale: CGFloat = 4
let bounds = page.bounds(for: .mediaBox)
let w = Int(bounds.width * scale), h = Int(bounds.height * scale)
let cs = CGColorSpaceCreateDeviceGray()
guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w, space: cs, bitmapInfo: CGImageAlphaInfo.none.rawValue) else {
    print("FAIL: no bitmap context"); exit(1)
}
ctx.setFillColor(gray: 1, alpha: 1)
ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
ctx.scaleBy(x: scale, y: scale)
if let cgPage = page.pageRef {
    ctx.drawPDFPage(cgPage)
}
var inked = 0
if let data = ctx.data {
    let p = data.bindMemory(to: UInt8.self, capacity: w * h)
    for i in 0..<(w * h) where p[i] < 128 { inked += 1 }
}
print("inked pixels at \(Int(scale))x: \(inked)")
let renderOK = inked > 0
print(renderOK ? "RENDER PASS" : "RENDER FAIL (blank page: font not loaded?)")
if args.count >= 3, let img = ctx.makeImage(),
   let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: args[2]) as CFURL, UTType.png.identifier as CFString, 1, nil) {
    CGImageDestinationAddImage(dest, img, nil)
    CGImageDestinationFinalize(dest)
}
exit(textOK && renderOK ? 0 : 1)
