import AppKit
import XCTest
@testable import FlashTeXMac

/// The stale marker (dimmed, orange border) appears only once a page has
/// stayed stale past the delay, clears at once, and is off with the setting;
/// `axStale` always reports the true state immediately.
@MainActor
final class PreviewStaleDimmingTests: XCTestCase {
    private func page(delay: TimeInterval? = 0.05, dims: Bool = true) -> EngineV3PageView {
        let v = EngineV3PageView(frame: NSRect(x: 0, y: 0, width: 100, height: 140))
        v.dimDelay = delay
        v.dimsStale = dims
        return v
    }

    private func spin(_ seconds: TimeInterval) {
        RunLoop.main.run(until: Date().addingTimeInterval(seconds))
    }

    private func assertLook(_ v: EngineV3PageView, dimmed: Bool, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(v.showsStaleLook, dimmed, file: file, line: line)
        XCTAssertEqual(v.layer?.opacity, dimmed ? 0.45 : 1, file: file, line: line)
        XCTAssertEqual(v.layer?.borderWidth, dimmed ? 2 : 0, file: file, line: line)
    }

    func testDefaultIsOnWithADelay() {
        XCTAssertTrue(PreviewStaleDimming.isEnabled(in: UserDefaults(suiteName: "stale-dim-\(UUID())")!))
        XCTAssertGreaterThan(PreviewStaleDimming.delay, 0.2)
    }

    func testDimsOnlyAfterTheDelayAndClearsAtOnce() {
        let v = page()
        v.setStale(true)
        XCTAssertTrue(v.axStale, "VoiceOver hears stale at once")
        assertLook(v, dimmed: false)
        spin(0.2)
        assertLook(v, dimmed: true)
        v.setStale(false)
        XCTAssertFalse(v.axStale)
        assertLook(v, dimmed: false)
    }

    func testAPageFreshWithinTheDelayNeverFlashes() {
        let v = page()
        v.setStale(true)
        v.setStale(false)
        spin(0.2)
        assertLook(v, dimmed: false)
    }

    func testOffNeverDimsAndTogglingAppliesLive() {
        let v = page(delay: nil, dims: false)
        v.setStale(true)
        XCTAssertTrue(v.axStale)
        assertLook(v, dimmed: false)
        v.dimsStale = true
        assertLook(v, dimmed: true)
        v.dimsStale = false
        assertLook(v, dimmed: false)
        XCTAssertTrue(v.axStale, "still stale, only not dimmed")
    }

    func testStoredSnapshotFollowsTheSameRule() {
        let v = page()
        v.showsStored = true
        XCTAssertTrue(v.axStale)
        assertLook(v, dimmed: false)
        spin(0.2)
        assertLook(v, dimmed: true)
        v.showsStored = false
        assertLook(v, dimmed: false)

        let off = page(delay: nil, dims: false)
        off.showsStored = true
        assertLook(off, dimmed: false)
    }
}
