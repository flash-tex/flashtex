import AppKit
import Foundation
import ImageIO

/// Keystroke → pixels bench for the engine-v3 preview (lane P3-APP-V3).
///
///   FLASHTEX_V3_BENCH=/path/main.tex   open it with the engine-v3 preview on
///   FLASHTEX_V3_BENCH_KEYS=40          keystrokes (a letter typed, then deleted, alternately)
///   FLASHTEX_V3_BENCH_MS=300           interval between keystrokes
///   FLASHTEX_V3_BENCH_AT="with "       type right after this text (TypingBenchConfig.insertionOffset): a prose word of the first page
///   FLASHTEX_V3_BENCH_OUT=path.json    the summary (default: next to the document)
///
/// Each keystroke is stamped just before it is inserted into the editor's
/// text view (as `TypingBenchDriver` stamps its synthetic keys); the sample
/// ends at the Core Animation commit (`CATransaction.commit()` + `flush()`)
/// of the first page the resulting compile changed (`EngineV3Latency`). The
/// app exits when done.
@MainActor
final class EngineV3Bench {
    static var current: EngineV3Bench?
    let url: URL
    let keys: Int
    let intervalMs: Double
    let at: String
    let out: URL
    weak var model: ShellModel?
    private var timer: Timer?
    private var typed = 0
    private var started = Date()
    private var textView: NSTextView?

    /// `FLASHTEX_V3_OPEN_BENCH=out.json` with `FLASHTEX_OPEN=file.tex`: open →
    /// first pixels (stored pages or the compile's) and → first current page,
    /// then wait for the snapshot to be saved (the next run reopens from it) and exit.
    static func openBenchIfConfigured(model: ShellModel) {
        let env = ProcessInfo.processInfo.environment
        guard let out = env["FLASHTEX_V3_OPEN_BENCH"], !out.isEmpty else { return }
        // FLASHTEX_V3_OPEN_BENCH_SWITCH=other.tex: after the launch's project is
        // compiled, open `other`, then reopen the first one in the running app
        // (File > Open), which is what the second measurement is.
        let switchTo = env["FLASHTEX_V3_OPEN_BENCH_SWITCH"].map { URL(fileURLWithPath: $0) }
        let first = env["FLASHTEX_OPEN"].map { URL(fileURLWithPath: $0) }
        var stage = 0
        var results: [[String: Any]] = []
        var stageStart = Date()
        var usedSnapshot = false
        Timer.scheduledTimer(withTimeInterval: 0.01, repeats: true) { t in
            MainActor.assumeIsolated {
                let s = model.engineV3
                if s.snapshot != nil { usedSnapshot = true }
                let done = s.openFirstCurrentNs != nil && s.statusNote.hasPrefix("ok")
                guard done || Date().timeIntervalSince(stageStart) > 120 else { return }
                let o = s.openStartNs ?? 0
                func ms(_ v: UInt64?) -> Double { v.map { Double($0 &- o) / 1e6 } ?? -1 }
                let names = ["launch", "switch", "reopen-in-app"]
                results.append(["stage": names[min(stage, 2)], "first_pixels_ms": ms(s.openFirstPixelsNs), "first_current_ms": ms(s.openFirstCurrentNs),
                                "from_snapshot": usedSnapshot, "pages": s.pageCount, "status": s.statusNote])
                usedSnapshot = false
                stage += 1
                if let switchTo, let first, stage < 3 {
                    // Wait for the snapshot of what is open to be saved, then open the next.
                    t.fireDate = Date().addingTimeInterval(3)
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
                        stageStart = Date()
                        _ = model.openTex(at: stage == 1 ? switchTo : first, dirty: .discard)
                    }
                    return
                }
                t.invalidate()
                let j: [String: Any] = ["runs": results,
                                        "definition": "open start: ShellModel.replaceProject (FLASHTEX_OPEN at launch, or File > Open in the running app); pixels: the first page bitmap committed (CATransaction commit+flush): a stored page (snapshot) or a compiled one"]
                if let d = try? JSONSerialization.data(withJSONObject: j, options: [.prettyPrinted, .sortedKeys]) { try? d.write(to: URL(fileURLWithPath: out)) }
                FlashTeXLog.write("v3openbench: \(results)")
                DispatchQueue.main.asyncAfter(deadline: .now() + 4) { model.engineV3.stop(); exit(0) } // let the snapshot save
            }
        }
    }

    static func startIfConfigured(model: ShellModel) {
        openBenchIfConfigured(model: model)
        let env = ProcessInfo.processInfo.environment
        guard current == nil, let path = env["FLASHTEX_V3_BENCH"], !path.isEmpty else { return }
        let b = EngineV3Bench(url: URL(fileURLWithPath: path), keys: Int(env["FLASHTEX_V3_BENCH_KEYS"] ?? "") ?? 40,
                              intervalMs: Double(env["FLASHTEX_V3_BENCH_MS"] ?? "") ?? 300,
                              at: env["FLASHTEX_V3_BENCH_AT"] ?? "with ",
                              out: URL(fileURLWithPath: env["FLASHTEX_V3_BENCH_OUT"] ?? path + ".v3bench.json"))
        b.model = model
        current = b
        b.start()
    }

    init(url: URL, keys: Int, intervalMs: Double, at: String, out: URL) {
        self.url = url; self.keys = keys; self.intervalMs = intervalMs; self.at = at; self.out = out
    }

    func log(_ s: String) { FlashTeXLog.write("v3bench: " + s); print("v3bench: " + s) }

    func start() {
        guard let model else { return }
        _ = model.openTex(at: url)
        model.engineV3Enabled = true
        model.engineV3.start(model: model)
        log("opened \(url.path); waiting for the first complete compile")
        started = Date()
        timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.poll() } }
    }

    private func poll() {
        guard let model else { return }
        let s = model.engineV3
        if Date().timeIntervalSince(started) > 300 { finish("timeout waiting for the first compile (\(s.phase))"); return }
        if case .failed(let why) = s.phase { finish("failed: \(why)"); return }
        guard s.phase == .ready, s.pageCount > 0, s.statusNote.hasPrefix("ok"), s.view != nil else { return }
        guard let tv = TypingBenchDriver.findTextView(in: NSApp.windows.compactMap(\.contentView)) else { return }
        textView = tv
        timer?.invalidate()
        let offset = TypingBenchConfig.insertionOffset(in: tv.string, beforeEndDocument: true, afterNeedle: at)
        tv.window?.makeFirstResponder(tv)
        // Key -> presented needs a window on screen: an occluded window's
        // frames are never shown (presentedTime 0). FLASHTEX_V3_BENCH_FRONT=1
        // orders it in front without activating the app (no focus is taken;
        // the keystrokes are synthetic).
        if ProcessInfo.processInfo.environment["FLASHTEX_V3_BENCH_FRONT"] == "1" { tv.window?.orderFrontRegardless() }
        tv.setSelectedRange(NSRange(location: offset, length: 0))
        s.latency.reset()
        log("first compile: \(s.statusNote); \(s.pageCount) pages; typing \(keys) keys at UTF-16 offset \(offset) every \(intervalMs) ms")
        // Let the first pages settle on screen before timing.
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            self.timer = Timer.scheduledTimer(withTimeInterval: self.intervalMs / 1000, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.tick() } }
        }
    }

    private func tick() {
        guard let tv = textView, let model else { return }
        if typed < keys {
            let ns = MonotonicClock.nowNs()
            model.engineV3.nextKeystrokeNs = ns
            if typed % 2 == 0 { tv.insertText("x", replacementRange: tv.selectedRange()) } else { tv.deleteBackward(nil) }
            typed += 1
            return
        }
        timer?.invalidate()
        // Settle: wait for the last samples.
        let deadline = Date().addingTimeInterval(10)
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self, let model = self.model else { return }
                if model.engineV3.latency.pendingCount == 0 || Date() > deadline { self.finish("done") }
            }
        }
    }

    struct Summary: Codable {
        var document: String, pages: Int, keystrokes: Int, samples: Int, unchanged: Int, offscreen: Int, pending: Int
        var p50Ms: Double?, p95Ms: Double?, minMs: Double?, maxMs: Double?, meanMs: Double?
        /// Keystroke -> the display link's target frame after the commit.
        var toVsyncP50Ms: Double?, toVsyncP95Ms: Double?
        /// Keystroke -> the frame carrying the page was on screen (EngineV3PresentProbe).
        var toPresentedP50Ms: Double?, toPresentedP95Ms: Double?
        /// Samples whose probe reported, and those whose frame was not shown.
        var presentedSamples: Int, notPresented: Int
        /// EngineV3FrameRateBoost was on (FLASHTEX_V3_BOOST=1).
        var frameRateBoost: Bool
        /// The boost display link's last frame duration (ms): 8.3 at 120 Hz.
        var boostFrameMs: Double
        /// The window's occlusion state said visible when the bench finished.
        var windowVisible: Bool
        var intervalMs: Double
        var fastEdits: Bool
        var definition: String
        var perKeystrokeMs: [Double]
        /// p50/p95/n per stage (EngineV3Latency.stageStats).
        var stages: [String: [String: Double]]
        var samplesDetail: [EngineV3Latency.Sample]
        var pixelsPerPoint: Double
        var status: String
    }

    private func finish(_ why: String) {
        timer?.invalidate()
        guard let model else { exit(1) }
        let l = model.engineV3.latency
        let ms = l.samples.map(\.ms)
        let st = LatencyStats(ms)
        let vs = LatencyStats(l.samples.compactMap(\.toVsyncMs))
        let ps = LatencyStats(l.samples.compactMap(\.toPresentedMs))
        let s = Summary(document: url.path, pages: model.engineV3.pageCount, keystrokes: typed, samples: ms.count,
                        unchanged: l.unchanged, offscreen: l.offscreenCount, pending: l.pendingCount,
                        p50Ms: st.p50Ms, p95Ms: st.p95Ms, minMs: st.minMs, maxMs: st.maxMs, meanMs: st.meanMs,
                        toVsyncP50Ms: vs.p50Ms, toVsyncP95Ms: vs.p95Ms,
                        toPresentedP50Ms: ps.p50Ms, toPresentedP95Ms: ps.p95Ms,
                        presentedSamples: l.samples.filter { ($0.presentedNs ?? 0) > 0 }.count,
                        notPresented: l.samples.filter { $0.presentedNs == 0 }.count,
                        frameRateBoost: EngineV3FrameRateBoost.enabled,
                        boostFrameMs: Double(model.engineV3.view?.boostFrameNs ?? 0) / 1e6,
                        windowVisible: textView?.window?.occlusionState.contains(.visible) ?? false,
                        intervalMs: intervalMs, fastEdits: model.engineV3.fastEdits,
                        definition: "keystroke stamped (CLOCK_UPTIME_RAW) just before NSTextView.insertText/deleteBackward -> CATransaction.commit()+flush() on the main thread installing the new bitmap of the first page the resulting compile changed; vsync = the next CADisplayLink targetTimestamp after that commit; presented = the presentedTime of a transparent 1x1 CAMetalLayer drawable presented with that same transaction (presentsWithTransaction)",
                        perKeystrokeMs: ms,
                        stages: l.stageStats(),
                        samplesDetail: l.samples,
                        pixelsPerPoint: model.engineV3.view?.currentPixelsPerPoint ?? 0,
                        status: why)
        if let img = model.engineV3.view?.installedImage(0),
           let dest = CGImageDestinationCreateWithURL(out.deletingPathExtension().appendingPathExtension("page1.png") as CFURL, "public.png" as CFString, 1, nil) {
            CGImageDestinationAddImage(dest, img, nil); CGImageDestinationFinalize(dest)
        }
        let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        try? enc.encode(s).write(to: out)
        log("\(why): \(ms.count) samples, p50 \(st.p50Ms ?? -1) ms, p95 \(st.p95Ms ?? -1) ms (to vsync \(vs.p50Ms ?? -1) / \(vs.p95Ms ?? -1); to presented \(ps.p50Ms ?? -1) / \(ps.p95Ms ?? -1)) -> \(out.path)")
        model.engineV3.stop()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { exit(0) }
    }
}
