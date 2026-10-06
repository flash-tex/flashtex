import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXDisplayListV3
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// Hyperref links in the engine-v3 preview (DESIGN §10, app parity): a
/// click on a link opens an allowlisted URI or scrolls to the destination,
/// with the v2 pane's policy (`DisplayListLinks`), and hovering shows the
/// pointing hand and the target as a tooltip. The pure rules are checked
/// against the v2 functions; the pane test compiles a hyperref document in
/// a hosted window (needs a built `flashtex-host` and TeX Live, else skips).
@MainActor
final class EngineV3LinksTests: XCTestCase {
    static let sp = EngineV3Links.spPerBP

    func link(_ kind: UInt8, _ data: String, rect: [Double] = [0, 0, 100, 20], file: String = "") -> DL3Link {
        DL3Link(rect: rect.map { Int32(($0 * Self.sp).rounded()) }, span: 0, kind: kind, file: Array(file.utf8), data: Array(data.utf8))
    }

    /// As the engine sends it (display-list-v3 §4.5): the words pdfTeX
    /// writes for the kind, the others 0 (a fitr: a 100 × 20 bp rectangle).
    func dest(_ name: String, named: Bool = true, kind: UInt8 = 0, left: Double, top: Double) -> DL3Dest {
        let usesLeft: Set<UInt8> = [0, 3, 6, 7], usesTop: Set<UInt8> = [0, 2, 5, 7]
        let l = usesLeft.contains(kind) ? left : 0, t = usesTop.contains(kind) ? top : 0
        let rect = kind == 7 ? [l, t, l + 100, t + 20] : [l, t, 0, 0]
        return DL3Dest(named: named, name: Array(name.utf8), kind: kind,
                       rect: rect.map { Int32(($0 * Self.sp).rounded()) }, zoom: 0)
    }

    // MARK: the v2 pane's rules

    /// Every URI gets the v2 pane's verdict: http, https and mailto open
    /// (case-insensitive scheme), anything else (file, javascript, none) is blocked.
    func testURIPolicyMatchesTheV2Pane() {
        let uris = ["https://example.com/a?b=c", "http://example.com", "mailto:someone@example.com", "HTTPS://EXAMPLE.COM",
                    "file:///etc/passwd", "javascript:alert(1)", "ftp://example.com", "no-scheme", ""]
        for uri in uris {
            let v2 = DisplayListLinks.action(for: .init(page: 1, rects: [.init(x0: 0, y0: 0, x1: 1, y1: 1)], target: .uri(uri)), destinations: [:])
            let v3 = EngineV3Links.action(for: link(4, uri), pages: [:])
            switch (v2, v3) {
            case (.openURI(let a), .openURI(let b)): XCTAssertEqual(a, b, uri)
            case (.rejectedScheme(let a), .rejectedScheme(let b)): XCTAssertEqual(a, b, uri)
            default: XCTFail("\(uri): v2 \(v2), v3 \(v3)")
            }
        }
    }

    /// Overlapping links: the last one under the point wins, as `DisplayListLinks.hit` does.
    func testLastLinkUnderThePointWinsAsOnV2() {
        var page = DL3Page(kind: .page, index: 0)
        page.links = [link(4, "https://a.example", rect: [0, 0, 100, 100]), link(4, "https://b.example", rect: [50, 50, 150, 150])]
        // v2 rects are in ticks (2^20 per point).
        let t = DisplayListLinksTests.t
        let v2nav = RenderingV2.Navigation(links: [
            .init(page: 1, rects: [.init(x0: t(0), y0: t(0), x1: t(100), y1: t(100))], target: .uri("https://a.example")),
            .init(page: 1, rects: [.init(x0: t(50), y0: t(50), x1: t(150), y1: t(150))], target: .uri("https://b.example")),
        ], destinations: [:])
        for (x, y) in [(10.0, 10.0), (75.0, 75.0), (140.0, 140.0), (200.0, 10.0)] {
            let v3 = EngineV3Links.hit(page, at: CGPoint(x: x, y: y)).map(EngineV3Links.tooltip(for:))
            let v2 = DisplayListLinks.hit(v2nav, page: 1, viewX: x, viewY: y, scale: 1).map(DisplayListLinks.tooltip(for:))
            XCTAssertEqual(v3, v2, "(\(x), \(y))")
        }
    }

    /// `goto name`, `goto num` and `goto page` reach their page and place;
    /// an unknown name, another file and a raw action are reported, not followed.
    func testInternalDestinationsResolveAcrossPages() {
        var p0 = DL3Page(kind: .page, index: 0), p1 = DL3Page(kind: .page, index: 1)
        p1.dests = [dest("section.2", left: 72, top: 100), dest("7", named: false, left: 90, top: 300), dest("page.2", kind: 1, left: 0, top: 0)]
        p0.dests = [dest("Doc-Start", left: 0, top: 0)]
        let pages = [0: p0, 1: p1]
        guard case .reveal(let byName) = EngineV3Links.action(for: link(1, "section.2"), pages: pages) else { return XCTFail("goto name") }
        XCTAssertEqual(byName.page, 1)
        XCTAssertEqual(byName.rect.minX, 72, accuracy: 1e-4)
        XCTAssertEqual(byName.rect.minY, 100, accuracy: 1e-4)
        guard case .reveal(let byNum) = EngineV3Links.action(for: link(2, "7"), pages: pages) else { return XCTFail("goto num") }
        XCTAssertEqual(byNum.page, 1)
        XCTAssertEqual(byNum.rect.minY, 300, accuracy: 1e-4)
        guard case .reveal(let byPage) = EngineV3Links.action(for: link(3, "2 /Fit"), pages: pages) else { return XCTFail("goto page") }
        XCTAssertEqual(byPage.page, 1)
        XCTAssertEqual(byPage.rect.minY, 0)
        XCTAssertEqual(EngineV3Links.action(for: link(1, "nope"), pages: pages), .unknownDestination("nope"))
        XCTAssertEqual(EngineV3Links.action(for: link(1, "x", file: "other.pdf"), pages: pages), .unsupported("link to another file (other.pdf)"))
        if case .unsupported = EngineV3Links.action(for: link(5, "<</S/Launch>>"), pages: pages) {} else { XCTFail("raw action") }
    }

    /// Every dest kind reaches the place its words give: the words pdfTeX
    /// writes for the kind, the others 0 (#1568). A fitr's rectangle may
    /// come either way round; the others' right and bottom are not a place.
    func testEveryDestKindTargetsItsOwnWords() {
        let sp = Self.sp
        func at(_ kind: UInt8, _ rect: [Double]) -> CGPoint {
            let d = DL3Dest(named: true, name: Array("d".utf8), kind: kind,
                            rect: rect.map { Int32(($0 * sp).rounded()) }, zoom: 0)
            return EngineV3Links.target(of: d, page: 3).rect.origin
        }
        let cases: [(UInt8, [Double], CGPoint, String)] = [
            (0, [72, 100, 0, 0], CGPoint(x: 72, y: 100), "xyz"),
            (1, [0, 0, 0, 0], .zero, "fit"),
            (2, [0, 100, 0, 0], CGPoint(x: 0, y: 100), "fith"),
            (3, [90, 0, 0, 0], CGPoint(x: 90, y: 0), "fitv"),
            (4, [0, 0, 0, 0], .zero, "fitb"),
            (5, [0, 100, 0, 0], CGPoint(x: 0, y: 100), "fitbh"),
            (6, [90, 0, 0, 0], CGPoint(x: 90, y: 0), "fitbv"),
            (7, [60, 80, 160, 120], CGPoint(x: 60, y: 80), "fitr"),
            (7, [160, 120, 60, 80], CGPoint(x: 60, y: 80), "fitr, corners swapped"),
        ]
        for (kind, rect, want, what) in cases {
            let p = at(kind, rect)
            XCTAssertEqual(p.x, want.x, accuracy: 1e-4, what)
            XCTAssertEqual(p.y, want.y, accuracy: 1e-4, what)
        }
    }

    /// The model's notes are the v2 pane's for the same outcome.
    func testModelNotesMatchTheV2Pane() throws {
        var opened: [URL] = []
        let saved = DisplayListLinks.openURL
        DisplayListLinks.openURL = { opened.append($0); return true }
        defer { DisplayListLinks.openURL = saved }
        let model = ShellModel()
        let fixture = DisplayListLinksTests.fixtures.appendingPathComponent("display-list-v2-links.json")
        let v2list = try RenderingV2.decode(try Data(contentsOf: fixture)).payload
        let uri = "https://example.com"
        model.activatePreviewLink(.init(page: 1, rects: [], target: .uri(uri)), in: v2list)
        let v2note = model.navigationNote
        model.activateEngineV3Link(link(4, uri), pages: [:])
        XCTAssertEqual(model.navigationNote, v2note)
        model.activatePreviewLink(.init(page: 1, rects: [], target: .uri("file:///x")), in: v2list)
        let v2blocked = model.navigationNote
        model.activateEngineV3Link(link(4, "file:///x"), pages: [:])
        XCTAssertEqual(model.navigationNote, v2blocked)
        XCTAssertEqual(opened, [URL(string: uri)!, URL(string: uri)!])
    }

    // MARK: the pane

    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-links-tests-\(getpid())")
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

    static let document = """
    \\documentclass{article}
    \\usepackage{hyperref}
    \\begin{document}
    \\section{First}\\label{sec:a}
    See \\url{https://example.com/flashtex} and Section~\\ref{sec:b}.
    \\newpage
    \\section{Second}\\label{sec:b}
    Target text.
    \\end{document}

    """

    /// With v3 on, a click on a `\\url` opens it, a click on a `\\ref` scrolls
    /// to the section on page 2 (no reverse search either time), and
    /// hovering a link shows its target as the tooltip.
    func testHyperrefLinksInThePaneOpenURLsAndScrollToTargets() async throws {
        try EngineV3TestHost.require()
        var opened: [URL] = []
        let saved = DisplayListLinks.openURL
        DisplayListLinks.openURL = { opened.append($0); return true }
        defer { DisplayListLinks.openURL = saved }
        let model = ShellModel()
        model.replaceProject(entryText: Self.document, named: "main.tex")
        model.engineV3Enabled = true
        let s = model.engineV3
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 640, height: 820), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        s.start(model: model)
        defer { s.stop(); window.contentView = nil }
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 2 && s.pages[0] != nil && s.pages[1] != nil }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        let links = s.pages[0]!.page.links
        let url = try XCTUnwrap(links.first { $0.kind == 4 }, "the \\url link: \(links.map(\.kind))")
        XCTAssertEqual(url.dataString, "https://example.com/flashtex")
        let ref = try XCTUnwrap(links.first { $0.kind == 1 && $0.dataString.hasPrefix("section.2") }, "the \\ref link: \(links.map(\.dataString))")

        func click(_ l: DL3Link) throws {
            let r = EngineV3Links.rect(l)
            let p = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: r.midX, y: r.midY)))
            let event = try XCTUnwrap(NSEvent.mouseEvent(with: .leftMouseDown, location: pages.convert(p, to: nil), modifierFlags: [],
                                                         timestamp: 0, windowNumber: window.windowNumber, context: nil,
                                                         eventNumber: 0, clickCount: 1, pressure: 1))
            pages.mouseDown(with: event)
        }
        let selectionBefore = model.selection

        // The URL: opened through the allowlist, no reverse search.
        try click(url)
        XCTAssertEqual(opened, [URL(string: "https://example.com/flashtex")!])
        XCTAssertEqual(model.navigationNote, "Opened https://example.com/flashtex")
        XCTAssertEqual(model.selection, selectionBefore, "a link click does not reverse-search")

        // The \ref: the preview scrolls so that section 2's anchor on page 2 is in view.
        let clip = try XCTUnwrap(pages.enclosingScrollView?.contentView)
        clip.scroll(to: .zero)
        try click(ref)
        XCTAssertEqual(model.navigationNote, "Scrolled to page 2")
        let anchor = try XCTUnwrap(s.pages[1]!.page.dests.first { $0.dataName.hasPrefix("section.2") })
        let target = EngineV3Links.target(of: anchor, page: 1)
        let anchorInView = try XCTUnwrap(pages.viewPoint(page: 1, target.rect.origin))
        XCTAssertTrue(clip.bounds.contains(anchorInView), "\(anchorInView) in \(clip.bounds)")
        XCTAssertEqual(opened.count, 1)
        XCTAssertEqual(model.selection, selectionBefore)

        // Hover: the link's target as the tooltip; off the link, none.
        let r = EngineV3Links.rect(url)
        pages.hover(at: pages.viewPoint(page: 0, CGPoint(x: r.midX, y: r.midY)))
        XCTAssertEqual(pages.toolTip, "https://example.com/flashtex")
        pages.hover(at: pages.viewPoint(page: 0, CGPoint(x: 2, y: 2)))
        XCTAssertNil(pages.toolTip)
        XCTAssertNil(pages.hoveredLink)
    }
}

private extension DL3Dest {
    var dataName: String { String(decoding: name, as: UTF8.self) }
}
