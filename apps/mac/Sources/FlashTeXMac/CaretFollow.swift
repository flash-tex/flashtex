import AppKit
import Observation
import FlashTeXProtocol

/// "When you're editing make the pdf automatically move to where the changes
/// are happening" (the owner, after v0.1.2).
///
/// The mapping already existed as a manual command: `revealCaretInPreview()`
/// (⌘⇧J, the Navigate menu, the palette, the accessibility command) selects
/// the preview item under the caret. This is the automatic half of it, and
/// the whole design is about *not* moving the preview more than necessary:
///
/// - **When.** A follow is considered after a text edit, after the recompiled
///   preview lands, and after a caret move; never per keystroke. Every trigger
///   restarts one debounce timer (`CaretFollow.debounceInterval`), so a burst
///   of typing produces exactly one follow, after the typist stops.
/// - **Whether.** `decide` scrolls only when the caret's target is outside the
///   viewport's comfort band (`visibleMargin` from each edge: off-screen "or
///   nearly so"). A target already on screen is left exactly where it is —
///   a preview that twitches while you type is worse than one that never moves.
/// - **How far.** The minimal scroll that brings the target back, plus
///   `revealInset` of breathing room, so the next few characters do not push
///   it straight back out and start a scroll-per-keystroke loop.
/// - **Manual scrolling wins.** A live scroll in the preview (wheel, trackpad,
///   scroller drag: `NSScrollView.willStartLiveScrollNotification`) disarms
///   following. The next *edit* re-arms it, and so does a caret move that
///   lands on a different source line or a different preview page than
///   where the reader scrolled away from — moving on is moving on, even
///   without typing. A caret move that stays on that same line, or a
///   recompile on its own, does not. ⌘⇧J always re-arms *and* scrolls at
///   once, whether or not the preference is even on: it is an explicit
///   instruction, not a hint. So scrolling away to read something stays put
///   until you actually type, jump elsewhere, or ask with ⌘⇧J.
/// - **No mapping, no motion.** Inside a comment, in the preamble, in a region
///   the compiler has not produced items for, or in another document, the
///   caret maps to nothing and the follower does nothing — quietly, with no
///   note and no scroll to a wrong place.
/// - **Motion.** Animated for short hops; a jump longer than
///   `animationDistanceLimit` viewports, or "Reduce motion", scrolls instantly
///   (`ReduceMotion.swift`).
///
/// The preference is `EditorPreferences.followCaretInPreview` (Settings ▸
/// Typing ▸ "Preview follows the caret"), default ON because the owner asked
/// for the behaviour. `enabledOverride` / `debounceOverride` are the
/// deterministic test hooks, in the style of
/// `CaptureInboxFeature.caretDestinationOverride`.
enum CaretFollow {
    /// Where the caret is on the preview's page column: a page number and a
    /// rect in that page's own points (y down from its top-left corner), so
    /// the target does not depend on the pane's fit-to-width scale or zoom.
    struct Target: Equatable {
        var page: Int
        var rect: CGRect
    }

    enum Decision: Equatable {
        /// The target's page is not in this layout (a frame that dropped it).
        case noPage
        /// The target is inside the comfort band, or the scroll view cannot
        /// move any further towards it: do not touch the scroll position.
        case alreadyVisible
        /// New content offset (document coordinates, y down) for the pane.
        case scroll(to: CGPoint, animated: Bool)
    }

    // MARK: policy constants

    /// How close to a viewport edge still counts as "off-screen, or nearly
    /// so". One line of body text at typical fit-to-width scales.
    static let visibleMargin: CGFloat = 24

    /// Extra room left beyond the edge when a scroll does happen: the target
    /// lands well inside the band, so growing the line by a few characters
    /// does not immediately push it out again (hysteresis). A quarter of the
    /// viewport, clamped, and never more than centring the target.
    static func revealInset(viewportHeight: CGFloat, targetHeight: CGFloat) -> CGFloat {
        let wanted = min(max(viewportHeight * 0.25, 64), 200)
        return max(visibleMargin, min(wanted, max(0, (viewportHeight - targetHeight) / 2)))
    }

    /// Animate a hop of at most this many viewport heights; a longer jump is
    /// instant (animating pages of distance is exactly where scroll animation
    /// drops frames, and nobody reads the blur).
    static let animationDistanceLimit: CGFloat = 2

    // MARK: preference and test overrides

    /// Test hook, checked before the preference (matches
    /// `CaptureInboxFeature.caretDestinationOverride`). Set it in `setUp`,
    /// clear it in `tearDown`.
    nonisolated(unsafe) static var enabledOverride: Bool?

    @MainActor static var isEnabled: Bool {
        enabledOverride ?? EditorPreferences.shared.followCaretInPreview
    }

    /// Test hook for the debounce; `0` fires synchronously, like the other
    /// debounced paths in this app (`ShellModel.debounceInterval`).
    nonisolated(unsafe) static var debounceOverride: TimeInterval?

    /// Measured on the Mac with `CaretFollowDebounceTests` replaying real
    /// typing cadences (see the printed table): 180 ms is the smallest value
    /// that still collapses a burst of fast typing (~60–110 ms between
    /// keystrokes, plus a recompile for each) into a single follow, while
    /// firing soon enough after the last character that the move reads as
    /// part of stopping rather than as a delayed jump.
    static let defaultDebounce: TimeInterval = 0.18

    /// `FLASHTEX_CARET_FOLLOW_DEBOUNCE_MS` (milliseconds) overrides the default.
    static let environmentDebounce: TimeInterval? = {
        guard let s = ProcessInfo.processInfo.environment["FLASHTEX_CARET_FOLLOW_DEBOUNCE_MS"], let ms = Double(s) else { return nil }
        return max(0, ms) / 1000
    }()

    static var debounceInterval: TimeInterval { debounceOverride ?? environmentDebounce ?? defaultDebounce }

    // MARK: the decision (pure)

    /// What the pane should do about `target`, given the page column's
    /// `layout`, the `visible` rect and the document's `contentSize` (both in
    /// document coordinates, y down). Pure: no AppKit, no clock, no state.
    static func decide(target: Target, layout: PreviewPageLayout, visible: CGRect,
                       contentSize: CGSize, reduceMotion: Bool) -> Decision {
        guard let pageFrame = layout.frame(of: target.page) else { return .noPage }
        guard visible.width > 0, visible.height > 0 else { return .alreadyVisible }
        let rect = CGRect(x: pageFrame.minX + target.rect.minX * layout.scale,
                          y: pageFrame.minY + target.rect.minY * layout.scale,
                          width: target.rect.width * layout.scale,
                          height: target.rect.height * layout.scale)
        let maxY = max(0, contentSize.height - visible.height)
        let maxX = max(0, contentSize.width - visible.width)

        // Vertical: the comfort band is the viewport inset by `visibleMargin`
        // (never more than half of it, for a very short pane).
        let inset = min(visibleMargin, max(0, visible.height / 2 - 1))
        let bandTop = visible.minY + inset, bandBottom = visible.maxY - inset
        let fits = rect.minY >= bandTop && rect.maxY <= bandBottom
        let coversBand = rect.minY <= bandTop && rect.maxY >= bandBottom // taller than the band: it fills the view
        var y = visible.minY
        if !(fits || coversBand) {
            let room = revealInset(viewportHeight: visible.height, targetHeight: rect.height)
            y = rect.minY < bandTop ? rect.minY - room : rect.maxY + room - visible.height
        }
        y = min(max(y, 0), maxY)

        // Horizontal: only when the document is actually wider than the pane
        // (otherwise AppKit owns x), and only to bring the target back in.
        var x = visible.minX
        if maxX > 0.5 {
            let hInset = min(visibleMargin, max(0, visible.width / 2 - 1))
            let left = visible.minX + hInset, right = visible.maxX - hInset
            if rect.minX < left || (rect.maxX > right && rect.width < visible.width) {
                x = rect.minX < left ? rect.minX - visibleMargin : rect.maxX + visibleMargin - visible.width
            }
            x = min(max(x, 0), maxX)
        }

        let dy = y - visible.minY, dx = x - visible.minX
        if abs(dy) < 0.5 && abs(dx) < 0.5 { return .alreadyVisible }
        let animated = !reduceMotion && abs(dy) <= animationDistanceLimit * visible.height
        return .scroll(to: CGPoint(x: x, y: y), animated: animated)
    }
}

// MARK: - when to follow

/// The debounced, armable state machine in front of `CaretFollow.decide`.
/// It answers *when* to look at the caret; the pane answers *whether* the
/// resulting target needs a scroll at all.
@MainActor
@Observable
final class CaretFollowController {
    /// Why a follow is being considered. `edit` and `explicit` always re-arm
    /// following after a manual scroll; `recompile` never does. A bare
    /// `note(.caretMove)` behaves like `recompile` (does not re-arm) — route
    /// caret moves through `noteCaretMove()` instead, which re-arms only
    /// when the caret lands on a different line or page.
    enum Reason: String, Equatable, Sendable { case edit, recompile, caretMove, explicit }

    /// One follow, for the pane. The pane acts on a token it has not acted on
    /// yet, so re-evaluating the view never re-scrolls.
    struct Request: Equatable {
        var token: Int
        var target: CaretFollow.Target
        var reason: Reason
    }

    private(set) var request: Request?

    /// Armed until the reader scrolls the preview by hand; the next edit (or
    /// ⌘⇧J) re-arms it.
    private(set) var isArmed = true

    // Evidence (tests, and the diagnostics panel's counters style).
    private(set) var follows = 0
    private(set) var skippedWithoutTarget = 0
    private(set) var skippedDisarmed = 0
    private(set) var lastReason: Reason?

    /// Where the caret is right now, asked for only when a follow actually
    /// fires (never per keystroke). `ShellModel` installs
    /// `caretPreviewTarget()`.
    @ObservationIgnored var target: (() -> CaretFollow.Target?)?

    /// The editor's one-based source line the caret is on right now. Read
    /// only at a manual scroll (to remember where the reader was) and when a
    /// caret move is considered while disarmed (to tell a move within that
    /// line from a move away from it) — never per keystroke, and never while
    /// armed. `ShellModel` installs a lookup over the active buffer.
    @ObservationIgnored var currentLine: (() -> Int?)?

    /// Injectable timer: tests replace it to drive the debounce without
    /// sleeping. Default = the main queue, like every other debounce here.
    @ObservationIgnored var schedule: (TimeInterval, DispatchWorkItem) -> Void = { delay, item in
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: item)
    }

    @ObservationIgnored private(set) var pending: DispatchWorkItem?
    @ObservationIgnored private var tokens = 0

    /// Line and (when the caret maps to a preview item) page as of the last
    /// manual scroll — the baseline a disarmed `.caretMove` is compared
    /// against in `noteCaretMove()`.
    @ObservationIgnored private var disarmedAtLine: Int?
    @ObservationIgnored private var disarmedAtPage: Int?

    /// True while a follow is scheduled but has not fired.
    var hasPendingFollow: Bool { pending != nil }

    func note(_ reason: Reason) {
        switch reason {
        case .edit, .explicit:
            isArmed = true
        case .recompile, .caretMove:
            guard isArmed else { skippedDisarmed += 1; return }
        }
        cancel()
        if reason == .explicit { fire(reason); return } // ⌘⇧J is an instruction: scrolls now, preference or not
        guard CaretFollow.isEnabled else { return }
        let item = DispatchWorkItem { [weak self] in self?.fire(reason) }
        pending = item
        let delay = CaretFollow.debounceInterval
        if delay <= 0 { item.perform(); return }
        schedule(delay, item)
    }

    /// A caret move, from `ShellModel.caretUTF16`'s `didSet`. Already armed,
    /// this is exactly `note(.caretMove)` — it just extends the debounce like
    /// any other trigger. Disarmed by an earlier manual scroll, it re-arms
    /// following only when the caret has landed on a different source line,
    /// or — when it still maps to a preview item — a different page than
    /// where that scroll happened: the editing moved on, which is exactly
    /// what following is for. A move that stays on the same line (arrow
    /// keys, clicking another word on it) does not re-arm, so reading in
    /// place never yanks the preview back out from under you.
    func noteCaretMove() {
        if !isArmed, currentLine?() != disarmedAtLine || target?()?.page != disarmedAtPage {
            isArmed = true
        }
        note(.caretMove)
    }

    /// The reader scrolled the preview themselves: stop following until they
    /// edit again, jump to another line or page, or ask with ⌘⇧J.
    func userDidScrollPreview() {
        isArmed = false
        disarmedAtLine = currentLine?()
        disarmedAtPage = target?()?.page
        cancel()
    }

    func cancel() {
        pending?.cancel()
        pending = nil
    }

    private func fire(_ reason: Reason) {
        pending = nil
        guard reason == .explicit || CaretFollow.isEnabled else { return }
        guard isArmed else { skippedDisarmed += 1; return }
        guard let target = target?() else { skippedWithoutTarget += 1; return } // no mapping: nothing happens
        tokens += 1
        request = Request(token: tokens, target: target, reason: reason)
        follows += 1
        lastReason = reason
    }
}

// MARK: - where the caret is, in the preview that is on screen

extension ShellModel {
    /// The caret's target in the preview currently showing: the v2 frame when
    /// the v2 pane has one, else the v1 result. Nil — quietly — when the caret
    /// maps to no preview item (a comment, the preamble, a region the producer
    /// has not laid out yet, another document, a historical snapshot).
    func caretPreviewTarget() -> CaretFollow.Target? {
        guard historicalPreview == nil else { return nil } // never follow onto an older snapshot
        guard let byte = caretByte else { return nil }
        if previewV2, let frame = displayListV2?.frame {
            return Self.caretTarget(byte: byte, path: activePath, in: frame)
        }
        guard let result else { return nil }
        return caretTarget(byte: byte, in: result)
    }

    /// v2 (display-list-v2): the union of the caret highlights the pane draws
    /// — the enclosing formula box for math, the exact caret bar or the whole
    /// cluster for text (`MathCaretHighlight.swift`) — on the first page that
    /// has one.
    static func caretTarget(byte: Int, path: String, in frame: V2Frame) -> CaretFollow.Target? {
        for page in frame.list.pages {
            var rect: CGRect?
            for highlight in V2Geometry.caretHighlights(containing: byte, path: path, in: page) {
                switch highlight {
                case .formula(let box):
                    rect = union(rect, Self.points(box.bounds))
                case .cluster(let match):
                    if let caret = match.caret {
                        rect = union(rect, CGRect(x: RenderingV2.points(caret.x), y: RenderingV2.points(caret.top),
                                                  width: 1, height: RenderingV2.points(caret.height)))
                    } else {
                        for r in match.hitRects { rect = union(rect, Self.points(r)) }
                    }
                }
            }
            if let rect { return CaretFollow.Target(page: page.number, rect: rect) }
        }
        return nil
    }

    /// v1 (`compile_result`): the union of the items under the caret on the
    /// first page that has one, at scale 1 — i.e. in page points.
    func caretTarget(byte: Int, in result: RuntimeV1.CompileResult) -> CaretFollow.Target? {
        let hits = caretIndex()?.itemsContaining(byte: byte)
            ?? CaretSync.itemsContaining(byte: byte, path: activePath, in: result)
        guard let number = hits.map(\.page).min(),
              let page = result.pages.first(where: { $0.number == number }) else { return nil }
        let wanted = Set(hits.filter { $0.page == number }.map(\.index))
        let rules = result.layoutCapabilities?.contains(RuntimeV1.LayoutCapabilities.rulesV1) == true
        var rect: CGRect?
        for hit in PreviewHitRects.compute(page: page, scale: 1, rulesNegotiated: rules) where wanted.contains(hit.index) {
            rect = union(rect, hit.rect)
        }
        guard let rect else { return nil }
        return CaretFollow.Target(page: number, rect: rect)
    }

    private static func points(_ r: RenderingV2.Rect) -> CGRect {
        CGRect(x: RenderingV2.points(r.x), y: RenderingV2.points(r.top),
               width: RenderingV2.points(r.width), height: RenderingV2.points(r.height))
    }
}

private func union(_ a: CGRect?, _ b: CGRect) -> CGRect { a.map { $0.union(b) } ?? b }
