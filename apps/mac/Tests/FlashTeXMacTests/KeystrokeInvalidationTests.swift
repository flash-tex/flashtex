import AppKit
import HostedWindows
import Observation
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// P5-KEYSTROKE-MAIN: a keystroke re-evaluates the editor, not the window,
/// the sidebar, the status bar or the preview, and the App scene's menus do
/// not observe what a keystroke changes (docs/evidence/keystroke-main-2026-10-03).
@MainActor
final class KeystrokeInvalidationTests: XCTestCase {
    private var window: NSWindow?

    override func tearDown() async throws {
        ViewBodyProbe.stop()
        window?.orderOut(nil)
        window = nil
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    /// Longer than the chrome's 100 ms refresh, the breadcrumb's 80 ms and
    /// the outline's 150 ms debounces, so anything they schedule has run.
    private func settle() async throws { try await Task.sleep(nanoseconds: 600_000_000) }

    static let document = """
    \\documentclass{article}
    \\begin{document}
    \\section{One}
    First paragraph with enough words to type into, the caret sits in the middle of it.

    Second paragraph.
    \\section{Two}
    Third paragraph.
    \\end{document}

    """

    private func host(_ model: ShellModel) async throws -> NSTextView {
        HostedWindowSupport.prepare() // non-activating
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 1400, height: 900), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        let hosting = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.contentView = hosting
        window.orderFrontRegardless() // never makeKey
        hosting.layoutSubtreeIfNeeded()
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        try await settle()
        return try XCTUnwrap(found)
    }

    /// Types `count` characters the way the typing benches do (NSTextView
    /// insertText at the caret), one run-loop turn apart.
    private func type(_ count: Int, into tv: NSTextView) async throws {
        for _ in 0..<count {
            tv.insertText("x", replacementRange: tv.selectedRange())
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    func testAKeystrokeReEvaluatesTheEditorButNotTheWindowSidebarStatusBarOrPreview() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: Self.document)
        let tv = try await host(model)
        let caret = (tv.string as NSString).range(of: "the caret").location
        XCTAssertNotEqual(caret, NSNotFound)
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        // The first edit makes the document dirty, which the tab, the tree and
        // the status bar show: let that settle before counting.
        try await type(1, into: tv)
        try await settle()

        let revision = model.editorRevision
        ViewBodyProbe.start()
        try await type(5, into: tv)
        try await settle()
        ViewBodyProbe.stop()
        let counts = ViewBodyProbe.counts

        XCTAssertEqual(model.editorRevision, revision + 5, "every keystroke reached the model")
        XCTAssertGreaterThanOrEqual(counts["EditorPane"] ?? 0, 1, "the editor still follows the model (\(counts))")
        for view in ["ContentView", "ToolRail", "WorkspaceSidebar", "ProjectSection", "PreviewPane", "StatusBreadcrumb", "WordCountStatusItem"] {
            XCTAssertEqual(counts[view] ?? 0, 0, "\(view) re-evaluated while typing (\(counts))")
        }
        // The status bar shows throttled mirrors (ShellChrome, 100 ms): it may
        // follow the revision a few times, never once per keystroke and per
        // flip of a mirror.
        XCTAssertLessThan(counts["StatusBar"] ?? 0, 5, "the status bar re-evaluated per keystroke (\(counts))")
    }

    /// The same under engine v3, with the caret on the page (gap C12): the
    /// caret moves on every keystroke, and the pages view is told directly,
    /// so neither the pane nor the pages' SwiftUI view re-evaluates, and the
    /// mark still follows the caret.
    func testOnEngineV3AKeystrokeReEvaluatesNeitherThePagesNorThePane() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: Self.document)
        model.engineV3Enabled = true
        let tv = try await host(model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        // A host (when one is built) compiles first; count after it settled.
        try await waitUntil("the engine settled", timeout: 120) {
            switch s.phase {
            case .ready: return !s.compiling && s.pageCount > 0
            case .failed, .idle: return true
            case .starting: return false
            }
        }
        let caret = (tv.string as NSString).range(of: "the caret").location
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        try await type(1, into: tv)
        try await settle()

        let followBefore = model.caretFollow.request?.token
        ViewBodyProbe.start()
        try await type(5, into: tv)
        try await settle()
        ViewBodyProbe.stop()
        let counts = ViewBodyProbe.counts
        XCTAssertGreaterThanOrEqual(counts["EditorPane"] ?? 0, 1, "the editor still follows the model (\(counts))")
        for view in ["ContentView", "PreviewV3Pane", "PreviewPane"] {
            XCTAssertEqual(counts[view] ?? 0, 0, "\(view) re-evaluated while typing (\(counts))")
        }
        // The pages' view reads one thing that changes after typing: the
        // caret follower's request (CaretFollow.swift), debounced to once per
        // burst. Never once per keystroke, and never for the caret itself.
        let follows = model.caretFollow.request?.token == followBefore ? 0 : 1
        XCTAssertLessThanOrEqual(counts["PreviewV3Scroll"] ?? 0, follows,
                                 "the pages' view re-evaluated beyond the follow request (\(follows)): \(counts)")
        let pages = try XCTUnwrap(s.view, "the v3 pages view is on screen")
        XCTAssertEqual(pages.caretKey?.utf16, tv.selectedRange().location, "the caret mark followed the caret")
    }

    /// The pages' own view (split out of `PreviewV3Pane` here) keeps main's
    /// dark ground (gap C10): the dark preview's ground, not the light one,
    /// fills the pane around the pages.
    func testThePagesViewUsesTheDarkPreviewGround() async throws {
        func centrePixel(dark: Bool) async throws -> (r: Int, g: Int, b: Int) {
            let model = ShellModel()
            model.darkPreview = dark
            HostedWindowSupport.prepare()
            let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 200, height: 200), styleMask: [.titled],
                                                    backing: .buffered, defer: false)
            window.appearance = NSAppearance(named: .aqua)
            let hosting = NSHostingView(rootView: PreviewV3Scroll().environment(model))
            window.contentView = hosting
            window.orderFrontRegardless()
            defer { window.orderOut(nil); model.engineV3.view = nil }
            hosting.layoutSubtreeIfNeeded()
            try await Task.sleep(nanoseconds: 300_000_000)
            window.displayIfNeeded()
            let layer = try XCTUnwrap(hosting.layer)
            let w = 200, h = 200
            let ctx = try XCTUnwrap(CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
            NSAppearance(named: .aqua)!.performAsCurrentDrawingAppearance { layer.render(in: ctx) }
            let px = try XCTUnwrap(ctx.data).assumingMemoryBound(to: UInt8.self)
            let o = (h / 2 * w + w / 2) * 4
            return (Int(px[o]), Int(px[o + 1]), Int(px[o + 2]))
        }
        let dark = try await centrePixel(dark: true)
        let light = try await centrePixel(dark: false)
        // DS.Preview.darkGround is white 0.12 (31 of 255).
        XCTAssertLessThan(abs(dark.r - 31) + abs(dark.g - 31) + abs(dark.b - 31), 12, "dark ground: \(dark)")
        XCTAssertGreaterThan(light.r + light.g + light.b, dark.r + dark.g + dark.b + 150, "light ground \(light) vs dark \(dark)")
    }

    /// `IsolatedTask` runs its action for every new id, as `.task(id:)` does,
    /// while the view it is attached to is not re-evaluated by the id's reads.
    func testAnIsolatedTaskFollowsItsIdWithoutReEvaluatingItsParent() async throws {
        let source = IsolatedTaskSource()
        let log = IsolatedTaskLog()
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 200, height: 100), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: IsolatedTaskParent(source: source, log: log))
        window.orderFrontRegardless()
        self.window = window
        try await waitUntil("the first run") { log.ran == [0] }
        let parentBodies = log.parentBodies
        for i in 1...3 {
            source.value = i
            try await waitUntil("the run for \(i)") { log.ran.last == i }
        }
        XCTAssertEqual(log.ran, [0, 1, 2, 3], "one run per id, in order")
        XCTAssertEqual(log.parentBodies, parentBodies, "the parent re-evaluated for a value only its isolated task reads")
        source.label = "changed"
        try await waitUntil("the parent follows what it reads itself") { log.parentBodies > parentBodies }
        XCTAssertEqual(log.ran, [0, 1, 2, 3], "the same id does not run the task again")
    }

    /// The App scene's menus read change-only mirrors: a keystroke changes
    /// none of them.
    func testTheMenuMirrorsDoNotChangeOnAKeystroke() {
        let model = ShellModel()
        model.replaceProject(entryText: Self.document)
        XCTAssertEqual(model.menuEntryPath, model.project.entryPath)
        XCTAssertEqual(model.documentCount, model.documents.count)
        var fired = false
        withObservationTracking {
            _ = model.menuEntryPath
            _ = model.documentCount
            _ = model.toolbarHasDocument
            _ = model.project.projectRoot
            _ = model.exportAvailable
        } onChange: { fired = true }
        model.updateActiveText(model.activeText + "x")
        model.updateActiveText(model.activeText + "y")
        XCTAssertFalse(fired, "a keystroke invalidated the App scene's menus")
        XCTAssertEqual(model.menuEntryPath, model.project.entryPath)
    }

    /// The mirrors still follow a real change: the entry path when the
    /// project changes, the count when a document opens.
    func testTheMenuMirrorsFollowTheProject() {
        let model = ShellModel()
        model.replaceProject(entryText: Self.document)
        let before = model.documentCount
        model.documents.append(.init(path: "chapter.tex", text: "x"))
        XCTAssertEqual(model.documentCount, before + 1)
        XCTAssertEqual(model.menuEntryPath, model.project.entryPath)
        model.documents = []
        XCTAssertEqual(model.documentCount, 0)
        XCTAssertEqual(model.menuEntryPath, model.project.entryPath, "with no documents the entry is the active path")
    }

    /// Under engine v3 the chrome's route, result and compiling fields are
    /// assigned once per refresh: an unchanged refresh fires no observation.
    func testAChromeRefreshThatChangesNothingFiresNothing() {
        let model = ShellModel()
        model.replaceProject(entryText: Self.document)
        model.engineV3Enabled = true
        model.engineV3.stop() // the v3 route without a host process: the phase stays idle
        defer { model.engineV3Enabled = false }
        model.flushChrome()
        var fired = false
        withObservationTracking {
            _ = model.chrome.route
            _ = model.chrome.routeHelp
            _ = model.chrome.hasResult
            _ = model.chrome.compiling
            _ = model.chrome.previewSource
            _ = model.chrome.staleText
            _ = model.chrome.capabilityNotes
        } onChange: { fired = true }
        model.flushChrome()
        model.flushChrome()
        XCTAssertFalse(fired, "an unchanged refresh flipped a chrome field and back")
        XCTAssertEqual(model.chrome.route, .engineV3)
    }

}

@MainActor @Observable
private final class IsolatedTaskSource {
    var value = 0
    var label = "label"
}

@MainActor
private final class IsolatedTaskLog {
    var ran: [Int] = []
    var parentBodies = 0
}

private struct IsolatedTaskParent: View {
    let source: IsolatedTaskSource
    let log: IsolatedTaskLog

    var body: some View {
        let _ = { log.parentBodies += 1 }()
        Text(source.label)
            .background(IsolatedTask(id: { source.value }) { id in log.ran.append(id) })
    }
}
