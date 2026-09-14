import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// Reviewed quick fixes: suggestion edits (compiled-text byte ranges) become
/// one grouped, previewable replacement for the current editor text, or a
/// typed refusal. Nothing is applied by these APIs.
final class EditorDiagnosticsQuickFixTests: XCTestCase {
    typealias QuickFix = EditorDiagnostics.QuickFix
    typealias Explanation = EditorDiagnostics.Explanation

    private func explanation(_ edits: [Explanation.Edit], suggestions extra: [Explanation.Suggestion] = [],
                             context: Explanation.Context? = nil) -> Explanation {
        Explanation(catalogID: "test", title: "t", category: "unsupported-command", severity: "error", message: "m",
                    why: "w", whatHappened: "h",
                    suggestions: [.init(text: "Fix it", confidence: "high", edits: edits)] + extra, context: context)
    }

    private func edit(_ start: Int, _ end: Int, _ text: String, path: String = "main.tex") -> Explanation.Edit {
        .init(path: path, startByte: start, endByte: end, replacement: text)
    }

    // MARK: real crate (skips loudly without the helper)

    /// The catalogue's `\foo` case: the crate's edit `181..<190 → "bar"`
    /// previews as `\foo{bar}` → `bar` on line 5 and yields one grouped edit;
    /// after a prefix edit it rebases; after an edit inside the span it is refused.
    @MainActor
    func testRealHelperFooSuggestionPreviewsRebasesAndRefuses() throws {
        let env = ProcessInfo.processInfo.environment
        guard let path = env["FLASHTEX_EXPLAIN"], FileManager.default.isExecutableFile(atPath: path) else {
            throw XCTSkip("FLASHTEX_EXPLAIN is not set to an executable flashtex-explain helper; build it from crates/diagnostic-explanations (see coordination/mac-editor-diagnostics.md) and export FLASHTEX_EXPLAIN=<path> to run this test against the real crate")
        }
        let client = try ExplanationClient(executable: URL(fileURLWithPath: path))
        defer { client.terminate() }
        let text = try String(contentsOf: EditorDiagnosticsExplanationsTests.samples.appendingPathComponent("recovery-demo.tex"), encoding: .utf8)
        let result = RuntimeV1.CompileResult(projectId: "demo", revision: 1, status: .recovered, pages: [],
                                             diagnostics: [EditorDiagnosticsExplanationsTests.fooDiagnostic], pdfPath: nil)
        let done = expectation(description: "explanations")
        var outcome: Result<[Explanation], EditorDiagnostics.ExplanationFailure>?
        client.explain(result: result, documents: [.init(path: "main.tex", text: text)], supported: Completion.defaultSupported) {
            outcome = $0; done.fulfill()
        }
        wait(for: [done], timeout: 20)
        let x = try XCTUnwrap(try XCTUnwrap(outcome).get().first)
        XCTAssertEqual(x.catalogID, "unsupported-command")

        let preview = try QuickFix.prepare(x, path: "main.tex", in: text, compiledText: text).get()
        XCTAssertEqual(preview.replacements.count, 1)
        XCTAssertEqual(preview.replacements[0].byteRange, 181..<190)
        XCTAssertEqual((text as NSString).substring(with: preview.replacements[0].nsRange), "\\foo{bar}")
        XCTAssertNotEqual(preview.replacements[0].nsRange.location, 181, "multi-byte text before the span: UTF-16 ≠ bytes")
        XCTAssertEqual(preview.before, "Math like $x^2$ is not implemented yet, and \\foo{bar} is unsupported: both are diagnosed,")
        XCTAssertEqual(preview.after, "Math like $x^2$ is not implemented yet, and bar is unsupported: both are diagnosed,")
        XCTAssertEqual((text as NSString).substring(with: preview.snippetRange), preview.before)
        XCTAssertEqual(preview.summary, "Remove \\foo and keep its argument text. (low confidence, 1 edit)")
        let grouped = preview.apply()
        XCTAssertEqual(grouped, preview.grouped)
        XCTAssertEqual(grouped.before, "\\foo{bar}"); XCTAssertEqual(grouped.text, "bar")
        XCTAssertEqual(grouped.nsRange, preview.replacements[0].nsRange)
        let after = try XCTUnwrap(grouped.applied(to: text))
        XCTAssertTrue(after.contains("and bar is unsupported"))
        XCTAssertEqual(after.utf8.count, text.utf8.count - 6)

        // Prefix edit since the compile: rebased byte-exactly (12 bytes; the document itself has multi-byte text before the span).
        let edited = "% Übersicht\n" + text
        let rebased = try QuickFix.prepare(x, path: "main.tex", in: edited, compiledText: text).get()
        XCTAssertEqual(rebased.replacements[0].byteRange, 181 + "% Übersicht\n".utf8.count..<190 + "% Übersicht\n".utf8.count)
        XCTAssertEqual((edited as NSString).substring(with: rebased.replacements[0].nsRange), "\\foo{bar}")
        XCTAssertEqual(rebased.after, preview.after)
        XCTAssertFalse(grouped.matches(edited), "a grouped edit prepared for another buffer state does not apply")
        XCTAssertNil(grouped.applied(to: edited))

        // Edit inside the span: refused, never stretched.
        let inside = text.replacingOccurrences(of: "\\foo{bar}", with: "\\foo{baz}")
        XCTAssertEqual(QuickFix.prepare(x, path: "main.tex", in: inside, compiledText: text).failureValue, .overlapsEdit(edit: 0))
        // The second suggestion is advice only.
        XCTAssertEqual(QuickFix.prepare(x, suggestion: 1, path: "main.tex", in: text, compiledText: text).failureValue, .noEdits)
        XCTAssertEqual(QuickFix.prepare(x, suggestion: 9, path: "main.tex", in: text, compiledText: text).failureValue, .noEdits)
        // The explanation's context excerpt disagrees with the compiled text: refused.
        let other = text.replacingOccurrences(of: "\\foo{bar}", with: "\\bar{bar}")
        XCTAssertEqual(QuickFix.prepare(x, path: "main.tex", in: other, compiledText: other).failureValue,
                       .bytesChanged(edit: 0, expected: "\\foo{bar}", actual: "\\bar{bar}"))
        XCTAssertEqual(QuickFix.prepare(x, path: "main.tex", in: text, compiledText: nil).failureValue, .noCompiledText)
    }

    // MARK: synthetic

    /// Two edits (delete in the preamble, insert after \begin{document}) group
    /// into one covering replacement whose result equals applying both.
    func testMultiEditSuggestionGroupsIntoOneReplacement() throws {
        let text = "\\documentclass{article}\n\\textbf{x}\n\\begin{document}\nHello\n\\end{document}\n"
        let del = text.range(of: "\\textbf{x}\n")!
        let delStart = text.utf8.distance(from: text.utf8.startIndex, to: del.lowerBound)
        let delEnd = text.utf8.distance(from: text.utf8.startIndex, to: del.upperBound)
        let ins = text.range(of: "\\begin{document}\n")!.upperBound
        let insAt = text.utf8.distance(from: text.utf8.startIndex, to: ins)
        // Given out of order on purpose: the insertion first.
        let x = explanation([edit(insAt, insAt, "\\textbf{x}\n"), edit(delStart, delEnd, "")])
        let preview = try QuickFix.prepare(x, path: "main.tex", in: text, compiledText: text).get()
        XCTAssertEqual(preview.replacements.map(\.byteRange), [delStart..<delEnd, insAt..<insAt], "ascending")
        XCTAssertEqual(preview.grouped.byteRange, delStart..<insAt)
        XCTAssertEqual(preview.grouped.before, "\\textbf{x}\n\\begin{document}\n")
        XCTAssertEqual(preview.grouped.text, "\\begin{document}\n\\textbf{x}\n")
        let expected = "\\documentclass{article}\n\\begin{document}\n\\textbf{x}\nHello\n\\end{document}\n"
        XCTAssertEqual(preview.grouped.applied(to: text), expected)
        XCTAssertEqual(preview.before, "\\textbf{x}\n\\begin{document}")
        XCTAssertEqual(preview.after, "\\begin{document}\n\\textbf{x}")
        XCTAssertEqual(preview.summary, "Fix it (high confidence, 2 edits)")
        // Sequential application of the individual replacements (highest first) gives the same text.
        var manual = text
        for r in preview.replacements.reversed() {
            manual = (manual as NSString).replacingCharacters(in: r.nsRange, with: r.text)
        }
        XCTAssertEqual(manual, expected)
    }

    func testRefusalsOverlapOtherDocumentInvalidAndStale() throws {
        let text = "Hello wörld end"   // "ö" = bytes 7..<9
        XCTAssertEqual(QuickFix.prepare(explanation([]), path: "main.tex", in: text, compiledText: text).failureValue, .noEdits)
        XCTAssertEqual(QuickFix.prepare(explanation([edit(0, 5, "Hi", path: "chapter.tex")]), path: "main.tex", in: text, compiledText: text).failureValue,
                       .otherDocument(path: "chapter.tex"))
        // Overlapping edits within one suggestion.
        XCTAssertEqual(QuickFix.prepare(explanation([edit(0, 6, "A"), edit(3, 9, "B")]), path: "main.tex", in: text, compiledText: text).failureValue,
                       .editsOverlap(a: 0, b: 1))
        // Adjacent edits and two insertions at one offset are fine and keep their order.
        let adjacent = try QuickFix.prepare(explanation([edit(0, 5, "A"), edit(5, 6, "B"), edit(6, 6, "1"), edit(6, 6, "2")]),
                                            path: "main.tex", in: text, compiledText: text).get()
        XCTAssertEqual(adjacent.grouped.text, "AB12")
        XCTAssertEqual(adjacent.grouped.applied(to: text), "AB12wörld end")
        // Inside a multi-byte scalar, reversed, out of range.
        XCTAssertEqual(QuickFix.prepare(explanation([edit(7, 8, "")]), path: "main.tex", in: text, compiledText: text).failureValue, .invalidRange(edit: 0))
        XCTAssertEqual(QuickFix.prepare(explanation([edit(5, 3, "")]), path: "main.tex", in: text, compiledText: text).failureValue, .invalidRange(edit: 0))
        XCTAssertEqual(QuickFix.prepare(explanation([edit(10, 99, "")]), path: "main.tex", in: text, compiledText: text).failureValue, .invalidRange(edit: 0))
        // Stale: the buffer changed under the edit since the compile.
        XCTAssertEqual(QuickFix.prepare(explanation([edit(6, 12, "world")]), path: "main.tex", in: "Hello wöXrld end", compiledText: text).failureValue,
                       .overlapsEdit(edit: 0))
        // Stale but disjoint: an edit after the change shifts; an edit before it does not.
        let shifted = try QuickFix.prepare(explanation([edit(13, 16, "END"), edit(0, 5, "HELLO")]), path: "main.tex",
                                           in: "Hello wöXrld end", compiledText: text).get()
        XCTAssertEqual(shifted.replacements.map(\.byteRange), [0..<5, 14..<17])
        XCTAssertEqual(shifted.grouped.applied(to: "Hello wöXrld end"), "HELLO wöXrld END")
        // Context excerpt disagreement: the explanation saw different bytes than the compiled text.
        let ctx = Explanation.Context(path: "main.tex", startByte: 0, endByte: 15, text: "Hello WORLD end",
                                      spanStart: 6, spanEnd: 12, line: 1, column: 7, spanInBounds: true)
        XCTAssertEqual(QuickFix.prepare(explanation([edit(6, 12, "x")], context: ctx), path: "main.tex", in: text, compiledText: text).failureValue,
                       .bytesChanged(edit: 0, expected: "WORLD ", actual: "wörld"))
        XCTAssertEqual(QuickFix.Refusal.overlapsEdit(edit: 0).text, "edit 1 spans text changed since the compile; recompile to refresh the fix")
    }

    /// Multi-byte ranges: the edit's bytes, the UTF-16 editor range and the
    /// preview all agree, including a replacement that itself is multi-byte
    /// and an edit after an emoji.
    func testMultiByteRangesMapExactly() throws {
        let text = "naïve 👨‍👩‍👧 café\nRésumé"
        // "café" starts after "naïve " (7 bytes) + family (18 bytes) + " " = byte 26; "café" = 5 bytes.
        let cafe = text.range(of: "café")!
        let start = text.utf8.distance(from: text.utf8.startIndex, to: cafe.lowerBound)
        let end = text.utf8.distance(from: text.utf8.startIndex, to: cafe.upperBound)
        XCTAssertEqual(start, 26); XCTAssertEqual(end, 31)
        let preview = try QuickFix.prepare(explanation([edit(start, end, "Kaffee ☕")]), path: "main.tex", in: text, compiledText: text).get()
        XCTAssertEqual(preview.replacements[0].nsRange, NSRange(cafe, in: text))
        XCTAssertEqual(preview.replacements[0].nsRange, NSRange(location: 15, length: 4), "5 + 8 + 1 UTF-16 units before café")
        XCTAssertEqual(preview.before, "naïve 👨‍👩‍👧 café")
        XCTAssertEqual(preview.after, "naïve 👨‍👩‍👧 Kaffee ☕")
        XCTAssertEqual(preview.grouped.applied(to: text), "naïve 👨‍👩‍👧 Kaffee ☕\nRésumé")
        XCTAssertEqual(preview.snippetRange, NSRange(location: 0, length: 19))
        // Editing the second line after a multi-byte first line; the edit is
        // one-byte "R" before a two-byte "é".
        let resume = text.range(of: "Résumé")!
        let s2 = text.utf8.distance(from: text.utf8.startIndex, to: resume.lowerBound)
        let p2 = try QuickFix.prepare(explanation([edit(s2, s2 + 1, "r")]), path: "main.tex", in: text, compiledText: text).get()
        XCTAssertEqual(p2.before, "Résumé"); XCTAssertEqual(p2.after, "résumé")
        XCTAssertEqual(p2.snippetRange, NSRange(location: 20, length: 6))
        // The same edit after a prefix insertion of multi-byte text rebases.
        let edited = "Ünicode\n" + text
        let p3 = try QuickFix.prepare(explanation([edit(s2, s2 + 1, "r")]), path: "main.tex", in: edited, compiledText: text).get()
        XCTAssertEqual(QuickFix.prepare(explanation([edit(s2, s2 + 2, "r")]), path: "main.tex", in: text, compiledText: text).failureValue,
                       .invalidRange(edit: 0), "ends inside the two-byte é")
        XCTAssertEqual((edited as NSString).substring(with: p3.replacements[0].nsRange), "R")
        XCTAssertEqual(p3.grouped.applied(to: edited), "Ünicode\n" + "naïve 👨‍👩‍👧 café\nrésumé")
    }

    func testHelpReplacementBoundsAndRevisionGate() throws {
        let text = "Hello wörld end" // "wörld" = bytes 6..<12
        let d = RuntimeV1.Diagnostic(
            severity: .error, message: "`\\tilde` is not implemented in text mode",
            source: .init(path: "main.tex", startByte: 6, endByte: 12), recovery: nil,
            help: .init(message: "wrap it", replacement: .init(startByte: 6, endByte: 12, text: "world")))
        XCTAssertTrue(EditorDiagnostics.canApplyHelpReplacement(d, path: "main.tex", currentText: text,
                                                                compiledRevision: 1, editorRevision: 1))
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(d, path: "main.tex", currentText: text,
                                                                 compiledRevision: 1, editorRevision: 2),
                       "stale editor revision hides Fix…")
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(d, path: "chapter.tex", currentText: text,
                                                                 compiledRevision: 1, editorRevision: 1),
                       "other document hides Fix…")
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(d, path: "main.tex", currentText: "Hi",
                                                                 compiledRevision: 1, editorRevision: 1),
                       "range past the current text hides Fix…")
        let midScalar = RuntimeV1.Diagnostic(
            severity: .error, message: "m", source: .init(path: "main.tex", startByte: 7, endByte: 8), recovery: nil,
            help: .init(message: "x", replacement: .init(startByte: 7, endByte: 8, text: "")))
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(midScalar, path: "main.tex", currentText: text,
                                                                 compiledRevision: 1, editorRevision: 1),
                       "a range inside ö is not a valid slice")
        let advice = RuntimeV1.Diagnostic(
            severity: .error, message: "m", source: .init(path: "main.tex", startByte: 6, endByte: 12), recovery: nil,
            help: .init(message: "advice only"))
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(advice, path: "main.tex", currentText: text,
                                                                 compiledRevision: 1, editorRevision: 1))
        XCTAssertEqual(EditorDiagnostics.prepareHelpReplacement(advice, path: "main.tex", in: text, compiledText: text).failureValue, .noEdits)

        let preview = try EditorDiagnostics.prepareHelpReplacement(d, path: "main.tex", in: text, compiledText: text).get()
        XCTAssertEqual(preview.replacements.count, 1)
        XCTAssertEqual(preview.grouped.byteRange, 6..<12)
        XCTAssertEqual(preview.grouped.before, "wörld")
        XCTAssertEqual(preview.grouped.text, "world")
        XCTAssertEqual(preview.grouped.applied(to: text), "Hello world end")
        XCTAssertEqual(preview.suggestionText, "wrap it")

        let withPath = RuntimeV1.Diagnostic(
            severity: .error, message: "m",
            source: .init(path: "main.tex", startByte: 6, endByte: 12), recovery: nil,
            help: .init(message: "wrap it", replacement: .init(startByte: 6, endByte: 12, text: "world", path: "main.tex")))
        XCTAssertTrue(EditorDiagnostics.canApplyHelpReplacement(withPath, path: "main.tex", currentText: text,
                                                                compiledRevision: 3, editorRevision: 3))
        let otherPath = RuntimeV1.Diagnostic(
            severity: .error, message: "m",
            source: .init(path: "main.tex", startByte: 6, endByte: 12), recovery: nil,
            help: .init(message: "wrap it", replacement: .init(startByte: 6, endByte: 12, text: "world", path: "other.tex")))
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(otherPath, path: "main.tex", currentText: text,
                                                                 compiledRevision: 1, editorRevision: 1))
        XCTAssertEqual(EditorDiagnostics.prepareHelpReplacement(otherPath, path: "main.tex", in: text, compiledText: text).failureValue,
                       .otherDocument(path: "other.tex"))
    }
}

private extension Result {
    var failureValue: Failure? { if case .failure(let f) = self { return f }; return nil }
}
