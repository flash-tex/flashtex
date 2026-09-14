import CoreGraphics
import CryptoKit
import ImageIO
import UniformTypeIdentifiers
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// `display-list-v2-images` consumer (protocol/proposals/display-list-v2-image.md):
/// image items decode and validate like the other items, bytes are read
/// under the project root and verified by SHA-256/length before painting,
/// PNG/JPEG/PDF paint through the one draw routine (preview bitmap and the
/// CoreGraphics PDF export), and every refusal keeps the frame and surfaces
/// one non-modal notice. The fixtures are generated here: nothing binary is
/// committed.
final class V2ImageTests: XCTestCase {
    struct Asset { var path: String; var data: Data; var sha256: String { V2FontStore.hex(SHA256.hash(data: data)) } }

    static func png(width: Int, height: Int, color: CGColor) -> Data { raster(width: width, height: height, color: color, type: UTType.png) }
    static func jpeg(width: Int, height: Int, color: CGColor) -> Data { raster(width: width, height: height, color: color, type: UTType.jpeg) }

    private static func raster(width: Int, height: Int, color: CGColor, type: UTType) -> Data {
        let ctx = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.setFillColor(color)
        ctx.fill(CGRect(x: 0, y: 0, width: width, height: height))
        let image = ctx.makeImage()!
        let out = NSMutableData()
        let dest = CGImageDestinationCreateWithData(out, type.identifier as CFString, 1, nil)!
        CGImageDestinationAddImage(dest, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(dest))
        return out as Data
    }

    /// One-page PDF, MediaBox `width`×`height` pt, filled with `color`
    /// except a white 10-pt band at the top (so rotation is observable).
    static func pdf(width: Double, height: Double, color: CGColor) -> Data {
        let data = NSMutableData()
        var box = CGRect(x: 0, y: 0, width: width, height: height)
        let ctx = CGContext(consumer: CGDataConsumer(data: data)!, mediaBox: &box, nil)!
        ctx.beginPDFPage(nil)
        ctx.setFillColor(color)
        ctx.fill(CGRect(x: 0, y: 0, width: width, height: height - 10))
        ctx.endPDFPage()
        ctx.closePDF()
        return data as Data
    }

    /// A project directory with `figures/{a.png,b.jpg,c.pdf}` and the display
    /// list that places them (one page, 200×300 pt, boxes at 20/120/220 pt).
    struct Fixture {
        var root: URL
        var assets: [Asset]
        var list: [String: Any]
        static let pageW = 200.0, pageH = 300.0
        /// Item boxes in points (x, top, w, h): PNG, JPEG, PDF in item order.
        static let boxes: [(Double, Double, Double, Double)] = [(20, 20, 60, 40), (20, 120, 60, 40), (20, 220, 100, 50)]
    }

    static func ticks(_ pt: Double) -> Int64 { Int64((pt * Double(RenderingV2.ticksPerPoint)).rounded()) }

    static func makeFixture(_ name: String) throws -> Fixture {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("v2img-\(name)-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("figures"), withIntermediateDirectories: true)
        let assets = [
            Asset(path: "figures/a.png", data: png(width: 6, height: 4, color: CGColor(srgbRed: 1, green: 0, blue: 0, alpha: 1))),
            Asset(path: "figures/b.jpg", data: jpeg(width: 6, height: 4, color: CGColor(srgbRed: 0, green: 0, blue: 1, alpha: 1))),
            Asset(path: "figures/c.pdf", data: pdf(width: 100, height: 50, color: CGColor(srgbRed: 0, green: 0.5, blue: 0, alpha: 1))),
        ]
        for a in assets { try a.data.write(to: root.appendingPathComponent(a.path)) }
        let tex = "\\includegraphics{figures/a}\n"
        try tex.write(to: root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        func item(_ i: Int, _ image: [String: Any]) -> [String: Any] {
            let (x, top, w, h) = Fixture.boxes[i]
            return ["kind": "image", "x": ticks(x), "top": ticks(top), "width": ticks(w), "height": ticks(h),
                    "transform": RenderingV2.Image.upright(x: x, top: top, width: w, height: h),
                    "image": image, "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 27]]]
        }
        let list: [String: Any] = [
            "protocol_version": 2, "id": "img-1", "type": "display_list",
            "payload": [
                "render_format": "display-list-v2", "coordinate_unit": "bp_2pow20", "color_space": "srgb", "text_extraction": "cluster-actualtext",
                "project_id": "p", "revision": 1, "required_features": ["image", "rgba-srgb", "cluster-actualtext"],
                "documents": [["path": "main.tex", "revision": 1, "sha256": SourceDigest.sha256Hex(tex), "byte_length": tex.utf8.count]],
                "fonts": [],
                "pages": [["number": 1, "width": ticks(Fixture.pageW), "height": ticks(Fixture.pageH), "items": [
                    item(0, ["image_id": assets[0].sha256, "sha256": assets[0].sha256, "byte_length": assets[0].data.count, "format": "png",
                             "path": "figures/a.png", "pixel_width": 6, "pixel_height": 4, "vendor_extra": "tolerated"]),
                    item(1, ["image_id": assets[1].sha256, "sha256": assets[1].sha256, "byte_length": assets[1].data.count, "format": "jpeg",
                             "path": "figures/b.jpg", "pixel_width": 6, "pixel_height": 4]),
                    item(2, ["image_id": assets[2].sha256, "sha256": assets[2].sha256, "byte_length": assets[2].data.count, "format": "pdf",
                             "path": "figures/c.pdf", "pdf_page": 1, "pdf_box": [0, 0, 100, 50], "pdf_rotate": 0]),
                ]]],
                "diagnostics": [],
            ],
        ]
        return Fixture(root: root, assets: assets, list: list)
    }

    static func data(_ o: [String: Any]) -> Data { try! JSONSerialization.data(withJSONObject: o) }

    /// RGBA of the pixel at page point (x, y down) in a bitmap at `scale`.
    static func pixel(_ image: CGImage, x: Double, yDown: Double, scale: Double) -> (UInt8, UInt8, UInt8) {
        let rgba = V2Parity.rgba(image)
        let px = Int(x * scale), py = Int((Fixture.pageH - yDown) * scale)
        let i = ((image.height - 1 - py) * image.width + px) * 4
        return (rgba[i], rgba[i + 1], rgba[i + 2])
    }
    static func isWhite(_ p: (UInt8, UInt8, UInt8)) -> Bool { p.0 > 250 && p.1 > 250 && p.2 > 250 }

    // MARK: decode + validation

    func testImageItemsDecodeOnBothReadersWithUnknownFieldsTolerated() throws {
        let f = try Self.makeFixture("decode")
        defer { try? FileManager.default.removeItem(at: f.root) }
        let bytes = Self.data(f.list)
        let fast = try RenderingV2Fast.envelope(bytes)
        let slow = try RenderingV2.decode(bytes)
        XCTAssertEqual(fast, slow, "the fast reader and JSONDecoder agree on image items")
        guard case .image(let png) = slow.payload.pages[0].items[0], case .image(let pdf) = slow.payload.pages[0].items[2] else { return XCTFail() }
        XCTAssertEqual(png.image.format, "png"); XCTAssertEqual(png.image.pixelWidth, 6); XCTAssertNil(png.image.pdfBox)
        XCTAssertEqual(png.transform, [60, 0, 0, -40, 20, 60])
        XCTAssertEqual(pdf.image.pdfPage, 1); XCTAssertEqual(pdf.image.pdfBox, [0, 0, 100, 50]); XCTAssertEqual(pdf.image.pdfRotate, 0)
        XCTAssertEqual(pdf.sources?.first?.path, "main.tex")
        XCTAssertEqual(try RenderingV2.decode(try JSONEncoder().encode(slow)), slow, "round-trips through the encoder")
    }

    func testImageResourceValidationRefusesMalformedShapes() throws {
        let f = try Self.makeFixture("validate")
        defer { try? FileManager.default.removeItem(at: f.root) }
        func code(_ edit: (inout [String: Any]) -> Void, item: Int = 0) -> String? {
            var o = f.list
            var p = o["payload"] as! [String: Any]; var pages = p["pages"] as! [[String: Any]]; var items = pages[0]["items"] as! [[String: Any]]
            edit(&items[item]); pages[0]["items"] = items; p["pages"] = pages; o["payload"] = p
            do { _ = try RenderingV2.decode(Self.data(o)); return nil } catch let e as RenderingV2.ValidationError { return e.code } catch { return "other" }
        }
        func setImage(_ i: inout [String: Any], _ k: String, _ v: Any?) { var im = i["image"] as! [String: Any]; im[k] = v; i["image"] = im }
        XCTAssertNil(code { _ in })
        XCTAssertEqual(code { setImage(&$0, "image_id", String(repeating: "a", count: 64)) }, "invalid_resource", "image_id must equal sha256")
        XCTAssertEqual(code { setImage(&$0, "sha256", "abc") }, "invalid_resource")
        XCTAssertEqual(code { setImage(&$0, "format", "gif") }, "unsupported_feature")
        XCTAssertEqual(code { setImage(&$0, "path", "../figures/a.png") }, "invalid_resource")
        XCTAssertEqual(code { setImage(&$0, "byte_length", 0) }, "invalid_resource")
        XCTAssertEqual(code { setImage(&$0, "pixel_width", nil) }, "invalid_resource", "raster formats need pixel dimensions")
        XCTAssertEqual(code({ setImage(&$0, "pdf_box", [0, 0, 0, 50]) }, item: 2), "invalid_resource", "empty pdf_box")
        XCTAssertEqual(code({ setImage(&$0, "pdf_rotate", 45) }, item: 2), "invalid_resource")
        XCTAssertEqual(code({ setImage(&$0, "pdf_page", 0) }, item: 2), "invalid_resource")
        XCTAssertEqual(code { $0["transform"] = [1, 2, 3] }, "invalid_display_list", "transform needs six numbers")
        XCTAssertEqual(code { $0["width"] = 0 }, "invalid_display_list")
        XCTAssertEqual(code { $0["sources"] = nil }, "invalid_display_list", "provenance is required like any item")
        // The feature must be declared: a list using images without `image` in required_features is refused.
        var o = f.list; var p = o["payload"] as! [String: Any]; p["required_features"] = ["rgba-srgb", "cluster-actualtext"]; o["payload"] = p
        XCTAssertThrowsError(try RenderingV2.decode(Self.data(o))) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "invalid_display_list") }
    }

    // MARK: painting

    func testPNGJPEGAndPDFPaintInsideTheirBoxesInPreviewAndCoreGraphicsExport() throws {
        let f = try Self.makeFixture("paint")
        defer { try? FileManager.default.removeItem(at: f.root) }
        let images = V2ImageStore(root: f.root)
        let frame = try V2Frame.prepare(data: Self.data(f.list), store: V2FontStore(directories: []), cache: nil, images: images)
        XCTAssertEqual(frame.imageNotices, [])
        XCTAssertEqual(frame.prepared[0].imageCount, 3)
        XCTAssertEqual(images.cachedCount, 3)
        let scale = 2.0
        let preview = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: scale))
        // Re-rasterize the CoreGraphics PDF export through the same bitmap configuration.
        let pdf = try XCTUnwrap(CGPDFDocument(CGDataProvider(data: GlyphRunRenderer.pdfData(frame: frame) as CFData)!))
        XCTAssertEqual(pdf.numberOfPages, 1)
        let exportCtx = try XCTUnwrap(GlyphRunRenderer.bitmapContext(widthPt: Fixture.pageW, heightPt: Fixture.pageH, scale: scale))
        exportCtx.drawPDFPage(try XCTUnwrap(pdf.page(at: 1)))
        let export = try XCTUnwrap(exportCtx.makeImage())
        for (bitmap, label) in [(preview, "preview"), (export, "export")] {
            let (ax, at, aw, ah) = Fixture.boxes[0]
            let a = Self.pixel(bitmap, x: ax + aw / 2, yDown: at + ah / 2, scale: scale)
            XCTAssertTrue(a.0 > 200 && a.1 < 60 && a.2 < 60, "\(label): PNG box centre is red, got \(a)")
            let (bx, bt, bw, bh) = Fixture.boxes[1]
            let b = Self.pixel(bitmap, x: bx + bw / 2, yDown: bt + bh / 2, scale: scale)
            XCTAssertTrue(b.2 > 200 && b.0 < 60 && b.1 < 60, "\(label): JPEG box centre is blue, got \(b)")
            let (cx, ct, cw, ch) = Fixture.boxes[2]
            let c = Self.pixel(bitmap, x: cx + cw / 2, yDown: ct + ch / 2, scale: scale)
            XCTAssertTrue(c.1 > 100 && c.0 < 60 && c.2 < 60, "\(label): PDF box centre is green, got \(c)")
            // The PDF's white top band (10 of 50 pt) maps to the top 10 pt of the box: unit square v up.
            XCTAssertTrue(Self.isWhite(Self.pixel(bitmap, x: cx + cw / 2, yDown: ct + 4, scale: scale)), "\(label): PDF top band stays white")
            XCTAssertTrue(Self.isWhite(Self.pixel(bitmap, x: ax + aw + 5, yDown: at + ah / 2, scale: scale)), "\(label): outside the boxes is page white")
            XCTAssertTrue(Self.isWhite(Self.pixel(bitmap, x: ax + aw / 2, yDown: at - 5, scale: scale)), "\(label): above the PNG box is white")
        }
        // Hit-testing the box navigates to the \includegraphics source; text stays nil.
        let hit = try XCTUnwrap(V2Geometry.hit(page: frame.list.pages[0], atPointX: 50, y: 40))
        XCTAssertEqual(hit.itemIndex, 0); XCTAssertNil(hit.text); XCTAssertEqual(hit.sources.first?.path, "main.tex")
    }

    func testRotatedPDFBoxMapsThroughTheProducerMatrix() throws {
        // Page box 100×50 with /Rotate 90 displayed as 50 wide × 100 tall: the
        // page's white top band (y in 40...50, page space) lands on the RIGHT
        // edge of the displayed image (clockwise quarter turn).
        let f = try Self.makeFixture("rotate")
        defer { try? FileManager.default.removeItem(at: f.root) }
        var o = f.list
        var p = o["payload"] as! [String: Any]; var pages = p["pages"] as! [[String: Any]]; var items = pages[0]["items"] as! [[String: Any]]
        var im = items[2]["image"] as! [String: Any]; im["pdf_rotate"] = 90; items[2]["image"] = im
        let (x, top) = (20.0, 100.0), w = 50.0, h = 100.0
        items[2]["x"] = Self.ticks(x); items[2]["top"] = Self.ticks(top); items[2]["width"] = Self.ticks(w); items[2]["height"] = Self.ticks(h)
        items[2]["transform"] = RenderingV2.Image.upright(x: x, top: top, width: w, height: h)
        pages[0]["items"] = [items[2]]; p["pages"] = pages; o["payload"] = p
        let frame = try V2Frame.prepare(data: Self.data(o), store: V2FontStore(directories: []), cache: nil, images: V2ImageStore(root: f.root))
        let bitmap = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: 2))
        XCTAssertTrue(Self.isWhite(Self.pixel(bitmap, x: x + w - 4, yDown: top + h / 2, scale: 2)), "right 10 pt strip is the page's top band")
        let left = Self.pixel(bitmap, x: x + 4, yDown: top + h / 2, scale: 2)
        XCTAssertTrue(left.1 > 100 && left.0 < 60, "left edge is the page's green body, got \(left)")
        let mid = Self.pixel(bitmap, x: x + w / 2, yDown: top + h / 2, scale: 2)
        XCTAssertTrue(mid.1 > 100 && mid.0 < 60, "centre is green, got \(mid)")
        XCTAssertTrue(Self.isWhite(Self.pixel(bitmap, x: x + w / 2, yDown: top - 3, scale: 2)), "nothing above the box")
    }

    // MARK: refusals

    func testStaleBytesRefuseOnlyThatImageAndSurfaceOneNotice() throws {
        let f = try Self.makeFixture("stale")
        defer { try? FileManager.default.removeItem(at: f.root) }
        // The PNG on disk changed after the producer sized it (same length, other bytes; then another length).
        var edited = f.assets[0].data; edited[edited.count - 1] ^= 0xFF
        try edited.write(to: f.root.appendingPathComponent("figures/a.png"))
        let images = V2ImageStore(root: f.root)
        let frame = try V2Frame.prepare(data: Self.data(f.list), store: V2FontStore(directories: []), cache: nil, images: images)
        XCTAssertEqual(frame.imageNotices, ["stale image: figures/a.png"], "one non-modal notice; the frame is kept")
        XCTAssertEqual(frame.prepared[0].imageCount, 2)
        let bitmap = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: 2))
        let (ax, at, aw, ah) = Fixture.boxes[0]
        XCTAssertTrue(Self.isWhite(Self.pixel(bitmap, x: ax + aw / 2, yDown: at + ah / 2, scale: 2)), "the stale image painted nothing")
        let (bx, bt, bw, bh) = Fixture.boxes[1]
        XCTAssertFalse(Self.isWhite(Self.pixel(bitmap, x: bx + bw / 2, yDown: bt + bh / 2, scale: 2)), "the verified JPEG still paints")
        try (f.assets[0].data + Data([0])).write(to: f.root.appendingPathComponent("figures/a.png"))
        let again = try V2Frame.prepare(data: Self.data(f.list), store: V2FontStore(directories: []), cache: nil, images: images)
        XCTAssertEqual(again.imageNotices, ["stale image: figures/a.png"], "a length mismatch is refused before hashing")
        XCTAssertEqual(images.cachedCount, 2, "nothing stale is ever cached")
        // Without a project root every image is refused, each with its own notice.
        let noRoot = try V2Frame.prepare(data: Self.data(f.list), store: V2FontStore(directories: []), cache: nil, images: V2ImageStore())
        XCTAssertEqual(noRoot.imageNotices.count, 3)
        XCTAssertTrue(noRoot.imageNotices[0].hasPrefix("image unavailable: figures/a.png: no project root"), noRoot.imageNotices[0])
    }

    func testSymlinkedImagePathIsRefusedEvenWhenTheTargetBytesMatch() throws {
        let f = try Self.makeFixture("symlink")
        defer { try? FileManager.default.removeItem(at: f.root) }
        // figures/a.png becomes a symlink to a copy with the exact same bytes outside the project.
        let outside = f.root.deletingLastPathComponent().appendingPathComponent("v2img-outside-\(UUID().uuidString).png")
        try f.assets[0].data.write(to: outside)
        defer { try? FileManager.default.removeItem(at: outside) }
        let link = f.root.appendingPathComponent("figures/a.png")
        try FileManager.default.removeItem(at: link)
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: outside)
        let frame = try V2Frame.prepare(data: Self.data(f.list), store: V2FontStore(directories: []), cache: nil, images: V2ImageStore(root: f.root))
        XCTAssertEqual(frame.imageNotices, ["image unavailable: figures/a.png: a.png is a symbolic link"])
        XCTAssertEqual(frame.prepared[0].imageCount, 2)
        // A symlinked directory component is refused the same way.
        let f2 = try Self.makeFixture("symlink-dir")
        defer { try? FileManager.default.removeItem(at: f2.root) }
        let real = f2.root.appendingPathComponent("figures")
        let moved = f2.root.appendingPathComponent("real-figures")
        try FileManager.default.moveItem(at: real, to: moved)
        try FileManager.default.createSymbolicLink(at: real, withDestinationURL: moved)
        let frame2 = try V2Frame.prepare(data: Self.data(f2.list), store: V2FontStore(directories: []), cache: nil, images: V2ImageStore(root: f2.root))
        XCTAssertEqual(frame2.imageNotices, ["image unavailable: figures/a.png: figures is a symbolic link",
                                             "image unavailable: figures/b.jpg: figures is a symbolic link",
                                             "image unavailable: figures/c.pdf: figures is a symbolic link"])
        XCTAssertEqual(frame2.prepared[0].imageCount, 0)
    }

    func testPageCacheKeysOnTheImageRootSoARefusalNeverLendsItsPage() throws {
        let f = try Self.makeFixture("cache")
        defer { try? FileManager.default.removeItem(at: f.root) }
        let cache = V2PageCache()
        let store = V2FontStore(directories: [])
        let images = V2ImageStore()
        let refused = try V2Frame.prepare(data: Self.data(f.list), store: store, cache: cache, images: images)
        XCTAssertEqual(refused.prepared[0].imageCount, 0)
        images.root = f.root
        let painted = try V2Frame.prepare(data: Self.data(f.list), store: store, cache: cache, images: images)
        XCTAssertEqual(painted.prepared[0].imageCount, 3, "a page prepared under another root is not reused")
        XCTAssertEqual(painted.reusedPages, 0)
        let again = try V2Frame.prepare(data: Self.data(f.list), store: store, cache: cache, images: images)
        XCTAssertEqual(again.reusedPages, 1, "same bytes, same root: reused")
        XCTAssertEqual(again.prepared[0].imageCount, 3)
    }

    // MARK: request wiring

    @MainActor
    func testPaneRequestsImagesAlongsideV2AndTheCompileRequestCarriesProjectRoot() throws {
        let model = ShellModel()
        model.setLiveV2(true)
        XCTAssertEqual(model.requestedLayoutCapabilities.suffix(3), [V2Live.capability, RenderingV2.imagesCapability, RenderingV2.diagnosticsCapability])
        model.setLiveV2(false)
        XCTAssertFalse(model.requestedLayoutCapabilities.contains(RenderingV2.imagesCapability))
        XCTAssertFalse(model.requestedLayoutCapabilities.contains(RenderingV2.diagnosticsCapability))
        let req = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "main.tex", documents: [.init(path: "main.tex", text: "x")],
                                           layoutCapabilities: ["display-list-v2", RenderingV2.imagesCapability], projectRoot: "/tmp/proj")
        let json = try JSONSerialization.jsonObject(with: try JSONEncoder().encode(req)) as! [String: Any]
        XCTAssertEqual(json["project_root"] as? String, "/tmp/proj")
        XCTAssertEqual(json["layout_capabilities"] as? [String], ["display-list-v2", "display-list-v2-images"])
        let without = try JSONSerialization.jsonObject(with: try JSONEncoder().encode(RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "main.tex", documents: []))) as! [String: Any]
        XCTAssertNil(without["project_root"], "omitted from the wire when nil (frozen runtime-v1 shape unchanged)")
        XCTAssertEqual(try JSONDecoder().decode(RuntimeV1.CompileRequest.self, from: try JSONEncoder().encode(req)).projectRoot, "/tmp/proj")
    }

    // MARK: live producer (FLASHTEX_RENDER)

    private func waitUntil(timeout: TimeInterval = 30, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout waiting for the producer") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    /// The real producer with `project_root`: `\includegraphics` in a figure
    /// float arrives as an image item, is read back under the project root,
    /// and paints non-white pixels inside its box.
    @MainActor
    func testRealProducerImageItemArrivesAndPaints() async throws {
        guard let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"], FileManager.default.isExecutableFile(atPath: render) else {
            throw XCTSkip("FLASHTEX_RENDER not set to a built flashtex-render")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("v2img-live-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("figures"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let pngData = Self.png(width: 72, height: 36, color: CGColor(srgbRed: 0.8, green: 0, blue: 0, alpha: 1))
        try pngData.write(to: root.appendingPathComponent("figures/red.png"))
        let tex = """
        \\documentclass{article}
        \\usepackage{graphicx}
        \\begin{document}
        Text before the figure.
        \\begin{figure}[h]
        \\centering
        \\includegraphics[width=2in]{figures/red}
        \\caption{A red box.}
        \\end{figure}
        \\end{document}

        """
        let texURL = root.appendingPathComponent("main.tex")
        try tex.write(to: texURL, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.autoCompile = false
        XCTAssertEqual(model.openTex(at: texURL), .opened)
        XCTAssertEqual(model.project.projectRoot?.standardizedFileURL.path, root.standardizedFileURL.path)
        model.attachWorker(at: URL(fileURLWithPath: render))
        model.previewV2 = true
        model.setLiveV2(true)
        model.compile()
        try await waitUntil { model.inFlightRevision == nil && model.displayListV2?.isLoading == false && model.displayListV2 != nil }
        defer { model.detachWorker() }
        XCTAssertTrue(model.acceptedLayoutCapabilities.contains(RenderingV2.imagesCapability), "accepted: \(model.acceptedLayoutCapabilities)")
        guard case .loaded(let frame, _)? = model.displayListV2 else { return XCTFail("\(String(describing: model.displayListV2)) diagnostics: \(model.result?.diagnostics ?? [])") }
        XCTAssertTrue(frame.list.requiredFeatures.contains("image"))
        let items = frame.list.pages.flatMap(\.items).compactMap { if case .image(let i) = $0 { i } else { nil } }
        XCTAssertEqual(items.count, 1, "one image item; diagnostics: \(frame.list.diagnostics)")
        let item = try XCTUnwrap(items.first)
        XCTAssertEqual(item.image.path, "figures/red.png")
        XCTAssertEqual(item.image.sha256, V2FontStore.hex(SHA256.hash(data: pngData)))
        XCTAssertEqual(item.image.byteLength, Int64(pngData.count))
        XCTAssertEqual(RenderingV2.points(item.width), 144, accuracy: 0.01, "width=2in")
        XCTAssertEqual(frame.imageNotices, [])
        XCTAssertEqual(frame.prepared[0].imageCount, 1)
        let bitmap = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: 1))
        let cx = RenderingV2.points(item.x) + RenderingV2.points(item.width) / 2, cy = RenderingV2.points(item.top) + RenderingV2.points(item.height) / 2
        let rgba = V2Parity.rgba(bitmap)
        let px = Int(cx), py = bitmap.height - 1 - Int(frame.prepared[0].heightPt - cy)
        let i = (py * bitmap.width + px) * 4
        XCTAssertTrue(rgba[i] > 150 && rgba[i + 1] < 80 && rgba[i + 2] < 80, "box centre is red, got \(rgba[i]), \(rgba[i + 1]), \(rgba[i + 2])")
    }
}
