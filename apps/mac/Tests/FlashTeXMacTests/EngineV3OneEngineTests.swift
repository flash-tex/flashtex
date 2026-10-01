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
    private var storedFlag: Any?

    override func setUp() {
        setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1)
        setenv("FLASHTEX_HOST", "none", 1)
        storedFlag = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
    }

    override func tearDown() {
        unsetenv("FLASHTEX_HOST")
        unsetenv("FLASHTEX_V3_CACHE")
        if let storedFlag { UserDefaults.standard.set(storedFlag, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) }
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
}
