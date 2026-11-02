import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// `SPAN_SHIFT` (`span-shift-v1`, protocol §5.3.1; review lane REVIEW-APP
/// round 6): a line inserted near the top of a long source moved every
/// later span, and the host re-declared them all in a `SOURCES` the app
/// applied span by span on main. Now one shift says it, taken lazily.
@MainActor
final class EngineV3SpanShiftTests: XCTestCase {
    private var env = EnvironmentOverride()

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-span-shift-\(UUID().uuidString)").path)
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
    }

    override func tearDown() { env.restore() }

    private static func sources(_ spans: [(UInt32, UInt32, UInt32)], files: [(UInt32, String)] = []) throws -> DL3Sources {
        try DL3Sources.from(.object([
            "files": .array(files.map { .array([.int(Int64($0.0)), .string($0.1)]) }),
            "spans": .array(spans.map { .array([.int(Int64($0.0)), .int(Int64($0.1)), .int(Int64($0.2))]) }),
        ]))
    }

    private func assertSame(_ a: DL3SourceMap, _ b: DL3SourceMap, truth: [UInt32: (UInt32, UInt32)], lines: Int,
                            _ why: String, file: StaticString = #filePath, line: UInt = #line) {
        let sa = a.spans, sb = b.spans
        XCTAssertEqual(sa.count, truth.count, why, file: file, line: line)
        XCTAssertEqual(sb.count, truth.count, why, file: file, line: line)
        for (s, t) in truth {
            XCTAssertTrue(sa[s].map { $0 == t } ?? false, "full re-send, span \(s): \(why)", file: file, line: line)
            XCTAssertTrue(sb[s].map { $0 == t } ?? false, "shifts, span \(s): \(why)", file: file, line: line)
            XCTAssertEqual(a.location(of: s)?.line, b.location(of: s)?.line, "location of \(s): \(why)", file: file, line: line)
        }
        for f in ["./main.tex", "./ch1.tex"] {
            for l in 0 ... lines {
                XCTAssertEqual(a.spans(line: l) { $0 == f }, b.spans(line: l) { $0 == f }, "\(f):\(l): \(why)", file: file, line: line)
            }
        }
    }

    /// Random edits (insertions, deletions, several per compile, two files),
    /// new spans between them, compactions now and then: the map that took
    /// `SPAN_SHIFT`s answers every lookup as the one that took the full
    /// re-send (`SOURCES` of every moved span), and both hold the host's spans.
    func testShiftsGiveTheSpansAFullResendGives() throws {
        var seed: UInt64 = 0x2545_f491_4f6c_dd1d
        func rnd(_ n: UInt32) -> UInt32 {
            seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17
            return UInt32(seed % UInt64(n))
        }
        var truth: [UInt32: (UInt32, UInt32)] = [:]
        var next: UInt32 = 1
        var full = DL3SourceMap(), shifted = DL3SourceMap()
        let files: [(UInt32, String)] = [(1, "./main.tex"), (2, "./ch1.tex")]
        let first = try Self.sources([], files: files)
        full.apply(first); shifted.apply(first)
        for round in 0 ..< 200 {
            // a compile's new spans (some on lines that already have one)
            var fresh: [(UInt32, UInt32, UInt32)] = []
            for _ in 0 ..< 6 {
                let at = (1 + rnd(2), 1 + rnd(80))
                truth[next] = at; fresh.append((next, at.0, at.1)); next += 1
            }
            let src = try Self.sources(fresh)
            full.apply(src); shifted.apply(src)
            // edits: the host moves the spans of later lines
            var shifts: [DL3SpanShifts.Shift] = []
            var moved: [UInt32: (UInt32, UInt32)] = [:]
            for _ in 0 ..< 1 + Int(rnd(3)) {
                let f = 1 + rnd(2), from = 1 + rnd(80), oldEnd = from + rnd(4), newEnd = from + rnd(6)
                let d = Int(newEnd) - Int(oldEnd)
                guard d != 0 else { continue }
                let sh = DL3SpanShifts.Shift(file: f, from: oldEnd, delta: d)
                shifts.append(sh)
                for (s, at) in truth where at.0 == f && at.1 >= oldEnd {
                    truth[s] = (f, sh.apply(file: f, line: at.1)); moved[s] = truth[s]
                }
            }
            full.apply(try Self.sources(moved.map { ($0.key, $0.value.0, $0.value.1) }))
            shifted.shift(DL3SpanShifts(shifts))
            if round % 37 == 36 { shifted = shifted.compacted() }
            if round % 20 == 19 { assertSame(full, shifted, truth: truth, lines: 120, "round \(round)") }
        }
        assertSame(full, shifted, truth: truth, lines: 120, "the end")
        assertSame(full, shifted.compacted(), truth: truth, lines: 120, "compacted")
    }

    /// Through the session as the host sends it, 60,000 spans of a long
    /// book, then five lines inserted near its top: the full re-send (every
    /// later span in `SOURCES`) against `SPAN_SHIFT`, on main; lookups agree,
    /// and the compaction off main keeps them.
    func testALineInsertedNearTheTopOfALongBookIsOneShiftOnMain() async throws {
        func session() -> (ShellModel, EngineV3Session) {
            let model = ShellModel()
            model.autoCompile = false
            model.replaceProject(entryText: "\\documentclass{book}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
            model.engineV3Enabled = true
            return (model, model.engineV3)
        }
        let n: UInt32 = 60_000
        let (ma, a) = session(), (mb, b) = session()
        defer { ma.engineV3Enabled = false; mb.engineV3Enabled = false }
        let all = try Self.sources((1 ... n).map { ($0, 1, $0 + 2) }, files: [(1, "./main.tex")])
        a.handle(.sources(all)); b.handle(.sources(all))
        var fullMs: [Double] = [], shiftMs: [Double] = []
        for k in 1 ... 5 {
            let moved = try Self.sources((1 ... n).map { ($0, 1, $0 + 2 + UInt32(k)) })
            var t0 = Date()
            a.handle(.sources(moved))
            fullMs.append(Date().timeIntervalSince(t0) * 1000)
            let shift = DL3SpanShifts([.init(file: 1, from: 3 + UInt32(k) - 1, delta: 1)])
            t0 = Date()
            b.handle(.spanShift(shift))
            shiftMs.append(Date().timeIntervalSince(t0) * 1000)
        }
        let full = fullMs.sorted()[2], shift = shiftMs.sorted()[2]
        print(String(format: "EngineV3SpanShiftTests: %d spans moved by a line: full re-send %.2f ms on main, SPAN_SHIFT %.3f ms (medians of 5)", n, full, shift))
        XCTAssertLessThan(shift, 1, "a shift is not a walk of the spans")
        for s: UInt32 in [1, 2, 500, 30_000, n] {
            XCTAssertEqual(b.sourceMap.location(of: s)?.line, a.sourceMap.location(of: s)?.line, "span \(s)")
            XCTAssertEqual(b.sourceMap.location(of: s)?.line, Int(s) + 7)
        }
        for l in [1, 2, 3, 7, 8, 507, 30_007, Int(n) + 7, Int(n) + 8] {
            XCTAssertEqual(b.sourceMap.spans(line: l) { _ in true }, a.sourceMap.spans(line: l) { _ in true }, "line \(l)")
        }
        // Sixteen shifts in all: the map is compacted off main, lookups unchanged.
        for k in 6 ... 16 { b.handle(.spanShift(DL3SpanShifts([.init(file: 1, from: 3 + UInt32(k) - 1, delta: 1)]))) }
        let end = Date().addingTimeInterval(60)
        while !b.sourceMap.shifts.isEmpty, Date() < end { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertGreaterThanOrEqual(b.sourceMapCompactions, 1)
        XCTAssertTrue(b.sourceMap.shifts.isEmpty, "\(b.sourceMap.shifts.count) shifts still pending")
        XCTAssertEqual(b.sourceMap.location(of: 30_000)?.line, 30_018)
        XCTAssertEqual(b.sourceMap.spans(line: 30_018) { _ in true }, [30_000])
        XCTAssertEqual(b.sourceMap.spans(line: 3) { _ in true }, [])
    }
}
