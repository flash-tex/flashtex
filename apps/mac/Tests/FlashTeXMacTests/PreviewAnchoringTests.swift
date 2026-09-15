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
/// Corrections are reported in points; the real-compiler and real
/// `PreviewView` cases print what they measured and assert only what the
/// hosted view actually contains (the v1 hook lives in a parent-retained file
/// and is delivered as a diff, so on a tree without it the v1 case reports
/// the uncorrected drift instead of failing).
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
    /// the keeper attached, so the mechanism is measured without the
    /// parent-retained `PreviewView`.
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

    // MARK: - v2 pane (PreviewV2View carries the keeper)

    private func v2Frame(pages n: Int) throws -> V2Frame {
        let envelope = try RenderingV2.decode(try Data(contentsOf: PreviewV2ParityTests.fixtures.appendingPathComponent("display-list-v2-text.json")))
        var frame = try V2Frame.prepare(envelope, store: PreviewV2ParityTests.store)
        let page = try XCTUnwrap(frame.list.pages.first), prepared = try XCTUnwrap(frame.prepared.first)
        for number in 2...max(n, 2) where number <= n {
            var p = page; p.number = number
            var q = prepared; q.number = number
            frame.list.pages.append(p); frame.prepared.append(q)
        }
        return frame
    }

    private func v2View(_ frame: V2Frame) -> PreviewV2View {
        PreviewV2View(frame: frame, dark: false, stale: false, caretPath: "main.tex", caretByte: nil) { _ in }
    }

    func testPreviewV2ViewHoldsTheAnchorAcrossResizeFrameSwapsAndIdenticalFrames() async throws {
        let three = try v2Frame(pages: 3)
        let pageH = CGFloat(three.prepared[0].heightPt), pageW = CGFloat(three.prepared[0].widthPt)
        let hosted = Hosted(v2View(three), width: 500, height: 400)
        try await settle()
        let scroll = try XCTUnwrap(Self.find(NSScrollView.self, in: hosted.hosting))
        let probe = try XCTUnwrap(Self.find(PreviewAnchorProbe.self, in: hosted.hosting), "PreviewV2View attaches the keeper")
        let wide = try XCTUnwrap(probe.layout)
        XCTAssertEqual(wide.pages.count, 3)
        XCTAssertEqual(wide.scale, PreviewPageLayout.fitScale(paneWidth: 500, widestPt: pageW), accuracy: 1e-6)
        XCTAssertEqual(scroll.documentView?.bounds.height ?? 0, wide.contentSize.height, accuracy: 1, "LazyVStack document height matches the layout model")

        let target = try XCTUnwrap(wide.frame(of: 2)).minY + 0.3 * pageH * wide.scale
        Self.scroll(scroll, toTop: target)
        try await settle(0.1)
        XCTAssertEqual(probe.anchor?.page, 2)

        // (c) resize
        hosted.resize(width: 350, height: 400)
        try await settle()
        let narrow = try XCTUnwrap(probe.layout)
        let expected = try XCTUnwrap(narrow.frame(of: 2)).minY + 0.3 * pageH * narrow.scale
        let correction = try XCTUnwrap(probe.corrections.first)
        print(String(format: "preview-anchoring v2 (%@): resize 500→350 pt: top %.1f → %.1f pt (Δ %.1f pt), expected %.1f pt", loadNote(), correction.before.y, correction.after.y, correction.deltaY, expected))
        XCTAssertEqual(Self.visibleTop(scroll), expected, accuracy: 1)
        XCTAssertEqual(correction.before.y, target, accuracy: 1)

        // (a) more pages (new frame instance, new nonce) → unchanged; fewer with the page gone → end, not top.
        let count = probe.corrections.count
        hosted.hosting.rootView = v2View(try v2Frame(pages: 5))
        try await settle()
        XCTAssertEqual(probe.layout?.pages.count, 5)
        XCTAssertEqual(Self.visibleTop(scroll), expected, accuracy: 1)
        XCTAssertEqual(probe.corrections.count, count)
        hosted.hosting.rootView = v2View(try v2Frame(pages: 1))
        try await settle()
        let end = max(0, (scroll.documentView?.bounds.height ?? 0) - scroll.contentView.bounds.height)
        XCTAssertEqual(Self.visibleTop(scroll), end, accuracy: 1)
        XCTAssertGreaterThan(Self.visibleTop(scroll), 0)

        // (b) an identical-geometry frame (what a re-verified or same-revision candidate looks like): no scroll.
        hosted.hosting.rootView = v2View(try v2Frame(pages: 3))
        try await settle()
        Self.scroll(scroll, toTop: target)
        try await settle(0.1)
        let stable = probe.corrections.count
        hosted.hosting.rootView = v2View(try v2Frame(pages: 3))
        try await settle()
        XCTAssertEqual(Self.visibleTop(scroll), target, accuracy: 0.5)
        XCTAssertEqual(probe.corrections.count, stable, "a frame with the same page geometry never re-scrolls")
    }

    // MARK: - v1 PreviewView with the real compiler (reports; asserts only when the hook is present)

    private static var compiler: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"].map { URL(fileURLWithPath: $0) }
    }

    private func waitUntil(timeout: TimeInterval = 20, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for worker"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    private func v1View(_ result: RuntimeV1.CompileResult) -> PreviewView {
        PreviewView(result: result, dark: false, caretItems: [:]) { _, _ in }
    }

    private static func body(paragraphs: Int) -> String {
        "\\section{Anchoring}\n" + (1...paragraphs).map { "Paragraph \($0) of the anchoring corpus keeps the reader on the same page fraction while the document grows and shrinks around it.\n\n" }.joined()
    }

    func testRealCompilerResultsAcrossPageCountStaleAndResize() async throws {
        guard let binary = Self.compiler, FileManager.default.isExecutableFile(atPath: binary.path) else {
            throw XCTSkip("set FLASHTEX_COMPILER to the built flashtex-compiler binary")
        }
        let model = ShellModel()
        model.attachWorker(at: binary)
        model.autoCompile = false
        func compile(_ text: String) async throws -> RuntimeV1.CompileResult {
            model.updateActiveText(text)
            model.compile()
            try await waitUntil { model.inFlightRevision == nil }
            return try XCTUnwrap(model.result)
        }
        let base = try await compile(Self.body(paragraphs: 60))
        guard base.pages.count >= 3 else { throw XCTSkip("compiler produced \(base.pages.count) page(s) for 60 paragraphs; need 3") }
        let pageH = CGFloat(base.pages[0].heightPt), pageW = CGFloat(base.pages[0].widthPt)

        let hosted = Hosted(v1View(base), width: 500, height: 400)
        try await settle()
        let scroll = try XCTUnwrap(Self.find(NSScrollView.self, in: hosted.hosting))
        let probe = Self.find(PreviewAnchorProbe.self, in: hosted.hosting)
        let applied = probe != nil
        let wide = PreviewPageLayout(pages: base.pages.map { .init(number: $0.number, widthPt: $0.widthPt, heightPt: $0.heightPt) },
                                     scale: PreviewPageLayout.fitScale(paneWidth: 500, widestPt: pageW))
        XCTAssertEqual(scroll.documentView?.bounds.height ?? 0, wide.contentSize.height, accuracy: 1, "PreviewView's document height matches the layout model")
        let target = try XCTUnwrap(wide.frame(of: 2)).minY + 0.3 * pageH * wide.scale
        Self.scroll(scroll, toTop: target)
        try await settle(0.1)
        var report = ["(\(loadNote()), keeper \(applied ? "present" : "ABSENT — parent diff not applied, drift reported not asserted")): base \(base.pages.count) page(s), revision \(base.revision)"]

        // (a) more pages
        let more = try await compile(Self.body(paragraphs: 120))
        XCTAssertGreaterThan(more.pages.count, base.pages.count)
        hosted.hosting.rootView = v1View(more)
        try await settle()
        report.append(String(format: "more pages (%d): top %.1f pt, expected %.1f pt (drift %.1f)", more.pages.count, Self.visibleTop(scroll), target, Self.visibleTop(scroll) - target))
        if applied { XCTAssertEqual(Self.visibleTop(scroll), target, accuracy: 1) }

        // (b) stale: an older revision arriving after a newer one is dropped by the model — the view sees no change.
        let stale = RuntimeV1.Envelope(protocolVersion: 1, id: "old", type: "compile_result",
                                       payload: RuntimeV1.CompileResult(projectId: "demo", revision: base.revision, status: .ok, pages: base.pages, diagnostics: [], pdfPath: nil))
        model.handleForTesting(.result(stale))
        XCTAssertEqual(model.result?.revision, more.revision)
        hosted.hosting.rootView = v1View(try XCTUnwrap(model.result))
        try await settle()
        report.append(String(format: "stale revision %d after %d: top %.1f pt (drift %.1f)", base.revision, more.revision, Self.visibleTop(scroll), Self.visibleTop(scroll) - target))
        XCTAssertEqual(Self.visibleTop(scroll), target, accuracy: 0.5, "a dropped stale result never moves the scroll")

        // (c) resize
        hosted.resize(width: 350, height: 400)
        try await settle()
        let narrow = PreviewPageLayout(pages: wide.pages, scale: PreviewPageLayout.fitScale(paneWidth: 350, widestPt: pageW))
        let expectedNarrow = try XCTUnwrap(narrow.frame(of: 2)).minY + 0.3 * pageH * narrow.scale
        report.append(String(format: "resize 500→350: top %.1f pt, anchored expectation %.1f pt (drift %.1f; correction needed %.1f)", Self.visibleTop(scroll), expectedNarrow, Self.visibleTop(scroll) - expectedNarrow, expectedNarrow - target))
        if applied { XCTAssertEqual(Self.visibleTop(scroll), expectedNarrow, accuracy: 1) }

        // (a) fewer pages with the anchored page gone
        let fewer = try await compile(Self.body(paragraphs: 3))
        XCTAssertLessThan(fewer.pages.count, base.pages.count)
        hosted.hosting.rootView = v1View(fewer)
        try await settle()
        let end = max(0, (scroll.documentView?.bounds.height ?? 0) - scroll.contentView.bounds.height)
        report.append(String(format: "fewer pages (%d): top %.1f pt, end of document %.1f pt", fewer.pages.count, Self.visibleTop(scroll), end))
        XCTAssertEqual(Self.visibleTop(scroll), end, accuracy: 1)
        for line in report { print("preview-anchoring PreviewView: " + line) }
        model.detachWorker()
    }
}
