import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Exact compile correlation on the helper route (AdmissionCorrelation.swift;
/// crates/preview-controller/STDIO.md §"Edit admission correlation", helper
/// from main 55bcf12): the in-flight edit is released by the outcome of the
/// compile the helper ADMITTED for it (`compile_request_id`), never by
/// comparing the helper's compile generation with the document's durable
/// revision. The numeric comparison survives only as the fallback for a
/// helper whose edit reply carries no pair (the fake helper).
///
/// Real-helper cases need FLASHTEX_PREVIEW_CONTROLLER (main 55bcf12 or later)
/// and FLASHTEX_COMPILER; they skip otherwise. The rapid-edit case is timing
/// sensitive and skips when the 1-minute load average exceeds 20.
@MainActor
final class AdmissionCorrelationTests: XCTestCase {
    static let fakeHelper = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_preview_controller.py")

    static var realHelper: URL? {
        guard let p = ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"],
              FileManager.default.isExecutableFile(atPath: p) else { return nil }
        return URL(fileURLWithPath: p)
    }

    static var loadAverage1: Double {
        var load = [Double](repeating: 0, count: 3)
        getloadavg(&load, 3)
        return load[0]
    }

    // MARK: decision (no helper)

    private let admitted = ControllerCompileAdmission(requestID: "preview-7", compileRevision: 7)

    func testEditReplyPairDecodesAndNullsAreNil() {
        XCTAssertEqual(ControllerCompileAdmission.from(editResult: ["compile_request_id": "preview-3", "compile_revision": 3]),
                       ControllerCompileAdmission(requestID: "preview-3", compileRevision: 3))
        XCTAssertNil(ControllerCompileAdmission.from(editResult: ["compile_request_id": NSNull(), "compile_revision": NSNull()]), "submission failed: both null")
        XCTAssertNil(ControllerCompileAdmission.from(editResult: ["compile_request_id": "preview-3"]), "half a pair is no identity")
        XCTAssertNil(ControllerCompileAdmission.from(editResult: ["compile_revision": 3]))
        XCTAssertNil(ControllerCompileAdmission.from(editResult: [:]), "older helper / history result")
        XCTAssertNil(ControllerCompileAdmission.from(editResult: ["compile_request_id": "", "compile_revision": 3]))
    }

    func testAdmittedIdentityReleasesOnlyTheMatchingOutcome() {
        for kind in ["preview", "stale", "discarded", "failed", "cancelled"] {
            XCTAssertTrue(AdmissionCorrelation.decision(kind: kind, requestID: "preview-7", compileRevision: 7, admitted: admitted, durableRevision: 41).releases, kind)
            // failed/cancelled carry no compile_revision on the wire; the id suffices.
            XCTAssertTrue(AdmissionCorrelation.decision(kind: kind, requestID: "preview-7", compileRevision: nil, admitted: admitted, durableRevision: 41).releases, kind)
            // (e) forged / mismatched id: never released, whatever the numbers say.
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "preview-999", compileRevision: 999, admitted: admitted, durableRevision: 41).releases, kind)
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "preview-6", compileRevision: 6, admitted: admitted, durableRevision: 1).releases, "older compile, generation >= durable, \(kind)")
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: nil, compileRevision: 7, admitted: admitted, durableRevision: 41).releases, "no id, \(kind)")
            // Same id but a different generation is an identity mismatch, not a match.
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "preview-7", compileRevision: 8, admitted: admitted, durableRevision: 41).releases, kind)
            // Nothing releases before the edit is durable.
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "preview-7", compileRevision: 7, admitted: admitted, durableRevision: nil).releases, kind)
        }
        // A superseded rebinding carries no generation: the id alone matches.
        let rebound = ControllerCompileAdmission(requestID: "preview-9", compileRevision: nil)
        XCTAssertTrue(AdmissionCorrelation.decision(kind: "preview", requestID: "preview-9", compileRevision: 9, admitted: rebound, durableRevision: 41).releases)
        XCTAssertFalse(AdmissionCorrelation.decision(kind: "preview", requestID: "preview-7", compileRevision: 7, admitted: rebound, durableRevision: 41).releases)
        // The preview's compiled version is irrelevant once an admission is recorded.
        XCTAssertTrue(AdmissionCorrelation.decision(kind: "preview", requestID: "preview-7", compileRevision: 7, previewVersion: 1, admitted: admitted, durableRevision: 41).releases)
        XCTAssertFalse(AdmissionCorrelation.decision(kind: "preview", requestID: "preview-8", compileRevision: 8, previewVersion: 99, admitted: admitted, durableRevision: 41).releases)
        // Non-outcomes never release.
        XCTAssertFalse(AdmissionCorrelation.decision(kind: "superseded", requestID: "preview-7", compileRevision: nil, admitted: admitted, durableRevision: 41).releases)
        XCTAssertFalse(AdmissionCorrelation.decision(kind: "completed_snapshot", requestID: "preview-7", compileRevision: 7, admitted: admitted, durableRevision: 41).releases)
    }

    func testNumericFallbackWithoutAPairIsThePreviousBehaviour() {
        // (d) stale/discarded: compile_revision >= durable releases (the old guard), less does not.
        for kind in ["stale", "discarded"] {
            XCTAssertTrue(AdmissionCorrelation.decision(kind: kind, requestID: "x", compileRevision: 2, admitted: nil, durableRevision: 2).releases, kind)
            XCTAssertTrue(AdmissionCorrelation.decision(kind: kind, requestID: nil, compileRevision: 5, admitted: nil, durableRevision: 2).releases, kind)
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "x", compileRevision: 1, admitted: nil, durableRevision: 2).releases, kind)
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "x", compileRevision: nil, admitted: nil, durableRevision: 2).releases, kind)
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "x", compileRevision: 9, admitted: nil, durableRevision: nil).releases, "not durable, \(kind)")
        }
        // failed/cancelled: any durable in-flight edit is released (compiler session gone).
        for kind in ["failed", "cancelled"] {
            XCTAssertTrue(AdmissionCorrelation.decision(kind: kind, requestID: "x", compileRevision: nil, admitted: nil, durableRevision: 2).releases, kind)
            XCTAssertFalse(AdmissionCorrelation.decision(kind: kind, requestID: "x", compileRevision: nil, admitted: nil, durableRevision: nil).releases, kind)
        }
        // preview: the compiled version of the in-flight path must reach the durable revision.
        XCTAssertTrue(AdmissionCorrelation.decision(kind: "preview", requestID: "x", compileRevision: 1, previewVersion: 2, admitted: nil, durableRevision: 2).releases)
        XCTAssertTrue(AdmissionCorrelation.decision(kind: "preview", requestID: "x", compileRevision: 1, previewVersion: 3, admitted: nil, durableRevision: 2).releases)
        XCTAssertFalse(AdmissionCorrelation.decision(kind: "preview", requestID: "x", compileRevision: 1, previewVersion: 1, admitted: nil, durableRevision: 2).releases)
        XCTAssertFalse(AdmissionCorrelation.decision(kind: "preview", requestID: "x", compileRevision: 1, previewVersion: nil, admitted: nil, durableRevision: 2).releases, "another path only")
    }

    func testSupersededRebindsOnlyTheAdmittedCompile() {
        XCTAssertEqual(AdmissionCorrelation.rebinding(kind: "superseded", requestID: "preview-7", byID: "preview-8", admitted: admitted),
                       ControllerCompileAdmission(requestID: "preview-8", compileRevision: nil))
        XCTAssertNil(AdmissionCorrelation.rebinding(kind: "superseded", requestID: "preview-6", byID: "preview-8", admitted: admitted), "another compile")
        XCTAssertNil(AdmissionCorrelation.rebinding(kind: "superseded", requestID: "preview-7", byID: nil, admitted: admitted))
        XCTAssertNil(AdmissionCorrelation.rebinding(kind: "superseded", requestID: "preview-7", byID: "preview-7", admitted: admitted), "self")
        XCTAssertNil(AdmissionCorrelation.rebinding(kind: "superseded", requestID: "preview-7", byID: "preview-8", admitted: nil), "no admission: nothing to rebind")
        XCTAssertNil(AdmissionCorrelation.rebinding(kind: "stale", requestID: "preview-7", byID: "preview-8", admitted: admitted))
    }

    // MARK: real helper (main 55bcf12+) and real compiler

    private struct Harness {
        let model: ShellModel
        let root: URL
        let tex: URL
        let originalCompiler: String?
    }

    /// A compiler wrapper around the real one: requests carrying `slowMarker`
    /// are answered after `slowSeconds`; requests carrying `dieMarker` kill it.
    private func makeHarness(name: String, text: String, slowMarker: String? = nil, slowSeconds: Double = 0.7, dieMarker: String? = nil) throws -> Harness {
        guard let realCompiler = ShellModel.locateCompiler() else { throw XCTSkip("set FLASHTEX_COMPILER to a built binary") }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("adm-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try text.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let originalCompiler = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"]
        if slowMarker != nil || dieMarker != nil {
            var cases = ""
            if let dieMarker { cases += "    *\(dieMarker)*) exit 3;;\n" }
            if let slowMarker { cases += "    *\(slowMarker)*) sleep \(slowSeconds);;\n" }
            let script = root.appendingPathComponent("wrapped-compiler.sh")
            let body = """
            #!/bin/sh
            while IFS= read -r line; do
              case "$line" in
            \(cases)  esac
              printf '%s\\n' "$line" | "\(realCompiler.path)"
            done
            exit 0

            """
            try body.write(to: script, atomically: true, encoding: .utf8)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
            setenv("FLASHTEX_COMPILER", script.path, 1)
        }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        return Harness(model: model, root: root, tex: tex, originalCompiler: originalCompiler)
    }

    private func teardown(_ h: Harness) {
        h.model.detachController()
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
        if let c = h.originalCompiler { setenv("FLASHTEX_COMPILER", c, 1) } else { unsetenv("FLASHTEX_COMPILER") }
        try? FileManager.default.removeItem(at: h.root)
    }

    private func attach(_ h: Harness, _ helper: URL, policy: ControllerReleasePolicy = .holdUntilPreview) async throws {
        h.model.attachController(at: helper)
        h.model.controllerState.releasePolicy = policy
        XCTAssertTrue(h.model.controllerAttached)
        try await waitUntil("initial preview", 20) {
            h.model.result?.revision == h.model.editorRevision && h.model.previewSource == .worker("flashtex-preview-controller") && h.model.inFlightRevision == nil
        }
    }

    private func doc(_ body: String) -> String { "\\begin{document}\n\(body)\n\\end{document}\n" }

    private func releaseLines(_ model: ShellModel) -> [String] { model.workerLog.filter { $0.hasPrefix("controller release:") } }
    private func holdLines(_ model: ShellModel) -> [String] { model.workerLog.filter { $0.hasPrefix("controller hold:") } }

    /// (a) The edit reply carries a non-null pair; the preview with that
    /// request id releases the in-flight edit (logged by identity).
    func testEditReplyCarriesTheAdmittedPairAndItsPreviewReleases() async throws {
        guard let helper = Self.realHelper else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER to a helper built from main 55bcf12 or later") }
        let h = try makeHarness(name: "pair", text: doc("Hello admission."), slowMarker: "SLOWCOMPILE", slowSeconds: 0.5)
        defer { teardown(h) }
        try await attach(h, helper)
        let model = h.model
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1)

        model.updateActiveText(doc("Hello admission, SLOWCOMPILE edited."))
        let edited = model.editorRevision
        try await waitUntil("edit durable") { model.controllerState.durable["main.tex"]?.revision == 2 }
        let inFlight = try XCTUnwrap(model.controllerState.inFlight, "hold-until-preview keeps the durable edit in flight")
        XCTAssertEqual(inFlight.durableRevision, 2)
        let pair = try XCTUnwrap(inFlight.admitted, "the edit reply named the admitted compile: \(model.controllerStatus)")
        XCTAssertFalse(pair.requestID.isEmpty)
        let generation = try XCTUnwrap(pair.compileRevision)
        XCTAssertGreaterThan(generation, 0)

        try await waitUntil("its preview") { model.result?.revision == edited && model.controllerState.inFlight == nil }
        XCTAssertNil(model.inFlightRevision)
        let release = try XCTUnwrap(releaseLines(model).last, "the release is logged: \(model.workerLog.suffix(8))")
        XCTAssertTrue(release.contains("preview \(pair.requestID) is the admitted compile"), release)
        XCTAssertTrue(release.contains("generation \(generation), durable r2"), release)
        XCTAssertTrue(holdLines(model).isEmpty, "no mismatched outcome was seen: \(holdLines(model))")
    }

    /// (b) Two rapid edits: A's admitted compile is slow; a helper-side
    /// `compile` admitted behind it (a later admission of the same document)
    /// makes A's outcome `stale`/`discarded {A, compile_revision}` (the runtime
    /// reports a completed compile of an older revision as `stale`; the helper
    /// reports a current-revision result whose snapshot moved as `discarded`).
    /// That outcome releases A exactly once (B goes out); the intermediate
    /// compile's outcome, a different request id, is a logged hold and never
    /// releases B, which stays held until ITS OWN preview.
    func testRapidEditsReleaseExactlyOncePerAdmittedCompile() async throws {
        guard let helper = Self.realHelper else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER to a helper built from main 55bcf12 or later") }
        let load = Self.loadAverage1
        guard load <= 20 else { throw XCTSkip("1-minute load average \(load) > 20: the compile ordering of this case is timing sensitive") }
        let h = try makeHarness(name: "rapid", text: doc("Hello rapid."), slowMarker: "SLOWCOMPILE", slowSeconds: 0.8)
        defer { teardown(h) }
        try await attach(h, helper)
        let model = h.model

        // A: durable r2, admitted X (compiling slowly), held.
        model.updateActiveText(doc("Hello rapid, SLOWCOMPILE A."))
        let revA = model.editorRevision
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight?.admitted != nil }
        let admittedA = try XCTUnwrap(model.controllerState.inFlight?.admitted)
        // B typed while A is in flight: queued, not sent (hold policy).
        model.updateActiveText(doc("Hello rapid, B."))
        let revB = model.editorRevision
        XCTAssertTrue(model.controllerState.queued)
        XCTAssertEqual(model.controllerState.inFlight?.editorRevision, revA)
        // A later admission of the same document (an explicit compile of the
        // durable source) supersedes or invalidates X: X ends in `stale`/`discarded`
        // (completed after the generation moved on) or `superseded` (never dispatched).
        _ = try XCTUnwrap(model.controller).compile()

        try await waitUntil("A released and B sent", 15) { model.controllerState.inFlight?.editorRevision == revB }
        let releasesAfterA = releaseLines(model)
        XCTAssertEqual(releasesAfterA.count, 1, "A was released exactly once: \(releasesAfterA)")
        let releaseA = try XCTUnwrap(releasesAfterA.first)
        XCTAssertTrue(releaseA.contains("(in flight revision \(revA))"), releaseA)
        let rebound = model.workerLog.first { $0.contains("superseded admitted compile \(admittedA.requestID)") }
        if let rebound {
            // X never ran: the wait moved to the superseding compile, whose outcome released A.
            XCTAssertFalse(releaseA.contains(admittedA.requestID), "\(releaseA) / \(rebound)")
        } else {
            XCTAssertTrue(releaseA.contains("stale \(admittedA.requestID) is the admitted compile") || releaseA.contains("discarded \(admittedA.requestID) is the admitted compile"), releaseA)
            XCTAssertTrue(releaseA.contains("generation \(admittedA.compileRevision!)"), "the outcome kept X's compile_revision: \(releaseA)")
        }

        // B: durable r3 with its own admission Z; the intermediate compile's
        // outcome (a different id) must not release it.
        try await waitUntil("B durable") { model.controllerState.durable["main.tex"]?.revision == 3 && model.controllerState.inFlight?.admitted != nil }
        let admittedB = try XCTUnwrap(model.controllerState.inFlight?.admitted)
        XCTAssertNotEqual(admittedB.requestID, admittedA.requestID)
        try await waitUntil("B previewed", 15) { model.result?.revision == revB && model.controllerState.inFlight == nil }
        let releases = releaseLines(model)
        XCTAssertEqual(releases.count, 2, "one release per edit: \(releases)")
        guard releases.count == 2 else { return XCTFail("expected two releases, got \(releases.count)") }
        XCTAssertTrue(releases[1].contains("preview \(admittedB.requestID) is the admitted compile"), releases[1])
        XCTAssertTrue(releases[1].contains("(in flight revision \(revB))"), releases[1])
        let holds = holdLines(model)
        XCTAssertFalse(holds.isEmpty, "the intermediate compile's outcome was seen and held: \(model.workerLog.suffix(10))")
        for hold in holds {
            XCTAssertFalse(hold.contains(" \(admittedB.requestID) is not the admitted"), "B's own outcome was never held: \(hold)")
            XCTAssertFalse(hold.contains(" \(admittedA.requestID) is not the admitted"), "A's outcome was matched, not held: \(hold)")
        }
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[3], model.activeText)
    }

    /// (c) `failed` for the admitted compile releases the edit and surfaces.
    func testFailedOutcomeOfTheAdmittedCompileReleasesAndSurfaces() async throws {
        guard let helper = Self.realHelper else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER to a helper built from main 55bcf12 or later") }
        let h = try makeHarness(name: "failed", text: doc("Hello failure."), dieMarker: "KILLCOMPILER")
        defer { teardown(h) }
        try await attach(h, helper)
        let model = h.model

        model.updateActiveText(doc("Hello failure, KILLCOMPILER."))
        let edited = model.editorRevision
        // The compiler dies on this request: `failed` follows the durable reply
        // within milliseconds, so the release is read from the log, not polled.
        try await waitUntil("edit durable and released by failed") {
            model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil && model.inFlightRevision == nil
        }
        let failedLine = try XCTUnwrap(model.workerLog.first { $0.hasPrefix("controller failed compile ") }, "\(model.workerLog.suffix(8))")
        let failedID = String(failedLine.dropFirst("controller failed compile ".count).prefix { $0 != ":" })
        XCTAssertFalse(failedID.isEmpty, failedLine)
        let release = try XCTUnwrap(releaseLines(model).last, model.workerLog.suffix(8).joined(separator: "\n"))
        XCTAssertTrue(release.contains("failed \(failedID) is the admitted compile"), "released by the admitted id, not by a number: \(release)")
        XCTAssertTrue(release.contains("(in flight revision \(edited))"), release)
        XCTAssertTrue(model.controllerStatus.hasPrefix("preview failed: "), "surfaced: \(model.controllerStatus)")
        XCTAssertEqual(model.result?.revision, edited - 1, "the last good preview stays")
        // Typing continues: the next edit is sent and made durable (preview_error from the dead compiler session).
        model.updateActiveText(doc("Hello failure, afterwards."))
        try await waitUntil("next edit durable") { model.controllerState.durable["main.tex"]?.revision == 3 && model.inFlightRevision == nil }
    }

    /// (e) A forged `stale`/`discarded`/`failed` naming another request id
    /// (with a generation past the durable revision, which the old numeric
    /// guard would have accepted) does not release the edit; its own preview does.
    func testForgedMismatchedOutcomeDoesNotRelease() async throws {
        guard let helper = Self.realHelper else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER to a helper built from main 55bcf12 or later") }
        let h = try makeHarness(name: "forged", text: doc("Hello forgery."), slowMarker: "SLOWCOMPILE", slowSeconds: 1.0)
        defer { teardown(h) }
        try await attach(h, helper)
        let model = h.model

        model.updateActiveText(doc("Hello forgery, SLOWCOMPILE."))
        let edited = model.editorRevision
        try await waitUntil("edit durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight?.admitted != nil }
        let pair = try XCTUnwrap(model.controllerState.inFlight?.admitted)
        let forgedID = "forged-\(pair.requestID)"
        model.handleController(.update(kind: "stale", payload: ["request_id": forgedID, "compile_revision": pair.compileRevision! + 50]))
        model.handleController(.update(kind: "discarded", payload: ["request_id": forgedID, "compile_revision": 2]))
        model.handleController(.update(kind: "failed", payload: ["request_id": forgedID, "reason": "forged"]))
        model.handleController(.update(kind: "stale", payload: ["request_id": pair.requestID, "compile_revision": pair.compileRevision! + 1]))
        XCTAssertNotNil(model.controllerState.inFlight, "a mismatched id never releases: \(model.workerLog.suffix(6))")
        XCTAssertEqual(model.inFlightRevision, edited)
        XCTAssertTrue(releaseLines(model).isEmpty, "\(releaseLines(model))")
        let holds = holdLines(model)
        XCTAssertEqual(holds.count, 4, "\(holds)")
        guard holds.count == 4 else { return XCTFail("expected four holds, got \(holds.count)") }
        XCTAssertTrue(holds[0].contains("stale \(forgedID) is not the admitted compile \(pair.requestID)"), holds[0])
        XCTAssertTrue(holds[3].contains("carries compile_revision \(pair.compileRevision! + 1), admitted \(pair.compileRevision!)"), holds[3])

        try await waitUntil("its own preview") { model.result?.revision == edited && model.controllerState.inFlight == nil }
        let finalReleases = releaseLines(model)
        XCTAssertEqual(finalReleases.count, 1)
        let finalRelease = try XCTUnwrap(finalReleases.first)
        XCTAssertTrue(finalRelease.contains("preview \(pair.requestID) is the admitted compile"))
    }

    // MARK: fake helper (edit replies without the pair)

    /// (d) The fake helper's edit result carries no pair: nothing is recorded
    /// and the numeric fallback releases a `stale` whose generation reaches
    /// the durable revision, exactly as before.
    func testNumericFallbackAgainstAHelperWithoutThePair() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("adm-fallback-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        try "Hello\n".write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: Self.fakeHelper)
        defer { model.detachController() }
        model.controllerState.releasePolicy = .holdUntilPreview
        try await waitUntil("initial preview") { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }

        model.updateActiveText("%hold\nA text\n")
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 }
        let inFlight = try XCTUnwrap(model.controllerState.inFlight)
        XCTAssertNil(inFlight.admitted, "the fake helper's edit reply has no compile_request_id/compile_revision")
        XCTAssertEqual(inFlight.durableRevision, 2)
        // A mismatched request id does not matter without an admission; a generation below the durable revision holds…
        model.handleController(.update(kind: "stale", payload: ["request_id": "whatever", "compile_revision": 1]))
        XCTAssertNotNil(model.controllerState.inFlight)
        XCTAssertTrue(releaseLines(model).isEmpty)
        // …and one reaching it releases (the pre-correlation guard).
        model.handleController(.update(kind: "discarded", payload: ["request_id": "whatever", "compile_revision": 2]))
        XCTAssertNil(model.controllerState.inFlight)
        XCTAssertNil(model.inFlightRevision)
        let release = try XCTUnwrap(releaseLines(model).last)
        XCTAssertTrue(release.contains("discarded generation 2 >= durable r2 (numeric fallback, no admission recorded)"), release)
    }

    private func waitUntil(_ what: String, _ timeout: TimeInterval = 10, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out waiting for \(what)"); throw XCTSkip("timeout: \(what)") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }
}
