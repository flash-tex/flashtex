import Foundation
import XCTest
@testable import FlashTeXMac

/// One engine at a time (lane P5-APP-PARITY, gap A2): with the engine-v3
/// preview on, the old engine compiles nothing (open, ⌘B, auto-compile) and
/// its last result is dropped, so underlines, Export and Print never read
/// old-engine output while v3 is shown. Turning v3 off compiles with the old
/// engine again. Runs against the fake worker and no host at all
/// (`FLASHTEX_HOST=none`), in a private v3 cache.
@MainActor
final class EngineV3OneEngineTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-one-engine-\(getpid())")
    /// Environment set for a test and put back after it (never just unset).
    private var env = EnvironmentOverride()

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_HOST", "none")
    }

    override func tearDown() {
        env.restore()
        try? FileManager.default.removeItem(at: Self.cache)
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    private func attachedModel() -> ShellModel {
        let model = ShellModel()
        model.engineV3Enabled = false
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.autoCompile = true
        return model
    }

    func testTurningV3OnDropsTheOldResultAndStopsOldCompiles() async throws {
        let model = attachedModel()
        defer { model.engineV3.stop(); model.detachWorker() }
        model.updateActiveText("Hello old engine. %diag:0")
        model.compile()
        try await waitUntil("the old engine's result") { model.result != nil && model.inFlightRevision == nil }
        XCTAssertFalse(model.editorMarkReport.marks.isEmpty, "the fake worker's diagnostic is an editor mark on the old path")

        model.engineV3Enabled = true

        // Nothing of the old result is left in use.
        XCTAssertNil(model.result)
        XCTAssertNil(model.resultID)
        XCTAssertNil(model.displayListV2)
        XCTAssertTrue(model.compiledDocuments.isEmpty)
        XCTAssertTrue(model.editorMarkReport.marks.isEmpty)
        XCTAssertNil(model.lastLatencyMs)
        XCTAssertEqual(model.displayedDiagnostics, model.engineV3Diagnostics)
        XCTAssertTrue(model.exportPDFRefusal()?.contains("engine-v3") == true, model.exportPDFRefusal() ?? "nil")
        XCTAssertTrue(PrintController.documentHelp(model).contains("engine-v3"))
        XCTAssertFalse(model.toolbarExportable)

        // ⌘B, typing and the file-open path send nothing to the old worker.
        model.compile()
        XCTAssertTrue(model.inFlightRequests.isEmpty, "⌘B under v3 must not reach the old worker")
        model.updateActiveText("Typed under v3. %diag:0")
        XCTAssertTrue(model.inFlightRequests.isEmpty, "an edit under v3 must not reach the old worker")
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertNil(model.result, "no old-engine result arrives while v3 is on")
        XCTAssertNil(model.inFlightRevision)
        XCTAssertTrue(model.workerAttached, "the old worker stays attached, idle")

        // Off again: the old engine compiles the current text at once.
        model.engineV3Enabled = false
        try await waitUntil("the old engine's result after turning v3 off") { model.result != nil && model.inFlightRevision == nil }
        XCTAssertEqual(model.result?.revision, model.editorRevision)
    }

    func testV3CompiledTextsAreTheNavigationBaselineOnlyUnderV3() {
        let model = ShellModel()
        model.engineV3Enabled = false
        let oldBaseline = model.compiledDocuments
        model.setEngineV3CompiledDocuments(["main.tex": "x"])
        XCTAssertEqual(model.compiledDocuments, oldBaseline, "the old path's baseline is its own result's request text")
        model.engineV3Enabled = true
        defer { model.engineV3.stop() }
        model.setEngineV3CompiledDocuments(["main.tex": "x"])
        XCTAssertEqual(model.compiledDocuments, ["main.tex": "x"])
        model.engineV3Enabled = false
        XCTAssertTrue(model.compiledDocuments.isEmpty)
    }

    func testCaptureReviewDoesNotCompileWithTheOldEngineUnderV3() {
        let model = ShellModel()
        defer { model.engineV3.stop() }
        model.engineV3Enabled = true
        let preview = ProposalPreview(executable: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        defer { preview.close() }
        preview.update(from: model, latex: "x^2")
        guard case .notPreviewable(let why) = preview.state else { return XCTFail("state \(preview.state)") }
        XCTAssertTrue(why.contains("engine-v3"), why)
        XCTAssertFalse(preview.workerIsRunning, "no old-engine shadow worker under v3")
        XCTAssertNil(preview.thumbnail)
    }

    func testMathHoverHasNoOldEngineFrameUnderV3() throws {
        let model = attachedModel()
        defer { model.engineV3.stop(); model.detachWorker() }
        model.engineV3Enabled = true
        XCTAssertNil(model.displayListV2, "no old-engine frame for the math hover to crop")
    }

    func testTheDurableHelperIsSetAsideUnderV3AndComesBack() async throws {
        let fake = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures/fake_preview_controller.py")
        let ledger = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-helper-\(UUID().uuidString)")
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-helper-doc-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: ledger); try? FileManager.default.removeItem(at: dir) }
        env.set("FLASHTEX_CONTROLLER_LEDGER_ROOT", ledger.path)
        let main = dir.appendingPathComponent("main.tex")
        try "Hello\n".write(to: main, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        model.engineV3Enabled = false
        XCTAssertEqual(model.openTex(at: main), .opened)
        model.attachController(at: fake)
        defer { model.detachController(); model.engineV3.stop() }
        func waitUntil(_ what: String, _ cond: () -> Bool) async throws {
            let start = Date()
            while !cond() {
                if Date().timeIntervalSince(start) > 10 { return XCTFail("timed out waiting for \(what)") }
                try await Task.sleep(nanoseconds: 20_000_000)
            }
        }
        try await waitUntil("helper ready") { model.controllerAttached && model.controllerState.ready }
        model.engineV3Enabled = true
        XCTAssertFalse(model.controllerAttached, "the helper compiles every edit it records: set aside under v3")
        XCTAssertEqual(model.controllerSuspendedForV3, fake)
        model.updateActiveText("Hello, under v3\n")
        XCTAssertNil(model.inFlightRevision)
        model.engineV3Enabled = false
        XCTAssertNil(model.controllerSuspendedForV3)
        try await waitUntil("helper back") { model.controllerAttached && model.controllerState.ready }
    }
}
