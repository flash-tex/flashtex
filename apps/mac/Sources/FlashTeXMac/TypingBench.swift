import AppKit
import Foundation

// Keystroke -> paint latency instrumentation and the programmatic typing bench.
//
// Nothing here needs Accessibility permission: keystrokes are observed with an
// in-process `NSEvent` local monitor (our own window's events only) and the
// "paint" point is derived from the preview's own render pass. See
// `docs/evidence/typing-bench-*.md` for the definition and its limitations.
//
// Owner: mac-typing-bench. Hooks (one line each) live in `SourceEditorView`
// (delegate stamp), `ShellModel` (revision + compile), `PreviewView`
// (render + draw) and `FlashTeXMacApp` (install).

/// Monotonic nanoseconds: `mach_absolute_time()` after timebase conversion
/// (`CLOCK_UPTIME_RAW`), the same clock `NSEvent.timestamp` and
/// `ProcessInfo.systemUptime` are expressed in (as seconds).
enum MonotonicClock {
    static func nowNs() -> UInt64 { clock_gettime_nsec_np(CLOCK_UPTIME_RAW) }
    static func ns(fromUptimeSeconds s: TimeInterval) -> UInt64 { UInt64(max(0, s) * 1_000_000_000) }
}

private func zip2<A, B>(_ a: A?, _ b: B?) -> (A, B)? { if let a, let b { (a, b) } else { nil } }

/// Nearest-rank percentile over a sample; `p` in 0...100. Empty -> nil.
func percentile(_ samples: [Double], _ p: Double) -> Double? {
    guard !samples.isEmpty else { return nil }
    let sorted = samples.sorted()
    let rank = Int((p / 100 * Double(sorted.count)).rounded(.up))
    return sorted[min(max(rank, 1), sorted.count) - 1]
}

struct LatencyStats: Codable, Equatable {
    var count: Int
    var p50Ms: Double?, p95Ms: Double?, p99Ms: Double?, maxMs: Double?, minMs: Double?, meanMs: Double?

    init(_ samples: [Double]) {
        count = samples.count
        p50Ms = percentile(samples, 50); p95Ms = percentile(samples, 95); p99Ms = percentile(samples, 99)
        maxMs = samples.max(); minMs = samples.min()
        meanMs = samples.isEmpty ? nil : samples.reduce(0, +) / Double(samples.count)
    }

    enum CodingKeys: String, CodingKey {
        case count, p50Ms = "p50_ms", p95Ms = "p95_ms", p99Ms = "p99_ms", maxMs = "max_ms", minMs = "min_ms", meanMs = "mean_ms"
    }
}

/// Pure aggregation of keystroke / revision / compile / paint events. Main-thread
/// use only (no locking); unit-tested in `TypingBenchTests`.
final class TypingLatencyRecorder {
    struct Keystroke: Codable, Equatable {
        /// Editor revision this keystroke produced.
        var revision: Int
        /// When the key event was stamped (HID time for real typing, call time for the bench).
        var keystrokeNs: UInt64
        /// When the text view delegate reported the change (nil when not stamped).
        var delegateNs: UInt64?
        var paintNs: UInt64?
        /// Revision whose paint made this keystroke visible (> `revision` when coalesced).
        var paintedByRevision: Int?
        /// Compile round trip of the request that painted it, when known.
        var compileMs: Double?
        /// Whether the paint that covered it actually redrew page canvases.
        var redrawn: Bool?

        var latencyMs: Double? { paintNs.map { Double($0 &- keystrokeNs) / 1e6 } }
        var coalesced: Bool { paintedByRevision.map { $0 != revision } ?? false }

        enum CodingKeys: String, CodingKey {
            case revision, keystrokeNs = "keystroke_ns", delegateNs = "delegate_ns", paintNs = "paint_ns",
                 paintedByRevision = "painted_by_revision", compileMs = "compile_ms", redrawn
        }
    }
    struct Paint: Equatable {
        var revision: Int; var ns: UInt64; var redrawn: Bool; var covered: Int
        /// When the compile result for this revision was applied (nil: no result, e.g. fixture).
        var resultNs: UInt64?
        /// When SwiftUI began rendering the revision and when its last page canvas drew.
        var renderStartNs: UInt64?, drawEndNs: UInt64?
        var renderPassMs: Double? { zip2(renderStartNs, drawEndNs).map { Double($1 &- $0) / 1e6 } }
        var resultToPaintMs: Double? { resultNs.map { Double(ns &- $0) / 1e6 } }
    }

    /// Sink for `keystroke:` / `paint:` lines (FlashTeXLog in the app).
    var log: (String) -> Void
    /// Oldest painted keystrokes are dropped beyond this (unbounded typing sessions).
    var capacity = 20_000

    private(set) var pendingKeystrokeNs: UInt64?
    private(set) var pendingDelegateNs: UInt64?
    private(set) var keystrokes: [Keystroke] = []
    private(set) var paints: [Paint] = []
    private(set) var compilesMs: [Int: Double] = [:]
    private(set) var resultNs: [Int: UInt64] = [:]
    private(set) var lastPaintedRevision = 0
    /// Revisions produced without a keystroke (fixture load, programmatic edits).
    private(set) var nonKeystrokeRevisions = 0

    init(log: @escaping (String) -> Void = { _ in }) { self.log = log }

    /// Arms a keystroke: the next revision bump adopts this timestamp.
    func keystroke(at ns: UInt64) { pendingKeystrokeNs = ns; pendingDelegateNs = nil }
    /// The text view delegate reported a change (keeps the earlier key stamp).
    func delegateReported(at ns: UInt64) { pendingDelegateNs = ns }
    /// A key event that changed no text must not be attributed to a later edit.
    func clearPendingKeystroke() { pendingKeystrokeNs = nil; pendingDelegateNs = nil }

    func revision(_ n: Int, at ns: UInt64) {
        guard let key = pendingKeystrokeNs else { nonKeystrokeRevisions += 1; return }
        let delegate = pendingDelegateNs
        pendingKeystrokeNs = nil; pendingDelegateNs = nil
        keystrokes.append(Keystroke(revision: n, keystrokeNs: key, delegateNs: delegate))
        if keystrokes.count > capacity, let i = keystrokes.firstIndex(where: { $0.paintNs != nil }) {
            keystrokes.remove(at: i)
        }
        log("keystroke: revision \(n) at \(key)" + (delegate.map { " (delegate \($0), revision \(ns))" } ?? " (revision \(ns))"))
    }

    func compile(revision: Int, ms: Double, at ns: UInt64? = nil) { compilesMs[revision] = ms; resultNs[revision] = ns }

    /// A paint of `revision` makes every unpainted keystroke with revision <= it
    /// visible. Repeated or older paints are ignored. Returns the covered count.
    @discardableResult
    func paint(revision: Int, at ns: UInt64, redrawn: Bool = true, renderStartNs: UInt64? = nil, drawEndNs: UInt64? = nil) -> Int {
        guard revision > lastPaintedRevision else { return 0 }
        lastPaintedRevision = revision
        var covered = 0
        for i in keystrokes.indices where keystrokes[i].paintNs == nil && keystrokes[i].revision <= revision {
            keystrokes[i].paintNs = ns
            keystrokes[i].paintedByRevision = revision
            keystrokes[i].compileMs = compilesMs[revision]
            keystrokes[i].redrawn = redrawn
            covered += 1
        }
        paints.append(Paint(revision: revision, ns: ns, redrawn: redrawn, covered: covered, resultNs: resultNs[revision],
                            renderStartNs: renderStartNs, drawEndNs: drawEndNs))
        log("paint: revision \(revision) at \(ns) (covers \(covered) keystrokes, redrawn \(redrawn))")
        return covered
    }

    var unpainted: [Keystroke] { keystrokes.filter { $0.paintNs == nil } }
    var painted: [Keystroke] { keystrokes.filter { $0.paintNs != nil } }
    var latenciesMs: [Double] { keystrokes.compactMap(\.latencyMs) }
    var coalescedCount: Int { keystrokes.filter(\.coalesced).count }

    func reset() {
        pendingKeystrokeNs = nil; pendingDelegateNs = nil
        keystrokes = []; paints = []; compilesMs = [:]; resultNs = [:]; lastPaintedRevision = 0; nonKeystrokeRevisions = 0
    }
}

/// JSON summary written by the bench (`FLASHTEX_TYPING_BENCH_OUT`).
struct TypingBenchSummary: Codable {
    var producer: String
    var script: String
    var intervalMs: Double
    /// Keystrokes in the script; `typed` is smaller when the typing budget ran out.
    var scriptKeystrokes: Int
    var typed: Int
    var typingBudgetExhausted: Bool
    var keystrokes: Int
    var painted: Int
    var unpainted: Int
    var coalesced: Int
    var paints: Int
    var paintsWithoutRedraw: Int
    var compiles: Int
    var documentBytesBefore: Int
    var documentBytesAfter: Int
    var elapsedMs: Double
    var keystrokeToPaintMs: LatencyStats
    var compileMs: LatencyStats
    /// PreviewView body -> last page canvas draw, per painted revision (main-thread render cost).
    var renderPassMs: LatencyStats
    /// Compile result applied -> paint recorded.
    var resultToPaintMs: LatencyStats
    var debounceMs: Double
    var paintPoint: String
    var perKeystroke: [TypingLatencyRecorder.Keystroke]
    var startedAt: String

    enum CodingKeys: String, CodingKey {
        case producer, script, intervalMs = "interval_ms", scriptKeystrokes = "script_keystrokes", typed,
             typingBudgetExhausted = "typing_budget_exhausted", keystrokes, painted, unpainted, coalesced, paints,
             paintsWithoutRedraw = "paints_without_redraw", compiles,
             documentBytesBefore = "document_bytes_before", documentBytesAfter = "document_bytes_after",
             elapsedMs = "elapsed_ms", keystrokeToPaintMs = "keystroke_to_paint_ms", compileMs = "compile_ms",
             renderPassMs = "render_pass_ms", resultToPaintMs = "result_to_paint_ms",
             debounceMs = "debounce_ms", paintPoint = "paint_point", perKeystroke = "per_keystroke", startedAt = "started_at"
        }
}

/// Bench configuration parsed from the environment (`FLASHTEX_TYPING_BENCH=<path>`).
struct TypingBenchConfig: Equatable {
    var scriptPath: String
    var intervalMs: Double = 30
    var outputPath: String
    /// How long to wait for the last keystroke's paint before giving up.
    var settleTimeoutMs: Double = 10_000
    /// Wall-clock budget for typing; when exhausted the remaining script is
    /// skipped (recorded as `typed < script_keystrokes`) and the run settles.
    var typingBudgetMs: Double = 120_000
    /// Insert before `\end{document}` when present so every character lays out.
    var insertBeforeEndDocument = true
    /// `FLASHTEX_TYPING_BENCH_AT=<needle>`: type right after the first occurrence
    /// of `needle` instead (an edit on a visible page: the end of a document is
    /// on its last page, which the v2 pane's lazy stack never materializes at
    /// the bench window size, so no paint of the edited page is observed there).
    /// `first-paragraph` is the end of the first paragraph after `\begin{document}`.
    var insertAfterNeedle: String?

    static func parse(_ env: [String: String]) -> TypingBenchConfig? {
        guard let path = env["FLASHTEX_TYPING_BENCH"], !path.isEmpty else { return nil }
        var c = TypingBenchConfig(scriptPath: path, outputPath: path + ".bench.json")
        if let s = env["FLASHTEX_TYPING_BENCH_MS"], let ms = Double(s), ms >= 0 { c.intervalMs = ms }
        if let out = env["FLASHTEX_TYPING_BENCH_OUT"], !out.isEmpty { c.outputPath = out }
        if let s = env["FLASHTEX_TYPING_BENCH_SETTLE_MS"], let ms = Double(s), ms > 0 { c.settleTimeoutMs = ms }
        if let s = env["FLASHTEX_TYPING_BENCH_MAX_MS"], let ms = Double(s), ms > 0 { c.typingBudgetMs = ms }
        if env["FLASHTEX_TYPING_BENCH_APPEND"] == "1" { c.insertBeforeEndDocument = false }
        if let at = env["FLASHTEX_TYPING_BENCH_AT"], !at.isEmpty { c.insertAfterNeedle = at }
        return c
    }

    /// One insertion per extended grapheme cluster, so `é` or an emoji is one
    /// keystroke like a real (composed) key event. CRLF is normalised to LF.
    static func keystrokes(from text: String) -> [String] {
        text.replacingOccurrences(of: "\r\n", with: "\n").map { String($0) }
    }

    /// UTF-16 offset to start typing at: after `needle` when given and found
    /// (`first-paragraph`: the first blank line after `\begin{document}`), else
    /// the `\end{document}` line, else the end.
    static func insertionOffset(in text: String, beforeEndDocument: Bool, afterNeedle needle: String? = nil) -> Int {
        let ns = text as NSString
        if let needle {
            if needle == "first-paragraph" {
                let begin = ns.range(of: "\\begin{document}")
                let from = begin.location == NSNotFound ? 0 : NSMaxRange(begin)
                let blank = ns.range(of: "\n\n", options: [], range: NSRange(location: from, length: ns.length - from))
                if blank.location != NSNotFound { return blank.location }
            } else {
                let r = ns.range(of: needle)
                if r.location != NSNotFound { return NSMaxRange(r) }
            }
        }
        guard beforeEndDocument else { return ns.length }
        let r = ns.range(of: "\\end{document}", options: .backwards)
        return r.location == NSNotFound ? ns.length : r.location
    }
}

/// Process-wide instrumentation. Main thread only.
@MainActor
final class TypingBench {
    static let shared = TypingBench()

    let recorder = TypingLatencyRecorder(log: { FlashTeXLog.write($0) })
    /// Definition of the recorded paint time; also written to the summary.
    static let paintPointDescription = "first main-queue turn after the run-loop iteration whose SwiftUI render pass evaluated PreviewView for the revision (Canvas draw closures of every page ran inside that pass); the CoreAnimation commit has completed, the display's next vsync scan-out is not observed"

    private var monitor: Any?
    private var renderingRevision = 0
    private var expectedPages = 0
    private var drawnPages = 0
    private var paintHops = 0
    private var renderStartNs: UInt64 = 0
    private var drawEndNs: UInt64?
    private var driver: TypingBenchDriver?
    var onPaint: ((Int) -> Void)?
    /// True while a scripted bench run is typing: gates the extra timeline log
    /// lines (send/decode/deliver/apply) that would be noise in normal use.
    /// Read from any thread (set once on the main thread before typing starts).
    nonisolated(unsafe) private(set) static var isBenchActive = false
    var isActive: Bool { Self.isBenchActive }
    func setActive(_ on: Bool) { Self.isBenchActive = on }

    /// Clears recorded events and render state (bench start, tests).
    func reset() { recorder.reset(); renderingRevision = 0; expectedPages = 0; drawnPages = 0; paintHops = 0; drawEndNs = nil }

    /// Installs the in-process key monitor and, when configured, the bench driver.
    func install(model: ShellModel) {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self else { return event }
            // HID timestamp of the key press, comparable with MonotonicClock (same clock).
            let stamp = MonotonicClock.ns(fromUptimeSeconds: event.timestamp)
            self.recorder.keystroke(at: stamp)
            // The event is dispatched synchronously after this monitor returns; a key
            // that changed no text (arrows, shortcuts) must not stay armed.
            DispatchQueue.main.async { self.recorder.clearPendingKeystroke() }
            return event
        }
        if let config = TypingBenchConfig.parse(ProcessInfo.processInfo.environment) {
            driver = TypingBenchDriver(config: config, model: model, bench: self)
            driver?.start()
        }
    }

    // MARK: hooks

    /// `SourceEditorView` delegate: the text view reported a change.
    func textViewDidChange() { recorder.delegateReported(at: MonotonicClock.nowNs()) }
    /// `ShellModel.updateActiveText`: the buffer is now `revision`.
    func noteRevision(_ revision: Int) { recorder.revision(revision, at: MonotonicClock.nowNs()) }
    /// `ShellModel.handle(.result)`: a compile result for `revision` was applied.
    func noteCompile(revision: Int, ms: Double) { recorder.compile(revision: revision, ms: ms, at: MonotonicClock.nowNs()) }

    /// `PreviewView.body`: SwiftUI is rendering `revision`. Idempotent per revision.
    func willRender(revision: Int, pages: Int) {
        guard revision > recorder.lastPaintedRevision, revision != renderingRevision else { return }
        renderingRevision = revision
        expectedPages = pages
        drawnPages = 0
        paintHops = 0
        renderStartNs = MonotonicClock.nowNs()
        drawEndNs = nil
        Self.nextRunLoopTurn { [self] in finishPaint(revision) }
    }

    /// Runs `block` at the head of the next main run-loop iteration — after
    /// this iteration's observers (SwiftUI render pass, CoreAnimation commit)
    /// have run. The main dispatch queue is not used: AppKit drains it late
    /// under typing, and worker results now arrive by the same run-loop path,
    /// so a queued paint hop would otherwise lose its revision to a newer one.
    static func nextRunLoopTurn(_ block: @escaping @MainActor () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue) {
            MainActor.assumeIsolated { block() }
        }
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }

    /// Canvas draw closure of one page ran (nonisolated caller: SwiftUI's draw closure).
    nonisolated func didDraw(page: Int) {
        MainActor.assumeIsolated { drawnPages += 1; drawEndNs = MonotonicClock.nowNs() }
    }

    private func finishPaint(_ revision: Int) {
        guard revision == renderingRevision, revision > recorder.lastPaintedRevision else { return }
        // The pages whose layout changed draw inside the same pass as `body`
        // (unchanged pages keep their display list and never draw — PageView is
        // Equatable). Allow two more turns only when nothing drew at all, in case
        // SwiftUI deferred the pass, then record honestly.
        if drawnPages == 0, expectedPages > 0, paintHops < 2 {
            paintHops += 1
            Self.nextRunLoopTurn { [self] in finishPaint(revision) }
            return
        }
        let redrawn = drawnPages > 0
        if paintHops > 0 { FlashTeXLog.write("paint: revision \(revision) needed \(paintHops) extra turn(s); drew \(drawnPages)/\(expectedPages) pages") }
        else if drawnPages < expectedPages { FlashTeXLog.write("paint: revision \(revision) redrew \(drawnPages)/\(expectedPages) pages") }
        recorder.paint(revision: revision, at: MonotonicClock.nowNs(), redrawn: redrawn,
                       renderStartNs: renderStartNs, drawEndNs: drawEndNs)
        onPaint?(revision)
    }
}

/// Types a script into the real editor `NSTextView` one keystroke at a time,
/// through `insertText(_:replacementRange:)` on the main run loop, so the
/// delegate -> `updateActiveText` -> compile -> result -> paint path is the one
/// real typing takes (minus the OS event queue). Writes a JSON summary and exits.
@MainActor
final class TypingBenchDriver {
    let config: TypingBenchConfig
    unowned let model: ShellModel
    unowned let bench: TypingBench
    private var keys: [String] = []
    private var index = 0
    private var timer: Timer?
    private var startNs: UInt64 = 0
    private var bytesBefore = 0
    private var startedAt = Date()
    private var settleDeadline: Date?
    private var budgetExhausted = false
    private var finished = false
    private(set) var textView: NSTextView?
    /// Test hook: called instead of `exit` when set.
    var onFinish: ((TypingBenchSummary) -> Void)?

    init(config: TypingBenchConfig, model: ShellModel, bench: TypingBench, textView: NSTextView? = nil) {
        self.config = config; self.model = model; self.bench = bench; self.textView = textView
    }

    func start() {
        guard let text = try? String(contentsOfFile: config.scriptPath, encoding: .utf8) else {
            FlashTeXLog.write("bench: cannot read \(config.scriptPath)")
            finish(reason: "script unreadable")
            return
        }
        keys = TypingBenchConfig.keystrokes(from: text)
        FlashTeXLog.write("bench: waiting for attach + first paint (\(keys.count) keystrokes, \(config.intervalMs) ms)")
        // Poll until the worker attached, its first result was applied and painted.
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.pollReady() }
        }
    }

    /// Finds the editor text view in the main window (labelled "LaTeX source").
    static func findTextView(in views: [NSView]) -> NSTextView? {
        for v in views {
            if let tv = v as? NSTextView, tv.accessibilityLabel() == "LaTeX source" { return tv }
            if let found = findTextView(in: v.subviews) { return found }
        }
        return nil
    }

    private func pollReady() {
        guard !finished else { return }
        if Date().timeIntervalSince(startedAt) > 60 {
            finish(reason: "worker never attached/painted (60 s)")
            return
        }
        guard model.workerAttached, model.inFlightRevision == nil, let result = model.result,
              !model.isFixture, result.revision == model.editorRevision,
              bench.recorder.lastPaintedRevision >= result.revision else { return }
        guard let tv = textView ?? TypingBenchDriver.findTextView(in: NSApp.windows.compactMap(\.contentView)) else {
            FlashTeXLog.write("bench: editor text view not found yet")
            return
        }
        textView = tv
        timer?.invalidate()
        begin(in: tv)
    }

    func begin(in tv: NSTextView) {
        let offset = TypingBenchConfig.insertionOffset(in: tv.string, beforeEndDocument: config.insertBeforeEndDocument, afterNeedle: config.insertAfterNeedle)
        tv.setSelectedRange(NSRange(location: offset, length: 0))
        bytesBefore = tv.string.utf8.count
        bench.reset()
        bench.setActive(true)
        startNs = MonotonicClock.nowNs()
        startedAt = Date()
        FlashTeXLog.write("bench: typing \(keys.count) keystrokes at UTF-16 offset \(offset) every \(config.intervalMs) ms")
        // A Timer fires at most once per run-loop iteration, so even at 0 ms each
        // keystroke gets its own turn and CoreAnimation can commit between them.
        timer = Timer.scheduledTimer(withTimeInterval: config.intervalMs / 1000, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.tick() }
        }
    }

    private func tick() {
        guard !finished, let tv = textView else { return }
        if index < keys.count, !budgetExhausted, Double(MonotonicClock.nowNs() &- startNs) / 1e6 > config.typingBudgetMs {
            budgetExhausted = true
            FlashTeXLog.write("bench: typing budget of \(config.typingBudgetMs) ms exhausted after \(index)/\(keys.count) keystrokes; settling")
        }
        if index < keys.count, !budgetExhausted {
            let key = keys[index]
            index += 1
            bench.recorder.keystroke(at: MonotonicClock.nowNs())
            tv.insertText(key, replacementRange: tv.selectedRange())
            bench.recorder.clearPendingKeystroke() // insertText is synchronous; no text change = no keystroke
            return
        }
        if settleDeadline == nil {
            settleDeadline = Date().addingTimeInterval(config.settleTimeoutMs / 1000)
            timer?.invalidate()
            timer = Timer.scheduledTimer(withTimeInterval: 0.02, repeats: true) { [weak self] _ in
                MainActor.assumeIsolated { self?.tick() }
            }
        }
        if bench.recorder.unpainted.isEmpty || Date() > settleDeadline! {
            finish(reason: bench.recorder.unpainted.isEmpty ? "all keystrokes painted" : "settle timeout")
        }
    }

    func summary() -> TypingBenchSummary {
        let r = bench.recorder
        return TypingBenchSummary(
            producer: { if case .worker(let name) = model.previewSource { return name } else { return "none" } }(),
            script: config.scriptPath,
            intervalMs: config.intervalMs,
            scriptKeystrokes: keys.count,
            typed: index,
            typingBudgetExhausted: budgetExhausted,
            keystrokes: r.keystrokes.count,
            painted: r.painted.count,
            unpainted: r.unpainted.count,
            coalesced: r.coalescedCount,
            paints: r.paints.count,
            paintsWithoutRedraw: r.paints.filter { !$0.redrawn }.count,
            compiles: r.compilesMs.count,
            documentBytesBefore: bytesBefore,
            documentBytesAfter: textView?.string.utf8.count ?? 0,
            elapsedMs: Double(MonotonicClock.nowNs() &- startNs) / 1e6,
            keystrokeToPaintMs: LatencyStats(r.latenciesMs),
            compileMs: LatencyStats(r.compilesMs.values.map { $0 }),
            renderPassMs: LatencyStats(r.paints.compactMap(\.renderPassMs)),
            resultToPaintMs: LatencyStats(r.paints.compactMap(\.resultToPaintMs)),
            debounceMs: ShellModel.debounceInterval * 1000,
            paintPoint: TypingBench.paintPointDescription,
            perKeystroke: r.keystrokes,
            startedAt: ISO8601DateFormatter().string(from: startedAt))
    }

    private func finish(reason: String) {
        guard !finished else { return }
        finished = true
        bench.setActive(false)
        timer?.invalidate()
        let s = summary()
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        do {
            try enc.encode(s).write(to: URL(fileURLWithPath: config.outputPath), options: .atomic)
        } catch {
            FlashTeXLog.write("bench: could not write \(config.outputPath): \(error)")
        }
        FlashTeXLog.write("bench: done (\(reason)); keystrokes \(s.keystrokes), painted \(s.painted), coalesced \(s.coalesced), p50 \(s.keystrokeToPaintMs.p50Ms ?? -1) ms")
        if let onFinish { onFinish(s); return }
        // The buffer is dirty by construction; bypass the save prompt. Give the
        // log queue a moment to drain before exiting.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { exit(0) }
    }
}
