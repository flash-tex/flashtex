import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Grouped admission consumption (crates/preview-controller STDIO.md "Edit
/// admission correlation", helper main c11c005 / 3ffd57f): an `apply_group`
/// reply carries the same nullable top-level `compile_request_id` /
/// `compile_revision` pair as an `edit` reply. The native side
/// (`controllerAdoptHistoryResult` → `applyDurableDocument`,
/// AdmissionCorrelation.swift) must record that pair as the in-flight
/// admission exactly like an edit's, keep the ledger's permanent
/// `history.command_revision` / `replayed_command` identity SEPARATE from
/// it, treat an exact retry's fresh pair as belonging to the CURRENT
/// document, record nothing for a failed command, and give undo/redo (whose
/// wire schemas carry no pair) the same adoption path with the numeric
/// fallback.
///
/// Real helper and compiler from this tree; every case skips cleanly without
/// `FLASHTEX_PREVIEW_CONTROLLER` / `FLASHTEX_COMPILER`. Nothing here is
/// timing sensitive: the pair is recorded synchronously by the adoption
/// call, and a forged outcome is injected before the run loop can deliver
/// the real one.
@MainActor
final class AdmissionGroupsTests: XCTestCase {
    static var realHelper: URL? {
        guard let p = ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"],
              FileManager.default.isExecutableFile(atPath: p) else { return nil }
        return URL(fileURLWithPath: p)
    }

    private struct Harness {
        let model: ShellModel
        let root: URL
        let tex: URL
    }

    private func makeHarness(name: String, text: String) throws -> Harness {
        guard ShellModel.locateCompiler() != nil else { throw XCTSkip("set FLASHTEX_COMPILER to a built binary") }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("admgrp-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try text.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        return Harness(model: model, root: root, tex: tex)
    }

    private func teardown(_ h: Harness) {
        h.model.detachController()
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
        try? FileManager.default.removeItem(at: h.root)
    }

    private func attach(_ h: Harness) async throws -> URL {
        guard let helper = Self.realHelper else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER to a helper built from main c11c005 or later") }
        h.model.attachController(at: helper)
        h.model.controllerState.releasePolicy = .holdUntilPreview
        XCTAssertTrue(h.model.controllerAttached)
        try await waitUntil("initial preview") {
            h.model.result?.revision == h.model.editorRevision && h.model.previewSource == .worker("flashtex-preview-controller") && h.model.inFlightRevision == nil
        }
        XCTAssertEqual(h.model.controllerState.durable["main.tex"]?.revision, 1)
        return helper
    }

    private func doc(_ body: String) -> String { "\\begin{document}\n\(body)\n\\end{document}\n" }

    /// One raw request through the controller's reply routing (what the
    /// search / citation / history panels do), with the request id.
    private func request(_ model: ShellModel, _ type: String, _ payload: PreviewControllerClient.JSONObject) async throws -> (id: String, reply: Result<[String: Any], ControllerError>) {
        let controller = try XCTUnwrap(model.controller)
        let id = try controller.send(type, payload)
        let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
            model.controllerState.awaiting[id] = { cont.resume(returning: $0) }
        }
        return (id, reply)
    }

    /// A one-edit `apply_group` payload replacing the first `needle` of the
    /// durable text at `revision` with `replacement`, under `commandID`.
    private func groupPayload(_ model: ShellModel, commandID: String, replacing needle: String, with replacement: String, label: String) throws -> PreviewControllerClient.JSONObject {
        let durable = try XCTUnwrap(model.controllerState.durable["main.tex"])
        let text = try XCTUnwrap(model.controllerState.textByDurable["main.tex"]?[durable.revision])
        let bytes = Array(text.utf8), n = Array(needle.utf8)
        var start: Int?
        var i = 0
        while i + n.count <= bytes.count { if Array(bytes[i..<i + n.count]) == n { start = i; break }; i += 1 }
        let s = try XCTUnwrap(start, "\(needle) not in the durable text")
        return ["path": "main.tex",
                "command": ["command_id": commandID, "expected_revision": durable.revision, "expected_sha256": durable.sha256, "label": label,
                            "edits": [["start_byte": s, "end_byte": s + n.count, "removed_text": needle, "replacement": replacement]]]]
    }

    private func releaseLines(_ model: ShellModel) -> [String] { model.workerLog.filter { $0.hasPrefix("controller release:") } }
    private func holdLines(_ model: ShellModel) -> [String] { model.workerLog.filter { $0.hasPrefix("controller hold:") } }

    /// Every wait is on the real helper running real compiles; a loaded runner
    /// has taken longer than the former 15 s (merge group 37088404222, "undo
    /// preview"). Nothing here depends on how fast it is, only that it answers.
    private func waitUntil(_ what: String, _ timeout: TimeInterval = 60, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout {
                XCTFail("timed out after \(Int(timeout)) s waiting for \(what) from the real helper and compiler")
                throw XCTSkip("timeout: \(what) (load-sensitive)")
            }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    // MARK: (1) group apply → pair recorded → released by its id

    func testGroupApplyRecordsTheAdmittedPairAndItsOutcomeReleasesByID() async throws {
        let h = try makeHarness(name: "pair", text: doc("Hello group."))
        defer { teardown(h) }
        _ = try await attach(h)
        let model = h.model
        let issuedAt = model.editorRevision
        let payload = try groupPayload(model, commandID: "admgrp-pair-\(UUID().uuidString)", replacing: "group.", with: "group, applied.", label: "Apply group")
        let (id, reply) = try await request(model, "apply_group", payload)
        let result = try reply.get()
        // The wire: a non-null pair at the top level (like `edit`), the ledger identity under `history`.
        let pair = try XCTUnwrap(ControllerCompileAdmission.from(editResult: result), "apply_group reply names the admitted compile: \(result.keys.sorted())")
        XCTAssertFalse(pair.requestID.isEmpty)
        XCTAssertGreaterThan(try XCTUnwrap(pair.compileRevision), 0)
        let history = try XCTUnwrap(result["history"] as? [String: Any])
        let document = try XCTUnwrap(history["document"] as? [String: Any])
        XCTAssertEqual(document["revision"] as? Int, 2)
        XCTAssertEqual(history["command_revision"] as? Int, 2)
        XCTAssertEqual(history["replayed_command"] as? Bool, false)
        XCTAssertNil(result["preview_error"] as? String)

        // Adoption (search / citation / history panels): the group is the in-flight edit with the group's pair.
        model.controllerAdoptHistoryResult(document, requestID: id, payload: result, issuedAtEditorRevision: issuedAt)
        XCTAssertTrue(model.activeText.sameBytes(as: doc("Hello group, applied.")))
        let inFlight = try XCTUnwrap(model.controllerState.inFlight, "the adopted group waits for its compile like an edit")
        XCTAssertEqual(inFlight.id, id)
        XCTAssertEqual(inFlight.durableRevision, 2)
        XCTAssertEqual(inFlight.admitted, pair, "the group's pair is the recorded admission")
        XCTAssertEqual(model.inFlightRevision, model.editorRevision)

        // A forged outcome for another request id never releases the group (logged hold)…
        model.handleController(.update(kind: "stale", payload: ["request_id": "not-the-group", "compile_revision": pair.compileRevision!]))
        XCTAssertNotNil(model.controllerState.inFlight)
        XCTAssertTrue(try XCTUnwrap(holdLines(model).last).contains("stale not-the-group is not the admitted compile \(pair.requestID)"))
        // …and a `discarded` for the group's own id (wrong generation) holds too.
        model.handleController(.update(kind: "discarded", payload: ["request_id": pair.requestID, "compile_revision": pair.compileRevision! + 100]))
        XCTAssertNotNil(model.controllerState.inFlight)
        XCTAssertTrue(try XCTUnwrap(holdLines(model).last).contains("carries compile_revision \(pair.compileRevision! + 100), admitted \(pair.compileRevision!)"))
        XCTAssertTrue(releaseLines(model).isEmpty)

        // The real outcome, keyed on the group's id, releases exactly that admission.
        let adopted = model.editorRevision
        try await waitUntil("the group's preview") { model.result?.revision == adopted && model.controllerState.inFlight == nil }
        XCTAssertNil(model.inFlightRevision)
        let release = try XCTUnwrap(releaseLines(model).last)
        XCTAssertTrue(release.contains("preview \(pair.requestID) is the admitted compile (generation \(pair.compileRevision!), durable r2)"), release)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "adoption resubmitted nothing")
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: model.activeText) == true)
    }

    // MARK: (2) exact retry: replayed command identity, FRESH pair for the CURRENT document

    func testExactRetryReplaysTheCommandAndRecordsAFreshPairForTheCurrentDocument() async throws {
        let h = try makeHarness(name: "retry", text: doc("Hello retry."))
        defer { teardown(h) }
        _ = try await attach(h)
        let model = h.model
        let commandID = "admgrp-retry-\(UUID().uuidString)"
        let payload = try groupPayload(model, commandID: commandID, replacing: "retry.", with: "retry, applied.", label: "Apply once")
        let (firstID, firstReply) = try await request(model, "apply_group", payload)
        let first = try firstReply.get()
        let firstPair = try XCTUnwrap(ControllerCompileAdmission.from(editResult: first))
        let firstDoc = try XCTUnwrap((first["history"] as? [String: Any])?["document"] as? [String: Any])
        model.controllerAdoptHistoryResult(firstDoc, requestID: firstID, payload: first, issuedAtEditorRevision: model.editorRevision)
        XCTAssertEqual(model.controllerState.inFlight?.admitted, firstPair)
        let applied = model.editorRevision
        let r2Text = model.activeText
        try await waitUntil("first preview") { model.result?.revision == applied && model.controllerState.inFlight == nil }

        // The document moves on (an ordinary edit): the command's r2 is no longer the current document.
        model.updateActiveText(doc("Hello retry, applied, then typed."))
        let typed = model.editorRevision
        try await waitUntil("typed durable and previewed") { model.controllerState.durable["main.tex"]?.revision == 3 && model.result?.revision == typed && model.controllerState.inFlight == nil }
        let current = model.activeText

        // The exact retry (same id, same payload) is replayed by the ledger: the permanent identity is
        // preserved, the returned document is the CURRENT one, and any admission is a fresh pair.
        let issuedAt = model.editorRevision
        let (retryID, retryReply) = try await request(model, "apply_group", payload)
        let retry = try retryReply.get()
        let history = try XCTUnwrap(retry["history"] as? [String: Any])
        XCTAssertEqual(history["replayed_command"] as? Bool, true)
        XCTAssertEqual(history["command_revision"] as? Int, 2, "the permanent command identity is the ORIGINAL revision")
        let document = try XCTUnwrap(history["document"] as? [String: Any])
        XCTAssertEqual(document["revision"] as? Int, 3, "…while the document is the CURRENT one")
        XCTAssertTrue((document["text"] as? String)?.sameBytes(as: current) == true, "the replay returns the saved current text, not the command's historical result")
        let freshPair = ControllerCompileAdmission.from(editResult: retry)
        if let freshPair {
            XCTAssertNotEqual(freshPair.requestID, firstPair.requestID, "a retry's admission is fresh, never the original command's compile")
            XCTAssertGreaterThan(try XCTUnwrap(freshPair.compileRevision), try XCTUnwrap(firstPair.compileRevision))
        }

        // Adoption keeps the command identity out of the controller state: durable stays r3 (never r2),
        // the buffer is intact, and the fresh pair (when admitted) is the in-flight admission.
        model.controllerAdoptHistoryResult(document, requestID: retryID, payload: retry, issuedAtEditorRevision: issuedAt)
        XCTAssertTrue(model.activeText.sameBytes(as: current), "the buffer is intact: the replay changed nothing")
        XCTAssertEqual(model.editorRevision, issuedAt, "no text adoption was needed")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3)
        XCTAssertTrue(model.controllerState.textByDurable["main.tex"]?[2]?.sameBytes(as: r2Text) == true, "the command's r2 text is not rewritten by the replay")
        XCTAssertTrue(model.controllerState.textByDurable["main.tex"]?[3]?.sameBytes(as: current) == true)
        if let freshPair {
            let inFlight = try XCTUnwrap(model.controllerState.inFlight, "a fresh admission holds the pipeline for the current document")
            XCTAssertEqual(inFlight.id, retryID)
            XCTAssertEqual(inFlight.durableRevision, 3, "the admission belongs to the current document, not the command's r2")
            XCTAssertEqual(inFlight.admitted, freshPair)
            // The original command's compile id never releases the fresh admission…
            model.handleController(.update(kind: "discarded", payload: ["request_id": firstPair.requestID, "compile_revision": firstPair.compileRevision!]))
            XCTAssertNotNil(model.controllerState.inFlight)
            XCTAssertTrue(try XCTUnwrap(holdLines(model).last).contains("\(firstPair.requestID) is not the admitted compile \(freshPair.requestID)"))
            // …its own outcome does (the helper compiles the unchanged current document: preview, stale or discarded).
            try await waitUntil("the fresh admission's outcome") { model.controllerState.inFlight == nil }
            let release = try XCTUnwrap(releaseLines(model).last)
            XCTAssertTrue(release.contains("\(freshPair.requestID) is the admitted compile (generation \(freshPair.compileRevision!), durable r3)"), release)
        } else {
            XCTAssertNil(model.controllerState.inFlight?.admitted, "no compile admitted: nothing recorded")
        }
        XCTAssertEqual(model.result?.revision, typed, "the shown preview is still the current document's")
        print("admission-groups: replay of \(commandID.prefix(20))… → command r2 replayed, document r3, first pair \(firstPair.requestID)/\(firstPair.compileRevision ?? -1), retry pair \(freshPair.map { "\($0.requestID)/\($0.compileRevision ?? -1)" } ?? "null")")
    }

    // MARK: (3) failed group → error reply, no pair, nothing recorded, buffer intact

    func testFailedGroupRecordsNoAdmissionAndLeavesTheBufferIntact() async throws {
        let h = try makeHarness(name: "failed", text: doc("Hello failure."))
        defer { teardown(h) }
        _ = try await attach(h)
        let model = h.model
        let before = model.activeText, beforeRevision = model.editorRevision
        let logCount = model.workerLog.count
        var stale = try groupPayload(model, commandID: "admgrp-failed-\(UUID().uuidString)", replacing: "failure.", with: "failure, applied.", label: "Stale group")
        var command = stale["command"] as! [String: Any]
        command["expected_sha256"] = String(repeating: "0", count: 64) // a stale expectation: the ledger refuses the command
        stale["command"] = command
        let (_, reply) = try await request(model, "apply_group", stale)
        guard case .failure(let e) = reply else { return XCTFail("a stale group was applied: \(reply)") }
        XCTAssertFalse(e.message.isEmpty)
        // Nothing to adopt, nothing recorded: no in-flight edit, no admission, the buffer and durable state as before.
        XCTAssertNil(model.controllerState.inFlight)
        XCTAssertNil(model.inFlightRevision)
        XCTAssertTrue(model.activeText.sameBytes(as: before))
        XCTAssertEqual(model.editorRevision, beforeRevision)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1)
        XCTAssertTrue(releaseLines(model).isEmpty)
        XCTAssertFalse(model.workerLog.dropFirst(logCount).contains { $0.hasPrefix("controller refused edit") }, "an error for a raw request is not an edit refusal: \(model.workerLog.dropFirst(logCount))")
        // The same helper still admits a well-formed group afterwards (the failure left no state behind).
        let ok = try groupPayload(model, commandID: "admgrp-after-failure-\(UUID().uuidString)", replacing: "failure.", with: "failure, applied.", label: "Good group")
        let (id, good) = try await request(model, "apply_group", ok)
        let result = try good.get()
        XCTAssertNotNil(ControllerCompileAdmission.from(editResult: result))
        let document = try XCTUnwrap((result["history"] as? [String: Any])?["document"] as? [String: Any])
        model.controllerAdoptHistoryResult(document, requestID: id, payload: result, issuedAtEditorRevision: model.editorRevision)
        XCTAssertEqual(model.controllerState.inFlight?.admitted, ControllerCompileAdmission.from(editResult: result))
        let adopted = model.editorRevision
        try await waitUntil("the good group's preview") { model.result?.revision == adopted && model.controllerState.inFlight == nil }
    }

    // MARK: (4) undo / redo: the same adoption path; no pair on the wire → numeric fallback

    func testUndoAndRedoAdoptThroughTheSamePathWithTheNumericFallback() async throws {
        let h = try makeHarness(name: "undo", text: doc("Hello undo."))
        defer { teardown(h) }
        _ = try await attach(h)
        let model = h.model
        let original = model.activeText
        let payload = try groupPayload(model, commandID: "admgrp-undo-group-\(UUID().uuidString)", replacing: "undo.", with: "undo, applied.", label: "Apply group")
        let (groupID, groupReply) = try await request(model, "apply_group", payload)
        let group = try groupReply.get()
        let groupDoc = try XCTUnwrap((group["history"] as? [String: Any])?["document"] as? [String: Any])
        model.controllerAdoptHistoryResult(groupDoc, requestID: groupID, payload: group, issuedAtEditorRevision: model.editorRevision)
        XCTAssertNotNil(model.controllerState.inFlight?.admitted)
        let applied = model.editorRevision
        try await waitUntil("group preview") { model.result?.revision == applied && model.controllerState.inFlight == nil }
        let groupedText = model.activeText

        for (direction, expectedText, expectedRevision) in [("undo", original, 3), ("redo", groupedText, 4)] {
            let durable = try XCTUnwrap(model.controllerState.durable["main.tex"])
            let issuedAt = model.editorRevision
            let (id, reply) = try await request(model, direction, ["path": "main.tex",
                                                                    "command": ["command_id": "admgrp-\(direction)-\(UUID().uuidString)",
                                                                                "expected_revision": durable.revision, "expected_sha256": durable.sha256]])
            let result = try reply.get()
            // Undo/redo wire schemas are unchanged: no pair, not even a null one.
            XCTAssertNil(result["compile_request_id"], "\(direction) reply carries no compile_request_id: \(result.keys.sorted())")
            XCTAssertNil(result["compile_revision"])
            XCTAssertNil(ControllerCompileAdmission.from(editResult: result))
            let history = try XCTUnwrap(result["history"] as? [String: Any])
            XCTAssertEqual(history["command_revision"] as? Int, expectedRevision)
            XCTAssertEqual(history["replayed_command"] as? Bool, false)
            let document = try XCTUnwrap(history["document"] as? [String: Any])
            XCTAssertEqual(document["revision"] as? Int, expectedRevision)
            model.controllerAdoptHistoryResult(document, requestID: id, payload: result, issuedAtEditorRevision: issuedAt)
            XCTAssertTrue(model.activeText.sameBytes(as: expectedText), "\(direction) adopted the exact durable text")
            let inFlight = try XCTUnwrap(model.controllerState.inFlight, "\(direction) waits for its preview like an edit")
            XCTAssertEqual(inFlight.id, id)
            XCTAssertEqual(inFlight.durableRevision, expectedRevision)
            XCTAssertNil(inFlight.admitted, "\(direction): null pair → no admission recorded")
            // Numeric fallback: a generation below the durable revision holds; the preview for the durable revision releases.
            let before = releaseLines(model).count
            model.handleController(.update(kind: "stale", payload: ["request_id": "whatever", "compile_revision": 0]))
            XCTAssertNotNil(model.controllerState.inFlight)
            XCTAssertEqual(releaseLines(model).count, before)
            let adopted = model.editorRevision
            try await waitUntil("\(direction) preview") { model.result?.revision == adopted && model.controllerState.inFlight == nil }
            let release = try XCTUnwrap(releaseLines(model).last)
            XCTAssertTrue(release.contains("(numeric fallback, no admission recorded)"), release)
            XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, expectedRevision, "\(direction) adoption resubmitted nothing")
        }
    }
}
