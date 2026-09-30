import CoreGraphics
import Foundation
import IOSurface
import PDFKit
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// High-zoom tiles are pixel-exact windows of the whole-page raster, which
/// the parity gate compares with Core Graphics' rendering of the engine's
/// PDF (DESIGN.md §6.2; DL3Tiles.swift). Zero tolerance, every tile.
///
/// * Always: the checked-in fixtures over the tiled range 3.25–8 px/pt in
///   0.25 steps: `beamer-overlays` (glyphs: drawn tile by tile),
///   `tile-text` (text and stroked table rules) and `tile-paths` (paths and
///   forms), both cut from one raster. `tile-text` and `tile-paths` are the parity
///   fixtures `divergence-probes/min-tabular` and `real-world/beamer-madrid`
///   as `tools/displaylist/check_positions.py` leaves them.
/// * Every parity fixture in `target/dl3-positions/` (or
///   `FLASHTEX_DL3_FIXTURES`) when `FLASHTEX_V3_TILE_SWEEP=1`; also against
///   the engine's PDF rendered by Core Graphics and by PDFKit at the same
///   scales. `FLASHTEX_V3_TILE_SWEEP_OUT=path.json` writes the report.
final class TileParityTests: XCTestCase {
    static let scales: [Double] = stride(from: 3.25, through: 8, by: 0.25).map { $0 }
    static let tile = 512

    static var fixtures: URL {
        PreviewParityTests.repoRoot.appendingPathComponent("apps/mac/Tests/FlashTeXDisplayListV3Tests/Fixtures")
    }

    static func rects(width: Int, height: Int) -> [DL3PixelRect] {
        var out: [DL3PixelRect] = []
        for y in stride(from: 0, to: height, by: tile) {
            for x in stride(from: 0, to: width, by: tile) {
                out.append(DL3PixelRect(x: x, y: y, width: min(tile, width - x), height: min(tile, height - y)))
            }
        }
        return out
    }

    /// RGBA bytes of `rect` of an RGBA page raster.
    static func window(_ page: [UInt8], pageWidth: Int, _ r: DL3PixelRect) -> [UInt8] {
        var out = [UInt8](); out.reserveCapacity(r.width * r.height * 4)
        for row in r.y ..< r.y + r.height {
            let start = (row * pageWidth + r.x) * 4
            out.append(contentsOf: page[start ..< start + r.width * 4])
        }
        return out
    }

    struct Result: Codable {
        var fixture: String, page: Int, scale: Double
        var translated: Bool
        var tiles: Int, differingTiles: Int, differingPixels: Int
        /// The tiled page against the engine's PDF (Core Graphics), and
        /// Core Graphics' own PDF rendering against PDFKit's vs. the tiles'.
        var pdfPixels: Int?
        var pdfKitPixels: Int?, pdfKitBaseline: Int?
    }

    /// Every tile of every page at every tiled scale against the whole page;
    /// with `pdf`, the reassembled tiles against the PDF too.
    func sweep(_ name: String, doc: DL3Document, pdf: URL?, scales: [Double] = scales, surfaces: Bool) throws -> [Result] {
        let cg = pdf.flatMap { CGPDFDocument($0 as CFURL) }
        let kit = pdf.flatMap { PDFDocument(url: $0) }
        var out: [Result] = []
        for page in doc.orderedPages {
            let translated = DL3Renderer.tilesByTranslation(page)
            for scale in scales { try autoreleasepool {  // page-sized bitmaps: freed per step
                let whole = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale))
                let pixels = DL3Parity.rgba(whole)
                let rects = Self.rects(width: whole.width, height: whole.height)
                var differing = 0, px = 0
                // The reassembled page, only when it is compared with the PDF.
                var assembled = [UInt8](repeating: 0, count: pdf == nil ? 0 : pixels.count)
                // IOSurfaces as the pane gets them: all at once, or (cut pages)
                // one tile row per call, as the pane's jobs ask for part of a
                // page, so the clipped page raster is checked per row.
                var fromSurfaces: [IOSurface?] = []
                if surfaces { fromSurfaces = DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects) }
                else if !translated {
                    for y in Set(rects.map(\.y)).sorted() {
                        fromSurfaces += DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects.filter { $0.y == y })
                    }
                }
                for (k, r) in rects.enumerated() {
                    let image: CGImage
                    if surfaces || !translated {
                        let s = try XCTUnwrap(fromSurfaces[k], "\(name) p\(page.page.index + 1) \(scale) tile \(r)")
                        image = try XCTUnwrap(DL3Renderer.image(of: s))
                    } else {
                        image = try XCTUnwrap(DL3Renderer.rasterizeTile(page, forms: doc.forms, scale: scale, rect: r))
                    }
                    XCTAssertEqual(image.width, r.width); XCTAssertEqual(image.height, r.height)
                    let t = DL3Parity.rgba(image)
                    let d = DL3Parity.diff(t, Self.window(pixels, pageWidth: whole.width, r))
                    if d.pixels > 0 { differing += 1; px += d.pixels }
                    if assembled.isEmpty { continue }
                    for row in 0 ..< r.height {
                        let dst = ((r.y + row) * whole.width + r.x) * 4
                        assembled.replaceSubrange(dst ..< dst + r.width * 4, with: t[row * r.width * 4 ..< (row + 1) * r.width * 4])
                    }
                }
                var result = Result(fixture: name, page: Int(page.page.index), scale: scale, translated: translated,
                                    tiles: rects.count, differingTiles: differing, differingPixels: px)
                if let cg, let p = cg.page(at: Int(page.page.index) + 1), !page.needsPDFFallback(forms: doc.forms),
                   let ref = DL3Renderer.rasterize(pdfPage: p, scale: scale) {
                    result.pdfPixels = DL3Parity.diff(assembled, DL3Parity.rgba(ref)).pixels
                    if let kit, let k = PreviewParityTests.pdfKitRender(kit, Int(page.page.index), scale) {
                        let kr = DL3Parity.rgba(k)
                        result.pdfKitPixels = DL3Parity.diff(assembled, kr).pixels
                        result.pdfKitBaseline = DL3Parity.diff(DL3Parity.rgba(ref), kr).pixels
                    }
                }
                out.append(result)
            } }
        }
        return out
    }

    func load(_ dl3: URL) throws -> DL3Document { try DL3Document(frames: Array(try Data(contentsOf: dl3))) }

    func testTilesAreExactWindowsOfTheCheckedInPages() throws {
        var rows: [Result] = []
        for name in ["beamer-overlays", "tile-text", "tile-paths"] {
            let dl3 = Self.fixtures.appendingPathComponent("\(name).dl3")
            guard FileManager.default.fileExists(atPath: dl3.path) else { XCTFail("missing \(dl3.path)"); continue }
            let pdf = Self.fixtures.appendingPathComponent("\(name).pdf")
            rows += try sweep(name, doc: try load(dl3), pdf: FileManager.default.fileExists(atPath: pdf.path) ? pdf : nil, surfaces: false)
        }
        XCTAssertTrue(rows.contains { $0.translated }, "no page tiled by translation")
        XCTAssertTrue(rows.contains { !$0.translated }, "no page cut from a whole raster")
        for r in rows {
            XCTAssertEqual(r.differingTiles, 0, "\(r.fixture) p\(r.page + 1) at \(r.scale) px/pt: \(r.differingTiles) of \(r.tiles) tiles differ (\(r.differingPixels) px)")
        }
        let tiles = rows.reduce(0) { $0 + $1.tiles }
        print("tile sweep (checked in): \(rows.count) page×scale, \(tiles) tiles, \(rows.reduce(0) { $0 + $1.differingTiles }) differing")
    }

    /// What the pane installs: the IOSurface tiles of `rasterizeTiles`.
    func testSurfaceTilesMatchTheWholePage() throws {
        for name in ["beamer-overlays", "tile-text", "tile-paths"] {
            let dl3 = Self.fixtures.appendingPathComponent("\(name).dl3")
            guard FileManager.default.fileExists(atPath: dl3.path) else { continue }
            for r in try sweep(name, doc: try load(dl3), pdf: nil, scales: [3.25, 4, 6, 8], surfaces: true) {
                XCTAssertEqual(r.differingTiles, 0, "\(r.fixture) p\(r.page + 1) at \(r.scale) px/pt (IOSurface)")
            }
        }
    }

    /// The pane reaches fit × zoom (≤ 4) × backing (2) px/pt: about 19 for a
    /// letter page and 32 for a 4:3 beamer frame in a 1,512 pt pane (a
    /// 14-inch MacBook Pro's full width). Tiles stay exact there, for cut
    /// pages (stroked table rules; paths and forms) and translated ones.
    func testTilesAreExactAtTheHighestReachableScales() throws {
        let cases: [(String, [Double])] = [("tile-text", [12, 16, 20]), ("tile-paths", [16, 24, 32]), ("beamer-overlays", [20, 32])]
        for (name, scales) in cases {
            let doc = try load(Self.fixtures.appendingPathComponent("\(name).dl3"))
            // Two pages each: enough for both kinds, page-sized references stay affordable.
            var small = doc
            small.pages = Dictionary(uniqueKeysWithValues: doc.pages.sorted { $0.key < $1.key }.prefix(2).map { ($0.key, $0.value) })
            for r in try sweep(name, doc: small, pdf: nil, scales: scales, surfaces: false) {
                XCTAssertEqual(r.differingTiles, 0, "\(name) p\(r.page + 1) at \(r.scale) px/pt: \(r.differingTiles) of \(r.tiles) tiles differ")
            }
        }
    }

    /// A cut page's raster is backed by memory only where the requested
    /// tiles are: one 512 px tile at the top right of a letter page at
    /// 20 px/pt stays a few MB (a page-sized raster there is 775 MB).
    func testCutRasterMemoryIsBoundedByTheTiles() throws {
        let doc = try load(Self.fixtures.appendingPathComponent("tile-text.dl3"))
        let page = try XCTUnwrap(doc.orderedPages.first)
        XCTAssertFalse(DL3Renderer.tilesByTranslation(page), "tile-text has stroked rules: a cut page")
        DL3Renderer.measureResidency = true
        defer { DL3Renderer.measureResidency = false }
        for scale in [8.0, 16, 20] {
            let (w, h) = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
            let corner = DL3PixelRect(x: w - 512, y: 0, width: 512, height: 512)
            let viewport = Self.rects(width: w, height: h).filter { $0.y < 2048 && $0.x + $0.width > w - 2560 } // 5×4 tiles
            for (label, rects) in [("one tile", [corner]), ("viewport", viewport)] {
                DL3Renderer.resetResidency()
                XCTAssertEqual(DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects).compactMap { $0 }.count, rects.count)
                let resident = DL3Renderer.lastCutResidentBytes, tiles = rects.reduce(0) { $0 + $1.width * $1.height * 4 }
                print("cut raster \(label) at \(scale) px/pt: \(resident) bytes resident for \(tiles) bytes of tiles (page raster \(w * h * 4))")
                XCTAssertGreaterThan(resident, 0)
                // Rows touch whole 16 KB pages: at most 2 per tile row span beyond the tiles' own bytes.
                XCTAssertLessThanOrEqual(resident, tiles + rects.map(\.height).reduce(0, +) * 2 * 16384, "\(label) at \(scale)")
            }
        }
    }

    /// A page drawn whole (paths, forms) tiles at a capped scale whose page
    /// raster stays within `wholeRasterMaxBytes`; glyph and rule pages tile at
    /// the screen's scale.
    func testPagesDrawnWholeTileAtABoundedScale() throws {
        let paths = try XCTUnwrap(try load(Self.fixtures.appendingPathComponent("tile-paths.dl3")).orderedPages.first { !DL3Renderer.clipExact($0) })
        let text = try XCTUnwrap(try load(Self.fixtures.appendingPathComponent("tile-text.dl3")).orderedPages.first)
        XCTAssertTrue(DL3Renderer.clipExact(text))
        for ppp in [8.0, 16, 20, 32] {
            let s = DL3Renderer.tileScale(widthPt: paths.widthPt, heightPt: paths.heightPt, drawnWhole: true, pixelsPerPoint: ppp)
            let (w, h) = DL3Renderer.pixelSize(widthPt: paths.widthPt, heightPt: paths.heightPt, scale: s)
            XCTAssertLessThanOrEqual(w * h * 4, DL3Renderer.wholeRasterMaxBytes + (w + h) * 4 * 2, "\(ppp) px/pt → \(s)")
            XCTAssertLessThanOrEqual(s, ppp)
            XCTAssertEqual(DL3Renderer.tileScale(widthPt: text.widthPt, heightPt: text.heightPt, drawnWhole: false, pixelsPerPoint: ppp), ppp)
        }
    }

    /// #1228's reviewed case: an origin whose page sum lands on a phase
    /// boundary keeps the page's side in a tile; others pass unchanged.
    func testTileCoordinateKeepsThePagesPhase() {
        let s = 6.0, v = 354.6665
        let page = s * v + 0.001
        let moved = DL3Renderer.tileCoordinate(v, scale: s)
        XCTAssertNotEqual(moved, v)
        XCTAssertEqual(floor((s * moved - 2048 + 0.001) * 4), floor(page * 4) - 2048 * 4)
        XCTAssertEqual(DL3Renderer.tileCoordinate(100.123, scale: 4), 100.123)
    }

    /// 8 px/pt memory and time (opt-in, release build):
    /// `FLASHTEX_V3_TILE_BENCH=<fixture dir name>[:page]`, optionally
    /// `FLASHTEX_V3_TILE_BENCH_OUT=path.json`. A pane of 1100×900 pt at 2×
    /// backing, scrolled to the top of the page; median of 9 runs.
    func testTileBenchmark() throws {
        let env = ProcessInfo.processInfo.environment
        guard let spec = env["FLASHTEX_V3_TILE_BENCH"] else { throw XCTSkip("FLASHTEX_V3_TILE_BENCH=<fixture>[:page]") }
        let parts = spec.split(separator: ":")
        let dir = URL(fileURLWithPath: env["FLASHTEX_DL3_FIXTURES"] ?? PreviewParityTests.repoRoot.appendingPathComponent("target/dl3-positions").path)
        let doc = try load(dir.appendingPathComponent(String(parts[0])).appendingPathComponent("display.dl3"))
        let pageIndex = parts.count > 1 ? Int(parts[1]) ?? 0 : 0
        let page = try XCTUnwrap(doc.orderedPages.dropFirst(pageIndex).first)
        let scale = Double(env["FLASHTEX_V3_TILE_BENCH_SCALE"] ?? "") ?? 8
        let size = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
        // The viewport in page pixels: 1100×900 pt at 2 px per view point, plus the 256 px prefetch margin.
        let view = CGRect(x: 0, y: 0, width: 2200, height: 1800).insetBy(dx: -256, dy: -256)
        var rects: [DL3PixelRect] = []
        for r in Self.rects(width: size.width, height: size.height) where view.intersects(CGRect(x: r.x, y: r.y, width: r.width, height: r.height)) { rects.append(r) }
        let row = Array(rects.filter { $0.y == rects.map(\.y).max()! })
        func median(_ body: () -> Void) -> Double {
            var t: [Double] = []
            for _ in 0 ..< 9 { let t0 = DispatchTime.now().uptimeNanoseconds; body(); t.append(Double(DispatchTime.now().uptimeNanoseconds - t0) / 1e6) }
            return t.sorted()[4]
        }
        var wholeBytes = 0
        let wholeMs = median { if let s = DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: scale) { wholeBytes = s.allocationSize } }
        var tileBytes = 0
        let tilesMs = median { tileBytes = DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects).reduce(0) { $0 + ($1?.allocationSize ?? 0) } }
        let sequentialMs = median { for r in rects { _ = DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: [r]) } }
        let rowMs = median { _ = DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: row) }
        let oneMs = median { _ = DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: [rects[rects.count / 2]]) }
        var backdropBytes = 0
        let backdropMs = median { if let s = DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: 2) { backdropBytes = s.allocationSize } }
        var load = [Double](repeating: 0, count: 3); getloadavg(&load, 3)
        let glyphs = page.page.items.reduce(0) { if case .glyph = $1 { $0 + 1 } else { $0 } }
        let result: [String: Any] = [
            "fixture": spec, "px_per_pt": scale, "page_px": [size.width, size.height], "glyphs": glyphs,
            "tiles_by_translation": DL3Renderer.tilesByTranslation(page), "workers": DL3Renderer.tileWorkers,
            "whole_page_ms": wholeMs, "whole_page_bytes": wholeBytes,
            "viewport_tiles": rects.count, "viewport_tiles_ms_parallel": tilesMs, "viewport_tiles_ms_sequential": sequentialMs, "viewport_tile_bytes": tileBytes,
            "scroll_row_tiles": row.count, "scroll_row_ms": rowMs, "one_tile_ms": oneMs,
            "backdrop_2ppt_ms": backdropMs, "backdrop_2ppt_bytes": backdropBytes, "load_average": load,
        ]
        let data = try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
        print("tile bench: " + String(decoding: data, as: UTF8.self))
        if let out = env["FLASHTEX_V3_TILE_BENCH_OUT"] { try data.write(to: URL(fileURLWithPath: out)) }
    }

    /// The full sweep over every parity fixture (opt-in: minutes).
    func testEveryParityFixtureTilesExactly() throws {
        let env = ProcessInfo.processInfo.environment
        guard env["FLASHTEX_V3_TILE_SWEEP"] == "1" else { throw XCTSkip("FLASHTEX_V3_TILE_SWEEP=1 runs the full sweep") }
        let dir = URL(fileURLWithPath: env["FLASHTEX_DL3_FIXTURES"] ?? PreviewParityTests.repoRoot.appendingPathComponent("target/dl3-positions").path)
        let only = env["FLASHTEX_V3_TILE_ONLY"].map { $0.split(separator: ",").map(String.init) }
        let names = try FileManager.default.contentsOfDirectory(atPath: dir.path).sorted()
            .filter { n in only.map { $0.contains { n.contains($0) } } ?? true }
        let scales = env["FLASHTEX_V3_TILE_SCALES"].map { $0.split(separator: ",").compactMap { Double($0) } } ?? Self.scales
        var rows: [Result] = []
        for name in names {
            let dl3 = dir.appendingPathComponent(name).appendingPathComponent("display.dl3")
            guard FileManager.default.fileExists(atPath: dl3.path) else { continue }
            let pdf = PreviewParityTests.enginePDF(in: dir.appendingPathComponent(name).appendingPathComponent("src"))
            // Above 8 px/pt the PDF comparison is skipped (page-sized arrays).
            rows += try sweep(name, doc: try load(dl3), pdf: scales.max()! <= 8 ? pdf : nil, scales: scales, surfaces: env["FLASHTEX_V3_TILE_SURFACES"] == "1")
        }
        if let out = env["FLASHTEX_V3_TILE_SWEEP_OUT"] {
            let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(rows).write(to: URL(fileURLWithPath: out))
        }
        let tiles = rows.reduce(0) { $0 + $1.tiles }, bad = rows.filter { $0.differingTiles > 0 }
        let pdfExact = rows.filter { $0.pdfPixels == 0 }.count, pdfRows = rows.filter { $0.pdfPixels != nil }.count
        let kitWorse = rows.filter { ($0.pdfKitPixels ?? 0) > ($0.pdfKitBaseline ?? 0) }.count
        print("tile sweep: \(rows.count) page×scale (\(rows.filter(\.translated).count) by translation), \(tiles) tiles, \(bad.reduce(0) { $0 + $1.differingTiles }) differing; tiled pages = PDF raster \(pdfExact)/\(pdfRows); further from PDFKit than Core Graphics is: \(kitWorse)")
        for r in bad { XCTFail("\(r.fixture) p\(r.page + 1) at \(r.scale) px/pt: \(r.differingTiles) of \(r.tiles) tiles differ (\(r.differingPixels) px)") }
    }
}
