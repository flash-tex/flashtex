import AppKit

/// Weak references to the objects a hosted test built, checked after the
/// test let go of them: every one must deallocate (no retain cycle, no
/// observer, timer or display link keeping it alive).
///
/// AppKit and SwiftUI release a closed window's objects over the next few
/// run-loop turns (deferred layout, autoreleased views, the window's own
/// close sequence), so ``awaitReleased(timeout:)`` turns the main run loop
/// in its own autorelease pool until every reference is nil or the bounded
/// wait ends. It never sleeps without turning the run loop, so the result
/// does not depend on how loaded the machine is, only on whether something
/// still holds the object.
@MainActor
public final class DeallocationProbe {
    private struct Entry {
        let label: String
        weak var object: AnyObject?
        /// Recorded when tracked: a nil entry later means "released", not "never set".
        let typeName: String
    }

    private var entries: [Entry] = []

    public init() {}

    /// Tracks `object` (weakly) under `label`.
    public func track(_ object: AnyObject?, _ label: String) {
        guard let object else { return }
        entries.append(Entry(label: label, object: object, typeName: String(describing: type(of: object))))
    }

    /// How many objects are tracked.
    public var count: Int { entries.count }

    /// The labels (and types) of the tracked objects still alive.
    public var alive: [String] {
        entries.filter { $0.object != nil }.map { "\($0.label) (\($0.typeName))" }
    }

    /// Turns the main run loop until every tracked object is gone or
    /// `timeout` elapses. Returns the ones still alive (empty: all released).
    ///
    /// Synchronous on purpose: a caller that suspended (`await`) would let
    /// XCTest's own run loop run the app's main-thread work, outside any
    /// autorelease pool the test controls, and an object autoreleased there
    /// stays alive until the whole test ends. Every turn here is in its own pool.
    @discardableResult
    public func waitForRelease(timeout: TimeInterval = 10) -> [String] {
        let deadline = Date().addingTimeInterval(timeout)
        repeat {
            DeallocationProbe.turnRunLoop()
            if alive.isEmpty { return [] }
        } while Date() < deadline
        return alive
    }

    /// One short turn of the main run loop inside an autorelease pool.
    public static func turnRunLoop(_ seconds: TimeInterval = 0.02) {
        autoreleasepool {
            _ = RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(seconds))
        }
    }

    /// Whether process `pid` has exited (and been reaped or is a zombie of
    /// another parent): `kill(pid, 0)` fails with ESRCH.
    public static func processGone(_ pid: Int32) -> Bool {
        kill(pid, 0) != 0 && errno == ESRCH
    }

    /// Waits (turning the run loop) until `pid` has exited; true when it has.
    public static func waitForExit(_ pid: Int32, timeout: TimeInterval = 15) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        repeat {
            if processGone(pid) { return true }
            turnRunLoop(0.05)
        } while Date() < deadline
        return processGone(pid)
    }

    /// `leaks(1)` run on this process: the report's lines, or nil when the
    /// tool is missing or cannot examine the process (its exit status is 0
    /// for no leaks, 1 for leaks, anything else for a failure).
    ///
    /// Weak references only see objects the test can name. Memory nothing
    /// references any more (an object an initializer made and dropped, a
    /// retain cycle no root reaches) is what `leaks` finds instead.
    public static func leaksReport(timeout: TimeInterval = 600) -> [String]? {
        let tool = URL(fileURLWithPath: "/usr/bin/leaks")
        guard FileManager.default.isExecutableFile(atPath: tool.path) else { return nil }
        let p = Process()
        p.executableURL = tool
        p.arguments = ["\(getpid())"]
        let out = Pipe()
        p.standardOutput = out
        p.standardError = FileHandle.nullDevice
        guard (try? p.run()) != nil else { return nil }
        // Read while it runs: a full pipe would block it.
        var data = Data()
        let reader = DispatchQueue(label: "leaks-report")
        let done = DispatchSemaphore(value: 0)
        reader.async { data = out.fileHandleForReading.readDataToEndOfFile(); done.signal() }
        guard done.wait(timeout: .now() + timeout) == .success else { p.terminate(); return nil }
        p.waitUntilExit()
        guard p.terminationStatus == 0 || p.terminationStatus == 1 else { return nil }
        return String(decoding: data, as: UTF8.self).split(separator: "\n").map(String.init)
    }

    /// The lines of a `leaks` report that name one of `types` as leaked
    /// (`<Type 0x…>` or `<Module.Type 0x…>` in a leak tree).
    public static func leakedLines(_ report: [String], types: [String]) -> [String] {
        report.filter { line in types.contains { line.contains("<\($0) 0x") || line.contains(".\($0) 0x") } }
    }

    /// Turns the run loop (each turn in its own autorelease pool) until
    /// `condition` holds or `timeout` elapses; returns whether it held.
    public static func turn(until condition: () -> Bool, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while !condition() {
            if Date() > deadline { return false }
            turnRunLoop(0.05)
        }
        return true
    }
}
