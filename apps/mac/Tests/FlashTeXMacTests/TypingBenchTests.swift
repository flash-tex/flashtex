import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// Latency aggregation, bench configuration parsing, and the insertion path
/// through the real `NSTextView` -> delegate -> `ShellModel` -> fake worker ->
/// `PreviewView` render, with the paint hooks recording every keystroke.
@MainActor
final class TypingBenchTests: XCTestCase {

    // MARK: aggregation

    func testPercentileIsNearestRank() {
        XCTAssertNil(percentile([], 50))
        XCTAssertEqual(percentile([7], 50), 7)
        XCTAssertEqual(percentile([7], 99), 7)
        let s: [Double] = [10, 1, 5, 3, 8, 2, 9, 4, 7, 6] // 1...10
        XCTAssertEqual(percentile(s, 50), 5)
        XCTAssertEqual(percentile(s, 95), 10)
        XCTAssertEqual(percentile(s, 99), 10)
        XCTAssertEqual(percentile(s, 0), 1)
        XCTAssertEqual(percentile(s, 100), 10)
        let stats = LatencyStats(s)
        XCTAssertEqual(stats.count, 10)
        XCTAssertEqual(stats.p50Ms, 5); XCTAssertEqual(stats.maxMs, 10); XCTAssertEqual(stats.minMs, 1)
        XCTAssertEqual(stats.meanMs!, 5.5, accuracy: 1e-9)
        XCTAssertNil(LatencyStats([]).p50Ms)
    }

    func testRecorderMeasuresEachKeystrokeToTheFirstPaintCoveringIt() {
        var lines: [String] = []
        let r = TypingLatencyRecorder { lines.append($0) }
        // Three keystrokes -> revisions 2, 3, 4; one paint of revision 4 covers all.
        r.keystroke(at: 1_000); r.delegateReported(at: 1_100); r.revision(2, at: 1_200)
        r.keystroke(at: 2_000); r.revision(3, at: 2_050)
        r.keystroke(at: 3_000); r.revision(4, at: 3_050)
        r.compile(revision: 4, ms: 1.5, at: 4_000)
        XCTAssertEqual(r.unpainted.count, 3)
        XCTAssertEqual(r.paint(revision: 4, at: 5_000_000), 3)
        XCTAssertEqual(r.latenciesMs, [4.999, 4.998, 4.997])
        XCTAssertEqual(r.coalescedCount, 2, "revisions 2 and 3 were made visible by revision 4's paint")
        XCTAssertEqual(r.keystrokes.map(\.paintedByRevision), [4, 4, 4])
        XCTAssertEqual(r.keystrokes.map(\.compileMs), [1.5, 1.5, 1.5])
        XCTAssertEqual(r.keystrokes[0].delegateNs, 1_100)
        XCTAssertNil(r.keystrokes[1].delegateNs)
        XCTAssertEqual(r.paints.count, 1)
        XCTAssertEqual(r.paints[0].resultToPaintMs, 4.996)
        XCTAssertTrue(lines[0].hasPrefix("keystroke: revision 2 at 1000"), lines[0])
        XCTAssertTrue(lines.last!.hasPrefix("paint: revision 4 at 5000000 (covers 3 keystrokes"), lines.last!)

        // A repeated or older paint changes nothing; a later one covers only new keystrokes.
        XCTAssertEqual(r.paint(revision: 4, at: 6_000_000), 0)
        XCTAssertEqual(r.paint(revision: 3, at: 6_000_000), 0)
        XCTAssertEqual(r.paints.count, 1)
        r.keystroke(at: 7_000_000); r.revision(5, at: 7_000_100)
        XCTAssertEqual(r.paint(revision: 6, at: 8_000_000, redrawn: false), 1)
        XCTAssertEqual(r.keystrokes[3].latencyMs!, 1.0, accuracy: 1e-9)
        XCTAssertEqual(r.keystrokes[3].redrawn, false)
        XCTAssertEqual(r.coalescedCount, 3)
        XCTAssertTrue(r.unpainted.isEmpty)
    }

    func testRecorderIgnoresRevisionsWithoutAKeystrokeAndClearedKeys() {
        let r = TypingLatencyRecorder()
        r.revision(2, at: 10) // fixture load / programmatic edit
        XCTAssertEqual(r.keystrokes.count, 0)
        XCTAssertEqual(r.nonKeystrokeRevisions, 1)
        // An arrow key arms a keystroke but changes no text: cleared before any edit.
        r.keystroke(at: 20); r.clearPendingKeystroke()
        r.revision(3, at: 30)
        XCTAssertEqual(r.keystrokes.count, 0)
        // A paint with nothing pending is recorded but covers nothing.
        XCTAssertEqual(r.paint(revision: 3, at: 40), 0)
        XCTAssertEqual(r.paints.count, 1)
        // Only the most recent arm counts.
        r.keystroke(at: 50); r.keystroke(at: 60); r.revision(4, at: 70)
        XCTAssertEqual(r.keystrokes.map(\.keystrokeNs), [60])
        r.reset()
        XCTAssertEqual(r.lastPaintedRevision, 0)
        XCTAssertTrue(r.keystrokes.isEmpty && r.paints.isEmpty)
    }

    func testRecorderCapacityDropsOldestPaintedOnly() {
        let r = TypingLatencyRecorder()
        r.capacity = 3
        for n in 2...4 { r.keystroke(at: UInt64(n)); r.revision(n, at: UInt64(n)) }
        r.paint(revision: 3, at: 100) // 2 and 3 painted, 4 not
        r.keystroke(at: 5); r.revision(5, at: 5)
        XCTAssertEqual(r.keystrokes.map(\.revision), [3, 4, 5], "the oldest painted keystroke is dropped, never an unpainted one")
    }

    // MARK: configuration / script parsing

    func testConfigParsesEnvironmentWithDefaults() {
        XCTAssertNil(TypingBenchConfig.parse([:]))
        XCTAssertNil(TypingBenchConfig.parse(["FLASHTEX_TYPING_BENCH": ""]))
        let c = TypingBenchConfig.parse(["FLASHTEX_TYPING_BENCH": "/tmp/t.txt"])!
        XCTAssertEqual(c.scriptPath, "/tmp/t.txt")
        XCTAssertEqual(c.intervalMs, 30)
        XCTAssertEqual(c.outputPath, "/tmp/t.txt.bench.json")
        XCTAssertEqual(c.settleTimeoutMs, 10_000)
        XCTAssertTrue(c.insertBeforeEndDocument)
        let d = TypingBenchConfig.parse(["FLASHTEX_TYPING_BENCH": "s.txt", "FLASHTEX_TYPING_BENCH_MS": "0",
                                         "FLASHTEX_TYPING_BENCH_OUT": "o.json", "FLASHTEX_TYPING_BENCH_SETTLE_MS": "500",
                                         "FLASHTEX_TYPING_BENCH_APPEND": "1"])!
        XCTAssertEqual(d.intervalMs, 0); XCTAssertEqual(d.outputPath, "o.json"); XCTAssertEqual(d.settleTimeoutMs, 500)
        XCTAssertFalse(d.insertBeforeEndDocument)
        // Garbage values keep the defaults.
        let e = TypingBenchConfig.parse(["FLASHTEX_TYPING_BENCH": "s", "FLASHTEX_TYPING_BENCH_MS": "-5", "FLASHTEX_TYPING_BENCH_SETTLE_MS": "x"])!
        XCTAssertEqual(e.intervalMs, 30); XCTAssertEqual(e.settleTimeoutMs, 10_000)
    }

    func testScriptSplitsIntoGraphemeKeystrokesAndFindsInsertionPoint() {
        XCTAssertEqual(TypingBenchConfig.keystrokes(from: "ab\r\nc"), ["a", "b", "\n", "c"])
        XCTAssertEqual(TypingBenchConfig.keystrokes(from: "naïve 👩‍💻"), ["n", "a", "ï", "v", "e", " ", "👩‍💻"])
        XCTAssertEqual(TypingBenchConfig.keystrokes(from: ""), [])
        let doc = "\\begin{document}\nRésumé\n\\end{document}\n"
        XCTAssertEqual(TypingBenchConfig.insertionOffset(in: doc, beforeEndDocument: true), (doc as NSString).range(of: "\\end{document}").location)
        XCTAssertEqual(TypingBenchConfig.insertionOffset(in: doc, beforeEndDocument: false), (doc as NSString).length)
        XCTAssertEqual(TypingBenchConfig.insertionOffset(in: "no end", beforeEndDocument: true), 6)
        // FLASHTEX_TYPING_BENCH_AT: after a literal needle, or the end of the first paragraph after \begin{document}.
        let body = "\\documentclass{article}\n\\begin{document}\nFirst para.\n\nSecond para.\n\\end{document}\n"
        XCTAssertEqual(TypingBenchConfig.insertionOffset(in: body, beforeEndDocument: true, afterNeedle: "First para."), (body as NSString).range(of: "\n\nSecond").location)
        XCTAssertEqual(TypingBenchConfig.insertionOffset(in: body, beforeEndDocument: true, afterNeedle: "first-paragraph"), (body as NSString).range(of: "\n\nSecond").location)
        XCTAssertEqual(TypingBenchConfig.insertionOffset(in: body, beforeEndDocument: true, afterNeedle: "absent"), (body as NSString).range(of: "\\end{document}").location)
        XCTAssertEqual(TypingBenchConfig.parse(["FLASHTEX_TYPING_BENCH": "s.txt", "FLASHTEX_TYPING_BENCH_AT": "first-paragraph"])?.insertAfterNeedle, "first-paragraph")
    }

    // MARK: insertion path against fake_worker.py

    /// Hosts the real editor and preview so keystrokes take the production path.
    private struct BenchHost: View {
        var model: ShellModel // @Observable: reads inside body are tracked
        var body: some View {
            HStack {
                SourceEditorView(text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                                 selection: model.selection, pendingEdit: model.pendingEdit, result: model.result)
                    .frame(width: 400)
                if let result = model.result {
                    PreviewView(result: result, dark: false) { _, _ in }
                }
            }
        }
    }

    private func makeWindow(_ model: ShellModel) -> NSWindow {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 900, height: 600), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: BenchHost(model: model))
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        return window
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 15, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    private func runBench(script: String, intervalMs: Double, seed: String) async throws -> (TypingBenchSummary, [String], NSWindow) {
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        XCTAssertTrue(model.workerAttached)
        let window = makeWindow(model)
        model.replaceProject(entryText: seed)
        model.compile()
        try await waitUntil("first compile") { model.inFlightRevision == nil && !model.isFixture }
        let tv = try XCTUnwrap(TypingBenchDriver.findTextView(in: [window.contentView!]))
        try await waitUntil("first paint") { TypingBench.shared.recorder.lastPaintedRevision >= model.result!.revision }

        var lines: [String] = []
        TypingBench.shared.recorder.log = { lines.append($0) }
        defer { TypingBench.shared.recorder.log = { FlashTeXLog.write($0) } }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("typing-bench-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let scriptURL = dir.appendingPathComponent("script.txt")
        try script.write(to: scriptURL, atomically: true, encoding: .utf8)
        var config = TypingBenchConfig(scriptPath: scriptURL.path, outputPath: dir.appendingPathComponent("out.json").path)
        config.intervalMs = intervalMs
        config.settleTimeoutMs = 5_000
        config.insertBeforeEndDocument = false

        let driver = TypingBenchDriver(config: config, model: model, bench: TypingBench.shared, textView: tv)
        var finished: TypingBenchSummary?
        driver.onFinish = { finished = $0 }
        driver.start()
        try await waitUntil("bench finish", timeout: 30) { finished != nil }
        let summary = try XCTUnwrap(finished)
        // The JSON summary round-trips.
        let data = try Data(contentsOf: URL(fileURLWithPath: config.outputPath))
        let decoded = try JSONDecoder().decode(TypingBenchSummary.self, from: data)
        XCTAssertEqual(decoded.keystrokes, summary.keystrokes)
        XCTAssertEqual(decoded.paintPoint, TypingBench.paintPointDescription)
        XCTAssertEqual(tv.string, model.activeText, "editor and model agree")
        return (summary, lines, window)
    }

    func testEveryTypedKeystrokeIsPaintedThroughTheRealEditorAndPreview() async throws {
        let script = "Hello naïve café — typed.\n"
        let (s, lines, window) = try await runBench(script: script, intervalMs: 5, seed: "Seed line\n")
        defer { window.orderOut(nil) }
        XCTAssertEqual(s.typed, TypingBenchConfig.keystrokes(from: script).count)
        XCTAssertEqual(s.keystrokes, s.typed, "every insertText produced exactly one keystroke revision")
        XCTAssertEqual(s.unpainted, 0)
        XCTAssertEqual(s.painted, s.keystrokes)
        XCTAssertGreaterThan(s.paints, 0)
        XCTAssertEqual(s.documentBytesAfter - s.documentBytesBefore, script.utf8.count)
        XCTAssertEqual(s.keystrokeToPaintMs.count, s.keystrokes)
        XCTAssertGreaterThan(s.keystrokeToPaintMs.p50Ms!, 0)
        XCTAssertGreaterThanOrEqual(s.keystrokeToPaintMs.p99Ms!, s.keystrokeToPaintMs.p50Ms!)
        XCTAssertGreaterThanOrEqual(s.keystrokeToPaintMs.maxMs!, s.keystrokeToPaintMs.p99Ms!)
        // Every keystroke revision has its own log line, and a paint line whose
        // revision is >= it appears after it.
        for k in s.perKeystroke {
            let ki = try XCTUnwrap(lines.firstIndex { $0.hasPrefix("keystroke: revision \(k.revision) at \(k.keystrokeNs)") }, "no keystroke line for revision \(k.revision)")
            XCTAssertNotNil(k.delegateNs, "the text view delegate stamped revision \(k.revision)")
            let painted = lines[ki...].contains { $0.hasPrefix("paint: revision \(k.paintedByRevision!) at \(k.paintNs!)") }
            XCTAssertTrue(painted, "no paint line covering revision \(k.revision)")
            XCTAssertGreaterThanOrEqual(k.paintedByRevision!, k.revision)
            XCTAssertNotNil(k.compileMs, "the covering paint knows its compile round trip")
        }
        // The final text is what the preview painted: the last keystroke's paint is
        // the result for the editor's current revision.
        let last = try XCTUnwrap(s.perKeystroke.last)
        XCTAssertEqual(last.paintedByRevision, last.revision)
        XCTAssertEqual(lines.filter { $0.hasPrefix("paint:") }.count, s.paints)
    }

    func testBurstTypingCoalescesButNeverLeavesTheFinalTextUnpainted() async throws {
        // `%slow` makes the fake worker answer after 400 ms, so a 0 ms burst of 12
        // keystrokes must coalesce into few compiles; the final revision still paints.
        let script = "abcdefghijkl"
        let (s, lines, window) = try await runBench(script: script, intervalMs: 0, seed: "%slow\nSeed line\n")
        defer { window.orderOut(nil) }
        XCTAssertEqual(s.keystrokes, 12)
        XCTAssertEqual(s.unpainted, 0, "coalescing must not strand a keystroke")
        XCTAssertGreaterThan(s.coalesced, 0, "burst keystrokes were made visible by a later revision's paint")
        XCTAssertLessThan(s.paints, s.keystrokes)
        XCTAssertLessThan(s.compiles, s.keystrokes, "one request in flight; the newest buffer goes out when it returns")
        let last = try XCTUnwrap(s.perKeystroke.last)
        XCTAssertEqual(last.paintedByRevision, last.revision, "the final text was painted at its own revision")
        XCTAssertTrue(lines.contains { $0.hasPrefix("paint: revision \(last.revision) at \(last.paintNs!)") })
        // Coalesced keystrokes wait for a later paint, so their latency is at least the
        // 400 ms worker delay they queued behind; none exceed two worker round trips + settle.
        for k in s.perKeystroke where k.coalesced { XCTAssertGreaterThan(k.latencyMs!, 100) }
        XCTAssertLessThan(s.keystrokeToPaintMs.maxMs!, 2_500)
    }
}
