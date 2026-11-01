import Foundation
import ObjectiveC

/// The shell's PDF export session (`ExportSession.swift`): one observable
/// state machine the status bar reads for progress/Cancel and the export
/// entry point drives (ExactPDFExport.swift). Stored as an associated object
/// so this stays a self-contained extension file (the parent may fold it
/// into `ShellModel` as a stored `let exportSession = ExportSession()`).
extension ShellModel {
    private static var exportSessionKey = 0

    var exportSession: ExportSession {
        if let existing = objc_getAssociatedObject(self, &Self.exportSessionKey) as? ExportSession { return existing }
        let session = ExportSession()
        objc_setAssociatedObject(self, &Self.exportSessionKey, session, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return session
    }

    /// The status bar's Cancel: ends the running export (the engine's
    /// export run); the destination is never touched.
    func cancelExactExport() {
        guard exportSession.state.isRunning else { return }
        captureNote = "Cancelling the export…"
        exportSession.cancel()
    }
}
