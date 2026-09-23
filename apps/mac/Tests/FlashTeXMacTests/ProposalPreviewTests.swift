import SwiftUI
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// Shadow-compile preview of a capture proposal (ProposalPreview.swift).
/// Worker-backed cases use `Fixtures/fake_worker.py` (`%diag:<n>` emits an
/// error diagnostic `n` bytes after the directive).
@MainActor
final class ProposalPreviewTests: XCTestCase {

    private func input(_ text: String, anchorByte: Int, revision: Int = 1, anchorRevision: Int? = nil,
                       context: String? = nil) -> ProposalPreview.Input {
        let ctx = context ?? String(decoding: Array(text.utf8.dropFirst(anchorByte).prefix(Insertion.contextLength)), as: UTF8.self)
        return .init(documents: [.init(path: "main.tex", text: text)], entryPath: "main.tex",
                     anchor: InsertionAnchor(id: "a1", path: "main.tex", byteOffset: anchorByte,
                                             revision: anchorRevision ?? revision, contextAfter: ctx),
                     editorRevision: revision, projectId: "demo")
    }

    private func makePreview() -> ProposalPreview {
        ProposalPreview(executable: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
    }

    private func waitForReady(_ preview: ProposalPreview) async throws {
        try await waitUntil("preview ready") { if case .ready = preview.state { return true }; return false }
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    // MARK: shadow text

    func testShadowAppliesInsertionWithNewlineRules() throws {
        // Mid-line anchor: newline added on both sides, matching approval.
        let mid = try ProposalPreview.makeShadow(input: input("abc def\n", anchorByte: 3), latex: "  X  \n").get()
        XCTAssertEqual(mid.shadowText, "abc\nX\n def\n")
        XCTAssertEqual(mid.insertedText, "\nX\n")
        XCTAssertEqual(mid.insertedRange, 3..<6)
        XCTAssertEqual(mid.bodyStart, 4)
        // Line start with text after: only a trailing newline.
        let start = try ProposalPreview.makeShadow(input: input("abc\ndef\n", anchorByte: 4), latex: "X").get()
        XCTAssertEqual(start.shadowText, "abc\nX\ndef\n")
        XCTAssertEqual(start.bodyStart, 4)
        // End of buffer after a newline: nothing added.
        let end = try ProposalPreview.makeShadow(input: input("abc\n", anchorByte: 4), latex: "X").get()
        XCTAssertEqual(end.shadowText, "abc\nX")
        // Same text as `Insertion.insertionText` would produce for approval.
        XCTAssertEqual(end.insertedText, Insertion.insertionText("X", into: "abc\n", atByte: 4))
        // Other documents are untouched and the original input is never mutated.
        var multi = input("abc\n", anchorByte: 0)
        multi.documents.append(.init(path: "other.tex", text: "keep"))
        let shadow = try ProposalPreview.makeShadow(input: multi, latex: "X").get()
        XCTAssertEqual(shadow.documents.map(\.text), ["X\nabc\n", "keep"])
        XCTAssertEqual(multi.documents.map(\.text), ["abc\n", "keep"])
    }

    func testShadowRebasesOrRefusesStaleAnchors() {
        // Anchor pinned at revision 1; buffer now at revision 3 with a prefix: rebased via context.
        let rebased = ProposalPreview.makeShadow(
            input: input("PRE abc\ndef\n", anchorByte: 4, revision: 3, anchorRevision: 1, context: "abc\ndef\n"), latex: "X")
        XCTAssertEqual(try rebased.get().insertByte, 4)
        // Context gone: refused, never guessed.
        let gone = ProposalPreview.makeShadow(
            input: input("zzz\n", anchorByte: 0, revision: 3, anchorRevision: 1, context: "abc"), latex: "X")
        guard case .failure(let why) = gone else { return XCTFail("expected refusal") }
        XCTAssertTrue(why.message.contains("deleted or changed"), why.message)
        // No anchor / empty proposal are not previewable.
        var noAnchor = input("abc\n", anchorByte: 0); noAnchor.anchor = nil
        XCTAssertEqual(ProposalPreview.makeShadow(input: noAnchor, latex: "X"), .failure(.init(message: "no insertion point pinned")))
        XCTAssertEqual(ProposalPreview.makeShadow(input: input("abc\n", anchorByte: 0), latex: "  \n"),
                       .failure(.init(message: "proposal is empty")))
    }

    // MARK: diffing (pure)

    func testNeighborhoodCoversEnclosingLinesPlusOneEitherSide() {
        let text = "l0\nl1\nl2\nl3\nl4\n" // each line 3 bytes
        XCTAssertEqual(ProposalPreview.neighborhood(of: 6..<8, in: text), 3..<11)   // l1..l3
        XCTAssertEqual(ProposalPreview.neighborhood(of: 0..<1, in: text), 0..<5)    // l0..l1
        XCTAssertEqual(ProposalPreview.neighborhood(of: 14..<15, in: text), 9..<15) // l3..end
        XCTAssertEqual(ProposalPreview.neighborhood(of: 2..<10, in: "abc"), 0..<3)  // clamped
    }

    func testNewDiagnosticsShiftRangesAfterTheInsertion() throws {
        let shadow = try ProposalPreview.makeShadow(input: input("A\n%diag:1 tail\n", anchorByte: 2), latex: "%diag:0 new").get()
        XCTAssertEqual(shadow.shadowText, "A\n%diag:0 new\n%diag:1 tail\n")
        func d(_ msg: String, _ at: Int, _ sev: RuntimeV1.Severity = .error) -> RuntimeV1.Diagnostic {
            .init(severity: sev, message: msg, source: .init(path: "main.tex", startByte: at, endByte: at + 1), recovery: nil)
        }
        let baseline = [d("fake diagnostic 1", 3), RuntimeV1.Diagnostic(severity: .warning, message: "global", source: nil, recovery: nil),
                        d("far", 100)]
        let after = [d("fake diagnostic 0", 2), d("fake diagnostic 1", 15),
                     RuntimeV1.Diagnostic(severity: .warning, message: "global", source: nil, recovery: nil),
                     RuntimeV1.Diagnostic(severity: .warning, message: "another", source: nil, recovery: nil),
                     d("far", 112)] // 100 shifted by the 12 inserted bytes
        let fresh = ProposalPreview.newDiagnostics(baseline: baseline, shadow: after, shadow: shadow)
        XCTAssertEqual(fresh.map(\.message), ["fake diagnostic 0", "another"])
        // Same message at an unshifted (wrong) offset after the insertion is new, not matched.
        let wrong = ProposalPreview.newDiagnostics(baseline: baseline, shadow: [d("fake diagnostic 1", 3)], shadow: shadow)
        XCTAssertEqual(wrong.count, 1)
        // Duplicates are matched as a multiset.
        let dup = ProposalPreview.newDiagnostics(baseline: [d("m", 0)], shadow: [d("m", 0), d("m", 0)], shadow: shadow)
        XCTAssertEqual(dup.count, 1)

        let result = RuntimeV1.CompileResult(projectId: "demo-preview", revision: 2, status: .recovered,
                                             pages: [], diagnostics: after, pdfPath: nil)
        let base = RuntimeV1.CompileResult(projectId: "demo-preview", revision: 1, status: .ok,
                                           pages: [.init(number: 1, widthPt: 1, heightPt: 1, items: [])], diagnostics: baseline, pdfPath: nil)
        let report = ProposalPreview.report(shadow: shadow, result: result, baseline: base)
        XCTAssertEqual(report.pageDelta, -1)
        XCTAssertEqual(report.new.map(\.diagnostic.message), ["fake diagnostic 0", "another"])
        let firstNew = try XCTUnwrap(report.new.first)
        XCTAssertEqual(firstNew.proposalOffset, 0)
        XCTAssertEqual(firstNew.fragment, "A⏎%diag:0 new⏎%")
        XCTAssertEqual(report.newErrorCount, 1)
        // Nearby: both located diagnostics (line 2 is adjacent) plus the unlocated new one; "far" and "global" are unrelated.
        XCTAssertEqual(report.nearby.map(\.diagnostic.message), ["fake diagnostic 0", "fake diagnostic 1", "another"])
        XCTAssertEqual(report.unrelatedCount, 2)
    }

    func testInsertionPageAndThumbnailComeFromTheShadowResult() throws {
        let shadow = try ProposalPreview.makeShadow(input: input("one\ntwo\n", anchorByte: 4), latex: "X").get()
        XCTAssertEqual(shadow.shadowText, "one\nX\ntwo\n")
        func item(_ text: String, _ start: Int, _ end: Int) -> RuntimeV1.PageItem {
            .text(.init(text: text, xPt: 72, baselineYPt: 84, fontSizePt: 12,
                        source: .init(path: "main.tex", startByte: start, endByte: end)))
        }
        let result = RuntimeV1.CompileResult(projectId: "demo-preview", revision: 1, status: .ok, pages: [
            .init(number: 1, widthPt: 612, heightPt: 792, items: [item("one", 0, 3)]),
            .init(number: 2, widthPt: 612, heightPt: 792, items: [item("X", 4, 5), item("two", 6, 9)]),
        ], diagnostics: [], pdfPath: nil)
        XCTAssertEqual(ProposalPreview.insertionPage(in: result, shadow: shadow), 2)
        let report = ProposalPreview.report(shadow: shadow, result: result, baseline: nil)
        XCTAssertEqual(report.insertionPage, 2)
        XCTAssertNil(report.pageDelta)
        let image = try XCTUnwrap(ProposalPreview.thumbnail(of: 2, in: result, width: 90))
        XCTAssertEqual(image.size.width, 90, accuracy: 1)
        XCTAssertEqual(image.size.height, 90 * 792 / 612, accuracy: 1)
        XCTAssertNil(ProposalPreview.thumbnail(of: 3, in: result))
        // No item maps the insertion (e.g. a failed compile with no pages).
        let empty = RuntimeV1.CompileResult(projectId: "demo-preview", revision: 1, status: .failed, pages: [], diagnostics: [], pdfPath: nil)
        XCTAssertNil(ProposalPreview.insertionPage(in: empty, shadow: shadow))
    }

    // MARK: worker-backed

    func testCompilesShadowAndBaselineAndReportsOnlyInsertedDiagnosticAsNew() async throws {
        let preview = makePreview()
        XCTAssertEqual(preview.state, .idle)
        preview.update(input: input("A\n%diag:1 tail\n", anchorByte: 2), latex: "%diag:0 new")
        XCTAssertEqual(preview.state, .compiling)
        try await waitUntil("preview ready") { if case .ready = preview.state { return true }; return false }
        guard case .ready(let r) = preview.state else { return XCTFail() }
        XCTAssertEqual(r.status, .ok)
        XCTAssertEqual(r.pageCount, 1)
        XCTAssertEqual(r.pageDelta, 0)
        XCTAssertEqual(r.new.map(\.diagnostic.message), ["fake diagnostic 0"])
        XCTAssertEqual(try XCTUnwrap(r.new.first).diagnostic.recovery, "test double: byte skipped")
        XCTAssertEqual(r.nearby.count, 2)
        XCTAssertNil(r.insertionPage, "the fake maps only the first line; the insertion is on line 2")
        XCTAssertNil(preview.thumbnail)
        XCTAssertEqual(preview.shadowCompileCount, 1)
        XCTAssertEqual(preview.baselineCompileCount, 1)
        XCTAssertTrue(preview.hasNewErrors)
        XCTAssertTrue(preview.statusText.hasPrefix("preview: ok, +0 pages, 1 new diagnostic"), preview.statusText)

        // Editing the proposal recompiles the shadow only; the baseline is reused.
        preview.update(input: input("A\n%diag:1 tail\n", anchorByte: 2), latex: "clean")
        try await waitUntil("second result") { preview.shadowCompileCount == 2 && !preview.isInFlight }
        guard case .ready(let r2) = preview.state else { return XCTFail("\(preview.state)") }
        XCTAssertTrue(r2.new.isEmpty)
        XCTAssertFalse(preview.hasNewErrors)
        XCTAssertEqual(preview.baselineCompileCount, 1)
        XCTAssertTrue(preview.workerIsRunning)
        preview.close()
        try await waitUntil("worker terminated") { !preview.workerIsRunning }
    }

    func testBurstOfEditsIsDebouncedAndCoalesced() async throws {
        let preview = makePreview()
        let doc = input("abc\n", anchorByte: 4)
        for i in 1...5 {
            preview.update(input: doc, latex: String(repeating: "x", count: i))
            try await Task.sleep(nanoseconds: 30_000_000)
        }
        XCTAssertEqual(preview.shadowCompileCount, 0, "nothing is sent inside the debounce window")
        try await waitUntil("ready") { if case .ready = preview.state { return true }; return false }
        try await Task.sleep(nanoseconds: 400_000_000) // let any (wrong) extra compiles surface
        XCTAssertEqual(preview.shadowCompileCount, 1)
        XCTAssertEqual(preview.baselineCompileCount, 1)
        XCTAssertLessThanOrEqual(preview.shadowCompileCount + preview.baselineCompileCount, 2)
        XCTAssertEqual(preview.shadow?.shadowText, "abc\nxxxxx")

        // Edits arriving while a compile is in flight (the fake delays `%slow`
        // documents) coalesce into exactly one more compile carrying the newest text.
        let slow = input("%slow\nabc\n", anchorByte: 10)
        preview.update(input: slow, latex: "first")
        try await waitUntil("in flight") { preview.isInFlight }
        XCTAssertEqual(preview.shadowCompileCount, 2)
        XCTAssertEqual(preview.baselineCompileCount, 2, "documents changed: one new baseline")
        preview.update(input: slow, latex: "second")
        preview.update(input: slow, latex: "third")
        try await waitUntil("third compiled") {
            preview.shadow?.shadowText == "%slow\nabc\nthird" && !preview.isInFlight && preview.debounceIdle
        }
        try await Task.sleep(nanoseconds: 200_000_000)
        XCTAssertEqual(preview.shadowCompileCount, 3)
        XCTAssertEqual(preview.baselineCompileCount, 2)
        guard case .ready = preview.state else { return XCTFail("\(preview.state)") }
        preview.close()
        try await waitUntil("worker terminated") { !preview.workerIsRunning }
    }

    func testNoCompilerGivesClearStatusAndNeverLaunches() {
        let preview = ProposalPreview(executable: nil)
        XCTAssertEqual(preview.state, .noCompiler)
        preview.update(input: input("abc\n", anchorByte: 0), latex: "X")
        XCTAssertEqual(preview.state, .noCompiler)
        XCTAssertEqual(preview.statusText, "no compiler attached — cannot preview")
        XCTAssertFalse(preview.workerIsRunning)
        XCTAssertEqual(preview.shadowCompileCount, 0)
    }

    func testAnchorlessProposalIsNotPreviewable() {
        let preview = makePreview()
        var noAnchor = input("abc\n", anchorByte: 0); noAnchor.anchor = nil
        preview.update(input: noAnchor, latex: "X")
        XCTAssertEqual(preview.state, .notPreviewable("no insertion point pinned"))
        XCTAssertFalse(preview.workerIsRunning)
    }

    func testWorkerFaultIsReportedAndRetryRelaunches() async throws {
        let preview = makePreview()
        // `%trailing` makes the fake exit mid-line: a protocol violation, then exit.
        preview.update(input: input("%trailing\n", anchorByte: 10), latex: "X")
        try await waitUntil("failure") { if case .failed = preview.state { return true }; return false }
        try await waitUntil("worker gone") { !preview.workerIsRunning }
        XCTAssertTrue(preview.statusText.hasPrefix("preview failed: protocol violation"), preview.statusText)
        // Retry relaunches a worker and re-sends the same request (which faults again).
        preview.retry()
        XCTAssertEqual(preview.state, .compiling)
        XCTAssertEqual(preview.shadowCompileCount, 2)
        try await waitUntil("second failure") { if case .failed = preview.state { return true }; return false }
        try await waitUntil("worker gone again") { !preview.workerIsRunning }
        // A worker `error` envelope is reported too, and the worker stays usable.
        preview.update(input: input("%error\n", anchorByte: 7), latex: "X")
        try await waitUntil("error reported") { preview.statusText.hasPrefix("preview failed: worker error") }
        XCTAssertTrue(preview.workerIsRunning)
        // The next edit compiles normally on the live worker.
        preview.update(input: input("fine\n", anchorByte: 5), latex: "X")
        try await waitUntil("recovered") { if case .ready = preview.state { return true }; return false }
        XCTAssertTrue(preview.workerIsRunning)
        preview.close()
        try await waitUntil("worker terminated") { !preview.workerIsRunning }
    }

    func testRealModelResultAndRevisionAreUntouched() async throws {
        let model = ShellModel()
        XCTAssertNil(model.loadError, model.loadError ?? "")
        let fixture = model.result, revision = model.editorRevision, docs = model.documents
        model.caretUTF16 = 0
        model.pinAnchorAtCaret()
        XCTAssertNotNil(model.anchor)

        let preview = makePreview()
        preview.update(from: model, latex: "%diag:0 inserted")
        try await waitUntil("ready") { if case .ready = preview.state { return true }; return false }
        guard case .ready(let r) = preview.state else { return XCTFail() }
        XCTAssertEqual(r.new.count, 1)
        XCTAssertEqual(preview.shadow?.shadowText, "%diag:0 inserted\nHello FlashTeX.\n")
        XCTAssertEqual(r.insertionPage, 1, "the fake maps the first line, which the insertion now occupies")
        XCTAssertNotNil(preview.thumbnail)

        XCTAssertEqual(model.result, fixture)
        XCTAssertEqual(model.editorRevision, revision)
        XCTAssertEqual(model.documents, docs)
        XCTAssertTrue(model.isFixture)
        XCTAssertFalse(model.workerAttached)
        XCTAssertNil(model.pendingEdit)
        XCTAssertTrue(model.inFlightRequests.isEmpty)
        preview.close()
        try await waitUntil("worker terminated") { !preview.workerIsRunning }
    }

}
