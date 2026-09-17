//  Proves the render loop itself works before anyone relies on it: the
//  dependency resolves, the target builds, both appearances render, and PNGs
//  land on disk where an agent can open them. If this fails, no other
//  snapshot result in this target means anything.
//
//  Two tests, deliberately: the render loop is checked on EVERY machine,
//  while comparing against the committed references is checked only where
//  those references mean something (SnapshotEnvironment.swift). Gating the
//  whole file would leave CI proving nothing about the harness at all.

import AppKit
import SwiftUI
import XCTest

@MainActor
final class HarnessSmokeTests: XCTestCase {
    /// A deliberately trivial surface using semantic colours, so a failure
    /// here is the harness and never the app.
    private struct Probe: View {
        var body: some View {
            VStack(spacing: 8) {
                Text("FlashTeX").font(.title2)
                Text("render loop").foregroundStyle(.secondary)
            }
            .padding(24)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .windowBackgroundColor))
        }
    }

    private static let probeSize = CGSize(width: 320, height: 160)

    /// Ungated, and the reason the gate is safe: wherever this suite runs, the
    /// window server still composites a hosted SwiftUI view in both
    /// appearances at this machine's scale. What a mismatched machine skips is
    /// pixel EQUALITY against someone else's hardware — not whether the
    /// harness works.
    func testHarnessRendersOnThisMachine() throws {
        let environment = try XCTUnwrap(SnapshotEnvironment.current,
                                        "the window server returned no capture at all")
        for appearance in Appearance.allCases {
            let capture = try XCTUnwrap(
                hostAndCapture(Probe(), size: Self.probeSize, styleMask: [.borderless],
                               appearance: appearance, settle: 0.25),
                "\(appearance.rawValue): window-server capture returned nil")
            XCTAssertEqual(capture.image.size, Self.probeSize,
                           "\(appearance.rawValue): the capture is the surface, in points")
            // Pixels are points times the backing scale: the property that
            // makes a capture unportable, measured rather than assumed.
            XCTAssertEqual(capture.pixels,
                           CGSize(width: Self.probeSize.width * CGFloat(environment.backingScale),
                                  height: Self.probeSize.height * CGFloat(environment.backingScale)),
                           "\(appearance.rawValue): captured at \(capture.pixels) on a \(environment.describe) machine")
        }
    }

    /// The reference comparison, which only means something on the machine the
    /// references came from.
    func testHarnessRendersBothAppearances() throws {
        try SnapshotEnvironment.requireComparableToReferences()
        assertSurfaceBothAppearances(Probe(), named: "harness-probe", size: Self.probeSize)
    }
}
