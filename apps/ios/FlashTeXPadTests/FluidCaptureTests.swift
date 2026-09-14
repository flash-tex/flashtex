import NearbyClient
import PencilKit
import XCTest
@testable import FlashTeXPad
@testable import FlashTeXPadKit

/// One-tap send and auto-reconnect (lane mac-capture-fluid) against the
/// loopback `FakeMac`: `sendNow` validates, drafts, sends and starts polling
/// in one call; a stored pairing reconnects at launch with the stored key
/// (remembered endpoint) without a code; the instruction chips remember what
/// was sent.
@MainActor
final class FluidCaptureTests: XCTestCase {
    var mac: FakeMac!
    var model: PadModel!
    let salt = Data((0..<16).map { UInt8($0 + 41) })
    let code = "482913"
    var store: PairFile!

    override func setUp() async throws {
        UserDefaults.standard.removeObject(forKey: PadModel.recentInstructionsKey)
        let d = NearbyCrypto.derive(code: code, salt: salt)
        mac = try FakeMac(keys: [.init(identity: d.pairId, psk: d.psk, bootstrap: true)], macName: "Fluid Mac",
                          destination: NearbyWire.Destination(destinationId: "mac-caret-1", projectId: "demo", path: "main.tex", baseRevision: 2))
        mac.start()
        store = try PairFile(url: FileManager.default.temporaryDirectory.appendingPathComponent("fluid-pairs-\(UUID()).json"))
        model = PadModel(link: MacLink(store: store))
        await model.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                         fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Fluid Mac", code: code)
        XCTAssertNil(model.linkError)
    }

    override func tearDown() async throws { model.disconnect(); mac.stop() }

    func testSendNowDraftsSendsAndRemembersTheInstruction() async throws {
        model.pollInterval = 0.05
        let png = CaptureQueueTests.trianglePNG()
        let r = await model.sendNow(png: png, source: .pencil, instructions: "  matrix  ", pixelSize: (600, 480))
        guard case .success(let id) = r else { return XCTFail("\(r)") }
        let rec = try XCTUnwrap(model.captures.first { $0.id == id })
        guard case .received(let ack) = rec.status else { return XCTFail("\(rec.status)") }
        XCTAssertEqual(ack.captureId, id)
        XCTAssertEqual(rec.instructions, "matrix", "trimmed once, sent as-is")
        XCTAssertEqual(rec.destinationId, "mac-caret-1")
        XCTAssertEqual(rec.baseRevision, 2)
        XCTAssertEqual(mac.captures.count, 1)
        XCTAssertEqual(mac.captures.first?.instructions, "matrix")
        XCTAssertEqual(model.recentInstructions.first, "matrix")
        XCTAssertEqual(model.recentInstructions.count, 4, "the three defaults follow the new chip")
        XCTAssertEqual(UserDefaults.standard.stringArray(forKey: PadModel.recentInstructionsKey)?.first, "matrix")
        // Polling started: the Mac gets capture_status without any tap.
        let deadline = Date().addingTimeInterval(3)
        while mac.statusRequests.isEmpty, Date() < deadline { try await Task.sleep(nanoseconds: 30_000_000) }
        XCTAssertTrue(mac.statusRequests.contains(id))
        mac.setStatus(id, state: "inserted", latex: "\\alpha", newRevision: 3)
        let final = Date().addingTimeInterval(3)
        while model.captures.first?.outcome?.state != "inserted", Date() < final { try await Task.sleep(nanoseconds: 30_000_000) }
        XCTAssertEqual(model.captures.first?.outcome?.state, "inserted")
        XCTAssertTrue(model.captures.first?.outcomeIsFinal ?? false)
    }

    func testSendNowRefusesWithoutSendingWhenInvalidOrDisconnected() async {
        let bad = await model.sendNow(png: Data([1, 2, 3]), source: .pencil, instructions: "x")
        guard case .failure(.invalid) = bad else { return XCTFail("\(bad)") }
        XCTAssertTrue(model.captures.isEmpty, "nothing drafted")
        let long = String(repeating: "a", count: CaptureQueue.maxInstructionBytes + 1)
        let tooLong = await model.sendNow(png: CaptureQueueTests.trianglePNG(), source: .pencil, instructions: long)
        guard case .failure(.invalid(let why)) = tooLong, why.contains("4096") else { return XCTFail("\(tooLong)") }
        model.disconnect()
        let off = await model.sendNow(png: CaptureQueueTests.trianglePNG(), source: .pencil, instructions: "x")
        guard case .failure(.notConnected) = off else { return XCTFail("\(off)") }
        XCTAssertTrue(model.captures.isEmpty)
        XCTAssertEqual(mac.captures.count, 0)
    }

    func testRecentInstructionsMergeIsBoundedAndCaseInsensitive() {
        let m = PadModel.merged(recent: ["Matrix", " matrix ", "", "Convert to TikZ"], defaults: PadModel.defaultInstructions)
        XCTAssertEqual(m, ["Matrix", "Convert to TikZ", "Transcribe as LaTeX", "This is a matrix"])
        XCTAssertEqual(PadModel.merged(recent: (1...9).map { "i\($0)" }, defaults: []).count, PadModel.maxRecentInstructions)
    }

    func testAutoReconnectUsesTheStoredKeyAndRememberedEndpointWithoutACode() async throws {
        let pair = try XCTUnwrap(model.pairedMac)
        XCTAssertEqual(PadModel.lastEndpoint(fingerprint: pair.fingerprint)?.port, mac.port, "the pairing remembered where the Mac was")
        model.disconnect()
        // The Mac after pairing: its key table holds the long-term pair_psk under the same pair_id.
        mac = try FakeMac.restart(mac, keys: [.init(identity: pair.pairId, psk: mac.longTermPSK, bootstrap: false)])
        // A fresh model, as after relaunch: the pairing comes back from the store, nothing is connected.
        let again = PadModel(link: MacLink(store: store))
        XCTAssertEqual(again.pairedMac?.pairId, pair.pairId)
        XCTAssertFalse(again.link.isConnected)
        let ok = await again.autoReconnect(endpoint: ("127.0.0.1", mac.port))
        XCTAssertTrue(ok, again.linkError ?? again.linkStatus)
        XCTAssertTrue(again.link.isConnected)
        XCTAssertEqual(again.linkStatus, "connected to Fluid Mac")
        XCTAssertEqual(again.destination?.destinationId, "mac-caret-1")
        XCTAssertEqual(PadModel.lastEndpoint(fingerprint: pair.fingerprint)?.port, mac.port, "the new address is remembered")
        XCTAssertEqual(mac.hellos.count, 1, "hello with the stored key, no bootstrap")
        // A second call is a no-op while connected.
        let noop = await again.autoReconnect(endpoint: ("127.0.0.1", mac.port))
        XCTAssertTrue(noop)
        XCTAssertEqual(mac.hellos.count, 1)
        again.disconnect()
        // A Mac that is gone: the pairing stays, the status says so, no crash.
        mac.stop()
        let gone = await again.autoReconnect(endpoint: ("127.0.0.1", mac.port))
        XCTAssertFalse(gone)
        XCTAssertEqual(again.pairedMac?.pairId, pair.pairId)
        XCTAssertTrue(again.linkStatus.contains("not reachable"), again.linkStatus)
    }

    func testAutoReconnectWithoutAPairingDoesNothing() async {
        let fresh = PadModel(link: MacLink(store: nil))
        let r = await fresh.autoReconnect(endpoint: ("127.0.0.1", mac.port))
        XCTAssertFalse(r)
        XCTAssertEqual(fresh.linkStatus, "not paired")
    }
}

/// The caret context the Mac announces with its destination (nearby-v1,
/// additive). The iPad only displays it — the Mac derives it and the bridge
/// enforces it — but it must survive the wire, and a Mac that predates the
/// field must still pair. See protocol/proposals/transfer-v1-caret-context.md.
@MainActor
final class DestinationCaretContextTests: XCTestCase {
    let salt = Data((0..<16).map { UInt8($0 + 7) })
    let code = "731559"
    var mac: FakeMac!
    var model: PadModel!

    private func connect(_ destination: NearbyWire.Destination) async throws {
        let d = NearbyCrypto.derive(code: code, salt: salt)
        mac = try FakeMac(keys: [.init(identity: d.pairId, psk: d.psk, bootstrap: true)],
                          macName: "Caret Mac", destination: destination)
        mac.start()
        let store = try PairFile(url: FileManager.default.temporaryDirectory
            .appendingPathComponent("caret-pairs-\(UUID()).json"))
        model = PadModel(link: MacLink(store: store))
        await model.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                         fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Caret Mac", code: code)
        XCTAssertNil(model.linkError)
    }

    override func tearDown() async throws { model?.disconnect(); mac?.stop() }

    func testCaretContextArrivesWithTheDestinationAndLabelsItself() async throws {
        try await connect(NearbyWire.Destination(
            destinationId: "mac-caret-1", projectId: "demo", path: "main.tex", baseRevision: 2,
            caretContext: NearbyWire.CaretContext(mode: "inline_math", delimiter: "$", wrap: "already_math")))
        let caret = try XCTUnwrap(model.destination?.caretContext)
        XCTAssertEqual(caret.mode, "inline_math")
        XCTAssertEqual(caret.wrap, "already_math")
        XCTAssertEqual(caret.delimiter, "$")
        XCTAssertTrue(caret.label.contains("no delimiters added"), caret.label)
    }

    func testAMacWithoutACaretContextStillPairs() async throws {
        try await connect(NearbyWire.Destination(
            destinationId: "mac-caret-2", projectId: "demo", path: "main.tex", baseRevision: 2))
        XCTAssertEqual(model.destination?.destinationId, "mac-caret-2")
        XCTAssertNil(model.destination?.caretContext)
    }
}
