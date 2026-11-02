import AppKit
import CoreImage
import Vision
import XCTest
import NearbyClient
@testable import FlashTeXMac

/// Pairing follow-up 1 (mac-pairing-ui-3): the pairing code as a QR image
/// carrying exactly the bootstrap payload the reference client accepts, plus
/// the copyable text fallback. The QR is rendered by CoreImage, decoded by
/// Vision (both Apple frameworks, no third-party code) and handed to the real
/// reference client, which pairs with the real loopback listener from it.
@MainActor
final class PairingQRTests: XCTestCase {
    private var dir: URL!
    private var announced: [String] = []

    override func setUp() {
        super.setUp()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("pairing-qr-\(UUID().uuidString)")
        try! FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        announced = []
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: dir)
        super.tearDown()
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 15, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)", file: file, line: line)
    }

    /// Decodes the first QR symbol in `image` with Vision (what a companion's
    /// camera pipeline does); nil when none is found.
    static func decodeQR(_ image: CGImage) throws -> String? {
        let request = VNDetectBarcodesRequest()
        request.symbologies = [.qr]
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        try handler.perform([request])
        return request.results?.first?.payloadStringValue
    }

    // MARK: payload

    /// The Mac and the reference client encode byte-identical strings for the
    /// pinned salt 00…0f and a fixed 52-character code, and each parses the other's.
    func testPayloadStringIsExactlyWhatTheReferenceClientAccepts() throws {
        let salt = try XCTUnwrap(Pairing.data(hex: "000102030405060708090a0b0c0d0e0f"))
        let mac = PairingBootstrapPayload(code: "0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789ABCDEFGHJK", salt: salt, macName: "Jay's Mac Studio")
        let client = NearbyBootstrapPayload(code: "0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789ABCDEFGHJK", salt: salt, macName: "Jay's Mac Studio")
        XCTAssertEqual(mac.fingerprint, Pairing.vectorFingerprint)
        XCTAssertEqual(mac.urlString, client.urlString)
        XCTAssertEqual(mac.urlString,
                       "flashtex-nearby://pair?v=2&code=0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789ABCDEFGHJK&salt=000102030405060708090a0b0c0d0e0f&fp=0e712816d64b7c47&name=Jay's%20Mac%20Studio")
        XCTAssertEqual(try PairingBootstrapPayload.parse(client.urlString).get(), mac)
        let parsedByClient = try NearbyBootstrapPayload.parse(mac.urlString)
        XCTAssertEqual(parsedByClient.code, "0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789ABCDEFGHJK")
        XCTAssertEqual(parsedByClient.salt, salt)
        XCTAssertEqual(parsedByClient.fingerprint, Pairing.vectorFingerprint)
        XCTAssertEqual(parsedByClient.macName, "Jay's Mac Studio")
        // Strict inverse on the Mac side too.
        XCTAssertEqual(try? PairingBootstrapPayload.parse("flashtex-nearby://pair?v=2&code=0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789ABCDEFGHJK&salt=000102030405060708090a0b0c0d0e0f&fp=beefbeefbeefbeef").get(), nil)
        XCTAssertEqual(try? PairingBootstrapPayload.parse("https://x/pair?v=1").get(), nil)
    }

    /// Off-screen: CoreImage renders the payload, Vision reads back the same
    /// string; the module image is square and crisp at the requested side.
    func testQRImageRoundTripsThroughVision() throws {
        let salt = try XCTUnwrap(Pairing.data(hex: "000102030405060708090a0b0c0d0e0f"))
        let payload = PairingBootstrapPayload(code: Pairing.generateCode(), salt: salt, macName: "Studio")
        let image = try XCTUnwrap(PairingQR.image(for: payload, side: 240))
        XCTAssertEqual(image.width, image.height)
        XCTAssertGreaterThanOrEqual(image.width, 240)
        XCTAssertEqual(try Self.decodeQR(image), payload.urlString)
        XCTAssertNotNil(PairingQR.nsImage(for: payload, side: 120))
    }

    // MARK: real listener + reference client

    /// The window's QR (controller.bootstrapPayload → PairingQR) is decoded by
    /// Vision and passed verbatim to `nearby-client pair --qr`; the reference
    /// client browses by the payload's fp, derives the bootstrap key from its
    /// code + salt and completes the pairing against the real loopback
    /// listener. No --code/--mac/--salt on the command line.
    func testDecodedQRPairsTheReferenceClient() async throws {
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let model = ShellModel()
        let macName = "FlashTeX QR \(UUID().uuidString.prefix(6))"
        let state = NearbyState(store: store, macName: macName, loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        let c = PairingFlowController(nearby: state, journal: PairingJournal(url: dir.appendingPathComponent("pairing-session.json")),
                                      announcer: { [weak self] in self?.announced.append($0) })
        XCTAssertNil(c.bootstrapPayload, "no payload before a code is shown")
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        let payload = try XCTUnwrap(c.bootstrapPayload)
        XCTAssertEqual(payload.code, a.code)
        XCTAssertEqual(payload.salt, store.salt)
        XCTAssertEqual(payload.macName, macName)
        XCTAssertEqual(payload.fingerprint, state.fingerprint, "the QR names the Mac the way its TXT record does")
        // From .off, showCode() starts the listener once, directly with the
        // bootstrap key (a single "ready on port" line, unlike beginPairing()
        // on an already advertising state).
        try await waitUntil("bootstrap key served") { state.isAdvertising && state.port != nil && state.coordinator.current?.code == a.code }

        // What the companion's camera sees.
        let image = try XCTUnwrap(PairingQR.image(for: payload, side: 240))
        let scanned = try XCTUnwrap(try Self.decodeQR(image))
        XCTAssertEqual(scanned, payload.urlString)

        let clientStore = dir.appendingPathComponent("client-pairs.json").path
        var out: [String] = []
        let code = await NearbyCLI.run(["pair", "--qr", scanned, "--name", "QR iPad", "--seconds", "10", "--store", clientStore, "-v"]) { out.append($0) }
        XCTAssertEqual(code, 0, out.joined(separator: "\n"))
        XCTAssertEqual(out.last, "paired")
        XCTAssertTrue(out.contains("qr payload: fp \(state.fingerprint) name \"\(macName)\""), "\(out)")
        XCTAssertTrue(out.contains { $0.hasPrefix("found \(macName) (fp \(state.fingerprint), v=1)") }, "\(out)")
        try await waitUntil("paired on the Mac") { state.pairs.count == 1 && state.pairingCode == nil }
        XCTAssertEqual(state.pairs.first?.companionName, "QR iPad")
        XCTAssertEqual(state.pairs.first?.effectivePermission, .captures, "a new pairing may send captures")
        try await waitUntil("controller paired") { if case .paired = c.phase { return true } else { return false } }
        XCTAssertNil(c.bootstrapPayload, "no payload once paired")
        let stored = try XCTUnwrap(try PairFile(url: URL(fileURLWithPath: clientStore)).pair(fingerprint: state.fingerprint))
        XCTAssertEqual(stored.pairId, state.pairs.first?.pairId)
        XCTAssertEqual(stored.psk, state.pairs.first?.pskData, "the long-term key was handed over on the QR-bootstrapped session")

        // A disagreeing --code beside the same QR is refused before any connection.
        var refused: [String] = []
        let wrong = Pairing.generateCode()
        let rc = await NearbyCLI.run(["pair", "--qr", scanned, "--code", wrong, "--name", "x", "--store", clientStore]) { refused.append($0) }
        XCTAssertEqual(rc, 64)
        XCTAssertTrue(refused.first?.contains("differs from the QR payload's code") == true, "\(refused)")
        state.stopAdvertising()
    }

    // MARK: copyable fallback

    /// "Copy code" / ⌘C put the code on the pasteboard (a private named
    /// pasteboard here) and announce it without reading it out; nothing is copied
    /// when no code is shown.
    func testCopyCodeWritesDigitsToThePasteboardAndAnnouncesPairs() async throws {
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let model = ShellModel()
        let state = NearbyState(store: store, macName: "Copy Mac", loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        let c = PairingFlowController(nearby: state, journal: PairingJournal(url: dir.appendingPathComponent("pairing-session.json")),
                                      announcer: { [weak self] in self?.announced.append($0) })
        let pasteboard = NSPasteboard(name: .init("flashtex.test.\(UUID().uuidString)"))
        defer { pasteboard.releaseGlobally() }
        pasteboard.clearContents()
        pasteboard.setString("untouched", forType: .string)
        XCTAssertFalse(c.copyCode(to: pasteboard), "nothing to copy before a code is shown")
        XCTAssertEqual(pasteboard.string(forType: .string), "untouched")

        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertTrue(c.copyCode(to: pasteboard))
        XCTAssertEqual(pasteboard.string(forType: .string), a.code)
        XCTAssertEqual(pasteboard.string(forType: .string)?.count, Pairing.codeLength)
        XCTAssertEqual(Pairing.clipboardCode(Pairing.displayCode(a.code)), a.code, "the display spacing never reaches the pasteboard")
        XCTAssertEqual(announced.last, "Pairing code copied.", "52 characters are not read out in an announcement")
        let spoken = Pairing.spokenCode(a.code)
        XCTAssertEqual(spoken.components(separatedBy: ", ").count, 13, "52 characters spoken in groups of four: \(spoken)")
        XCTAssertEqual(spoken.filter { $0 != " " && $0 != "," }, a.code)

        c.cancel()
        try await waitUntil("code withdrawn") { state.pairingCode == nil }
        pasteboard.clearContents()
        XCTAssertFalse(c.copyCode(to: pasteboard), "a cancelled code is not copied")
        state.stopAdvertising()
    }
}
