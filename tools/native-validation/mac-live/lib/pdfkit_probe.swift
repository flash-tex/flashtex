// PDFKit probe: prints {"pages": N, "text": "...", "bytes": B} for one PDF.
// Compiled on the fly by run.sh (swiftc) so the exact-export check uses the
// same reader macOS applications use, not a regex over the file.
import Foundation
import PDFKit

guard CommandLine.arguments.count > 1 else {
    FileHandle.standardError.write("usage: pdfkit_probe <file.pdf>\n".data(using: .utf8)!)
    exit(2)
}
let url = URL(fileURLWithPath: CommandLine.arguments[1])
guard let doc = PDFDocument(url: url) else {
    print("{\"pages\": 0, \"text\": \"\", \"error\": \"PDFDocument could not open the file\"}")
    exit(1)
}
var text = ""
for i in 0..<doc.pageCount {
    if let page = doc.page(at: i), let s = page.string { text += s + "\n" }
}
let bytes = (try? Data(contentsOf: url).count) ?? 0
let obj: [String: Any] = ["pages": doc.pageCount, "text": text, "bytes": bytes,
                          "page_size_pt": doc.page(at: 0).map { let b = $0.bounds(for: .mediaBox); return [b.width, b.height] } ?? []]
let data = try! JSONSerialization.data(withJSONObject: obj, options: [.sortedKeys])
print(String(data: data, encoding: .utf8)!)
