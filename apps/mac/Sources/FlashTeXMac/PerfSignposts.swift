import Foundation
import os

/// `os_signpost` intervals and events along the keystroke -> pixels path, for
/// Instruments (`xcrun xctrace record --template 'Time Profiler'` shows them in
/// the "Points of Interest" / os_signpost lanes; subsystem
/// `tech.jay3332.flashtex.mac`, category `PointsOfInterest`).
///
/// Debug flag: `FLASHTEX_SIGNPOSTS=1`. Without it every signposter is
/// `OSSignposter.disabled`, whose calls return immediately (no string
/// formatting: the messages are `OSLogMessage` interpolations evaluated only
/// when enabled), so the hooks cost nothing in normal use.
///
/// Names (one per stage; APP-PERF-AUDIT, docs/evidence/app-perf-2026-09-30):
///   keyDown            event: the key event reached the window (HID time is in the log line)
///   editorChange       interval: NSTextView delegate `textDidChange` (highlight flush, gutter, binding push)
///     syntaxFlush      interval: the incremental highlighter repaints the changed lines
///     commitUserChange interval: auto-close, linked `\end`, buffer copy, binding push, brace highlight
///     bufferCopy       interval: `SourceEditorView.nativeText` (the whole buffer as a native String)
///     braceHighlight   interval: delimiter / `\begin`–`\end` pair highlight
///   modelUpdate        interval: `ShellModel.updateActiveText` (revision bump, schedules)
///   compileSend        event: runtime-v1 request written
///   resultApplied      event: compile result applied on main
///   renderPass         interval: a preview render pass began -> the paint point for its revision
///   pageDraw           event: one page canvas drew / page bitmap installed
enum PerfSignposts {
    static let enabled = ProcessInfo.processInfo.environment["FLASHTEX_SIGNPOSTS"] == "1"

    static let signposter: OSSignposter = enabled
        ? OSSignposter(subsystem: "tech.jay3332.flashtex.mac", category: .pointsOfInterest)
        : .disabled

    @inline(__always)
    static func event(_ name: StaticString) {
        guard enabled else { return }
        signposter.emitEvent(name)
    }

    @inline(__always)
    static func event(_ name: StaticString, _ value: Int) {
        guard enabled else { return }
        signposter.emitEvent(name, "\(value)")
    }

    /// Runs `body` inside a signpost interval (just `body()` when disabled).
    @inline(__always)
    static func interval<T>(_ name: StaticString, _ body: () throws -> T) rethrows -> T {
        guard enabled else { return try body() }
        let state = signposter.beginInterval(name, id: signposter.makeSignpostID())
        defer { signposter.endInterval(name, state) }
        return try body()
    }

    static func begin(_ name: StaticString, _ value: Int) -> OSSignpostIntervalState? {
        guard enabled else { return nil }
        return signposter.beginInterval(name, id: signposter.makeSignpostID(), "\(value)")
    }

    static func end(_ name: StaticString, _ state: OSSignpostIntervalState?) {
        guard let state else { return }
        signposter.endInterval(name, state)
    }
}
