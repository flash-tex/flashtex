import XCTest
import CoreGraphics
@testable import FlashTeXMac

/// "Preview follows the caret" (gap: follow-caret, lane mac-follow-caret):
/// the pure decision function (`FollowCaret.decide`) and the debounce
/// coalescing policy (`FollowCaret.debounce`), both plain value → value
/// functions with no timer, window, or `ShellModel` involved. The AppKit
/// wiring (`PreviewAnchorProbe` in PreviewAnchor.swift) is exercised by the
/// live `FLASHTEX_RENDER`-gated test in FollowCaretLiveTests.swift.
final class FollowCaretTests: XCTestCase {
    private static let epoch = Date(timeIntervalSince1970: 1_700_000_000)
    private static let target = FollowCaret.Target(page: 3, rect: CGRect(x: 100, y: 500, width: 40, height: 12))
    private static let visible = CGRect(x: 0, y: 400, width: 600, height: 300) // page 3's target sits inside this rect
    // Instance shims so the test bodies below can say `epoch`/`target`/`visible` plainly.
    private var epoch: Date { Self.epoch }
    private var target: FollowCaret.Target { Self.target }
    private var visible: CGRect { Self.visible }

    /// `target` defaults to the standard page-3 target; pass `target: nil`
    /// explicitly (as the no-mapping test does) to mean "no mapping", not
    /// "use the default" — there is no `??` fallback here to confuse the two.
    private func input(target: FollowCaret.Target? = FollowCaretTests.target, visibleRect: CGRect?, preferenceEnabled: Bool = true,
                       lastUserScrollAt: Date? = nil, lastUserScrollTarget: FollowCaret.Target? = nil,
                       isUserDragging: Bool = false, now: Date = FollowCaretTests.epoch) -> FollowCaret.Input {
        .init(preferenceEnabled: preferenceEnabled, target: target, visibleRect: visibleRect,
             now: now, lastUserScrollAt: lastUserScrollAt, lastUserScrollTarget: lastUserScrollTarget,
             isUserDragging: isUserDragging)
    }

    // MARK: - decide

    func testDisabledByPreferenceIsAlwaysANoOpEvenWhenEverythingElseWouldScroll() {
        let outside = CGRect(x: 5000, y: 5000, width: 10, height: 10)
        let action = FollowCaret.decide(input(visibleRect: outside, preferenceEnabled: false))
        XCTAssertEqual(action, .none(.disabledByPreference))
    }

    func testNoMappingIsAQuietNoOpRegardlessOfGeometry() {
        // Preamble/comment/uncompiled region: ShellModel.followCaretTargetV1/V2
        // return nil; decide must not scroll just because a visible rect exists.
        let action = FollowCaret.decide(input(target: nil, visibleRect: CGRect(x: 0, y: 0, width: 10, height: 10)))
        XCTAssertEqual(action, .none(.noMapping))
    }

    func testAlreadyVisibleWithMarginIsANoOp() {
        // Target is well inside `visible`.
        XCTAssertEqual(FollowCaret.decide(input(visibleRect: visible)), .none(.alreadyVisible))

        // Target sits just outside the raw visible rect but within `margin` (24pt default) of it.
        let edge = FollowCaret.Target(page: 3, rect: CGRect(x: 100, y: visible.minY - 10, width: 40, height: 12))
        XCTAssertEqual(FollowCaret.decide(input(target: edge, visibleRect: visible)), .none(.alreadyVisible))
    }

    func testOutsideVisibleRectScrollsToTheTargetRect() {
        let below = CGRect(x: 0, y: 0, width: 600, height: 100) // page 3's target (y 500) is far below this
        XCTAssertEqual(FollowCaret.decide(input(visibleRect: below)), .scroll(to: target.rect))
    }

    func testNoVisibleRectIsANoOp() {
        XCTAssertEqual(FollowCaret.decide(input(visibleRect: nil)), .none(.noVisibleRect))
    }

    func testUserIsDraggingWinsOverAnOtherwiseDueScroll() {
        let below = CGRect(x: 0, y: 0, width: 600, height: 100)
        let action = FollowCaret.decide(input(visibleRect: below, isUserDragging: true))
        XCTAssertEqual(action, .none(.userIsDragging))
    }

    func testYieldsToARecentManualScrollOnTheSameTarget() {
        let below = CGRect(x: 0, y: 0, width: 600, height: 100)
        let scrolledAt = epoch.addingTimeInterval(-1) // 1s ago, inside the 3s yield window
        let action = FollowCaret.decide(input(visibleRect: below, lastUserScrollAt: scrolledAt, lastUserScrollTarget: target))
        XCTAssertEqual(action, .none(.yieldingToUser))
    }

    func testYieldWindowExpiresAfterTheConfiguredDuration() {
        let below = CGRect(x: 0, y: 0, width: 600, height: 100)
        let scrolledAt = epoch.addingTimeInterval(-3.5) // just past the 3s window
        let action = FollowCaret.decide(input(visibleRect: below, lastUserScrollAt: scrolledAt, lastUserScrollTarget: target))
        XCTAssertEqual(action, .scroll(to: target.rect))
    }

    func testCaretMovingToADifferentPageLineReArmsInsideTheYieldWindow() {
        // The user scrolled 0.5s ago while the caret was on a different item
        // (a different page/line): the caret moving since means the yield no
        // longer applies, even though the 3s window has not elapsed.
        let below = CGRect(x: 0, y: 0, width: 600, height: 100)
        let scrolledAt = epoch.addingTimeInterval(-0.5)
        let previousTarget = FollowCaret.Target(page: 1, rect: CGRect(x: 0, y: 0, width: 10, height: 10))
        let action = FollowCaret.decide(input(visibleRect: below, lastUserScrollAt: scrolledAt, lastUserScrollTarget: previousTarget))
        XCTAssertEqual(action, .scroll(to: target.rect))
    }

    // MARK: - debounce coalescing

    func testATypingBurstCoalescesToOneFireAfterTheLastEvent() {
        let t0 = epoch
        let events: [(at: Date, value: Int)] = [
            (t0, 1), (t0.addingTimeInterval(0.05), 2), (t0.addingTimeInterval(0.1), 3), (t0.addingTimeInterval(0.15), 4),
        ]
        let fires = FollowCaret.debounce(events, window: 0.25)
        XCTAssertEqual(fires.count, 1, "one debounce fire for the whole burst, not one per keystroke")
        XCTAssertEqual(fires[0].value, 4, "the last caret position in the burst wins")
        XCTAssertEqual(fires[0].at.timeIntervalSince(t0.addingTimeInterval(0.15 + 0.25)), 0, accuracy: 1e-9)
    }

    func testEventsFartherApartThanTheWindowFireSeparately() {
        let t0 = epoch
        let events: [(at: Date, value: Int)] = [(t0, 1), (t0.addingTimeInterval(1.0), 2)]
        let fires = FollowCaret.debounce(events, window: 0.25)
        XCTAssertEqual(fires.map(\.value), [1, 2])
        XCTAssertEqual(fires[0].at.timeIntervalSince(t0.addingTimeInterval(0.25)), 0, accuracy: 1e-9)
        XCTAssertEqual(fires[1].at.timeIntervalSince(t0.addingTimeInterval(1.25)), 0, accuracy: 1e-9)
    }

    func testEmptyEventsFireNothing() {
        XCTAssertTrue(FollowCaret.debounce([(at: Date, value: Int)](), window: 0.25).isEmpty)
    }
}
