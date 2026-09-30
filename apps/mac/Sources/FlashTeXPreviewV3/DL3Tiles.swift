import CoreGraphics
import Foundation
import IOSurface
import FlashTeXDisplayListV3

// High-zoom tiles (DESIGN.md §6.2; lane P3-V3-ZOOM-TILES, from #1228's
// preview-v2 tiles). Above about 3 px/pt a whole-page bitmap is large
// (8 px/pt: 4896×6336 px, 124 MB for US letter) and slow, while the pane
// shows a small part of it. The pane then holds 512 px tiles of the visible
// area instead, each drawn here, off the main thread.
//
// Every tile is a pixel-exact window of the whole-page raster (`rasterize`),
// which is what the zero-tolerance parity gate compares with Core Graphics'
// rendering of the engine's PDF, so the gate covers tiled pages too
// (`TileParityTests` sweeps 3.25–8 px/pt with 0 differing tiles).
//
// A tile context is the page context translated by whole pixels. That alone
// is not exact: Core Graphics rounds some device coordinates at the
// magnitude they have, and a tile's coordinates are smaller than the page's
// (measured in #1228, docs/evidence/preview-smoothing-2026-09-29/README.md §2):
//
// - A glyph's pixel and subpixel phase are `floor((d + 0.001)·N)` per axis
//   (device coordinate d, N = 1, 2 or 4 phases by glyph size). When
//   `d + 0.001` lies within an ulp of a phase boundary the sum rounds one way
//   on the page and the other in the tile. `tileCoordinate` moves such an
//   origin 1e-7 px onto the page's side; every other origin is unchanged.
// - Rule edges are single-precision device coordinates: `tileEdge` hands
//   over the page's single-precision value, which the translation keeps exact.
// - Paths, clips, images, forms, stroked rules and stroked text depend on
//   device coordinates in ways no snapping fixes (curve flattening, stroke
//   expansion, edge clipping, image sampling). A page with any of them, and
//   a page drawn from the PDF (fallbacks), is drawn with the page's own
//   context (nothing translated), clipped to the requested tiles, and the
//   tiles are copied out (`cutTiles`): exact, and only the tiles' pixels
//   are ever backed by memory, at any scale.

/// A rectangle of a page raster in pixels, top-left origin (image rows).
public struct DL3PixelRect: Hashable, Sendable {
    public var x: Int, y: Int, width: Int, height: Int
    public init(x: Int, y: Int, width: Int, height: Int) { self.x = x; self.y = y; self.width = width; self.height = height }
}

extension DL3Renderer {
    /// The context is a tile of the page raster at `scale` px/pt; `visible`
    /// is the tile in the page's user space (bp, y up).
    struct Tile {
        let scale: Double
        let visible: CGRect
        /// Beyond an item's geometric box, what antialiasing can still ink
        /// (bp): at the tiled scales (> 3 px/pt) 2 bp is at least 6 px.
        static let margin = 2.0
    }

    /// Glyph ink boxes for culling, recomputed when the font or glyph matrix changes.
    struct TileInk {
        private var font: DL3RenderFont?
        private var a = 0.0, b = 0.0, c = 0.0, d = 0.0
        private var box: CGRect?

        /// Whether a glyph of `font` drawn with text matrix `tm` may ink `visible`.
        mutating func meets(_ visible: CGRect, font f: DL3RenderFont, matrix tm: CGAffineTransform) -> Bool {
            if font !== f || a != tm.a || b != tm.b || c != tm.c || d != tm.d {
                font = f; a = tm.a; b = tm.b; c = tm.c; d = tm.d
                box = f.inkBox.map {
                    $0.applying(CGAffineTransform(a: tm.a, b: tm.b, c: tm.c, d: tm.d, tx: 0, ty: 0))
                        .insetBy(dx: -Tile.margin, dy: -Tile.margin)
                }
            }
            guard let box, box.width.isFinite, box.height.isFinite else { return true }
            return box.offsetBy(dx: tm.tx, dy: tm.ty).intersects(visible)
        }
    }

    /// Margin around a phase boundary (device px) inside which an origin is
    /// moved to the page's side; the move is 1e-7 px.
    static let tilePhaseMargin = 1e-6

    /// A glyph origin coordinate (user space, bp) for a tile of the page
    /// raster at `scale` (a page whose box starts at the origin).
    static func tileCoordinate(_ v: Double, scale s: Double) -> Double {
        // The page's device coordinate and phase sum as Core Graphics forms
        // them; ×4 is exact and covers the boundaries of N = 1, 2 and 4.
        let d = s * v
        let q = (d + 0.001) * 4
        let k = q.rounded()
        guard abs(q - k) < tilePhaseMargin else { return v }
        return (k / 4 - 0.001 + (q >= k ? 1e-7 : -1e-7)) / s
    }

    /// A rule edge (user space, bp) for a tile at `scale`: the page's
    /// single-precision device coordinate, back in user space.
    static func tileEdge(_ v: Double, scale s: Double) -> Double { Double(Float(s * v)) / s }

    /// A filled rule in a tile, with the whole page's rounding: its edges
    /// are the page's single-precision device coordinates (`tileEdge`).
    /// (Stroked rules never reach a tile drawn by translation: see
    /// `tilesByTranslation`.)
    static func tileRule(left: Double, right: Double, top: Double, bottom: Double, tile: Tile, in ctx: CGContext) {
        guard CGRect(x: left, y: bottom, width: right - left, height: top - bottom).standardized
            .insetBy(dx: -Tile.margin, dy: -Tile.margin).intersects(tile.visible) else { return }
        let s = tile.scale
        let x0 = tileEdge(left, scale: s), x1 = tileEdge(right, scale: s), y0 = tileEdge(bottom, scale: s), y1 = tileEdge(top, scale: s)
        ctx.beginPath()
        ctx.addRect(CGRect(x: x0, y: y0, width: x1 - x0, height: y1 - y0))
        ctx.fillPath()
    }

    /// Whether a page's tiles may come from a raster clipped to them: glyphs
    /// and rules only (filled or stroked), as measured over the 83 parity
    /// fixtures up to 32 px/pt (TileParityTests). Pages with paths, clips,
    /// images, forms or stroked text are drawn whole.
    public static func clipExact(_ prepared: DL3PreparedPage) -> Bool {
        let p = prepared.page
        guard p.box[0] == 0, p.box[1] == 0, !prepared.needsPDFFallback else { return false }
        return !p.items.contains { switch $0 { case .path, .clip, .image, .form: true; case .textRender(let m): m == 1 || m == 2; default: false } }
    }

    /// The largest page raster a tile job draws for a page that is drawn
    /// whole (`clipExact` false): 128 MB, a letter page at 8 px/pt.
    public static let wholeRasterMaxBytes = Int(ProcessInfo.processInfo.environment["FLASHTEX_V3_WHOLE_RASTER_MAX_MB"] ?? "").map { $0 << 20 } ?? 128 << 20

    /// The scale a page's tiles are drawn at when the pane shows it at
    /// `pixelsPerPoint`: that scale, except for a page drawn whole, whose
    /// scale is capped so its raster stays within `wholeRasterMaxBytes`
    /// (above it the tiles are stretched on screen, like the backdrop).
    public static func tileScale(widthPt: Double, heightPt: Double, drawnWhole: Bool, pixelsPerPoint ppp: Double) -> Double {
        guard drawnWhole, widthPt > 0, heightPt > 0 else { return ppp }
        let cap = (Double(wholeRasterMaxBytes) / 4 / (widthPt * heightPt)).squareRoot()
        guard ppp > cap else { return ppp }
        // A multiple of 1/64 px/pt below the cap (a stable key for the tile source).
        return (cap * 64).rounded(.down) / 64
    }

    /// The page raster's size at `scale` (what `rasterize` allocates).
    public static func pixelSize(widthPt: Double, heightPt: Double, scale: Double) -> (width: Int, height: Int) {
        (Int((widthPt * scale).rounded(.up)), Int((heightPt * scale).rounded(.up)))
    }

    /// Whether a translated tile context reproduces `prepared`'s whole-page
    /// raster exactly: only glyphs (filled or invisible), filled rules and
    /// state items, on a page whose box starts at the origin.
    ///
    /// Stroked rules (pdfTeX's thin rules: `\hrule`, table lines) are
    /// excluded after measurement (TileParityTests over the 83 parity
    /// fixtures at 3.25–8 px/pt): stroking the translated line differs from
    /// the page in 1–2,000 px per affected page; filling the stroke's outline
    /// with page-rounded edges matched everywhere except one rule (a 1-level
    /// difference along one pixel row, `proof-practice` at 3.75 px/pt, 16
    /// pages), and no rounding of the centre line and width reproduced Core
    /// Graphics' stroker there. Their pages are cut from one raster: exact.
    public static func tilesByTranslation(_ prepared: DL3PreparedPage) -> Bool {
        let p = prepared.page
        guard p.box[0] == 0, p.box[1] == 0, !prepared.needsPDFFallback else { return false }
        for it in p.items {
            switch it {
            case .glyph, .save, .restore, .fillColor, .strokeColor, .matrix, .span, .unsupported: continue
            case .rule(let kind, _, _, _, _): if kind != .fill { return false }
            case .textRender(let m): if m == 1 || m == 2 { return false }
            case .path, .clip, .image, .form: return false
            }
        }
        return true
    }

    /// The context configuration of `bitmapContext`/`surface`, for a tile:
    /// the page context translated by whole pixels (`origin`: the tile's
    /// bottom-left corner in page pixels, y up).
    static func configureTile(_ ctx: CGContext, width w: Int, height h: Int, scale: Double, origin: (x: Int, y: Int)) {
        ctx.setFillColor(CGColor(gray: 1, alpha: 1))
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        if origin.x != 0 || origin.y != 0 { ctx.translateBy(x: CGFloat(-origin.x), y: CGFloat(-origin.y)) }
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        ctx.setShouldSmoothFonts(false)
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
    }

    static func tileSpace() -> CGColorSpace { CGColorSpace(name: CGColorSpace.sRGB)! }

    /// Draws tile `rect` of a translatable page into `ctx` (w×h = rect size).
    static func drawTile(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage], scale: Double, rect r: DL3PixelRect, in ctx: CGContext) {
        let (_, pageHeight) = pixelSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale)
        let origin = (x: r.x, y: pageHeight - r.y - r.height)
        configureTile(ctx, width: r.width, height: r.height, scale: scale, origin: origin)
        let visible = CGRect(x: Double(origin.x) / scale, y: Double(origin.y) / scale, width: Double(r.width) / scale, height: Double(r.height) / scale)
        drawStream(prepared, forms: forms, in: ctx, depth: 0, tile: Tile(scale: scale, visible: visible))
    }

    /// One tile as a CGImage in `layout` (tests, evidence). Pages that do
    /// not tile by translation are cut from a whole-page raster.
    public static func rasterizeTile(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                     rect r: DL3PixelRect, layout: Layout = .rgba) -> CGImage? {
        guard tilesByTranslation(prepared) else {
            // What the pane installs: the tile cut from the clipped page raster.
            return cutTiles(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, rects: [r], clipped: clipExact(prepared)) { draw(prepared, forms: forms, in: $0) }[0]
                .flatMap { image(of: $0) } // (BGRA whatever `layout`: DL3Parity.rgba normalises)
        }
        guard r.width > 0, r.height > 0,
              let ctx = CGContext(data: nil, width: r.width, height: r.height, bitsPerComponent: 8, bytesPerRow: r.width * 4,
                                  space: tileSpace(), bitmapInfo: layout.bitmapInfo) else { return nil }
        drawTile(prepared, forms: forms, scale: scale, rect: r, in: ctx)
        return ctx.makeImage()
    }

    /// Parallel workers for a tile job.
    public static let tileWorkers = max(1, min(ProcessInfo.processInfo.activeProcessorCount, 8))

    /// Tiles `rects` of `prepared` at `scale`, each an IOSurface (BGRA,
    /// tagged sRGB: a layer shows it without a copy or a colour conversion
    /// in the main thread's commit), in the order given. Translatable pages
    /// draw each tile on its own, in parallel; others draw the whole page
    /// once and cut. Safe off the main thread.
    public static func rasterizeTiles(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                      rects: [DL3PixelRect]) -> [IOSurface?] {
        guard !rects.isEmpty else { return [] }
        guard tilesByTranslation(prepared) else {
            return cutTiles(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, rects: rects, clipped: clipExact(prepared)) { draw(prepared, forms: forms, in: $0) }
        }
        var out = [IOSurface?](repeating: nil, count: rects.count)
        let n = min(tileWorkers, rects.count)
        out.withUnsafeMutableBufferPointer { buffer in
            let base = buffer.baseAddress!
            DispatchQueue.concurrentPerform(iterations: n) { worker in
                var k = worker
                while k < rects.count {
                    let r = rects[k]
                    base[k] = tileSurface(width: r.width, height: r.height) { drawTile(prepared, forms: forms, scale: scale, rect: r, in: $0) }
                    k += n
                }
            }
        }
        return out
    }

    /// Tiles of a PDF page (the fallback for pages the display list cannot
    /// draw exactly), cut from one raster of it.
    public static func rasterizeTiles(pdfPage: CGPDFPage, scale: Double, rects: [DL3PixelRect]) -> [IOSurface?] {
        let box = pdfPage.getBoxRect(.mediaBox)
        return cutTiles(widthPt: box.width, heightPt: box.height, scale: scale, rects: rects, clipped: false) { ctx in
            ctx.translateBy(x: -box.minX, y: -box.minY)
            ctx.drawPDFPage(pdfPage)
        }
    }

    /// Tiles `rects` of the page raster `body` draws, drawn with the page's
    /// own context (the configuration of `bitmapContext`, the same device
    /// coordinates) but clipped to `rects`, then copied out.
    ///
    /// The context spans the page (its device origin is the page's
    /// bottom-left corner, so nothing is translated), but its memory is an
    /// anonymous mapping, which the system backs only where it is written:
    /// the clip and the white fill cover just the rects, so the resident size
    /// is about the rects' own (`lastCutResidentBytes`), at any scale. The
    /// clip is pixel-aligned and changes no pixel inside it
    /// (TileParityTests: 0 differing tiles up to 32 px/pt). Before, each job
    /// drew a page-sized raster: 470 MB at 16 px/pt and 743 MB at 20 px/pt for
    /// a letter page (review of #1287).
    ///
    /// `clipped` false (pages with paths, clips, images, forms or stroked
    /// text, and PDF pages): the clip is not exact for them (it changed 1–17
    /// px of one tile per page on a beamer page with shadings, `tile-paths`,
    /// at 4–7.75 px/pt, with clip margins up to 256 px too), so the whole page
    /// is drawn; the pane caps their tile scale so that this raster stays
    /// within `wholeRasterMaxBytes` (`tileScale(for:pixelsPerPoint:)`).
    static func cutTiles(widthPt: Double, heightPt: Double, scale: Double, rects: [DL3PixelRect], clipped: Bool,
                         _ body: (CGContext) -> Void) -> [IOSurface?] {
        let (W, H) = pixelSize(widthPt: widthPt, heightPt: heightPt, scale: scale)
        let ok = rects.filter { $0.width > 0 && $0.height > 0 && $0.x >= 0 && $0.y >= 0 && $0.x + $0.width <= W && $0.y + $0.height <= H }
        let stride = W * 4, size = stride * H
        guard !ok.isEmpty, W > 0, H > 0 else { return rects.map { _ in nil } }
        let mem = mmap(nil, size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0)
        guard let mem, mem != MAP_FAILED else { return rects.map { _ in nil } }
        defer { munmap(mem, size) }
        guard let ctx = CGContext(data: mem, width: W, height: H, bitsPerComponent: 8, bytesPerRow: stride,
                                  space: tileSpace(), bitmapInfo: Layout.screen.bitmapInfo) else { return rects.map { _ in nil } }
        // Device rects (y up) of the requested tiles: clip, then the page's white.
        let device = ok.map { CGRect(x: $0.x, y: H - $0.y - $0.height, width: $0.width, height: $0.height) }
        if clipped {
            ctx.clip(to: device)
            ctx.setFillColor(CGColor(gray: 1, alpha: 1))
            ctx.fill(device)
        } else {
            ctx.setFillColor(CGColor(gray: 1, alpha: 1))
            ctx.fill(CGRect(x: 0, y: 0, width: W, height: H))
        }
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        ctx.setShouldSmoothFonts(false)
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
        body(ctx)
        ctx.flush()
        if measureResidency { recordResidency(mem, size) }
        let base = mem.assumingMemoryBound(to: UInt8.self)
        var out = [IOSurface?](repeating: nil, count: rects.count)
        out.withUnsafeMutableBufferPointer { buffer in
            let dst = buffer.baseAddress!
            DispatchQueue.concurrentPerform(iterations: rects.count) { k in
                let r = rects[k]
                guard r.width > 0, r.height > 0, r.x >= 0, r.y >= 0, r.x + r.width <= W, r.y + r.height <= H,
                      let s = newSurface(width: r.width, height: r.height) else { return }
                s.lock(options: [], seed: nil)
                let o = s.baseAddress.assumingMemoryBound(to: UInt8.self), os = s.bytesPerRow
                for row in 0 ..< r.height { memcpy(o + row * os, base + (r.y + row) * stride + r.x * 4, r.width * 4) }
                s.unlock(options: [], seed: nil)
                tag(s)
                dst[k] = s
            }
        }
        return out
    }

    /// Measurement (benches, tests): the resident size of each cut raster.
    nonisolated(unsafe) public static var measureResidency = false
    private static let residencyLock = NSLock()
    nonisolated(unsafe) private static var _lastResident = 0, _maxResident = 0
    /// Resident bytes of the last cut raster, and the largest since `resetResidency`.
    public static var lastCutResidentBytes: Int { residencyLock.lock(); defer { residencyLock.unlock() }; return _lastResident }
    public static var maxCutResidentBytes: Int { residencyLock.lock(); defer { residencyLock.unlock() }; return _maxResident }
    public static func resetResidency() { residencyLock.lock(); _lastResident = 0; _maxResident = 0; residencyLock.unlock() }

    private static func recordResidency(_ mem: UnsafeMutableRawPointer, _ size: Int) {
        let page = Int(getpagesize())
        var vec = [CChar](repeating: 0, count: (size + page - 1) / page)
        guard mincore(mem, size, &vec) == 0 else { return }
        let bytes = vec.reduce(0) { $0 + (($1 & 1) != 0 ? page : 0) }
        residencyLock.lock(); _lastResident = bytes; _maxResident = max(_maxResident, bytes); residencyLock.unlock()
    }

    static func newSurface(width w: Int, height h: Int) -> IOSurface? {
        guard w > 0, h > 0 else { return nil }
        return IOSurface(properties: [.width: w, .height: h, .bytesPerElement: 4, .pixelFormat: 0x4247_5241 /* 'BGRA' */])
    }

    static func tag(_ s: IOSurface) {
        if let plist = tileSpace().copyPropertyList() { IOSurfaceSetValue(s, kIOSurfaceColorSpace, plist) }
    }

    /// A surface of w×h px drawn by `body` into a context over its memory.
    static func tileSurface(width w: Int, height h: Int, _ body: (CGContext) -> Void) -> IOSurface? {
        guard let s = newSurface(width: w, height: h) else { return nil }
        s.lock(options: [], seed: nil)
        defer { s.unlock(options: [], seed: nil) }
        guard let ctx = CGContext(data: s.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: s.bytesPerRow,
                                  space: tileSpace(), bitmapInfo: Layout.screen.bitmapInfo) else { return nil }
        body(ctx)
        ctx.flush()
        tag(s)
        return s
    }
}
