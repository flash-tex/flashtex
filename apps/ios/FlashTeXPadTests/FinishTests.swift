import FlashTeXPadKit
import NearbyClient
import Security
import XCTest
@testable import FlashTeXPad

/// The three documented gaps, closed and proven against the in-process
/// FakeMac: the Mac-side outcome reaches the iPad (`capture_status`, with
/// the LaTeX read-only), QR-payload pairing, and persistence (Keychain
/// pairing, drafts / receipts / outcomes on disk) across a "relaunch".
@MainActor
final class FinishTests: XCTestCase {
    var mac: FakeMac!
    let salt = Data((0..<16).map { UInt8($0 * 7 + 3) })
    let code = "440912"
    var tmp: URL!

    override func setUp() async throws {
        let d = NearbyCrypto.derive(code: code, salt: salt)
        mac = try FakeMac(keys: [.init(identity: d.pairId, psk: d.psk, bootstrap: true)], macName: "Status Mac",
                          destination: NearbyWire.Destination(destinationId: "dest-1", projectId: "demo", path: "main.tex", baseRevision: 4))
        mac.start()
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("FinishTests-\(UUID().uuidString)", isDirectory: true)
    }

    override func tearDown() async throws { mac.stop(); try? FileManager.default.removeItem(at: tmp) }

    private func pairedModel(store: CaptureStore? = nil, pairs: PairingStore? = nil) async -> PadModel {
        let m = PadModel(link: MacLink(store: pairs), captureStore: store)
        m.pollInterval = 0.05
        await m.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                     fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Status Mac", code: code)
        XCTAssertNil(m.linkError)
        return m
    }

    private func waitUntil(_ timeout: TimeInterval = 5, _ cond: () -> Bool) async {
        let end = Date().addingTimeInterval(timeout)
        while !cond(), Date() < end { try? await Task.sleep(nanoseconds: 20_000_000) }
    }

    // MARK: 1. outcome status

    func testOutcomeFollowsTheMacThroughProposalAndInsertion() async throws {
        let model = await pairedModel()
        let r = model.draft(CaptureRecord(source: .pencil, png: CaptureQueueTests.trianglePNG(), instructions: "to TikZ"))
        await model.send(r.id)
        guard case .received? = model.captures.first?.status else { return XCTFail("\(String(describing: model.captures.first?.status))") }
        // Unscripted: the FakeMac answers from its inbox → received, not final, polling continues.
        await waitUntil { model.captures.first?.outcome != nil }
        XCTAssertEqual(model.captures.first?.outcome?.state, "received")
        XCTAssertFalse(model.captures.first?.outcomeIsFinal ?? true)
        // The Mac moves on: journaled → converting → proposal_ready (LaTeX carried) → inserted (final).
        mac.setStatus(r.id, state: "converting", note: "converting…")
        await waitUntil { model.captures.first?.outcome?.state == "converting" }
        XCTAssertEqual(model.captures.first?.outcomeLabel, "converting on the Mac…")
        mac.setStatus(r.id, state: "proposal_ready", latex: "\\begin{tikzpicture}\\draw (0,0)--(1,0)--(0.5,1)--cycle;\\end{tikzpicture}", note: "awaiting review")
        await waitUntil { model.captures.first?.outcome?.state == "proposal_ready" }
        XCTAssertEqual(model.captures.first?.outcome?.latex, "\\begin{tikzpicture}\\draw (0,0)--(1,0)--(0.5,1)--cycle;\\end{tikzpicture}")
        XCTAssertEqual(model.captures.first?.outcomeLabel, "proposal ready — review and approve it on the Mac")
        mac.setStatus(r.id, state: "inserted", latex: "\\begin{tikzpicture}\\end{tikzpicture}", newRevision: 5)
        await waitUntil { model.captures.first?.outcome?.state == "inserted" }
        XCTAssertEqual(model.captures.first?.outcome?.newRevision, 5)
        XCTAssertTrue(model.captures.first?.outcomeIsFinal ?? false)
        let polls = mac.statusRequests.count
        try await Task.sleep(nanoseconds: 200_000_000)
        XCTAssertEqual(mac.statusRequests.count, polls, "polling stops at a final state")
        XCTAssertEqual(mac.captures.count, 1, "a status probe never re-delivers")
    }

    func testOlderMacWithoutCaptureStatusIsReportedNotGuessed() async throws {
        mac.answersStatus = false
        let model = await pairedModel()
        let r = model.draft(CaptureRecord(source: .sample, png: CaptureQueueTests.trianglePNG(), instructions: "x"))
        await model.send(r.id)
        await waitUntil { model.captures.first?.outcomeProblem != nil }
        XCTAssertNil(model.captures.first?.outcome)
        XCTAssertTrue(model.captures.first?.outcomeProblem?.contains("unknown_type") ?? false)
        XCTAssertFalse(model.queue.shouldPoll(r.id), "no polling loop against a Mac that cannot answer")
        XCTAssertEqual(model.captures.first?.outcomeLabel?.hasPrefix("outcome unavailable"), true)
    }

    func testStatusAckNamingAnotherCaptureIsAProtocolViolation() async throws {
        let model = await pairedModel()
        let r = model.draft(CaptureRecord(source: .pencil, png: CaptureQueueTests.trianglePNG(), instructions: "x"))
        mac.statusEchoWrongId = true
        await model.send(r.id)
        await waitUntil { model.captures.first?.outcomeProblem != nil }
        XCTAssertNil(model.captures.first?.outcome, "a mismatched echo is never shown as this capture's outcome")
        XCTAssertTrue(model.captures.first?.outcomeProblem?.contains("protocol violation") ?? false, model.captures.first?.outcomeProblem ?? "nil")
        do {
            _ = try await model.link.captureStatus(captureId: r.id)
            XCTFail("expected protocolViolation")
        } catch NearbyError.protocolViolation(let why) {
            XCTAssertTrue(why.contains("someone-else"))
        }
    }

    /// The Mac's per-pairing acknowledgement memory lost the capture (here: a
    /// restarted FakeMac with nothing received) while the iPad still holds
    /// the saved envelope: `unknown_capture` → the iPad re-delivers the same
    /// capture_id / destination / base_revision → acknowledged → status answered.
    func testUnknownCaptureAfterMacRestartIsRecoveredByRedelivery() async throws {
        let model = await pairedModel()
        model.maxPolls = 1
        let r = model.draft(CaptureRecord(source: .pencil, png: CaptureQueueTests.trianglePNG(), instructions: "keep me"))
        await model.send(r.id)
        await waitUntil { model.captures.first?.outcome != nil }
        XCTAssertEqual(mac.captures.count, 1)
        let pairId = try XCTUnwrap(model.pairedMac?.pairId)
        model.maxPolls = 0 // the explicit probe below is the only one
        mac = try FakeMac.restart(mac, keys: [.init(identity: pairId, psk: mac.longTermPSK, bootstrap: false)])
        XCTAssertNotEqual(mac.port, 0, "restarted FakeMac is listening on a fresh port")
        mac.destination = NearbyWire.Destination(destinationId: "dest-2", projectId: "demo", path: "main.tex", baseRevision: 9) // the pin moved meanwhile
        mac.setStatus(r.id, state: "proposal_ready", latex: "\\alpha")
        await model.reconnect(host: "127.0.0.1", port: String(mac.port))
        XCTAssertNil(model.linkError)
        await model.refreshOutcome(r.id)
        XCTAssertEqual(mac.statusRequests, [r.id, r.id], "refused once, answered after the re-delivery")
        XCTAssertEqual(mac.captures.count, 1)
        XCTAssertEqual(mac.captures[0].captureId, r.id)
        XCTAssertEqual(mac.captures[0].destinationId, "dest-1", "re-delivery keeps the original destination")
        XCTAssertEqual(mac.captures[0].baseRevision, 4, "and the original base_revision (a fresh destination_query would change the payload)")
        XCTAssertEqual(mac.captures[0].instructions, "keep me")
        XCTAssertEqual(model.queue.redeliveries, [r.id])
        XCTAssertEqual(model.captures.first?.outcome?.state, "proposal_ready")
        XCTAssertEqual(model.captures.first?.outcome?.latex, "\\alpha")
    }

    /// An interrupted send (receipt lost) retries with the payload it first
    /// left with — saved destination and base_revision — even though the
    /// Mac's pin moved meanwhile; a fresh destination_query would change the
    /// payload under the same capture_id and the real listener refuses that.
    func testInterruptedStoredCaptureKeepsItsOriginalDestinationOnRetry() async throws {
        let store = CaptureStore(directory: tmp)
        let png = CaptureQueueTests.trianglePNG()
        let d = NearbyCrypto.derive(code: code, salt: salt)
        var interrupted = CaptureRecord(id: "cap-interrupted", source: .pencil, png: png, instructions: "first payload")
        interrupted.status = .sending(attempt: 1)
        interrupted.destinationId = "dest-1"; interrupted.baseRevision = 4
        interrupted.pairId = d.pairId; interrupted.macFingerprint = NearbyCrypto.fingerprint(salt: salt)
        try store.save([interrupted])
        mac.destination = NearbyWire.Destination(destinationId: "dest-moved", projectId: "demo", path: "main.tex", baseRevision: 9)
        let model = await pairedModel(store: store)
        model.maxPolls = 0
        XCTAssertEqual(model.pairedMac?.pairId, d.pairId, "the bootstrap pair_id is the pairing's id")
        guard case .disconnected(_, 1)? = model.captures.first?.status else { return XCTFail("\(String(describing: model.captures.first?.status))") }
        await model.send("cap-interrupted")
        guard case .received? = model.captures.first?.status else { return XCTFail("\(String(describing: model.captures.first?.status))") }
        XCTAssertEqual(mac.captures.count, 1)
        XCTAssertEqual(mac.captures[0].destinationId, "dest-1")
        XCTAssertEqual(mac.captures[0].baseRevision, 4)
        XCTAssertEqual(mac.captures[0].instructions, "first payload")
        XCTAssertEqual(Data(base64Encoded: mac.captures[0].image.dataBase64), png)
        // A never-sent draft resolves the current pin (once) and freezes the pairing.
        let fresh = model.draft(CaptureRecord(source: .sample, png: png, instructions: "new"))
        await model.send(fresh.id)
        XCTAssertEqual(mac.captures.last?.destinationId, "dest-moved")
        XCTAssertEqual(model.captures.first?.pairId, d.pairId)
        XCTAssertEqual(model.captures.first?.macFingerprint, NearbyCrypto.fingerprint(salt: salt))
    }

    /// Records are bound to the Mac they were first sent to: on a link to a
    /// different Mac they are neither polled, re-delivered nor retried.
    func testCapturesSentToAnotherMacAreNeverPolledOrResentHere() async throws {
        let store = CaptureStore(directory: tmp)
        let png = CaptureQueueTests.trianglePNG()
        var foreign = CaptureRecord(id: "cap-other-mac", source: .pencil, png: png, instructions: "secret sketch")
        foreign.status = .received(FakeMac.wire(["capture_id": "cap-other-mac", "durable": true, "has_proposal": false, "applied": false]) as NearbyWire.CaptureReceived)
        foreign.destinationId = "dest-x"; foreign.baseRevision = 2
        foreign.pairId = "pair-of-another-mac"; foreign.macFingerprint = "0123456789abcdef"
        var retry = CaptureRecord(id: "cap-other-mac-retry", source: .pencil, png: png, instructions: "secret retry")
        retry.status = .disconnected(reason: "dropped", attempt: 1)
        retry.destinationId = "dest-x"; retry.baseRevision = 2
        retry.pairId = "pair-of-another-mac"; retry.macFingerprint = "0123456789abcdef"
        try store.save([foreign, retry])
        let model = await pairedModel(store: store) // reconnect path calls resumeOutcomePolling too
        model.resumeOutcomePolling()
        XCTAssertFalse(model.queue.shouldPoll("cap-other-mac"))
        await model.refreshOutcome("cap-other-mac")
        XCTAssertTrue(model.captures.first { $0.id == "cap-other-mac" }?.outcomeProblem?.contains("another Mac") ?? false)
        await model.send("cap-other-mac-retry")
        guard case .refused(let code, let msg)? = model.captures.first { $0.id == "cap-other-mac-retry" }?.status else { return XCTFail() }
        XCTAssertEqual(code, "invalid_input"); XCTAssertTrue(msg.contains("another Mac") || msg.contains("the Mac 0123456789abcdef"), msg)
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertTrue(mac.statusRequests.isEmpty, "no capture_status for another Mac's capture")
        XCTAssertTrue(mac.captures.isEmpty, "no re-delivery and no retry of another Mac's payload")
    }

    // MARK: 2. QR pairing (payload path; the simulator has no camera)

    func testPairFromTheMacsQRPayloadWithTypedHostPort() async throws {
        let payload = NearbyBootstrapPayload(code: code, salt: salt, macName: "Status Mac")
        XCTAssertTrue(payload.urlString.hasPrefix("flashtex-nearby://pair?v=1&code=\(code)&salt=\(NearbyCrypto.hex(salt))&fp=\(NearbyCrypto.fingerprint(salt: salt))&name="))
        let model = PadModel(link: MacLink(store: nil))
        let ok = await model.pair(bootstrapText: " \(payload.urlString)\n", host: "127.0.0.1", port: String(mac.port))
        XCTAssertTrue(ok, model.linkError ?? "")
        XCTAssertEqual(model.pairedMac?.fingerprint, NearbyCrypto.fingerprint(salt: salt))
        XCTAssertEqual(model.pairedMac?.macName, "Status Mac")
        XCTAssertTrue(model.link.isConnected)
        XCTAssertEqual(mac.hellos.count, 1)
        XCTAssertEqual(model.destination?.destinationId, "dest-1")
        // A tampered payload (fp not derived from the salt) never connects.
        let bad = payload.urlString.replacingOccurrences(of: "fp=\(NearbyCrypto.fingerprint(salt: salt))", with: "fp=0000000000000000")
        let m2 = PadModel(link: MacLink(store: nil))
        let badOK = await m2.pair(bootstrapText: bad, host: "127.0.0.1", port: String(mac.port))
        XCTAssertFalse(badOK)
        XCTAssertTrue(m2.linkError?.contains("fp does not match") ?? false, m2.linkError ?? "nil")
        XCTAssertEqual(mac.hellos.count, 1, "nothing was sent for the tampered payload")
        let foreignOK = await m2.pair(bootstrapText: "https://example.com/?code=1", host: "127.0.0.1", port: String(mac.port))
        XCTAssertFalse(foreignOK)
        // No host/port and no Bonjour service with that fp: an explicit error, not a hang.
        let m3 = PadModel(link: MacLink(store: nil))
        let browsed = await m3.pair(bootstrapText: payload.urlString, host: "", port: "")
        XCTAssertFalse(browsed)
        XCTAssertNotNil(m3.linkError, "browse outcome is reported (no matching Mac, or Bonjour unavailable in this simulator)")
        XCTAssertFalse(m3.link.isConnected)
    }

    // MARK: 3. persistence

    func testPairingSurvivesInTheKeychainAndCapturesOnDisk() async throws {
        let keychain = KeychainPairStore(service: "tech.jay3332.flashtex.pad.tests.\(UUID().uuidString)")
        defer { try? keychain.removeAll() }
        let store = CaptureStore(directory: tmp)
        let model = await pairedModel(store: store, pairs: keychain)
        let png = CaptureQueueTests.trianglePNG()
        let sent = model.draft(CaptureRecord(source: .pencil, png: png, instructions: "persist me", pixelSize: (width: 600, height: 480)))
        let draft = model.draft(CaptureRecord(source: .sample, png: png, instructions: "still a draft"))
        let gone = model.draft(CaptureRecord(source: .photo, png: png, instructions: "discarded"))
        model.discard(gone.id)
        mac.setStatus(sent.id, state: "proposal_ready", latex: "\\beta")
        await model.send(sent.id)
        await waitUntil { model.captures.first { $0.id == sent.id }?.outcome?.latex == "\\beta" }

        // Disk: index + one PNG per capture. Independent of Keychain
        // availability, so this is asserted before the Keychain-dependent
        // part below might skip the rest of the test.
        XCTAssertTrue(FileManager.default.fileExists(atPath: tmp.appendingPathComponent("captures.json").path))
        XCTAssertEqual(try Data(contentsOf: store.pngURL(sent.id)), png)
        let index = String(decoding: try Data(contentsOf: tmp.appendingPathComponent("captures.json")), as: UTF8.self)
        XCTAssertTrue(index.contains("\"latex\" : \"\\\\beta\""), index)

        // Keychain: one generic-password item per Mac fingerprint, the
        // reference record shape. `MacLink.storePairing` swallows a Keychain
        // failure (it logs "pairing not stored: …" and lets the session stay
        // usable), so a missing item here needs its own diagnosis: retry the
        // exact same write ourselves so the thrown `KeychainError` names the
        // real `OSStatus`. The GitHub Actions runner's iOS Simulator has no
        // keychain-access-groups entitlement for the test bundle, which
        // fails every generic-password write with `errSecMissingEntitlement`
        // (-34018) — a runner-environment limitation, not a product bug — so
        // only that exact status is skipped; anything else still fails the
        // test here and everywhere else (including locally, where the
        // entitlement is present, this retry succeeds, and the skip must
        // never trigger).
        if keychain.pair(fingerprint: NearbyCrypto.fingerprint(salt: salt)) == nil {
            do {
                try keychain.upsert(try XCTUnwrap(model.pairedMac))
            } catch let error as KeychainPairStore.KeychainError where error.status == errSecMissingEntitlement {
                throw XCTSkip("Keychain unavailable on this runner: OSStatus \(error.status) (errSecMissingEntitlement) -- \(error)")
            }
        }
        let stored = try XCTUnwrap(keychain.pair(fingerprint: NearbyCrypto.fingerprint(salt: salt)),
                                   "the pairing was not stored: \(model.link.transcript.filter { $0.text.hasPrefix("pairing not stored") }.map(\.text))")
        XCTAssertEqual(stored.pairId, model.pairedMac?.pairId)
        XCTAssertEqual(stored.pairPsk, model.pairedMac?.pairPsk)
        XCTAssertEqual(KeychainPairStore(service: keychain.service).pairs.map(\.fingerprint), [stored.fingerprint], "read back by a fresh instance")

        // "Relaunch": a new model over the same stores, before any connection.
        model.disconnect()
        let again = PadModel(link: MacLink(store: KeychainPairStore(service: keychain.service)), captureStore: CaptureStore(directory: tmp))
        XCTAssertEqual(again.pairedMac?.pairId, stored.pairId, "pairing restored from the Keychain")
        XCTAssertEqual(again.captures.map(\.id), [gone.id, draft.id, sent.id], "order kept, newest first")
        let restored = try XCTUnwrap(again.captures.first { $0.id == sent.id })
        guard case .received(let ack) = restored.status else { return XCTFail("\(restored.status)") }
        XCTAssertEqual(ack.captureId, sent.id)
        XCTAssertEqual(restored.outcome?.state, "proposal_ready")
        XCTAssertEqual(restored.outcome?.latex, "\\beta")
        XCTAssertEqual(restored.instructions, "persist me")
        XCTAssertEqual(restored.destinationId, "dest-1"); XCTAssertEqual(restored.baseRevision, 4)
        XCTAssertEqual(restored.pixelSize?.width, 600)
        XCTAssertEqual(restored.png, png)
        XCTAssertEqual(again.captures.first { $0.id == draft.id }?.status, .drafted)
        XCTAssertEqual(again.captures.first { $0.id == gone.id }?.status, .discarded)
        // Reconnect with the restored key → the draft can still be sent; polling resumes for the received one.
        let pairId = try XCTUnwrap(again.pairedMac?.pairId)
        mac = try FakeMac.restart(mac, keys: [.init(identity: pairId, psk: mac.longTermPSK, bootstrap: false)])
        XCTAssertNotEqual(mac.port, 0, "restarted FakeMac is listening on a fresh port")
        again.pollInterval = 0.05
        await again.reconnect(host: "127.0.0.1", port: String(mac.port))
        XCTAssertNil(again.linkError)
        await again.send(draft.id)
        guard case .received? = again.captures.first { $0.id == draft.id }?.status else { return XCTFail("draft not sent after relaunch") }
        await waitUntil { Set(self.mac.captures.map(\.captureId)).count == 2 }
        XCTAssertEqual(Set(mac.captures.map(\.captureId)), [draft.id, sent.id], "the draft plus the re-delivered received capture (fresh FakeMac memory)")

        // An interrupted send comes back retryable with the same id. Its own
        // directory: `again` is still polling the draft's outcome and persists
        // through its own CaptureStore over `tmp` on every poll, and two
        // instances saving different lists into one directory prune each
        // other's PNGs (a real relaunch never has two stores over one index).
        let interruptedDir = tmp.appendingPathComponent("interrupted", isDirectory: true)
        var interrupted = CaptureRecord(id: "cap-interrupted", source: .pencil, png: png, instructions: "mid-flight")
        interrupted.status = .sending(attempt: 2)
        try CaptureStore(directory: interruptedDir).save([interrupted])
        let r = try XCTUnwrap(CaptureStore(directory: interruptedDir).load().first)
        guard case .disconnected(_, let attempt) = r.status else { return XCTFail("\(r.status)") }
        XCTAssertEqual(attempt, 2)
    }
}
