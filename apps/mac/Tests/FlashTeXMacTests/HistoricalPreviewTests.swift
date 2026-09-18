import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Native consumer of the helper's `completed_snapshot` side channel
/// (HistoricalPreview.swift). Unit tests cover the token, the decoder and the
/// state machine; the end-to-end tests drive a real `ShellModel` against
/// `Fixtures/fake_preview_controller.py`, which speaks the helper protocol v1
/// plus the negotiated channel exactly as the helper at ab945e6 does, and can
/// misbehave on request (wrong session/project/token, claims of currency) the
/// way a real helper never should.
@MainActor
final class HistoricalPreviewTests: XCTestCase {
    static let fakeHelper = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_preview_controller.py")

    // MARK: token

    func testTokenRoundTripAndForeignTokensRefused() throws {
        let nonce = SourceBindingToken.makeNonce()
        XCTAssertEqual(nonce.count, 16)
        let token = SourceBindingToken(nonce: nonce, editorRevision: 42)
        XCTAssertEqual(token.encoded, "ftx1:\(nonce):42")
        XCTAssertLessThanOrEqual(token.encoded.utf8.count, CompletedSnapshots.maxTokenBytes)
        XCTAssertEqual(SourceBindingToken.parse(token.encoded, nonce: nonce), token)
        XCTAssertNil(SourceBindingToken.parse(token.encoded, nonce: SourceBindingToken.makeNonce()), "another negotiation's nonce")
        XCTAssertNil(SourceBindingToken.parse("ftx1:\(nonce):-1", nonce: nonce))
        XCTAssertNil(SourceBindingToken.parse("ftx1:\(nonce):x", nonce: nonce))
        XCTAssertNil(SourceBindingToken.parse("ftx0:\(nonce):1", nonce: nonce))
        XCTAssertNil(SourceBindingToken.parse("", nonce: nonce))
        XCTAssertTrue(SourceBindingToken.isWellFormed(String(repeating: "α", count: 64)), "128 bytes")
        XCTAssertFalse(SourceBindingToken.isWellFormed(String(repeating: "α", count: 65)), "130 bytes")
        XCTAssertFalse(SourceBindingToken.isWellFormed(""))
        // Tokens are minted only after negotiation; the wire to an old helper is unchanged.
        var state = HistoricalPreviewState()
        XCTAssertNil(state.token(forEditorRevision: 3))
        state.beginNegotiation(sessionID: "s", projectID: "p", requestID: "pc-1")
        XCTAssertNil(state.token(forEditorRevision: 3), "not before the acknowledgement")
        XCTAssertEqual(state.acknowledge(requestID: "pc-9", payload: ["capability": "completed-snapshots-v1", "enabled": true]), nil, "another request's reply")
        XCTAssertEqual(state.acknowledge(requestID: "pc-1", payload: ["capability": "completed-snapshots-v1", "enabled": true]), true)
        let minted = try XCTUnwrap(state.token(forEditorRevision: 3))
        XCTAssertEqual(SourceBindingToken.parse(minted, nonce: state.nonce)?.editorRevision, 3)
        XCTAssertEqual(state.lastSubmittedEditorRevision, 3)
        // A refused or differently-shaped acknowledgement never negotiates.
        var refused = HistoricalPreviewState()
        refused.beginNegotiation(sessionID: "s", projectID: "p", requestID: "pc-2")
        XCTAssertEqual(refused.acknowledge(requestID: "pc-2", payload: ["capability": "completed-snapshots-v1", "enabled": false]), false)
        XCTAssertNil(refused.token(forEditorRevision: 1))
        var old = HistoricalPreviewState()
        old.beginNegotiation(sessionID: "s", projectID: "p", requestID: "pc-3")
        XCTAssertTrue(old.refuseNegotiation(requestID: "pc-3"))
        XCTAssertFalse(old.isNegotiated)
    }

    func testNegotiationPayloadMatchesHelperWire() {
        let payload = CompletedSnapshots.negotiationPayload()
        XCTAssertEqual(payload["capability"] as? String, "completed-snapshots-v1")
        XCTAssertEqual(payload["enabled"] as? Bool, true)
        XCTAssertEqual(CompletedSnapshots.operation, "configure_completed_snapshots")
        XCTAssertFalse(CompletedSnapshots.requested(environment: [:]), "default OFF")
        XCTAssertFalse(CompletedSnapshots.requested(environment: ["FLASHTEX_COMPLETED_SNAPSHOTS": "true"]))
        XCTAssertTrue(CompletedSnapshots.requested(environment: ["FLASHTEX_COMPLETED_SNAPSHOTS": "1"]))
    }

    // MARK: decoder

    private func frameLine(_ overrides: [String: Any], drop: [String] = []) -> Data {
        var payload: [String: Any] = [
            "kind": "completed_snapshot", "project_id": "demo", "session_id": "sess", "request_id": "preview-10",
            "compile_revision": 10, "source_versions": ["main.tex": 7], "current_compile_revision": 12,
            "is_current": false, "source_actions_enabled": false, "source_binding_token": "ftx1:00ff:5",
            "result": ["protocol_version": 1, "id": "preview-10", "type": "compile_result",
                       "payload": ["project_id": "demo", "revision": 10, "status": "ok",
                                   "pages": [["number": 1, "width_pt": 612, "height_pt": 792, "items": [
                                       ["kind": "text", "text": "A", "x_pt": 72, "baseline_y_pt": 84, "font_size_pt": 12,
                                        "source": ["path": "main.tex", "start_byte": 0, "end_byte": 1]]]]],
                                   "diagnostics": [], "pdf_path": NSNull()]],
        ]
        for (k, v) in overrides { payload[k] = v }
        for k in drop { payload.removeValue(forKey: k) }
        let frame: [String: Any] = ["protocol_version": 1, "session_id": "sess", "id": NSNull(), "type": "update", "payload": payload]
        return try! JSONSerialization.data(withJSONObject: frame)
    }

    func testDecodesCompletedSnapshotAndRefusesMalformedOnes() throws {
        guard case .completedSnapshot(let frame) = PreviewControllerClient.decode(frameLine([:]), sessionID: "sess") else {
            return XCTFail("expected a completed snapshot")
        }
        XCTAssertEqual(frame.projectID, "demo")
        XCTAssertEqual(frame.sessionID, "sess")
        XCTAssertEqual(frame.requestID, "preview-10")
        XCTAssertEqual(frame.compileRevision, 10)
        XCTAssertEqual(frame.currentCompileRevision, 12)
        XCTAssertEqual(frame.sourceVersions, ["main.tex": 7])
        XCTAssertEqual(frame.sourceBindingToken, "ftx1:00ff:5")
        XCTAssertEqual(frame.result.id, "preview-10")
        guard let firstPage = frame.result.payload.pages.first, let firstItem = firstPage.items.first else {
            return XCTFail("expected at least one page with at least one item")
        }
        guard case .text(let item) = firstItem else { return XCTFail() }
        XCTAssertEqual(item.text, "A")

        func violation(_ line: Data, _ label: String) {
            guard case .protocolViolation(let m) = PreviewControllerClient.decode(line, sessionID: "sess") else {
                return XCTFail("\(label): expected a protocol violation")
            }
            XCTAssertFalse(m.isEmpty, label)
        }
        violation(frameLine(["is_current": true]), "claims currency")
        violation(frameLine([:], drop: ["is_current"]), "no is_current")
        violation(frameLine(["source_actions_enabled": true]), "claims source actions")
        violation(frameLine(["session_id": "other"]), "payload session differs from the frame's")
        violation(frameLine([:], drop: ["session_id"]), "no session_id")
        violation(frameLine([:], drop: ["project_id"]), "no project_id")
        violation(frameLine([:], drop: ["source_binding_token"]), "no token")
        violation(frameLine(["source_binding_token": ""]), "empty token")
        violation(frameLine(["source_binding_token": String(repeating: "x", count: 129)]), "129-byte token")
        violation(frameLine(["compile_revision": 12]), "not older than current")
        violation(frameLine(["compile_revision": 13]), "newer than current")
        violation(frameLine([:], drop: ["source_versions"]), "no source versions")
        violation(frameLine([:], drop: ["result"]), "no result")
        violation(frameLine(["result": "nope"]), "result not an object")
        violation(frameLine(["result": ["protocol_version": 1, "id": "x", "type": "error", "payload": ["message": "m"]]]), "not a compile_result")
        // The frame's own session is still checked first, as for every frame.
        guard case .protocolViolation = PreviewControllerClient.decode(frameLine([:]), sessionID: "another-helper") else {
            return XCTFail("frame for another session must be refused")
        }
        // A 128-byte opaque token with non-ASCII content round-trips untouched.
        let opaque = String(repeating: "α", count: 64)
        guard case .completedSnapshot(let f2) = PreviewControllerClient.decode(frameLine(["source_binding_token": opaque]), sessionID: "sess") else {
            return XCTFail("128-byte token")
        }
        XCTAssertEqual(f2.sourceBindingToken, opaque)
    }

    // MARK: state machine

    private func makeFrame(compile: Int, current: Int, token: String, session: String = "s", project: String = "p",
                           versions: [String: Int] = ["main.tex": 2]) -> HistoricalFrame {
        let result = RuntimeV1.CompileResult(projectId: project, revision: compile, status: .ok, pages: [], diagnostics: [], pdfPath: nil)
        return HistoricalFrame(projectID: project, sessionID: session, requestID: "preview-\(compile)", compileRevision: compile,
                               sourceVersions: versions, currentCompileRevision: current, sourceBindingToken: token,
                               result: RuntimeV1.Envelope(protocolVersion: 1, id: "preview-\(compile)", type: "compile_result", payload: result))
    }

    func testStateKeepsOnePendingFrameHonoursTheFloorAndInvalidatesOnSwitch() throws {
        var state = HistoricalPreviewState()
        state.beginNegotiation(sessionID: "s", projectID: "p", requestID: "n")
        XCTAssertEqual(state.acknowledge(requestID: "n", payload: ["capability": "completed-snapshots-v1", "enabled": true]), true)
        let t5 = try XCTUnwrap(state.token(forEditorRevision: 5)), t6 = try XCTUnwrap(state.token(forEditorRevision: 6))
        let t7 = try XCTUnwrap(state.token(forEditorRevision: 7))

        // Not negotiated → refused (a fresh state).
        var cold = HistoricalPreviewState()
        XCTAssertEqual(cold.admit(makeFrame(compile: 1, current: 2, token: t5), activePath: "main.tex", displayedEditorRevision: nil),
                       .refused("completed snapshots not negotiated"))

        // Wrong session / project / token: refused before queueing.
        guard case .refused(let s) = state.admit(makeFrame(compile: 1, current: 2, token: t5, session: "other"), activePath: "main.tex", displayedEditorRevision: nil) else { return XCTFail() }
        XCTAssertTrue(s.contains("session other"))
        guard case .refused(let p) = state.admit(makeFrame(compile: 1, current: 2, token: t5, project: "other"), activePath: "main.tex", displayedEditorRevision: nil) else { return XCTFail() }
        XCTAssertTrue(p.contains("project other"))
        guard case .refused(let t) = state.admit(makeFrame(compile: 1, current: 2, token: "ftx1:deadbeefdeadbeef:5"), activePath: "main.tex", displayedEditorRevision: nil) else { return XCTFail() }
        XCTAssertTrue(t.contains("not minted"))
        guard case .refused(let v) = state.admit(makeFrame(compile: 1, current: 2, token: t5, versions: ["other.tex": 1]), activePath: "main.tex", displayedEditorRevision: nil) else { return XCTFail() }
        XCTAssertTrue(v.contains("active document"))
        XCTAssertEqual(state.refused, 4)
        XCTAssertNil(state.pending)

        // Queue one; a newer arrival replaces it (at most one pending); an older one is refused.
        XCTAssertEqual(state.admit(makeFrame(compile: 1, current: 3, token: t5), activePath: "main.tex", displayedEditorRevision: nil), .queued)
        XCTAssertEqual(state.admit(makeFrame(compile: 2, current: 3, token: t6), activePath: "main.tex", displayedEditorRevision: nil), .replacedPending("preview-1"))
        guard case .refused = state.admit(makeFrame(compile: 1, current: 3, token: t5), activePath: "main.tex", displayedEditorRevision: nil) else { return XCTFail("older than pending") }
        XCTAssertEqual(state.pending?.requestID, "preview-2")
        XCTAssertEqual(state.dropped, 1)

        // A document switch between admission and paint voids the queued delivery.
        XCTAssertNil(state.takePending(activePath: "chapter.tex"))
        XCTAssertNil(state.pending)
        XCTAssertEqual(state.dropped, 2)

        // Monotonic floor: after generation 3 was displayed (a current preview),
        // generations ≤ 3 are refused — a delayed callback cannot repaint over it.
        XCTAssertEqual(state.admit(makeFrame(compile: 2, current: 4, token: t6), activePath: "main.tex", displayedEditorRevision: nil), .queued)
        state.noteCurrentDisplayed(compileRevision: 3)
        XCTAssertNil(state.pending, "a current preview evicts the pending history")
        guard case .refused(let f) = state.admit(makeFrame(compile: 3, current: 5, token: t7), activePath: "main.tex", displayedEditorRevision: 7) else { return XCTFail() }
        XCTAssertTrue(f.contains("not newer than the displayed generation 3"))
        XCTAssertEqual(state.admit(makeFrame(compile: 4, current: 5, token: t7), activePath: "main.tex", displayedEditorRevision: 7), .queued)
        // Editor revision older than the displayed one: refused even with a newer generation number.
        guard case .refused(let e) = state.admit(makeFrame(compile: 6, current: 8, token: t5), activePath: "main.tex", displayedEditorRevision: 7) else { return XCTFail() }
        XCTAssertTrue(e.contains("older than the displayed revision 7"))
        // Painting raises the floor.
        let frame = try XCTUnwrap(state.takePending(activePath: "main.tex"))
        state.notePainted(compileRevision: frame.compileRevision)
        XCTAssertEqual(state.displayFloor, 4)
        XCTAssertEqual(state.painted, 1)

        // Invalidation (close/exit/reattach) forgets negotiation, pending and floor; the
        // old nonce's tokens no longer bind anything.
        state.invalidate()
        XCTAssertFalse(state.isNegotiated)
        XCTAssertNil(state.displayFloor)
        XCTAssertNil(state.token(forEditorRevision: 9))
        XCTAssertNil(state.editorRevision(of: makeFrame(compile: 4, current: 5, token: t7)))
        XCTAssertTrue(state.requested == CompletedSnapshots.requested(), "the opt-in survives invalidation")
    }

    func testHistoricalDisplayLabel() {
        let h = HistoricalDisplay(shownEditorRevision: 12, compilingEditorRevision: 15, shownCompileRevision: 3,
                                  currentCompileRevision: 5, requestID: "preview-3", resultID: "preview-3")
        XCTAssertEqual(h.label, "revision 12 shown — revision 15 compiling")
    }

    // MARK: end to end against the fake helper

    private struct Harness {
        let model: ShellModel
        let root: URL
        let tex: URL
    }

    private func makeHarness(text: String = "Hello\n", requested: Bool = true, name: String) throws -> Harness {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("hist-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try text.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        model.historicalState.requested = requested
        XCTAssertEqual(model.openTex(at: tex), .opened)
        return Harness(model: model, root: root, tex: tex)
    }

    private func attach(_ h: Harness) async throws {
        h.model.attachController(at: Self.fakeHelper)
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

    func testHistoricalFrameBeforeCurrentIsPaintedLabelledGatedThenReplaced() async throws {
        let h = try makeHarness(name: "before")
        defer { teardown(h) }
        try await attach(h)
        let model = h.model
        XCTAssertTrue(model.historicalNegotiated, "the fake acknowledged completed-snapshots-v1")
        XCTAssertNil(model.historicalPreview)
        XCTAssertNil(model.historicalRefusal(of: "navigation"))

        // A: slow compile (held by the fake); B: the next keystroke. In negotiated
        // mode the adapter releases on the durable receipt, so B is submitted while
        // A is still compiling — exactly the burst that produces history.
        model.updateActiveText("%hold\nA text\n")
        let revA = model.editorRevision
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        model.updateActiveText("B text\n")
        let revB = model.editorRevision
        try await waitUntil("A painted as historical") { model.historicalPreview != nil }
        let shown = try XCTUnwrap(model.historicalPreview)
        XCTAssertEqual(shown.shownEditorRevision, revA)
        XCTAssertEqual(shown.compilingEditorRevision, revB)
        XCTAssertEqual(shown.label, "revision \(revA) shown — revision \(revB) compiling")
        XCTAssertEqual(shown.currentCompileRevision, shown.shownCompileRevision + 1, "B is the generation right after A")
        XCTAssertEqual(shown.requestID, "preview-\(shown.shownCompileRevision)")
        XCTAssertEqual(model.result?.revision, revA)
        XCTAssertEqual(firstItemText(model), "A text")
        XCTAssertTrue(model.previewIsStale)
        XCTAssertEqual(model.compiledDocuments["main.tex"], "%hold\nA text\n", "the ORIGINATING text, never the buffer")
        XCTAssertTrue(model.workerStatus.hasPrefix("historical revision \(revA)"))
        XCTAssertEqual(model.historicalState.painted, 1)

        // Source actions are refused by the flag, not by staleness inference.
        model.navigateExactly(to: .init(path: "main.tex", startByte: 0, endByte: 1))
        XCTAssertTrue(model.navigationNote?.contains("historical revision \(revA)") == true, model.navigationNote ?? "nil")
        XCTAssertNil(model.selection)
        model.navigationNote = nil
        model.goToDiagnostic(forward: true)
        XCTAssertTrue(model.navigationNote?.contains("Diagnostic navigation is unavailable") == true)
        model.caretUTF16 = 0
        XCTAssertEqual(model.exactCaretItems, [:], "no caret sync onto an older snapshot")
        XCTAssertTrue(model.editorMarkReport.marks.isEmpty)
        model.pinAnchorAtCaret()
        XCTAssertNil(model.anchor)
        XCTAssertTrue(model.captureNote?.contains("Pinning an insertion point is unavailable") == true)
        XCTAssertTrue(model.historicalRefusal(of: "export")?.contains("Export is unavailable") == true)

        // B (current) replaces A: flag cleared, actions back.
        try await waitUntil("B painted") { model.historicalPreview == nil && model.result?.revision == revB }
        XCTAssertEqual(firstItemText(model), "B text")
        XCTAssertFalse(model.previewIsStale)
        XCTAssertNil(model.historicalRefusal(of: "navigation"))
        model.navigateExactly(to: .init(path: "main.tex", startByte: 0, endByte: 1))
        XCTAssertNotNil(model.selection, "navigation works again on the current preview")
        XCTAssertEqual(model.historicalState.displayFloor, shown.currentCompileRevision)
        XCTAssertTrue(model.workerLog.contains { $0.contains("historical: painted \(shown.requestID) as revision \(revA)") })
    }

    func testHistoricalFrameAfterCurrentIsDropped() async throws {
        let h = try makeHarness(name: "after")
        defer { teardown(h) }
        try await attach(h)
        let model = h.model
        model.updateActiveText("%hold\nA text\n")
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        model.updateActiveText("%after\nB text\n")
        let revB = model.editorRevision
        try await waitUntil("B painted") { model.result?.revision == revB }
        XCTAssertNil(model.historicalPreview)
        // The late A arrives after B; it must never repaint over B.
        try await waitUntil("A refused") { model.historicalState.refused >= 1 }
        XCTAssertEqual(model.result?.revision, revB)
        XCTAssertEqual(firstItemText(model), "B text")
        XCTAssertNil(model.historicalPreview)
        XCTAssertEqual(model.historicalState.painted, 0)
        XCTAssertTrue(model.workerLog.contains { $0.contains("historical: refused preview-") && $0.contains("not newer than the displayed generation") })
    }

    func testFramesWithForeignTokenOrProjectAreRefusedAndWrongSessionIsAViolation() async throws {
        for (directive, expectLog) in [("%badtoken", "not minted for this session"), ("%badproject", "project other-project")] {
            let h = try makeHarness(name: "bad")
            defer { teardown(h) }
            try await attach(h)
            let model = h.model
            model.updateActiveText("%hold\nA text\n")
            try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
            model.updateActiveText("\(directive)\nB text\n")
            let revB = model.editorRevision
            try await waitUntil("refused") { model.historicalState.refused >= 1 }
            XCTAssertEqual(model.historicalState.painted, 0, directive)
            XCTAssertTrue(model.workerLog.contains { $0.contains("historical: refused preview-") && $0.contains(expectLog) }, directive)
            try await waitUntil("B painted") { model.result?.revision == revB }
            XCTAssertNil(model.historicalPreview)
            XCTAssertEqual(firstItemText(model), "B text")
        }
        // A payload session id that is not the frame's is malformed: refused at
        // decode as a protocol violation; nothing painted.
        let h = try makeHarness(name: "badsession")
        defer { teardown(h) }
        try await attach(h)
        let model = h.model
        model.updateActiveText("%hold\nA text\n")
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        model.updateActiveText("%badsession\nB text\n")
        let revB = model.editorRevision
        try await waitUntil("violation") { model.workerLog.contains { $0.contains("protocol violation: completed_snapshot for session other-session") } }
        try await waitUntil("B painted") { model.result?.revision == revB }
        XCTAssertEqual(model.historicalState.painted, 0)
        XCTAssertNil(model.historicalPreview)
        XCTAssertTrue(model.controllerAttached, "a malformed optional frame does not end the session")
    }

    func testMalformedClaimsOfCurrencyAreViolationsNotFrames() async throws {
        for directive in ["%current", "%actions"] {
            let h = try makeHarness(name: "claim")
            defer { teardown(h) }
            try await attach(h)
            let model = h.model
            model.updateActiveText("%hold\nA text\n")
            try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
            model.updateActiveText("\(directive)\nB text\n")
            let revB = model.editorRevision
            try await waitUntil("violation \(directive)") { model.workerLog.contains { $0.contains("protocol violation: completed_snapshot preview-") && $0.contains("does not declare") } }
            try await waitUntil("B painted") { model.result?.revision == revB }
            XCTAssertEqual(model.historicalState.painted, 0, directive)
            XCTAssertNil(model.historicalPreview, directive)
        }
    }

    func testNoEmissionAndNoTokenWithoutNegotiation() async throws {
        // 1. Default OFF: the app never asks; the old wire (stale) is all there is.
        do {
            let h = try makeHarness(requested: false, name: "off")
            defer { teardown(h) }
            try await attach(h)
            let model = h.model
            XCTAssertFalse(model.historicalNegotiated)
            XCTAssertFalse(model.workerLog.contains { $0.contains("historical: requested") })
            model.updateActiveText("%hold\nA text\n")
            let revA = model.editorRevision
            // Hold-until-preview stands: the edit stays in flight until its preview.
            try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 }
            XCTAssertEqual(model.controllerState.inFlight?.editorRevision, revA)
            model.updateActiveText("B text\n") // coalesced behind A
            XCTAssertTrue(model.controllerState.queued)
            XCTAssertNil(model.historicalPreview)
            XCTAssertEqual(model.historicalState.painted, 0)
            XCTAssertTrue(model.workerLog.contains { $0.contains("edit r2 token=absent") }, "no token on the wire")
            XCTAssertFalse(model.workerLog.contains { $0.contains("token=present") })
        }
        // 2. Requested but the helper predates the channel ("unknown operation"):
        //    negotiation off, no token, no historical paint.
        do {
            setenv("FAKE_PC_REFUSE_SNAPSHOTS", "1", 1)
            defer { unsetenv("FAKE_PC_REFUSE_SNAPSHOTS") }
            let h = try makeHarness(name: "refused")
            defer { teardown(h) }
            try await attach(h)
            let model = h.model
            try await waitUntil("refusal logged") { model.workerLog.contains { $0.contains("historical: helper refused configure_completed_snapshots: unknown operation") } }
            XCTAssertFalse(model.historicalNegotiated)
            model.updateActiveText("%hold\nA text\n")
            try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 }
            XCTAssertTrue(model.workerLog.contains { $0.contains("edit r2 token=absent") })
            XCTAssertNil(model.historicalToken(forEditorRevision: model.editorRevision))
            XCTAssertEqual(model.historicalState.painted, 0)
        }
    }

    func testCloseAndReattachInvalidateNegotiationAndFloor() async throws {
        let h = try makeHarness(name: "close")
        defer { teardown(h) }
        try await attach(h)
        let model = h.model
        XCTAssertTrue(model.historicalNegotiated)
        let nonce = model.historicalState.nonce
        model.updateActiveText("%hold\nA text\n")
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        model.updateActiveText("B text\n")
        try await waitUntil("A painted") { model.historicalPreview != nil }
        // Close while historical: queued deliveries and negotiation are void; the
        // painted historical result keeps its flag (source actions stay off).
        model.detachController()
        XCTAssertFalse(model.historicalNegotiated)
        XCTAssertNil(model.historicalState.displayFloor)
        XCTAssertNotNil(model.historicalPreview, "old content stays flagged until a current result replaces it")
        XCTAssertNotNil(model.historicalRefusal(of: "export"))
        // Reattach: a fresh negotiation with a fresh nonce; the first current preview clears the flag.
        try await attach(h)
        XCTAssertTrue(model.historicalNegotiated)
        XCTAssertNotEqual(model.historicalState.nonce, nonce)
        XCTAssertNil(model.historicalPreview)
        XCTAssertNil(model.historicalRefusal(of: "export"))
        XCTAssertEqual(firstItemText(model), "B text")
    }

    func testReplaceProjectClearsTheFlag() async throws {
        let h = try makeHarness(name: "replace")
        defer { teardown(h) }
        try await attach(h)
        let model = h.model
        model.updateActiveText("%hold\nA text\n")
        try await waitUntil("A durable") { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        model.updateActiveText("B text\n")
        try await waitUntil("A painted") { model.historicalPreview != nil }
        model.replaceProject(entryText: "fresh\n")
        XCTAssertNil(model.historicalPreview)
        XCTAssertNil(model.result)
    }

    // MARK: real helper (ab945e6 or later) with the real compiler

    static var realHelper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }

    /// The published helper negotiates the channel, echoes the token on a
    /// `completed_snapshot` for a compile that completed after a newer edit was
    /// submitted, and the shell paints it labelled/gated and then replaces it
    /// with the current preview bound to the final editor revision. Skipped
    /// unless `FLASHTEX_PREVIEW_CONTROLLER` (ab945e6+) and `FLASHTEX_COMPILER`
    /// point at built binaries. Whether a given burst yields a historical frame
    /// depends on compile timing; the test asserts the invariants on every frame
    /// that did arrive and reports the counts.
    func testRealHelperNegotiatesEchoesTokensAndPaintsHistoryDuringABurst() async throws {
        guard let helper = Self.realHelper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER (ab945e6+) and FLASHTEX_COMPILER to built binaries")
        }
        let demo = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Samples/demo.tex")
        let seed = try String(contentsOf: demo, encoding: .utf8)
        let h = try makeHarness(text: seed, name: "real")
        defer { teardown(h) }
        let model = h.model
        model.attachController(at: helper)
        try await waitUntil("initial preview", timeout: 30) { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }
        try await waitUntil("negotiation answered") { model.historicalNegotiated || model.workerLog.contains { $0.contains("historical: helper") } }
        XCTAssertTrue(model.historicalNegotiated, "the real helper acknowledged \(CompletedSnapshots.capability): \(model.workerLog.filter { $0.contains("historical") })")

        // A burst: 40 edits at ~5 ms, released on the durable receipt (negotiated
        // mode) so compiles overlap; those completing after a newer submission
        // come back as completed snapshots.
        let marker = "\\end{document}"
        let insertAt = try XCTUnwrap(seed.range(of: marker))
        var text = seed
        var paintedHistorical: [HistoricalDisplay] = []
        var submitted: [Int] = []
        for _ in 1...40 {
            text.insert(contentsOf: "x", at: insertAt.lowerBound)
            model.updateActiveText(text)
            submitted.append(model.editorRevision)
            let deadline = Date().addingTimeInterval(0.005)
            while Date() < deadline {
                try await Task.sleep(nanoseconds: 1_000_000)
                if let hp = model.historicalPreview, paintedHistorical.last != hp { paintedHistorical.append(hp) }
            }
        }
        let final = model.editorRevision
        let start = Date()
        while Date().timeIntervalSince(start) < 30 {
            if let hp = model.historicalPreview, paintedHistorical.last != hp { paintedHistorical.append(hp) }
            if model.result?.revision == final, model.historicalPreview == nil, model.controllerState.inFlight == nil, !model.controllerState.queued { break }
            try await Task.sleep(nanoseconds: 5_000_000)
        }
        XCTAssertEqual(model.result?.revision, final, "the current preview is bound to the final editor revision")
        XCTAssertNil(model.historicalPreview)
        XCTAssertFalse(model.previewIsStale)
        XCTAssertEqual(model.compiledDocuments["main.tex"], model.activeText)
        // Every painted historical frame was an editor revision we submitted with a
        // token, older than the revision it said was compiling, painted in
        // increasing generation order (the display floor).
        for (i, hp) in paintedHistorical.enumerated() {
            XCTAssertTrue(submitted.contains(hp.shownEditorRevision), "shown \(hp.shownEditorRevision) was submitted")
            XCTAssertLessThan(hp.shownEditorRevision, hp.compilingEditorRevision, hp.label)
            XCTAssertLessThan(hp.shownCompileRevision, hp.currentCompileRevision, hp.label)
            if i > 0 { XCTAssertGreaterThan(hp.shownCompileRevision, paintedHistorical[i - 1].shownCompileRevision, "monotonic") }
        }
        let st = model.historicalState
        let log = model.workerLog.filter { $0.hasPrefix("historical:") }
        print("real helper burst: historical painted=\(st.painted) refused=\(st.refused) dropped=\(st.dropped) observed=\(paintedHistorical.map(\.label)) log=\(log)")
        XCTAssertGreaterThanOrEqual(st.painted, paintedHistorical.count)
        XCTAssertTrue(log.contains { $0.contains("acknowledged") })
    }

    // MARK: -

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out waiting for \(what)"); throw XCTSkip("timeout: \(what)") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }
}
