import Network
import XCTest
@testable import NearbyClient

final class NearbyCryptoTests: XCTestCase {
    /// Pinned vector from apps/mac/docs/nearby-v1-proposal.md §2 (also in the
    /// Mac's PairingTests) — the companion must derive exactly this.
    func testDerivationMatchesProposalVector() throws {
        let salt = try XCTUnwrap(NearbyCrypto.data(hex: "000102030405060708090a0b0c0d0e0f"))
        let d = NearbyCrypto.derive(code: "123456", salt: salt)
        XCTAssertEqual(d.pairId, "3917d7c3e5eef7ce")
        XCTAssertEqual(NearbyCrypto.hex(d.psk), "b127a48a782dbece3edbed18d027d658890624ecf21da77d19f47d5e604d1221")
        XCTAssertEqual(NearbyCrypto.fingerprint(salt: salt), "0e712816d64b7c47")
        XCTAssertEqual(NearbyCrypto.helloProof(psk: d.psk, nonce: "n-1"), "rIBrMvRrNjs2eQueTGVsBNF6K130BW6fqV93Rw//WgM=")
        XCTAssertTrue(NearbyCrypto.verifyHelloProof("rIBrMvRrNjs2eQueTGVsBNF6K130BW6fqV93Rw//WgM=", psk: d.psk, nonce: "n-1"))
        XCTAssertFalse(NearbyCrypto.verifyHelloProof("rIBrMvRrNjs2eQueTGVsBNF6K130BW6fqV93Rw//WgM=", psk: d.psk, nonce: "n-2"))
        XCTAssertNotEqual(NearbyCrypto.derive(code: "123457", salt: salt).pairId, d.pairId)
    }

    func testHexRoundTrip() {
        XCTAssertEqual(NearbyCrypto.data(hex: "00ff10"), Data([0x00, 0xFF, 0x10]))
        XCTAssertNil(NearbyCrypto.data(hex: "0"))
        XCTAssertNil(NearbyCrypto.data(hex: "zz"))
        XCTAssertEqual(NearbyCrypto.hex(Data([0xAB, 0x01])), "ab01")
    }

    func testClientParametersPinTLS12AndSuite() {
        let params = NearbyCrypto.parameters(pairId: "p", psk: Data(repeating: 1, count: 32))
        XCTAssertTrue(params.defaultProtocolStack.applicationProtocols.contains { $0 is NWProtocolTLS.Options })
        XCTAssertTrue(params.allowLocalEndpointReuse)
        XCTAssertFalse(params.includePeerToPeer)
    }
}

final class NearbyWireTests: XCTestCase {
    func testHelloLineHasEveryFieldTheMacRequires() throws {
        let h = NearbyWire.Hello(pairId: "3917d7c3e5eef7ce", companionName: "iPad", nonce: "N", proof: "P")
        let line = try NearbyWire.line(id: "h1", type: "hello", h)
        XCTAssertEqual(line.last, 0x0A)
        let obj = try XCTUnwrap(JSONSerialization.jsonObject(with: line) as? [String: Any])
        XCTAssertEqual(obj["protocol_version"] as? Int, 1)
        XCTAssertEqual(obj["id"] as? String, "h1")
        XCTAssertEqual(obj["type"] as? String, "hello")
        let p = try XCTUnwrap(obj["payload"] as? [String: Any])
        XCTAssertEqual(p["role"] as? String, "companion")
        XCTAssertEqual(p["pair_id"] as? String, "3917d7c3e5eef7ce")
        XCTAssertEqual(p["companion_name"] as? String, "iPad")
        XCTAssertEqual(p["protocol_version"] as? Int, 1)
        XCTAssertEqual(p["nonce"] as? String, "N")
        XCTAssertEqual(p["proof"] as? String, "P")
    }

    func testCaptureSubmitMatchesFixtureShape() throws {
        // Same field names as protocol/fixtures/capture-submission.json.
        let c = NearbyWire.CaptureSubmit(captureId: "fixture-capture-1", destinationId: "fixture-anchor-1", baseRevision: 1,
                                         image: .init(mimeType: "image/png", dataBase64: "AAAA"), instructions: "x")
        let s = String(decoding: try NearbyWire.line(id: "r", type: "capture_submit", c), as: UTF8.self)
        for key in ["\"capture_id\":\"fixture-capture-1\"", "\"destination_id\":\"fixture-anchor-1\"", "\"base_revision\":1",
                    "\"mime_type\":\"image/png\"", "\"data_base64\":\"AAAA\"", "\"instructions\":\"x\""] {
            XCTAssertTrue(s.contains(key), "missing \(key) in \(s)")
        }
    }

    func testReplyDecoding() throws {
        let ack = try NearbyWire.decode(Data("""
        {"protocol_version":1,"id":"h1","type":"hello_ack","payload":{"mac_name":"M","nonce":"N","destination":null,"pair_psk":"AA=="}}
        """.utf8), as: NearbyWire.HelloAck.self)
        XCTAssertEqual(ack.payload, .init(macName: "M", nonce: "N", destination: nil, pairPsk: "AA=="))
        let dest = try NearbyWire.decode(Data("""
        {"protocol_version":1,"id":"h1","type":"hello_ack","payload":{"mac_name":"M","nonce":"N","destination":{"destination_id":"d","project_id":"p","path":"main.tex","base_revision":7}}}
        """.utf8), as: NearbyWire.HelloAck.self)
        XCTAssertEqual(dest.payload.destination, .init(destinationId: "d", projectId: "p", path: "main.tex", baseRevision: 7))
        XCTAssertNil(dest.payload.pairPsk)
        let err = try JSONDecoder().decode(NearbyWire.ErrorLine.self, from: Data("""
        {"protocol_version":1,"id":null,"type":"error","payload":{"code":"line_too_long","message":"m"}}
        """.utf8))
        XCTAssertNil(err.id)
        XCTAssertEqual(err.payload.code, "line_too_long")
        XCTAssertTrue(NearbyError.remote(code: "pair_mismatch", message: "").isClosing)
        XCTAssertFalse(NearbyError.remote(code: "unknown_type", message: "").isClosing)
    }

    func testIDRuleAndLineSplitter() {
        XCTAssertTrue(NearbyWire.isValidID("cap-1_A"))
        XCTAssertFalse(NearbyWire.isValidID(""))
        XCTAssertFalse(NearbyWire.isValidID("a b"))
        XCTAssertFalse(NearbyWire.isValidID(String(repeating: "a", count: 129)))
        var s = LineSplitter()
        XCTAssertEqual(s.append(Data("a\nbb".utf8)), [Data("a".utf8)])
        XCTAssertEqual(s.pendingBytes, 2)
        XCTAssertEqual(s.append(Data("c\n\n".utf8)), [Data("bbc".utf8), Data()])
    }

    func testMimeSniffing() {
        XCTAssertEqual(NearbyCLI.sniffMime(Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0])), "image/png")
        XCTAssertEqual(NearbyCLI.sniffMime(Data([0xFF, 0xD8, 0xFF, 0xE0])), "image/jpeg")
        XCTAssertNil(NearbyCLI.sniffMime(Data("GIF89a".utf8)))
    }
}

final class DiscoveredMacTests: XCTestCase {
    func testTXTValidation() {
        let salt = NearbyCrypto.data(hex: "000102030405060708090a0b0c0d0e0f")!
        let ep = NWEndpoint.service(name: "M", type: "_flashtex._tcp", domain: "local.", interface: nil)
        let good = DiscoveredMac(name: "M", endpoint: ep, txt: ["v": "1", "name": "My Mac", "fp": "0e712816d64b7c47", "salt": NearbyCrypto.hex(salt)])
        XCTAssertTrue(good.isSupported)
        XCTAssertEqual(good.macName, "My Mac")
        XCTAssertEqual(DiscoveredMac(name: "M", endpoint: ep, txt: ["v": "2"]).unsupportedReason, "TXT v=2 (want 1)")
        XCTAssertEqual(DiscoveredMac(name: "M", endpoint: ep, txt: ["v": "1", "salt": "00"]).unsupportedReason, "TXT salt missing or not 16 bytes")
        XCTAssertEqual(DiscoveredMac(name: "M", endpoint: ep, txt: ["v": "1", "salt": NearbyCrypto.hex(salt), "fp": "beef"]).unsupportedReason,
                       "TXT fp does not match SHA-256(mac-id ‖ salt)")
    }
}

final class NearbyBootstrapPayloadTests: XCTestCase {
    /// Pinned vector (salt 00…0f, code 123456): the Mac's PairingQRTests
    /// encode exactly this string into its QR image.
    func testPayloadRoundTripAndPinnedString() throws {
        let salt = try XCTUnwrap(NearbyCrypto.data(hex: "000102030405060708090a0b0c0d0e0f"))
        let p = NearbyBootstrapPayload(code: "123456", salt: salt, macName: "Jay's Mac Studio")
        XCTAssertEqual(p.fingerprint, "0e712816d64b7c47")
        XCTAssertEqual(p.urlString,
                       "flashtex-nearby://pair?v=1&code=123456&salt=000102030405060708090a0b0c0d0e0f&fp=0e712816d64b7c47&name=Jay's%20Mac%20Studio")
        XCTAssertEqual(try NearbyBootstrapPayload.parse(p.urlString), p)
        XCTAssertEqual(try NearbyBootstrapPayload.parse(" \(p.urlString)\n"), p, "scanner whitespace is tolerated")
    }

    func testPayloadRefusals() throws {
        let salt = "000102030405060708090a0b0c0d0e0f"
        func reason(_ s: String) -> String {
            do { _ = try NearbyBootstrapPayload.parse(s); return "accepted" } catch let e as NearbyError { return e.description } catch { return "\(error)" }
        }
        XCTAssertEqual(reason("https://example.com/pair?v=1"), "invalid input: not a flashtex-nearby://pair payload")
        XCTAssertEqual(reason("flashtex-nearby://pair?v=2&code=123456&salt=\(salt)&fp=0e712816d64b7c47"), "invalid input: payload version 2 is not 1")
        XCTAssertEqual(reason("flashtex-nearby://pair?v=1&code=12345&salt=\(salt)&fp=0e712816d64b7c47"), "invalid input: payload code must be 6 digits")
        XCTAssertEqual(reason("flashtex-nearby://pair?v=1&code=123456&salt=0001&fp=0e712816d64b7c47"), "invalid input: payload salt must be 16 hex bytes")
        XCTAssertEqual(reason("flashtex-nearby://pair?v=1&code=123456&salt=\(salt)&fp=deadbeefdeadbeef"), "invalid input: payload fp does not match its salt")
        XCTAssertEqual(reason("flashtex-nearby://pair?v=1&code=123456&salt=\(salt)&fp=0e712816d64b7c47"), "accepted", "name is optional")
    }

    /// `capture_not_permitted` is its own class: pairing intact (no re-pair),
    /// same capture later (no new capture), not backpressure, not retried.
    func testCaptureNotPermittedIsNeitherRepairNorNewCapture() {
        let e = NearbyError.remote(code: "capture_not_permitted", message: "view-only")
        XCTAssertTrue(e.needsPermission)
        XCTAssertFalse(e.needsRepair)
        XCTAssertFalse(e.needsNewCapture)
        XCTAssertFalse(e.needsNewDestination)
        XCTAssertFalse(e.isRetryable)
        XCTAssertFalse(e.isBackpressure)
        XCTAssertFalse(e.isClosing)
        XCTAssertFalse(NearbyError.remote(code: "pair_mismatch", message: "").needsPermission)
        var lines: [String] = []
        XCTAssertEqual(NearbyCLI.exitCode(for: e) { lines.append($0) }, 6)
        XCTAssertTrue(lines[0].contains("view-only") && lines[0].contains("no re-pair"), "\(lines)")
    }
}

final class PairFileTests: XCTestCase {
    func testRoundTripAndPermissions() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-client-\(UUID().uuidString)")
        let url = dir.appendingPathComponent("pairs.json")
        let f = try PairFile(url: url)
        XCTAssertTrue(f.pairs.isEmpty)
        let p = PairedMac(fingerprint: "fp1", macName: "Mac", pairId: "pid", pairPsk: Data(repeating: 7, count: 32).base64EncodedString(),
                          companionName: "iPad", lastDestination: .init(destinationId: "d", projectId: "p", path: "a.tex", baseRevision: 2))
        try f.upsert(p)
        let mode = try FileManager.default.attributesOfItem(atPath: url.path)[.posixPermissions] as? Int
        XCTAssertEqual(mode, 0o600)
        let again = try PairFile(url: url)
        XCTAssertEqual(again.pairs.count, 1)
        XCTAssertEqual(again.pair(matching: "mac")?.pairId, "pid")
        XCTAssertEqual(again.pair(matching: "fp1")?.lastDestination?.baseRevision, 2)
        XCTAssertEqual(again.pairs[0].psk?.count, 32)
        XCTAssertTrue(try again.remove(fingerprint: "fp1"))
        XCTAssertTrue(try PairFile(url: url).pairs.isEmpty)
        let json = String(decoding: try Data(contentsOf: url), as: UTF8.self)
        XCTAssertTrue(json.contains("\"version\" : 1"))
        try? FileManager.default.removeItem(at: dir)
    }

    /// Hammers one `PairFile` from many concurrent tasks (upserts, removes,
    /// and reads interleaved) and asserts the final state is internally
    /// consistent: no duplicate fingerprints, every surviving pair readable
    /// back, and the file on disk agreeing with memory. Completing without a
    /// crash or torn array is the lock's whole point (cf. #372).
    func testConcurrentUpsertRemoveKeepsStoreConsistent() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-client-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appendingPathComponent("pairs.json")
        let f = try PairFile(url: url)
        let fingerprints = (0..<50).map { "fp-\($0)" }
        await withTaskGroup(of: Void.self) { group in
            for fp in fingerprints {
                group.addTask {
                    let p = PairedMac(fingerprint: fp, macName: "Mac \(fp)", pairId: "pid-\(fp)",
                                      pairPsk: Data(repeating: 7, count: 32).base64EncodedString(),
                                      companionName: "iPad")
                    _ = try? f.upsert(p)
                }
                group.addTask { _ = f.pair(fingerprint: fp) }
            }
            for fp in fingerprints.prefix(25) {
                group.addTask { _ = try? f.remove(fingerprint: fp) }
            }
        }
        let final = f.pairs
        let finalFPs = final.map(\.fingerprint)
        XCTAssertEqual(Set(finalFPs).count, finalFPs.count, "duplicate fingerprints after concurrent mutation")
        XCTAssertLessThanOrEqual(final.count, fingerprints.count)
        for p in final {
            XCTAssertEqual(f.pair(fingerprint: p.fingerprint)?.pairId, p.pairId)
            XCTAssertEqual(f.pair(matching: p.fingerprint)?.pairId, p.pairId)
        }
        let reloaded = try PairFile(url: url)
        XCTAssertEqual(Set(reloaded.pairs.map(\.fingerprint)), Set(finalFPs), "file on disk must match memory")
    }
}

/// Full client flow against the in-test fake Mac: bootstrap pairing, PSK
/// hand-over, reconnect with the long-term key, capture, refusals.
final class NearbyClientFlowTests: XCTestCase {
    let salt = NearbyCrypto.data(hex: "0f0e0d0c0b0a09080706050403020100")!

    func testPairSendReconnectViaLibrary() async throws {
        let boot = NearbyCrypto.derive(code: "424242", salt: salt)
        let mac = try FakeMac(keys: [.init(identity: boot.pairId, psk: boot.psk, bootstrap: true)],
                              destination: .init(destinationId: "anchor-1", projectId: "demo", path: "main.tex", baseRevision: 5))
        mac.start()
        defer { mac.stop() }
        XCTAssertGreaterThan(mac.port, 0)
        let ep = NWEndpoint.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: mac.port)!)

        var transcript: [String] = []
        let (pair, session) = try await NearbyClient.pair(endpoint: ep, salt: salt, fingerprint: NearbyCrypto.fingerprint(salt: salt),
                                                          macName: "x", code: "424242", companionName: "Test iPad") { dir, line in
            transcript.append((dir == .sent ? ">> " : "<< ") + String(decoding: line, as: UTF8.self))
        }
        XCTAssertEqual(pair.pairId, boot.pairId)
        XCTAssertEqual(pair.psk, mac.longTermPSK)
        XCTAssertEqual(pair.macName, "Fake Mac")
        XCTAssertEqual(session.destination?.destinationId, "anchor-1")
        XCTAssertEqual(session.connection.negotiated?.tlsv12, true)
        XCTAssertEqual(session.connection.negotiated?.suite, 0x00A8)
        XCTAssertEqual(mac.hellos.count, 1)
        XCTAssertEqual(mac.hellos[0].companionName, "Test iPad")
        XCTAssertEqual(mac.hellos[0].protocolVersion, 1)
        XCTAssertTrue(transcript[0].hasPrefix(">> {\"id\":\"") && transcript[0].contains("\"type\":\"hello\""), "\(transcript)")
        XCTAssertTrue(transcript[1].contains("\"pair_psk\""))

        // The bootstrap connection stays usable.
        let cap = try session.makeCapture(captureId: "cap-1", image: TestImages.png1x1, mimeType: "image/png", instructions: "hi")
        XCTAssertEqual(cap.destinationId, "anchor-1")
        XCTAssertEqual(cap.baseRevision, 5)
        let ack = try await session.submitCapture(cap)
        XCTAssertEqual(ack, .init(captureId: "cap-1", durable: false, hasProposal: false, applied: false))
        XCTAssertEqual(mac.captures.first?.image.dataBase64, TestImages.png1x1.base64EncodedString())
        session.close()

        // Reconnect with the long-term key only (what the Mac keeps after pairing).
        let mac2 = try FakeMac(keys: [.init(identity: pair.pairId, psk: mac.longTermPSK, bootstrap: false)])
        mac2.start()
        defer { mac2.stop() }
        let ep2 = NWEndpoint.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: mac2.port)!)
        let s2 = try await NearbyClient.connect(endpoint: ep2, pair: pair)
        XCTAssertNil(s2.ack.pairPsk)
        XCTAssertNil(s2.destination)
        let d = try await s2.destinationQuery()
        XCTAssertNil(d)
        XCTAssertThrowsError(try s2.makeCapture(image: Data(), mimeType: "image/png", instructions: "")) // no destination
        // Unknown type → error reply, session stays open.
        do {
            let _: NearbyWire.Envelope<NearbyWire.Empty> = try await s2.connection.request(type: "bogus", NearbyWire.Empty(), expecting: "never")
            XCTFail("expected an error reply")
        } catch let e as NearbyError {
            XCTAssertEqual(e, .remote(code: "unknown_type", message: "unknown message type bogus"))
            XCTAssertFalse(e.isClosing)
        }
        let again = try await s2.destinationQuery()
        XCTAssertNil(again)
        s2.close()

        // The bootstrap key is gone: handshake refused before any line.
        let stale = NearbyConnection(endpoint: ep2, pairId: boot.pairId, psk: boot.psk)
        do { try await stale.connect(timeout: 5); XCTFail("bootstrap key must be refused") } catch let e as NearbyError {
            if case .handshakeFailed = e {} else { XCTFail("unexpected \(e)") }
        }
        // Wrong proof (right key, claims another pair_id) → pair_mismatch and close.
        let liar = NearbyConnection(endpoint: ep2, pairId: pair.pairId, psk: mac.longTermPSK)
        try await liar.connect()
        let lie = NearbyWire.Hello(pairId: "0000000000000000", companionName: "x", nonce: "n", proof: NearbyCrypto.helloProof(psk: mac.longTermPSK, nonce: "n"))
        do {
            let _: NearbyWire.Envelope<NearbyWire.HelloAck> = try await liar.request(type: "hello", lie, expecting: "hello_ack")
            XCTFail("expected pair_mismatch")
        } catch let e as NearbyError {
            XCTAssertEqual(e, .remote(code: "pair_mismatch", message: "unknown pair or bad proof"))
            XCTAssertTrue(e.isClosing)
        }
        liar.close()
    }

    func testCLICommandsAgainstFakeMac() async throws {
        let boot = NearbyCrypto.derive(code: "777777", salt: salt)
        let mac = try FakeMac(keys: [.init(identity: boot.pairId, psk: boot.psk, bootstrap: true)],
                              destination: .init(destinationId: "anchor-2", projectId: "demo", path: "notes.tex", baseRevision: 9))
        mac.start()
        defer { mac.stop() }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-cli-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let store = dir.appendingPathComponent("pairs.json").path
        let png = dir.appendingPathComponent("dot.png")
        try Data(base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4//8/AAX+Av4N70a4AAAAAElFTkSuQmCC")!.write(to: png)

        var out: [String] = []
        let code = await NearbyCLI.run(["pair", "--code", "777777", "--name", "CLI iPad", "--host", "127.0.0.1", "--port", "\(mac.port)",
                                        "--salt", NearbyCrypto.hex(salt), "--store", store, "-v"]) { out.append($0) }
        XCTAssertEqual(code, 0, out.joined(separator: "\n"))
        XCTAssertEqual(out.last, "paired")
        XCTAssertTrue(out.contains { $0.hasPrefix(">> {") && $0.contains("\"type\":\"hello\"") })
        XCTAssertTrue(out.contains { $0.contains("received long-term pair_psk (32 bytes)") })
        XCTAssertTrue(out.contains { $0.contains("anchor-2 (demo/notes.tex @ rev 9)") })

        // Mac now only holds the long-term key; the CLI reconnects with it and sends.
        let mac2 = try FakeMac(keys: [.init(identity: boot.pairId, psk: mac.longTermPSK, bootstrap: false)],
                               destination: .init(destinationId: "anchor-2", projectId: "demo", path: "notes.tex", baseRevision: 10))
        mac2.start()
        defer { mac2.stop() }
        out = []
        let sent = await NearbyCLI.run(["send", "--image", png.path, "--instructions", "transcribe", "--capture-id", "cli-cap-1",
                                        "--host", "127.0.0.1", "--port", "\(mac2.port)", "--store", store, "-v"]) { out.append($0) }
        XCTAssertEqual(sent, 0, out.joined(separator: "\n"))
        XCTAssertEqual(out.last, "sent")
        XCTAssertTrue(out.contains("capture_received cli-cap-1: durable=false has_proposal=false applied=false"), "\(out)")
        XCTAssertTrue(out.contains { $0.hasPrefix(">> ") && $0.contains("\"data_base64\":\"<") }, "verbose output abbreviates the image")
        XCTAssertEqual(mac2.captures.first?.captureId, "cli-cap-1")
        XCTAssertEqual(mac2.captures.first?.baseRevision, 10, "uses the destination from this connection's hello_ack")
        XCTAssertEqual(mac2.captures.first?.instructions, "transcribe")

        out = []
        let statusCode = await NearbyCLI.run(["status", "--seconds", "0.2", "--store", store]) { out.append($0) }
        XCTAssertEqual(statusCode, 0)
        XCTAssertTrue(out.contains { $0.contains("Fake Mac fp=\(NearbyCrypto.fingerprint(salt: salt)) pair_id=\(boot.pairId) as \"CLI iPad\"") }, "\(out)")

        out = []
        let forgetCode = await NearbyCLI.run(["forget", "--mac", "Fake Mac", "--store", store]) { out.append($0) }
        XCTAssertEqual(forgetCode, 0)
        out = []
        let unpaired = await NearbyCLI.run(["send", "--image", png.path, "--store", store]) { out.append($0) }
        XCTAssertEqual(unpaired, 1)
        XCTAssertTrue(out[0].contains("no pairings stored"))
        let unknown = await NearbyCLI.run(["nope"]) { _ in }
        XCTAssertEqual(unknown, 64)
        let missing = await NearbyCLI.run(["pair", "--name", "x"]) { _ in }
        XCTAssertEqual(missing, 64)
        try? FileManager.default.removeItem(at: dir)
    }

    func testBonjourDiscoveryFindsAdvertisedFakeMac() async throws {
        let name = "nearby-client test \(UUID().uuidString.prefix(6))"
        let boot = NearbyCrypto.derive(code: "111111", salt: salt)
        let mac = try FakeMac(keys: [.init(identity: boot.pairId, psk: boot.psk, bootstrap: true)], advertise: name, salt: salt)
        mac.start()
        defer { mac.stop() }
        let fp = NearbyCrypto.fingerprint(salt: salt)
        let results = try await NearbyBrowser.discover(seconds: 8) { $0.fingerprint == fp }
        let found = try XCTUnwrap(results.first { $0.fingerprint == fp }, "advertised service not found: \(results)")
        XCTAssertEqual(found.name, name)
        XCTAssertEqual(found.macName, "Fake Mac")
        XCTAssertTrue(found.isSupported)
        XCTAssertEqual(found.salt, salt)
        // And connect through the Bonjour endpoint itself (resolved to host:port first).
        let (pair, session) = try await NearbyClient.pair(mac: found, code: "111111", companionName: "Browsing iPad")
        XCTAssertEqual(pair.fingerprint, fp)
        XCTAssertEqual(session.macName, "Fake Mac")
        if case .hostPort(_, let port) = try XCTUnwrap(session.connection.dialed) { XCTAssertEqual(port.rawValue, mac.port) } else { XCTFail() }
        session.close()

        // A refused key through the Bonjour endpoint must be reported promptly.
        // Dialing the `.service` endpoint directly never leaves `.preparing`
        // (measured >60 s); resolving first surfaces `.waiting(-9820: bad MAC)`.
        let wrong = NearbyConnection(endpoint: found.endpoint, pairId: boot.pairId, psk: Data(repeating: 9, count: 32))
        let started = Date()
        do { try await wrong.connect(timeout: 20); XCTFail("wrong key must be refused") } catch let e as NearbyError {
            guard case .handshakeFailed(let why) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(why.contains("-9820"), why)
        }
        XCTAssertLessThan(Date().timeIntervalSince(started), 5)
        let resolved = try await NearbyResolver.resolve(found.endpoint)
        XCTAssertEqual(resolved.port, mac.port)
        XCTAssertFalse(resolved.host.isEmpty)
    }
}
