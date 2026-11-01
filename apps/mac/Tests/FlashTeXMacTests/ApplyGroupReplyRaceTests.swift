import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The `apply_group` reply versus its compile outcome in the SAME read
/// (found by the #1510 review of ProjectSearchPanel.swift).
///
/// Every controller line is delivered to the main run loop as its own block
/// (`PreviewControllerClient.deliver`, `CFRunLoopPerformBlock`), while an
/// `await`ed reply resumes its continuation on the main dispatch queue. When
/// the reply and the follow-up preview are delivered together, the run loop
/// runs both blocks before the main queue resumes the awaiting task. If the
/// reply is reconciled only after the `await`, the preview is handled first:
/// nothing is in flight and its source version is unknown, so it is dropped;
/// the late reconciliation then records the `apply_group` as the in-flight
/// edit whose preview was already consumed, and the preview pipeline waits on
/// an outcome that never comes. The fix settles the reply inside the
/// `awaiting` waiter (the history panel's and the reviewed reload's pattern).
///
/// Deterministic against `Fixtures/fake_preview_controller.py`: its
/// `apply_group` writes the reply and the preview in one write, and the test
/// holds the main thread (not the run loop) across that write, so both lines
/// are queued as run-loop blocks before the main thread can service anything.
@MainActor
final class ApplyGroupReplyRaceTests: XCTestCase {
    static let fakeHelper = HistoricalPreviewTests.fakeHelper
    static let text = "Hello café\n"
    static let replaced = "Hello tea\n"

    private static let envKeys = ["FLASHTEX_CONTROLLER_LEDGER_ROOT", "FAKE_PC_APPLY_MARK", "FAKE_PC_APPLY_RELEASE", "FAKE_PC_APPLY_EMITTED"]

    func testApplyGroupReplyAndPreviewInOneReadReleaseThePipeline() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("apply-race-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try Self.text.write(to: tex, atomically: true, encoding: .utf8)
        let mark = root.appendingPathComponent("apply-received"), release = root.appendingPathComponent("apply-release"),
            emitted = root.appendingPathComponent("apply-emitted")
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        setenv("FAKE_PC_APPLY_MARK", mark.path, 1)
        setenv("FAKE_PC_APPLY_RELEASE", release.path, 1)
        setenv("FAKE_PC_APPLY_EMITTED", emitted.path, 1)
        let model = ShellModel()
        model.autoCompile = true
        defer {
            model.detachController()
            for k in Self.envKeys { unsetenv(k) }
            try? FileManager.default.removeItem(at: root)
        }
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: Self.fakeHelper)
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil("initial preview", model) {
            model.controllerState.durable["main.tex"]?.revision == 1 && model.result?.revision == model.editorRevision
                && model.controllerState.inFlight == nil && model.inFlightRevision == nil
        }

        // "café" is bytes 6..<11 of durable r1.
        let edit = ProjectSearch.ReplacementEdit(path: "main.tex", revision: 1, start: 6, end: 11, expectedText: "café", replacement: "tea")
        let client = ProjectSearchClient(model: model)
        let apply = Task { @MainActor in
            await client.applyReviewedEdits([edit], sourceVersions: ["main.tex": 1], membershipGeneration: 0,
                                            label: "Replace “café” with “tea”", commandPrefix: "race-test")
        }
        try await waitUntil("apply_group received", model) { FileManager.default.fileExists(atPath: mark.path) }

        // Hold the main thread across the helper's single write of reply +
        // preview, so both lines are queued as run-loop blocks before any
        // main-queue work (the awaiting task's continuation) can run.
        try Data().write(to: release)
        let deadline = Date().addingTimeInterval(10)
        while !FileManager.default.fileExists(atPath: emitted.path), Date() < deadline { usleep(5_000) }
        XCTAssertTrue(FileManager.default.fileExists(atPath: emitted.path), "the fake never wrote the apply_group reply")
        usleep(300_000) // the reader thread splits the read and queues both blocks meanwhile

        guard case .success(let outcomes) = await apply.value else { return XCTFail("applyReviewedEdits refused") }
        XCTAssertEqual(outcomes.count, 1)
        guard case .applied(let revision, _, let note)? = outcomes.first?.state else { return XCTFail("\(outcomes)") }
        XCTAssertEqual(revision, 2)
        XCTAssertEqual(note, "", "unchanged buffer: no moved-editor note")
        XCTAssertTrue(model.activeText.sameBytes(as: Self.replaced), model.activeText)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2)

        // The preview delivered in the same read as the reply was consumed by
        // the adopted edit: nothing is left in flight and it is shown.
        let log = model.workerLog.suffix(20).joined(separator: " | ")
        XCTAssertNil(model.controllerState.inFlight.map { "\($0.id) editor \($0.editorRevision) durable \($0.durableRevision.map(String.init) ?? "nil")" },
                     "the apply_group stayed in flight after its preview was consumed; log: \(log)")
        XCTAssertNil(model.inFlightRevision)
        XCTAssertEqual(model.result?.revision, model.editorRevision, "the post-apply preview was dropped; log: \(log)")
        XCTAssertFalse(model.workerLog.contains { $0.contains("ignored controller preview") && $0.contains("\"main.tex\": 2") }, log)
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: Self.replaced) == true)

        // And the pipeline is not wedged: the next keystroke becomes durable and is previewed.
        unsetenv("FAKE_PC_APPLY_RELEASE")
        let typed = Self.replaced + "more\n"
        model.updateActiveText(typed)
        try await waitUntil("next edit durable and previewed", model) {
            model.controllerState.durable["main.tex"]?.revision == 3 && model.result?.revision == model.editorRevision
                && model.controllerState.inFlight == nil
        }
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: typed) == true)
    }

    private func waitUntil(_ what: String, _ model: ShellModel, timeout: TimeInterval = 15, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout {
                XCTFail("timed out waiting for \(what); inFlight \(model.controllerState.inFlight.map { $0.id } ?? "nil"), result \(model.result?.revision.description ?? "nil"), editor \(model.editorRevision), log: \(model.workerLog.suffix(20).joined(separator: " | "))")
                throw XCTSkip("timeout: \(what)")
            }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }
}
