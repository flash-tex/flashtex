import Darwin
import dnssd
import Network
import XCTest
import FlashTeXProtocol
import NearbyClient
@testable import FlashTeXMac

/// Multi-interface LAN advertisement and reachability without a second
/// device (mac-nearby-transport-3). The listener binds to every interface and
/// the companion-side client dials the Mac's own non-loopback addresses
/// (`en*` IPv4 and scoped `fe80::%en*` link-local IPv6; `utun`, `awdl`,
/// `llw`, `bridge` excluded) with the interface scoped on the connection.
/// Packets to a local address are delivered in the stack, so this measures
/// binding, address scoping, Bonjour registration per interface and the
/// framing deadlines on that path — not wire latency between two machines.
/// Every test skips (with the reason) on a Mac with no active `en*` link.
@MainActor
final class NearbyLANInterfaceTests: XCTestCase {
    static let psk = Data(repeating: 0x3C, count: 32)
    static let pairId = "pair-lan"
    let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-lan-\(UUID().uuidString)")

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    struct LANAddress: CustomStringConvertible {
        let interface: String
        let host: String
        let family: String
        var description: String { "\(family) \(host) on \(interface)" }
    }

    /// Active non-loopback, non-point-to-point `en*` interfaces with their
    /// IPv4 and link-local IPv6 addresses (scoped `%name`).
    static func lanAddresses() -> [LANAddress] {
        var out: [LANAddress] = []
        var ifap: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifap) == 0, let first = ifap else { return [] }
        defer { freeifaddrs(ifap) }
        for p in sequence(first: first, next: { $0.pointee.ifa_next }) {
            let ifa = p.pointee
            let name = String(cString: ifa.ifa_name)
            let flags = Int32(bitPattern: ifa.ifa_flags)
            guard flags & IFF_UP != 0, flags & IFF_RUNNING != 0, flags & IFF_LOOPBACK == 0, flags & IFF_POINTOPOINT == 0,
                  name.hasPrefix("en"), let sa = ifa.ifa_addr else { continue }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            let len = socklen_t(sa.pointee.sa_len)
            guard getnameinfo(sa, len, &host, socklen_t(host.count), nil, 0, NI_NUMERICHOST) == 0 else { continue }
            let h = String(cString: host)
            if sa.pointee.sa_family == UInt8(AF_INET) {
                out.append(.init(interface: name, host: h, family: "ipv4"))
            } else if sa.pointee.sa_family == UInt8(AF_INET6), h.hasPrefix("fe80:") {
                out.append(.init(interface: name, host: h.contains("%") ? h : "\(h)%\(name)", family: "ipv6-link-local"))
            }
        }
        return out.sorted { ($0.interface, $0.family) < ($1.interface, $1.family) }
    }

    /// `NWInterface` objects the path monitor knows, by name (a link the
    /// system routes nothing through may be absent; the test then reports it).
    static func knownInterfaces() async -> [String: NWInterface] {
        let monitor = NWPathMonitor()
        final class Once: @unchecked Sendable { var done = false; let lock = NSLock() }
        let once = Once()
        return await withCheckedContinuation { cont in
            monitor.pathUpdateHandler = { path in
                let first = once.lock.withLock { let f = !once.done; once.done = true; return f }
                guard first else { return }
                cont.resume(returning: Dictionary(path.availableInterfaces.map { ($0.name, $0) }, uniquingKeysWith: { a, _ in a }))
                monitor.cancel()
            }
            monitor.start(queue: DispatchQueue(label: "nearby.test.path"))
        }
    }

    func requireLAN() throws -> [LANAddress] {
        let addrs = Self.lanAddresses()
        try XCTSkipIf(addrs.isEmpty, "no active non-loopback en* interface on this Mac (getifaddrs)")
        return addrs
    }

    static func loadAverage() -> Double {
        var l = [Double](repeating: 0, count: 3)
        return getloadavg(&l, 3) >= 1 ? l[0] : 0
    }

    /// A companion-side client dialling one literal address, optionally
    /// pinned to an interface (`NWParameters.requiredInterface`).
    final class LANClient {
        let connection: NWConnection
        private let queue = DispatchQueue(label: "nearby.test.lan")
        private var splitter = FlashTeXProtocol.LineSplitter()
        private let lock = NSLock()
        private(set) var lines: [Data] = []
        private(set) var isReady = false
        private(set) var isClosed = false
        private(set) var failure: NWError?
        private(set) var closedAt: Date?
        init(host: String, port: UInt16, interface: NWInterface? = nil,
             identity: String = NearbyLANInterfaceTests.pairId, psk: Data = NearbyLANInterfaceTests.psk) {
            let params = NearbyListener.clientParameters(identity: identity, psk: psk)
            params.requiredInterface = interface
            connection = NWConnection(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!, using: params)
            connection.stateUpdateHandler = { [weak self] state in
                guard let self else { return }
                switch state {
                case .ready: self.isReady = true; self.receiveLoop()
                case .failed(let e), .waiting(let e): self.failure = e; self.isClosed = true; self.closedAt = Date()
                case .cancelled: self.isClosed = true; self.closedAt = self.closedAt ?? Date()
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
                if complete || error != nil { self.isClosed = true; self.closedAt = self.closedAt ?? Date(); return }
                self.receiveLoop()
            }
        }
        var lineCount: Int { lock.withLock { lines.count } }
        var allLines: [Data] { lock.withLock { lines } }
        func send(_ data: Data) { connection.send(content: data, completion: .contentProcessed { _ in }) }
        func send<P: Codable>(id: String, type: String, _ payload: P) { send(NearbyV1.line(id: id, type: type, payload)) }
        var localEndpoint: NWEndpoint? { connection.currentPath?.localEndpoint }
        var remoteEndpoint: NWEndpoint? { connection.currentPath?.remoteEndpoint }
        var pathInterfaces: [String] { connection.currentPath?.availableInterfaces.map(\.name) ?? [] }
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

    /// All-interface state (the product default) with one long-term pairing.
    func makeState(name: String, limits: NearbyReceiveLimits = .init()) -> (NearbyState, PairStore, RecordingSink) {
        let store = PairStore(url: tmp.appendingPathComponent("pairs.json"))
        XCTAssertTrue(store.upsert(PairRecord(pairId: Self.pairId, psk: Self.psk.base64EncodedString(), companionName: "LAN iPad",
                                              createdAt: Date(), lastSeenAt: nil)))
        let sink = RecordingSink()
        let destinations = FixedDestinations(.init(destinationId: "dest-lan", projectId: "proj-lan", path: "main.tex", baseRevision: 2))
        let state = NearbyState(store: store, macName: name, loopbackOnly: false, limits: limits)
        state.attach(sink: sink, destinations: destinations)
        retained.append(destinations)
        return (state, store, sink)
    }
    /// Weakly held by the state; kept alive for the test.
    private var retained: [AnyObject] = []

    func hello(_ c: LANClient, nonce: String) {
        c.send(id: "h", type: "hello", NearbyV1.Hello(pairId: Self.pairId, companionName: "LAN iPad", nonce: nonce,
                                                      proof: Pairing.helloProof(psk: Self.psk, nonce: nonce)))
    }

    static let png = TestImages.png(width: 24, height: 24)
    func submit(_ captureId: String) -> RuntimeV1.CaptureSubmit {
        .init(captureId: captureId, destinationId: "dest-lan", baseRevision: 2,
              image: .init(mimeType: "image/png", dataBase64: Self.png.base64EncodedString()), instructions: "lan")
    }
    func ack(_ line: Data) -> NearbyV1.CaptureReceived? {
        let e = try? JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: line)
        return e?.type == "capture_received" ? e?.payload : nil
    }

    // MARK: reachability on every LAN address, interface scoped

    /// The all-interface listener accepts the pairing on each `en*` IPv4 and
    /// link-local IPv6 address with the client pinned to that interface, and
    /// every session carries the same identity (nonce echo, name, destination).
    func testAllInterfaceListenerIsReachableOnEachLANAddressWithScopedClient() async throws {
        let addrs = try requireLAN()
        let known = await Self.knownInterfaces()
        let (state, _, _) = makeState(name: "FlashTeX LAN \(UUID().uuidString.prefix(6))")
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)
        defer { state.stopAdvertising() }

        var measured: [String] = []
        for a in addrs {
            let iface = known[a.interface]
            let c = LANClient(host: a.host, port: port, interface: iface)
            try await waitUntil("\(a) ready (\(String(describing: c.failure)))") { c.isReady || c.isClosed }
            XCTAssertTrue(c.isReady, "\(a): \(String(describing: c.failure))")
            guard c.isReady else { continue }
            let nonce = "n-\(a.interface)-\(a.family)"
            hello(c, nonce: nonce)
            try await waitUntil("\(a) hello_ack") { c.lineCount >= 1 }
            let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: c.allLines[0])
            XCTAssertEqual(ack.type, "hello_ack", "\(a)")
            XCTAssertEqual(ack.payload.nonce, nonce, "\(a)")
            XCTAssertEqual(ack.payload.macName, state.macName)
            XCTAssertEqual(ack.payload.destination?.destinationId, "dest-lan", "\(a): same destination on every address")
            try await waitUntil("\(a) connected") { state.connectedPairIds.contains(Self.pairId) }
            let remote = c.remoteEndpoint
            if case .hostPort(let h, let p) = remote {
                XCTAssertEqual(p.rawValue, port, "\(a)")
                switch h {
                case .ipv4(let v4): XCTAssertEqual(a.family, "ipv4"); XCTAssertFalse(v4.isLoopback, "\(a) is not loopback")
                case .ipv6(let v6):
                    XCTAssertEqual(a.family, "ipv6-link-local")
                    XCTAssertTrue(v6.isLinkLocal, "\(a)")
                    XCTAssertEqual(v6.interface?.name, a.interface, "\(a): scoped to its interface")
                default: XCTFail("\(a): unexpected host \(h)")
                }
            } else { XCTFail("\(a): no remote endpoint") }
            measured.append("\(a): scoped=\(iface != nil) remote=\(String(describing: remote)) local=\(String(describing: c.localEndpoint))")
            c.cancel()
            try await waitUntil("\(a) closed") { !state.connectedPairIds.contains(Self.pairId) }
        }
        XCTAssertEqual(state.log.filter { $0.hasPrefix("hello from LAN iPad") }.count, addrs.count)
        for m in measured { print("measured: \(m)") }
        print("measured: interfaces known to NWPathMonitor \(known.keys.sorted()); LAN addresses \(addrs)")
    }

    // MARK: Bonjour per interface

    /// Local DNSServiceResolve pinned to one interface index.
    static func resolve(name: String, interfaceIndex: UInt32, timeout: TimeInterval = 5) -> (host: String, port: UInt16)? {
        final class Box: @unchecked Sendable { var result: (String, UInt16)?; let sema = DispatchSemaphore(value: 0) }
        let box = Box()
        var ref: DNSServiceRef?
        let ctx = Unmanaged.passUnretained(box).toOpaque()
        let err = DNSServiceResolve(&ref, 0, interfaceIndex, name, NearbyV1.serviceType, "local.", { _, _, _, errorCode, _, hosttarget, port, _, _, context in
            guard let context, errorCode == kDNSServiceErr_NoError, let hosttarget else { return }
            let box = Unmanaged<Box>.fromOpaque(context).takeUnretainedValue()
            var host = String(cString: hosttarget)
            if host.hasSuffix(".") { host.removeLast() }
            if box.result == nil { box.result = (host, UInt16(bigEndian: port)); box.sema.signal() }
        }, ctx)
        guard err == kDNSServiceErr_NoError, let ref else { return nil }
        DNSServiceSetDispatchQueue(ref, DispatchQueue(label: "nearby.test.resolve.\(interfaceIndex)"))
        _ = box.sema.wait(timeout: .now() + timeout)
        DNSServiceRefDeallocate(ref)
        return box.result
    }

    /// NWBrowser reports the service on every `en*` interface (and on no
    /// `utun`), with the TXT record the companion pairs from; a resolve
    /// pinned to each interface index returns the listener's port, and the
    /// reference client's `DiscoveredMac` carries the fingerprint on each.
    func testBonjourIsAdvertisedOnEachLANInterfaceAndResolvesPerInterface() async throws {
        let addrs = try requireLAN()
        let names = Set(addrs.map(\.interface))
        let name = "FlashTeX LAN Bonjour \(UUID().uuidString.prefix(6))"
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
        browser.start(queue: DispatchQueue(label: "nearby.test.browse.lan"))
        defer { browser.cancel() }
        func mine() -> NWBrowser.Result? {
            found.lock.withLock {
                found.results.first { if case .service(let n, let t, _, _) = $0.endpoint { return n == name && t == NearbyV1.serviceType }; return false }
            }
        }
        // Registration on a second interface can arrive after the first; wait
        // until every LAN interface is listed (or the timeout names the gap).
        try await waitUntil("service \(name) on \(names.sorted())", timeout: 20) {
            guard let r = mine() else { return false }
            return names.isSubset(of: Set(r.interfaces.map(\.name)))
        }
        let result = try XCTUnwrap(mine())
        let seen = result.interfaces.map(\.name)
        XCTAssertTrue(names.isSubset(of: Set(seen)), "advertised on every LAN interface \(names.sorted()); saw \(seen)")
        XCTAssertFalse(seen.contains { $0.hasPrefix("utun") }, "no utun in \(seen)")
        guard case .bonjour(let txt) = result.metadata else { return XCTFail("no TXT record: \(result.metadata)") }
        XCTAssertEqual(txt.dictionary, state.txtRecord)
        XCTAssertEqual(txt["fp"], Pairing.fingerprint(salt: store.salt))
        let discovered = try XCTUnwrap(DiscoveredMac(result), "reference client accepts the browse result")
        XCTAssertEqual(discovered.fingerprint, state.fingerprint)
        XCTAssertTrue(discovered.isSupported, discovered.unsupportedReason ?? "")
        print("measured: \(name) advertised on \(result.interfaces.map { "\($0.name)/\($0.type)" })")

        for ifname in names.sorted() {
            let index = if_nametoindex(ifname)
            XCTAssertNotEqual(index, 0, "\(ifname) has an index")
            let r = Self.resolve(name: name, interfaceIndex: index)
            XCTAssertEqual(r?.port, port, "SRV port on \(ifname) (index \(index)): \(String(describing: r))")
            print("measured: resolve on \(ifname)#\(index) -> \(String(describing: r))")
        }
    }

    // MARK: refusals on a LAN address

    /// A wrong key is refused in the handshake on a non-loopback address (no
    /// application byte parsed, listener survives), and a pairing that
    /// remembers a different Mac fingerprint does not match this service
    /// however many interfaces advertise it.
    func testWrongKeyIsRefusedAndForeignFingerprintIsNotMatchedOnLAN() async throws {
        let addrs = try requireLAN()
        let known = await Self.knownInterfaces()
        let name = "FlashTeX LAN fp \(UUID().uuidString.prefix(6))"
        let (state, _, _) = makeState(name: name)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)
        defer { state.stopAdvertising() }

        for a in addrs {
            let wrong = LANClient(host: a.host, port: port, interface: known[a.interface], psk: Data(repeating: 0x11, count: 32))
            try await waitUntil("\(a) wrong-key verdict") { wrong.isClosed || wrong.isReady }
            XCTAssertFalse(wrong.isReady, "\(a): wrong key must not reach ready")
            XCTAssertEqual(wrong.lineCount, 0, "\(a): no application line")
            print("measured: wrong key on \(a) -> \(String(describing: wrong.failure))")
            wrong.cancel()
        }
        try await waitUntil("handshake failures logged") {
            state.log.filter { $0.contains("handshake failed") }.count >= addrs.count
        }
        XCTAssertTrue(state.connectedPairIds.isEmpty)

        // Listener still serves the right key on the same address afterwards.
        let a = addrs[0]
        let right = LANClient(host: a.host, port: port, interface: known[a.interface])
        try await waitUntil("\(a) right key ready (\(String(describing: right.failure)))") { right.isReady || right.isClosed }
        XCTAssertTrue(right.isReady, "\(a): \(String(describing: right.failure))")
        hello(right, nonce: "after-refusals")
        try await waitUntil("hello_ack") { right.lineCount >= 1 }
        right.cancel()

        // Foreign fingerprint: the reference client's `find` sees this Mac on
        // the LAN interfaces but refuses to match it.
        let foreign = PairedMac(fingerprint: String(repeating: "0", count: state.fingerprint.count), macName: name,
                                pairId: Self.pairId, pairPsk: Self.psk.base64EncodedString(), companionName: "LAN iPad",
                                pairedAt: Date(), lastSeenAt: nil)
        do {
            let mac = try await NearbyClient.find(pair: foreign, seconds: 4)
            XCTFail("foreign fingerprint matched \(mac.name)")
        } catch let e as NearbyError {
            guard case .noMatchingMac(let why) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(why.contains(name), "the refusal names the service it saw: \(why)")
            XCTAssertTrue(why.contains("fp=\(state.fingerprint)"), "…with its real fingerprint: \(why)")
            print("measured: foreign fp refused: \(why)")
        }
        let mine = PairedMac(fingerprint: state.fingerprint, macName: name, pairId: Self.pairId,
                             pairPsk: Self.psk.base64EncodedString(), companionName: "LAN iPad", pairedAt: Date(), lastSeenAt: nil)
        let matched = try await NearbyClient.find(pair: mine, seconds: 8)
        XCTAssertEqual(matched.name, name)
    }

    // MARK: reconnect through another address

    /// The first session (one address) delivers a capture; that interface
    /// "goes away" (the session is closed) and the companion reconnects
    /// through a second address: its retry is acknowledged from the
    /// pairing's memory and the sink never sees the capture twice.
    func testReconnectViaASecondAddressIsAcknowledgedFromMemoryNotRedelivered() async throws {
        let addrs = try requireLAN()
        try XCTSkipIf(addrs.count < 2, "one LAN address only (\(addrs)); a second address is needed")
        let known = await Self.knownInterfaces()
        let (state, _, sink) = makeState(name: "FlashTeX LAN reconnect \(UUID().uuidString.prefix(6))")
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)
        defer { state.stopAdvertising() }
        // Prefer two different interfaces; otherwise two families of one.
        let first = addrs[0]
        let second = addrs.first { $0.interface != first.interface } ?? addrs[1]

        let a = LANClient(host: first.host, port: port, interface: known[first.interface])
        try await waitUntil("\(first) ready (\(String(describing: a.failure)))") { a.isReady || a.isClosed }
        XCTAssertTrue(a.isReady, "\(first): \(String(describing: a.failure))")
        hello(a, nonce: "first")
        try await waitUntil("hello_ack") { a.lineCount >= 1 }
        a.send(NearbyV1.line(id: "s1", type: "capture_submit", submit("cap-lan-1")))
        try await waitUntil("first ack") { a.lineCount >= 2 }
        XCTAssertEqual(ack(a.allLines[1])?.captureId, "cap-lan-1")
        XCTAssertEqual(sink.count, 1)
        a.cancel()
        try await waitUntil("first session gone") { !state.connectedPairIds.contains(Self.pairId) }

        let b = LANClient(host: second.host, port: port, interface: known[second.interface])
        try await waitUntil("\(second) ready (\(String(describing: b.failure)))") { b.isReady || b.isClosed }
        XCTAssertTrue(b.isReady, "\(second): \(String(describing: b.failure))")
        hello(b, nonce: "second")
        try await waitUntil("hello_ack 2") { b.lineCount >= 1 }
        b.send(NearbyV1.line(id: "s2", type: "capture_submit", submit("cap-lan-1")))
        try await waitUntil("retry ack") { b.lineCount >= 2 }
        XCTAssertEqual(ack(b.allLines[1])?.captureId, "cap-lan-1", "acknowledged again on the second address")
        XCTAssertEqual(sink.count, 1, "never re-delivered")
        try await waitUntil("duplicate counted") { state.duplicateCaptureCount == 1 }
        XCTAssertEqual(state.lastDuplicateCaptureId, "cap-lan-1")
        // A genuinely new capture on the second address is delivered.
        b.send(NearbyV1.line(id: "s3", type: "capture_submit", submit("cap-lan-2")))
        try await waitUntil("new ack") { b.lineCount >= 3 }
        XCTAssertEqual(ack(b.allLines[2])?.captureId, "cap-lan-2")
        XCTAssertEqual(sink.count, 2)
        print("measured: reconnect \(first) -> \(second): retry acknowledged from memory, sink saw 1 then 2")
        b.cancel()
    }

    // MARK: frame timeout on a non-loopback path

    /// A partial frame over a LAN address is refused at the frame deadline
    /// (typed error, then close), measured from the first byte on that path.
    func testFrameTimeoutIsEnforcedOnANonLoopbackPath() async throws {
        let addrs = try requireLAN()
        let load = Self.loadAverage()
        print("measured: 1-min load \(load)")
        try XCTSkipIf(load > 20, "timing test skipped under load \(load)")
        let known = await Self.knownInterfaces()
        var limits = NearbyReceiveLimits()
        limits.frameTimeout = 0.5
        limits.helloTimeout = 5
        let (state, _, sink) = makeState(name: "FlashTeX LAN timeout \(UUID().uuidString.prefix(6))", limits: limits)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)
        defer { state.stopAdvertising() }

        for a in addrs where a.family == "ipv4" || addrs.count == 1 {
            let c = LANClient(host: a.host, port: port, interface: known[a.interface])
            try await waitUntil("\(a) ready (\(String(describing: c.failure)))") { c.isReady || c.isClosed }
            XCTAssertTrue(c.isReady, "\(a): \(String(describing: c.failure))")
            hello(c, nonce: "t-\(a.interface)")
            try await waitUntil("hello_ack") { c.lineCount >= 1 }
            let partial = Data(NearbyV1.line(id: "p", type: "capture_submit", submit("cap-partial")).dropLast(40))
            let sentAt = Date()
            c.send(partial)
            try await waitUntil("\(a) frame timeout close", timeout: 5) { c.isClosed }
            let elapsed = (c.closedAt ?? Date()).timeIntervalSince(sentAt)
            let errors = c.allLines.dropFirst().compactMap { try? JSONDecoder().decode(NearbyListenerTests.ErrorLine.self, from: $0) }
            XCTAssertEqual(errors.first?.payload.code, "frame_timeout", "\(a): \(c.allLines.map { String(decoding: $0, as: UTF8.self) })")
            XCTAssertGreaterThanOrEqual(elapsed, 0.45, "\(a): not before the deadline")
            XCTAssertLessThan(elapsed, 3.0, "\(a): closed soon after the deadline (load \(load))")
            try await waitUntil("\(a) closed reason") { state.log.contains { $0.contains("frame timed out after 0.50s") } }
            print("measured: frame timeout on \(a): closed after \(String(format: "%.3f", elapsed)) s (deadline 0.5 s)")
            c.cancel()
        }
        XCTAssertEqual(sink.count, 0, "nothing delivered from a partial frame")
    }
}
