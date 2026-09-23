import Foundation
import FlashTeXProtocol

/// The complete display list of a document whose list cannot travel in one
/// reply — the input `File > Export PDF…` and `File > Print…` need when the
/// pane is showing a page window.
///
/// `display-list-v2-window` engages only *after* the producer has refused the
/// unwindowed reply ("compile_result would be N bytes …, over the L-byte reply
/// limit"), so simply re-asking for an unwindowed reply would be refused for
/// the same reason: the limit belongs to the JSON Lines transport, not to the
/// document. The producer's own `--v2 FILE` side output has no such limit, and
/// `flashtex-render` writes the complete list of the last render to that file
/// **even when the reply it sent was the over-limit refusal** — `Outputs::write`
/// runs off `Reply.rendered`, which that refusal still carries
/// (`crates/render-pipeline/src/protocol.rs`).
///
/// So Export runs the same producer binary one-shot in worker mode, hands it
/// the current buffers on stdin, ignores whatever reply comes back, and takes
/// the complete list from the file. That is what `flashtex build --v2` does.
/// The list is produced from the very request the app just sent, so it matches
/// the editor exactly; there is no freshness question and no torn read (the
/// file is private to this run and is read only after the process exited).
@MainActor
enum WholeDocumentList {
    struct Outcome: Equatable, Sendable {
        var exitCode: Int32
        var stderr: String
        var bytes: Int
        /// The producer may exit non-zero on a failed compile and still have
        /// written a usable list; the file is what matters.
        var succeeded: Bool { bytes > 0 }
    }

    struct Failure: Error, CustomStringConvertible { let description: String }

    /// Where the list to export came from. `temporary` files are this run's
    /// own and the caller deletes them.
    struct Resolved: Equatable, Sendable { var url: URL; var temporary: Bool }

    /// A refusal, in the words `captureNote` should show.
    struct Refusal: Error, Equatable, CustomStringConvertible {
        let reason: String
        var description: String { reason }
    }

    /// Runs `producer --v2 <out> [--images]` with `requestLine` on stdin,
    /// draining both output pipes, and returns once it has exited (or been
    /// terminated at `timeout`). Never touches the main actor.
    nonisolated static func render(producer: URL, requestLine: Data, out: URL,
                                   environment: [String: String]?, images: Bool,
                                   timeout: TimeInterval = 180) throws -> Outcome {
        let process = Process()
        process.executableURL = producer
        process.arguments = ["--v2", out.path] + (images ? ["--images"] : [])
        if let environment { process.environment = environment }
        let stdin = Pipe(), stdout = Pipe(), stderr = Pipe()
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = stderr

        // Drain both pipes from the moment the child starts: a producer that
        // writes a reply before reading could otherwise fill a pipe and
        // deadlock against us while we are still writing the request.
        // Published under a lock: the bounded wait below can return while a
        // drain is still running.
        let captured = CapturedStderr()
        let drained = DispatchGroup()
        drained.enter(); DispatchQueue.global().async { _ = stdout.fileHandleForReading.readDataToEndOfFile(); drained.leave() }
        drained.enter(); DispatchQueue.global().async { let d = stderr.fileHandleForReading.readDataToEndOfFile(); captured.put(d); drained.leave() }
        try process.run()
        // The request goes out on its own thread for the same reason.
        drained.enter()
        DispatchQueue.global().async {
            defer { drained.leave() }
            try? stdin.fileHandleForWriting.write(contentsOf: requestLine)
            // EOF ends the worker loop, so the process exits on its own.
            try? stdin.fileHandleForWriting.close()
        }
        let exited = DispatchGroup()
        exited.enter(); DispatchQueue.global().async { process.waitUntilExit(); exited.leave() }
        var timedOut = false
        if exited.wait(timeout: .now() + timeout) == .timedOut {
            timedOut = true
            if process.isRunning { process.terminate() }
            _ = exited.wait(timeout: .now() + 5)
        }
        // `timeout` bounded the process, not the pipes. `terminate()` is SIGTERM
        // to the direct child only: if it ignores it, or a descendant still
        // holds the write ends, `readDataToEndOfFile` never sees EOF and an
        // unbounded `drained.wait()` blocks this thread forever. In the Mac test
        // bundle that thread is the main thread, so the whole xctest process
        // hangs with no output and no failure (observed twice, main thread
        // parked in `_dispatch_group_wait_slow` at this line). Bound it,
        // escalate to SIGKILL, and report whatever was drained.
        if drained.wait(timeout: .now() + 10) == .timedOut {
            if process.isRunning { kill(process.processIdentifier, SIGKILL) }
            _ = drained.wait(timeout: .now() + 5)
        }
        if timedOut {
            throw Failure(description: "\(producer.lastPathComponent) did not finish within \(Int(timeout)) s; terminated")
        }
        let bytes = (try? FileManager.default.attributesOfItem(atPath: out.path)[.size] as? Int) ?? 0
        return Outcome(exitCode: process.terminationStatus,
                       stderr: String(decoding: captured.read(), as: UTF8.self), bytes: bytes)
    }

    /// The stderr drain's output, published under a lock (see the bounded wait).
    private final class CapturedStderr: @unchecked Sendable {
        private let lock = NSLock()
        private var data = Data()
        func put(_ d: Data) { lock.lock(); data = d; lock.unlock() }
        func read() -> Data { lock.lock(); defer { lock.unlock() }; return data }
    }
}

extension ShellModel {
    /// The producer Export may run one-shot for a whole-document list: the
    /// attached `flashtex-render`, else a built or bundled one. An arbitrary
    /// attached worker is not used — `--v2` is `flashtex-render`'s flag.
    var wholeDocumentProducer: URL? {
        if let attached = attachedWorkerExecutable, attached.lastPathComponent == "flashtex-render" { return attached }
        return Self.locateRenderPipeline()
    }

    /// The `compile` line the one-shot producer is fed: the same documents,
    /// entry, project root and date `compile()` would send, with none of the
    /// per-request view siblings (no window, no delta, no v1 elision — the
    /// side output ignores them anyway, and the request should say what it
    /// means).
    func wholeDocumentRequestLine() throws -> Data {
        let sendDocuments = documents + project.implicitClosureDocuments().map { RuntimeV1.Document(path: $0.path, text: $0.text) }
        var capabilities = [V2Live.capability]
        if requestedLayoutCapabilities.contains(RenderingV2.imagesCapability) { capabilities.append(RenderingV2.imagesCapability) }
        if requestedLayoutCapabilities.contains(RenderingV2.linksCapability) { capabilities.append(RenderingV2.linksCapability) }
        let request = RuntimeV1.CompileRequest(
            projectId: result?.projectId ?? "demo",
            revision: editorRevision,
            entryPath: project.entryPath,
            documents: sendDocuments,
            layoutCapabilities: capabilities,
            displayListBase: nil,
            displayListWindow: nil,
            projectRoot: project.projectRoot?.path,
            date: RuntimeV1.localDate())
        return try RuntimeV1.encodeLine(RuntimeV1.Envelope(protocolVersion: RuntimeV1.protocolVersion,
                                                           id: "mac-export", type: "compile", payload: request))
    }

    /// Why a frame that must be re-rendered cannot be without the render
    /// pipeline: a page window, or a live frame built from a delta.
    static func noProducerReason(window: RenderingV2.Window?) -> String {
        if let window {
            return "Cannot export: this document is too large to send in one reply, so the preview is showing a page window (pages \(window.firstPage)–\(window.firstPage + window.pageCount - 1) of \(window.documentPageCount)). Exporting it needs the render pipeline: attach it with ⌘⇧R, build crates/render-pipeline, or run `flashtex build` on the command line."
        }
        return "Cannot export: the preview was updated incrementally, so there is no complete display list to export. Exporting it needs the render pipeline: attach it with ⌘⇧R, build crates/render-pipeline, or run `flashtex build` on the command line."
    }

    /// The display list `File > Export PDF…` and `File > Print…` should hand to
    /// `flashtex-pdf-exact`, produced if necessary.
    ///
    /// The fast path is the frame already on screen. A windowed frame is an
    /// incomplete view (window proposal §4.1) and is never exported: the whole
    /// document is re-rendered to a private file first. The returned URL is the
    /// caller's to delete when `temporary` is true.
    func exportListURL() async -> Result<WholeDocumentList.Resolved, WholeDocumentList.Refusal> {
        guard let (frame, source) = displayListV2?.retained else {
            return .failure(.init(reason: "Nothing to export: no rendering-v2 display list."))
        }
        // Unwindowed and received as a full list: the line on screen is the
        // whole document already. A frame reconstructed from a
        // `display_list_delta` (every edit after the first full frame, i.e.
        // exactly the unsaved-edit case) has no full line to hand the tool, so
        // it is re-rendered like a windowed one.
        let window = frame.list.window
        if window == nil, !source.isDeltaLine {
            do { return .success(.init(url: try source.listFileURL(), temporary: false)) } catch {
                return .failure(.init(reason: "PDF export: could not write the display list to a file: \(error.localizedDescription)"))
            }
        }
        guard let producer = wholeDocumentProducer else {
            return .failure(.init(reason: Self.noProducerReason(window: window)))
        }
        let requestLine: Data
        do { requestLine = try wholeDocumentRequestLine() } catch {
            return .failure(.init(reason: "PDF export: could not encode the document for the render pipeline: \(error.localizedDescription)"))
        }
        let out = FileManager.default.temporaryDirectory
            .appendingPathComponent("flashtex-whole-document-\(UUID().uuidString).json")
        let images = requestedLayoutCapabilities.contains(RenderingV2.imagesCapability)
        let environment = BundledMetrics.producerEnvironment()
        captureNote = "Rendering all \(window?.documentPageCount ?? frame.list.pages.count) pages…"
        let outcome: WholeDocumentList.Outcome
        do {
            outcome = try await Task.detached(priority: .userInitiated) {
                try WholeDocumentList.render(producer: producer, requestLine: requestLine, out: out,
                                             environment: environment, images: images)
            }.value
        } catch {
            try? FileManager.default.removeItem(at: out)
            return .failure(.init(reason: "Export failed: \(producer.lastPathComponent) could not render the whole document: \(error). `flashtex build` on the command line writes the same PDF."))
        }
        guard outcome.succeeded else {
            try? FileManager.default.removeItem(at: out)
            let why = outcome.stderr.trimmingCharacters(in: .whitespacesAndNewlines)
            return .failure(.init(reason: "Export failed: \(producer.lastPathComponent) produced no display list for this document (exit \(outcome.exitCode))\(why.isEmpty ? "" : ": \(why.prefix(500))"). `flashtex build` on the command line writes the same PDF."))
        }
        return .success(.init(url: out, temporary: true))
    }
}
