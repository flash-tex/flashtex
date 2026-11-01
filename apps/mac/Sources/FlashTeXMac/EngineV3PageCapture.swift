import AppKit
import Foundation
import ImageIO
import FlashTeXPreviewV3

/// Whole-document evidence for the engine-v3 pane (lane INFDESC-APP; never in
/// the product path). Opens with the window (`FLASHTEX_OPEN`), waits for the
/// document to settle, then shows chosen pages one at a time and writes what
/// the pane drew.
///
///   FLASHTEX_V3_CAPTURE_OUT=<dir>        turns it on; `capture.json` and the page bitmaps go there
///   FLASHTEX_V3_CAPTURE_PAGES=1,57,575   1-based pages to show and write (`page-0057.png`), in this order
///   FLASHTEX_V3_CAPTURE_SETTLE=5         seconds after a DONE with no event from the host that count as
///                                        settled (a host running external tools compiles again by itself)
///   FLASHTEX_V3_CAPTURE_HOLD=2           seconds each page stays on screen once drawn (window screenshots)
///   FLASHTEX_V3_CAPTURE_TIMEOUT=1800     seconds to wait for the document to settle
///   FLASHTEX_V3_CAPTURE_EXIT=1           quit when done
///   FLASHTEX_V3_CAPTURE_TILES=1          on a tiled pane, also wait for each page's visible tiles and write
///                                        them as one image at the tiles' scale (`page-0057-tiles.png`,
///                                        magenta where no tile is up)
///   FLASHTEX_V3_CAPTURE_EDIT='needle|text' once settled, insert `text` (`\n` is a newline) in the editor right
///                                        after the first `needle`, as one typed insertion, wait to settle
///                                        again, then capture: `edit` lists the pages whose content changed
///                                        and the pages still marked stale
///
/// `capture.json` has, from the open (`EngineV3Session.openStartNs`): the
/// first page's arrival, every DONE (status, pages, passes, pages typeset),
/// the peak footprint of the app and of `flashtex-host`, the pages drawn
/// from the PDF (INCOMPLETE), and per captured page whether its bitmap was
/// current. Each page shown is logged as `v3capture: page N on screen`, so a
/// script can capture the window by id (`screencapture -l`) while it holds.
@MainActor
final class EngineV3PageCapture {
    struct Config: Equatable {
        var out: URL
        var pages: [Int]
        var settle: Double = 5
        var hold: Double = 2
        var timeout: Double = 1800
        var exitWhenDone = false
        var tiles = false
        var edit: Edit?

        struct Edit: Equatable { var needle: String; var text: String }

        /// nil unless `FLASHTEX_V3_CAPTURE_OUT` is set. Pages are 1-based in
        /// the environment and kept 1-based here; bad entries are skipped.
        static func parse(_ env: [String: String]) -> Config? {
            guard let out = env["FLASHTEX_V3_CAPTURE_OUT"], !out.isEmpty else { return nil }
            let pages = (env["FLASHTEX_V3_CAPTURE_PAGES"] ?? "").split(separator: ",")
                .compactMap { Int($0.trimmingCharacters(in: .whitespaces)) }.filter { $0 >= 1 }
            var c = Config(out: URL(fileURLWithPath: (out as NSString).expandingTildeInPath, isDirectory: true), pages: pages)
            if let v = env["FLASHTEX_V3_CAPTURE_SETTLE"].flatMap(Double.init), v >= 0 { c.settle = v }
            if let v = env["FLASHTEX_V3_CAPTURE_HOLD"].flatMap(Double.init), v >= 0 { c.hold = v }
            if let v = env["FLASHTEX_V3_CAPTURE_TIMEOUT"].flatMap(Double.init), v > 0 { c.timeout = v }
            c.exitWhenDone = env["FLASHTEX_V3_CAPTURE_EXIT"] == "1"
            c.tiles = env["FLASHTEX_V3_CAPTURE_TILES"] == "1"
            if let e = env["FLASHTEX_V3_CAPTURE_EDIT"], let bar = e.firstIndex(of: "|"), bar != e.startIndex {
                c.edit = Edit(needle: String(e[..<bar]), text: e[e.index(after: bar)...].replacingOccurrences(of: "\\n", with: "\n"))
            }
            return c
        }
    }

    struct Done: Codable {
        var ms: Double, status: String, pages: Int?, passes: Int?, typesetPages: Int?, elapsedMs: Double?, mode: String?
    }

    struct PageResult: Codable {
        var page: Int, current: Bool, waitedMs: Double, image: String?, incomplete: Bool
        /// `pane`: the bitmap the pane installed; `renderer`: the pane did not
        /// show the page current within 30 s, so the page was drawn with the
        /// pane's renderer (`DL3Renderer`, light, 2 px/pt) from the same
        /// prepared page instead; nil: no image.
        var source: String?
        /// The pane's frame when the page was shown (a collapsed pane holds no pages).
        var pane: String
        /// The page's pixels per point on screen, and the scale of its whole
        /// bitmap (`image`; the 2 px/pt backdrop when the pane is tiled).
        var pixelsPerPoint: Double?
        var bitmapPixelsPerPoint: Double?
        /// Drawn from the compile's PDF (an INCOMPLETE page), not the display list.
        var pdfFallback: Bool?
        /// Form XObjects the page draws (pgfpages, \pgfuseimage, beamer's shaded balls…).
        var forms: Int?
        var tiles: Tiles?
        /// Reverse search: the source lines the page's glyphs carry ("path:line": glyphs), and
        /// where a click at the page's centre goes.
        var sourceLines: [String: Int]?
        var reverseAtCentre: String?
    }

    /// A tiled page's tiles once its visible ones were up (`FLASHTEX_V3_CAPTURE_TILES`).
    struct Tiles: Codable {
        /// The tiles' scale and the page's pixel grid at it.
        var pixelsPerPoint: Double, gridWidth: Int, gridHeight: Int
        var held: Int, stale: Int, missingVisible: Int, pending: Int, deferred: Bool
        /// Cut from one kept raster of the whole page (paths, images, forms, PDF), not drawn per tile.
        var drawnWhole: Bool
        /// Tiles whose surface is not exactly its grid rectangle.
        var wrongSize: Int
        var waitedMs: Double
        var image: String?
    }

    struct EditResult: Codable {
        var needle: String, text: String
        /// UTF-16 offset of the insertion; nil when the needle was not found.
        var offset: Int?
        /// 1-based pages whose content hash changed (or that appeared or went), after the document settled again.
        var changedPages: [Int]
        /// 1-based pages still marked stale then.
        var stalePages: [Int]
        var pagesBefore: Int, pagesAfter: Int
    }

    struct Summary: Codable {
        var document: String
        var firstPageMs: Double?
        var firstPixelsMs: Double?
        var settledMs: Double?
        var dones: [Done]
        var pages: Int
        var incompletePages: [Int]
        var appFootprintMaxBytes: Int
        var hostFootprintMaxBytes: Int
        var hostFootprintEndBytes: Int
        var captured: [PageResult]
        var load: [Double]
        var status: String
        var definition: String
        var edit: EditResult?
        /// Forward search from each line of the main file that has a place:
        /// line -> 1-based page (the first page showing it).
        var forward: [String: Int]?
    }

    private static var running: EngineV3PageCapture?
    private let config: Config
    private weak var model: ShellModel?
    private var timer: Timer?
    private let launched = Date()
    private var dones: [Done] = []
    private var seenDones = 0
    private var lastDoneAt: Date?
    private var firstPageMs: Double?
    private var appMax = 0, hostMax = 0
    private var captured: [PageResult] = []
    private var settledMs: Double?
    private var editResult: EditResult?
    private var hashesBeforeEdit: [Int: [UInt8]]?
    private var pagesBeforeEdit = 0

    private init(config: Config, model: ShellModel) { self.config = config; self.model = model }

    static func startIfConfigured(model: ShellModel) {
        guard running == nil, let c = Config.parse(ProcessInfo.processInfo.environment) else { return }
        try? FileManager.default.createDirectory(at: c.out, withIntermediateDirectories: true)
        // FLASHTEX_V3_CAPTURE_NARROW_PREVIEW=1: in the narrow layout (a window a tiling
        // window manager made half the screen wide) show the preview column, not the editor.
        if ProcessInfo.processInfo.environment["FLASHTEX_V3_CAPTURE_NARROW_PREVIEW"] == "1" { model.narrowPreviewShown = true }
        let r = EngineV3PageCapture(config: c, model: model)
        running = r
        r.timer = Timer.scheduledTimer(withTimeInterval: 0.02, repeats: true) { _ in MainActor.assumeIsolated { r.poll() } }
        FlashTeXLog.write("v3capture: armed, pages \(c.pages), out \(c.out.path)")
    }

    /// Milliseconds from the open to `ns` (monotonic).
    private func sinceOpen(_ ns: UInt64?) -> Double? {
        guard let ns, let o = model?.engineV3.openStartNs else { return nil }
        return Double(ns &- o) / 1e6
    }

    private func nowMs() -> Double? { sinceOpen(MonotonicClock.nowNs()) }

    private func sample() {
        guard let s = model?.engineV3 else { return }
        appMax = max(appMax, EngineV3ScrollBench.footprint())
        if let pid = s.hostPID { hostMax = max(hostMax, Self.footprint(pid: pid)) }
        if firstPageMs == nil, s.pageCount > 0, !s.pages.isEmpty { firstPageMs = nowMs() }
        if s.doneCount != seenDones, let j = s.lastDone {
            seenDones = s.doneCount
            lastDoneAt = Date()
            dones.append(Done(ms: nowMs() ?? -1, status: j["status"]?.string ?? "?", pages: j["pages"]?.int.map(Int.init),
                              passes: j["passes"]?.int.map(Int.init), typesetPages: j["typeset_pages"]?.int.map(Int.init),
                              elapsedMs: j["elapsed_ms"]?.double, mode: j["mode"]?.string))
            FlashTeXLog.write("v3capture: DONE \(dones.count) at \(String(format: "%.0f", dones.last!.ms)) ms: \(s.statusNote)")
        }
    }

    private func poll() {
        guard let model else { return }
        sample()
        let s = model.engineV3
        if case .failed(let why) = s.phase { finish("failed: \(why)"); return }
        if Date().timeIntervalSince(launched) > config.timeout { finish("timeout waiting for the document to settle"); return }
        let quietMs = Double(MonotonicClock.nowNs() &- s.lastEventNs) / 1e6
        guard lastDoneAt != nil, !s.compiling, s.view != nil, quietMs >= config.settle * 1000 else { return }
        if let e = config.edit, editResult == nil {
            applyEdit(e)
            return
        }
        timer?.invalidate()
        settledMs = dones.last?.ms
        FlashTeXLog.write("v3capture: settled after \(dones.count) DONE(s); \(s.pageCount) pages")
        if var e = editResult, let before = hashesBeforeEdit {
            let after = s.pages.mapValues(\.page.hash)
            e.changedPages = Self.changedPages(before: before, after: after, countBefore: pagesBeforeEdit, countAfter: s.pageCount)
            e.stalePages = s.stale.sorted().map { $0 + 1 }
            e.pagesAfter = s.pageCount
            editResult = e
            hashesBeforeEdit = nil
            FlashTeXLog.write("v3capture: edit changed pages \(e.changedPages); stale \(e.stalePages)")
        }
        capture(config.pages)
    }

    /// 1-based pages whose content differs between two settled states.
    nonisolated static func changedPages(before: [Int: [UInt8]], after: [Int: [UInt8]], countBefore: Int, countAfter: Int) -> [Int] {
        (0 ..< max(countBefore, countAfter)).filter { before[$0] == nil || after[$0] == nil || before[$0] != after[$0] }.map { $0 + 1 }
    }

    /// Inserts `edit.text` after the first `edit.needle` in the editor (one
    /// insertion, as typing it at once would), then waits for the document
    /// to settle again.
    private func applyEdit(_ edit: Config.Edit) {
        guard let model else { return }
        let s = model.engineV3
        var result = EditResult(needle: edit.needle, text: edit.text, offset: nil, changedPages: [], stalePages: [],
                                pagesBefore: s.pageCount, pagesAfter: s.pageCount)
        guard let tv = TypingBenchDriver.findTextView(in: NSApp.windows.compactMap(\.contentView)) else {
            editResult = result
            finish("edit: no editor text view")
            return
        }
        let r = (tv.string as NSString).range(of: edit.needle)
        guard r.location != NSNotFound else {
            editResult = result
            finish("edit: needle not found")
            return
        }
        hashesBeforeEdit = s.pages.mapValues(\.page.hash)
        pagesBeforeEdit = s.pageCount
        result.offset = r.location + r.length
        editResult = result
        tv.window?.makeFirstResponder(tv)
        tv.setSelectedRange(NSRange(location: r.location + r.length, length: 0))
        tv.insertText(edit.text, replacementRange: tv.selectedRange())
        lastDoneAt = nil // settled again only after a DONE of the edit's compile
        FlashTeXLog.write("v3capture: edit inserted \(edit.text.debugDescription) at \(r.location + r.length)")
    }

    private func capture(_ remaining: [Int]) {
        guard let model, let view = model.engineV3.view else { finish("no pane"); return }
        guard let page = remaining.first else { finish("done"); return }
        let rest = Array(remaining.dropFirst())
        let i = page - 1
        guard view.scrollToPage(i) else {
            captured.append(PageResult(page: page, current: false, waitedMs: 0, image: nil, incomplete: false, source: nil,
                                       pane: NSStringFromRect(view.enclosingScrollView?.frame ?? .zero)))
            capture(rest)
            return
        }
        let start = Date()
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] t in
            MainActor.assumeIsolated {
                guard let self, let model = self.model, let view = model.engineV3.view else { t.invalidate(); return }
                self.sample()
                let current = view.pageShowsCurrent(i)
                guard current || Date().timeIntervalSince(start) > 30 else { return }
                t.invalidate()
                let waited = Date().timeIntervalSince(start) * 1000
                // The raster is installed off-main after the key is set: give it a moment.
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                    var name: String?, source: String?
                    let file = String(format: "page-%04d.png", page)
                    if current, let img = view.installedImage(i) {
                        if Self.writePNG(img, to: self.config.out.appendingPathComponent(file)) { name = file; source = "pane" }
                    } else if let prepared = model.engineV3.pages[i],
                              let img = DL3Renderer.rasterizeToSurface(prepared, forms: model.engineV3.forms, scale: 2, appearance: .light)
                                  .flatMap({ DL3Renderer.image(of: $0) }) {
                        if Self.writePNG(img, to: self.config.out.appendingPathComponent(file)) { name = file; source = "renderer" }
                    }
                    let incomplete = model.engineV3.incompletePages.contains(i)
                    let forms = model.engineV3.pages[i]?.page.items.filter { if case .form = $0 { true } else { false } }.count
                    var result = PageResult(page: page, current: current, waitedMs: waited, image: name, incomplete: incomplete,
                                            source: source, pane: NSStringFromRect(view.enclosingScrollView?.frame ?? .zero),
                                            pixelsPerPoint: view.currentPixelsPerPoint, bitmapPixelsPerPoint: view.heldPageView(i)?.rasterScale,
                                            pdfFallback: model.engineV3.pdfFallback[i] != nil, forms: forms)
                    if let ix = model.engineV3.sourceIndex(page: i) {
                        var lines: [String: Int] = [:]
                        for g in ix.glyphs {
                            guard let loc = model.engineV3.sourceMap.location(of: g.span) else { continue }
                            lines["\(model.engineV3.projectPath(ofEngineFile: loc.path) ?? loc.path):\(loc.line)", default: 0] += 1
                        }
                        result.sourceLines = lines
                        if let p = model.engineV3.pages[i] {
                            result.reverseAtCentre = model.engineV3.source(page: i, at: CGPoint(x: p.widthPt / 2, y: p.heightPt / 2))
                                .map { "\($0.path):\($0.line)" }
                        }
                    }
                    let done = {
                        self.captured.append(result)
                        FlashTeXLog.write("v3capture: page \(page) on screen (current \(current), incomplete \(incomplete), tiled \(result.tiles != nil))")
                        DispatchQueue.main.asyncAfter(deadline: .now() + self.config.hold) { self.capture(rest) }
                    }
                    guard self.config.tiles, current, view.tiled else { done(); return }
                    self.awaitTiles(page: page, view: view) { tiles in result.tiles = tiles; done() }
                }
            }
        }
    }

    /// Waits (up to 30 s) until page `page`'s visible tiles are up and
    /// current, then writes them as one image at the tiles' scale.
    private func awaitTiles(page: Int, view: EngineV3PagesView, _ then: @escaping (Tiles?) -> Void) {
        let i = page - 1, start = Date()
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] t in
            MainActor.assumeIsolated {
                guard let self, let v = view.heldPageView(i) else { t.invalidate(); then(nil); return }
                let tiles = v.tiles
                let settled = tiles.source != nil && tiles.missingVisible(v.visibleRect) == 0 && tiles.pending == 0
                    && tiles.staleCount == 0 && tiles.deferred == nil
                guard settled || Date().timeIntervalSince(start) > 30 else { return }
                t.invalidate()
                guard let src = tiles.source else { then(nil); return }
                let (w, h) = src.pixelSize
                let m = Self.mosaic(tiles)
                var name: String?
                let file = String(format: "page-%04d-tiles.png", page)
                if let img = m.image, Self.writePNG(img, to: self.config.out.appendingPathComponent(file)) { name = file }
                then(Tiles(pixelsPerPoint: src.pixelsPerPoint, gridWidth: w, gridHeight: h, held: tiles.count, stale: tiles.staleCount,
                           missingVisible: tiles.missingVisible(v.visibleRect), pending: tiles.pending, deferred: tiles.deferred != nil,
                           drawnWhole: src.drawnWhole, wrongSize: m.wrongSize, waitedMs: Date().timeIntervalSince(start) * 1000, image: name))
            }
        }
    }

    /// The tiles up on a page, each at its grid rectangle, over magenta (no
    /// tile there), at the tiles' own scale; and how many tiles have a
    /// surface that is not exactly their rectangle's size.
    static func mosaic(_ tiles: EngineV3PageTiles) -> (image: CGImage?, wrongSize: Int) {
        guard let src = tiles.source else { return (nil, 0) }
        let (w, h) = src.pixelSize
        guard w > 0, h > 0, let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                                 space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                                 bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return (nil, 0) }
        ctx.setFillColor(CGColor(srgbRed: 1, green: 0, blue: 1, alpha: 1))
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        var wrong = 0
        for (index, layer) in tiles.layers {
            guard let surface = layer.contents as! IOSurface?, let img = DL3Renderer.image(of: surface) else { continue }
            let r = EngineV3TileGrid.rect(index, pageWidth: w, pageHeight: h)
            if img.width != r.width || img.height != r.height { wrong += 1 }
            ctx.draw(img, in: CGRect(x: r.x, y: h - r.y - r.height, width: r.width, height: r.height))
        }
        return (ctx.makeImage(), wrong)
    }

    /// Forward search from every line of the main file (`EngineV3Session.place`).
    private func forwardSearch() -> [String: Int]? {
        guard let model, let text = model.documentURL.flatMap({ try? String(contentsOf: $0, encoding: .utf8) }) else { return nil }
        let path = model.documentURL?.lastPathComponent ?? "main.tex"
        var out: [String: Int] = [:]
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false).count
        for line in 1 ... max(1, lines) {
            if let place = model.engineV3.place(path: path, line: line, col: nil) { out[String(line)] = place.page + 1 }
        }
        return out
    }

    private func finish(_ status: String) {
        timer?.invalidate()
        guard let model else { return }
        sample()
        let s = model.engineV3
        // The compile's own PDF (the reference the pane is drawn to match, and what
        // INCOMPLETE pages are drawn from), before the host's project copy goes.
        if let pdf = s.lastDone?["pdf"]?.string {
            let dest = config.out.appendingPathComponent("engine.pdf")
            try? FileManager.default.removeItem(at: dest)
            try? FileManager.default.copyItem(at: URL(fileURLWithPath: pdf), to: dest)
        }
        let summary = Summary(document: model.documentURL?.path ?? "", firstPageMs: firstPageMs, firstPixelsMs: sinceOpen(s.openFirstPixelsNs),
                              settledMs: settledMs, dones: dones, pages: s.pageCount, incompletePages: s.incompletePages.map { $0 + 1 },
                              appFootprintMaxBytes: appMax, hostFootprintMaxBytes: hostMax,
                              hostFootprintEndBytes: s.hostPID.map(Self.footprint(pid:)) ?? 0,
                              captured: captured, load: EngineV3ScrollBench.loadAverage(), status: status,
                              definition: "ms from EngineV3Session.openStartNs (the open); firstPageMs: the first page decoded on main; firstPixelsMs: the first page bitmap committed; dones: each DONE as main applied it (sampled every 20 ms); footprints: phys_footprint sampled every 20 ms (incompletePages are 1-based)",
                              edit: editResult, forward: forwardSearch())
        let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        try? enc.encode(summary).write(to: config.out.appendingPathComponent("capture.json"))
        FlashTeXLog.write("v3capture: \(status) -> \(config.out.path)")
        if config.exitWhenDone {
            s.stop()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { exit(0) }
        }
    }

    /// A child process's physical footprint (Activity Monitor's Memory), 0 if unknown.
    nonisolated static func footprint(pid: Int32) -> Int {
        var info = rusage_info_v4()
        let r = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: rusage_info_t?.self, capacity: 1) { proc_pid_rusage(pid, RUSAGE_INFO_V4, $0) }
        }
        return r == 0 ? Int(info.ri_phys_footprint) : 0
    }

    nonisolated static func writePNG(_ image: CGImage, to url: URL) -> Bool {
        guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else { return false }
        CGImageDestinationAddImage(dest, image, nil)
        return CGImageDestinationFinalize(dest)
    }
}
