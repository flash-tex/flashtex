import AppKit
import Foundation

/// `File > Export PDF (exact, v2)…`: the loaded rendering-v2 display list is
/// handed to `flashtex-pdf-exact from-v2` (crates/pdf), which embeds the
/// exact font programs (GID-preserving CFF subsets), places every glyph by
/// original GID at its exact tick position, writes typed rules and ToUnicode,
/// and refuses anything it cannot express (alpha, images, missing fonts,
/// non-integer ticks) naming the item — never a silent approximation.
///
/// Runs off the main thread with the same drained pipes and timeout as the
/// runtime-v1 export; the result is reported in `captureNote`.
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

    struct Outcome: Equatable {
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
        var outData = Data(), errData = Data()
        let group = DispatchGroup()
        group.enter(); DispatchQueue.global().async { outData = stdout.fileHandleForReading.readDataToEndOfFile(); group.leave() }
        group.enter(); DispatchQueue.global().async { errData = stderr.fileHandleForReading.readDataToEndOfFile(); group.leave() }
        try p.run()
        let deadline = DispatchTime.now() + timeout
        let waiter = DispatchGroup()
        waiter.enter(); DispatchQueue.global().async { p.waitUntilExit(); waiter.leave() }
        if waiter.wait(timeout: deadline) == .timedOut {
            p.terminate()
            _ = waiter.wait(timeout: .now() + 5)
        }
        group.wait()
        let bytes = (try? FileManager.default.attributesOfItem(atPath: out.path)[.size] as? Int) ?? 0
        return Outcome(exitCode: p.terminationStatus, stdout: String(decoding: outData, as: UTF8.self),
                       stderr: String(decoding: errData, as: UTF8.self), bytes: bytes)
    }
}

extension ShellModel {
    /// Export the loaded v2 display list through the exact route. The list's
    /// source JSON (already verified by the pane) is handed to the tool as is.
    func exportPDFExact() {
        guard case .loaded(let frame, let source)? = displayListV2 else {
            captureNote = displayListV2?.isLoading == true ? "Nothing to export yet: a display list is still loading." : "Nothing to export: no v2 display list loaded (File > Open Display List (v2)…)."
            return
        }
        // display-list-v2-window §4.1: a windowed reply is not a complete
        // compile and is never the source of a PDF export. An export that
        // silently omitted the elided pages would be worse than one that refuses.
        if let window = frame.window, window.elidedCount > 0 {
            captureNote = "Export refused: this preview is a \(window.pageCount)-page window over a \(window.documentPageCount)-page document (\(window.elidedCount) page\(window.elidedCount == 1 ? "" : "s") not loaded). A PDF must be the whole document."
            return
        }
        let listURL: URL
        do { listURL = try source.listFileURL() } catch { // a live frame's line is written to a temporary file (V2Source)
            captureNote = "Exact export: could not write the live display list to a file: \(error.localizedDescription)"
            return
        }
        guard let tool = ExactPDFExport.locateTool() else {
            captureNote = "No flashtex-pdf-exact found (build crates/pdf or set FLASHTEX_PDF_EXACT); exact export unavailable."
            return
        }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.pdf]
        panel.nameFieldStringValue = "\(frame.list.projectId)-r\(frame.list.revision)-exact.pdf"
        panel.message = "Export the v2 display list through flashtex-pdf-exact from-v2 (exact glyphs by original GID, embedded font programs)"
        guard panel.runModal() == .OK, let out = panel.url else { return }
        // The panel already asked about overwriting: whatever is on disk now is
        // what the user approved. Any later change is refused (ExportSession).
        exportPDFExact(listURL: listURL, tool: tool, destination: .recordingCurrentDisk(out))
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
