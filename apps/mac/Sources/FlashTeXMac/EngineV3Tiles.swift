import AppKit
import IOSurface
import QuartzCore
import FlashTeXDisplayListV3
import FlashTeXPreviewV3

// Zoom and high-zoom tiles for the engine-v3 pane (DESIGN.md §6.2, lane
// P3-V3-ZOOM-TILES; the preview-v2 design of #1228 brought to v3).
//
// Above `EngineV3TileGrid.threshold` pixels per point a page on screen is
// not one bitmap but 512 px tiles of its visible area (plus a prefetch
// margin), over a 2 px/pt backdrop of the whole page. Tiles are drawn off
// the main thread, in parallel (`DL3Renderer.rasterizeTiles`), as IOSurfaces
// already in Core Animation's pixel format and tagged sRGB, so the commit
// neither copies nor converts them. The main thread only assigns layer
// contents, and only for the tile generation it queued: a job for a page,
// scale or content the view has left is skipped before it draws, and its
// result is dropped if the view moved on while it drew. Each tile is a
// pixel-exact window of the whole-page raster (TileParityTests), so the
// zero-tolerance parity gate covers tiled pages.

enum EngineV3TileGrid {
    static let tilePixels = 512
    /// Pixels per point above which pages tile. `FLASHTEX_V3_TILE_THRESHOLD`
    /// overrides it for measurements (a huge value turns tiling off).
    static let threshold: Double = Double(ProcessInfo.processInfo.environment["FLASHTEX_V3_TILE_THRESHOLD"] ?? "") ?? 3
    /// The whole-page bitmap under a tiled page's tiles, shown (stretched,
    /// linear) wherever no tile has landed yet.
    static let backdropPixelsPerPoint = 2.0
    /// Tiles within this many pixels of the visible rect are drawn too, so a
    /// scroll step usually finds its next row ready.
    static let prefetchPixels: CGFloat = 256
    /// Tile jobs run here in order (visible tiles first); each job draws its
    /// tiles in parallel.
    static let queue = DispatchQueue(label: "flashtex.engine-v3.tiles", qos: .userInteractive)

    struct Index: Hashable { var column: Int; var row: Int }

    static func tiles(_ pixelsPerPoint: Double) -> Bool { pixelsPerPoint > threshold }

    static func rect(_ i: Index, pageWidth: Int, pageHeight: Int) -> DL3PixelRect {
        let x = i.column * tilePixels, y = i.row * tilePixels
        return DL3PixelRect(x: x, y: y, width: min(tilePixels, pageWidth - x), height: min(tilePixels, pageHeight - y))
    }

    /// The tiles meeting `pixels` (page pixels, top-left origin), row-major.
    static func indices(covering pixels: CGRect, pageWidth: Int, pageHeight: Int) -> [Index] {
        let r = pixels.intersection(CGRect(x: 0, y: 0, width: pageWidth, height: pageHeight))
        guard !r.isNull, r.width > 0, r.height > 0 else { return [] }
        let c0 = Int(r.minX.rounded(.down)) / tilePixels, c1 = (Int(r.maxX.rounded(.up)) - 1) / tilePixels
        let r0 = Int(r.minY.rounded(.down)) / tilePixels, r1 = (Int(r.maxY.rounded(.up)) - 1) / tilePixels
        var out: [Index] = []
        out.reserveCapacity((c1 - c0 + 1) * (r1 - r0 + 1))
        for row in r0 ... r1 { for column in c0 ... c1 { out.append(Index(column: column, row: row)) } }
        return out
    }

    /// Actions off: contents, frames and filters change without implicit fades.
    static let noActions: [String: CAAction] = ["contents": NSNull(), "position": NSNull(), "bounds": NSNull(), "frame": NSNull(),
                                                "hidden": NSNull(), "magnificationFilter": NSNull(), "minificationFilter": NSNull(),
                                                "sublayers": NSNull(), "onOrderIn": NSNull(), "onOrderOut": NSNull()]

    // Counters for the scroll bench and tests (main thread).
    @MainActor static var jobs = 0
    @MainActor static var jobTiles = 0
    @MainActor static var jobMs = 0.0
    @MainActor static var maxJobMs = 0.0
    /// Queue-to-install latency of the slowest job (ms).
    @MainActor static var maxLatencyMs = 0.0
    /// Queued tiles skipped undrawn (no longer wanted when their job ran).
    @MainActor static var skippedTiles = 0
    /// Tile jobs with tiles that could not be drawn (retried).
    @MainActor static var failedTiles = 0
    /// New contents of pages drawn whole held back by the redraw throttle.
    @MainActor static var deferredSources = 0
    /// Per new source (page entering, zoom step, edit): ms from its first tile job queued to its first tiles on screen.
    @MainActor static var firstTileMs: [Double] = []
    @MainActor static func resetCounters() { jobs = 0; jobTiles = 0; jobMs = 0; maxJobMs = 0; maxLatencyMs = 0; skippedTiles = 0; firstTileMs = []; failedTiles = 0; deferredSources = 0 }
}

/// What a tiled page shows: one page's content at one scale. Tiles of equal
/// sources are interchangeable.
struct EngineV3TileSource: @unchecked Sendable {
    let prepared: DL3PreparedPage
    let forms: [UInt32: DL3PreparedPage]
    /// The PDF page drawn instead of the display list (fallback pages).
    let pdf: CGPDFPage?
    /// The content key (hash; fallback and form revisions included).
    let key: [UInt8]
    /// The scale the tiles are drawn at: the screen's (below it only for a
    /// page drawn whole whose raster would exceed 1 GiB, `DL3Renderer.tileScale`).
    let pixelsPerPoint: Double
    /// Backing pixels per view point.
    let displayScale: Double
    /// The page's pixels per point on screen.
    let screenPixelsPerPoint: Double
    /// Light or dark (the pane's preview appearance; part of `key`).
    var appearance: DL3Appearance = .light

    /// Tile pixels per view point: the backing scale, less when the tiles
    /// are drawn below the screen's scale (then stretched, linearly).
    var pixelsPerViewPoint: Double { displayScale * pixelsPerPoint / screenPixelsPerPoint }
    var stretched: Bool { pixelsPerPoint < screenPixelsPerPoint }

    /// The pixel grid the tiles partition: the raster's own (a PDF
    /// fallback's is its media box's, which its `DL3PageRaster` has).
    var pixelSize: (width: Int, height: Int) {
        if let pdf { return DL3PageRaster.gridSize(pdfPage: pdf, scale: pixelsPerPoint) }
        return DL3Renderer.pixelSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: pixelsPerPoint)
    }

    /// Whether the tiles are cut from one kept full-scale page raster
    /// (paths, clips, images, forms, stroked text; PDF fallbacks).
    var drawnWhole: Bool { pdf != nil || (!DL3Renderer.tilesByTranslation(prepared) && !DL3Renderer.clipExact(prepared)) }

    func sameTiles(as o: EngineV3TileSource?) -> Bool {
        guard let o else { return false }
        return o.key == key && o.pixelsPerPoint == pixelsPerPoint && o.displayScale == displayScale && o.screenPixelsPerPoint == screenPixelsPerPoint
    }

    /// The same pixel grid in the same place: a held tile stays up, at its
    /// place, until its replacement for this source lands.
    func sameGeometry(as o: EngineV3TileSource?) -> Bool {
        guard let o else { return false }
        return o.pixelsPerPoint == pixelsPerPoint && o.displayScale == displayScale && o.screenPixelsPerPoint == screenPixelsPerPoint
            && o.prepared.widthPt == prepared.widthPt && o.prepared.heightPt == prepared.heightPt
    }

    /// On the tile queue only. A page drawn whole is cut from `raster`'s
    /// kept page raster for this source's `rasterIdentity`, drawn on first
    /// use and again if the kernel purged it. Nil surfaces mean the raster
    /// could not be drawn (no memory): the caller asks again later.
    func render(_ rects: [DL3PixelRect], raster: EngineV3RasterHolder) -> [IOSurface?] {
        guard drawnWhole else { return DL3Renderer.rasterizeTiles(prepared, forms: forms, scale: pixelsPerPoint, rects: rects, appearance: appearance) }
        let cut = raster.cut(rects, identity: rasterIdentity, scale: pixelsPerPoint) {
            if let pdf { return DL3PageRaster(pdfPage: pdf, scale: pixelsPerPoint) }
            return DL3PageRaster(prepared, forms: forms, scale: pixelsPerPoint, appearance: appearance)
        }
        guard let cut else { return rects.map { _ in nil } }
        return pdf != nil ? DL3Renderer.pdfTiles(cut, appearance: appearance) : cut
    }

    /// What the kept raster shows: the content key and scale, but not the
    /// appearance for a PDF fallback (its raster is always light; dark is a
    /// per-tile pass), so a dark toggle does not draw an identical raster.
    var rasterIdentity: [UInt8] { (pdf != nil ? pdfIdentity : key) }
    /// The key without the appearance (`EngineV3PagesView` builds the PDF key so).
    var pdfIdentity: [UInt8] = []
}

/// A page's kept full-scale raster (`DL3PageRaster`), for the identity it was
/// drawn for. Confined to the tile queue (serial).
///
/// - At most `budget` rasters are kept over all pages (least recently cut
///   first out); each is purgeable, so an idle one costs no footprint and
///   the kernel may purge it, after which it is drawn again.
/// - A new identity (content, scale, appearance) frees the old raster before
///   drawing; the page frees it when its tiles leave the keep set or it is
///   torn down (`release`).
final class EngineV3RasterHolder: @unchecked Sendable {
    private var identity: [UInt8]?
    private var scale = 0.0
    private var kept: DL3PageRaster?
    /// Rasters drawn (tests: one per source, not one per job; one more after a purge).
    private(set) var drawn = 0

    /// Kept rasters over all pages: at most this many (`FLASHTEX_V3_KEPT_RASTERS`).
    static let budget = max(1, Int(ProcessInfo.processInfo.environment["FLASHTEX_V3_KEPT_RASTERS"] ?? "") ?? 2)
    /// Holders with a raster, least recently cut first (tile queue only).
    nonisolated(unsafe) private static var lru: [EngineV3RasterHolder] = []

    private func touch() {
        Self.lru.removeAll { $0 === self }
        Self.lru.append(self)
        while Self.lru.count > Self.budget { let old = Self.lru.removeFirst(); old.kept = nil; old.identity = nil }
    }

    private func drop() { kept = nil; identity = nil; Self.lru.removeAll { $0 === self } }

    /// Tiles `rects` of the raster for `identity` at `scale`, drawn by `make`
    /// first if it is missing, of another identity, or purged. Nil when it
    /// cannot be drawn.
    func cut(_ rects: [DL3PixelRect], identity: [UInt8], scale: Double, _ make: () -> DL3PageRaster?) -> [IOSurface?]? {
        dispatchPrecondition(condition: .onQueue(EngineV3TileGrid.queue))
        for attempt in 0 ..< 2 {
            if kept == nil || self.identity != identity || self.scale != scale || attempt > 0 {
                drop() // free the previous (or purged) raster before drawing the next
                if Self.failDraws > 0 { Self.failDraws -= 1; return nil } // (tests: memory not available)
                guard let r = make() else { return nil }
                kept = r; self.identity = identity; self.scale = scale
                drawn += 1
                Self.noteDrawn(r)
            }
            touch()
            if let tiles = kept?.cut(rects) { return tiles }
            Self.notePurged() // the kernel purged it: draw again (once)
        }
        return nil
    }

    /// Frees the raster (on the tile queue, after jobs already queued).
    func release() { EngineV3TileGrid.queue.async { self.drop() } }

    var holding: Bool { EngineV3TileGrid.queue.sync { kept != nil } }
    var rastersDrawn: Int { EngineV3TileGrid.queue.sync { drawn } }
    /// Purges the kept raster as the kernel may (tests).
    func purgeForTesting() { EngineV3TileGrid.queue.sync { kept?.purgeForTesting() } }
    static var keptCount: Int { EngineV3TileGrid.queue.sync { lru.count } }
    /// Tests: the next this many raster draws fail, as when memory is not available.
    nonisolated(unsafe) private static var failDraws = 0
    static func failNextDrawsForTesting(_ n: Int) { EngineV3TileGrid.queue.sync { failDraws = n } }

    /// Measurement: the largest kept raster (bytes), purges seen.
    private static let statsLock = NSLock()
    nonisolated(unsafe) private static var _maxBytes = 0, _purges = 0
    static var maxRasterBytes: Int { statsLock.lock(); defer { statsLock.unlock() }; return _maxBytes }
    static var purges: Int { statsLock.lock(); defer { statsLock.unlock() }; return _purges }
    static func resetStats() { statsLock.lock(); _maxBytes = 0; _purges = 0; statsLock.unlock() }
    private static func noteDrawn(_ r: DL3PageRaster) { statsLock.lock(); _maxBytes = max(_maxBytes, r.bytes); statsLock.unlock() }
    private static func notePurged() { statsLock.lock(); _purges += 1; statsLock.unlock() }
}

/// A page view's tile generation and the tiles it still wants (its keep
/// set), readable from the tile queue: a queued job draws only tiles of the
/// current generation that are still wanted, so a fast scroll or a dropped
/// page view leaves no backlog of drawing behind it.
final class EngineV3TileGeneration: @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0
    private var wanted: Set<EngineV3TileGrid.Index> = []
    func bump() -> Int { lock.lock(); defer { lock.unlock() }; value &+= 1; wanted = []; return value }
    var current: Int { lock.lock(); defer { lock.unlock() }; return value }
    func setWanted(_ w: Set<EngineV3TileGrid.Index>) { lock.lock(); wanted = w; lock.unlock() }
    /// The positions in `indices` still wanted by generation `g` (empty if it is not current).
    func live(_ indices: [EngineV3TileGrid.Index], generation g: Int) -> [Int] {
        lock.lock(); defer { lock.unlock() }
        guard value == g else { return [] }
        return indices.indices.filter { wanted.contains(indices[$0]) }
    }
}

/// The tiles of one page view: sublayers of a container that fills the page
/// (and clips the last row and column, which overhang it by under a pixel).
@MainActor
final class EngineV3PageTiles {
    let container: CALayer
    private(set) var source: EngineV3TileSource?
    private let generation = EngineV3TileGeneration()
    private var token = 0
    /// The page's kept raster, for a page drawn whole (tile queue only).
    let raster = EngineV3RasterHolder()
    /// The visible rect of the last update (a job whose tiles were skipped
    /// updates again with it: they may be wanted again by then).
    private var lastVisible: CGRect = .null
    /// Redraw throttle for a page drawn whole (leading edge, trailing redraw):
    /// new content is drawn at once unless a redraw of this page started less
    /// than `throttleWindow` ago; then the tiles stay up (stale) and one
    /// trailing redraw takes the newest content once edits pause for
    /// `trailingPause`, or when the window expires, whichever comes first.
    /// Occasional edits pay no delay; fast typing draws about twice a second.
    /// `FLASHTEX_V3_WHOLE_THROTTLE_MS` (500; 0 turns it off) and
    /// `FLASHTEX_V3_WHOLE_PAUSE_MS` (300).
    static let throttleWindow = (Double(ProcessInfo.processInfo.environment["FLASHTEX_V3_WHOLE_THROTTLE_MS"] ?? "") ?? 500) / 1000
    static let trailingPause = (Double(ProcessInfo.processInfo.environment["FLASHTEX_V3_WHOLE_PAUSE_MS"] ?? "") ?? 300) / 1000
    /// When the current content of a page drawn whole was last applied (a redraw started).
    private var lastWholeApplyNs: UInt64 = 0
    /// The newest content held back by the throttle, with its compile.
    private(set) var deferred: (EngineV3TileSource, Int?)?
    private var deferredGeneration = 0
    private var forceNext = false
    /// The current source awaits its first tiles; when its first job was queued.
    private var awaitingFirst = false
    private var shownNs: UInt64?
    private var requested: Set<EngineV3TileGrid.Index> = []
    private(set) var layers: [EngineV3TileGrid.Index: CALayer] = [:]
    /// Tiles of an earlier content of the same geometry: up until replaced.
    private var stale: Set<EngineV3TileGrid.Index> = []
    /// Tiles of an earlier scale, re-framed (stretched, linear) under the
    /// current tiles until the visible ones of the current scale are up.
    /// `points`: the tile's rectangle in page points, top-left origin.
    private var outgoing: [(layer: CALayer, points: CGRect)] = []
    static let maxOutgoing = 96
    /// A keystroke's compile whose visible tiles are awaited (latency is
    /// stamped when they are installed).
    private var pendingCompile: Int?
    var pinching = false { didSet { if pinching != oldValue { setFilters() } } }
    /// (compile, raster start, commit) when a keystroke's visible tiles are up.
    var onCommitted: ((Int, UInt64, UInt64) -> Void)?
    /// Tiles installed over this page's life, jobs queued (tests, bench).
    private(set) var installed = 0
    private(set) var jobsQueued = 0

    init() {
        container = CALayer()
        container.actions = EngineV3TileGrid.noActions
        container.masksToBounds = true
    }

    /// Queued jobs of a torn-down page skip undrawn; its raster is freed.
    deinit {
        _ = generation.bump()
        raster.release()
    }

    var count: Int { layers.count }
    var pending: Int { requested.count }
    /// Bytes of the surfaces held (tiles, outgoing ones included).
    var retainedBytes: Int {
        func bytes(_ l: CALayer) -> Int { (l.contents as! IOSurface?)?.allocationSize ?? 0 }
        return layers.values.reduce(0) { $0 + bytes($1) } + outgoing.reduce(0) { $0 + bytes($1.layer) }
    }

    func tileImage(_ i: EngineV3TileGrid.Index) -> IOSurface? { layers[i]?.contents as! IOSurface? }

    /// Shows `new` (a new content, scale or backing scale; the same source
    /// is a no-op) and requests the missing visible tiles.
    /// `visible`: the page view's visible rect (view points, top-left origin).
    func show(_ new: EngineV3TileSource, visible: CGRect, compileID: Int?) {
        // New content of a page drawn whole (same scale and appearance): a
        // leading-edge throttle with a trailing redraw (`throttleWindow`).
        let now = MonotonicClock.nowNs()
        let windowNs = UInt64(Self.throttleWindow * 1e9)
        if new.drawnWhole, let old = source, !new.sameTiles(as: old), new.sameGeometry(as: old), new.appearance == old.appearance,
           Self.throttleWindow > 0, !forceNext, deferred != nil || now &- lastWholeApplyNs < windowNs {
            // The same pending content shown again (a scroll, another page's
            // arrival): the trailing redraw stays scheduled as it is.
            if let (pending, compile) = deferred, pending.sameTiles(as: new) {
                if let compileID, compileID != compile { deferred = (pending, compileID) }
                update(visible: visible)
                return
            }
            // (A re-show without a compile, e.g. the PDF fallback swapped in at
            // DONE, keeps the keystroke's compile pending: its latency is
            // stamped when these tiles land.)
            deferred = (new, compileID ?? deferred?.1 ?? pendingCompile)
            deferredGeneration &+= 1
            let g = deferredGeneration
            EngineV3TileGrid.deferredSources += 1
            // The trailing redraw: after the pause, but no later than the window's end.
            let windowLeft = Double(windowNs &- min(windowNs, now &- lastWholeApplyNs)) / 1e9
            let delay = min(Self.trailingPause, max(0, windowLeft))
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
                MainActor.assumeIsolated {
                    guard let self, self.deferredGeneration == g, let (pending, compile) = self.deferred else { return }
                    self.deferred = nil
                    self.forceNext = true
                    self.show(pending, visible: self.lastVisible.isNull ? visible : self.lastVisible, compileID: compile)
                    self.forceNext = false
                }
            }
            update(visible: visible)
            return
        }
        deferred = nil
        deferredGeneration &+= 1
        if let compileID { pendingCompile = compileID }
        if !new.sameTiles(as: source) {
            failedRetries = 0
            if new.drawnWhole { lastWholeApplyNs = MonotonicClock.nowNs() } // a redraw of the page starts
            if new.sameGeometry(as: source) {
                stale.formUnion(layers.keys)
            } else if let old = source {
                retire(old)
            }
            source = new
            token = generation.bump()
            requested.removeAll()
            // (The kept raster stays: the next job draws a new one only if the
            // raster identity changed, so a PDF page's dark toggle keeps it.)
            awaitingFirst = true
            shownNs = nil
            reframeOutgoing(new)
        }
        update(visible: visible)
    }

    /// Drops every tile (the page left the tiled scales or the screen).
    func removeAll() {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for l in layers.values { l.removeFromSuperlayer() }
        for o in outgoing { o.layer.removeFromSuperlayer() }
        CATransaction.commit()
        layers.removeAll(); stale.removeAll(); outgoing.removeAll(); requested.removeAll()
        source = nil
        token = generation.bump()
        pendingCompile = nil
        lastVisible = .null
        deferred = nil
        deferredGeneration &+= 1
        raster.release()
    }

    /// Visible tiles (current source) not yet up: the backdrop or the
    /// previous scale shows there.
    func missingVisible(_ visible: CGRect) -> Int {
        guard let source else { return 0 }
        return wanted(source, visible).visible.filter { layers[$0] == nil }.count
    }

    private func wanted(_ s: EngineV3TileSource, _ visible: CGRect) -> (visible: Set<EngineV3TileGrid.Index>, want: [EngineV3TileGrid.Index], keep: Set<EngineV3TileGrid.Index>) {
        guard !visible.isEmpty else { return ([], [], []) }
        let (pw, ph) = s.pixelSize
        let ds = CGFloat(s.pixelsPerViewPoint)
        let px = CGRect(x: visible.minX * ds, y: visible.minY * ds, width: visible.width * ds, height: visible.height * ds)
        let m = EngineV3TileGrid.prefetchPixels, t = CGFloat(EngineV3TileGrid.tilePixels)
        return (Set(EngineV3TileGrid.indices(covering: px, pageWidth: pw, pageHeight: ph)),
                EngineV3TileGrid.indices(covering: px.insetBy(dx: -m, dy: -m), pageWidth: pw, pageHeight: ph),
                Set(EngineV3TileGrid.indices(covering: px.insetBy(dx: -m - t, dy: -m - t), pageWidth: pw, pageHeight: ph)))
    }

    /// Brings the tiles in line with the visible rect, drawing nothing here:
    /// tiles beyond a one-tile margin go, and the missing or stale tiles of
    /// the visible rect, then of the prefetch margin, are queued as jobs.
    func update(visible: CGRect) {
        guard let s = source else { return }
        lastVisible = visible
        let (vis, want, keep) = wanted(s, visible)
        generation.setWanted(keep) // queued jobs skip tiles that left it
        if keep.isEmpty, s.drawnWhole { raster.release() } // the page left the keep set
        let wantSet = Set(want)
        let drop = layers.keys.filter { !keep.contains($0) || (stale.contains($0) && !wantSet.contains($0)) }
        if !drop.isEmpty {
            CATransaction.begin(); CATransaction.setDisableActions(true)
            for i in drop { layers.removeValue(forKey: i)?.removeFromSuperlayer(); stale.remove(i) }
            CATransaction.commit()
        }
        retireOutgoingIfCovered(vis, visible: visible)
        if pendingCompile != nil, !vis.isEmpty, vis.allSatisfy({ layers[$0] != nil && !stale.contains($0) }) { pendingCompile = nil }
        let missing = want.filter { (layers[$0] == nil || stale.contains($0)) && !requested.contains($0) }
        guard !missing.isEmpty else { return }
        let now = missing.filter { vis.contains($0) }, later = missing.filter { !vis.contains($0) }
        if !now.isEmpty { request(now, source: s, visible: visible, compile: pendingCompile) }
        if !later.isEmpty { request(later, source: s, visible: visible, compile: nil) }
    }

    private var failedRetries = 0
    /// Failed tiles of generation `token`: update again after a delay.
    private func retryFailed(token: Int) {
        guard failedRetries < 8 else { return }
        failedRetries += 1
        EngineV3TileGrid.failedTiles += 1
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.25 * Double(failedRetries)) { [weak self] in
            MainActor.assumeIsolated {
                guard let self, self.token == token, !self.lastVisible.isNull else { return }
                self.update(visible: self.lastVisible)
            }
        }
    }

    private func request(_ indices: [EngineV3TileGrid.Index], source s: EngineV3TileSource, visible: CGRect, compile: Int?) {
        requested.formUnion(indices)
        jobsQueued += 1
        if awaitingFirst, shownNs == nil { shownNs = MonotonicClock.nowNs() }
        let expected = token, generation = self.generation, raster = self.raster
        let (pw, ph) = s.pixelSize
        let rects = indices.map { EngineV3TileGrid.rect($0, pageWidth: pw, pageHeight: ph) }
        let queued = MonotonicClock.nowNs()
        EngineV3TileGrid.queue.async { [weak self] in
            // Only tiles still wanted by the current generation: a page, scale
            // or content change, a dropped page view or a scroll past them
            // skips them undrawn.
            let live = generation.live(indices, generation: expected)
            var surfaces = [IOSurface?](repeating: nil, count: indices.count)
            let t0 = MonotonicClock.nowNs()
            if !live.isEmpty {
                dispatchPrecondition(condition: .notOnQueue(.main)) // DESIGN §1.2: no drawing on main
                for (k, v) in zip(live, s.render(live.map { rects[$0] }, raster: raster)) { surfaces[k] = v }
            }
            let ms = Double(MonotonicClock.nowNs() &- t0) / 1e6
            let skipped = indices.count - live.count
            let failed = live.filter { surfaces[$0] == nil }.count // could not be drawn (no memory)
            EngineV3Session.onMain {
                EngineV3TileGrid.skippedTiles += skipped
                // (`requested` is cleared on a generation change; otherwise the
                // skipped tiles become requestable again here.)
                guard let self, self.token == expected, generation.current == expected, let current = self.source else { return }
                self.requested.subtract(indices)
                // Tiles skipped because they had left the keep set may be
                // back in it by now (a scroll back): request them again.
                defer { if skipped > 0, !self.lastVisible.isNull { self.update(visible: self.lastVisible) } }
                // Tiles that could not be drawn (the raster's memory was not
                // available) are asked for again shortly, a bounded number of
                // times per source; their place shows the backdrop meanwhile.
                if failed > 0 { self.retryFailed(token: expected) }
                guard !live.isEmpty else { return }
                self.install(surfaces, at: indices, source: current, compile: compile, t0: t0)
                EngineV3TileGrid.jobs += 1
                EngineV3TileGrid.jobTiles += live.count
                EngineV3TileGrid.jobMs += ms
                EngineV3TileGrid.maxJobMs = max(EngineV3TileGrid.maxJobMs, ms)
                EngineV3TileGrid.maxLatencyMs = max(EngineV3TileGrid.maxLatencyMs, Double(MonotonicClock.nowNs() &- queued) / 1e6)
            }
        }
    }

    /// Compositing only: each surface becomes a tile layer's contents.
    private func install(_ surfaces: [IOSurface?], at indices: [EngineV3TileGrid.Index], source s: EngineV3TileSource, compile: Int?, t0: UInt64) {
        let (pw, ph) = s.pixelSize
        let ds = CGFloat(s.pixelsPerViewPoint)
        let flipped = container.contentsAreFlipped()
        let height = container.bounds.height
        let filter: CALayerContentsFilter = pinching || s.stretched ? .linear : .nearest
        CATransaction.begin(); CATransaction.setDisableActions(true)
        var n = 0
        for (i, surface) in zip(indices, surfaces) {
            guard let surface else { continue }
            let tile: CALayer
            if let existing = layers[i] { tile = existing } else {
                tile = CALayer()
                tile.actions = EngineV3TileGrid.noActions
                tile.contentsGravity = .resize
                tile.isOpaque = true
                container.addSublayer(tile) // above any outgoing tiles
                layers[i] = tile
            }
            let r = EngineV3TileGrid.rect(i, pageWidth: pw, pageHeight: ph)
            tile.magnificationFilter = filter
            tile.minificationFilter = filter
            tile.contentsScale = ds
            let y = flipped ? CGFloat(r.y) / ds : height - CGFloat(r.y + r.height) / ds
            tile.frame = CGRect(x: CGFloat(r.x) / ds, y: y, width: CGFloat(r.width) / ds, height: CGFloat(r.height) / ds)
            tile.contents = surface
            stale.remove(i)
            n += 1
        }
        CATransaction.commit()
        installed += n
        if n > 0, awaitingFirst, let shown = shownNs {
            awaitingFirst = false
            shownNs = nil
            EngineV3TileGrid.firstTileMs.append(Double(MonotonicClock.nowNs() &- shown) / 1e6)
        }
        if let compile, compile == pendingCompile {
            pendingCompile = nil
            onCommitted?(compile, t0, MonotonicClock.nowNs())
        }
    }

    private func retire(_ old: EngineV3TileSource) {
        let (pw, ph) = old.pixelSize
        let ppp = CGFloat(old.pixelsPerPoint)
        for (i, l) in layers {
            let r = EngineV3TileGrid.rect(i, pageWidth: pw, pageHeight: ph)
            l.magnificationFilter = .linear
            l.minificationFilter = .linear
            outgoing.append((l, CGRect(x: CGFloat(r.x) / ppp, y: CGFloat(r.y) / ppp, width: CGFloat(r.width) / ppp, height: CGFloat(r.height) / ppp)))
        }
        layers.removeAll()
        stale.removeAll()
        if outgoing.count > Self.maxOutgoing {
            let excess = outgoing.count - Self.maxOutgoing
            CATransaction.begin(); CATransaction.setDisableActions(true)
            for o in outgoing.prefix(excess) { o.layer.removeFromSuperlayer() }
            CATransaction.commit()
            outgoing.removeFirst(excess)
        }
    }

    private func reframeOutgoing(_ s: EngineV3TileSource) {
        guard !outgoing.isEmpty else { return }
        let k = CGFloat(s.screenPixelsPerPoint / s.displayScale)
        let flipped = container.contentsAreFlipped(), height = container.bounds.height
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for o in outgoing {
            let r = CGRect(x: o.points.minX * k, y: o.points.minY * k, width: o.points.width * k, height: o.points.height * k)
            o.layer.frame = flipped ? r : CGRect(x: r.minX, y: height - r.maxY, width: r.width, height: r.height)
        }
        CATransaction.commit()
    }

    private func retireOutgoingIfCovered(_ vis: Set<EngineV3TileGrid.Index>, visible: CGRect) {
        guard !outgoing.isEmpty, visible.isEmpty || (!vis.isEmpty && vis.allSatisfy { layers[$0] != nil }) else { return }
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for o in outgoing { o.layer.removeFromSuperlayer() }
        CATransaction.commit()
        outgoing.removeAll()
    }

    private func setFilters() {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        let f: CALayerContentsFilter = pinching || source?.stretched == true ? .linear : .nearest
        for l in layers.values { l.magnificationFilter = f; l.minificationFilter = f }
        CATransaction.commit()
    }
}

/// The pane's scroll view: pinch-to-zoom as a Core Animation transform of
/// the clip view's content (linear filtering, nothing laid out or drawn
/// while the gesture runs), committed once when it ends. Only this pane's
/// pinches reach it, and a cancelled gesture commits like an ended one.
final class EngineV3ScrollContainer: NSScrollView {
    weak var pages: EngineV3PagesView?
    private var pinchBase: CGFloat?
    private var pinchScale: CGFloat = 1
    /// The gesture's fixed point in the clip view's bounds.
    private var pinchAnchor: CGPoint = .zero

    override func magnify(with event: NSEvent) {
        guard let pages else { return super.magnify(with: event) }
        switch event.phase {
        case .began:
            beginPinch(at: contentView.convert(event.locationInWindow, from: nil), zoom: pages.zoom)
            updatePinch(by: event.magnification)
        case .changed:
            if pinchBase == nil { beginPinch(at: contentView.convert(event.locationInWindow, from: nil), zoom: pages.zoom) }
            updatePinch(by: event.magnification)
        case .ended, .cancelled:
            if pinchBase != nil { updatePinch(by: event.magnification); endPinch() }
        default:
            // A magnify without phases (older devices, synthesized events): one step.
            beginPinch(at: contentView.convert(event.locationInWindow, from: nil), zoom: pages.zoom)
            updatePinch(by: event.magnification)
            endPinch()
        }
    }

    var isPinching: Bool { pinchBase != nil }

    func beginPinch(at anchor: CGPoint, zoom: CGFloat) {
        pinchBase = zoom
        pinchScale = 1
        pinchAnchor = anchor
        contentView.wantsLayer = true
        pages?.setPinching(true)
    }

    /// Multiplies the pinch by (1 + m), clamped to the zoom range.
    func updatePinch(by m: CGFloat) {
        guard let base = pinchBase, base > 0, let layer = contentView.layer else { return }
        let target = PreviewZoom.clamped(base * pinchScale * (1 + m))
        pinchScale = target / base
        CATransaction.begin(); CATransaction.setDisableActions(true)
        layer.sublayerTransform = Self.transform(pinchScale, about: pinchAnchor, in: layer)
        CATransaction.commit()
    }

    /// Removes the transform and commits the zoom in the same pass, keeping
    /// the page point under the gesture where it was.
    func endPinch() {
        guard let base = pinchBase else { return }
        pinchBase = nil
        CATransaction.begin(); CATransaction.setDisableActions(true)
        contentView.layer?.sublayerTransform = CATransform3DIdentity
        pages?.commitZoom(PreviewZoom.clamped(base * pinchScale), anchor: pinchAnchor)
        pages?.setPinching(false)
        CATransaction.commit()
    }

    /// Scale by `m` about `point` (the layer's bounds space): a sublayer
    /// transform acts about the anchor point, so the fixed point is moved.
    static func transform(_ m: CGFloat, about point: CGPoint, in layer: CALayer) -> CATransform3D {
        let a = CGPoint(x: layer.bounds.minX + layer.anchorPoint.x * layer.bounds.width,
                        y: layer.bounds.minY + layer.anchorPoint.y * layer.bounds.height)
        return CATransform3DConcat(CATransform3DMakeScale(m, m, 1),
                                   CATransform3DMakeTranslation((1 - m) * (point.x - a.x), (1 - m) * (point.y - a.y), 0))
    }
}
