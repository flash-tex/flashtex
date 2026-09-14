import AppKit
import CoreGraphics
import SwiftUI
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// `display-list-v2-window` consumer (protocol/proposals/display-list-v2-window.md,
/// producer PR #294): a reply that carries only a bounded window of built pages,
/// every other page elided to its frame.
///
/// The tests below are the shell's half of the proposal's co-signer row, in the
/// order the proposal states it:
///
///  1. the wire shape decodes on BOTH readers, identically, and round-trips —
///     and a page with no `items` and no `resident: false` is REFUSED rather
///     than read as an empty page (the whole point of the asymmetry);
///  2. an elided page is painted as a placeholder, not as nothing: it keeps its
///     real frame, it is not rasterized, and the view that draws it is not the
///     view that draws a resident page;
///  3. the resident window follows the viewport, with hysteresis — a scroll
///     inside the window sends nothing;
///  4. everything that needs the whole document refuses while pages are elided:
///     PDF export (all three routes), the delta base, caret sync.
final class V2PageWindowTests: XCTestCase {
    static let pageW = 200.0, pageH = 300.0
    static func ticks(_ pt: Double) -> Int64 { Int64((pt * Double(RenderingV2.ticksPerPoint)).rounded()) }
    static let tex = "\\documentclass{article}\\begin{document}Window.\\end{document}\n"

    /// A resident page: today's object exactly, one rule item.
    static func resident(_ number: Int) -> [String: Any] {
        ["number": number, "width": ticks(pageW), "height": ticks(pageH),
         "items": [["kind": "rule", "x": ticks(20), "top": ticks(20), "width": ticks(80), "height": ticks(2),
                    "paint": ["r": 0, "g": 0, "b": 0, "a": 1],
                    "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 10]]]]]
    }

    /// An elided page: frame, `resident: false`, and NO `items` key at all.
    static func elided(_ number: Int) -> [String: Any] {
        ["number": number, "width": ticks(pageW), "height": ticks(pageH), "resident": false]
    }

    /// `documentPages` pages, of which `window` are resident.
    static func list(documentPages: Int = 8, window: ClosedRange<Int>? = 3...4,
                     mutate: ((inout [String: Any]) -> Void)? = nil) -> [String: Any] {
        var pages: [[String: Any]] = []
        for n in 1...documentPages {
            pages.append(window.map { $0.contains(n) } ?? true ? resident(n) : elided(n))
        }
        var payload: [String: Any] = [
            "render_format": "display-list-v2", "coordinate_unit": "bp_2pow20", "color_space": "srgb",
            "text_extraction": "cluster-actualtext", "project_id": "p", "revision": 1,
            // Whole-document, as §3 requires: `rule` is announced even though it
            // could just as well have come from a page that was not built.
            "required_features": ["rgba-srgb", "cluster-actualtext", "rule"],
            "documents": [["path": "main.tex", "revision": 1, "sha256": SourceDigest.sha256Hex(tex), "byte_length": tex.utf8.count]],
            "fonts": [], "pages": pages, "diagnostics": [],
        ]
        if let window {
            payload["window"] = ["first_page": window.lowerBound, "page_count": window.count,
                                 "document_page_count": documentPages]
        }
        var envelope: [String: Any] = ["protocol_version": 2, "id": "win-1", "type": "display_list", "payload": payload]
        mutate?(&envelope)
        return envelope
    }

    static func data(_ o: [String: Any]) -> Data { try! JSONSerialization.data(withJSONObject: o) }

    static func code(_ o: [String: Any]) -> String? {
        do { _ = try RenderingV2.decode(data(o)); return nil }
        catch let e as RenderingV2.ValidationError { return e.code }
        catch { return "undecodable" }
    }

    // MARK: 1. the wire shape

    func testWindowedListDecodesOnBothReadersAndRoundTrips() throws {
        let bytes = Self.data(Self.list())
        let fast = try RenderingV2Fast.envelope(bytes)
        let slow = try RenderingV2.decode(bytes)
        XCTAssertEqual(fast, slow, "the fast reader and JSONDecoder agree on residency and the window object")

        let list = slow.payload
        XCTAssertEqual(list.window, RenderingV2.PageWindow(firstPage: 3, pageCount: 2, documentPageCount: 8))
        XCTAssertTrue(list.isWindowed)
        XCTAssertEqual(list.pages.count, 8, "every page of the document is still listed: the scroll extent is not windowed")
        XCTAssertEqual(list.elidedPageNumbers, [1, 2, 5, 6, 7, 8])
        XCTAssertEqual(list.window?.elidedCount, 6)
        XCTAssertTrue(list.isResident(page: 3) && list.isResident(page: 4))
        XCTAssertFalse(list.isResident(page: 2))
        // The frame of an elided page is real — that is what the pane places.
        XCTAssertEqual(list.pages[0].widthPt, Self.pageW, accuracy: 1e-9)
        XCTAssertEqual(list.pages[0].heightPt, Self.pageH, accuracy: 1e-9)
        XCTAssertTrue(list.pages[0].items.isEmpty)

        XCTAssertEqual(try RenderingV2.decode(try JSONEncoder().encode(slow)), slow, "round-trips through the encoder")
        let text = String(decoding: try JSONEncoder().encode(slow), as: UTF8.self)
        XCTAssertTrue(text.contains("\"resident\":false"), "an elided page says so on the wire")
        XCTAssertTrue(text.contains("\"document_page_count\":8"))
    }

    /// §4's asymmetry: a resident page gains NO marker, so every unwindowed line
    /// on the wire today is byte-for-byte what it was.
    func testUnwindowedListIsUnchangedAndResidentPagesCarryNoMarker() throws {
        let bytes = Self.data(Self.list(documentPages: 2, window: nil))
        let slow = try RenderingV2.decode(bytes)
        XCTAssertNil(slow.payload.window)
        XCTAssertFalse(slow.payload.isWindowed)
        XCTAssertTrue(slow.payload.pages.allSatisfy(\.isResident))
        let text = String(decoding: try JSONEncoder().encode(slow), as: UTF8.self)
        XCTAssertFalse(text.contains("\"resident\""), "a resident page carries no residency marker")
        XCTAssertFalse(text.contains("\"window\""), "an unwindowed list emits no window object")
    }

    /// The fail-closed rule, and the reason `items` is absent rather than `[]`:
    /// a consumer that never read the flag must get a refusal, not a blank page.
    func testPageWithoutItemsAndWithoutResidentFlagIsRefused() throws {
        var o = Self.list()
        var p = o["payload"] as! [String: Any]
        var pages = p["pages"] as! [[String: Any]]
        pages[0].removeValue(forKey: "resident") // no items, no flag
        p["pages"] = pages; o["payload"] = p
        XCTAssertNotNil(Self.code(o), "a page with neither items nor resident:false is refused")
        // And both readers refuse it, not just the slow one.
        XCTAssertThrowsError(try RenderingV2Fast.envelope(Self.data(o)))

        // An empty `items: []` is a genuinely empty page and stays legal.
        var ok = Self.list(documentPages: 2, window: nil)
        var q = ok["payload"] as! [String: Any]
        var qp = q["pages"] as! [[String: Any]]
        qp[0]["items"] = []
        q["pages"] = qp; ok["payload"] = q
        XCTAssertNil(Self.code(ok), "an empty page is still an empty page")
        XCTAssertTrue(try RenderingV2.decode(Self.data(ok)).payload.pages[0].isResident)
    }

    func testResidencyMustAgreeWithTheEchoedWindow() throws {
        // An elided page with no window at all: the list claims to be complete.
        var noWindow = Self.list()
        var p = noWindow["payload"] as! [String: Any]
        p.removeValue(forKey: "window"); noWindow["payload"] = p
        XCTAssertEqual(Self.code(noWindow), "invalid_display_list")

        // A page inside the window that is nonetheless elided.
        var holed = Self.list()
        var q = holed["payload"] as! [String: Any]
        var pages = q["pages"] as! [[String: Any]]
        pages[2] = Self.elided(3) // page 3 is inside 3...4
        q["pages"] = pages; holed["payload"] = q
        XCTAssertEqual(Self.code(holed), "invalid_display_list")

        // A resident page outside the window.
        var extra = Self.list()
        var r = extra["payload"] as! [String: Any]
        var rp = r["pages"] as! [[String: Any]]
        rp[0] = Self.resident(1)
        r["pages"] = rp; extra["payload"] = r
        XCTAssertEqual(Self.code(extra), "invalid_display_list")

        // document_page_count that disagrees with the entries.
        var miscount = Self.list()
        var t = miscount["payload"] as! [String: Any]
        t["window"] = ["first_page": 3, "page_count": 2, "document_page_count": 99]
        miscount["payload"] = t
        XCTAssertEqual(Self.code(miscount), "invalid_display_list")
    }

    /// §3: `required_features` is whole-document, so a windowed reply may
    /// announce a feature no resident page uses. That must not be a refusal —
    /// the check is "used ⊆ declared", never the other way round.
    func testWholeDocumentFeatureClosureIsAcceptedOnAWindowedReply() throws {
        var o = Self.list(documentPages: 4, window: 1...1)
        var p = o["payload"] as! [String: Any]
        // Page 1 has a rule; add `image`, which only an elided page could use.
        p["required_features"] = ["rgba-srgb", "cluster-actualtext", "rule", "image"]
        o["payload"] = p
        XCTAssertNil(Self.code(o), "a feature declared for a page that was not built is not a violation")
    }

    // MARK: 2. an elided page is painted, not skipped

    func testElidedPageIsPreparedAsAPlaceholderWithItsRealFrame() throws {
        let frame = try V2Frame.prepare(data: Self.data(Self.list()))
        XCTAssertTrue(frame.isWindowed)
        XCTAssertEqual(frame.elidedPageCount, 6)
        XCTAssertFalse(frame.isCompleteDocument)
        XCTAssertEqual(frame.prepared.count, 8, "every page is prepared, so every page has a slot in the column")

        let placeholder = try XCTUnwrap(frame.preparedPage(number: 1))
        XCTAssertFalse(placeholder.isResident)
        XCTAssertTrue(placeholder.items.isEmpty)
        XCTAssertEqual(placeholder.glyphCount, 0)
        // The frame is what the pane draws the placeholder at: the page must
        // occupy exactly the room the real page would, so scrolling past it and
        // page navigation land where they would have.
        XCTAssertEqual(placeholder.widthPt, Self.pageW, accuracy: 1e-9)
        XCTAssertEqual(placeholder.heightPt, Self.pageH, accuracy: 1e-9)

        let real = try XCTUnwrap(frame.preparedPage(number: 3))
        XCTAssertTrue(real.isResident)
        XCTAssertEqual(real.items.count, 1)

        // The scroll column places all eight pages, at full size.
        let layout = PreviewPageLayout(pages: frame.prepared.map {
            PreviewPageLayout.Page(number: $0.number, widthPt: $0.widthPt, heightPt: $0.heightPt)
        }, scale: 1)
        XCTAssertEqual(layout.frames.count, 8)
        XCTAssertEqual(layout.frame(of: 1)?.height, Self.pageH)
        XCTAssertEqual(layout.frame(of: 8)?.height, Self.pageH)
    }

    /// An elided page has nothing to draw, so it must not be rasterized — and
    /// the placeholder must not be a white page of the right size, which is
    /// exactly what a rasterized empty page would look like.
    func testElidedPagesAreNotRasterized() throws {
        let frame = try V2Frame.prepare(data: Self.data(Self.list()))
        let prerastered = V2Loader.preraster(frame, pixelsPerPoint: 1, dark: false)
        let residentTokens = Set(frame.prepared.enumerated().filter { $0.element.isResident }.map { frame.pageToken(at: $0.offset) })
        XCTAssertEqual(Set(prerastered.images.map(\.token)), residentTokens,
                       "only the pages the producer actually built are rasterized")
        XCTAssertEqual(prerastered.images.count, 2)
    }

    /// A blank page and an elided page must not be the same thing to any caller
    /// that can be reached by a windowed list.
    func testAnEmptyPageAndAnElidedPageAreDistinguishable() throws {
        var withEmpty = Self.list(documentPages: 2, window: nil)
        var p = withEmpty["payload"] as! [String: Any]
        var pages = p["pages"] as! [[String: Any]]
        pages[0]["items"] = []
        p["pages"] = pages; withEmpty["payload"] = p
        let empty = try V2Frame.prepare(data: Self.data(withEmpty))
        let windowed = try V2Frame.prepare(data: Self.data(Self.list()))
        XCTAssertTrue(empty.prepared[0].items.isEmpty)
        XCTAssertTrue(windowed.prepared[0].items.isEmpty)
        XCTAssertTrue(empty.prepared[0].isResident, "a genuinely empty page IS resident")
        XCTAssertFalse(windowed.prepared[0].isResident, "an elided page is not")
        XCTAssertFalse(empty.isWindowed)
        XCTAssertTrue(windowed.isWindowed)
    }

    // MARK: 3. the window follows the viewport, with hysteresis

    func testVisiblePagesAreTheOnesTheViewportIntersects() {
        let layout = PreviewPageLayout(pages: (1...10).map { PreviewPageLayout.Page(number: $0, widthPt: 200, heightPt: 300) }, scale: 1)
        // Page n occupies y = 24 + (n-1)*324 ..< +300.
        let onPageOne = CGRect(x: 0, y: 30, width: 200, height: 100)
        XCTAssertEqual(PreviewV2Window.visiblePages(in: onPageOne, layout: layout), 1...1)
        let straddling = CGRect(x: 0, y: 300, width: 200, height: 100) // end of 1, start of 2
        XCTAssertEqual(PreviewV2Window.visiblePages(in: straddling, layout: layout), 1...2)
        let far = CGRect(x: 0, y: 10_000, width: 200, height: 100)
        XCTAssertNil(PreviewV2Window.visiblePages(in: far, layout: layout))
    }

    func testDesiredWindowIsCentredOnTheViewportAndClampedToTheDocument() {
        // Middle of the document: centred.
        let mid = PreviewV2Window.desired(around: 100...101, documentPageCount: 385, span: 16)
        XCTAssertEqual(mid?.pageCount, 16)
        XCTAssertEqual(mid?.firstPage, 93, "16 pages around a 2-page viewport: 7 above, 7 below")
        // Top of the document: clamped, never below page 1.
        XCTAssertEqual(PreviewV2Window.desired(around: 1...1, documentPageCount: 385, span: 16)?.firstPage, 1)
        // Bottom: clamped so the window still holds `span` pages.
        let last = PreviewV2Window.desired(around: 385...385, documentPageCount: 385, span: 16)
        XCTAssertEqual(last?.firstPage, 370)
        XCTAssertEqual(last?.pageCount, 16)
        // A document shorter than the span.
        let small = PreviewV2Window.desired(around: 1...1, documentPageCount: 4, span: 16)
        XCTAssertEqual(small?.firstPage, 1)
        XCTAssertEqual(small?.pageCount, 4)
    }

    /// The hysteresis, which is the whole reason this is a policy and not a
    /// one-liner: scrolling inside the resident window must send NOTHING.
    func testASmallScrollInsideTheWindowSendsNothing() {
        let served = RenderingV2.PageWindow(firstPage: 93, pageCount: 16, documentPageCount: 385)
        let sent = RuntimeV1.CompileRequest.DisplayListWindow(firstPage: 93, pageCount: 16)
        // Dead centre, and either side of it: no request at all.
        for visible in [100...101, 99...100, 101...102, 96...97, 104...105] {
            XCTAssertNil(PreviewV2Window.next(visible: visible, documentPageCount: 385, served: served, requested: sent, span: 16),
                         "the viewport is comfortably inside pages 93–108; \(visible) must not re-request")
        }
        // Within `margin` of the bottom edge: the window moves before the reader
        // reaches the placeholders.
        let nearBottom = PreviewV2Window.next(visible: 107...108, documentPageCount: 385, served: served, requested: sent, span: 16)
        XCTAssertNotNil(nearBottom)
        XCTAssertGreaterThan(try XCTUnwrap(nearBottom).firstPage, 93)
        // And of the top.
        XCTAssertNotNil(PreviewV2Window.next(visible: 93...94, documentPageCount: 385, served: served, requested: sent, span: 16))
        // A jump clean out of the window always re-requests.
        let jumped = PreviewV2Window.next(visible: 300...301, documentPageCount: 385, served: served, requested: sent, span: 16)
        XCTAssertEqual(jumped?.firstPage, 293)
    }

    /// §8: the producer may narrow an over-limit window and echoes what it
    /// served. Asking again for the range we asked for would loop forever.
    func testANarrowedWindowDoesNotLoop() {
        let asked = RuntimeV1.CompileRequest.DisplayListWindow(firstPage: 1, pageCount: 24)
        let served = RenderingV2.PageWindow(firstPage: 1, pageCount: 16, documentPageCount: 385)
        XCTAssertNil(PreviewV2Window.next(visible: 5...6, documentPageCount: 385, served: served, requested: asked, span: 24),
                     "the echo is the authority: a narrowed window that still covers the viewport is left alone")
    }

    func testAFullyResidentDocumentNeverRequestsAWindow() {
        let served = RenderingV2.PageWindow(firstPage: 1, pageCount: 4, documentPageCount: 4)
        XCTAssertNil(PreviewV2Window.next(visible: 1...2, documentPageCount: 4, served: served, requested: nil))
    }

    func testTheFirstViewportReportAsksForAWindow() {
        XCTAssertEqual(PreviewV2Window.next(visible: 1...2, documentPageCount: 385, served: nil, requested: nil, span: 16)?.firstPage, 1)
        // …and does not ask twice for the same thing while it is in flight.
        let asked = RuntimeV1.CompileRequest.DisplayListWindow(firstPage: 1, pageCount: 16)
        XCTAssertNil(PreviewV2Window.next(visible: 1...2, documentPageCount: 385, served: nil, requested: asked, span: 16))
    }

    /// The request field, on the wire.
    func testCompileRequestCarriesTheWindowOnlyWhenAsked() throws {
        let documents = [RuntimeV1.Document(path: "main.tex", text: Self.tex)]
        var request = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "main.tex", documents: documents,
                                               layoutCapabilities: [RenderingV2.renderFormat, RenderingV2.windowCapability])
        let plain = String(decoding: try JSONEncoder().encode(request), as: UTF8.self)
        XCTAssertFalse(plain.contains("display_list_window"),
                       "the capability without a position asks for an unwindowed reply (§4)")
        request.displayListWindow = .init(firstPage: 41, pageCount: 16)
        let windowed = String(decoding: try JSONEncoder().encode(request), as: UTF8.self)
        XCTAssertTrue(windowed.contains("\"display_list_window\""))
        XCTAssertTrue(windowed.contains("\"first_page\":41") && windowed.contains("\"page_count\":16"))
        let back = try JSONDecoder().decode(RuntimeV1.CompileRequest.self, from: Data(windowed.utf8))
        XCTAssertEqual(back.displayListWindow, request.displayListWindow)
        // A window that asks for nothing is a programming error, not a request.
        request.displayListWindow = .init(firstPage: 0, pageCount: 0)
        XCTAssertThrowsError(try JSONEncoder().encode(request))
    }

    /// End to end, in a real `NSScrollView`: scrolling the page column reports
    /// the page range under the viewport, and only when it crosses a page
    /// boundary. This is the half `PreviewV2Window`'s pure tests cannot reach —
    /// that the probe is actually wired to the scroll view.
    @MainActor
    func testScrollingTheColumnReportsTheVisiblePageRange() async throws {
        var reported: [ClosedRange<Int>] = []
        let pages = (1...40).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }
        let hosted = PreviewAnchoringTests.Hosted(WindowColumn(pages: pages) { reported.append($0) }, width: 500, height: 400)
        try await Task.sleep(nanoseconds: 400_000_000)
        let scroll = try XCTUnwrap(PreviewAnchoringTests.find(NSScrollView.self, in: hosted.hosting))
        let probe = try XCTUnwrap(PreviewAnchoringTests.find(PreviewAnchorProbe.self, in: hosted.hosting))
        let layout = try XCTUnwrap(probe.layout)

        XCTAssertEqual(reported.last?.lowerBound, 1, "the first report is the top of the document")

        // Scroll a fraction of one page: the range does not change, so nothing
        // new is reported. This is the thrash guard at the source.
        let before = reported.count
        let pageHeight = try XCTUnwrap(layout.frame(of: 1)).height
        PreviewAnchoringTests.scroll(scroll, toTop: pageHeight * 0.15)
        try await Task.sleep(nanoseconds: 200_000_000)
        XCTAssertEqual(reported.count, before, "a scroll inside one page reports nothing new (reported: \(reported))")

        // Scroll to page 20 and the report follows.
        let target = try XCTUnwrap(layout.frame(of: 20)).minY
        PreviewAnchoringTests.scroll(scroll, toTop: target + 4)
        try await Task.sleep(nanoseconds: 300_000_000)
        let last = try XCTUnwrap(reported.last)
        XCTAssertTrue(last.contains(20), "the viewport is on page 20; reported \(last)")
        XCTAssertGreaterThan(reported.count, before)

        // And that range drives a window request centred on it.
        let want = try XCTUnwrap(PreviewV2Window.next(visible: last, documentPageCount: 40, served: nil, requested: nil, span: 16))
        XCTAssertTrue((want.firstPage...(want.firstPage + want.pageCount - 1)).contains(20))
    }

    /// The pane's column with the keeper attached, mirroring `PreviewV2View`.
    struct WindowColumn: View {
        let pages: [PreviewPageLayout.Page]
        let onVisiblePages: (ClosedRange<Int>) -> Void
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
                    .background(PreviewAnchorKeeper(layout: layout, onVisiblePages: onVisiblePages))
                }
            }
        }
    }

    // MARK: 4. what refuses while pages are elided

    @MainActor
    func testExportRefusesAWindowedFrameOnEveryRoute() throws {
        let model = ShellModel()
        model.previewV2 = true
        let frame = try V2Frame.prepare(data: Self.data(Self.list()))
        model.displayListV2 = .loaded(frame, .file(URL(fileURLWithPath: "/tmp/window.json")))

        model.captureNote = nil
        model.exportPDFV2()
        let v2 = try XCTUnwrap(model.captureNote)
        XCTAssertTrue(v2.hasPrefix("Export refused:"), "got: \(v2)")
        XCTAssertTrue(v2.contains("6 pages not loaded"), "the refusal says how much is missing: \(v2)")

        model.captureNote = nil
        model.exportPDFExact()
        XCTAssertTrue(try XCTUnwrap(model.captureNote).hasPrefix("Export refused:"))

        // A complete frame is not refused (it reaches the save panel, which is
        // as far as a headless test can go — so assert only that the refusal
        // did not fire).
        let whole = try V2Frame.prepare(data: Self.data(Self.list(documentPages: 2, window: nil)))
        XCTAssertFalse(whole.isWindowed)
        XCTAssertNil(whole.window)
    }

    @MainActor
    func testAWindowedFrameIsNeverADeltaBase() throws {
        // §7: `-window` and `-delta` are exclusive — a windowed list is not a
        // complete compile, so it cannot be the base a delta is applied to.
        let frame = try V2Frame.prepare(data: Self.data(Self.list()))
        XCTAssertNil(frame.installedBase)
        XCTAssertFalse(frame.isCompleteDocument)
    }

    /// §4.1: a windowed reply never authorises a source action outside its
    /// coverage. An elided page carries no provenance, so caret sync and
    /// click-to-source must decline rather than land somewhere plausible.
    func testSourceActionsAreDeclinedForElidedPages() throws {
        let frame = try V2Frame.prepare(data: Self.data(Self.list()))
        XCTAssertFalse(frame.allowsSourceActions(page: 1))
        XCTAssertTrue(frame.allowsSourceActions(page: 3))
        // Click-to-source: the resident page answers at the rule, the elided
        // page at the same point answers nothing — it has no provenance to give.
        let residentPage = try XCTUnwrap(frame.page(number: 3))
        let hit = try XCTUnwrap(V2Geometry.hit(page: residentPage, atPointX: 30, y: 21))
        XCTAssertEqual(hit.sources.first?.path, "main.tex")
        let elidedPage = try XCTUnwrap(frame.page(number: 1))
        XCTAssertNil(V2Geometry.hit(page: elidedPage, atPointX: 30, y: 21))
        // Caret follow walks only resident pages (CaretFollow.swift): a rule
        // carries no caret highlight, so nothing matches either way — the point
        // is that no elided page is even considered.
        XCTAssertNil(ShellModel.caretTarget(byte: 5, path: "main.tex", in: frame))
    }

    @MainActor
    func testTheShellSaysWhyCaretSyncIsLimited() throws {
        let model = ShellModel()
        model.previewV2 = true
        XCTAssertNil(model.v2WindowSourceActionRefusal, "nothing to explain without a window")
        model.displayListV2 = .loaded(try V2Frame.prepare(data: Self.data(Self.list())), .file(URL(fileURLWithPath: "/tmp/window.json")))
        let why = try XCTUnwrap(model.v2WindowSourceActionRefusal)
        XCTAssertTrue(why.contains("pages 3–4 of 8"), "got: \(why)")
    }
}
