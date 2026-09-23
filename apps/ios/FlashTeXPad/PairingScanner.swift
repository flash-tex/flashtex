import AVFoundation
import SwiftUI
import VisionKit

/// Scans the Mac's pairing QR (`flashtex-nearby://pair?…`) with VisionKit's
/// `DataScannerViewController` where the device supports it (camera + Neural
/// Engine; never in the simulator, where `isSupported` is false). The first
/// recognised barcode whose payload has our scheme is handed back; anything
/// else is ignored. The caller falls back to pasting the URL text.
struct PairingScannerView: UIViewControllerRepresentable {
    var onPayload: (String) -> Void
    /// Reports what the camera can actually do once the scanner is running,
    /// so the sheet can say "fixed-focus camera" instead of silently not
    /// focusing. Called on the main queue.
    var onFocus: ((CameraFocus.Outcome) -> Void)?

    /// `DataScannerViewController` owns its capture session privately, so
    /// continuous autofocus is set on the shared `AVCaptureDevice` shortly
    /// after `startScanning()` — the scanner reconfigures the device as its
    /// session starts, and anything applied before that is overwritten.
    /// `.near` range restriction is what makes a QR held at reading distance
    /// resolve instead of the lens hunting past it.
    static let focusSettleDelay: TimeInterval = 0.6

    static var isAvailable: Bool { DataScannerViewController.isSupported && DataScannerViewController.isAvailable }

    func makeUIViewController(context: Context) -> DataScannerViewController {
        let vc = DataScannerViewController(recognizedDataTypes: [.barcode(symbologies: [.qr])], qualityLevel: .balanced,
                                           recognizesMultipleItems: false, isHighFrameRateTrackingEnabled: false,
                                           isHighlightingEnabled: true)
        vc.delegate = context.coordinator
        try? vc.startScanning()
        context.coordinator.startFocusing(after: Self.focusSettleDelay)
        return vc
    }

    func updateUIViewController(_ vc: DataScannerViewController, context: Context) {}

    static func dismantleUIViewController(_ vc: DataScannerViewController, coordinator: Coordinator) {
        vc.stopScanning()
        coordinator.stopFocusing()
    }

    func makeCoordinator() -> Coordinator { Coordinator(onPayload: onPayload, onFocus: onFocus) }

    final class Coordinator: NSObject, DataScannerViewControllerDelegate {
        let onPayload: (String) -> Void
        let onFocus: ((CameraFocus.Outcome) -> Void)?
        private var delivered = false
        private var focus: CameraFocus.Reapplier?
        private var focusWork: DispatchWorkItem?

        init(onPayload: @escaping (String) -> Void, onFocus: ((CameraFocus.Outcome) -> Void)? = nil) {
            self.onPayload = onPayload
            self.onFocus = onFocus
        }

        /// Applies continuous near-range autofocus once the scanner's own
        /// session has settled, and keeps it applied via subject-area changes.
        func startFocusing(after delay: TimeInterval) {
            let work = DispatchWorkItem { [weak self] in
                guard let self else { return }
                self.focus = CameraFocus.Reapplier(nearRange: true) { [weak self] outcome in
                    self?.onFocus?(outcome)
                }
            }
            focusWork = work
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: work)
        }

        func stopFocusing() {
            focusWork?.cancel()
            focusWork = nil
            focus = nil
        }

        func dataScanner(_ scanner: DataScannerViewController, didAdd addedItems: [RecognizedItem], allItems: [RecognizedItem]) {
            guard !delivered else { return }
            for item in addedItems {
                if case .barcode(let b) = item, let text = b.payloadStringValue, text.hasPrefix("flashtex-nearby://") {
                    delivered = true
                    scanner.stopScanning()
                    onPayload(text)
                    return
                }
            }
        }
    }
}
