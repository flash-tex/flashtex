import XCTest
import SwiftUI
import FlashTeXProtocol
@testable import FlashTeXMac

/// "When you're editing make the pdf automatically move to where the changes
/// are happening" (the owner, after v0.1.2) — `CaretFollow.swift`.
///
/// Four levels, all deterministic (no sleeps except the SwiftUI settle the
/// hosted anchoring harness already uses):
/// 1. `CaretFollowDecisionTests`: the pure rule — already visible → never
///    scroll; off screen (or within `visibleMargin` of an edge) → the minimal
///    scroll plus breathing room; animation only for short hops.
/// 2. `CaretFollowControllerTests`: when a follow is considered — the debounce
///    (driven by an injected clock, measured rather than guessed), the
///    manual-scroll disarm and the preference.
/// 3. `CaretFollowTargetTests`: where the caret is, on the real v1 sample and
///    the real v2 display list, and the quiet nil for a caret that maps to
///    nothing.
/// 4. `CaretFollowHostedTests`: the whole thing inside a real `NSScrollView`.

// MARK: - 1. the pure decision

@MainActor
final class CaretFollowDecisionTests: XCTestCase {
    static let pages = (1...3).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }
    let layout = PreviewPageLayout(pages: CaretFollowDecisionTests.pages, scale: 0.5)
    /// A paragraph-sized target, in page points.
    func target(page: Int, top: CGFloat, height: CGFloat = 12) -> CaretFollow.Target {
        CaretFollow.Target(page: page, rect: CGRect(x: 72, y: top, width: 180, height: height))
    }
    var content: CGSize { layout.contentSize }
    func viewport(top: CGFloat, height: CGFloat = 400, width: CGFloat = 500) -> CGRect {
        CGRect(x: 0, y: top, width: width, height: height)
    }

    /// Document y of a target's top edge under this layout.
    func documentTop(_ t: CaretFollow.Target) -> CGFloat {
        (layout.frame(of: t.page)?.minY ?? 0) + t.rect.minY * layout.scale
    }

    func testATargetInTheMiddleOfTheViewportIsNeverScrolled() {
        // Page 1 at scale 0.5 starts at y = 24; the item 200 pt down the page
        // is at 124 in the document, well inside a viewport showing 0...400.
        let t = target(page: 1, top: 200)
        XCTAssertEqual(documentTop(t), 124)
        let decision = CaretFollow.decide(target: t, layout: layout, visible: viewport(top: 0),
                                          contentSize: content, reduceMotion: false)
        XCTAssertEqual(decision, .alreadyVisible)
        // Still true with the target just inside the comfort band at either
        // edge: the rule is "off screen, or nearly so", not "not centred".
        // (Page 2, so there is document above and below to sit in.)
        let t2 = target(page: 2, top: 200)
        let height = 12 * layout.scale
        for top in [documentTop(t2) - CaretFollow.visibleMargin - 1,
                    documentTop(t2) + height + CaretFollow.visibleMargin - 400 + 1] {
            XCTAssertEqual(CaretFollow.decide(target: t2, layout: layout, visible: viewport(top: top),
                                              contentSize: content, reduceMotion: false),
                           .alreadyVisible, "top \(top)")
        }
    }

    func testATargetInsideTheEdgeMarginCountsAsOffScreenAndScrollsWithBreathingRoom() throws {
        let t = target(page: 2, top: 200)
        let docTop = documentTop(t), height = 12 * layout.scale
        // Viewport whose bottom edge cuts one point into the margin below the
        // target: still on screen, but "nearly off", so it is revealed.
        let visible = viewport(top: docTop + height + CaretFollow.visibleMargin - 400 - 1)
        guard case .scroll(let to, let animated) = CaretFollow.decide(target: t, layout: layout, visible: visible,
                                                                     contentSize: content, reduceMotion: false) else {
            return XCTFail("a target inside the bottom margin must be revealed")
        }
        XCTAssertTrue(animated, "a hop of a few points is animated")
        // Revealed from below: the target's bottom edge plus the reveal inset,
        // which is much more than the margin that triggered it — that gap is
        // what stops the next few characters from scrolling again.
        let room = CaretFollow.revealInset(viewportHeight: 400, targetHeight: height)
        XCTAssertGreaterThan(room, CaretFollow.visibleMargin)
        XCTAssertEqual(to.y, docTop + height + room - 400, accuracy: 0.001)
        XCTAssertGreaterThan(to.y, visible.minY, "the content moved up, the target came in from the bottom")
        // And the result is stable: deciding again from there does nothing, and
        // so does a target a few points further along the same line.
        XCTAssertEqual(CaretFollow.decide(target: t, layout: layout, visible: viewport(top: to.y),
                                          contentSize: content, reduceMotion: false), .alreadyVisible)
        XCTAssertEqual(CaretFollow.decide(target: target(page: 2, top: 210), layout: layout, visible: viewport(top: to.y),
                                          contentSize: content, reduceMotion: false), .alreadyVisible)
    }

    func testATargetAboveTheViewportIsRevealedFromAboveAndAFarOneIsNotAnimated() throws {
        let high = target(page: 1, top: 100)
        let visible = viewport(top: 600) // page 2 territory
        guard case .scroll(let up, let animatedUp) = CaretFollow.decide(target: high, layout: layout, visible: visible,
                                                                       contentSize: content, reduceMotion: false) else {
            return XCTFail("a target above the viewport must be revealed")
        }
        let room = CaretFollow.revealInset(viewportHeight: 400, targetHeight: 12 * layout.scale)
        XCTAssertEqual(up.y, max(0, documentTop(high) - room), accuracy: 0.001)
        XCTAssertTrue(animatedUp, "600 → 74 pt is well under two viewports: animated")

        // The same target from the last page is a jump of more than two
        // viewports: instant, because animating that far only blurs.
        guard case .scroll(_, let animatedFar) = CaretFollow.decide(target: high, layout: layout, visible: viewport(top: 1500),
                                                                    contentSize: content, reduceMotion: false) else {
            return XCTFail("expected a scroll")
        }
        XCTAssertFalse(animatedFar)

        // A short hop up is animated; reduce motion turns every animation off.
        let near = target(page: 2, top: 100)
        guard case .scroll(_, let animatedNear) = CaretFollow.decide(target: near, layout: layout,
                                                                     visible: viewport(top: documentTop(near) + 60),
                                                                     contentSize: content, reduceMotion: false) else {
            return XCTFail("expected a scroll")
        }
        XCTAssertTrue(animatedNear)
        guard case .scroll(_, let reduced) = CaretFollow.decide(target: near, layout: layout,
                                                                visible: viewport(top: documentTop(near) + 60),
                                                                contentSize: content, reduceMotion: true) else {
            return XCTFail("expected a scroll")
        }
        XCTAssertFalse(reduced, "reduce motion: the preview jumps, it does not animate")
    }

    func testItNeverScrollsPastTheDocumentAndSaysAlreadyVisibleWhenItCannotMove() {
        // The last line of the last page, with the viewport already at the end:
        // clamping leaves nothing to do, so the pane is told to stay put.
        let last = target(page: 3, top: 780)
        let end = max(0, content.height - 400)
        XCTAssertEqual(CaretFollow.decide(target: last, layout: layout, visible: viewport(top: end),
                                          contentSize: content, reduceMotion: false), .alreadyVisible)
        // From the top of the document the same target scrolls, but only to the end.
        guard case .scroll(let to, _) = CaretFollow.decide(target: last, layout: layout, visible: viewport(top: 0),
                                                           contentSize: content, reduceMotion: false) else {
            return XCTFail("expected a scroll to the end")
        }
        XCTAssertEqual(to.y, end, accuracy: 0.001)
    }

    func testAPageTheLayoutNoLongerHasIsNoPageAndAnEmptyViewportIsLeftAlone() {
        XCTAssertEqual(CaretFollow.decide(target: target(page: 9, top: 10), layout: layout, visible: viewport(top: 0),
                                          contentSize: content, reduceMotion: false), .noPage)
        XCTAssertEqual(CaretFollow.decide(target: target(page: 1, top: 10), layout: layout,
                                          visible: CGRect(x: 0, y: 0, width: 0, height: 0),
                                          contentSize: content, reduceMotion: false), .alreadyVisible)
    }

    func testHorizontalOnlyMovesWhenTheDocumentIsWiderThanThePane() throws {
        let t = target(page: 1, top: 200)
        // Narrow pane, zoomed-in column: the document is wider than the viewport.
        let zoomed = PreviewPageLayout(pages: Self.pages, scale: 2)
        let wide = CGSize(width: zoomed.contentSize.width, height: zoomed.contentSize.height)
        let visible = CGRect(x: 800, y: 0, width: 400, height: 400)
        guard case .scroll(let to, _) = CaretFollow.decide(target: t, layout: zoomed, visible: visible,
                                                           contentSize: wide, reduceMotion: false) else {
            return XCTFail("the target is off to the left: it must come back")
        }
        XCTAssertLessThan(to.x, visible.minX)
        // Same target, a pane wider than the document: x is never touched.
        guard case .scroll(let narrow, _) = CaretFollow.decide(target: t, layout: layout, visible: viewport(top: 2000),
                                                               contentSize: content, reduceMotion: false) else {
            return XCTFail("expected a vertical scroll")
        }
        XCTAssertEqual(narrow.x, 0)
    }
}

// MARK: - 2. when a follow is considered

/// Deterministic replacement for `DispatchQueue.main.asyncAfter`: the test
/// owns the clock, so the debounce is measured, not slept through.
@MainActor
final class FollowClock {
    private(set) var now: TimeInterval = 0
    private(set) var scheduled: [(at: TimeInterval, delay: TimeInterval, item: DispatchWorkItem)] = []
    private(set) var delays: [TimeInterval] = []

    func install(on controller: CaretFollowController) {
        controller.schedule = { [weak self] delay, item in
            guard let self else { return }
            self.delays.append(delay)
            self.scheduled.append((self.now + delay, delay, item))
        }
    }

    var live: [(at: TimeInterval, delay: TimeInterval, item: DispatchWorkItem)] { scheduled.filter { !$0.item.isCancelled } }

    /// Runs every live item due within `seconds`, in time order.
    func advance(_ seconds: TimeInterval) {
        let end = now + seconds
        while true {
            scheduled.removeAll { $0.item.isCancelled }
            guard let next = scheduled.enumerated().filter({ $0.element.at <= end + 1e-9 }).min(by: { $0.element.at < $1.element.at }) else { break }
            scheduled.remove(at: next.offset)
            now = max(now, next.element.at)
            next.element.item.perform()
        }
        now = end
    }
}

@MainActor
final class CaretFollowControllerTests: XCTestCase {
    let target = CaretFollow.Target(page: 1, rect: CGRect(x: 10, y: 20, width: 100, height: 12))
    var clock = FollowClock()
    var controller = CaretFollowController()

    override func setUp() {
        super.setUp()
        CaretFollow.enabledOverride = true // never read the developer's own preference
        CaretFollow.debounceOverride = nil
        clock = FollowClock()
        controller = CaretFollowController()
        controller.target = { [target] in target }
        clock.install(on: controller)
    }

    override func tearDown() {
        CaretFollow.enabledOverride = nil
        CaretFollow.debounceOverride = nil
        super.tearDown()
    }

    func testOneBurstOfTypingProducesExactlyOneFollowAfterTheTypistStops() {
        // 12 keystrokes 80 ms apart, each followed 5 ms later by the recompiled
        // preview landing (the compiler answers in ~1–9 ms for this size).
        for _ in 0..<12 {
            controller.note(.edit)
            clock.advance(0.005)
            controller.note(.recompile)
            clock.advance(0.075)
        }
        XCTAssertEqual(controller.follows, 0, "nothing moves while the keys are still going down")
        XCTAssertEqual(clock.live.count, 1, "one live timer: every keystroke replaced the previous one")
        clock.advance(CaretFollow.debounceInterval)
        XCTAssertEqual(controller.follows, 1)
        XCTAssertEqual(controller.request?.token, 1)
        XCTAssertEqual(controller.request?.target, target)
        XCTAssertEqual(controller.lastReason, .recompile)
        XCTAssertFalse(controller.hasPendingFollow)
        XCTAssertEqual(Set(clock.delays), [CaretFollow.debounceInterval])
    }

    /// The number itself: replayed typing cadences against candidate debounces.
    /// Printed as evidence; the assertions are the properties the value was
    /// chosen for.
    func testDebounceChoiceMeasuredAgainstRealTypingCadences() {
        let cadences: [(name: String, gaps: [TimeInterval])] = [
            ("fast burst, 60 ms/key", Array(repeating: 0.06, count: 25)),
            ("typical, 110 ms/key", Array(repeating: 0.11, count: 20)),
            ("hunt and peck, 250 ms/key", Array(repeating: 0.25, count: 12)),
            ("two sentences with pauses", [0.09, 0.08, 0.1, 0.11, 0.09, 0.6, 0.09, 0.1, 0.08, 0.09, 0.7, 0.1, 0.09]),
        ]
        let candidates: [TimeInterval] = [0, 0.05, 0.1, CaretFollow.defaultDebounce, 0.25, 0.4]
        var followsFor: [String: [TimeInterval: Int]] = [:]
        var settleFor: [TimeInterval: TimeInterval] = [:]
        print("caret-follow debounce (follows per replayed cadence; fewer is calmer, and the last column is how long after the last keystroke the preview moves):")
        for cadence in cadences {
            var row = "  \(cadence.name.padding(toLength: 26, withPad: " ", startingAt: 0))"
            for candidate in candidates {
                CaretFollow.debounceOverride = candidate
                let clock = FollowClock()
                let controller = CaretFollowController()
                controller.target = { [target] in target }
                clock.install(on: controller)
                for gap in cadence.gaps {
                    controller.note(.edit)
                    clock.advance(0.005)
                    controller.note(.recompile)
                    clock.advance(max(0, gap - 0.005))
                }
                let duringTyping = controller.follows
                clock.advance(candidate + 0.001)
                followsFor[cadence.name, default: [:]][candidate] = controller.follows
                settleFor[candidate] = candidate
                row += String(format: " %@=%d(+%d)", String(format: "%.0fms", candidate * 1000), duringTyping, controller.follows - duringTyping)
            }
            print(row)
        }
        CaretFollow.debounceOverride = nil

        let chosen = CaretFollow.defaultDebounce
        XCTAssertEqual(followsFor["fast burst, 60 ms/key"]?[chosen], 1, "a burst of fast typing is one follow, not 25")
        XCTAssertEqual(followsFor["typical, 110 ms/key"]?[chosen], 1)
        XCTAssertEqual(followsFor["two sentences with pauses"]?[chosen], 3, "one per natural pause, so the preview keeps up with a paragraph")
        // Why not shorter: 50 ms follows almost every keystroke of a fast burst.
        XCTAssertGreaterThan(followsFor["fast burst, 60 ms/key"]?[0.05] ?? 0, 10)
        XCTAssertGreaterThan(followsFor["typical, 110 ms/key"]?[0.1] ?? 0, 10)
        // Why not longer: the move must land while stopping still feels like
        // one action (≤ 250 ms after the last keystroke).
        XCTAssertLessThanOrEqual(chosen, 0.25)
        XCTAssertGreaterThan(0.4, 0.25)
        print(String(format: "caret-follow debounce chosen: %.0f ms (settles %.0f ms after the last keystroke)", chosen * 1000, chosen * 1000))
    }

    func testEachTriggerReplacesThePendingFollowRatherThanQueueingAnother() {
        controller.note(.edit)
        let first = try? XCTUnwrap(controller.pending)
        controller.note(.caretMove)
        controller.note(.recompile)
        XCTAssertEqual(first?.isCancelled, true, "the first timer was cancelled, not left to fire")
        XCTAssertEqual(clock.live.count, 1)
        clock.advance(1)
        XCTAssertEqual(controller.follows, 1)
    }

    func testAManualPreviewScrollStopsFollowingUntilTheNextEdit() {
        controller.note(.edit)
        clock.advance(1)
        XCTAssertEqual(controller.follows, 1)

        controller.userDidScrollPreview() // wheel / trackpad / scroller drag
        XCTAssertFalse(controller.isArmed)
        XCTAssertFalse(controller.hasPendingFollow, "a scheduled follow is dropped too")
        controller.note(.caretMove)
        controller.note(.recompile)
        clock.advance(1)
        XCTAssertEqual(controller.follows, 1, "moving the caret or recompiling does not fight the reader")
        XCTAssertEqual(controller.skippedDisarmed, 2)

        controller.note(.edit) // typing again is the re-arm
        XCTAssertTrue(controller.isArmed)
        clock.advance(1)
        XCTAssertEqual(controller.follows, 2)
        XCTAssertEqual(controller.lastReason, .edit)

        // ⌘⇧J is the other re-arm, and it does not wait for the debounce.
        controller.userDidScrollPreview()
        controller.note(.explicit)
        XCTAssertEqual(controller.follows, 3)
        XCTAssertEqual(controller.lastReason, .explicit)
        XCTAssertTrue(clock.live.isEmpty, "explicit reveal is immediate, not scheduled")
    }

    func testTheCaretMappingToNothingIsAQuietNoOp() {
        controller.target = { nil } // in a comment, in the preamble, not yet compiled
        controller.note(.edit)
        clock.advance(1)
        XCTAssertNil(controller.request)
        XCTAssertEqual(controller.follows, 0)
        XCTAssertEqual(controller.skippedWithoutTarget, 1)
        XCTAssertTrue(controller.isArmed, "nothing to follow is not a reason to stop following")
    }

    func testThePreferenceOffMeansNothingIsEvenScheduled() {
        CaretFollow.enabledOverride = false
        controller.note(.edit)
        XCTAssertTrue(clock.scheduled.isEmpty)
        clock.advance(1)
        XCTAssertNil(controller.request)
        XCTAssertEqual(controller.follows, 0)
        // Turned back on, the next edit follows again.
        CaretFollow.enabledOverride = true
        controller.note(.edit)
        clock.advance(1)
        XCTAssertEqual(controller.follows, 1)
    }

    func testAZeroDebounceFiresSynchronously() {
        CaretFollow.debounceOverride = 0 // FLASHTEX_CARET_FOLLOW_DEBOUNCE_MS=0
        controller.note(.edit)
        XCTAssertEqual(controller.follows, 1)
        XCTAssertTrue(clock.scheduled.isEmpty)
    }

    /// A caret move re-arms after a manual scroll only when it lands on a
    /// different source line or a different preview page than where the
    /// scroll happened — not on a move that merely stays on the same line
    /// (a different column, an arrow key, clicking another word on it).
    /// Table-driven over `noteCaretMove()`, the entry point `ShellModel`
    /// actually calls; a bare `note(.caretMove)` is unaffected (still never
    /// re-arms, see `testAManualPreviewScrollStopsFollowingUntilTheNextEdit`).
    func testCaretMoveReArmsOnlyAcrossALineOrPageBoundary() {
        var line = 1
        var page: Int? = 1
        controller.currentLine = { line }
        controller.target = { page.map { CaretFollow.Target(page: $0, rect: .zero) } }

        let cases: [(fromLine: Int, fromPage: Int?, toLine: Int, toPage: Int?, reArms: Bool, why: String)] = [
            (5, 1, 5, 1, false, "same line, same page: reading in place"),
            (5, 1, 5, 2, true, "same line, different page"),
            (5, 1, 6, 1, true, "different line, same page"),
            (5, 1, 6, 2, true, "different line and page"),
            (5, nil, 5, nil, false, "same line, still mapping to nothing"),
            (5, nil, 5, 1, true, "same line, now maps to a page"),
        ]

        for c in cases {
            controller.note(.edit) // start each case armed, with a clean slate
            clock.advance(1)

            line = c.fromLine; page = c.fromPage
            controller.userDidScrollPreview() // wheel / trackpad / scroller drag
            XCTAssertFalse(controller.isArmed, c.why)

            line = c.toLine; page = c.toPage
            let skippedBefore = controller.skippedDisarmed
            controller.noteCaretMove()
            XCTAssertEqual(controller.isArmed, c.reArms, c.why)
            XCTAssertEqual(controller.skippedDisarmed, skippedBefore + (c.reArms ? 0 : 1), c.why)
        }
    }

    /// ⌘⇧J is an instruction, not a hint: it scrolls at once even with
    /// "Preview follows the caret" off, and even while a manual scroll has
    /// disarmed following.
    func testExplicitRevealScrollsAtOnceRegardlessOfThePreference() {
        CaretFollow.enabledOverride = false
        controller.userDidScrollPreview()
        XCTAssertFalse(controller.isArmed)

        controller.note(.explicit)
        XCTAssertEqual(controller.follows, 1, "⌘⇧J scrolls even though the preference is off")
        XCTAssertEqual(controller.lastReason, .explicit)
        XCTAssertTrue(controller.isArmed, "⌘⇧J also re-arms, for when the preference comes back on")
        XCTAssertTrue(clock.live.isEmpty, "immediate, not scheduled")
    }

    func testThePreferenceIsOnByDefaultAndPersists() throws {
        CaretFollow.enabledOverride = nil
        let defaults = UserDefaults(suiteName: "CaretFollowTests.\(UUID().uuidString)")!
        let prefs = EditorPreferences(defaults: defaults)
        XCTAssertTrue(prefs.followCaretInPreview, "the owner asked for the behaviour: it ships on")
        XCTAssertTrue(EditorPreferences.defaultSnapshot.followCaretInPreview)
        XCTAssertTrue(prefs.lastLoadRepairs.contains(.followCaretInPreview), "absent value repaired to the default and written back")
        prefs.followCaretInPreview = false
        XCTAssertEqual(defaults.object(forKey: EditorPreferences.Key.followCaretInPreview.storageKey) as? Bool, false)
        XCTAssertFalse(EditorPreferences(defaults: defaults).followCaretInPreview, "a fresh read sees the stored value")
        XCTAssertFalse(prefs.snapshot.followCaretInPreview)
        prefs.resetToDefaults()
        XCTAssertTrue(prefs.followCaretInPreview)
    }
}

// MARK: - 3. where the caret is

@MainActor
final class CaretFollowTargetTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")

    override func setUp() { super.setUp(); CaretFollow.enabledOverride = true; CaretFollow.debounceOverride = 0 }
    override func tearDown() { CaretFollow.enabledOverride = nil; CaretFollow.debounceOverride = nil; super.tearDown() }

    /// v1 (`compile_result`): the sample's "naïve" is item 2 of page 1, and
    /// the target is that item's own rectangle in page points.
    func testV1TargetIsTheItemUnderTheCaretInPagePoints() throws {
        let model = ShellModel()
        model.previewV2 = false
        model.loadFixtures(request: nil, result: CaretSyncTests.resultURL)
        XCTAssertNil(model.loadError, model.loadError ?? "")
        let ns = (model.activeText as NSString).range(of: "naïve")
        model.caretUTF16 = ns.location + 3
        XCTAssertEqual(model.caretItems, [1: [2]])
        let target = try XCTUnwrap(model.caretPreviewTarget())
        XCTAssertEqual(target.page, 1)
        let result = try XCTUnwrap(model.result)
        let expected = try XCTUnwrap(PreviewHitRects.compute(page: result.pages[0], scale: 1, rulesNegotiated: false)
            .first { $0.index == 2 }?.rect)
        XCTAssertEqual(target.rect, expected)
        XCTAssertGreaterThan(target.rect.width, 0)
        XCTAssertGreaterThan(target.rect.height, 0)

        // Page 2: "Résumé" — the page number comes from the item, not the caret.
        model.caretUTF16 = (model.activeText as NSString).range(of: "Résumé").location
        XCTAssertEqual(model.caretPreviewTarget()?.page, 2)

        // A caret in the whitespace between items, and one past the buffer,
        // map to nothing at all — the follower then does nothing, quietly.
        model.caretUTF16 = 62
        XCTAssertEqual(model.caretItems, [:])
        XCTAssertNil(model.caretPreviewTarget())
        model.caretUTF16 = (model.activeText as NSString).length + 10
        XCTAssertNil(model.caretPreviewTarget())
    }

    /// An edit on this model schedules and fires one follow end to end (zero
    /// debounce), and the request carries the same target. Also end to end:
    /// the two re-arm rules, on the real fixture text ("naïve" is one line
    /// into section 1; "Résumé" is a different line *and*, after the
    /// `\newpage`, a different page).
    func testAnEditOnTheModelProducesTheFollowRequestThePaneWillSee() throws {
        let model = ShellModel()
        model.previewV2 = false
        model.loadFixtures(request: nil, result: CaretSyncTests.resultURL)
        let naive = (model.activeText as NSString).range(of: "naïve").location
        model.caretUTF16 = naive + 3
        let request = try XCTUnwrap(model.caretFollow.request, "moving the caret is enough to aim the preview")
        XCTAssertEqual(request.target, model.caretPreviewTarget())
        XCTAssertEqual(request.reason, .caretMove)

        // A manual preview scroll stops it; a caret move that stays on the
        // same source line does not wake it back up.
        model.caretFollow.userDidScrollPreview()
        model.caretUTF16 = naive + 1
        XCTAssertEqual(model.caretFollow.request?.token, request.token, "same line: no new request while the reader is reading")

        // Moving on to "Résumé" — a different line and page — re-arms on its
        // own and produces a new .caretMove request, without any edit.
        model.caretUTF16 = (model.activeText as NSString).range(of: "Résumé").location
        let moved = try XCTUnwrap(model.caretFollow.request)
        XCTAssertGreaterThan(moved.token, request.token, "a different line/page re-arms on its own")
        XCTAssertEqual(moved.reason, .caretMove)
        XCTAssertEqual(moved.target, model.caretPreviewTarget())

        // Scrolled away again; this time an edit is what brings it back.
        model.caretFollow.userDidScrollPreview()
        model.updateActiveText(model.activeText + " more")
        let after = try XCTUnwrap(model.caretFollow.request)
        XCTAssertGreaterThan(after.token, moved.token)
        XCTAssertEqual(after.reason, .edit)
    }

    /// v2 (display-list-v2), on the real producer's list: the caret inside
    /// `\frac{x}{y}` targets the enclosing formula box, converted from ticks
    /// to page points; a caret in the preamble targets nothing.
    func testV2TargetIsTheFormulaBoxUnderTheCaret() throws {
        let store = V2FontStore(directories: [Self.fontsDir.path])
        guard store.fonts.contains(where: { $0.url.lastPathComponent == "latinmodern-math.otf" }) else { throw XCTSkip("bundled math font missing") }
        let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-math-nav.tex"), encoding: .utf8)
        let model = ShellModel()
        model.replaceProject(entryText: tex)
        model.previewV2 = true
        let done = expectation(description: "load")
        model.loadDisplayListV2(url: Self.fixtures.appendingPathComponent("display-list-v2-math-nav.json")) { done.fulfill() }
        wait(for: [done], timeout: 20)
        guard case .loaded? = model.displayListV2 else { return XCTFail("expected a prepared frame") }

        model.caretUTF16 = (tex as NSString).range(of: "\\frac{x}{y}").location + 6 // inside `x`
        let target = try XCTUnwrap(model.caretPreviewTarget())
        let box = try XCTUnwrap(model.caretFormulaBoxes().first)
        XCTAssertEqual(target.page, box.page)
        XCTAssertEqual(target.rect.minX, RenderingV2.points(box.box.bounds.x), accuracy: 1e-9)
        XCTAssertEqual(target.rect.minY, RenderingV2.points(box.box.bounds.top), accuracy: 1e-9)
        XCTAssertEqual(target.rect.width, RenderingV2.points(box.box.bounds.width), accuracy: 1e-9)
        XCTAssertEqual(target.rect.height, RenderingV2.points(box.box.bounds.height), accuracy: 1e-9)
        XCTAssertGreaterThan(target.rect.height, 0)

        // Plain text keeps a target too (the exact caret or the cluster).
        model.caretUTF16 = (tex as NSString).range(of: "Inline").location + 2
        XCTAssertTrue(model.caretFormulaBoxes().isEmpty)
        let text = try XCTUnwrap(model.caretPreviewTarget())
        XCTAssertEqual(text.page, 1)
        XCTAssertGreaterThan(text.rect.height, 0)

        // The preamble (byte 0 is `\documentclass`) is in no item: no target,
        // no note, no scroll.
        model.caretUTF16 = 0
        XCTAssertNil(model.caretPreviewTarget())
    }

    /// A historical snapshot is never followed onto (the caret belongs to the
    /// live buffer, the preview to an older revision).
    func testNoTargetWhileAHistoricalPreviewIsShown() throws {
        let model = ShellModel()
        model.previewV2 = false
        model.loadFixtures(request: nil, result: CaretSyncTests.resultURL)
        model.caretUTF16 = (model.activeText as NSString).range(of: "naïve").location + 3
        XCTAssertNotNil(model.caretPreviewTarget())
        model.historicalPreview = HistoricalDisplay(shownEditorRevision: 1, compilingEditorRevision: 2,
                                                    shownCompileRevision: 1, currentCompileRevision: 2,
                                                    requestID: "req-1", resultID: "res-1")
        XCTAssertNil(model.caretPreviewTarget())
        XCTAssertEqual(model.caretItems, [:], "caret sync already refuses an older snapshot; following agrees")
    }
}

// MARK: - 4. the whole mechanism in a real scroll view

/// The page column the panes build, with the keeper wired for following.
/// Mirrors `PreviewAnchoringTests.Column` (same padding, spacing and fit rule).
private struct FollowColumn: View {
    let pages: [PreviewPageLayout.Page]
    var follow: CaretFollowController.Request?
    var onUserScroll: (() -> Void)?

    var body: some View {
        GeometryReader { geo in
            let scale = PreviewPageLayout.fitScale(paneWidth: geo.size.width, widestPt: pages.map(\.widthPt).max() ?? 612)
            let layout = PreviewPageLayout(pages: pages, scale: scale)
            ScrollView([.vertical, .horizontal]) {
                VStack(spacing: 24) {
                    ForEach(pages, id: \.number) { page in
                        Rectangle().fill(Color.white)
                            .frame(width: page.widthPt * scale, height: page.heightPt * scale)
                            .id(page.number)
                    }
                }
                .padding(24)
                .background(PreviewAnchorKeeper(layout: layout, follow: follow, onUserScroll: onUserScroll))
            }
        }
    }
}

@MainActor
final class CaretFollowHostedTests: XCTestCase {
    override func setUp() { super.setUp(); CaretFollow.enabledOverride = true }
    override func tearDown() { CaretFollow.enabledOverride = nil; super.tearDown() }

    private func settle(_ seconds: TimeInterval = 0.4) async throws { try await Task.sleep(nanoseconds: UInt64(seconds * 1e9)) }

    func testTheHostedPaneScrollsToAnOffScreenCaretAndLeavesAVisibleOneAlone() async throws {
        let pages = (1...3).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }
        let hosted = PreviewAnchoringTests.Hosted(FollowColumn(pages: pages), width: 500, height: 400)
        try await settle()
        let scroll = try XCTUnwrap(PreviewAnchoringTests.find(NSScrollView.self, in: hosted.hosting))
        let probe = try XCTUnwrap(PreviewAnchoringTests.find(PreviewAnchorProbe.self, in: hosted.hosting))
        probe.reduceMotion = { true } // no animation: the assertions are about where it lands
        let layout = try XCTUnwrap(probe.layout)
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), 0, accuracy: 1)

        // (1) The caret lands on page 3, far below the fold: the pane scrolls.
        let far = CaretFollow.Target(page: 3, rect: CGRect(x: 72, y: 300, width: 200, height: 12))
        hosted.hosting.rootView = FollowColumn(pages: pages, follow: .init(token: 1, target: far, reason: .edit))
        try await settle(0.3)
        // Revealed from below: the target's bottom edge, plus the reveal inset,
        // brought to the bottom of the viewport.
        let viewportHeight = scroll.contentView.bounds.height
        let expected = try XCTUnwrap(layout.frame(of: 3)).minY + (300 + 12) * layout.scale
            + CaretFollow.revealInset(viewportHeight: viewportHeight, targetHeight: 12 * layout.scale)
            - viewportHeight
        let top = PreviewAnchoringTests.visibleTop(scroll)
        print(String(format: "caret-follow hosted: page 3 @ 300 pt is off screen → scrolled 0 → %.1f pt (expected %.1f)", top, expected))
        XCTAssertEqual(top, expected, accuracy: 1.5)
        XCTAssertEqual(probe.followDecisions.count, 1)
        guard case .scroll = probe.followDecisions[0].decision else { return XCTFail("expected a scroll decision") }

        // (2) A caret a few lines further down the same page is already on
        // screen: the decision is `alreadyVisible` and nothing moves at all.
        let near = CaretFollow.Target(page: 3, rect: CGRect(x: 72, y: 360, width: 200, height: 12))
        hosted.hosting.rootView = FollowColumn(pages: pages, follow: .init(token: 2, target: near, reason: .edit))
        try await settle(0.3)
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), top, accuracy: 0.5, "a visible caret never yanks the preview")
        XCTAssertEqual(probe.followDecisions.count, 2)
        XCTAssertEqual(probe.followDecisions[1].decision, .alreadyVisible)

        // (3) Re-applying the same request (SwiftUI re-evaluates the body for
        // any reason) is not a second scroll.
        PreviewAnchoringTests.scroll(scroll, toTop: 0)
        try await settle(0.2)
        hosted.hosting.rootView = FollowColumn(pages: pages, follow: .init(token: 2, target: near, reason: .edit))
        try await settle(0.3)
        XCTAssertEqual(probe.followDecisions.count, 2, "acted on once per token")
        XCTAssertEqual(PreviewAnchoringTests.visibleTop(scroll), 0, accuracy: 1)
    }

    func testALiveScrollInThePaneReportsTheUserScrollThatStopsFollowing() async throws {
        let controller = CaretFollowController()
        controller.target = { CaretFollow.Target(page: 1, rect: CGRect(x: 0, y: 0, width: 10, height: 10)) }
        let pages = (1...2).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }
        let hosted = PreviewAnchoringTests.Hosted(FollowColumn(pages: pages, onUserScroll: { controller.userDidScrollPreview() }),
                                                  width: 500, height: 400)
        try await settle()
        let scroll = try XCTUnwrap(PreviewAnchoringTests.find(NSScrollView.self, in: hosted.hosting))
        XCTAssertTrue(controller.isArmed)
        // Exactly what AppKit posts when a wheel, trackpad or scroller-knob
        // scroll begins (a programmatic `scroll(to:)` posts nothing).
        PreviewAnchoringTests.scroll(scroll, toTop: 120)
        try await settle(0.1)
        XCTAssertTrue(controller.isArmed, "our own corrections and programmatic scrolls are not the reader")
        NotificationCenter.default.post(name: NSScrollView.willStartLiveScrollNotification, object: scroll)
        try await settle(0.1)
        XCTAssertFalse(controller.isArmed, "the reader took over")
    }
}
