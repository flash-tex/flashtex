import AppKit
import Foundation

/// `File > Export PDF…` (⌘⇧E) — the app's only PDF export route, and the bytes
/// `File > Print…` sends to the printer: the PDF pdflatex would write for the
/// editor's text, from the engine host's one-shot `export: true` run
/// (`EngineV3Session.export`), published through the export session (a
/// sibling temp file, the overwrite-conflict check, an atomic rename;
/// ExportSession.swift). The result is reported in `captureNote`.
extension ShellModel {
    /// Why `File > Export PDF…` would refuse right now, or nil if it would open
    /// the save panel. `File > Print…` shares this predicate
    /// (`PrintController.exportWouldProceed`) so the two commands cannot drift:
    /// they print and write the same bytes from the same run.
    func exportPDFRefusal() -> String? {
        engineV3.exportRefusal()
    }

    /// `File > Export PDF…`: the engine's PDF, to a file chosen in a save panel.
    func exportPDF() {
        exportPDFEngineV3()
    }

    /// Export PDF…: the PDF pdflatex would write for the editor's text, from
    /// the host (`EngineV3Session.export`), published through the export
    /// session (atomic, conflict-checked, cancellable from the status bar).
    func exportPDFEngineV3() {
        if let why = engineV3.exportRefusal() { reportExportFailure(why); return }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.pdf]
        let job = (engineV3.mainFile as NSString).lastPathComponent.replacingOccurrences(of: ".tex", with: "")
        panel.nameFieldStringValue = "\(job.isEmpty ? "document" : job).pdf"
        panel.message = "Export the PDF pdflatex would write for this document"
        guard panel.runModal() == .OK, let out = panel.url else { return }
        exportPDFEngineV3(to: .recordingCurrentDisk(out))
    }

    /// The engine-v3 export to a chosen destination (no panel; tests).
    func exportPDFEngineV3(to destination: ExportSession.Destination, completion: (@MainActor (ExportSession.Report) -> Void)? = nil) {
        captureNote = "Exporting PDF…"
        let started = exportSession.startExternal(destination: destination, cancel: { [weak self] in self?.engineV3.cancelExport() }) { [weak self] report in
            guard let self else { return }
            switch report.state {
            case .succeeded(let bytes, let sha):
                self.captureNote = "Exported PDF (\(bytes) bytes, sha256 \(sha.prefix(12))) to \(destination.url.path)"
            case .cancelled:
                self.captureNote = "Export cancelled; nothing was written to \(destination.url.lastPathComponent)."
            case .failed(let reason):
                self.captureNote = reason
            case .idle, .running:
                self.captureNote = "Export ended in an unexpected state (\(report.state))."
            }
            completion?(report)
        }
        guard started else { return }
        engineV3.export(model: self) { [weak self] result in
            guard let self else { return }
            switch result {
            case .success(let data): self.exportSession.finishExternal(pdf: data)
            case .failure(.cancelled): self.exportSession.finishExternal(pdf: nil)
            case .failure(.refused(let why)), .failure(.failed(let why)): self.exportSession.finishExternal(pdf: nil, failure: why)
            }
        }
    }

    /// An interactive export that did not write a file says so where the
    /// user is looking: the status line, and an alert (never under XCTest).
    /// The status line alone read as "export fails silently".
    func reportExportFailure(_ why: String) {
        captureNote = why
        guard !Self.runningUnderXCTest else { return }
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = "PDF export failed"
        alert.informativeText = why
        alert.runModal()
    }
}
