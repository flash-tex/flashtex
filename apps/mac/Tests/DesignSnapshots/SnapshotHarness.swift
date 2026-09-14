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

/// Renders `view` at `size` in one appearance and asserts (or records) its PNG.
///
/// The appearance is applied to the hosting view *and* forced as the current
/// drawing appearance, because SwiftUI resolves semantic colours lazily: a
/// view merely placed inside a dark `NSHostingView` still paints light unless
/// the draw happens inside `performAsCurrentDrawingAppearance`. That exact
/// mistake is what made the completion popup unreadable in light mode.
func assertSurface<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1400, height: 900),
    appearance: Appearance,
    record recording: Bool = ProcessInfo.processInfo.environment["RECORD_SNAPSHOTS"] == "1",
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height))
    host.frame = CGRect(origin: .zero, size: size)
    host.appearance = appearance.nsAppearance
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
}

/// Both appearances of one surface, which the acceptance criteria require for
/// every screen.
func assertSurfaceBothAppearances<V: View>(
    _ view: V,
    named name: String,
    size: CGSize = CGSize(width: 1400, height: 900),
    file: StaticString = #filePath,
    testName: String = #function,
    line: UInt = #line
) {
    for appearance in Appearance.allCases {
        assertSurface(view, named: name, size: size, appearance: appearance,
                      file: file, testName: testName, line: line)
    }
}
