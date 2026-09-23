import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Oversized producer output (ShellModel+OutputBounds.swift): how the shell
/// reports replies that exceed a transport bound, and what the real helper
/// and the real compiler send for a fully-prose 560 KB document. Since
/// e26847c1 the compiler bounds its own reply to its 8 MiB transport frame
/// (`MAX_RESULT_BYTES`, crates/compiler/src/protocol.rs), dropping trailing
/// pages with an explicit diagnostic, so with DEFAULT limits the real
/// compiler can no longer emit an oversized line or frame. The oversized
/// paths are exercised where they remain real: a producer stub that does not
/// self-bound (direct route) and the helper's supported lower
/// `compiler_max_frame_bytes` (helper route); the real compiler's own
/// self-bounding is pinned by its own test. Pure tests always run; the
/// helper/worker tests need `FLASHTEX_PREVIEW_CONTROLLER` /
/// `FLASHTEX_COMPILER` and skip above a 1-minute load of 20 (they record,
/// never assert, compile times).
///
/// The hooks into the parent-retained files are diff requests
/// (coordination/mac-large-document.md). The live tests detect whether they
/// are applied (the in-flight edit is released by the model itself within
/// 2 s of the `failed` frame) and otherwise dispatch the hook by hand after
/// the real frame was observed in the model's log — the wire behaviour is
/// real either way; the printed `output-bounds:` lines say which.
@MainActor
final class OutputBoundsTests: XCTestCase {
    static var helper: URL? { ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) } }

    /// The paste-recovery lane's prose shape. Measured against the real
    /// compiler (2026-09-15): ≈45.7 bytes of compile_result JSON per source
    /// byte until the reply saturates the compiler's 8 MiB self-bound; from
    /// ≈180 KB of prose up the reply is a capped ~8.2 MB regardless of size.
    static func prose(bytes: Int) -> String {
        var s = "\\documentclass{article}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < bytes {
            s += "Paragraph \(n): the quick brown fox — naïve café \\textbf{bold} $x^2 + y^2 = z^2$ jumps over the lazy dog.\n"
            n += 1
        }
        return s + "\\end{document}\n"
    }

    /// The compiler frame bound the helper-route test configures: a quarter
    /// of the compiler's 8 MiB self-bound. With the DEFAULT 8 MiB
    /// `compiler_max_frame_bytes` a self-bounding compiler can never exceed
    /// the frame, so the oversized-frame path would be dead code in the test;
    /// the supported lower bound (`FLASHTEX_CONTROLLER_MAX_FRAME_BYTES` →
    /// `start.compiler_max_frame_bytes`, range 128 B..15 MiB, advertised back
    /// in `ready`) re-arms it with margin on BOTH sides: the 560 KB
    /// document's capped ~8.2 MB reply is 3.9× this bound, and the 20 KB base
    /// document's ~0.9 MB reply sits at 45% of it. If the compiler ever
    /// honours the helper's `FLASHTEX_MAX_REPLY_BYTES` (the natural
    /// completion of issue #21), the helper route stops seeing oversized
    /// frames entirely and the helper test fails on its `sawFailed` wait —
    /// switch it to a non-self-bounding producer stub then.
    static let configuredCompilerFrameBytes = 2 * 1024 * 1024

    // MARK: pure

    func testViolationMessagesAreParsed() {
        let a = OutputBounds.parseViolation("line of 18157062 bytes exceeds the 16777216-byte limit")
        XCTAssertEqual(a?.replyBytes, 18_157_062)
        XCTAssertEqual(a?.boundBytes, 16_777_216)
        let b = OutputBounds.parseViolation("unterminated line exceeds the 16777216-byte limit")
        XCTAssertNil(b?.replyBytes)
        XCTAssertEqual(b?.boundBytes, 16_777_216)
        XCTAssertNotNil(OutputBounds.parseViolation("frame of 17000000 bytes exceeds the 16777216-byte limit"))
        XCTAssertNil(OutputBounds.parseViolation("frame is not valid JSON: x"))
        XCTAssertNil(OutputBounds.parseViolation("compile_result mac-3 reports project a revision 2; request was project b revision 2"))
    }

    func testStatusNamesBoundAndDocumentSizeAndRetryIsBounded() {
        // A hypothetical unbounded producer's 18.2 MB reply (the pre-e26847c1
        // compiler really sent these); the numbers only need to be
        // self-consistent — the ratio the retry gate uses (31.7) is the
        // notice's own replyBytes / documentBytes.
        let direct = OutputBoundNotice(route: .direct, editorRevision: 7, documentBytes: 573_476, boundBytes: 16_777_216,
                                       boundName: OutputBounds.workerLineBoundName, replyBytes: 18_157_062)
        XCTAssertEqual(direct.status, "revision 7: reply of 17.3 MiB exceeds the worker's worker line limit (RuntimeV1.maxLineBytes) (16 MiB) for this 560 KB document; last preview kept")
        XCTAssertTrue(direct.banner.contains("560 KB document exceeds the worker line limit"))
        XCTAssertTrue(direct.banner.contains("⌘B"))
        XCTAssertFalse(direct.allowsRetry(documentBytes: 573_476), "the same document is never re-sent")
        XCTAssertFalse(direct.allowsRetry(documentBytes: 573_000), "a few bytes less still overflows by the measured ratio")
        XCTAssertFalse(direct.allowsRetry(documentBytes: 540_000), "540 KB × 31.7 = 17.1 MB > 16 MiB")
        XCTAssertTrue(direct.allowsRetry(documentBytes: 500_000), "500 KB × 31.7 = 15.8 MB < 16 MiB")
        let helper = OutputBoundNotice(route: .helper, editorRevision: 3, documentBytes: 573_476, boundBytes: 8_388_608,
                                       boundName: OutputBounds.compilerFrameBoundName, replyBytes: nil)
        XCTAssertEqual(helper.status, "revision 3: reply exceeds the helper's compiler frame bound (compiler_max_frame_bytes) (8 MiB) for this 560 KB document; last preview kept")
        XCTAssertFalse(helper.allowsRetry(documentBytes: 573_476))
        XCTAssertFalse(helper.allowsRetry(documentBytes: 560_000), "unknown reply size: needs a tenth off")
        XCTAssertTrue(helper.allowsRetry(documentBytes: 516_000))
    }

    func testDirectRouteHookBlocksTheRelaunchCompileUntilTheDocumentShrinks() {
        let model = ShellModel()
        model.documents = [.init(path: "main.tex", text: Self.prose(bytes: 560 * 1024))]
        model.activePath = "main.tex"
        XCTAssertFalse(model.outputBoundHandleWorkerViolation("frame is not valid JSON"), "other violations are not size overflows")
        XCTAssertTrue(model.outputBoundHandleWorkerViolation("line of 18157062 bytes exceeds the \(RuntimeV1.maxLineBytes)-byte limit"))
        let notice = try! XCTUnwrap(model.outputBound)
        XCTAssertEqual(notice.route, .direct)
        XCTAssertEqual(notice.documentBytes, model.documents[0].text.utf8.count)
        XCTAssertTrue(model.workerStatus.contains("560 KB document"))
        XCTAssertTrue(model.workerStatus.contains("16 MiB"))
        XCTAssertTrue(model.outputBoundBlocksCompile, "the relaunch's auto-compile of the same document is refused")
        model.documents[0].text = Self.prose(bytes: 200 * 1024)
        XCTAssertFalse(model.outputBoundBlocksCompile, "200 KB × 31.7 fits: the next compile is allowed")
        XCTAssertFalse(model.outputBoundExplicitRetry(), "no helper attached: ⌘B just clears the notice")
        XCTAssertNil(model.outputBound)
    }

    func testHelperUpdateHookReleasesTheHeldEditAndKeepsTheStatus() {
        let model = ShellModel()
        model.documents = [.init(path: "main.tex", text: Self.prose(bytes: 560 * 1024))]
        model.activePath = "main.tex"
        model.outputBoundNoteReady(compilerMaxFrameBytes: 15_728_640, helperMaxOutputBytes: 16_777_216)
        model.controllerState.inFlight = ("pc-9", "main.tex", 4, Date(), model.documents[0].text, 2, nil)
        model.inFlightRevision = 4
        XCTAssertFalse(model.outputBoundHandleControllerUpdate(kind: "stale", payload: ["compile_revision": 2]))
        XCTAssertFalse(model.outputBoundHandleControllerUpdate(kind: "failed", payload: ["reason": "compiler input closed"]), "other failures are not size overflows")
        XCTAssertNotNil(model.controllerState.inFlight)
        XCTAssertTrue(model.outputBoundHandleControllerUpdate(kind: "failed", payload: ["kind": "failed", "request_id": "preview-2", "reason": OutputBounds.helperFailedReason]))
        XCTAssertNil(model.controllerState.inFlight, "the hold-until-preview pipeline is released")
        XCTAssertNil(model.inFlightRevision)
        XCTAssertEqual(model.outputBounds.releasedInFlightCount, 1)
        XCTAssertEqual(model.outputBound?.boundBytes, 15_728_640, "the bound advertised by ready")
        XCTAssertEqual(model.outputBound?.editorRevision, 4)
        XCTAssertTrue(model.workerStatus.contains("15 MiB") && model.workerStatus.contains("560 KB document"), model.workerStatus)
        XCTAssertTrue(model.outputBoundHandleControllerError(id: nil, message: "response exceeds output limit; source may already be durable"))
        XCTAssertEqual(model.outputBound?.boundName, OutputBounds.helperOutputBoundName)
        XCTAssertFalse(model.outputBoundHandlePreviewError("compiler session failed; create a new session with complete snapshots"), "no helper attached: nothing to restart")
        model.outputBoundNotePreviewApplied()
        XCTAssertNil(model.outputBound)
    }

    // MARK: real helper / real compiler

    private func gate(_ name: String) throws -> (helper: URL, compiler: URL) {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path), let compiler = ShellModel.locateCompiler() else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let load = PasteRecoveryTests.loadAverage1()
        // `FLASHTEX_OUTPUT_BOUNDS_LOAD_LIMIT` raises the skip threshold for a
        // correctness run on a shared host; its numbers are then labelled.
        let limit = ProcessInfo.processInfo.environment["FLASHTEX_OUTPUT_BOUNDS_LOAD_LIMIT"].flatMap(Double.init) ?? 20
        print("output-bounds: \(name) uptime: \(PasteRecoveryTests.uptimeLine())\(load > 20 ? " [under shared load, not an isolated result]" : "")")
        if load > limit { throw XCTSkip("1-minute load average \(load) > \(limit); timing-sensitive test skipped") }
        return (helper, compiler)
    }

    private func waitUntil(timeout: TimeInterval, _ cond: () -> Bool) async -> Bool {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { return false }
            try? await Task.sleep(nanoseconds: 2_000_000)
        }
        return true
    }

    func testRealHelperRefusesThe560KBPreviewWithOneFailedFrameAndTypingContinues() async throws {
        let (helper, _) = try gate("helper")
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("output-bounds-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer {
            try? FileManager.default.removeItem(at: root)
            unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
            unsetenv("FLASHTEX_CONTROLLER_MAX_FRAME_BYTES")
        }
        let tex = root.appendingPathComponent("project/main.tex")
        // ~0.9 MB reply: inside the configured frame bound with a 2× margin.
        let base = Self.prose(bytes: 20 * 1024)
        try base.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        // The self-bounding compiler never exceeds the DEFAULT 8 MiB frame;
        // the supported lower bound keeps this refusal path real end-to-end
        // (see configuredCompilerFrameBytes for the margins).
        setenv("FLASHTEX_CONTROLLER_MAX_FRAME_BYTES", String(Self.configuredCompilerFrameBytes), 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        defer { model.detachController() }
        guard await waitUntil(timeout: 60, { model.result?.revision == model.editorRevision && model.controllerState.inFlight == nil }) else {
            throw XCTSkip("no preview for the 20 KB base within 60 s (load \(PasteRecoveryTests.loadAverage1()))")
        }
        let goodRevision = model.editorRevision
        let goodPages = model.result?.pages.count ?? 0
        // 560 KB of prose saturates the compiler's 8 MiB self-bound (~8.2 MB
        // reply, 3.9× the configured frame bound) and, JSON-escaped, its edit
        // line (~0.59 MB) stays under the helper's 1 MiB stdin line bound —
        // the fixture cannot grow much past this.
        let big = Self.prose(bytes: 560 * 1024)
        let t0 = Date()
        model.updateActiveText(big)
        let bigRevision = model.editorRevision
        XCTAssertEqual(model.controllerState.inFlight?.editorRevision, bigRevision)
        // Durable ACK of the paste, then the helper's answer to its compile.
        let ok1 = await waitUntil(timeout: 60, { model.controllerState.durable["main.tex"]?.revision == 2 })
        XCTAssertTrue(ok1, "durable ACK")
        let ackMs = Date().timeIntervalSince(t0) * 1000
        let sawFailed = await waitUntil(timeout: 60, { model.workerLog.contains { $0.contains("update failed") || $0.contains("output bound") } })
        XCTAssertTrue(sawFailed, "the helper answered with an update kind:failed frame: \(model.workerLog.suffix(5))")
        let failedMs = Date().timeIntervalSince(t0) * 1000
        let failedLines = model.workerLog.filter { $0.contains("update failed") || $0.contains("output bound") }
        XCTAssertFalse(model.workerLog.contains { $0.contains("protocol violation") }, "no partial or oversized frame reached the client")
        // Hook applied? The model releases the held edit by itself.
        let hooked = await waitUntil(timeout: 2, { model.controllerState.inFlight == nil })
        if !hooked {
            XCTAssertNotNil(model.controllerState.inFlight, "without the hook the edit stays held (the gap)")
            model.outputBoundHandleControllerUpdate(kind: "failed", payload: ["kind": "failed", "request_id": "preview-2", "reason": OutputBounds.helperFailedReason])
        }
        XCTAssertNil(model.controllerState.inFlight)
        XCTAssertNil(model.inFlightRevision)
        let notice = try XCTUnwrap(model.outputBound)
        XCTAssertEqual(notice.route, .helper)
        XCTAssertEqual(notice.boundBytes, Self.configuredCompilerFrameBytes, "the compiler_max_frame_bytes advertised by ready")
        XCTAssertEqual(notice.documentBytes, big.utf8.count)
        XCTAssertTrue(model.workerStatus.contains(OutputBoundNotice.mib(Self.configuredCompilerFrameBytes)) && model.workerStatus.contains("\(OutputBoundNotice.kb(big.utf8.count)) document"), model.workerStatus)
        // Last good preview kept, marked stale; never blank.
        XCTAssertEqual(model.result?.revision, goodRevision)
        XCTAssertEqual(model.result?.pages.count, goodPages)
        XCTAssertTrue(model.previewIsStale)
        XCTAssertNil(model.historicalPreview)
        // Typing continues: the next edit is submitted and ACKed durably
        // (with preview_error: the helper's compiler session is gone).
        let t1 = Date()
        model.updateActiveText(big + "% more\n")
        XCTAssertEqual(model.controllerState.inFlight?.editorRevision, model.editorRevision, "submitted immediately, not queued behind the failed one")
        let ok2 = await waitUntil(timeout: 60, { model.controllerState.durable["main.tex"]?.revision == 3 })
        XCTAssertTrue(ok2, "the next durable edit is still ACKed")
        let nextAckMs = Date().timeIntervalSince(t1) * 1000
        let ok3 = await waitUntil(timeout: 10, { model.controllerState.inFlight == nil })
        XCTAssertTrue(ok3, "preview_error releases the pipeline")
        XCTAssertTrue(model.controllerStatus.contains("preview error: compiler session failed") || model.controllerStatus.contains("exceeds"), model.controllerStatus)
        XCTAssertEqual(model.controllerRelaunchCount, 0, "no helper relaunch")
        XCTAssertFalse(model.outputBounds.retryInFlight, "no automatic retry for a document that did not shrink")
        // Shrinking back to the base: the retry restarts the helper's compiler and a preview arrives.
        let t2 = Date()
        model.updateActiveText(base + "% back\n")
        let shrunkDurable = await waitUntil(timeout: 60, { model.controllerState.durable["main.tex"]?.revision == 4 })
        XCTAssertTrue(shrunkDurable, "the shrunk document is durable")
        if !hooked, let e = "compiler session failed; create a new session with complete snapshots" as String? {
            XCTAssertTrue(model.outputBoundHandlePreviewError(e), "restart sent for the 20 KB document")
        }
        let recovered = await waitUntil(timeout: 60, { model.result?.revision == model.editorRevision })
        XCTAssertTrue(recovered, "a preview for the shrunk document arrives after the restart: \(model.workerLog.suffix(6))")
        if !hooked { model.outputBoundNotePreviewApplied() }
        XCTAssertNil(model.outputBound)
        XCTAssertFalse(model.previewIsStale)
        print(String(format: "output-bounds: helper route (frame bound %@ configured): 560 KB (%d B) paste ack %.0f ms, failed frame at %.0f ms, next edit ack %.0f ms, recovery preview %.0f ms; hooks %@; frames: %@",
                     OutputBoundNotice.mib(Self.configuredCompilerFrameBytes), big.utf8.count, ackMs, failedMs, nextAckMs,
                     Date().timeIntervalSince(t2) * 1000, hooked ? "applied" : "dispatched by the test", failedLines.joined(separator: " | ")))
    }

    /// Since e26847c1 the real compiler bounds its reply to 8 MiB — less than
    /// half of `RuntimeV1.maxLineBytes` — so a line the client must refuse can
    /// only come from a producer that does not self-bound (the real-compiler
    /// half of the story is `testRealCompilerDirectRouteDeliversABoundedReply…`).
    /// This stub answers every request with one line derived from the limit
    /// itself (maxLineBytes + 1 MiB) over a real pipe: WorkerClient must
    /// refuse it typed, deliver nothing partial, and the relaunch must not
    /// re-send the same document.
    func testDirectRouteOversizedLineIsReportedAndNotResent() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("output-bounds-stub-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let stub = dir.appendingPathComponent("oversized-producer.sh")
        let replyBytes = RuntimeV1.maxLineBytes + 1024 * 1024
        try """
        #!/bin/sh
        while read -r _; do
          head -c \(replyBytes) /dev/zero | tr '\\0' a
          echo
        done
        """.write(to: stub, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: stub.path)
        let model = ShellModel()
        model.autoCompile = false
        model.documents = [.init(path: "main.tex", text: Self.prose(bytes: 560 * 1024))]
        model.activePath = "main.tex"
        model.attachWorker(at: stub)
        defer { model.detachWorker() }
        XCTAssertTrue(model.workerAttached)
        let before = (model.result?.revision, model.result?.pages.count, model.previewSource)
        let t0 = Date()
        model.compile()
        XCTAssertNotNil(model.inFlightRevision)
        // The pipe delivers the 17 MiB line in chunks, so WorkerClient's
        // partial-buffer check fires first ("unterminated line exceeds …"),
        // never the complete-line one; the reply size is therefore unknown.
        let isOverflow: (String) -> Bool = { $0.hasPrefix("protocol violation: ") && $0.contains("exceeds the \(RuntimeV1.maxLineBytes)-byte limit") }
        let ok4 = await waitUntil(timeout: 60, { model.workerLog.contains(where: isOverflow) })
        XCTAssertTrue(ok4, "the \(replyBytes)-byte line is refused by WorkerClient: \(model.workerLog.suffix(5))")
        let violationMs = Date().timeIntervalSince(t0) * 1000
        let line = try XCTUnwrap(model.workerLog.last(where: isOverflow))
        let parsed = try XCTUnwrap(OutputBounds.parseViolation(line))
        XCTAssertEqual(parsed.boundBytes, RuntimeV1.maxLineBytes)
        // Worker terminated, relaunched (bounded): with the hook the relaunch does not re-send.
        _ = await waitUntil(timeout: 5, { model.workerRelaunchCount >= 1 })
        let hooked = model.outputBound != nil
        if !hooked { model.outputBoundHandleWorkerViolation(String(line.dropFirst("protocol violation: ".count))) }
        let notice = try XCTUnwrap(model.outputBound)
        XCTAssertEqual(notice.route, .direct)
        XCTAssertEqual(notice.replyBytes, parsed.replyBytes)
        XCTAssertEqual(notice.documentBytes, model.documents[0].text.utf8.count)
        XCTAssertTrue(model.outputBoundBlocksCompile)
        XCTAssertTrue(model.workerStatus.contains("16 MiB") && model.workerStatus.contains("KB document"), model.workerStatus)
        XCTAssertTrue(before == (model.result?.revision, model.result?.pages.count, model.previewSource), "nothing partial was painted; the previous result stays")
        print(String(format: "output-bounds: direct route (stub producer): 560 KB (%d B) → %d B line refused (%@) after %.0f ms; relaunches %d; hooks %@; log: %@",
                     model.documents[0].text.utf8.count, replyBytes, line, violationMs, model.workerRelaunchCount, hooked ? "applied" : "dispatched by the test",
                     model.workerLog.filter { $0.contains("violation") || $0.contains("exited") || $0.contains("relaunch") }.joined(separator: " | ")))
    }

    /// The real-compiler half of the direct route: the compiler can no longer
    /// exceed `RuntimeV1.maxLineBytes` because it bounds its own reply to its
    /// 8 MiB transport frame and says what it dropped (e26847c1, issue #21).
    /// An oversized document yields ONE bounded reply that is applied as a
    /// preview — no protocol violation, no output-bound notice — carrying the
    /// explicit dropped-pages diagnostic. The document grows until that
    /// diagnostic appears, so a more compact future encoding cannot silently
    /// turn this into a trivially-passing compile test; if the compiler
    /// regresses to oversized lines, the violation assertions fail instead.
    func testRealCompilerDirectRouteDeliversABoundedReplyWithDroppedPagesReported() async throws {
        let (_, compiler) = try gate("direct-bounded")
        let model = ShellModel()
        model.autoCompile = false
        model.documents = [.init(path: "main.tex", text: "x")]
        model.activePath = "main.tex"
        model.attachWorker(at: compiler)
        defer { model.detachWorker() }
        XCTAssertTrue(model.workerAttached)
        // 560 KB saturates today's 8 MiB self-bound about 3× (~8.2 MB reply,
        // ≈45.7 output bytes per source byte). The growth cap keeps the
        // JSON-escaped request line under the compiler's own 8 MiB stdin
        // line bound.
        var bytes = 560 * 1024
        var t0 = Date()
        while true {
            model.updateActiveText(Self.prose(bytes: bytes))
            let revision = model.editorRevision
            t0 = Date()
            model.compile()
            let violated: () -> Bool = { model.workerLog.contains { $0.contains("protocol violation") } }
            let replied = await waitUntil(timeout: 60, { model.result?.revision == revision || violated() })
            XCTAssertFalse(violated(), "the compiler must bound its own reply under RuntimeV1.maxLineBytes: \(model.workerLog.suffix(5))")
            XCTAssertTrue(replied, "no reply for the \(OutputBoundNotice.kb(bytes)) document: \(model.workerLog.suffix(5))")
            guard replied, !violated() else { return }
            if model.result?.diagnostics.contains(where: { $0.message.contains("pages were not delivered") }) == true { break }
            bytes *= 2
            guard bytes <= 4 * 1024 * 1024 else {
                XCTFail("no dropped-pages diagnostic up to \(OutputBoundNotice.kb(bytes / 2)) of prose; the compiler no longer saturates its self-bound — re-derive this suite's sizes")
                return
            }
        }
        XCTAssertNil(model.outputBound, "a self-bounded reply is not an output-bound overflow")
        XCTAssertEqual(model.result?.revision, model.editorRevision)
        XCTAssertNotEqual(model.result?.pages.count ?? 0, 0, "the leading pages are delivered")
        print(String(format: "output-bounds: direct route (real compiler): %@ document → bounded reply, dropped pages reported, in %.0f ms; diagnostic: %@",
                     OutputBoundNotice.kb(model.documents[0].text.utf8.count), Date().timeIntervalSince(t0) * 1000,
                     model.result?.diagnostics.first(where: { $0.message.contains("pages were not delivered") })?.message ?? "(missing)"))
    }
}
