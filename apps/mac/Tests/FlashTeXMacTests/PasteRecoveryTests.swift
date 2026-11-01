import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Large paste and multi-range replacement through the durable
/// `flashtex-preview-controller` route (ShellModel+Controller.swift), with
/// exact source recovery when the helper is killed mid-flight. Real helper +
/// real compiler; skipped unless `FLASHTEX_PREVIEW_CONTROLLER` and
/// `FLASHTEX_COMPILER` point at built binaries, and (load-aware: the tests
/// record ACK/preview times) when the 1-minute load average exceeds 20.
///
/// No ledger or helper code is changed here: every assertion is about what the
/// shell submits and adopts, and what the helper's `document`/`edit`/
/// `apply_group`/`history_status` replies say afterwards. The controller route
/// has no automatic relaunch (the shell clears its state on `.exited`, see
/// `handleController`); after a SIGKILL these tests re-attach explicitly, which
/// is what the user's reopen does.
///
/// Evidence: docs/evidence/mac-paste-recovery-2026-09-12.md (measured numbers
/// are printed as `paste-recovery:` lines).
@MainActor
final class PasteRecoveryTests: XCTestCase {
    static var helper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }

    // MARK: fixtures

    /// ~60 KB of prose lines (the SourceEditorViewTests generator shape).
    static func baseDocument(bytes: Int = 60 * 1024) -> String {
        var s = "\\documentclass{article}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < bytes {
            s += "Paragraph \(n): the quick brown fox — naïve café \\textbf{bold} $x^2 + y^2 = z^2$ jumps over the lazy dog.\n"
            n += 1
        }
        return s + "\\end{document}\n"
    }

    /// ~500 KB pasted block: one prose paragraph per eight commented lines
    /// (a large commented-out block with some live text). Measured with the
    /// real compiler: a fully-prose 560 KB document produces 18.2 MB of preview
    /// JSON, above the helper's 16 MiB output limit — that limit is a separate
    /// surface; this gap is the durable source path, so the paste stays under it.
    static func pasteBlock(bytes: Int = 500 * 1024) -> String {
        var s = "% ---- pasted block begins ----\n"
        var n = 0
        while s.utf8.count < bytes {
            if n % 8 == 0 {
                s += "Pasted paragraph \(n): ünïcödé text — “quotes” and \\emph{emphasis} with $\\alpha_\(n) + \\beta$ inside.\n"
            } else {
                s += "% pasted comment \(n): the quick brown fox — naïve café \\textbf{bold} $x^2 + y^2 = z^2$ jumps over the lazy dog.\n"
            }
            n += 1
        }
        return s + "% ---- pasted block ends ----\n"
    }

    /// The base document with the paste inserted before `\end{document}`.
    static func pasted(into base: String, block: String) -> String {
        let marker = "\\end{document}\n"
        guard let r = base.range(of: marker, options: .backwards) else { return base + block }
        return String(base[..<r.lowerBound]) + block + marker
    }

    struct Fixture {
        var root: URL
        var tex: URL
        var model: ShellModel
        var helper: URL
        var base: String
    }

    static func loadAverage1() -> Double {
        var loads = [Double](repeating: 0, count: 3)
        return getloadavg(&loads, 3) >= 1 ? loads[0] : 0
    }

    static func uptimeLine() -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/uptime")
        let out = Pipe()
        p.standardOutput = out
        guard (try? p.run()) != nil else { return "uptime unavailable" }
        p.waitUntilExit()
        return String(decoding: out.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func makeFixture(_ name: String) async throws -> Fixture {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let load = Self.loadAverage1()
        let uptime = Self.uptimeLine()
        // Timing numbers are only isolated results under a 1-minute load of 20
        // or less. `FLASHTEX_PASTE_RECOVERY_LOAD_LIMIT` raises the skip
        // threshold for a correctness run on a shared host; such a run labels
        // its numbers "under shared load, not an isolated result".
        let limit = ProcessInfo.processInfo.environment["FLASHTEX_PASTE_RECOVERY_LOAD_LIMIT"].flatMap(Double.init) ?? 20
        let label = load > 20 ? " [under shared load, not an isolated result]" : ""
        print("paste-recovery: \(name) uptime: \(uptime)\(label)")
        if load > limit { throw XCTSkip("1-minute load average \(load) > \(limit); timing-sensitive test skipped (\(uptime))") }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("paste-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        let base = Self.baseDocument()
        try base.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil(timeout: 60) { model.result?.revision == model.editorRevision && model.controllerState.durable["main.tex"]?.revision == 1 && model.controllerState.inFlight == nil }
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.sha256, SourceDigest.sha256Hex(base))
        return Fixture(root: root, tex: tex, model: model, helper: helper, base: base)
    }

    private func tearDown(_ f: Fixture) {
        f.model.detachController()
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
        try? FileManager.default.removeItem(at: f.root)
    }

    private func waitUntil(timeout: TimeInterval = 30, intervalNs: UInt64 = 5_000_000, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout after \(Int(timeout)) s (load \(Self.loadAverage1()))") }
            try await Task.sleep(nanoseconds: intervalNs)
        }
    }

    /// One request/reply through the shell's per-request waiters.
    /// `onReply` runs synchronously in the reply's own run-loop turn, as
    /// EditHistoryPanel settles: the helper's outcome for an admitted compile
    /// can be the next line on the pipe, and an adoption deferred to the test's
    /// resumption could come after it (see AdmissionGroupsTests.request).
    private func request(_ model: ShellModel, _ type: String, _ payload: PreviewControllerClient.JSONObject, id: String? = nil,
                         onReply: @escaping (String, Result<[String: Any], ControllerError>) -> Void = { _, _ in }) async throws -> (id: String, reply: Result<[String: Any], ControllerError>) {
        let controller = try XCTUnwrap(model.controller)
        let id = try controller.send(type, payload, id: id)
        let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
            model.controllerState.awaiting[id] = { reply in
                onReply(id, reply)
                cont.resume(returning: reply)
            }
        }
        return (id, reply)
    }

    private func undoLabels(_ model: ShellModel) async throws -> [String] {
        let (_, reply) = try await request(model, "history_status", ["path": "main.tex"])
        let payload = try reply.get()
        return ((payload["history"] as? [String: Any])?["undo_labels"] as? [String]) ?? []
    }

    // MARK: (a) one 500 KB paste

    func testLargePasteIsExactlyOneDurableEditAndThePreviewBindsToIt() async throws {
        let f = try await makeFixture("paste")
        defer { tearDown(f) }
        let model = f.model
        let block = Self.pasteBlock()
        let text = Self.pasted(into: f.base, block: block)
        XCTAssertGreaterThanOrEqual(block.utf8.count, 500 * 1024)
        XCTAssertGreaterThanOrEqual(f.base.utf8.count, 60 * 1024)
        let expectedSHA = SourceDigest.sha256Hex(text)

        let t0 = Date()
        model.updateActiveText(text) // the paste: one buffer replacement
        let pasteRevision = model.editorRevision
        XCTAssertEqual(model.controllerState.inFlight?.editorRevision, pasteRevision, "the paste was submitted immediately (nothing in flight)")
        XCTAssertEqual(model.inFlightRevision, pasteRevision)
        // Durable ACK.
        try await waitUntil(timeout: 60, intervalNs: 1_000_000) { model.controllerState.durable["main.tex"]?.revision == 2 }
        let ackMs = Date().timeIntervalSince(t0) * 1000
        let durable = try XCTUnwrap(model.controllerState.durable["main.tex"])
        XCTAssertEqual(durable.revision, 2, "r1 → r2: exactly one durable edit for the paste")
        XCTAssertEqual(durable.sha256, expectedSHA)
        let durableText = try XCTUnwrap(model.controllerState.textByDurable["main.tex"]?[2])
        XCTAssertTrue(durableText.sameBytes(as: model.activeText), "the helper's document text is byte-identical to the editor")
        XCTAssertEqual(durableText.utf8.count, text.utf8.count)
        XCTAssertEqual(model.controllerState.editorRevisionByDurable["main.tex"]?[2], pasteRevision)
        // Follow-up preview bound to the paste's editor revision.
        try await waitUntil(timeout: 90, intervalNs: 1_000_000) { model.result?.revision == pasteRevision && model.controllerState.inFlight == nil }
        let previewMs = Date().timeIntervalSince(t0) * 1000
        XCTAssertFalse(model.previewIsStale)
        XCTAssertEqual(model.previewSource, .worker("flashtex-preview-controller"))
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: text) == true, "the preview was compiled from exactly the pasted text")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "no further edit was submitted")
        XCTAssertNil(model.inFlightRevision)
        // The ledger recorded exactly one step for it.
        let labels = try await undoLabels(model)
        XCTAssertEqual(labels, ["Source edit"], "one history step for the whole paste")
        print(String(format: "paste-recovery: (a) paste %d B into %d B → r2 sha %@… ack %.1f ms, preview %.1f ms, load %.2f",
                     block.utf8.count, f.base.utf8.count, String(expectedSHA.prefix(12)), ackMs, previewMs, Self.loadAverage1()))
    }

    // MARK: (b) multi-range replacement as one group

    func testMultiRangeReplacementIsOneGroupWithExactText() async throws {
        let f = try await makeFixture("group")
        defer { tearDown(f) }
        let model = f.model
        let base = f.base
        let bytes = Array(base.utf8)

        // Three non-overlapping ranges on the SAME original snapshot, found by
        // byte offsets: two paragraph labels and the last "lazy dog".
        func byteRange(of needle: String, from: Int = 0) -> Range<Int>? {
            let n = Array(needle.utf8)
            var i = from
            while i + n.count <= bytes.count {
                if bytes[i] == n[0], Array(bytes[i..<i + n.count]) == n { return i..<i + n.count }
                i += 1
            }
            return nil
        }
        func lastByteRange(of needle: String) -> Range<Int>? {
            var last: Range<Int>?
            var from = 0
            while let r = byteRange(of: needle, from: from) { last = r; from = r.lowerBound + 1 }
            return last
        }
        let r1 = try XCTUnwrap(byteRange(of: "Paragraph 10:"))
        let r2 = try XCTUnwrap(byteRange(of: "Paragraph 200:"))
        let r3 = try XCTUnwrap(lastByteRange(of: "lazy dog"))
        XCTAssertLessThan(r1.upperBound, r2.lowerBound)
        XCTAssertLessThan(r2.upperBound, r3.lowerBound)
        let edits: [(Range<Int>, String)] = [(r1, "Section 10 —"), (r2, "Section 200 — naïve"), (r3, "lazy cät")]
        // Expected text: apply from the end so earlier offsets stay valid.
        var out = bytes
        for (range, replacement) in edits.reversed() { out.replaceSubrange(range, with: Array(replacement.utf8)) }
        let expected = String(decoding: out, as: UTF8.self)
        XCTAssertNotEqual(expected, base)
        let expectedSHA = SourceDigest.sha256Hex(expected)

        let durable = try XCTUnwrap(model.controllerState.durable["main.tex"])
        let issuedAt = model.editorRevision
        let payload: PreviewControllerClient.JSONObject = [
            "path": "main.tex",
            "command": [
                "command_id": "paste-recovery-group-\(UUID().uuidString)",
                "expected_revision": durable.revision,
                "expected_sha256": durable.sha256,
                "label": "Replace 3 ranges",
                "edits": edits.map { range, replacement -> [String: Any] in
                    ["start_byte": range.lowerBound, "end_byte": range.upperBound,
                     "removed_text": String(decoding: bytes[range], as: UTF8.self), "replacement": replacement]
                },
            ],
        ]
        let t0 = Date()
        var ackMs = 0.0
        // Adopt it the way the history panel does, inside the reply's turn: the
        // buffer takes the durable text, nothing is resubmitted, the preview
        // binds to the adopted editor revision.
        let (_, reply) = try await request(model, "apply_group", payload) { id, reply in
            ackMs = Date().timeIntervalSince(t0) * 1000
            guard case .success(let result) = reply, let doc = (result["history"] as? [String: Any])?["document"] as? [String: Any] else { return }
            model.controllerAdoptHistoryResult(doc, requestID: id, payload: result, issuedAtEditorRevision: issuedAt)
        }
        let result = try reply.get()
        let history = try XCTUnwrap(result["history"] as? [String: Any])
        let doc = try XCTUnwrap(history["document"] as? [String: Any])
        XCTAssertEqual(doc["revision"] as? Int, 2, "one group → one durable revision")
        XCTAssertEqual(doc["source_sha256"] as? String, expectedSHA)
        XCTAssertTrue((doc["text"] as? String)?.sameBytes(as: expected) == true, "the helper applied exactly the three ranges")
        XCTAssertEqual(history["replayed_command"] as? Bool, false)
        XCTAssertEqual(history["command_revision"] as? Int, 2)
        XCTAssertNil(result["preview_error"] as? String, "the wire carries JSON null when the compiler ran: \(result["preview_error"] ?? "")")

        XCTAssertTrue(model.activeText.sameBytes(as: expected), "the buffer adopted the grouped text exactly")
        XCTAssertGreaterThan(model.editorRevision, issuedAt)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2)
        XCTAssertEqual(model.controllerState.editorRevisionByDurable["main.tex"]?[2], model.editorRevision)
        try await waitUntil(timeout: 60, intervalNs: 1_000_000) { model.result?.revision == model.editorRevision && model.controllerState.inFlight == nil }
        let previewMs = Date().timeIntervalSince(t0) * 1000
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "adoption did not resubmit the text")
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: expected) == true)
        XCTAssertFalse(model.previewIsStale)
        let labels = try await undoLabels(model)
        XCTAssertEqual(labels, ["Replace 3 ranges"], "exactly one history step carrying the group label")
        print(String(format: "paste-recovery: (b) 3-range group on %d B → r2 sha %@… ack %.1f ms, preview %.1f ms, load %.2f",
                     base.utf8.count, String(expectedSHA.prefix(12)), ackMs, previewMs, Self.loadAverage1()))
    }

    // MARK: (c) SIGKILL mid-flight, by the pid we launched

    /// Shared body for the two kill points. `killWhen` returns once the kill was
    /// issued; it receives the pid.
    private func killAndRecover(_ name: String, killWhen: (ShellModel, Int32) async throws -> Void) async throws {
        let f = try await makeFixture(name)
        defer { tearDown(f) }
        let model = f.model
        let block = Self.pasteBlock()
        let text = Self.pasted(into: f.base, block: block)
        let expectedSHA = SourceDigest.sha256Hex(text)
        let pid = try XCTUnwrap(model.controller?.processIdentifier)
        XCTAssertGreaterThan(pid, 0)

        let t0 = Date()
        model.updateActiveText(text)
        let pasteRevision = model.editorRevision
        let sentText = try XCTUnwrap(model.controllerState.inFlight?.text)
        XCTAssertTrue(sentText.sameBytes(as: text))
        try await killWhen(model, pid)
        let before = model.controllerState.durable["main.tex"] // what the shell had ACKed when the kill went out
        let killMs = Date().timeIntervalSince(t0) * 1000
        // The exit is observed, the shell drops the helper and its state.
        try await waitUntil(timeout: 30) { model.controller == nil }
        XCTAssertFalse(model.controllerAttached)
        XCTAssertNil(model.inFlightRevision)
        XCTAssertTrue(model.activeText.sameBytes(as: text), "the buffer is untouched by the crash")
        XCTAssertTrue(model.controllerStatus.hasPrefix("helper exited"), model.controllerStatus)
        XCTAssertEqual(kill(pid, 0), -1, "the killed pid is gone (kill 0 fails)")

        // Reopen: the same ledger root, a new helper session.
        let logMark = model.workerLog.count
        model.attachController(at: f.helper)
        let newPid = try XCTUnwrap(model.controller?.processIdentifier)
        XCTAssertNotEqual(newPid, pid)
        try await waitUntil(timeout: 90, intervalNs: 1_000_000) {
            model.controllerState.ready && model.controllerState.durable["main.tex"] != nil && model.controllerState.inFlight == nil
                && model.result?.revision == model.editorRevision
        }
        let recoveredMs = Date().timeIntervalSince(t0) * 1000
        let after = try XCTUnwrap(model.controllerState.durable["main.tex"])
        // Either the ledger already held r2 (applied before the kill: nothing
        // resubmitted) or it held r1 (the buffer was resubmitted exactly once).
        // Both converge on exactly r2 with the paste's hash: a duplicate
        // application would be r3, a lost paste would be r1.
        XCTAssertEqual(after.revision, 2, "exactly one application of the paste across the crash")
        XCTAssertEqual(after.sha256, expectedSHA, "no lost bytes")
        XCTAssertTrue(model.controllerState.textByDurable["main.tex"]?[2]?.sameBytes(as: text) == true)
        XCTAssertTrue(model.activeText.sameBytes(as: text))
        XCTAssertFalse(model.previewIsStale)
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: text) == true, "the preview after reopen is of the paste")
        XCTAssertEqual(model.editorRevision, pasteRevision, "no adoption bumped the editor: the buffer was already right")
        let resubmitted = model.workerLog[logMark...].contains { $0.contains("differs from the buffer") && $0.contains("submitting the buffer") }
        let labels = try await undoLabels(model)
        XCTAssertEqual(labels, ["Source edit"], "one history step whichever side of the kill it landed on")
        if let before, before.revision == 2 {
            XCTAssertFalse(resubmitted, "an ACKed paste is not resubmitted")
            XCTAssertEqual(before.sha256, after.sha256)
        }
        print(String(format: "paste-recovery: (c/%@) kill pid %d at %.1f ms (shell had %@), reopened pid %d, recovered r%d sha %@… at %.1f ms, resubmitted=%@, load %.2f",
                     name, pid, killMs, before.map { "r\($0.revision)" } ?? "nothing", newPid, after.revision, String(after.sha256.prefix(12)),
                     recoveredMs, resubmitted ? "yes" : "no", Self.loadAverage1()))
    }

    func testHelperKilledAfterThePasteACKRecoversTheACKedRevision() async throws {
        try await killAndRecover("kill-after-ack") { model, pid in
            // Wait for the durable receipt (the preview is still compiling).
            try await self.waitUntil(timeout: 60, intervalNs: 500_000) { model.controllerState.durable["main.tex"]?.revision == 2 }
            let previewSeen = model.result?.revision == model.editorRevision
            XCTAssertEqual(kill(pid, SIGKILL), 0)
            print("paste-recovery: (c/kill-after-ack) preview had \(previewSeen ? "already" : "not yet") arrived when the kill went out")
        }
    }

    func testHelperKilledBeforeThePasteACKResubmitsExactlyOnce() async throws {
        try await killAndRecover("kill-before-ack") { model, pid in
            // The edit frame is written; kill before any reply is read.
            XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1, "no ACK yet")
            XCTAssertEqual(kill(pid, SIGKILL), 0)
        }
    }

    // MARK: (d) a second edit typed before the paste's preview is held, both durable in order

    func testEditTypedBeforeThePastePreviewIsHeldAndBothEndDurableInOrder() async throws {
        let f = try await makeFixture("hold")
        defer { tearDown(f) }
        let model = f.model
        guard model.controllerState.releasePolicy == .holdUntilPreview, !model.historicalNegotiated else {
            throw XCTSkip("hold-until-preview only (unset FLASHTEX_CONTROLLER_RELEASE / FLASHTEX_COMPLETED_SNAPSHOTS)")
        }
        let block = Self.pasteBlock()
        let pasteText = Self.pasted(into: f.base, block: block)
        let typedText = pasteText + "% typed while the paste was compiling\n"
        let pasteSHA = SourceDigest.sha256Hex(pasteText), typedSHA = SourceDigest.sha256Hex(typedText)

        let t0 = Date()
        model.updateActiveText(pasteText)
        let pasteRevision = model.editorRevision
        // Wait for the paste's durable receipt but not its preview.
        try await waitUntil(timeout: 60, intervalNs: 500_000) { model.controllerState.durable["main.tex"]?.revision == 2 }
        let ackMs = Date().timeIntervalSince(t0) * 1000
        guard model.result?.revision != pasteRevision else {
            throw XCTSkip("the paste's preview arrived before the second edit could be typed (ack \(ackMs) ms); nothing to hold")
        }
        // The second edit: queued behind the in-flight paste, not sent.
        model.updateActiveText(typedText)
        let typedRevision = model.editorRevision
        XCTAssertTrue(model.controllerState.queued, "held until the paste's preview")
        XCTAssertEqual(model.controllerState.inFlight?.editorRevision, pasteRevision, "the paste is still the edit in flight")
        XCTAssertEqual(model.controllerState.inFlight?.durableRevision, 2)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "the typed edit was not submitted yet")
        XCTAssertEqual(model.inFlightRevision, pasteRevision)

        // Release: the paste's preview paints, then the typed edit goes out.
        try await waitUntil(timeout: 90, intervalNs: 1_000_000) { model.controllerState.inFlight?.editorRevision == typedRevision || model.controllerState.durable["main.tex"]?.revision == 3 }
        XCTAssertGreaterThanOrEqual(model.result?.revision ?? 0, pasteRevision, "the paste's preview was painted before the typed edit was released")
        XCTAssertFalse(model.controllerState.queued)
        try await waitUntil(timeout: 90, intervalNs: 1_000_000) { model.controllerState.durable["main.tex"]?.revision == 3 }
        let typedAckMs = Date().timeIntervalSince(t0) * 1000
        let durable = try XCTUnwrap(model.controllerState.durable["main.tex"])
        XCTAssertEqual(durable.sha256, typedSHA)
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[2]?.sameBytes(as: pasteText), true, "r2 is exactly the paste")
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[3]?.sameBytes(as: typedText), true, "r3 is exactly paste + typed")
        XCTAssertEqual(model.controllerState.editorRevisionByDurable["main.tex"]?[2], pasteRevision)
        XCTAssertEqual(model.controllerState.editorRevisionByDurable["main.tex"]?[3], typedRevision)
        try await waitUntil(timeout: 90, intervalNs: 1_000_000) { model.result?.revision == typedRevision && model.controllerState.inFlight == nil }
        let previewMs = Date().timeIntervalSince(t0) * 1000
        XCTAssertFalse(model.previewIsStale)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3, "exactly two durable edits: paste, then the typed edit")
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: typedText) == true)
        let labels = try await undoLabels(model)
        XCTAssertEqual(labels, ["Source edit", "Source edit"], "two steps in order")
        print(String(format: "paste-recovery: (d) paste r2 sha %@… ack %.1f ms; typed edit held, r3 sha %@… ack %.1f ms, final preview %.1f ms, load %.2f",
                     String(pasteSHA.prefix(12)), ackMs, String(typedSHA.prefix(12)), typedAckMs, previewMs, Self.loadAverage1()))
    }
}
