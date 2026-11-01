import Foundation

/// Main-thread accounting for APP-EDITOR-INSTANT (docs/evidence/editor-instant-2026-10-06):
/// the time the main thread spends in named sections (host events, the
/// preview's layout, the chrome refresh, the editor's own change handling)
/// and when the editor's text view last drew. `EditorInstantTests` and the
/// cold-typing bench (`FLASHTEX_V3_BENCH_COLD=1`) read it. Off, each call is
/// one Bool test.
@MainActor
enum MainThreadProbe {
    static var isRecording = false

    struct Bucket: Codable {
        var count = 0
        var totalNs: UInt64 = 0
        var maxNs: UInt64 = 0
        /// Each section's duration (ns), in order: the tests take percentiles.
        var samples: [UInt64] = []
        mutating func add(_ ns: UInt64) {
            count += 1; totalNs &+= ns; maxNs = max(maxNs, ns)
            if samples.count < 100_000 { samples.append(ns) }
        }
    }

    private(set) static var buckets: [String: Bucket] = [:]
    /// When the editor's text view finished a draw (`CompletingTextView.draw`), CLOCK_UPTIME_RAW ns.
    private(set) static var editorDraws: [UInt64] = []

    static func start() { buckets = [:]; editorDraws = []; isRecording = true; recordingFlag = true }
    static func stop() { isRecording = false; recordingFlag = false }

    /// `isRecording`, readable off the main actor (PerfSignposts' sections).
    nonisolated(unsafe) static var recordingFlag = false

    /// A section timed outside the main actor's view (PerfSignposts.interval on the main thread).
    nonisolated static func record(_ name: StaticString, since t0: UInt64) {
        let d = MonotonicClock.nowNs() &- t0
        MainActor.assumeIsolated {
            guard isRecording else { return }
            buckets["sp.\(name)", default: Bucket()].add(d)
        }
    }

    /// A section's start (0 when not recording); pass it to `end`.
    static func begin() -> UInt64 { isRecording ? MonotonicClock.nowNs() : 0 }

    static func end(_ name: StaticString, _ t0: UInt64) {
        guard isRecording, t0 != 0 else { return }
        buckets["\(name)", default: Bucket()].add(MonotonicClock.nowNs() &- t0)
    }

    /// Times `body` under `name` while recording.
    static func time<T>(_ name: StaticString, _ body: () throws -> T) rethrows -> T {
        guard isRecording else { return try body() }
        let t0 = MonotonicClock.nowNs()
        defer { end(name, t0) }
        return try body()
    }

    /// The editor's text view drew (its glyphs are in the layer's backing store).
    static func editorDrew() {
        guard isRecording else { return }
        if editorDraws.count < 100_000 { editorDraws.append(MonotonicClock.nowNs()) }
    }
}
