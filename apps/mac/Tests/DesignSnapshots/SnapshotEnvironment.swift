//  Which machine the reference PNGs were recorded on, and whether this one
//  can be compared against them.
//
//  A window-server capture is not portable. The committed references are
//  640x320 px for a 320x160 pt probe -- exactly @2x, recorded on a Retina
//  Mac. A GitHub `macos-26` runner composites its windows on a virtual
//  display at @1x, and rasterises text with a different OS build besides. The
//  harness already compares perceptually (precision 0.99,
//  perceptualPrecision 0.98) and the difference still exceeds that budget:
//  even `harness-probe`, two `Text`s on `windowBackgroundColor`, mismatches.
//  A tolerance wide enough to absorb a resolution change would be wide enough
//  to miss the design regressions these tests exist to catch.
//
//  So the suite states the environment its references came from, measures the
//  one it is running in, and SKIPS -- loudly, with both environments named in
//  the message, counted by XCTest -- when they differ. It never passes
//  quietly. `SnapshotRecordingTests` rewrites the manifest whenever the
//  references themselves are re-recorded, so this stays in step with them
//  instead of drifting into a lie.

import AppKit
import HostedWindows
import SwiftUI
import XCTest

/// The properties of a machine that decide what a capture looks like.
struct SnapshotEnvironment: Codable, Equatable {
    /// Capture pixels per point: 2 on a Retina display, 1 on the virtual
    /// display a CI runner composites onto. Measured, never assumed.
    var backingScale: Int
    /// `major.minor` of the OS that rasterised the text and the materials.
    /// Optional because it cannot be recovered from a PNG: the references in
    /// this repo were committed before the manifest existed, so their scale
    /// is known (from their pixel dimensions) and their OS is not. A nil here
    /// means "not recorded" and is compared as "matches anything"; the next
    /// recording fills it in and it is enforced from then on.
    var macOS: String?

    static let manifestName = "environment.json"

    var describe: String {
        "backing scale @\(backingScale)x, macOS \(macOS ?? "unrecorded")"
    }

    /// Only the properties both sides actually recorded are compared, so an
    /// old manifest gates on what it knows instead of failing shut.
    func matches(_ other: SnapshotEnvironment) -> Bool {
        guard backingScale == other.backingScale else { return false }
        guard let a = macOS, let b = other.macOS else { return true }
        return a == b
    }

    // MARK: this machine

    @MainActor private static var measured: SnapshotEnvironment?

    /// The environment the current process renders in, or nil when the probe
    /// capture failed (no window server at all). The scale is *measured*
    /// through the real capture path -- a probe window, captured by the
    /// window server, pixels divided by points -- rather than read off
    /// `NSScreen`, because these windows are deliberately parked off every
    /// display and it is the capture that has to match, not the desktop.
    /// Cached: the probe costs a window and a settle, and the answer cannot
    /// change within a run.
    @MainActor
    static var current: SnapshotEnvironment? {
        if let cached = measured { return cached }
        guard let scale = measuredBackingScale() else { return nil }
        let v = ProcessInfo.processInfo.operatingSystemVersion
        let env = SnapshotEnvironment(backingScale: scale, macOS: "\(v.majorVersion).\(v.minorVersion)")
        measured = env
        return env
    }

    @MainActor
    private static func measuredBackingScale() -> Int? {
        let size = CGSize(width: 64, height: 64)
        let window = HostedWindowSupport.window(
            contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless])
        defer { window.orderOut(nil) }
        let controller = NSHostingController(rootView: Color.gray.frame(width: size.width, height: size.height))
        controller.sizingOptions = []
        controller.view.frame = CGRect(origin: .zero, size: size)
        window.contentViewController = controller
        window.setContentSize(size)
        window.makeKeyAndOrderFront(nil)
        // The window server needs a turn before it has anything to composite;
        // capturing straight away can return nil (the same reason every
        // assertion in SnapshotHarness settles first).
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.1))
        window.layoutIfNeeded()
        guard let cg = windowServerCapture(of: window), size.width > 0 else { return nil }
        return Int((CGFloat(cg.width) / size.width).rounded())
    }

    // MARK: the references

    /// `__Snapshots__/environment.json`, next to the PNGs it describes.
    static var manifestURL: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .appendingPathComponent("__Snapshots__")
            .appendingPathComponent(manifestName)
    }

    /// What the committed references were recorded on, or nil when the
    /// manifest is missing or unreadable.
    static var reference: SnapshotEnvironment? {
        guard let data = try? Data(contentsOf: manifestURL) else { return nil }
        return try? JSONDecoder().decode(SnapshotEnvironment.self, from: data)
    }

    static func writeManifest(_ env: SnapshotEnvironment) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try FileManager.default.createDirectory(at: manifestURL.deletingLastPathComponent(),
                                                withIntermediateDirectories: true)
        try (encoder.encode(env) + Data("\n".utf8)).write(to: manifestURL)
    }

    // MARK: the gate

    /// True while `RECORD_SNAPSHOTS=1`: recording writes new references for
    /// whatever machine it runs on, so it must never be gated.
    static var isRecording: Bool {
        ProcessInfo.processInfo.environment["RECORD_SNAPSHOTS"] == "1"
    }

    /// Call from `setUp()`. Throws `XCTSkip` -- which XCTest counts and
    /// prints, so the run reports "N skipped" and says why -- when this
    /// machine cannot be compared against the committed references.
    @MainActor
    static func requireComparableToReferences() throws {
        guard !isRecording else { return }
        guard let here = current else {
            throw XCTSkip("""
                the window server returned no capture for the probe window, so this machine \
                cannot render snapshots at all -- there is nothing to compare. Snapshot \
                comparison is SKIPPED, not passed.
                """)
        }
        guard let reference = reference else {
            throw XCTSkip("""
                no snapshot reference manifest (\(manifestURL.path)), so there is nothing to \
                compare against. This machine renders at \(here.describe). Record references \
                with `RECORD_SNAPSHOTS=1 swift test --filter DesignSnapshots`, or run the \
                "Record design snapshots" workflow.
                """)
        }
        guard here.matches(reference) else {
            throw XCTSkip("""
                snapshot references were recorded at \(reference.describe); this machine renders \
                at \(here.describe). A window-server capture is not portable across either, and \
                the harness already compares perceptually, so comparing them would report a \
                design regression that is really a hardware difference. Pixel comparison is \
                SKIPPED, not passed. To compare here, re-record with `RECORD_SNAPSHOTS=1 swift \
                test --filter DesignSnapshots` on this machine.
                """)
        }
    }
}
