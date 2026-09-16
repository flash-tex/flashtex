import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Consumer of `display-list-v2-window` (V2PageWindow.swift; producer PR
/// #294): request field, windowed-reply decode and validation, the engage /
/// scroll state machine, and the §4.1 exclusions (never a delta base, never
/// an export source).
final class V2WindowTests: XCTestCase {
    static let sha = String(repeating: "ab", count: 32)
    static let w612: Int64 = 612 << 20
    static let h792: Int64 = 792 << 20

    // MARK: request wire shape (§4)

    func testCompileRequestCarriesTheWindowFieldOnlyWhenSet() throws {
        var req = RuntimeV1.CompileRequest(projectId: "p", revision: 3, entryPath: "main.tex",
                                           documents: [.init(path: "main.tex", text: "x")])
        var line = String(decoding: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "r1", req)), as: UTF8.self)
        XCTAssertFalse(line.contains("display_list_window"), "absent field is omitted from the wire")
        req.displayListWindow = .init(firstPage: 41, pageCount: 16)
        line = String(decoding: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "r2", req)), as: UTF8.self)
        XCTAssertTrue(line.contains(#""display_list_window""#), line)
        XCTAssertTrue(line.contains(#""first_page":41"#), line)
        XCTAssertTrue(line.contains(#""page_count":16"#), line)
    }

    func testCompileRequestWindowRoundTrips() throws {
        let req = RuntimeV1.CompileRequest(projectId: "p", revision: 3, entryPath: "main.tex",
                                           documents: [.init(path: "main.tex", text: "x")],
                                           displayListWindow: .init(firstPage: 7, pageCount: 16))
        let data = try JSONEncoder().encode(req)
        let back = try JSONDecoder().decode(RuntimeV1.CompileRequest.self, from: data)
        XCTAssertEqual(back.displayListWindow, .init(firstPage: 7, pageCount: 16))
        XCTAssertEqual(back, req)
    }

    // MARK: reply wire shape (§4)

    /// A three-page list windowed to page 2: elided entries carry geometry and
    /// `"resident": false` and no `items` key at all.
    static func windowedJSON(window: String = #"{"first_page":2,"page_count":1,"document_page_count":3}"#,
                             page1: String = #"{"number":1,"width":641728512,"height":830472192,"resident":false}"#,
                             page2: String = #"{"number":2,"width":641728512,"height":830472192,"items":[]}"#,
                             page3: String = #"{"number":3,"width":641728512,"height":830472192,"resident":false}"#) -> Data {
        Data("""
        {"protocol_version":2,"id":"w1","type":"display_list","payload":{\
        "render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb",\
        "text_extraction":"cluster-actualtext","project_id":"demo","revision":3,\
        "required_features":["rgba-srgb","cluster-actualtext"],\
        "documents":[{"path":"main.tex","revision":3,"sha256":"\(sha)","byte_length":10}],\
        "fonts":[],"pages":[\(page1),\(page2),\(page3)],"diagnostics":[],"window":\(window)}}
        """.utf8)
    }

    func testWindowedListDecodesOnBothPaths() throws {
        let data = Self.windowedJSON()
        // Combined path (fast reader first, then full validation).
        let envelope = try RenderingV2.decode(data)
        let list = envelope.payload
        XCTAssertEqual(list.window, RenderingV2.Window(firstPage: 2, pageCount: 1, documentPageCount: 3))
        XCTAssertEqual(list.window?.pageRange, 2...2)
        XCTAssertEqual(list.pages.map(\.resident), [false, true, false])
        XCTAssertEqual(list.pages.map(\.number), [1, 2, 3])
        XCTAssertTrue(list.pages[0].items.isEmpty)
        XCTAssertEqual(list.pages[0].width, Self.w612)
        // Fast reader alone produces the same values.
        let fast = try RenderingV2Fast.envelope(data)
        XCTAssertEqual(fast, envelope)
        // JSONDecoder alone (the slow path's decoder) produces the same values.
        let slow = try JSONDecoder().decode(RenderingV2.Envelope.self, from: data)
        XCTAssertEqual(slow, envelope)
    }

    func testUnwindowedLineIsByteForByteUnchanged() throws {
        // A resident page encodes without any `resident` key; a non-resident
        // page encodes `resident: false` and no `items` key (§4: the
        // asymmetry keeps every unwindowed line unchanged).
        let resident = try JSONSerialization.jsonObject(with: JSONEncoder().encode(
            RenderingV2.Page(number: 1, width: Self.w612, height: Self.h792, items: []))) as! [String: Any]
        XCTAssertNil(resident["resident"])
        XCTAssertNotNil(resident["items"])
        let elided = try JSONSerialization.jsonObject(with: JSONEncoder().encode(
            RenderingV2.Page(number: 2, width: Self.w612, height: Self.h792, items: [], resident: false))) as! [String: Any]
        XCTAssertEqual(elided["resident"] as? Bool, false)
        XCTAssertNil(elided["items"], "no items key at all — absent, not []")
    }

    func testNonResidentPageWithItemsKeyIsRefused() {
        let bad = Self.windowedJSON(page1: #"{"number":1,"width":641728512,"height":830472192,"resident":false,"items":[]}"#)
        XCTAssertThrowsError(try RenderingV2.decode(bad)) { error in
            XCTAssertTrue("\(error)".contains("resident"), "\(error)")
        }
        XCTAssertThrowsError(try JSONDecoder().decode(RenderingV2.Envelope.self, from: bad))
    }

    func testResidencyMustMatchTheWindowExactly() throws {
        // Page 1 resident although outside the window.
        let outside = Self.windowedJSON(page1: #"{"number":1,"width":641728512,"height":830472192,"items":[]}"#)
        XCTAssertThrowsError(try RenderingV2.decode(outside)) { error in
            XCTAssertTrue("\(error)".contains("residency"), "\(error)")
        }
        // Page 2 elided although inside the window.
        let inside = Self.windowedJSON(page2: #"{"number":2,"width":641728512,"height":830472192,"resident":false}"#)
        XCTAssertThrowsError(try RenderingV2.decode(inside))
    }

    func testNonResidentPageWithoutAWindowObjectIsRefused() {
        let data = Data(String(decoding: Self.windowedJSON(), as: UTF8.self)
            .replacingOccurrences(of: #","window":{"first_page":2,"page_count":1,"document_page_count":3}"#, with: "").utf8)
        XCTAssertThrowsError(try RenderingV2.decode(data)) { error in
            XCTAssertTrue("\(error)".contains("window"), "\(error)")
        }
    }

    func testWindowRangeMustLieWithinTheDocument() {
        let past = Self.windowedJSON(window: #"{"first_page":3,"page_count":2,"document_page_count":3}"#)
        XCTAssertThrowsError(try RenderingV2.decode(past))
        let count = Self.windowedJSON(window: #"{"first_page":2,"page_count":1,"document_page_count":4}"#)
        XCTAssertThrowsError(try RenderingV2.decode(count), "document_page_count must equal pages.count")
    }

    // MARK: controller (engage on demand, scroll re-anchoring)

    func testControllerEngagesOnDemandAndAnchorsAtTheViewer() {
        var c = V2Window.Controller()
        XCTAssertNil(c.desired, "never windowed by default")
        _ = c.sawVisiblePage(30, served: nil)
        XCTAssertNil(c.desired, "scrolling alone never engages")
        c.engage()
        XCTAssertEqual(c.desired, .init(firstPage: 30 - V2Window.margin, pageCount: V2Window.pageCount))
        c.disengage()
        XCTAssertNil(c.desired)
        XCTAssertNil(c.applied)
    }

    func testControllerClampsTheAnchorToPageOne() {
        var c = V2Window.Controller()
        c.engage()
        XCTAssertEqual(c.desired, .init(firstPage: 1, pageCount: V2Window.pageCount), "anchor 1 - margin clamps to 1")
    }

    func testScrollInsideTheComfortableRangeDoesNotRefetch() {
        var c = V2Window.Controller()
        c.engage()
        let served = RenderingV2.Window(firstPage: 11, pageCount: 16, documentPageCount: 100)
        XCTAssertEqual(c.comfortableRange(served: served), (11 + V2Window.margin)...(26 - V2Window.margin))
        XCTAssertFalse(c.sawVisiblePage(20, served: served))
        XCTAssertTrue(c.sawVisiblePage(25, served: served), "within margin of the far edge")
        XCTAssertEqual(c.desired?.firstPage, 25 - V2Window.margin)
    }

    func testDocumentEdgesStayComfortable() {
        var c = V2Window.Controller()
        c.engage()
        let top = RenderingV2.Window(firstPage: 1, pageCount: 16, documentPageCount: 100)
        XCTAssertFalse(c.sawVisiblePage(1, served: top), "page 1 at the top of the first window never refetches")
        let bottom = RenderingV2.Window(firstPage: 85, pageCount: 16, documentPageCount: 100)
        XCTAssertFalse(c.sawVisiblePage(100, served: bottom), "the last page at the end of the last window never refetches")
        let whole = RenderingV2.Window(firstPage: 1, pageCount: 16, documentPageCount: 16)
        XCTAssertEqual(c.comfortableRange(served: whole), 1...16, "a window covering the whole document is comfortable everywhere")
    }

    func testNothingServedYetNeverRefetches() {
        var c = V2Window.Controller()
        c.engage()
        XCTAssertFalse(c.sawVisiblePage(50, served: nil), "the in-flight request will answer; re-anchoring waits for a served window")
    }

    // MARK: producer failure classification

    static func result(status: RuntimeV1.Status, diagnostics: [RuntimeV1.Diagnostic] = [], pages: [RuntimeV1.Page] = []) -> RuntimeV1.CompileResult {
        RuntimeV1.CompileResult(projectId: "p", revision: 1, status: status, pages: pages, diagnostics: diagnostics, pdfPath: nil)
    }

    func testOverLimitFailureIsRecognised() {
        let failed = Self.result(status: .failed, diagnostics: [
            .init(severity: .error, message: "compile_result would be 20339674 bytes for 385 pages, over the 16777216-byte reply limit; split the project or compile fewer pages", source: nil, recovery: nil),
        ])
        XCTAssertTrue(V2Window.overLimitFailure(failed))
        let other = Self.result(status: .failed, diagnostics: [.init(severity: .error, message: "Undefined control sequence", source: nil, recovery: nil)])
        XCTAssertFalse(V2Window.overLimitFailure(other), "an ordinary failed compile never engages the window")
        XCTAssertFalse(V2Window.overLimitFailure(Self.result(status: .ok)))
    }

    func testDeclinedSiblingIsRecognisedByItsCode() {
        let declined = Self.result(status: .recovered, diagnostics: [
            .init(severity: .warning, message: "display-list-v2 declined: the display_list line would be about 152106263 bytes for 385 pages, over the 16777216-byte line limit",
                  source: nil, recovery: nil, code: "display_list_declined"),
        ])
        XCTAssertTrue(V2Window.declinedSibling(declined))
        XCTAssertFalse(V2Window.declinedSibling(Self.result(status: .ok)))
    }

    // MARK: the REAL producer's bytes

    /// The wire shape as `flashtex-render` (main, #294) actually emits it:
    /// generated by a real windowed request, committed verbatim. Guards the
    /// consumer against drift in the producer's serialisation rather than
    /// against a hand-written approximation of it.
    func testRealProducerWindowedLineDecodesAndPaints() throws {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("Fixtures/display-list-v2-window.json")
        let data = try Data(contentsOf: url)
        let list = try RenderingV2.decode(data).payload
        let window = try XCTUnwrap(list.window)
        XCTAssertEqual(window, RenderingV2.Window(firstPage: 3, pageCount: 4, documentPageCount: 12))
        XCTAssertEqual(list.pages.count, 12, "one entry per document page, in order")
        XCTAssertEqual(list.pages.filter(\.resident).map(\.number), [3, 4, 5, 6])
        // Elided pages keep their geometry (the scroll extent) and carry nothing else.
        for page in list.pages where !page.resident {
            XCTAssertTrue(page.items.isEmpty)
            XCTAssertGreaterThan(page.width, 0)
            XCTAssertGreaterThan(page.height, 0)
        }
        // Resident pages carry real content and prepare like any other frame.
        XCTAssertFalse(list.pages[2].items.isEmpty)
        // The fast reader and JSONDecoder agree on the real bytes too.
        XCTAssertEqual(try RenderingV2Fast.envelope(data).payload, list)
        XCTAssertEqual(try JSONDecoder().decode(RenderingV2.Envelope.self, from: data).payload, list)
    }

    // MARK: §4.1 exclusions and per-request plumbing

    func testWindowedFrameIsNeverADeltaBase() throws {
        let envelope = try RenderingV2.decode(Self.windowedJSON())
        XCTAssertNil(DisplayListDelta.installed(from: envelope, pageBytes: [10, 10, 10], lineBytes: 100),
                     "a windowed reply is an incomplete view (§4.1): never installed as a delta base")
    }

    func testWindowIsAPerRequestCapability() {
        XCTAssertTrue(DisplayListDelta.perRequestCapabilities.contains(V2Window.capability))
        XCTAssertEqual(DisplayListDelta.stripPerRequest(["rules-v1", V2Window.capability, "display-list-v2"]),
                       ["rules-v1", "display-list-v2"])
    }
}

/// The engaged window through the live route: the producer's over-limit
/// failure engages it, the re-request paints a windowed frame, and leaving
/// the served window re-anchors it (fake_worker_v2.py `%v2window`).
@MainActor
final class V2WindowLiveTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fakeWorker = fixtures.appendingPathComponent("fake_worker_v2.py")
    static let template = fixtures.appendingPathComponent("display-list-v2-text.json")
    static let python = URL(fileURLWithPath: "/usr/bin/python3")

    private func waitUntil(timeout: TimeInterval = 15, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    func testOverLimitFailureEngagesTheWindowAndScrollingMovesIt() async throws {
        let tex = "%v2window\n" + (try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.tex"), encoding: .utf8))
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: tex)
        model.attachWorker(at: Self.python, arguments: [Self.fakeWorker.path, Self.template.path])
        model.previewV2 = true
        model.setLiveV2(true) // what PreviewV2Pane.onAppear does
        XCTAssertNil(model.v2Window.desired, "not engaged before the producer says the reply cannot fit")

        // First reply: the producer's over-limit failure. The shell engages a
        // window anchored at the viewer and re-requests the SAME revision on
        // its own; the second reply is the windowed frame.
        model.compile()
        try await waitUntil {
            model.inFlightRevision == nil && model.displayListV2?.isLoading == false && model.displayListV2?.frame?.list.window != nil
        }
        let frame = try XCTUnwrap(model.displayListV2?.frame)
        let window = try XCTUnwrap(frame.list.window)
        XCTAssertEqual(window, RenderingV2.Window(firstPage: 1, pageCount: V2Window.pageCount, documentPageCount: 40))
        XCTAssertEqual(frame.list.pages.count, 40, "one entry per document page")
        XCTAssertEqual(frame.list.pages.filter(\.resident).count, V2Window.pageCount)
        XCTAssertEqual(frame.list.pages.prefix(V2Window.pageCount).filter(\.resident).count, V2Window.pageCount)
        XCTAssertTrue(frame.list.pages.dropFirst(V2Window.pageCount).allSatisfy { !$0.resident && $0.items.isEmpty })
        XCTAssertEqual(model.result?.status, .ok)
        XCTAssertTrue(model.acceptedLayoutCapabilities.contains(V2Window.capability), "\(model.acceptedLayoutCapabilities)")
        XCTAssertNil(model.deltaInstalled, "a windowed frame is never a delta base")
        XCTAssertTrue(model.captureNote?.contains("window") == true, model.captureNote ?? "")

        // A windowed frame is never itself an export source (§4.1): Export
        // resolves the whole document instead of writing the resident pages.
        // (`exportPDF()` is not called here — it opens a save panel.)
        let resolved = await model.exportListURL()
        if case .success(let list) = resolved {
            XCTAssertTrue(list.temporary, "a windowed frame must never be exported as it stands")
            try? FileManager.default.removeItem(at: list.url)
        } else if case .failure(let why) = resolved {
            XCTAssertTrue(why.reason.contains("render pipeline") || why.reason.contains("flashtex-render"), why.reason)
        }

        // Scrolling inside the comfortable interior refetches nothing.
        let requestsBefore = model.latestRequestID
        model.v2WindowSawVisiblePage(6)
        XCTAssertEqual(model.latestRequestID, requestsBefore)

        // Leaving the served window re-anchors it at the viewer: desired
        // first 30 - margin = 26, which the 40-page producer clamps to 25.
        model.v2WindowSawVisiblePage(30)
        try await waitUntil {
            model.inFlightRevision == nil && model.displayListV2?.isLoading == false && model.displayListV2?.frame?.list.window?.firstPage == 25
        }
        let moved = try XCTUnwrap(model.displayListV2?.frame?.list.window)
        XCTAssertEqual(moved, RenderingV2.Window(firstPage: 25, pageCount: V2Window.pageCount, documentPageCount: 40))
    }

    func testAProducerWithoutTheCapabilityDisengagesInsteadOfLooping() async throws {
        // fake_worker_v2 without the directive accepts display-list-v2 but
        // never echoes the window capability: a positioned request that comes
        // back unhonoured must disengage (no re-request storm on scroll).
        let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.tex"), encoding: .utf8)
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: tex)
        model.attachWorker(at: Self.python, arguments: [Self.fakeWorker.path, Self.template.path])
        model.previewV2 = true
        model.setLiveV2(true)
        model.v2Window.engage() // as if a failure had engaged it earlier
        model.compile()
        try await waitUntil { model.inFlightRevision == nil && model.displayListV2?.isLoading == false && model.displayListV2?.frame != nil }
        XCTAssertNil(model.v2Window.desired, "disengaged: the producer did not honour the positioned window")
        XCTAssertNil(model.displayListV2?.frame?.list.window)
        let latest = model.latestRequestID
        model.v2WindowSawVisiblePage(3)
        XCTAssertEqual(model.latestRequestID, latest, "scrolling never re-requests once disengaged")
    }
}
