import AppKit
import HostedWindows
import SwiftUI
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

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
    /// within a frame budget (`FLASHTEX_EDITOR_KEY_BUDGET_MS`, default 100 ms
    /// for a loaded CI runner; the freeze was well over a second).
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
        for out in ManyProblemsFixture.compile(id: 1, page: page, pages: n, warnings: ManyProblemsFixture.warnings(problems, path: path, lines: lines)) {
            delivery.post(out, connection: nil)
        }
        try await waitUntil("the problems listed", timeout: 60) { model.problemsList.count >= problems }
        try await Task.sleep(nanoseconds: 500_000_000) // the list laid out
        // A long compile is running: the host has started and not answered.
        delivery.post(.started(.object(["id": .int(2), "keep": .bool(true), "mode": .string("incremental")])), connection: nil)
        let phase = try await typePhase("problems", keys: 40, into: tv, session: s)
        report([phase], extra: ["pages": n, "problems": model.problemsList.count,
                                "groups": EditorDiagnostics.groups(of: model.problemsList).count])
        let budget = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_KEY_BUDGET_MS"].flatMap(Double.init) ?? 100
        XCTAssertFalse(phase.timedOut, "the main thread did not take the keys")
        XCTAssertEqual(phase.keys, 40)
        let glyph = phase.stats(phase.keyToGlyphMs), wait = RunLoopLatencyProbe.stats(phase.pingMs)
        XCTAssertLessThanOrEqual(glyph["p95"] ?? .infinity, budget, "key → glyph p95 with \(problems) problems shown: \(glyph)")
        XCTAssertLessThanOrEqual(wait["p99"] ?? .infinity, budget, "run-loop wait p99 with \(problems) problems shown: \(wait)")
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
    /// The main thread must be free (not running: `MainThreadFreeProbe`)
    /// for at least `FLASHTEX_EDITOR_FREE_MIN` of the time (default 0.6, for
    /// a debug build on a shared CI runner; a debug build on this Mac read
    /// 0.75, see the PR for its release numbers), and key → glyph must stay within
    /// `FLASHTEX_EDITOR_KEY_BUDGET_MS`.
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
        report([phase], extra: ["pages": n, "problems": problems, "compiles_done": s.doneCount - doneBefore,
                                "main_free": fraction, "measured_ms": ms])
        XCTAssertFalse(phase.timedOut, "the main thread did not take the keys")
        XCTAssertEqual(phase.keys, 2 * compiles)
        XCTAssertGreaterThan(s.doneCount - doneBefore, compiles / 2, "the compiles were answered while typing")
        // Every group is listed (no cap), from the last compile.
        XCTAssertEqual(model.engineV3Problems.groups.count, problems)
        let minFree = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_FREE_MIN"].flatMap(Double.init) ?? 0.6
        XCTAssertGreaterThanOrEqual(fraction, minFree, "main thread free \(fraction) of \(ms) ms while typing during compiles")
        let budget = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_KEY_BUDGET_MS"].flatMap(Double.init) ?? 100
        let glyph = phase.stats(phase.keyToGlyphMs)
        XCTAssertLessThanOrEqual(glyph["p95"] ?? .infinity, budget, "key → glyph p95 during compiles with \(problems) problems: \(glyph)")
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
