import AppKit
import XCTest
@testable import FlashTeXMac

/// Latency aggregation, bench configuration parsing and the summary
/// (TypingBench.swift). The end-to-end runs through the old engine's worker
/// and its preview retired with that engine (#1236 S6); the engine's own
/// keystroke-to-page bench is EngineV3Bench.
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
        guard r.keystrokes.count > 1 else { return XCTFail("expected more than one keystroke, got \(r.keystrokes.count)") }
        XCTAssertEqual(r.keystrokes[0].delegateNs, 1_100)
        XCTAssertNil(r.keystrokes[1].delegateNs)
        XCTAssertEqual(r.paints.count, 1)
        guard let firstPaint = r.paints.first else { return XCTFail("expected one paint") }
        XCTAssertEqual(firstPaint.resultToPaintMs, 4.996)
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

    // MARK: the summary

    /// The summary names the engine that typeset the run: the new one, the
    /// only engine since S6; no worker produced anything.
    func testTheSummaryNamesTheNewEngine() throws {
        let model = ShellModel()
        let config = TypingBenchConfig(scriptPath: "/nonexistent/script.txt", outputPath: "/nonexistent/script.txt.bench.json")
        let driver = TypingBenchDriver(config: config, model: model, bench: TypingBench.shared)
        let s = driver.summary()
        XCTAssertEqual(s.engine, "new")
        XCTAssertEqual(s.producer, "none")
        XCTAssertEqual(s.paintPoint, TypingBench.paintPointDescription)
        XCTAssertEqual(s.typed, 0)
        // The JSON summary round-trips, the engine with it.
        let decoded = try JSONDecoder().decode(TypingBenchSummary.self, from: try JSONEncoder().encode(s))
        XCTAssertEqual(decoded.engine, "new")
        XCTAssertEqual(decoded.paintPoint, s.paintPoint)
    }
}
