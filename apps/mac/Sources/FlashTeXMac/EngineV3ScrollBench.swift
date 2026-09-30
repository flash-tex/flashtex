import AppKit
import QuartzCore
import FlashTeXPreviewV3

/// `FLASHTEX_V3_SCROLL_BENCH=<seconds>` (evidence capture, never in the
/// product path): once a page is on screen, scrolls the engine-v3 pane down
/// and up at `FLASHTEX_V3_SCROLL_SPEED` pt/s (default 2400) from the display
/// link and writes frame intervals, dropped frames, the main thread's scroll
/// step, the off-main tile jobs, frames with visible tiles still missing,
/// bitmap bytes held, footprint and load to `FLASHTEX_V3_SCROLL_BENCH_OUT`
/// (JSON) and the log. `FLASHTEX_V3_PPP` fixes the pixels per point.
/// `FLASHTEX_V3_PINCH_HOLD=<m>` instead holds a pinch transform of `m`
/// (screenshot evidence).
@MainActor
final class EngineV3ScrollBench: NSObject {
    private static var started = false
    private static var running: EngineV3ScrollBench?
    private weak var pages: EngineV3PagesView?
    private let duration: Double, speed: Double
    private var link: CADisplayLink?
    private var observer: CFRunLoopObserver?
    private var start: CFTimeInterval = 0, last: CFTimeInterval = 0
    private var direction: CGFloat = 1
    private var intervals: [Double] = [], nominal: [Double] = []
    private var dropped = 0, framesMissing = 0, missingMax = 0, lastMissing = 0
    private var maxBytes = 0, maxFootprint = 0
    private var lastScrollMs = 0.0, scrollMsMax = 0.0
    private var hitches: [String] = [], longPasses: [String] = []
    private var loadStart: [Double] = []

    private init(pages: EngineV3PagesView, duration: Double, speed: Double) {
        self.pages = pages; self.duration = duration; self.speed = speed
    }

    static func startIfRequested(from pages: EngineV3PagesView) {
        guard !started else { return }
        let env = ProcessInfo.processInfo.environment
        guard env["FLASHTEX_V3_PINCH_HOLD"] != nil || env["FLASHTEX_V3_SCROLL_BENCH"] != nil else { started = true; return }
        // A settled pane: SwiftUI may build (and replace) a transient one
        // while the window lays out (380×50 pt was seen).
        guard pages.window != nil, let scroll = pages.enclosingScrollView as? EngineV3ScrollContainer,
              !pages.heldPageViews.isEmpty, scroll.frame.height >= 300, scroll.frame.width >= 300 else { return }
        started = true
        let delay = env["FLASHTEX_V3_SCROLL_BENCH_DELAY"].flatMap(Double.init) ?? 4
        if let hold = env["FLASHTEX_V3_PINCH_HOLD"].flatMap(Double.init) {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
                MainActor.assumeIsolated {
                    let b = scroll.contentView.bounds
                    scroll.beginPinch(at: CGPoint(x: b.midX, y: b.midY), zoom: pages.zoom)
                    scroll.updatePinch(by: CGFloat(hold) - 1)
                    FlashTeXLog.write("engine-v3: pinch hold \(hold) applied")
                }
            }
            return
        }
        guard let seconds = env["FLASHTEX_V3_SCROLL_BENCH"].flatMap(Double.init) else { return }
        let speed = env["FLASHTEX_V3_SCROLL_SPEED"].flatMap(Double.init) ?? 2400
        moveToFastestScreen(pages.window)
        // The split positions are restored from shared defaults, which other
        // app runs change: the Problems panel can leave the pane 50 pt tall.
        // The bench hides it and starts once the live pane is at least 300 pt.
        pages.session?.model?.problemsVisible = false
        func attempt(_ n: Int) {
            DispatchQueue.main.asyncAfter(deadline: .now() + (n == 0 ? delay : 0.5)) {
                MainActor.assumeIsolated {
                    let live = pages.session?.view ?? pages
                    let pane = live.enclosingScrollView?.frame.size ?? .zero
                    guard pane.height >= 300 || n >= 40 else { return attempt(n + 1) }
                    let bench = EngineV3ScrollBench(pages: live, duration: seconds, speed: speed)
                    running = bench
                    bench.begin()
                }
            }
        }
        attempt(0)
    }

    /// The bench runs visible on the fastest screen (the built-in 120 Hz
    /// panel on this Mac); `orderFrontRegardless` neither activates the app
    /// nor takes key focus. The window keeps its size.
    private static func moveToFastestScreen(_ window: NSWindow?) {
        guard let window else { return }
        window.orderFrontRegardless()
        guard let fastest = NSScreen.screens.max(by: { $0.maximumFramesPerSecond < $1.maximumFramesPerSecond }), window.screen != fastest else { return }
        let f = fastest.visibleFrame
        window.setFrameOrigin(NSPoint(x: f.minX + 20, y: max(f.minY, f.maxY - window.frame.height - 20)))
    }

    private func begin() {
        guard let pages = current, let scroll = pages.enclosingScrollView else { return }
        Self.moveToFastestScreen(scroll.window)
        // The screen's own link: a view's link made right after the window
        // changed screens can stay bound to the old display.
        let screen = NSScreen.screens.max(by: { $0.maximumFramesPerSecond < $1.maximumFramesPerSecond }) ?? scroll.window?.screen
        let link = screen?.displayLink(target: self, selector: #selector(tick(_:))) ?? scroll.displayLink(target: self, selector: #selector(tick(_:)))
        link.preferredFrameRateRange = CAFrameRateRange(minimum: 120, maximum: 120, preferred: 120)
        link.add(to: .main, forMode: .common)
        self.link = link
        EngineV3TileGrid.resetCounters()
        DL3Renderer.measureResidency = true
        DL3Renderer.resetResidency()
        loadStart = Self.loadAverage()
        var passStart: UInt64 = 0
        let observer = CFRunLoopObserverCreateWithHandler(nil, CFRunLoopActivity.afterWaiting.rawValue | CFRunLoopActivity.beforeWaiting.rawValue, true, 0) { [weak self] _, activity in
            MainActor.assumeIsolated {
                guard let self else { return }
                let now = MonotonicClock.nowNs()
                if activity == .afterWaiting { passStart = now; return }
                guard passStart > 0 else { return }
                let ms = Double(now &- passStart) / 1e6
                if ms > 8, self.longPasses.count < 40 { self.longPasses.append(String(format: "t=%.3fs %.1fms", CACurrentMediaTime() - self.start, ms)) }
            }
        }
        CFRunLoopAddObserver(CFRunLoopGetMain(), observer, .commonModes)
        self.observer = observer
        // Watchdog: a link that never ticks (window hidden, display asleep) ends the bench with a note.
        DispatchQueue.main.asyncAfter(deadline: .now() + duration + 5) { [weak self] in
            MainActor.assumeIsolated {
                guard let self, Self.running === self else { return }
                FlashTeXLog.write("engine-v3: scroll bench stalled after \(self.intervals.count) frames (visible \(self.pages?.window?.occlusionState.contains(.visible) == true), screen \(self.pages?.window?.screen?.localizedName ?? "none"))")
                self.finish()
            }
        }
        FlashTeXLog.write("engine-v3: scroll bench started (\(duration) s at \(speed) pt/s, \(pages.currentPixelsPerPoint) px/pt, screen \(scroll.window?.screen?.localizedName ?? "?") max \(scroll.window?.screen?.maximumFramesPerSecond ?? 0) fps, visible \(scroll.window?.occlusionState.contains(.visible) == true), pane \(NSStringFromRect(scroll.frame)))")
    }

    /// The session's pages view now (SwiftUI may have replaced the one the bench started from).
    private var current: EngineV3PagesView? { pages?.session?.view ?? pages }

    @objc private func tick(_ link: CADisplayLink) {
        guard let pages = current, let scroll = pages.enclosingScrollView else { return finish() }
        if start == 0 { start = link.timestamp; last = link.timestamp }
        let dt = link.timestamp - last
        if dt > 0 {
            intervals.append(dt * 1000)
            let frame = link.targetTimestamp - link.timestamp
            nominal.append(frame * 1000)
            if frame > 0, dt > frame * 1.5 {
                dropped += Int((dt / frame).rounded()) - 1
                hitches.append(String(format: "t=%.3fs %.1fms prev-scroll=%.2fms missing-tiles=%d", link.timestamp - start, dt * 1000, lastScrollMs, lastMissing))
            }
        }
        last = link.timestamp
        maxBytes = max(maxBytes, pages.retainedBytes)
        lastMissing = pages.missingVisibleTiles
        if lastMissing > 0 { framesMissing += 1; missingMax = max(missingMax, lastMissing) }
        maxFootprint = max(maxFootprint, Self.footprint())
        let clip = scroll.contentView
        var y = clip.bounds.origin.y + direction * CGFloat(speed * max(dt, 0))
        let maxY = max(0, pages.frame.height - clip.bounds.height)
        if y >= maxY { y = maxY; direction = -1 } else if y <= 0 { y = 0; direction = 1 }
        let t0 = MonotonicClock.nowNs()
        clip.scroll(to: NSPoint(x: clip.bounds.origin.x, y: y))
        scroll.reflectScrolledClipView(clip)
        lastScrollMs = Double(MonotonicClock.nowNs() &- t0) / 1e6
        scrollMsMax = max(scrollMsMax, lastScrollMs)
        if link.timestamp - start >= duration { finish() }
    }

    private func finish() {
        link?.invalidate(); link = nil
        if let observer { CFRunLoopRemoveObserver(CFRunLoopGetMain(), observer, .commonModes) }
        observer = nil
        let sorted = intervals.sorted()
        func pct(_ p: Double) -> Double { sorted.isEmpty ? 0 : sorted[min(sorted.count - 1, Int(Double(sorted.count - 1) * p))] }
        let result: [String: Any] = [
            "frames": intervals.count, "dropped_frames": dropped,
            "nominal_frame_ms": nominal.isEmpty ? 0 : nominal.reduce(0, +) / Double(nominal.count),
            "interval_ms_p50": pct(0.5), "interval_ms_p99": pct(0.99), "interval_ms_max": sorted.last ?? 0,
            "tile_jobs_off_main": EngineV3TileGrid.jobs, "tiles_rastered_off_main": EngineV3TileGrid.jobTiles,
            "tile_job_ms_total": EngineV3TileGrid.jobMs, "tile_job_ms_max": EngineV3TileGrid.maxJobMs,
            "tile_queue_to_install_ms_max": EngineV3TileGrid.maxLatencyMs, "tiles_skipped_undrawn": EngineV3TileGrid.skippedTiles,
            "cut_raster_resident_bytes_max": DL3Renderer.maxCutResidentBytes, "scroll_step_main_ms_max": scrollMsMax,
            "frames_with_missing_visible_tiles": framesMissing, "missing_visible_tiles_max": missingMax,
            "load_average_start": loadStart, "load_average_end": Self.loadAverage(),
            "bitmap_bytes_max": maxBytes, "footprint_bytes_max": maxFootprint,
            "px_per_pt": current?.currentPixelsPerPoint ?? 0, "tiled": current?.tiled ?? false,
            "tile_threshold_px_per_pt": EngineV3TileGrid.threshold, "speed_pt_per_s": speed, "seconds": duration,
            "pages": current?.session?.pageCount ?? 0, "screen": current?.window?.screen?.localizedName ?? "?",
            "pane": current?.enclosingScrollView.map { NSStringFromRect($0.frame) } ?? "?", "hitches": Array(hitches.prefix(40)), "long_main_passes": longPasses,
            "window_color_space": current?.window?.colorSpace?.localizedName ?? "nil (the screen's)",
            "screen_color_space": current?.window?.screen?.colorSpace?.localizedName ?? "?",
        ]
        let data = (try? JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])) ?? Data()
        FlashTeXLog.write("engine-v3: scroll bench " + (String(data: data, encoding: .utf8) ?? "").replacingOccurrences(of: "\n", with: " "))
        if let out = ProcessInfo.processInfo.environment["FLASHTEX_V3_SCROLL_BENCH_OUT"] { try? data.write(to: URL(fileURLWithPath: out)) }
        Self.running = nil
    }

    static func loadAverage() -> [Double] {
        var l = [Double](repeating: 0, count: 3)
        return getloadavg(&l, 3) == 3 ? l : []
    }

    /// The process's physical footprint (Activity Monitor's Memory).
    static func footprint() -> Int {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count) }
        }
        return kr == KERN_SUCCESS ? Int(info.phys_footprint) : 0
    }
}
