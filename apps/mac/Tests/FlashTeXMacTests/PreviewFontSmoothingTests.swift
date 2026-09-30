import AppKit
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Settings > "Smooth fonts in preview" (PreviewFontSmoothing.swift): off by
/// default, and the flag reaches both preview rasterizers as a value.
@MainActor
final class PreviewFontSmoothingTests: XCTestCase {
    private func freshDefaults() -> UserDefaults {
        let name = "PreviewFontSmoothingTests-\(UUID().uuidString)"
        let d = UserDefaults(suiteName: name)!
        d.removePersistentDomain(forName: name)
        return d
    }

    func testDefaultIsOff() {
        XCTAssertFalse(PreviewFontSmoothing.defaultValue)
        let d = freshDefaults()
        XCTAssertFalse(PreviewFontSmoothing.isEnabled(in: d), "nothing stored: off (exact parity)")
        d.set(true, forKey: PreviewFontSmoothing.key)
        XCTAssertTrue(PreviewFontSmoothing.isEnabled(in: d))
        XCTAssertFalse(V2PageRasterizer().smoothFonts)
        XCTAssertFalse(EngineV3RasterPlan().smoothFonts)
        XCTAssertFalse(EngineV3Session(smoothFonts: false).rasterPlan.smoothFonts)
    }

    func testSettingStoresAndNotifies() {
        let saved = UserDefaults.standard.object(forKey: PreviewFontSmoothing.key)
        defer { UserDefaults.standard.set(saved, forKey: PreviewFontSmoothing.key) }
        var seen: [Bool] = []
        let token = PreviewFontSmoothing.observe { seen.append($0) }
        defer { NotificationCenter.default.removeObserver(token) }
        PreviewFontSmoothing.enabled = true
        XCTAssertTrue(PreviewFontSmoothing.enabled)
        PreviewFontSmoothing.enabled = false
        XCTAssertFalse(PreviewFontSmoothing.enabled)
        XCTAssertEqual(seen, [true, false])
    }

    /// The v2 pane's rasterizer draws with its flag, keys bitmaps on it, and
    /// a toggle drops the bitmaps drawn the other way so the pages redraw.
    func testV2RasterizerDrawsWithTheFlagAndRedrawsOnToggle() throws {
        let envelope = try RenderingV2.decode(try Data(contentsOf: PreviewV2ShellTests.fixtures.appendingPathComponent("display-list-v2-text.json")))
        let frame = try V2Frame.prepare(envelope, store: PreviewV2Tests.store)
        let page = frame.prepared[0], token = frame.pageToken(at: 0)
        let rasterizer = V2PageRasterizer(maxBytes: 64 << 20)
        rasterizer.setCurrent(frame: frame)

        func bitmap() throws -> CGImage {
            _ = rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 2, dark: false)
            let deadline = Date().addingTimeInterval(20)
            while rasterizer.images.isEmpty, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
            return try XCTUnwrap(rasterizer.image(for: page, pageToken: token, pixelsPerPoint: 2, dark: false))
        }

        let off = try bitmap()
        XCTAssertEqual(V2Parity.rgba(off), V2Parity.rgba(try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: 2))), "off: the parity bytes")
        XCTAssertEqual(rasterizer.rasterHint?.smoothFonts, false)

        rasterizer.smoothFonts = true
        XCTAssertTrue(rasterizer.images.isEmpty, "bitmaps drawn without smoothing are dropped")
        XCTAssertEqual(rasterizer.rasterHint?.smoothFonts, true, "the loader pre-rasterizes new frames smoothed too")
        let on = try bitmap()
        XCTAssertEqual(V2Parity.rgba(on), V2Parity.rgba(try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: 2, smoothFonts: true))))
        XCTAssertGreaterThan(V2Parity.differingPixels(V2Parity.rgba(off), V2Parity.rgba(on)), 0, "smoothing changes the glyphs")
        XCTAssertNotNil(rasterizer.images[V2PageRasterizer.Key(pageToken: token, pixelsPerPoint: 2, dark: false, smoothFonts: true)])

        let prerastered = V2Loader.preraster(frame, hint: try XCTUnwrap(rasterizer.rasterHint.map { var h = $0; h.cachedTokens = []; return h }))
        XCTAssertTrue(prerastered.smoothFonts)
        XCTAssertEqual(prerastered.images.first.map { V2Parity.rgba($0.image) }, V2Parity.rgba(on))
    }

    /// The v3 pane: the session hands its flag to the reader thread's plan.
    func testEngineV3SessionHandsTheFlagToTheReaderPlan() {
        let session = EngineV3Session(smoothFonts: false)
        let target = EngineV3LayerTarget(layer: CALayer())
        session.rasterPlan.set(targets: [0: target], pixelsPerPoint: 2)
        XCTAssertEqual(session.rasterPlan.target(for: 0)?.smoothFonts, false)
        session.smoothFonts = true
        XCTAssertTrue(session.rasterPlan.smoothFonts)
        XCTAssertEqual(session.rasterPlan.target(for: 0)?.smoothFonts, true)
    }
}
