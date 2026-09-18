//  The manifest half of recording: `RECORD_SNAPSHOTS=1` rewrites the
//  reference PNGs, and this rewrites the record of which machine they came
//  from, in the same run. Keeping the two together is what makes the gate
//  re-runnable rather than a hand-edit — after a design pass invalidates
//  every reference, re-recording updates both.

import XCTest

@MainActor
final class SnapshotRecordingTests: XCTestCase {

    /// Runs only while recording, alongside the PNGs being rewritten.
    func testRecordingAlsoRecordsTheEnvironment() throws {
        guard SnapshotEnvironment.isRecording else {
            throw XCTSkip("""
                not recording. Set RECORD_SNAPSHOTS=1 to rewrite the reference PNGs and \
                \(SnapshotEnvironment.manifestName) together.
                """)
        }
        let environment = try XCTUnwrap(SnapshotEnvironment.current,
                                        "cannot record references on a machine that captures nothing")
        try SnapshotEnvironment.writeManifest(environment)
        let readBack = try XCTUnwrap(SnapshotEnvironment.reference, "the manifest must be readable after writing")
        XCTAssertEqual(readBack, environment)
        print("recorded snapshot references at \(environment.describe) -> \(SnapshotEnvironment.manifestURL.path)")
    }

    // MARK: the gate's own logic, checked on every machine

    /// The comparison is narrow on purpose: an unrecorded OS must not lock a
    /// machine out, and a different scale must always lock it out.
    func testEnvironmentComparison() {
        let retina26 = SnapshotEnvironment(backingScale: 2, macOS: "26.0")
        XCTAssertTrue(retina26.matches(retina26))
        XCTAssertFalse(retina26.matches(SnapshotEnvironment(backingScale: 1, macOS: "26.0")),
                       "a @1x capture of a @2x reference is a different image, always")
        XCTAssertFalse(retina26.matches(SnapshotEnvironment(backingScale: 2, macOS: "27.0")),
                       "a different OS rasterises text and materials differently")
        // The references committed in #471/#476 predate this manifest: their
        // scale is recoverable from the PNG dimensions, their OS is not. An
        // unrecorded OS compares as "matches anything" so the missing fact
        // gates on nothing, rather than locking every machine out.
        let scaleOnly = SnapshotEnvironment(backingScale: 2, macOS: nil)
        XCTAssertTrue(scaleOnly.matches(retina26))
        XCTAssertTrue(retina26.matches(scaleOnly))
        XCTAssertFalse(scaleOnly.matches(SnapshotEnvironment(backingScale: 1, macOS: nil)))
    }

    /// Both environments end up in the skip message, so a skipped run says
    /// what it would have needed instead of just disappearing.
    func testDescriptionNamesBothFacts() {
        XCTAssertEqual(SnapshotEnvironment(backingScale: 2, macOS: "26.0").describe,
                       "backing scale @2x, macOS 26.0")
        XCTAssertEqual(SnapshotEnvironment(backingScale: 1, macOS: nil).describe,
                       "backing scale @1x, macOS unrecorded")
    }

    /// The committed manifest has to describe the committed PNGs: they are
    /// 640x320 px for a 320x160 pt probe, so the references are @2x.
    func testCommittedManifestMatchesTheCommittedReferences() throws {
        let reference = try XCTUnwrap(SnapshotEnvironment.reference,
                                      "the references are committed, so the manifest describing them must be too")
        XCTAssertEqual(reference.backingScale, 2,
                       "every committed reference PNG is exactly twice its surface's point size")
    }
}
