import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import XCTest
@testable import FlashTeXMac

/// Settings > Performance (PerformanceMode.swift; lane PERF-MODES).
@MainActor
final class PerformanceModeTests: XCTestCase {
    private var saved: Any?

    override func setUp() {
        saved = UserDefaults.standard.object(forKey: PerformanceMode.defaultsKey)
    }

    override func tearDown() {
        if let saved { UserDefaults.standard.set(saved, forKey: PerformanceMode.defaultsKey) } else {
            UserDefaults.standard.removeObject(forKey: PerformanceMode.defaultsKey)
        }
        PerformanceMode.apply(PerformanceMode.stored)
    }

    func testBalancedIsTheDefault() {
        UserDefaults.standard.removeObject(forKey: PerformanceMode.defaultsKey)
        XCTAssertEqual(PerformanceMode.stored, .balanced)
        UserDefaults.standard.set("turbo", forKey: PerformanceMode.defaultsKey)
        XCTAssertEqual(PerformanceMode.stored, .balanced, "an unknown stored value is Balanced")
    }

    func testHostProfileNamesAreTheProtocols() {
        // docs/protocol/display-list-v3.md §6.9
        XCTAssertEqual(PerformanceMode.allCases.map(\.hostProfile), ["low-memory", "balanced", "high-performance"])
        XCTAssertEqual(DL3.profileCapability, "profile-v1")
        XCTAssertEqual(DL3.Kind.name(DL3.Kind.cProfile), "client-profile")
        XCTAssertEqual(DL3.Kind.name(DL3.Kind.profile), "profile")
    }

    func testBalancedKeepsTodaysCaches() {
        XCTAssertEqual(PerformanceMode.balanced.keptRasters, 2)
        XCTAssertEqual(PerformanceMode.balanced.overscanScreens, 1)
        XCTAssertEqual(PerformanceMode.balanced.imageLimits.count, 256)
        XCTAssertEqual(PerformanceMode.balanced.imageLimits.bytes, 512 << 20)
    }

    func testModesOrderMemory() {
        let l = PerformanceMode.lowMemory, b = PerformanceMode.balanced, h = PerformanceMode.highPerformance
        XCTAssertLessThan(l.keptRasters, b.keptRasters)
        XCTAssertLessThan(b.keptRasters, h.keptRasters)
        XCTAssertLessThan(l.overscanScreens, b.overscanScreens)
        XCTAssertLessThan(b.overscanScreens, h.overscanScreens)
        XCTAssertLessThan(l.imageLimits.bytes, b.imageLimits.bytes)
        XCTAssertLessThan(b.imageLimits.bytes, h.imageLimits.bytes)
        for m in PerformanceMode.allCases { XCTAssertFalse(m.explanation.isEmpty) }
    }

    func testChangingTheModeAppliesItAndTellsTheSessions() {
        PerformanceMode.stored = .balanced
        let note = expectation(forNotification: PerformanceMode.changed, object: nil) { n in
            (n.userInfo?["mode"] as? String) == "low-memory"
        }
        PerformanceMode.stored = .lowMemory
        wait(for: [note], timeout: 2)
        XCTAssertEqual(PerformanceMode.current, .lowMemory)
        XCTAssertEqual(DL3ResourceCache.shared.imageLimit, 64)
        XCTAssertEqual(DL3ResourceCache.shared.imageByteLimit, 96 << 20)
        XCTAssertEqual(EngineV3RasterHolder.budget, ProcessInfo.processInfo.environment["FLASHTEX_V3_KEPT_RASTERS"].flatMap(Int.init) ?? 1)
    }

    func testMemoryPressureOnlySuggests() {
        PerformanceMode.stored = .balanced
        let a = PerformanceAdvisor.shared
        a.pressure()
        XCTAssertTrue(a.suggestingLowMemory, "pressure in Balanced suggests Low Memory")
        XCTAssertEqual(PerformanceMode.current, .balanced, "and never switches by itself")
        a.notNow()
        XCTAssertFalse(a.suggestingLowMemory)
        a.pressure()
        XCTAssertFalse(a.suggestingLowMemory, "not again after Not Now")
    }
}
