import CoreGraphics
import CoreText
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// Unicode mode (`flashtex-host-unicode`) and the Typst host send their
/// native OpenType/TrueType fonts as `format` `opentype` with `glyph_ids`
/// (spec §11.1): a GLYPH's `code` is the glyph id in the face. The renderer
/// read those codes as character codes through the font's Unicode cmap, so
/// the default XeLaTeX document (TU-encoded Latin Modern, no fontspec)
/// showed `hello there world and all` as `?2HHQi?2`2 r Q`H/ M/ HH` (glyph 63
/// of LMRoman10-Regular is `h`; U+003F is `?`). No host is needed here: the
/// FONT is the repository's lmroman10-regular.otf with the info the XeTeX
/// host writes (crates/flashtex-xetex/src/out/fonts.rs `load_native`).
final class GlyphIdFontTests: XCTestCase {
    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    static let text = "hello there world and all"

    /// The FONT frame the XeTeX host sends for a native font.
    static func nativeFont(_ program: [UInt8], face: Int = 0, upem: Int = 1000) throws -> DL3Font {
        let m = 1.0 / Double(upem)
        let json = #"{"format":"opentype","glyph_ids":true,"face_index":\#(face),"units_per_em":\#(upem),"font_matrix":"\#(m) 0 0 \#(m) 0 0","outlines":"cff","variations":[]}"#
        return DL3Font(id: 1, key: [UInt8](repeating: 7, count: 32), info: try DL3JSON.parse(Array(json.utf8)), program: program)
    }

    static func lmRoman() throws -> [UInt8] {
        Array(try Data(contentsOf: repoRoot.appendingPathComponent("apps/mac/Fonts/lmroman10-regular.otf")))
    }

    /// The glyph ids XeTeX's shaping gives plain Latin text: the cmap's.
    static func glyphIds(_ s: String, in cg: CGFont) -> [CGGlyph] {
        let ct = CTFontCreateWithGraphicsFont(cg, 1, nil, nil)
        var units = Array(s.utf16), gs = [CGGlyph](repeating: 0, count: units.count)
        XCTAssertTrue(CTFontGetGlyphsForCharacters(ct, &units, &gs, units.count))
        return gs
    }

    func testGlyphIdCodesAreGlyphIdsNotCharacterCodes() throws {
        let font = try DL3RenderFont.load(Self.nativeFont(try Self.lmRoman())).get()
        let cg = try XCTUnwrap(font.cgFont)
        XCTAssertEqual(cg.postScriptName as String?, "LMRoman10-Regular")
        let ids = Self.glyphIds(Self.text, in: cg)
        // `h` is glyph 63 here: the code the owner's page carried.
        XCTAssertEqual(ids.first, 63)
        var names = ""
        for (ch, id) in zip(Self.text, ids) {
            XCTAssertEqual(font.glyph(Int(id)), id, "code \(id) (\(ch)) must draw glyph \(id)")
            XCTAssertTrue(font.draws(Int(id)), "\(ch)")
            if ch != " " { names += (cg.name(for: font.glyph(Int(id))) as String?) ?? "?" } else { names += " " }
        }
        XCTAssertEqual(names, Self.text, "the glyphs drawn spell the text")
        // Glyph ids past 255 draw (no 8-bit code limit) and past the face's count do not.
        XCTAssertGreaterThan(cg.numberOfGlyphs, 256)
        XCTAssertEqual(font.glyph(300), 300)
        XCTAssertTrue(font.draws(300))
        XCTAssertFalse(font.draws(cg.numberOfGlyphs))
        XCTAssertFalse(font.draws(0), ".notdef")
    }

    /// The page drawn through `DL3Renderer` equals Core Graphics' rendering
    /// of a PDF that shows the same glyph ids of the same font (zero tolerance).
    func testUnicodeModePageMatchesThePDFOfTheSameGlyphs() throws {
        let program = try Self.lmRoman()
        let font = try DL3RenderFont.load(Self.nativeFont(program)).get()
        let cg = try XCTUnwrap(font.cgFont)
        let ids = Self.glyphIds(Self.text, in: cg)
        let size = 9.962640099626, pageSize = CGSize(width: 200, height: 40)
        var x = 10.0
        var placed: [(CGGlyph, Double)] = []
        for g in ids {
            var gg = g, adv: Int32 = 0
            _ = cg.getGlyphAdvances(glyphs: &gg, count: 1, advances: &adv)
            placed.append((g, x))
            x += Double(adv) / 1000 * size
        }
        // The display list: matrix 1 (the font size), glyphs at sp positions.
        var page = DL3Page(kind: .page, index: 0)
        page.box = [0, 0, pageSize.width, pageSize.height]
        page.matrices = [DL3Matrix(a: size, b: 0, c: 0, d: size, e: 0, f: 0)]
        let baseline = 20.0
        page.items = [.matrix(1)] + placed.map { g, x in
            .glyph(font: 1, code: UInt16(g), x: Int32((x * DL3.spPerBp).rounded()), y: Int32(((pageSize.height - baseline) * DL3.spPerBp).rounded()), col: 0xFFFF)
        }
        let prepared = DL3PreparedPage(page: page, fonts: [1: font], images: [:])
        // The reference: a PDF showing the same glyph ids (as the engine's PDF does).
        let data = NSMutableData()
        var box = CGRect(origin: .zero, size: pageSize)
        let ctx = CGContext(consumer: CGDataConsumer(data: data as CFMutableData)!, mediaBox: &box, nil)!
        ctx.beginPDFPage(nil)
        ctx.setFont(cg); ctx.setFontSize(1)
        for (g, _) in placed.enumerated() {
            let item = prepared.page.items[g + 1]
            guard case .glyph(_, let code, let px, let py, _) = item, code != 0, font.draws(Int(code)) else { continue }
            ctx.textMatrix = CGAffineTransform(a: size, b: 0, c: 0, d: size, tx: Double(px) / DL3.spPerBp, ty: pageSize.height - Double(py) / DL3.spPerBp)
            ctx.showGlyphs([CGGlyph(code)], at: [.zero])
        }
        ctx.endPDFPage()
        ctx.closePDF()
        let pdf = try XCTUnwrap(CGPDFDocument(CGDataProvider(data: data as CFData)!)?.page(at: 1))
        var ink = 0
        for s in [1.0, 2.0, 4.0] {
            let a = try XCTUnwrap(DL3Renderer.rasterize(prepared, scale: s))
            let b = try XCTUnwrap(DL3Renderer.rasterize(pdfPage: pdf, scale: s))
            let ra = DL3Parity.rgba(a)
            let d = DL3Parity.diff(ra, DL3Parity.rgba(b))
            XCTAssertEqual(d.pixels, 0, "at \(s) px/pt: \(d.pixels) px differ (max delta \(d.maxDelta))")
            ink = max(ink, ra.enumerated().filter { $0.offset % 4 != 3 && $0.element < 128 }.count)
        }
        XCTAssertGreaterThan(ink, 100, "the page draws ink")
    }

    /// A collection's face (`face_index`) loads that face, not the first.
    func testCollectionFaceIndex() throws {
        let path = "/System/Library/Fonts/Helvetica.ttc"
        guard let data = FileManager.default.contents(atPath: path) else { throw XCTSkip("no \(path)") }
        let descs = try XCTUnwrap(CTFontManagerCreateFontDescriptorsFromData(data as CFData) as? [CTFontDescriptor])
        XCTAssertGreaterThan(descs.count, 1)
        for face in [0, 1] {
            let want = CTFontCopyPostScriptName(CTFontCreateWithFontDescriptor(descs[face], 1, nil)) as String
            let font = try DL3RenderFont.load(Self.nativeFont(Array(data), face: face, upem: 2048)).get()
            XCTAssertEqual(font.cgFont?.postScriptName as String?, want, "face \(face)")
        }
        XCTAssertNotEqual(try DL3RenderFont.load(Self.nativeFont(Array(data), face: 0, upem: 2048)).get().cgFont?.postScriptName as String?,
                          try DL3RenderFont.load(Self.nativeFont(Array(data), face: 1, upem: 2048)).get().cgFont?.postScriptName as String?)
    }
}
