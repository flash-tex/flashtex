import CoreGraphics
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// path-v0 consumer (crates/render-pipeline `display.rs` `PathItem`; TikZ
/// pictures): `path_fill` / `path_stroke` items decode on both readers,
/// validate fail-closed, paint through the one draw routine (preview bitmap
/// and the CoreGraphics PDF export), hit-test to their source span, and take
/// part in the delta digest and relocation. Fixtures are built here in ticks.
final class V2PathTests: XCTestCase {
    static let pageW = 200.0, pageH = 200.0
    static func ticks(_ pt: Double) -> Int64 { Int64((pt * Double(RenderingV2.ticksPerPoint)).rounded()) }
    static func m(_ x: Double, _ y: Double) -> [Any] { ["m", ticks(x), ticks(y)] }
    static func l(_ x: Double, _ y: Double) -> [Any] { ["l", ticks(x), ticks(y)] }
    static func c(_ x1: Double, _ y1: Double, _ x2: Double, _ y2: Double, _ x: Double, _ y: Double) -> [Any] {
        ["c", ticks(x1), ticks(y1), ticks(x2), ticks(y2), ticks(x), ticks(y)]
    }
    static let z: [Any] = ["z"]
    static let tex = "\\begin{tikzpicture}\\draw (0,0) -- (2,1);\\end{tikzpicture}\n"
    static let sources: [[String: Any]] = [["path": "main.tex", "start_byte": 0, "end_byte": 51]]
    static func paint(_ r: Double, _ g: Double, _ b: Double) -> [String: Any] { ["r": r, "g": g, "b": b, "a": 1] }

    /// A circle of radius `r` at (cx, cy) as four cubic Béziers (κ = 0.5523).
    static func circle(_ cx: Double, _ cy: Double, _ r: Double) -> [[Any]] {
        let k = 0.5523 * r
        return [m(cx + r, cy),
                c(cx + r, cy + k, cx + k, cy + r, cx, cy + r),
                c(cx - k, cy + r, cx - r, cy + k, cx - r, cy),
                c(cx - r, cy - k, cx - k, cy - r, cx, cy - r),
                c(cx + k, cy - r, cx + r, cy - k, cx + r, cy),
                z]
    }

    /// Item 0: black triangle (20,20)-(80,20)-(50,70), nonzero fill.
    /// Item 1: red circle r=20 at (140,50) via curves.
    /// Item 2: dashed 2 pt black line (20,120)→(100,120) with 8 on / 8 off, plus
    ///         an arrowhead stroke (round caps/joins).
    /// Item 3: blue rectangle (110,100)-(190,180) filled, clipped to the circle
    ///         r=25 at (150,140), evenodd clip.
    static func items() -> [[String: Any]] {
        [
            ["kind": "path_fill", "fill_rule": "nonzero", "path": [m(20, 20), l(80, 20), l(50, 70), z], "paint": paint(0, 0, 0), "sources": sources],
            ["kind": "path_fill", "fill_rule": "nonzero", "path": circle(140, 50, 20), "paint": paint(1, 0, 0), "sources": sources, "vendor_extra": "tolerated"],
            ["kind": "path_stroke", "path": [m(20, 120), l(100, 120)], "paint": paint(0, 0, 0), "sources": sources,
             "stroke": ["width": ticks(2), "cap": "butt", "join": "miter", "miter_limit": 10, "dash": ["array": [ticks(8), ticks(8)], "phase": 0]]],
            ["kind": "path_stroke", "path": [m(92, 114), l(100, 120), l(92, 126)], "paint": paint(0, 0, 0), "sources": sources,
             "stroke": ["width": ticks(2), "cap": "round", "join": "round", "miter_limit": 10]],
            ["kind": "path_fill", "fill_rule": "nonzero", "path": [m(110, 100), l(190, 100), l(190, 180), l(110, 180), z], "paint": paint(0, 0, 1),
             "clips": [["kind": "path", "fill_rule": "evenodd", "path": circle(150, 140, 25)]], "sources": sources],
        ]
    }

    static func list(items: [[String: Any]] = items(), features: [String] = ["rgba-srgb", "cluster-actualtext", "path_fill", "path_stroke", "clip"]) -> [String: Any] {
        [
            "protocol_version": 2, "id": "path-1", "type": "display_list",
            "payload": [
                "render_format": "display-list-v2", "coordinate_unit": "bp_2pow20", "color_space": "srgb", "text_extraction": "cluster-actualtext",
                "project_id": "p", "revision": 1, "required_features": features,
                "documents": [["path": "main.tex", "revision": 1, "sha256": SourceDigest.sha256Hex(tex), "byte_length": tex.utf8.count]],
                "fonts": [],
                "pages": [["number": 1, "width": ticks(pageW), "height": ticks(pageH), "items": items]],
                "diagnostics": [],
            ],
        ]
    }

    static func data(_ o: [String: Any]) -> Data { try! JSONSerialization.data(withJSONObject: o) }

    /// RGB at page point (x, y down) of a bitmap at `scale`.
    static func pixel(_ image: CGImage, x: Double, yDown: Double, scale: Double, pageH: Double = pageH) -> (UInt8, UInt8, UInt8) {
        let rgba = V2Parity.rgba(image)
        let px = Int(x * scale), py = Int((pageH - yDown) * scale)
        let i = ((image.height - 1 - py) * image.width + px) * 4
        return (rgba[i], rgba[i + 1], rgba[i + 2])
    }
    static func isWhite(_ p: (UInt8, UInt8, UInt8)) -> Bool { p.0 > 250 && p.1 > 250 && p.2 > 250 }
    static func isDark(_ p: (UInt8, UInt8, UInt8)) -> Bool { p.0 < 80 && p.1 < 80 && p.2 < 80 }
    static func isRed(_ p: (UInt8, UInt8, UInt8)) -> Bool { p.0 > 200 && p.1 < 60 && p.2 < 60 }
    static func isBlue(_ p: (UInt8, UInt8, UInt8)) -> Bool { p.2 > 200 && p.0 < 60 && p.1 < 60 }

    // MARK: decode + validation

    func testPathItemsDecodeOnBothReadersAndRoundTrip() throws {
        let bytes = Self.data(Self.list())
        let fast = try RenderingV2Fast.envelope(bytes)
        let slow = try RenderingV2.decode(bytes)
        XCTAssertEqual(fast, slow, "the fast reader and JSONDecoder agree on path items")
        let items = slow.payload.pages[0].items
        XCTAssertEqual(items.count, 5)
        guard items.count == 5 else { return XCTFail("expected five items, got \(items.count)") }
        guard case .path(let tri) = items[0], case .path(let dashed) = items[2], case .path(let clipped) = items[4] else { return XCTFail() }
        XCTAssertEqual(tri.op, .fill(.nonzero))
        XCTAssertEqual(tri.path, [.move(x: Self.ticks(20), y: Self.ticks(20)), .line(x: Self.ticks(80), y: Self.ticks(20)), .line(x: Self.ticks(50), y: Self.ticks(70)), .close])
        XCTAssertEqual(tri.sources?.first?.endByte, 51)
        XCTAssertEqual(dashed.stroke, RenderingV2.Stroke(width: Self.ticks(2), cap: .butt, join: .miter, miterLimit: 10, dash: RenderingV2.Dash(array: [Self.ticks(8), Self.ticks(8)], phase: 0)))
        XCTAssertEqual(clipped.clips.count, 1)
        XCTAssertEqual(clipped.clips[0].fillRule, .evenodd)
        XCTAssertEqual(clipped.clips[0].path.count, 6)
        XCTAssertEqual(try RenderingV2.decode(try JSONEncoder().encode(slow)), slow, "round-trips through the encoder")
        // The kinds the producer writes survive re-encoding.
        let text = String(decoding: try JSONEncoder().encode(slow), as: UTF8.self)
        XCTAssertTrue(text.contains("\"path_fill\"") && text.contains("\"path_stroke\"") && text.contains("\"fill_rule\""))
    }

    func testPathValidationRefusesMalformedShapes() throws {
        func code(_ edit: (inout [String: Any]) -> Void, item: Int = 0) -> String? {
            var o = Self.list()
            var p = o["payload"] as! [String: Any]; var pages = p["pages"] as! [[String: Any]]; var items = pages[0]["items"] as! [[String: Any]]
            edit(&items[item]); pages[0]["items"] = items; p["pages"] = pages; o["payload"] = p
            do { _ = try RenderingV2.decode(Self.data(o)); return nil } catch let e as RenderingV2.ValidationError { return e.code } catch { return "other" }
        }
        func setStroke(_ i: inout [String: Any], _ k: String, _ v: Any?) { var s = i["stroke"] as! [String: Any]; s[k] = v; i["stroke"] = s }
        XCTAssertNil(code { _ in })
        XCTAssertEqual(code { $0["path"] = [] }, "invalid_display_list", "a path needs at least one command")
        XCTAssertEqual(code { $0["path"] = [Self.l(10, 10)] }, "invalid_display_list", "a line needs a current point")
        XCTAssertEqual(code { $0["path"] = [Self.m(10, 10), ["q", 1, 2]] }, "invalid_display_list", "unknown command letter")
        XCTAssertEqual(code { $0["path"] = [Self.m(10, 10), ["l", 1]] }, "malformed_payload", "wrong operand count (JSONDecoder shape error; the fast reader defers to it)")
        XCTAssertEqual(code { $0["path"] = [["m", 1 << 60, 0]] }, "invalid_display_list", "coordinate outside the exact tick range")
        XCTAssertEqual(code { $0["fill_rule"] = "positive" }, "malformed_payload", "unknown fill rule")
        XCTAssertEqual(code { $0["paint"] = Self.paint(2, 0, 0) }, "invalid_display_list")
        XCTAssertEqual(code { $0["sources"] = nil }, "invalid_display_list", "provenance is required like any item")
        XCTAssertEqual(code({ setStroke(&$0, "width", 0) }, item: 2), "invalid_display_list", "zero stroke width")
        XCTAssertEqual(code({ setStroke(&$0, "cap", "flat") }, item: 2), "malformed_payload", "unknown cap")
        XCTAssertEqual(code({ setStroke(&$0, "miter_limit", 0.5) }, item: 2), "invalid_display_list", "miter limit below 1")
        XCTAssertEqual(code({ setStroke(&$0, "dash", ["array": [0, 0], "phase": 0]) }, item: 2), "invalid_display_list", "all-zero dash")
        XCTAssertEqual(code({ setStroke(&$0, "dash", ["array": [Self.ticks(1), -1], "phase": 0]) }, item: 2), "invalid_display_list", "negative dash entry")
        XCTAssertEqual(code({ $0["stroke"] = nil }, item: 2), "malformed_payload", "path_stroke needs a stroke")
        XCTAssertEqual(code({ $0["clips"] = Array(repeating: ["kind": "path", "fill_rule": "nonzero", "path": [Self.m(0, 0)]], count: 17) }, item: 4), "invalid_display_list", "too many clips")
        XCTAssertEqual(code({ $0["clips"] = [["kind": "path", "fill_rule": "nonzero", "path": [Self.z]]] }, item: 4), "invalid_display_list", "a clip closes nothing")
        XCTAssertEqual(code { $0["kind"] = "path" }, "unknown_item_kind", "bare 'path' is a clip kind, not an item kind")
        // Used features must be declared.
        for missing in ["path_fill", "path_stroke", "clip"] {
            let o = Self.list(features: ["rgba-srgb", "cluster-actualtext", "path_fill", "path_stroke", "clip"].filter { $0 != missing })
            XCTAssertThrowsError(try RenderingV2.decode(Self.data(o)), missing) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "invalid_display_list") }
        }
        XCTAssertTrue(RenderingV2.pathFeatures.isSubset(of: RenderingV2.knownFeatures))
    }

    // MARK: painting + hit-testing

    func testPathsPaintInPreviewAndCoreGraphicsExportAndHitTestToSource() throws {
        let frame = try V2Frame.prepare(data: Self.data(Self.list()), store: V2FontStore(directories: []), cache: nil, images: V2ImageStore(root: nil))
        XCTAssertEqual(frame.prepared[0].pathCount, 5)
        XCTAssertTrue(frame.prepared[0].mayContain(byte: 10, path: "main.tex"))
        let scale = 4.0
        let preview = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: scale))
        let pdf = try XCTUnwrap(CGPDFDocument(CGDataProvider(data: GlyphRunRenderer.pdfData(frame: frame) as CFData)!))
        XCTAssertEqual(pdf.numberOfPages, 1)
        let exportCtx = try XCTUnwrap(GlyphRunRenderer.bitmapContext(widthPt: Self.pageW, heightPt: Self.pageH, scale: scale))
        exportCtx.drawPDFPage(try XCTUnwrap(pdf.page(at: 1)))
        let export = try XCTUnwrap(exportCtx.makeImage())
        for (bitmap, label) in [(preview, "preview"), (export, "export")] {
            let px = { (x: Double, y: Double) in Self.pixel(bitmap, x: x, yDown: y, scale: scale) }
            // Triangle: inside dark, above the apex white, y flipped correctly (apex at top, y=70 is below the base at y=20 in y-down).
            XCTAssertTrue(Self.isDark(px(50, 35)), "\(label): triangle interior is ink, got \(px(50, 35))")
            XCTAssertTrue(Self.isWhite(px(50, 80)), "\(label): below the triangle apex is white, got \(px(50, 80))")
            XCTAssertTrue(Self.isWhite(px(50, 10)), "\(label): above the triangle base is white")
            XCTAssertTrue(Self.isWhite(px(22, 60)), "\(label): outside the slanted edge is white")
            // Circle via curves: centre red, just outside the radius white.
            XCTAssertTrue(Self.isRed(px(140, 50)), "\(label): circle centre is red, got \(px(140, 50))")
            XCTAssertTrue(Self.isRed(px(140 + 17, 50)), "\(label): near the rim (inside) is red")
            XCTAssertTrue(Self.isWhite(px(140 + 24, 50)), "\(label): outside the circle is white")
            XCTAssertTrue(Self.isWhite(px(140 + 15, 50 + 15)), "\(label): the corner a square would cover is white (it is a circle, not a box)")
            // Dashed line: 8 on / 8 off from x=20 → ink at x=24, gap at x=32, ink at x=40.
            XCTAssertTrue(Self.isDark(px(24, 120)), "\(label): first dash is ink, got \(px(24, 120))")
            XCTAssertTrue(Self.isWhite(px(32, 120)), "\(label): first gap is white, got \(px(32, 120))")
            XCTAssertTrue(Self.isDark(px(40, 120)), "\(label): second dash is ink")
            XCTAssertTrue(Self.isWhite(px(40, 125)), "\(label): 2 pt line does not reach 5 pt away")
            XCTAssertTrue(Self.isDark(px(96, 117)), "\(label): arrowhead stroke is ink, got \(px(96, 117))")
            // Clipped fill: inside the clip circle blue, inside the rectangle but outside the clip white.
            XCTAssertTrue(Self.isBlue(px(150, 140)), "\(label): clipped fill centre is blue, got \(px(150, 140))")
            XCTAssertTrue(Self.isWhite(px(115, 105)), "\(label): rectangle corner outside the clip stays white, got \(px(115, 105))")
            XCTAssertTrue(Self.isWhite(px(185, 175)), "\(label): opposite corner outside the clip stays white")
        }
        // Hit-testing: ink navigates to the tikz span; the clipped-away region and blank page do not.
        let page = frame.list.pages[0]
        let tri = try XCTUnwrap(V2Geometry.hit(page: page, atPointX: 50, y: 35))
        XCTAssertEqual(tri.itemIndex, 0); XCTAssertNil(tri.text); XCTAssertEqual(tri.sources.first?.path, "main.tex"); XCTAssertEqual(tri.sources.first?.endByte, 51)
        XCTAssertEqual(tri.rect, RenderingV2.Rect(x: Self.ticks(20), top: Self.ticks(20), width: Self.ticks(60), height: Self.ticks(50)))
        XCTAssertEqual(V2Geometry.hit(page: page, atPointX: 140, y: 50)?.itemIndex, 1, "circle centre")
        XCTAssertNil(V2Geometry.hit(page: page, atPointX: 155, y: 65), "the corner outside the circle is not a hit")
        XCTAssertEqual(V2Geometry.hit(page: page, atPointX: 24, y: 120.5)?.itemIndex, 2, "on the stroked line")
        XCTAssertNil(V2Geometry.hit(page: page, atPointX: 24, y: 130), "10 pt off the line")
        XCTAssertEqual(V2Geometry.hit(page: page, atPointX: 150, y: 140)?.itemIndex, 4, "inside the clip")
        XCTAssertNil(V2Geometry.hit(page: page, atPointX: 115, y: 105), "inside the rectangle but clipped away")
        XCTAssertNil(V2Geometry.hit(page: page, atPointX: 5, y: 195), "blank page")
        // The dark preview inverts path ink like rules: the triangle turns light.
        let dark = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: scale, dark: true))
        let inverted = Self.pixel(dark, x: 50, yDown: 35, scale: scale)
        XCTAssertTrue(inverted.0 > 200 && inverted.1 > 200 && inverted.2 > 200, "dark preview inverts the black fill, got \(inverted)")
    }

    func testDigestAndRelocationCoverPaths() throws {
        let a = try RenderingV2.decode(Self.data(Self.list())).payload.pages[0]
        var moved = Self.items()
        moved[0]["path"] = [Self.m(21, 20), Self.l(80, 20), Self.l(50, 70), Self.z]
        let b = try RenderingV2.decode(Self.data(Self.list(items: moved))).payload.pages[0]
        XCTAssertNotEqual(DisplayListDelta.pageDigest(a), DisplayListDelta.pageDigest(b), "a moved vertex changes the page digest")
        var dashless = Self.items()
        var s = dashless[2]["stroke"] as! [String: Any]; s["dash"] = nil; dashless[2]["stroke"] = s
        let c = try RenderingV2.decode(Self.data(Self.list(items: dashless))).payload.pages[0]
        XCTAssertNotEqual(DisplayListDelta.pageDigest(a), DisplayListDelta.pageDigest(c), "dropping the dash changes the digest")
        XCTAssertEqual(DisplayListDelta.pageDigest(a), DisplayListDelta.pageDigest(try RenderingV2.decode(Self.data(Self.list())).payload.pages[0]), "stable")
        // Relocation past an edit before the span moves every path's sources.
        let reloc = DisplayListDelta.Relocation(path: "main.tex", editStart: 0, editEnd: 0, delta: 7)
        let shifted = try XCTUnwrap(DisplayListDelta.relocatePage(a, [reloc]))
        for item in shifted.items { guard case .path(let p) = item else { return XCTFail() }; XCTAssertEqual(p.sources?.first?.startByte, 7); XCTAssertEqual(p.sources?.first?.endByte, 58) }
        // An edit inside the span refuses relocation (a full list must follow).
        XCTAssertNil(DisplayListDelta.relocatePage(a, [DisplayListDelta.Relocation(path: "main.tex", editStart: 10, editEnd: 12, delta: 1)]))
    }

    // MARK: live producer

    /// `FLASHTEX_RENDER`: a `tikzpicture` through the real producer yields
    /// `path_fill` + `path_stroke` items that decode, prepare and paint (the
    /// red disc's centre is red on the preview bitmap and the export).
    func testRealProducerTikzPictureArrivesAsPathsAndPaints() throws {
        guard let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"], FileManager.default.isExecutableFile(atPath: render) else {
            throw XCTSkip("FLASHTEX_RENDER not set to a built flashtex-render")
        }
        let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Fonts")
        let tex = "\\documentclass{article}\n\\usepackage{tikz}\n\\begin{document}\nHi.\n\\begin{tikzpicture}\n\\draw (0,0) -- (2,1);\n\\fill[red] (1,1) circle (0.3);\n\\end{tikzpicture}\n\\end{document}\n"
        let payload: [String: Any] = ["project_id": "p", "revision": 1, "entry_path": "main.tex",
                                      "documents": [["path": "main.tex", "text": tex]], "layout_capabilities": ["display-list-v2"]]
        let request = try JSONSerialization.data(withJSONObject: ["protocol_version": 1, "id": "tikz", "type": "compile", "payload": payload])
        let home = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-paths-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: home) }
        let process = Process()
        process.executableURL = URL(fileURLWithPath: render)
        process.environment = ["PATH": "/usr/bin:/bin", "HOME": home.path, "FLASHTEX_FONT_DIRS": fontsDir.path,
                               "FLASHTEX_TFM_DIRS": fontsDir.appendingPathComponent("texmf/fonts/tfm/public/lm").path]
        process.currentDirectoryURL = home
        let stdin = Pipe(), stdout = Pipe()
        process.standardInput = stdin; process.standardOutput = stdout; process.standardError = FileHandle.nullDevice
        try process.run()
        try stdin.fileHandleForWriting.write(contentsOf: request + Data("\n".utf8))
        try stdin.fileHandleForWriting.close()
        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        let line = try XCTUnwrap(output.split(separator: UInt8(ascii: "\n")).first { $0.starts(with: Data("{\"id\":\"tikz\",\"payload\":".utf8)) && String(decoding: $0, as: UTF8.self).contains("\"type\":\"display_list\"") },
                                 "the producer emitted a display_list line")
        let envelope = try RenderingV2.decode(Data(line))
        let features = envelope.payload.requiredFeatures
        XCTAssertTrue(features.contains("path_fill") && features.contains("path_stroke"), "required_features \(features)")
        let paths = envelope.payload.pages.flatMap(\.items).compactMap { item -> RenderingV2.Path? in if case .path(let p) = item { return p } else { return nil } }
        XCTAssertGreaterThanOrEqual(paths.count, 2, "one stroke and one fill")
        let fill = try XCTUnwrap(paths.first { $0.isFill }), stroke = try XCTUnwrap(paths.first { $0.stroke != nil })
        XCTAssertEqual(fill.paint, RenderingV2.Paint(r: 1, g: 0, b: 0, a: 1))
        XCTAssertTrue(fill.path.contains { if case .cubic = $0 { return true } else { return false } }, "the circle is curves")
        XCTAssertEqual(stroke.path.count, 2, "a straight \\draw is move + line")
        XCTAssertEqual(fill.sources?.first?.path, "main.tex")
        let spanText = String(decoding: Array(tex.utf8)[(fill.sources?.first?.startByte ?? 0)..<(fill.sources?.first?.endByte ?? 0)], as: UTF8.self)
        XCTAssertTrue(spanText.contains("tikzpicture"), "the path's source span is the picture: \(spanText)")
        // Paint: the disc centre (its bounding-box centre) is red in the preview raster and the export.
        let frame = try V2Frame.prepare(envelope, store: V2FontStore(directories: [fontsDir.path]), images: V2ImageStore(root: nil))
        let pageIndex = try XCTUnwrap(envelope.payload.pages.firstIndex { $0.items.contains { if case .path = $0 { return true } else { return false } } })
        let prepared = frame.prepared[pageIndex]
        let coords = fill.path.flatMap(\.coordinates)
        let xs = stride(from: 0, to: coords.count, by: 2).map { RenderingV2.points(coords[$0]) }, ys = stride(from: 1, to: coords.count, by: 2).map { RenderingV2.points(coords[$0]) }
        let cx = (xs.min()! + xs.max()!) / 2, cy = (ys.min()! + ys.max()!) / 2
        let scale = 2.0
        let preview = try XCTUnwrap(GlyphRunRenderer.rasterize(prepared, scale: scale))
        XCTAssertTrue(Self.isRed(Self.pixel(preview, x: cx, yDown: cy, scale: scale, pageH: prepared.heightPt)), "preview disc centre is red")
        let pdf = try XCTUnwrap(CGPDFDocument(CGDataProvider(data: GlyphRunRenderer.pdfData(frame: frame) as CFData)!))
        let ctx = try XCTUnwrap(GlyphRunRenderer.bitmapContext(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale))
        ctx.drawPDFPage(try XCTUnwrap(pdf.page(at: pageIndex + 1)))
        XCTAssertTrue(Self.isRed(Self.pixel(try XCTUnwrap(ctx.makeImage()), x: cx, yDown: cy, scale: scale, pageH: prepared.heightPt)), "export disc centre is red")
        XCTAssertEqual(V2Geometry.hit(page: envelope.payload.pages[pageIndex], atPointX: cx, y: cy)?.sources.first?.path, "main.tex")
    }
}
