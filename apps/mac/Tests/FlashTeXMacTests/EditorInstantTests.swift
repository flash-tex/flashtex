import AppKit
import HostedWindows
import SwiftUI
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// The limits of `EditorInstantTests.testTheMainThreadStaysFreeWhileCompilesReportThousandsOfProblems`,
/// every one relative to the same run's floor (the same keys with no compile answering).
///
/// Without `FLASHTEX_EDITOR_HOSTED_VM` (a Mac: the reference) they are as before:
///   - more than half the compiles were answered while the keys were typed (the load was applied);
///   - the main thread is busy at most `ratio` times as much as typing alone;
///   - key → glyph p95 is within `budgetMs` of typing alone's (or `ratio` times it).
///
/// With it (CI's mac-app job, a GitHub-hosted 3-vCPU VM) those limits failed 16 of main's 30 runs
/// of 2026-11-08 to 11-10 (`recordedHostedRuns`): the p95 bound in 15, by 1 to 183 ms. The VM
/// pauses: in run 42686171082 a key of the floor phase, with no compile to wait for, waited 1.1 s
/// for the main thread. A pause late-delivers every key posted during it (keys are 50 ms apart),
/// so the p95 of 40 keys is one long pause or two short ones. There the limits are (as
/// `RealDocStallLimits`, #2154, for the same VM):
///   - key → glyph p95 within `hardMs`, max(5 x budget, 500 ms), of typing alone's: the freezes
///     this test guards against cost seconds a key;
///   - key → glyph p50 within `budgetMs` of typing alone's: a pause moves the tail, while a
///     regression that costs every key or every compile's reply a long turn moves the median
///     (a compile answers every other key);
///   - the main thread's busy fraction, which is CPU time and so does not count a pause: at most
///     `ratio` times typing alone's, or `hostedBusyOver` more than it when that is more. Typing
///     alone was busy 16 % to 48 % of the time between those runs, and the compiles added up to 32
///     points: 2.5 x 16 % failed the run at 48 % that 2.5 x 25 % passes;
///   - a run in which half the compiles or fewer were answered (2 of the 30: 4 and 10 of 20, the
///     VM at under half speed) is not a measurement of the app: `notMeasured`, a skip, and only
///     once the p95 and busy limits above have held. It is never a pass, and a freeze still fails.
struct EditorCompileLoadLimits {
    var budgetMs = 100.0
    var ratio = 2.5
    var hosted = false
    /// The compiles' share of the main thread allowed on the hosted VM, as a fraction of the time.
    static let hostedBusyOver = 0.4

    struct Measured: Equatable {
        /// Key → glyph (ms): typing alone, then with the compiles answering.
        var floorP50: Double, floorP95: Double, p50: Double, p95: Double
        /// The main thread's busy fraction: typing alone, then with the compiles.
        var busyFloor: Double, busy: Double
        var compilesDone: Int, compiles: Int
    }

    enum Verdict: Equatable {
        case passed
        case failed([String])
        case notMeasured(String)
    }

    static func fromEnvironment(_ e: [String: String]) -> EditorCompileLoadLimits {
        EditorCompileLoadLimits(budgetMs: e["FLASHTEX_EDITOR_KEY_BUDGET_MS"].flatMap(Double.init) ?? 100,
                                ratio: e["FLASHTEX_EDITOR_BUSY_RATIO"].flatMap(Double.init) ?? 2.5,
                                hosted: !["", "0"].contains(e["FLASHTEX_EDITOR_HOSTED_VM"] ?? ""))
    }

    var hardMs: Double { max(5 * budgetMs, 500) }

    /// `floor` plus `over`, or `ratio` times `floor` when that is more (as `EditorInstantTests.keyBound`).
    func bound(_ floor: Double, over: Double) -> Double { max(floor + over, ratio * floor) }

    func busyBound(_ floor: Double) -> Double {
        let byRatio = ratio * max(0.02, floor)
        return hosted ? max(byRatio, floor + Self.hostedBusyOver) : byRatio
    }

    func verdict(_ m: Measured) -> Verdict {
        var misses: [String] = []
        let loadApplied = m.compilesDone > m.compiles / 2
        let answered = "\(m.compilesDone) of \(m.compiles) compiles were answered while the keys were typed"
        if m.busy > busyBound(m.busyFloor) {
            misses.append("main thread busy \(m.busy) while typing during compiles, against \(m.busyFloor) typing alone "
                + "(at most \(busyBound(m.busyFloor)))")
        }
        if hosted {
            let hard = bound(m.floorP95, over: hardMs), soft = bound(m.floorP50, over: budgetMs)
            if m.p95 > hard {
                misses.append("key → glyph p95 \(m.p95) ms during compiles, over the hard limit \(hard) ms (typing alone \(m.floorP95) ms)")
            }
            if loadApplied, m.p50 > soft {
                misses.append("key → glyph p50 \(m.p50) ms during compiles, over \(soft) ms (typing alone \(m.floorP50) ms)")
            }
        } else {
            let limit = bound(m.floorP95, over: budgetMs)
            if !loadApplied { misses.append("the compiles were not answered while typing: \(answered)") }
            if m.p95 > limit {
                misses.append("key → glyph p95 \(m.p95) ms during compiles, over \(limit) ms (typing alone \(m.floorP95) ms)")
            }
        }
        if !misses.isEmpty { return .failed(misses) }
        return loadApplied ? .passed : .notMeasured("not a measurement on this hosted VM: \(answered)")
    }

    /// main's mac-app runs of 2026-11-08 to 11-10 that printed this test's `EditorInstant:` line
    /// (GitHub-hosted, artifact `mac-swift-test-log`): the run, key → glyph p50 and p95 typing
    /// alone, the same during the compiles (ms), the busy fraction typing alone and during the
    /// compiles, and the compiles answered, of 20.
    private static let recordedRows: [(Int, Double, Double, Double, Double, Double, Double, Int)] = [
        (42532886711, 24.2, 38.8, 27.0, 90.2, 0.252, 0.468, 20), (42537390446, 26.4, 50.8, 45.7, 116.8, 0.267, 0.476, 20),
        (42625595127, 42.2, 126.3, 61.8, 193.4, 0.318, 0.464, 20), (42627671192, 24.0, 37.2, 69.0, 258.9, 0.260, 0.435, 20),
        (42628210854, 21.9, 32.6, 36.5, 106.4, 0.224, 0.512, 20), (42630208261, 16.7, 25.7, 52.1, 155.6, 0.161, 0.484, 20),
        (42633332042, 20.2, 33.9, 26.1, 74.0, 0.203, 0.460, 20), (42635826419, 20.4, 39.8, 22.6, 65.4, 0.221, 0.384, 20),
        (42637138187, 36.1, 67.5, 64.4, 172.9, 0.398, 0.554, 20), (42639683505, 36.7, 110.2, 26.8, 62.3, 0.331, 0.464, 20),
        (42640427015, 17.3, 25.5, 19.0, 37.0, 0.167, 0.299, 20), (42640550059, 25.3, 47.3, 47.5, 191.3, 0.273, 0.495, 20),
        (42641259981, 19.4, 30.1, 30.0, 56.1, 0.196, 0.418, 20), (42676708285, 16.9, 32.7, 23.5, 51.6, 0.163, 0.374, 20),
        (42685195078, 28.0, 50.8, 74.0, 276.9, 0.280, 0.465, 20), (42685396930, 22.6, 45.5, 20.8, 66.9, 0.261, 0.354, 20),
        (42686171082, 87.3, 367.2, 92.7, 222.5, 0.323, 0.550, 18), (42687384757, 19.9, 39.4, 49.2, 178.3, 0.266, 0.514, 20),
        (42688805911, 20.9, 36.9, 32.2, 120.8, 0.218, 0.429, 20), (42689032957, 23.4, 50.6, 52.4, 157.4, 0.261, 0.530, 20),
        (42697622454, 75.1, 177.1, 150.4, 404.4, 0.483, 0.304, 4), (42729417935, 30.0, 49.3, 83.6, 219.0, 0.334, 0.561, 17),
        (42731520153, 32.9, 40.8, 103.4, 246.8, 0.317, 0.452, 15), (42734504987, 24.5, 38.9, 47.7, 140.3, 0.263, 0.495, 20),
        (42739252236, 17.4, 26.4, 34.6, 170.0, 0.176, 0.391, 20), (42740742971, 17.3, 29.9, 37.5, 219.3, 0.170, 0.369, 20),
        (42740813798, 27.0, 41.7, 68.0, 214.1, 0.312, 0.461, 20), (42753653675, 21.0, 52.3, 128.6, 335.3, 0.254, 0.337, 10),
        (42910974948, 19.7, 29.5, 45.8, 149.3, 0.217, 0.362, 20), (42912571683, 24.2, 54.1, 36.8, 134.4, 0.281, 0.484, 20),
    ]

    static let recordedHostedRuns: [(run: Int, m: Measured)] = recordedRows.map { r in
        (run: r.0, m: Measured(floorP50: r.1, floorP95: r.2, p50: r.3, p95: r.4, busyFloor: r.5, busy: r.6, compilesDone: r.7, compiles: 20))
    }
}

/// APP-EDITOR-INSTANT (docs/evidence/editor-instant-2026-10-06): typing in the
/// editor is never held up by the preview. The owner typed into a 1,000-page
/// document while its first compile streamed pages in, and the editor lagged
/// with the preview.
///
/// The hosted window (ContentView, engine-v3 pane) holds an owner-sized
/// document (4.5 MB, ~1,142 pages: Infinite Descent ×2's page count) while a
/// background thread feeds the session synthetic host events through the
/// reader thread's own path (`EngineV3Delivery`), at host rate: a cold
/// compile's 1,142 PAGEs with PROGRESS, PAGES and warning frames, then a
/// keystroke compile per key, each with a DONE carrying 100 warnings.
/// Keys are posted to the main run loop from another thread every 50 ms, as
/// an input source arrives, so a key waits behind whatever main is doing.
///
/// Measured per key: posted → handled (queueing), the handler, posted → the
/// editor's next draw (`MainThreadProbe.editorDrew`) and → the end of the
/// run-loop turn that committed it (after Core Animation's commit). A ping
/// every 2 ms measures how long any event waits for main. The summary is
/// printed as one `EditorInstant:` JSON line (and written to
/// `FLASHTEX_EDITOR_INSTANT_OUT` when set).
///
/// `FLASHTEX_EDITOR_INSTANT_DOC=/path/infdesc-x2.tex` (with a built host and a
/// full TeX Live; `scripts/editor-instant-infdesc.sh`) runs the same typing
/// during a real cold compile of that document instead (`testTypingDuringARealColdCompile`).
@MainActor
final class EditorInstantTests: XCTestCase {
    private var env = EnvironmentOverride()
    private var window: NSWindow?

    override func setUp() async throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("editor-instant-\(getpid())").path)
    }

    override func tearDown() async throws {
        MainThreadProbe.stop()
        ViewBodyProbe.stop()
        window?.orderOut(nil)
        window?.contentView = nil
        window = nil
        env.restore()
    }

    // MARK: the document and the pages

    /// An owner-shaped book: the 1,000-page test's preamble, sections of six
    /// long paragraphs with inline math, a display equation each.
    nonisolated static func book(sections: Int) -> String {
        let words = ["lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", "sed", "do", "eiusmod",
                     "tempor", "incididunt", "ut", "labore", "et", "dolore", "magna", "aliqua", "enim", "ad", "minim", "veniam"]
        var s = """
        \\documentclass[11pt]{article}
        \\usepackage[margin=1in]{geometry}\\usepackage{amsmath,amssymb,amsthm,mathtools}\\usepackage{graphicx,booktabs,xcolor}
        \\usepackage{tikz}\\usetikzlibrary{arrows.meta,positioning,calc}\\usepackage{siunitx}\\usepackage{hyperref}\\usepackage{cleveref}
        \\begin{document}

        """
        var k = 0
        s.reserveCapacity(sections * 4200)
        for sec in 0 ..< sections {
            s += "\\section{Part \(sec / 12 + 1), Section \(sec % 12 + 1)}\n\n"
            for par in 0 ..< 6 {
                var line = ""
                for _ in 0 ..< 95 { line += words[k % words.count] + " "; k = (k &* 31 &+ 7) % 9973 }
                s += line + "with $x_{\(par)}^2+\\frac{a}{b}=\\sum_{i=1}^n c_i$.\n\n"
            }
            s += "\\begin{equation}\\int_0^\\infty e^{-x^2}\\,dx=\\frac{\\sqrt\\pi}{2}\\end{equation}\n\n"
        }
        return s + "\\end{document}\n"
    }

    /// `n` distinct pages made from the beamer fixture's first page.
    nonisolated static func pages(_ n: Int, salt: UInt8 = 0) throws -> [DL3PreparedPage] {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("FlashTeXDisplayListV3Tests/Fixtures/beamer-overlays.dl3")
        let base = try XCTUnwrap(DL3Document(frames: Array(Data(contentsOf: url))).orderedPages.first)
        return (0 ..< n).map { i in
            var p = base
            p.page.index = UInt32(i)
            var h = [UInt8](repeating: 0, count: 32)
            h[0] = UInt8(i & 0xFF); h[1] = UInt8((i >> 8) & 0xFF); h[2] = salt; h[3] = 0xA5
            p.page.hash = h
            return p
        }
    }

    // MARK: the driver

    /// Results of one phase.
    struct Phase: Codable {
        var name: String
        var keys = 0
        var keysDrawn = 0
        var keyToDrawnMs: [Double] = []
        var keyToCommitMs: [Double] = []
        var queueMs: [Double] = []
        var handlerMs: [Double] = []
        var pingMs: [Double] = []
        /// Main-thread time per section (ms): total, count, max.
        var sections: [String: [Double]] = [:]
        var pagesAtStart = 0, pagesAtEnd = 0
        var timedOut = false

        func stats(_ v: [Double]) -> [String: Double] {
            guard !v.isEmpty else { return [:] }
            let s = v.sorted()
            func p(_ q: Double) -> Double { s[min(s.count - 1, max(0, Int((q / 100 * Double(s.count)).rounded(.up)) - 1))] }
            return ["p50": p(50), "p95": p(95), "p99": p(99), "max": s.last!, "n": Double(s.count)]
        }

        var summary: [String: Any] {
            var o: [String: Any] = ["keys": keys, "keys_drawn": keysDrawn, "pages_at_start": pagesAtStart, "pages_at_end": pagesAtEnd, "timed_out": timedOut]
            o["key_to_drawn_ms"] = stats(keyToDrawnMs)
            o["key_to_commit_ms"] = stats(keyToCommitMs)
            o["queue_ms"] = stats(queueMs)
            o["handler_ms"] = stats(handlerMs)
            o["ping_ms"] = stats(pingMs)
            o["main_sections_ms"] = sections
            return o
        }

        /// Key → glyph: the draw when the editor drew, else the turn's commit.
        var keyToGlyphMs: [Double] { keysDrawn * 2 >= keys ? keyToDrawnMs : keyToCommitMs }
    }

    /// Shared between the posting threads and main.
    final class Box: @unchecked Sendable {
        let lock = NSLock()
        var stop = false
        var posts: [UInt64] = []
        var starts: [UInt64] = []
        var ends: [UInt64] = []
        var commits: [UInt64] = []
        var stopped: Bool { lock.lock(); defer { lock.unlock() }; return stop }
    }

    final class Feed: @unchecked Sendable {
        let outs: [EngineV3Reader.Output]
        init(_ outs: [EngineV3Reader.Output]) { self.outs = outs }
    }

    nonisolated static func onMain(_ block: @escaping @MainActor () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue) { MainActor.assumeIsolated { block() } }
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }

    /// Posts `outs` to the session from a background thread, `gapUs` apart.
    nonisolated static func feed(_ delivery: EngineV3Delivery, _ feed: Feed, gapUs: UInt32, box: Box) {
        let t = Thread {
            for out in feed.outs {
                if box.stopped { return }
                delivery.post(out, connection: nil)
                if gapUs > 0 { usleep(gapUs) }
            }
        }
        t.qualityOfService = .userInteractive
        t.start()
    }

    /// The cold compile: STARTED (a new document), every page with PROGRESS,
    /// PAGES every 25 pages, a warning every 10 pages, DONE.
    nonisolated static func coldCompile(_ pages: [DL3PreparedPage], id: Int, path: String, lines: Int) -> [EngineV3Reader.Output] {
        let n = pages.count
        var outs: [EngineV3Reader.Output] = [.started(.object(["id": .int(Int64(id)), "keep": .bool(false), "mode": .string("full")]))]
        for (i, p) in pages.enumerated() {
            outs.append(.progress(.object(["pass": .int(1), "page": .int(Int64(i + 1))])))
            outs.append(.page(p, compileID: id, timing: .init(), image: nil))
            if i % 10 == 9 { outs.append(.diagnostic(warning(path: path, line: (i * 37) % max(lines, 1) + 1))) }
            if i % 25 == 24 {
                outs.append(.pages(.object(["count": .int(Int64(i + 1)), "current": .array([.array([.int(0), .int(Int64(i))])])])))
            }
        }
        outs.append(.pages(.object(["count": .int(Int64(n)), "current": .array([.array([.int(0), .int(Int64(n - 1))])])])))
        outs.append(.done(.object(["id": .int(Int64(id)), "status": .string("ok"), "mode": .string("full"), "pages": .int(Int64(n)),
                                   "elapsed_ms": .double(2000)]), compileID: id))
        return outs
    }

    /// A keystroke's compile: STARTED, the edited page, PAGES, 100 warnings, DONE.
    nonisolated static func keystrokeCompile(_ page: DL3PreparedPage, n: Int, id: Int, path: String, lines: Int) -> [EngineV3Reader.Output] {
        var outs: [EngineV3Reader.Output] = [.started(.object(["id": .int(Int64(id)), "keep": .bool(true), "mode": .string("incremental")]))]
        outs.append(.page(page, compileID: id, timing: .init(), image: nil))
        outs.append(.pages(.object(["count": .int(Int64(n)), "current": .array([.array([.int(0), .int(Int64(n - 1))])])])))
        for w in 0 ..< 100 { outs.append(.diagnostic(warning(path: path, line: (w * 397) % max(lines, 1) + 1))) }
        outs.append(.done(.object(["id": .int(Int64(id)), "status": .string("ok"), "mode": .string("incremental"), "pages": .int(Int64(n)),
                                   "elapsed_ms": .double(20)]), compileID: id))
        return outs
    }

    nonisolated static func warning(path: String, line: Int) -> DL3JSON {
        .object(["severity": .string("warning"), "message": .string("Overfull \\hbox (12.3pt too wide) in paragraph"),
                 "file": .string(path), "line": .int(Int64(line))])
    }

    @discardableResult
    private func waitUntil(_ what: String, timeout: TimeInterval, failing: Bool = true, _ cond: @escaping @MainActor () -> Bool) async throws -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return true }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        if failing { XCTFail("timed out waiting for \(what)") }
        return false
    }

    private func host(_ model: ShellModel) async throws -> NSTextView {
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 1400, height: 900), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        let hosting = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.contentView = hosting
        window.orderFrontRegardless()
        hosting.layoutSubtreeIfNeeded()
        self.window = window
        var found: NSTextView?
        try await waitUntil("the editor's text view", timeout: 60) {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        return try XCTUnwrap(found)
    }

    /// Types `keys` keys 50 ms apart (a letter typed, then deleted), each posted
    /// to main from another thread, while a ping measures main's latency; runs
    /// `during` (on main) as typing starts and `keysDone` once the last key is handled.
    private func typePhase(_ name: String, keys: Int, into tv: NSTextView, session: EngineV3Session,
                           stamp: Bool = false, pauseEvery: Int = 0, pauseMs: UInt32 = 0,
                           during: (() -> Void)? = nil, keysDone: (() -> Void)? = nil) async throws -> Phase {
        let box = Box()
        var phase = Phase(name: name)
        phase.pagesAtStart = session.pageCount
        // The end of each run-loop turn, after Core Animation's commit (order 2,000,000).
        let observer = CFRunLoopObserverCreateWithHandler(nil, CFRunLoopActivity.beforeWaiting.rawValue | CFRunLoopActivity.exit.rawValue,
                                                          true, 2_000_001) { _, _ in
            let now = MonotonicClock.nowNs()
            box.lock.lock(); if box.commits.count < 200_000 { box.commits.append(now) }; box.lock.unlock()
        }
        CFRunLoopAddObserver(CFRunLoopGetMain(), observer, .commonModes)
        defer { CFRunLoopRemoveObserver(CFRunLoopGetMain(), observer, .commonModes) }
        MainThreadProbe.start()
        let ping = RunLoopLatencyProbe() // MainThreadStall.swift
        ping.start()
        during?()
        let tvRef = Unmanaged.passUnretained(tv)
        let sessionRef = EngineV3WeakRef(session)
        let typist = Thread {
            for k in 0 ..< keys {
                if box.stopped { return }
                usleep(pauseEvery > 0 && k > 0 && k % pauseEvery == 0 ? pauseMs * 1000 : 50_000)
                let t0 = MonotonicClock.nowNs()
                box.lock.lock(); box.posts.append(t0); box.lock.unlock()
                EditorInstantTests.onMain {
                    let start = MonotonicClock.nowNs()
                    let tv = tvRef.takeUnretainedValue()
                    if stamp { sessionRef.value?.nextKeystrokeNs = t0 }
                    if k % 2 == 0 { tv.insertText("x", replacementRange: tv.selectedRange()) } else { tv.deleteBackward(nil) }
                    let end = MonotonicClock.nowNs()
                    box.lock.lock(); box.starts.append(start); box.ends.append(end); box.lock.unlock()
                }
            }
        }
        typist.qualityOfService = .userInteractive
        typist.start()
        // Not a failure by itself: a main thread too busy to take the keys is
        // what this measures (the keys it did take are reported, `timed_out`).
        phase.timedOut = !(try await waitUntil("\(keys) keys handled", timeout: Double(keys) * (0.05 + Double(pauseMs) / 1000) + 240, failing: false) {
            box.lock.lock(); defer { box.lock.unlock() }; return box.ends.count >= keys
        })
        keysDone?()
        try await Task.sleep(nanoseconds: 300_000_000) // the last key's draw
        box.lock.lock(); box.stop = true; box.lock.unlock()
        let pings = ping.stop()
        MainThreadProbe.stop()
        box.lock.lock(); defer { box.lock.unlock() }
        let draws = MainThreadProbe.editorDraws
        phase.keys = box.ends.count
        for k in 0 ..< box.ends.count {
            let post = box.posts[k], start = box.starts[k], end = box.ends[k]
            let next = k + 1 < box.starts.count ? box.starts[k + 1] : UInt64.max
            phase.queueMs.append(Double(start &- post) / 1e6)
            phase.handlerMs.append(Double(end &- start) / 1e6)
            if let d = draws.first(where: { $0 >= end && $0 < next }) {
                phase.keysDrawn += 1
                phase.keyToDrawnMs.append(Double(d &- post) / 1e6)
            }
            if let c = box.commits.first(where: { $0 >= end }) { phase.keyToCommitMs.append(Double(c &- post) / 1e6) }
        }
        phase.pingMs = pings
        for (k, b) in MainThreadProbe.buckets {
            phase.sections[k] = [Double(b.totalNs) / 1e6, Double(b.count), Double(b.maxNs) / 1e6]
        }
        phase.pagesAtEnd = session.pageCount
        if let d = try? JSONSerialization.data(withJSONObject: phase.summary, options: [.sortedKeys]) {
            print("EditorInstantPhase \(name): " + String(decoding: d, as: UTF8.self))
        }
        return phase
    }

    private func report(_ phases: [Phase], extra: [String: Any] = [:]) {
        var o: [String: Any] = extra
        for p in phases { o[p.name] = p.summary }
        guard let d = try? JSONSerialization.data(withJSONObject: o, options: [.sortedKeys]) else { return }
        print("EditorInstant: " + String(decoding: d, as: UTF8.self))
        if let out = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_INSTANT_OUT"], !out.isEmpty {
            try? d.write(to: URL(fileURLWithPath: out))
        }
    }

    // MARK: the synthetic cold compile

    func testTypingStaysInstantWhileAColdThousandPageCompileStreams() async throws {
        env.set("FLASHTEX_HOST", "none") // the feed is the host
        let n = 1142
        let text = Self.book(sections: 1080)
        let lines = text.utf8.reduce(0) { $1 == 0x0A ? $0 + 1 : $0 }
        let model = ShellModel()
        model.replaceProject(entryText: text)
        model.engineV3Enabled = true
        let tv = try await host(model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await waitUntil("the pages view", timeout: 30) { s.view != nil }
        let caret = (tv.string as NSString).range(of: "Section 1}\n\n").location
        XCTAssertNotEqual(caret, NSNotFound)
        tv.setSelectedRange(NSRange(location: caret + 13, length: 0))
        tv.window?.makeFirstResponder(tv)
        try await Task.sleep(nanoseconds: 500_000_000)
        let path = model.activePath
        let delivery: EngineV3Delivery = s.delivery
        let coldPages = try Self.pages(n)
        let cold = Feed(Self.coldCompile(coldPages, id: 1, path: path, lines: lines))

        // A: typing alone.
        let idle = try await typePhase("idle", keys: 40, into: tv, session: s)

        // B: typing while the cold compile streams in at host rate (a page every 2 ms).
        let coldBox = Box()
        let coldPhase = try await typePhase("cold", keys: 40, into: tv, session: s) {
            Self.feed(delivery, cold, gapUs: 2000, box: coldBox)
        }
        try await waitUntil("the cold compile applied", timeout: 120) { s.doneCount >= 1 }

        // C: a keystroke compile answers each key (its DONE with 100 warnings).
        let edited = try Self.pages(4, salt: 7)
        let steadyBox = Box()
        let steady = try await typePhase("steady", keys: 40, into: tv, session: s) {
            let t = Thread {
                for k in 0 ..< 40 {
                    usleep(50_000)
                    if steadyBox.stopped { return }
                    let f = Feed(Self.keystrokeCompile(edited[k % 4], n: n, id: 10 + k, path: path, lines: lines))
                    for out in f.outs { delivery.post(out, connection: nil) }
                }
            }
            t.qualityOfService = .userInteractive
            t.start()
        }
        steadyBox.lock.lock(); steadyBox.stop = true; steadyBox.lock.unlock()

        // E: typing in bursts (3 keys, then 400 ms still): what runs once the
        // typing pauses (debounced whole-document scans) and holds the next key.
        let bursts = try await typePhase("bursts", keys: 24, into: tv, session: s, pauseEvery: 3, pauseMs: 400)

        // D: a cold compile with no typing: which views re-evaluate.
        let againPages = try Self.pages(n, salt: 9)
        let again = Feed(Self.coldCompile(againPages, id: 100, path: path, lines: lines))
        let doneBefore = s.doneCount
        ViewBodyProbe.start()
        MainThreadProbe.start()
        let t0 = Date()
        Self.feed(delivery, again, gapUs: 0, box: Box()) // a burst: the whole compile queued at once
        try await waitUntil("the burst applied", timeout: 120) { s.doneCount > doneBefore }
        let burstMs = Date().timeIntervalSince(t0) * 1000
        try await Task.sleep(nanoseconds: 300_000_000)
        ViewBodyProbe.stop()
        MainThreadProbe.stop()
        var burstSections: [String: [Double]] = [:]
        for (k, b) in MainThreadProbe.buckets { burstSections[k] = [Double(b.totalNs) / 1e6, Double(b.count), Double(b.maxNs) / 1e6] }

        print("EditorInstantEnv: bridge \(model.bridgeStatus)")
        report([idle, coldPhase, steady, bursts], extra: ["pages": n, "document_bytes": text.utf8.count, "burst_applied_ms": burstMs,
                                                  "burst_bodies": ViewBodyProbe.counts, "burst_main_sections_ms": burstSections])
        XCTAssertEqual(s.pageCount, n)
        for p in [idle, coldPhase, steady, bursts] { XCTAssertGreaterThan(p.keys, 0, "\(p.name): no key was handled") }
    }

    // MARK: thousands of problems on screen (EDITOR-FREEZE)

    /// The owner typed into Infinite Descent ×2 (1,152 pages) while a long
    /// compile ran, and the editor did not even show the typed text: the
    /// Problems panel listed thousands of distinct overfull/underfull groups,
    /// and every row read the editor's text (for its Fix… link), so each key
    /// re-diffed the whole list on the main thread (seconds per key). Here:
    /// the panel shows 3,000 groups, a compile is running (STARTED, no DONE),
    /// and 40 keys are typed. Key → glyph and the run loop's wait must stay
    /// within `keyBound` of the same 40 keys typed before the problems were
    /// listed (the floor plus `FLASHTEX_EDITOR_KEY_BUDGET_MS`, default 100 ms,
    /// or 2.5 times the floor on a slow runner; the freeze was well over a second).
    func testTypingStaysInstantWithThousandsOfProblemsShown() async throws {
        env.set("FLASHTEX_HOST", "none") // the feed is the host
        let n = 1142
        let text = Self.book(sections: 1080)
        let lines = text.utf8.reduce(0) { $1 == 0x0A ? $0 + 1 : $0 }
        let model = ShellModel()
        model.replaceProject(entryText: text)
        model.engineV3Enabled = true
        model.problemsVisible = true
        let tv = try await host(model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await waitUntil("the pages view", timeout: 30) { s.view != nil }
        let caret = (tv.string as NSString).range(of: "Section 1}\n\n").location
        XCTAssertNotEqual(caret, NSNotFound)
        tv.setSelectedRange(NSRange(location: caret + 13, length: 0))
        tv.window?.makeFirstResponder(tv)
        let path = model.activePath
        let delivery: EngineV3Delivery = s.delivery
        let problems = 3000
        let page = try XCTUnwrap(Self.pages(1).first)
        // The floor: the same keys with no problems listed, what typing itself
        // costs on this machine and build (on the hosted runner its p95 ranges
        // from 27 to 256 ms between runs of one commit).
        let floorPhase = try await typePhase("problems_floor", keys: 40, into: tv, session: s)
        for out in ManyProblemsFixture.compile(id: 1, page: page, pages: n, warnings: ManyProblemsFixture.warnings(problems, path: path, lines: lines)) {
            delivery.post(out, connection: nil)
        }
        try await waitUntil("the problems listed", timeout: 60) { model.problemsList.count >= problems }
        try await Task.sleep(nanoseconds: 500_000_000) // the list laid out
        // A long compile is running: the host has started and not answered.
        delivery.post(.started(.object(["id": .int(2), "keep": .bool(true), "mode": .string("incremental")])), connection: nil)
        let phase = try await typePhase("problems", keys: 40, into: tv, session: s)
        report([floorPhase, phase], extra: ["pages": n, "problems": model.problemsList.count,
                                            "groups": EditorDiagnostics.groups(of: model.problemsList).count])
        XCTAssertFalse(phase.timedOut, "the main thread did not take the keys")
        XCTAssertEqual(phase.keys, 40)
        let glyph = phase.stats(phase.keyToGlyphMs), wait = RunLoopLatencyProbe.stats(phase.pingMs)
        let alone = floorPhase.stats(floorPhase.keyToGlyphMs), aloneWait = RunLoopLatencyProbe.stats(floorPhase.pingMs)
        let glyphBound = Self.keyBound(floor: alone["p95"] ?? 0), waitBound = Self.keyBound(floor: aloneWait["p99"] ?? 0)
        XCTAssertLessThanOrEqual(glyph["p95"] ?? .infinity, glyphBound,
                                 "key → glyph p95 with \(problems) problems shown: \(glyph); with none: \(alone)")
        XCTAssertLessThanOrEqual(wait["p99"] ?? .infinity, waitBound,
                                 "run-loop wait p99 with \(problems) problems shown: \(wait); with none: \(aloneWait)")
    }

    /// The bound on a key latency measured against the same run's floor (the
    /// same keys without the load under test): the floor plus
    /// `FLASHTEX_EDITOR_KEY_BUDGET_MS` (default 100), or `FLASHTEX_EDITOR_BUSY_RATIO`
    /// (default 2.5) times the floor when that is more. On this Mac the floor
    /// is a few ms and the first term holds; on the hosted runner, where one
    /// commit's floor p95 went from 27 to 256 ms between runs (main, runs
    /// 41305845461 to 41318785370), a fixed 100 ms over it failed one run in
    /// three. The freezes these tests guard against cost seconds a key.
    static func keyBound(floor: Double) -> Double {
        let e = ProcessInfo.processInfo.environment
        let budget = e["FLASHTEX_EDITOR_KEY_BUDGET_MS"].flatMap(Double.init) ?? 100
        let ratio = e["FLASHTEX_EDITOR_BUSY_RATIO"].flatMap(Double.init) ?? 2.5
        return max(floor + budget, ratio * floor)
    }

    // MARK: the main thread stays free while compiles report thousands of problems (APP-MAIN-THREAD)

    /// After #1890 the owner's sample of typing into Infinite Descent ×2
    /// found the main thread busy at each compile's reply: it made the
    /// reply's thousands of Problems rows (~25 ms), and the panel's List made
    /// every row twice a second. Here, as there, the author types in one
    /// chapter (a 250 KB text) of a book whose compiles report 3,000 warnings,
    /// one in 30 of them in that chapter: the Problems panel shows every
    /// group, 40 keys are typed 50 ms apart, and a compile answers every other
    /// key with 3,000 warnings whose lines all moved (every group changes).
    /// The main thread's free time (not running: `MainThreadFreeProbe`) is
    /// measured twice in the same run: first while the same 40 keys are
    /// typed with the problems listed and no compile answering (the floor:
    /// what typing itself costs on this machine and build), then with the
    /// compiles. With the compiles, the main thread may be busy at most
    /// `FLASHTEX_EDITOR_BUSY_RATIO` (default 2.5) times as much as typing
    /// alone: the compiles' replies cost at most 1.5 times the typing. A
    /// fixed free fraction (0.6) did not hold on the hosted runner, whose
    /// typing alone is far slower than this Mac's (a debug build here: 13 %
    /// busy typing alone, 24 % with the compiles, a ratio of 1.85); the
    /// ratio is the compiles' share whatever the runner's speed.
    /// `FLASHTEX_EDITOR_FREE_MIN` adds an absolute bound when set. Key →
    /// glyph p95 is bounded by `keyBound` of typing alone's. These limits,
    /// and the ones CI's hosted VM gets (`FLASHTEX_EDITOR_HOSTED_VM`), are
    /// `EditorCompileLoadLimits`.
    func testTheMainThreadStaysFreeWhileCompilesReportThousandsOfProblems() async throws {
        env.set("FLASHTEX_HOST", "none") // the feed is the host
        let n = 1142
        let text = Self.book(sections: 60)
        let lines = text.utf8.reduce(0) { $1 == 0x0A ? $0 + 1 : $0 }
        let model = ShellModel()
        model.replaceProject(entryText: text)
        model.engineV3Enabled = true
        model.problemsVisible = true
        let tv = try await host(model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await waitUntil("the pages view", timeout: 30) { s.view != nil }
        let caret = (tv.string as NSString).range(of: "Section 1}\n\n").location
        XCTAssertNotEqual(caret, NSNotFound)
        tv.setSelectedRange(NSRange(location: caret + 13, length: 0))
        tv.window?.makeFirstResponder(tv)
        let path = model.activePath
        let delivery: EngineV3Delivery = s.delivery
        let problems = 3000
        let page = try XCTUnwrap(Self.pages(1).first)
        for out in ManyProblemsFixture.compile(id: 1, page: page, pages: n,
                                                warnings: ManyProblemsFixture.warnings(problems, path: path, lines: lines, chapters: 30)) {
            delivery.post(out, connection: nil)
        }
        try await waitUntil("the problems listed", timeout: 60) { model.problems.groups.count >= problems }
        try await Task.sleep(nanoseconds: 500_000_000) // the list laid out
        // Every compile's frames, made before typing starts (the reader thread's work, not main's).
        let compiles = 20
        let feeds = (0 ..< compiles).map { k in
            Feed(ManyProblemsFixture.compile(id: 10 + k, page: page, pages: n,
                                             warnings: ManyProblemsFixture.warnings(problems, path: path, lines: lines, shift: k + 1, chapters: 30)))
        }
        // The floor: the same keys, no compile answering.
        let floor = MainThreadFreeProbe()
        let floorPhase = try await typePhase("problems_floor", keys: 2 * compiles, into: tv, session: s,
                                             during: { floor.start() }, keysDone: { floor.stop() })
        let (floorFraction, floorMs) = floor.result
        let free = MainThreadFreeProbe()
        let feedBox = Box()
        let doneBefore = s.doneCount
        let phase = try await typePhase("problems_compiles", keys: 2 * compiles, into: tv, session: s, during: {
            free.start()
            let t = Thread {
                for f in feeds {
                    usleep(100_000) // a compile per two keys
                    if feedBox.stopped { return }
                    for out in f.outs { delivery.post(out, connection: nil) }
                }
            }
            t.qualityOfService = .userInteractive
            t.start()
        }, keysDone: { free.stop() })
        feedBox.lock.lock(); feedBox.stop = true; feedBox.lock.unlock()
        try await waitUntil("the compiles' rows published", timeout: 60) { s.doneCount > doneBefore && !s.problemsPending }
        let (fraction, ms) = free.result
        report([floorPhase, phase], extra: ["pages": n, "problems": problems, "compiles_done": s.doneCount - doneBefore,
                                            "main_free": fraction, "measured_ms": ms,
                                            "main_free_floor": floorFraction, "floor_measured_ms": floorMs])
        XCTAssertFalse(phase.timedOut, "the main thread did not take the keys")
        XCTAssertEqual(phase.keys, 2 * compiles)
        // Every group is listed (no cap), from the last compile.
        XCTAssertEqual(model.engineV3Problems.groups.count, problems)
        let e = ProcessInfo.processInfo.environment
        if let minFree = e["FLASHTEX_EDITOR_FREE_MIN"].flatMap(Double.init) {
            XCTAssertGreaterThanOrEqual(fraction, minFree, "main thread free \(fraction) of \(ms) ms while typing during compiles")
        }
        // The compiles answered while typing, the busy fractions and key → glyph, against the floor.
        let glyph = phase.stats(phase.keyToGlyphMs), alone = floorPhase.stats(floorPhase.keyToGlyphMs)
        let limits = EditorCompileLoadLimits.fromEnvironment(e)
        let measured = EditorCompileLoadLimits.Measured(
            floorP50: alone["p50"] ?? 0, floorP95: alone["p95"] ?? 0, p50: glyph["p50"] ?? .infinity, p95: glyph["p95"] ?? .infinity,
            busyFloor: 1 - floorFraction, busy: 1 - fraction, compilesDone: s.doneCount - doneBefore, compiles: compiles)
        let detail = "measured over \(ms) ms (typing alone: \(floorMs) ms) with \(problems) problems; key → glyph during compiles: \(glyph); "
            + "typing alone: \(alone); hosted VM limits: \(limits.hosted)"
        switch limits.verdict(measured) {
        case .passed: break
        case .failed(let misses): XCTFail(misses.joined(separator: "; ") + ". " + detail)
        case .notMeasured(let why): throw XCTSkip(why + ". " + detail)
        }
    }

    /// `EditorCompileLoadLimits` on what main's hosted runs measured, and on the freezes the test
    /// above is for. No window and no clock: the limits are a pure value.
    func testCompileLoadLimitsPassTheRecordedHostedRunsAndFailTheFreezes() {
        typealias L = EditorCompileLoadLimits
        let strict = L(), hosted = L(hosted: true)
        XCTAssertEqual([hosted.hardMs, hosted.budgetMs, hosted.ratio], [500, 100, 2.5])
        let runs = L.recordedHostedRuns
        XCTAssertEqual(runs.count, 30)
        func failed(_ v: L.Verdict) -> Bool { if case .failed = v { return true } else { return false } }
        // The limits as they were (a Mac's) fail 16 of the 30 on the hosted VM's noise, as main did.
        let strictFailures = runs.filter { failed(strict.verdict($0.m)) }.map { $0.run }
        XCTAssertEqual(strictFailures.count, 16, "\(strictFailures)")
        XCTAssertTrue(strictFailures.contains(42910974948) && !strictFailures.contains(42912571683), "\(strictFailures)")
        // The hosted limits: none fails; the two runs with half the compiles or fewer are not measurements.
        for r in runs {
            let v = hosted.verdict(r.m)
            if r.m.compilesDone > 10 {
                XCTAssertEqual(v, .passed, "run \(r.run)")
            } else if case .notMeasured = v {
                continue
            } else {
                XCTFail("run \(r.run): \(v)")
            }
        }
        XCTAssertEqual(runs.filter { $0.m.compilesDone <= 10 }.map { $0.run }, [42697622454, 42753653675])
        // The freezes fail under both, from a hosted run's own floor (42912571683).
        let base = runs[29].m
        func with(_ change: (inout L.Measured) -> Void) -> L.Measured { var m = base; change(&m); return m }
        let freezes: [(String, L.Measured)] = [
            // every key re-diffs the Problems list (the owner's Infinite Descent x2: seconds a key)
            ("seconds a key", with { $0.p50 = 1900; $0.p95 = 2400 }),
            // the same, so slow that hardly a compile is answered: still a failure, not a skip
            ("seconds a key, few compiles answered", with { $0.p50 = 1900; $0.p95 = 2400; $0.compilesDone = 3 }),
            // every reply costs a long turn (after #1890: the reply's rows made on main): the median key waits
            ("a long turn per reply", with { $0.p50 = base.floorP50 + 150; $0.p95 = base.floorP95 + 300 }),
            // a long stall now and then: the tail past the hard limit
            ("stalls of over half a second", with { $0.p95 = base.floorP95 + 600 }),
            // the rows made on main at every reply, in turns too short to make a key late: CPU time
            ("main thread busy with the replies", with { $0.busy = 0.95 }),
        ]
        for (name, m) in freezes {
            XCTAssertTrue(failed(strict.verdict(m)), "\(name), a Mac's limits: \(strict.verdict(m))")
            XCTAssertTrue(failed(hosted.verdict(m)), "\(name), the hosted VM's limits: \(hosted.verdict(m))")
        }
        // A Mac's measurement (floor p95 8-29 ms on an M5 Pro) passes both; the allowances are the VM's only.
        let mac = L.Measured(floorP50: 6, floorP95: 12, p50: 8, p95: 30, busyFloor: 0.13, busy: 0.24, compilesDone: 20, compiles: 20)
        XCTAssertEqual([strict.verdict(mac), hosted.verdict(mac)], [.passed, .passed])
        var late = mac
        late.p95 = 130 // over 100 ms more than typing alone's: a Mac fails; the VM's noise did this in half its runs
        XCTAssertTrue(failed(strict.verdict(late)), "\(strict.verdict(late))")
        XCTAssertEqual(hosted.verdict(late), .passed)
        var busy = mac
        busy.busy = 0.4 // 3 x typing alone's: over the ratio on a Mac, within typing alone's + 0.4 on the VM
        XCTAssertTrue(failed(strict.verdict(busy)), "\(strict.verdict(busy))")
        XCTAssertEqual(hosted.verdict(busy), .passed)
        XCTAssertTrue(L.fromEnvironment(["FLASHTEX_EDITOR_HOSTED_VM": "1"]).hosted)
        XCTAssertFalse(L.fromEnvironment([:]).hosted || L.fromEnvironment(["FLASHTEX_EDITOR_HOSTED_VM": "0"]).hosted)
    }

    // MARK: a real cold compile (evidence; FLASHTEX_EDITOR_INSTANT_DOC)

    func testTypingDuringARealColdCompile() async throws {
        guard let doc = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_INSTANT_DOC"], !doc.isEmpty else {
            throw XCTSkip("evidence only: set FLASHTEX_EDITOR_INSTANT_DOC (scripts/editor-instant-infdesc.sh)")
        }
        try EngineV3TestHost.require()
        let model = ShellModel()
        _ = model.openTex(at: URL(fileURLWithPath: doc))
        model.engineV3Enabled = true
        model.autoCompile = true
        let tv = try await host(model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await EngineV3TestHost.awaitReady(s)
        // Type as soon as the first pages are on screen, during the cold compile.
        try await waitUntil("the first page", timeout: 300) { s.pageCount > 0 }
        let ns = tv.string as NSString
        var at = ns.range(of: "\\begin{document}").location
        at = at == NSNotFound ? 0 : NSMaxRange(ns.range(of: "\n", range: NSRange(location: at, length: ns.length - at)))
        tv.setSelectedRange(NSRange(location: at, length: 0))
        tv.window?.makeFirstResponder(tv)
        let keys = Int(ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_INSTANT_KEYS"] ?? "") ?? 120
        let cold = try await typePhase("real_cold", keys: keys, into: tv, session: s, stamp: true)
        try await waitUntil("the compile settled", timeout: 1800) { !s.compiling && s.statusNote.hasPrefix("ok") }
        let after = try await typePhase("real_after", keys: 40, into: tv, session: s, stamp: true)
        report([cold, after], extra: ["document": doc, "pages": s.pageCount, "status": s.statusNote])
        if let budget = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_KEY_BUDGET_MS"].flatMap(Double.init) {
            let p99 = cold.stats(cold.keyToGlyphMs)["p99"] ?? .infinity
            XCTAssertLessThanOrEqual(p99, budget, "key → glyph p99 during the cold compile")
        }
    }
}
