import Dispatch
import Foundation
import FlashTeXPreviewV3

/// The system's memory-pressure events (mem-research §2.9 #11): one
/// `DispatchSource.makeMemoryPressureSource` for the app, started with the
/// first engine-v3 session. On a warning or a critical event it drops what
/// is cheap to build again: the decoded images of the shared resource cache
/// no page holds, and each running session's glyph indexes
/// (`EngineV3Session.trimMemory`), and each session asks its host to trim
/// (`TRIM`, only to a host offering `trim-v1`). Nothing on screen goes, and
/// the performance mode does not change. It costs nothing until the
/// system asks.
@MainActor
final class EngineV3MemoryPressure {
    enum Level: String, Sendable { case warning, critical }

    static let shared = EngineV3MemoryPressure(listens: true)

    /// Whether this instance installs the dispatch source (tests use one
    /// that does not, and inject events with `handle`).
    private let listens: Bool
    private var source: DispatchSourceMemoryPressure?
    private var sessions: [EngineV3WeakRef] = []
    /// Events applied (tests, evidence).
    private(set) var events = 0

    init(listens: Bool) { self.listens = listens }

    /// A running session: trimmed on each event until `unregister`.
    func register(_ s: EngineV3Session) {
        sessions.removeAll { $0.value == nil || $0.value === s }
        sessions.append(EngineV3WeakRef(s))
        if listens, source == nil { listen() }
    }

    func unregister(_ s: EngineV3Session) {
        sessions.removeAll { $0.value == nil || $0.value === s }
    }

    /// Sessions registered now (tests).
    var registered: Int { sessions.filter { $0.value != nil }.count }

    private func listen() {
        let src = DispatchSource.makeMemoryPressureSource(eventMask: [.warning, .critical], queue: .main)
        src.setEventHandler { [weak self] in
            MainActor.assumeIsolated {
                guard let self, let e = self.source?.data else { return }
                if e.contains(.critical) { self.handle(.critical) } else if e.contains(.warning) { self.handle(.warning) }
            }
        }
        src.activate()
        source = src
    }

    /// Applies one event (the source's handler; tests call it directly).
    func handle(_ level: Level) {
        events &+= 1
        FlashTeXLog.write("engine-v3: memory pressure \(level.rawValue): trimming caches")
        DL3ResourceCache.shared.trimImages()
        for s in sessions.compactMap(\.value) { s.trimMemory(level) }
    }
}
