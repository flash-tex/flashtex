import CryptoKit
import Foundation
import FlashTeXProtocol

/// SHA-256 hex digest of a string's UTF-8 bytes (contract: `document_before_sha256`).
enum SourceDigest {
    static func sha256Hex(_ text: String) -> String {
        SHA256.hash(data: Data(text.utf8)).map { String(format: "%02x", $0) }.joined()
    }
}

/// One attached bridge process plus everything the Mac must remember about it:
/// the bridge's view of the document (what was sent), the pinned destination,
/// the captures in flight, and the durable edit ledger. Owned by `ShellModel`;
/// all state changes happen on the main actor and `onChange` lets the model publish.
///
/// Durability is delegated to the `flashtex-edit-ledger` helper (crates/edit-ledger
/// at d1dd1d7): it holds the authoritative document and every applied edit ID in
/// one atomically fsynced record. The Swift side keeps only an in-memory mirror.
/// Order for a reviewed insertion: bridge `capture_prepare_insert` → local
/// verification → helper `apply` (durable source + ledger, receipt returned) →
/// adopt the durable document in the editor as one undoable edit → export the
/// `.tex` when file-backed → `capture_applied` → helper `confirm`.
///
/// Both children are relaunched after an abnormal exit (same executable,
/// arguments and store) with the shell's worker policy — 0.2 / 1 / 3 s backoff,
/// at most `maxRelaunches` per minute per child, never after a clean exit or an
/// explicit `terminate()` — followed by the existing reconciliation: the ledger
/// is reopened and realigned (`openLedger`), pending receipts go through
/// `reconcile`, the document is reopened on the bridge and an exactly
/// restorable pinned destination is re-pinned (else reported). A capture whose
/// request was in flight at the crash is shown as `uncertain`, never dropped
/// or duplicated: the same capture ID is safe to resubmit.
@MainActor
final class BridgeSession {
    enum CaptureState: String {
        case received, converting, proposed, prepared, applied, confirmed, rejected, needsReselection, failed
        /// The bridge exited while the request was in flight: not known whether it
        /// was journaled. An identical resubmission is idempotent on the bridge.
        case uncertain
    }
    struct Capture: Equatable {
        var captureId: String
        var destinationId: String
        var state: CaptureState
        var note: String
    }

    /// Replaced by an automatic relaunch; callbacks compare identity before touching state.
    private(set) var client: BridgeClient
    let projectId: String
    let storeDirectory: URL
    private(set) var status: String
    private(set) var running = true
    private let bridgeLaunch: (executable: URL, arguments: [String], provider: ConversionProvider, environment: [String: String]?)
    /// The conversion provider the bridge was launched with (`.none` when no
    /// provider was selected or no key was present at attach time).
    var conversionProvider: ConversionProvider { bridgeLaunch.provider }
    var conversionEnabled: Bool { bridgeLaunch.provider != .none }

    // MARK: automatic relaunch (policy mirrors ShellModel's worker relaunch)

    enum Child: String { case bridge, ledger }
    static let maxRelaunches = 3
    /// Relaunch delays after the 1st, 2nd, 3rd abnormal exit within a minute.
    static let relaunchDelays: [TimeInterval] = [0.2, 1.0, 3.0]
    /// Automatic relaunches performed so far, per child (status/tests).
    private(set) var relaunchCount: [Child: Int] = [:]
    /// A relaunch (launch + reconciliation) is in progress for this child.
    private(set) var relaunching: Child?
    private var relaunchTimes: [Child: [Date]] = [:]
    private var relaunchWork: [Child: DispatchWorkItem] = [:]
    /// `terminate()` was called: no child is relaunched afterwards.
    private(set) var detached = false
    /// After an automatic relaunch and its reconciliation: which child, and a
    /// one-line summary the shell may show (captures reported uncertain,
    /// destination restored or lost, reconciliation actions).
    var onRelaunched: (Child, String) -> Void = { _, _ in }
    /// Reconciliation after a relaunch may need the editor revision to pass a
    /// revision the ledger/bridge already confirmed (`advanceEditorRevision`).
    var onRevisionFloor: (Int) -> Void = { _ in }
    /// An ordinary edit overlapped the pinned destination (id given): the pin
    /// is kept listed as invalid, never re-pinned; the shell tells the user.
    var onDestinationDropped: ((String) -> Void)?
    /// How the shell reads the live source, remembered from `reconcile`/`open`
    /// so a relaunch can reconcile and resynchronize without the shell.
    struct DocumentContext {
        var path: String
        var currentSource: () -> (text: String, revision: Int)
        var statusTimeout: TimeInterval
    }
    private var documentContext: DocumentContext?
    private(set) var destination: TransferV1.Anchor?
    private(set) var captures: [Capture] = []
    private(set) var log: [String] = []
    /// Snapshot the bridge is believed to hold per path (mirrors sent edits).
    private(set) var shadow: [String: (revision: Int, text: String)] = [:]
    /// Paths whose snapshot the bridge holds; edits before the initial
    /// `document_open` only update the shadow (the open sends the newest text).
    private(set) var openPaths: Set<String> = []
    /// Edit the editor is about to apply; the next text change equal to
    /// `afterText` is its application and must not be sent as `document_edit`.
    private(set) var expectedApplication: (edit: TransferV1.CaptureEdit, afterText: String)?
    var onChange: () -> Void = {}
    /// The durable document differs from the editor buffer (helper retry,
    /// crash recovery): the shell adopts `text` as one undoable operation.
    var onAdoptDocument: (String) -> Void = { _ in }
    /// Called after the post-edit `.tex` export was written atomically.
    var onSourceWritten: (URL, String) -> Void = { _, _ in }

    // Durable edit ledger (helper process) and its in-memory mirror.
    private(set) var ledger: EditLedgerClient?
    private(set) var ledgerStatus = "no edit ledger"
    private(set) var ledgerError: String?
    private(set) var durable: EditLedgerV1.Document?
    /// Applied transactions known to the helper (pending and confirmed), by edit ID.
    private(set) var transactions: [String: EditLedgerV1.AppliedTransaction] = [:]
    /// Entries the bridge has no durable record for; they keep their evidence in
    /// the helper until `resolveReconciliation` confirms them explicitly.
    private(set) var needsReconciliation: [String: String] = [:]
    var ledgerUsable: Bool { ledger?.isRunning == true && ledgerError == nil }
    private var ledgerLaunch: (executable: URL, arguments: [String], store: URL)?

    init(executable: URL, arguments: [String] = [], storeDirectory: URL, provider: ConversionProvider = .none, environment: [String: String]? = nil,
         projectId: String) throws {
        self.projectId = projectId
        self.storeDirectory = storeDirectory
        bridgeLaunch = (executable, arguments, provider, environment)
        status = "launching \(executable.lastPathComponent)"
        let (c, bind) = try Self.makeBridgeClient(bridgeLaunch, storeDirectory: storeDirectory)
        client = c
        bind(self)
        status = "attached: \(executable.lastPathComponent)"
    }

    /// Launches the bridge and returns it with a binder: once bound, its events
    /// reach the session only while it is still `session.client` (a replaced
    /// process's queued exit never overwrites the relaunched one).
    private static func makeBridgeClient(_ launch: (executable: URL, arguments: [String], provider: ConversionProvider, environment: [String: String]?),
                                         storeDirectory: URL) throws -> (BridgeClient, (BridgeSession) -> Void) {
        var events: ((BridgeClient.Event) -> Void)?
        let c = try BridgeClient(executable: launch.executable, arguments: launch.arguments, storeDirectory: storeDirectory,
                                 provider: launch.provider, environment: launch.environment) { events?($0) }
        return (c, { session in
            events = { [weak session, weak c] event in
                Task { @MainActor in
                    guard let session, let c, session.client === c else { return }
                    session.handle(event)
                }
            }
        })
    }

    func terminate() {
        detached = true
        for item in relaunchWork.values { item.cancel() }
        relaunchWork.removeAll()
        relaunching = nil
        client.terminate()
        ledger?.terminate()
        running = false
        status = "bridge detached"
        onChange()
    }

    private func handle(_ event: BridgeClient.Event) {
        switch event {
        case .stderr(let s): note("bridge: " + s.trimmingCharacters(in: .whitespacesAndNewlines))
        case .protocolViolation(let m): status = "protocol violation: \(m)"; note(status)
        case .unsolicited(let id, let type, let code): note("unsolicited \(type) id=\(id ?? "null") code=\(code ?? "-")")
        case .exited(let code):
            running = false
            status = "bridge exited (\(code))"
            note(status)
            // `expectedApplication` is kept: an adoption the editor performs while
            // the bridge is down is recognized after the relaunch, not sent as an edit.
            scheduleRelaunch(.bridge, afterExit: code)
        }
        onChange()
    }

    private func handleLedger(_ event: EditLedgerClient.Event) {
        switch event {
        case .stderr(let s): note("ledger: " + s.trimmingCharacters(in: .whitespacesAndNewlines))
        case .protocolViolation(let m): ledgerError = "edit ledger protocol violation: \(m)"; note(ledgerError!)
        case .unsolicited(let id, let type, let code): note("ledger unsolicited \(type) id=\(id ?? "null") code=\(code ?? "-")")
        case .exited(let code):
            if ledgerError == nil { ledgerError = "edit ledger exited (\(code))" }
            ledgerStatus = ledgerError!
            note(ledgerStatus)
            scheduleRelaunch(.ledger, afterExit: code)
        }
        onChange()
    }

    /// An abnormal exit of a child we launched relaunches it after a short
    /// backoff, at most `maxRelaunches` times per minute; beyond that the exit
    /// stays visible with a re-attach hint. A clean exit (0) or an explicit
    /// `terminate()` never relaunches. Nothing is relaunched twice: the work
    /// item checks that the exited process is still the current one.
    private func scheduleRelaunch(_ child: Child, afterExit code: Int32) {
        guard code != 0, !detached else { return }
        if child == .ledger, ledgerLaunch == nil { return }
        let now = Date()
        var times = (relaunchTimes[child] ?? []).filter { now.timeIntervalSince($0) < 60 }
        let name = child == .bridge ? bridgeLaunch.executable.lastPathComponent : ledgerLaunch!.executable.lastPathComponent
        guard times.count < Self.maxRelaunches else {
            let hint = "\(Self.maxRelaunches) relaunches in the last minute — Edit > Attach Capture Bridge to retry"
            if child == .bridge {
                status = "bridge exited (\(code)); not relaunched: \(hint)"
            } else {
                ledgerError = "edit ledger exited (\(code)); not relaunched: \(hint)"
                ledgerStatus = ledgerError!
            }
            relaunchTimes[child] = times
            note("\(child.rawValue) relaunch limit reached")
            return
        }
        let delay = Self.relaunchDelays[min(times.count, Self.relaunchDelays.count - 1)]
        times.append(now)
        relaunchTimes[child] = times
        let attempt = times.count
        if child == .bridge {
            status = String(format: "bridge exited (%d); relaunching in %.1f s", code, delay)
        } else {
            ledgerStatus = String(format: "edit ledger exited (%d); relaunching in %.1f s", code, delay)
        }
        note("relaunching \(name) in \(delay) s (attempt \(attempt) this minute)")
        armRelaunch(child, exitedBridge: child == .bridge ? client : nil, exitedLedger: child == .ledger ? ledger : nil, after: delay)
    }

    private func armRelaunch(_ child: Child, exitedBridge: BridgeClient?, exitedLedger: EditLedgerClient?, after delay: TimeInterval) {
        relaunchWork[child]?.cancel()
        let item = DispatchWorkItem { [weak self] in
            guard let self, !self.detached else { return }
            // Only the process that exited is replaced; a client someone else already
            // replaced (`reopenLedger` after a poisoned handle) is left alone.
            switch child {
            case .bridge: guard self.client === exitedBridge, !self.client.isRunning else { return }
            case .ledger: guard self.ledger === exitedLedger, self.ledger?.isRunning != true else { return }
            }
            if self.relaunching != nil { // the other child is mid-relaunch: wait for it
                self.armRelaunch(child, exitedBridge: exitedBridge, exitedLedger: exitedLedger, after: 0.1)
                return
            }
            self.relaunchWork[child] = nil
            Task { @MainActor in await self.relaunch(child) }
        }
        relaunchWork[child] = item
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: item)
    }

    /// Relaunches `child` and runs the reconciliation that a fresh attach runs,
    /// against the live source: ledger → `openLedger` (realign/adopt), then
    /// `reconcile` for pending receipts, then the bridge document is reopened
    /// and an exactly restorable destination re-pinned.
    private func relaunch(_ child: Child) async {
        guard !detached else { return }
        relaunching = child
        defer { relaunching = nil }
        var summary: [String] = []
        switch child {
        case .bridge:
            do {
                let (c, bind) = try Self.makeBridgeClient(bridgeLaunch, storeDirectory: storeDirectory)
                client = c
                bind(self)
            } catch {
                status = "bridge relaunch failed: \(error.localizedDescription) — Edit > Attach Capture Bridge to retry"
                note(status)
                onChange()
                onRelaunched(.bridge, status)
                return
            }
            relaunchCount[.bridge, default: 0] += 1
            running = true
            openPaths = [] // the bridge's documents and anchors were in memory
            status = "relaunched \(bridgeLaunch.executable.lastPathComponent); reconciling…"
            note("relaunched \(bridgeLaunch.executable.lastPathComponent) (relaunch \(relaunchCount[.bridge]!))")
            summary += markInFlightUncertain()
        case .ledger:
            guard let launch = ledgerLaunch, let context = documentContext ?? shadowContext() else { return }
            let source = context.currentSource()
            let outcome = await openLedger(executable: launch.executable, arguments: launch.arguments, store: launch.store,
                                           path: context.path, currentText: source.text, currentRevision: source.revision)
            guard !detached else { return }
            relaunchCount[.ledger, default: 0] += 1
            note("relaunched \(launch.executable.lastPathComponent) (relaunch \(relaunchCount[.ledger]!)): \(ledgerStatus)")
            switch outcome {
            case .fresh(let r), .aligned(let r), .bufferReplacedStore(let r):
                onRevisionFloor(r)
                summary.append("edit ledger relaunched; \(ledgerStatus)")
            case .adoptDurable(let text, let r):
                onRevisionFloor(r)
                summary.append("edit ledger relaunched; the durable document (with unconfirmed insertions) replaces the buffer")
                onAdoptDocument(text)
            case .unavailable(let why):
                summary.append("edit ledger relaunched but unusable: \(why)")
            }
            ledgerStatus = "relaunched: " + ledgerStatus
        }
        onChange()
        // Pending receipts (a receipt in flight at the crash, or older ones) go through the
        // ledger-guarded reconciliation; the live source is then reopened on the bridge.
        if running, let context = documentContext ?? shadowContext() {
            if ledgerUsable, durable != nil {
                let (actions, minimum) = await reconcile(path: context.path, currentSource: context.currentSource, statusTimeout: context.statusTimeout)
                guard !detached else { return }
                onRevisionFloor(minimum)
                for action in actions {
                    switch action {
                    case .confirmed(let id): summary.append("\(id) confirmed by the bridge")
                    case .replayedReceipt(let id, let rev): summary.append("replayed receipt for \(id) (revision \(rev))")
                    case .needsReconciliation(let id, let why), .retryLater(let id, let why): summary.append("\(id): \(why)")
                    }
                }
                if reconciliationIncomplete { summary.append("reconciliation incomplete; use Edit > Retry Bridge Reconciliation") }
            } else if child == .bridge, ledger != nil {
                summary.append("pending receipts not reconciled: \(ledgerError ?? ledgerStatus)")
            }
            let source = context.currentSource()
            do {
                try await open(path: context.path, revision: source.revision, text: source.text)
                guard !detached else { return }
                if child == .bridge { summary.append(await restoreDestination(path: context.path, source: source)) }
            } catch {
                summary.append("could not reopen \(context.path) after the relaunch: \((error as? BridgeClient.Failure)?.text ?? "\(error)")")
            }
        }
        let line = summary.joined(separator: "; ")
        note("\(child.rawValue) relaunch: \(line)")
        onChange()
        onRelaunched(child, line)
    }

    /// Fallback document context when the shell never called `reconcile`: the
    /// bridge's shadow of the last opened path.
    private func shadowContext() -> DocumentContext? {
        guard let (path, _) = shadow.first else { return nil }
        return .init(path: path, currentSource: { [weak self] in
            let s = self?.shadow[path]
            return (s?.text ?? "", s?.revision ?? 0)
        }, statusTimeout: 15)
    }

    /// A transaction whose receipt was in flight (or owed) when the bridge exited
    /// is uncertain: it leaves live tracking and is settled from the ledger by
    /// `reconcile` (confirmed, replayed from the pre-edit snapshot, or held as
    /// evidence). A durable edit the editor has not adopted yet stays live: its
    /// receipt follows the adoption, on the relaunched bridge. Deferred edits
    /// are dropped — the reopen carries the newest text.
    private func markInFlightUncertain() -> [String] {
        var lines: [String] = []
        deferredEdits.removeAll()
        if let expected = expectedApplication {
            let current = (documentContext ?? shadowContext())?.currentSource().text
            if current == expected.afterText {
                expectedApplication = nil // adopted while the bridge was down; the receipt is owed
            } else {
                lines.append("edit \(expected.edit.editId) awaits adoption in the editor; its receipt follows")
                return lines
            }
        }
        if let tx = pendingTransaction {
            pendingTransaction = nil
            let why = tx.receiptSent ? "receipt for \(tx.edit.editId) was in flight when the bridge exited" : "receipt for \(tx.edit.editId) was owed when the bridge exited"
            setCapture(tx.edit.captureId, .uncertain, "\(why); settling from the edit ledger")
            lines.append(why)
        }
        return lines
    }

    /// After a bridge relaunch its anchors are gone. The pinned destination is
    /// re-pinned identically when the document is still at the pinned revision
    /// (same ID, range and source hash: captures bound to it stay valid);
    /// otherwise it is dropped and reported, and a new pin is required.
    private func restoreDestination(path: String, source: (text: String, revision: Int)) async -> String {
        guard let d = destination else { return "no pinned destination" }
        if d.isCaret {
            // The automatic destination is re-pinned at the caret the next time a
            // companion asks, a capture arrives or one is approved; nothing to restore.
            destination = nil
            onChange()
            return "automatic destination \(d.destinationId) will be re-pinned at the caret on demand"
        }
        guard d.valid else {
            destination = nil
            onChange()
            return "pinned destination \(d.destinationId) had been dropped by an edit before the bridge exited; pin again"
        }
        guard d.path == path, d.pinnedRevision == source.revision, d.binding.sourceSha256 == SourceDigest.sha256Hex(source.text) else {
            destination = nil
            onChange()
            return "pinned destination \(d.destinationId) was lost with the bridge (source moved past revision \(d.pinnedRevision)); pin again"
        }
        do {
            let attached = status
            let a = try await pin(destinationId: d.destinationId, path: d.path, revision: d.pinnedRevision,
                                  startByte: d.binding.startByte, endByte: d.binding.endByte)
            status = attached // the pin is a restoration, not news
            guard a.binding == d.binding else {
                destination = nil
                onChange()
                return "pinned destination \(d.destinationId) could not be restored identically; pin again"
            }
            return "pinned destination \(d.destinationId) restored"
        } catch {
            destination = nil
            onChange()
            return "pinned destination \(d.destinationId) could not be restored (\((error as? BridgeClient.Failure)?.text ?? "\(error)")); pin again"
        }
    }

    private func note(_ line: String) {
        log.append(line)
        if log.count > 200 { log.removeFirst(log.count - 200) }
    }

    private func fail(_ what: String, _ error: Error) -> BridgeClient.Failure {
        let f = (error as? BridgeClient.Failure) ?? .undecodable("\(error)")
        if f.isTransient, !running {
            // The process is gone: the exit (and its relaunch schedule) is the status; the
            // failed request is only logged.
            note("\(what) failed — \(f.text)")
        } else {
            status = "\(what) failed — \(f.text)"
            note(status)
        }
        onChange()
        return f
    }

    func capture(_ id: String) -> Capture? { captures.first { $0.captureId == id } }

    private func setCapture(_ id: String, destination: String? = nil, _ state: CaptureState, _ text: String) {
        if let i = captures.firstIndex(where: { $0.captureId == id }) {
            captures[i].state = state; captures[i].note = text
        } else {
            captures.append(.init(captureId: id, destinationId: destination ?? "", state: state, note: text))
        }
        onChange()
    }

    // MARK: durable ledger (helper process)

    enum LedgerOpen: Equatable {
        case fresh(revision: Int)
        /// Store and buffer agree (after revision alignment).
        case aligned(revision: Int)
        /// The durable document is authoritative (pending receipts) and differs
        /// from the buffer: the shell must adopt `text` (one undoable operation).
        case adoptDurable(text: String, revision: Int)
        /// No pending receipts and the buffer differs: the buffer replaced the store.
        case bufferReplacedStore(revision: Int)
        case unavailable(String)
    }

    /// Launches the helper on `store` and aligns it with the editor. Returns
    /// the revision the editor must be at (only ever advancing it).
    func openLedger(executable: URL, arguments: [String] = [], store: URL, path: String,
                    currentText: String, currentRevision: Int, timeout: TimeInterval = 15) async -> LedgerOpen {
        ledger?.terminate()
        ledger = nil
        ledgerError = nil
        transactions = [:]
        durable = nil
        ledgerLaunch = (executable, arguments, store)
        do {
            try FileManager.default.createDirectory(at: store.deletingLastPathComponent(), withIntermediateDirectories: true,
                                                    attributes: [.posixPermissions: 0o700])
            var events: ((EditLedgerClient.Event) -> Void)?
            let l = try EditLedgerClient(executable: executable, arguments: arguments, storeDirectory: store) { events?($0) }
            // Events from a helper that is no longer `ledger` (replaced after a poisoned handle) are dropped.
            events = { [weak self, weak l] e in Task { @MainActor in guard let self, let l, self.ledger === l else { return }; self.handleLedger(e) } }
            ledger = l
            let st = try await l.status(timeout: timeout)
            for tx in st.pendingReceipts { transactions[tx.edit.editId] = tx }
            guard let existing = st.document else {
                durable = try await l.initialize(.init(projectId: projectId, path: path, revision: currentRevision, text: currentText))
                ledgerStatus = "edit ledger: fresh store at revision \(currentRevision)"
                onChange()
                return .fresh(revision: currentRevision)
            }
            guard existing.projectId == projectId, existing.path == path else {
                ledgerError = "edit ledger store holds \(existing.projectId)/\(existing.path), not \(projectId)/\(path)"
                ledgerStatus = ledgerError!
                onChange()
                return .unavailable(ledgerError!)
            }
            durable = existing
            if existing.text == currentText {
                let target = max(existing.revision, currentRevision)
                try await alignStore(to: target)
                ledgerStatus = "edit ledger: store matches the buffer at revision \(target)"
                onChange()
                return .aligned(revision: target)
            }
            if !st.pendingReceipts.isEmpty {
                // Durable document is authoritative: it carries applied edits whose receipts are still owed.
                let target = max(existing.revision, currentRevision)
                try await alignStore(to: target)
                ledgerStatus = "edit ledger: durable document adopted (\(st.pendingReceipts.count) pending receipt\(st.pendingReceipts.count == 1 ? "" : "s"))"
                onChange()
                return .adoptDurable(text: existing.text, revision: target)
            }
            // No receipts owed: the buffer (the user's current source) replaces the durable text.
            durable = try await l.replaceDocument(expectedRevision: existing.revision, expectedSha256: existing.sourceSha256, text: currentText)
            let target = max(durable!.revision, currentRevision)
            try await alignStore(to: target)
            ledgerStatus = "edit ledger: buffer replaced the stored document (revision \(target))"
            onChange()
            return .bufferReplacedStore(revision: target)
        } catch {
            let f = (error as? LineProcessFailure) ?? .undecodable("\(error)")
            ledgerError = "edit ledger unavailable — \(f.text)"
            ledgerStatus = ledgerError!
            note(ledgerStatus)
            onChange()
            return .unavailable(ledgerError!)
        }
    }

    /// Editor revisions and the helper's document revision advance in lockstep
    /// (one per edit); an existing store can lag the editor after a restart.
    private func alignStore(to revision: Int) async throws {
        guard let l = ledger, var doc = durable else { return }
        var steps = 0
        while doc.revision < revision {
            doc = try await l.replaceDocument(expectedRevision: doc.revision, expectedSha256: doc.sourceSha256, text: doc.text)
            durable = doc
            steps += 1
            if steps > 10_000 { throw LineProcessFailure.undecodable("revision alignment exceeded 10000 steps") }
        }
    }

    /// Relaunches the helper after a persistence error (its handle is poisoned:
    /// the rename may or may not have completed) and re-reads the durable state.
    private func reopenLedger(timeout: TimeInterval = 15) async -> Bool {
        guard let launch = ledgerLaunch else { return false }
        ledger?.terminate()
        ledger = nil
        ledgerError = nil
        do {
            var events: ((EditLedgerClient.Event) -> Void)?
            let l = try EditLedgerClient(executable: launch.executable, arguments: launch.arguments, storeDirectory: launch.store) { events?($0) }
            // Events from a helper that is no longer `ledger` (replaced after a poisoned handle) are dropped.
            events = { [weak self, weak l] e in Task { @MainActor in guard let self, let l, self.ledger === l else { return }; self.handleLedger(e) } }
            ledger = l
            let st = try await l.status(timeout: timeout)
            durable = st.document
            transactions = [:]
            for tx in st.pendingReceipts { transactions[tx.edit.editId] = tx }
            ledgerStatus = "edit ledger reopened at revision \(st.document?.revision ?? 0)"
            onChange()
            return true
        } catch {
            let f = (error as? LineProcessFailure) ?? .undecodable("\(error)")
            ledgerError = "edit ledger unavailable — \(f.text)"
            ledgerStatus = ledgerError!
            onChange()
            return false
        }
    }

    // MARK: documents

    func open(path: String, revision: Int, text: String) async throws {
        do {
            _ = try await client.request(.documentOpen, TransferV1.DocumentOpen(projectId: projectId, path: path, revision: revision, text: text),
                                         as: TransferV1.Empty.self)
            shadow[path] = (revision, text)
            openPaths.insert(path)
            documentContext?.path = path
            let relaunched = relaunchCount[.bridge].map { $0 > 0 ? " · relaunched \($0)×" : "" } ?? ""
            status = "attached: \(client.executable.lastPathComponent)\(relaunched) · \(path) open at revision \(revision)"
            onChange()
        } catch { throw fail("document_open", error) }
    }

    /// Ordinary typing/undo: persisted through the helper (`replace_document`,
    /// tombstones kept) and sent to the bridge as one `document_edit` (byte
    /// range + replacement). Fire-and-forget: a refused bridge edit triggers a
    /// `document_open` resynchronization; a refused durable replace means the
    /// store diverged and application is disabled until the next attach. The
    /// durable store follows the editor even while the bridge is down (awaiting
    /// relaunch): the relaunch reopens the newest text.
    func edited(path: String, oldText: String, newText: String, base: Int, revision: Int) {
        guard !detached else { return }
        let region = SourceMapping.changedRegion(from: oldText, to: newText)
        let edit = TransferV1.DocumentEdit(projectId: projectId, path: path, baseRevision: base, revision: revision,
                                           startByte: region.startByte, endByte: region.oldEndByte, replacement: region.replacement)
        shadow[path] = (revision, newText)
        followDestination(path: path, region: region, revision: revision)
        if let l = ledger, ledgerError == nil, let doc = durable, doc.path == path {
            if consumeAdoption(newText) {
                // The store already holds this text; only its revision number must catch up.
                Task { [weak self] in try? await self?.alignStore(to: revision) }
            } else if doc.revision == base, doc.text == oldText {
                durable = .init(projectId: projectId, path: path, revision: revision, text: newText)
                l.send({ EditLedgerV1.ReplaceRequest(id: $0, expectedRevision: base, expectedSha256: doc.sourceSha256, text: newText) },
                       as: EditLedgerV1.DocumentReply.self) { [weak self] result in
                    guard let self, self.ledger === l else { return } // a replaced helper's late reply is not ours
                    if case .failure(let f) = result {
                        if f.isTransient {
                            // The helper did not answer (exited mid-request): the relaunch reopens the
                            // store and realigns it with the buffer; nothing is concluded here.
                            self.ledgerStatus = "edit ledger did not acknowledge replace_document (\(f.text)); realigning after relaunch"
                            self.note(self.ledgerStatus)
                        } else {
                            self.ledgerError = "durable document diverged from the editor (\(f.text)); reattach to reconcile"
                            self.ledgerStatus = self.ledgerError!
                            self.note(self.ledgerStatus)
                        }
                        self.onChange()
                    }
                }
            } else {
                ledgerError = "durable document diverged from the editor (store at revision \(doc.revision), edit based on \(base)); reattach to reconcile"
                ledgerStatus = ledgerError!
                note(ledgerStatus)
                onChange()
            }
        }
        guard running, openPaths.contains(path) else { return } // the (re)open will carry this text
        if let tx = pendingTransaction, tx.edit.path == path {
            // The bridge must apply the receipt before edits based on the post-edit revision.
            deferredEdits.append(edit)
            return
        }
        sendEdit(edit)
    }

    private func sendEdit(_ edit: TransferV1.DocumentEdit) {
        client.send(.documentEdit, edit, as: TransferV1.DocumentUpdated.self) { [weak self] result in
            guard let self else { return }
            switch result {
            case .success(let updated):
                if updated.revision != edit.revision { self.note("document_updated reports revision \(updated.revision), sent \(edit.revision)") }
            case .failure(let f):
                self.note("document_edit \(edit.baseRevision)->\(edit.revision) refused (\(f.text)); resynchronizing snapshot")
                self.resync(path: edit.path)
            }
        }
    }

    private func resync(path: String) {
        guard running, let snap = shadow[path] else { return }
        client.send(.documentOpen, TransferV1.DocumentOpen(projectId: projectId, path: path, revision: snap.revision, text: snap.text),
                    as: TransferV1.Empty.self) { [weak self] result in
            guard let self else { return }
            if case .failure(let f) = result { self.status = "snapshot resync failed — \(f.text)"; self.note(self.status); self.onChange() }
            else { self.note("snapshot resynchronized at revision \(snap.revision)") }
        }
    }

    // MARK: destinations and captures

    /// `mode: .caret` is the automatic destination (re-pinnable under the same
    /// id, follows edits, never invalidated); `.fixed` (default) is the user's
    /// explicit pin with the strict contract. The bridge's reply is the mirror.
    func pin(destinationId: String, path: String, revision: Int, startByte: Int, endByte: Int,
             mode: TransferV1.AnchorMode = .fixed) async throws -> TransferV1.Anchor {
        do {
            let anchor = try await client.request(.destinationPin,
                TransferV1.DestinationPin(destinationId: destinationId, projectId: projectId, path: path, revision: revision,
                                          startByte: startByte, endByte: endByte, mode: mode == .fixed ? nil : mode),
                as: TransferV1.Anchor.self)
            destination = anchor
            status = "pinned \(destinationId) at \(path) bytes \(startByte)..<\(endByte) (revision \(revision))\(mode == .caret ? " · caret" : "")"
            onChange()
            return anchor
        } catch { throw fail("destination_pin", error) }
    }

    func submit(_ capture: RuntimeV1.CaptureSubmit) async throws -> TransferV1.CaptureReceived {
        do {
            let received = try await client.request(.captureSubmit, capture, as: TransferV1.CaptureReceived.self)
            setCapture(capture.captureId, destination: capture.destinationId, .received,
                       "received (durable: \(received.durable), proposal: \(received.hasProposal), applied: \(received.applied))")
            status = "capture \(capture.captureId) received durably"
            onChange()
            return received
        } catch {
            let f = fail("capture_submit", error)
            if f.isTransient, self.capture(capture.captureId).map({ $0.state == .uncertain || $0.state == .failed }) ?? true {
                // The bridge did not answer: the capture may or may not be journaled. It is
                // never dropped; the same capture ID can be resubmitted (idempotent on the bridge).
                setCapture(capture.captureId, destination: capture.destinationId, .uncertain,
                           "bridge did not acknowledge capture_submit (\(f.text)); not known whether it was journaled — resubmit with the same capture ID")
            } else {
                // A conflicting retry does not change the state of the capture already journaled.
                setCapture(capture.captureId, destination: capture.destinationId, self.capture(capture.captureId)?.state ?? .failed, f.text)
            }
            throw f
        }
    }

    /// Forgets the pinned destination (after an insertion at it, or when the
    /// bridge reported that reselection is required). A new pin gets a new ID.
    func invalidateDestination() {
        destination = nil
        onChange()
    }

    /// Mirrors the bridge's own anchor rule for an ordinary edit
    /// (`DestinationTracking`): a pin the edit overlapped stays listed but
    /// `valid == false` (shown "(invalid)", announced to companions as no
    /// destination) until the user pins again; a pin after the edit is
    /// shifted. Never re-pins.
    private func followDestination(path: String, region: SourceMapping.ChangedRegion, revision: Int) {
        guard let d = destination, d.projectId == projectId, d.path == path else { return }
        let next = DestinationTracking.follow(d, region: region, revision: revision)
        guard next != d else { return }
        destination = next
        if d.valid, !next.valid {
            note("pinned destination \(d.destinationId) dropped: the edit at bytes \(region.startByte)..<\(region.oldEndByte) (revision \(revision)) overlapped it; pin again")
            onDestinationDropped?(d.destinationId)
        }
        onChange()
    }

    func convert(captureId: String, supportedFeatures: [String] = []) async throws -> RuntimeV1.CaptureProposal {
        let previous = capture(captureId)?.state ?? .received
        setCapture(captureId, .converting, "converting…")
        do {
            let proposal = try await client.request(.captureConvert,
                TransferV1.CaptureConvert(captureId: captureId, supportedFeatures: supportedFeatures),
                as: RuntimeV1.CaptureProposal.self)
            setCapture(captureId, .proposed, "proposal ready (context revision \(proposal.contextRevision.map(String.init) ?? "?"))")
            status = "proposal for \(captureId) awaiting review"
            onChange()
            return proposal
        } catch {
            let f = fail("capture_convert", error)
            // Provider errors are plain text; nothing here prompts for a key. The
            // capture stays convertible unless the bridge says it is rejected.
            setCapture(captureId, f.code == "capture_rejected" ? .rejected : previous, f.text)
            throw f
        }
    }

    /// `wrap` is what the shell computed from the caret's context at approval
    /// (`CaretContext.wrapping`); the bridge journals it with the edit.
    func prepare(captureId: String, expectedRevision: Int, wrap: TransferV1.InsertionWrap? = nil) async throws -> TransferV1.CaptureEdit {
        do {
            let edit = try await client.request(.capturePrepareInsert,
                TransferV1.CapturePrepareInsert(captureId: captureId, expectedRevision: expectedRevision, approved: true, wrap: wrap),
                as: TransferV1.CaptureEdit.self)
            setCapture(captureId, .prepared, "edit \(edit.editId) prepared at revision \(edit.expectedRevision)")
            return edit
        } catch {
            let f = fail("capture_prepare_insert", error)
            // The row state must say what the bridge holds: no proposal exists after
            // `proposal_missing` (received, convertible), `capture_rejected` is terminal,
            // `already_applied` is settled by the shell; other refusals keep the proposal.
            let state: CaptureState
            switch f.code {
            case "destination_reselection_required": state = .needsReselection
            case "proposal_missing": state = .received
            case "capture_rejected": state = .rejected
            case "already_applied": state = capture(captureId)?.state == .confirmed ? .confirmed : .applied
            default: state = .proposed
            }
            setCapture(captureId, state, f.text)
            throw f
        }
    }

    func reject(captureId: String) async throws {
        do {
            _ = try await client.request(.captureReject, TransferV1.CaptureID(captureId: captureId), as: TransferV1.CaptureID.self)
            setCapture(captureId, .rejected, "rejected (durable, terminal)")
            status = "capture \(captureId) rejected"
            onChange()
        } catch {
            let f = fail("capture_reject", error)
            setCapture(captureId, capture(captureId)?.state ?? .failed, f.text)
            throw f
        }
    }

    func status(captureId: String) async throws -> TransferV1.CaptureStatus {
        do { return try await client.request(.captureStatus, TransferV1.CaptureID(captureId: captureId), as: TransferV1.CaptureStatus.self) }
        catch { throw fail("capture_status", error) }
    }

    // MARK: verified application and durable ledger

    enum Verification: Equatable { case ok(afterText: String); case refused(String) }

    /// Contract step 2: revision, SHA-256 of the UTF-8 source, scalar boundaries
    /// and removed text must all match the current buffer before anything is applied.
    static func verify(_ edit: TransferV1.CaptureEdit, projectId: String, path: String, revision: Int, text: String) -> Verification {
        guard edit.projectId == projectId, edit.path == path else {
            return .refused("edit targets \(edit.projectId)/\(edit.path), not the open document")
        }
        guard edit.expectedRevision == revision else {
            return .refused("edit expects revision \(edit.expectedRevision) but the editor is at \(revision)")
        }
        guard SourceDigest.sha256Hex(text) == edit.documentBeforeSha256 else {
            return .refused("document SHA-256 does not match the prepared edit (buffer changed)")
        }
        guard let range = text.rangeOfUTF8(start: edit.startByte, end: edit.endByte) else {
            return .refused("bytes \(edit.startByte)..<\(edit.endByte) are not a scalar-aligned range of the buffer")
        }
        guard String(text[range]) == edit.removedText else {
            return .refused("removed_text does not match the bytes at \(edit.startByte)..<\(edit.endByte)")
        }
        return .ok(afterText: text.replacingCharacters(in: range, with: edit.replacement))
    }

    enum LedgerError: Error, Equatable {
        case unusable(String)
        case transactionPending(String)
        case alreadyApplied(String)
        /// The helper refused the edit (its code); nothing was inserted anywhere.
        case refused(String)
    }

    /// Ordered record of durability steps, for evidence:
    /// "ledger" (helper commit), "source" (.tex export), "receipt", "confirmed".
    private(set) var transactionTrace: [String] = []
    /// One reviewed edit that is durable in the helper but not yet acknowledged
    /// by the bridge. Survives receipt failures; `commitPendingTransaction` retries.
    struct Transaction: Equatable {
        var edit: TransferV1.CaptureEdit
        var receipt: TransferV1.CaptureApplied
        var afterText: String
        var sourceURL: URL?
        var sourceWritten = false
        var receiptSent = false
        var lastFailure: String?
    }
    private(set) var pendingTransaction: Transaction?
    /// Bridge edits held back until the pending receipt is acknowledged.
    private var deferredEdits: [TransferV1.DocumentEdit] = []
    /// Whole-document adoption the shell is about to perform (not an ordinary edit).
    private var expectedAdoption: String?

    /// Step 3 (helper side): commits the verified edit durably — source and
    /// applied ID together, fsynced — and returns the durable document the
    /// editor must adopt. Nothing is inserted in the editor before this returns.
    /// An identical retry returns the original receipt without inserting twice.
    func applyDurably(_ edit: TransferV1.CaptureEdit, afterText: String) async throws -> EditLedgerV1.Applied {
        guard ledgerUsable, let l = ledger else { throw LedgerError.unusable(ledgerError ?? ledgerStatus) }
        guard pendingTransaction == nil else { throw LedgerError.transactionPending(pendingTransaction!.edit.editId) }
        if let tx = transactions[edit.editId] {
            setCapture(edit.captureId, tx.confirmed ? .confirmed : .applied, "edit \(edit.editId) already applied durably; not inserting again")
            throw LedgerError.alreadyApplied(edit.editId)
        }
        do {
            let applied = try await l.apply(edit)
            durable = applied.document
            transactions[edit.editId] = .init(edit: edit, receipt: applied.receipt, documentBefore: nil,
                                              documentAfterSha256: applied.document.sourceSha256, confirmed: false)
            transactionTrace.append("ledger")
            if applied.document.text != afterText {
                note("durable document after \(edit.editId) differs from the expected text; adopting the durable document")
            }
            expectedApplication = (edit, applied.document.text)
            pendingTransaction = Transaction(edit: edit, receipt: applied.receipt, afterText: applied.document.text)
            setCapture(edit.captureId, .applied, "durable in the edit ledger as revision \(applied.receipt.newRevision); adopting in the editor…")
            onChange()
            return applied
        } catch let f as LineProcessFailure {
            if f.isTransient || f.code == "storage_error" || f.code == "recovery_required" {
                // Persistence uncertainty: the handle is poisoned. Reopen and read what is on disk.
                note("edit ledger apply failed (\(f.text)); reopening the store to inspect its durable state")
                if await reopenLedger(), let tx = transactions[edit.editId], tx.edit == edit {
                    transactionTrace.append("ledger")
                    expectedApplication = (edit, durable?.text ?? afterText)
                    pendingTransaction = Transaction(edit: edit, receipt: tx.receipt, afterText: durable?.text ?? afterText)
                    setCapture(edit.captureId, .applied, "edit \(edit.editId) was committed before the failure; adopting the durable document")
                    onChange()
                    return .init(receipt: tx.receipt, document: durable ?? .init(projectId: edit.projectId, path: edit.path,
                                                                                    revision: tx.receipt.newRevision, text: afterText))
                }
                setCapture(edit.captureId, .proposed, "not inserted: edit ledger persistence failed (\(f.text)); retry approval")
                throw LedgerError.refused(f.text)
            }
            setCapture(edit.captureId, f.code == "capture_id_conflict" || f.code == "edit_id_conflict" ? .applied : .proposed,
                       "edit ledger refused \(edit.editId): \(f.text)")
            throw LedgerError.refused(f.text)
        }
    }

    /// The shell will replace the whole buffer with `text` (durable adoption);
    /// the resulting text change is not an ordinary edit.
    func expectAdoption(of text: String) { expectedAdoption = text }

    /// The editor's text change matched an armed adoption; consume it.
    func consumeAdoption(_ text: String) -> Bool {
        guard expectedAdoption == text else { return false }
        expectedAdoption = nil
        return true
    }

    /// The editor changed differently than the durable document it was asked to
    /// adopt. The durable store stays authoritative (the edit is committed and its
    /// receipt is still owed); the session is marked diverged so nothing else is
    /// persisted on top, and the next attach adopts the durable text and replays
    /// the receipt through reconciliation.
    func abandonExpectedApplication(_ why: String) {
        guard let expected = expectedApplication else { return }
        expectedApplication = nil
        ledgerError = "editor diverged from the durable document after \(expected.edit.editId) (\(why)); reattach to adopt it and replay the receipt"
        ledgerStatus = ledgerError!
        note(ledgerStatus)
        setCapture(expected.edit.captureId, .applied, "durable but not adopted in the editor: \(why); reattach to reconcile")
        onChange()
    }

    /// Step 3 (editor side): the editor adopted the durable document. Export the
    /// `.tex` when file-backed, then send the receipt. Never sends `document_edit`.
    func applicationApplied(newRevision: Int, afterText: String, sourceURL: URL?) {
        guard let expected = expectedApplication, var tx = pendingTransaction else { return }
        expectedApplication = nil
        shadow[expected.edit.path] = (newRevision, afterText)
        destination = nil // the insertion intersected (or sat exactly at) the pinned target; the bridge invalidated it
        if newRevision != tx.receipt.newRevision {
            note("editor revision \(newRevision) differs from the durable receipt revision \(tx.receipt.newRevision); the receipt is authoritative")
        }
        tx.sourceURL = sourceURL
        pendingTransaction = tx
        commitPendingTransaction()
    }

    /// The document was saved to `url`; a pending transaction without an export
    /// yet records it when the saved text is the durable state (the post-edit
    /// text or the current durable document after later typing).
    func sourceSaved(url: URL, text: String) {
        guard var tx = pendingTransaction, !tx.sourceWritten, text == tx.afterText || text == durable?.text else { return }
        tx.sourceURL = url
        tx.sourceWritten = true
        pendingTransaction = tx
        transactionTrace.append("source")
        if !tx.receiptSent { commitPendingTransaction() }
    }

    /// Retries the export/receipt steps of the pending transaction.
    @discardableResult
    func commitPendingTransaction() -> Bool {
        guard var tx = pendingTransaction, !tx.receiptSent else { return false }
        let edit = tx.edit
        // Export: the durable store is authoritative; a file-backed document is
        // re-exported atomically (current durable text, never an older snapshot)
        // before the receipt so the .tex never lags the store.
        if !tx.sourceWritten, let url = tx.sourceURL {
            let text = durable?.text ?? tx.afterText
            do {
                try text.write(to: url, atomically: true, encoding: .utf8)
                tx.sourceWritten = true
                transactionTrace.append("source")
                onSourceWritten(url, text)
            } catch {
                tx.lastFailure = "export to \(url.lastPathComponent) failed: \(error.localizedDescription)"
                pendingTransaction = tx
                status = "capture \(edit.captureId) durable; receipt withheld — \(tx.lastFailure!) (Edit > Retry Bridge Receipt)"
                setCapture(edit.captureId, .applied, status)
                onChange()
                return false
            }
        }
        tx.receiptSent = true
        pendingTransaction = tx
        transactionTrace.append("receipt")
        setCapture(edit.captureId, .applied, "applied as revision \(tx.receipt.newRevision); durable; confirming…")
        sendApplied(tx.receipt)
        return true
    }

    private func sendApplied(_ receipt: TransferV1.CaptureApplied) {
        client.send(.captureApplied, receipt, as: TransferV1.CaptureApplied.self) { [weak self] result in
            guard let self else { return }
            switch result {
            case .success(let ack):
                self.pendingTransaction = nil
                self.transactionTrace.append("confirmed")
                self.confirmDurably(ack)
                self.setCapture(receipt.captureId, .confirmed, "insertion confirmed (edit \(ack.editId), revision \(ack.newRevision))")
                self.status = "capture \(receipt.captureId) inserted and confirmed"
                self.flushDeferredEdits()
            case .failure(let f):
                // The helper keeps the transaction and its snapshot; reconciliation retries the receipt.
                if var tx = self.pendingTransaction { tx.receiptSent = false; tx.lastFailure = f.text; self.pendingTransaction = tx }
                self.setCapture(receipt.captureId, .applied, "durable locally; receipt not acknowledged (\(f.text))")
                self.status = "capture_applied failed — \(f.text)"
                if !f.isTransient { self.pendingTransaction = nil; self.flushDeferredEdits() }
            }
            self.onChange()
        }
    }

    /// Exact bridge acknowledgement → helper drops the recovery snapshot.
    private func confirmDurably(_ receipt: TransferV1.CaptureApplied) {
        guard let l = ledger else { return }
        l.send({ EditLedgerV1.ConfirmRequest(id: $0, receipt: receipt) }, as: EditLedgerV1.Confirmed.self) { [weak self] result in
            guard let self, self.ledger === l else { return }
            switch result {
            case .success: self.transactions[receipt.editId]?.confirmed = true; self.transactions[receipt.editId]?.documentBefore = nil
            case .failure(let f): self.note("edit ledger confirm \(receipt.editId) failed: \(f.text)")
            }
        }
    }

    private func flushDeferredEdits() {
        let edits = deferredEdits
        deferredEdits.removeAll()
        for edit in edits { sendEdit(edit) }
    }

    // MARK: restart reconciliation (contract step 5)

    enum ReconcileAction: Equatable {
        case confirmed(editId: String)
        case replayedReceipt(editId: String, newRevision: Int)
        /// The bridge has no (or a conflicting) durable record: the helper keeps
        /// the evidence; a person must resolve it (`resolveReconciliation`).
        case needsReconciliation(editId: String, why: String)
        /// Transport failure: nothing was concluded; retry reconciliation later.
        case retryLater(editId: String, why: String)
    }

    /// True after a reconciliation that could not reach a conclusion for every entry.
    private(set) var reconciliationIncomplete = false

    /// Ledger-guarded reconciliation (helper `recovery_export` → bridge
    /// `capture_status` per pending capture → `recovery_import` with the
    /// observations): the helper confirms exact `applied` receipts durably,
    /// returns `replay_receipt` (with the retained pre-edit snapshot) for edits
    /// the bridge only prepared, and keeps evidence for anything unavailable.
    /// The snapshot token refuses an import if source or ledger changed during
    /// the bridge round trip (edits while awaiting); we re-export and retry a
    /// bounded number of times. Missing/conflicting bridge records are reported
    /// as `needsReconciliation` (explicit resolution), transport failures as
    /// `retryLater`. Returns the actions and the minimum editor revision.
    func reconcile(path: String, currentSource: @escaping () -> (text: String, revision: Int),
                   statusTimeout: TimeInterval = 15, maxRounds: Int = 3) async -> (actions: [ReconcileAction], minimumRevision: Int) {
        var actions: [ReconcileAction] = []
        var minimum = currentSource().revision
        reconciliationIncomplete = false
        documentContext = .init(path: path, currentSource: currentSource, statusTimeout: statusTimeout)
        guard ledgerUsable, let l = ledger, durable != nil else {
            status = "edit ledger unavailable (\(ledgerError ?? ledgerStatus)); capture insertion disabled until it is repaired"
            reconciliationIncomplete = true
            onChange()
            return ([], minimum)
        }
        for round in 1...max(1, maxRounds) {
            let export: EditLedgerV1.RecoveryExport
            do { export = try await l.recoveryExport(timeout: statusTimeout) } catch {
                ledgerError = "recovery export failed — \((error as? LineProcessFailure)?.text ?? "\(error)")"
                reconciliationIncomplete = true
                onChange()
                return (actions, minimum)
            }
            guard ledger === l else { return (actions, minimum) }
            durable = export.currentDocument
            for tx in export.pendingReceipts { transactions[tx.edit.editId] = tx }
            let pending = export.pendingReceipts.filter { $0.edit.path == path && $0.edit.projectId == projectId
                && $0.edit.editId != pendingTransaction?.edit.editId } // a live transaction is not a restart case
            if pending.isEmpty { break }
            // Ask the bridge about every pending capture; classify for the helper and for the UI.
            var observations: [EditLedgerV1.BridgeObservation] = []
            var transient: [String: String] = [:], missing: [String: String] = [:]
            for tx in pending {
                let captureId = tx.edit.captureId
                do {
                    let st = try await client.request(.captureStatus, TransferV1.CaptureID(captureId: captureId),
                                                      as: TransferV1.CaptureStatus.self, timeout: statusTimeout)
                    guard ledger === l else { return (actions, minimum) }
                    if let applied = st.applied, applied.editId == tx.edit.editId, applied.newRevision == tx.receipt.newRevision {
                        observations.append(.applied(receipt: tx.receipt))
                    } else if st.applied == nil, st.prepared == tx.edit {
                        observations.append(.prepared(edit: tx.edit))
                    } else {
                        let why = st.applied != nil
                            ? "bridge holds a different receipt for \(captureId) (\(st.applied!.editId) @ \(st.applied!.newRevision)); evidence kept"
                            : "bridge holds no matching prepared edit for \(tx.edit.editId); evidence kept"
                        missing[tx.edit.editId] = why
                        observations.append(.unavailable(captureId: captureId, reason: why))
                    }
                } catch {
                    let f = (error as? LineProcessFailure) ?? .undecodable("\(error)")
                    if f.isTransient || !(f.code == "capture_missing" || f.code == "invalid_journal") {
                        transient[tx.edit.editId] = f.text
                    } else {
                        missing[tx.edit.editId] = "bridge has no usable durable record for \(captureId) (\(f.text)); evidence kept in the edit ledger for explicit reconciliation"
                    }
                    observations.append(.unavailable(captureId: captureId, reason: f.text))
                }
            }
            // Hand the observations back under the snapshot token; the helper decides durably.
            let plan: EditLedgerV1.RecoveryPlan
            do { plan = try await l.recoveryImport(snapshotToken: export.snapshotToken, observations: observations, timeout: statusTimeout) } catch {
                let f = (error as? LineProcessFailure) ?? .undecodable("\(error)")
                guard ledger === l else { return (actions, minimum) }
                if f.code == "stale_recovery_snapshot", round < maxRounds {
                    note("recovery snapshot changed during the bridge round trip (round \(round)); exporting again")
                    continue
                }
                ledgerError = f.code == "stale_recovery_snapshot" ? nil : "recovery import refused — \(f.text)"
                status = "reconciliation could not be recorded — \(f.text)"
                reconciliationIncomplete = true
                note(status)
                onChange()
                return (actions, minimum)
            }
            guard ledger === l else { return (actions, minimum) }
            durable = plan.recovery.currentDocument
            for tx in plan.recovery.pendingReceipts { transactions[tx.edit.editId] = tx }
            for action in plan.actions {
                switch action {
                case .confirmed(let receipt):
                    transactions[receipt.editId]?.confirmed = true
                    transactions[receipt.editId]?.documentBefore = nil
                    needsReconciliation[receipt.editId] = nil
                    minimum = max(minimum, receipt.newRevision + 1)
                    if capture(receipt.captureId) != nil { // a capture of this session (e.g. uncertain after a relaunch)
                        setCapture(receipt.captureId, .confirmed, "insertion confirmed by the bridge (edit \(receipt.editId), revision \(receipt.newRevision))")
                    }
                    actions.append(.confirmed(editId: receipt.editId))
                case .replayReceipt(let before, let receipt):
                    // Reopen the pre-edit snapshot on the bridge, replay the receipt, confirm locally on an
                    // exact acknowledgement; the caller resynchronizes the live source afterwards.
                    do {
                        try await open(path: path, revision: before.revision, text: before.text)
                        let ack = try await client.request(.captureApplied, receipt, as: TransferV1.CaptureApplied.self, timeout: statusTimeout)
                        guard ledger === l else { return (actions, minimum) }
                        guard ack == receipt else {
                            let why = "bridge acknowledged \(ack.editId) @ \(ack.newRevision), not the durable receipt \(receipt.newRevision); evidence kept"
                            needsReconciliation[receipt.editId] = why
                            actions.append(.needsReconciliation(editId: receipt.editId, why: why))
                            continue
                        }
                        _ = try await l.confirm(receipt)
                        transactions[receipt.editId]?.confirmed = true
                        transactions[receipt.editId]?.documentBefore = nil
                        needsReconciliation[receipt.editId] = nil
                        let now = currentSource()
                        shadow[path] = (receipt.newRevision, transactions[receipt.editId]?.documentAfterSha256 == SourceDigest.sha256Hex(now.text) ? now.text : before.text)
                        minimum = max(minimum, receipt.newRevision + 1)
                        setCapture(receipt.captureId, .confirmed, "receipt replayed after restart")
                        actions.append(.replayedReceipt(editId: receipt.editId, newRevision: receipt.newRevision))
                    } catch {
                        let f = fail("receipt replay", error)
                        reconciliationIncomplete = reconciliationIncomplete || f.isTransient
                        actions.append(.retryLater(editId: receipt.editId, why: "receipt replay refused (\(f.text)); the edit ledger keeps the transaction"))
                    }
                case .retryStatus(let captureId, let reason):
                    guard let editId = transactions.values.first(where: { $0.edit.captureId == captureId })?.edit.editId else { continue }
                    if let why = missing[editId] {
                        needsReconciliation[editId] = why
                        setCapture(captureId, .needsReselection, why)
                        actions.append(.needsReconciliation(editId: editId, why: why))
                    } else {
                        reconciliationIncomplete = true
                        note("capture_status \(captureId) failed (\(transient[editId] ?? reason)); transaction \(editId) kept for retry")
                        actions.append(.retryLater(editId: editId, why: transient[editId] ?? reason))
                    }
                }
            }
            break
        }
        onChange()
        return (actions, minimum)
    }

    /// Explicit human resolution of a transaction the bridge has no durable
    /// record for: the operator confirms the insertion is in the document, so
    /// the helper may drop its recovery snapshot. Evidence is only dropped here.
    func resolveReconciliation(editId: String, note why: String) async throws {
        guard let tx = transactions[editId], let l = ledger else { throw LedgerError.unusable("no such transaction") }
        _ = try await l.confirm(tx.receipt)
        transactions[editId]?.confirmed = true
        transactions[editId]?.documentBefore = nil
        needsReconciliation[editId] = nil
        note("transaction \(editId) resolved by operator: \(why)")
        onChange()
    }
}
