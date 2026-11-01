import Foundation
import ObjectiveC

/// The shell's exact-PDF export session (`ExportSession.swift`): one observable
/// state machine the capture bar reads for progress/Cancel and the export
/// entry points drive. Stored as an associated object so this stays a
/// self-contained extension file (the parent may fold it into `ShellModel` as
/// a stored `let exportSession = ExportSession()`).
extension ShellModel {
    private static var exportSessionKey = 0

    var exportSession: ExportSession {
        if let existing = objc_getAssociatedObject(self, &Self.exportSessionKey) as? ExportSession { return existing }
        let session = ExportSession()
        objc_setAssociatedObject(self, &Self.exportSessionKey, session, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return session
    }

    /// Cancel affordance for the capture bar: terminates only the export's
    /// own tool process; the destination is never touched.
    func cancelExactExport() {
        guard exportSession.state.isRunning else { return }
        captureNote = "Cancelling exact export…"
        exportSession.cancel()
    }

    /// Runs the exact route through the session and mirrors the outcome into
    /// `captureNote`. `destination.expectedDiskSHA256` is what the user saw
    /// when choosing the file; any change since is refused, keeping that file.
    func exportPDFExact(listURL: URL, tool: URL, destination: ExportSession.Destination,
                        timeout: TimeInterval = 60, completion: (@MainActor (ExportSession.Report) -> Void)? = nil) {
        captureNote = "Exporting exact PDF…"
        exportSession.start(tool: tool, list: listURL, destination: destination,
                            fontDirs: ExactPDFExport.fontDirectories(), timeout: timeout) { [weak self] report in
            guard let self else { return }
            switch report.state {
            case .succeeded(let bytes, let sha):
                self.captureNote = "Exported exact PDF (\(bytes) bytes, sha256 \(sha.prefix(12))) to \(destination.url.path)"
            case .cancelled:
                self.captureNote = "Exact export cancelled; nothing was written to \(destination.url.lastPathComponent)."
            case .failed(let reason):
                self.captureNote = reason.hasPrefix("Exact export") || reason.hasPrefix("Export refused") ? reason : "Exact export failed to run: \(reason)"
            case .idle, .running:
                self.captureNote = "Exact export ended in an unexpected state (\(report.state))."
            }
            completion?(report)
        }
    }
}
