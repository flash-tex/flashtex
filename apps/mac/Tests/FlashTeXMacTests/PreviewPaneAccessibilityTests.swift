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

    func testV2PageTreeIsUpdatedInPlaceAcrossZoomPageCountAndANewPageOfTheSameShape() throws {
        let page = try XCTUnwrap(textEnvelope().payload.pages.first)
        let view = hostPage(page, totalPages: 1)
        let first = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertEqual(view.rebuilds, 1)
        XCTAssertEqual(view.layoutChangesPosted, 0)
        let frames = first.map(\.viewFrame)
        // A keystroke that left this page's bytes alone keeps the same token: nothing changes, nothing is posted.
        view.update(page: page, pageToken: "t1", totalPages: 1, scale: 1, onSelect: { _ in })
        XCTAssertTrue(view.hasBuiltTree)
        XCTAssertTrue(zip(first, try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])).allSatisfy { $0 === $1 })
        XCTAssertEqual(view.layoutChangesPosted, 0)
        // Zoom: the same elements move; VoiceOver's cursor on a line survives.
        view.update(page: page, pageToken: "t1", totalPages: 1, scale: 2, onSelect: { _ in })
        let zoomed = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertTrue(zip(first, zoomed).allSatisfy { $0 === $1 }, "zoom keeps the element objects")
        XCTAssertEqual(view.rebuilds, 1)
        for (ax, before) in zip(zoomed, frames) {
            XCTAssertEqual(ax.viewFrame.minX, before.minX * 2, accuracy: 0.001)
            XCTAssertEqual(ax.viewFrame.width, before.width * 2, accuracy: 0.001)
            XCTAssertEqual(ax.accessibilityFrameInParentSpace().origin.x, before.minX * 2, accuracy: 0.001, "parent-space frame follows")
        }
        XCTAssertEqual(zoomed.map { $0.accessibilityLabel() }, first.map { $0.accessibilityLabel() })
        XCTAssertEqual(view.layoutChangesPosted, 1, "one layoutChanged for the move")
        // The page count changes only the landmark's own label: no element changes, nothing is posted.
        view.update(page: page, pageToken: "t1", totalPages: 4, scale: 2, onSelect: { _ in })
        XCTAssertEqual(view.accessibilityLabel(), "Page 1 of 4, 2 lines")
        XCTAssertTrue(zip(first, try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])).allSatisfy { $0 === $1 })
        XCTAssertEqual(view.layoutChangesPosted, 1)
        XCTAssertEqual(view.rebuilds, 1)
        // A new page with the same line count (an edit inside a line): the same
        // elements are relabelled and re-armed, one layoutChanged.
        var edited = page
        for i in edited.items.indices {
            guard case .glyphRun(var run) = edited.items[i], run.text == "office" else { continue }
            run.text = "kitchen"
            edited.items[i] = .glyphRun(run)
        }
        var received: [V2Geometry.Hit] = []
        view.update(page: edited, pageToken: "t2", totalPages: 4, scale: 2, onSelect: { received.append($0) })
        let relabelled = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertTrue(zip(first, relabelled).allSatisfy { $0 === $1 }, "same objects")
        XCTAssertEqual(relabelled[1].accessibilityValue() as? String, "The AV kitchen fixed the fi ligature: kitchen, bold, and café.")
        XCTAssertEqual(relabelled[1].accessibilityLabel(), "Page 1, line 2: The AV kitchen fixed the fi ligature: kitchen, bold, and café.")
        XCTAssertEqual(view.accessibilityValue() as? String, "Office fixtures\nThe AV kitchen fixed the fi ligature: kitchen, bold, and café.")
        XCTAssertEqual(view.layoutChangesPosted, 2)
        XCTAssertEqual(view.rebuilds, 1)
        XCTAssertTrue(try XCTUnwrap(relabelled[1].accessibilityCustomActions()?.first).handler?() ?? false)
        XCTAssertEqual(received.map(\.text), ["The"], "the action now goes to the new onSelect")
        // A new page with another line count is the one case that rebuilds.
        var shorter = edited
        shorter.items = shorter.items.filter { if case .glyphRun(let r) = $0 { return r.text != "Office" && r.text != "fixtures" } else { return true } }
        view.update(page: shorter, pageToken: "t3", totalPages: 4, scale: 2, onSelect: { _ in })
        XCTAssertFalse(view.hasBuiltTree)
        XCTAssertEqual(view.layoutChangesPosted, 3)
        let rebuilt = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertEqual(rebuilt.count, 1)
        XCTAssertEqual(view.rebuilds, 2)
        XCTAssertEqual(view.accessibilityLabel(), "Page 1 of 4, 1 line")
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

    // MARK: the Pages rotor

    func testPagesRotorSearchFollowsAppKitSemantics() {
        let layout = PreviewPageLayout(pages: (1...4).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }, scale: 0.5)
        let items = PreviewPagesRotor.items(layout: layout, elided: [3])
        XCTAssertEqual(items.map(\.label), ["Page 1 of 4", "Page 2 of 4", "Page 3 of 4, not loaded", "Page 4 of 4"])
        XCTAssertEqual(PreviewPagesRotor.label(number: 2, totalPages: 0, elided: false), "Page 2")
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: ""), items[0])
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: false, filter: ""), items[3])
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: 2, forward: true, filter: ""), items[2], "strictly after the current page")
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: 2, forward: false, filter: ""), items[0])
        XCTAssertNil(PreviewPagesRotor.resolve(items: items, start: 4, forward: true, filter: ""), "no wrap-around")
        XCTAssertNil(PreviewPagesRotor.resolve(items: items, start: 1, forward: false, filter: ""))
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: "not loaded"), items[2], "type-ahead on the label")
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: "PAGE 4"), items[3])
        XCTAssertNil(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: "page 9"))
        // A current item names its page by its target view or by its loading token.
        let token = NSAccessibilityCustomRotor.SearchParameters()
        token.currentItem = NSAccessibilityCustomRotor.ItemResult(itemLoadingToken: NSNumber(value: 3), customLabel: "Page 3 of 4")
        XCTAssertEqual(PreviewPagesRotor.start(of: token), 3)
        XCTAssertNil(PreviewPagesRotor.start(of: NSAccessibilityCustomRotor.SearchParameters()))
    }

    /// A short host: the lazy stack builds only the page(s) in view, so the
    /// Landmarks rotor would list one page of six. The Pages rotor lists all
    /// six from every page view and line element, marks the elided one, and
    /// loading a chosen page scrolls there and hands back its view once the
    /// stack built it.
    func testPagesRotorListsEveryPageOfAShortHostAndLoadsAChosenOffscreenPage() async throws {
        var frame = try frame(pages: 6)
        frame.list.pages[4].resident = false // a windowed frame: page 5 elided
        let hosting = host(PreviewV2View(frame: frame, dark: false, stale: false, caretPath: "main.tex", caretByte: nil) { _ in },
                           width: 500, height: 400)
        try await settle()
        let scroll = try XCTUnwrap(PreviewAnchoringTests.find(NSScrollView.self, in: hosting))
        let probe = try XCTUnwrap(PreviewAnchoringTests.find(PreviewAnchorProbe.self, in: hosting))
        probe.reduceMotion = { true }
        var landed: [Int] = []
        probe.onPageJump = { landed.append($0) }
        let layout = try XCTUnwrap(probe.layout)
        XCTAssertEqual(layout.pages.count, 6)
        XCTAssertEqual(probe.elidedPages, [5])
        let built = Self.findAll(PageV2AXView.self, in: hosting)
        XCTAssertLessThan(built.count, 6, "the lazy stack did not build every page (\(built.count) built)")
        let first = try XCTUnwrap(built.first { $0.previewPageNumber == 1 })

        // Every page view and every line element offer the pane's one Pages rotor.
        let rotors = first.accessibilityCustomRotors()
        XCTAssertEqual(rotors.map(\.label), ["Pages"])
        XCTAssertTrue(rotors.first === probe.pagesRotor.rotors.first)
        let line = try XCTUnwrap((first.accessibilityChildren() as? [PreviewAXElement])?.first)
        XCTAssertTrue(line.accessibilityCustomRotors().first === rotors.first)
        let rotor = try XCTUnwrap(rotors.first)
        let delegate = try XCTUnwrap(rotor.itemSearchDelegate)

        // Walking next from the ends lists all six pages, the elided one marked, none scrolled to.
        func params(_ current: NSAccessibilityCustomRotor.ItemResult?, forward: Bool, filter: String = "") -> NSAccessibilityCustomRotor.SearchParameters {
            let p = NSAccessibilityCustomRotor.SearchParameters()
            p.currentItem = current; p.searchDirection = forward ? .next : .previous; p.filterString = filter
            return p
        }
        var results: [NSAccessibilityCustomRotor.ItemResult] = []
        var current: NSAccessibilityCustomRotor.ItemResult?
        while let next = delegate.rotor(rotor, resultFor: params(current, forward: true)) { results.append(next); current = next }
        XCTAssertEqual(results.map(\.customLabel), ["Page 1 of 6", "Page 2 of 6", "Page 3 of 6", "Page 4 of 6", "Page 5 of 6, not loaded", "Page 6 of 6"])
        XCTAssertTrue(results[0].targetElement as AnyObject === first, "a built page is the target itself")
        XCTAssertNil(results[0].itemLoadingToken)
        XCTAssertNil(results[3].targetElement, "an unbuilt page is a loading token, not a view")
        XCTAssertEqual(results[3].itemLoadingToken as? NSNumber, 4)
        XCTAssertEqual(probe.pagesRotor.loads, [], "listing never loads or scrolls")
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), 0, accuracy: 0.5)
        XCTAssertEqual(delegate.rotor(rotor, resultFor: params(nil, forward: false))?.customLabel, "Page 6 of 6")
        XCTAssertEqual(delegate.rotor(rotor, resultFor: params(results[2], forward: false))?.customLabel, "Page 2 of 6")
        XCTAssertEqual(delegate.rotor(rotor, resultFor: params(nil, forward: true, filter: "4"))?.customLabel, "Page 4 of 6")
        XCTAssertNil(delegate.rotor(rotor, resultFor: params(results[5], forward: true)))

        // Choosing page 4 loads it: the pane scrolls there (announced like Page Down),
        // and the element handed back is the page's view — or a stand-in at its place
        // until the lazy stack builds it, after which the real view is the answer.
        let loaded = try XCTUnwrap(first.accessibilityElement(withToken: NSNumber(value: 4)))
        XCTAssertEqual(probe.pagesRotor.loads, [4])
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), try XCTUnwrap(layout.frame(of: 4)).minY, accuracy: 0.5)
        XCTAssertEqual(landed, [4])
        let loadedLabel = (loaded as? NSAccessibilityProtocol)?.accessibilityLabel()
        XCTAssertEqual(loadedLabel?.hasPrefix("Page 4 of 6"), true, String(describing: loadedLabel))
        if let standIn = loaded as? PreviewAXElement {
            XCTAssertEqual(standIn.accessibilityRole(), .group)
            XCTAssertEqual(standIn.accessibilitySubrole(), PreviewAccessibility.landmarkSubrole)
            XCTAssertEqual(standIn.viewFrame, try XCTUnwrap(layout.frame(of: 4)), "the stand-in sits where the page is")
        }
        try await settle()
        let page4 = try XCTUnwrap(Self.findAll(PageV2AXView.self, in: hosting).first { $0.previewPageNumber == 4 }, "the stack built page 4 after the scroll")
        XCTAssertTrue(probe.pagesRotor.pageView(4) === page4)
        XCTAssertTrue(first.accessibilityElement(withToken: NSNumber(value: 4)) as AnyObject === page4, "loaded again: the real view")
        XCTAssertTrue(delegate.rotor(rotor, resultFor: params(nil, forward: true, filter: "4"))?.targetElement as AnyObject === page4, "and the rotor now targets it directly")
        // A line element loads through the same path; the elided page is reachable too.
        let page4Line = try XCTUnwrap((page4.accessibilityChildren() as? [PreviewAXElement])?.first)
        _ = page4Line.accessibilityElement(withToken: NSNumber(value: 5))
        XCTAssertEqual(probe.pagesRotor.loads, [4, 4, 5])
        XCTAssertEqual(landed, [4, 4, 5])
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), min(try XCTUnwrap(layout.frame(of: 5)).minY, max(0, (scroll.documentView?.bounds.height ?? 0) - scroll.contentView.bounds.height)), accuracy: 0.5)
        XCTAssertNil(first.accessibilityElement(withToken: "not a page" as NSString), "an unknown token loads nothing")
    }

    // MARK: announcements

    func testAnnouncerSpeaksTheFirstResultAtOnceAndCoalescesEverythingAfterNewestWins() {
        typealias S = PreviewAnnouncer.State
        XCTAssertEqual(PreviewAnnouncer.decide(previous: nil, next: S(revision: 1, status: .ok, pages: 2, errors: 0)), .now)
        XCTAssertEqual(PreviewAnnouncer.decide(previous: S(revision: 1, status: .ok, pages: 2, errors: 0), next: S(revision: 2, status: .ok, pages: 3, errors: 0)), .wait)
        XCTAssertEqual(PreviewAnnouncer.decide(previous: S(revision: 1, status: .ok, pages: 2, errors: 0), next: S(revision: 2, status: .failed, pages: 0, errors: 1)), .wait, "a status flip waits too")
        XCTAssertEqual(PreviewAnnouncer.decide(previous: S(revision: 2, status: .failed, pages: 0, errors: 1), next: S(revision: 3, status: .ok, pages: 2, errors: 0)), .wait)
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
        // Typing through a brace: failed, ok, failed, ok on consecutive keystrokes
        // is one announcement of the newest state, not four.
        announcer.note(S(revision: 5, status: .failed, pages: 0, errors: 2))
        announcer.note(S(revision: 6, status: .ok, pages: 4, errors: 0))
        announcer.note(S(revision: 7, status: .failed, pages: 0, errors: 1))
        announcer.note(S(revision: 8, status: .ok, pages: 5, errors: 0))
        XCTAssertEqual(posted.count, 2, "flips coalesce like everything else")
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        XCTAssertEqual(posted.last?.0, "Preview updated: 5 pages")
        XCTAssertEqual(posted.count, 3)
        // A failure that lasts is spoken once the typing pauses, at high priority.
        announcer.note(S(revision: 9, status: .failed, pages: 0, errors: 1))
        announcer.note(S(revision: 10, status: .failed, pages: 0, errors: 2))
        XCTAssertEqual(posted.count, 3)
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        XCTAssertEqual(posted.last?.0, "Compile failed: 2 errors")
        XCTAssertEqual(posted.last?.1, .high)
        XCTAssertEqual(posted.count, 4)
        // Nothing pending: the quiet interval passing again says nothing.
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        XCTAssertEqual(posted.count, 4)
        XCTAssertEqual(announcer.announcements, posted.map(\.0))
        // Page jumps and file refusals are spoken directly.
        announcer.notePageJump(page: 2, of: 4)
        XCTAssertEqual(posted.last?.0, "Page 2 of 4")
        announcer.noteRefusal(RenderingV2.ValidationError(code: "font_unavailable", message: "no font for hash abc"))
        XCTAssertEqual(posted.last?.0, "Preview display list refused: no font for hash abc")
        XCTAssertEqual(posted.last?.1, .high)
        // A live refusal is quiet and deduped until a frame verifies.
        let missing = RenderingV2.ValidationError(code: "font_unavailable", message: "no font for hash abc")
        announcer.noteLiveRefusal(missing)
        XCTAssertEqual(posted.last?.0, "Preview not updated: no font for hash abc")
        XCTAssertEqual(posted.last?.1, .low)
        let count = posted.count
        announcer.noteLiveRefusal(missing)
        announcer.noteLiveRefusal(missing)
        XCTAssertEqual(posted.count, count, "the same live refusal again is silent")
        // Ordering: a result whose live sibling is then refused never becomes
        // "Preview updated" — the refusal withdraws the pending update.
        announcer.note(S(revision: 11, status: .ok, pages: 6, errors: 0))
        announcer.noteLiveRefusal(RenderingV2.ValidationError(code: "correlation_mismatch", message: "wrong revision"))
        XCTAssertEqual(posted.last?.0, "Preview not updated: wrong revision")
        let afterRefusal = posted.count
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        XCTAssertEqual(posted.count, afterRefusal, "no 'Preview updated' follows a refusal")
        XCTAssertEqual(announcer.spoken?.revision, 10, "the refused revision was never spoken")
        // The next result is judged afresh and, being new, is spoken after the quiet interval.
        announcer.note(S(revision: 12, status: .ok, pages: 6, errors: 0))
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        XCTAssertEqual(posted.last?.0, "Preview updated: 6 pages")
        XCTAssertEqual(posted.count, afterRefusal + 1)
        announcer.noteLiveRefusal(RenderingV2.ValidationError(code: "source_mismatch", message: "sha differs"))
        XCTAssertEqual(posted.last?.0, "Preview not updated: sha differs")
        announcer.noteFrameVerified()
        announcer.noteLiveRefusal(missing)
        XCTAssertEqual(posted.last?.0, "Preview not updated: no font for hash abc")
        XCTAssertEqual(posted.count, count + 4) // the two live refusals plus the ordering block above
    }

    func testShellAnnouncesAnAppliedResultOnceAndClearingIsQuiet() throws {
        let model = ShellModel()
        // `ShellModel()` applies protocol/fixtures/compile-result.json at init (revision 1,
        // ok, one page): that first result was spoken at once, to the default poster.
        XCTAssertEqual(model.previewAnnouncer.announcements, ["Preview updated: 1 page"])
        var posted: [String] = []
        model.previewAnnouncer.post = { message, _ in posted.append(message) }
        model.loadFixtures(request: Self.samples.appendingPathComponent("multipage-request.json"),
                           result: Self.samples.appendingPathComponent("multipage-result.json"))
        XCTAssertNotNil(model.result)
        // multipage-result.json: status recovered, one error and one warning, two pages —
        // a later result, so it waits for the quiet interval (newest wins) and then speaks.
        XCTAssertEqual(posted, [], "a later result is coalesced, not spoken at once")
        model.previewAnnouncer.flush()
        XCTAssertEqual(posted, ["Preview updated with 1 error recovered: 2 pages"])
        model.loadFixtures(request: Self.samples.appendingPathComponent("multipage-request.json"),
                           result: Self.samples.appendingPathComponent("multipage-result.json"))
        model.previewAnnouncer.flush()
        XCTAssertEqual(posted.count, 1, "the same revision and status again is not repeated")
        model.result = nil
        model.previewAnnouncer.flush()
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

    func testLiveRefusalKeepsTheFrameAndIsAnnouncedQuietlyOncePerDistinctError() throws {
        let model = ShellModel()
        var posted: [(String, NSAccessibilityPriorityLevel)] = []
        model.previewAnnouncer.post = { posted.append(($0, $1)) }
        let frame = try frame(pages: 1)
        let live = { (id: String) in V2Source.worker(requestID: id, projectId: "p", revision: 1, line: Data()) }
        let missing = RenderingV2.ValidationError(code: "font_unavailable", message: "no font")
        // A verified live frame on screen; the next live sibling is refused: the frame stays, VoiceOver hears why, quietly.
        model.displayListV2 = .loading(live("r1"), ticket: 1, previous: frame, previousSource: live("r0"))
        XCTAssertTrue(model.deliverDisplayListV2(ticket: 1, source: live("r1"), outcome: .failed(missing)))
        XCTAssertNotNil(model.displayListV2?.frame, "the previous verified frame is kept")
        XCTAssertEqual(posted.map(\.0), ["Preview not updated: no font"])
        XCTAssertEqual(posted.last?.1, .low)
        // The same refusal on the next keystrokes is silent.
        for (ticket, id) in [(2, "r2"), (3, "r3")] {
            model.displayListV2 = .loading(live(id), ticket: ticket, previous: frame, previousSource: live("r0"))
            XCTAssertTrue(model.deliverDisplayListV2(ticket: ticket, source: live(id), outcome: .failed(missing)))
        }
        XCTAssertEqual(posted.count, 1)
        // A different refusal is news.
        model.displayListV2 = .loading(live("r4"), ticket: 4, previous: frame, previousSource: live("r0"))
        XCTAssertTrue(model.deliverDisplayListV2(ticket: 4, source: live("r4"), outcome: .failed(RenderingV2.ValidationError(code: "source_mismatch", message: "sha differs"))))
        XCTAssertEqual(posted.map(\.0), ["Preview not updated: no font", "Preview not updated: sha differs"])
        // A frame verifies, then the first refusal recurs: spoken again.
        model.displayListV2 = .loading(live("r5"), ticket: 5, previous: frame, previousSource: live("r0"))
        XCTAssertTrue(model.deliverDisplayListV2(ticket: 5, source: live("r5"), outcome: .loaded(frame)))
        model.displayListV2 = .loading(live("r6"), ticket: 6, previous: frame, previousSource: live("r5"))
        XCTAssertTrue(model.deliverDisplayListV2(ticket: 6, source: live("r6"), outcome: .failed(missing)))
        XCTAssertEqual(posted.count, 3)
        XCTAssertEqual(posted.last?.0, "Preview not updated: no font")
        XCTAssertNotNil(model.displayListV2?.frame)
        // Ordering through the shell: a compile result lands (a coalesced "Preview
        // updated" is pending), then its live sibling is refused — the pending
        // update is withdrawn, so nothing follows the refusal.
        var result = try RuntimeV1.decodeCompileResult(Data(contentsOf: Self.samples.appendingPathComponent("multipage-result.json"))).payload
        result.revision = 99
        model.result = result
        model.displayListV2 = .loading(live("r7"), ticket: 7, previous: frame, previousSource: live("r5"))
        XCTAssertTrue(model.deliverDisplayListV2(ticket: 7, source: live("r7"), outcome: .failed(RenderingV2.ValidationError(code: "source_mismatch", message: "stale text"))))
        XCTAssertEqual(posted.last?.0, "Preview not updated: stale text")
        let count = posted.count
        model.previewAnnouncer.flush()
        XCTAssertEqual(posted.count, count, "the refused revision's update was withdrawn")
        XCTAssertNotEqual(model.previewAnnouncer.spoken?.revision, 99)
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
        XCTAssertTrue(note.contains("Preview not updated"))
        XCTAssertTrue(note.contains("Pages rotor"))
    }
}
