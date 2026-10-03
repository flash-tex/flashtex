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

    private init(config: Config, model: ShellModel) { self.config = config; self.model = model }

    static func startIfConfigured(model: ShellModel) {
        guard running == nil, let c = Config.parse(ProcessInfo.processInfo.environment) else { return }
        try? FileManager.default.createDirectory(at: c.out, withIntermediateDirectories: true)
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
        timer?.invalidate()
        settledMs = dones.last?.ms
        FlashTeXLog.write("v3capture: settled after \(dones.count) DONE(s); \(s.pageCount) pages")
        capture(config.pages)
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
                    self.captured.append(PageResult(page: page, current: current, waitedMs: waited, image: name, incomplete: incomplete,
                                                    source: source, pane: NSStringFromRect(view.enclosingScrollView?.frame ?? .zero)))
                    FlashTeXLog.write("v3capture: page \(page) on screen (current \(current), incomplete \(incomplete))")
                    DispatchQueue.main.asyncAfter(deadline: .now() + self.config.hold) { self.capture(rest) }
                }
            }
        }
    }

    private func finish(_ status: String) {
        timer?.invalidate()
        guard let model else { return }
        sample()
        let s = model.engineV3
        let summary = Summary(document: model.documentURL?.path ?? "", firstPageMs: firstPageMs, firstPixelsMs: sinceOpen(s.openFirstPixelsNs),
                              settledMs: settledMs, dones: dones, pages: s.pageCount, incompletePages: s.incompletePages.map { $0 + 1 },
                              appFootprintMaxBytes: appMax, hostFootprintMaxBytes: hostMax,
                              hostFootprintEndBytes: s.hostPID.map(Self.footprint(pid:)) ?? 0,
                              captured: captured, load: EngineV3ScrollBench.loadAverage(), status: status,
                              definition: "ms from EngineV3Session.openStartNs (the open); firstPageMs: the first page decoded on main; firstPixelsMs: the first page bitmap committed; dones: each DONE as main applied it (sampled every 20 ms); footprints: phys_footprint sampled every 20 ms (incompletePages are 1-based)")
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
