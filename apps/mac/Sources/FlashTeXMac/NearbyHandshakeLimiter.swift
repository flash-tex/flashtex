import Foundation

/// Failed TLS-PSK handshakes, per peer address (proposal nearby-v1 §5). Network.framework does not say why a handshake
/// failed or which key the peer tried, so a failure is a connection that
/// ended before `.ready`: the stack reported `.failed`, or the peer did not
/// finish within `handshakeTimeout`. A wrong key, a plaintext probe and a
/// stalled peer all count; a refusal by this limiter itself does not (no
/// handshake was attempted).
///
/// - **Per address:** the first `freeFailures` failures cost nothing (a
///   companion that was forgotten on the Mac, a typo). After that the
///   address waits `baseBackoff × 2^(n − freeFailures)` seconds from its
///   last failure, at most `maxBackoff`, before a new connection from it is
///   even accepted. A success (any key) clears the address; failures older
///   than `forgetAfter` are forgotten.
/// - **Pending handshakes:** at most `maxPendingPerAddress` connections
///   from one address may be in the handshake at once, so a burst of
///   simultaneous connections cannot spend failures faster than the backoff
///   allows.
///
/// Nothing here ends a pairing attempt. Anyone on the network can fail a
/// handshake, so a count of failures against the code would let an
/// unauthenticated peer withdraw it; with a 256-bit code an online guess
/// cannot succeed, and the code expires on its own (`Pairing.codeLifetime`).
struct NearbyHandshakeLimiter: Equatable {
    struct Policy: Equatable {
        var freeFailures = 3
        var baseBackoff: TimeInterval = 1
        var maxBackoff: TimeInterval = 300
        var forgetAfter: TimeInterval = 600
        /// Connections from one address still in the TLS handshake at once.
        var maxPendingPerAddress = 2
        /// Addresses remembered at once; the stalest is dropped first.
        var maxTrackedAddresses = 256
    }

    struct Entry: Equatable {
        var failures: Int
        var last: Date
    }

    var policy: Policy
    private(set) var entries: [String: Entry] = [:]

    init(policy: Policy = .init()) { self.policy = policy }

    /// Seconds `address` must still wait before a connection from it is
    /// accepted, or nil when it may connect now.
    func retryAfter(_ address: String, now: Date) -> TimeInterval? {
        guard let e = entries[address], e.failures >= policy.freeFailures,
              now.timeIntervalSince(e.last) < policy.forgetAfter else { return nil }
        let wait = min(policy.maxBackoff, policy.baseBackoff * pow(2, Double(e.failures - policy.freeFailures)))
        let left = e.last.addingTimeInterval(wait).timeIntervalSince(now)
        return left > 0 ? left : nil
    }

    func failures(_ address: String) -> Int { entries[address]?.failures ?? 0 }

    /// Records one failed handshake from `address`.
    mutating func recordFailure(_ address: String, now: Date) {
        entries = entries.filter { now.timeIntervalSince($0.value.last) < policy.forgetAfter }
        var e = entries[address] ?? Entry(failures: 0, last: now)
        e.failures += 1
        e.last = now
        entries[address] = e
        if entries.count > policy.maxTrackedAddresses,
           let stalest = entries.min(by: { $0.value.last < $1.value.last })?.key {
            entries.removeValue(forKey: stalest)
        }
    }

    /// A handshake from `address` completed with one of the table's keys.
    mutating func recordSuccess(_ address: String) {
        entries.removeValue(forKey: address)
    }
}
