import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Reduce-motion path (lane mac-editor-a11y-3): `ReduceMotion` as the one
/// source of truth (injected flag, never the system setting), the
/// `PreviewAnchorProbe` settle window under both paths, and the
/// `withAnimation` wrapper. Hosted tests reuse `PreviewAnchoringTests`'
/// off-screen, never-key harness.
@MainActor
final class ReduceMotionTests: XCTestCase {
    typealias A = PreviewAnchoringTests

    override func tearDown() { ReduceMotion.override = nil }

    func testOverrideDrivesIsEnabledAndTheAnimationWrapper() {
        ReduceMotion.override = true
        XCTAssertTrue(ReduceMotion.isEnabled)
        var ran = 0
        let value = ReduceMotion.animate { ran += 1; return 7 }
        XCTAssertEqual(ran, 1, "the body runs immediately, without an animation transaction")
        XCTAssertEqual(value, 7)
        ReduceMotion.override = false
        XCTAssertFalse(ReduceMotion.isEnabled)
        XCTAssertEqual(ReduceMotion.animate(.easeOut) { ran += 1; return "x" }, "x")
        XCTAssertEqual(ran, 2)
        ReduceMotion.override = nil
        XCTAssertEqual(ReduceMotion.isEnabled, NSWorkspace.shared.accessibilityDisplayShouldReduceMotion, "nil reads the system setting")
        // A fresh probe reads the same source.
        ReduceMotion.override = true
        XCTAssertTrue(PreviewAnchorProbe().reduceMotion())
    }

    private func settle(_ seconds: TimeInterval) async throws { try await Task.sleep(nanoseconds: UInt64(seconds * 1e9)) }

    private func hostedAtPageTwo() async throws -> (A.Hosted<A.Column>, NSScrollView, PreviewAnchorProbe, CGFloat) {
        let hosted = A.Hosted(A.Column(pages: A.pages(3)), width: 500, height: 400)
        try await settle(0.4)
        let scroll = try XCTUnwrap(A.find(NSScrollView.self, in: hosted.hosting))
        let probe = try XCTUnwrap(A.find(PreviewAnchorProbe.self, in: hosted.hosting))
        let wide = try XCTUnwrap(probe.layout)
        let target = try XCTUnwrap(wide.frame(of: 2)).minY + 0.3 * 792 * wide.scale
        A.scroll(scroll, toTop: target)
        try await settle(0.1)
        XCTAssertEqual(probe.anchor?.page, 2)
        XCTAssertEqual(probe.anchor?.fraction ?? 0, 0.3, accuracy: 1e-3)
        return (hosted, scroll, probe, target)
    }

    /// Under reduce motion the anchor still holds across a pane resize: the
    /// corrections that keep the content still are synchronous (inside the
    /// layout pass), and the end-of-window step only closes the window.
    func testAnchorStillHoldsAcrossResizeUnderReduceMotion() async throws {
        let (hosted, scroll, probe, _) = try await hostedAtPageTwo()
        probe.reduceMotion = { true }
        hosted.resize(width: 350, height: 400)
        try await settle(0.4)
        let narrow = try XCTUnwrap(probe.layout)
        let expected = try XCTUnwrap(narrow.frame(of: 2)).minY + 0.3 * 792 * narrow.scale
        XCTAssertEqual(A.visibleTop(scroll), expected, accuracy: 1)
        XCTAssertFalse(probe.corrections.isEmpty, "synchronous corrections still run")
        XCTAssertEqual(probe.anchor?.page, 2)
        XCTAssertEqual(probe.anchor?.fraction ?? 0, 0.3, accuracy: 1e-3)
        let events = probe.trace.map(\.event)
        XCTAssertTrue(events.contains { $0.hasPrefix("settled (reduce motion)") }, "\(events)")
        XCTAssertFalse(events.contains { $0.hasPrefix("settled x=") }, "the default settle step did not run: \(events)")
        XCTAssertEqual(probe.driftsLeftUncorrected, 0, "nothing drifted after the synchronous corrections")
        print("reduce-motion hosted resize: corrections \(probe.corrections.count), trace \(events.suffix(4))")
    }

    /// Drift that no notification reports (simulated by scrolling with bounds
    /// notifications off) inside the settle window: the default path scrolls
    /// back from the timer; reduce motion leaves the content where it is,
    /// counts the drift and re-captures the anchor from the drifted position.
    func testDeferredSettleStepScrollsOnlyWithoutReduceMotion() async throws {
        for reduced in [false, true] {
            let (_, scroll, probe, _) = try await hostedAtPageTwo()
            probe.reduceMotion = { reduced }
            let base = try XCTUnwrap(probe.layout)
            // A layout change the keeper hears directly (same pages, smaller
            // scale): the synchronous correction moves the top to page 2 @ 0.3
            // under the new scale.
            let smaller = PreviewPageLayout(pages: base.pages, scale: base.scale * 0.8)
            let target = try XCTUnwrap(smaller.frame(of: 2)).minY + 0.3 * 792 * smaller.scale
            let before = probe.corrections.count
            let settledBefore = probe.trace.filter { $0.event.hasPrefix("settled") }.count
            probe.layoutDidChange(to: smaller)
            XCTAssertEqual(probe.corrections.count, before + 1, "one synchronous correction")
            XCTAssertEqual(A.visibleTop(scroll), target, accuracy: 1)
            // Silent drift of 60 pt inside the window: clip bounds
            // notifications stay off until the window has closed, because
            // AppKit's later display/tile pass posts bounds changes of its own
            // and the observer would correct the drift synchronously in both
            // paths (measured). The timer's step reads the clip directly.
            scroll.contentView.postsBoundsChangedNotifications = false
            var origin = scroll.contentView.bounds.origin
            origin.y += (scroll.documentView?.isFlipped == true ? 60 : -60)
            scroll.contentView.setBoundsOrigin(origin) // no reflectScrolledClipView: no scroller re-tile
            // Inside the settle window by construction: the settle step is a
            // main-queue timer and nothing above yielded the main thread. Checked
            // on the trace, not the wall clock, which a loaded runner stretches
            // past 150 ms without changing the order.
            XCTAssertEqual(probe.trace.filter { $0.event.hasPrefix("settled") }.count, settledBefore,
                           "the drift happened before the settle step ran: \(probe.trace.map(\.event))")
            XCTAssertEqual(A.visibleTop(scroll), target + 60, accuracy: 1)
            try await settle(PreviewAnchorProbe.settleWindow + 0.15)
            scroll.contentView.postsBoundsChangedNotifications = true
            let events = probe.trace.map(\.event)
            if reduced {
                XCTAssertEqual(A.visibleTop(scroll), target + 60, accuracy: 1, "no scroll from the timer")
                XCTAssertEqual(probe.corrections.count, before + 1)
                XCTAssertEqual(probe.driftsLeftUncorrected, 1)
                XCTAssertTrue(events.contains { $0.hasPrefix("drift left (reduce motion)") }, "\(events)")
                let recaptured = try XCTUnwrap(probe.anchor)
                let fromDrift = try XCTUnwrap(PreviewAnchor.capture(visible: CGRect(x: 0, y: target + 60, width: scroll.contentView.bounds.width, height: scroll.contentView.bounds.height), layout: smaller))
                XCTAssertEqual(recaptured.page, fromDrift.page)
                XCTAssertEqual(recaptured.fraction, fromDrift.fraction, accuracy: 1e-3, "the anchor follows the content instead of moving it")
            } else {
                XCTAssertEqual(A.visibleTop(scroll), target, accuracy: 1, "the timer scrolled back")
                XCTAssertEqual(probe.corrections.count, before + 2)
                XCTAssertEqual(probe.driftsLeftUncorrected, 0)
                XCTAssertTrue(events.contains { $0.hasPrefix("settled x=") }, "\(events)")
                XCTAssertEqual(probe.anchor?.page, 2)
                XCTAssertEqual(probe.anchor?.fraction ?? 0, 0.3, accuracy: 1e-3)
            }
            print("reduce-motion=\(reduced) deferred step: top \(A.visibleTop(scroll)) target \(target), corrections \(probe.corrections.count - before), drifts \(probe.driftsLeftUncorrected)")
            // Back to the real layout so the hosted view is consistent when it goes away.
            probe.layoutDidChange(to: base)
            try await settle(PreviewAnchorProbe.settleWindow + 0.1)
        }
    }
}
