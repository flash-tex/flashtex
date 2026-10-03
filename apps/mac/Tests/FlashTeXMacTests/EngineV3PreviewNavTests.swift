import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// Navigation in the engine-v3 preview that the v2 pane has (DESIGN §10 app
/// parity, gaps C2, C6, C7 and C10): Page Up / Page Down step whole pages
/// (v2's `PreviewPageStep`) and are announced, Fit Page (⌘⇧9) fits the
/// tallest page to the pane, the reading position survives a reflow that
/// changes the size of the pages above it, and the dark preview darkens the
/// ground. The pane lives in a hosted window; the host tests need a built
/// `flashtex-host` and TeX Live (skipped otherwise).
@MainActor
final class EngineV3PreviewNavTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-nav-tests-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    static func document(height: Int) -> String {
        """
        \\documentclass{article}
        \\begin{document}
        \\pdfpageheight=\(height)pt
        First page.\\newpage
        Second page.\\newpage
        Third page, with a line to read.\\newpage
        Fourth page.
        \\end{document}

        """
    }

    /// A model with v3 on, its pane in a hosted window, compiled.
    func pane(_ text: String) async throws -> (ShellModel, EngineV3PagesView, NSClipView, NSWindow) {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: text, named: "main.tex")
        model.engineV3Enabled = true
        model.autoCompile = true // edits compile as they are typed
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 560, height: 700), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        try await EngineV3TestHost.awaitReady(model.engineV3)
        let s = model.engineV3
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 4 && (0 ..< 4).allSatisfy { s.pages[$0] != nil } }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        let clip = try XCTUnwrap(pages.enclosingScrollView?.contentView)
        return (model, pages, clip, window)
    }

    func key(_ code: UInt16, in window: NSWindow) -> NSEvent {
        NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: window.windowNumber,
                         context: nil, characters: "", charactersIgnoringModifiers: "", isARepeat: false, keyCode: code)!
    }

    /// Page Down goes to the next page's top, Page Up from mid-page to the
    /// page's own top and from a page top to the previous page (v2's
    /// `PreviewPageStep`), each announced "Page N of 4".
    func testPageUpAndDownStepWholePagesAsOnV2() async throws {
        let (model, pages, clip, window) = try await pane(Self.document(height: 600))
        defer { model.engineV3.stop(); window.contentView = nil }
        clip.scroll(to: .zero)
        pages.keyDown(with: key(121, in: window)) // Page Down
        let p2 = try XCTUnwrap(pages.viewPoint(page: 1, .zero))
        XCTAssertEqual(clip.bounds.minY, p2.y - 16, accuracy: 0.5, "page 2's top (less the margin) at the top")
        pages.keyDown(with: key(121, in: window))
        let p3 = try XCTUnwrap(pages.viewPoint(page: 2, .zero))
        XCTAssertEqual(clip.bounds.minY, p3.y - 16, accuracy: 0.5)
        // Mid-page: Page Up returns to this page's top, then to the previous page.
        clip.scroll(to: CGPoint(x: 0, y: p3.y + 100))
        pages.keyDown(with: key(116, in: window)) // Page Up
        XCTAssertEqual(clip.bounds.minY, p3.y - 16, accuracy: 0.5)
        pages.keyDown(with: key(116, in: window))
        XCTAssertEqual(clip.bounds.minY, p2.y - 16, accuracy: 0.5)
        model.previewAnnouncer.flush()
        XCTAssertEqual(model.previewAnnouncer.announcements.suffix(1), ["Page 2 of 4"])
        // The first page at its top: nowhere to go up.
        clip.scroll(to: .zero)
        XCTAssertNil(pages.pageStep(.up))
    }

    /// ⌘⇧9: the tallest page fills the pane's height (v1's Fit Page rule).
    func testFitPageFitsTheTallestPageToThePane() async throws {
        let (model, pages, clip, window) = try await pane(Self.document(height: 600))
        defer { model.engineV3.stop(); window.contentView = nil }
        XCTAssertNotEqual(model.previewFitPageZoom, 1, "the pane published its Fit Page zoom")
        model.previewFitPage()
        pages.update(revision: model.engineV3.layoutRevision, zoom: model.previewZoom)
        let top = try XCTUnwrap(pages.viewPoint(page: 0, .zero))
        let bottom = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: 0, y: 600)))
        // Within 1 %: the horizontal scroller a zoomed page brings, and device-pixel rounding.
        XCTAssertEqual(bottom.y - top.y, clip.bounds.height - 32, accuracy: 0.01 * clip.bounds.height, "page height = pane height less the margins")
    }

    /// Reading page 3, an edit makes every page taller: the same page point
    /// stays at the top of the view (gap C7), where before the view kept its
    /// absolute offset and drifted up into page 2.
    func testReadingPositionSurvivesAReflowAboveIt() async throws {
        let (model, pages, clip, window) = try await pane(Self.document(height: 600))
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        let p3 = try XCTUnwrap(pages.viewPoint(page: 2, CGPoint(x: 0, y: 200)))
        clip.scroll(to: CGPoint(x: 0, y: p3.y))
        pages.enclosingScrollView?.reflectScrolledClipView(clip)
        let before = try XCTUnwrap(pages.pagePoint(at: CGPoint(x: clip.bounds.midX, y: clip.bounds.minY + 1)))
        XCTAssertEqual(before.page, 2)
        let oldHash = s.pages[0]?.page.hash
        model.updateActiveText(Self.document(height: 700))
        try await waitUntil("the reflow (\(s.statusNote), \(s.pages[0]?.heightPt ?? -1))") { !s.compiling && s.pages[0]?.page.hash != oldHash && s.pages[0].map { abs($0.heightPt - 700 * 72 / 72.27) < 0.5 } == true } // TeX pt → bp
        pages.update(revision: s.layoutRevision, zoom: model.previewZoom)
        let after = try XCTUnwrap(pages.pagePoint(at: CGPoint(x: clip.bounds.midX, y: clip.bounds.minY + 1)))
        XCTAssertEqual(after.page, 2, "still on page 3")
        XCTAssertEqual(after.point.y, before.point.y, accuracy: 1, "the same place on page 3")
    }

    /// Focus: Tab (a key event) and VoiceOver focus the pane; a click and
    /// AppKit's own pick of a first key view (no event) do not.
    func testThePaneTakesFocusOnlyFromTheKeyboardOrVoiceOver() {
        XCTAssertTrue(EngineV3PagesView.acceptsFocus(eventType: .keyDown, voiceOver: false))
        XCTAssertTrue(EngineV3PagesView.acceptsFocus(eventType: nil, voiceOver: true))
        XCTAssertFalse(EngineV3PagesView.acceptsFocus(eventType: nil, voiceOver: false))
        XCTAssertFalse(EngineV3PagesView.acceptsFocus(eventType: .leftMouseDown, voiceOver: true))
    }

    /// At launch the editor has the focus and keeps it: the pane (ahead of
    /// it in the view order here) refuses a focus change made without an
    /// event (VoiceOver off), which is the launch and window-key case.
    func testTheEditorKeepsFocusAtLaunch() throws {
        guard !NSWorkspace.shared.isVoiceOverEnabled else { throw XCTSkip("VoiceOver is on: the pane may take focus by design") }
        env.set("FLASHTEX_HOST", "none")
        let model = ShellModel()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled])
        window.isReleasedWhenClosed = false
        defer { window.contentView = nil }
        let root = NSView(frame: NSRect(x: 0, y: 0, width: 600, height: 400))
        let pane = EngineV3PagesView(session: model.engineV3)
        pane.frame = NSRect(x: 0, y: 0, width: 300, height: 400)
        let editor = NSTextView(frame: NSRect(x: 300, y: 0, width: 300, height: 400))
        root.addSubview(pane)
        root.addSubview(editor)
        window.contentView = root
        // AppKit's own pick of a first responder (at launch, when the window
        // becomes key) walks the key view loop, which skips a view that
        // cannot become key: with no event at hand the pane is skipped and
        // the editor, after it in the view order, is the window's pick.
        window.recalculateKeyViewLoop()
        XCTAssertFalse(pane.acceptsFirstResponder, "no event, no VoiceOver: the pane refuses focus")
        XCTAssertFalse(pane.canBecomeKeyView)
        XCTAssertTrue(editor.canBecomeKeyView)
        XCTAssertTrue(pane.nextValidKeyView === editor, "next valid key view: \(String(describing: pane.nextValidKeyView))")
    }

    /// Keys the pane does not use: Tab and Escape (and ⌘/⌃ keys) go up the
    /// chain; other keys are dropped rather than beeping; arrows scroll.
    func testKeysThePaneDoesNotUseAreDroppedOrPassedOn() {
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 121, modifiers: []), .page(.down))
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 116, modifiers: []), .page(.up))
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 125, modifiers: []), .scroll(40))
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 48, modifiers: []), .pass)
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 53, modifiers: []), .pass)
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 0, modifiers: [.command]), .pass)
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 0, modifiers: []), .ignore) // "a": no beep
    }

    /// The dark preview darkens the ground around the pages, as on v2.
    func testDarkPreviewDarkensTheGroundAsOnV2() {
        XCTAssertEqual(PreviewV3Pane.ground(dark: true), DS.Preview.darkGround)
        XCTAssertEqual(PreviewV3Pane.ground(dark: false), DS.Colors.surfaceGround)
    }
}
