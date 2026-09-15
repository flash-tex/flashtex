import CoreGraphics
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Inline math preview on hover (lane mac-math-hover, deferred from
/// mac-editor-dx-3): the span-finding pure function
/// (`EditorIntelligence.inlineMathSpan`) and the crop-rect computation
/// (`MathHoverPreview.crop`) against small fixture display lists — no
/// rendering, no AppKit. `testRealProducer…` at the bottom drives the actual
/// producer and is gated on `FLASHTEX_RENDER`.
final class MathHoverTests: XCTestCase {
    typealias EI = EditorIntelligence

    // MARK: inlineMathSpan (pure)

    func testInlineMathSpanDollarAndParen() {
        // Indices: 0123456789…: "Text $a+b$ and \(c\) end." — `$a+b$` is
        // 5..<10, `\(c\)` (a literal backslash-paren, not an escape) is 15..<20.
        let s = "Text $a+b$ and \\(c\\) end." as NSString
        // Anywhere inside `$a+b$`, including on a delimiter, resolves to the
        // whole formula (delimiters included).
        for at in 5...9 { XCTAssertEqual(EI.inlineMathSpan(in: s, at: at), NSRange(location: 5, length: 5), "at \(at)") }
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 4)) // the space before "$"
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 10)) // the space after "$"
        for at in 15...19 { XCTAssertEqual(EI.inlineMathSpan(in: s, at: at), NSRange(location: 15, length: 5), "at \(at)") } // \(c\)
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 20)) // the space after "\)"
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 0)) // plain text
    }

    func testInlineMathSpanExcludesDisplayMathAndEnvironments() {
        XCTAssertNil(EI.inlineMathSpan(in: "$$a+b$$" as NSString, at: 3))
        XCTAssertNil(EI.inlineMathSpan(in: "\\[a+b\\]" as NSString, at: 3))
        XCTAssertNil(EI.inlineMathSpan(in: "\\begin{align}a+b\\end{align}" as NSString, at: 14))
    }

    func testInlineMathSpanTwoLinesOkThreeLinesNil() {
        let two = "$a+\nb$" as NSString // "$a+\n" / "b$": 2 lines, closes on the second
        XCTAssertEqual(EI.inlineMathSpan(in: two, at: 1), NSRange(location: 0, length: two.length))
        XCTAssertEqual(EI.inlineMathSpan(in: two, at: two.length - 1), NSRange(location: 0, length: two.length))
        let three = "$a\n+\nb$" as NSString // opens, then two more line breaks before it closes: 3 lines
        XCTAssertNil(EI.inlineMathSpan(in: three, at: 1))
        XCTAssertNil(EI.inlineMathSpan(in: three, at: three.length - 1))
    }

    // MARK: crop (pure; fixture display list)

    static func ticks(_ pt: Double) -> Int64 { Int64((pt * Double(RenderingV2.ticksPerPoint)).rounded()) }
    static func rect(_ x: Double, _ top: Double, _ w: Double, _ h: Double) -> RenderingV2.Rect {
        RenderingV2.Rect(x: ticks(x), top: ticks(top), width: ticks(w), height: ticks(h))
    }

    /// A single cluster carrying `span` with `hitRects` (a run per rect,
    /// glyphs elided — `formulaBox` only reads `sources`/`hitRects`).
    static func glyphRun(hitRects: [RenderingV2.Rect], span: RenderingV2.SourceRange) -> RenderingV2.Item {
        .glyphRun(RenderingV2.GlyphRun(fontId: "f", fontSize: ticks(10), text: "x",
                                       glyphs: [.init(gid: 1, originX: 0, baselineY: 0, advanceX: 0, advanceY: 0, cluster: 0)],
                                       clusters: [.init(textStartByte: 0, textEndByte: 1, hitRects: hitRects, carets: [], sources: [span])],
                                       paint: .black))
    }

    /// `"Text $a+b$ end.\n"`: the formula (`$a+b$`) is bytes 5..<10. Two
    /// glyph-run items (as `\frac`'s numerator/denominator might be) and one
    /// rule item (a fraction bar, deliberately near the page's left/bottom
    /// edge) all carry that span; their rects union to x:[1,65) top:[18,93) —
    /// the crop pads that by 4pt (x:[−3,69) top:[14,97)) and clamps to the
    /// 200×100 pt page (x clamped to 0).
    static let text = "Text $a+b$ end.\n"
    static let span = RenderingV2.SourceRange(path: "main.tex", startByte: 5, endByte: 10)
    static func page(items: [RenderingV2.Item]? = nil) -> RenderingV2.Page {
        let defaultItems: [RenderingV2.Item] = [
            glyphRun(hitRects: [rect(10, 20, 30, 15)], span: span),
            glyphRun(hitRects: [rect(45, 18, 20, 17)], span: span),
            .rule(.init(x: ticks(1), top: ticks(90), width: ticks(2), height: ticks(3), paint: .black, sources: [span])), // near the left/bottom edge: exercises clamping
        ]
        return RenderingV2.Page(number: 1, width: ticks(200), height: ticks(100), items: items ?? defaultItems)
    }

    func testCropUnionsMemberItemsPadsAndClamps() {
        let crop = MathHoverPreview.crop(in: Self.text as NSString, at: 7, path: "main.tex", pages: [Self.page()], previewIsStale: false)
        // Union of the three items: x:[1,65) top:[18,93) -> padded 4pt -> x:[0,69) top:[14,97), x clamped at 0.
        XCTAssertEqual(crop, MathHoverPreview.Crop(pageIndex: 0, rect: Self.rect(0, 14, 69, 83)))
    }

    func testCropNilWhenStale() {
        XCTAssertNil(MathHoverPreview.crop(in: Self.text as NSString, at: 7, path: "main.tex", pages: [Self.page()], previewIsStale: true))
    }

    func testCropNilWhenNotInFormula() {
        XCTAssertNil(MathHoverPreview.crop(in: Self.text as NSString, at: 1, path: "main.tex", pages: [Self.page()], previewIsStale: false)) // "e" of "Text": plain
    }

    /// The frame doesn't cover the span: no item on any page carries it (a
    /// stale/different revision's frame, or a formula the compiler dropped).
    func testCropNilWhenFrameDoesNotCoverSpan() {
        let otherSpan = RenderingV2.SourceRange(path: "main.tex", startByte: 100, endByte: 105)
        let page = RenderingV2.Page(number: 1, width: Self.ticks(200), height: Self.ticks(100),
                                    items: [Self.glyphRun(hitRects: [Self.rect(10, 20, 30, 15)], span: otherSpan)])
        XCTAssertNil(MathHoverPreview.crop(in: Self.text as NSString, at: 7, path: "main.tex", pages: [page], previewIsStale: false))
    }

    /// A two-line formula: `formulaBox` unions member items exactly as it
    /// does for a one-line one, since it only ever matches by source span —
    /// what makes this "multi-line" is purely `inlineMathSpan` finding the
    /// span across the line break (already covered above); this checks the
    /// crop pipeline end to end for it.
    func testCropForMultiLineFormula() {
        let text = "$a+\nb$\n" as NSString // "$a+\n" / "b$\n": the formula is the whole first 6 UTF-16 units
        let span = RenderingV2.SourceRange(path: "main.tex", startByte: 0, endByte: 6)
        let page = RenderingV2.Page(number: 1, width: Self.ticks(200), height: Self.ticks(100),
                                    items: [Self.glyphRun(hitRects: [Self.rect(20, 20, 10, 10)], span: span)])
        let crop = MathHoverPreview.crop(in: text, at: 1, path: "main.tex", pages: [page], previewIsStale: false)
        XCTAssertEqual(crop, MathHoverPreview.Crop(pageIndex: 0, rect: Self.rect(16, 16, 18, 18)))
        // Hovering the second line resolves to the identical span/crop.
        XCTAssertEqual(MathHoverPreview.crop(in: text, at: 5, path: "main.tex", pages: [page], previewIsStale: false), crop)
    }

    func testPixelRectConversion() {
        let r = Self.rect(10, 20, 30, 15)
        XCTAssertEqual(MathHoverPreview.pixelRect(r, pixelsPerPoint: 2), CGRect(x: 20, y: 40, width: 60, height: 30))
    }

    // MARK: live producer (FLASHTEX_RENDER)

    /// Real producer on `$\frac{a}{b}$`: the crop over the formula's box has
    /// non-white pixels (the fraction actually painted there).
    @MainActor
    func testRealProducerFracFormulaCropHasInk() async throws {
        guard let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"], FileManager.default.isExecutableFile(atPath: render) else {
            throw XCTSkip("FLASHTEX_RENDER not set to a built flashtex-render")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("mathhover-live-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = "\\documentclass{article}\n\\begin{document}\nSome text $\\frac{a}{b}$ more text.\n\\end{document}\n"
        let texURL = root.appendingPathComponent("main.tex")
        try tex.write(to: texURL, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.autoCompile = false
        XCTAssertEqual(model.openTex(at: texURL), .opened)
        model.attachWorker(at: URL(fileURLWithPath: render))
        model.previewV2 = true
        model.setLiveV2(true)
        model.compile()
        let deadline = Date().addingTimeInterval(30)
        while model.inFlightRevision != nil || model.displayListV2 == nil || model.displayListV2?.isLoading == true {
            if Date() > deadline { throw XCTSkip("producer did not deliver a v2 frame within 30 s") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
        defer { model.detachWorker() }

        guard case .loaded(let frame, _)? = model.displayListV2 else {
            throw XCTSkip("no v2 frame; diagnostics: \(model.result?.diagnostics ?? [])")
        }
        let ns = tex as NSString
        let formulaRange = ns.range(of: "$\\frac{a}{b}$")
        guard formulaRange.location != NSNotFound else { throw XCTSkip("formula not found in the round-tripped source") }
        let at = formulaRange.location + 3 // inside "\frac"
        guard let crop = MathHoverPreview.crop(in: ns, at: at, path: "main.tex", pages: frame.list.pages, previewIsStale: false) else {
            return XCTFail("no crop for the \\frac formula; diagnostics: \(model.result?.diagnostics ?? [])")
        }
        let prepared = frame.prepared[crop.pageIndex]
        let bitmap = try XCTUnwrap(GlyphRunRenderer.rasterize(prepared, scale: 4))
        let pixel = MathHoverPreview.pixelRect(crop.rect, pixelsPerPoint: 4).integral
            .intersection(CGRect(x: 0, y: 0, width: bitmap.width, height: bitmap.height))
        XCTAssertFalse(pixel.isEmpty)
        let cropped = try XCTUnwrap(bitmap.cropping(to: pixel))
        let rgba = V2Parity.rgba(cropped)
        var nonWhite = 0
        var i = 0
        while i < rgba.count { if rgba[i] < 250 || rgba[i + 1] < 250 || rgba[i + 2] < 250 { nonWhite += 1 }; i += 4 }
        XCTAssertGreaterThan(nonWhite, 0, "the \\frac{a}{b} crop should have painted (non-white) pixels")
    }
}
