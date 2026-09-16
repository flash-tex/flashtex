import Foundation
import Network

/// One `_flashtex._tcp` service seen by Bonjour, with its TXT record parsed
/// (proposal §1). Nothing here is authenticated; `fingerprint` only picks
/// which stored pairing to try, the TLS handshake decides.
public struct DiscoveredMac: Hashable, Sendable {
    /// Bonjour instance name (the Mac's user-visible name).
    public let name: String
    public let endpoint: NWEndpoint
    public let txt: [String: String]

    public init(name: String, endpoint: NWEndpoint, txt: [String: String]) {
        self.name = name; self.endpoint = endpoint; self.txt = txt
    }

    public var version: String? { txt["v"] }
    public var fingerprint: String? { txt["fp"] }
    public var saltHex: String? { txt["salt"] }
    public var salt: Data? { saltHex.flatMap(NearbyCrypto.data(hex:)) }
    public var macName: String { txt["name"] ?? name }

    /// `v == "1"`, a 16-byte salt and an `fp` that is really derived from it.
    public var isSupported: Bool { unsupportedReason == nil }

    public var unsupportedReason: String? {
        guard version == "1" else { return "TXT v=\(version ?? "missing") (want 1)" }
        guard let salt, salt.count == NearbyCrypto.saltLength else { return "TXT salt missing or not 16 bytes" }
        guard let fp = fingerprint, fp == NearbyCrypto.fingerprint(salt: salt) else {
            return "TXT fp does not match SHA-256(mac-id ‖ salt)"
        }
        return nil
    }

    /// Builds from an `NWBrowser.Result`; nil for non-Bonjour results.
    public init?(_ result: NWBrowser.Result) {
        guard case .service(let name, _, _, _) = result.endpoint else { return nil }
        var txt: [String: String] = [:]
        if case .bonjour(let record) = result.metadata { txt = record.dictionary }
        self.init(name: name, endpoint: result.endpoint, txt: txt)
    }
}

/// `NWBrowser` for `_flashtex._tcp` on infrastructure networks (no AWDL),
/// delivering parsed results.
public final class NearbyBrowser: @unchecked Sendable {
    private let browser: NWBrowser
    private let queue: DispatchQueue
    /// Called on `queue` with the complete current result set every time it changes.
    public var onResults: (([DiscoveredMac]) -> Void)?
    public var onFailure: ((NWError) -> Void)?

    public init(queue: DispatchQueue = DispatchQueue(label: "flashtex.nearby.browser")) {
        self.queue = queue
        let params = NWParameters()
        params.includePeerToPeer = false
        // `.bonjour(type:domain:)` results carry no TXT record; the companion's
        // current browser uses it and therefore never sees `fp`/`salt`.
        browser = NWBrowser(for: .bonjourWithTXTRecord(type: NearbyWire.serviceType, domain: nil), using: params)
        browser.browseResultsChangedHandler = { [weak self] results, _ in
            guard let self else { return }
            self.onResults?(results.compactMap(DiscoveredMac.init).sorted { $0.name < $1.name })
        }
        browser.stateUpdateHandler = { [weak self] state in
            if case .failed(let e) = state { self?.onFailure?(e) }
        }
    }

    public func start() { browser.start(queue: queue) }
    public func stop() { browser.cancel() }

    /// Browses for up to `seconds`, returning early as soon as `until` accepts
    /// a result (e.g. the fingerprint of a stored pairing).
    public static func discover(seconds: TimeInterval, until: ((DiscoveredMac) -> Bool)? = nil) async throws -> [DiscoveredMac] {
        let b = NearbyBrowser()
        let box = ResultBox()
        return try await withCheckedThrowingContinuation { cont in
            b.onResults = { results in
                box.latest = results
                if let until, results.contains(where: until), box.finish() {
                    b.stop(); cont.resume(returning: results)
                }
            }
            b.onFailure = { e in
                if box.finish() { b.stop(); cont.resume(throwing: NearbyError.browseFailed(String(describing: e))) }
            }
            b.start()
            b.queue.asyncAfter(deadline: .now() + seconds) {
                if box.finish() { b.stop(); cont.resume(returning: box.latest) }
            }
        }
    }

    private final class ResultBox: @unchecked Sendable {
        var latest: [DiscoveredMac] = []
        private var done = false
        func finish() -> Bool { if done { return false }; done = true; return true }
    }
}
