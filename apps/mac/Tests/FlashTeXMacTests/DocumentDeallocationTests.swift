import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// Permanent memory guards for the document window (owner, 2026-10-04: zero
/// leaks). Each test opens a document the way the app does (a `ShellModel`,
/// File > Open's `openTex`, `ContentView` in a window under a window
/// controller), closes the window, lets go of everything and then checks
/// with weak references that the window, its controller, the hosting view,
/// the editor's text view, the preview's views, the model and its engine
/// session all deallocate, and that a closed document's `flashtex-host` exits.
///
/// The tests are synchronous and never suspend: every wait turns the main
/// run loop itself, one autorelease pool per turn (`DeallocationProbe`).
/// Suspending would let XCTest's own run loop do the app's main-thread work
/// outside any pool the test drains, and what it autoreleased would stay
/// alive until the test ends (a false leak). Every wait is bounded, so a slow
/// machine waits longer but never fails a release that happens.
@MainActor
final class DocumentDeallocationTests: XCTestCase {
    private var dir: URL!

    override func setUp() {
        OwnerStateGuard.install()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("dealloc-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    override func tearDown() {
        if let dir { try? FileManager.default.removeItem(at: dir) }
    }

    static let document = """
    \\documentclass{article}
    \\begin{document}
    \\section{One}
    Hello, deallocation.
    \\end{document}

    """

    private func writeDocument(_ name: String) throws -> URL {
        let url = dir.appendingPathComponent(name)
        try Self.document.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    struct Timeout: Error, CustomStringConvertible { let description: String }

    private func waitUntil(_ what: String, timeout: TimeInterval = 30, file: StaticString = #filePath, line: UInt = #line, _ cond: () -> Bool) throws {
        guard DeallocationProbe.turn(until: cond, timeout: timeout) else {
            XCTFail("timeout waiting for \(what)", file: file, line: line)
            throw Timeout(description: what)
        }
    }

    /// What one open document window is made of.
    struct Opened {
        let model: ShellModel
        let controller: NSWindowController
        let textView: NSTextView
    }

    /// Opens `file` as the app does: a model, the document opened in it,
    /// `ContentView` in a window under a window controller, shown.
    private func openWindow(_ file: URL, engineV3: Bool) throws -> Opened {
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        model.engineV3Enabled = engineV3
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 1440, height: 900), styleMask: [.titled, .closable, .resizable])
        window.isReleasedWhenClosed = false // ARC owns it (the app's windows are SwiftUI's)
        let hosting = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        hosting.sizingOptions = [] // the window keeps its size, as WindowGroup's .defaultSize does (not the content's minimum)
        window.contentView = hosting
        let controller = NSWindowController(window: window)
        window.orderFrontRegardless()
        var found: NSTextView?
        try waitUntil("the editor's text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        if engineV3 { try waitUntil("the v3 preview's pages view") { model.engineV3.view != nil } }
        return Opened(model: model, controller: controller, textView: try XCTUnwrap(found))
    }

    /// Records everything of `o` that must deallocate once it is closed and released.
    private func track(_ o: Opened, in probe: DeallocationProbe) {
        let window = o.controller.window
        probe.track(o.model, "ShellModel")
        probe.track(o.model.engineV3, "EngineV3Session")
        probe.track(o.model.project, "ProjectDocuments")
        probe.track(o.controller, "NSWindowController")
        probe.track(window, "NSWindow")
        probe.track(window?.contentView, "NSHostingView")
        probe.track(o.textView, "editor NSTextView")
        probe.track(o.textView.enclosingScrollView, "editor NSScrollView")
        probe.track(o.textView.textStorage, "editor NSTextStorage")
        // The open document's file watcher: its vnode source runs (and keeps
        // the file open) until cancelled.
        let watcher = o.model.documentWatcher
        XCTAssertTrue(watcher.isWatching, "the open document is watched")
        probe.track(watcher, "DocumentWatcher")
        probe.track(watcher.source.map { $0 as AnyObject }, "DocumentWatcher's vnode source")
        if let pages = o.model.engineV3.view {
            probe.track(pages, "EngineV3PagesView")
            probe.track(pages.enclosingScrollView, "EngineV3ScrollContainer")
        }
    }

    /// What the session installed while it ran (AppKit's monitor list,
    /// NotificationCenter and the run loop hold these until they are
    /// removed): recorded just before the window closes.
    private func trackInstalled(_ s: EngineV3Session, in probe: DeallocationProbe) {
        probe.track(s.keyMonitor.map { $0 as AnyObject }, "session key monitor")
        probe.track(s.clickMonitor.map { $0 as AnyObject }, "session click monitor")
        probe.track(s.storageObserver, "session text-storage observer")
        probe.track(s.stallTimer, "session stall timer")
    }

    /// A typed edit before "Hello": the editor, the model and (v3) the
    /// session's fast path all run; with a host, its compile commits a page
    /// for a keystroke, which arms the pane's vsync display link.
    private func typeEdit(_ o: Opened) throws {
        o.textView.window?.makeFirstResponder(o.textView)
        o.textView.setSelectedRange(NSRange(location: (o.textView.string as NSString).range(of: "Hello").location, length: 0))
        o.model.engineV3.nextKeystrokeNs = MonotonicClock.nowNs()
        o.textView.insertText("Again. ", replacementRange: o.textView.selectedRange())
        try waitUntil("the edit reaches the model") { o.model.activeText.contains("Again. ") }
    }

    /// Waits for the host to be ready (fails the test if it does not start).
    private func awaitReady(_ s: EngineV3Session) throws {
        try waitUntil("the engine-v3 host to be ready", timeout: 180) {
            if case .failed(let why) = s.phase { XCTFail("the engine-v3 host did not start: \(why)"); return true }
            return s.phase == .ready
        }
        guard s.phase == .ready else { throw Timeout(description: "host failed") }
    }

    /// Open, edit and close a document window three times with the old
    /// engine's preview: every window, view and model goes away.
    func testClosedDocumentWindowsDeallocate() throws {
        var env = EnvironmentOverride()
        env.set("FLASHTEX_HOST", "none") // no host: this test is about the window path alone
        defer { env.restore() }
        let file = try writeDocument("plain.tex")
        let probe = DeallocationProbe()
        for _ in 0 ..< 3 {
            try autoreleasepool {
                let o = try openWindow(file, engineV3: false)
                track(o, in: probe)
                try typeEdit(o)
                o.controller.close()
            }
        }
        XCTAssertGreaterThanOrEqual(probe.count, 3 * 9)
        let alive = probe.waitForRelease(timeout: 20)
        XCTAssertEqual(alive, [], "still alive after close")
    }

    /// The same with the engine-v3 pane mounted and no host (the session
    /// fails to find one): the pane's views, its session and the model go away.
    func testClosedV3DocumentWindowsDeallocateWithoutAHost() throws {
        var env = EnvironmentOverride()
        env.set("FLASHTEX_HOST", "none")
        defer { env.restore() }
        let file = try writeDocument("v3-nohost.tex")
        let probe = DeallocationProbe()
        for _ in 0 ..< 3 {
            try autoreleasepool {
                let o = try openWindow(file, engineV3: true)
                track(o, in: probe)
                try typeEdit(o)
                trackInstalled(o.model.engineV3, in: probe)
                o.controller.close()
            }
        }
        let alive = probe.waitForRelease(timeout: 20)
        XCTAssertEqual(alive, [], "still alive after close")
    }

    /// With a real `flashtex-host`: each document compiles, takes a typed
    /// edit and its compile, is closed and released; every object
    /// deallocates and every host process exits.
    func testClosedV3DocumentWindowsDeallocateAndTheirHostsExit() throws {
        try EngineV3TestHost.require() // the hosts use this test process's private cache and format cache (EngineV3.cacheDirectory)
        let probe = DeallocationProbe()
        var pids: [Int32] = []
        defer { for pid in pids where !DeallocationProbe.processGone(pid) { kill(pid, SIGKILL) } } // never leave a host behind
        for i in 0 ..< 3 {
            let file = try writeDocument("doc\(i).tex")
            try autoreleasepool {
                let o = try openWindow(file, engineV3: true)
                track(o, in: probe)
                let s = o.model.engineV3
                try awaitReady(s)
                if let pid = s.hostPID { pids.append(pid) } else { XCTFail("document \(i) has no host") }
                try waitUntil("the first compile", timeout: 120) { s.statusNote.hasPrefix("ok") && s.pageCount == 1 && !s.compiling }
                let done = s.doneCount
                try typeEdit(o)
                try waitUntil("the edit's compile", timeout: 60) { s.doneCount > done && !s.compiling }
                // The keystroke's page was committed on screen: the pane armed its vsync link.
                try waitUntil("the keystroke's page committed", timeout: 30) { !s.latency.samples.isEmpty }
                trackInstalled(s, in: probe)
                o.controller.close()
            }
        }
        let alive = probe.waitForRelease(timeout: 30)
        XCTAssertEqual(alive, [], "still alive after close")
        for pid in pids {
            XCTAssertTrue(DeallocationProbe.waitForExit(pid, timeout: 20), "host \(pid) of a closed, released document is still running")
        }
    }

    /// Opening one document after another in the same window keeps one
    /// host, and everything goes away once the window is closed.
    func testReopeningDocumentsInOneWindowLeavesNothingBehind() throws {
        try EngineV3TestHost.require()
        let probe = DeallocationProbe()
        var pids: Set<Int32> = []
        defer { for pid in pids where !DeallocationProbe.processGone(pid) { kill(pid, SIGKILL) } }
        try autoreleasepool {
            let o = try openWindow(try writeDocument("first.tex"), engineV3: true)
            track(o, in: probe)
            let s = o.model.engineV3
            try awaitReady(s)
            for i in 0 ..< 3 {
                let url = try writeDocument("next\(i).tex")
                XCTAssertEqual(o.model.openTex(at: url, dirty: .discard), .opened)
                try waitUntil("next\(i).tex compiled", timeout: 120) { s.mainFile == "next\(i).tex" && s.statusNote.hasPrefix("ok") && !s.compiling }
                if let p = s.hostPID { pids.insert(p) }
            }
            let current = try XCTUnwrap(s.hostPID)
            for p in pids where p != current {
                XCTAssertTrue(DeallocationProbe.waitForExit(p, timeout: 20), "the host \(p) of a replaced document is still running")
            }
            trackInstalled(s, in: probe)
            o.controller.close()
        }
        let alive = probe.waitForRelease(timeout: 30)
        XCTAssertEqual(alive, [], "still alive after close")
        for p in pids {
            XCTAssertTrue(DeallocationProbe.waitForExit(p, timeout: 20), "host \(p) still running after its window closed")
        }
    }

    /// An editor leaves no unreachable memory: `NSTextView.init(frame:)`
    /// runs the subclass's property initializers twice (through
    /// `init(frame:textContainer:)`), and the first objects they made were
    /// never released, one set per editor (leaks(1) found them after the
    /// suites). Weak references cannot see an object nothing names, so this
    /// asks `leaks` about this process.
    func testEditorsLeaveNoUnreachableObjects() throws {
        for _ in 0 ..< 4 {
            autoreleasepool {
                let scroll = CompletingTextView.scrollable() // as SourceEditorView makes its editor
                let tv = scroll.documentView as? CompletingTextView
                XCTAssertNotNil(tv)
                _ = tv?.folds; _ = tv?.scheduler; _ = tv?.recentlyUsed
                let direct = CompletingTextView(frame: NSRect(x: 0, y: 0, width: 100, height: 100)) // and as tests do
                _ = direct.folds
            }
        }
        DeallocationProbe.turnRunLoop()
        guard let report = DeallocationProbe.leaksReport() else { throw XCTSkip("leaks(1) cannot examine this process") }
        let ours = DeallocationProbe.leakedLines(report, types: ["CompletingTextView", "CompletionScheduler", "EditorFoldStore", "RecentlyUsed"])
        XCTAssertEqual(ours, [], "unreachable editor objects")
    }

    /// The keystroke latency's per-compile bookkeeping stays bounded over a
    /// long session: every keystroke is a compile, and its entry (and the
    /// painted/expected marks) were kept for as long as the window was open.
    func testLatencyBookkeepingPerCompileStaysBounded() {
        let l = EngineV3Latency()
        let n = 20 * EngineV3Latency.compileWindow
        for i in 1 ... n {
            let t = UInt64(i) * 1_000_000_000
            l.sent(compile: i, keystrokeNs: t, editNs: t, path: "main.tex", at: t + 1_000)
            l.pageOnMain(compile: i, timing: EngineV3Latency.PageTiming(), at: t + 10_000_000)
            if i % 3 == 0 { l.offscreen(compile: i) } else if i % 3 == 1 { l.committed(compile: i, page: 0, at: t + 20_000_000) }
            l.done(compile: i, cancelled: false, hostFirstPageMs: 1)
        }
        XCTAssertLessThanOrEqual(l.heldCompileEntries, 4 * 2 * EngineV3Latency.compileWindow + 8)
        XCTAssertEqual(l.samples.last?.compile, n - 1, "the newest committed keystroke is still sampled")
    }

    /// The tile timings the app records on every page entering a zoom stay
    /// bounded over a long session (they grew by one per source for ever).
    func testFirstTileTimingsStayBounded() {
        EngineV3TileGrid.resetCounters()
        defer { EngineV3TileGrid.resetCounters() }
        for i in 0 ..< EngineV3TileGrid.firstTileKeep * 3 { EngineV3TileGrid.noteFirstTile(ms: Double(i)) }
        XCTAssertLessThanOrEqual(EngineV3TileGrid.firstTileMs.count, EngineV3TileGrid.firstTileKeep + 512)
        XCTAssertEqual(EngineV3TileGrid.firstTileMs.last, Double(EngineV3TileGrid.firstTileKeep * 3 - 1), "the newest is kept")
    }
}
