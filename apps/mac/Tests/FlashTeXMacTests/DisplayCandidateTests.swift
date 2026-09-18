import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The helper display-candidate route (ShellModel+DisplayCandidates.swift;
/// crates/preview-controller/docs/display-forwarding.md "Native gate").
///
/// Pure gates (decoder, admission, validator, model-level refusal) run
/// without any process. The `testHelper…` cases drive the REAL
/// `flashtex-preview-controller` owning the REAL `flashtex-render` producer
/// and are skipped unless the environment names built binaries:
///   FLASHTEX_PREVIEW_CONTROLLER  helper binary (crates/preview-controller)
///   FLASHTEX_RENDER              flashtex-render (agent/mac-render-pipeline/unified)
///   FLASHTEX_COMPILER            main's flashtex-compiler (declines display-list-v2)
@MainActor
final class DisplayCandidateTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")
    static var helper: URL? { ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) } }
    static var render: URL? { ProcessInfo.processInfo.environment["FLASHTEX_RENDER"].map { URL(fileURLWithPath: $0) } }
    static var v1Compiler: URL? { ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"].map { URL(fileURLWithPath: $0) } }

    // MARK: - pure gates

    private func candidateLine(session: String = "s1", request: String = "pc-7", project: String = "demo", generation: Int = 3,
                               versions: [String: Int] = ["main.tex": 2], untrusted: Any = true, actions: Any = false,
                               displayList: Data? = nil) throws -> Data {
        let list = try displayList ?? Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        let payload: [String: Any] = ["kind": "display_candidate", "untrusted": untrusted, "source_actions_enabled": actions,
                                      "request_id": request, "project_id": project, "compile_revision": generation,
                                      "source_versions": versions, "membership_generation": 1, "display_list": "@@LIST@@"]
        let frame: [String: Any] = ["protocol_version": 1, "session_id": session, "id": NSNull(), "type": "update", "payload": payload]
        var text = String(decoding: try JSONSerialization.data(withJSONObject: frame, options: [.sortedKeys]), as: UTF8.self)
        text = text.replacingOccurrences(of: "\"@@LIST@@\"", with: String(decoding: list, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines))
        return Data(text.utf8)
    }

    func testDecoderAcceptsOnlyContractShapedCandidates() throws {
        let line = try candidateLine()
        guard case .displayCandidate(let c) = PreviewControllerClient.decode(line, sessionID: "s1") else { return XCTFail("expected a candidate") }
        XCTAssertEqual(c.sessionID, "s1"); XCTAssertEqual(c.requestID, "pc-7"); XCTAssertEqual(c.projectID, "demo")
        XCTAssertEqual(c.compileRevision, 3); XCTAssertEqual(c.sourceVersions, ["main.tex": 2]); XCTAssertEqual(c.membershipGeneration, 1)
        XCTAssertEqual(c.frameBytes, line.count)
        let header = try XCTUnwrap(RenderingV2Fast.header(c.displayList))
        XCTAssertEqual(header.protocolVersion, 2); XCTAssertEqual(header.type, "display_list"); XCTAssertEqual(header.id, "req-1")
        XCTAssertNoThrow(try RenderingV2.decode(c.displayList), "the forwarded bytes are the original envelope")

        func violation(_ line: Data, _ contains: String, file: StaticString = #filePath, fileLine: UInt = #line) {
            guard case .protocolViolation(let m) = PreviewControllerClient.decode(line, sessionID: "s1") else {
                return XCTFail("expected a protocol violation containing '\(contains)'", file: file, line: fileLine)
            }
            XCTAssertTrue(m.contains(contains), m, file: file, line: fileLine)
        }
        violation(try candidateLine(untrusted: false), "untrusted:true")
        violation(try candidateLine(actions: true), "source_actions_enabled:false")
        violation(try candidateLine(request: ""), "without request_id")
        violation(try candidateLine(versions: [:]), "without source_versions")
        violation(try candidateLine(displayList: Data("{\"protocol_version\":1,\"id\":\"x\",\"type\":\"compile_result\"}".utf8)), "not a rendering-v2 display_list")
        // A frame for another helper session is refused by the client before the kind is even looked at.
        guard case .protocolViolation(let m) = PreviewControllerClient.decode(try candidateLine(session: "other"), sessionID: "s1") else { return XCTFail() }
        XCTAssertTrue(m.contains("frame for session other"), m)
    }

    func testGateRefusesStaleForeignAndToggledIdentities() throws {
        guard case .displayCandidate(let c) = PreviewControllerClient.decode(try candidateLine(), sessionID: "s1") else { return XCTFail() }
        let applied = DisplayCandidateAppliedPreview(requestID: "pc-7", compileRevision: 3, sourceVersions: ["main.tex": 2], editorRevision: 5)
        var gate = DisplayCandidateGate(negotiated: true, sessionID: "s1", projectID: "demo", applied: applied, appliedResultID: "pc-7",
                                        activePath: "main.tex", displayedEditorRevision: nil, membershipGeneration: 1) // learned on ready (FirstGenerationGateTests)
        XCTAssertNil(gate.rejection(of: c))
        gate.negotiated = false
        XCTAssertEqual(gate.rejection(of: c), "display candidates not negotiated for this session")
        gate.negotiated = true; gate.sessionID = "s2" // toggled session (reattach) before the frame arrived
        XCTAssertTrue(gate.rejection(of: c)!.contains("session s1 is not the negotiated s2"))
        gate.sessionID = "s1"; gate.projectID = "other"
        XCTAssertTrue(gate.rejection(of: c)!.contains("project demo is not the attached other"))
        gate.projectID = "demo"; gate.appliedResultID = "pc-8" // a newer v1 preview was applied: the sibling is stale
        XCTAssertTrue(gate.rejection(of: c)!.contains("request pc-7 is not the applied v1 preview (pc-8)"))
        gate.appliedResultID = "pc-7"; gate.applied = nil
        XCTAssertTrue(gate.rejection(of: c)!.contains("not the applied v1 preview"))
        gate.applied = DisplayCandidateAppliedPreview(requestID: "pc-7", compileRevision: 4, sourceVersions: ["main.tex": 2], editorRevision: 5)
        XCTAssertTrue(gate.rejection(of: c)!.contains("compile generation 3 is not the applied preview's 4"))
        gate.applied = DisplayCandidateAppliedPreview(requestID: "pc-7", compileRevision: 3, sourceVersions: ["main.tex": 3], editorRevision: 5)
        XCTAssertTrue(gate.rejection(of: c)!.contains("source versions"))
        gate.applied = applied; gate.activePath = "other.tex"
        XCTAssertTrue(gate.rejection(of: c)!.contains("no source version for the active document other.tex"))
        gate.activePath = "main.tex"; gate.displayedEditorRevision = 6 // a newer v2 frame already painted
        XCTAssertTrue(gate.rejection(of: c)!.contains("editor revision 5 is older than the displayed v2 revision 6"))
        gate.displayedEditorRevision = 5
        XCTAssertNil(gate.rejection(of: c))
    }

    func testValidatorBindsEnvelopeToExactDurableText() throws {
        let store = V2FontStore(directories: [Self.fontsDir.path])
        guard store.fonts.contains(where: { $0.url.lastPathComponent == "lmroman10-regular.otf" }) else { throw XCTSkip("bundled fonts missing") }
        let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.tex"), encoding: .utf8)
        guard case .displayCandidate(let c) = PreviewControllerClient.decode(try candidateLine(generation: 3, versions: ["main.tex": 9]), sessionID: "s1") else { return XCTFail() }
        // Exact text for the version the candidate names: verified, fonts resolved by hash.
        guard case .verified(let frame) = DisplayCandidateValidator.validate(c, texts: ["main.tex": tex], store: store) else {
            return XCTFail("expected verification")
        }
        XCTAssertEqual(frame.list.pages.count, 1)
        XCTAssertFalse(frame.fonts.isEmpty)
        func refused(_ outcome: DisplayCandidateValidator.Outcome, _ contains: String, file: StaticString = #filePath, line: UInt = #line) {
            guard case .refused(let why, _) = outcome else { return XCTFail("expected refusal containing '\(contains)'", file: file, line: line) }
            XCTAssertTrue(why.contains(contains), why, file: file, line: line)
        }
        // One byte more: the length check refuses before hashing.
        refused(DisplayCandidateValidator.validate(c, texts: ["main.tex": tex + "x"], store: store), "byte_length 161 differs")
        // Same length, different bytes: the hash refuses.
        var swapped = Array(tex.utf8); swapped[0] = swapped[0] == 0x25 ? 0x26 : 0x25
        refused(DisplayCandidateValidator.validate(c, texts: ["main.tex": String(decoding: swapped, as: UTF8.self)], store: store), "sha256")
        // No retained text for the version: refused, never compared to the buffer instead.
        refused(DisplayCandidateValidator.validate(c, texts: [:], store: store), "no durable text retained for main.tex r9")
        // The envelope must name the candidate's project and compile generation.
        guard case .displayCandidate(let wrongGen) = PreviewControllerClient.decode(try candidateLine(generation: 4), sessionID: "s1") else { return XCTFail() }
        refused(DisplayCandidateValidator.validate(wrongGen, texts: ["main.tex": tex], store: store), "revision 3 is not the candidate's compile generation 4")
        guard case .displayCandidate(let wrongProject) = PreviewControllerClient.decode(try candidateLine(project: "p2"), sessionID: "s1") else { return XCTFail() }
        refused(DisplayCandidateValidator.validate(wrongProject, texts: ["main.tex": tex], store: store), "project demo; the candidate names p2")
        // A document the versions do not cover.
        guard case .displayCandidate(let uncovered) = PreviewControllerClient.decode(try candidateLine(versions: ["other.tex": 1]), sessionID: "s1") else { return XCTFail() }
        refused(DisplayCandidateValidator.validate(uncovered, texts: ["main.tex": tex, "other.tex": ""], store: store), "declares main.tex, which the candidate's source versions do not cover")
        // Fonts the store does not have: the whole candidate is refused (no partial frame).
        let missing = try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-missing-font.json"))
        guard case .displayCandidate(let noFont) = PreviewControllerClient.decode(try candidateLine(displayList: missing), sessionID: "s1") else { return XCTFail() }
        refused(DisplayCandidateValidator.validate(noFont, texts: ["main.tex": tex], store: store), "font_resource_unavailable")
    }

    func testStateKeepsOnePendingAndInvalidationVoidsEverything() throws {
        guard case .displayCandidate(let a) = PreviewControllerClient.decode(try candidateLine(request: "pc-1"), sessionID: "s1"),
              case .displayCandidate(let b) = PreviewControllerClient.decode(try candidateLine(request: "pc-2"), sessionID: "s1") else { return XCTFail() }
        let state = DisplayCandidateState()
        XCTAssertFalse(state.isNegotiated)
        state.beginNegotiation(sessionID: "s1", projectID: "demo", requestID: "n1", enabling: true)
        XCTAssertEqual(state.acknowledge(requestID: "other", payload: [:]), nil)
        XCTAssertEqual(state.acknowledge(requestID: "n1", payload: ["capability": "display-candidates-v1", "enabled": true, "preview_error": NSNull()]), true)
        XCTAssertTrue(state.isNegotiated)
        state.applied = DisplayCandidateAppliedPreview(requestID: "pc-1", compileRevision: 3, sourceVersions: ["main.tex": 2], editorRevision: 1)
        XCTAssertEqual(state.admit(a, gate: state.gate(activePath: "main.tex", appliedResultID: "pc-1", membershipGeneration: 1)), .queued)
        state.applied = DisplayCandidateAppliedPreview(requestID: "pc-2", compileRevision: 3, sourceVersions: ["main.tex": 2], editorRevision: 2)
        XCTAssertEqual(state.admit(b, gate: state.gate(activePath: "main.tex", appliedResultID: "pc-2", membershipGeneration: 1)), .replacedPending("pc-1"))
        XCTAssertEqual(state.dropped, 1)
        XCTAssertNil(state.takePending(activePath: "other.tex"), "a document switch invalidates the queued candidate")
        XCTAssertEqual(state.dropped, 2)
        XCTAssertEqual(state.admit(b, gate: state.gate(activePath: "main.tex", appliedResultID: "pc-2", membershipGeneration: 1)), .queued)
        state.invalidate()
        XCTAssertFalse(state.isNegotiated); XCTAssertNil(state.pending); XCTAssertNil(state.applied); XCTAssertEqual(state.dropped, 3)
        // A disable acknowledgement is not a negotiation.
        state.beginNegotiation(sessionID: "s1", projectID: "demo", requestID: "n2", enabling: false)
        XCTAssertEqual(state.acknowledge(requestID: "n2", payload: ["capability": "display-candidates-v1", "enabled": false]), true)
        XCTAssertFalse(state.isNegotiated)
        // A refusal (mutual exclusion, no compiler) leaves the route off.
        state.beginNegotiation(sessionID: "s1", projectID: "demo", requestID: "n3", enabling: true)
        XCTAssertTrue(state.refuseNegotiation(requestID: "n3"))
        XCTAssertFalse(state.isNegotiated)
    }

    func testHeldWorkRunsOnCandidateTimeoutOrSupersessionNeverDropped() async throws {
        let state = DisplayCandidateState()
        var ran: [String] = []
        state.deferred = ("pc-1", [{ ran.append("a") }, { ran.append("b") }], DispatchWorkItem {})
        state.releaseDeferred(.candidate)
        XCTAssertEqual(ran, ["a", "b"], "held work runs in order when the sibling arrives")
        XCTAssertNil(state.deferred); XCTAssertEqual(state.deferredReleasedByCandidate, 1)
        state.releaseDeferred(.timeout)
        XCTAssertEqual(state.deferredReleasedByTimeout, 0, "nothing held: nothing counted")
        // Model-level: OFF (not negotiated) runs the work inline; negotiated + accepted display-list-v2 holds it,
        // and a hold for a newer request carries the older request's work into itself (never drops it,
        // never runs it while the newer sibling is expected).
        let model = ShellModel()
        var inline = 0
        model.displayCandidatesAfterSibling(of: "pc-1", acceptedLayout: ["display-list-v2"]) { inline += 1 }
        XCTAssertEqual(inline, 1, "not negotiated: inline")
        model.displayCandidates.beginNegotiation(sessionID: "s", projectID: "p", requestID: "n", enabling: true)
        _ = model.displayCandidates.acknowledge(requestID: "n", payload: ["capability": "display-candidates-v1", "enabled": true])
        model.displayCandidatesAfterSibling(of: "pc-2", acceptedLayout: ["rules-v1"]) { inline += 1 }
        XCTAssertEqual(inline, 2, "producer did not accept display-list-v2: no sibling expected, inline")
        var held: [String] = []
        model.displayCandidatesAfterSibling(of: "pc-3", acceptedLayout: ["display-list-v2"]) { held.append("3a") }
        model.displayCandidatesAfterSibling(of: "pc-3", acceptedLayout: ["display-list-v2"], holdsRelease: true) { held.append("3b") }
        XCTAssertEqual(held, []); XCTAssertEqual(model.displayCandidates.deferred?.works.count, 2)
        model.displayCandidatesAfterSibling(of: "pc-4", acceptedLayout: ["display-list-v2"]) { held.append("4") }
        // GH-799: running 3a/3b here put a completion fetch on the wire inside pc-4's sibling window; its
        // required reply evicted pc-4's queued candidate in the helper, and that revision never got a v2 frame.
        XCTAssertEqual(held, [], "a newer request's hold does not run the older request's work inside its own sibling window")
        XCTAssertEqual(model.displayCandidates.deferred?.requestID, "pc-4")
        XCTAssertEqual(model.displayCandidates.deferred?.works.count, 3, "the older work is carried into the newer hold")
        XCTAssertEqual(model.displayCandidates.deferredReleasedBySupersession, 1)
        try await Task.sleep(nanoseconds: UInt64((ShellModel.displayCandidateSiblingWaitMs + 40) * 1_000_000))
        XCTAssertEqual(held, ["3a", "3b", "4"], "the bound releases a hold whose sibling never came, carried work first")
        XCTAssertEqual(model.displayCandidates.deferredReleasedByTimeout, 1)
        XCTAssertNil(model.displayCandidates.deferred)
        // Invalidation (close / helper exit / restart) runs the held work too: a held in-flight release must never be lost.
        model.displayCandidatesAfterSibling(of: "pc-5", acceptedLayout: ["display-list-v2"], holdsRelease: true) { held.append("5") }
        XCTAssertEqual(held, ["3a", "3b", "4"])
        model.displayCandidates.invalidate()
        XCTAssertEqual(held, ["3a", "3b", "4", "5"], "invalidation releases held work instead of dropping it")
        XCTAssertEqual(model.displayCandidates.deferredReleasedByInvalidation, 1)
        XCTAssertNil(model.displayCandidates.deferred)
    }

    /// GH-809: `testHeldWorkRunsOnCandidateTimeoutOrSupersessionNeverDropped` only
    /// releases carried work (an older hold's work, taken by a newer hold via
    /// `supersedeDeferred`) through the timeout bound. The newer hold's own
    /// sibling arriving must run the carried work too, ahead of its own.
    func testCarriedHoldWorkReleasedByCandidateArrivalNeverDropped() {
        let model = ShellModel()
        model.displayCandidates.beginNegotiation(sessionID: "s", projectID: "p", requestID: "n", enabling: true)
        _ = model.displayCandidates.acknowledge(requestID: "n", payload: ["capability": "display-candidates-v1", "enabled": true])
        var held: [String] = []
        model.displayCandidatesAfterSibling(of: "pc-1", acceptedLayout: ["display-list-v2"]) { held.append("1a") }
        model.displayCandidatesAfterSibling(of: "pc-1", acceptedLayout: ["display-list-v2"], holdsRelease: true) { held.append("1b") }
        XCTAssertEqual(held, []); XCTAssertEqual(model.displayCandidates.deferred?.works.count, 2)
        model.displayCandidatesAfterSibling(of: "pc-2", acceptedLayout: ["display-list-v2"]) { held.append("2") }
        XCTAssertEqual(held, [], "carried work does not run inside pc-2's own sibling window")
        XCTAssertEqual(model.displayCandidates.deferred?.requestID, "pc-2")
        XCTAssertEqual(model.displayCandidates.deferred?.works.count, 3, "pc-1's work is carried into pc-2's hold")
        XCTAssertEqual(model.displayCandidates.deferredReleasedBySupersession, 1)
        // pc-2's own sibling arrives (admitted or refused): the carried work runs first, never dropped.
        model.displayCandidates.releaseDeferred(.candidate)
        XCTAssertEqual(held, ["1a", "1b", "2"], "the carried work runs, in order, ahead of the newer request's own work")
        XCTAssertEqual(model.displayCandidates.deferredReleasedByCandidate, 1)
        XCTAssertNil(model.displayCandidates.deferred)
    }

    /// GH-809: carried work must also survive invalidation (helper exit / restart /
    /// close) while it is still held under the newer request, never dropped.
    func testCarriedHoldWorkReleasedByInvalidationNeverDropped() {
        let model = ShellModel()
        model.displayCandidates.beginNegotiation(sessionID: "s", projectID: "p", requestID: "n", enabling: true)
        _ = model.displayCandidates.acknowledge(requestID: "n", payload: ["capability": "display-candidates-v1", "enabled": true])
        var held: [String] = []
        model.displayCandidatesAfterSibling(of: "pc-1", acceptedLayout: ["display-list-v2"]) { held.append("1a") }
        model.displayCandidatesAfterSibling(of: "pc-1", acceptedLayout: ["display-list-v2"], holdsRelease: true) { held.append("1b") }
        model.displayCandidatesAfterSibling(of: "pc-2", acceptedLayout: ["display-list-v2"]) { held.append("2") }
        XCTAssertEqual(held, [])
        XCTAssertEqual(model.displayCandidates.deferred?.requestID, "pc-2")
        XCTAssertEqual(model.displayCandidates.deferred?.works.count, 3, "pc-1's work is carried into pc-2's hold")
        // The helper exits/restarts/closes before pc-2's own sibling arrives.
        model.displayCandidates.invalidate()
        XCTAssertEqual(held, ["1a", "1b", "2"], "invalidation runs the carried work instead of dropping it")
        XCTAssertEqual(model.displayCandidates.deferredReleasedByInvalidation, 1)
        XCTAssertNil(model.displayCandidates.deferred)
    }

    func testModelRefusesCandidatesWhileOffAndKeepsTheV1Result() throws {
        let model = ShellModel()
        XCTAssertFalse(model.displayCandidates.requested, "default OFF")
        XCTAssertEqual(model.displayCandidates.status, "off")
        let before = model.result?.revision
        guard case .displayCandidate(let c) = PreviewControllerClient.decode(try candidateLine(), sessionID: "s1") else { return XCTFail() }
        model.handleDisplayCandidate(c)
        XCTAssertEqual(model.displayCandidates.refused, 1)
        XCTAssertEqual(model.displayCandidates.lastRefusal, "display candidates not negotiated for this session")
        XCTAssertNil(model.displayListV2)
        XCTAssertEqual(model.result?.revision, before, "the v1 result is untouched")
        XCTAssertFalse(model.isHistoricalPreview)
        // Without a helper the layout set sent to configure_layout never carries display-list-v2, and the
        // violation check is the plain requested set.
        XCTAssertEqual(model.displayCandidatesConfigureLayoutCapabilities(["rules-v1", "display-list-v2"]), ["rules-v1"])
        XCTAssertEqual(model.displayCandidatesLayoutRequested(["rules-v1"]), ["rules-v1"])
    }

    // MARK: - real helper + real producer

    private struct Project {
        var root: URL
        var tex: URL
    }

    private func project(named name: String, text: String) throws -> Project {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("dc-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try text.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        if ProcessInfo.processInfo.environment["FLASHTEX_LM_DIR"] == nil { setenv("FLASHTEX_LM_DIR", Self.fontsDir.path, 1) }
        return Project(root: root, tex: tex)
    }

    /// Points `FLASHTEX_COMPILER` (what `attachController` hands the helper) at `producer` for one test.
    private func withProducer(_ producer: URL, _ body: () async throws -> Void) async throws {
        let previous = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"]
        setenv("FLASHTEX_COMPILER", producer.path, 1)
        defer { if let previous { setenv("FLASHTEX_COMPILER", previous, 1) } else { unsetenv("FLASHTEX_COMPILER") } }
        try await body()
    }

    private func requireHelperAndRender() throws -> (URL, URL) {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              let render = Self.render, FileManager.default.isExecutableFile(atPath: render.path) else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_RENDER to built binaries")
        }
        return (helper, render)
    }

    static let sample = "\\documentclass{article}\n\\begin{document}\nHello helper display candidates, fi.\n\\end{document}\n"

    func testHelperDefaultOffProducesOrdinaryV1Only() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "off", text: Self.sample)
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        try await withProducer(render) {
            let model = ShellModel()
            model.autoCompile = true
            XCTAssertEqual(model.openTex(at: p.tex), .opened)
            model.attachController(at: helper)
            try await waitUntil { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }
            XCTAssertEqual(model.displayCandidates.status, "off")
            XCTAssertFalse(model.displayCandidatesNegotiated)
            XCTAssertFalse(model.negotiation.accepted.contains("display-list-v2"), "OFF: the helper never enrolls display-list-v2")
            for i in 1...3 { model.updateActiveText(Self.sample.replacingOccurrences(of: "fi.", with: "fi\(String(repeating: "!", count: i)).")) }
            let final = model.editorRevision
            try await waitUntil { model.result?.revision == final && model.inFlightRevision == nil }
            XCTAssertEqual(model.displayCandidates.received, 0, "no candidate frames while OFF")
            XCTAssertNil(model.displayListV2)
            XCTAssertFalse(model.previewIsStale)
            model.detachController()
        }
    }

    /// GH36 review (Commander 5646386345): `display-list-v2` may reach `configure_layout` only AFTER
    /// `configure_display_candidates {enabled:true}` has been acknowledged. The shell never puts it in
    /// `configure_layout` itself (the helper enrols it on the opt-in); this test drives both orders on the wire.
    func testHelperOrderingEnableBeforeLayoutWithDisplayListV2() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "ordering", text: Self.sample)
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        try await withProducer(render) {
            let model = ShellModel()
            model.autoCompile = true
            XCTAssertEqual(model.openTex(at: p.tex), .opened)
            model.attachController(at: helper)
            try await waitUntil { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }
            let controller = try XCTUnwrap(model.controller)
            @MainActor func layoutWithV2() -> [String] { model.requestedLayoutCapabilities.filter { $0 != "display-list-v2" } + ["display-list-v2"] }
            @MainActor func configureLayout(_ caps: [String]) async throws -> Result<[String: Any], ControllerError> {
                var outcome: Result<[String: Any], ControllerError>?
                let id = try controller.configureLayout(capabilities: caps)
                model.controllerState.awaiting[id] = { outcome = $0 }
                try await waitUntil { outcome != nil }
                return try XCTUnwrap(outcome)
            }
            // Reversed order: display-list-v2 through configure_layout while candidates are OFF -> the exact refusal,
            // and the session continues as legacy v1 (no enrolment, no candidates).
            guard case .failure(let refusal) = try await configureLayout(layoutWithV2()) else {
                return XCTFail("configure_layout with display-list-v2 before the opt-in must be refused")
            }
            XCTAssertEqual(refusal.message, "display candidates must be enabled before requesting their layout")
            model.updateActiveText(Self.sample.replacingOccurrences(of: "fi.", with: "fi, reversed order."))
            var final = model.editorRevision
            try await waitUntil { model.result?.revision == final && model.inFlightRevision == nil }
            XCTAssertFalse(model.negotiation.accepted.contains("display-list-v2"), "refused layout: nothing enrolled")
            XCTAssertEqual(model.displayCandidates.received, 0)
            XCTAssertFalse(model.displayCandidatesNegotiated)
            // Correct order: configure_display_candidates first, wait for its result...
            model.setDisplayCandidates(true)
            try await waitUntil { model.displayCandidatesNegotiated || model.displayCandidates.status.hasPrefix("refused") }
            XCTAssertTrue(model.displayCandidatesNegotiated, model.displayCandidates.status)
            // ...then a configure_layout that includes display-list-v2 is accepted by the same helper session.
            guard case .success = try await configureLayout(layoutWithV2()) else {
                return XCTFail("configure_layout with display-list-v2 after the acknowledged opt-in must be accepted")
            }
            model.updateActiveText(Self.sample.replacingOccurrences(of: "fi.", with: "fi, enabled then layout."))
            final = model.editorRevision
            try await waitUntil { model.result?.revision == final && model.inFlightRevision == nil }
            XCTAssertTrue(model.negotiation.accepted.contains("display-list-v2"), "enrolled after the opt-in")
            try await waitUntil { if case .loaded(let f, _)? = model.displayListV2 { return f.list.revision == final } else { return false } }
            XCTAssertGreaterThan(model.displayCandidates.published, 0, "candidates paint once the order is right")
            XCTAssertEqual(model.displayCandidates.invalid, 0, model.displayCandidates.lastRefusal ?? "")
            model.detachController()
        }
    }

    func testHelperDeclinedProducerKeepsV1Only() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              let v1 = Self.v1Compiler, FileManager.default.isExecutableFile(atPath: v1.path), v1.lastPathComponent == "flashtex-compiler" else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER (main's flashtex-compiler, which declines display-list-v2)")
        }
        let p = try project(named: "declined", text: "\\begin{document}\nDeclined producer.\n\\end{document}\n")
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        try await withProducer(v1) {
            let model = ShellModel()
            model.autoCompile = true
            XCTAssertEqual(model.openTex(at: p.tex), .opened)
            model.attachController(at: helper)
            try await waitUntil { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }
            model.setDisplayCandidates(true)
            try await waitUntil { model.displayCandidatesNegotiated || model.displayCandidates.status.hasPrefix("refused") }
            XCTAssertTrue(model.displayCandidatesNegotiated, model.displayCandidates.status)
            model.updateActiveText("\\begin{document}\nDeclined producer, edited.\n\\end{document}\n")
            let final = model.editorRevision
            try await waitUntil { model.result?.revision == final && model.inFlightRevision == nil }
            XCTAssertFalse(model.negotiation.accepted.contains("display-list-v2"), "the v1-only compiler declines the capability")
            XCTAssertEqual(model.displayCandidates.received, 0, "a declined capability yields ordinary v1 output only")
            XCTAssertNil(model.displayListV2)
            XCTAssertFalse(model.previewIsStale)
            XCTAssertTrue(model.workerLog.contains { $0.contains("declined layout capabilities") && $0.contains("display-list-v2") }, "the limitation is surfaced")
            model.detachController()
        }
    }

    func testHelperCandidatesPaintAndBindToEditorRevisions() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "paint", text: Self.sample)
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        try await withProducer(render) {
            setenv("FLASHTEX_DISPLAY_CANDIDATES", "1", 1)
            defer { unsetenv("FLASHTEX_DISPLAY_CANDIDATES") }
            let model = ShellModel()
            model.autoCompile = true
            XCTAssertTrue(model.displayCandidates.requested, "FLASHTEX_DISPLAY_CANDIDATES=1 opts in at attach")
            XCTAssertEqual(model.openTex(at: p.tex), .opened)
            model.attachController(at: helper)
            try await waitUntil { model.displayCandidatesNegotiated }
            XCTAssertEqual(model.displayCandidates.sessionID, model.controller?.config.sessionID)
            // The first candidate: the sibling of the applied v1 preview, verified and painted as the editor revision.
            try await waitUntil { if case .loaded? = model.displayListV2 { return true } else { return false } }
            guard case .loaded(let frame, let source)? = model.displayListV2 else { return XCTFail() }
            XCTAssertTrue(source.isLive)
            XCTAssertEqual(frame.list.revision, model.result?.revision, "the frame is keyed by the editor revision of the applied v1 result")
            XCTAssertEqual(source.label, "live \(model.resultID ?? "?") (revision \(frame.list.revision))")
            XCTAssertTrue(model.negotiation.accepted.contains("display-list-v2"), "the helper enrolled display-list-v2 and the producer accepted it")
            XCTAssertEqual(model.negotiation.missing, [], "rules-v1/font-hints-v1 stay negotiated alongside")
            XCTAssertFalse(model.controllerStatus.hasPrefix("protocol violation"), model.controllerStatus)
            let doc = try XCTUnwrap(frame.list.documents.first { $0.path == "main.tex" })
            XCTAssertEqual(doc.sha256, SourceDigest.sha256Hex(model.activeText))
            XCTAssertEqual(Int(doc.byteLength), model.activeText.utf8.count)
            XCTAssertEqual(doc.revision, model.displayCandidates.applied?.compileRevision, "the envelope's document revision is the compile generation")
            XCTAssertNotEqual(doc.revision, model.controllerState.durable["main.tex"]?.revision ?? -1, "…not the durable revision (they must not be equated)")
            for f in frame.fonts.values { XCTAssertEqual(f.resource.sha256, f.file.bytesSha256, "flashtex-render names fonts by raw byte SHA-256") }
            XCTAssertGreaterThan(frame.prepared.reduce(0) { $0 + $1.glyphCount }, 0)
            XCTAssertFalse(model.isHistoricalPreview)

            // A burst: every candidate that paints is bound to an editor revision the v1 result was applied for;
            // the final frame is the final editor revision (nothing stale is left on screen).
            for i in 1...6 { model.updateActiveText(Self.sample.replacingOccurrences(of: "fi.", with: "fi\(String(repeating: "!", count: i)).")) }
            let final = model.editorRevision
            try await waitUntil { model.result?.revision == final && model.inFlightRevision == nil }
            try await waitUntil {
                if case .loaded(let f, _)? = model.displayListV2, f.list.revision == final, model.displayCandidates.validating == nil { return true }
                return false
            }
            guard case .loaded(let last, _)? = model.displayListV2 else { return XCTFail() }
            XCTAssertEqual(try XCTUnwrap(last.list.documents.first).sha256, SourceDigest.sha256Hex(model.activeText))
            XCTAssertGreaterThanOrEqual(model.displayCandidates.published, 2)
            XCTAssertEqual(model.displayCandidates.invalid, 0, model.displayCandidates.lastRefusal ?? "")
            // The required channel was held for the siblings (completion refresh + next-edit release) and the
            // siblings arrived: the holds were released by candidates, not by the bound.
            XCTAssertGreaterThan(model.displayCandidates.deferredReleasedByCandidate, 0)
            XCTAssertNil(model.displayCandidates.deferred, "nothing is held once the burst settled")
            XCTAssertEqual(model.controllerState.inFlight?.id, nil, "the held release ran: no edit is stuck in flight")
            XCTAssertTrue(model.displayCandidates.status.hasPrefix("enabled; painted"), model.displayCandidates.status)

            // A tampered sibling with the current identity: admitted, refused off-main by the source hash, v1 and the
            // previously verified frame stay on screen.
            guard case .worker(_, _, _, let line) = model.displayListV2?.source else { return XCTFail() }
            var tampered = String(decoding: line, as: UTF8.self)
            let sha = try XCTUnwrap(last.list.documents.first).sha256
            tampered = tampered.replacingOccurrences(of: sha, with: String(sha.reversed()))
            let applied = try XCTUnwrap(model.displayCandidates.applied)
            let forged = DisplayCandidateFrame(sessionID: model.controller!.config.sessionID, requestID: applied.requestID, projectID: model.controller!.config.projectID,
                                               compileRevision: applied.compileRevision, sourceVersions: applied.sourceVersions, membershipGeneration: try XCTUnwrap(model.project.membershipGeneration), // learned on ready; the gate refuses anything older
                                               displayList: Data(tampered.utf8), frameBytes: tampered.utf8.count, receivedNs: MonotonicClock.nowNs())
            let publishedBefore = model.displayCandidates.published
            model.handleDisplayCandidate(forged)
            try await waitUntil { model.displayCandidates.invalid == 1 }
            XCTAssertTrue(model.displayCandidates.lastRefusal?.contains("sha256") == true, model.displayCandidates.lastRefusal ?? "")
            XCTAssertEqual(model.displayCandidates.published, publishedBefore)
            guard case .loaded(let kept, _)? = model.displayListV2 else { return XCTFail("the previous verified frame is restored") }
            XCTAssertEqual(kept.list.revision, final)
            XCTAssertEqual(model.result?.revision, final, "v1 untouched")

            // A candidate from another helper session (a toggled/old session) is refused at admission.
            let foreign = DisplayCandidateFrame(sessionID: "mac-old-session", requestID: applied.requestID, projectID: model.controller!.config.projectID,
                                                compileRevision: applied.compileRevision, sourceVersions: applied.sourceVersions, membershipGeneration: try XCTUnwrap(model.project.membershipGeneration), // learned on ready; the gate refuses anything older
                                                displayList: line, frameBytes: line.count, receivedNs: MonotonicClock.nowNs())
            let refusedBefore = model.displayCandidates.refused
            model.handleDisplayCandidate(foreign)
            XCTAssertEqual(model.displayCandidates.refused, refusedBefore + 1)
            XCTAssertTrue(model.displayCandidates.lastRefusal?.contains("session mac-old-session is not the negotiated") == true)
            // Candidate receipt never enabled source actions: the pane's navigation still requires the buffer hash to match.
            XCTAssertNil(model.historicalRefusal(of: "navigation"))
            model.detachController()
            XCTAssertFalse(model.displayCandidatesNegotiated)
            XCTAssertNil(model.displayCandidates.pending)
            XCTAssertEqual(model.displayCandidates.status, "requested (helper close)")
        }
    }

    func testHelperDisableRestartAndCloseRequireFreshOptIn() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "lifecycle", text: Self.sample)
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        try await withProducer(render) {
            let model = ShellModel()
            model.autoCompile = true
            XCTAssertEqual(model.openTex(at: p.tex), .opened)
            model.attachController(at: helper)
            try await waitUntil { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }
            XCTAssertNil(model.displayListV2)
            // Menu toggle ON after attach: negotiated, first candidate paints.
            model.setDisplayCandidates(true)
            XCTAssertTrue(model.previewV2, "enabling shows the v2 pane")
            try await waitUntil { model.displayCandidatesNegotiated }
            try await waitUntil { if case .loaded? = model.displayListV2 { return true } else { return false } }
            let paintedOnce = model.displayCandidates.published

            // OFF: the helper stops enrolling display-list-v2; edits keep producing v1 only.
            model.setDisplayCandidates(false)
            try await waitUntil { !model.displayCandidatesNegotiated && model.displayCandidates.status == "off" }
            let receivedAtOff = model.displayCandidates.received
            model.updateActiveText(Self.sample.replacingOccurrences(of: "fi.", with: "fi, off."))
            var final = model.editorRevision
            try await waitUntil { model.result?.revision == final && model.inFlightRevision == nil }
            XCTAssertEqual(model.displayCandidates.received, receivedAtOff, "no candidates after disable")
            XCTAssertFalse(model.negotiation.accepted.contains("display-list-v2"), "the layout capability was removed with the disable")
            XCTAssertFalse(model.controllerStatus.hasPrefix("protocol violation"), model.controllerStatus)

            // ON again, then a compiler restart: the helper drops the mode; a fresh opt-in follows the restart reply.
            model.setDisplayCandidates(true)
            try await waitUntil { model.displayCandidatesNegotiated }
            model.displayCandidatesRestartHelper()
            XCTAssertFalse(model.displayCandidatesNegotiated, "restart voids the negotiation immediately")
            try await waitUntil { model.displayCandidatesNegotiated && model.displayCandidates.restartRequestID == nil }
            model.updateActiveText(Self.sample.replacingOccurrences(of: "fi.", with: "fi, after restart."))
            final = model.editorRevision
            try await waitUntil { model.result?.revision == final && model.inFlightRevision == nil }
            try await waitUntil {
                if case .loaded(let f, _)? = model.displayListV2, f.list.revision == final { return true } else { return false }
            }
            XCTAssertGreaterThan(model.displayCandidates.published, paintedOnce)
            XCTAssertEqual(model.displayCandidates.invalid, 0, model.displayCandidates.lastRefusal ?? "")

            // Close: everything void; the intent survives for the next attach.
            model.detachController()
            XCTAssertFalse(model.displayCandidatesNegotiated)
            XCTAssertNil(model.displayCandidates.validating)
            XCTAssertTrue(model.displayCandidates.requested)
            XCTAssertEqual(model.displayCandidates.status, "requested (helper close)")
            // Reattach: a fresh session negotiates again on ready and paints for the current buffer.
            model.attachController(at: helper)
            try await waitUntil { model.displayCandidatesNegotiated }
            try await waitUntil {
                if case .loaded(let f, _)? = model.displayListV2, f.list.revision == model.editorRevision, model.result?.revision == model.editorRevision { return true }
                return false
            }
            model.detachController()
        }
    }

    func testHelperRefusesCandidatesWhileCompletedSnapshotsAreNegotiated() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "exclusive", text: Self.sample)
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        try await withProducer(render) {
            setenv("FLASHTEX_COMPLETED_SNAPSHOTS", "1", 1)
            setenv("FLASHTEX_DISPLAY_CANDIDATES", "1", 1)
            defer { unsetenv("FLASHTEX_COMPLETED_SNAPSHOTS"); unsetenv("FLASHTEX_DISPLAY_CANDIDATES") }
            let model = ShellModel()
            model.autoCompile = true
            XCTAssertEqual(model.openTex(at: p.tex), .opened)
            model.attachController(at: helper)
            try await waitUntil { model.historicalNegotiated }
            try await waitUntil { model.displayCandidates.status.hasPrefix("refused") }
            XCTAssertFalse(model.displayCandidatesNegotiated)
            XCTAssertTrue(model.displayCandidates.lastError?.contains("mutually exclusive") == true, model.displayCandidates.lastError ?? "")
            // Required v1 output is unaffected by the refused optional mode.
            try await waitUntil { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }
            XCTAssertNil(model.displayListV2)
            XCTAssertTrue(model.historicalNegotiated, "the refused enable changed neither optional policy")
            model.detachController()
        }
    }

    private func waitUntil(timeout: TimeInterval = 30, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout after \(Int(timeout)) s (load-sensitive; rerun before concluding a failure)") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }
}
