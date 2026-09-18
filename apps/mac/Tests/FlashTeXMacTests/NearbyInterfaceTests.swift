import Network
import XCTest
import FlashTeXProtocol
import NearbyClient
@testable import FlashTeXMac

/// IPv6, link-local and Bonjour advertisement (mac-nearby-transport-2,
/// follow-up 1). Loopback only: `::1` and `fe80::1%lo0` are the IPv6
/// addresses every Mac has on `lo0`, so a dual-stack listener and scoped
/// link-local dialling are exercised without a second device. The Bonjour
/// test goes through the same NWBrowser / DNSServiceResolve path the
/// reference companion uses and checks the TXT record it is handed.
@MainActor
final class NearbyInterfaceTests: XCTestCase {
    static let psk = Data(repeating: 0x6B, count: 32)
    static let pairId = "pair-v6"
    let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-if-\(UUID().uuidString)")

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    /// A companion-side client dialling one literal address (any family).
    final class AddressClient {
        let connection: NWConnection
        private let queue = DispatchQueue(label: "nearby.test.v6")
        private var splitter = FlashTeXProtocol.LineSplitter()
        private let lock = NSLock()
        private(set) var lines: [Data] = []
        private(set) var isReady = false
        private(set) var isClosed = false
        private(set) var failure: NWError?
        init(host: String, port: UInt16, identity: String = NearbyInterfaceTests.pairId, psk: Data = NearbyInterfaceTests.psk) {
            connection = NWConnection(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!,
                                      using: NearbyListener.clientParameters(identity: identity, psk: psk))
            connection.stateUpdateHandler = { [weak self] state in
                guard let self else { return }
                switch state {
                case .ready: self.isReady = true; self.receiveLoop()
                case .failed(let e), .waiting(let e): self.failure = e; self.isClosed = true
                case .cancelled: self.isClosed = true
                default: break
                }
            }
            connection.start(queue: queue)
        }
        private func receiveLoop() {
            connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, complete, error in
                guard let self else { return }
                if let data, !data.isEmpty {
                    let new = self.splitter.append(data)
                    self.lock.withLock { self.lines.append(contentsOf: new) }
                }
                if complete || error != nil { self.isClosed = true; return }
                self.receiveLoop()
            }
        }
        var lineCount: Int { lock.withLock { lines.count } }
        var allLines: [Data] { lock.withLock { lines } }
        func send<P: Codable>(id: String, type: String, _ payload: P) {
            connection.send(content: NearbyV1.line(id: id, type: type, payload), completion: .contentProcessed { _ in })
        }
        /// Local address family/scope the stack picked, once connected.
        var localEndpoint: NWEndpoint? { connection.currentPath?.localEndpoint }
        var remoteEndpoint: NWEndpoint? { connection.currentPath?.remoteEndpoint }
        func cancel() { connection.cancel() }
    }

    struct TimedOut: Error {}

    private func waitUntil(_ what: String, timeout: TimeInterval = 8, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)", file: file, line: line)
        throw TimedOut()
    }

    func makeState(name: String) -> (NearbyState, PairStore, ShellModel) {
        let store = PairStore(url: tmp.appendingPathComponent("pairs.json"))
        XCTAssertTrue(store.upsert(PairRecord(pairId: Self.pairId, psk: Self.psk.base64EncodedString(), companionName: "v6 iPad",
                                              createdAt: Date(), lastSeenAt: nil)))
        let model = ShellModel()
        model.caretUTF16 = 3
        model.pinAnchorAtCaret()
        let state = NearbyState(store: store, macName: name, loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        return (state, store, model)
    }

    func hello(_ c: AddressClient, nonce: String) {
        c.send(id: "h", type: "hello", NearbyV1.Hello(pairId: Self.pairId, companionName: "v6 iPad", nonce: nonce,
                                                      proof: Pairing.helloProof(psk: Self.psk, nonce: nonce)))
    }

    /// The listener is dual-stack: the same pairing connects over IPv4
    /// loopback, IPv6 loopback and the scoped link-local address, and each
    /// session keeps its exact identity (pair_id, nonce echo, destination).
    func testIPv4IPv6AndLinkLocalConnectionsReachTheSameListener() async throws {
        let (state, _, model) = makeState(name: "FlashTeX v6 \(UUID().uuidString.prefix(6))")
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)
        let anchor = try XCTUnwrap(model.nearbyDestination)
        defer { state.stopAdvertising() }

        var seen: [String: NWEndpoint] = [:]
        for (label, host) in [("ipv4", "127.0.0.1"), ("ipv6", "::1"), ("link-local", "fe80::1%lo0")] {
            let c = AddressClient(host: host, port: port)
            try await waitUntil("\(label) ready (\(String(describing: c.failure)))") { c.isReady || c.isClosed }
            XCTAssertTrue(c.isReady, "\(label): \(String(describing: c.failure))")
            let nonce = "n-\(label)"
            hello(c, nonce: nonce)
            try await waitUntil("\(label) hello_ack") { c.lineCount >= 1 }
            let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: c.allLines[0])
            XCTAssertEqual(ack.type, "hello_ack")
            XCTAssertEqual(ack.payload.nonce, nonce, "\(label): nonce echoed")
            XCTAssertEqual(ack.payload.macName, state.macName)
            XCTAssertEqual(ack.payload.destination, anchor, "\(label): same destination identity on every family")
            XCTAssertNil(ack.payload.pairPsk)
            try await waitUntil("\(label) connected") { state.connectedPairIds.contains(Self.pairId) }
            if let remote = c.remoteEndpoint { seen[label] = remote }
            c.cancel()
            try await waitUntil("\(label) closed") { !state.connectedPairIds.contains(Self.pairId) }
        }
        XCTAssertEqual(seen.count, 3, "\(seen)")
        if case .hostPort(let h, _) = seen["ipv6"], case .ipv6(let a) = h { XCTAssertTrue(a.isLoopback, "\(a)") } else { XCTFail("\(String(describing: seen["ipv6"]))") }
        if case .hostPort(let h, _) = seen["link-local"], case .ipv6(let a) = h {
            XCTAssertTrue(a.isLinkLocal, "\(a)")
            XCTAssertNotNil(a.interface, "scoped link-local address keeps its interface: \(a)")
        } else { XCTFail("\(String(describing: seen["link-local"]))") }
        XCTAssertEqual(state.log.filter { $0.hasPrefix("hello from v6 iPad") }.count, 3)
        print("measured: remote endpoints \(seen)")
    }

    /// The Bonjour advertisement carries the TXT record the companion pairs
    /// from (v, name, fp, salt), under the name the listener registered, and
    /// resolves (the reference client's path) to the advertised port; the
    /// resolved host then accepts the pairing's TLS-PSK session.
    func testBonjourAdvertisementCarriesTXTAndResolvesToTheListener() async throws {
        let name = "FlashTeX Bonjour \(UUID().uuidString.prefix(6))"
        let (state, store, _) = makeState(name: name)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)
        defer { state.stopAdvertising() }

        let params = NWParameters()
        params.includePeerToPeer = false
        let browser = NWBrowser(for: .bonjourWithTXTRecord(type: NearbyV1.serviceType, domain: nil), using: params)
        final class Found: @unchecked Sendable { var results: [NWBrowser.Result] = []; let lock = NSLock() }
        let found = Found()
        browser.browseResultsChangedHandler = { results, _ in found.lock.withLock { found.results = Array(results) } }
        browser.start(queue: DispatchQueue(label: "nearby.test.browse"))
        defer { browser.cancel() }
        func mine() -> NWBrowser.Result? {
            found.lock.withLock {
                found.results.first { if case .service(let n, let t, _, _) = $0.endpoint { return n == name && t == NearbyV1.serviceType }; return false }
            }
        }
        try await waitUntil("service \(name) browsed", timeout: 15) { mine() != nil }
        let result = try XCTUnwrap(mine())
        guard case .bonjour(let txt) = result.metadata else { return XCTFail("no TXT record: \(result.metadata)") }
        XCTAssertEqual(txt.dictionary, state.txtRecord)
        XCTAssertEqual(txt["v"], "1")
        XCTAssertEqual(txt["name"], name)
        XCTAssertEqual(txt["fp"], Pairing.fingerprint(salt: store.salt))
        XCTAssertEqual(txt["salt"], Pairing.hex(store.salt))
        XCTAssertFalse(result.interfaces.isEmpty, "advertised on at least one interface: \(result.interfaces)")
        print("measured: \(name) advertised on \(result.interfaces.map { "\($0.name)/\($0.type)" })")

        let resolved = try await NearbyResolver.resolve(result.endpoint)
        XCTAssertEqual(resolved.port, port, "SRV port is the listener's")
        XCTAssertFalse(resolved.host.isEmpty)
        let c = AddressClient(host: resolved.host, port: resolved.port)
        try await waitUntil("connected through the resolved host \(resolved.host) (\(String(describing: c.failure)))") { c.isReady || c.isClosed }
        XCTAssertTrue(c.isReady, "\(resolved.host): \(String(describing: c.failure))")
        hello(c, nonce: "bonjour-1")
        try await waitUntil("hello_ack") { c.lineCount >= 1 }
        let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: c.allLines[0])
        XCTAssertEqual(ack.payload.macName, name)
        XCTAssertEqual(ack.payload.nonce, "bonjour-1")
        c.cancel()
    }
}
