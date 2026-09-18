import AppKit
import UniformTypeIdentifiers
import FlashTeXProtocol

/// Capture-bridge lifecycle for the shell (contract: transfer-v1, bridge commit
/// b5ca96b). The Mac owns UI review, the document transaction and the bridge
/// process: it opens the active document, streams edits, pins destinations,
/// submits captures, converts them, verifies prepared edits and applies each
/// one exactly once through the editor's undo manager.
extension ShellModel {
    var bridgeAttached: Bool { bridge?.running == true }

    func isBridgeCapture(_ captureId: String) -> Bool { bridge?.capture(captureId) != nil }

    /// The most recent capture that can still be converted or reviewed.
    var latestConvertibleCapture: BridgeSession.Capture? {
        bridgeCaptures.last { $0.state == .received || $0.state == .proposed }
    }

    // MARK: attach / detach

    /// Attaches `$FLASHTEX_BRIDGE`, a bundled `flashtex-bridge`, or
    /// `crates/bridge/target/{release,debug}/flashtex-bridge`. With a
    /// conversion provider selected and its key present
    /// (ConversionCredential.swift: Keychain, then the environment) the bridge
    /// runs with that provider's flag and the key in its environment, so
    /// `Edit > Convert Capture` is a live conversion; without one it runs
    /// without any provider and conversion reports `provider_disabled`.
    @discardableResult
    func attachDiscoveredBridge() -> Bool {
        guard let url = BridgeClient.locateBridge() else {
            bridgeStatus = "no built flashtex-bridge found (build crates/bridge or set FLASHTEX_BRIDGE)"
            return false
        }
        let launch = Self.bridgeConversionLaunch()
        attachBridge(executable: url, storeDirectory: BridgeClient.defaultStoreDirectory(),
                     provider: launch.provider, environment: launch.environment)
        return true
    }

    /// The provider the bridge is launched with plus its environment for the
    /// resolved credential (no credential or provider "None": `.none`, secrets
    /// stripped, no key). Pure given its inputs so tests can assert the key
    /// lands only on this child.
    static func bridgeConversionLaunch(environment: [String: String] = ProcessInfo.processInfo.environment,
                                       keychain: any ConversionKeychainStore = SecItemKeychain.shared,
                                       preferences: ConversionPreferences = .shared) -> (provider: ConversionProvider, environment: [String: String], credential: ConversionCredential.Resolution?) {
        let selected = ConversionCredential.provider(environment: environment, preferences: preferences)
        let credential = ConversionCredential.resolve(for: selected, environment: environment, keychain: keychain)
        let provider: ConversionProvider = credential == nil ? .none : selected
        let model = ConversionCredential.model(for: selected, environment: environment, preferences: preferences)
        return (provider, ConversionCredential.bridgeEnvironment(provider: provider, credential: credential, model: model, from: environment), credential)
    }

    /// Where the edit-ledger helper comes from: `$FLASHTEX_EDIT_LEDGER`, a bundled
    /// `flashtex-edit-ledger`, or `crates/edit-ledger/target/{release,debug}/…`.
    /// Without it the bridge still attaches but capture insertion is disabled.
    struct LedgerLaunch { var executable: URL; var arguments: [String] = [] }

    /// Launches `executable arguments... --store <storeDirectory>` plus the
    /// edit-ledger helper for the active document, adopts/aligns the durable
    /// document, runs the contract's restart reconciliation, then opens the
    /// active document on the bridge.
    func attachBridge(executable: URL, arguments: [String] = [], storeDirectory: URL, provider: ConversionProvider = .none,
                      environment: [String: String]? = nil) {
        Task { await attachBridgeAndWait(executable: executable, arguments: arguments, storeDirectory: storeDirectory,
                                         provider: provider, environment: environment) }
    }

    @discardableResult
    /// `ledger` nil with `discoverLedger` true uses the discovered helper; pass
    /// `discoverLedger: false` to attach without any edit ledger (insertion disabled).
    /// `environment` nil inherits the app's environment (as before); the
    /// discovered-bridge path passes `ConversionCredential.bridgeEnvironment`.
    func attachBridgeAndWait(executable: URL, arguments: [String] = [], storeDirectory: URL, provider: ConversionProvider = .none,
                             environment: [String: String]? = nil,
                             ledger: LedgerLaunch? = nil, discoverLedger: Bool = true, ledgerStore: URL? = nil) async -> Bool {
        let ledger = ledger ?? (discoverLedger ? EditLedgerClient.locate().map { LedgerLaunch(executable: $0) } : nil)
        let session: BridgeSession
        do {
            session = try BridgeSession(executable: executable, arguments: arguments, storeDirectory: storeDirectory,
                                        provider: provider, environment: environment, projectId: projectId)
        } catch {
            bridgeStatus = "bridge launch failed: \(error.localizedDescription)"
            return false
        }
        setBridge(session)
        // Callbacks and continuations of a session that is no longer `bridge` (detached,
        // replaced) must never touch the model: a queued exit event from an old process
        // would otherwise overwrite the new session's status.
        session.onChange = { [weak self, weak session] in
            guard let self, let session, self.bridge === session else { return }
            self.bridgeStatus = session.status + (session.ledgerError.map { " · " + $0 } ?? "")
            self.bridgeCaptures = session.captures
            self.bridgeDestination = session.destination
        }
        session.onSourceWritten = { [weak self, weak session] url, text in
            guard let self, let session, self.bridge === session, self.documentURL == url else { return }
            self.savedText = text
        }
        session.onAdoptDocument = { [weak self, weak session] text in
            guard let self, let session, self.bridge === session else { return }
            self.adoptDurableDocument(text, session: session)
        }
        // Relaunch after an abnormal helper exit (mac-bridge-recovery): the
        // reconciled ledger may raise the revision floor; the user is told.
        session.onRevisionFloor = { [weak self, weak session] revision in
            guard let self, let session, self.bridge === session else { return }
            self.advanceEditorRevision(atLeast: revision)
        }
        session.onRelaunched = { [weak self, weak session] child, summary in
            guard let self, let session, self.bridge === session else { return }
            self.captureNote = "\(child == .bridge ? "Bridge" : "Edit ledger") relaunched after an abnormal exit: \(summary)"
        }
        // An ordinary edit overlapped the pin (mac-nearby-errors): the bridge
        // would refuse captures at it, so the pin is listed "(invalid)", the
        // companion is told `destination: null`, and the user is asked to pin
        // again. The shell never re-pins on the user's behalf.
        session.onDestinationDropped = { [weak self, weak session] destinationId in
            guard let self, let session, self.bridge === session else { return }
            self.captureNote = "Pinned insertion point \(destinationId) was dropped by an edit that overlapped it; pin again (Edit > Pin Insertion Point) before the next capture."
        }
        // Durable ledger first: the helper's document is the authoritative source.
        if let ledger {
            let store = ledgerStore ?? EditLedgerClient.storeDirectory(under: storeDirectory, documentURL: documentURL)
            let outcome = await session.openLedger(executable: ledger.executable, arguments: ledger.arguments, store: store,
                                                   path: activePath, currentText: activeText, currentRevision: editorRevision)
            guard bridge === session else { return false }
            switch outcome {
            case .fresh(let r), .aligned(let r), .bufferReplacedStore(let r):
                advanceEditorRevision(atLeast: r)
            case .adoptDurable(let text, let r):
                advanceEditorRevision(atLeast: r)
                captureNote = "Edit ledger: the durable document (with unconfirmed insertions) replaced the buffer; ⌘Z undoes the adoption."
                adoptDurableDocument(text, session: session)
            case .unavailable(let why):
                captureNote = "Edit ledger unavailable: \(why). Capture insertion is disabled."
            }
        } else {
            captureNote = "No flashtex-edit-ledger helper found (build crates/edit-ledger or set FLASHTEX_EDIT_LEDGER); capture insertion is disabled."
        }
        // Contract step 5: consult capture_status and the ledger before anything else.
        // The source is re-read after every await so edits made meanwhile are honored.
        let (actions, minimumRevision) = await session.reconcile(path: activePath,
                                                                 currentSource: { [weak self] in (self?.activeText ?? "", self?.editorRevision ?? 0) },
                                                                 statusTimeout: bridgeStatusTimeout)
        guard bridge === session else { return false }
        advanceEditorRevision(atLeast: minimumRevision)
        for action in actions {
            switch action {
            case .confirmed(let editId): captureNote = "Ledger: edit \(editId) confirmed by the bridge."
            case .replayedReceipt(let editId, let rev): captureNote = "Ledger: replayed missing receipt for \(editId) (revision \(rev))."
            case .needsReconciliation(let editId, let why), .retryLater(let editId, let why):
                captureNote = "Ledger: \(editId) — \(why)"
            }
        }
        if session.reconciliationIncomplete {
            captureNote = (captureNote.map { $0 + " " } ?? "") + "Reconciliation incomplete; use Edit > Retry Bridge Reconciliation."
        }
        do {
            try await session.open(path: activePath, revision: editorRevision, text: activeText)
        } catch {
            guard bridge === session else { return false }
            captureNote = "Bridge could not open \(activePath): \((error as? BridgeClient.Failure)?.text ?? "\(error)")"
            return false
        }
        return bridge === session
    }

    /// Replaces the whole buffer with the durable document as one undoable
    /// editor operation. The change is flagged so it is not persisted again.
    func adoptDurableDocument(_ text: String, session: BridgeSession) {
        guard bridge === session, text != activeText else { return }
        session.expectAdoption(of: text)
        let whole = NSRange(location: 0, length: (activeText as NSString).length)
        pendingEdit = .init(path: activePath, nsRange: whole, text: text, token: nextEditToken())
    }

    /// Re-runs restart reconciliation for entries a transport failure left
    /// undecided, then re-synchronizes the current document.
    @discardableResult
    func retryBridgeReconciliation() async -> Bool {
        guard let session = bridge, session.running else { return false }
        let (actions, minimumRevision) = await session.reconcile(path: activePath,
                                                                 currentSource: { [weak self] in (self?.activeText ?? "", self?.editorRevision ?? 0) },
                                                                 statusTimeout: bridgeStatusTimeout)
        guard bridge === session else { return false }
        advanceEditorRevision(atLeast: minimumRevision)
        captureNote = actions.isEmpty ? "Nothing left to reconcile." : "Reconciliation: \(actions.count) entr\(actions.count == 1 ? "y" : "ies") processed\(session.reconciliationIncomplete ? "; still incomplete" : "")."
        do { try await session.open(path: activePath, revision: editorRevision, text: activeText) } catch { return false }
        return !session.reconciliationIncomplete
    }

    /// Retries the export/receipt steps of a durable edit whose receipt was withheld.
    func retryBridgeReceipt() {
        guard let bridge else { captureNote = "No bridge attached."; return }
        if bridge.pendingTransaction == nil { captureNote = "No withheld receipt."; return }
        if bridge.commitPendingTransaction() { captureNote = "Receipt sent." } else { captureNote = bridge.status }
    }

    /// Called after a successful save so a pending export is recorded.
    func bridgeSourceSaved(url: URL, text: String) {
        bridge?.sourceSaved(url: url, text: text)
    }

    func detachBridge() { setBridge(nil) }

    // MARK: document synchronization

    func bridgeTextChanged(path: String, old: String, new: String, base: Int, revision: Int) {
        guard let bridge, !bridge.detached, bridge.running || bridge.ledgerUsable else { return } // ledger keeps following typing while the bridge relaunches
        if let expected = bridge.expectedApplication, expected.edit.path == path {
            if new == expected.afterText {
                // Contract step 4: the editor adopted the durable document; export, receipt. No document_edit.
                appliedCaptureIDs.insert(expected.edit.captureId)
                // `documentURL` is the entry's file: a session opened on a member
                // (attached while it was active) exports nothing — the member
                // stays dirty and reaches its own file through save/autosave.
                bridge.applicationApplied(newRevision: revision, afterText: new,
                                          sourceURL: path == project.entryPath ? documentURL : nil) // also drops the pinned destination
                captureNote = "Inserted \(expected.edit.captureId) via bridge edit \(expected.edit.editId) (undo with ⌘Z); pin a new insertion point for the next capture."
                return
            }
            bridge.abandonExpectedApplication("buffer changed differently than the durable document")
            captureNote = "Durable edit \(expected.edit.editId) was not adopted as committed; reattach the bridge to adopt the durable document."
        }
        bridge.edited(path: path, oldText: old, newText: new, base: base, revision: revision)
    }

    func bridgeDocumentReplaced() {
        guard let bridge, bridge.running else { return }
        bridge.abandonExpectedApplication("document replaced")
        bridge.invalidateDestination()
        Task { try? await bridge.open(path: activePath, revision: editorRevision, text: activeText) }
    }

    // MARK: destinations

    /// Mirrors a local pin to the bridge: the caret (or selection) as a byte
    /// range at the current revision. The local anchor stays for offline use.
    func bridgePin(_ anchor: InsertionAnchor) {
        guard let bridge, bridge.running else { return }
        let text = activeText
        let end = text.utf8ByteRange(of: NSRange(location: caretUTF16, length: caretLengthUTF16))?.end ?? anchor.byteOffset
        Task { await bridgePinAndWait(destinationId: anchor.id, path: anchor.path, revision: anchor.revision,
                                      startByte: anchor.byteOffset, endByte: max(end, anchor.byteOffset)) }
    }

    @discardableResult
    func bridgePinAndWait(destinationId: String, path: String, revision: Int, startByte: Int, endByte: Int) async -> TransferV1.Anchor? {
        guard let bridge, bridge.running else { return nil }
        do {
            let a = try await bridge.pin(destinationId: destinationId, path: path, revision: revision, startByte: startByte, endByte: endByte)
            captureNote = "Pinned \(a.destinationId) on the bridge at \(a.path) bytes \(a.startByte)..<\(a.endByte) (revision \(a.pinnedRevision))."
            return a
        } catch {
            captureNote = "Bridge pin failed: \((error as? BridgeClient.Failure)?.text ?? "\(error)")"
            return nil
        }
    }

    // MARK: captures

    /// `Edit > Submit Sample Capture…`: a PNG/JPEG file becomes one
    /// `capture_submit` bound to the pinned destination.
    func submitSampleCapturePanel() {
        guard bridgeAttached else { captureNote = "Attach the capture bridge first (Edit > Attach Capture Bridge)."; return }
        guard bridgeDestination?.valid == true else { captureNote = "Pin an insertion point first (⌘⌥P) so the capture has a destination."; return }
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.png, .jpeg]
        panel.message = "Choose a PNG or JPEG capture to submit through the bridge"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task { await submitCapture(imageAt: url) }
    }

    @discardableResult
    func submitCapture(imageAt url: URL, captureId: String? = nil, instructions: String = CaptureFeatures.defaultInstructions) async -> TransferV1.CaptureReceived? {
        guard let data = try? Data(contentsOf: url) else { captureNote = "Could not read \(url.lastPathComponent)."; return nil }
        let mime = url.pathExtension.lowercased() == "png" ? "image/png" : "image/jpeg"
        return await submitCapture(image: .init(mimeType: mime, dataBase64: data.base64EncodedString()),
                                   captureId: captureId, instructions: instructions)
    }

    /// Builds `capture_submit` from the pinned destination (`base_revision` is
    /// the anchor's pinned revision, never a guess) and shows the receipt.
    @discardableResult
    func submitCapture(image: RuntimeV1.CaptureImage, captureId: String? = nil, instructions: String) async -> TransferV1.CaptureReceived? {
        guard let bridge, bridge.running else { captureNote = "No bridge attached."; return nil }
        guard let destination = bridgeDestination else { captureNote = "Pin an insertion point first (⌘⌥P)."; return nil }
        // A pin an edit overlapped is listed "(invalid)"; the bridge would refuse it
        // (`destination_reselection_required`), so say so instead of sending.
        guard destination.valid else { captureNote = "Pinned insertion point \(destination.destinationId) was dropped by an edit; pin again (⌘⌥P) first."; return nil }
        guard RuntimeV1.acceptedCaptureMimeTypes.contains(image.mimeType) else { captureNote = "Only PNG and JPEG captures are accepted."; return nil }
        let id = captureId ?? "mac-capture-\(UUID().uuidString.lowercased())"
        let submit = RuntimeV1.CaptureSubmit(captureId: id, destinationId: destination.destinationId,
                                             baseRevision: destination.pinnedRevision, image: image, instructions: instructions)
        do {
            let received = try await bridge.submit(submit)
            captureNote = "Capture \(received.captureId) received (durable: \(received.durable)); use Edit > Convert Capture."
            return received
        } catch {
            captureNote = "Capture submit failed: \((error as? BridgeClient.Failure)?.text ?? "\(error)")"
            return nil
        }
    }

    /// `Edit > Convert Capture`: `capture_convert` for the latest received
    /// capture; a proposal is queued in the review sheet. Provider errors
    /// (`provider_disabled`, `provider_auth_missing`) are shown as plain text.
    func convertLatestCapture() {
        guard let c = latestConvertibleCapture else { captureNote = "No received capture to convert."; return }
        Task { await convertCapture(captureId: c.captureId) }
    }

    @discardableResult
    /// `supportedFeatures` defaults to what our compiler renders (CaptureFeatures.swift).
    func convertCapture(captureId: String, supportedFeatures: [String] = CaptureFeatures.supportedFeatures()) async -> RuntimeV1.CaptureProposal? {
        guard let bridge, bridge.running else { captureNote = "No bridge attached."; return nil }
        do {
            let proposal = try await bridge.convert(captureId: captureId, supportedFeatures: supportedFeatures)
            enqueue(proposal)
            return proposal
        } catch {
            captureNote = Self.conversionFailureNote(error, provider: bridge.conversionProvider)
            return nil
        }
    }

    /// Plain-text note for a failed `capture_convert`. Bridge provider codes
    /// (crates/bridge grok.rs) are translated into what the user can do; the
    /// bridge's message never contains the key or a provider body.
    static func conversionFailureNote(_ error: Error, provider: ConversionProvider) -> String {
        let failure = error as? BridgeClient.Failure
        let name = provider == .none ? "the conversion provider" : provider.displayName
        switch failure?.code {
        case "provider_disabled":
            return "Conversion unavailable: the bridge was attached without a conversion provider (provider_disabled). Choose a provider and add its API key in Preferences (⌘,) → Capture conversion, or set FLASHTEX_AI_API_KEY, then Edit > Attach Capture Bridge again."
        case "provider_auth_missing":
            return "Conversion unavailable: the bridge found no API key in its environment (provider_auth_missing); re-attach the bridge after adding the key."
        case "provider_auth_error":
            return "\(name) rejected the API key (HTTP 401/403); check the key in Preferences (⌘,) → Capture conversion."
        case "provider_rate_limited":
            return "\(name) is rate limiting this key (HTTP 429); no retry was made — try again later."
        case "provider_timeout":
            return "\(name) did not answer within the bridge's 90 s timeout; no retry was made."
        case "provider_transport_error":
            return "Could not reach \(name) (transport error); no retry was made."
        default:
            return "Conversion unavailable\(provider == .none ? "" : " (\(provider.displayName) enabled)"): \(failure?.text ?? "\(error)")"
        }
    }

    /// Approval for a bridge capture: `capture_prepare_insert` at the current
    /// editor revision, full verification of the returned edit, durable commit
    /// through the edit-ledger helper (source + applied ID, fsynced, receipt
    /// returned), then adoption of the durable document as one undoable editor
    /// edit; `capture_applied` follows once the editor reports the change.
    /// The reviewer's LaTeX must equal the journaled proposal: the contract has
    /// no field to send edited text, so an edited proposal is refused.
    @discardableResult
    func approveBridgeProposal(_ proposal: RuntimeV1.CaptureProposal, latex: String) async -> ApproveOutcome {
        guard let bridge, bridge.running else { captureNote = "No bridge attached."; return .refused("no bridge") }
        guard !appliedCaptureIDs.contains(proposal.captureId) else {
            proposals.removeAll { $0.captureId == proposal.captureId }
            if reviewing?.captureId == proposal.captureId { reviewing = proposals.first }
            captureNote = "Capture \(proposal.captureId) already inserted."
            return .duplicate
        }
        guard latex.trimmingCharacters(in: .whitespacesAndNewlines) == proposal.latex.trimmingCharacters(in: .whitespacesAndNewlines) else {
            captureNote = "Edited LaTeX cannot be inserted through the bridge (transfer-v1 prepares only the journaled proposal); reject and resubmit instead."
            return .refused("edited LaTeX")
        }
        guard bridge.expectedApplication == nil, bridge.pendingTransaction == nil else {
            captureNote = "Another prepared edit is still being applied or committed."
            return .refused("edit in progress")
        }
        guard bridge.ledgerUsable else {
            captureNote = "Edit ledger unusable; not applying: \(bridge.ledgerError ?? bridge.ledgerStatus). Reattach to reconcile."
            return .refused("ledger")
        }
        let edit: TransferV1.CaptureEdit
        do {
            edit = try await bridge.prepare(captureId: proposal.captureId, expectedRevision: editorRevision)
        } catch {
            let f = (error as? BridgeClient.Failure)
            captureNote = "Cannot insert \(proposal.captureId): \(f?.text ?? "\(error)")"
            if f?.code == "already_applied" {
                appliedCaptureIDs.insert(proposal.captureId)
                proposals.removeAll { $0.captureId == proposal.captureId }
                reviewing = proposals.first
                return .duplicate
            }
            if f?.code == "destination_reselection_required" || f?.code == "revision_conflict" {
                bridge.invalidateDestination()
                return .needsReselection(f?.text ?? "reselection required")
            }
            return .refused(f?.text ?? "\(error)")
        }
        guard self.bridge === bridge else { return .refused("bridge detached") }
        let text = activeText
        switch BridgeSession.verify(edit, projectId: projectId, path: activePath, revision: editorRevision, text: text) {
        case .refused(let why):
            captureNote = "Refused bridge edit \(edit.editId): \(why). Pin a new destination and submit a new capture."
            bridge.invalidateDestination()
            return .needsReselection(why)
        case .ok(let afterText):
            // Durable first: source + applied ID committed together by the helper.
            let applied: EditLedgerV1.Applied
            do {
                applied = try await bridge.applyDurably(edit, afterText: afterText)
            } catch let e as BridgeSession.LedgerError {
                switch e {
                case .unusable(let why): captureNote = "Edit ledger unusable; not applying: \(why)"
                case .transactionPending(let id): captureNote = "Edit \(id) is still being committed; not applying another."
                case .alreadyApplied(let id):
                    captureNote = "Edit \(id) is already durable; not inserting again."
                    appliedCaptureIDs.insert(proposal.captureId)
                    proposals.removeAll { $0.captureId == proposal.captureId }
                    reviewing = proposals.first
                    return .duplicate
                case .refused(let why): captureNote = "Edit ledger refused \(edit.editId); nothing inserted: \(why)"
                }
                return .refused("ledger")
            } catch {
                captureNote = "Edit ledger failed; nothing inserted: \(error.localizedDescription)"
                return .refused("ledger")
            }
            guard self.bridge === bridge else { return .refused("bridge detached") }
            // Adopt the durable document: the prepared range when it matches, else the whole text.
            activePath = edit.path
            if applied.document.text == afterText,
               let ns = text.nsRange(utf8Bytes: .init(path: edit.path, startByte: edit.startByte, endByte: edit.endByte)) {
                pendingEdit = .init(path: edit.path, nsRange: ns, text: edit.replacement, token: nextEditToken())
            } else {
                let whole = NSRange(location: 0, length: (text as NSString).length)
                pendingEdit = .init(path: edit.path, nsRange: whole, text: applied.document.text, token: nextEditToken())
            }
            proposals.removeAll { $0.captureId == proposal.captureId }
            reviewing = proposals.first
            captureNote = "Durable edit \(edit.editId) (revision \(applied.receipt.newRevision)); adopting at bytes \(edit.startByte)..<\(edit.endByte)…"
            return .inserted(byteOffset: edit.startByte)
        }
    }

    func bridgeReject(captureId: String) {
        guard let bridge, bridge.running, bridge.capture(captureId) != nil else { return }
        Task { await bridgeRejectAndWait(captureId: captureId) }
    }

    @discardableResult
    func bridgeRejectAndWait(captureId: String) async -> Bool {
        guard let bridge, bridge.running else { return false }
        do { try await bridge.reject(captureId: captureId); return true }
        catch { captureNote = "Reject failed: \((error as? BridgeClient.Failure)?.text ?? "\(error)")"; return false }
    }
}
