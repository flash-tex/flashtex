import Foundation

/// Transport counters the Nearby window shows on one line, derived from the
/// listener's events only (no extra wiring in the transport). Cumulative for
/// the app run across listener restarts; `reset()` starts over.
struct NearbyTransportMetrics: Equatable {
    /// Capture frames the listener answered: accepted + refused + duplicates.
    var frames = 0
    var accepted = 0
    var refused = 0
    var duplicates = 0
    /// Authenticated connections that completed `hello`.
    var sessions = 0
    /// Hellos from a pairing already seen since the last reset.
    var reconnects = 0
    /// Connections closed for any reason (both peers, deadlines, caps).
    var closed = 0
    /// Closes caused by a handshake/hello/frame deadline.
    var timedOut = 0
    /// `.ready` beyond the first: key-table restarts and port recoveries.
    var listenerRestarts = 0
    private var started = false
    private var seenPairIds: Set<String> = []

    mutating func record(_ event: NearbyListener.Event) {
        switch event {
        case .ready:
            if started { listenerRestarts += 1 } else { started = true }
        case .hello(let pairId, _, _):
            sessions += 1
            if !seenPairIds.insert(pairId).inserted { reconnects += 1 }
        case .capture:
            frames += 1; accepted += 1
        case .captureRefused:
            frames += 1; refused += 1
        case .captureDuplicate:
            frames += 1; duplicates += 1
        case .connectionClosed(_, let reason):
            closed += 1
            if reason.contains("timed out") { timedOut += 1 }
        case .failed, .stopped, .connectionOpened, .receiving, .handshakeRefused, .pairingLockedOut:
            break
        }
    }

    /// Zeroes the counters; what the listener already knows (it is running,
    /// which pairings it has seen) is kept so the next `.ready` still counts
    /// as a restart and a returning pairing as a reconnect.
    mutating func reset() {
        frames = 0; accepted = 0; refused = 0; duplicates = 0
        sessions = 0; reconnects = 0; closed = 0; timedOut = 0; listenerRestarts = 0
    }

    /// The window's line, e.g. `frames 12 (2 refused, 3 duplicates) · sessions 4 (2 reconnects) · closed 3 (1 timed out) · restarts 1`.
    var line: String {
        var parts = ["frames \(frames) (\(refused) refused, \(duplicates) duplicate\(duplicates == 1 ? "" : "s"))",
                     "sessions \(sessions) (\(reconnects) reconnect\(reconnects == 1 ? "" : "s"))",
                     "closed \(closed) (\(timedOut) timed out)"]
        if listenerRestarts > 0 { parts.append("restarts \(listenerRestarts)") }
        return parts.joined(separator: " · ")
    }
}
