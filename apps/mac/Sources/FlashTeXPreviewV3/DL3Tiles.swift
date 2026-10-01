import CoreGraphics
import CoreImage
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
// - Everything else is not tiled by translation. Pages of glyphs and rules
//   (stroked rules included, `clipExact`) are drawn with the page's own
//   context, clipped to the requested tiles, over memory that is backed only
//   where it is written (`clippedTiles`). Pages with paths, clips, images,
//   forms or stroked text, and pages drawn from the PDF, draw one full-scale
//   page raster per source (`DL3PageRaster`, purgeable anonymous memory,
//   never written to disk), which every tile job of that source cuts from. Both are exact: the page's device coordinates, nothing
//   translated.

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
    /// and rules only (filled or stroked). Measured exact (TileParityTests):
    /// all 83 parity fixtures at 3.25–8 px/pt, 9 of them (stroked-rule and
    /// path pages) at 12, 16 and 20 px/pt, the checked-in `tile-text` up to
    /// 20 px/pt. Pages with paths, clips, images, forms or stroked text are
    /// not: a pixel-aligned clip changed 1–17 px of one tile per page of
    /// `tile-paths` (shadings), with clip margins up to 256 px too; they use
    /// a `DL3PageRaster`.
    public static func clipExact(_ prepared: DL3PreparedPage) -> Bool {
        let p = prepared.page
        guard p.box[0] == 0, p.box[1] == 0, !prepared.needsPDFFallback else { return false }
        return !p.items.contains { switch $0 { case .path, .clip, .image, .form: true; case .textRender(let m): m == 1 || m == 2; default: false } }
    }

    /// The last-resort bound on a `DL3PageRaster`: 1 GiB, a letter page at
    /// 23.5 px/pt, above the ~19.3 px/pt the pane reaches on a 14-inch
    /// MacBook Pro (a 4:3 beamer frame reaches it at about 52 px/pt). Only a
    /// page drawn whole in a wider pane than that reaches it. Measured
    /// (testPageRasterCostAtTheLastResortBound): 712 ms to draw, +2 MB of
    /// footprint (the pixels are file-backed); 201 ms at 19.3 px/pt.
    public static let wholeRasterMaxBytes = Int(ProcessInfo.processInfo.environment["FLASHTEX_V3_WHOLE_RASTER_MAX_MB"] ?? "").map { $0 << 20 } ?? 1 << 30

    /// The scale a page's tiles are drawn at when the pane shows it at
    /// `pixelsPerPoint`: that scale, except for a page drawn whole whose
    /// raster would exceed `wholeRasterMaxBytes` (last resort: above it the
    /// tiles are stretched on screen, like the backdrop).
    public static func tileScale(widthPt: Double, heightPt: Double, drawnWhole: Bool, pixelsPerPoint ppp: Double) -> Double {
        guard drawnWhole, widthPt > 0, heightPt > 0 else { return ppp }
        let cap = (Double(wholeRasterMaxBytes) / 4 / (widthPt * heightPt)).squareRoot()
        guard ppp > cap else { return ppp }
        // A multiple of 1/64 px/pt below the cap (a stable key for the tile source).
        return (cap * 64).rounded(.down) / 64
    }

    /// The page raster's size at `scale` (what `rasterize` allocates).
    /// 0×0 for a scale or size that is not finite and positive, or beyond
    /// 1 Mpx a side (never a trap; callers then draw nothing).
    public static func pixelSize(widthPt: Double, heightPt: Double, scale: Double) -> (width: Int, height: Int) {
        let w = (widthPt * scale).rounded(.up), h = (heightPt * scale).rounded(.up)
        guard w.isFinite, h.isFinite, w >= 0, h >= 0, w <= Double(1 << 20), h <= Double(1 << 20) else { return (0, 0) }
        return (Int(w), Int(h))
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
    static func configureTile(_ ctx: CGContext, width w: Int, height h: Int, scale: Double, origin: (x: Int, y: Int),
                              background: CGColor = CGColor(gray: 1, alpha: 1)) {
        ctx.setFillColor(background)
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
    static func drawTile(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage], scale: Double, rect r: DL3PixelRect, in ctx: CGContext,
                         appearance: DL3Appearance = .light) {
        let (_, pageHeight) = pixelSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale)
        let origin = (x: r.x, y: pageHeight - r.y - r.height)
        configureTile(ctx, width: r.width, height: r.height, scale: scale, origin: origin, background: appearance.background)
        let visible = CGRect(x: Double(origin.x) / scale, y: Double(origin.y) / scale, width: Double(r.width) / scale, height: Double(r.height) / scale)
        drawStream(prepared, forms: forms, in: ctx, depth: 0, appearance: appearance, tile: Tile(scale: scale, visible: visible))
    }

    /// One tile as a CGImage in `layout` (tests, evidence). Pages that do
    /// not tile by translation are cut from a whole-page raster.
    public static func rasterizeTile(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                     rect r: DL3PixelRect, layout: Layout = .rgba, appearance: DL3Appearance = .light) -> CGImage? {
        guard tilesByTranslation(prepared) else {
            // What the pane installs (BGRA whatever `layout`: DL3Parity.rgba normalises).
            return rasterizeTiles(prepared, forms: forms, scale: scale, rects: [r], appearance: appearance)[0].flatMap { image(of: $0) }
        }
        guard r.width > 0, r.height > 0,
              let ctx = CGContext(data: nil, width: r.width, height: r.height, bitsPerComponent: 8, bytesPerRow: r.width * 4,
                                  space: tileSpace(), bitmapInfo: layout.bitmapInfo) else { return nil }
        drawTile(prepared, forms: forms, scale: scale, rect: r, in: ctx, appearance: appearance)
        return ctx.makeImage()
    }

    /// Parallel workers for a tile job.
    public static let tileWorkers = max(1, min(ProcessInfo.processInfo.activeProcessorCount, 8))

    /// Tiles `rects` of `prepared` at `scale`, each an IOSurface (BGRA,
    /// tagged sRGB: a layer shows it without a copy or a colour conversion
    /// in the main thread's commit), in the order given. Translatable pages
    /// draw each tile on its own, in parallel; clip-exact pages draw the page
    /// clipped to `rects`; others draw a `DL3PageRaster` (here one per call;
    /// the pane keeps one per source, `EngineV3PageTiles`). Safe off the main
    /// thread.
    public static func rasterizeTiles(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                      rects: [DL3PixelRect], appearance: DL3Appearance = .light) -> [IOSurface?] {
        guard !rects.isEmpty else { return [] }
        // A scale that is not finite and positive, or a page too large to address: no tiles (never a trap).
        guard DL3PageRaster.checkedSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale) != nil else { return rects.map { _ in nil } }
        guard tilesByTranslation(prepared) else {
            if clipExact(prepared) {
                return clippedTiles(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, rects: rects,
                                    background: appearance.background) { draw(prepared, forms: forms, in: $0, appearance: appearance) }
            }
            return DL3PageRaster(prepared, forms: forms, scale: scale, appearance: appearance)?.cut(rects) ?? rects.map { _ in nil }
        }
        var out = [IOSurface?](repeating: nil, count: rects.count)
        let n = min(tileWorkers, rects.count)
        out.withUnsafeMutableBufferPointer { buffer in
            let base = buffer.baseAddress!
            DispatchQueue.concurrentPerform(iterations: n) { worker in
                var k = worker
                while k < rects.count {
                    let r = rects[k]
                    base[k] = tileSurface(width: r.width, height: r.height) { drawTile(prepared, forms: forms, scale: scale, rect: r, in: $0, appearance: appearance) }
                    k += n
                }
            }
        }
        return out
    }

    /// Tiles of a PDF page (the fallback for pages the display list cannot
    /// draw exactly), cut from one raster of it.
    public static func rasterizeTiles(pdfPage: CGPDFPage, scale: Double, rects: [DL3PixelRect], appearance: DL3Appearance = .light) -> [IOSurface?] {
        guard let raster = DL3PageRaster(pdfPage: pdfPage, scale: scale), let cut = raster.cut(rects) else { return rects.map { _ in nil } }
        return pdfTiles(cut, appearance: appearance)
    }

    /// A PDF page's tiles in `appearance`: dark is `rasterizeToSurface(pdfPage:
    /// appearance: .dark)`'s Core Image pass (invert, then rotate hue by half
    /// a turn), applied tile by tile. Both filters work pixel by pixel, so a
    /// tile equals the same window of the dark whole page (TileParityTests).
    public static func pdfTiles(_ light: [IOSurface?], appearance: DL3Appearance) -> [IOSurface?] {
        guard appearance == .dark else { return light }
        return light.map { s in
            guard let s, let out = newSurface(width: s.width, height: s.height) else { return nil }
            let ci = CIImage(ioSurface: s)
                .applyingFilter("CIColorInvert")
                .applyingFilter("CIHueAdjust", parameters: [kCIInputAngleKey: Double.pi])
            ciContext.render(ci, to: out, bounds: CGRect(x: 0, y: 0, width: s.width, height: s.height),
                             colorSpace: CGColorSpace(name: CGColorSpace.sRGB))
            tag(out)
            return out
        }
    }

    /// Tiles `rects` of the page raster `body` draws, drawn with the page's
    /// own context (the configuration of `bitmapContext`, the same device
    /// coordinates) but clipped to `rects`, then copied out. For clip-exact
    /// pages only (`clipExact`).
    ///
    /// The context spans the page (its device origin is the page's
    /// bottom-left corner, so nothing is translated), but its memory is an
    /// anonymous mapping, which the system backs only where it is written:
    /// the clip and the white fill cover just the rects, so the resident size
    /// is about the rects' rows (`lastCutResidentBytes`), at any scale.
    static func clippedTiles(widthPt: Double, heightPt: Double, scale: Double, rects: [DL3PixelRect],
                             background: CGColor = CGColor(gray: 1, alpha: 1), _ body: (CGContext) -> Void) -> [IOSurface?] {
        guard let (W, H, size) = DL3PageRaster.checkedSize(widthPt: widthPt, heightPt: heightPt, scale: scale) else { return rects.map { _ in nil } }
        let ok = rects.filter { $0.width > 0 && $0.height > 0 && $0.x >= 0 && $0.y >= 0 && $0.x + $0.width <= W && $0.y + $0.height <= H }
        let stride = W * 4
        guard !ok.isEmpty, W > 0, H > 0 else { return rects.map { _ in nil } }
        let mem = mmap(nil, size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0)
        guard let mem, mem != MAP_FAILED else { return rects.map { _ in nil } }
        defer { munmap(mem, size) }
        guard let ctx = CGContext(data: mem, width: W, height: H, bitsPerComponent: 8, bytesPerRow: stride,
                                  space: tileSpace(), bitmapInfo: Layout.screen.bitmapInfo) else { return rects.map { _ in nil } }
        // Device rects (y up) of the requested tiles: clip, then the page's ground.
        let device = ok.map { CGRect(x: $0.x, y: H - $0.y - $0.height, width: $0.width, height: $0.height) }
        ctx.clip(to: device)
        ctx.setFillColor(background)
        ctx.fill(device)
        configurePage(ctx, scale: scale)
        body(ctx)
        ctx.flush()
        if measureResidency { recordResidency(resident(mem, size)) }
        return copyTiles(rects, from: mem.assumingMemoryBound(to: UInt8.self), width: W, height: H)
    }

    /// The page context configuration after the background (as `bitmapContext`).
    static func configurePage(_ ctx: CGContext, scale: Double) {
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        ctx.setShouldSmoothFonts(false)
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
    }

    /// Each rect of a BGRA page raster (top row first) copied into its own surface.
    static func copyTiles(_ rects: [DL3PixelRect], from base: UnsafePointer<UInt8>, width W: Int, height H: Int) -> [IOSurface?] {
        let stride = W * 4
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

    /// Resident bytes of a mapping (`mincore`).
    static func resident(_ mem: UnsafeMutableRawPointer, _ size: Int) -> Int {
        let page = Int(getpagesize())
        var vec = [CChar](repeating: 0, count: (size + page - 1) / page)
        guard mincore(mem, size, &vec) == 0 else { return 0 }
        return vec.reduce(0) { $0 + (($1 & 1) != 0 ? page : 0) }
    }

    /// Measurement (benches, tests): the resident size of each clipped
    /// raster. Every access is under `residencyLock`, so tile jobs may read
    /// the flag off the main thread.
    private static let residencyLock = NSLock()
    nonisolated(unsafe) private static var _measuring = false, _lastResident = 0, _maxResident = 0
    public static var measureResidency: Bool {
        get { residencyLock.lock(); defer { residencyLock.unlock() }; return _measuring }
        set { residencyLock.lock(); _measuring = newValue; residencyLock.unlock() }
    }
    /// Resident bytes of the last clipped raster, and the largest since `resetResidency`.
    public static var lastCutResidentBytes: Int { residencyLock.lock(); defer { residencyLock.unlock() }; return _lastResident }
    public static var maxCutResidentBytes: Int { residencyLock.lock(); defer { residencyLock.unlock() }; return _maxResident }
    public static func resetResidency() { residencyLock.lock(); _lastResident = 0; _maxResident = 0; residencyLock.unlock() }
    static func recordResidency(_ bytes: Int) {
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

/// One full-scale page raster, kept by the pane for a page drawn whole
/// (paths, clips, images, forms, stroked text; PDF fallbacks) and cut into
/// tiles by every tile job of that page's source: drawn once per page
/// content, scale and appearance, not once per job.
///
/// The pixels are anonymous **purgeable** memory (`mach_vm_allocate` with
/// `VM_FLAGS_PURGABLE`), never a file: nothing is written to disk (a
/// file-backed mapping wrote the whole raster to the SSD on every redraw,
/// review of #1287 @2cd7cfc8e). The raster is nonvolatile while it is drawn,
/// until its first cut, and while tiles are cut (`cut`), and volatile otherwise: volatile
/// pages are not part of the process's footprint, and the kernel may purge
/// them under memory pressure. `cut` reports a purged raster (nil), and its
/// owner draws it again. Its pixels are exactly `rasterizeToSurface`'s (same
/// configuration, ground and coordinates).
public final class DL3PageRaster: @unchecked Sendable {
    public let width: Int, height: Int
    public let scale: Double
    private let address: mach_vm_address_t
    private let size: Int
    private let lock = NSLock()

    /// The page raster's pixel size at `scale`, or nil when the scale is not
    /// finite and positive or the size does not fit (`W·4·H` overflows, or
    /// beyond 1 Mpx a side).
    public static func checkedSize(widthPt: Double, heightPt: Double, scale: Double) -> (width: Int, height: Int, bytes: Int)? {
        guard scale.isFinite, scale > 0, widthPt.isFinite, heightPt.isFinite, widthPt > 0, heightPt > 0 else { return nil }
        let w = (widthPt * scale).rounded(.up), h = (heightPt * scale).rounded(.up)
        guard w >= 1, h >= 1, w <= Double(1 << 20), h <= Double(1 << 20) else { return nil }
        let (row, o1) = Int(w).multipliedReportingOverflow(by: 4)
        let (bytes, o2) = row.multipliedReportingOverflow(by: Int(h))
        guard !o1, !o2 else { return nil }
        return (Int(w), Int(h), bytes)
    }

    init?(widthPt: Double, heightPt: Double, scale: Double, background: CGColor = CGColor(gray: 1, alpha: 1), _ body: (CGContext) -> Void) {
        guard let (W, H, size) = Self.checkedSize(widthPt: widthPt, heightPt: heightPt, scale: scale) else { return nil }
        var addr: mach_vm_address_t = 0
        guard mach_vm_allocate(mach_task_self_, &addr, mach_vm_size_t(size), VM_FLAGS_ANYWHERE | VM_FLAGS_PURGABLE) == KERN_SUCCESS,
              let mem = UnsafeMutableRawPointer(bitPattern: UInt(addr)) else { return nil }
        guard let ctx = CGContext(data: mem, width: W, height: H, bitsPerComponent: 8, bytesPerRow: W * 4,
                                  space: DL3Renderer.tileSpace(), bitmapInfo: DL3Renderer.Layout.screen.bitmapInfo) else {
            mach_vm_deallocate(mach_task_self_, addr, mach_vm_size_t(size)); return nil
        }
        ctx.setFillColor(background)
        ctx.fill(CGRect(x: 0, y: 0, width: W, height: H))
        DL3Renderer.configurePage(ctx, scale: scale)
        body(ctx)
        ctx.flush()
        // Nonvolatile until the first `cut` (which leaves it volatile): a raster
        // is never purged between being drawn and being used.
        self.width = W; self.height = H; self.scale = scale; self.address = addr; self.size = size
    }

    /// `prepared` drawn whole at `scale` in `appearance` (as `rasterizeToSurface`).
    public convenience init?(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double, appearance: DL3Appearance = .light) {
        self.init(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, background: appearance.background) {
            DL3Renderer.draw(prepared, forms: forms, in: $0, appearance: appearance)
        }
    }

    /// A PDF page (the fallback) drawn whole at `scale`, light: its grid is
    /// its media box's (`gridSize(pdfPage:scale:)`); a dark pane passes its
    /// tiles through `DL3Renderer.pdfTiles`.
    public convenience init?(pdfPage: CGPDFPage, scale: Double) {
        let box = pdfPage.getBoxRect(.mediaBox)
        self.init(widthPt: box.width, heightPt: box.height, scale: scale) { ctx in
            ctx.translateBy(x: -box.minX, y: -box.minY)
            ctx.drawPDFPage(pdfPage)
        }
    }

    /// The pixel grid a PDF page's raster (and so its tiles) has at `scale`.
    public static func gridSize(pdfPage: CGPDFPage, scale: Double) -> (width: Int, height: Int) {
        let box = pdfPage.getBoxRect(.mediaBox)
        return DL3Renderer.pixelSize(widthPt: box.width, heightPt: box.height, scale: scale)
    }

    deinit { mach_vm_deallocate(mach_task_self_, address, mach_vm_size_t(size)) }

    /// Sets the purgeable state; returns the previous one.
    private func setState(_ s: Int32) -> Int32 {
        var state = s
        guard mach_vm_purgable_control(mach_task_self_, address, VM_PURGABLE_SET_STATE, &state) == KERN_SUCCESS else { return -1 }
        return state
    }

    /// Bytes of the raster.
    public var bytes: Int { size }
    /// Whether the kernel purged it (tests).
    public var purged: Bool {
        lock.lock(); defer { lock.unlock() }
        var state: Int32 = 0
        guard mach_vm_purgable_control(mach_task_self_, address, VM_PURGABLE_GET_STATE, &state) == KERN_SUCCESS else { return true }
        return state == VM_PURGABLE_EMPTY
    }
    /// Bytes of it resident now (`mincore`).
    public var residentBytes: Int {
        guard let mem = UnsafeMutableRawPointer(bitPattern: UInt(address)) else { return 0 }
        return DL3Renderer.resident(mem, size)
    }

    /// Purges it now, as the kernel may under memory pressure (tests).
    public func purgeForTesting() {
        lock.lock(); defer { lock.unlock() }
        var state: Int32 = VM_PURGABLE_EMPTY
        _ = mach_vm_purgable_control(mach_task_self_, address, VM_PURGABLE_SET_STATE, &state)
    }

    /// Tiles `rects` (page pixels, top-left origin), each copied into its own
    /// surface; nil when the kernel purged the raster (draw it again).
    public func cut(_ rects: [DL3PixelRect]) -> [IOSurface?]? {
        lock.lock(); defer { lock.unlock() }
        guard let mem = UnsafeMutableRawPointer(bitPattern: UInt(address)) else { return nil }
        let previous = setState(VM_PURGABLE_NONVOLATILE)
        defer { _ = setState(VM_PURGABLE_VOLATILE) }
        guard previous != VM_PURGABLE_EMPTY, previous >= 0 else { return nil }
        return DL3Renderer.copyTiles(rects, from: mem.assumingMemoryBound(to: UInt8.self), width: width, height: height)
    }
}
