// Per-character box extractor for docs/comparison.md (Apple PDFKit only).
// Oracle tooling: reads a PDF produced by the reference pdflatex; not part of
// the product. Modelled on tools/native-validation/oracle_extract.swift, but
// selects one character at a time so math glyph boxes are individual.
//
// Usage: swiftc -O -o oracle_glyphs oracle_glyphs.swift && ./oracle_glyphs file.pdf
// Output: {"pages":[{"number":1,"width_pt":612,"height_pt":792,
//   "chars":[{"text":"x","x_pt":..,"right_pt":..,"top_pt":..,"bottom_pt":..},..]}]}
// Coordinates are PDF points with a top-left origin (top_pt/bottom_pt are the
// distances from the page top to the selection box's top and bottom edges).
import Foundation
import PDFKit

struct Char: Encodable {
    let text: String
    let x_pt: Double
    let right_pt: Double
    let top_pt: Double
    let bottom_pt: Double
}
struct Page: Encodable {
    let number: Int
    let width_pt: Double
    let height_pt: Double
    let chars: [Char]
}
struct Doc: Encodable { let pages: [Page] }

guard CommandLine.arguments.count == 2,
      let doc = PDFDocument(url: URL(fileURLWithPath: CommandLine.arguments[1])) else {
    FileHandle.standardError.write("usage: oracle_glyphs <file.pdf>\n".data(using: .utf8)!)
    exit(2)
}

var pages: [Page] = []
for i in 0..<doc.pageCount {
    guard let page = doc.page(at: i) else { continue }
    let box = page.bounds(for: .mediaBox)
    var chars: [Char] = []
    if let s = page.string {
        let ns = s as NSString
        let ws = CharacterSet.whitespacesAndNewlines
        for idx in 0..<ns.length {
            if let u = Unicode.Scalar(ns.character(at: idx)), ws.contains(u) { continue }
            let range = NSRange(location: idx, length: 1)
            guard let sel = page.selection(for: range) else { continue }
            let b = sel.bounds(for: page)
            if b.isNull || b.isEmpty { continue }
            chars.append(Char(
                text: ns.substring(with: range),
                x_pt: Double(b.minX - box.minX),
                right_pt: Double(b.maxX - box.minX),
                top_pt: Double(box.maxY - b.maxY),
                bottom_pt: Double(box.maxY - b.minY)))
        }
    }
    pages.append(Page(number: i + 1, width_pt: Double(box.width), height_pt: Double(box.height), chars: chars))
}

let enc = JSONEncoder()
enc.outputFormatting = [.sortedKeys]
let data = try! enc.encode(Doc(pages: pages))
FileHandle.standardOutput.write(data)
FileHandle.standardOutput.write("\n".data(using: .utf8)!)
