import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Adversarial review of the edit → durable → preview pipeline through the
/// real `flashtex-preview-controller` (lane mac-core-review). Each test is
/// a reproduced defect; the assertions are explicit failures, never skips,
/// once the helper is available.
@MainActor
final class ControllerPipelineReviewTests: XCTestCase {
    static var helper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }

    /// A compiler that answers every request through the real compiler until
    /// one carries `marker`, on which it dies: the helper reports that compile
    /// as `update {kind:"failed", reason:"compiler output closed"}` and its
    /// compiler session is gone (crates/document-runtime `fail`;
    /// crates/preview-controller/src/main.rs).
    private func writeDyingCompiler(in dir: URL, real: URL, marker: String) throws -> URL {
        let script = dir.appendingPathComponent("dying-compiler.sh")
        let text = """
        #!/bin/sh
        while IFS= read -r line; do
          case "$line" in *\(marker)*) exit 3;; esac
          printf '%s\\n' "$line" | "\(real.path)"
        done
        exit 0

        """
        try text.write(to: script, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
        return script
    }

    /// Finding 1: under the default hold-until-preview policy the in-flight edit
    /// is released only by a `preview` for its durable revision or by a
    /// `stale`/`discarded` update whose `compile_revision` passes a numeric
    /// guard. The helper reports a compile that failed (compiler crash) as
    /// `update {kind:"failed", request_id, reason}`, which the shell ignored:
    /// the edit stayed in flight forever, every later keystroke was queued and
    /// never sent (typing stall until relaunch), and the buffer stopped being
    /// made durable.
    func testCompilerFailureReleasesTheInFlightEditSoTypingStaysDurable() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              let realCompiler = ShellModel.locateCompiler() else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        XCTAssertEqual(ControllerReleasePolicy.fromEnvironment(), .holdUntilPreview, "this reproduction is for the default policy")
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-review-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        try "\\begin{document}\nHello failing compiler.\n\\end{document}\n".write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let dying = try writeDyingCompiler(in: root, real: realCompiler, marker: "KILLCOMPILER")
        let originalCompiler = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"]
        setenv("FLASHTEX_COMPILER", dying.path, 1)
        defer { if let originalCompiler { setenv("FLASHTEX_COMPILER", originalCompiler, 1) } else { unsetenv("FLASHTEX_COMPILER") } }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        defer { model.detachController() }
        // First compile goes through the real compiler (via the wrapper): a current preview.
        let ok1 = await settles(15) { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") && model.inFlightRevision == nil }
        XCTAssertTrue(ok1,
                      "initial preview never arrived: \(model.controllerStatus) / \(model.workerStatus)")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1)
        let initial = model.editorRevision

        // The edit is durable (r2); its compile carries the marker: the compiler dies.
        model.updateActiveText("\\begin{document}\nHello failing compiler, KILLCOMPILER.\n\\end{document}\n")
        let edited = model.editorRevision
        let ok2 = await settles(10) { model.controllerState.durable["main.tex"]?.revision == 2 }
        XCTAssertTrue(ok2,
                      "edit never became durable: \(model.controllerStatus)")
        // The helper reports the failed compile; the pipeline must be released.
        let ok3 = await settles(10) { model.controllerState.inFlight == nil && model.inFlightRevision == nil }
        XCTAssertTrue(ok3,
                      "in-flight edit for revision \(edited) never released after the compiler failure: \(model.controllerStatus); log tail: \(model.workerLog.suffix(6))")
        XCTAssertTrue(model.workerLog.contains { $0.contains("failed") && $0.contains("compiler output closed") }, "the failure is logged: \(model.workerLog.suffix(6))")
        XCTAssertTrue(model.controllerStatus.contains("compiler output closed"), "the failure is visible: \(model.controllerStatus)")

        // Typing continues: each later edit is sent and made durable (the helper
        // answers with a separate preview_error since its compiler session is gone).
        model.updateActiveText("\\begin{document}\nHello failing compiler, edited twice.\n\\end{document}\n")
        let ok4 = await settles(10) { model.controllerState.durable["main.tex"]?.revision == 3 && model.inFlightRevision == nil }
        XCTAssertTrue(ok4,
                      "typing after the failure stalled: durable \(String(describing: model.controllerState.durable["main.tex"]?.revision)), inFlight \(String(describing: model.inFlightRevision)), \(model.controllerStatus)")
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[3], model.activeText)
        // The last good preview is kept; nothing older or newer was invented.
        XCTAssertEqual(model.result?.revision, initial)
    }

    /// Finding 2: `openTex` names the entry document `main.tex` whatever the
    /// file is called, and `attachController` therefore roots the helper in a
    /// SESSION TEMPORARY project (a copy of the buffer) whenever the file name
    /// differs. `saveTexInteractive` routed the save through the helper on
    /// `controllerAttached` alone, so the helper's rooted export wrote the
    /// temporary copy, the shell reported "Saved paper.tex" and marked the
    /// buffer clean, and the real file was never written. The file-routing
    /// predicate the reload/status paths use (`controllerRoutesFiles`) already
    /// refuses this case; the save path must use it too.
    ///
    /// Since the entry document took the opened file's own name
    /// (`replaceProject(entryText:entryPath:)`), `paper.tex` is a rooted
    /// project like `main.tex`; the guarantee under test is unchanged — the
    /// file on disk holds the buffer after the save — now through the
    /// helper's rooted export instead of the direct writer.
    func testSaveOfAFileNotNamedMainTexWritesThatFileNotTheHelpersSessionCopy() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-review-save-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/paper.tex")
        let original = "\\begin{document}\nA paper.\n\\end{document}\n"
        try original.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        XCTAssertEqual(model.activePath, "paper.tex", "the entry document is named after the file")
        model.attachController(at: helper)
        defer { model.detachController() }
        let ready = await settles(15) { model.result?.revision == model.editorRevision && model.controllerState.durable["paper.tex"] != nil && model.inFlightRevision == nil }
        XCTAssertTrue(ready, "initial preview never arrived: \(model.controllerStatus)")
        // The entry and the file agree on the name, so the helper's project is
        // rooted at the file's directory and the save is its rooted export.
        XCTAssertTrue(model.controllerRoutesFiles, "a rooted project routes the open file's saves through the helper")

        let edited = "\\begin{document}\nA paper, edited.\n\\end{document}\n"
        model.updateActiveText(edited)
        XCTAssertTrue(model.isDirty)
        model.saveTexInteractive()
        let saved = await settles(10) { !model.isDirty || model.captureNote?.contains("failed") == true }
        XCTAssertTrue(saved, "the save never completed: \(model.captureNote ?? "-")")
        XCTAssertEqual(try String(contentsOf: tex, encoding: .utf8), edited, "the open file holds the buffer (note: \(model.captureNote ?? "-"))")
        XCTAssertFalse(model.isDirty, model.captureNote ?? "-")
        XCTAssertEqual(model.savedText, edited)
    }

    static let fakeHelper = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_preview_controller.py")

    /// Finding 3 (the DocumentKinds use-after-free pattern, elsewhere):
    /// `ProjectDocuments` keeps an `unowned` back-reference to the model and
    /// its async helpers (`flushToHelper` polling the in-flight edit,
    /// `syncWithHelper`, `helperRequest`'s timeout) read it after awaits, while
    /// the app launches them from `Task`s that hold `ProjectDocuments`
    /// strongly (`switchDocument`, `armControllerTracking`, `init`). A model
    /// torn down during such a wait (tests do; a closed window would) left the
    /// task touching a freed object: a fatal unowned read that takes the whole
    /// process down. The fix keeps the model alive for the duration of the
    /// call, as `DocumentKinds.refresh` does.
    func testProjectDocumentsFlushSurvivesTheModelBeingReleasedMidWait() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-review-unowned-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        try "Hello\n".write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        var model: ShellModel? = ShellModel()
        model!.autoCompile = true
        XCTAssertEqual(model!.openTex(at: tex), .opened)
        model!.attachController(at: Self.fakeHelper)
        let ready = await settles(10) { model!.controllerState.ready && model!.result?.revision == model!.editorRevision }
        XCTAssertTrue(ready, model!.controllerStatus)
        weak var weakModel = model
        let project = model!.project
        // An edit whose preview is slow to arrive: the flush polls the in-flight slot for it.
        model!.controllerState.inFlight = ("review-held", "main.tex", model!.editorRevision, Date(), model!.activeText, 99, nil)
        model!.inFlightRevision = model!.editorRevision
        let flush = Task { @MainActor in await project.flushToHelper("main.tex", timeout: 0.5) }
        await Task.yield()
        await Task.yield()
        // The last strong reference goes while the flush is waiting.
        model = nil
        let flushed = await flush.value
        XCTAssertFalse(flushed, "the held edit never became durable within the bounded wait")
        // Run-loop blocks queued by the helper client may hold the model for a turn.
        let released = await settles(2) { weakModel == nil }
        XCTAssertTrue(released, "nothing retains the model once the flush has returned")
    }

    /// Finding 8: a capture approved while the editor is composing (IME marked
    /// text) produced a `pendingEdit` whose UTF-16 range was computed against
    /// the model text, which is BEHIND the storage by the marked run; the view
    /// applied it at once (before its own marked-text guard), landing the
    /// capture inside/before the composition, or — when AppKit refused the
    /// change — reported it applied anyway, so `appliedCaptureIDs` recorded a
    /// capture that was never inserted (a retry is refused as a duplicate).
    /// Expected: the edit waits for the composition; a range that no longer
    /// matches the buffer is refused explicitly, the capture returns to the
    /// review queue, and approving it again inserts at the pinned anchor.
    func testCaptureApprovedDuringCompositionIsNotMisplacedOrSilentlyLost() async throws {
        let h = try await IMEHarness.attached("capture-ime", text: "AB\n")
        defer { h.close() }
        let model = h.model
        // Pin the insertion point after "AB" (byte 2).
        h.textView.setSelectedRange(NSRange(location: 2, length: 0))
        XCTAssertEqual(model.caretUTF16, 2)
        model.pinAnchorAtCaret()
        XCTAssertEqual(model.anchor?.byteOffset, 2)
        // The IME composes two characters at the start: the storage is ahead of the model.
        h.textView.setSelectedRange(NSRange(location: 0, length: 0))
        h.compose("かな")
        XCTAssertTrue(h.hasMarkedText)
        XCTAssertEqual(h.string, "かなAB\n")
        XCTAssertEqual(model.activeText, "AB\n", "the model sees the buffer only once the composition commits")

        let proposal = RuntimeV1.CaptureProposal(captureId: "cap-ime-1", latex: "X", ambiguities: [], requiredDependencies: [])
        model.enqueue(proposal)
        XCTAssertEqual(model.approveProposal(proposal, latex: "X"), .inserted(byteOffset: 2))
        // Give the view its update passes while the composition is still open.
        try? await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertTrue(h.hasMarkedText, "the pending edit must not break the composition")
        XCTAssertEqual(h.string, "かなAB\n", "nothing is inserted into a buffer that is still being composed: \(h.probe.editApplied.map(\.1))")
        h.commit("かな")
        XCTAssertFalse(h.hasMarkedText)
        let settled = await settles(5) { model.pendingEdit == nil && model.activeText == "かなAB\n" }
        XCTAssertTrue(settled, "text \(model.activeText.debugDescription), pendingEdit \(String(describing: model.pendingEdit)), applied \(h.probe.editApplied.map(\.1))")
        XCTAssertNotEqual(h.string, "かなXAB\n", "the capture must not land inside the composed run")
        // Either the capture was inserted at the anchor, or it was refused explicitly and re-queued.
        if model.appliedCaptureIDs.contains("cap-ime-1") {
            XCTAssertTrue(model.activeText.hasPrefix("かなAB") && model.activeText.contains("X"), "an inserted capture sits after the pinned anchor: \(model.activeText.debugDescription)")
        } else {
            XCTAssertTrue(model.proposals.contains { $0.captureId == "cap-ime-1" }, "a refused capture returns to the queue: \(model.captureNote ?? "-")")
            XCTAssertEqual(model.approveProposal(proposal, latex: "X").isInserted, true, model.captureNote ?? "-")
            let inserted = await settles(5) { model.activeText.hasPrefix("かなAB") && model.activeText.contains("X") }
            XCTAssertTrue(inserted, "text \(model.activeText.debugDescription), note \(model.captureNote ?? "-")")
        }
        XCTAssertTrue(model.appliedCaptureIDs.contains("cap-ime-1"))
    }

    /// Finding 9 (direct worker route): an `error` envelope for the latest
    /// compile request cleared `compileQueued`, so a keystroke coalesced behind
    /// that request was never recompiled — the preview stayed stale until the
    /// next keystroke.
    func testWorkerErrorForTheLatestRequestStillCompilesTheQueuedBuffer() async throws {
        guard ShellModel.locateCompiler() != nil else { throw XCTSkip("set FLASHTEX_COMPILER to a built compiler") }
        let model = ShellModel()
        model.autoCompile = true
        model.replaceProject(entryText: "\\begin{document}\nWorker error.\n\\end{document}\n")
        XCTAssertTrue(model.attachDiscoveredWorker())
        defer { model.detachWorker() }
        model.compile()
        let first = await settles(15) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertTrue(first, model.workerStatus)
        // A goes out; B coalesces behind it; the worker answers A with an error.
        model.updateActiveText("\\begin{document}\nWorker error A.\n\\end{document}\n")
        let requestA = try XCTUnwrap(model.latestRequestID)
        XCTAssertEqual(model.inFlightRevision, model.editorRevision)
        model.updateActiveText("\\begin{document}\nWorker error B.\n\\end{document}\n")
        let revisionB = model.editorRevision
        model.handleForTesting(.error(id: requestA, message: "simulated worker error"))
        // B must be requested now (a fresh id) and previewed.
        let requested = await settles(5) { model.latestRequestID != requestA && model.inFlightRevision == revisionB || model.result?.revision == revisionB }
        XCTAssertTrue(requested, "the coalesced buffer was not recompiled after the error: latest \(model.latestRequestID ?? "-"), inFlight \(String(describing: model.inFlightRevision)), \(model.workerStatus)")
        let previewed = await settles(15) { model.result?.revision == revisionB }
        XCTAssertTrue(previewed, "preview never reached revision \(revisionB): \(model.workerStatus)")
        XCTAssertFalse(model.previewIsStale)
    }

    /// Finding 6: `completionFetcher` kept its outstanding query across a helper
    /// exit/relaunch while request ids restart at `pc-1` on the new client. A
    /// query still waiting for `pc-N` when the helper died swallowed the
    /// relaunched helper's reply with that id (`.refused` returns before
    /// `applyDurableDocument`): here the `document` reply, so the shell never
    /// learned the durable revision again and typing never became durable.
    /// The bounded auto-relaunch (`scheduleControllerRelaunch`) makes this an
    /// in-app path, not just a test one.
    func testCompletionQueryOutstandingAtHelperExitDoesNotSwallowTheRelaunchedHelpersReplies() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-review-ids-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        try "\\begin{document}\nRelaunch ids.\n\\end{document}\n".write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        defer { model.detachController() }
        let ready = await settles(15) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil && model.completionFetcher.query == nil }
        XCTAssertTrue(ready, model.controllerStatus)
        // One edit, to learn this session's id numbering: ready sends R requests
        // (configure_layout, display-candidate negotiation, document — the
        // document last), the document reply compiles (+1), the preview asks
        // three completion categories (+3) and document kinds a snapshot (+1); the
        // relaunched helper sends the same ready requests: document = pc-2.
        model.updateActiveText("\\begin{document}\nRelaunch ids, edited.\n\\end{document}\n")
        let editID = try XCTUnwrap(model.controllerState.inFlight?.id)
        guard let k = Int(editID.dropFirst("pc-".count)), k >= 6 else { throw XCTSkip("unexpected request id \(editID)") }
        let documentID = "pc-2" // configure_layout is pc-1, document pc-2 on every launch
        let edited = await settles(15) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil && model.completionFetcher.query == nil }
        XCTAssertTrue(edited, model.controllerStatus)
        // A completion query still waiting for the relaunched helper's document
        // id when the helper dies (the helper answered nothing for it).
        var handed = [documentID, "review-stale-a", "review-stale-b", "review-stale-kinds"] // 3 complete + 1 snapshot
        model.completionFetcher.request(sourceVersions: ["main.tex": 2], editorRevision: model.editorRevision) { _, _ in handed.removeFirst() }
        XCTAssertEqual(model.completionFetcher.query?.outstanding.count, 4)
        let pid = try XCTUnwrap(model.controller?.processIdentifier)
        XCTAssertEqual(kill(pid, SIGKILL), 0)
        let relaunched = await settles(15) { model.controllerRelaunchCount == 1 && model.controllerAttached && model.controllerState.ready }
        XCTAssertTrue(relaunched, model.controllerStatus)
        // The relaunched helper's document reply must be learned; typing must become durable.
        let learned = await settles(5) { model.controllerState.durable["main.tex"]?.revision == 2 }
        XCTAssertTrue(learned, "the relaunched helper's document reply was swallowed: durable \(String(describing: model.controllerState.durable["main.tex"]?.revision)), query \(String(describing: model.completionFetcher.query?.outstanding.keys.sorted()))")
        model.updateActiveText("\\begin{document}\nRelaunch ids, edited twice.\n\\end{document}\n")
        let durable = await settles(10) { model.controllerState.durable["main.tex"]?.revision == 3 && model.inFlightRevision == nil }
        XCTAssertTrue(durable, "typing after the relaunch stalled: \(model.controllerStatus)")
    }

    /// Finding 5: `controllerSave`/`controllerFileStatus` park a continuation in
    /// `controllerState.awaiting` with no timeout; an explicit
    /// `detachController()` reset the state without resuming it, so the
    /// awaiting Task hung forever (`.exited` fails the waiters; detach did not).
    func testExplicitDetachResumesAwaitingFileStatusAndSave() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-review-detach-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        try "\\begin{document}\nDetach.\n\\end{document}\n".write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        let ready = await settles(15) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertTrue(ready, model.controllerStatus)
        // Freeze the helper (a pid this test launched) so it cannot answer, ask, then detach.
        let pid = try XCTUnwrap(model.controller?.processIdentifier)
        XCTAssertEqual(kill(pid, SIGSTOP), 0)
        defer { kill(pid, SIGCONT); kill(pid, SIGKILL) }
        var status: ShellModel.ControllerDiskStatus?? = nil
        let ask = Task { @MainActor in status = .some(await model.controllerFileStatus(path: "main.tex")) }
        await Task.yield(); await Task.yield()
        XCTAssertEqual(model.controllerState.awaiting.count, 1, "the file_status request is awaiting its reply")
        model.detachController()
        let resumed = await settles(3) { status != nil }
        XCTAssertTrue(resumed, "controllerFileStatus never returned after the explicit detach")
        ask.cancel()
        XCTAssertNil(status ?? nil, "no status without a helper")
    }

    /// Polls `cond` on the main actor until it holds or `timeout` elapses.
    private func settles(_ timeout: TimeInterval, _ cond: () -> Bool) async -> Bool {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { return false }
            try? await Task.sleep(nanoseconds: 30_000_000)
        }
        return true
    }
}

private extension ShellModel.ApproveOutcome {
    var isInserted: Bool { if case .inserted = self { return true } else { return false } }
}
