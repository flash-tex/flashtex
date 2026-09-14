import AppKit

/// Hosted tests build real `NSWindow`s so that AppKit and SwiftUI geometry
/// (text container widths, caret rects, scroll offsets) is measured the way the
/// shipping app measures it. Two separate things have to be true for that to be
/// invisible to whoever is actually using the Mac, and they are not the same
/// thing:
///
/// 1. **The process must not activate.** An `xctest` process starts life as a
///    regular app, and a regular app that orders a window front is pulled in
///    front of the user. ``prepare()`` drops the process to a non-activating
///    activation policy, which fixes the focus theft.
///
/// 2. **The window must not be drawn over a display.** A non-activating process
///    still draws its windows. The earlier version of this file claimed
///    `.prohibited` "also keeps the windows off the screen"; that was wrong, and
///    the owner kept seeing editor windows flash up during test runs.
///    `CGWindowListCopyWindowInfo` caught them at layer 0, on screen, near the
///    top-left corner.
///
/// ``window(contentRect:styleMask:backing:defer:)`` is the fix for (2), and it
/// is the only supported way for a hosted test to build a window.
///
/// ## Why the obvious fix does not work
///
/// Passing a far-offscreen origin straight to `NSWindow(contentRect:…)` does
/// nothing, because **AppKit relocates the window inside the initializer**,
/// before the window is ever ordered in. Measured on macOS 26.6:
///
/// ```
/// NSWindow(contentRect: NSRect(x: -10_000, y: -10_000, width: 600, height: 400), styleMask: [.titled], …)
/// // .frame immediately after init: (80.0, 816.0, 600.0, 432.0)   ← already back on the display
/// // .frame after orderFrontRegardless(): (80.0, 517.0, 600.0, 432.0)
/// // CGWindowList: 600x432@(80,33)
/// ```
///
/// That CoreGraphics position is exactly what the owner was seeing, and it is
/// why the three test sites that already passed `-10_000` origins never helped.
/// Overriding `constrainFrameRect` alone does not help either — it only governs
/// the second move, and the initializer has already done the first one.
///
/// Two moves are therefore needed, and both are load-bearing:
///
/// * set the origin **after** `init`, which the initializer cannot undo, and
/// * override `constrainFrameRect(_:to:)` so ordering the window front does not
///   drag its title bar back onto a display.
///
/// With both, the window sits at ``offscreenOrigin`` with `window.screen == nil`
/// and a frame that intersects no display, while still laying out, drawing,
/// accepting `makeFirstResponder`, and reporting `AXWindow`/`AXStandardWindow`
/// to the accessibility suites.
///
/// Child windows the app creates itself follow the host off-screen: the
/// completion popup positions itself from the parent's caret rect in screen
/// coordinates, so an off-screen parent yields an off-screen popup.
public enum HostedWindowSupport {

    /// Far outside the union of any plausible display arrangement, in AppKit's
    /// global (bottom-left origin) coordinate space. Displays are laid out
    /// within a few thousand points of the origin, so -30 000 clears every
    /// arrangement while staying far away from any coordinate range where
    /// CoreGraphics starts misbehaving.
    public static let offscreenOrigin = NSPoint(x: -30_000, y: -30_000)

    /// The window class hosted tests get. `constrainFrameRect` is overridden so
    /// AppKit stops "helpfully" pulling the title bar back onto a display when
    /// the window is ordered front. `canBecomeKey`/`canBecomeMain` are forced on
    /// so that a borderless hosted window behaves like the titled ones — several
    /// suites drive first-responder behaviour and must not be weakened just
    /// because the window moved off-screen.
    public final class Window: NSWindow {
        public override func constrainFrameRect(_ frameRect: NSRect, to screen: NSScreen?) -> NSRect { frameRect }
        public override var canBecomeKey: Bool { true }
        public override var canBecomeMain: Bool { true }
    }

    private static let applyOnce: Bool = {
        let app = NSApplication.shared
        if app.setActivationPolicy(.prohibited) { return true }
        return app.setActivationPolicy(.accessory)
    }()

    /// Puts the test process into a non-activating activation policy, once.
    ///
    /// This stops the process being *activated*. It does not stop its windows
    /// being *drawn* — that is ``window(contentRect:styleMask:backing:defer:)``'s
    /// job. Callers do not need to call this directly; the window factory calls
    /// it before building anything.
    @discardableResult
    public static func prepare() -> Bool { applyOnce }

    /// Builds a hosted window that is laid out and drawn normally but never
    /// placed over a display.
    ///
    /// The **origin of `contentRect` is deliberately ignored**; only its size is
    /// used. Every hosted window goes to ``offscreenOrigin`` regardless of what
    /// the caller asked for, because a hosted window on a display is the bug
    /// this helper exists to prevent. Test geometry is unaffected: hosted tests
    /// measure in view or window coordinates (`convert(_:to: nil)`), which do
    /// not depend on where the window sits in the global space.
    public static func window(contentRect: NSRect,
                       styleMask: NSWindow.StyleMask,
                       backing: NSWindow.BackingStoreType = .buffered,
                       defer flag: Bool = false) -> Window {
        prepare()
        let window = Window(contentRect: contentRect, styleMask: styleMask, backing: backing, defer: flag)
        // After init, never in it: the initializer refuses off-screen origins
        // and silently moves the window back onto a display.
        window.setFrameOrigin(offscreenOrigin)
        return window
    }

    /// The policy actually in force, for the guard tests below.
    public static var currentPolicy: NSApplication.ActivationPolicy {
        NSApplication.shared.activationPolicy()
    }

    /// True when the process can never be brought to the front by ordering a
    /// window in.
    public static var isNonActivating: Bool {
        currentPolicy == .prohibited || currentPolicy == .accessory
    }

    /// True when `window` is drawn somewhere no display can show it. This is the
    /// property the owner's "stop the flashing" request actually needs, and it
    /// is what the guard tests assert.
    ///
    /// Note that this is *not* the same as being absent from CoreGraphics'
    /// on-screen window list: `kCGWindowListOptionOnScreenOnly` means "ordered
    /// in and not hidden", so a window parked at ``offscreenOrigin`` is still
    /// listed, with bounds that lie outside every display.
    public static func isClearOfEveryDisplay(_ window: NSWindow) -> Bool {
        window.screen == nil && !NSScreen.screens.contains { $0.frame.intersects(window.frame) }
    }
}
