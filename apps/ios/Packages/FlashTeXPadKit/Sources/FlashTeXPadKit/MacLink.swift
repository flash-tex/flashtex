import Foundation
import Network
import NearbyClient

/// The iPad's link to a Mac over the EXISTING nearby-v1 / transfer-v1
/// contract (apps/mac/docs/nearby-v1-proposal.md §4). Every request below is
/// one the reference client already speaks; this file adds no message type.
///
/// What the transport carries to a companion, and therefore what this link
/// can truthfully do:
///   hello → hello_ack {mac_name, destination, pair_psk?}     (pairing, current pin)
///   destination_query → destination {destination | null}    (re-read the pin)
///   capture_submit → capture_received {capture_id, durable, has_proposal, applied}
/// What it does NOT carry (proposal §6 "Not provided in v1"): compile results,
/// diagnostics, completions, proposals or insertion results. Those stay on the
/// Mac; the app labels every such panel "not carried by transfer-v1".
public final class MacLink: @unchecked Sendable {
    public struct TranscriptLine: Identifiable, Equatable, Sendable {
        public enum Direction: String, Sendable { case sent, received, note }
        public let id = UUID()
        public let at: Date
        public let direction: Direction
        public let text: String
        public init(at: Date = Date(), direction: Direction, text: String) { self.at = at; self.direction = direction; self.text = text }
    }

    private let lock = NSLock()
    private var _session: NearbySession?
    private var _pair: PairedMac?
    private var _transcript: [TranscriptLine] = []
    /// Called on an arbitrary queue whenever the transcript grows.
    public var onTranscript: (@Sendable (TranscriptLine) -> Void)?
    /// Pairings (Keychain in the app; a `PairFile` or nil in tests).
    public let store: PairingStore?

    public init(store: PairingStore? = nil) { self.store = store }

    public var session: NearbySession? { lock.withLock { _session } }
    public var pair: PairedMac? { lock.withLock { _pair } }
    public var transcript: [TranscriptLine] { lock.withLock { _transcript } }
    public var isConnected: Bool { session?.isOpen ?? false }
    public var destination: NearbyWire.Destination? { session?.destination }

    private func log(_ d: TranscriptLine.Direction, _ text: String) {
        let line = TranscriptLine(direction: d, text: text)
        lock.withLock { _transcript.append(line); if _transcript.count > 200 { _transcript.removeFirst() } }
        onTranscript?(line)
    }

    private func onLine(_ dir: NearbyConnection.Direction, _ data: Data) {
        // The bootstrap hello_ack carries pair_psk; never echo key material.
        var s = String(decoding: data, as: UTF8.self).trimmingCharacters(in: .newlines)
        if s.contains("\"pair_psk\"") { s = s.replacingOccurrences(of: #""pair_psk":"[^"]*""#, with: #""pair_psk":"<redacted>""#, options: .regularExpression) }
        if s.contains("\"data_base64\"") { s = s.replacingOccurrences(of: #""data_base64":"[^"]*""#, with: #""data_base64":"<image bytes>""#, options: .regularExpression) }
        log(dir == .sent ? .sent : .received, s)
    }

    /// Pairing-code bootstrap (proposal §2/§7 step 2): derive pair_id/psk_boot
    /// from (code, salt), TLS-PSK, `hello`, keep `hello_ack.pair_psk`.
    /// `salt` is the Mac's TXT `salt` (16 bytes hex) and `fingerprint` its `fp`.
    @discardableResult
    public func pair(host: String, port: UInt16, saltHex: String, fingerprint: String, macName: String,
                     code: String, companionName: String) async throws -> PairedMac {
        guard let salt = NearbyCrypto.data(hex: saltHex) else { throw NearbyError.invalidInput("salt is not hex") }
        let endpoint = NWEndpoint.hostPort(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!)
        log(.note, "pairing with \(macName) at \(host):\(port) fp=\(fingerprint)")
        let (pair, session) = try await NearbyClient.pair(endpoint: endpoint, salt: salt, fingerprint: fingerprint, macName: macName,
                                                          code: code, companionName: companionName, onLine: { [weak self] in self?.onLine($0, $1) })
        session.connection.onClose = { [weak self] why in self?.log(.note, "closed: \(why)") }
        lock.withLock { _session = session; _pair = pair }
        storePairing(pair)
        log(.note, "paired: \(pair.macName) pair_id=\(pair.pairId) destination=\(session.destination.map { $0.destinationId } ?? "null")")
        return pair
    }

    /// Pairing from the Mac's QR payload (`flashtex-nearby://pair?…`, parsed
    /// by the reference client's `NearbyBootstrapPayload`): the same bootstrap
    /// as `pair(host:…)`, with the salt/fp/code taken from the payload. The
    /// payload carries no address, so the Mac is found by Bonjour `fp`
    /// (`browseSeconds`) unless `host`/`port` are given (simulator: no Bonjour
    /// listener to browse for in the tests).
    @discardableResult
    public func pair(bootstrap payload: NearbyBootstrapPayload, host: String?, port: UInt16?, companionName: String,
                     browseSeconds: TimeInterval = 5) async throws -> PairedMac {
        if let host, let port {
            return try await pair(host: host, port: port, saltHex: NearbyCrypto.hex(payload.salt), fingerprint: payload.fingerprint,
                                  macName: payload.macName, code: payload.code, companionName: companionName)
        }
        log(.note, "browsing \(NearbyWire.serviceType) for fp=\(payload.fingerprint) (\(Int(browseSeconds)) s)")
        let found = try await NearbyBrowser.discover(seconds: browseSeconds) { $0.fingerprint == payload.fingerprint }
        guard let mac = found.first(where: { $0.fingerprint == payload.fingerprint }) else {
            throw NearbyError.noMatchingMac("no \(NearbyWire.serviceType) service with fp \(payload.fingerprint) within \(Int(browseSeconds)) s — enter the host and port shown in the Mac's Nearby window")
        }
        log(.note, "found \(mac.macName) at \(mac.endpoint)")
        let (pair, session) = try await NearbyClient.pair(mac: mac, code: payload.code, companionName: companionName,
                                                          onLine: { [weak self] in self?.onLine($0, $1) })
        session.connection.onClose = { [weak self] why in self?.log(.note, "closed: \(why)") }
        lock.withLock { _session = session; _pair = pair }
        storePairing(pair)
        log(.note, "paired: \(pair.macName) pair_id=\(pair.pairId)")
        return pair
    }

    /// The pairing is usable for this session even when the store refuses it
    /// (a Keychain OSStatus); the refusal is logged rather than swallowed so a
    /// pairing that does not survive relaunch has a named cause in the transcript.
    private func storePairing(_ pair: PairedMac) {
        do { try store?.upsert(pair) } catch { log(.note, "pairing not stored: \(error)") }
    }

    /// Every later connection (proposal §7 step 3) with the stored `pair_psk`.
    public func connect(host: String, port: UInt16, pair: PairedMac) async throws {
        let endpoint = NWEndpoint.hostPort(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!)
        log(.note, "connecting to \(pair.macName) at \(host):\(port)")
        let session = try await NearbyClient.connect(endpoint: endpoint, pair: pair, onLine: { [weak self] in self?.onLine($0, $1) })
        session.connection.onClose = { [weak self] why in self?.log(.note, "closed: \(why)") }
        lock.withLock { _session = session; _pair = pair }
    }

    /// Adopts a session another path opened (auto-reconnect through Bonjour).
    public func adopt(session: NearbySession, pair: PairedMac) {
        session.connection.onClose = { [weak self] why in self?.log(.note, "closed: \(why)") }
        lock.withLock { _session = session; _pair = pair }
        log(.note, "connected to \(pair.macName) (\(pair.pairId)) via discovery")
    }

    /// Tap-to-pair with a Mac found by Bonjour (same bootstrap as the QR path;
    /// the code is still typed by the user).
    @discardableResult
    public func pair(discovered mac: DiscoveredMac, code: String, companionName: String) async throws -> PairedMac {
        log(.note, "pairing with \(mac.macName) at \(mac.endpoint)")
        let (pair, session) = try await NearbyClient.pair(mac: mac, code: code, companionName: companionName,
                                                          onLine: { [weak self] in self?.onLine($0, $1) })
        session.connection.onClose = { [weak self] why in self?.log(.note, "closed: \(why)") }
        lock.withLock { _session = session; _pair = pair }
        storePairing(pair)
        log(.note, "paired: \(pair.macName) pair_id=\(pair.pairId)")
        return pair
    }

    public func destinationQuery() async throws -> NearbyWire.Destination? {
        guard let s = session else { throw NearbyError.closed("not connected") }
        return try await s.destinationQuery()
    }

    /// transfer-v1 `capture_submit` → `capture_received`. The Mac converts and
    /// reviews on its side; the receipt is all the companion learns.
    public func submitCapture(image: Data, mimeType: String, instructions: String,
                              captureId: String = "cap-" + UUID().uuidString.lowercased()) async throws -> NearbyWire.CaptureReceived {
        guard let s = session else { throw NearbyError.closed("not connected") }
        let dest = try await s.destinationQuery()
        guard dest != nil else {
            throw NearbyError.invalidInput("the Mac has no pinned insertion point (Edit > Pin Insertion Point on the Mac)")
        }
        let cap = try s.makeCapture(captureId: captureId, image: image, mimeType: mimeType, instructions: instructions, destination: dest)
        let r = try await s.submitCapture(cap)
        log(.note, "capture_received \(r.captureId) durable=\(r.durable) has_proposal=\(r.hasProposal) applied=\(r.applied)")
        return r
    }

    /// `capture_status` for a capture this pairing submitted (additive).
    public func captureStatus(captureId: String) async throws -> NearbyWire.CaptureStatus {
        guard let s = session else { throw NearbyError.closed("not connected") }
        return try await s.captureStatus(captureId: captureId)
    }

    public func disconnect() {
        let s = lock.withLock { () -> NearbySession? in let s = _session; _session = nil; return s }
        s?.close()
    }
}
