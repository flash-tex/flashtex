import Foundation

/// Oversized producer output (lane mac-large-document). Measured with the
/// real compiler (2026-09-15, after e26847c1): a fully-prose document
/// produces ≈45.7 bytes of preview JSON per source byte, but the compiler now
/// bounds its own reply to its 8 MiB transport frame (`MAX_RESULT_BYTES`,
/// crates/compiler/src/protocol.rs), dropping trailing pages with an explicit
/// diagnostic — from ≈180 KB of prose up the reply is a capped ~8.2 MB. So
/// with DEFAULT limits the real compiler's frames stay admissible; an
/// oversized frame still comes from a producer that does not self-bound, or
/// from a helper started with a lower `compiler_max_frame_bytes` (8 MiB
/// default, 128 B..15 MiB configurable; crates/document-runtime). Then the
/// helper answers with ONE small `update {kind: failed, reason: "compiler
/// output malformed, truncated or oversized"}` per pending compile (never a
/// partial preview), kills its compiler session, and acknowledges every later
/// edit durably with `preview_error: "compiler session failed; …"` until a
/// `restart`. The helper's own 16 MiB `MAX_OUTPUT_BYTES`
/// (crates/preview-controller) turns a required reply into `error "response
/// exceeds output limit; …"`; an optional preview over it is refused silently
/// (not reachable with ≤ 15 MiB compiler frames: the measured envelope adds
/// 288 bytes).
///
/// On the direct worker route an unbounded producer's oversized
/// `compile_result` line is refused by `WorkerClient` at
/// `RuntimeV1.maxLineBytes` (protocol violation → worker terminated →
/// relaunch). Without this file the relaunch re-sends the same document and
/// the loop ends only at the relaunch budget (3/min); on the helper route the
/// in-flight edit stays HELD under `holdUntilPreview`, so typing stops being
/// submitted.
///
/// None of the limits are changed here. The shell records the overflow as an
/// `OutputBoundNotice` (status line names the bound and the document size),
/// keeps the last good preview on screen marked stale, releases the pipeline
/// so the next durable edit is still ACKed, and retries only when the document
/// shrank enough (or on an explicit ⌘B). Hooks into the parent-retained files
/// are listed in coordination/mac-large-document.md.
struct OutputBoundNotice: Equatable {
    enum Route: String, Equatable { case helper = "helper", direct = "worker" }
    var route: Route
    /// Editor revision whose compile overflowed.
    var editorRevision: Int
    /// UTF-8 bytes of the documents that were compiled.
    var documentBytes: Int
    /// The bound that was exceeded, in bytes, and its protocol name.
    var boundBytes: Int
    var boundName: String
    /// Size of the refused reply when the route reports it (direct route:
    /// "line of N bytes exceeds …"); the helper does not say.
    var replyBytes: Int?

    static func mib(_ bytes: Int) -> String {
        bytes % (1024 * 1024) == 0 ? "\(bytes / 1024 / 1024) MiB" : String(format: "%.1f MiB", Double(bytes) / 1024 / 1024)
    }
    static func kb(_ bytes: Int) -> String { String(format: "%.0f KB", Double(bytes) / 1024) }

    /// Bottom status line (`workerStatus`).
    var status: String {
        let reply = replyBytes.map { "reply of \(Self.mib($0)) " } ?? "reply "
        return "revision \(editorRevision): \(reply)exceeds the \(route.rawValue)'s \(boundName) (\(Self.mib(boundBytes))) for this \(Self.kb(documentBytes)) document; last preview kept"
    }
    /// The stale banner replacing "compiling…" while the overflow stands.
    var banner: String {
        "editor at revision \(editorRevision) — preview not recompiled: \(Self.kb(documentBytes)) document exceeds the \(boundName) (\(Self.mib(boundBytes))); shrink it or press ⌘B to retry"
    }

    /// Whether a document of `bytes` may be retried automatically. With a
    /// measured reply size the linear ratio decides; otherwise the document
    /// must have shrunk by at least a tenth, so a constant-size document is
    /// never re-sent (no retry loop) and shrinking converges geometrically.
    func allowsRetry(documentBytes bytes: Int) -> Bool {
        guard bytes < documentBytes else { return false }
        if let replyBytes, documentBytes > 0 {
            return Double(bytes) * Double(replyBytes) / Double(documentBytes) < Double(boundBytes)
        }
        return Double(bytes) <= Double(documentBytes) * 0.9
    }
}

enum OutputBounds {
    /// document-runtime's reader failure (crates/document-runtime/src/lib.rs).
    static let helperFailedReason = "compiler output malformed, truncated or oversized"
    /// The runtime's refusal after that failure, echoed as `preview_error`.
    static let helperSessionFailed = "compiler session failed"
    /// The helper's required-reply overflow (crates/preview-controller/src/main.rs).
    static let helperOverflowError = "response exceeds output limit"
    static let compilerFrameBoundName = "compiler frame bound (compiler_max_frame_bytes)"
    static let helperOutputBoundName = "helper output bound (16 MiB MAX_OUTPUT_BYTES)"
    static let workerLineBoundName = "worker line limit (RuntimeV1.maxLineBytes)"

    /// Parses `WorkerClient`/`PreviewControllerClient` violations of the form
    /// "line|frame of N bytes exceeds the M-byte limit" or "unterminated
    /// line|frame exceeds the M-byte limit".
    static func parseViolation(_ message: String) -> (replyBytes: Int?, boundBytes: Int)? {
        guard let r = message.range(of: "exceeds the "), let end = message[r.upperBound...].range(of: "-byte limit"),
              let bound = Int(message[r.upperBound..<end.lowerBound]) else { return nil }
        var reply: Int?
        if let of = message.range(of: " of "), let b = message[of.upperBound...].range(of: " bytes") {
            reply = Int(message[of.upperBound..<b.lowerBound])
        }
        return (reply, bound)
    }
}

/// Per-model side state (the model's stored properties are parent-retained).
@Observable final class OutputBoundState {
    var notice: OutputBoundNotice?
    /// From the helper's `ready`.
    var compilerMaxFrameBytes = 8 * 1024 * 1024
    var helperMaxOutputBytes = PreviewControllerClient.maxFrameBytes
    /// Set while a `restart` after an overflow is pending (one at a time).
    var retryInFlight = false
    var releasedInFlightCount = 0
}

private final class OutputBoundRegistry {
    static let shared = OutputBoundRegistry()
    private let table = NSMapTable<ShellModel, OutputBoundState>(keyOptions: .weakMemory, valueOptions: .strongMemory)
    func state(for model: ShellModel) -> OutputBoundState {
        if let s = table.object(forKey: model) { return s }
        let s = OutputBoundState()
        table.setObject(s, forKey: model)
        return s
    }
}

extension ShellModel {
    var outputBounds: OutputBoundState { OutputBoundRegistry.shared.state(for: self) }
    var outputBound: OutputBoundNotice? { outputBounds.notice }

    private var activeDocumentBytes: Int { documents.reduce(0) { $0 + $1.text.utf8.count } }

    // MARK: helper route hooks (ShellModel+Controller.swift)

    /// `.ready`: remembers the advertised bounds.
    func outputBoundNoteReady(compilerMaxFrameBytes: Int, helperMaxOutputBytes: Int) {
        if compilerMaxFrameBytes > 0 { outputBounds.compilerMaxFrameBytes = compilerMaxFrameBytes }
        if helperMaxOutputBytes > 0 { outputBounds.helperMaxOutputBytes = helperMaxOutputBytes }
        outputBounds.retryInFlight = false
    }

    /// `.update(kind:payload:)`: a `failed` compile for an oversized reply.
    /// Returns true when handled (the caller logs nothing more).
    @discardableResult
    func outputBoundHandleControllerUpdate(kind: String, payload: PreviewControllerClient.JSONObject) -> Bool {
        guard kind == "failed", let reason = payload["reason"] as? String, reason.hasPrefix(OutputBounds.helperFailedReason) else { return false }
        let inFlight = controllerState.inFlight
        let bytes = inFlight.map { f in documents.reduce(0) { $0 + ($1.path == f.path ? f.text.utf8.count : $1.text.utf8.count) } } ?? activeDocumentBytes
        let notice = OutputBoundNotice(route: .helper, editorRevision: inFlight?.editorRevision ?? editorRevision, documentBytes: bytes,
                                       boundBytes: outputBounds.compilerMaxFrameBytes, boundName: OutputBounds.compilerFrameBoundName, replyBytes: nil)
        record(notice, requestID: payload["request_id"] as? String ?? "-")
        outputBoundReleaseHelperInFlight()
        return true
    }

    /// `.error(id: nil, …)`: the helper refused a required reply for size.
    @discardableResult
    func outputBoundHandleControllerError(id: String?, message: String) -> Bool {
        guard message.hasPrefix(OutputBounds.helperOverflowError) else { return false }
        let notice = OutputBoundNotice(route: .helper, editorRevision: controllerState.inFlight?.editorRevision ?? editorRevision,
                                       documentBytes: activeDocumentBytes, boundBytes: outputBounds.helperMaxOutputBytes,
                                       boundName: OutputBounds.helperOutputBoundName, replyBytes: nil)
        record(notice, requestID: id ?? "-")
        if let inFlight = controllerState.inFlight, id == nil || inFlight.id == id { outputBoundReleaseHelperInFlight() }
        return true
    }

    /// The edit result carried `preview_error` because the compiler session is
    /// gone after an overflow: retry (helper `restart`, which recompiles the
    /// current source) once the document shrank enough. Returns true when a
    /// restart was sent.
    @discardableResult
    func outputBoundHandlePreviewError(_ error: String) -> Bool {
        guard let notice = outputBounds.notice, notice.route == .helper, error.hasPrefix(OutputBounds.helperSessionFailed) else { return false }
        return outputBoundRetryHelper(force: false, documentBytes: activeDocumentBytes)
    }

    /// A preview arrived: the overflow is over.
    func outputBoundNotePreviewApplied() {
        outputBounds.notice = nil
        outputBounds.retryInFlight = false
    }

    /// Explicit ⌘B on the helper route: restart regardless of size.
    /// Returns true when the restart was sent (the caller skips its compile).
    func outputBoundExplicitRetry() -> Bool {
        guard outputBounds.notice != nil else { return false }
        if controllerAttached { return outputBoundRetryHelper(force: true, documentBytes: activeDocumentBytes) }
        outputBounds.notice = nil
        return false
    }

    private func outputBoundRetryHelper(force: Bool, documentBytes: Int) -> Bool {
        guard let notice = outputBounds.notice, let controller, controller.isRunning, !outputBounds.retryInFlight else { return false }
        guard force || notice.allowsRetry(documentBytes: documentBytes) else { return false }
        guard (try? controller.restart()) != nil else { return false }
        outputBounds.retryInFlight = true
        log("output bound: restarting the helper's compiler for a \(OutputBoundNotice.kb(documentBytes)) document (overflowed at \(OutputBoundNotice.kb(notice.documentBytes))\(force ? ", explicit retry" : ""))")
        workerStatus = "retrying revision \(editorRevision) (\(OutputBoundNotice.kb(documentBytes)) document) after the \(notice.boundName) overflow…"
        return true
    }

    /// Releases the held edit so typing keeps being submitted; the queued
    /// buffer is sent and comes back durable (with `preview_error`).
    private func outputBoundReleaseHelperInFlight() {
        guard controllerState.inFlight != nil else { return }
        controllerState.hybridRelease?.cancel()
        controllerState.hybridRelease = nil
        controllerState.inFlight = nil
        inFlightRevision = nil
        outputBounds.releasedInFlightCount += 1
        if controllerState.queued {
            controllerState.queued = false
            controllerSubmitEdit()
        }
    }

    // MARK: direct route hooks (ShellModel.swift)

    /// `handle(.protocolViolation)`: an oversized `compile_result` line.
    @discardableResult
    func outputBoundHandleWorkerViolation(_ message: String) -> Bool {
        guard let (reply, bound) = OutputBounds.parseViolation(message) else { return false }
        let notice = OutputBoundNotice(route: .direct, editorRevision: inFlightRevision ?? editorRevision, documentBytes: activeDocumentBytes,
                                       boundBytes: bound, boundName: OutputBounds.workerLineBoundName, replyBytes: reply)
        record(notice, requestID: "-")
        return true
    }

    /// `compile()` on the direct route: true when the document is not smaller
    /// than the one that overflowed (the relaunch's auto-compile must not
    /// re-send it). The status line keeps naming the bound.
    var outputBoundBlocksCompile: Bool {
        guard let notice = outputBounds.notice, notice.route == .direct else { return false }
        if notice.allowsRetry(documentBytes: activeDocumentBytes) { return false }
        workerStatus = notice.status
        return true
    }

    private func record(_ notice: OutputBoundNotice, requestID: String) {
        outputBounds.notice = notice
        outputBounds.retryInFlight = false
        workerStatus = notice.status
        if controllerAttached { controllerStatus = notice.status }
        log("output bound (\(requestID)): \(notice.status)")
    }
}
