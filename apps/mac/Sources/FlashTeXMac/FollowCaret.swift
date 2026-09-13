import Foundation
import CoreGraphics

/// Pure decision logic for "Preview follows the caret" (gap: follow-caret,
/// lane mac-follow-caret; `EditorPreferences.previewFollowsCaret`, default
/// on). `ShellModel.revealCaretInPreview()` stays the manual command
/// (⌘⇧J / View menu / palette / accessibility); this type is the automatic
/// counterpart that a debounced timer consults on every caret move.
///
/// One place answers whether an automatic preview scroll should happen; the
/// AppKit side (`PreviewAnchorProbe` in PreviewAnchor.swift, already the
/// owner of the enclosing `NSScrollView` for both the v1 and v2 panes) only
/// supplies the caret's resolved rect, runs the 250ms debounce timer, tracks
/// manual-scroll timestamps from live-scroll notifications, and performs the
/// scroll `decide` returns. Keeping the policy here means it is unit-tested
/// as a value → value function, no window server or timer involved.
enum FollowCaret {
    /// One page item's rect, in the same document coordinate space as
    /// `Input.visibleRect` (top-down points, the pane's scroll content):
    /// the caret's current preview target when resolving, or the caret
    /// target snapshotted at the moment of the last manual scroll when
    /// carried as `Input.lastUserScrollTarget` (comparing the two is how a
    /// caret move to a different page/line re-arms following even inside
    /// the yield window).
    struct Target: Equatable {
        var page: Int
        var rect: CGRect
    }

    /// Why `decide` returned `.none`, for logging/tests; never surfaced to the user.
    enum Reason: Equatable {
        /// "Preview follows the caret" is off.
        case disabledByPreference
        /// The caret has no preview mapping (preamble, comment, uncompiled
        /// region, or no compiled result/frame at all) — a quiet no-op.
        case noMapping
        /// The target is already inside the visible rect, inset by `margin`.
        case alreadyVisible
        /// A scrollbar drag / trackpad / scroll-wheel gesture is in progress.
        case userIsDragging
        /// The user scrolled manually within `yieldWindow` and the caret is
        /// still on the same page/line it was then (not re-armed).
        case yieldingToUser
        /// No scroll view geometry is available yet (pane not laid out).
        case noVisibleRect
    }

    enum Action: Equatable {
        case scroll(to: CGRect)
        case none(Reason)
    }

    struct Input {
        var preferenceEnabled: Bool
        /// The caret's page item, or nil when it has no preview mapping.
        var target: Target?
        /// The pane's current visible rect (document coordinates), nil when
        /// the pane has no scroll view attached yet.
        var visibleRect: CGRect?
        var now: Date
        /// When the user last scrolled manually (wheel/trackpad/scrollbar),
        /// nil if never observed.
        var lastUserScrollAt: Date?
        /// The follow target current at that last manual scroll.
        var lastUserScrollTarget: Target?
        /// True while a scrollbar drag / live trackpad scroll is in progress.
        var isUserDragging: Bool
        /// Visible-rect slack: a target within `margin` of the edge already
        /// counts as visible, so a caret sitting near the edge does not
        /// trigger a one-pixel correction on every keystroke.
        var margin: CGFloat
        /// How long a manual scroll suppresses following, for the same target.
        var yieldWindow: TimeInterval

        init(preferenceEnabled: Bool, target: Target?, visibleRect: CGRect?, now: Date,
             lastUserScrollAt: Date? = nil, lastUserScrollTarget: Target? = nil, isUserDragging: Bool = false,
             margin: CGFloat = 24, yieldWindow: TimeInterval = 3) {
            self.preferenceEnabled = preferenceEnabled
            self.target = target
            self.visibleRect = visibleRect
            self.now = now
            self.lastUserScrollAt = lastUserScrollAt
            self.lastUserScrollTarget = lastUserScrollTarget
            self.isUserDragging = isUserDragging
            self.margin = margin
            self.yieldWindow = yieldWindow
        }
    }

    /// The single decision point. Order matters: the preference gate and the
    /// no-mapping check are absolute; dragging always wins over an otherwise
    /// due scroll; a recent manual scroll yields UNLESS the caret has since
    /// moved to a different page/line (`target != lastUserScrollTarget`);
    /// only then is the visible-rect (with `margin`) comparison consulted.
    static func decide(_ input: Input) -> Action {
        guard input.preferenceEnabled else { return .none(.disabledByPreference) }
        guard let target = input.target else { return .none(.noMapping) }
        if input.isUserDragging { return .none(.userIsDragging) }
        if let lastAt = input.lastUserScrollAt, input.now.timeIntervalSince(lastAt) < input.yieldWindow,
           input.lastUserScrollTarget == target {
            return .none(.yieldingToUser)
        }
        guard let visible = input.visibleRect else { return .none(.noVisibleRect) }
        let tolerant = visible.insetBy(dx: -input.margin, dy: -input.margin)
        if tolerant.contains(target.rect) { return .none(.alreadyVisible) }
        return .scroll(to: target.rect)
    }

    // MARK: - debounce coalescing (pure)

    /// Trailing-edge debounce over an already time-sorted event stream:
    /// consecutive events less than `window` apart merge into one burst:
    /// only the LAST event of the burst fires, at `window` after it. Models
    /// exactly what a timer reset on every caret move produces (a typing
    /// burst fires the decision once, not once per keystroke) without a real
    /// run loop, so the coalescing policy is tested with plain `Date` math.
    static func debounce<T>(_ events: [(at: Date, value: T)], window: TimeInterval) -> [(at: Date, value: T)] {
        guard !events.isEmpty else { return [] }
        var fires: [(Date, T)] = []
        var i = 0
        while i < events.count {
            var j = i
            while j + 1 < events.count, events[j + 1].at.timeIntervalSince(events[j].at) < window { j += 1 }
            fires.append((events[j].at.addingTimeInterval(window), events[j].value))
            i = j + 1
        }
        return fires
    }
}
