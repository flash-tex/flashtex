import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import FlashTeXAccessibility
import HostedWindows
@testable import FlashTeXMac

/// VoiceOver over the preview pane (task ux-preview-pane-voiceover): the v2
/// page tree (PreviewV2Accessibility.swift) read back through the
/// NSAccessibility protocol, Page Up/Down stepping whole pages through the
/// anchor probe (PreviewAnchor.swift), the compile announcer's coalescing
/// (PreviewAnnouncements.swift), and the pane's container label. Hosted
/// windows come from `HostedWindowSupport` and are never made key.
///
/// SwiftUI materialises its own accessibility modifiers only for an
/// assistive client (see PanelAccessibilityTests), so the pane's
/// `.accessibilityLabel` / `.focusable()` are pinned at the source level
/// here, the way `CommandTableTests` pins the focus order.
@MainActor
final class PreviewPaneAccessibilityTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let sources = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Sources")
    static let samples = sources.deletingLastPathComponent().appendingPathComponent("Samples")

    private var windows: [NSWindow] = []

    override func tearDown() {
        for w in windows { w.orderOut(nil) }
        windows.removeAll()
        super.tearDown()
    }

    private func textEnvelope() throws -> RenderingV2.Envelope {
        try RenderingV2.decode(try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json")))
    }

    /// The prepared text fixture, its one page duplicated to `n` pages (as
    /// PreviewAnchoringTests does), so the pane has pages to step between.
    private func frame(pages n: Int) throws -> V2Frame {
        var frame = try V2Frame.prepare(try textEnvelope(), store: PreviewV2Tests.store)
        let page = try XCTUnwrap(frame.list.pages.first), prepared = try XCTUnwrap(frame.prepared.first)
        for number in 2...max(n, 2) where number <= n {
            var p = page; p.number = number
            var q = prepared; q.number = number
            frame.list.pages.append(p); frame.prepared.append(q)
        }
        return frame
    }

    private func host<V: View>(_ view: V, width: CGFloat, height: CGFloat) -> NSHostingView<V> {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: width, height: height), styleMask: [.titled], backing: .buffered, defer: false)
        let hosting = NSHostingView(rootView: view)
        window.contentView = hosting
        window.orderFrontRegardless() // off-screen and never key: no focus change
        windows.append(window)
        return hosting
    }

    private func settle(_ seconds: TimeInterval = 0.4) async throws { try await Task.sleep(nanoseconds: UInt64(seconds * 1e9)) }

    static func findAll<T: NSView>(_ type: T.Type, in view: NSView) -> [T] {
        var out: [T] = []
        if let v = view as? T { out.append(v) }
        for sub in view.subviews { out += findAll(type, in: sub) }
        return out
    }

    // MARK: V2PageText (pure)

    func testV2PageTextGroupsRunsIntoLinesAndJoinsWordsAtGlue() throws {
        let page = try XCTUnwrap(textEnvelope().payload.pages.first)
        let lines = V2PageText.lines(of: page)
        // The section heading (unnumbered in this producer build) and the body line, top to bottom.
        XCTAssertEqual(lines.map(\.text), ["Office fixtures", "The AV office fixed the fi ligature: office, bold, and café."])
        XCTAssertEqual(lines.map(\.number), [1, 2])
        // Every run is one word (the italic word's comma is its own upright run,
        // joined without a space); kerned pairs and ligatures stay inside their run.
        XCTAssertEqual(lines[1].words.map(\.text), ["The", "AV", "office", "fixed", "the", "fi", "ligature:", "office", ",", "bold", ",", "and", "café."])
        XCTAssertTrue(lines.indices.dropFirst().allSatisfy { lines[$0].rect.minY > lines[$0 - 1].rect.minY }, "lines are ordered top to bottom")
        XCTAssertTrue(lines[1].words.indices.dropFirst().allSatisfy { lines[1].words[$0].rect.minX > lines[1].words[$0 - 1].rect.minX }, "words are ordered left to right")
        // The body's "Go to source" target is its first word's whole source span (the clusters' ranges merged).
        XCTAssertEqual(lines[1].sources, [RenderingV2.SourceRange(path: "main.tex", startByte: 67, endByte: 70)])
        XCTAssertEqual(lines[1].words.last?.sources, [RenderingV2.SourceRange(path: "main.tex", startByte: 138, endByte: 145)], "café. spans caf\\'e.")
        XCTAssertEqual(V2PageText.pageLabel(number: 3, totalPages: 12, lineCount: 1), "Page 3 of 12, 1 line")
        XCTAssertEqual(V2PageText.pageLabel(number: 3, totalPages: 0, lineCount: 2), "Page 3, 2 lines")
        XCTAssertEqual(V2PageText.elidedPageLabel(number: 7, totalPages: 12), "Page 7 of 12, not loaded")
        XCTAssertEqual(V2PageText.lineLabel(page: 2, line: lines[0]), "Page 2, line 1: Office fixtures")
    }

    func testV2PageTextSplitsWordsOnlyAtProducerGaps() {
        func run(_ text: String, x: Double, size: Double = 10, baseline: Double = 100) -> RenderingV2.Item {
            let t = { (pt: Double) in Int64((pt * Double(RenderingV2.ticksPerPoint)).rounded()) }
            var glyphs: [RenderingV2.Glyph] = []
            var clusters: [RenderingV2.Cluster] = []
            for (i, _) in text.utf8.enumerated() {
                glyphs.append(RenderingV2.Glyph(gid: 1, originX: t(x + Double(i) * size * 0.5), baselineY: t(baseline), advanceX: t(size * 0.5), advanceY: 0, cluster: i))
                clusters.append(RenderingV2.Cluster(textStartByte: i, textEndByte: i + 1, hitRects: [], carets: [], sources: nil))
            }
            return .glyphRun(RenderingV2.GlyphRun(fontId: "f", fontSize: t(size), text: text, glyphs: glyphs, clusters: clusters, paint: .black))
        }
        // "AV" kerned into two runs 0.5 pt apart; "office" then a 3.3 pt glue; a superscript
        // 4 pt above the baseline is its own line; item order is not reading order.
        let page = RenderingV2.Page(number: 1, width: 1, height: 1, items: [
            run("office", x: 30),
            run("A", x: 10), run("V", x: 15.5),
            run("2", x: 70, size: 6, baseline: 96),
            run("below", x: 10, baseline: 120),
        ])
        let lines = V2PageText.lines(of: page)
        XCTAssertEqual(lines.map(\.text), ["2", "AV office", "below"])
        XCTAssertEqual(lines[1].words.map(\.itemIndex), [1, 2, 0], "sorted by x, not item order")
    }

    // MARK: PageV2AXView, read back through the NSAccessibility protocol

    /// Hosts one `PageV2AXView` in an ordered-out window (never key) at the
    /// page's size, as the v2 pane does, and hands the page to it.
    private func hostPage(_ page: RenderingV2.Page, token: String = "t1", totalPages: Int, scale: CGFloat = 1,
                          onSelect: @escaping (V2Geometry.Hit) -> Void = { _ in }) -> PageV2AXView {
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: page.widthPt * scale, height: page.heightPt * scale),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        let view = PageV2AXView(frame: window.contentView!.bounds)
        window.contentView!.addSubview(view)
        window.orderFrontRegardless()
        windows.append(window)
        view.update(page: page, pageToken: token, totalPages: totalPages, scale: scale, onSelect: onSelect)
        return view
    }

    func testV2PageIsALandmarkWhoseValueIsItsTextAndLinesAreItsChildren() throws {
        let page = try XCTUnwrap(textEnvelope().payload.pages.first)
        let view = hostPage(page, totalPages: 3, scale: 0.5)
        XCTAssertFalse(view.hasBuiltTree, "nothing is built until an assistive client asks")
        XCTAssertTrue(view.isAccessibilityElement())
        XCTAssertEqual(view.accessibilityRole(), .group)
        XCTAssertEqual(view.accessibilitySubrole(), PreviewAccessibility.landmarkSubrole)
        XCTAssertEqual(view.accessibilityRoleDescription(), "page")
        XCTAssertEqual(view.accessibilityLabel(), "Page 1 of 3, 2 lines")
        XCTAssertEqual(view.accessibilityValue() as? String, "Office fixtures\nThe AV office fixed the fi ligature: office, bold, and café.")
        XCTAssertFalse(view.hasBuiltTree, "the label and value alone do not build the tree")
        XCTAssertNil(view.hitTest(NSPoint(x: 10, y: 10)), "never hit-tested: clicks reach the page")

        let lines = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertTrue(view.hasBuiltTree)
        XCTAssertEqual(lines.map { $0.accessibilityRole() }, [.staticText, .staticText])
        XCTAssertEqual(lines.map { $0.accessibilityRoleDescription() }, ["line", "line"])
        XCTAssertEqual(lines.map { $0.accessibilityLabel() },
                       ["Page 1, line 1: Office fixtures", "Page 1, line 2: The AV office fixed the fi ligature: office, bold, and café."])
        XCTAssertEqual(lines.map { $0.accessibilityValue() as? String }, V2PageText.lines(of: page).map(\.text))
        XCTAssertEqual(lines.map { $0.accessibilityHelp() }, ["Page 1, line 1", "Page 1, line 2"])
        XCTAssertTrue(lines.allSatisfy { $0.accessibilityParent() as AnyObject === view })
        let nav = try XCTUnwrap(view.accessibilityChildrenInNavigationOrder())
        XCTAssertEqual(nav.count, lines.count)
        XCTAssertTrue(zip(nav, lines).allSatisfy { $0 as AnyObject === $1 }, "navigation order is the children array")
        // Frames follow the scale and are converted through the flipped view.
        let model = V2PageText.lines(of: page)
        XCTAssertEqual(lines[1].viewFrame.minX, model[1].rect.minX * 0.5, accuracy: 0.001)
        XCTAssertEqual(lines[1].viewFrame.width, model[1].rect.width * 0.5, accuracy: 0.001)
        XCTAssertEqual(lines[1].accessibilityFrame(), NSAccessibility.screenRect(fromView: view, rect: lines[1].viewFrame))
        XCTAssertLessThan(lines[0].viewFrame.minY, lines[1].viewFrame.minY, "the heading sits above the body line")
    }

    func testV2PageTreeSurvivesAnUnchangedTokenAndIsRebuiltForANewOne() throws {
        let page = try XCTUnwrap(textEnvelope().payload.pages.first)
        let view = hostPage(page, totalPages: 1)
        let first = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        // A keystroke that left this page's bytes alone keeps the same token: same tree.
        view.update(page: page, pageToken: "t1", totalPages: 1, scale: 1, onSelect: { _ in })
        XCTAssertTrue(view.hasBuiltTree)
        let same = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertTrue(zip(first, same).allSatisfy { $0 === $1 })
        // A new token (page content changed) or a new page count drops it.
        view.update(page: page, pageToken: "t2", totalPages: 1, scale: 1, onSelect: { _ in })
        XCTAssertFalse(view.hasBuiltTree)
        XCTAssertEqual(view.accessibilityLabel(), "Page 1 of 1, 2 lines")
        _ = view.accessibilityChildren()
        view.update(page: page, pageToken: "t2", totalPages: 4, scale: 1, onSelect: { _ in })
        XCTAssertFalse(view.hasBuiltTree)
        XCTAssertEqual(view.accessibilityLabel(), "Page 1 of 4, 2 lines")
    }

    func testGoToSourceActionForwardsTheLinesFirstSourcedWord() throws {
        let page = try XCTUnwrap(textEnvelope().payload.pages.first)
        var received: [V2Geometry.Hit] = []
        let view = hostPage(page, totalPages: 1) { received.append($0) }
        let lines = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        let body = lines[1]
        let action = try XCTUnwrap(body.accessibilityCustomActions()?.first)
        XCTAssertEqual(action.name, PreviewAccessibility.goToSourceAction)
        XCTAssertTrue(action.handler?() ?? false)
        XCTAssertEqual(received.count, 1)
        let hit = try XCTUnwrap(received.first)
        XCTAssertEqual(hit.text, "The")
        XCTAssertEqual(hit.sources, [RenderingV2.SourceRange(path: "main.tex", startByte: 67, endByte: 70)], "the whole word, one span")
        XCTAssertNil(hit.syntheticReason)
        // The same shape a click produces: the pane's `navigateV2` accepts it unchanged.
        XCTAssertEqual(hit.clusterIndex, 0)
        XCTAssertEqual(hit.itemIndex, V2PageText.lines(of: page)[1].words[0].itemIndex)
    }

    func testElidedPageSaysNotLoadedAndHasNoLines() throws {
        var page = try XCTUnwrap(textEnvelope().payload.pages.first)
        page.resident = false
        let view = hostPage(page, totalPages: 12)
        XCTAssertEqual(view.accessibilityLabel(), "Page 1 of 12, not loaded")
        XCTAssertEqual(view.accessibilityValue() as? String, "")
        XCTAssertEqual((view.accessibilityChildren() ?? []).count, 0)
    }

    // MARK: the hosted v2 pane

    func testHostedV2PaneExposesOnePageLandmarkPerPage() async throws {
        let hosting = host(PreviewV2View(frame: try frame(pages: 3), dark: false, stale: false, caretPath: "main.tex", caretByte: nil) { _ in },
                           width: 400, height: 2000) // tall enough for the lazy stack to create all three
        try await settle()
        let pages = Self.findAll(PageV2AXView.self, in: hosting)
        XCTAssertEqual(pages.count, 3, "one accessibility view per page")
        XCTAssertEqual(pages.compactMap { $0.accessibilityLabel() }.sorted(), ["Page 1 of 3, 2 lines", "Page 2 of 3, 2 lines", "Page 3 of 3, 2 lines"])
        XCTAssertTrue(pages.allSatisfy { $0.accessibilitySubrole() == PreviewAccessibility.landmarkSubrole })
        XCTAssertTrue(pages.allSatisfy { $0.window != nil && !$0.isHiddenOrHasHiddenAncestor })
        // Sized to the drawn page: the fit-to-width scale times the page's points.
        let scale = PreviewPageLayout.fitScale(paneWidth: 400, widestPt: 612)
        let expected = NSSize(width: 612 * scale, height: 792 * scale)
        for p in pages {
            XCTAssertEqual(p.frame.width, expected.width, accuracy: 1)
            XCTAssertEqual(p.frame.height, expected.height, accuracy: 1)
        }
        XCTAssertTrue(pages.allSatisfy { !$0.hasBuiltTree }, "no tree is built unless VoiceOver reads a page")
    }

    // MARK: Page Up / Page Down

    func testPageStepTargetsWholePages() {
        let layout = PreviewPageLayout(pages: (1...3).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }, scale: 0.5)
        XCTAssertEqual(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 1, fraction: 0, horizontal: 0), layout: layout), 2)
        XCTAssertEqual(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 2, fraction: 0.7, horizontal: 0), layout: layout), 3)
        XCTAssertNil(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 3, fraction: 0.2, horizontal: 0), layout: layout), "already on the last page")
        XCTAssertNil(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 1, fraction: 0, horizontal: 0), layout: layout), "first page at its top")
        XCTAssertEqual(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 2, fraction: 0, horizontal: 0), layout: layout), 1)
        XCTAssertEqual(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 2, fraction: 0.005, horizontal: 0), layout: layout), 1, "within the slack: previous page")
        XCTAssertEqual(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 2, fraction: 0.5, horizontal: 0), layout: layout), 2, "mid-page: this page's top first")
        XCTAssertNil(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 9, fraction: 0, horizontal: 0), layout: layout), "unknown page")
    }

    func testPageDownAndPageUpStepThePaneBetweenPagesAndReportTheLanding() async throws {
        let hosting = host(PreviewV2View(frame: try frame(pages: 3), dark: false, stale: false, caretPath: "main.tex", caretByte: nil) { _ in },
                           width: 500, height: 400)
        try await settle()
        let scroll = try XCTUnwrap(PreviewAnchoringTests.find(NSScrollView.self, in: hosting))
        let probe = try XCTUnwrap(PreviewAnchoringTests.find(PreviewAnchorProbe.self, in: hosting))
        probe.reduceMotion = { true } // instant scrolls: the position is measurable right after the jump
        var landed: [Int] = []
        probe.onPageJump = { landed.append($0) }
        let layout = try XCTUnwrap(probe.layout)
        XCTAssertEqual(probe.anchor?.page, 1)

        probe.jump(PreviewPageJump(token: 1, step: .down))
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), try XCTUnwrap(layout.frame(of: 2)).minY, accuracy: 0.5)
        XCTAssertEqual(probe.anchor?.page, 2)
        probe.jump(PreviewPageJump(token: 1, step: .down))
        XCTAssertEqual(probe.anchor?.page, 2, "the same token is acted on once")
        probe.jump(PreviewPageJump(token: 2, step: .down))
        XCTAssertEqual(probe.anchor?.page, 3)
        // The last page cannot reach the top of a viewport shorter than the document's tail: clamped to the end.
        let end = max(0, (scroll.documentView?.bounds.height ?? 0) - scroll.contentView.bounds.height)
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), min(try XCTUnwrap(layout.frame(of: 3)).minY, end), accuracy: 0.5)
        probe.jump(PreviewPageJump(token: 3, step: .down))
        XCTAssertEqual(landed, [2, 3], "a step with nowhere to go reports nothing")

        // Up from mid-page 3 goes to its top; up again to page 2; the reader hears each landing.
        PreviewAnchoringTests.scroll(scroll, toTop: try XCTUnwrap(layout.frame(of: 2)).minY + 60)
        try await settle(0.1)
        XCTAssertEqual(probe.anchor?.page, 2)
        probe.jump(PreviewPageJump(token: 4, step: .up))
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), try XCTUnwrap(layout.frame(of: 2)).minY, accuracy: 0.5)
        probe.jump(PreviewPageJump(token: 5, step: .up))
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), try XCTUnwrap(layout.frame(of: 1)).minY, accuracy: 0.5)
        XCTAssertEqual(probe.anchor?.page, 1)
        XCTAssertEqual(landed, [2, 3, 2, 1])
    }

    // MARK: announcements

    func testAnnouncerSpeaksStatusFlipsAtOnceAndCoalescesSameStatusResults() {
        typealias S = PreviewAnnouncer.State
        XCTAssertEqual(PreviewAnnouncer.decide(previous: nil, next: S(revision: 1, status: .ok, pages: 2, errors: 0)), .now)
        XCTAssertEqual(PreviewAnnouncer.decide(previous: S(revision: 1, status: .ok, pages: 2, errors: 0), next: S(revision: 2, status: .ok, pages: 3, errors: 0)), .wait)
        XCTAssertEqual(PreviewAnnouncer.decide(previous: S(revision: 1, status: .ok, pages: 2, errors: 0), next: S(revision: 2, status: .failed, pages: 0, errors: 1)), .now)
        XCTAssertEqual(PreviewAnnouncer.decide(previous: S(revision: 2, status: .failed, pages: 0, errors: 1), next: S(revision: 3, status: .ok, pages: 2, errors: 0)), .now)
        XCTAssertEqual(PreviewAnnouncer.decide(previous: S(revision: 1, status: .ok, pages: 2, errors: 0), next: S(revision: 1, status: .ok, pages: 2, errors: 0)), .drop, "the same result again")
        XCTAssertEqual(S(revision: 1, status: .ok, pages: 1, errors: 0).message, "Preview updated: 1 page")
        XCTAssertEqual(S(revision: 1, status: .recovered, pages: 3, errors: 2).message, "Preview updated with 2 errors recovered: 3 pages")
        XCTAssertEqual(S(revision: 1, status: .failed, pages: 0, errors: 1).message, "Compile failed: 1 error")
        XCTAssertEqual(S(revision: 1, status: .failed, pages: 0, errors: 1).priority, .high)
        XCTAssertEqual(S(revision: 1, status: .ok, pages: 1, errors: 0).priority, .low)

        let announcer = PreviewAnnouncer()
        announcer.quietInterval = 0.05
        var posted: [(String, NSAccessibilityPriorityLevel)] = []
        announcer.post = { posted.append(($0, $1)) }
        announcer.note(S(revision: 1, status: .ok, pages: 2, errors: 0))
        XCTAssertEqual(posted.map(\.0), ["Preview updated: 2 pages"], "the first result is spoken at once")
        // Typing under auto-compile: three same-status results inside the quiet interval speak once, the latest.
        announcer.note(S(revision: 2, status: .ok, pages: 2, errors: 0))
        announcer.note(S(revision: 3, status: .ok, pages: 3, errors: 0))
        announcer.note(S(revision: 4, status: .ok, pages: 4, errors: 0))
        XCTAssertEqual(posted.count, 1, "nothing until the quiet interval passes")
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        XCTAssertEqual(posted.map(\.0), ["Preview updated: 2 pages", "Preview updated: 4 pages"])
        XCTAssertEqual(announcer.spoken?.revision, 4)
        // A failure interrupts immediately, at high priority; the recovery too.
        announcer.note(S(revision: 5, status: .failed, pages: 0, errors: 2))
        XCTAssertEqual(posted.last?.0, "Compile failed: 2 errors")
        XCTAssertEqual(posted.last?.1, .high)
        announcer.note(S(revision: 6, status: .failed, pages: 0, errors: 2))
        XCTAssertEqual(posted.count, 3, "a failure after a failure waits")
        announcer.note(S(revision: 7, status: .ok, pages: 4, errors: 0))
        XCTAssertEqual(posted.last?.0, "Preview updated: 4 pages")
        XCTAssertEqual(posted.count, 4, "the pending failure repeat was superseded, not spoken")
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        XCTAssertEqual(posted.count, 4)
        XCTAssertEqual(announcer.announcements, posted.map(\.0))
        // Page jumps and refusals are spoken directly.
        announcer.notePageJump(page: 2, of: 4)
        XCTAssertEqual(posted.last?.0, "Page 2 of 4")
        announcer.noteRefusal(RenderingV2.ValidationError(code: "font_unavailable", message: "no font for hash abc"))
        XCTAssertEqual(posted.last?.0, "Preview display list refused: no font for hash abc")
        XCTAssertEqual(posted.last?.1, .high)
    }

    func testShellAnnouncesAnAppliedResultOnceAndClearingIsQuiet() throws {
        let model = ShellModel()
        var posted: [String] = []
        model.previewAnnouncer.post = { message, _ in posted.append(message) }
        model.loadFixtures(request: Self.samples.appendingPathComponent("multipage-request.json"),
                           result: Self.samples.appendingPathComponent("multipage-result.json"))
        XCTAssertNotNil(model.result)
        // multipage-result.json: status recovered, one error and one warning, two pages.
        XCTAssertEqual(posted, ["Preview updated with 1 error recovered: 2 pages"])
        model.loadFixtures(request: Self.samples.appendingPathComponent("multipage-request.json"),
                           result: Self.samples.appendingPathComponent("multipage-result.json"))
        XCTAssertEqual(posted.count, 1, "the same revision and status again is not repeated")
        model.result = nil
        XCTAssertEqual(posted.count, 1, "clearing the preview says nothing")
    }

    func testRefusedDisplayListIsAnnouncedOnceUntilAFrameVerifies() throws {
        let model = ShellModel()
        var posted: [String] = []
        model.previewAnnouncer.post = { message, _ in posted.append(message) }
        let refusal = RenderingV2.ValidationError(code: "font_unavailable", message: "no font")
        model.displayListV2 = .failed(refusal, .file(URL(fileURLWithPath: "/tmp/a.json")))
        XCTAssertEqual(posted, ["Preview display list refused: no font"])
        model.displayListV2 = .failed(refusal, .file(URL(fileURLWithPath: "/tmp/b.json")))
        XCTAssertEqual(posted.count, 1, "a refusal after a refusal is quiet")
        model.displayListV2 = .loaded(try frame(pages: 1), .file(URL(fileURLWithPath: "/tmp/c.json")))
        model.displayListV2 = .failed(refusal, .file(URL(fileURLWithPath: "/tmp/d.json")))
        XCTAssertEqual(posted.count, 2, "a refusal after a verified frame is spoken again")
    }

    // MARK: the pane's container and keyboard focus, pinned at the source

    func testPreviewPaneIsALabelledContainerAndBothPanesTakeFocusForPageKeys() throws {
        XCTAssertEqual(PreviewPane.accessibilityLabel, "PDF preview")
        XCTAssertEqual(PreviewPane.accessibilityValue(page: 2, of: 5), "Page 2 of 5")
        XCTAssertEqual(PreviewPane.accessibilityValue(page: 9, of: 5), "Page 5 of 5", "clamped like the HUD readout")
        XCTAssertNil(PreviewPane.accessibilityValue(page: 1, of: 0), "no pages, no value")

        let content = try String(contentsOf: Self.sources.appendingPathComponent("FlashTeXMac/ContentView.swift"), encoding: .utf8)
        let pane = try XCTUnwrap(content.range(of: "struct PreviewPane: View {"))
        let paneBody = content[pane.lowerBound...]
        let next = paneBody.dropFirst().range(of: "\nstruct ")?.lowerBound ?? paneBody.endIndex
        let paneSource = paneBody[..<next]
        XCTAssertTrue(paneSource.contains(".accessibilityElement(children: .contain)"), "the pane is one container")
        XCTAssertTrue(paneSource.contains(".accessibilityLabel(Self.accessibilityLabel)"))
        XCTAssertTrue(paneSource.contains(".accessibilityValue(Self.accessibilityValue(page: model.previewVisiblePage, of: model.toolbarPageCount)"))
        XCTAssertTrue(paneSource.contains("onPageJump: { model.previewAnnouncer.notePageJump("), "the v1 pane's page jumps are announced")

        for file in ["FlashTeXMac/PreviewView.swift", "FlashTeXMac/PreviewV2View.swift"] {
            let source = try String(contentsOf: Self.sources.appendingPathComponent(file), encoding: .utf8)
            XCTAssertTrue(source.contains(".focusable()"), "\(file): the pane takes keyboard focus")
            XCTAssertTrue(source.contains(".onKeyPress(.pageDown)"), "\(file): Page Down steps pages")
            XCTAssertTrue(source.contains(".onKeyPress(.pageUp)"), "\(file): Page Up steps pages")
            XCTAssertTrue(source.contains("pageJump: pageJump, onPageJump: onPageJump"), "\(file): the keeper carries the jump")
        }
        let v2 = try String(contentsOf: Self.sources.appendingPathComponent("FlashTeXMac/PreviewV2View.swift"), encoding: .utf8)
        XCTAssertTrue(v2.contains("onPageJump: { model.previewAnnouncer.notePageJump("), "the v2 pane's page jumps are announced")
        XCTAssertTrue(v2.contains("PageV2AccessibilityOverlay(page: page, pageToken: pageToken, totalPages: totalPages"), "every v2 page carries the overlay")

        // The help window and the focus-order table describe what ships.
        let preview = try XCTUnwrap(FocusOrder.panes.first { $0.name == "Preview" })
        XCTAssertTrue(preview.contents.contains("“PDF preview”"))
        XCTAssertTrue(preview.contents.contains("Page Down / Page Up"))
        let note = try XCTUnwrap(AccessibilityHelpView.voiceOverNotes.first { $0.hasPrefix("Preview:") })
        XCTAssertTrue(note.contains("“PDF preview”"))
        XCTAssertTrue(note.contains("Page Down / Page Up"))
        XCTAssertTrue(note.contains("Preview updated: 3 pages"))
    }
}
