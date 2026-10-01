import CoreGraphics
import Darwin
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

    /// The rect sets the pane requests for a viewport (`EngineV3PageTiles`):
    /// the visible block, then the ring of the 256 px prefetch margin around
    /// it, as separate jobs. `view` is the viewport in page pixels.
    static func blockAndRing(width w: Int, height h: Int, view: CGRect) -> (block: [DL3PixelRect], ring: [DL3PixelRect]) {
        func covering(_ r: CGRect) -> Set<[Int]> {
            let c = r.intersection(CGRect(x: 0, y: 0, width: w, height: h))
            guard !c.isNull, c.width > 0, c.height > 0 else { return [] }
            var out = Set<[Int]>()
            for row in Int(c.minY) / tile ... (Int(c.maxY.rounded(.up)) - 1) / tile {
                for col in Int(c.minX) / tile ... (Int(c.maxX.rounded(.up)) - 1) / tile { out.insert([col, row]) }
            }
            return out
        }
        func rect(_ i: [Int]) -> DL3PixelRect {
            DL3PixelRect(x: i[0] * tile, y: i[1] * tile, width: min(tile, w - i[0] * tile), height: min(tile, h - i[1] * tile))
        }
        let block = covering(view), want = covering(view.insetBy(dx: -256, dy: -256))
        let order: ([Int], [Int]) -> Bool = { ($0[1], $0[0]) < ($1[1], $1[0]) }
        return (block.sorted(by: order).map(rect), want.subtracting(block).sorted(by: order).map(rect))
    }

    /// Surfaces against the same windows of the whole-page raster: the number of differing tiles.
    func differing(_ surfaces: [IOSurface?], _ rects: [DL3PixelRect], whole: [UInt8], width: Int, _ label: String) throws -> Int {
        XCTAssertEqual(surfaces.count, rects.count, label)
        var n = 0
        for (s, r) in zip(surfaces, rects) {
            let img = try XCTUnwrap(s.flatMap { DL3Renderer.image(of: $0) }, "\(label) \(r)")
            if DL3Parity.diff(DL3Parity.rgba(img), Self.window(whole, pageWidth: width, r)).pixels > 0 { n += 1 }
        }
        return n
    }

    /// A clip-exact page's raster is backed by memory only where the
    /// requested tiles are, and the tiles are exact: one 512 px tile at the
    /// top right and a 5×4-tile block of a letter page with table rules, at 8,
    /// 16 and 20 px/pt (a page-sized raster there is 124, 496 and 776 MB).
    func testCutRasterMemoryIsBoundedByTheTiles() throws {
        let doc = try load(Self.fixtures.appendingPathComponent("tile-text.dl3"))
        let page = try XCTUnwrap(doc.orderedPages.first)
        XCTAssertFalse(DL3Renderer.tilesByTranslation(page), "tile-text has stroked rules")
        XCTAssertTrue(DL3Renderer.clipExact(page))
        DL3Renderer.measureResidency = true
        defer { DL3Renderer.measureResidency = false }
        for scale in [8.0, 16, 20] { try autoreleasepool {
            let whole = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale))
            let pixels = DL3Parity.rgba(whole)
            let (w, h) = (whole.width, whole.height)
            let corner = DL3PixelRect(x: w - 512, y: 0, width: 512, height: 512)
            let lastColumn = (w - 1) / 512 * 512
            let block = Self.rects(width: w, height: h).filter { $0.y < 2048 && $0.x >= lastColumn - 4 * 512 } // the top-right 5×4 tiles
            XCTAssertEqual(block.count, 20)
            for (label, rects) in [("one tile", [corner]), ("5×4 block", block)] {
                DL3Renderer.resetResidency()
                let surfaces = DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects)
                XCTAssertEqual(try differing(surfaces, rects, whole: pixels, width: w, "\(label) at \(scale)"), 0, "\(label) at \(scale) px/pt")
                let resident = DL3Renderer.lastCutResidentBytes, tiles = rects.reduce(0) { $0 + $1.width * $1.height * 4 }
                print("cut raster \(label) at \(scale) px/pt: \(resident) bytes resident for \(tiles) bytes of tiles (page raster \(w * h * 4))")
                XCTAssertGreaterThan(resident, 0)
                // Rows touch whole 16 KB pages: at most 2 per tile row beyond the tiles' own bytes.
                XCTAssertLessThanOrEqual(resident, tiles + rects.map(\.height).reduce(0, +) * 2 * 16384, "\(label) at \(scale)")
            }
        } }
    }

    /// Clipped tiles are exact in the rect sets the pane requests: the
    /// visible block and the prefetch ring, each its own clipped raster, at
    /// several viewport positions (top left, middle, bottom right) of a page
    /// with table rules, at 8, 16 and 20 px/pt.
    func testClippedTilesAreExactInThePanesBlockAndRingSets() throws {
        let doc = try load(Self.fixtures.appendingPathComponent("tile-text.dl3"))
        let page = try XCTUnwrap(doc.orderedPages.first)
        for scale in [8.0, 16, 20] { try autoreleasepool {
            let whole = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale))
            let pixels = DL3Parity.rgba(whole)
            let (w, h) = (whole.width, whole.height)
            let views = [CGRect(x: 0, y: 0, width: 1420, height: 1692),                                // 710×846 pt at 2×
                         CGRect(x: w / 2 - 700, y: h / 2 - 800, width: 1420, height: 1692),
                         CGRect(x: w - 1420, y: h - 1692, width: 1420, height: 1692)].map { $0.integral }
            var rings = 0
            for v in views {
                let (block, ring) = Self.blockAndRing(width: w, height: h, view: v)
                XCTAssertFalse(block.isEmpty)
                // (The ring is empty when the 256 px margin crosses no tile boundary.)
                if !ring.isEmpty { rings += 1 }
                for (label, rects) in [("block", block), ("ring", ring)] where !rects.isEmpty {
                    let surfaces = DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects)
                    XCTAssertEqual(try differing(surfaces, rects, whole: pixels, width: w, label), 0, "\(label) of \(v) at \(scale) px/pt")
                }
            }
            XCTAssertGreaterThan(rings, 0, "at \(scale) px/pt")
        } }
    }

    /// A page drawn whole (paths, forms: `tile-paths`) keeps one full-scale
    /// raster, cut exactly for the block and the ring at 12, 16 and 20 px/pt.
    /// Its pixels are purgeable memory: nothing is written to disk, and once
    /// idle (volatile) it is not part of the process's footprint.
    func testPageRasterIsExactWritesNothingAndIsOutsideTheFootprintWhenIdle() throws {
        let doc = try load(Self.fixtures.appendingPathComponent("tile-paths.dl3"))
        let page = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        for scale in [12.0, 16, 20] { try autoreleasepool {
            let whole = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale))
            let pixels = DL3Parity.rgba(whole)
            let before = Self.footprint(), diskBefore = Self.diskBytesWritten()
            let t0 = DispatchTime.now().uptimeNanoseconds
            let raster = try XCTUnwrap(DL3PageRaster(page, forms: doc.forms, scale: scale))
            let drawn = DispatchTime.now().uptimeNanoseconds
            XCTAssertEqual(raster.width, whole.width); XCTAssertEqual(raster.height, whole.height)
            let (block, ring) = Self.blockAndRing(width: raster.width, height: raster.height,
                                                  view: CGRect(x: raster.width / 4, y: raster.height / 4, width: 1420, height: 1692).integral)
            let first = try XCTUnwrap(raster.cut(block), "not purged")
            let cut = DispatchTime.now().uptimeNanoseconds
            let after = Self.footprint()
            XCTAssertEqual(try differing(first, block, whole: pixels, width: whole.width, "block"), 0, "block at \(scale)")
            XCTAssertEqual(try differing(try XCTUnwrap(raster.cut(ring)), ring, whole: pixels, width: whole.width, "ring"), 0, "ring at \(scale)")
            let grew = after - before, tiles = block.reduce(0) { $0 + $1.width * $1.height * 4 }
            let written = Self.diskBytesWritten() &- diskBefore
            print(String(format: "page raster at %.0f px/pt: %d bytes (purgeable), drawn in %.1f ms, first block (%d tiles) cut in %.1f ms; idle footprint +%d bytes (the block's surfaces: %d); disk written %llu bytes",
                         scale, raster.bytes, Double(drawn - t0) / 1e6, block.count, Double(cut - drawn) / 1e6, grew, tiles, written))
            // Volatile between cuts: growth is about the cut tiles, not the raster.
            XCTAssertLessThan(grew, raster.bytes / 2, "\(scale) px/pt: the idle raster counted in the footprint")
            XCTAssertLessThan(written, 1 << 20, "\(scale) px/pt: the raster was written to disk")
        } }
    }

    /// A PDF-fallback page's tiles come from a `DL3PageRaster` of the PDF
    /// page, whose grid is the media box's (the grid the pane partitions),
    /// and equal `rasterize(pdfPage:)` exactly.
    func testPDFPageRasterIsExact() throws {
        let pdf = try XCTUnwrap(CGPDFDocument(Self.fixtures.appendingPathComponent("tile-paths.pdf") as CFURL))
        let page = try XCTUnwrap(pdf.page(at: 1))
        for scale in [8.0, 16] { try autoreleasepool {
            let whole = try XCTUnwrap(DL3Renderer.rasterize(pdfPage: page, scale: scale))
            let raster = try XCTUnwrap(DL3PageRaster(pdfPage: page, scale: scale))
            let grid = DL3PageRaster.gridSize(pdfPage: page, scale: scale)
            XCTAssertEqual([raster.width, raster.height], [whole.width, whole.height])
            XCTAssertEqual([grid.width, grid.height], [whole.width, whole.height])
            let rects = Self.rects(width: whole.width, height: whole.height)
            XCTAssertEqual(try differing(try XCTUnwrap(raster.cut(rects)), rects, whole: DL3Parity.rgba(whole), width: whole.width, "pdf"), 0, "at \(scale) px/pt")
        } }
    }

    /// The cost at the last-resort bound (opt-in: `FLASHTEX_V3_RASTER_BENCH=1`,
    /// writes a 1 GiB file): a letter page drawn as a `DL3PageRaster` just
    /// below 1 GiB (23.2 px/pt), and at 19.3 px/pt (the most the pane reaches
    /// on a 14-inch MacBook Pro). Prints size, draw and cut time, footprint growth.
    func testPageRasterCostAtTheLastResortBound() throws {
        guard ProcessInfo.processInfo.environment["FLASHTEX_V3_RASTER_BENCH"] == "1" else { throw XCTSkip("FLASHTEX_V3_RASTER_BENCH=1") }
        let doc = try load(Self.fixtures.appendingPathComponent("tile-text.dl3"))
        let page = try XCTUnwrap(doc.orderedPages.first)
        let bound = DL3Renderer.tileScale(widthPt: page.widthPt, heightPt: page.heightPt, drawnWhole: true, pixelsPerPoint: 1000)
        for scale in [19.3, bound] { try autoreleasepool {
            let before = Self.footprint()
            let t0 = DispatchTime.now().uptimeNanoseconds
            let raster = try XCTUnwrap(DL3PageRaster(page, forms: doc.forms, scale: scale))
            let drawn = DispatchTime.now().uptimeNanoseconds
            let (block, _) = Self.blockAndRing(width: raster.width, height: raster.height,
                                               view: CGRect(x: raster.width / 3, y: raster.height / 3, width: 1420, height: 1692).integral)
            let tiles = try XCTUnwrap(raster.cut(block))
            let cut = DispatchTime.now().uptimeNanoseconds
            XCTAssertEqual(tiles.compactMap { $0 }.count, block.count)
            print(String(format: "last-resort bench at %.3f px/pt: raster %d bytes (≤ %d), drawn in %.0f ms, first block (%d tiles) in %.1f ms, idle footprint +%d bytes, resident %d bytes",
                         scale, raster.bytes, DL3Renderer.wholeRasterMaxBytes, Double(drawn - t0) / 1e6, block.count, Double(cut - drawn) / 1e6,
                         Self.footprint() - before, raster.residentBytes))
        } }
    }


    /// Dark appearance (#1254's dark preview): tiles of every kind equal the
    /// same windows of `rasterizeToSurface(…, appearance: .dark)`, the dark
    /// whole page the pane shows below the tile threshold: by translation
    /// (`beamer-overlays`), clipped (`tile-text`), kept raster (`tile-paths`)
    /// and a PDF fallback (`tile-paths.pdf`, its Core Image pass per tile).
    func testDarkTilesEqualTheDarkWholePage() throws {
        func check(_ surfaces: [IOSurface?], _ rects: [DL3PixelRect], whole: IOSurface?, _ label: String) throws {
            let w = try XCTUnwrap(whole.flatMap { DL3Renderer.image(of: $0) })
            XCTAssertEqual(try differing(surfaces, rects, whole: DL3Parity.rgba(w), width: w.width, label), 0, label)
        }
        for (name, pick) in [("beamer-overlays", 0), ("tile-text", 0), ("tile-paths", 1)] {
            let doc = try load(Self.fixtures.appendingPathComponent("\(name).dl3"))
            let page = doc.orderedPages[pick]
            for scale in [4.0, 12] { try autoreleasepool {
                let whole = DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: scale, appearance: .dark)
                let (w, h) = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
                let rects = Self.rects(width: w, height: h)
                try check(DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects, appearance: .dark), rects, whole: whole,
                          "\(name) dark at \(scale) (translated \(DL3Renderer.tilesByTranslation(page)), clipped \(DL3Renderer.clipExact(page)))")
            } }
        }
        let pdf = try XCTUnwrap(CGPDFDocument(Self.fixtures.appendingPathComponent("tile-paths.pdf") as CFURL)?.page(at: 2))
        for scale in [4.0, 12] { try autoreleasepool {
            let grid = DL3PageRaster.gridSize(pdfPage: pdf, scale: scale)
            let rects = Self.rects(width: grid.width, height: grid.height)
            try check(DL3Renderer.rasterizeTiles(pdfPage: pdf, scale: scale, rects: rects, appearance: .dark), rects,
                      whole: DL3Renderer.rasterizeToSurface(pdfPage: pdf, scale: scale, appearance: .dark), "pdf dark at \(scale)")
        } }
    }

    /// Settings > Smooth fonts in preview (#1304) holds for tiles too: with it
    /// on, every tile is the same window of the smoothed whole page, for
    /// each tile kind (by translation, clipped, cut from one raster, PDF).
    /// Smoothing must actually change the glyph page, or this proves nothing.
    func testSmoothedTilesEqualTheSmoothedWholePage() throws {
        func check(_ surfaces: [IOSurface?], _ rects: [DL3PixelRect], whole: IOSurface?, _ label: String) throws {
            let w = try XCTUnwrap(whole.flatMap { DL3Renderer.image(of: $0) })
            XCTAssertEqual(try differing(surfaces, rects, whole: DL3Parity.rgba(w), width: w.width, label), 0, label)
        }
        for (name, pick) in [("beamer-overlays", 0), ("tile-text", 0), ("tile-paths", 1)] {
            let doc = try load(Self.fixtures.appendingPathComponent("\(name).dl3"))
            let page = doc.orderedPages[pick]
            for scale in [4.0, 12] { try autoreleasepool {
                let whole = DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: scale, smoothFonts: true)
                if name == "beamer-overlays", scale == 4 {
                    let plain = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: scale).flatMap { DL3Renderer.image(of: $0) })
                    let smooth = try XCTUnwrap(whole.flatMap { DL3Renderer.image(of: $0) })
                    XCTAssertNotEqual(DL3Parity.rgba(plain), DL3Parity.rgba(smooth), "font smoothing changes the glyph page")
                }
                let (w, h) = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
                let rects = Self.rects(width: w, height: h)
                try check(DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: scale, rects: rects, smoothFonts: true), rects, whole: whole,
                          "\(name) smoothed at \(scale) (translated \(DL3Renderer.tilesByTranslation(page)), clipped \(DL3Renderer.clipExact(page)))")
            } }
        }
        let pdf = try XCTUnwrap(CGPDFDocument(Self.fixtures.appendingPathComponent("tile-paths.pdf") as CFURL)?.page(at: 2))
        for scale in [4.0, 12] { try autoreleasepool {
            let grid = DL3PageRaster.gridSize(pdfPage: pdf, scale: scale)
            let rects = Self.rects(width: grid.width, height: grid.height)
            try check(DL3Renderer.rasterizeTiles(pdfPage: pdf, scale: scale, rects: rects, smoothFonts: true), rects,
                      whole: DL3Renderer.rasterizeToSurface(pdfPage: pdf, scale: scale, smoothFonts: true), "pdf smoothed at \(scale)")
        } }
    }

    /// Bytes this process has written to disk so far (ri_diskio_byteswritten).
    static func diskBytesWritten() -> UInt64 {
        var ri = rusage_info_v4()
        let r = withUnsafeMutablePointer(to: &ri) { $0.withMemoryRebound(to: rusage_info_t?.self, capacity: 1) { proc_pid_rusage(getpid(), RUSAGE_INFO_V4, $0) } }
        return r == 0 ? ri.ri_diskio_byteswritten : 0
    }

    /// A scale that is not finite and positive, or a raster too large to
    /// address, yields no raster and no tiles; nothing traps.
    func testBadScalesYieldNoRasterAndNoTrap() throws {
        let doc = try load(Self.fixtures.appendingPathComponent("tile-paths.dl3"))
        let page = try XCTUnwrap(doc.orderedPages.first)
        let r = [DL3PixelRect(x: 0, y: 0, width: 16, height: 16)]
        for bad in [Double.nan, .infinity, -1, 0, 1e9] {
            XCTAssertNil(DL3PageRaster(page, forms: doc.forms, scale: bad), "\(bad)")
            XCTAssertNil(DL3PageRaster.checkedSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: bad))
            XCTAssertEqual(DL3Renderer.rasterizeTiles(page, forms: doc.forms, scale: bad, rects: r).compactMap { $0 }.count, 0)
        }
        XCTAssertNotNil(DL3PageRaster.checkedSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: 8))
    }

    /// The process's physical footprint.
    static func footprint() -> Int {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count) }
        }
        return kr == KERN_SUCCESS ? Int(info.phys_footprint) : 0
    }

    /// Only a raster over 1 GiB takes a lower scale (last resort): every
    /// scale the pane reaches on a 14-inch MacBook Pro keeps full scale.
    func testPagesDrawnWholeKeepFullScaleBelowTheLastResortBound() throws {
        let paths = try XCTUnwrap(try load(Self.fixtures.appendingPathComponent("tile-paths.dl3")).orderedPages.first { !DL3Renderer.clipExact($0) })
        let text = try XCTUnwrap(try load(Self.fixtures.appendingPathComponent("tile-text.dl3")).orderedPages.first)
        XCTAssertEqual(DL3Renderer.wholeRasterMaxBytes, 1 << 30)
        for ppp in [8.0, 16, 20, 32] {
            XCTAssertEqual(DL3Renderer.tileScale(widthPt: paths.widthPt, heightPt: paths.heightPt, drawnWhole: true, pixelsPerPoint: ppp), ppp)
        }
        for ppp in [8.0, 16, 19.3] { // letter: fit × 4 × 2 on a 1,512 pt pane
            XCTAssertEqual(DL3Renderer.tileScale(widthPt: text.widthPt, heightPt: text.heightPt, drawnWhole: true, pixelsPerPoint: ppp), ppp)
        }
        let capped = DL3Renderer.tileScale(widthPt: text.widthPt, heightPt: text.heightPt, drawnWhole: true, pixelsPerPoint: 30)
        XCTAssertLessThan(capped, 30)
        let (w, h) = DL3Renderer.pixelSize(widthPt: text.widthPt, heightPt: text.heightPt, scale: capped)
        XCTAssertLessThanOrEqual(w * h * 4, DL3Renderer.wholeRasterMaxBytes + (w + h) * 8)
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
