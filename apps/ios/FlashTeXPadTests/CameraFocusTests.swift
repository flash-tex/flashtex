import AVFoundation
import XCTest
@testable import FlashTeXPad

/// `CameraFocus` decides what to say about a camera as much as what to set on
/// it. The device configuration itself needs real hardware — the simulator has
/// no `AVCaptureDevice` — so what is pinned here is the part that must stay
/// honest: a camera that cannot autofocus is reported as such rather than
/// being silently treated as fixed.
final class CameraFocusTests: XCTestCase {
    func testFixedFocusCameraIsReportedNotSilentlyIgnored() {
        let outcome = CameraFocus.Outcome.unsupported(deviceName: "Front Camera")
        XCTAssertFalse(outcome.isFocusing)
        let note = try? XCTUnwrap(outcome.note)
        XCTAssertNotNil(note, "a fixed-focus camera must explain itself to the user")
        XCTAssertTrue(outcome.note?.contains("Front Camera") == true)
        XCTAssertTrue(outcome.note?.contains("fixed-focus") == true)
    }

    func testFocusingCameraShowsNoNote() {
        XCTAssertNil(CameraFocus.Outcome.focusing(deviceName: "Back Camera", nearRange: true).note,
                     "a working camera must not nag")
        XCTAssertTrue(CameraFocus.Outcome.focusing(deviceName: "Back Camera", nearRange: true).isFocusing)
    }

    func testNoDeviceIsDistinctFromFixedFocus() {
        XCTAssertEqual(CameraFocus.Outcome.noDevice.note, "No camera available here.")
        XCTAssertFalse(CameraFocus.Outcome.noDevice.isFocusing)
        XCTAssertNotEqual(CameraFocus.Outcome.noDevice, .unsupported(deviceName: "x"))
    }

    /// In the simulator there is no capture device, so `apply` must return
    /// `.noDevice` rather than trapping. On hardware this is `.focusing`.
    func testApplyWithoutADeviceIsNoDevice() {
        XCTAssertEqual(CameraFocus.apply(nearRange: true, device: nil), .noDevice)
    }
}
