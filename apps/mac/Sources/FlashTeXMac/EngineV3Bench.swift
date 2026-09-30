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

    static func startIfConfigured(model: ShellModel) {
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

    private func zip2<A, B>(_ a: A?, _ b: B?) -> (A, B)? { if let a, let b { (a, b) } else { nil } }

    struct Summary: Codable {
        var document: String, pages: Int, keystrokes: Int, samples: Int, unchanged: Int, offscreen: Int, pending: Int
        var p50Ms: Double?, p95Ms: Double?, minMs: Double?, maxMs: Double?, meanMs: Double?
        var intervalMs: Double
        var definition: String
        var perKeystrokeMs: [Double]
        /// Phase medians over the samples: keystroke -> COMPILE written, -> first
        /// changed page on the main thread, its raster, and the rest (queueing, commit).
        var phaseP50Ms: [String: Double]
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
        let s = Summary(document: url.path, pages: model.engineV3.pageCount, keystrokes: typed, samples: ms.count,
                        unchanged: l.unchanged, offscreen: l.offscreenCount, pending: l.pendingCount,
                        p50Ms: st.p50Ms, p95Ms: st.p95Ms, minMs: st.minMs, maxMs: st.maxMs, meanMs: st.meanMs,
                        intervalMs: intervalMs,
                        definition: "keystroke stamped (CLOCK_UPTIME_RAW) just before NSTextView.insertText/deleteBackward -> CATransaction.commit()+flush() on the main thread installing the new bitmap of the first page the resulting compile changed",
                        perKeystrokeMs: ms,
                        phaseP50Ms: [
                            "key_to_hook": percentile(l.samples.compactMap { s in s.hookNs.map { Double($0 &- s.keystrokeNs) / 1e6 } }, 50) ?? -1,
                            "key_to_sent": percentile(l.samples.compactMap { $0.sentNs.map { Double($0 &- $0) } }.isEmpty ? [] : l.samples.compactMap { s in s.sentNs.map { Double($0 &- s.keystrokeNs) / 1e6 } }, 50) ?? -1,
                            "sent_to_page_on_main": percentile(l.samples.compactMap { s in zip2(s.sentNs, s.arrivedNs).map { Double($1 &- $0) / 1e6 } }, 50) ?? -1,
                            "raster": percentile(l.samples.compactMap(\.rasterMs), 50) ?? -1,
                            "page_on_main_to_commit": percentile(l.samples.compactMap { s in s.arrivedNs.map { Double(s.commitNs &- $0) / 1e6 } }, 50) ?? -1,
                        ],
                        samplesDetail: l.samples,
                        pixelsPerPoint: model.engineV3.view?.currentPixelsPerPoint ?? 0,
                        status: why)
        if let img = model.engineV3.view?.installedImage(0),
           let dest = CGImageDestinationCreateWithURL(out.deletingPathExtension().appendingPathExtension("page1.png") as CFURL, "public.png" as CFString, 1, nil) {
            CGImageDestinationAddImage(dest, img, nil); CGImageDestinationFinalize(dest)
        }
        let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        try? enc.encode(s).write(to: out)
        log("\(why): \(ms.count) samples, p50 \(st.p50Ms ?? -1) ms, p95 \(st.p95Ms ?? -1) ms -> \(out.path)")
        model.engineV3.stop()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { exit(0) }
    }
}
