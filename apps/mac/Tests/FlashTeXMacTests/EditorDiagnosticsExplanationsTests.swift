import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXAccessibility
@testable import FlashTeXMac

/// Explanation replies (the `flashtex-explain` wire shape; the helper and its
/// client retired with the old engine in S7): decoding, bounds, per-result
/// caching, and the cost a keystroke pays once explanations exist.
final class EditorDiagnosticsExplanationsTests: XCTestCase {
    typealias Explanation = EditorDiagnostics.Explanation
    typealias Limits = EditorDiagnostics.ExplanationLimits

    static let samples = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Samples")

    /// The message, recovery and span the real compiler
    /// (`crates/compiler` release, main checkout) reports for
    /// `Samples/recovery-demo.tex` — captured 2026-09-12, cross-checked
    /// against a live compile below when `FLASHTEX_COMPILER` is set.
    static let fooDiagnostic = RuntimeV1.Diagnostic(
        severity: .error,
        message: "\\foo is not supported by this compiler version; unrestricted TeX math mode is not implemented",
        source: .init(path: "main.tex", startByte: 181, endByte: 185),
        recovery: "skipped the command; any braced argument was typeset as plain text")

    /// One reply line as `flashtex-explain` wrote it for the diagnostic above
    /// (crate 2bf14cd, `explain_all` with supported = DEFAULT_SUPPORTED_COMMANDS).
    static let realReplyLine = #"""
    {"id":"explain-1","type":"explanations","explanations":[{"catalog_id":"unsupported-command","title":"\\foo is not supported","category":"unsupported-command","severity":"error","message":"\\foo is not supported by this compiler version; unrestricted TeX math mode is not implemented","why":"The compiler implements a fixed, documented set of commands and has no macro packages, so \\foo has no definition here. It may be a misspelling of a supported command, a package command, or a command this version does not implement yet.","what_happened":"The compiler skipped the command; any braced argument was typeset as plain text.","suggestions":[{"text":"Remove \\foo and keep its argument text.","confidence":"low","edits":[{"path":"main.tex","start_byte":181,"end_byte":190,"replacement":"bar"}]},{"text":"No similar supported command; this compiler version has no packages or macro expansion, so the command's effect is unavailable.","confidence":"low","edits":[]}],"context":{"path":"main.tex","start_byte":115,"end_byte":296,"text":"\\subsection{Recovery}\nMath like $x^2$ is not implemented yet, and \\foo{bar} is unsupported: both are diagnosed,\nnever silently dropped, and the rest of the paragraph still typesets.","span_start":181,"span_end":185,"line":5,"column":45,"span_in_bounds":true}}]}
    """#

    private func recoveryDemoText() throws -> String {
        try String(contentsOf: Self.samples.appendingPathComponent("recovery-demo.tex"), encoding: .utf8)
    }

    private func result(_ diagnostics: [RuntimeV1.Diagnostic], status: RuntimeV1.Status = .recovered) -> RuntimeV1.CompileResult {
        .init(projectId: "demo", revision: 1, status: status, pages: [], diagnostics: diagnostics, pdfPath: nil)
    }

    // MARK: decoding and bounds

    /// The crate's fixed-key JSON decodes field for field; a context with a
    /// drifted key is a malformed reply (the shape is the contract), never a
    /// silently dropped field.
    func testDecodesTheCrateShapeAndRefusesADriftedKey() throws {
        let drifted = Data(Self.realReplyLine.replacingOccurrences(of: "\"end_byte\":296", with: "\"end_byt\":296").utf8)
        XCTAssertThrowsError(try EditorDiagnostics.decodeExplanations(drifted, expectedCount: 1)) { error in
            guard case EditorDiagnostics.ExplanationFailure.malformed(let why) = error else { return XCTFail("\(error)") }
            XCTAssertTrue(why.contains("end_byte"), why)
        }
        let fixed = Data(Self.realReplyLine.utf8)
        let decoded = try EditorDiagnostics.decodeExplanations(fixed, expectedCount: 1)
        XCTAssertEqual(decoded.count, 1)
        let x = try XCTUnwrap(decoded.first)
        XCTAssertEqual(x.catalogID, "unsupported-command")
        XCTAssertTrue(x.isCatalogued)
        XCTAssertEqual(x.title, "\\foo is not supported")
        XCTAssertEqual(x.category, "unsupported-command")
        XCTAssertEqual(x.severity, "error")
        XCTAssertEqual(x.suggestions.count, 2)
        let firstSuggestion = try XCTUnwrap(x.suggestions.first)
        XCTAssertEqual(firstSuggestion.edits, [.init(path: "main.tex", startByte: 181, endByte: 190, replacement: "bar")])
        XCTAssertEqual(firstSuggestion.confidence, "low")
        XCTAssertEqual(x.context?.line, 5); XCTAssertEqual(x.context?.column, 45); XCTAssertEqual(x.context?.spanInBounds, true)
        XCTAssertTrue(x.line.hasPrefix("explain: \\foo is not supported — The compiler implements a fixed, documented set of commands and has no macro packages, so \\foo has no definition here."), x.line)
        XCTAssertTrue(x.line.hasSuffix("…"), "the why paragraph is cut at the line limit")
        XCTAssertEqual(x.line.count, Limits.maxLineCharacters)
        // A bare array (the crate's `explanations_to_json` output) decodes too.
        let bare = try EditorDiagnostics.decodeExplanations(Data("[]".utf8), expectedCount: 0)
        XCTAssertEqual(bare, [])
        // Count mismatch: entries are matched to diagnostics by index.
        XCTAssertThrowsError(try EditorDiagnostics.decodeExplanations(fixed, expectedCount: 2)) { error in
            XCTAssertEqual(error as? EditorDiagnostics.ExplanationFailure, .countMismatch(expected: 2, actual: 1))
        }
    }

    /// Oversized replies are refused whole; long fields are cut, suggestion
    /// and edit lists capped, and the displayed line never exceeds its limit.
    func testBoundsCutLongFieldsAndRefuseOversizedReplies() throws {
        let long = String(repeating: "w", count: 5000)
        let suggestions = (0..<10).map { i in
            "{\"text\":\"s\(i) \(long)\",\"confidence\":\"high\",\"edits\":[" + (0..<20).map { _ in
                "{\"path\":\"main.tex\",\"start_byte\":0,\"end_byte\":0,\"replacement\":\"\(long)\"}"
            }.joined(separator: ",") + "]}"
        }.joined(separator: ",")
        let json = "[{\"catalog_id\":null,\"title\":\"\(long)\",\"category\":\"unknown\",\"severity\":\"warning\",\"message\":\"m\",\"why\":\"\(long)\",\"what_happened\":\"\(long)\",\"suggestions\":[\(suggestions)],\"context\":{\"path\":\"main.tex\",\"start_byte\":0,\"end_byte\":1,\"text\":\"\(long)\",\"span_start\":0,\"span_end\":1,\"line\":1,\"column\":1,\"span_in_bounds\":true}}]"
        let x = try XCTUnwrap(EditorDiagnostics.decodeExplanations(Data(json.utf8), expectedCount: 1).first)
        XCTAssertEqual(x.title.count, Limits.maxTitleCharacters)
        XCTAssertTrue(x.title.hasSuffix("…"))
        XCTAssertEqual(x.why.count, Limits.maxParagraphCharacters)
        XCTAssertEqual(x.whatHappened.count, Limits.maxParagraphCharacters)
        XCTAssertEqual(x.suggestions.count, Limits.maxSuggestions)
        let firstBoundedSuggestion = try XCTUnwrap(x.suggestions.first)
        XCTAssertEqual(firstBoundedSuggestion.edits.count, Limits.maxEditsPerSuggestion)
        let firstBoundedEdit = try XCTUnwrap(firstBoundedSuggestion.edits.first)
        XCTAssertEqual(firstBoundedEdit.replacement.count, Limits.maxContextCharacters)
        XCTAssertEqual(x.context?.text.count, Limits.maxContextCharacters)
        XCTAssertFalse(x.isCatalogued)
        XCTAssertEqual(x.line, "explain: " + x.title + " (not in the catalogue)", "uncatalogued: title only, never the why")
        XCTAssertLessThanOrEqual(x.line.count, Limits.maxLineCharacters)
        XCTAssertTrue(x.line.hasPrefix("explain: www"))

        XCTAssertThrowsError(try EditorDiagnostics.decodeExplanations(Data(json.utf8), expectedCount: 1, maxBytes: 1024)) { error in
            XCTAssertEqual(error as? EditorDiagnostics.ExplanationFailure, .oversized(bytes: json.utf8.count))
        }
        XCTAssertThrowsError(try EditorDiagnostics.decodeExplanations(Data("{\"explanations\":5}".utf8), expectedCount: 0)) { error in
            guard case EditorDiagnostics.ExplanationFailure.malformed = error else { return XCTFail("\(error)") }
        }
        XCTAssertThrowsError(try EditorDiagnostics.decodeExplanations(Data("not json".utf8), expectedCount: 0)) { error in
            guard case EditorDiagnostics.ExplanationFailure.malformed = error else { return XCTFail("\(error)") }
        }
    }

    // MARK: cache, marks, tooltip, navigation

    func testCacheIsBoundedPerResultAndAttachFollowsDiagnosticIndex() throws {
        var cache = EditorDiagnostics.ExplanationCache(maxResults: 2)
        let a = try EditorDiagnostics.decodeExplanations(Data(Self.realReplyLine.utf8), expectedCount: 1)
        cache.store(a, for: "r1"); cache.store([], for: "r2"); cache.store(a, for: "r3")
        XCTAssertNil(cache["r1"], "oldest result evicted")
        XCTAssertEqual(cache["r3"], a)
        XCTAssertEqual(cache.count, 2)
        XCTAssertNil(cache[nil])
        XCTAssertNil(cache.explanation(resultID: "r3", index: 1))
        XCTAssertEqual(cache.explanation(resultID: "r3", index: 0)?.catalogID, "unsupported-command")
        cache.store(a, for: "r3")
        XCTAssertEqual(cache.count, 2, "re-storing does not evict")

        // Two diagnostics, the first without a source (no mark), the second
        // marked: the mark for diagnostic 1 gets explanation 1, not 0.
        let text = try recoveryDemoText()
        let res = result([.init(severity: .warning, message: "unsourced", source: nil, recovery: nil), Self.fooDiagnostic])
        let two = [Explanation(catalogID: nil, title: "zero", category: "unknown", severity: "warning", message: "unsourced",
                               why: "", whatHappened: "", suggestions: [], context: nil), a[0]]
        let report = EditorDiagnostics.report(for: res, resultID: "r3", path: "main.tex", compiledText: text, currentText: text)
        XCTAssertEqual(report.marks.map(\.explanation), [nil])
        let attached = EditorDiagnostics.attach(two, to: report)
        XCTAssertEqual(attached.marks.count, 1)
        let attachedMark = try XCTUnwrap(attached.marks.first)
        XCTAssertEqual(attachedMark.diagnosticIndex, 1)
        XCTAssertEqual(attachedMark.explanation, a[0].line)
        XCTAssertEqual(attachedMark.toolTip,
                       Self.fooDiagnostic.message + "\n↳ recovery: " + Self.fooDiagnostic.recovery! + "\n↳ " + a[0].line)
        XCTAssertTrue(attachedMark.spokenDescription.hasSuffix(" — " + a[0].line))
        XCTAssertEqual(EditorDiagnostics.attach(nil, to: report), report, "no explanations: report unchanged")
        XCTAssertEqual(EditorDiagnostics.attach([], to: report), report)
        // Fewer explanations than diagnostics never crashes; the mark just has none.
        XCTAssertEqual(EditorDiagnostics.attach([two[0]], to: report).marks[0].explanation, nil)
        // A stale mark (span edited) stays stale; explanations attach only to drawn marks.
        var edited = text
        edited.insert("X", at: edited.utf8.index(edited.utf8.startIndex, offsetBy: 183))
        let stale = EditorDiagnostics.attach(two, to: EditorDiagnostics.report(for: res, resultID: "r3", path: "main.tex", compiledText: text, currentText: edited))
        XCTAssertEqual(stale.marks, []); XCTAssertEqual(stale.staleCount, 1)

        // Keyboard navigation announces the explanation after the recovery line.
        let step = try XCTUnwrap(EditorDiagnostics.step(attached.marks, fromUTF16: 0, forward: true, in: text))
        XCTAssertEqual(step.announcement, "Error 1 of 1, line 5: " + Self.fooDiagnostic.message
                       + " — recovery: " + Self.fooDiagnostic.recovery! + " — " + a[0].line)
        XCTAssertEqual(EditorDiagnostics.navigationItems(attached.marks).map(\.explanation), [a[0].line])
    }

    // MARK: cost after the first fetch

    /// A keystroke pays `report` + `attach` with explanations already cached:
    /// 62 KB document, 200 diagnostics, 200 explanations (decoded once, as
    /// the first fetch does). Bound 2 ms on the best of 25 samples.
    func testMarksPlusExplanationsStayFastAfterFirstFetch() throws {
        let line = "\\textbf{Wörter} und $x_i^2$ auf Zeile mit Ünicode und Text.\n"
        var compiled = ""
        for _ in 0..<1000 { compiled += line }
        let lineBytes = line.utf8.count
        var diags: [RuntimeV1.Diagnostic] = []
        for i in 0..<200 {
            let start = (i * 5) * lineBytes
            diags.append(.init(severity: i % 3 == 0 ? .error : .warning, message: "\\cmd\(i) is not supported by this compiler version",
                               source: .init(path: "main.tex", startByte: start, endByte: start + 15),
                               recovery: i % 3 == 0 ? "rendered plain" : nil))
        }
        let res = result(diags)
        // The first fetch: one reply with 200 crate-shaped entries, decoded and bounded once.
        let entries = diags.enumerated().map { i, d in
            "{\"catalog_id\":\"unsupported-command\",\"title\":\"\\\\cmd\(i) is not supported\",\"category\":\"unsupported-command\",\"severity\":\"\(d.severity.rawValue)\",\"message\":\"\(d.message.replacingOccurrences(of: "\\", with: "\\\\"))\",\"why\":\"The compiler implements a fixed, documented set of commands and has no macro packages, so the command has no definition here.\",\"what_happened\":\"The compiler skipped the command.\",\"suggestions\":[{\"text\":\"Remove the command.\",\"confidence\":\"low\",\"edits\":[{\"path\":\"main.tex\",\"start_byte\":\(d.source!.startByte),\"end_byte\":\(d.source!.endByte),\"replacement\":\"\"}]}],\"context\":{\"path\":\"main.tex\",\"start_byte\":0,\"end_byte\":10,\"text\":\"\\\\textbf{Wö\",\"span_start\":0,\"span_end\":8,\"line\":1,\"column\":1,\"span_in_bounds\":true}}"
        }
        let replyLine = "{\"id\":\"explain-1\",\"type\":\"explanations\",\"explanations\":[" + entries.joined(separator: ",") + "]}"
        let t0 = DispatchTime.now().uptimeNanoseconds
        let explanations = try EditorDiagnostics.decodeExplanations(Data(replyLine.utf8), expectedCount: 200)
        let firstFetchMs = Double(DispatchTime.now().uptimeNanoseconds - t0) / 1e6
        var cache = EditorDiagnostics.ExplanationCache()
        cache.store(explanations, for: "bench")

        let cut = 500 * lineBytes + 5
        let cutIndex = compiled.utf8.index(compiled.utf8.startIndex, offsetBy: cut)
        let current = String(compiled[..<cutIndex]) + "Z" + String(compiled[cutIndex...])
        var samples: [Double] = []
        var report = EditorDiagnostics.Report.empty
        for _ in 0..<25 {
            let currentCopy = String(decoding: Array(current.utf8), as: UTF8.self)
            let s0 = DispatchTime.now().uptimeNanoseconds
            report = EditorDiagnostics.attach(cache["bench"], to: EditorDiagnostics.report(
                for: res, resultID: "bench", path: "main.tex", compiledText: compiled, currentText: currentCopy))
            samples.append(Double(DispatchTime.now().uptimeNanoseconds - s0) / 1e6)
        }
        XCTAssertEqual(report.marks.count, 199)
        XCTAssertEqual(report.staleCount, 1)
        XCTAssertEqual(report.marks.compactMap(\.explanation).count, 199)
        guard report.marks.count == 199 else { return XCTFail("expected 199 marks, got \(report.marks.count)") }
        XCTAssertEqual(report.marks[150].explanation, "explain: \\cmd151 is not supported — The compiler implements a fixed, documented set of commands and has no macro packages, so the command has no definition here.")
        XCTAssertTrue(report.marks[0].toolTip.hasSuffix(report.marks[0].explanation!))
        let sorted = samples.sorted()
        let build: String
        #if DEBUG
        build = "debug"
        #else
        build = "release"
        #endif
        print(String(format: "EditorDiagnostics marks+explanations bench (%@): %d bytes, %d diagnostics, reply %d bytes: first fetch decode %.3f ms; per keystroke best %.3f ms, median %.3f ms, worst %.3f ms",
                     build, compiled.utf8.count, diags.count, replyLine.utf8.count, firstFetchMs, sorted[0], sorted[sorted.count / 2], sorted[sorted.count - 1]))
        // marks + cached explanations must stay far below one frame after the
        // first fetch; a speed budget (TimingBudget.swift).
        TimingBudget.assertWithin(sorted[0], 2.0, "EditorDiagnostics marks + cached explanations (\(build), best of \(sorted.count))")
    }
}
