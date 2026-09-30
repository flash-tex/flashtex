import Foundation
import os

/// Keystroke → pixels for the engine-v3 preview, by stage.
///
/// A sample runs from the keystroke to the Core Animation commit of the
/// first page its compile changed (and on to the display link's next frame,
/// `vsyncNs`, the frame that shows it). Every stage boundary is also an
/// os_signpost event (subsystem `tech.jay3332.flashtex.mac`, category
/// `EngineV3Latency`; the interval `keystroke-to-pixels` spans send → commit),
/// so Instruments shows the same stages on a timeline.
///
/// Stages (all `CLOCK_UPTIME_RAW` / `DispatchTime` nanoseconds, one clock):
///   key      the key event (or the bench's stamp just before it edits)
///   edit     the change reached the engine-v3 hook: the editor's text
///            storage (fast path) or `ShellModel.updateActiveText`
///   sent     the COMPILE was written to the socket
///   read     the first changed PAGE frame was read off the socket (reader thread)
///   decoded  … decoded
///   prepared … its resources resolved
///   raster0/raster1  its bitmap drawn (reader thread, when on screen)
///   main     it reached the main thread
///   commit   `CATransaction.commit()` + `flush()` installed it
///   vsync    the display link's `targetTimestamp` after the commit
/// plus the host's own `first_page_ms` (COMPILE received → first page) from `DONE`.
@MainActor
final class EngineV3Latency {
    static let signposter = OSSignposter(subsystem: "tech.jay3332.flashtex.mac", category: "EngineV3Latency")

    struct PageTiming: Sendable {
        var readNs: UInt64 = 0, decodedNs: UInt64 = 0, preparedNs: UInt64 = 0
        var raster0Ns: UInt64 = 0, raster1Ns: UInt64 = 0
    }

    struct Sample: Codable {
        var compile: Int
        var page: Int
        var keyNs: UInt64
        var editNs: UInt64?
        var editPath: String?
        var sentNs: UInt64?
        var readNs: UInt64?, decodedNs: UInt64?, preparedNs: UInt64?
        var raster0Ns: UInt64?, raster1Ns: UInt64?
        var mainNs: UInt64?
        var commitNs: UInt64
        var vsyncNs: UInt64?
        var hostFirstPageMs: Double?
        var ms: Double { Double(commitNs &- keyNs) / 1e6 }
        var toVsyncMs: Double? { vsyncNs.map { Double($0 &- keyNs) / 1e6 } }
    }

    private struct Compile {
        var sentNs: UInt64?
        var editNs: UInt64?
        var editPath: String?
        var page: PageTiming?
        var mainNs: UInt64?
        var interval: OSSignpostIntervalState?
        var hostFirstPageMs: Double?
    }

    private var compiles: [Int: Compile] = [:]
    private var pending: [(compile: Int, ns: UInt64)] = []
    private(set) var samples: [Sample] = []
    /// Keystrokes whose compile changed no page (e.g. a comment).
    private(set) var unchanged = 0
    private var painted: Set<Int> = []
    private var expected: Set<Int> = []
    /// Keystrokes whose first changed page was not on screen.
    private(set) var offscreenCount = 0
    private var awaitingVsync: [Int] = [] // indexes into samples

    func sent(compile: Int, keystrokeNs: UInt64, editNs: UInt64, path: String, at ns: UInt64) {
        pending.append((compile, keystrokeNs))
        var c = compiles[compile] ?? Compile()
        c.sentNs = ns; c.editNs = editNs; c.editPath = path
        c.interval = Self.signposter.beginInterval("keystroke-to-pixels", id: Self.signposter.makeSignpostID(), "compile \(compile) \(path)")
        compiles[compile] = c
        Self.signposter.emitEvent("sent", "compile \(compile)")
    }

    /// The first changed page of `compile` reached the main thread.
    func pageOnMain(compile: Int, timing: PageTiming, at ns: UInt64) {
        expected.insert(compile)
        var c = compiles[compile] ?? Compile()
        guard c.mainNs == nil else { return }
        c.page = timing; c.mainNs = ns
        compiles[compile] = c
        Self.signposter.emitEvent("page-on-main", "compile \(compile)")
    }

    func offscreen(compile: Int) {
        guard expected.contains(compile), !painted.contains(compile) else { return }
        painted.insert(compile)
        offscreenCount += pending.filter { $0.compile <= compile }.count
        pending.removeAll { $0.compile <= compile }
        endInterval(compile)
    }

    /// The first changed page of `compile` was committed.
    func committed(compile: Int, page: Int, at ns: UInt64) {
        guard !painted.contains(compile) else { return }
        painted.insert(compile)
        Self.signposter.emitEvent("commit", "compile \(compile) page \(page)")
        endInterval(compile)
        let c = compiles[compile]
        let covered = pending.filter { $0.compile <= compile }
        pending.removeAll { $0.compile <= compile }
        for k in covered {
            let t = c?.page
            samples.append(Sample(compile: compile, page: page, keyNs: k.ns, editNs: c?.editNs, editPath: c?.editPath,
                                  sentNs: c?.sentNs, readNs: t?.readNs, decodedNs: t?.decodedNs, preparedNs: t?.preparedNs,
                                  raster0Ns: t.flatMap { $0.raster0Ns == 0 ? nil : $0.raster0Ns },
                                  raster1Ns: t.flatMap { $0.raster1Ns == 0 ? nil : $0.raster1Ns },
                                  mainNs: c?.mainNs, commitNs: ns, vsyncNs: nil, hostFirstPageMs: c?.hostFirstPageMs))
            awaitingVsync.append(samples.count - 1)
        }
    }

    var wantsVsync: Bool { !awaitingVsync.isEmpty }

    /// The display link fired: its `targetTimestamp` is when the frame that
    /// carries every commit before this callback is shown.
    func vsync(targetNs: UInt64) {
        for i in awaitingVsync where i < samples.count { samples[i].vsyncNs = targetNs }
        if !awaitingVsync.isEmpty { Self.signposter.emitEvent("vsync") }
        awaitingVsync = []
    }

    func done(compile: Int, cancelled: Bool, hostFirstPageMs: Double?) {
        if let hostFirstPageMs {
            compiles[compile, default: Compile()].hostFirstPageMs = hostFirstPageMs
            for i in samples.indices where samples[i].compile == compile { samples[i].hostFirstPageMs = hostFirstPageMs }
        }
        guard !cancelled, !expected.contains(compile) else { return }
        // This compile (and any it superseded) changed no page.
        unchanged += pending.filter { $0.compile <= compile }.count
        pending.removeAll { $0.compile <= compile }
        endInterval(compile)
    }

    private func endInterval(_ compile: Int) {
        if let s = compiles[compile]?.interval {
            Self.signposter.endInterval("keystroke-to-pixels", s)
            compiles[compile]?.interval = nil
        }
    }

    var pendingCount: Int { pending.count }
    func reset() {
        compiles = [:]; pending = []; samples = []; unchanged = 0; painted = []; expected = []; offscreenCount = 0; awaitingVsync = []
    }

    /// Medians and p95 of every stage over the samples.
    func stageStats() -> [String: [String: Double]] {
        func ms(_ a: UInt64?, _ b: UInt64?) -> Double? {
            guard let a, let b, a > 0, b >= a else { return nil }
            return Double(b - a) / 1e6
        }
        let stages: [(String, (Sample) -> Double?)] = [
            ("key_to_edit_hook", { ms($0.keyNs, $0.editNs) }),
            ("edit_hook_to_sent", { ms($0.editNs, $0.sentNs) }),
            ("sent_to_frame_read", { ms($0.sentNs, $0.readNs) }),
            ("host_first_page", { $0.hostFirstPageMs }),
            ("decode", { ms($0.readNs, $0.decodedNs) }),
            ("prepare", { ms($0.decodedNs, $0.preparedNs) }),
            ("raster", { ms($0.raster0Ns, $0.raster1Ns) }),
            ("to_main", { s in ms(s.raster1Ns ?? s.preparedNs, s.mainNs) }),
            ("main_to_commit", { ms($0.mainNs, $0.commitNs) }),
            ("commit_to_vsync", { ms($0.commitNs, $0.vsyncNs) }),
            ("key_to_commit", { $0.ms }),
            ("key_to_vsync", { $0.toVsyncMs }),
        ]
        var out: [String: [String: Double]] = [:]
        for (name, f) in stages {
            let v = samples.compactMap(f)
            guard !v.isEmpty else { continue }
            out[name] = ["p50": percentile(v, 50) ?? -1, "p95": percentile(v, 95) ?? -1, "n": Double(v.count)]
        }
        return out
    }
}
