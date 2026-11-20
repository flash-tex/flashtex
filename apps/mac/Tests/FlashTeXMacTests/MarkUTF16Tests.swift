import Foundation
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// APP-KEYPATH: the per-keystroke rebase of diagnostic marks
/// (`EditorDiagnostics.report`, from `ShellModel.editorMarks`) never walks
/// the keystroke's new text for UTF-16 offsets (MarkUTF16.swift).
///
/// Work counted, not timed, where the work is the app's: the bytes of a new
/// text walked from its start for a UTF-16 offset (`freshWalkBytes`). The
/// second test also compares CPU with one such walk, measured in the same
/// process, so it fails on a build where the report still walks.
final class MarkUTF16Tests: XCTestCase {
    override func setUp() { MarkUTF16Anchors.shared.reset() }
    override func tearDown() { MarkUTF16Anchors.shared.reset() }

    // MARK: fixtures

    struct LCG: RandomNumberGenerator {
        var s: UInt64
        mutating func next() -> UInt64 { s = s &* 6364136223846793005 &+ 1442695040888963407; return s }
    }

    /// Accents, combining marks, emoji (4-byte, skin tones, ZWJ), flags.
    static let pieces = ["a", "b", " ", "\n", "é", "e\u{301}", "\u{301}", "😀", "👍🏽", "🇫🇷", "🇩", "中文", "\\section{x}", "—", "ö",
                         "\u{200D}", "👨‍👩‍👧"]

    static func text(bytes: Int, _ g: inout LCG) -> String {
        var s = ""
        while s.utf8.count < bytes { s += pieces[Int(g.next() % UInt64(pieces.count))] }
        s.makeContiguousUTF8()
        return s
    }

    static func scalarByte(_ u: [UInt8], _ g: inout LCG) -> Int {
        var b = Int(g.next() % UInt64(u.count + 1))
        while b < u.count, u[b] & 0xC0 == 0x80 { b += 1 }
        return b
    }

    /// `old` with bytes `at..<at+delete` replaced, as a fresh native string
    /// (as `EditorTextShadow` makes one per keystroke).
    static func splice(_ old: String, at: Int, delete: Int = 0, insert: String) -> String {
        var o = old, ins = insert
        let total = o.utf8.count - delete + ins.utf8.count
        return o.withUTF8 { ob in
            ins.withUTF8 { ib in
                String(unsafeUninitializedCapacity: total) { buf in
                    let dst = buf.baseAddress!
                    if at > 0 { memcpy(dst, ob.baseAddress!, at) }
                    if ib.count > 0 { memcpy(dst + at, ib.baseAddress!, ib.count) }
                    let tail = ob.count - at - delete
                    if tail > 0 { memcpy(dst + at + ib.count, ob.baseAddress! + at + delete, tail) }
                    return total
                }
            }
        }
    }

    // MARK: exactness

    /// Through an anchor (the compiled text, or another warm text), every
    /// range is exactly `NSRange(r, in: current)`: random edits of accents,
    /// combining marks, ZWJ emoji and flags, spans widened to clusters.
    func testAnchoredRangesEqualTheDirectConversion() {
        var g = LCG(s: 42)
        let anchors = MarkUTF16Anchors()
        var compiled = Self.text(bytes: 20_000, &g)
        anchors.prepare(compiled)
        anchors.settle()
        var current = compiled
        var checks = 0
        for step in 0 ..< 600 {
            var u = Array(current.utf8)
            let a = Self.scalarByte(u, &g)
            var b = a
            if g.next() % 3 == 0 { b = min(u.count, a + Int(g.next() % 12)); while b < u.count, u[b] & 0xC0 == 0x80 { b += 1 } }
            var insert = ""
            for _ in 0 ..< Int(g.next() % 3) { insert += Self.pieces[Int(g.next() % UInt64(Self.pieces.count))] }
            current = Self.splice(current, at: a, delete: b - a, insert: insert)
            if step % 97 == 0 { compiled = current; anchors.prepare(compiled); anchors.settle() }
            let region = compiled.sameBytes(as: current) ? nil : SourceMapping.changedRegion(from: compiled, to: current)
            let map = MarkUTF16(current: current, compiled: compiled, region: region, anchors: anchors)
            u = Array(current.utf8)
            for _ in 0 ..< 20 {
                let x = Self.scalarByte(u, &g), y = Self.scalarByte(u, &g)
                guard let r = current.clusterAlignedRange(utf8Start: min(x, y), utf8End: max(x, y)) else { continue }
                XCTAssertEqual(map.nsRange(r), NSRange(r, in: current), "step \(step), bytes \(min(x, y))..<\(max(x, y))")
                checks += 1
            }
        }
        XCTAssertGreaterThan(checks, 10_000)
        XCTAssertGreaterThan(anchors.anchored, 500)
        XCTAssertEqual(anchors.freshWalkBytes, 0, "every report had a warm anchor")
    }

    // MARK: the keystroke

    /// A 3 MB non-ASCII book with 100 warnings across it: after the compiled
    /// text is warm, a keystroke's report walks none of its new text, draws
    /// the same ranges as the direct conversion, and costs a fraction of one
    /// walk of the text (before APP-KEYPATH: at least one walk per key).
    func testKeystrokeRebaseNeverWalksTheNewText() throws {
        var line = ""
        for w in ["Naïve", "café", "patrons", "—", "résumé", "in", "hand", "—", "watched", "the", "tide", "turn", "$x^2$", "\\emph{ö}"] { line += w + " " }
        var compiled = ""
        while compiled.utf8.count < 3_000_000 { compiled += line + "\n" }
        compiled.makeContiguousUTF8()
        let n = compiled.utf8.count
        let lineBytes = (line + "\n").utf8.count
        // 100 warnings, each on the first word of a line spread over the book.
        let diagnostics: [RuntimeV1.Diagnostic] = (0 ..< 100).map { i in
            let start = (n / lineBytes / 101 * (i + 1)) * lineBytes
            return .init(severity: .warning, message: "Overfull \\hbox", source: .init(path: "main.tex", startByte: start, endByte: start + 6),
                         recovery: nil)
        }
        let result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .ok, pages: [], diagnostics: diagnostics, pdfPath: nil)
        let anchors = MarkUTF16Anchors.shared
        // The first report after a compile: the compiled text's breadcrumbs
        // are built off the main thread, once.
        _ = EditorDiagnostics.report(for: result, path: "main.tex", compiledText: compiled, currentText: compiled)
        anchors.settle()
        anchors.resetCounters()

        // Type 20 keys in the middle of the book, each a fresh native text.
        let caret = n / 2 / lineBytes * lineBytes + 6
        var current = compiled
        var reportCpu: [Double] = []
        for k in 0 ..< 20 {
            current = Self.splice(current, at: caret + k, insert: "x")
            let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            let report = EditorDiagnostics.report(for: result, path: "main.tex", compiledText: compiled, currentText: current)
            reportCpu.append(Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6)
            XCTAssertEqual(report.marks.count, 100)
            if k % 5 == 0 { // the same ranges as the direct conversion (on a copy: its own walk is not counted)
                let copy = Self.splice(current, at: 0, insert: "")
                for (d, m) in zip(diagnostics, report.marks) {
                    let shift = d.source!.startByte >= caret ? k + 1 : 0
                    XCTAssertEqual(m.nsRange, copy.clusterAlignedNSRange(utf8Start: d.source!.startByte + shift, utf8End: d.source!.endByte + shift))
                }
            }
        }
        XCTAssertEqual(anchors.freshWalkBytes, 0, "no keystroke walked its new text for a UTF-16 offset")
        XCTAssertEqual(anchors.anchored, 20)

        // One walk of a fresh text, for scale: what each report cost before.
        var walkCpu: [Double] = []
        for k in 0 ..< 5 {
            let fresh = Self.splice(current, at: k, insert: "y")
            let end = fresh.utf8.index(fresh.utf8.startIndex, offsetBy: fresh.utf8.count - 1)
            let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            _ = NSRange(end ..< end, in: fresh)
            walkCpu.append(Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6)
        }
        let report = reportCpu.sorted()[reportCpu.count / 2], walk = walkCpu.sorted()[walkCpu.count / 2]
        print(String(format: "MarkUTF16: %d bytes, 100 marks: report p50 %.3f ms cpu, one fresh walk p50 %.3f ms cpu", n, report, walk))
        XCTAssertLessThan(report, walk * 0.5, "a keystroke's rebase costs less than one walk of the text")
    }
}
