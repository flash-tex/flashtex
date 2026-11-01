import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// The window chrome under the new engine, with no host (`FLASHTEX_HOST=none`):
/// the title bar's Compile tooltip and the preview HUD's tooltip name the new
/// engine with its route help (app-parity row C21), and say only that the
/// engine is not running while its session is off; the HUD's "N / M"
/// readout and the pane's accessibility value count the v3 pages (rows C9,
/// C20).
@MainActor
final class EngineV3ChromeTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-chrome-\(getpid())")
    private var env = EnvironmentOverride()

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_HOST", "none") // the v3 route without a host process
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
    }

    override func tearDown() {
        env.restore()
        try? FileManager.default.removeItem(at: Self.cache)
    }

    static let document = "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n"
    static let sources = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Sources/FlashTeXMac")

    private func makeModel(v3: Bool) -> ShellModel {
        let model = ShellModel()
        model.replaceProject(entryText: Self.document)
        model.engineV3Enabled = v3
        model.flushChrome()
        return model
    }

    // MARK: C21: the tooltips

    func testUnderV3BothTooltipsCarryTheRouteHelpNotTheProducer() {
        let model = makeModel(v3: true)
        defer { model.engineV3Enabled = false }
        XCTAssertEqual(model.chrome.route, .engineV3)
        let routeHelp = model.chrome.routeHelp
        XCTAssertFalse(routeHelp.isEmpty)

        let compile = TitleBarRow.compileHelp(route: model.chrome.route, routeHelp: routeHelp)
        let hud = PreviewHUD.help(chrome: model.chrome)
        for tip in [compile, hud] {
            XCTAssertTrue(tip.contains(routeHelp), tip)
            XCTAssertFalse(tip.contains("producer"), tip)
        }
        XCTAssertEqual(compile, "Compile (File > Compile, ⌘B) — engine v3: " + routeHelp)
        XCTAssertEqual(hud, "Engine v3 preview — " + routeHelp)
    }

    /// There is no old route any more (S6): with the engine's session off
    /// (route `.none`), the Compile tooltip is the bare command and the HUD
    /// says the engine is not running; neither names a producer.
    func testOnTheOldRouteBothTooltipsAreUnchanged() {
        let model = makeModel(v3: false)
        XCTAssertEqual(model.chrome.route, .none)
        XCTAssertEqual(model.chrome.routeHelp, "")
        XCTAssertEqual(TitleBarRow.compileHelp(route: model.chrome.route, routeHelp: model.chrome.routeHelp),
                       "Compile (File > Compile, ⌘B)")
        XCTAssertEqual(PreviewHUD.help(chrome: model.chrome), "Preview — the engine is not running")
    }

    /// The views use these helpers (so the strings tested above are the ones shown).
    func testTheViewsUseTheTooltipHelpers() throws {
        let titleBar = try String(contentsOf: Self.sources.appendingPathComponent("TitleBar.swift"), encoding: .utf8)
        XCTAssertTrue(titleBar.contains(".help(TitleBarRow.compileHelp(route: model.chrome.route, routeHelp: model.chrome.routeHelp))"))
        XCTAssertEqual(titleBar.components(separatedBy: "— engine v3: ").count - 1, 1, "the route wording lives in the helper only")
        let content = try String(contentsOf: Self.sources.appendingPathComponent("ContentView.swift"), encoding: .utf8)
        XCTAssertTrue(content.contains(".help(Self.help(chrome: chrome))"))
        XCTAssertTrue(content.contains("if let readout = Self.pageReadout(page: model.previewVisiblePage, of: pageCount)"))
        XCTAssertTrue(content.contains("let pageCount = model.previewPageCount"))
    }

    // MARK: C9 and C20: the page readout and the pane's value

    func testTheReadoutAndThePaneValueCountTheV3Pages() {
        let model = makeModel(v3: true)
        defer { model.engineV3Enabled = false }
        let s = model.engineV3
        s.handle(.pages(.object(["count": .int(4)])))
        XCTAssertEqual(s.pageCount, 4)
        XCTAssertEqual(model.previewPageCount, 4, "the preview counts the v3 pages")
        model.previewVisiblePage = 2
        XCTAssertEqual(PreviewHUD.pageReadout(page: model.previewVisiblePage, of: model.previewPageCount), "2 / 4")
        XCTAssertEqual(PreviewPane.accessibilityValue(page: model.previewVisiblePage, of: model.previewPageCount), "Page 2 of 4")
        XCTAssertEqual(PreviewHUD.pageReadout(page: 9, of: 4), "4 / 4", "clamped to the count")
        XCTAssertNil(PreviewHUD.pageReadout(page: 1, of: 0), "no pages, no readout")

        model.engineV3Enabled = false
        XCTAssertEqual(model.previewPageCount, 0, "the session off: no pages")
        XCTAssertNil(PreviewHUD.pageReadout(page: model.previewVisiblePage, of: model.previewPageCount))
    }
}
