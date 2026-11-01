// compare_coretext.swift — measure strings with CoreText and with the
// flashtex-font-engine `measure` example, and print the deltas.
//
// Usage (macOS, from crates/font-engine):
//   swift examples/compare_coretext.swift [--size 12] [<face> <text>]...
//
// <face> is a Core 14 name (Times-Roman, Times-Bold, Times-Italic,
// Times-BoldItalic, Helvetica, Courier, Symbol) or a path to a .ttf.
// For Core 14 names CoreText is asked for the same PostScript name, which on
// macOS resolves to Apple's Times/Helvetica/Courier/Symbol faces — different
// font programs from Adobe's AFMs, so small deltas are expected and recorded,
// not hidden. For a path, both sides read the identical program.
//
// Two numbers per side:
//   plain  = sum of per-glyph advances, no kerning/ligatures
//            (CTFontGetGlyphsForCharacters + CTFontGetAdvancesForGlyphs)
//   shaped = CTLine typographic width with CoreText's default shaping
//            (kerning + standard ligatures) versus the engine's default shaping.
// Text is passed to the Rust side on stdin so macOS cannot NFD-rewrite it.

import CoreText
import Foundation

var size = 12.0
var cases: [(String, String)] = []
var args = Array(CommandLine.arguments.dropFirst())
while !args.isEmpty {
    let a = args.removeFirst()
    if a == "--size" {
        size = Double(args.removeFirst())!
    } else {
        guard !args.isEmpty else {
            FileHandle.standardError.write("expected <face> <text> pairs\n".data(using: .utf8)!)
            exit(2)
        }
        cases.append((a, args.removeFirst()))
    }
}
if cases.isEmpty {
    let tnr = "/System/Library/Fonts/Supplemental/Times New Roman.ttf"
    let sentence = "The quick brown fox jumps over the lazy dog. AVATAR office, fluffy waffle."
    let accents = "Café naïve façade — “quoted” … 100% €5 ™ e\u{301}"
    for face in ["Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic", "Helvetica", "Courier"] {
        cases.append((face, "Hello"))
        cases.append((face, sentence))
        cases.append((face, accents))
    }
    cases.append(("Symbol", "\u{03B1}\u{03B2}\u{03B3} \u{2211}\u{222B}"))
    if FileManager.default.fileExists(atPath: tnr) {
        cases.append((tnr, "Hello"))
        cases.append((tnr, sentence))
        cases.append((tnr, accents))
        cases.append((tnr, "AV fi fl ffi"))
    }
}

func ctFont(_ face: String) -> CTFont? {
    if face.hasPrefix("/") {
        guard let provider = CGDataProvider(filename: face), let cg = CGFont(provider) else { return nil }
        return CTFontCreateWithGraphicsFont(cg, CGFloat(size), nil, nil)
    }
    return CTFontCreateWithName(face as CFString, CGFloat(size), nil)
}

func plainWidth(_ font: CTFont, _ text: String) -> (Double, Int) {
    var total = 0.0
    var missing = 0
    for scalar in text.unicodeScalars {
        var units = Array(String(Character(scalar)).utf16)
        var glyphs = [CGGlyph](repeating: 0, count: units.count)
        let ok = CTFontGetGlyphsForCharacters(font, &units, &glyphs, units.count)
        if !ok || glyphs[0] == 0 {
            missing += 1
            // Count the .notdef advance, as the engine does.
            glyphs = [0]
        }
        var adv = CGSize.zero
        total += CTFontGetAdvancesForGlyphs(font, .horizontal, &glyphs, &adv, 1)
    }
    return (total, missing)
}

func shapedWidth(_ font: CTFont, _ text: String) -> Double {
    let attrs: [NSAttributedString.Key: Any] = [kCTFontAttributeName as NSAttributedString.Key: font]
    let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: attrs))
    return CTLineGetTypographicBounds(line, nil, nil, nil)
}

func rustMeasure(_ face: String, _ text: String) -> [String: [String]] {
    let p = Process()
    p.executableURL = URL(fileURLWithPath: "/usr/bin/env")
    p.arguments = ["cargo", "run", "--quiet", "--example", "measure", "--", face, String(size), "--stdin"]
    let input = Pipe(), output = Pipe()
    p.standardInput = input
    p.standardOutput = output
    p.standardError = FileHandle.standardError
    try! p.run()
    input.fileHandleForWriting.write((text + "\n").data(using: .utf8)!)
    input.fileHandleForWriting.closeFile()
    let data = output.fileHandleForReading.readDataToEndOfFile()
    p.waitUntilExit()
    var out: [String: [String]] = [:]
    for line in String(data: data, encoding: .utf8)!.split(separator: "\n") {
        let f = line.split(separator: "\t").map(String.init)
        out[f[0]] = Array(f.dropFirst())
    }
    return out
}

print("size \(size) pt; delta = engine - CoreText (pt)")
print("face\tmode\tcoretext\tengine\tdelta\tnote")
var worst = 0.0
for (face, text) in cases {
    guard let font = ctFont(face) else {
        print("\(face)\t-\t-\t-\t-\tCoreText could not load")
        continue
    }
    let r = rustMeasure(face, text)
    let (ctPlain, ctMissing) = plainWidth(font, text)
    let ctShaped = shapedWidth(font, text)
    let label = face.hasPrefix("/") ? (face as NSString).lastPathComponent : face
    let short = text.count > 24 ? String(text.prefix(24)) + "…" : text
    if let plain = r["plain"], plain.count >= 3, let w = Double(plain[0]) {
        let d = w - ctPlain
        worst = max(worst, abs(d))
        print("\(label)\tplain\t\(String(format: "%.4f", ctPlain))\t\(String(format: "%.4f", w))\t\(String(format: "%+.4f", d))\t\"\(short)\" missing ct=\(ctMissing) engine=\(plain[2])")
    } else {
        print("\(label)\tplain\t\(String(format: "%.4f", ctPlain))\t-\t-\t\(r["plain"] ?? [])")
    }
    if let shaped = r["shaped"], shaped.count >= 5, let w = Double(shaped[0]) {
        let d = w - ctShaped
        print("\(label)\tshaped\t\(String(format: "%.4f", ctShaped))\t\(String(format: "%.4f", w))\t\(String(format: "%+.4f", d))\tligatures=\(shaped[3]) kerning=\(shaped[4])")
    } else {
        print("\(label)\tshaped\t\(String(format: "%.4f", ctShaped))\t-\t-\t\(r["shaped"] ?? [])")
    }
}
print("largest |plain delta|: \(String(format: "%.4f", worst)) pt")
