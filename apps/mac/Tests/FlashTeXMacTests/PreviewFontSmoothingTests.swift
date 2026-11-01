import AppKit
import XCTest
@testable import FlashTeXMac

/// Settings > "Smooth fonts in preview" (PreviewFontSmoothing.swift): off by
/// default, and the flag reaches the preview rasterizer as a value.
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

    /// The v3 pane: the session hands its flag to the reader thread's plan.
    func testEngineV3SessionHandsTheFlagToTheReaderPlan() {
        let session = EngineV3Session(smoothFonts: false)
        let target = EngineV3LayerTarget(layer: CALayer())
        session.rasterPlan.set(targets: [0: target], pixelsPerPoint: 2, appearance: .light)
        XCTAssertEqual(session.rasterPlan.target(for: 0)?.smoothFonts, false)
        session.smoothFonts = true
        XCTAssertTrue(session.rasterPlan.smoothFonts)
        XCTAssertEqual(session.rasterPlan.target(for: 0)?.smoothFonts, true)
        XCTAssertTrue(session.rasterPlan.appearance == .light, "the smoothing flag leaves the plan's appearance alone")
    }

    /// A v3 page bitmap is keyed on appearance and smoothing, so either
    /// change redraws it; off keeps the key it had before the setting.
    func testEngineV3ContentKeyCoversAppearanceAndSmoothing() {
        let h: [UInt8] = [1, 2, 3]
        XCTAssertEqual(EngineV3PagesView.contentKey(h, .light), h + [0])
        XCTAssertEqual(EngineV3PagesView.contentKey(h, .light, smoothFonts: false), h + [0])
        let keys = [EngineV3PagesView.contentKey(h, .light), EngineV3PagesView.contentKey(h, .dark),
                    EngineV3PagesView.contentKey(h, .light, smoothFonts: true), EngineV3PagesView.contentKey(h, .dark, smoothFonts: true)]
        XCTAssertEqual(Set(keys).count, 4)
    }
}
