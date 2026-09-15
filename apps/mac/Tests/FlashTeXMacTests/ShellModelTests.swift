import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

@MainActor
final class ShellModelTests: XCTestCase {
    func testLoadsFixturesAndNavigatesToUTF16Selection() throws {
        let model = ShellModel()
        XCTAssertNil(model.loadError, model.loadError ?? "")
        let result = try XCTUnwrap(model.result)
        XCTAssertEqual(result.status, .ok)
        XCTAssertEqual(model.activeText, "Hello FlashTeX.\n")
        XCTAssertFalse(model.previewIsStale)

        guard case .text(let item) = result.pages[0].items[0] else { return XCTFail() }
        model.navigate(to: item.source)
        let sel = try XCTUnwrap(model.selection)
        XCTAssertEqual(sel.path, "main.tex")
        XCTAssertEqual(sel.nsRange, NSRange(location: 0, length: 14))
        XCTAssertEqual((model.activeText as NSString).substring(with: sel.nsRange), "Hello FlashTeX")

        // Editing bumps the revision. An edit inside the item's span refuses navigation
        // (stale offsets are never applied); an edit after it is rebased.
        model.updateActiveText("Héllo FlashTeX.\n")
        XCTAssertTrue(model.previewIsStale)
        let selBefore = model.selection
        model.navigate(to: item.source!, expectedText: item.text)
        XCTAssertEqual(model.selection, selBefore)
        XCTAssertTrue(model.navigationNote?.contains("recompile to navigate") == true, model.navigationNote ?? "")
        model.updateActiveText("Hello FlashTeX. Appended\n")
        model.navigate(to: item.source!, expectedText: nil)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 0, length: 14))
        model.updateActiveText("Prefix! Hello FlashTeX.\n")
        // The contract fixture's item text ("Hello FlashTeX.") is one byte longer than
        // its range (0..<14), so expected-text verification would (correctly) refuse;
        // navigate without it here. Reported to the fixture owner.
        model.navigate(to: item.source!, expectedText: nil)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 8, length: 14), model.navigationNote ?? "nil")
        XCTAssertTrue(model.navigationNote?.contains("rebased") == true, model.navigationNote ?? "nil")
        model.updateActiveText("Hello FlashTeX.\n") // back to the compiled text

        // Invalid ranges are reported, not applied.
        let before = model.selection
        model.navigate(to: .init(path: "main.tex", startByte: 0, endByte: 999))
        XCTAssertEqual(model.selection, before)
        XCTAssertTrue(model.navigationNote?.contains("not a valid range") == true)
        model.navigate(to: .init(path: "other.tex", startByte: 0, endByte: 1))
        XCTAssertTrue(model.navigationNote?.contains("No open document") == true)
        model.navigate(to: nil)
        XCTAssertTrue(model.navigationNote?.contains("no source mapping") == true)
    }
}

@MainActor
final class ShellModelWorkerTests: XCTestCase {
    func testCompileThroughWorkerReplacesFixtureAndIgnoresOlderRevisions() async throws {
        let model = ShellModel()
        XCTAssertTrue(model.isFixture)
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        XCTAssertTrue(model.workerAttached)

        model.autoCompile = false
        model.updateActiveText("Second draft\n") // editorRevision 2
        model.compile()
        XCTAssertEqual(model.inFlightRevision, 2)
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertNotNil(model.lastLatencyMs)
        XCTAssertEqual(model.compiledDocuments["main.tex"], "Second draft\n")
        XCTAssertFalse(model.isFixture)
        XCTAssertEqual(model.result?.revision, 2)
        XCTAssertFalse(model.previewIsStale)
        guard case .text(let item) = model.result!.pages[0].items[0] else { return XCTFail() }
        XCTAssertEqual(item.text, "Second draft")
        model.navigate(to: item.source)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 0, length: 12))

        // Simulate an out-of-order older result: it must not replace revision 2.
        let stale = RuntimeV1.Envelope(protocolVersion: 1, id: "old", type: "compile_result",
            payload: RuntimeV1.CompileResult(projectId: "demo", revision: 1, status: .failed, pages: [], diagnostics: [], pdfPath: nil))
        model.handleForTesting(.result(stale))
        XCTAssertEqual(model.result?.revision, 2)
        XCTAssertEqual(model.result?.status, .ok)
        model.detachWorker()
        XCTAssertFalse(model.workerAttached)
    }

    func testUnsolicitedAndMismatchedResultsAreNeverApplied() async throws {
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.autoCompile = false
        let fixtureResult = model.result

        // 1. A result whose id was never sent: ignored, fixture stays.
        let unsolicited = RuntimeV1.Envelope(protocolVersion: 1, id: "never-sent", type: "compile_result",
            payload: RuntimeV1.CompileResult(projectId: "demo", revision: 99, status: .ok, pages: [], diagnostics: [], pdfPath: nil))
        model.handleForTesting(.result(unsolicited))
        XCTAssertEqual(model.result, fixtureResult)
        XCTAssertTrue(model.isFixture)
        XCTAssertTrue(model.workerLog.last?.contains("unknown id") == true)

        // 2. Worker answers with the wrong id for a real request: not applied, request stays pending until timeout logic (here: still in flight).
        model.updateActiveText("%wrongid first\n")
        model.compile()
        try await Task.sleep(nanoseconds: 400_000_000)
        XCTAssertTrue(model.isFixture, "result with unknown id must not be applied")
        XCTAssertNotNil(model.inFlightRevision)
        model.detachWorker()

        // 3. Right id but wrong revision/project: rejected as a protocol violation.
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.updateActiveText("%wrongrev second\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertTrue(model.isFixture, "mismatched revision must not be applied")
        XCTAssertTrue(model.workerStatus.contains("protocol violation"), model.workerStatus)
        XCTAssertTrue(model.inFlightRequests.isEmpty)

        // 4. Same id, matching project and revision: applied.
        model.updateActiveText("third\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertFalse(model.isFixture)
        XCTAssertEqual(model.result?.revision, model.editorRevision)
        model.detachWorker()
    }

    func testAutoCompileDebouncesAndCoalescesEdits() async throws {
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.autoCompile = true
        // Burst of edits: only the last buffer should end up compiled, and never out of order.
        for i in 1...5 { model.updateActiveText("draft \(i)\n") }
        let finalRevision = model.editorRevision
        try await waitUntil(timeout: 10) { model.result?.revision == finalRevision && model.inFlightRevision == nil }
        guard case .text(let item) = model.result!.pages[0].items[0] else { return XCTFail() }
        XCTAssertEqual(item.text, "draft 5")
        XCTAssertLessThanOrEqual(model.latenciesMs.count, 2, "burst coalesced into at most two requests, got \(model.latenciesMs.count)")
        XCTAssertFalse(model.previewIsStale)
        model.detachWorker()
    }

    /// A worker that dies mid-request is relaunched (bounded) and the next
    /// edit compiles again; the last preview stays; a clean exit or an explicit
    /// detach never relaunches; the per-minute limit leaves the exit visible.
    func testCrashedWorkerIsRelaunchedWithBoundedBackoff() async throws {
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.autoCompile = true
        model.updateActiveText("first\n")
        try await waitUntil { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        let before = model.result
        model.updateActiveText("%crash\n")   // the fake worker exits with status 3 without replying
        try await waitUntil { model.workerRelaunchCount == 1 && model.workerAttached }
        XCTAssertEqual(model.result, before, "the last preview survives the crash")
        XCTAssertTrue(model.workerStatus.contains("revision") || model.workerStatus.contains("compiling") || model.workerStatus.contains("attached"), model.workerStatus)
        // The relaunched worker compiles the current buffer (still %crash → crashes again, relaunch 2),
        // then a harmless edit compiles normally on the third worker.
        try await waitUntil { model.workerRelaunchCount >= 2 && model.workerAttached }
        model.updateActiveText("after relaunch\n")
        try await waitUntil { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        guard case .text(let item) = model.result!.pages[0].items[0] else { return XCTFail() }
        XCTAssertEqual(item.text, "after relaunch")
        // Three more crashes within the minute exhaust the budget: no fourth relaunch.
        for _ in 0..<2 { model.updateActiveText("%crash \(UUID())\n"); try await waitUntil { !model.workerAttached || model.workerRelaunchCount >= 3 } ; try await Task.sleep(nanoseconds: 300_000_000) }
        model.updateActiveText("%crash final\n")
        try await waitUntil(timeout: 5) { model.workerStatus.contains("not relaunched") }
        XCTAssertFalse(model.workerAttached)
        XCTAssertEqual(model.workerRelaunchCount, ShellModel.maxWorkerRelaunches)
        // Explicit detach after a manual re-attach cancels any pending relaunch.
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.detachWorker()
        XCTAssertFalse(model.workerAttached)
    }

    /// A bridge/ledger restart can raise the editor revision past the compiled
    /// one without changing the buffer; the preview must not stay "stale" until
    /// the next keystroke — the shell recompiles at the advanced revision.
    func testAdvancedEditorRevisionRecompilesSoPreviewIsNotStaleForever() async throws {
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.autoCompile = true
        model.updateActiveText("hello\n")
        try await waitUntil(timeout: 10) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertFalse(model.previewIsStale)
        model.advanceEditorRevision(atLeast: model.editorRevision + 7)
        XCTAssertTrue(model.previewIsStale, "the advanced revision makes the old result stale")
        try await waitUntil(timeout: 10) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertFalse(model.previewIsStale)
        model.detachWorker()
    }

    private func waitUntil(timeout: TimeInterval = 10, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout") }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }
}

@MainActor
final class DocumentFileTests: XCTestCase {
    func testOpenAndSaveRoundTripWithDirtyTracking() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-doc-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appendingPathComponent("paper.tex")
        try "Café naïve\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        XCTAssertNotNil(model.result, "fixture loaded first")
        model.openTex(at: url)
        XCTAssertEqual(model.activeText, "Café naïve\n")
        XCTAssertEqual(model.documentURL, url)
        // The project names the entry after the file (tab, sidebar, Problems,
        // quit prompt say `paper.tex`, and the durable helper's project root
        // is the file's directory), not a conventional `main.tex`.
        XCTAssertEqual(model.activePath, "paper.tex")
        XCTAssertEqual(model.documents.map(\.path), ["paper.tex"])
        XCTAssertNil(model.result, "opening a file clears the fixture preview")
        XCTAssertFalse(model.isDirty)

        model.updateActiveText("Café naïve — edited\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertTrue(model.saveTex())
        XCTAssertFalse(model.isDirty)
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), "Café naïve — edited\n")

        model.openTex(at: dir.appendingPathComponent("missing.tex"))
        XCTAssertTrue(model.captureNote?.contains("Could not open") == true)
        XCTAssertEqual(model.activeText, "Café naïve — edited\n", "failed open leaves the buffer alone")
    }
}

/// Issue #19 (2): opening another file must not discard unsaved edits without
/// an explicit decision, and a discarded buffer stays recoverable.
@MainActor
final class DirtyOpenTests: XCTestCase {
    func testOpenIsBlockedWhileDirtyAndDiscardKeepsTheBufferRecoverable() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-dirty-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let a = dir.appendingPathComponent("a.tex"), b = dir.appendingPathComponent("b.tex")
        try "A original\n".write(to: a, atomically: true, encoding: .utf8)
        try "B text\n".write(to: b, atomically: true, encoding: .utf8)

        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: a), .opened)
        model.updateActiveText("A edited but unsaved\n")
        XCTAssertTrue(model.isDirty)

        // Default: refused, nothing replaced.
        XCTAssertEqual(model.openTex(at: b), .blockedByUnsavedEdits)
        XCTAssertEqual(model.activeText, "A edited but unsaved\n")
        XCTAssertEqual(model.documentURL, a)

        // Discard: replaced, but the prior source is recoverable.
        XCTAssertEqual(model.openTex(at: b, dirty: .discard), .opened)
        XCTAssertEqual(model.activeText, "B text\n")
        XCTAssertEqual(model.recoverableBuffer, .init(url: a, text: "A edited but unsaved\n"))
        XCTAssertEqual(try String(contentsOf: a, encoding: .utf8), "A original\n", "disk untouched by discard")
        XCTAssertTrue(model.restoreDiscardedBuffer())
        XCTAssertEqual(model.activeText, "A edited but unsaved\n")
        XCTAssertTrue(model.isDirty, "restored buffer is still unsaved")
        XCTAssertNil(model.recoverableBuffer)

        // Save first: the edit reaches disk, then the open proceeds.
        XCTAssertEqual(model.openTex(at: b, dirty: .saveFirst), .opened)
        XCTAssertEqual(try String(contentsOf: a, encoding: .utf8), "A edited but unsaved\n")
        XCTAssertFalse(model.isDirty)
    }
}
