//  The render loop for design work.
//
//  A design pass cannot see what it changed. `swift test --filter
//  DesignSnapshots` renders each surface to a PNG under
//  `Tests/DesignSnapshots/__Snapshots__/`, which the agent then opens. That
//  turns "writing blind" into "writing and looking", and it doubles as
//  regression protection once the images are committed.
//
//  Headless on purpose: no window server, no Screen Recording permission, so
//  it runs over SSH where `screencapture` cannot.
//
//  Recording: set `RECORD_SNAPSHOTS=1` to (re)write the reference PNGs instead
//  of comparing against them. Do that deliberately, never to make a red test
//  green.

import AppKit
import HostedWindows
import SnapshotTesting
import SwiftUI
import XCTest

enum Appearance: String, CaseIterable {
    case light = "light"
    case dark = "dark"

    var nsAppearance: NSAppearance {
        NSAppearance(named: self == .light ? .aqua : .darkAqua) ?? NSAppearance()
    }
}


/// Off-screen windows have nothing behind them for `NSVisualEffectView` to
/// sample, so materials draw black. Forcing every effect view inactive makes
/// them draw their plain (Reduce-Transparency-style) colour instead — the
/// same ground the app must remain legible on anyway.
@MainActor
private func neutralizeMaterials(in view: NSView) {
    if let effect = view as? NSVisualEffectView { effect.state = .inactive }
    for sub in view.subviews { neutralizeMaterials(in: sub) }
}

/// Renders `view` at `size` in one appearance and asserts (or records) its PNG.
///
/// The appearance is applied to the hosting view *and* forced as the current
/// drawing appearance, because SwiftUI resolves semantic colours lazily: a
/// view merely placed inside a dark `NSHostingView` still paints light unless
/// the draw happens inside `performAsCurrentDrawingAppearance`. That exact
/// mistake is what made the completion popup unreadable in light mode.
@MainActor
func assertSurface<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1400, height: 900),
    appearance: Appearance,
    settle: TimeInterval = 0.2,
    record recording: Bool = ProcessInfo.processInfo.environment["RECORD_SNAPSHOTS"] == "1",
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    // A real (off-screen, HostedWindows) window, not a bare NSHostingView:
    // List/NSTableView surfaces only populate their rows inside a window,
    // and `.task`s need a few run-loop turns.
    let window = HostedWindowSupport.window(
        contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless])
    window.appearance = appearance.nsAppearance
    let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height))
    host.frame = CGRect(origin: .zero, size: size)
    window.contentView = host
    window.makeKeyAndOrderFront(nil)
    RunLoop.main.run(until: Date(timeIntervalSinceNow: settle))
    neutralizeMaterials(in: host)
    appearance.nsAppearance.performAsCurrentDrawingAppearance {
        host.layoutSubtreeIfNeeded()
    }
    withSnapshotTesting(record: recording ? .all : .missing) {
        assertSnapshot(
            of: host,
            as: .image(precision: 0.99, perceptualPrecision: 0.98),
            named: "\(name)-\(appearance.rawValue)",
            file: file,
            testName: testName,
            line: line
        )
    }
    window.orderOut(nil)
}

/// Renders `view` inside a real (off-screen, HostedWindows) `NSWindow` and
/// asserts (or records) the whole window frame — title bar, toolbar and
/// content. `NavigationSplitView`, `.toolbar` and focus behaviour only work
/// in a window, so the full shell must go through here; individual panes can
/// use the cheaper `assertSurface`.
///
/// The run loop is spun briefly so SwiftUI can install the toolbar, populate
/// its `List`s and finish the first `.task`s; keep fixtures deterministic
/// within that window (no timers, no producers).
@MainActor
func assertWindowSurface<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1440, height: 900),
    appearance: Appearance,
    settle: TimeInterval = 0.35,
    record recording: Bool = ProcessInfo.processInfo.environment["RECORD_SNAPSHOTS"] == "1",
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    let window = HostedWindowSupport.window(
        contentRect: NSRect(origin: .zero, size: size),
        styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView])
    window.appearance = appearance.nsAppearance
    let controller = NSHostingController(rootView: view)
    // Keep the window at `size`: with sizing options on, assigning the
    // controller momentarily resizes the window to SwiftUI's preferred size,
    // and NavigationSplitView collapses its sidebar during that dip.
    controller.sizingOptions = []
    controller.view.frame = CGRect(origin: .zero, size: size)
    window.contentViewController = controller
    window.setContentSize(size)
    window.makeKeyAndOrderFront(nil)
    RunLoop.main.run(until: Date(timeIntervalSinceNow: settle))
    window.layoutIfNeeded()
    guard let frameView = window.contentView?.superview else {
        XCTFail("hosted window has no frame view", file: file, line: line)
        return
    }
    neutralizeMaterials(in: frameView)
    appearance.nsAppearance.performAsCurrentDrawingAppearance {
        frameView.layoutSubtreeIfNeeded()
    }
    withSnapshotTesting(record: recording ? .all : .missing) {
        assertSnapshot(
            of: frameView,
            as: .image(precision: 0.99, perceptualPrecision: 0.98),
            named: "\(name)-\(appearance.rawValue)",
            file: file,
            testName: testName,
            line: line
        )
    }
    window.orderOut(nil)
}

/// Both appearances of one windowed surface.
@MainActor
func assertWindowSurfaceBothAppearances<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1440, height: 900),
    settle: TimeInterval = 0.35,
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    for appearance in Appearance.allCases {
        assertWindowSurface(view, named: name, size: size, appearance: appearance, settle: settle,
                            file: file, testName: testName, line: line)
    }
}

/// Both appearances of one surface, which the acceptance criteria require for
/// every screen.
@MainActor
func assertSurfaceBothAppearances<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1400, height: 900),
    settle: TimeInterval = 0.2,
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    for appearance in Appearance.allCases {
        assertSurface(view, named: name, size: size, appearance: appearance, settle: settle,
                      file: file, testName: testName, line: line)
    }
}
