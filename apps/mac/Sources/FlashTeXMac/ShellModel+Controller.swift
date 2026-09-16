import AppKit
import FlashTeXProtocol

/// The durable helper route: `flashtex-preview-controller` owns the edit
/// ledger, the lexical index and the original compiler (crates/preview-
/// controller, STDIO.md). The shell keeps its in-memory buffer authoritative
/// for typing; every edit is submitted as `edit {expected_revision,
/// expected_sha256, text}` (one in flight, newest buffer coalesced), the
/// helper answers with the durable document, and previews arrive as
/// asynchronous `update {kind: preview, source_versions, result}` frames that
/// are mapped back to the editor revision they were compiled from and
/// refused when a newer edit was already submitted.
///
/// Measured cost of this route (M1 Max, demo.tex): edit → durable result
/// 11–16 ms (fsync), edit → preview update 20–30 ms, versus 2 ms for the
/// direct worker; see tools/typing-bench for the native keystroke → paint
/// numbers of each route.
struct ControllerState {
    /// Durable revision and SHA-256 per document path, from the last `document`/`edit` result.
    var durable: [String: (revision: Int, sha256: String)] = [:]
    /// Editor revision that produced each durable revision (per path).
    var editorRevisionByDurable: [String: [Int: Int]] = [:]
    /// Text per durable revision (per path), for stale-diagnostic rebasing;
    /// pruned to the last few revisions.
    var textByDurable: [String: [Int: String]] = [:]
    /// The edit in flight, if any. It stays in flight until the PREVIEW for its
    /// durable revision arrived (not merely the durable receipt): the helper
    /// publishes only previews matching its current source, so submitting a
    /// newer edit while one is compiling discards that preview — under
    /// continuous typing nothing would ever paint (measured: 1.3 s gaps).
    /// `admitted` is the compile the helper admitted for this edit (its reply's
    /// `compile_request_id`/`compile_revision`, AdmissionCorrelation.swift);
    /// nil until the reply, or for a helper without the pair (numeric fallback).
    var inFlight: (id: String, path: String, editorRevision: Int, sentAt: Date, text: String, durableRevision: Int?, admitted: ControllerCompileAdmission?)?
    /// Newest buffer changed while an edit was in flight.
    var queued = false
    /// How the next edit is released behind the one in flight
    /// (ControllerRelease.swift); read from the environment once per attach.
    var releasePolicy = ControllerReleasePolicy.fromEnvironment()
    /// Last observed edit → preview round trip, the hybrid bound's input.
    var lastEditToPreviewMs: Double?
    /// When the edit that produced each durable revision was sent (per path);
    /// pruned with `textByDurable`.
    var sentAtByDurable: [String: [Int: Date]] = [:]
    /// The scheduled hybrid bound check, if any.
    var hybridRelease: DispatchWorkItem?
    var ready = false
    var compilerError: String?
    var lastPreviewRequestID: String?
    /// Request ids of `export`/`file_status` calls awaiting their reply.
    var awaiting: [String: (Result<[String: Any], ControllerError>) -> Void] = [:]
}

struct ControllerError: Error, Equatable { var message: String }

extension ShellModel {
    // MARK: attach / detach

    /// Where the helper's private ledgers live: `FLASHTEX_CONTROLLER_LEDGER_ROOT`
    /// or Application Support/FlashTeX/ledgers/<project root hash>.
    static func controllerLedgerRoot(for projectRoot: URL) -> URL {
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_CONTROLLER_LEDGER_ROOT"], !env.isEmpty {
            return URL(fileURLWithPath: env)
        }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        let key = SourceDigest.sha256Hex(projectRoot.standardizedFileURL.path).prefix(16)
        return base.appendingPathComponent("FlashTeX/ledgers/\(key)")
    }

    /// Attaches the helper for the current project. A saved document's
    /// directory is the rooted project; an unsaved buffer is written to a
    /// session temporary project so the helper can import it.
    func attachController(at url: URL) {
        detachWorker()
        detachController()
        let projectRoot: URL
        let entryPath: String
        // The entry document is the first member, not necessarily the one
        // being edited (ProjectDocuments keeps it first).
        let entry = documents.first ?? .init(path: activePath, text: activeText)
        if let documentURL, documentURL.lastPathComponent == entry.path {
            // The helper names documents by their rooted path; the project's
            // paths and the editor's must agree (a seeded buffer whose file name
            // differs from the entry path uses a session project instead).
            projectRoot = documentURL.deletingLastPathComponent()
            entryPath = documentURL.lastPathComponent
        } else {
            projectRoot = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-project-\(UUID().uuidString)")
            try? FileManager.default.createDirectory(at: projectRoot, withIntermediateDirectories: true)
            entryPath = entry.path
            try? entry.text.write(to: projectRoot.appendingPathComponent(entryPath), atomically: true, encoding: .utf8)
        }
        let ledgerRoot = Self.controllerLedgerRoot(for: projectRoot)
        try? FileManager.default.createDirectory(at: ledgerRoot, withIntermediateDirectories: true)
        var config = PreviewControllerClient.Config(sessionID: "mac-\(UUID().uuidString)", projectID: projectId,
                                                    entryPath: entryPath, projectRoot: projectRoot,
                                                    privateLedgerRoot: ledgerRoot, compilerPath: Self.locateCompiler())
        if let s = ProcessInfo.processInfo.environment["FLASHTEX_CONTROLLER_MAX_FRAME_BYTES"], let n = Int(s) {
            config.compilerMaxFrameBytes = n
        }
        config.bibliographyPaths = documentKinds.startupBibliographyPaths(projectRoot: projectRoot, privateLedgerRoot: ledgerRoot, projectID: projectId, entry: entryPath) // DocumentKinds.swift: persisted explicit declarations, never inferred
        controllerState = ControllerState()
        controllerLaunchURL = url
        controllerRelaunchTimes = [] // an explicit attach starts a fresh relaunch budget
        do {
            controller = try PreviewControllerClient(executable: url, config: config) { [weak self] event in
                self?.handleController(event)
            }
            controllerStatus = "attached: \(url.lastPathComponent) (waiting for ready)"
            workerStatus = "attached: \(url.lastPathComponent)"
            PreviewFonts.producerFace = PreviewFonts.face(forProducer: config.compilerPath?.lastPathComponent)
            log("launched \(url.path) for \(projectRoot.path) (ledger \(ledgerRoot.path))")
        } catch {
            controllerStatus = "launch failed: \(error.localizedDescription)"
            workerStatus = controllerStatus
        }
    }

    func detachController() {
        controllerRelaunchWork?.cancel()
        controllerRelaunchWork = nil
        controllerLaunchURL = nil
        guard let controller else { return }
        controller.close()
        controller.terminate()
        self.controller = nil
        // Replies parked in `awaiting` (save, file_status, project requests)
        // never come now: resume them with a failure instead of dropping their
        // continuations; the completion query's ids mean nothing on the next client.
        for (_, waiter) in controllerState.awaiting { waiter(.failure(.init(message: "helper exited (detached)"))) }
        completionFetcher.discard()
        controllerState = ControllerState()
        historicalInvalidate(reason: "close")
        displayCandidatesInvalidate(reason: "close") // ShellModel+DisplayCandidates.swift
        inFlightRevision = nil
        controllerStatus = "no preview controller attached"
        if previewSource != .fixture { workerStatus = "no worker attached" }
    }

    // MARK: edits

    /// Submits the active buffer as one durable edit (newest buffer coalesced
    /// behind the edit in flight). Never blocks; the reply carries the durable
    /// revision and, separately, any preview error.
    func controllerSubmitEdit() {
        guard let controller, controller.isRunning, controllerState.ready else { return }
        if controllerState.inFlight != nil {
            controllerState.queued = true
            controllerHybridCheck() // hybrid: release now if the in-flight edit is past its bound
            return
        }
        guard let durable = controllerState.durable[activePath] else { return }
        let text = activeText
        if let last = controllerState.textByDurable[activePath]?[durable.revision], last.sameBytes(as: text) {
            return // buffer already durable at this revision
        }
        do {
            if TypingBench.isBenchActive { FlashTeXLog.write("compile: sending revision \(editorRevision) at \(MonotonicClock.nowNs())") }
            let id = try controller.edit(path: activePath, expectedRevision: durable.revision,
                                         expectedSHA256: durable.sha256, text: text,
                                         sourceBindingToken: historicalToken(forEditorRevision: editorRevision))
            controllerState.inFlight = (id, activePath, editorRevision, Date(), text, nil, nil)
            controllerState.queued = false
            inFlightRevision = editorRevision
        } catch {
            controllerStatus = "edit failed to send: \(error.localizedDescription)"
            log(controllerStatus)
        }
    }

    /// Explicit compile (⌘B): submits pending text first, else asks for a compile.
    func controllerCompile() {
        guard let controller, controller.isRunning, controllerState.ready else { return }
        if outputBoundExplicitRetry() { return } // ⌘B after an overflow: restart the compiler regardless of size
        if let durable = controllerState.durable[activePath],
           !(controllerState.textByDurable[activePath]?[durable.revision]?.sameBytes(as: activeText) ?? false) {
            controllerSubmitEdit()
            return
        }
        _ = try? controller.compile(sourceBindingToken: historicalToken(forEditorRevision: editorRevision))
    }

    // MARK: events

    func handleController(_ event: PreviewControllerClient.Event) {
        guard let controller else { return }
        switch event {
        case .ready(let compilerError, let maxFrame, let maxOut):
            controllerState.ready = true
            controllerState.compilerError = compilerError
            outputBoundNoteReady(compilerMaxFrameBytes: maxFrame, helperMaxOutputBytes: maxOut) // ShellModel+OutputBounds.swift
            controllerStatus = "ready: compiler frames ≤ \(maxFrame / 1024 / 1024) MiB, helper output ≤ \(maxOut / 1024 / 1024) MiB"
                + (compilerError.map { "; compiler unavailable: \($0)" } ?? "")
            log("controller " + controllerStatus)
            // Negotiate the historical side channel first (its reply precedes
            // the document's, so the first edit already carries a token), opt
            // into the negotiated primitives the preview draws, then learn the
            // durable document so edits can name the revision they expect.
            historicalNegotiate()
            // display-list-v2 is enrolled by the helper itself through the
            // display-candidate opt-in (ShellModel+DisplayCandidates.swift), never by configure_layout.
            let layout = displayCandidatesConfigureLayoutCapabilities(requestedLayoutCapabilities)
            if !layout.isEmpty {
                _ = try? controller.configureLayout(capabilities: layout)
            }
            displayCandidatesNegotiate() // after configure_layout: the helper enrolls display-list-v2 into that set
            _ = try? controller.document(path: activePath)
        case .result(let id, let payload):
            if let waiter = controllerState.awaiting.removeValue(forKey: id) { waiter(.success(payload)); return }
            if historicalHandle(resultID: id, payload: payload) { return }
            if displayCandidatesHandle(resultID: id, payload: payload) { return }
            switch completionFetcher.handle(resultID: id, payload: payload) {
            case .notMine: break
            case .pending: return
            case .complete(let metadata): completionMetadata = metadata; return
            case .refused(let why): log("completion metadata refused: \(why)"); return
            }
            if let doc = payload["document"] as? [String: Any] {
                applyDurableDocument(doc, requestID: id, payload: payload)
            } else if payload["submitted"] != nil || payload["closed"] != nil || payload["configured"] != nil {
                break
            } else {
                log("controller result \(id): \(payload.keys.sorted().joined(separator: ","))")
            }
        case .error(let id, let message):
            if let id, let waiter = controllerState.awaiting.removeValue(forKey: id) { waiter(.failure(.init(message: message))); return }
            if historicalHandle(errorID: id, message: message) { return }
            if displayCandidatesHandle(errorID: id, message: message) { return }
            if case .refused(let why) = completionFetcher.handle(errorID: id, message: message) {
                log("completion metadata refused: \(why)")
                break
            }
            if outputBoundHandleControllerError(id: id, message: message) { break } // oversized required reply (ShellModel+OutputBounds.swift)
            if let inFlight = controllerState.inFlight, inFlight.id == id {
                controllerState.inFlight = nil
                inFlightRevision = nil
                log("controller refused edit \(id): \(message)")
                controllerStatus = "edit refused: \(message)"
                if message.hasPrefix("document_conflict") {
                    // Our durable snapshot is stale (restart, external writer): re-read it.
                    _ = try? controller.document(path: inFlight.path)
                    controllerState.queued = true
                }
            } else {
                log("controller error \(id ?? "-"): \(message)")
                controllerStatus = "helper error: \(message)"
            }
        case .preview(let update):
            applyControllerPreview(update)
        case .completedSnapshot(let frame):
            historicalReceive(frame)
        case .displayCandidate(let candidate):
            handleDisplayCandidate(candidate) // ShellModel+DisplayCandidates.swift
        case .update(let kind, let payload):
            // stale / discarded previews name the request they replaced; nothing
            // to paint. If it was the compile our in-flight edit waits for, the
            // helper will not send its preview: release the pipeline. Matched by
            // the admitted request id (AdmissionCorrelation.swift); `superseded`
            // rebinds the wait to the superseding request.
            if kind == "stale" || kind == "discarded" || kind == "superseded" {
                if controllerAdmissionReleases(kind: kind, payload: payload) {
                    controllerReleaseInFlight()
                }
            } else if outputBoundHandleControllerUpdate(kind: kind, payload: payload) {
                // `failed` for an oversized compiler reply: status names the bound, the held edit is released (ShellModel+OutputBounds.swift)
            } else if kind == "failed" || kind == "cancelled" {
                // The compiler session is gone (or the compile was cancelled): no
                // preview follows for any admitted compile, ours included. An
                // in-flight edit that is already durable would otherwise wait
                // forever and every later keystroke would queue behind it. An
                // edit not yet durable is released by its own reply's
                // `preview_error` (applyDurableDocument). The runtime reports
                // every admitted compile separately, so the admitted id is matched.
                let detail = (payload["reason"] as? String).map { ": \($0)" } ?? ""
                log("controller \(kind) compile \(payload["request_id"] as? String ?? "-")\(detail)")
                controllerStatus = "preview \(kind)\(detail)"
                if controllerAdmissionReleases(kind: kind, payload: payload) {
                    controllerReleaseInFlight()
                }
            } else {
                log("controller update \(kind): \(payload.keys.sorted().joined(separator: ","))")
            }
        case .protocolViolation(let message):
            controllerStatus = "protocol violation: \(message)"
            workerStatus = controllerStatus
            log("controller protocol violation: \(message)")
        case .stderr(let text):
            log("controller: " + text.trimmingCharacters(in: .whitespacesAndNewlines))
        case .exited(let code):
            let wasAttached = controller != nil // an explicit detach already cleared it: never relaunch
            // Carry the helper's own last words. `helper exited (1)` alone is
            // not diagnosable -- it is what CI reported for a real failure --
            // and everything that branches on this message uses hasPrefix, so
            // a trailing reason is safe to append.
            // `controller` here is the non-optional binding from the
            // `guard let controller` at the top of this function, not the
            // optional property — hence no optional chaining.
            let reason = Self.exitReason(code: code, stderr: controller.recentStderr)
            for (_, waiter) in controllerState.awaiting { waiter(.failure(.init(message: reason))) }
            controllerStatus = reason
            workerStatus = "worker exited (\(code))"
            log("controller exited with status \(code)" + (controller.recentStderr.map { "; stderr: " + $0 } ?? "; no stderr"))
            self.controller = nil
            completionFetcher.discard() // its ids restart at pc-1 on the relaunched client
            controllerState = ControllerState()
            inFlightRevision = nil
            historicalInvalidate(reason: "helper exited")
            displayCandidatesInvalidate(reason: "helper exited") // ShellModel+DisplayCandidates.swift
            if wasAttached { scheduleControllerRelaunch(afterExit: code) }
        }
    }

    /// `helper exited (1): thread 'main' panicked at ...` -- the status code
    /// with the helper's own last stderr line, bounded so a status string
    /// stays a status string. Callers match on the `helper exited` prefix.
    static func exitReason(code: Int32, stderr: String?, limit: Int = 200) -> String {
        let base = "helper exited (\(code))"
        guard let last = stderr?.split(separator: "\n").last(where: { !$0.trimmingCharacters(in: .whitespaces).isEmpty }) else {
            return base
        }
        let line = last.trimmingCharacters(in: .whitespaces)
        return base + ": " + (line.count > limit ? String(line.prefix(limit)) + "…" : line)
    }

    /// An abnormal helper exit relaunches the same executable for the same
    /// project after a short backoff, at most `maxWorkerRelaunches` times per
    /// minute (the same policy as the direct worker). The helper reopens its
    /// ledger, so the durable document comes back through the normal `document`
    /// reply and the buffer is resubmitted only if it differs from it. A clean
    /// exit (0) or an explicit detach never relaunches.
    private func scheduleControllerRelaunch(afterExit code: Int32) {
        guard code != 0, let url = controllerLaunchURL else { return }
        let now = Date()
        controllerRelaunchTimes = controllerRelaunchTimes.filter { now.timeIntervalSince($0) < 60 }
        guard controllerRelaunchTimes.count < Self.maxWorkerRelaunches else {
            controllerStatus = "helper exited (\(code)); not relaunched: \(Self.maxWorkerRelaunches) relaunches in the last minute — relaunch FlashTeX to retry"
            workerStatus = controllerStatus
            log("controller relaunch limit reached")
            return
        }
        let delay = Self.workerRelaunchDelays[min(controllerRelaunchTimes.count, Self.workerRelaunchDelays.count - 1)]
        controllerRelaunchTimes.append(now)
        controllerStatus = String(format: "helper exited (%d); relaunching in %.1f s", code, delay)
        workerStatus = controllerStatus
        log("relaunching \(url.lastPathComponent) in \(delay) s (attempt \(controllerRelaunchTimes.count))")
        let item = DispatchWorkItem { [weak self] in
            guard let self, self.controller == nil, self.controllerLaunchURL == url else { return }
            let times = self.controllerRelaunchTimes
            self.attachController(at: url) // resets the relaunch bookkeeping…
            self.controllerRelaunchTimes = times // …which must survive so the bound holds
            self.controllerLaunchURL = url
            self.controllerRelaunchCount += 1
            if self.controller != nil { self.log("relaunched \(url.lastPathComponent)") }
        }
        controllerRelaunchWork = item
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: item)
    }

    /// A `document` or `edit` result: records the durable revision/hash and
    /// maps it to the editor revision it came from, then sends any queued edit.
    private func applyDurableDocument(_ doc: [String: Any], requestID: String, payload: [String: Any]) {
        guard let path = doc["path"] as? String, let revision = doc["revision"] as? Int,
              let sha = doc["source_sha256"] as? String, let text = doc["text"] as? String else {
            log("controller document result \(requestID) is missing fields")
            return
        }
        controllerState.durable[path] = (revision, sha)
        controllerState.textByDurable[path, default: [:]][revision] = text
        if let old = controllerState.textByDurable[path], old.count > 8 {
            for key in old.keys.sorted().dropLast(8) { controllerState.textByDurable[path]?.removeValue(forKey: key) }
        }
        if let inFlight = controllerState.inFlight, inFlight.id == requestID {
            controllerState.editorRevisionByDurable[path, default: [:]][revision] = inFlight.editorRevision
            controllerState.sentAtByDurable[path, default: [:]][revision] = inFlight.sentAt
            if let old = controllerState.sentAtByDurable[path], old.count > 8 {
                for key in old.keys.sorted().dropLast(8) { controllerState.sentAtByDurable[path]?.removeValue(forKey: key) }
            }
            if TypingBench.isBenchActive { FlashTeXLog.write("durable: r\(revision) for revision \(inFlight.editorRevision) at \(MonotonicClock.nowNs())") }
            if let e = payload["preview_error"] as? String {
                // Durable, but no preview will follow: release the pipeline now.
                controllerState.inFlight = nil
                controllerStatus = "durable r\(revision); preview error: \(e)"
                log("controller preview_error: \(e)")
                outputBoundHandlePreviewError(e) // restarts the compiler once the document shrank after an overflow
            } else {
                controllerState.inFlight?.durableRevision = revision
                controllerState.inFlight?.admitted = ControllerCompileAdmission.from(editResult: payload) // nil for an older helper or a history result
                if let ms = payload["save_and_submit_ms"] as? Double { controllerStatus = String(format: "durable r%d in %.1f ms", revision, ms) }
                switch controllerState.releasePolicy {
                case .hybrid:
                    // Hold for the preview, but not past the bound (ControllerRelease.swift).
                    controllerScheduleHybridRelease()
                case .holdUntilPreview:
                    // Negotiated historical mode (HistoricalPreview.swift): every
                    // keystroke is its own durable edit and completed older compiles
                    // arrive as labelled historical frames, so the pipeline does not
                    // hold the next edit for this one's preview.
                    if historicalNegotiated { controllerState.inFlight = nil }
                }
            }
        } else {
            // Initial `document` (or a re-read after a conflict): the ledger is
            // authoritative for durability, the buffer for what the user sees.
            // A differing ledger text is kept in history; the buffer is submitted.
            controllerState.editorRevisionByDurable[path, default: [:]][revision] = editorRevision
            if path == activePath, !text.sameBytes(as: activeText) {
                log("controller durable r\(revision) differs from the buffer (\(text.utf8.count) vs \(activeText.utf8.count) bytes); submitting the buffer")
                controllerState.queued = true
            } else if path == activePath, result == nil || previewIsStale {
                _ = try? controller?.compile()
            }
        }
        if controllerState.inFlight == nil, controllerState.queued || (path == activePath && !text.sameBytes(as: activeText)) {
            controllerState.queued = false
            controllerSubmitEdit()
        } else if controllerState.inFlight == nil {
            inFlightRevision = nil
        }
    }

    /// A durable `history` result (undo/redo/apply_group; EditHistoryPanel.swift)
    /// adopted like the reply to an edit: the durable revision/text are
    /// recorded FIRST so nothing is resubmitted, the buffer takes the durable
    /// text when it has not moved since the command was issued (else the
    /// normal document path resubmits the newer buffer on top), and the result
    /// is then recorded as the in-flight edit for `requestID` so the helper's
    /// follow-up preview binds to the new editor revision.
    func controllerAdoptHistoryResult(_ doc: [String: Any], requestID: String, payload: [String: Any], issuedAtEditorRevision: Int?) {
        guard controller != nil, let path = doc["path"] as? String, let revision = doc["revision"] as? Int,
              let sha = doc["source_sha256"] as? String, let text = doc["text"] as? String else {
            log("controller history result \(requestID) is missing fields")
            return
        }
        controllerState.durable[path] = (revision, sha)
        controllerState.textByDurable[path, default: [:]][revision] = text
        let bufferUnchanged = issuedAtEditorRevision.map { $0 == editorRevision } ?? true
        if path == activePath, bufferUnchanged, !text.sameBytes(as: activeText) {
            updateActiveText(text) // bumps editorRevision; controllerSubmitEdit sees the text is already durable
        }
        if path == activePath, text.sameBytes(as: activeText), controllerState.inFlight == nil {
            controllerState.inFlight = (requestID, path, editorRevision, Date(), text, nil, nil)
            controllerState.queued = false
            inFlightRevision = editorRevision
        }
        applyDurableDocument(doc, requestID: requestID, payload: payload)
    }

    /// Hybrid policy: arms a check for when the in-flight (durable) edit has
    /// been in flight for the bound; a keystroke meanwhile checks immediately.
    private func controllerScheduleHybridRelease() {
        guard let inFlight = controllerState.inFlight, inFlight.durableRevision != nil else { return }
        controllerState.hybridRelease?.cancel()
        let elapsed = Date().timeIntervalSince(inFlight.sentAt) * 1000
        let delay = ReleaseBound.remainingMs(inFlightMs: elapsed, lastEditToPreviewMs: controllerState.lastEditToPreviewMs)
        let item = DispatchWorkItem { [weak self] in self?.controllerHybridCheck() }
        controllerState.hybridRelease = item
        DispatchQueue.main.asyncAfter(deadline: .now() + delay / 1000, execute: item)
    }

    /// Hybrid policy: releases the in-flight edit when it is durable, newer
    /// text is waiting and the bound elapsed. Logged under the bench.
    func controllerHybridCheck() {
        guard let inFlight = controllerState.inFlight else { return }
        let elapsed = Date().timeIntervalSince(inFlight.sentAt) * 1000
        guard ReleaseBound.shouldRelease(policy: controllerState.releasePolicy, durable: inFlight.durableRevision != nil,
                                         queued: controllerState.queued, inFlightMs: elapsed,
                                         lastEditToPreviewMs: controllerState.lastEditToPreviewMs) else { return }
        let bound = ReleaseBound.boundMs(lastEditToPreviewMs: controllerState.lastEditToPreviewMs)
        log(String(format: "controller hybrid release: revision %d durable r%d after %.1f ms (bound %.0f ms)", inFlight.editorRevision, inFlight.durableRevision ?? 0, elapsed, bound))
        if TypingBench.isBenchActive { FlashTeXLog.write(String(format: "release: hybrid revision %d after %.1f ms (bound %.0f ms) at %llu", inFlight.editorRevision, elapsed, bound, MonotonicClock.nowNs())) }
        controllerReleaseInFlight()
    }

    /// The preview for the in-flight edit arrived (or was dropped): release the
    /// pipeline and send the newest buffer if it changed meanwhile.
    private func controllerReleaseInFlight() {
        controllerState.hybridRelease?.cancel()
        controllerState.hybridRelease = nil
        controllerState.inFlight = nil
        inFlightRevision = nil
        if controllerState.queued {
            controllerState.queued = false
            controllerSubmitEdit()
        }
    }

    /// A preview compiled from exact durable versions: bind it to the editor
    /// revision that produced those versions and refuse anything older than the
    /// preview already shown.
    private func applyControllerPreview(_ update: PreviewControllerClient.PreviewUpdate) {
        let versionForActive = update.sourceVersions[activePath]
        if let rev = versionForActive, let sent = controllerState.sentAtByDurable[activePath]?[rev] {
            controllerState.lastEditToPreviewMs = Date().timeIntervalSince(sent) * 1000 // the hybrid bound's input
        }
        // The in-flight edit waits for its admitted compile's outcome, matched by
        // request id (AdmissionCorrelation.swift; the numeric fallback names the
        // in-flight edit's own path: after a document switch the active path's
        // version says nothing about it).
        if controllerAdmissionReleases(preview: update) {
            // Held briefly for this request's display-candidate sibling when that route is
            // negotiated (ShellModel+DisplayCandidates.swift); otherwise released now.
            displayCandidatesAfterSibling(of: update.requestID, acceptedLayout: update.result.payload.layoutCapabilities ?? [], holdsRelease: true) { [weak self] in
                self?.controllerReleaseInFlight()
            }
        }
        guard let durableRevision = versionForActive,
              let editorRev = controllerState.editorRevisionByDurable[activePath]?[durableRevision] else {
            log("ignored controller preview \(update.requestID): versions \(update.sourceVersions) unknown to this session")
            return
        }
        if let current = result, previewSource != .fixture, editorRev < current.revision {
            log("ignored stale controller preview \(update.requestID) (editor revision \(editorRev) < \(current.revision))")
            return
        }
        var incoming = update.result.payload
        incoming.revision = editorRev
        // display-list-v2 accepted by the producer is expected while the helper enrolls it for candidates (ShellModel+DisplayCandidates.swift).
        let requested = displayCandidatesLayoutRequested(requestedLayoutCapabilities)
        if let violation = LayoutNegotiation.violation(in: incoming, requested: requested) {
            log("rejected controller preview \(update.requestID): \(violation)")
            controllerStatus = "protocol violation: \(violation)"
            return
        }
        result = incoming
        resultID = update.result.id
        previewSource = .worker("flashtex-preview-controller")
        outputBoundNotePreviewApplied()
        historicalNoteCurrentPreview(compileRevision: update.compileRevision)
        displayCandidatesNotePreview(update, editorRevision: editorRev) // the only request a candidate may correlate to
        bindLayout(of: incoming, requested: requested)
        if !update.missingLayoutCapabilities.isEmpty {
            log("controller: compiler declined layout capabilities \(update.missingLayoutCapabilities)")
        }
        var compiled: [String: String] = [:]
        for (path, rev) in update.sourceVersions { if let t = controllerState.textByDurable[path]?[rev] { compiled[path] = t } }
        controllerState.lastPreviewRequestID = update.requestID
        setCompiledDocuments(compiled)
        retainMarksAfterResultBound() // ShellModel+DiagnosticRetention.swift
        let ms = update.controllerTotalMs ?? update.runtimeTotalMs ?? 0
        TypingBench.shared.noteCompile(revision: editorRev, ms: ms)
        if TypingBench.isBenchActive { FlashTeXLog.write("compile: applied revision \(editorRev) at \(MonotonicClock.nowNs())") }
        recordLatency(ms)
        workerStatus = String(format: "revision %d: %@, %d diagnostics in %.0f ms (durable r%d)", editorRev,
                              incoming.status.rawValue, incoming.diagnostics.count, ms, durableRevision)
        selection = nil
        // Refresh the completion vocabulary for exactly these source versions
        // (held back briefly while the helper's display-candidate sibling of this
        // request is expected: ShellModel+DisplayCandidates.swift).
        if let controller {
            displayCandidatesAfterSibling(of: update.requestID, acceptedLayout: incoming.layoutCapabilities ?? []) { [weak self] in
                guard let self, self.controller === controller, controller.isRunning else { return }
                self.completionFetcher.request(sourceVersions: update.sourceVersions, editorRevision: editorRev) { try controller.send($0, $1) }
            }
        }
    }
}

// MARK: saving through the durable helper

extension ShellModel {
    /// Saves the active document through the helper's `export`: waits (bounded)
    /// for the buffer to be durable, then writes exactly that durable source
    /// with the mandatory disk expectation (the hash we last saw on disk, or
    /// "must not exist"). Stale source, a changed file, and symlink components
    /// are refused by the helper; a refusal names the disk conflict so the
    /// existing Resolve On-Disk Conflict flow applies. Never blocks the UI.
    func controllerSave(timeout: TimeInterval = 10) async -> DocumentFilesState.SaveResult {
        guard let controller, controller.isRunning, controllerState.ready else { return .failed("preview controller not ready") }
        guard let documentURL else { return .failed("no document URL") }
        let path = activePath
        let text = activeText
        // 1. The durable source must equal the buffer.
        let deadline = Date().addingTimeInterval(timeout)
        controllerSubmitEdit()
        while true {
            if let durable = controllerState.durable[path], controllerState.textByDurable[path]?[durable.revision]?.sameBytes(as: text) == true,
               controllerState.inFlight == nil { break }
            if Date() > deadline { return .failed("buffer did not become durable within \(Int(timeout)) s") }
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
        guard let durable = controllerState.durable[path] else { return .failed("no durable revision") }
        // 2. Export exactly that revision.
        let id: String
        do { id = try controller.export(path: path, expectedRevision: durable.revision, expectedSHA256: durable.sha256, expectedDiskSHA256: baselineSha256) }
        catch { return .failed("export failed to send: \(error.localizedDescription)") }
        let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
            controllerState.awaiting[id] = { cont.resume(returning: $0) }
        }
        switch reply {
        case .success(let payload):
            let sha = payload["sha256"] as? String ?? SourceDigest.sha256Hex(text)
            savedText = text
            files.conflict = nil
            captureNote = "Saved \(documentURL.lastPathComponent) through the preview controller (durable r\(durable.revision))"
            bridgeSourceSaved(url: documentURL, text: text)
            return .saved(sha256: sha)
        case .failure(let e):
            // The rooted save primitive refuses changed/appeared/missing files;
            // the helper relays its message ("save of <path> refused: <Kind>").
            // Only such a refusal is a conflict; anything else is a failure.
            guard let kind = Self.conflictKind(inExportRefusal: e.message) else { return .failed(e.message) }
            let theirs = await controllerFileStatus(path: path)?.diskSHA256
            files.conflict = DocumentConflict(url: documentURL, kind: kind, ours: baselineSha256, theirs: theirs,
                                              size: nil, mtimeUnixMs: nil, viaHelper: true)
            return .conflict(files.conflict!)
        }
    }

    /// Maps the project-files refusal named in an export error to a conflict kind.
    static func conflictKind(inExportRefusal message: String) -> ProjectFilesV1.ConflictKind? {
        guard message.contains("refused:") else { return nil }
        if message.hasSuffix("ModifiedExternally") { return .modifiedExternally }
        if message.hasSuffix("DeletedExternally") { return .deletedExternally }
        if message.hasSuffix("AlreadyExists") { return .alreadyExists }
        if message.hasSuffix("ModifiedDuringSave") { return .modifiedDuringSave }
        return nil
    }

    struct ControllerDiskStatus: Equatable {
        /// `matches_source`, `differs_from_source`, `missing` or `unavailable`.
        var state: String
        var diskSHA256: String?
        var reason: String?
    }

    /// `file_status` through the helper (rereads the disk, never reloads);
    /// nil when the helper cannot answer.
    func controllerFileStatus(path: String) async -> ControllerDiskStatus? {
        guard let controller, controller.isRunning, controllerState.ready else { return nil }
        guard let id = try? controller.fileStatus(path: path) else { return nil }
        let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
            controllerState.awaiting[id] = { cont.resume(returning: $0) }
        }
        guard case .success(let payload) = reply, let disk = payload["disk"] as? [String: Any],
              let state = disk["state"] as? String else { return nil }
        // `matches_source` replies carry `sha256`; the others `disk_sha256`.
        return ControllerDiskStatus(state: state, diskSHA256: (disk["disk_sha256"] ?? disk["sha256"]) as? String, reason: disk["reason"] as? String)
    }
}
