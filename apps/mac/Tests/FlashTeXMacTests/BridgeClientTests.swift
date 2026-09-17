import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Transport round trips through `Fixtures/fake_bridge.py` (a Python test
/// double, not the Rust bridge): one reply per request type, error envelopes
/// with codes, and the protocol-violation paths.
final class BridgeClientTests: XCTestCase {
    static let fakeBridge = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_bridge.py")
    static let python = URL(fileURLWithPath: "/usr/bin/python3")
    static var repoRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
    static func fixtureCapture() throws -> RuntimeV1.CaptureSubmit {
        try RuntimeV1.decodeCaptureSubmit(Data(contentsOf: repoRoot.appendingPathComponent("protocol/fixtures/capture-submission.json"))).payload
    }
    static func tempStore() throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-bridge-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    private func makeClient(events: @escaping (BridgeClient.Event) -> Void = { _ in }) throws -> BridgeClient {
        try BridgeClient(executable: Self.python, arguments: [Self.fakeBridge.path], storeDirectory: try Self.tempStore(),
                         queue: DispatchQueue(label: "bridge-test"), events: events)
    }

    func testEveryRequestTypeRoundTripsWithSnakeCaseKeys() async throws {
        let client = try makeClient()
        defer { client.terminate() }
        let text = "Hello naïve FlashTeX.\n"
        _ = try await client.request(.documentOpen, TransferV1.DocumentOpen(projectId: "demo", path: "main.tex", revision: 1, text: text), as: TransferV1.Empty.self)
        let updated = try await client.request(.documentEdit, TransferV1.DocumentEdit(projectId: "demo", path: "main.tex", baseRevision: 1, revision: 2,
                                                                                   startByte: 0, endByte: 5, replacement: "Hullo"), as: TransferV1.DocumentUpdated.self)
        XCTAssertEqual(updated.revision, 2)
        // Pin after "naïve " (ï is two bytes): byte 13.
        let anchor = try await client.request(.destinationPin, TransferV1.DestinationPin(destinationId: "fixture-anchor-1", projectId: "demo", path: "main.tex",
                                                                                      revision: 2, startByte: 13, endByte: 13), as: TransferV1.Anchor.self)
        XCTAssertEqual(anchor.pinnedRevision, 2)
        XCTAssertTrue(anchor.valid)
        XCTAssertEqual(anchor.binding.sourceSha256, SourceDigest.sha256Hex("Hullo naïve FlashTeX.\n"))

        var capture = try Self.fixtureCapture()
        capture.baseRevision = 2
        let received = try await client.request(.captureSubmit, capture, as: TransferV1.CaptureReceived.self)
        XCTAssertEqual(received, .init(captureId: "fixture-capture-1", durable: true, hasProposal: false, applied: false))
        // Identical retry returns the same record; a different payload conflicts.
        let again = try await client.request(.captureSubmit, capture, as: TransferV1.CaptureReceived.self)
        XCTAssertEqual(again, received)
        var different = capture
        different.instructions = "changed"
        await assertBridgeError("capture_id_conflict") { try await client.request(.captureSubmit, different, as: TransferV1.CaptureReceived.self) }

        let proposal = try await client.request(.captureConvert, TransferV1.CaptureConvert(captureId: capture.captureId, supportedFeatures: []),
                                                as: RuntimeV1.CaptureProposal.self)
        XCTAssertEqual(proposal.latex, "\\fakecapture{fixture-capture-1}")
        XCTAssertEqual(proposal.ambiguities.count, 1)
        XCTAssertEqual(proposal.contextRevision, 2)

        let edit = try await client.request(.capturePrepareInsert, TransferV1.CapturePrepareInsert(captureId: capture.captureId, expectedRevision: 2, approved: true),
                                            as: TransferV1.CaptureEdit.self)
        XCTAssertEqual(edit.editId, "capture-fixture-capture-1")
        XCTAssertEqual(edit.startByte, 13); XCTAssertEqual(edit.endByte, 13)
        XCTAssertEqual(edit.removedText, "")
        XCTAssertEqual(edit.documentBeforeSha256, SourceDigest.sha256Hex("Hullo naïve FlashTeX.\n"))

        let status = try await client.request(.captureStatus, TransferV1.CaptureID(captureId: capture.captureId), as: TransferV1.CaptureStatus.self)
        XCTAssertEqual(status.prepared, edit)
        XCTAssertNil(status.applied)
        XCTAssertFalse(status.rejected)
        XCTAssertEqual(status.proposal?.latex, proposal.latex)

        let receipt = try await client.request(.captureApplied, TransferV1.CaptureApplied(captureId: capture.captureId, editId: edit.editId, newRevision: 3),
                                               as: TransferV1.CaptureApplied.self)
        XCTAssertEqual(receipt.newRevision, 3)
        // Rejection after an issued edit is refused; unknown captures are missing.
        await assertBridgeError("receipt_conflict") { try await client.request(.captureReject, TransferV1.CaptureID(captureId: capture.captureId), as: TransferV1.CaptureID.self) }
        await assertBridgeError("capture_missing") { try await client.request(.captureStatus, TransferV1.CaptureID(captureId: "never-sent"), as: TransferV1.CaptureStatus.self) }

        // A fresh capture can be rejected; conversion then fails with capture_rejected.
        var second = capture
        second.captureId = "fixture-capture-2"
        await assertBridgeError("destination_reselection_required") { try await client.request(.captureSubmit, second, as: TransferV1.CaptureReceived.self) }
        _ = try await client.request(.destinationPin, TransferV1.DestinationPin(destinationId: "fixture-anchor-2", projectId: "demo", path: "main.tex",
                                                                             revision: 3, startByte: 0, endByte: 5), as: TransferV1.Anchor.self)
        second.destinationId = "fixture-anchor-2"; second.baseRevision = 3
        _ = try await client.request(.captureSubmit, second, as: TransferV1.CaptureReceived.self)
        let rejected = try await client.request(.captureReject, TransferV1.CaptureID(captureId: second.captureId), as: TransferV1.CaptureID.self)
        XCTAssertEqual(rejected.captureId, second.captureId)
        await assertBridgeError("capture_rejected") { try await client.request(.captureConvert, TransferV1.CaptureConvert(captureId: second.captureId, supportedFeatures: []), as: RuntimeV1.CaptureProposal.self) }
        await assertBridgeError("already_applied") { try await client.request(.capturePrepareInsert, TransferV1.CapturePrepareInsert(captureId: capture.captureId, expectedRevision: 99, approved: true), as: TransferV1.CaptureEdit.self) }
        await assertBridgeError("review_required") { try await client.request(.capturePrepareInsert, TransferV1.CapturePrepareInsert(captureId: second.captureId, expectedRevision: 3, approved: false), as: TransferV1.CaptureEdit.self) }
    }

    func testProviderErrorsAreErrorsWithCodesNotCrashes() async throws {
        let client = try makeClient()
        defer { client.terminate() }
        _ = try await client.request(.documentOpen, TransferV1.DocumentOpen(projectId: "demo", path: "main.tex", revision: 1, text: "x"), as: TransferV1.Empty.self)
        _ = try await client.request(.destinationPin, TransferV1.DestinationPin(destinationId: "d1", projectId: "demo", path: "main.tex", revision: 1, startByte: 1, endByte: 1), as: TransferV1.Anchor.self)
        var capture = try Self.fixtureCapture()
        capture.destinationId = "d1"; capture.instructions = "%provider_disabled"
        _ = try await client.request(.captureSubmit, capture, as: TransferV1.CaptureReceived.self)
        await assertBridgeError("provider_disabled") { try await client.request(.captureConvert, TransferV1.CaptureConvert(captureId: capture.captureId, supportedFeatures: []), as: RuntimeV1.CaptureProposal.self) }
        var auth = capture
        auth.captureId = "auth-1"; auth.instructions = "%provider_auth_missing"
        _ = try await client.request(.captureSubmit, auth, as: TransferV1.CaptureReceived.self)
        await assertBridgeError("provider_auth_missing") { try await client.request(.captureConvert, TransferV1.CaptureConvert(captureId: "auth-1", supportedFeatures: []), as: RuntimeV1.CaptureProposal.self) }
        // Wrong reply type for the request is a failure, not a silent decode.
        do {
            _ = try await client.request(.captureStatus, TransferV1.CaptureID(captureId: "auth-1"), as: TransferV1.CaptureEdit.self)
            XCTFail("capture_status decoded as capture_edit")
        } catch let f as BridgeClient.Failure {
            if case .undecodable = f {} else { XCTFail("\(f)") }
        }
    }

    func testRequestedErrorGarbageAndOversizedLines() async throws {
        let violation = expectation(description: "violation"), exited = expectation(description: "exited")
        violation.assertForOverFulfill = false // the garbage line is one violation, the oversized line another
        let client = try makeClient { event in
            switch event {
            case .protocolViolation: violation.fulfill()
            case .exited: exited.fulfill()
            default: break
            }
        }
        var capture = try Self.fixtureCapture()
        capture.instructions = "%error:storage_error"
        await assertBridgeError("storage_error") { try await client.request(.captureSubmit, capture, as: TransferV1.CaptureReceived.self) }
        // Garbage line: reported as a violation event, pending request keeps waiting (no reply); next request still works.
        capture.instructions = "%garbage"
        let garbageDone = expectation(description: "garbage never answered"); garbageDone.isInverted = true
        client.send(.captureSubmit, capture, as: TransferV1.CaptureReceived.self) { _ in garbageDone.fulfill() }
        await fulfillment(of: [garbageDone], timeout: 0.5)
        // Oversized line (> 12 MiB): violation, bridge terminated, pending request fails.
        capture.instructions = "%huge"
        do {
            _ = try await client.request(.captureSubmit, capture, as: TransferV1.CaptureReceived.self)
            XCTFail("oversized line delivered")
        } catch let f as BridgeClient.Failure {
            switch f { case .protocolViolation, .exited: break; default: XCTFail("\(f)") }
        }
        await fulfillment(of: [violation, exited], timeout: 30)
        XCTAssertFalse(client.isRunning)
        do { _ = try await client.request(.captureStatus, TransferV1.CaptureID(captureId: "x"), as: TransferV1.CaptureStatus.self); XCTFail() }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f, .notRunning) }
    }

    func testTrailingBytesAtExitAndOversizedRequest() async throws {
        let violation = expectation(description: "violation")
        var message = ""
        let client = try makeClient { if case .protocolViolation(let m) = $0 { message = m; violation.fulfill() } }
        var capture = try Self.fixtureCapture()
        capture.instructions = "%trailing"
        do { _ = try await client.request(.captureSubmit, capture, as: TransferV1.CaptureReceived.self); XCTFail() }
        catch let f as BridgeClient.Failure { if case .exited = f {} else { XCTFail("\(f)") } }
        await fulfillment(of: [violation], timeout: 10)
        XCTAssertTrue(message.contains("unterminated"), message)

        let fresh = try makeClient()
        defer { fresh.terminate() }
        let huge = TransferV1.DocumentOpen(projectId: "demo", path: "main.tex", revision: 1, text: String(repeating: "x", count: TransferV1.maxLineBytes))
        do { _ = try await fresh.request(.documentOpen, huge, as: TransferV1.Empty.self); XCTFail() }
        catch let f as BridgeClient.Failure { if case .requestTooLarge = f {} else { XCTFail("\(f)") } }
        XCTAssertTrue(fresh.isRunning, "an oversized request is refused locally without touching the bridge")
    }

    func testChangedRegionIsScalarAligned() {
        let r = SourceMapping.changedRegion(from: "café", to: "cafè")
        XCTAssertEqual(r.startByte, 3); XCTAssertEqual(r.oldEndByte, 5); XCTAssertEqual(r.newEndByte, 5)
        XCTAssertEqual(r.replacement, "è")
        let ins = SourceMapping.changedRegion(from: "ab", to: "aXb")
        XCTAssertEqual(ins, .init(startByte: 1, oldEndByte: 1, newEndByte: 2, replacement: "X"))
        let del = SourceMapping.changedRegion(from: "naïve", to: "nave")
        XCTAssertEqual(del.replacement, "")
        XCTAssertEqual(del.oldEndByte - del.startByte, 2)
        XCTAssertEqual(SourceMapping.changedRegion(from: "same", to: "same"), .init(startByte: 4, oldEndByte: 4, newEndByte: 4, replacement: ""))
    }

    static let fakeEditLedger = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_edit_ledger.py")

    /// Edit-ledger helper protocol round trip through `Fixtures/fake_edit_ledger.py`
    /// (a Python double of crates/edit-ledger d1dd1d7): initialize → apply →
    /// status → replace_document → confirm, plus its error codes.
    func testEditLedgerHelperRoundTrip() async throws {
        let store = try Self.tempStore().appendingPathComponent("doc")
        let client = try EditLedgerClient(executable: Self.python, arguments: [Self.fakeEditLedger.path], storeDirectory: store,
                                          queue: DispatchQueue(label: "ledger-test"))
        defer { client.terminate() }
        let empty = try await client.status()
        XCTAssertNil(empty.document); XCTAssertEqual(empty.pendingReceipts, [])
        let text = "Hello naïve FlashTeX.\n"
        let doc = try await client.initialize(.init(projectId: "demo", path: "main.tex", revision: 3, text: text))
        XCTAssertEqual(doc.revision, 3); XCTAssertEqual(doc.sourceSha256, SourceDigest.sha256Hex(text))
        let edit = TransferV1.CaptureEdit(captureId: "c1", editId: "capture-c1", projectId: "demo", path: "main.tex", expectedRevision: 3,
                                          startByte: 13, endByte: 13, removedText: "", replacement: "X", documentBeforeSha256: doc.sourceSha256)
        let applied = try await client.apply(edit)
        XCTAssertEqual(applied.receipt, .init(captureId: "c1", editId: "capture-c1", newRevision: 4))
        XCTAssertEqual(applied.document.text, "Hello naïve XFlashTeX.\n")
        XCTAssertTrue(FileManager.default.fileExists(atPath: store.appendingPathComponent("document.json").path), "durable before the reply")
        let again = try await client.apply(edit)
        XCTAssertEqual(again.receipt, applied.receipt, "identical retry returns the original receipt")
        XCTAssertEqual(again.document, applied.document, "and never inserts twice")
        var different = edit; different.replacement = "Y"
        await assertBridgeError("edit_id_conflict") { try await client.apply(different) }
        var otherEdit = edit; otherEdit.editId = "capture-c1-b"; otherEdit.expectedRevision = 4; otherEdit.documentBeforeSha256 = applied.document.sourceSha256
        await assertBridgeError("capture_id_conflict") { try await client.apply(otherEdit) }
        let st = try await client.status()
        XCTAssertEqual(st.pendingReceipts.count, 1)
        XCTAssertEqual(st.pendingReceipts.first?.documentBefore?.text, text, "before-source retained until confirmed")
        let pendingReceipt = try XCTUnwrap(st.pendingReceipts.first)
        XCTAssertFalse(pendingReceipt.confirmed)
        // Undo (ordinary replace) keeps the tombstone: the same edit still cannot insert twice.
        let undone = try await client.replaceDocument(expectedRevision: 4, expectedSha256: applied.document.sourceSha256, text: text)
        XCTAssertEqual(undone.revision, 5); XCTAssertEqual(undone.text, text)
        await assertBridgeError("document_conflict") { try await client.replaceDocument(expectedRevision: 4, expectedSha256: "stale", text: "x") }
        let retry = try await client.apply(edit)
        XCTAssertEqual(retry.receipt, applied.receipt); XCTAssertEqual(retry.document.text, text, "dedup survives undo")
        _ = try await client.confirm(applied.receipt)
        let after = try await client.status()
        XCTAssertEqual(after.pendingReceipts, [])
        await assertBridgeError("receipt_conflict") { try await client.confirm(.init(captureId: "c1", editId: "capture-c1", newRevision: 9)) }
        await assertBridgeError("edit_missing") { try await client.confirm(.init(captureId: "zz", editId: "nope", newRevision: 1)) }
        await assertBridgeError("document_exists") { try await client.initialize(.init(projectId: "demo", path: "main.tex", revision: 1, text: "other")) }
    }

    private func assertBridgeError<T>(_ code: String, file: StaticString = #filePath, line: UInt = #line, _ body: () async throws -> T) async {
        do { _ = try await body(); XCTFail("expected \(code)", file: file, line: line) }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f.code, code, f.text, file: file, line: line) }
        catch { XCTFail("\(error)", file: file, line: line) }
    }
}
