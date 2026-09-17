import XCTest
@testable import FlashTeXMac

/// The `capture_list` client-side stub (`CaptureList.swift`): pure, hermetic,
/// no bridge. Shapes follow `docs/handoffs/transfer-v1-capture-list.md` §3;
/// outcomes, merge, pagination and the restore gate follow §6.
final class CaptureListTests: XCTestCase {
    func testFeatureFlagIsOffByDefaultAndHonoursEnvThenDefaults() {
        let defaults = UserDefaults(suiteName: "capture-list-tests-\(UUID().uuidString)")!
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        XCTAssertFalse(CaptureListFeature.isEnabled(environment: [:], defaults: defaults))
        XCTAssertTrue(CaptureListFeature.isEnabled(environment: ["FLASHTEX_CAPTURE_LIST": "1"], defaults: defaults))
        XCTAssertFalse(CaptureListFeature.isEnabled(environment: ["FLASHTEX_CAPTURE_LIST": "yes"], defaults: defaults))
        defaults.set(true, forKey: CaptureListFeature.defaultsKey)
        XCTAssertTrue(CaptureListFeature.isEnabled(environment: [:], defaults: defaults))
        XCTAssertFalse(CaptureListFeature.isEnabled(environment: ["FLASHTEX_CAPTURE_LIST": "0"], defaults: defaults), "env wins")
        var pager = CaptureList.Pager(base: .init(projectId: "demo"))
        pager.start(enabled: false)
        XCTAssertEqual(pager.phase, .idle, "off: nothing is ever requested")
    }

    func testRequestEncodesTheHandoffExampleWithSnakeCaseKeys() throws {
        let r = CaptureList.Request(projectId: "demo", storeId: "3f1c", destination: .path("main.tex"),
                                    sinceReceipt: "000000000000000017", limit: 64, includeTerminal: false)
        let enc = JSONEncoder(); enc.outputFormatting = [.sortedKeys]
        let json = String(decoding: try enc.encode(r), as: UTF8.self)
        XCTAssertEqual(json, #"{"destination":{"path":"main.tex"},"include_terminal":false,"limit":64,"project_id":"demo","since_receipt":"000000000000000017","store_id":"3f1c"}"#)
        XCTAssertEqual(CaptureList.Request.type, "capture_list")
        XCTAssertEqual(CaptureList.Request(projectId: "p").limit, 64)
        let byId = String(decoding: try enc.encode(CaptureList.Request(projectId: "p", destination: .destinationId("dest-1"))), as: UTF8.self)
        XCTAssertTrue(byId.contains(#""destination":{"destination_id":"dest-1"}"#), byId)
    }

    static let replyJSON = """
    {"project_id":"demo","store_id":"3f1c","captures":[{"receipt":"000000000000000018","capture_id":"acceptance-relaunch-1",
    "status":"journaled","request_sha256":"9e0b","image_mime_type":"image/png","image_bytes":1234,"instructions_bytes":15,
    "destination_id":"dest-1","base_revision":1,"destination_binding":{"project_id":"demo","path":"main.tex","revision":1,
    "start_byte":5,"end_byte":5,"source_sha256":"f93b"},"binding_missing":false,"has_proposal":false,"proposal_sha256":null,
    "prepared":null,"applied":null,"rejected":false,"reason":null}],"next_receipt":null,"truncated":false}
    """

    func testReplyDecodesTheHandoffExample() throws {
        let reply = try JSONDecoder().decode(CaptureList.Reply.self, from: Data(Self.replyJSON.utf8))
        XCTAssertEqual(reply.projectId, "demo"); XCTAssertEqual(reply.storeId, "3f1c")
        XCTAssertFalse(reply.truncated); XCTAssertNil(reply.nextReceipt)
        let row = try XCTUnwrap(reply.captures.first)
        XCTAssertEqual(row.captureId, "acceptance-relaunch-1"); XCTAssertEqual(row.status, .journaled)
        XCTAssertEqual(row.destinationBinding?.sourceSha256, "f93b"); XCTAssertEqual(row.imageBytes, 1234)
        XCTAssertNil(row.prepared); XCTAssertFalse(row.rejected)
        // Round trip keeps every key.
        let again = try JSONDecoder().decode(CaptureList.Reply.self, from: try JSONEncoder().encode(reply))
        XCTAssertEqual(again, reply)
    }

    func row(_ id: String, _ status: CaptureList.Status, receipt: String = "000000000000000001", binding: Bool = true,
             prepared: String? = nil, applied: String? = nil, reason: String? = nil) -> CaptureList.Row {
        .init(receipt: receipt, captureId: id, status: status, destinationId: "dest-1", baseRevision: 1,
              destinationBinding: binding ? .init(projectId: "demo", path: "main.tex", revision: 1, startByte: 5, endByte: 5, sourceSha256: "x") : nil,
              bindingMissing: !binding, prepared: prepared.map { .init(editId: $0, expectedRevision: 1) },
              applied: applied.map { .init(editId: $0, newRevision: 2) }, rejected: status == .rejected, reason: reason)
    }

    func testOutcomePerStatusFollowsTheHandoffTable() {
        let ledger = CaptureList.LedgerKnowledge(knownEditIds: ["e-known", "e-confirmed"], confirmedEditIds: ["e-confirmed"])
        XCTAssertEqual(CaptureList.outcome(for: row("a", .journaled), ledger: ledger), .pending)
        XCTAssertEqual(CaptureList.outcome(for: row("b", .journaled, binding: false), ledger: ledger), .needsReselection(note: "legacy record: capture again"))
        XCTAssertEqual(CaptureList.outcome(for: row("c", .staged, prepared: "e-known"), ledger: ledger), .leftToReconcile)
        XCTAssertEqual(CaptureList.outcome(for: row("d", .staged, prepared: "e-unknown"), ledger: ledger),
                       .needsReselection(note: "issued edit never recorded here — resolve before a new attempt"))
        XCTAssertEqual(CaptureList.outcome(for: row("e", .inserted, applied: "e-confirmed"), ledger: ledger), .confirmed)
        XCTAssertEqual(CaptureList.outcome(for: row("f", .inserted, applied: "e-known"), ledger: ledger),
                       .insertedUnconfirmed(note: "inserted on the bridge; no confirmed receipt in this ledger (audit)"))
        XCTAssertEqual(CaptureList.outcome(for: row("g", .rejected), ledger: ledger), .rejected)
        XCTAssertEqual(CaptureList.outcome(for: row("h", .failed, reason: "invalid_journal: digest mismatch"), ledger: ledger),
                       .failed(reason: "invalid_journal: digest mismatch"))
        XCTAssertEqual(CaptureList.outcome(for: row("i", .failed), ledger: ledger).note, "record could not be loaded")
        XCTAssertEqual(CaptureList.Outcome.pending.note, "received before relaunch; review to continue")
    }

    func testMergeNeverDuplicatesAndKeepsRowsThisSessionOwns() {
        let existing = [BridgeSession.Capture(captureId: "mine-1", destinationId: "dest-1", state: .proposed, note: "converted here")]
        let rows = [row("listed-2", .journaled, receipt: "000000000000000002"),
                    row("mine-1", .journaled, receipt: "000000000000000001"),
                    row("listed-2", .journaled, receipt: "000000000000000002"), // a repeated page
                    row("listed-3", .rejected, receipt: "000000000000000003")]
        let plan = CaptureList.merge(rows: rows, into: existing, ledger: .init())
        XCTAssertEqual(plan.map(\.captureId), ["mine-1", "listed-2", "listed-3"], "existing first, then receipt order, no duplicates")
        guard plan.count == 3 else { return XCTFail("expected three plan rows, got \(plan.count)") }
        XCTAssertEqual(plan[0].kind, .keepExisting(.proposed), "the session's own row keeps its state")
        XCTAssertEqual(plan[1].kind, .add(.pending))
        XCTAssertEqual(plan[2].kind, .add(.rejected))
        XCTAssertEqual(CaptureList.merge(rows: [], into: existing, ledger: .init()).count, 1)
    }

    func testPagerFollowsCursorsAndStopsAtEightPages() {
        var pager = CaptureList.Pager(base: .init(projectId: "demo", destination: .path("main.tex")))
        pager.start(enabled: true)
        guard case .listing(1, let first) = pager.phase else { return XCTFail("\(pager.phase)") }
        XCTAssertNil(first.sinceReceipt); XCTAssertEqual(first.limit, 64)
        for n in 1...8 {
            guard case .listing(n, let req) = pager.phase else { return XCTFail("page \(n): \(pager.phase)") }
            XCTAssertEqual(req.sinceReceipt, n == 1 ? nil : String(format: "%018d", (n - 1) * 2))
            pager.received(.init(projectId: "demo", storeId: nil, captures: [row("c\(n)a", .journaled, receipt: String(format: "%018d", n * 2 - 1)),
                                                                           row("c\(n)b", .journaled, receipt: String(format: "%018d", n * 2))],
                                 nextReceipt: String(format: "%018d", n * 2), truncated: true))
        }
        XCTAssertEqual(pager.phase, .moreAvailable(pages: 8, note: "more journaled captures; open the capture list to load more"))
        XCTAssertEqual(pager.rows.count, 16, "every page's rows are kept")
        pager.received(.init(projectId: "demo", storeId: nil, captures: [], nextReceipt: nil, truncated: false))
        XCTAssertEqual(pager.rows.count, 16, "a reply after the listing ended is ignored")

        // A short listing ends on the first untruncated page.
        pager.reset(); pager.start(enabled: true)
        pager.received(.init(projectId: "demo", storeId: nil, captures: [row("x", .journaled)], nextReceipt: nil, truncated: false))
        XCTAssertEqual(pager.phase, .done(pages: 1)); XCTAssertEqual(pager.rows.map(\.captureId), ["x"])

        // A truncated reply without a cursor cannot be followed: done, not a loop.
        pager.reset(); pager.start(enabled: true)
        pager.received(.init(projectId: "demo", storeId: nil, captures: [], nextReceipt: nil, truncated: true))
        XCTAssertEqual(pager.phase, .done(pages: 1))
    }

    func testPagerFailureKeepsReceivedRowsAndNamesTheCode() {
        var pager = CaptureList.Pager(base: .init(projectId: "demo"))
        pager.start(enabled: true)
        pager.received(.init(projectId: "demo", storeId: nil, captures: [row("kept", .journaled)], nextReceipt: "000000000000000001", truncated: true))
        pager.failed(code: "store_unavailable", message: "journal directory unreadable")
        XCTAssertEqual(pager.phase, .failed(note: "capture listing unavailable, retry: store_unavailable — journal directory unreadable"))
        XCTAssertEqual(pager.rows.map(\.captureId), ["kept"])
        pager.reset(); pager.start(enabled: true)
        pager.failed(code: "store_mismatch", message: "store 3f1c != 9a00")
        XCTAssertEqual(pager.phase, .failed(note: "capture listing refused: store_mismatch — store 3f1c != 9a00"))
        pager.reset(); pager.start(enabled: true)
        pager.failed(code: nil, message: "bridge exited")
        XCTAssertEqual(pager.phase, .failed(note: "capture listing refused: transport — bridge exited"))
        XCTAssertTrue(CaptureList.Refusal.storeUnavailable.isTransient)
        for c in [CaptureList.Refusal.storeMismatch, .destinationUnbound, .badRequest, .invalidLimit, .invalidCursor] { XCTAssertFalse(c.isTransient) }
    }

    func testRestoreDestinationGateRequiresExactTextAndRevision() {
        let text = "\\section{Intro}\nHello."
        var r = row("a", .journaled)
        r.destinationBinding?.sourceSha256 = CaptureList.sha256Hex(text).uppercased()
        r.destinationBinding?.revision = 4
        XCTAssertTrue(CaptureList.canRestoreDestination(r, activeText: text, editorRevision: 4))
        XCTAssertFalse(CaptureList.canRestoreDestination(r, activeText: text + " ", editorRevision: 4), "buffer changed")
        XCTAssertFalse(CaptureList.canRestoreDestination(r, activeText: text, editorRevision: 5), "revision moved")
        var legacy = r; legacy.bindingMissing = true
        XCTAssertFalse(CaptureList.canRestoreDestination(legacy, activeText: text, editorRevision: 4))
        var staged = r; staged.status = .staged
        XCTAssertFalse(CaptureList.canRestoreDestination(staged, activeText: text, editorRevision: 4), "only journaled rows are re-pinned")
        XCTAssertEqual(CaptureList.sha256Hex(""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    }
}
