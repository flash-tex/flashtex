import AVFoundation
import Foundation

/// Continuous autofocus for the capture sessions this app does not own.
///
/// Both camera paths here are system-owned controllers — VisionKit's
/// `DataScannerViewController` (QR pairing) and `UIImagePickerController`
/// (`.camera`) — and neither exposes its `AVCaptureSession` or its
/// `AVCaptureDeviceInput`. The one lever left is the `AVCaptureDevice`
/// itself, which is a process-wide object: focus settings applied to it
/// while a session is running do apply to that session.
///
/// That is why `apply` is called *after* the controller has started, and
/// again on `subjectAreaDidChangeNotification` — a system controller
/// reconfigures the device when its session starts, so a setting applied
/// too early is simply overwritten.
///
/// `Outcome` is deliberately honest: a camera that reports
/// `isFocusModeSupported(.continuousAutoFocus) == false` is a fixed-focus
/// camera and there is nothing to fix in software. Callers surface
/// `.unsupported` rather than implying a fix that did not happen.
enum CameraFocus {
    enum Outcome: Equatable {
        /// Continuous autofocus is now set on `deviceName`.
        case focusing(deviceName: String, nearRange: Bool)
        /// The device exists but cannot autofocus (fixed-focus camera).
        case unsupported(deviceName: String)
        /// No video capture device (the simulator, or no camera permission yet).
        case noDevice

        var isFocusing: Bool { if case .focusing = self { return true }; return false }

        /// Shown in the scanner when the camera cannot autofocus, so the
        /// owner is told the truth instead of being left wondering.
        var note: String? {
            switch self {
            case .focusing: return nil
            case .unsupported(let name):
                return "\(name) is a fixed-focus camera — hold the code about 20 cm away and steady."
            case .noDevice:
                return "No camera available here."
            }
        }
    }

    /// The device a scanner/picker session will have picked: the back
    /// wide-angle camera, falling back to the system default video device.
    static func videoDevice() -> AVCaptureDevice? {
        AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back)
            ?? AVCaptureDevice.default(for: .video)
    }

    /// Puts `device` into continuous autofocus (plus continuous auto
    /// exposure) and turns on subject-area monitoring so the system
    /// re-evaluates focus when the framing changes.
    ///
    /// `nearRange` sets `autoFocusRangeRestriction = .near`, which matters
    /// for QR codes held close: without it the lens hunts across the whole
    /// range and often settles on the far field.
    @discardableResult
    static func apply(nearRange: Bool, device: AVCaptureDevice? = videoDevice()) -> Outcome {
        guard let device else { return .noDevice }
        let name = device.localizedName
        guard device.isFocusModeSupported(.continuousAutoFocus) else { return .unsupported(deviceName: name) }
        do {
            try device.lockForConfiguration()
            defer { device.unlockForConfiguration() }
            device.focusMode = .continuousAutoFocus
            var near = false
            if nearRange, device.isAutoFocusRangeRestrictionSupported {
                device.autoFocusRangeRestriction = .near
                near = true
            }
            if device.isExposureModeSupported(.continuousAutoExposure) {
                device.exposureMode = .continuousAutoExposure
            }
            // Fires subjectAreaDidChangeNotification so `Reapplier` can
            // re-assert focus when the iPad or the paper moves.
            device.isSubjectAreaChangeMonitoringEnabled = true
            return .focusing(deviceName: name, nearRange: near)
        } catch {
            return .unsupported(deviceName: name)
        }
    }

    /// Keeps continuous autofocus asserted for as long as it is retained:
    /// applies once now and again on every subject-area change. Retain it
    /// for the lifetime of the camera screen and drop it on dismiss.
    final class Reapplier {
        private let nearRange: Bool
        private var observer: NSObjectProtocol?
        private(set) var outcome: Outcome

        init(nearRange: Bool, onOutcome: ((Outcome) -> Void)? = nil) {
            self.nearRange = nearRange
            self.outcome = CameraFocus.apply(nearRange: nearRange)
            onOutcome?(outcome)
            observer = NotificationCenter.default.addObserver(
                forName: AVCaptureDevice.subjectAreaDidChangeNotification, object: nil, queue: .main
            ) { [weak self] _ in
                guard let self else { return }
                self.outcome = CameraFocus.apply(nearRange: self.nearRange)
            }
        }

        deinit {
            if let observer { NotificationCenter.default.removeObserver(observer) }
            // Leave the shared device as we found it for other sessions.
            if let d = CameraFocus.videoDevice(), d.isSubjectAreaChangeMonitoringEnabled,
               (try? d.lockForConfiguration()) != nil {
                d.isSubjectAreaChangeMonitoringEnabled = false
                if d.isAutoFocusRangeRestrictionSupported { d.autoFocusRangeRestriction = .none }
                d.unlockForConfiguration()
            }
        }
    }
}
