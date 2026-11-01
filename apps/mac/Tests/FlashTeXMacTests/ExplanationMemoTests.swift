import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// Explanation memo keyed by (diagnostic, source sha) (ExplanationMemo.swift,
/// `ExplanationCache.store(_:for:result:documents:)` / `reuse`): an unchanged
/// diagnostic reported by a new compile result is answered from memory
/// without a helper request; any change in text, message, range, recovery or
/// severity misses and is fetched as before. The fetch-or-reuse decision is
/// driven here exactly as the `ShellModel.fetchExplanations` hook does it
/// (parent diff in coordination/mac-ai-review-2.md).
@MainActor
final class ExplanationMemoTests: XCTestCase {
    typealias Explanation = EditorDiagnostics.Explanation
    typealias Memo = EditorDiagnostics.ExplanationMemo

    private func diag(_ msg: String, _ at: Int, len: Int = 4, severity: RuntimeV1.Severity = .error, recovery: String? = "skipped",
                      path: String = "main.tex") -> RuntimeV1.Diagnostic {
        .init(severity: severity, message: msg, source: .init(path: path, startByte: at, endByte: at + len), recovery: recovery)
    }

    private func result(_ diagnostics: [RuntimeV1.Diagnostic], revision: Int = 1) -> RuntimeV1.CompileResult {
        .init(projectId: "demo", revision: revision, status: diagnostics.isEmpty ? .ok : .recovered, pages: [], diagnostics: diagnostics, pdfPath: nil)
    }

    private func explanation(_ title: String) -> Explanation {
        Explanation(catalogID: "unsupported-command", title: title, category: "unsupported-command", severity: "error", message: title,
                    why: "why \(title)", whatHappened: "skipped", suggestions: [], context: nil)
    }

    private func loadAverage() -> Double {
        var l = [Double](repeating: 0, count: 3)
        return getloadavg(&l, 3) >= 1 ? l[0] : -1
    }

    func testUnchangedDiagnosticsAreRecalledAndAnyChangeMisses() throws {
        let docs = [RuntimeV1.Document(path: "main.tex", text: "\\foo{bar} and \\baz\n"), .init(path: "ch.tex", text: "chapter\n")]
        let d0 = diag("\\foo is not supported", 0), d1 = diag("\\baz is not supported", 14), unlocated = RuntimeV1.Diagnostic(severity: .warning, message: "global", source: nil, recovery: nil)
        let r1 = result([d0, d1, unlocated])
        let list = [explanation("foo"), explanation("baz"), explanation("global")]
        var memo = Memo()
        XCTAssertNil(memo.recall(for: r1, documents: docs), "nothing remembered yet")
        XCTAssertEqual(memo.misses, 1)
        memo.remember(list, for: r1, documents: docs)
        XCTAssertEqual(memo.count, 3)

        // The same diagnostics from another result (new id, revision, order) for the same text: recalled in the new order.
        let r2 = result([unlocated, d1, d0], revision: 7)
        XCTAssertEqual(memo.recall(for: r2, documents: docs), [list[2], list[1], list[0]])
        XCTAssertEqual(memo.hits, 1)
        // A subset is recalled too; an empty result is trivially answered.
        XCTAssertEqual(memo.recall(for: result([d1]), documents: docs), [list[1]])
        XCTAssertEqual(memo.recall(for: result([]), documents: docs), [])

        // Text changed in the diagnostic's own document: miss (even with the same range and message).
        var edited = docs; edited[0].text = "\\foo{bar} and \\baz % edited\n"
        XCTAssertNil(memo.recall(for: result([d0]), documents: edited))
        // Text changed in ANOTHER document only: the located diagnostic still hits, the unlocated one misses.
        var other = docs; other[1].text = "chapter two\n"
        XCTAssertEqual(memo.recall(for: result([d0]), documents: other), [list[0]])
        XCTAssertNil(memo.recall(for: result([unlocated]), documents: other))
        // Message / range / recovery / severity changes miss.
        XCTAssertNil(memo.recall(for: result([diag("\\foo is not supported!", 0)]), documents: docs))
        XCTAssertNil(memo.recall(for: result([diag("\\foo is not supported", 1)]), documents: docs))
        XCTAssertNil(memo.recall(for: result([diag("\\foo is not supported", 0, len: 5)]), documents: docs))
        XCTAssertNil(memo.recall(for: result([diag("\\foo is not supported", 0, recovery: nil)]), documents: docs))
        XCTAssertNil(memo.recall(for: result([diag("\\foo is not supported", 0, severity: .warning)]), documents: docs))
        // A diagnostic into a document whose text is unknown can never be reused.
        XCTAssertNil(memo.recall(for: result([diag("x", 0, path: "missing.tex")]), documents: docs))
        XCTAssertNil(Memo.keys(for: result([diag("x", 0, path: "missing.tex")]), documents: docs))
        // One unknown diagnostic among known ones: the whole result is fetched (the helper explains all at once).
        XCTAssertNil(memo.recall(for: result([d0, diag("new", 3)]), documents: docs))
        // A short reply remembers only what it explained; a re-remember replaces.
        memo.remember([explanation("foo2")], for: result([d0, d1]), documents: docs)
        XCTAssertEqual(memo.recall(for: result([d0]), documents: docs)?.first?.title, "foo2")
        XCTAssertEqual(memo.count, 3)

        // Bounded: the oldest keys go first.
        var small = Memo(capacity: 3)
        for i in 0..<5 { small.remember([explanation("e\(i)")], for: result([diag("m\(i)", i)]), documents: docs) }
        XCTAssertEqual(small.count, 3)
        XCTAssertNil(small.recall(for: result([diag("m0", 0)]), documents: docs))
        XCTAssertNil(small.recall(for: result([diag("m1", 1)]), documents: docs))
        XCTAssertEqual(small.recall(for: result([diag("m4", 4)]), documents: docs)?.first?.title, "e4")
    }

    /// The cache's fetch-or-reuse decision as the model hook makes it: the
    /// first result is fetched (decoded once from the crate-shaped reply), a
    /// later result with the same diagnostics for the same text is filled
    /// from the memo with no helper request, and the per-result eviction
    /// never forgets what was explained.
    func testReuseFillsANewResultWithoutAFetchAndSurvivesPerResultEviction() throws {
        let text = try String(contentsOf: EditorDiagnosticsExplanationsTests.samples.appendingPathComponent("recovery-demo.tex"), encoding: .utf8)
        let docs = [RuntimeV1.Document(path: "main.tex", text: text)]
        let foo = EditorDiagnosticsExplanationsTests.fooDiagnostic
        let decoded = try EditorDiagnostics.decodeExplanations(Data(EditorDiagnosticsExplanationsTests.realReplyLine.utf8), expectedCount: 1)
        var cache = EditorDiagnostics.ExplanationCache(maxResults: 2)
        var helperRequests = 0
        func compileArrived(_ id: String, _ res: RuntimeV1.CompileResult, _ documents: [RuntimeV1.Document]) {
            // ShellModel.fetchExplanations with the hook applied:
            guard cache[id] == nil else { return }
            if cache.reuse(for: id, result: res, documents: documents) { return }
            helperRequests += 1 // the client would run here; its reply is stored with the result
            cache.store(decoded, for: id, result: res, documents: documents)
        }
        let r1 = result([foo], revision: 1)
        compileArrived("r1", r1, docs)
        XCTAssertEqual(helperRequests, 1)
        XCTAssertEqual(cache["r1"], decoded)
        // Recompile (an undo, a reopen, an edit elsewhere that left this text): new id, no fetch.
        compileArrived("r2", result([foo], revision: 2), docs)
        XCTAssertEqual(helperRequests, 1, "answered from the memo")
        XCTAssertEqual(cache["r2"], decoded)
        XCTAssertEqual(cache.memo.hits, 1)
        // Attached to marks exactly as a fetched reply would be.
        let report = EditorDiagnostics.attach(cache["r2"], to: EditorDiagnostics.report(for: r1, resultID: "r2", path: "main.tex", compiledText: text, currentText: text))
        XCTAssertEqual(report.marks.map(\.explanation), [decoded[0].line])
        XCTAssertEqual(cache.explanation(resultID: "r2", index: 0)?.suggestions.first?.edits.first?.replacement, "bar", "quick fixes come from the recalled explanation")
        // The text changed under the diagnostic: fetched again.
        var edited = docs; edited[0].text = text + "% trailing edit\n"
        compileArrived("r3", result([foo], revision: 3), edited)
        XCTAssertEqual(helperRequests, 2)
        // Per-result entries evict (maxResults 2: r1 is gone), the memo does not.
        XCTAssertNil(cache["r1"])
        compileArrived("r4", result([foo], revision: 4), docs)
        XCTAssertEqual(helperRequests, 2, "the original text's explanation is still remembered")
        XCTAssertEqual(cache["r4"], decoded)
        XCTAssertTrue(cache.reuse(for: "r4", result: r1, documents: docs), "an already-stored result is left alone")
        XCTAssertEqual(cache.memo.count, 2, "one key per (diagnostic, text)")
    }

    /// Re-explaining an unchanged diagnostic costs one document digest plus
    /// dictionary lookups: 62 KB document, 200 diagnostics. Bound 5 ms on the
    /// best of 25 samples (load-aware; the digest dominates).
    func testRecallOfTwoHundredDiagnosticsIsInstant() throws {
        let load = loadAverage()
        let line = "\\textbf{Wörter} und $x_i^2$ auf Zeile mit Ünicode und Text.\n"
        var text = ""
        for _ in 0..<1000 { text += line }
        let docs = [RuntimeV1.Document(path: "main.tex", text: text)]
        var diags: [RuntimeV1.Diagnostic] = []
        for i in 0..<200 { diags.append(diag("\\cmd\(i) is not supported by this compiler version", i * 5 * line.utf8.count, len: 15)) }
        let res = result(diags)
        var cache = EditorDiagnostics.ExplanationCache()
        cache.store(diags.enumerated().map { explanation("cmd\($0.offset)") }, for: "first", result: res, documents: docs)
        var samples: [Double] = []
        for i in 0..<25 {
            let again = result(diags, revision: i + 2)
            let docCopy = [RuntimeV1.Document(path: "main.tex", text: String(decoding: Array(text.utf8), as: UTF8.self))]
            let t0 = DispatchTime.now().uptimeNanoseconds
            let reused = cache.reuse(for: "again-\(i)", result: again, documents: docCopy)
            samples.append(Double(DispatchTime.now().uptimeNanoseconds - t0) / 1e6)
            XCTAssertTrue(reused)
        }
        XCTAssertEqual(cache.explanation(resultID: "again-24", index: 150)?.title, "cmd150")
        XCTAssertEqual(cache.memo.hits, 25)
        let sorted = samples.sorted()
        print(String(format: "ExplanationMemo recall bench: %d bytes, %d diagnostics: best %.3f ms, median %.3f ms, worst %.3f ms; 1-min load %.1f",
                     text.utf8.count, diags.count, sorted[0], sorted[sorted.count / 2], sorted[sorted.count - 1], load))
        if load > 20 { throw XCTSkip("1-min load \(load) > 20; timing bound skipped (measured best \(sorted[0]) ms)") }
        XCTAssertLessThan(sorted[0], 5.0, "re-explaining an unchanged diagnostic must be instant")
    }
}
