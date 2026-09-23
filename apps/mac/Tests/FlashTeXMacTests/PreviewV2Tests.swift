import XCTest
import CoreGraphics
import CoreText
import CryptoKit
import PDFKit
import FlashTeXProtocol
@testable import FlashTeXMac

/// Rendering, export and hit-testing of the experimental v2 display list.
/// Fonts come from the repository's bundled Latin Modern (apps/mac/Fonts),
/// resolved by content hash — never by platform name.
final class PreviewV2Tests: XCTestCase {
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")
    static let store = V2FontStore(directories: [fontsDir.path])

    /// The bundled lmroman10-regular.otf as a manifest entry (schema hash convention).
    static func lmRoman10() throws -> (RenderingV2.FontResource, V2FontStore.ResolvedFont) {
        guard let file = store.fonts.first(where: { $0.url.lastPathComponent == "lmroman10-regular.otf" }) else {
            throw XCTSkip("bundled lmroman10-regular.otf not found")
        }
        let cg = CGFont(CGDataProvider(url: file.url as CFURL)!)!
        let resource = RenderingV2.FontResource(fontId: "lm10", sha256: file.bytesSha256, byteLength: file.byteLength, format: "opentype-cff",
                                                faceIndex: 0, unitsPerEm: Int(cg.unitsPerEm), glyphCount: Int(cg.numberOfGlyphs),
                                                postscriptName: cg.postScriptName! as String)
        return (resource, try store.resolve(resource))
    }

    static let ticks = Double(RenderingV2.ticksPerPoint)
    static func t(_ pt: Double) -> Int64 { Int64((pt * ticks).rounded()) }

    /// Shapes `text` with CoreText's own layout (kerning + ligatures on) and
    /// turns the resulting glyph IDs/positions into a one-run display list at
    /// `origin` (top-left page space). Clusters come from the run's string
    /// indices, so a ligature is one cluster covering several source bytes.
    static func displayList(text: String, resource: RenderingV2.FontResource, font: CTFont, size: Double,
                            origin: CGPoint, page: CGSize) throws -> (RenderingV2.DisplayList, CTLine) {
        let attributed = NSAttributedString(string: text, attributes: [.font: font])
        let line = CTLineCreateWithAttributedString(attributed)
        let runs = CTLineGetGlyphRuns(line) as! [CTRun]
        XCTAssertEqual(runs.count, 1, "single-font text must shape to one CoreText run")
        let run = try XCTUnwrap(runs.first)
        let n = CTRunGetGlyphCount(run)
        var glyphs = [CGGlyph](repeating: 0, count: n)
        var positions = [CGPoint](repeating: .zero, count: n)
        var indices = [CFIndex](repeating: 0, count: n)
        CTRunGetGlyphs(run, CFRange(location: 0, length: 0), &glyphs)
        CTRunGetPositions(run, CFRange(location: 0, length: 0), &positions)
        CTRunGetStringIndices(run, CFRange(location: 0, length: 0), &indices)
        let utf16 = Array(text.utf16)
        // Cluster boundaries: UTF-16 string indices of each glyph, end-exclusive at the next glyph's index.
        func utf8Offset(utf16Index: Int) -> Int { String(utf16CodeUnits: Array(utf16[0..<utf16Index]), count: utf16Index).utf8.count }
        let ascent = CTFontGetAscent(font), descent = CTFontGetDescent(font)
        var clusters: [RenderingV2.Cluster] = []
        var v2glyphs: [RenderingV2.Glyph] = []
        for i in 0..<n {
            let start = utf8Offset(utf16Index: indices[i])
            let end = i + 1 < n ? utf8Offset(utf16Index: indices[i + 1]) : text.utf8.count
            let advance = (i + 1 < n ? positions[i + 1].x : Double(CTLineGetTypographicBounds(line, nil, nil, nil))) - positions[i].x
            let gx = origin.x + positions[i].x
            clusters.append(RenderingV2.Cluster(
                textStartByte: start, textEndByte: end,
                hitRects: [RenderingV2.Rect(x: t(gx), top: t(origin.y - ascent), width: t(advance), height: t(ascent + descent))],
                carets: [RenderingV2.Caret(textByte: start, x: t(gx), top: t(origin.y - ascent), height: t(ascent + descent))],
                sources: [RenderingV2.SourceRange(path: "main.tex", startByte: 10 + start, endByte: 10 + end)]))
            v2glyphs.append(RenderingV2.Glyph(gid: Int(glyphs[i]), originX: t(gx), baselineY: t(origin.y), advanceX: t(advance), advanceY: 0, cluster: i))
        }
        let list = RenderingV2.DisplayList(
            projectId: "golden", revision: 1, requiredFeatures: ["glyph_run", "rgba-srgb", "cluster-actualtext"],
            documents: [RenderingV2.DocumentResource(path: "main.tex", revision: 1, sha256: String(repeating: "0", count: 64), byteLength: 100)],
            fonts: [resource],
            pages: [RenderingV2.Page(number: 1, width: t(page.width), height: t(page.height),
                                     items: [.glyphRun(RenderingV2.GlyphRun(fontId: resource.fontId, fontSize: t(size), text: text, glyphs: v2glyphs, clusters: clusters, paint: .black))])],
            diagnostics: [])
        return (list, line)
    }

    static func pixels(_ image: CGImage) -> [UInt8] {
        let ctx = CGContext(data: nil, width: image.width, height: image.height, bitsPerComponent: 8, bytesPerRow: image.width * 4,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        return Array(UnsafeBufferPointer(start: ctx.data!.assumingMemoryBound(to: UInt8.self), count: image.width * image.height * 4))
    }

    static func differingPixels(_ a: CGImage, _ b: CGImage) -> Int {
        XCTAssertEqual(a.width, b.width); XCTAssertEqual(a.height, b.height)
        let pa = pixels(a), pb = pixels(b)
        var n = 0
        for i in stride(from: 0, to: min(pa.count, pb.count), by: 4) where pa[i..<i+4] != pb[i..<i+4] { n += 1 }
        return n
    }

    static func inkPixels(_ a: CGImage) -> Int {
        let p = pixels(a); var n = 0
        for i in stride(from: 0, to: p.count, by: 4) where p[i] < 250 || p[i+1] < 250 || p[i+2] < 250 { n += 1 }
        return n
    }

    // MARK: golden render: shared routine == CoreText's own CTLineDraw

    func testGlyphRunMatchesCTLineDrawPixelForPixel() throws {
        let (resource, resolved) = try Self.lmRoman10()
        let size = 12.0, scale = 4.0
        let font = resolved.ctFont(size: size)
        let page = CGSize(width: 120, height: 40)
        let origin = CGPoint(x: 8, y: 26)
        let (list, line) = try Self.displayList(text: "AV office fi", resource: resource, font: font, size: size, origin: origin, page: page)
        try RenderingV2.validate(list)
        let frame = try V2Frame.prepare(RenderingV2.Envelope(id: "golden", payload: list), store: Self.store)

        // Reference: CoreText draws the line itself at the same baseline origin.
        let reference = GlyphRunRenderer.bitmapContext(widthPt: page.width, heightPt: page.height, scale: scale)!
        reference.setFillColor(CGColor(gray: 0, alpha: 1))
        reference.textMatrix = .identity
        reference.textPosition = CGPoint(x: origin.x, y: page.height - origin.y)
        CTLineDraw(line, reference)
        let referenceImage = reference.makeImage()!

        let ours = GlyphRunRenderer.rasterize(page: list.pages[0], frame: frame, scale: scale)!
        XCTAssertGreaterThan(Self.inkPixels(ours), 200, "the run must actually paint glyphs")
        XCTAssertEqual(Self.differingPixels(ours, referenceImage), 0, "shared draw routine must match CTLineDraw exactly")
        // The ligature shaped by CoreText is one glyph/cluster over two bytes ("fi").
        guard case .glyphRun(let run) = list.pages[0].items[0] else { return XCTFail() }
        let fi = run.clusters.last!
        XCTAssertEqual(fi.textEndByte - fi.textStartByte, 2)
        XCTAssertEqual(run.glyphs.filter { $0.cluster == run.clusters.count - 1 }.count, 1)
    }

    // MARK: export == preview for the same list

    func testPDFExportRasterEqualsPreviewRaster() throws {
        let (resource, resolved) = try Self.lmRoman10()
        let size = 10.0, scale = 3.0
        let page = CGSize(width: 100, height: 36)
        var (list, _) = try Self.displayList(text: "Office", resource: resource, font: resolved.ctFont(size: size), size: size,
                                         origin: CGPoint(x: 6, y: 20), page: page)
        // Add a typed rule under the word so both primitives are covered.
        list.requiredFeatures.insert("rule", at: 1)
        list.pages[0].items.append(.rule(RenderingV2.Rule(x: Self.t(6), top: Self.t(23), width: Self.t(40), height: Self.t(0.5), paint: .black,
                                                          sources: [RenderingV2.SourceRange(path: "main.tex", startByte: 0, endByte: 5)])))
        try RenderingV2.validate(list)
        let frame = try V2Frame.prepare(RenderingV2.Envelope(id: "x", payload: list), store: Self.store)
        let preview = GlyphRunRenderer.rasterize(page: list.pages[0], frame: frame, scale: scale)!

        let pdf = GlyphRunRenderer.pdfData(frame: frame)
        let doc = try XCTUnwrap(PDFDocument(data: pdf))
        XCTAssertEqual(doc.pageCount, 1)
        let cgPage = try XCTUnwrap(doc.page(at: 0)?.pageRef)
        XCTAssertEqual(cgPage.getBoxRect(.mediaBox).size, page)
        // Rasterize the PDF page with the same bitmap setup and compare.
        let ctx = GlyphRunRenderer.bitmapContext(widthPt: page.width, heightPt: page.height, scale: scale)!
        ctx.drawPDFPage(cgPage)
        let exported = ctx.makeImage()!
        XCTAssertGreaterThan(Self.inkPixels(exported), 100)
        // The PDF path re-encodes glyph outlines through the embedded font program;
        // CoreGraphics rasterizes both from the same outlines and positions.
        XCTAssertEqual(Self.differingPixels(preview, exported), 0, "export must equal preview pixel for pixel")
    }

    // MARK: hit-testing and carets from clusters

    func testHitTestReturnsLigatureClusterWithTwoSourceBytes() throws {
        let (resource, resolved) = try Self.lmRoman10()
        let (list, _) = try Self.displayList(text: "fi", resource: resource, font: resolved.ctFont(size: 12), size: 12,
                                         origin: CGPoint(x: 10, y: 20), page: CGSize(width: 60, height: 30))
        guard case .glyphRun(let run) = list.pages[0].items[0] else { return XCTFail() }
        XCTAssertEqual(run.glyphs.count, 1, "Latin Modern shapes fi to one ligature glyph")
        XCTAssertEqual(run.clusters.count, 1)
        let rect = run.clusters[0].hitRects[0]
        // Inside the glyph's hit rect → the whole cluster, both source bytes.
        let hit = try XCTUnwrap(V2Geometry.hit(page: list.pages[0], tickX: rect.x + rect.width / 2, tickY: rect.top + rect.height / 2))
        XCTAssertEqual(hit.clusterIndex, 0)
        XCTAssertEqual(hit.text, "fi")
        XCTAssertEqual(hit.sources, [RenderingV2.SourceRange(path: "main.tex", startByte: 10, endByte: 12)])
        // Half-open edges: the right/bottom edge is outside.
        XCTAssertNil(V2Geometry.hit(page: list.pages[0], tickX: rect.x + rect.width, tickY: rect.top))
        XCTAssertNil(V2Geometry.hit(page: list.pages[0], tickX: rect.x, tickY: rect.top + rect.height))
        XCTAssertNotNil(V2Geometry.hit(page: list.pages[0], tickX: rect.x, tickY: rect.top))
        // Point-space query floors to ticks.
        XCTAssertNotNil(V2Geometry.hit(page: list.pages[0], atPointX: RenderingV2.points(rect.x) + 0.001, y: RenderingV2.points(rect.top) + 0.001))
    }

    func testHitTestPrefersLaterPaintedItemAndFindsRules() {
        let rect = RenderingV2.Rect(x: 0, top: 0, width: 100, height: 100)
        let run = RenderingV2.GlyphRun(fontId: "f", fontSize: 1, text: "a", glyphs: [.init(gid: 1, originX: 0, baselineY: 50, advanceX: 10, advanceY: 0, cluster: 0)],
                                       clusters: [.init(textStartByte: 0, textEndByte: 1, hitRects: [rect], carets: [], sources: [.init(path: "main.tex", startByte: 0, endByte: 1)])], paint: .black)
        let rule = RenderingV2.Rule(x: 50, top: 50, width: 10, height: 10, paint: .black, sources: nil, syntheticReason: "fraction bar")
        let page = RenderingV2.Page(number: 1, width: 200, height: 200, items: [.glyphRun(run), .rule(rule)])
        let onRule = V2Geometry.hit(page: page, tickX: 55, tickY: 55)
        XCTAssertEqual(onRule?.itemIndex, 1)
        XCTAssertNil(onRule?.clusterIndex)
        XCTAssertEqual(onRule?.syntheticReason, "fraction bar")
        XCTAssertEqual(V2Geometry.hit(page: page, tickX: 5, tickY: 5)?.itemIndex, 0)
        XCTAssertNil(V2Geometry.hit(page: page, tickX: 150, tickY: 150))
    }

    func testCaretMapsToExactClusterCaretOrWholeClusterFallback() throws {
        let (resource, resolved) = try Self.lmRoman10()
        let (list, _) = try Self.displayList(text: "AV fi", resource: resource, font: resolved.ctFont(size: 12), size: 12,
                                         origin: CGPoint(x: 10, y: 20), page: CGSize(width: 80, height: 30))
        guard case .glyphRun(let run) = list.pages[0].items[0] else { return XCTFail() }
        // Source bytes 10..: 'A'=10, 'V'=11, ' '=12, 'f'=13, 'i'=14.
        let onV = V2Geometry.clusters(containing: 11, path: "main.tex", in: list.pages[0])
        XCTAssertEqual(onV.count, 1)
        let onVFirst = try XCTUnwrap(onV.first)
        XCTAssertEqual(onVFirst.clusterIndex, 1)
        guard run.clusters.count > 1 else { return XCTFail("expected more than one run cluster") }
        XCTAssertEqual(onVFirst.caret, try XCTUnwrap(run.clusters[1].carets.first), "1:1 byte cluster → exact caret")
        // Inside the ligature: the cluster has a caret only at its start, so the
        // caret at byte 'i' (14) has no exact geometry → whole-cluster fallback.
        let onI = V2Geometry.clusters(containing: 14, path: "main.tex", in: list.pages[0])
        XCTAssertEqual(onI.count, 1)
        let onIFirst = try XCTUnwrap(onI.first)
        XCTAssertEqual(onIFirst.clusterIndex, run.clusters.count - 1)
        XCTAssertNil(onIFirst.caret)
        let lastRunCluster = try XCTUnwrap(run.clusters.last)
        XCTAssertEqual(onIFirst.hitRects, lastRunCluster.hitRects)
        XCTAssertEqual(V2Geometry.clusters(containing: 13, path: "main.tex", in: list.pages[0])[0].caret?.textByte, run.clusters.last!.textStartByte)
        // Other documents and out-of-range bytes match nothing.
        XCTAssertTrue(V2Geometry.clusters(containing: 11, path: "other.tex", in: list.pages[0]).isEmpty)
        XCTAssertTrue(V2Geometry.clusters(containing: 99, path: "main.tex", in: list.pages[0]).isEmpty)
    }

    // MARK: fail closed on resources

    func testUnavailableFontHashFailsTheWholeFrame() throws {
        let (resource, _) = try Self.lmRoman10()
        var missing = resource
        missing.fontId = "ghost"
        missing.sha256 = String(repeating: "ab", count: 32)
        let ok = RenderingV2.GlyphRun(fontId: "lm10", fontSize: Self.t(12), text: "a", glyphs: [.init(gid: 28, originX: 0, baselineY: Self.t(20), advanceX: 0, advanceY: 0, cluster: 0)],
                                      clusters: [.init(textStartByte: 0, textEndByte: 1, hitRects: [.init(x: 0, top: 0, width: 1, height: 1)], carets: [], sources: [.init(path: "main.tex", startByte: 0, endByte: 1)])], paint: .black)
        var bad = ok; bad.fontId = "ghost"
        let list = RenderingV2.DisplayList(projectId: "p", revision: 1, requiredFeatures: ["glyph_run", "rgba-srgb", "cluster-actualtext"],
                                           documents: [.init(path: "main.tex", revision: 1, sha256: String(repeating: "0", count: 64), byteLength: 1)],
                                           fonts: [resource, missing],
                                           pages: [.init(number: 1, width: Self.t(50), height: Self.t(50), items: [.glyphRun(ok), .glyphRun(bad)])], diagnostics: [])
        try RenderingV2.validate(list)
        XCTAssertThrowsError(try V2Frame.prepare(RenderingV2.Envelope(id: "x", payload: list), store: Self.store)) { error in
            let e = error as? RenderingV2.ValidationError
            XCTAssertEqual(e?.code, "font_resource_unavailable")
            XCTAssertTrue(e?.message.contains("font resource abab") == true, e?.message ?? "")
        }
    }

    func testManifestMismatchWithBundledBytesIsRefused() throws {
        let (resource, _) = try Self.lmRoman10()
        var wrongCount = resource; wrongCount.glyphCount = resource.glyphCount + 1
        XCTAssertThrowsError(try Self.store.resolve(wrongCount)) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "font_resource_mismatch") }
        var wrongLength = resource; wrongLength.byteLength += 1
        XCTAssertThrowsError(try Self.store.resolve(wrongLength)) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "font_resource_mismatch") }
        var wrongName = resource; wrongName.postscriptName = "Times-Roman"
        XCTAssertThrowsError(try Self.store.resolve(wrongName)) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "font_resource_mismatch") }
        var metricsOnly = resource; metricsOnly.format = "core14-afm"; metricsOnly.byteLength = 0
        XCTAssertThrowsError(try Self.store.resolve(metricsOnly)) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "font_resource_unavailable") }
    }

    /// D3: the historical SHA-256(bytes ‖ face_index BE u32) engine identifier
    /// is not a resource digest (draft contract L55–57); the producer emits raw
    /// digests and the store no longer tolerates the obsolete spelling.
    func testObsoleteBytesFace0DigestIsRefusedAsUnknown() throws {
        let (resource, byBytes) = try Self.lmRoman10()
        XCTAssertEqual(resource.sha256, byBytes.file.bytesSha256)
        var face0 = try Data(contentsOf: byBytes.file.url); face0.append(contentsOf: [0, 0, 0, 0])
        var obsolete = resource; obsolete.sha256 = V2FontStore.hex(SHA256.hash(data: face0))
        XCTAssertNotEqual(obsolete.sha256, resource.sha256)
        XCTAssertThrowsError(try Self.store.resolve(obsolete)) {
            let e = $0 as? RenderingV2.ValidationError
            XCTAssertEqual(e?.code, "font_resource_unavailable")
            XCTAssertTrue(e?.message.contains("no bundled font has this content hash") == true, e?.message ?? "")
        }
    }
}

/// Shell integration: opening a real display list and navigating from clusters.
@MainActor
final class PreviewV2ShellTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")

    private func model(withFixtureText: Bool = true) throws -> ShellModel {
        let model = ShellModel()
        if withFixtureText {
            let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.tex"), encoding: .utf8)
            model.replaceProject(entryText: tex)
        }
        return model
    }

    /// Starts the off-main load and runs the main run loop until it delivered.
    private func load(_ model: ShellModel, _ url: URL, file: StaticString = #filePath, line: UInt = #line) {
        let done = expectation(description: "load \(url.lastPathComponent)")
        model.loadDisplayListV2(url: url) { done.fulfill() }
        XCTAssertTrue(model.displayListV2?.isLoading == true, "state is .loading until the result arrives", file: file, line: line)
        wait(for: [done], timeout: 20)
        XCTAssertFalse(model.displayListV2?.isLoading == true, "load delivered", file: file, line: line)
    }

    func testOpeningTheRealDisplayListNavigatesLigatureClustersToSourceBytes() throws {
        let model = try model()
        XCTAssertEqual(model.previewV2, ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_V2"] != "0", "the v2 pane is the default; FLASHTEX_PREVIEW_V2=0 opts out")
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        guard case .loaded(let frame, _) = model.displayListV2 else { return XCTFail("expected a prepared frame: \(String(describing: model.displayListV2))") }
        XCTAssertTrue(model.previewV2)
        let page = frame.list.pages[0]
        guard case .glyphRun(let office) = page.items[4] else { return XCTFail() }
        // Click the ffi ligature (one glyph, cluster 1, three source bytes).
        let ffi = office.clusters[1].hitRects[0]
        let hit = try XCTUnwrap(V2Geometry.hit(page: page, tickX: ffi.x + ffi.width / 2, tickY: ffi.top + ffi.height / 2))
        XCTAssertEqual(hit.text, "ffi")
        model.navigateV2(hit)
        let sel = try XCTUnwrap(model.selection)
        XCTAssertEqual(sel.path, "main.tex")
        XCTAssertEqual((model.activeText as NSString).substring(with: sel.nsRange), "ffi")
        XCTAssertTrue(model.navigationNote?.hasPrefix("Selected main.tex bytes 75..<78") == true, model.navigationNote ?? "")
        // café: the é cluster maps to the three source bytes of \'e.
        guard case .glyphRun(let cafe) = page.items[14] else { return XCTFail() }
        let e = cafe.clusters[3].hitRects[0]
        model.navigateV2(try XCTUnwrap(V2Geometry.hit(page: page, tickX: e.x, tickY: e.top)))
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "\\'e")
        // Caret sync back into the preview: the editor caret inside \'e lights the é cluster (whole-cluster fallback).
        let matches = V2Geometry.clusters(containing: 142, path: "main.tex", in: page)
        XCTAssertEqual(matches.map(\.clusterIndex), [3])
        XCTAssertNil(try XCTUnwrap(matches.first).caret)
    }

    func testStaleBufferIsRefusedAndSyntheticContentHasNoSource() throws {
        let model = try model()
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        guard case .loaded(let frame, _) = model.displayListV2 else { return XCTFail() }
        let page = frame.list.pages[0]
        guard case .glyphRun(let run) = page.items[2] else { return XCTFail() }
        let r = run.clusters[0].hitRects[0]
        let hit = try XCTUnwrap(V2Geometry.hit(page: page, tickX: r.x, tickY: r.top))
        model.updateActiveText("edited " + model.activeText)
        model.selection = nil
        model.navigateV2(hit)
        XCTAssertNil(model.selection, "a display list for other bytes never selects")
        XCTAssertTrue(model.navigationNote?.contains("differs from the current buffer") == true, model.navigationNote ?? "")
        model.navigateV2(V2Geometry.Hit(itemIndex: 0, clusterIndex: nil, text: nil, sources: [], syntheticReason: "fraction bar", rect: r))
        XCTAssertEqual(model.navigationNote, "Generated content (fraction bar) has no source range.")
        XCTAssertNil(model.selection)
    }

    func testRefusedDisplayListShowsNoFrame() throws {
        let model = try model()
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-missing-font.json"))
        guard case .failed(let error, let source) = model.displayListV2 else { return XCTFail("expected refusal") }
        XCTAssertEqual(error.code, "font_resource_unavailable")
        XCTAssertEqual(source.url?.lastPathComponent, "display-list-v2-missing-font.json")
        XCTAssertNil(model.displayListV2?.frame)
        XCTAssertTrue(model.captureNote?.hasPrefix("Display list refused: font_resource_unavailable") == true, model.captureNote ?? "")
        // A runtime-v1 fixture is not a display list either.
        load(model, Self.fixtures.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("protocol/fixtures/compile-result.json"))
        guard case .failed(let e2, _) = model.displayListV2 else { return XCTFail() }
        XCTAssertEqual(e2.code, "unsupported_protocol_version")
    }

    // MARK: off-main loading, stale-result suppression, stale indicator

    func testLoadingRetainsThePreviousFrameAsStaleUntilTheNewOneIsVerified() throws {
        let model = try model()
        let text = Self.fixtures.appendingPathComponent("display-list-v2-text.json")
        load(model, text)
        guard case .loaded(let first, _) = model.displayListV2 else { return XCTFail() }
        // A second load: the first frame stays paintable, explicitly stale.
        let done = expectation(description: "reload")
        model.loadDisplayListV2(url: text) { done.fulfill() }
        guard case .loading(let source, let ticket, let previous, _, _) = model.displayListV2 else { return XCTFail("expected .loading, got \(String(describing: model.displayListV2))") }
        XCTAssertEqual(source, .file(text))
        XCTAssertEqual(previous?.preparedNonce, first.preparedNonce, "the previous verified frame is retained while loading")
        XCTAssertEqual(model.displayListV2?.frame?.preparedNonce, first.preparedNonce)
        XCTAssertTrue(model.captureNote?.hasPrefix("Loading display list") == true)
        wait(for: [done], timeout: 20)
        guard case .loaded(let second, _) = model.displayListV2 else { return XCTFail() }
        XCTAssertNotEqual(second.preparedNonce, first.preparedNonce, "a reload is a new frame instance")
        XCTAssertNotEqual(V2FrameIdentity.token(second), V2FrameIdentity.token(first))
        XCTAssertGreaterThan(ticket, 0)
        // A refusal after a good frame drops it: nothing unverified stays on screen.
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-missing-font.json"))
        XCTAssertNil(model.displayListV2?.frame)
    }

    func testStaleLoadResultNeverOverwritesANewerState() throws {
        let model = try model()
        let text = Self.fixtures.appendingPathComponent("display-list-v2-text.json")
        load(model, text)
        guard case .loaded(let frame, _) = model.displayListV2 else { return XCTFail() }
        let dropped = V2Loader.staleResultsDropped
        // A result for a ticket that is not the one in flight is dropped whether
        // it is a frame or a refusal, and the state is untouched.
        XCTAssertFalse(model.deliverDisplayListV2(ticket: -1, source: .file(text), outcome: .failed(RenderingV2.ValidationError(code: "x", message: "stale"))))
        guard case .loaded(let still, _) = model.displayListV2 else { return XCTFail("stale refusal must not replace the frame") }
        XCTAssertEqual(still.preparedNonce, frame.preparedNonce)
        XCTAssertFalse(model.deliverDisplayListV2(ticket: -2, source: .file(text), outcome: .loaded(frame)))
        XCTAssertEqual(V2Loader.staleResultsDropped, dropped + 2)
        // Three loads back to back: one preparation in flight (a), the newest
        // arrival waits (c), the one in between is dropped undecoded (b, coalesced).
        // a publishes (it is newer than what is on screen), then c; the final
        // state is c's refusal.
        let published = V2Loader.resultsPublished, coalesced = V2Loader.coalescedLoads
        let a = expectation(description: "a"), b = expectation(description: "b"), c = expectation(description: "c")
        model.loadDisplayListV2(url: text) { a.fulfill() }
        let ticketA = model.displayListV2?.ticket
        model.loadDisplayListV2(url: text) { b.fulfill() }
        XCTAssertEqual(model.displayListV2?.ticket, ticketA, "b waits behind a; no new ticket yet")
        XCTAssertNotNil(model.displayListV2?.queued)
        model.loadDisplayListV2(url: Self.fixtures.appendingPathComponent("display-list-v2-missing-font.json")) { c.fulfill() }
        XCTAssertEqual(V2Loader.coalescedLoads, coalesced + 1, "b was dropped undecoded")
        wait(for: [b, a, c], timeout: 20, enforceOrder: true)
        guard case .failed(let error, let source) = model.displayListV2 else { return XCTFail("the newest load (a refusal) is the final state") }
        XCTAssertEqual(source.url?.lastPathComponent, "display-list-v2-missing-font.json")
        XCTAssertEqual(error.code, "font_resource_unavailable")
        XCTAssertEqual(V2Loader.staleResultsDropped, dropped + 2, "nothing prepared was dropped after preparation")
        XCTAssertEqual(V2Loader.resultsPublished, published + 2, "a and c were published, b never prepared")
    }

    func testPreparedPagesCarryPDFSpaceGeometryForEveryItem() throws {
        let model = try model()
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        guard case .loaded(let frame, _) = model.displayListV2 else { return XCTFail() }
        XCTAssertEqual(frame.prepared.count, frame.list.pages.count)
        for (page, prepared) in zip(frame.list.pages, frame.prepared) {
            XCTAssertEqual(prepared.number, page.number)
            XCTAssertEqual(prepared.items.count, page.items.count, "one prepared item per list item, same order")
            XCTAssertEqual(prepared.glyphCount, page.items.reduce(0) { n, i in if case .glyphRun(let r) = i { n + r.glyphs.count } else { n } })
            for (item, ready) in zip(page.items, prepared.items) {
                switch (item, ready) {
                case (.glyphRun(let run), .run(let r)):
                    XCTAssertEqual(r.glyphs, run.glyphs.map { CGGlyph($0.gid) }, "original glyph IDs, unchanged")
                    XCTAssertEqual(r.positions.count, run.glyphs.count)
                    for (g, p) in zip(run.glyphs, r.positions) {
                        XCTAssertEqual(p.x, V2PreparedPage.serialized(RenderingV2.points(g.originX)))
                        XCTAssertEqual(p.y, V2PreparedPage.serialized(page.heightPt - RenderingV2.points(g.baselineY)), "y flipped once into PDF space")
                    }
                    XCTAssertEqual(CTFontGetSize(r.font), V2PreparedPage.serialized(RenderingV2.points(run.fontSize)))
                    XCTAssertEqual(CTFontCopyPostScriptName(r.font) as String, frame.fonts[run.fontId]?.resource.postscriptName)
                case (.rule(let rule), .rule(let rect, let paint)):
                    let exact = GlyphRunRenderer.pdfRect(x: rule.x, top: rule.top, width: rule.width, height: rule.height, pageHeight: page.heightPt)
                    XCTAssertEqual(rect, CGRect(x: V2PreparedPage.serialized(exact.minX), y: V2PreparedPage.serialized(exact.minY),
                                                width: V2PreparedPage.serialized(exact.width), height: V2PreparedPage.serialized(exact.height)))
                    XCTAssertEqual(paint, rule.paint)
                default: XCTFail("item kind changed during preparation")
                }
            }
        }
    }

    func testPageRasterizerDeliversOffMainBitmapsAndDropsStaleOnes() throws {
        let model = try model()
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        guard case .loaded(let frame, _) = model.displayListV2 else { return XCTFail() }
        let page = frame.prepared[0]
        let rasterizer = V2PageRasterizer(maxBytes: 64 << 20)
        let token = frame.pageToken(at: 0)
        rasterizer.setCurrent(frame: frame)
        // First request: nothing yet, rasterization starts off-main.
        XCTAssertNil(rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 1, dark: false))
        XCTAssertNil(rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 1, dark: false), "a second request while in flight does not start another")
        let deadline = Date().addingTimeInterval(20)
        while rasterizer.images.isEmpty, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        let bitmap = try XCTUnwrap(rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 1, dark: false), "bitmap arrived on the main run loop")
        XCTAssertEqual(rasterizer.rasterizations, 1)
        // Exactly the bytes of the shared routine: what the pane blits is what parity compares.
        let direct = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: 1))
        XCTAssertEqual(V2Parity.rgba(bitmap), V2Parity.rgba(direct))
        XCTAssertGreaterThan(PreviewV2Tests.inkPixels(bitmap), 500)
        // A different appearance/scale is a different key.
        XCTAssertNil(rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 2, dark: true))
        // The current frame changes to one without this page before that bitmap
        // arrives: it is dropped, never installed.
        rasterizer.setCurrent(pageTokens: ["other-page"])
        XCTAssertTrue(rasterizer.images.isEmpty, "bitmaps of pages not in the current frame are evicted")
        while rasterizer.rasterizations < 2, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        XCTAssertEqual(rasterizer.staleBitmapsDropped, 1)
        XCTAssertTrue(rasterizer.images.isEmpty)
        XCTAssertEqual(rasterizer.retainedBytes, 0)
        // Requests for a page that is not current never start work.
        XCTAssertNil(rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 1, dark: false))
        let settle = Date().addingTimeInterval(0.1)
        while Date() < settle { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        XCTAssertEqual(rasterizer.rasterizations, 2)
    }

    func testPageRasterizerRetentionIsBounded() throws {
        let model = try model()
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        guard case .loaded(let frame, _) = model.displayListV2 else { return XCTFail() }
        let page = frame.prepared[0]
        // Bitmap rows are padded by CoreGraphics; measure real bitmaps.
        let onePage = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: 1)).bytesPerRow * Int(page.heightPt.rounded(.up))
        let half = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: 0.5))
        let quarterPage = half.bytesPerRow * half.height
        // Room for one full bitmap at 1 px/pt plus a small one, not two full ones.
        let rasterizer = V2PageRasterizer(maxBytes: onePage + onePage / 2)
        let token = frame.pageToken(at: 0)
        rasterizer.setCurrent(frame: frame)
        let deadline = Date().addingTimeInterval(20)
        for dark in [false, true, false] {
            _ = rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 1, dark: dark)
        }
        while rasterizer.rasterizations < 2, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        // The second full bitmap evicted the first (least recently used); the newest is kept.
        XCTAssertEqual(rasterizer.images.count, 1)
        XCTAssertEqual(rasterizer.retainedBytes, onePage)
        XCTAssertNotNil(rasterizer.images[V2PageRasterizer.Key(pageToken: token, pixelsPerPoint: 1, dark: true)], "the newest bitmap is kept")
        XCTAssertNil(rasterizer.images[V2PageRasterizer.Key(pageToken: token, pixelsPerPoint: 1, dark: false)], "the oldest was evicted")
        // A small bitmap fits next to it.
        _ = rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 0.5, dark: false)
        while rasterizer.rasterizations < 3, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        XCTAssertLessThanOrEqual(rasterizer.retainedBytes, rasterizer.maxBytes)
        XCTAssertEqual(rasterizer.images.count, 2)
        XCTAssertEqual(rasterizer.retainedBytes, onePage + quarterPage)
        rasterizer.clear()
        XCTAssertEqual(rasterizer.retainedBytes, 0)
        XCTAssertTrue(rasterizer.images.isEmpty)
    }

    /// The preview HUD's page readout, on the pane that is actually shipped.
    ///
    /// Regression: `toolbarPageCount` read `result?.pages.count`, and the v2
    /// route asks for `display-list-v2-only`, so a live reply carries no v1
    /// pages and the count was 0. The readout itself was additionally gated on
    /// `!model.previewV2` while `previewV2` defaults true — so on the shipped
    /// default there was no page number at all, from either half.
    @MainActor
    func testPageReadoutCountsTheDisplayListNotTheElidedV1Pages() throws {
        let model = try model()
        XCTAssertTrue(model.previewV2)
        load(model, Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        guard case .loaded(let frame, _) = model.displayListV2 else {
            return XCTFail("expected a prepared frame: \(String(describing: model.displayListV2))")
        }
        // A live v2 reply: `display-list-v2-only` elides the v1 pages.
        model.result = RuntimeV1.CompileResult(projectId: frame.list.projectId, revision: frame.list.revision,
                                               status: .ok, pages: [], diagnostics: [], pdfPath: nil)
        XCTAssertEqual(model.toolbarPageCount, frame.list.pages.count)
        XCTAssertGreaterThan(model.toolbarPageCount, 0, "the readout must not vanish when v1 pages are elided")

        // Either pane reports the page under the viewport through the model.
        XCTAssertEqual(model.previewVisiblePage, 1)
        model.v2WindowSawVisiblePage(2)
        XCTAssertEqual(model.previewVisiblePage, 2)

        // The readout is not gated on the pane any more.
        let source = try String(contentsOf: URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("Sources/FlashTeXMac/ContentView.swift"), encoding: .utf8)
        XCTAssertFalse(source.contains("if !model.previewV2, chrome.hasResult"),
                       "the page readout must not be v1-only")
        XCTAssertTrue(source.contains("model.previewVisiblePage"))
    }
}

/// The negotiated live route (docs/contracts/runtime-v1-display-list-v2.md):
/// `display-list-v2` requested while the pane is visible, the worker's sibling
/// `display_list` line applied only for the applied compile_result.
@MainActor
final class PreviewV2LiveTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fakeWorker = fixtures.appendingPathComponent("fake_worker_v2.py")
    static let template = fixtures.appendingPathComponent("display-list-v2-text.json")
    static let python = URL(fileURLWithPath: "/usr/bin/python3")

    private func fixtureText() throws -> String {
        try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.tex"), encoding: .utf8)
    }

    /// A model with the fake v2 producer attached and the pane "visible".
    private func liveModel(text: String) -> ShellModel {
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: text)
        model.attachWorker(at: Self.python, arguments: [Self.fakeWorker.path, Self.template.path])
        model.previewV2 = true
        model.setLiveV2(true) // what PreviewV2Pane.onAppear does
        return model
    }

    private func waitUntil(timeout: TimeInterval = 15, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    func testWorkerClientRoutesTheSiblingLineAndRejectsOtherV2Messages() throws {
        let line = try Data(contentsOf: Self.template)
        guard case .displayList(let id, let bytes) = WorkerClient.decode(line) else { return XCTFail("expected .displayList") }
        XCTAssertEqual(id, "req-1")
        XCTAssertEqual(bytes, line, "the raw line is handed on; decoding happens in V2Loader")
        let other = Data("{\"protocol_version\":2,\"id\":\"x\",\"type\":\"render_capabilities\",\"payload\":{}}".utf8)
        guard case .protocolViolation(let message) = WorkerClient.decode(other) else { return XCTFail("other v2 messages stay violations") }
        XCTAssertTrue(message.contains("protocol_version 2"), message)
    }

    func testPaneVisibilityRequestsTheCapability() {
        let model = ShellModel()
        XCTAssertFalse(model.requestedLayoutCapabilities.contains(V2Live.capability), "never requested by default (gate: consumer tests first)")
        model.setLiveV2(true)
        XCTAssertEqual(model.requestedLayoutCapabilities.filter { $0 == V2Live.capability }.count, 1)
        model.setLiveV2(true)
        XCTAssertEqual(model.requestedLayoutCapabilities.filter { $0 == V2Live.capability }.count, 1, "idempotent")
        model.setLiveV2(false)
        XCTAssertFalse(model.requestedLayoutCapabilities.contains(V2Live.capability))
        XCTAssertEqual(model.requestedLayoutCapabilities, ShellModel.defaultLayoutCapabilities(), "the other capabilities are untouched")
    }

    func testLiveFrameArrivesWithTheCompileResultAndNavigates() async throws {
        let tex = try fixtureText()
        let model = liveModel(text: tex)
        let accepted = V2Live.linesAccepted
        model.compile()
        try await waitUntil { model.inFlightRevision == nil && model.displayListV2?.frame != nil && model.displayListV2?.isLoading == false }
        XCTAssertTrue(model.liveV2Accepted)
        XCTAssertEqual(model.acceptedLayoutCapabilities, [V2Live.capability])
        guard case .loaded(let frame, let source) = model.displayListV2 else { return XCTFail("\(String(describing: model.displayListV2))") }
        guard case .worker(let id, let project, let revision, let line) = source else { return XCTFail("live source expected") }
        XCTAssertEqual(try RenderingV2.decode(line).payload.revision, revision, "the live line's bytes are retained with the frame")
        XCTAssertEqual(try Data(contentsOf: try source.listFileURL()), line, "and can be handed to file-taking tools")
        XCTAssertEqual(id, model.resultID)
        XCTAssertEqual(revision, model.result?.revision)
        XCTAssertEqual(project, model.result?.projectId)
        XCTAssertEqual(frame.list.revision, model.result?.revision)
        XCTAssertEqual(frame.list.documents[0].sha256, SourceDigest.sha256Hex(tex), "the producer attests the request text")
        XCTAssertEqual(V2Live.linesAccepted, accepted + 1)
        XCTAssertTrue(model.captureNote?.hasPrefix("Live display list live \(id)") == true, model.captureNote ?? "")
        // Cluster → source navigation works on the live frame (digest matches the buffer).
        let page = frame.list.pages[0]
        guard case .glyphRun(let office) = page.items[4] else { return XCTFail() }
        let ffi = office.clusters[1].hitRects[0]
        model.navigateV2(try XCTUnwrap(V2Geometry.hit(page: page, tickX: ffi.x + ffi.width / 2, tickY: ffi.top + ffi.height / 2)))
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "ffi")
        // Zero-tolerance parity holds on the live frame too.
        XCTAssertTrue(V2Parity.compare(frame: frame, scale: 2).identical)
        // A second edit: the previous frame stays (stale-labelled) until the new one is verified, then is replaced.
        let firstNonce = frame.preparedNonce
        model.updateActiveText(tex + "% edit\n")
        model.compile()
        try await waitUntil { model.result?.revision == model.editorRevision && model.displayListV2?.isLoading == false && (model.displayListV2?.frame?.preparedNonce ?? firstNonce) != firstNonce }
        XCTAssertEqual(model.displayListV2?.frame?.list.revision, model.editorRevision)
        model.detachWorker()
    }

    func testOldProducerWithoutTheCapabilityKeepsTheV1PreviewOnly() async throws {
        let model = liveModel(text: "%v2nocap\n" + (try fixtureText()))
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertFalse(model.liveV2Accepted)
        XCTAssertEqual(model.result?.status, .ok)
        XCTAssertNil(model.displayListV2, "no line, no frame; the v1 pages are the preview")
        model.detachWorker()
    }

    func testDeclinedPerRequestCarriesTheWarningAndNoFrame() async throws {
        let model = liveModel(text: "%v2decline\n" + (try fixtureText()))
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertFalse(model.liveV2Accepted)
        XCTAssertTrue(model.result?.diagnostics.contains { $0.severity == .warning && $0.message.hasPrefix("display-list-v2 declined:") } == true)
        XCTAssertNil(model.displayListV2)
        model.detachWorker()
    }

    func testFailedResultSendsNoLine() async throws {
        let model = liveModel(text: "%v2failed\n" + (try fixtureText()))
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertEqual(model.result?.status, .failed)
        XCTAssertTrue(model.liveV2Accepted)
        try await Task.sleep(nanoseconds: 200_000_000)
        XCTAssertNil(model.displayListV2)
        model.detachWorker()
    }

    func testStaleUnsolicitedAndMismatchedLinesNeverApply() async throws {
        let tex = try fixtureText()
        // Stale/unknown id: dropped and counted, no state change.
        let stale = liveModel(text: "%v2stale\n" + tex)
        let dropped = V2Live.staleLinesDropped
        stale.compile()
        try await waitUntil { roundTripDone(stale) && V2Live.staleLinesDropped == dropped + 1 }
        XCTAssertNil(stale.displayListV2)
        XCTAssertTrue(stale.liveV2Accepted)
        stale.detachWorker()
        // Unsolicited: the line arrives although acceptance was not echoed → violation, dropped.
        let unsolicited = liveModel(text: "%v2unsolicited\n" + tex)
        let rejected = V2Live.unsolicitedLinesDropped
        unsolicited.compile()
        try await waitUntil { roundTripDone(unsolicited) && V2Live.unsolicitedLinesDropped == rejected + 1 }
        XCTAssertNil(unsolicited.displayListV2)
        XCTAssertTrue(unsolicited.workerStatus.hasPrefix("protocol violation: display_list"), unsolicited.workerStatus)
        unsolicited.detachWorker()
        // Correlation mismatch inside the payload: refused with a diagnostic, nothing painted.
        let mismatch = liveModel(text: "%v2mismatch\n" + tex)
        mismatch.compile()
        try await waitUntil { roundTripDone(mismatch) && mismatch.displayListV2 != nil && mismatch.displayListV2?.isLoading == false }
        guard case .failed(let error, let source) = mismatch.displayListV2 else { return XCTFail("\(String(describing: mismatch.displayListV2))") }
        XCTAssertEqual(error.code, "correlation_mismatch")
        XCTAssertTrue(source.isLive)
        XCTAssertNil(mismatch.displayListV2?.frame)
        mismatch.detachWorker()
        // Direct: a line for an id that is not the applied result never touches a loaded frame.
        let model = liveModel(text: tex)
        model.compile()
        try await waitUntil { model.inFlightRevision == nil && model.displayListV2?.frame != nil && model.displayListV2?.isLoading == false }
        let nonce = model.displayListV2?.frame?.preparedNonce
        let before = V2Live.staleLinesDropped
        model.receiveDisplayListV2(id: "mac-999", line: try Data(contentsOf: Self.template))
        XCTAssertEqual(V2Live.staleLinesDropped, before + 1)
        XCTAssertEqual(model.displayListV2?.frame?.preparedNonce, nonce)
        model.detachWorker()
    }

    /// The compile round trip finished.
    private func roundTripDone(_ m: ShellModel) -> Bool { m.inFlightRevision == nil && m.result != nil }
}

/// Export-versus-preview identity with NO tolerance, on real pipeline output.
final class PreviewV2ParityTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    /// The bundled fonts plus, when present, MacTeX's Latin Modern Math (for
    /// the rule fixture; skipped elsewhere). Font files are assets, not an engine.
    static let mathDir = "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm-math"
    static let store = V2FontStore(directories: [PreviewV2Tests.fontsDir.path, mathDir])

    func frame(_ name: String) throws -> V2Frame {
        let envelope = try RenderingV2.decode(try Data(contentsOf: Self.fixtures.appendingPathComponent(name)))
        return try V2Frame.prepare(envelope, store: Self.store)
    }

    /// Pinned rasterizer configuration for the zero-tolerance gate: sRGB
    /// premultiplied RGBA, 1 and 2 pixels per point (non-Retina and Retina
    /// display scales), antialiased, font smoothing off, subpixel positioning
    /// on. At other (fractional) scales CoreGraphics rasterizes thin glyph
    /// stems differently through a CTFont than through the PDF-embedded font
    /// (measured: a 0.7 px en dash at 1.37 px/pt, 52 px on a page; 0 px at
    /// 1.0/2.0 on 22 real pages), so those scales are reported, not asserted.
    static let pinnedScales = [1.0, 2.0]

    /// Fractional scales are measured and printed for the record, never
    /// asserted (see `pinnedScales`).
    static let reportedScales = [0.5, 1.37, 1.5, 1.9, 3.0]

    func testRealTextDisplayListExportsPixelIdenticalAtPinnedScales() throws {
        let frame = try frame("display-list-v2-text.json")
        for scale in Self.pinnedScales {
            let report = V2Parity.compare(frame: frame, scale: scale)
            XCTAssertEqual(report.tolerance, 0)
            XCTAssertEqual(report.pages.count, frame.list.pages.count)
            for page in report.pages {
                XCTAssertEqual(page.differingPixels, 0, "page \(page.page) at \(scale) px/pt")
                XCTAssertEqual(page.previewSha256, page.exportSha256)
                XCTAssertEqual(page.widthPx, Int((frame.prepared[0].widthPt * scale).rounded(.up)))
            }
            XCTAssertTrue(report.identical)
            XCTAssertGreaterThan(report.pdfBytes, 1000)
        }
        for scale in Self.reportedScales {
            print("preview-v2 parity (reported, not asserted): text @\(scale) px/pt → \(V2Parity.compare(frame: frame, scale: scale).totalDifferingPixels) differing pixel(s)")
        }
        // The preview bitmap is not blank.
        let preview = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: 2))
        XCTAssertGreaterThan(PreviewV2Tests.inkPixels(preview), 2000)
    }

    func testRealMathDisplayListWithTypedRulesExportsPixelIdentical() throws {
        guard FileManager.default.fileExists(atPath: Self.mathDir + "/latinmodern-math.otf") else {
            throw XCTSkip("latinmodern-math.otf not available at \(Self.mathDir)")
        }
        let frame = try frame("display-list-v2-math-rules.json")
        let rules = frame.list.pages.flatMap(\.items).filter { if case .rule = $0 { true } else { false } }.count
        XCTAssertGreaterThanOrEqual(rules, 3, "the fixture carries typed fraction rules")
        XCTAssertTrue(frame.fonts.values.contains { $0.resource.postscriptName == "LatinModernMath-Regular" })
        for scale in Self.pinnedScales {
            let report = V2Parity.compare(frame: frame, scale: scale)
            XCTAssertTrue(report.identical, "\(report.totalDifferingPixels) differing pixel(s) at \(scale) px/pt")
        }
        for scale in Self.reportedScales {
            print("preview-v2 parity (reported, not asserted): math-rules @\(scale) px/pt → \(V2Parity.compare(frame: frame, scale: scale).totalDifferingPixels) differing pixel(s)")
        }
    }

    /// The prepared geometry is what CoreGraphics' PDF writer serializes (7
    /// significant digits), so preview and export start from identical
    /// numbers; the displacement from the tick geometry is bounded.
    func testPreparedCoordinatesAreTheSerializedValues() throws {
        XCTAssertEqual(V2PreparedPage.serialized(637.7059526443481), 637.706)
        XCTAssertEqual(V2PreparedPage.serialized(0.39850521087646484), 0.3985052)
        XCTAssertEqual(V2PreparedPage.serialized(12.20423412322998), 12.20423)
        XCTAssertEqual(V2PreparedPage.serialized(174.43918323516846), 174.4392)
        XCTAssertEqual(V2PreparedPage.serialized(0), 0)
        XCTAssertEqual(V2PreparedPage.serialized(-1.0346), -1.0346)
        XCTAssertEqual(V2PreparedPage.serialized(V2PreparedPage.serialized(657.2353677749634)), V2PreparedPage.serialized(657.2353677749634), "idempotent")
        let frame = try frame("display-list-v2-math-rules.json")
        for (page, prepared) in zip(frame.list.pages, frame.prepared) {
            for (item, ready) in zip(page.items, prepared.items) {
                switch (item, ready) {
                case (.glyphRun(let run), .run(let r)):
                    for (g, p) in zip(run.glyphs, r.positions) {
                        XCTAssertEqual(p.x, V2PreparedPage.serialized(RenderingV2.points(g.originX)))
                        XCTAssertEqual(abs(p.x - RenderingV2.points(g.originX)) <= 5e-5, true, "≤ 5e-5 pt from the tick geometry")
                        XCTAssertEqual(abs(p.y - (page.heightPt - RenderingV2.points(g.baselineY))) <= 5e-5, true)
                    }
                case (.rule(let rule), .rule(let rect, _)):
                    let exact = GlyphRunRenderer.pdfRect(x: rule.x, top: rule.top, width: rule.width, height: rule.height, pageHeight: page.heightPt)
                    XCTAssertEqual(abs(rect.minX - exact.minX) <= 5e-5, true)
                    XCTAssertEqual(abs(rect.height - exact.height) <= 5e-8, true, "sub-point heights keep 7 significant digits")
                default: XCTFail()
                }
            }
        }
    }

    func testParityDetectsAOnePointDisplacement() throws {
        // Sanity: the comparator is not vacuous. Shift one run's export-side
        // glyphs by 1pt by preparing a second frame from an edited list and
        // rasterizing that through the PDF path.
        var frame = try frame("display-list-v2-text.json")
        var moved = frame.list
        guard case .glyphRun(var run) = moved.pages[0].items[2] else { return XCTFail() }
        run.glyphs = run.glyphs.map { var g = $0; g.originX += RenderingV2.ticksPerPoint; return g }
        moved.pages[0].items[2] = .glyphRun(run)
        let shifted = try V2Frame.prepare(RenderingV2.Envelope(id: "shifted", payload: moved), store: Self.store)
        let preview = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: 2))
        let other = try XCTUnwrap(GlyphRunRenderer.rasterize(shifted.prepared[0], scale: 2))
        XCTAssertGreaterThan(V2Parity.differingPixels(V2Parity.rgba(preview), V2Parity.rgba(other)), 50)
        // And a frame compared with itself is identical (bitmaps callback fires per page).
        var pages = 0
        let report = V2Parity.compare(frame: frame, scale: 2) { pages += 1; XCTAssertEqual($0.preview.width, $0.export.width) }
        XCTAssertEqual(pages, frame.list.pages.count)
        XCTAssertTrue(report.identical)
        frame.preparedNonce = 0 // silence the unused-mutation warning; frame is a value
    }
}
