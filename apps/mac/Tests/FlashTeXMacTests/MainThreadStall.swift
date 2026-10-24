import AppKit
import Foundation
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// The one-frame gate's measuring tools, shared by the real-document typing gate
/// (EngineV3RealDocTypingTests, lane REAL-DOC-GATE) and lane EDITOR-FREEZE's narrower
/// tests: DESIGN.md §1.2, "the editor is a text editor first: key → glyph drawn in the
/// editor ≤ 1 frame … independent of document size, compile state and host traffic. No
/// preview work ever blocks typing."
///
/// - `MainThreadStallMonitor` times every main run-loop turn (after-waiting → before-waiting:
///   the main thread busy without a break) and pings main every 2 ms from another thread (how
///   long an event waits for it). A turn longer than a frame is a stall: a key arriving then
///   waits behind it.
/// - `ClockTypist` types on a clock, as a person does: key k is posted to the main run loop
///   from another thread at k × interval, whatever main is doing, and each key's handler
///   checks the editor's text storage took the edit. Its time is posted → in the text storage.
///
/// The gate's budget is one frame, `MainThreadStall.frameMs` (16 ms: 60 Hz, the bound the
/// owner set for a stall; the editor's drawn-glyph target is 8 ms at 120 Hz, measured by
/// EditorInstantTests).
enum MainThreadStall {
    static let frameMs = 16.0

    static func nowNs() -> UInt64 { clock_gettime_nsec_np(CLOCK_UPTIME_RAW) }

    static func onMain(_ block: @escaping @MainActor () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue) { MainActor.assumeIsolated { block() } }
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }

    static func stats(_ v: [Double]) -> [String: Double] {
        guard !v.isEmpty else { return ["n": 0] }
        let s = v.sorted()
        func p(_ q: Double) -> Double { s[min(s.count - 1, max(0, Int((q * Double(s.count)).rounded(.up)) - 1))] }
        return ["n": Double(s.count), "p50": p(0.5), "p95": p(0.95), "p99": p(0.99), "max": s.last!]
    }

    /// The hosted window with the whole app (ContentView) on `model`, and its editor's text view.
    @MainActor
    static func hostEditor(_ model: ShellModel, size: NSSize = NSSize(width: 1400, height: 900)) async throws -> (NSWindow, NSTextView) {
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(origin: .zero, size: size), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        let hosting = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.contentView = hosting
        window.orderFrontRegardless()
        hosting.layoutSubtreeIfNeeded()
        let deadline = Date().addingTimeInterval(60)
        while Date() < deadline {
            if let tv = TypingBenchDriver.findTextView(in: [hosting]) { return (window, tv) }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("no editor text view in the hosted window")
        throw CancellationError()
    }
}

/// Times main's run-loop turns and its responsiveness to a 2 ms ping (see `MainThreadStall`).
final class MainThreadStallMonitor: @unchecked Sendable {
    /// One turn: when it began (ns, `MainThreadStall.nowNs`) and how long main was busy (ms).
    struct Turn { var startNs: UInt64; var ms: Double }

    struct Report {
        var turns = 0
        /// Turns over the frame budget, longest first (at most 50).
        var stalls: [Turn] = []
        var longestTurnMs = 0.0
        var pingMs: [Double] = []
        var startNs: UInt64 = 0
        func over(_ ms: Double) -> [Turn] { stalls.filter { $0.ms > ms } }
        var summary: [String: Any] {
            ["turns": turns, "longest_turn_ms": longestTurnMs,
             "stalls_over_frame": stalls.map { ["at_ms": Double($0.startNs &- startNs) / 1e6, "ms": $0.ms] },
             "ping_ms": MainThreadStall.stats(pingMs)]
        }
    }

    private let lock = NSLock()
    private var turnStart: UInt64 = 0
    private var turnCount = 0
    private var long: [Turn] = []
    private var longest = 0.0
    private var pings: [Double] = []
    private var stopped = false
    private var observer: CFRunLoopObserver?
    private var startNs: UInt64 = 0

    /// Starts timing: call on main.
    @MainActor func start() {
        startNs = MainThreadStall.nowNs()
        turnStart = startNs
        let obs = CFRunLoopObserverCreateWithHandler(nil, CFRunLoopActivity.afterWaiting.rawValue | CFRunLoopActivity.beforeWaiting.rawValue,
                                                     true, 0) { [weak self] _, activity in
            guard let self else { return }
            let now = MainThreadStall.nowNs()
            self.lock.lock(); defer { self.lock.unlock() }
            if activity == .afterWaiting {
                self.turnStart = now
            } else {
                self.endTurn(now)
            }
        }
        observer = obs
        CFRunLoopAddObserver(CFRunLoopGetMain(), obs, .commonModes)
        let ping = Thread { [weak self] in
            while let self, !self.isStopped {
                let t0 = MainThreadStall.nowNs()
                MainThreadStall.onMain { [weak self] in
                    let d = Double(MainThreadStall.nowNs() &- t0) / 1e6
                    self?.lock.lock(); if (self?.pings.count ?? 0) < 1_000_000 { self?.pings.append(d) }; self?.lock.unlock()
                }
                usleep(2000)
            }
        }
        ping.qualityOfService = .userInteractive
        ping.start()
    }

    private var isStopped: Bool { lock.lock(); defer { lock.unlock() }; return stopped }

    /// (lock held)
    private func endTurn(_ now: UInt64) {
        let ms = Double(now &- turnStart) / 1e6
        turnCount += 1
        longest = max(longest, ms)
        if ms > MainThreadStall.frameMs {
            long.append(Turn(startNs: turnStart, ms: ms))
            if long.count > 200 { long.sort { $0.ms > $1.ms }; long.removeLast(long.count - 50) }
        }
        turnStart = now
    }

    /// Stops timing (on main); the turn in progress counts up to now.
    @MainActor func stop() -> Report {
        if let observer { CFRunLoopRemoveObserver(CFRunLoopGetMain(), observer, .commonModes) }
        observer = nil
        lock.lock(); defer { lock.unlock() }
        stopped = true
        endTurn(MainThreadStall.nowNs())
        return Report(turns: turnCount, stalls: Array(long.sorted { $0.ms > $1.ms }.prefix(50)), longestTurnMs: longest,
                      pingMs: pings, startNs: startNs)
    }
}

/// Types on a clock into an `NSTextView` (see `MainThreadStall`): a letter inserted, then
/// deleted, alternately, so the text ends as it began.
final class ClockTypist: @unchecked Sendable {
    struct Key { var postedNs: UInt64; var handledNs: UInt64; var inStorage: Bool
        var ms: Double { Double(handledNs &- postedNs) / 1e6 } }

    private let lock = NSLock()
    private var keys: [Key] = []
    private var stopped = false

    /// Types `count` keys `intervalMs` apart into `tv` and returns once all were handled, or
    /// after `timeout` seconds (a main thread too busy to take them: what is measured). The
    /// keys handled, in order; `stamp` runs on main before each (e.g. the session's keystroke clock).
    @MainActor
    func type(_ count: Int, every intervalMs: UInt32 = 50, into tv: NSTextView, timeout: TimeInterval,
              stamp: (@MainActor (UInt64) -> Void)? = nil) async -> [Key] {
        let tvRef = Unmanaged.passUnretained(tv)
        let t = Thread { [weak self] in
            for k in 0 ..< count {
                usleep(intervalMs * 1000)
                guard let self, !self.isStopped else { return }
                let posted = MainThreadStall.nowNs()
                MainThreadStall.onMain { [weak self] in
                    let tv = tvRef.takeUnretainedValue()
                    stamp?(posted)
                    let before = tv.textStorage?.length ?? 0
                    let at = tv.selectedRange().location
                    if k % 2 == 0 { tv.insertText("x", replacementRange: tv.selectedRange()) } else { tv.deleteBackward(nil) }
                    let storage = tv.textStorage
                    let ok: Bool
                    if k % 2 == 0 {
                        ok = storage?.length == before + 1 && at < (storage?.length ?? 0)
                            && (storage?.string as NSString?)?.substring(with: NSRange(location: at, length: 1)) == "x"
                    } else {
                        ok = storage?.length == before - 1
                    }
                    let handled = MainThreadStall.nowNs()
                    self?.lock.lock(); self?.keys.append(Key(postedNs: posted, handledNs: handled, inStorage: ok)); self?.lock.unlock()
                }
            }
        }
        t.qualityOfService = .userInteractive
        t.start()
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            lock.lock(); let n = keys.count; lock.unlock()
            if n >= count { break }
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
        lock.lock(); defer { lock.unlock() }
        stopped = true
        return keys
    }

    private var isStopped: Bool { lock.lock(); defer { lock.unlock() }; return stopped }
}
