//  The render loop for design work.
//
//  A design pass cannot see what it changed. `swift test --filter
//  DesignSnapshots` renders each surface to a PNG under
//  `Tests/DesignSnapshots/__Snapshots__/`, which the agent then opens. That
//  turns "writing blind" into "writing and looking", and it doubles as
//  regression protection once the images are committed.
//
//  Headless on purpose: no Screen Recording permission, no window over any
//  display, so it runs over SSH where `screencapture` cannot. Surfaces are
//  hosted in real HostedWindows windows (parked off every display) and
//  captured through `CGWindowListCreateImage` of our own window: the window
//  server composites what the user would actually see — including the
//  macOS 26 glass materials, which never reach `cacheDisplay` or
//  `CALayer.render` (both painted the sidebar blank white; that bug cost an
//  afternoon, do not go back to them).
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

/// `CGWindowListCreateImage`, bound at runtime.
///
/// The symbol is deprecated (its replacement, ScreenCaptureKit, needs Screen
/// Recording permission — which a headless SSH test run does not have and
/// must not require), and Swift offers no scoped way to silence a
/// deprecation without deprecating every caller. It is permission-free for
/// the calling process's own windows, which is all this harness captures.
private typealias WindowListCreateImage =
    @convention(c) (CGRect, UInt32, UInt32, UInt32) -> Unmanaged<CGImage>?
private let cgWindowListCreateImage: WindowListCreateImage? = {
    guard let sym = dlsym(dlopen(nil, RTLD_LAZY), "CGWindowListCreateImage") else { return nil }
    return unsafeBitCast(sym, to: WindowListCreateImage.self)
}()

/// The window-server composite of `window`, at backing-store resolution:
/// what the user would actually see, glass materials included.
@MainActor
private func windowServerImage(of window: NSWindow) -> NSImage? {
    window.displayIfNeeded()
    let options: CGWindowListOption = [.optionIncludingWindow]
    let imageOptions: CGWindowImageOption = [.boundsIgnoreFraming, .bestResolution]
    guard let cg = cgWindowListCreateImage?(
        .null, options.rawValue, CGWindowID(window.windowNumber), imageOptions.rawValue
    )?.takeRetainedValue() else { return nil }
    return NSImage(cgImage: cg, size: window.frame.size)
}

/// Hosts `view` in a window (off every display), lets SwiftUI settle, and
/// asserts (or records) the window-server capture as a PNG.
///
/// `styleMask: [.borderless]` renders just the surface; pass a titled mask
/// (see `assertWindowSurface`) for the whole window frame with title bar and
/// toolbar. The appearance is set on the window, so semantic colours resolve
/// for real — no `performAsCurrentDrawingAppearance` tricks needed.
@MainActor
private func assertHostedSurface<V: View>(
    _ view: V,
    named name: String,
    size: CGSize,
    styleMask: NSWindow.StyleMask,
    appearance: Appearance,
    settle: TimeInterval,
    record recording: Bool,
    file: StaticString,
    testName: String,
    line: UInt
) {
    let window = HostedWindowSupport.window(
        contentRect: NSRect(origin: .zero, size: size), styleMask: styleMask)
    window.appearance = appearance.nsAppearance
    window.colorSpace = .sRGB
    let controller = NSHostingController(rootView: view)
    // Keep the window at `size`: with sizing options on, assigning the
    // controller momentarily resizes the window to SwiftUI's preferred size,
    // and NavigationSplitView collapses its sidebar during that dip.
    controller.sizingOptions = []
    controller.view.frame = CGRect(origin: .zero, size: size)
    window.contentViewController = controller
    window.setContentSize(size)
    window.makeKeyAndOrderFront(nil)
    // Real run-loop turns: List/NSTableView rows only populate in a window,
    // `.toolbar` installs asynchronously, and `.task`s need to run.
    RunLoop.main.run(until: Date(timeIntervalSinceNow: settle))
    window.layoutIfNeeded()
    guard let image = windowServerImage(of: window) else {
        XCTFail("window-server capture returned nil", file: file, line: line)
        return
    }
    withSnapshotTesting(record: recording ? .all : .missing) {
        assertSnapshot(
            of: image,
            as: .image(precision: 0.99, perceptualPrecision: 0.98),
            named: "\(name)-\(appearance.rawValue)",
            file: file,
            testName: testName,
            line: line
        )
    }
    window.orderOut(nil)
}

/// One surface, no window chrome, one appearance.
@MainActor
func assertSurface<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1400, height: 900),
    appearance: Appearance,
    settle: TimeInterval = 0.25,
    record recording: Bool = ProcessInfo.processInfo.environment["RECORD_SNAPSHOTS"] == "1",
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    assertHostedSurface(view.frame(width: size.width, height: size.height),
                        named: name, size: size, styleMask: [.borderless],
                        appearance: appearance, settle: settle, record: recording,
                        file: file, testName: testName, line: line)
}

/// Both appearances of one surface, which the acceptance criteria require for
/// every screen.
@MainActor
func assertSurfaceBothAppearances<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1400, height: 900),
    settle: TimeInterval = 0.25,
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    for appearance in Appearance.allCases {
        assertSurface(view, named: name, size: size, appearance: appearance, settle: settle,
                      file: file, testName: testName, line: line)
    }
}

/// A whole window — title bar, toolbar, content — for shell-level review.
@MainActor
func assertWindowSurface<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1440, height: 900),
    appearance: Appearance,
    settle: TimeInterval = 0.5,
    record recording: Bool = ProcessInfo.processInfo.environment["RECORD_SNAPSHOTS"] == "1",
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    assertHostedSurface(view, named: name, size: size,
                        styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
                        appearance: appearance, settle: settle, record: recording,
                        file: file, testName: testName, line: line)
}

/// Both appearances of one windowed surface.
@MainActor
func assertWindowSurfaceBothAppearances<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1440, height: 900),
    settle: TimeInterval = 0.5,
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    for appearance in Appearance.allCases {
        assertWindowSurface(view, named: name, size: size, appearance: appearance, settle: settle,
                            file: file, testName: testName, line: line)
    }
}
