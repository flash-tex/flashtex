import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Mac consumer of `display-list-v2-links` (protocol/proposals/display-list-v2-links.md §3, §5, §6).
/// Fixtures live under Mac test resources, not protocol/ frozen contracts.
final class DisplayListLinksTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let ticks = RenderingV2.ticksPerPoint
    static func t(_ pt: Double) -> Int64 { Int64((pt * Double(ticks)).rounded()) }

    private func fixture() throws -> Data {
        try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-links.json"))
    }

    private func list() throws -> RenderingV2.DisplayList {
        try RenderingV2.decode(try fixture()).payload
    }

    // MARK: decoding

    func testProposalNavigationDecodesOnBothReadersAndAbsenceIsTolerated() throws {
        let env = try RenderingV2.decode(try fixture())
        let nav = try XCTUnwrap(env.payload.navigation)
        XCTAssertEqual(nav.links.count, 6)
        XCTAssertEqual(nav.links[0].className, "link")
        XCTAssertEqual(nav.links[0].target, .destination("section.1"))
        XCTAssertEqual(nav.links[0].source, .init(document: "main.tex", start: 120, end: 139))
        XCTAssertEqual(nav.links[0].rects, [.init(x0: Self.t(72), y0: Self.t(120), x1: Self.t(150), y1: Self.t(132))])
        XCTAssertEqual(nav.links[1].target, .uri("https://example.com"))
        XCTAssertEqual(nav.links[2].rects.count, 2, "wrapped url as rects[]")
        XCTAssertEqual(nav.destinations["section.1"]?.page, 2)
        XCTAssertEqual(nav.destinations["section.1"]?.view, "xyz")
        XCTAssertEqual(nav.outline?.first?.title, "Introduction")
        XCTAssertEqual(nav.outlineOpen, false)
        XCTAssertEqual(nav.pageMode, "use_outlines")
        XCTAssertEqual(nav.info?.creator, "LaTeX with hyperref")

        let fast = try RenderingV2Fast.envelope(try fixture())
        XCTAssertEqual(fast.payload.navigation, nav)

        // A list without the object (legacy fixture) is unchanged.
        let text = try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        XCTAssertNil(try RenderingV2.decode(text).payload.navigation)
        XCTAssertNil(try RenderingV2Fast.envelope(text).payload.navigation)
    }

    func testUnknownNavigationKeysAndMissingOptionalFieldsAreIgnored() throws {
        let raw = try JSONSerialization.jsonObject(with: try fixture()) as! [String: Any]
        var payload = raw["payload"] as! [String: Any]
        var nav = payload["navigation"] as! [String: Any]
        nav["vendor_extra"] = "tolerated"
        var links = nav["links"] as! [[String: Any]]
        links[0].removeValue(forKey: "border")
        links[0].removeValue(forKey: "color")
        links[0].removeValue(forKey: "source")
        nav["links"] = links
        payload["navigation"] = nav
        var obj = raw
        obj["payload"] = payload
        let data = try JSONSerialization.data(withJSONObject: obj)
        let decoded = try RenderingV2.decode(data).payload.navigation
        XCTAssertEqual(decoded?.links.first?.className, "link")
        XCTAssertNil(decoded?.links.first?.source)
        XCTAssertNil(decoded?.links.first?.border)
    }

    // MARK: capability gating

    @MainActor
    func testLinksRideWithV2AndAreStrippedWithoutIt() {
        let model = ShellModel()
        XCTAssertFalse(model.requestedLayoutCapabilities.contains(DisplayListLinks.capability))
        model.setLiveV2(true)
        XCTAssertTrue(model.requestedLayoutCapabilities.contains(V2Live.capability))
        XCTAssertTrue(model.requestedLayoutCapabilities.contains(DisplayListLinks.capability))
        XCTAssertEqual(model.requestedLayoutCapabilities.filter { $0 == DisplayListLinks.capability }.count, 1)
        model.setLiveV2(true)
        XCTAssertEqual(model.requestedLayoutCapabilities.filter { $0 == DisplayListLinks.capability }.count, 1, "idempotent")
        model.setLiveV2(false)
        XCTAssertFalse(model.requestedLayoutCapabilities.contains(DisplayListLinks.capability))
        XCTAssertEqual(model.requestedLayoutCapabilities, ShellModel.defaultLayoutCapabilities())

        XCTAssertEqual(DisplayListLinks.sent(with: ["display-list-v2"]), ["display-list-v2"])
        XCTAssertEqual(DisplayListLinks.sent(with: ["rules-v1", DisplayListLinks.capability]), ["rules-v1"])
        XCTAssertEqual(DisplayListLinks.sent(with: ["display-list-v2", DisplayListLinks.capability]),
                       ["display-list-v2", DisplayListLinks.capability])
    }

    func testLiveFramesIgnoreNavigationUnlessTheProducerEchoedTheCapability() throws {
        let nav = try XCTUnwrap(try list().navigation)
        XCTAssertNil(DisplayListLinks.effective(nav, accepted: [V2Live.capability], live: true),
                     "producer did not echo display-list-v2-links")
        XCTAssertNil(DisplayListLinks.effective(nav, accepted: [], live: true))
        XCTAssertEqual(DisplayListLinks.effective(nav, accepted: [V2Live.capability, DisplayListLinks.capability], live: true), nav)
        XCTAssertEqual(DisplayListLinks.effective(nav, accepted: [], live: false), nav, "file/fixture: object present is enough")
        XCTAssertNil(DisplayListLinks.effective(nil, accepted: [DisplayListLinks.capability], live: true))
    }

    @MainActor
    func testConfigureLayoutKeepsLinksWhenV2IsTheHelpersToEnroll() {
        let model = ShellModel()
        model.setLiveV2(true)
        let layout = model.displayCandidatesConfigureLayoutCapabilities(model.requestedLayoutCapabilities)
        XCTAssertFalse(layout.contains(V2Live.capability), "helper enrolls display-list-v2 itself")
        XCTAssertTrue(layout.contains(DisplayListLinks.capability))
        XCTAssertTrue(layout.contains(RenderingV2.imagesCapability))
    }

    // MARK: hit-testing

    func testHitTestFindsEachRectOfAWrappedLinkAndHonoursPageScaleAndOffset() throws {
        let nav = try XCTUnwrap(try list().navigation)
        // First piece of the wrapped URL (page 1, 200…250 × 100…112 pt).
        XCTAssertEqual(DisplayListLinks.hit(nav, page: 1, tickX: Self.t(220), tickY: Self.t(106))?.target,
                       .uri("https://example.com/wrapped"))
        // Second piece (72…120 × 140…152 pt).
        XCTAssertEqual(DisplayListLinks.hit(nav, page: 1, tickX: Self.t(80), tickY: Self.t(145))?.target,
                       .uri("https://example.com/wrapped"))
        // Gap between the stacked dest (y 120…132) and the wrapped second piece (y 140…152).
        XCTAssertNil(DisplayListLinks.hit(nav, page: 1, tickX: Self.t(80), tickY: Self.t(136)))
        // Far from every rect on page 1.
        XCTAssertNil(DisplayListLinks.hit(nav, page: 1, tickX: Self.t(10), tickY: Self.t(10)))
        // Half-open: the max edge is out.
        XCTAssertNil(DisplayListLinks.hit(nav, page: 1, tickX: Self.t(250), tickY: Self.t(106)))
        // Page 2's mailto, not page 1.
        XCTAssertNil(DisplayListLinks.hit(nav, page: 1, tickX: Self.t(5), tickY: Self.t(5)))
        XCTAssertEqual(DisplayListLinks.hit(nav, page: 2, tickX: Self.t(5), tickY: Self.t(5))?.target,
                       .uri("mailto:tex@example.com"))

        // scale=2, origin (10, 20): view (10 + 220*2, 20 + 106*2) → page (220, 106).
        let view = DisplayListLinks.hit(nav, page: 1, viewX: 10 + 220 * 2, viewY: 20 + 106 * 2, scale: 2, originX: 10, originY: 20)
        XCTAssertEqual(view?.target, .uri("https://example.com/wrapped"))
        let ticks = DisplayListLinks.ticks(viewX: 10 + 72, viewY: 20 + 100, scale: 1, originX: 10, originY: 20)
        XCTAssertEqual(ticks.x, Self.t(72))
        XCTAssertEqual(ticks.y, Self.t(100))
    }

    func testHitTestPrefersTheLaterLinkWhenRectsOverlap() {
        let a = RenderingV2.Navigation.Link(page: 1, rects: [.init(x0: 0, y0: 0, x1: 100, y1: 100)], target: .uri("https://a.example"))
        let b = RenderingV2.Navigation.Link(page: 1, rects: [.init(x0: 50, y0: 50, x1: 150, y1: 150)], target: .uri("https://b.example"))
        let nav = RenderingV2.Navigation(links: [a, b])
        XCTAssertEqual(DisplayListLinks.hit(nav, page: 1, tickX: 60, tickY: 60)?.target, .uri("https://b.example"))
        XCTAssertEqual(DisplayListLinks.hit(nav, page: 1, tickX: 10, tickY: 10)?.target, .uri("https://a.example"))
    }

    // MARK: scheme allowlist

    func testSchemeAllowlistAcceptsHttpHttpsMailtoAndRejectsFileJavascriptAndSchemeless() throws {
        let nav = try XCTUnwrap(try list().navigation)
        let dests = nav.destinations

        guard case .openURI(let https) = DisplayListLinks.action(for: nav.links[1], destinations: dests) else {
            return XCTFail("https")
        }
        XCTAssertEqual(https.absoluteString, "https://example.com")
        XCTAssertNotNil(DisplayListLinks.allowedURL(from: "HTTP://Example.COM/x"))
        XCTAssertNotNil(DisplayListLinks.allowedURL(from: "mailto:tex@example.com"))

        guard case .rejectedScheme("file") = DisplayListLinks.action(for: nav.links[4], destinations: dests) else {
            return XCTFail("file:")
        }
        guard case .rejectedScheme("javascript") = DisplayListLinks.action(for: nav.links[5], destinations: dests) else {
            return XCTFail("javascript:")
        }
        XCTAssertNil(DisplayListLinks.allowedURL(from: "data:text/html,hi"))
        XCTAssertNil(DisplayListLinks.allowedURL(from: "ftp://example.com"))
        XCTAssertNil(DisplayListLinks.allowedURL(from: "/etc/passwd"))
        XCTAssertNil(DisplayListLinks.allowedURL(from: "example.com"))

        guard case .reveal(let dest) = DisplayListLinks.action(for: nav.links[0], destinations: dests) else {
            return XCTFail("internal dest")
        }
        XCTAssertEqual(dest.page, 2)
        XCTAssertEqual(dest.y, Self.t(10))
        let missing = RenderingV2.Navigation.Link(page: 1, rects: [.init(x0: 0, y0: 0, x1: 1, y1: 1)], target: .destination("nope"))
        guard case .unknownDestination("nope") = DisplayListLinks.action(for: missing, destinations: dests) else {
            return XCTFail("unknown dest")
        }
    }

    @MainActor
    func testActivatePreviewLinkOpensAllowlistedURIsAndRecordsInternalReveal() throws {
        let list = try list()
        let nav = try XCTUnwrap(list.navigation)
        let model = ShellModel()
        var opened: [URL] = []
        DisplayListLinks.openURL = { opened.append($0); return true }
        defer { DisplayListLinks.openURL = { NSWorkspace.shared.open($0) } }

        model.activatePreviewLink(nav.links[1], in: list)
        XCTAssertEqual(opened.map(\.absoluteString), ["https://example.com"])
        guard case .openURI = model.lastPreviewLinkAction else { return XCTFail() }

        opened.removeAll()
        model.activatePreviewLink(nav.links[0], in: list)
        XCTAssertTrue(opened.isEmpty, "internal dest does not open a URL")
        guard case .reveal(let dest) = model.lastPreviewLinkAction else { return XCTFail() }
        XCTAssertEqual(dest.page, 2)
        XCTAssertEqual(model.previewReveal?.target.page, 2)
        XCTAssertEqual(model.previewReveal?.reason, .explicit)

        model.activatePreviewLink(nav.links[4], in: list)
        XCTAssertTrue(opened.isEmpty)
        guard case .rejectedScheme("file") = model.lastPreviewLinkAction else { return XCTFail() }
        XCTAssertTrue(model.navigationNote?.contains("file") == true)
    }
}
