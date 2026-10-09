import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The helper route's release policy (ControllerRelease.swift): hold-until-
/// preview by default, `hybrid` releases a durable in-flight edit once it has
/// been in flight for the adaptive bound. End-to-end cases drive a real
/// `ShellModel` against `Fixtures/fake_preview_controller.py`, whose `%hold`
/// directive never answers a compile until the next one — under the default
/// policy the next keystroke waits forever; under `hybrid` it goes out at the bound.
@MainActor
final class ControllerReleaseTests: XCTestCase {
    static let fakeHelper = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_preview_controller.py")

    func testPolicyDefaultsToHoldAndOptsIntoHybrid() {
        XCTAssertEqual(ControllerReleasePolicy.fromEnvironment([:]), .holdUntilPreview)
        XCTAssertEqual(ControllerReleasePolicy.fromEnvironment(["FLASHTEX_CONTROLLER_RELEASE": "hold"]), .holdUntilPreview)
        XCTAssertEqual(ControllerReleasePolicy.fromEnvironment(["FLASHTEX_CONTROLLER_RELEASE": "Hybrid"]), .holdUntilPreview)
        XCTAssertEqual(ControllerReleasePolicy.fromEnvironment(["FLASHTEX_CONTROLLER_RELEASE": "hybrid"]), .hybrid)
        XCTAssertEqual(ControllerState().releasePolicy, ControllerReleasePolicy.fromEnvironment())
    }

    func testBoundIsTwiceTheLastLatencyClampedTo40And250() {
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: nil), 40)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 0), 40)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: -5), 40)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: .nan), 40)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 10), 40)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 20), 40)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 30), 60)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 100), 200)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 125), 250)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 900), 250)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 100, multiplier: 1), 100)
        XCTAssertEqual(ReleaseBound.boundMs(lastEditToPreviewMs: 100, multiplier: 3), 250)
        XCTAssertEqual(ReleaseBound.defaultMultiplier, 2)
        XCTAssertEqual(ReleaseBound.remainingMs(inFlightMs: 10, lastEditToPreviewMs: 100), 190)
        XCTAssertEqual(ReleaseBound.remainingMs(inFlightMs: 300, lastEditToPreviewMs: 100), 0)
    }

    func testReleaseNeedsHybridDurableQueuedAndTheBound() {
        // Hold never releases early, whatever the timing.
        XCTAssertFalse(ReleaseBound.shouldRelease(policy: .holdUntilPreview, durable: true, queued: true, inFlightMs: 10_000, lastEditToPreviewMs: 30))
        // Hybrid: not before durable (the helper does not hold the text yet).
        XCTAssertFalse(ReleaseBound.shouldRelease(policy: .hybrid, durable: false, queued: true, inFlightMs: 10_000, lastEditToPreviewMs: 30))
        // Not without newer text to send.
        XCTAssertFalse(ReleaseBound.shouldRelease(policy: .hybrid, durable: true, queued: false, inFlightMs: 10_000, lastEditToPreviewMs: 30))
        // Not before the bound.
        XCTAssertFalse(ReleaseBound.shouldRelease(policy: .hybrid, durable: true, queued: true, inFlightMs: 59, lastEditToPreviewMs: 30))
        XCTAssertTrue(ReleaseBound.shouldRelease(policy: .hybrid, durable: true, queued: true, inFlightMs: 60, lastEditToPreviewMs: 30))
        // A fast compiler (last 15 ms → bound 40) still holds for 40 ms; a slow one caps at 250.
        XCTAssertFalse(ReleaseBound.shouldRelease(policy: .hybrid, durable: true, queued: true, inFlightMs: 39, lastEditToPreviewMs: 15))
        XCTAssertTrue(ReleaseBound.shouldRelease(policy: .hybrid, durable: true, queued: true, inFlightMs: 250, lastEditToPreviewMs: 2000))
    }

    // MARK: end to end against the fake helper

    private struct Harness { let model: ShellModel; let root: URL; let tex: URL }

    private func makeHarness(name: String, historical: Bool = false) throws -> Harness {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("rel-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try "Hello\n".write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        model.historicalState.requested = historical
        XCTAssertEqual(model.openTex(at: tex), .opened)
        return Harness(model: model, root: root, tex: tex)
    }

    private func attach(_ h: Harness, policy: ControllerReleasePolicy) async throws {
        h.model.attachController(at: Self.fakeHelper)
        h.model.controllerState.releasePolicy = policy
        XCTAssertTrue(h.model.controllerAttached)
        try await waitUntil("initial preview") { h.model.result?.revision == h.model.editorRevision && h.model.previewSource == .worker("flashtex-preview-controller") }
    }

    private func teardown(_ h: Harness) {
        h.model.detachController()
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
        try? FileManager.default.removeItem(at: h.root)
    }

    private func firstItemText(_ model: ShellModel) -> String? {
        guard let page = model.result?.pages.first, case .text(let item)? = page.items.first else { return nil }
        return item.text
    }

    func testHybridReleasesADurableEditAtTheBoundAndHoldDoesNot() async throws {
        // hold: the second keystroke stays queued behind the never-answered compile.
        do {
            let h = try makeHarness(name: "hold")
            defer { teardown(h) }
            try await attach(h, policy: .holdUntilPreview)
            let model = h.model
            model.updateActiveText("%hold\nA text\n")
            try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 }
            model.updateActiveText("B text\n")
            XCTAssertTrue(model.controllerState.queued)
            try await Task.sleep(nanoseconds: 400_000_000)
            XCTAssertNotNil(model.controllerState.inFlight, "hold keeps the edit in flight until its preview")
            XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "B was not sent")
            XCTAssertFalse(model.workerLog.contains { $0.contains("hybrid release") })
        }
        // hybrid: B goes out once A has been in flight for the bound (40 ms here: no
        // latency observed yet beyond the initial ~1 ms round trips → minimum).
        do {
            let h = try makeHarness(name: "hybrid")
            defer { teardown(h) }
            try await attach(h, policy: .hybrid)
            let model = h.model
            model.updateActiveText("%hold\nA text\n")
            let revA = model.editorRevision
            try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 }
            XCTAssertNotNil(model.controllerState.inFlight, "still held right after the durable receipt")
            model.updateActiveText("B text\n")
            let revB = model.editorRevision
            XCTAssertTrue(model.controllerState.queued || model.controllerState.inFlight?.editorRevision == revB)
            try await waitUntil("B previewed") { model.result?.revision == revB }
            XCTAssertEqual(firstItemText(model), "B text")
            XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3, "B became durable r3 without waiting for A's preview")
            let release = try XCTUnwrap(model.workerLog.first { $0.contains("hybrid release") })
            XCTAssertTrue(release.contains("revision \(revA) durable r2"), release)
            XCTAssertTrue(release.contains("(bound 40 ms)"), release)
            XCTAssertFalse(model.previewIsStale)
            XCTAssertNil(model.historicalPreview, "no historical mode: A's late completion is a plain stale update")
            XCTAssertNotNil(model.controllerState.lastEditToPreviewMs)
        }
    }

    func testHybridWithHistoricalPaintsTheSupersededCompileAsHistory() async throws {
        let h = try makeHarness(name: "hybrid-hist", historical: true)
        defer { teardown(h) }
        try await attach(h, policy: .hybrid)
        let model = h.model
        XCTAssertTrue(model.historicalNegotiated)
        model.updateActiveText("%hold\nA text\n")
        let revA = model.editorRevision
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 }
        XCTAssertNotNil(model.controllerState.inFlight, "hybrid holds even when negotiated (unlike plain historical mode)")
        model.updateActiveText("B text\n")
        let revB = model.editorRevision
        try await waitUntil("A painted as historical") { model.historicalPreview != nil }
        XCTAssertEqual(model.historicalPreview?.label, "revision \(revA) shown — revision \(revB) compiling")
        XCTAssertTrue(model.workerLog.contains { $0.contains("hybrid release") && $0.contains("revision \(revA)") })
        try await waitUntil("B replaces A") { model.historicalPreview == nil && model.result?.revision == revB }
        XCTAssertEqual(firstItemText(model), "B text")
    }

    /// Waits on a helper process (the Python preview-controller double); generous because a loaded runner
    /// delays process start-up and replies by seconds, not because anything is timed.
    private func waitUntil(_ what: String, timeout: TimeInterval = 30, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out after \(Int(timeout)) s waiting for \(what) from the helper double"); throw XCTSkip("timeout: \(what)") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }
}
