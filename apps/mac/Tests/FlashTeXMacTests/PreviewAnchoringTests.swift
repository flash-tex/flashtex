import XCTest
import SwiftUI
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// Gap 5 — preview scroll anchoring across a page-count change, a stale
/// candidate and a pane resize. Pure `PreviewAnchor` checks first; then the
/// `PreviewAnchorKeeper` probe measured inside real `NSScrollView`s hosted
/// off-screen by `HostedWindowSupport.window` (never made key: no focus is
/// stolen, and the window is parked clear of every display).
/// Corrections are reported in points. The engine's pane is measured by
/// EngineV3PreviewNavTests (its v1 and v2 cases retired with those panes,
/// #1236 S6).
@MainActor
final class PreviewAnchoringTests: XCTestCase {
    static let letter = PreviewPageLayout.Page(number: 1, widthPt: 612, heightPt: 792)
    static func pages(_ n: Int, width: CGFloat = 612, height: CGFloat = 792) -> [PreviewPageLayout.Page] {
        (1...max(n, 1)).map { PreviewPageLayout.Page(number: $0, widthPt: width, heightPt: height) }
    }

    // MARK: - Pure model

    func testFitScaleMatchesThePanesRule() {
        XCTAssertEqual(PreviewPageLayout.fitScale(paneWidth: 660, widestPt: 612), 1) // exactly fits
        XCTAssertEqual(PreviewPageLayout.fitScale(paneWidth: 2000, widestPt: 612), 1) // never upscaled
        XCTAssertEqual(PreviewPageLayout.fitScale(paneWidth: 354, widestPt: 612), 0.5, accuracy: 1e-9)
        XCTAssertEqual(PreviewPageLayout.fitScale(paneWidth: 10, widestPt: 612), 0.2) // floor
    }

    func testFramesStackPagesWithPaddingSpacingAndCenteredWidths() {
        let layout = PreviewPageLayout(pages: [Self.letter, .init(number: 2, widthPt: 306, heightPt: 400)], scale: 0.5)
        let frames = layout.frames
        XCTAssertEqual(frames.map(\.number), [1, 2])
        guard frames.count == 2 else { return XCTFail("expected two frames, got \(frames.count)") }
        XCTAssertEqual(frames[0].frame, CGRect(x: 24, y: 24, width: 306, height: 396))
        XCTAssertEqual(frames[1].frame, CGRect(x: 24 + 76.5, y: 24 + 396 + 24, width: 153, height: 200))
        XCTAssertEqual(layout.contentSize, CGSize(width: 306 + 48, height: 24 + 396 + 24 + 200 + 24))
        XCTAssertEqual(PreviewPageLayout(pages: [], scale: 1).contentSize, CGSize(width: 48, height: 48))
    }

    func testCaptureSelectsThePageUnderTheTopEdgeAndRoundTrips() throws {
        let layout = PreviewPageLayout(pages: Self.pages(3), scale: 0.75)
        let page2 = try XCTUnwrap(layout.frame(of: 2))
        let visible = CGRect(x: 0, y: page2.minY + 0.3 * page2.height, width: 500, height: 400)
        let anchor = try XCTUnwrap(PreviewAnchor.capture(visible: visible, layout: layout))
        XCTAssertEqual(anchor.page, 2)
        XCTAssertEqual(anchor.fraction, 0.3, accuracy: 1e-9)
        XCTAssertEqual(anchor.horizontal, -24 / page2.width, accuracy: 1e-9) // 24pt of padding left of the page
        let restored = try XCTUnwrap(anchor.restore(in: layout, visibleSize: visible.size))
        XCTAssertEqual(restored.y, visible.minY, accuracy: 1e-9)
        XCTAssertEqual(restored.x, 0) // clamped: the content is narrower than the viewport

        // Top edge inside the gap above page 2: anchored to page 2 with a small negative fraction.
        let gap = try XCTUnwrap(PreviewAnchor.capture(visible: CGRect(x: 0, y: page2.minY - 10, width: 500, height: 400), layout: layout))
        XCTAssertEqual(gap.page, 2)
        XCTAssertLessThan(gap.fraction, 0)
        XCTAssertEqual(gap.restore(in: layout, visibleSize: visible.size)?.y ?? -1, page2.minY - 10, accuracy: 1e-9)

        // Beyond the last page: last page, fraction above 1; restore clamps to the end.
        let beyond = try XCTUnwrap(PreviewAnchor.capture(visible: CGRect(x: 0, y: layout.contentSize.height + 50, width: 500, height: 400), layout: layout))
        XCTAssertEqual(beyond.page, 3)
        XCTAssertGreaterThan(beyond.fraction, 1)
        XCTAssertEqual(beyond.restore(in: layout, visibleSize: visible.size)?.y, layout.contentSize.height - 400)

        XCTAssertNil(PreviewAnchor.capture(visible: visible, layout: PreviewPageLayout(pages: [], scale: 1)))
        XCTAssertNil(anchor.restore(in: PreviewPageLayout(pages: [], scale: 1), visibleSize: visible.size))
    }

    func testMorePagesKeepTheOffsetAndFewerPagesClampToTheLastPage() throws {
        let three = PreviewPageLayout(pages: Self.pages(3), scale: 0.6)
        let anchor = PreviewAnchor(page: 3, fraction: 0.4, horizontal: 0)
        let viewport = CGSize(width: 500, height: 300)
        let y3 = try XCTUnwrap(three.frame(of: 3)).minY + 0.4 * 792 * 0.6
        XCTAssertEqual(anchor.restore(in: three, visibleSize: viewport)?.y ?? -1, y3, accuracy: 1e-9)

        // Pages appended after the anchored page: same offset (no jump).
        let six = PreviewPageLayout(pages: Self.pages(6), scale: 0.6)
        XCTAssertEqual(anchor.restore(in: six, visibleSize: viewport)?.y ?? -1, y3, accuracy: 1e-9)

        // The anchored page vanished: clamp to the last page at the same fraction,
        // then to the scrollable range — never to the top.
        let two = PreviewPageLayout(pages: Self.pages(2), scale: 0.6)
        let restored = try XCTUnwrap(anchor.restore(in: two, visibleSize: viewport))
        let y2 = try XCTUnwrap(two.frame(of: 2)).minY + 0.4 * 792 * 0.6
        XCTAssertEqual(restored.y, min(y2, two.contentSize.height - viewport.height), accuracy: 1e-9)
        XCTAssertGreaterThan(restored.y, 0)

        // A taller viewport than the whole document: offset 0.
        XCTAssertEqual(anchor.restore(in: two, visibleSize: CGSize(width: 500, height: 5000))?.y, 0)
    }

    func testScaleChangeMovesTheOffsetToKeepThePageFractionAndIdenticalLayoutIsANoOp() throws {
        let wide = PreviewPageLayout(pages: Self.pages(3), scale: PreviewPageLayout.fitScale(paneWidth: 500, widestPt: 612))
        let narrow = PreviewPageLayout(pages: Self.pages(3), scale: PreviewPageLayout.fitScale(paneWidth: 350, widestPt: 612))
        let viewport = CGSize(width: 350, height: 400)
        let anchor = PreviewAnchor(page: 2, fraction: 0.3, horizontal: 0)
        let before = try XCTUnwrap(anchor.restore(in: wide, visibleSize: CGSize(width: 500, height: 400)))
        let after = try XCTUnwrap(anchor.restore(in: narrow, visibleSize: viewport))
        let expected = try XCTUnwrap(narrow.frame(of: 2)).minY + 0.3 * 792 * narrow.scale
        XCTAssertEqual(after.y, expected, accuracy: 1e-9)
        XCTAssertNotEqual(before.y, after.y)
        let recaptured = try XCTUnwrap(PreviewAnchor.capture(visible: CGRect(origin: after, size: viewport), layout: narrow))
        XCTAssertEqual(recaptured.page, 2)
        XCTAssertEqual(recaptured.fraction, 0.3, accuracy: 1e-9)
        print(String(format: "preview-anchoring model: pane 500→350 pt, page 2 @ 0.3: offset %.1f → %.1f pt (correction %.1f pt)", before.y, after.y, after.y - before.y))

        // A stale/identical layout (same pages, same scale) restores the very same offset.
        let same = PreviewPageLayout(pages: Self.pages(3), scale: wide.scale)
        XCTAssertEqual(same, wide)
        XCTAssertEqual(anchor.restore(in: same, visibleSize: CGSize(width: 500, height: 400)), before)
    }

    // MARK: - Hosted harness (real NSScrollView, off-screen window)

    /// Mirrors the panes' structure exactly (GeometryReader → fit scale →
    /// ScrollView → VStack of page-sized children, 24pt spacing/padding) with
    /// the keeper attached, so the mechanism is measured without a pane.
    struct Column: View {
        let pages: [PreviewPageLayout.Page]
        var body: some View {
            GeometryReader { geo in
                let scale = PreviewPageLayout.fitScale(paneWidth: geo.size.width, widestPt: pages.map(\.widthPt).max() ?? 612)
                let layout = PreviewPageLayout(pages: pages, scale: scale)
                ScrollView([.vertical, .horizontal]) {
                    VStack(spacing: 24) {
                        ForEach(pages, id: \.number) { page in
                            Rectangle().fill(Color.white)
                                .frame(width: page.widthPt * scale, height: page.heightPt * scale)
                                .id(page.number)
                        }
                    }
                    .padding(24)
                    .background(PreviewAnchorKeeper(layout: layout))
                }
            }
        }
    }

    final class Hosted<V: View> {
        let window: NSWindow
        let hosting: NSHostingView<V>
        init(_ view: V, width: CGFloat, height: CGFloat) {
            HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
            window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: width, height: height), styleMask: [.titled], backing: .buffered, defer: false)
            hosting = NSHostingView(rootView: view)
            window.contentView = hosting
            window.orderFrontRegardless() // off-screen and never key: no focus change
        }
        deinit { window.orderOut(nil) }
        func resize(width: CGFloat, height: CGFloat) { window.setContentSize(NSSize(width: width, height: height)) }
    }

    static func find<T: NSView>(_ type: T.Type, in view: NSView) -> T? {
        if let v = view as? T { return v }
        for sub in view.subviews { if let found = find(type, in: sub) { return found } }
        return nil
    }

    private func settle(_ seconds: TimeInterval = 0.4) async throws { try await Task.sleep(nanoseconds: UInt64(seconds * 1e9)) }

    /// Visible top edge in document coordinates, y down.
    static func visibleTop(_ scroll: NSScrollView) -> CGFloat {
        let rect = scroll.documentVisibleRect
        guard let doc = scroll.documentView else { return rect.minY }
        return doc.isFlipped ? rect.minY : doc.bounds.height - rect.maxY
    }

    static func scroll(_ scroll: NSScrollView, toTop y: CGFloat) {
        guard let doc = scroll.documentView else { return }
        let clip = scroll.contentView
        let origin = CGPoint(x: 0, y: doc.isFlipped ? y : doc.bounds.height - y - clip.bounds.height)
        clip.scroll(to: origin)
        scroll.reflectScrolledClipView(clip)
    }

    private func loadNote() -> String {
        var avg = [Double](repeating: 0, count: 3)
        _ = getloadavg(&avg, 3)
        return String(format: "load %.2f", avg[0])
    }

    func testHostedColumnHoldsTheAnchorAcrossResizeAndPageCountChanges() async throws {
        let hosted = Hosted(Column(pages: Self.pages(3)), width: 500, height: 400)
        try await settle()
        let scroll = try XCTUnwrap(Self.find(NSScrollView.self, in: hosted.hosting), "SwiftUI ScrollView is backed by an NSScrollView")
        let probe = try XCTUnwrap(Self.find(PreviewAnchorProbe.self, in: hosted.hosting))
        XCTAssertEqual(probe.layout?.scale ?? 0, PreviewPageLayout.fitScale(paneWidth: 500, widestPt: 612), accuracy: 1e-6)
        let wide = try XCTUnwrap(probe.layout)
        XCTAssertEqual(scroll.documentView?.bounds.height ?? 0, wide.contentSize.height, accuracy: 1, "layout model matches the hosted document height")

        // User scroll to page 2 @ 0.3.
        let target = try XCTUnwrap(wide.frame(of: 2)).minY + 0.3 * 792 * wide.scale
        Self.scroll(scroll, toTop: target)
        try await settle(0.1)
        XCTAssertEqual(Self.visibleTop(scroll), target, accuracy: 0.5)
        XCTAssertEqual(probe.anchor?.page, 2)
        XCTAssertEqual(probe.anchor?.fraction ?? 0, 0.3, accuracy: 1e-3)

        // (c) Resize the pane: scale changes, the anchor must hold.
        hosted.resize(width: 350, height: 400)
        try await settle()
        let narrow = try XCTUnwrap(probe.layout)
        XCTAssertEqual(narrow.scale, PreviewPageLayout.fitScale(paneWidth: 350, widestPt: 612), accuracy: 1e-6)
        let expectedNarrow = try XCTUnwrap(narrow.frame(of: 2)).minY + 0.3 * 792 * narrow.scale
        let correction = try XCTUnwrap(probe.corrections.first, "the keeper re-scrolled after the scale change")
        print(String(format: "preview-anchoring hosted (%@): resize 500→350 pt: uncorrected top %.1f pt → corrected %.1f pt (Δ %.1f pt), expected %.1f pt", loadNote(), correction.before.y, correction.after.y, correction.deltaY, expectedNarrow))
        XCTAssertEqual(Self.visibleTop(scroll), expectedNarrow, accuracy: 1)
        XCTAssertEqual(correction.before.y, target, accuracy: 1, "without the keeper the offset would have stayed at the wide-scale value")
        XCTAssertEqual(probe.anchor?.page, 2)
        XCTAssertEqual(probe.anchor?.fraction ?? 0, 0.3, accuracy: 1e-3)

        // (a) More pages after the anchored one: offset unchanged, no correction.
        let before = probe.corrections.count
        hosted.hosting.rootView = Column(pages: Self.pages(6))
        try await settle()
        XCTAssertEqual(probe.layout?.pages.count, 6)
        probe.note("measure(test sees \(Self.visibleTop(scroll)) same scroll=\(scroll === probe.enclosingScrollView) same doc=\(scroll.documentView === probe.enclosingScrollView?.documentView))")
        print("preview-anchoring hosted trace: " + probe.trace.map { String(format: "%.0fms %@ top=%.1f doc=%.1f", $0.ms, $0.event, $0.top, $0.docHeight) }.joined(separator: " | "))
        XCTAssertEqual(Self.visibleTop(scroll), expectedNarrow, accuracy: 1)
        XCTAssertEqual(probe.corrections.count, before, "appending pages needs no correction")
        XCTAssertEqual(probe.anchor?.page, 2)

        // (a) Fewer pages, anchored page still present: unchanged unless the
        // shorter document cannot scroll that far (then the end of the document).
        hosted.hosting.rootView = Column(pages: Self.pages(2))
        try await settle()
        let twoPageEnd = max(0, (scroll.documentView?.bounds.height ?? 0) - scroll.contentView.bounds.height)
        print(String(format: "preview-anchoring hosted: pages 6→2 with page 2 @ 0.3 anchored: top %.1f pt (anchor %.1f pt, end of document %.1f pt)", Self.visibleTop(scroll), expectedNarrow, twoPageEnd))
        XCTAssertEqual(Self.visibleTop(scroll), min(expectedNarrow, twoPageEnd), accuracy: 1)
        XCTAssertEqual(probe.anchor?.page, 2)

        // (a) The anchored page vanished: clamped to the end of the last page, not the top.
        Self.scroll(scroll, toTop: try XCTUnwrap(narrow.frame(of: 2)).minY + 0.5 * 792 * narrow.scale)
        try await settle(0.1)
        hosted.hosting.rootView = Column(pages: Self.pages(1))
        try await settle()
        let one = try XCTUnwrap(probe.layout)
        XCTAssertEqual(one.pages.count, 1)
        let end = max(0, (scroll.documentView?.bounds.height ?? 0) - scroll.contentView.bounds.height)
        print(String(format: "preview-anchoring hosted: pages 2→1 with page 2 anchored: top %.1f pt (end of document %.1f pt)", Self.visibleTop(scroll), end))
        XCTAssertEqual(Self.visibleTop(scroll), end, accuracy: 1)
        XCTAssertGreaterThan(Self.visibleTop(scroll), 0, "no jump to the top")

        // (b) Identical layout (a stale candidate that changes nothing): no scroll, no correction.
        Self.scroll(scroll, toTop: 100)
        try await settle(0.1)
        let count = probe.corrections.count
        hosted.hosting.rootView = Column(pages: Self.pages(1))
        try await settle()
        XCTAssertEqual(Self.visibleTop(scroll), 100, accuracy: 0.5)
        XCTAssertEqual(probe.corrections.count, count)
    }

    func testHostedColumnKeepsHorizontalFractionWhenContentIsWiderThanThePane() async throws {
        // A 20% floor makes a 4000pt-wide page wider than a 500pt pane: horizontal scrolling exists.
        let pages = [PreviewPageLayout.Page(number: 1, widthPt: 4000, heightPt: 792), PreviewPageLayout.Page(number: 2, widthPt: 4000, heightPt: 792)]
        let hosted = Hosted(Column(pages: pages), width: 500, height: 300)
        try await settle()
        let scroll = try XCTUnwrap(Self.find(NSScrollView.self, in: hosted.hosting))
        let probe = try XCTUnwrap(Self.find(PreviewAnchorProbe.self, in: hosted.hosting))
        let layout = try XCTUnwrap(probe.layout)
        XCTAssertEqual(layout.scale, 0.2)
        let doc = try XCTUnwrap(scroll.documentView)
        let clip = scroll.contentView
        let frame2 = try XCTUnwrap(layout.frame(of: 2))
        let y = frame2.minY + 0.25 * frame2.height
        clip.scroll(to: CGPoint(x: 200, y: doc.isFlipped ? y : doc.bounds.height - y - clip.bounds.height))
        scroll.reflectScrolledClipView(clip)
        try await settle(0.1)
        let anchor = try XCTUnwrap(probe.anchor)
        XCTAssertEqual(anchor.page, 2)
        XCTAssertEqual(anchor.horizontal, (200 - frame2.minX) / frame2.width, accuracy: 1e-3)
        // Page count change keeps both fractions.
        hosted.hosting.rootView = Column(pages: pages + [PreviewPageLayout.Page(number: 3, widthPt: 4000, heightPt: 792)])
        try await settle()
        XCTAssertEqual(scroll.documentVisibleRect.minX, 200, accuracy: 0.5)
        XCTAssertEqual(Self.visibleTop(scroll), y, accuracy: 0.5)
        XCTAssertEqual(probe.anchor, anchor)
    }
}
