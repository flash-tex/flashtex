import AppKit
import Darwin
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Several project windows at once, shown as the app shows them
/// (`ProjectWindowRoot` in real windows, ProjectWindows.swift): each has its
/// own editor, model and engine host; typing in one costs the others
/// nothing; closing one lets go of it and its host and leaves the others
/// running. Synchronous, turning the main run loop itself, as
/// DocumentDeallocationTests (whose release checks this reuses).
@MainActor
final class ProjectWindowsHostedTests: XCTestCase {
    private var dir: URL!
    private var env = EnvironmentOverride()
    private var shown: [NSWindowController] = []

    override func setUp() {
        OwnerStateGuard.install()
        HostedWindowSupport.prepare()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("windows-hosted-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    override func tearDown() {
        ViewBodyProbe.stop()
        for c in shown { c.close() }
        shown = []
        DeallocationProbe.turnRunLoop(0.1)
        env.restore()
        if let dir { try? FileManager.default.removeItem(at: dir) }
    }

    struct Timeout: Error {}

    private func waitUntil(_ what: String, timeout: TimeInterval = 30, file: StaticString = #filePath, line: UInt = #line, _ cond: () -> Bool) throws {
        guard DeallocationProbe.turn(until: cond, timeout: timeout) else {
            XCTFail("timeout waiting for \(what)", file: file, line: line)
            throw Timeout()
        }
    }

    private func write(_ path: String, _ body: String) throws -> URL {
        let url = dir.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try "\\documentclass{article}\n\\begin{document}\n\(body)\n\\end{document}\n".write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    private func makeWindows() -> ProjectWindows {
        let w = ProjectWindows()
        w.makeModel = { _ in ShellModel(startup: .untitledDocument, launchHooks: false) }
        return w
    }

    /// Shows window `id` of `windows` as the app's WindowGroup does: its root
    /// view in a window under a controller. Returns the editor.
    private func show(_ id: ProjectWindowID, of windows: ProjectWindows) throws -> NSTextView {
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 1200, height: 800), styleMask: [.titled, .closable, .resizable])
        window.isReleasedWhenClosed = false
        let hosting = NSHostingView(rootView: ProjectWindowRoot(id: id, windows: windows).environmentObject(NearbyState()))
        hosting.sizingOptions = []
        window.contentView = hosting
        let controller = NSWindowController(window: window)
        shown.append(controller)
        window.orderFrontRegardless()
        var found: NSTextView?
        try waitUntil("the editor of window \(shown.count)") { found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil }
        return try XCTUnwrap(found)
    }

    private func close(_ id: ProjectWindowID, of windows: ProjectWindows) {
        guard let w = windows.entries.first(where: { $0.id == id })?.window,
              let i = shown.firstIndex(where: { $0.window === w }) else { return XCTFail("window not shown") }
        shown.remove(at: i).close()
    }

    /// Types `n` characters into `tv` at its caret, one run-loop turn apart (as KeystrokeInvalidationTests).
    private func type(_ n: Int, into tv: NSTextView) {
        for _ in 0 ..< n {
            tv.insertText("x", replacementRange: tv.selectedRange())
            run(0.03)
        }
    }

    /// Turns the main run loop for `seconds` (a turn can return early, when a source fires).
    private func run(_ seconds: TimeInterval) {
        let end = Date().addingTimeInterval(seconds)
        while Date() < end { DeallocationProbe.turnRunLoop(min(0.05, max(0.001, end.timeIntervalSinceNow))) }
    }

    private func settle() { run(0.6) } // past the chrome's and outline's debounces

    private func caretInBody(_ tv: NSTextView, _ needle: String) {
        tv.window?.makeFirstResponder(tv)
        let r = (tv.string as NSString).range(of: needle)
        tv.setSelectedRange(NSRange(location: r.location == NSNotFound ? 0 : r.location, length: 0))
    }

    // MARK: windows, without a host

    func testEachWindowHasItsOwnProjectAndClosingOneLeavesTheOthers() throws {
        env.set("FLASHTEX_HOST", "none")
        let windows = makeWindows()
        let id1 = ProjectWindowID()
        let probe = DeallocationProbe()
        // Window 1's objects are named only in here, so nothing of the test holds them.
        let (id2, b, tvB): (ProjectWindowID, ShellModel, NSTextView) = try autoreleasepool {
            let a = windows.model(for: id1)
            windows.open(try write("a/main.tex", "Alpha project."), from: a)
            guard case .openedNew(let id2) = windows.open(try write("b/paper.tex", "Beta project."), from: a).result else { throw Timeout() }
            let b = windows.model(for: id2)
            let tvA = try show(id1, of: windows), tvB = try show(id2, of: windows)
            XCTAssertTrue(tvA.string.contains("Alpha project.") && tvB.string.contains("Beta project."), "each window's editor shows its own project")
            XCTAssertTrue(windows.entries.first { $0.id == id1 }?.window === tvA.window, "the window told the list it appeared")
            XCTAssertFalse(windows.entries.first { $0.id == id1 }?.window?.isRestorable ?? true, "AppKit's own restoration is off: the app restores")

            caretInBody(tvA, "Alpha")
            let revB = b.editorRevision, textB = b.activeText
            type(3, into: tvA)
            XCTAssertTrue(a.activeText.contains("xxxAlpha"))
            XCTAssertEqual(b.editorRevision, revB, "typing in one window is no edit of another")
            XCTAssertEqual(b.activeText, textB)

            probe.track(a, "window 1's ShellModel")
            probe.track(a.engineV3, "window 1's EngineV3Session")
            probe.track(a.project, "window 1's ProjectDocuments")
            probe.track(tvA, "window 1's editor")
            probe.track(tvA.window, "window 1's NSWindow")
            close(id1, of: windows)
            return (id2, b, tvB)
        }
        XCTAssertEqual(windows.entries.map(\.id), [id2], "the closed window left the list")
        XCTAssertEqual(probe.waitForRelease(timeout: 20), [], "still alive after its window closed")
        XCTAssertTrue(windows.commandTarget(focused: nil) === b, "the menus now act on the window left")
        caretInBody(tvB, "Beta")
        type(2, into: tvB)
        XCTAssertTrue(b.activeText.contains("xxBeta"), "the other window still edits")
    }

    /// The main-thread budget with several projects open (#1900's
    /// measures): a keystroke in one window re-evaluates no more SwiftUI
    /// views than with that window alone, and the run loop's waits stay
    /// within the budget (`FLASHTEX_EDITOR_KEY_BUDGET_MS`, default 100 ms
    /// as in EditorInstantTests; the target is a 16 ms frame and the
    /// measured numbers are printed as one `MultiWindow:` line).
    func testTypingInOneWindowCostsTheOtherWindowsNothing() throws {
        env.set("FLASHTEX_HOST", "none")
        let windows = makeWindows()
        let id1 = ProjectWindowID()
        let a = windows.model(for: id1)
        let body = (0 ..< 40).map { "Paragraph \($0) with some words to fill a line or two of the page." }.joined(separator: "\n\n")
        windows.open(try write("one/main.tex", "\\section{One}\nCaret here.\n\n" + body), from: a)
        let tv = try show(id1, of: windows)
        caretInBody(tv, "Caret here")
        type(1, into: tv) // the first edit makes the document dirty (tab, tree, status bar): settle before counting
        settle()

        var keyTimes: [UInt64] = []
        func measure(_ keys: Int) -> (bodies: [String: Int], waits: [Double]) {
            let ping = RunLoopLatencyProbe()
            ViewBodyProbe.start()
            ping.start()
            keyTimes = [MonotonicClock.nowNs()]
            for _ in 0 ..< keys {
                tv.insertText("x", replacementRange: tv.selectedRange())
                keyTimes.append(MonotonicClock.nowNs())
                run(0.03)
            }
            settle()
            let waits = ping.stop()
            ViewBodyProbe.stop()
            return (ViewBodyProbe.counts, waits)
        }
        let alone = measure(8)
        do {
            let t0 = keyTimes[0]
            func ms(_ ns: UInt64) -> String { String(format: "%.1f", Double(ns &- t0) / 1e6) }
            print("MultiWindowDiag: alone keys at \(keyTimes.map(ms)) ms; w1 \(ViewBodyProbe.counts(of: a)); events \(ViewBodyProbe.events.map { "\($0.view)@\(ms($0.ns))\($0.model == ObjectIdentifier(a) ? "" : "(other)")" })")
        }

        // Two more projects, each in its own window, both on screen.
        var others: [ShellModel] = []
        for name in ["two", "three"] {
            guard case .openedNew(let id) = windows.open(try write("\(name)/main.tex", "\\section{\(name)}\n" + body), from: a).result else { return XCTFail(name) }
            others.append(windows.model(for: id))
            _ = try show(id, of: windows)
        }
        tv.window?.makeFirstResponder(tv)
        // The new windows' own first work (discovery, word counts, the
        // companion's state, seconds after they appear) is not typing: wait
        // until a whole second passes with no body evaluated (bounded).
        let quietBy = Date().addingTimeInterval(20)
        var quiet = 0, quietRounds = 0
        while quiet < 2, Date() < quietBy {
            ViewBodyProbe.start(); run(1); ViewBodyProbe.stop()
            quietRounds += 1
            if !ViewBodyProbe.counts.isEmpty { print("MultiWindowDiag: quiet round \(quietRounds): \(ViewBodyProbe.counts)") }
            quiet = ViewBodyProbe.counts.isEmpty ? quiet + 1 : 0
        }
        print("MultiWindowDiag: quiet after \(quietRounds) rounds (quiet=\(quiet)); key window \(String(describing: NSApp.keyWindow)); tv window key \(tv.window?.isKeyWindow ?? false) main \(tv.window?.isMainWindow ?? false); first responder is tv \(tv.window?.firstResponder === tv); policy \(NSApp.activationPolicy().rawValue) active \(NSApp.isActive)")
        ViewBodyProbe.recordsStacks = true
        let three = measure(8)
        ViewBodyProbe.recordsStacks = false
        do {
            let names: [ObjectIdentifier: String] = [ObjectIdentifier(a): "w1", ObjectIdentifier(others[0]): "w2", ObjectIdentifier(others[1]): "w3"]
            let t0 = keyTimes[0]
            func ms(_ ns: UInt64) -> String { String(format: "%.1f", Double(ns &- t0) / 1e6) }
            print("MultiWindowDiag: keys at \(keyTimes.map(ms)) ms; w1 \(ViewBodyProbe.counts(of: a)) w2 \(ViewBodyProbe.counts(of: others[0])) w3 \(ViewBodyProbe.counts(of: others[1]))")
            for (i, e) in ViewBodyProbe.events.enumerated() {
                let who = e.model.map { names[$0] ?? "other-\($0.hashValue)" } ?? "?"
                print("MultiWindowDiag: event \(i) \(e.view) \(who) at \(ms(e.ns)) ms")
                if e.view == "EditorPane" || e.view == "ProjectSection" {
                    for f in ViewBodyProbe.stacks[i].prefix(70) { print("MultiWindowDiag:   \(f)") }
                }
            }
        }

        XCTAssertEqual(windows.entries.count, 3)
        for (view, n) in three.bodies {
            XCTAssertLessThanOrEqual(n, (alone.bodies[view] ?? 0) + 1, "\(view) re-evaluated more with two other windows open (alone \(alone.bodies), three \(three.bodies))")
        }
        let budget = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_KEY_BUDGET_MS"].flatMap(Double.init) ?? 100
        let s1 = RunLoopLatencyProbe.stats(alone.waits), s3 = RunLoopLatencyProbe.stats(three.waits)
        XCTAssertLessThanOrEqual(s3["p99"] ?? .infinity, budget, "run-loop wait p99 with three windows: \(s3)")
        let line: [String: Any] = ["alone": ["bodies": alone.bodies, "wait_ms": s1], "three_windows": ["bodies": three.bodies, "wait_ms": s3]]
        if let d = try? JSONSerialization.data(withJSONObject: line, options: [.sortedKeys]), let s = String(data: d, encoding: .utf8) { print("MultiWindow: \(s)") }
    }

    // MARK: with the engine

    /// Each window has its own `flashtex-host`: both compile, an edit in
    /// one compiles only there, and closing one ends its host while the
    /// other keeps compiling. The hosts' resident memory is printed
    /// (`MultiWindowHosts:`) for the design's memory budget.
    func testEachWindowHasItsOwnHostAndClosingOneEndsOnlyThatHost() throws {
        try EngineV3TestHost.require()
        let windows = makeWindows()
        let id1 = ProjectWindowID()
        let probe = DeallocationProbe()
        var pids: [Int32] = []
        defer { for pid in pids where !DeallocationProbe.processGone(pid) { kill(pid, SIGKILL) } } // never leave a host behind
        // Window a's objects are named only in here, so nothing of the test holds them.
        let b: ShellModel = try autoreleasepool {
            let a = windows.model(for: id1)
            windows.open(try write("a/main.tex", "\\section{A}\nAlpha."), from: a)
            guard case .openedNew(let id2) = windows.open(try write("b/main.tex", "\\section{B}\nBeta."), from: a).result else { throw Timeout() }
            let b = windows.model(for: id2)
            a.engineV3Enabled = true
            b.engineV3Enabled = true
            let tvA = try show(id1, of: windows)
            _ = try show(id2, of: windows)
            for (name, m) in [("a", a), ("b", b)] {
                let s = m.engineV3
                try waitUntil("\(name)'s first compile", timeout: 180) {
                    if case .failed(let why) = s.phase { XCTFail("\(name)'s host did not start: \(why)"); return true }
                    return s.phase == .ready && s.statusNote.hasPrefix("ok") && s.pageCount == 1 && !s.compiling
                }
                pids.append(try XCTUnwrap(s.hostPID, "\(name) has no host"))
            }
            XCTAssertNotEqual(pids[0], pids[1], "one host per project window")

            let doneB = b.engineV3.doneCount, doneA = a.engineV3.doneCount
            caretInBody(tvA, "Alpha")
            a.engineV3.nextKeystrokeNs = MonotonicClock.nowNs()
            tvA.insertText("Again. ", replacementRange: tvA.selectedRange())
            try waitUntil("a's edit compiled", timeout: 60) { a.engineV3.doneCount > doneA && !a.engineV3.compiling }
            run(0.5)
            XCTAssertEqual(b.engineV3.doneCount, doneB, "an edit in window a compiled in window b's host")
            print("MultiWindowHosts: {\"hosts\": \(pids.count), \"footprint_mb\": \(pids.map(Self.footprintMB))}")

            probe.track(a, "window a's ShellModel")
            probe.track(a.engineV3, "window a's EngineV3Session")
            probe.track(tvA, "window a's editor")
            close(id1, of: windows)
            return b
        }
        XCTAssertEqual(probe.waitForRelease(timeout: 30), [], "still alive after its window closed")
        XCTAssertTrue(DeallocationProbe.waitForExit(pids[0], timeout: 20), "window a's host is still running after its window closed")
        XCTAssertFalse(DeallocationProbe.processGone(pids[1]), "window b's host ended with window a")
        XCTAssertEqual(b.engineV3.hostPID, pids[1])
    }

    /// Closing a project's window and opening the project again in the
    /// same run reuses its project copy (EngineV3Session.end,
    /// EngineV3Mirror.release): the new window's session takes the copy
    /// over, with its `.aux` and paths, so the host starts from its S₀ and
    /// not from the format. The cold and warm first compiles are printed
    /// (`MultiWindowReopen:`).
    func testReopeningAClosedProjectInTheSameRunTakesItsCopyOver() throws {
        try EngineV3TestHost.require()
        let windows = makeWindows()
        let url = dir.appendingPathComponent("book/main.tex")
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try EditorInstantTests.book(sections: 120).write(to: url, atomically: true, encoding: .utf8)
        var pids: [Int32] = []
        defer { for pid in pids where !DeallocationProbe.processGone(pid) { kill(pid, SIGKILL) } }
        /// Opens the book in a window of its own, waits for its first compile: its copy and the time it took.
        func openAndCompile(_ name: String) throws -> (copy: URL, ms: Double, id: ProjectWindowID) {
            try autoreleasepool {
                let t0 = MonotonicClock.nowNs()
                let first = windows.entries.isEmpty ? windows.model(for: ProjectWindowID()) : nil
                let model = try XCTUnwrap(windows.open(url, from: first).model)
                let id = try XCTUnwrap(windows.entry(for: model)?.id)
                model.engineV3Enabled = true
                _ = try show(id, of: windows)
                let s = model.engineV3
                try waitUntil("the \(name) compile", timeout: 300) {
                    if case .failed(let why) = s.phase { XCTFail("the \(name) host did not start: \(why)"); return true }
                    return s.phase == .ready && s.doneCount > 0 && !s.compiling && s.pageCount > 1
                }
                pids.append(try XCTUnwrap(s.hostPID))
                return (try XCTUnwrap(s.projectCopy), Double(MonotonicClock.nowNs() &- t0) / 1e6, id)
            }
        }
        let cold = try openAndCompile("cold")
        let owner = cold.copy.deletingLastPathComponent().appendingPathComponent("owner")
        close(cold.id, of: windows)
        try waitUntil("the closed window's copy released", timeout: 30) {
            (try? String(contentsOf: owner, encoding: .utf8))?.hasSuffix(EngineV3Mirror.releasedSuffix) == true
        }
        XCTAssertTrue(DeallocationProbe.waitForExit(pids[0], timeout: 20), "the closed window's host still runs")
        let warm = try openAndCompile("warm")
        XCTAssertEqual(warm.copy.standardizedFileURL.path, cold.copy.standardizedFileURL.path, "the reopened project took its released copy over")
        XCTAssertEqual(try String(contentsOf: owner, encoding: .utf8), EngineV3.instanceOwner, "claimed again by this instance")
        print("MultiWindowReopen: {\"cold_ms\": \(Int(cold.ms)), \"warm_ms\": \(Int(warm.ms))}")
    }

    /// Physical footprint of `pid` (Activity Monitor's Memory) in MB, 0 if unknown.
    static func footprintMB(_ pid: Int32) -> Double {
        (Double(EngineV3PageCapture.footprint(pid: pid)) / 1_048_576 * 10).rounded() / 10
    }

    /// The memory budget of several large projects open at once
    /// (opt in: `FLASHTEX_MULTIWINDOW_LARGE=<windows>`, 2–4, with
    /// `FLASHTEX_MULTIWINDOW_SECTIONS`, default 300, about a page each):
    /// each window compiles its own book in its own host; once all are
    /// done, each host's and the app's footprint is printed as one
    /// `MultiWindowLarge:` line, and again after a memory-pressure warning
    /// (EngineV3MemoryPressure trims every session and asks every host to trim).
    func testSeveralLargeProjectsMemory() throws {
        guard let n = ProcessInfo.processInfo.environment["FLASHTEX_MULTIWINDOW_LARGE"].flatMap(Int.init), n > 0 else {
            throw XCTSkip("opt in with FLASHTEX_MULTIWINDOW_LARGE=<windows>")
        }
        try EngineV3TestHost.require()
        let sections = ProcessInfo.processInfo.environment["FLASHTEX_MULTIWINDOW_SECTIONS"].flatMap(Int.init) ?? 300
        let windows = makeWindows()
        var pids: [Int32] = []
        defer { for pid in pids where !DeallocationProbe.processGone(pid) { kill(pid, SIGKILL) } }
        var models: [ShellModel] = []
        let start = MonotonicClock.nowNs()
        for i in 0 ..< n {
            let url = dir.appendingPathComponent("book\(i)/main.tex")
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try (EditorInstantTests.book(sections: sections).replacingOccurrences(of: "\\begin{document}\n", with: "\\begin{document}\nBook \(i).\n"))
                .write(to: url, atomically: true, encoding: .utf8)
            let id = ProjectWindowID()
            let m = i == 0 ? windows.model(for: id) : nil
            let r = windows.open(url, from: m)
            let model = try XCTUnwrap(r.model)
            model.engineV3Enabled = true
            _ = try show(try XCTUnwrap(windows.entry(for: model)?.id), of: windows)
            models.append(model)
        }
        for (i, m) in models.enumerated() {
            let s = m.engineV3
            try waitUntil("book \(i)'s compile", timeout: 900) {
                if case .failed(let why) = s.phase { XCTFail("book \(i)'s host did not start: \(why)"); return true }
                return s.phase == .ready && s.doneCount > 0 && !s.compiling && s.pageCount > 1
            }
            pids.append(try XCTUnwrap(s.hostPID))
        }
        let seconds = Double(MonotonicClock.nowNs() &- start) / 1e9
        run(2)
        let hosts = pids.map(Self.footprintMB), app = Self.footprintMB(getpid())
        EngineV3MemoryPressure.shared.handle(.warning)
        run(3)
        let hostsTrimmed = pids.map(Self.footprintMB), appTrimmed = Self.footprintMB(getpid())
        let pages = models.map(\.engineV3.pageCount)
        print("MultiWindowLarge: {\"windows\": \(n), \"pages\": \(pages), \"all_compiled_s\": \(Int(seconds)), \"host_mb\": \(hosts), \"app_mb\": \(app), "
              + "\"after_warning\": {\"host_mb\": \(hostsTrimmed), \"app_mb\": \(appTrimmed)}}")
    }
}
