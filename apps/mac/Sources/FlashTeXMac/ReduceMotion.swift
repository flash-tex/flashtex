import AppKit
import SwiftUI

/// "Reduce motion" (System Settings › Accessibility › Display), as one
/// source of truth for the preview (lane mac-editor-a11y-3). Reads
/// `NSWorkspace.shared.accessibilityDisplayShouldReduceMotion` on every use
/// (the setting can change while the app runs; reading is a property access,
/// so nothing caches it). Tests inject `override`.
///
/// What it changes:
/// - `animate(_:_:)`: SwiftUI `withAnimation` becomes an immediate change.
///   (The v1 pane's caret-page `scrollTo` that used it is gone; caret
///   following is the animated site now, and it reads `isEnabled` through
///   `PreviewAnchorProbe.reduceMotion` below.)
/// - `CaretFollow` (CaretFollow.swift): a follow scroll is animated only when
///   reduce motion is off (and the hop is short); otherwise the preview jumps
///   straight to the caret with no animation.
/// - `PreviewAnchorProbe`: the settle window keeps every *synchronous*
///   anchor correction (they run inside SwiftUI's layout pass, before the
///   frame is drawn, and are what keeps the content still) but no longer
///   scrolls from the deferred end-of-window timer: a scroll that fires
///   150 ms after the layout change, with no user action, on an already
///   drawn frame is exactly the unexpected motion the setting asks to avoid.
///   Drift found at that point is counted (`driftsLeftUncorrected`) and the
///   anchor is re-captured from where the content actually is.
enum ReduceMotion {
    /// Test injection; nil reads the system setting.
    @MainActor static var override: Bool?

    @MainActor static var isEnabled: Bool {
        override ?? NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
    }

    /// `withAnimation(animation, body)` unless reduce motion is on, in which
    /// case `body` runs without an animation transaction.
    @MainActor @discardableResult
    static func animate<R>(_ animation: Animation = .default, _ body: () throws -> R) rethrows -> R {
        if isEnabled { return try body() }
        return try withAnimation(animation, body)
    }
}
