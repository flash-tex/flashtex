import AppKit
import Foundation

/// `File > Export PDF…` (⌘⇧E) — the app's only PDF export route, and the bytes
/// `File > Print…` sends to the printer. The loaded rendering-v2 display list
/// is handed to `flashtex-pdf-exact from-v2` (crates/pdf), which embeds the
/// exact font programs (GID-preserving CFF subsets), places every glyph by
/// original GID at its exact tick position, writes typed rules and ToUnicode,
/// and refuses anything it cannot express (alpha, missing fonts, non-integer
/// ticks) naming the item — never a silent approximation. `flashtex build`
/// (the CLI) standardised on this same route.
///
/// Runs off the main thread with drained pipes and a timeout; the result is
/// reported in `captureNote`.
@MainActor
enum ExactPDFExport {
    /// $FLASHTEX_PDF_EXACT, the app bundle, then crates/pdf/target/{release,debug}.
    static func locateTool() -> URL? {
        let fm = FileManager.default
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_PDF_EXACT"], fm.isExecutableFile(atPath: env) {
            return URL(fileURLWithPath: env)
        }
        if let bundled = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("flashtex-pdf-exact"),
           fm.isExecutableFile(atPath: bundled.path) {
            return bundled
        }
        guard let root = ShellModel.locateRepoRoot() else { return nil }
        for profile in ["release", "debug"] {
            let url = root.appendingPathComponent("crates/pdf/target/\(profile)/flashtex-pdf-exact")
            if fm.isExecutableFile(atPath: url.path) { return url }
        }
        return nil
    }

    /// Font directories handed to the tool in addition to its own defaults:
    /// the display list's producer directories are unknown here, so the
    /// bundled/vendored Latin Modern and `FLASHTEX_LM_DIR` are offered.
    static func fontDirectories() -> [String] {
        var dirs: [String] = []
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_FONT_DIRS"] {
            dirs += env.split(separator: ":").map(String.init)
        }
        dirs += PreviewFonts.latinModernSearchPaths
        return dirs.filter { FileManager.default.fileExists(atPath: $0) }
    }

    struct Outcome: Equatable, Sendable {
        var exitCode: Int32
        var stdout: String
        var stderr: String
        var bytes: Int
        var succeeded: Bool { exitCode == 0 && bytes > 0 }
    }

    /// Runs `from-v2 LIST --out PDF [--font-dir …]`, draining both pipes;
    /// kills the tool after `timeout` seconds.
    nonisolated static func run(tool: URL, list: URL, out: URL, fontDirs: [String], timeout: TimeInterval = 60) throws -> Outcome {
        let p = Process()
        p.executableURL = tool
        var args = ["from-v2", list.path, "--out", out.path]
        for d in fontDirs { args += ["--font-dir", d] }
        p.arguments = args
        let stdout = Pipe(), stderr = Pipe()
        p.standardOutput = stdout; p.standardError = stderr
        // The drains are read back after a bounded wait that can expire while
        // they are still running, so they publish under a lock rather than
        // writing captured vars the caller may read concurrently.
        let drained = Drained()
        let group = DispatchGroup()
        group.enter(); DispatchQueue.global().async { let d = stdout.fileHandleForReading.readDataToEndOfFile(); drained.put(out: d); group.leave() }
        group.enter(); DispatchQueue.global().async { let d = stderr.fileHandleForReading.readDataToEndOfFile(); drained.put(err: d); group.leave() }
        try p.run()
        let deadline = DispatchTime.now() + timeout
        let waiter = DispatchGroup()
        waiter.enter(); DispatchQueue.global().async { p.waitUntilExit(); waiter.leave() }
        if waiter.wait(timeout: deadline) == .timedOut {
            p.terminate()
            _ = waiter.wait(timeout: .now() + 5)
        }
        // `timeout` bounded the process, not the pipes. `terminate()` is SIGTERM
        // to the direct child only: if it ignores the signal, or a descendant it
        // spawned still holds the write ends, `readDataToEndOfFile` never sees
        // EOF and an unbounded `group.wait()` blocks this thread forever. In the
        // Mac test bundle that thread is the main thread, so the whole xctest
        // process hangs with no output and no failure (observed: a 65-minute
        // hang in SearchableTextTests, main thread parked in
        // `_dispatch_group_wait_slow`). Bound it, escalate to SIGKILL, and
        // report whatever was drained.
        if group.wait(timeout: .now() + 10) == .timedOut {
            if p.isRunning { kill(p.processIdentifier, SIGKILL) }
            _ = group.wait(timeout: .now() + 5)
        }
        let bytes = (try? FileManager.default.attributesOfItem(atPath: out.path)[.size] as? Int) ?? 0
        let (outData, errData) = drained.read()
        return Outcome(exitCode: p.terminationStatus, stdout: String(decoding: outData, as: UTF8.self),
                       stderr: String(decoding: errData, as: UTF8.self), bytes: bytes)
    }

    /// Both pipe drains' output, published under a lock: the bounded wait above
    /// can return while a drain is still running.
    private final class Drained: @unchecked Sendable {
        private let lock = NSLock()
        private var out = Data(), err = Data()
        func put(out d: Data) { lock.lock(); out = d; lock.unlock() }
        func put(err d: Data) { lock.lock(); err = d; lock.unlock() }
        func read() -> (Data, Data) { lock.lock(); defer { lock.unlock() }; return (out, err) }
    }
}

extension ShellModel {
    /// Why `File > Export PDF…` would refuse right now, or nil if it would open
    /// the save panel. `File > Print…` shares this predicate
    /// (`PrintController.exportWouldProceed`) so the two commands cannot drift:
    /// they print and write the same bytes from the same display list.
    func exportPDFRefusal() -> String? {
        if let why = historicalRefusal(of: "export") { return why }
        guard let frame = displayListV2?.retained?.frame else {
            return displayListV2?.isLoading == true
                ? "Nothing to export yet: a display list is still loading."
                : "Nothing to export: no rendering-v2 display list. Attach the render pipeline (⌘⇧R) and compile, or open a list with File > Open Display List (v2)…."
        }
        if frame.list.pages.isEmpty {
            return "Nothing to export: this compile produced no pages."
        }
        if frame.list.window != nil || displayListV2?.retained?.source.isDeltaLine == true, wholeDocumentProducer == nil {
            // display-list-v2-window §4.1: a windowed reply is an incomplete
            // view and never the source of a PDF export or a print job; a
            // frame rebuilt from a display_list_delta has no full line. With
            // the render pipeline available the whole document is re-rendered
            // instead (WholeDocumentList.swift); without it, say so.
            return Self.noProducerReason(window: frame.list.window)
        }
        if ExactPDFExport.locateTool() == nil {
            return "No flashtex-pdf-exact found (build crates/pdf, or set FLASHTEX_PDF_EXACT); PDF export is unavailable."
        }
        return nil
    }

    /// `File > Export PDF…`: the loaded v2 display list through the exact
    /// route. The list's source JSON (already verified by the pane) is handed
    /// to the tool as is.
    func exportPDF() {
        if let why = exportPDFRefusal() { reportExportFailure(why); return }
        // `retained`, not `.loaded`: while a newer list is being verified the
        // pane keeps showing the last verified frame, and Export writes exactly
        // what the pane is showing.
        guard let (frame, _) = displayListV2?.retained, let tool = ExactPDFExport.locateTool() else {
            reportExportFailure("Nothing to export: the preview has no display list right now.")
            return
        }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.pdf]
        panel.nameFieldStringValue = "\(frame.list.projectId)-r\(frame.list.revision).pdf"
        panel.message = "Export the document through flashtex-pdf-exact (exact glyphs by original GID, embedded font programs)"
        // The destination is chosen before any long re-render, so the user is
        // never left waiting on a panel that has not appeared yet.
        guard panel.runModal() == .OK, let out = panel.url else { return }
        // The panel already asked about overwriting: whatever is on disk now is
        // what the user approved. Any later change is refused (ExportSession).
        let destination = ExportSession.Destination.recordingCurrentDisk(out)
        Task { @MainActor in
            // A windowed frame re-renders the whole document first
            // (WholeDocumentList.swift); an unwindowed one is used as is.
            switch await exportListURL() {
            case .failure(let why):
                reportExportFailure(why.reason)
            case .success(let list):
                exportPDFExact(listURL: list.url, tool: tool, destination: destination) { [weak self] report in
                    if list.temporary { try? FileManager.default.removeItem(at: list.url) }
                    if case .failed = report.state, let note = self?.captureNote { self?.reportExportFailure(note) }
                }
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

    /// Non-interactive core (tests, automation) in the pre-session shape:
    /// the destination as found on disk right now is taken as approved, the
    /// run goes through `exportSession` (sibling temp file, atomic replace,
    /// cancel/timeout by pid), and `captureNote` reports on the main actor.
    /// `Outcome` is nil when the tool could not be launched or was cancelled/
    /// timed out (see `ExportSession.Report` for the full state).
    func exportPDFExact(listURL: URL, tool: URL, to out: URL, completion: (@MainActor (ExactPDFExport.Outcome?) -> Void)? = nil) {
        exportPDFExact(listURL: listURL, tool: tool, destination: .recordingCurrentDisk(out)) { report in
            switch report.state {
            case .succeeded(let bytes, _):
                completion?(ExactPDFExport.Outcome(exitCode: 0, stdout: report.stdout, stderr: report.stderr, bytes: bytes))
            case .failed where report.exitCode != nil:
                completion?(ExactPDFExport.Outcome(exitCode: report.exitCode ?? -1, stdout: report.stdout, stderr: report.stderr, bytes: 0))
            default:
                completion?(nil)
            }
        }
    }
}
