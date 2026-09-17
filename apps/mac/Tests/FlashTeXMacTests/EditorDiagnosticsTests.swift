import AppKit
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXAccessibility
@testable import FlashTeXMac

/// Inline diagnostic marks: byte ranges → UTF-16 ranges, rebased across edits
/// or withheld as stale, never drawn on the wrong text; every mark names the
/// exact diagnostic it came from.
final class EditorDiagnosticsTests: XCTestCase {
    // "Hello wörld end": "ö" is 2 bytes, so "wörld" is bytes 6..<12 and UTF-16 6..<11.
    static let text = "Hello wörld end"

    private func result(_ diagnostics: [RuntimeV1.Diagnostic], status: RuntimeV1.Status = .recovered) -> RuntimeV1.CompileResult {
        .init(projectId: "demo", revision: 1, status: status, pages: [], diagnostics: diagnostics, pdfPath: nil)
    }

    private func diagnostic(_ severity: RuntimeV1.Severity, path: String = "main.tex",
                            start: Int = 6, end: Int = 12, source: Bool = true,
                            message: String? = nil, recovery: String?? = nil) -> RuntimeV1.Diagnostic {
        .init(severity: severity, message: message ?? "\(severity.rawValue) here",
              source: source ? .init(path: path, startByte: start, endByte: end) : nil,
              recovery: recovery ?? (severity == .error ? "rendered without bold" : nil))
    }

    // MARK: exact identity

    /// (a) A sourced error becomes one mark with the UTF-16 range of "wörld";
    /// a warning with null source is not a mark. The mark carries the result
    /// id, the diagnostic's index and its original byte span.
    func testSourcedErrorMarksAndUnsourcedWarningDoesNot() throws {
        let res = result([diagnostic(.warning, source: false), diagnostic(.error)])
        let marks = EditorDiagnostics.marks(for: res, resultID: "r-42", path: "main.tex",
                                            compiledText: Self.text, currentText: Self.text)
        XCTAssertEqual(marks.count, 1)
        let mark = try XCTUnwrap(marks.first)
        XCTAssertEqual(mark.nsRange, NSRange(location: 6, length: 5))
        XCTAssertEqual((Self.text as NSString).substring(with: mark.nsRange), "wörld")
        XCTAssertEqual(mark.severity, .error)
        XCTAssertEqual(mark.message, "error here")
        XCTAssertEqual(mark.recovery, "rendered without bold")
        XCTAssertEqual(mark.resultStatus, .recovered)
        XCTAssertEqual(mark.identity, .init(resultID: "r-42", index: 1, source: .init(path: "main.tex", startByte: 6, endByte: 12)))
        XCTAssertEqual(mark.diagnosticIndex, 1, "index into result.diagnostics, not into the marks")
        XCTAssertEqual(mark.id, "r-42#1@main.tex:6..<12")
        XCTAssertEqual(EditorDiagnostics.identity(resultID: "r-42", index: 1, in: res), mark.identity,
                       "the list row computes the same identity from the result")
        XCTAssertNil(EditorDiagnostics.identity(resultID: "r-42", index: 0, in: res), "unsourced: no mark to agree with")
        XCTAssertNil(EditorDiagnostics.identity(resultID: "r-42", index: 7, in: res))
        XCTAssertEqual(mark.toolTip, "error here\n↳ recovery: rendered without bold")
        XCTAssertEqual(mark.spokenDescription, "Error: error here — recovery: rendered without bold")
    }

    /// Two diagnostics on the same bytes with the same message are still two
    /// distinct marks (the index tells them apart), and the identity ignores
    /// the rebased range: after a prefix edit the same mark keeps its id.
    func testIdentityDistinguishesDuplicatesAndSurvivesRebase() throws {
        let res = result([diagnostic(.error), diagnostic(.error)])
        let before = EditorDiagnostics.marks(for: res, resultID: "r", path: "main.tex",
                                             compiledText: Self.text, currentText: Self.text)
        XCTAssertEqual(before.map(\.id), ["r#0@main.tex:6..<12", "r#1@main.tex:6..<12"])
        let after = EditorDiagnostics.marks(for: res, resultID: "r", path: "main.tex",
                                            compiledText: Self.text, currentText: "XY " + Self.text)
        XCTAssertEqual(after.map(\.id), before.map(\.id))
        XCTAssertEqual(after.map(\.nsRange), [NSRange(location: 9, length: 5), NSRange(location: 9, length: 5)])
        XCTAssertEqual(after.map(\.originalSource.startByte), [6, 6], "original span is kept, not the rebased one")
        XCTAssertNotEqual(before, after, "Equatable still sees the moved range")
    }

    // MARK: recovery

    /// Marks of a `recovered` result always carry a recovery line: the
    /// worker's note, or "no provisional rendering" — and an error stays an
    /// error. Marks of an `ok` result with no note have no recovery line.
    func testRecoveredResultsAlwaysShowRecoveryAndKeepErrors() throws {
        let recovered = result([diagnostic(.error, recovery: .some(nil)), diagnostic(.warning, start: 0, end: 5)], status: .recovered)
        let marks = EditorDiagnostics.marks(for: recovered, path: "main.tex", compiledText: Self.text, currentText: Self.text)
        XCTAssertEqual(marks.map(\.severity), [.error, .warning], "recovery never downgrades or hides the error")
        XCTAssertEqual(marks.map(\.recoveryLine), ["no provisional rendering", "no provisional rendering"])
        guard marks.count == 2 else { return XCTFail("expected two marks, got \(marks.count)") }
        XCTAssertEqual(marks[0].toolTip, "error here\n↳ no provisional rendering")
        XCTAssertEqual(marks[1].spokenDescription, "Warning: warning here — no provisional rendering")

        let ok = result([diagnostic(.warning)], status: .ok)
        let okMarks = EditorDiagnostics.marks(for: ok, path: "main.tex", compiledText: Self.text, currentText: Self.text)
        XCTAssertEqual(okMarks.map(\.recoveryLine), [nil])
        guard let firstOkMark = okMarks.first else { return XCTFail("expected one mark") }
        XCTAssertEqual(firstOkMark.toolTip, "warning here")

        let failed = result([diagnostic(.error)], status: .failed)
        let failedMarks = EditorDiagnostics.marks(for: failed, path: "main.tex", compiledText: Self.text, currentText: Self.text)
        XCTAssertEqual(failedMarks.map(\.toolTip), ["error here\n↳ recovery: rendered without bold"],
                       "a note given by the worker is shown whatever the status")
        XCTAssertEqual(EditorDiagnostics.recoveryLine(recovery: nil, status: .failed), nil)
    }

    // MARK: rebase and stale

    /// (b) An edit before the range shifts the mark; an edit inside withholds
    /// it and reports it as stale with its identity and a count.
    func testMarksRebaseAcrossPrefixEditAndDropWhenEditOverlaps() throws {
        let res = result([diagnostic(.warning)])
        let shifted = EditorDiagnostics.report(for: res, path: "main.tex",
                                               compiledText: Self.text, currentText: "XY " + Self.text)
        XCTAssertEqual(shifted.marks.count, 1)
        XCTAssertEqual(shifted.staleCount, 0)
        XCTAssertNil(shifted.staleNote)
        XCTAssertEqual(shifted.edit, .init(startByte: 0, oldEndByte: 0, newEndByte: 3, replacement: "XY "))
        let mark = try XCTUnwrap(shifted.marks.first)
        XCTAssertEqual(mark.nsRange, NSRange(location: 9, length: 5))
        XCTAssertEqual(("XY " + Self.text as NSString).substring(with: mark.nsRange), "wörld")
        XCTAssertEqual(mark.severity, .warning)
        XCTAssertNil(mark.recovery)
        XCTAssertEqual(mark.toolTip, "warning here\n↳ no provisional rendering")

        let inside = EditorDiagnostics.report(for: res, resultID: "r", path: "main.tex",
                                              compiledText: Self.text, currentText: "Hello wöXrld end")
        XCTAssertEqual(inside.marks, [], "a range overlapping the edit must be dropped, not stretched")
        XCTAssertEqual(inside.stale, [.init(identity: .init(resultID: "r", index: 0, source: .init(path: "main.tex", startByte: 6, endByte: 12)),
                                            severity: .warning, message: "warning here")])
        XCTAssertEqual(inside.staleCount, 1)
        XCTAssertEqual(inside.staleIdentities, [inside.stale[0].identity])
        XCTAssertEqual(inside.staleNote, "1 warning under edited text not underlined until the next compile")

        // Without compiled text there is nothing to rebase against; the raw
        // offsets are applied only if they are a valid range in the buffer.
        let unknown = EditorDiagnostics.report(for: res, path: "main.tex", compiledText: nil, currentText: "Hi")
        XCTAssertEqual(unknown, .empty)
        let invalid = EditorDiagnostics.marks(for: result([diagnostic(.error, start: 7, end: 8)]),
                                              path: "main.tex", compiledText: Self.text, currentText: Self.text)
        XCTAssertEqual(invalid, [], "a range starting inside a multi-byte scalar is refused")
    }

    /// Only the diagnostics under the edit go stale; the others keep exact
    /// positions on both sides of it, and the note counts by severity.
    func testOnlyOverlappingMarksGoStale() throws {
        // "Hello wörld end": edit replaces "wö" (bytes 6..<9) with "W".
        let res = result([
            diagnostic(.error, start: 0, end: 5),      // "Hello": before the edit
            diagnostic(.error, start: 6, end: 12),     // "wörld": overlaps
            diagnostic(.warning, start: 7, end: 9),    // "ö": overlaps
            diagnostic(.warning, start: 12, end: 15),  // " en": after
        ])
        let edited = "Hello Wrld end"
        let report = EditorDiagnostics.report(for: res, resultID: "r", path: "main.tex", compiledText: Self.text, currentText: edited)
        XCTAssertEqual(report.marks.map(\.diagnosticIndex), [0, 3])
        XCTAssertEqual(report.marks.map { (edited as NSString).substring(with: $0.nsRange) }, ["Hello", " en"])
        XCTAssertEqual(report.stale.map(\.identity.index), [1, 2])
        XCTAssertEqual(report.staleNote, "1 error and 1 warning under edited text not underlined until the next compile")
        let twoErrors = EditorDiagnostics.report(for: result([diagnostic(.error), diagnostic(.error, start: 8, end: 10)]),
                                                 path: "main.tex", compiledText: Self.text, currentText: edited)
        XCTAssertEqual(twoErrors.staleNote, "2 errors under edited text not underlined until the next compile")
    }

    /// (c) Diagnostics for another document do not mark this one.
    func testOtherPathProducesNoMark() {
        let marks = EditorDiagnostics.marks(for: result([diagnostic(.error, path: "chapter.tex")]),
                                            path: "main.tex", compiledText: Self.text, currentText: Self.text)
        XCTAssertEqual(marks, [])
    }

    /// (d) The multipage sample's error mark slices to `\textbf{oops`.
    func testMultipageSampleErrorMarkSlicesToItsText() throws {
        let req = try RuntimeV1.decodeCompileRequest(Data(contentsOf: CaretSyncTests.requestURL))
        let res = try RuntimeV1.decodeCompileResult(Data(contentsOf: CaretSyncTests.resultURL))
        let doc = try XCTUnwrap(req.payload.documents.first { $0.path == req.payload.entryPath })
        let marks = EditorDiagnostics.marks(for: res.payload, resultID: res.id, path: doc.path,
                                            compiledText: doc.text, currentText: doc.text)
        XCTAssertEqual(marks.count, 1, "one sourced error; the warning has null source")
        let mark = try XCTUnwrap(marks.first)
        XCTAssertEqual(mark.severity, .error)
        XCTAssertEqual((doc.text as NSString).substring(with: mark.nsRange), "\\textbf{oops")
        XCTAssertNotNil(mark.recovery)
        XCTAssertEqual(mark.identity.resultID, res.id)
        let index = try XCTUnwrap(res.payload.diagnostics.firstIndex { $0.source != nil })
        XCTAssertEqual(mark.identity, EditorDiagnostics.identity(resultID: res.id, index: index, in: res.payload))
    }

    // MARK: grapheme clusters

    /// A byte span never splits a user-perceived character: a combining mark
    /// typed right after the marked text joins the underline (the edit sits
    /// after the span, so the mark is not stale), and a span reported inside
    /// an emoji sequence widens to the whole sequence.
    func testMarksNeverSplitGraphemeClusters() throws {
        // "abc" → "abć" typed as c + U+0301: the mark on "c" (bytes 2..<3)
        // is before the insertion at byte 3, so it rebases unchanged, but the
        // cluster is now "c\u{301}" (2 UTF-16 units).
        let res = result([diagnostic(.error, start: 2, end: 3)])
        let edited = "abc\u{301}"
        let marks = EditorDiagnostics.marks(for: res, path: "main.tex", compiledText: "abc", currentText: edited)
        XCTAssertEqual(marks.map(\.nsRange), [NSRange(location: 2, length: 2)])
        let mark = try XCTUnwrap(marks.first)
        XCTAssertEqual((edited as NSString).substring(with: mark.nsRange), "c\u{301}")
        XCTAssertEqual(edited[try XCTUnwrap(Range(mark.nsRange, in: edited))].count, 1, "one Character")

        // Family emoji: 👨‍👩‍👧 = 3 scalars joined by 2 ZWJ, 18 bytes, 8 UTF-16 units.
        let family = "x\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}y"
        XCTAssertEqual(family.utf8.count, 20)
        // Bytes 5..<8 is the first ZWJ (scalar-aligned, mid-cluster).
        let mid = EditorDiagnostics.marks(for: result([diagnostic(.warning, start: 5, end: 8)]), path: "main.tex",
                                          compiledText: family, currentText: family)
        XCTAssertEqual(mid.map(\.nsRange), [NSRange(location: 1, length: 8)])
        let midMark = try XCTUnwrap(mid.first)
        XCTAssertEqual((family as NSString).substring(with: midMark.nsRange), "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}")
        // A zero-length span inside the cluster covers the cluster too; a span
        // ending exactly at the cluster end is left alone.
        let point = EditorDiagnostics.marks(for: result([diagnostic(.warning, start: 5, end: 5)]), path: "main.tex",
                                            compiledText: family, currentText: family)
        XCTAssertEqual(point.map(\.nsRange), [NSRange(location: 1, length: 8)])
        let whole = EditorDiagnostics.marks(for: result([diagnostic(.warning, start: 1, end: 19)]), path: "main.tex",
                                            compiledText: family, currentText: family)
        XCTAssertEqual(whole.map(\.nsRange), [NSRange(location: 1, length: 8)])
        let tail = EditorDiagnostics.marks(for: result([diagnostic(.warning, start: 19, end: 20)]), path: "main.tex",
                                           compiledText: family, currentText: family)
        XCTAssertEqual(tail.map(\.nsRange), [NSRange(location: 9, length: 1)])
        // Inside a multi-byte scalar is still refused (not a scalar boundary).
        XCTAssertEqual(EditorDiagnostics.marks(for: result([diagnostic(.warning, start: 2, end: 8)]), path: "main.tex",
                                               compiledText: family, currentText: family), [])
    }

    // MARK: performance — stale invalidation never blocks typing

    /// Marks on a 60 KB document with 200 diagnostics, recomputed after a
    /// keystroke in the middle of the text. Measured with the build the test
    /// runs in; the number is recorded in the coordination handoff. The bound
    /// is 2 ms per call (best of 25 calls, so a preempted run does not fail a
    /// green tree; the median is printed for the record).
    func testRebaseOfLargeDocumentIsFast() throws {
        // ~60 KB: 1000 lines of LaTeX-ish text with some non-ASCII, ~60 bytes each.
        let line = "\\textbf{Wörter} und $x_i^2$ auf Zeile mit Ünicode und Text.\n"
        var compiled = ""
        for _ in 0..<1000 { compiled += line }
        let bytes = compiled.utf8.count
        XCTAssertGreaterThanOrEqual(bytes, 60_000)
        let lineBytes = line.utf8.count
        // 200 diagnostics spread over the document, each on "\textbf{Wörter}".
        var diags: [RuntimeV1.Diagnostic] = []
        for i in 0..<200 {
            let start = (i * 5) * lineBytes
            diags.append(.init(severity: i % 3 == 0 ? .error : .warning, message: "diag \(i)",
                               source: .init(path: "main.tex", startByte: start, endByte: start + 15),
                               recovery: i % 3 == 0 ? "rendered plain" : nil))
        }
        let res = result(diags)
        // Keystroke: insert one character in the middle of line 500.
        let cut = 500 * lineBytes + 5 // inside "\\textbf{Wörter}" of line 500 (ASCII byte)
        let cutIndex = compiled.utf8.index(compiled.utf8.startIndex, offsetBy: cut)
        let current = String(compiled[..<cutIndex]) + "Z" + String(compiled[cutIndex...])
        XCTAssertEqual(current.utf8.count, bytes + 1)

        var samples: [Double] = []
        var report = EditorDiagnostics.Report.empty
        for _ in 0..<25 {
            // A fresh copy each round so breadcrumb/bridging caches of the
            // current String instance do not flatter the measurement.
            let currentCopy = String(decoding: Array(current.utf8), as: UTF8.self)
            let t0 = DispatchTime.now().uptimeNanoseconds
            report = EditorDiagnostics.report(for: res, resultID: "bench", path: "main.tex",
                                              compiledText: compiled, currentText: currentCopy)
            let t1 = DispatchTime.now().uptimeNanoseconds
            samples.append(Double(t1 - t0) / 1e6)
        }
        // 100 marks before the edit (rebased unchanged), 99 after (shifted by
        // one byte), and the one on line 500 is stale.
        XCTAssertEqual(report.marks.count, 199)
        guard report.marks.count == 199 else { return XCTFail("expected 199 marks, got \(report.marks.count)") }
        XCTAssertEqual(report.staleCount, 1)
        XCTAssertEqual(report.stale.first?.identity.index, 100)
        XCTAssertEqual(report.marks[99].nsRange.location, report.marks[98].nsRange.location + 5 * (line as NSString).length)
        XCTAssertEqual(report.marks[100].nsRange.location, report.marks[98].nsRange.location + 15 * (line as NSString).length + 1)
        let sorted = samples.sorted()
        let best = sorted[0], median = sorted[sorted.count / 2], worst = sorted[sorted.count - 1]
        let build: String
        #if DEBUG
        build = "debug"
        #else
        build = "release"
        #endif
        print(String(format: "EditorDiagnostics.report bench (%@): %d bytes, %d diagnostics, %d stale: best %.3f ms, median %.3f ms, worst %.3f ms",
                     build, bytes, diags.count, report.staleCount, best, median, worst))
        XCTAssertLessThan(best, 2.0, "marks() after an edit must stay far below one frame so stale invalidation never blocks typing")
    }

    // MARK: keyboard navigation

    /// Next/previous visit marks in document order, wrap at both ends, and
    /// announce "n of m" with the line and recovery line.
    func testKeyboardNavigationOrderWrapAndAnnouncement() throws {
        let text = "Hello wörld end\nsecond line\nthird"
        let res = result([
            diagnostic(.warning, start: 17, end: 23, message: "second"),  // "second" on line 2
            diagnostic(.error, start: 6, end: 12),                        // "wörld" on line 1
            diagnostic(.warning, start: 29, end: 34, message: "third", recovery: .some("kept")),
        ])
        let marks = EditorDiagnostics.marks(for: res, resultID: "r", path: "main.tex", compiledText: text, currentText: text)
        XCTAssertEqual(marks.map(\.diagnosticIndex), [0, 1, 2], "marks follow result order")
        guard marks.count == 3 else { return XCTFail("expected three marks, got \(marks.count)") }
        let items = EditorDiagnostics.navigationItems(marks)
        XCTAssertEqual(items.map(\.id), ["r#1@main.tex:6..<12", "r#0@main.tex:17..<23", "r#2@main.tex:29..<34"],
                       "navigation is in document order")
        XCTAssertEqual(EditorDiagnosticNavigation.summary(items), "1 error, 2 warnings")
        XCTAssertEqual(EditorDiagnosticNavigation.summary([]), "no diagnostics")

        // From the top: next is the error on line 1.
        let first = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 0, forward: true, in: text))
        XCTAssertEqual(first.ordinal, 1); XCTAssertEqual(first.total, 3); XCTAssertFalse(first.wrapped)
        XCTAssertEqual(first.item.id, marks[1].id)
        XCTAssertEqual(first.announcement, "Error 1 of 3, line 1: error here — recovery: rendered without bold")
        // The caret lands on the mark; stepping again by identity goes on.
        let second = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: first.item.nsRange.location, forward: true,
                                                          currentID: first.item.id, in: text))
        XCTAssertEqual(second.announcement, "Warning 2 of 3, line 2: second — no provisional rendering")
        let third = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 0, forward: true, currentID: second.item.id, in: text))
        XCTAssertEqual(third.announcement, "Warning 3 of 3, line 3: third — recovery: kept")
        // Past the end: wraps to the first and says so.
        let wrapped = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 0, forward: true, currentID: third.item.id, in: text))
        XCTAssertTrue(wrapped.wrapped)
        XCTAssertEqual(wrapped.announcement, "Error 1 of 3, line 1: error here — recovery: rendered without bold (wrapped to start)")
        // Previous from the first wraps to the last.
        let back = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 6, forward: false, in: text))
        XCTAssertTrue(back.wrapped)
        XCTAssertEqual(back.announcement, "Warning 3 of 3, line 3: third — recovery: kept (wrapped to end)")
        // By position, without an identity: caret inside "second" on line 2.
        // Previous goes to the start of the mark the caret is in; from that
        // start, previous goes on to the mark before it.
        let mid = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 18, forward: false, in: text))
        XCTAssertEqual(mid.ordinal, 2); XCTAssertFalse(mid.wrapped)
        let beforeMid = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: mid.item.nsRange.location, forward: false, in: text))
        XCTAssertEqual(beforeMid.ordinal, 1)
        let midNext = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 18, forward: true, in: text))
        XCTAssertEqual(midNext.ordinal, 3)
        // Unknown identity falls back to the caret position.
        let unknown = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 18, forward: true, currentID: "gone", in: text))
        XCTAssertEqual(unknown.ordinal, 3)
        XCTAssertNil(EditorDiagnostics.step([], fromUTF16: 0, forward: true, in: text))
        // The diagnostic under the caret, without moving.
        let here = try XCTUnwrap(EditorDiagnosticNavigation.current(items, atUTF16: 11))
        XCTAssertEqual(here.ordinal, 1)
        XCTAssertEqual(here.announcement, "Error 1 of 3: error here — recovery: rendered without bold")
        XCTAssertNil(EditorDiagnosticNavigation.current(items, atUTF16: 13))
    }

    /// Two marks starting at the same offset are both reachable: the longer
    /// (enclosing) one comes first, and stepping by identity visits each.
    func testNavigationVisitsMarksSharingAStart() throws {
        let res = result([diagnostic(.warning, start: 6, end: 9, message: "inner"),  // "wö"
                          diagnostic(.error, start: 6, end: 12, message: "outer")])
        let marks = EditorDiagnostics.marks(for: res, resultID: "r", path: "main.tex", compiledText: Self.text, currentText: Self.text)
        let a = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 0, forward: true))
        XCTAssertEqual(a.item.message, "outer"); XCTAssertEqual(a.ordinal, 1)
        let b = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 6, forward: true, currentID: a.item.id))
        XCTAssertEqual(b.item.message, "inner"); XCTAssertEqual(b.ordinal, 2)
        XCTAssertNil(b.line, "no text given: line unknown")
        let c = try XCTUnwrap(EditorDiagnostics.step(marks, fromUTF16: 6, forward: true, currentID: b.item.id))
        XCTAssertEqual(c.ordinal, 1); XCTAssertTrue(c.wrapped)
    }

    // MARK: editor application (SourceEditorView, not owned here; behaviour pinned)

    /// Marks are temporary layout attributes: the text storage stays plain and
    /// out-of-range marks are ignored.
    @MainActor
    func testApplyMarksUsesTemporaryAttributesOnly() throws {
        let tv = NSTextView(frame: NSRect(x: 0, y: 0, width: 200, height: 100))
        tv.string = Self.text
        let marks = EditorDiagnostics.marks(for: result([diagnostic(.error)]), path: "main.tex",
                                            compiledText: Self.text, currentText: Self.text)
        let bogus = EditorDiagnostics.Mark(identity: .init(resultID: nil, index: 9, source: .init(path: "main.tex", startByte: 100, endByte: 105)),
                                           nsRange: NSRange(location: 100, length: 5), severity: .warning,
                                           message: "stale", recovery: nil, resultStatus: .ok)
        SourceEditorView.applyMarks(marks + [bogus], to: tv)
        let lm = try XCTUnwrap(tv.layoutManager)
        var effective = NSRange()
        let attrs = lm.temporaryAttributes(atCharacterIndex: 6, effectiveRange: &effective)
        XCTAssertEqual(effective, NSRange(location: 6, length: 5))
        XCTAssertEqual(attrs[.underlineColor] as? NSColor, .systemRed)
        XCTAssertEqual(attrs[.toolTip] as? String, "error here\n↳ recovery: rendered without bold")
        XCTAssertNil(lm.temporaryAttribute(.underlineStyle, atCharacterIndex: 0, effectiveRange: nil))
        XCTAssertEqual(tv.string, Self.text)
        let storage = try XCTUnwrap(tv.textStorage)
        XCTAssertNil(storage.attribute(.underlineStyle, at: 6, effectiveRange: nil))

        SourceEditorView.applyMarks([], to: tv)
        XCTAssertNil(lm.temporaryAttribute(.underlineStyle, atCharacterIndex: 6, effectiveRange: nil))
    }
}
