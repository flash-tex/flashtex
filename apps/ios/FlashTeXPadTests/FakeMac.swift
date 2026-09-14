import Foundation
import Network
import NearbyClient
import Security

/// Deterministic Mac-side fixture for the simulator tests: a copy of
/// apps/mac/tools/nearby-client/Tests/NearbyClientTests/FakeMac.swift with
/// the fault-injection knobs removed. Same TLS-PSK parameters as
/// NearbyListener, verifies `hello.proof`, hands over `pair_psk` on the
/// bootstrap key, answers `destination_query` and `capture_submit`
/// (in-memory inbox: `durable:false`), errors on anything else. Loopback only.
/// No conversion provider, no Rust helper: it answers exactly what nearby-v1
/// §4 lets a Mac answer, nothing more.
final class FakeMac {
    struct Key { let identity: String; let psk: Data; let bootstrap: Bool }
    let keys: [Key]
    let macName: String
    let longTermPSK = Data((0..<32).map { _ in UInt8.random(in: 0...255) })
    private var _destination: NearbyWire.Destination?
    private var _captures: [NearbyWire.CaptureSubmit] = []
    /// Scripted `capture_status_ack` payloads per capture_id (nearby-v1 §4,
    /// additive). A capture this instance received but has no script for is
    /// answered `received` / `durable:false` (inbox); an id it never received
    /// is `unknown_capture` — so `FakeMac.restart` (fresh instance) models a
    /// Mac whose acknowledgement memory lost the capture.
    private var _status: [String: [String: Any]] = [:]
    private var _statusRequests: [String] = []
    /// Knobs: `answersStatus == false` → `unknown_type` (a Mac that predates
    /// the message); `statusEchoWrongId` → an ack naming another capture.
    var answersStatus = true
    var statusEchoWrongId = false
    private var _hellos: [NearbyWire.Hello] = []
    private var listener: NWListener!
    private let queue = DispatchQueue(label: "fake.mac")
    private var connections: [NWConnection] = []
    private(set) var port: UInt16 = 0
    private let ready = DispatchSemaphore(value: 0)

    static func dispatchData(_ data: Data) -> __DispatchData {
        data.withUnsafeBytes { DispatchData(bytes: $0) as __DispatchData }
    }

    /// The wire structs' memberwise inits are internal to NearbyClient; the
    /// fixture builds replies from their documented JSON instead.
    static func wire<T: Decodable>(_ object: [String: Any]) -> T {
        try! JSONDecoder().decode(T.self, from: try! JSONSerialization.data(withJSONObject: object))
    }

    /// Runs on `queue`.
    private var destinationJSON: Any {
        guard let d = _destination else { return NSNull() }
        var payload: [String: Any] = ["destination_id": d.destinationId, "project_id": d.projectId, "path": d.path, "base_revision": d.baseRevision]
        if let caret = d.caretContext {
            // Absent optionals are omitted, not written as null: JSONSerialization
            // refuses a Swift `Optional` and the decoder treats both the same.
            var context: [String: Any] = ["mode": caret.mode, "wrap": caret.wrap,
                                          "environments": caret.environments, "amsmath": caret.amsmath]
            if let delimiter = caret.delimiter { context["delimiter"] = delimiter }
            if let environment = caret.environment { context["environment"] = environment }
            payload["caret_context"] = context
        }
        return payload
    }

    init(keys: [Key], macName: String = "Fake Mac", destination: NearbyWire.Destination? = nil, port: UInt16 = 0) throws {
        self.keys = keys
        self.macName = macName
        self._destination = destination
        let tls = NWProtocolTLS.Options()
        let sec = tls.securityProtocolOptions
        sec_protocol_options_set_min_tls_protocol_version(sec, .TLSv12)
        sec_protocol_options_set_max_tls_protocol_version(sec, .TLSv12)
        sec_protocol_options_append_tls_ciphersuite(sec, tls_ciphersuite_t(rawValue: NearbyCrypto.cipherSuiteRawValue)!)
        sec_protocol_options_set_tls_resumption_enabled(sec, false)
        sec_protocol_options_set_tls_tickets_enabled(sec, false)
        for k in keys {
            sec_protocol_options_add_pre_shared_key(sec, FakeMac.dispatchData(k.psk), FakeMac.dispatchData(Data(k.identity.utf8)))
        }
        let params = NWParameters(tls: tls, tcp: NWProtocolTCP.Options())
        params.allowLocalEndpointReuse = true
        params.requiredInterfaceType = .loopback
        listener = try NWListener(using: params, on: port == 0 ? .any : NWEndpoint.Port(rawValue: port)!)
        listener.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            if case .ready = state { self.port = self.listener.port?.rawValue ?? 0; self.ready.signal() }
            if case .failed = state { self.ready.signal() }
        }
        listener.newConnectionHandler = { [weak self] c in self?.accept(c) }
    }

    func start() {
        listener.start(queue: queue)
        _ = ready.wait(timeout: .now() + 5)
    }

    func stop() { listener.cancel(); queue.sync { connections.forEach { $0.cancel() }; connections.removeAll() } }

    /// Stops `previous` and starts a replacement on its port with `keys` — the
    /// Mac after pairing, whose key table now holds the long-term `pair_psk`
    /// under the same `pair_id` (retrying the bind while the socket is released).
    /// The replacement listens on a fresh ephemeral port (the old socket may
    /// linger in the kernel; rebinding the same port raced `bind` with
    /// EADDRINUSE and crashed the test host). Callers reconnect to `port`
    /// after the bounded readiness check here; the pairing (`pair_id`, PSK)
    /// is what carries over, the acknowledgement memory is deliberately lost.
    static func restart(_ previous: FakeMac, keys: [Key]) throws -> FakeMac {
        previous.stop()
        let m = try FakeMac(keys: keys, macName: previous.macName, destination: previous.destination, port: 0)
        m.start() // waits up to 5 s for `.ready`
        guard m.port != 0 else { throw NearbyError.unreachable("restarted FakeMac did not become ready on an ephemeral port") }
        return m
    }

    var destination: NearbyWire.Destination? {
        get { queue.sync { _destination } }
        set { queue.sync { _destination = newValue } }
    }
    var captures: [NearbyWire.CaptureSubmit] { queue.sync { _captures } }
    var statusRequests: [String] { queue.sync { _statusRequests } }
    func setStatus(_ captureId: String, state: String, durable: Bool = true, latex: String? = nil, note: String? = nil, newRevision: Int? = nil) {
        var j: [String: Any] = ["capture_id": captureId, "state": state, "durable": durable]
        if let latex { j["latex"] = latex }
        if let note { j["note"] = note }
        if let newRevision { j["new_revision"] = newRevision }
        queue.sync { _status[captureId] = j }
    }
    var hellos: [NearbyWire.Hello] { queue.sync { _hellos } }

    private func accept(_ c: NWConnection) {
        connections.append(c)
        var splitter = LineSplitter()
        var helloDone = false
        c.stateUpdateHandler = { state in if case .failed = state { c.cancel() } }
        func receive() {
            c.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, complete, error in
                guard let self else { return }
                if let data {
                    for line in splitter.append(data) {
                        guard let header = try? NearbyWire.header(of: line) else { c.cancel(); return }
                        let id = header.id ?? "?"
                        func reply<P: Codable>(_ type: String, _ p: P) { c.send(content: try! NearbyWire.line(id: id, type: type, p), completion: .idempotent) }
                        func fail(_ code: String, _ msg: String) { reply("error", FakeMac.wire(["code": code, "message": msg]) as NearbyWire.ErrorPayload) }
                        func closeAfterFlush() {
                            c.send(content: nil, contentContext: .finalMessage, isComplete: true, completion: .contentProcessed { _ in c.cancel() })
                        }
                        switch header.type {
                        case "hello":
                            guard let h = try? NearbyWire.decode(line, as: NearbyWire.Hello.self).payload else { fail("bad_request", "undecodable hello"); closeAfterFlush(); return }
                            self._hellos.append(h)
                            guard let key = self.keys.first(where: { $0.identity == h.pairId }),
                                  NearbyCrypto.verifyHelloProof(h.proof, psk: key.psk, nonce: h.nonce) else {
                                fail("pair_mismatch", "unknown pair or bad proof"); closeAfterFlush(); return
                            }
                            helloDone = true
                            var ack: [String: Any] = ["mac_name": self.macName, "nonce": h.nonce, "destination": self.destinationJSON]
                            if key.bootstrap { ack["pair_psk"] = self.longTermPSK.base64EncodedString() }
                            reply("hello_ack", FakeMac.wire(ack) as NearbyWire.HelloAck)
                        case _ where !helloDone:
                            fail("hello_required", "first message must be hello"); closeAfterFlush(); return
                        case "destination_query":
                            reply("destination", FakeMac.wire(["destination": self.destinationJSON]) as NearbyWire.DestinationReply)
                        case "capture_submit":
                            guard let cap = try? NearbyWire.decode(line, as: NearbyWire.CaptureSubmit.self).payload else { fail("bad_request", "undecodable capture_submit"); continue }
                            self._captures.append(cap)
                            reply("capture_received", FakeMac.wire(["capture_id": cap.captureId, "durable": false, "has_proposal": false, "applied": false]) as NearbyWire.CaptureReceived)
                        case "capture_status" where self.answersStatus:
                            guard let req = try? NearbyWire.decode(line, as: NearbyWire.CaptureStatusRequest.self).payload else { fail("bad_request", "undecodable capture_status"); continue }
                            self._statusRequests.append(req.captureId)
                            guard self._captures.contains(where: { $0.captureId == req.captureId }) else {
                                fail("unknown_capture", "capture \(req.captureId) was not accepted on this pairing"); continue
                            }
                            var j = self._status[req.captureId] ?? ["capture_id": req.captureId, "state": "received", "durable": false, "note": "FakeMac inbox"]
                            if self.statusEchoWrongId { j["capture_id"] = "someone-else" }
                            reply("capture_status_ack", FakeMac.wire(j) as NearbyWire.CaptureStatus)
                        default:
                            fail("unknown_type", "unknown message type \(header.type)")
                        }
                    }
                }
                if complete || error != nil { c.cancel(); return }
                receive()
            }
        }
        c.start(queue: queue)
        receive()
    }
}
