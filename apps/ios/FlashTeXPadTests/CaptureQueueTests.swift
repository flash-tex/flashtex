import FlashTeXPadKit
import NearbyClient
import PencilKit
import XCTest
@testable import FlashTeXPad

/// Capture companion, hosted in the app in the simulator against an
/// in-process FakeMac: a PencilKit drawing rendered to PNG is sent as
/// transfer-v1 `capture_submit` and acknowledged; a discarded draft never
/// leaves the iPad; a retry after a dropped connection re-sends the same
/// capture_id; a refusal is terminal for that payload.
@MainActor
final class CaptureQueueTests: XCTestCase {
    var mac: FakeMac!
    var model: PadModel!
    let salt = Data((0..<16).map { UInt8($0 + 11) })
    let code = "205577"

    override func setUp() async throws {
        let d = NearbyCrypto.derive(code: code, salt: salt)
        mac = try FakeMac(keys: [.init(identity: d.pairId, psk: d.psk, bootstrap: true)], macName: "Fixture Mac",
                          destination: NearbyWire.Destination(destinationId: "dest-9", projectId: "demo", path: "main.tex", baseRevision: 7))
        mac.start()
        model = PadModel(link: MacLink(store: nil))
        await model.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                         fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Fixture Mac", code: code)
        XCTAssertNil(model.linkError)
    }

    override func tearDown() async throws { model.disconnect(); mac.stop() }

    static func trianglePNG() -> Data {
        var drawing = PKDrawing()
        let pts = [CGPoint(x: 40, y: 200), CGPoint(x: 260, y: 200), CGPoint(x: 150, y: 40), CGPoint(x: 40, y: 200)]
        var strokes: [PKStroke] = []
        for i in 0..<3 {
            let path = PKStrokePath(controlPoints: [pts[i], pts[i + 1]].map {
                PKStrokePoint(location: $0, timeOffset: 0, size: CGSize(width: 4, height: 4), opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
            }, creationDate: Date())
            strokes.append(PKStroke(ink: PKInk(.pen, color: .black), path: path))
        }
        drawing = PKDrawing(strokes: strokes)
        return drawing.image(from: CGRect(x: 0, y: 0, width: 300, height: 240), scale: 2).pngData()!
    }

    func testDrawingIsSentAsValidPNGWithInstruction() async throws {
        let png = Self.trianglePNG()
        XCTAssertNil(NearbyWire.checkImage(png, mimeType: "image/png"))
        let r = model.draft(CaptureRecord(source: .pencil, png: png, instructions: "convert this to TikZ"))
        XCTAssertEqual(model.captures.first?.status, .drafted)
        await model.send(r.id)
        guard case .received(let ack)? = model.captures.first?.status else { return XCTFail("\(String(describing: model.captures.first?.status))") }
        XCTAssertEqual(ack.captureId, r.id)
        XCTAssertFalse(ack.durable)
        XCTAssertEqual(mac.captures.count, 1)
        XCTAssertEqual(mac.captures[0].instructions, "convert this to TikZ")
        XCTAssertEqual(mac.captures[0].destinationId, "dest-9")
        XCTAssertEqual(mac.captures[0].baseRevision, 7)
        XCTAssertEqual(Data(base64Encoded: mac.captures[0].image.dataBase64), png, "bytes arrive unchanged")
        XCTAssertEqual(model.captures.first?.destinationId, "dest-9")
        for l in model.link.transcript { print("CAPTURE-TRANSCRIPT \(l.direction.rawValue) \(l.text)") }
    }

    func testDiscardedDraftNeverLeavesTheIPad() async throws {
        let r = model.draft(CaptureRecord(source: .sample, png: Self.trianglePNG(), instructions: "x"))
        model.discard(r.id)
        XCTAssertEqual(model.captures.first?.status, .discarded)
        await model.send(r.id) // no-op on a terminal record
        XCTAssertEqual(model.captures.first?.status, .discarded)
        XCTAssertTrue(mac.captures.isEmpty)
    }

    func testRetryAfterDisconnectReusesCaptureID() async throws {
        let r = model.draft(CaptureRecord(source: .pencil, png: Self.trianglePNG(), instructions: "retry me"))
        model.link.disconnect()
        await model.send(r.id)
        guard case .disconnected(_, let attempt)? = model.captures.first?.status else { return XCTFail("expected disconnected") }
        XCTAssertEqual(attempt, 1)
        XCTAssertTrue(mac.captures.isEmpty)
        // The Mac now holds the long-term pair_psk under the same pair_id (its key
        // table after pairing); reconnect with the stored key (proposal §7 step 3),
        // then retry: same capture_id.
        let pairId = try XCTUnwrap(model.pairedMac?.pairId)
        mac = try FakeMac.restart(mac, keys: [.init(identity: pairId, psk: mac.longTermPSK, bootstrap: false)])
        XCTAssertNotEqual(mac.port, 0, "restarted FakeMac is listening on a fresh port")
        await model.reconnect(host: "127.0.0.1", port: String(mac.port))
        XCTAssertNil(model.linkError)
        await model.send(r.id)
        guard case .received(let ack)? = model.captures.first?.status else { return XCTFail("expected received") }
        XCTAssertEqual(ack.captureId, r.id)
        XCTAssertEqual(mac.captures.map(\.captureId), [r.id])
        XCTAssertEqual(mac.hellos.count, 1, "one reconnect hello on the restarted listener")
        XCTAssertEqual(mac.hellos[0].pairId, pairId, "reconnect uses the pairing's id with the long-term key")
    }

    func testInvalidImageRefusedLocally() {
        XCTAssertNotNil(CaptureQueue.validate(png: Data([1, 2, 3]), instructions: "x"))
        XCTAssertNotNil(CaptureQueue.validate(png: Self.trianglePNG(), instructions: String(repeating: "a", count: 4097)))
        XCTAssertNil(CaptureQueue.validate(png: Self.trianglePNG(), instructions: "ok"))
    }

    func testNoDestinationIsRefusedNotSent() async throws {
        mac.destination = nil
        let r = model.draft(CaptureRecord(source: .pencil, png: Self.trianglePNG(), instructions: "x"))
        await model.send(r.id)
        guard case .refused(let code, _)? = model.captures.first?.status else { return XCTFail("expected refused") }
        XCTAssertEqual(code, "invalid_input")
        XCTAssertTrue(mac.captures.isEmpty)
    }

    /// GH #359: `CaptureQueue.records` was a bare Array. `refreshOutcome`/`send`
    /// resume off the main actor after `await` and mutate it while PadModel
    /// (and other pollers) copy the array. Without a lock this crashes the
    /// test host (EXC_BAD_ACCESS) or TSan-reports a data race.
    func testConcurrentRefreshSendAndRecordsRead() async throws {
        model.maxPolls = 0
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("CaptureQueueRace-\(UUID().uuidString)", isDirectory: true)
        defer { try? FileManager.default.removeItem(at: tmp) }
        let q = CaptureQueue(link: model.link, store: CaptureStore(directory: tmp))
        let png = Self.trianglePNG()
        let a = q.draft(CaptureRecord(source: .pencil, png: png, instructions: "race-a"))
        let b = q.draft(CaptureRecord(source: .sample, png: png, instructions: "race-b"))
        await q.send(a.id)
        await q.send(b.id)
        XCTAssertEqual(q.records.filter { if case .received = $0.status { return true }; return false }.count, 2)

        let id1 = a.id, id2 = b.id
        let rounds = 60
        await withTaskGroup(of: Void.self) { group in
            for _ in 0..<4 {
                group.addTask {
                    await Task.detached {
                        for _ in 0..<rounds {
                            await q.refreshOutcome(id1)
                            await q.refreshOutcome(id2)
                        }
                    }.value
                }
            }
            for _ in 0..<2 {
                group.addTask {
                    await Task.detached {
                        for _ in 0..<rounds {
                            await q.send(id1)
                            await q.send(id2)
                        }
                    }.value
                }
            }
            group.addTask {
                await Task.detached {
                    for _ in 0..<rounds {
                        let extra = q.draft(CaptureRecord(source: .fixture, png: png, instructions: "tmp"))
                        q.discard(extra.id)
                    }
                }.value
            }
            for _ in 0..<4 {
                group.addTask {
                    await Task.detached {
                        for _ in 0..<(rounds * 200) {
                            let snap = q.records
                            _ = snap.count
                            _ = q.record(id1)
                            _ = q.shouldPoll(id2)
                            _ = q.lastStoreError
                            _ = q.redeliveries
                        }
                    }.value
                }
            }
        }
        XCTAssertGreaterThanOrEqual(q.records.count, 2)
        XCTAssertNotNil(q.record(id1))
        XCTAssertNotNil(q.record(id2))
    }
}
