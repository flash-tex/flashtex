import Foundation

/// Polls `cond` on the main actor until it holds or `timeout` passes, for
/// state a main-queue hop sets later (an `asyncAfter`, a deferred editor
/// swap). A fixed sleep before the assertion assumes the hop has run, which a
/// loaded runner does not promise. This only waits: the caller asserts the
/// state afterwards, so a timeout still fails with the test's own message.
@MainActor
func eventually(timeout: TimeInterval = 10, _ cond: () -> Bool) async throws {
    let deadline = Date().addingTimeInterval(timeout)
    while !cond(), Date() < deadline {
        try await Task.sleep(nanoseconds: 10_000_000)
    }
}

/// How long a Nearby test waits for a loopback TLS listener or connection to
/// reach a state (ready, closed, failed, N lines). A bound on a real network
/// stack and its queues, not a measurement: a loaded runner stretches a
/// handshake well past the former 5 s without anything being wrong, and a
/// state that never comes still fails, only later.
let loopbackWait: TimeInterval = 20
