import AppKit
import CryptoKit
import ObjectiveC
import Observation
import UniformTypeIdentifiers
import FlashTeXProtocol

/// The on-disk file no longer matches what the editor last opened or saved.
/// Nothing was written; the buffer is kept. The UI resolves it explicitly
/// (`overwriteOnDisk`, `reloadFromDisk`, or keep editing).
struct DocumentConflict: Equatable {
    var url: URL
    var kind: ProjectFilesV1.ConflictKind
    /// Hash the editor last saw on disk (nil when it expected no file).
    var ours: String?
    /// Hash actually on disk now (nil when the file is gone).
    var theirs: String?
    var size: Int?
    var mtimeUnixMs: Int?
    /// Detected by the rooted helper (locked compare-and-replace) or by the
    /// direct Foundation path (best-effort read-compare-write, not locked).
    var viaHelper: Bool

    var summary: String {
        let what: String = switch kind {
        case .modifiedExternally: "was modified on disk since it was opened"
        case .deletedExternally: "was deleted on disk since it was opened"
        case .alreadyExists: "appeared on disk although this buffer was never saved there"
        case .modifiedDuringSave: "changed while it was being saved"
        }
        let by = viaHelper ? "" : " (direct file access: check is best-effort, not locked)"
        return "\(url.lastPathComponent) \(what)\(by); your buffer is kept unsaved. Overwrite, reload, or keep editing."
    }
}

/// File-layer state of the shell: which backend performs reads/saves, its
/// human-readable status, and the explicit conflict the UI must resolve.
/// Stored on the model as an associated object so `DocumentFiles.swift` stays a
/// self-contained extension file (parent may fold it into `ShellModel` as a
/// stored `var files = DocumentFilesState()`).
///
/// Backends: the rooted `flashtex-project-files` helper (one child process per
/// project directory; locked, symlink-refusing, hash-checked compare-and-replace)
/// or, when no helper binary exists, direct Foundation I/O with a best-effort
/// hash check. The helper is never bypassed once it exists: a helper failure
/// (timeout, crash, refusal) is reported and the buffer stays unsaved.
///
/// Helper calls are synchronous with a bounded wait (`helperTimeout`) because
/// `saveTex()` must answer the quit/open flows. A reply arriving after the wait
/// is *not* dropped: it is reconciled on the main actor (a late save receipt for
/// the same file marks that text as on disk; a late conflict is surfaced) so a
/// lost reply never desynchronizes the dirty state, and the helper is restarted
/// before the next request because its replies are strictly in order.
@MainActor
@Observable
final class DocumentFilesState {
    enum Backend: Equatable {
        case helper(URL)
        case direct(reason: String)
    }
    /// How the helper binary is chosen; tests pin a fake or disable it.
    enum HelperPolicy: Equatable {
        case discover
        case disabled(reason: String)
        case executable(URL, arguments: [String])
    }

    var policy: HelperPolicy = .discover
    /// How long an `offline` `resolve_packages` may take (tests shorten it).
    var packagesOfflineTimeout: TimeInterval = 10
    /// Backend of the most recent file operation (nil before any).
    private(set) var backend: Backend?
    private(set) var status = "no file operation yet"
    var conflict: DocumentConflict?
    /// Durable unsaved-text snapshots found for the open project that still
    /// differ from disk (DirtySnapshots.swift); the UI offers Restore/Discard.
    var offeredSnapshots: [DirtySnapshot] = []
    /// Result of the last `status` query for the open document.
    private(set) var lastDiskState: ProjectFilesV1.DiskState?
    /// Bounded wait for one helper reply.
    var helperTimeout: TimeInterval = 10
    /// Notes about replies that arrived after their wait expired (tests read these).
    private(set) var lateReplies: [String] = []
    private(set) var helperExits = 0
    private(set) var helperRestarts = 0
    /// Runs on the main actor after a save's *late* reply (receipt, conflict or
    /// failure) has been reconciled, i.e. once the request that kept
    /// `helperBusy` true is no longer outstanding. The shell uses it to re-run
    /// a file-watcher check it had to drop while the save was in flight (#831).
    @ObservationIgnored var onSaveSettledLate: (@MainActor () -> Void)?
    /// Runs on the main actor once the last background save in flight
    /// (`saveInBackground`) was applied: the shell re-arms an autosave the
    /// timer asked for meanwhile (`ShellModel.rearmDeferredAutosave`).
    @ObservationIgnored var onBackgroundSavesDrained: (@MainActor () -> Void)?

    @ObservationIgnored fileprivate var client: ProjectFilesClient?
    @ObservationIgnored private let replyQueue = DispatchQueue(label: "flashtex.project-files.replies")
    /// A second helper process for package resolution (ProjectPackages.swift),
    /// bound to the same root but never shared with reads and saves: the
    /// helper answers in request order and a fetch may take seconds, so it
    /// must never sit in front of a save. Launched on first use.
    @ObservationIgnored private var packagesClient: ProjectFilesClient?

    enum Outcome<T> {
        case reply(T)
        case failed(LineProcessFailure)
        case timedOut(TimeInterval)
    }

    enum ReadResult: Equatable {
        case text(String)
        case missing
        case failed(String)
    }

    struct StatusFailure: Error, Equatable {
        var reason: String
        init(_ reason: String) { self.reason = reason }
    }

    enum SaveResult: Equatable {
        case saved(sha256: String)
        case conflict(DocumentConflict)
        case failed(String)
    }

    private enum Acquired {
        case client(ProjectFilesClient)
        case direct(String)
        case unavailable(String)
    }

    /// Records a disk state observed outside a probe (an open, a reload) so
    /// the UI sees one field either way.
    func noteDiskState(_ state: ProjectFilesV1.DiskState?) { lastDiskState = state }

    var helperRunning: Bool { client?.isRunning == true }
    /// A request is still unanswered (its reply is reconciled late); a new
    /// request now would restart the helper and lose that reply.
    var helperBusy: Bool { (client?.outstanding ?? 0) > 0 }
    var usesHelper: Bool { if case .helper = backend { true } else { false } }
    var helperRoot: URL? { client?.root }

    /// Stops the helper process (if any); the next operation respawns it.
    func detachHelper() {
        settleBackgroundSaves()
        client?.terminate()
        client = nil
    }

    private func note(_ s: String) {
        status = s
        FlashTeXLog.write("files: " + s)
    }

    private func executable() -> (URL, [String])? {
        switch policy {
        case .disabled: return nil
        case .executable(let url, let args): return (url, args)
        case .discover: return ProjectFilesClient.locate().map { ($0, []) }
        }
    }

    /// The project root the helper is bound to for `url`: its directory with
    /// symlinks resolved (the helper refuses a symlinked root; components above
    /// the root are the caller's choice).
    static func root(for url: URL) -> URL {
        url.standardizedFileURL.deletingLastPathComponent().resolvingSymlinksInPath()
    }

    /// Every operation, a background save's too, first settles the
    /// background saves in flight (`settleBackgroundSaves`): a helper with an
    /// unanswered request is restarted, which would lose their replies. The
    /// autosave sends its saves one after another, so its own never wait here.
    private func acquire(for url: URL) -> Acquired {
        settleBackgroundSaves()
        if case .disabled(let reason) = policy {
            backend = .direct(reason: reason)
            return .direct(reason)
        }
        guard let (exe, args) = executable() else {
            let reason = "no flashtex-project-files helper (set FLASHTEX_PROJECT_FILES or build crates/project-files); direct file access with best-effort conflict checks"
            backend = .direct(reason: reason)
            return .direct(reason)
        }
        let root = Self.root(for: url)
        if let client {
            let sameBinary = client.executable == exe && client.arguments == args
            if client.isRunning, sameBinary, client.root == root, client.outstanding == 0 { return .client(client) }
            let why = !client.isRunning ? "exited" : !sameBinary ? "helper binary changed" : client.root != root ? "bound to \(client.root.path)" : "\(client.outstanding) unanswered request(s)"
            FlashTeXLog.write("files: restarting helper (\(why))")
            client.terminate()
            self.client = nil
            helperRestarts += 1
        }
        do {
            let client = try ProjectFilesClient(executable: exe, arguments: args, root: root, queue: replyQueue) { [weak self] event in
                DispatchQueue.main.async { [weak self] in
                    MainActor.assumeIsolated { self?.handle(event) }
                }
            }
            self.client = client
            backend = .helper(exe)
            return .client(client)
        } catch {
            let reason = "helper \(exe.lastPathComponent) failed to launch: \(error.localizedDescription)"
            backend = .helper(exe)
            note(reason)
            return .unavailable(reason)
        }
    }

    private func handle(_ event: ProjectFilesClient.Event) {
        switch event {
        case .stderr(let s): FlashTeXLog.write("files helper stderr: " + s.trimmingCharacters(in: .newlines))
        case .protocolViolation(let s): note("helper protocol violation: \(s)")
        case .unsolicited(let id, let type, _): FlashTeXLog.write("files helper: unsolicited \(type) reply \(id ?? "-")")
        case .exited(let code):
            helperExits += 1
            note("project-files helper exited (\(code)); it restarts on the next file operation")
            if client?.isRunning != true { client = nil }
        }
    }

    /// Sends one request and waits at most `helperTimeout` for its reply on the
    /// calling (main) thread. A reply after the deadline goes to `late` on the
    /// main actor instead of being dropped.
    private func roundTrip<T>(_ send: (@escaping (Result<T, LineProcessFailure>) -> Void) -> Void,
                              late: @escaping @MainActor (Result<T, LineProcessFailure>) -> Void) -> Outcome<T> {
        let box = ReplyBox<T>()
        let sem = DispatchSemaphore(value: 0)
        let timeout = helperTimeout
        send { result in
            let deliverLate: Bool = box.lock.withLock {
                if box.settled { return true }
                box.result = result
                box.settled = true
                return false
            }
            if deliverLate {
                DispatchQueue.main.async { MainActor.assumeIsolated { late(result) } }
            } else {
                sem.signal()
            }
        }
        _ = sem.wait(timeout: .now() + timeout)
        let result: Result<T, LineProcessFailure>? = box.lock.withLock {
            defer { box.settled = true }
            return box.result
        }
        switch result {
        case .success(let v): return .reply(v)
        case .failure(let f): return .failed(f)
        case nil: return .timedOut(timeout)
        }
    }

    private final class ReplyBox<T> {
        let lock = NSLock()
        var settled = false
        var result: Result<T, LineProcessFailure>?
    }

    // MARK: operations

    func read(_ url: URL) -> ReadResult {
        switch acquire(for: url) {
        case .direct(let reason):
            note(reason)
            return readDirect(url)
        case .unavailable(let reason):
            return .failed(reason)
        case .client(let client):
            let name = url.lastPathComponent
            let outcome: Outcome<ProjectFilesV1.Read> = roundTrip({ done in
                client.send({ ProjectFilesV1.ReadRequest(id: $0, path: name) }, as: ProjectFilesV1.Read.self, completion: done)
            }, late: { [weak self] result in
                self?.lateReplies.append("read \(name): \(Self.describe(result))")
                self?.note("late reply to read \(name) arrived after \(String(format: "%.1f", self?.helperTimeout ?? 0)) s; ignored (buffer untouched)")
            })
            switch outcome {
            case .reply(let r):
                note("rooted helper \(client.executable.lastPathComponent) at \(client.root.path)")
                guard r.exists, let text = r.text else { return .missing }
                return .text(text)
            case .failed(let f):
                note("helper read of \(name) failed: \(f.text)")
                return .failed(f.text)
            case .timedOut(let t):
                note("helper did not answer read of \(name) within \(String(format: "%.1f", t)) s; buffer untouched")
                return .failed("no reply from the project-files helper within \(String(format: "%.1f", t)) s")
            }
        }
    }

    /// The `flashtex.toml` governing `root` (ProjectManifest.swift), through
    /// the helper only: the shell has no TOML parser of its own, so without
    /// the helper there is no manifest — the reason is returned, and the
    /// caller falls back to what a directory says by itself.
    /// The helper's `set_fonts`: the manifest text with its `[fonts]` table
    /// replaced (ProjectFonts.swift saves it through `save`). Same helper
    /// binding and bounded wait as `manifest(for:entry:)`.
    /// `resolve_packages` on the dedicated packages helper (see
    /// `packagesClient`), awaited off the main thread: up to 30 s for a
    /// description, 120 s when `consent` fetches, 10 s `offline` (the
    /// library and the cache only; `libraries` lists every library too).
    func resolvePackages(for root: URL, names: [String], consent: Bool, entry: String, offline: Bool = false,
                         libraries: Bool = false) async -> Result<ProjectFilesV1.ResolvePackages, ProjectManifest.Failure> {
        switch acquirePackagesClient(for: root) {
        case .direct(let reason):
            note(reason)
            return .failure(.init("packages are resolved by the project-files helper, which is not available: \(reason)"))
        case .unavailable(let reason):
            return .failure(.init(reason))
        case .client(let client):
            do {
                return .success(try await client.resolvePackages(names: names, consent: consent, entry: entry, offline: offline, libraries: libraries,
                                                                 timeout: offline ? packagesOfflineTimeout : consent ? 120 : 30))
            } catch let f as LineProcessFailure {
                note("helper resolve_packages failed: \(f.text)")
                return .failure(.init(f.text))
            } catch {
                return .failure(.init(error.localizedDescription))
            }
        }
    }

    /// `set_packages`: the manifest's text with `fetch` and/or `pin`
    /// rewritten, for the caller to `save` (like `setFonts`).
    func setPackages(for root: URL, entry: String, fetch: String?, pin: [String: String]?) -> Result<ProjectFilesV1.SetPackages, ProjectManifest.Failure> {
        switch acquire(for: root.appendingPathComponent(ProjectManifest.fileName)) {
        case .direct(let reason):
            note(reason)
            return .failure(.init("flashtex.toml is written by the project-files helper, which is not available: \(reason)"))
        case .unavailable(let reason):
            return .failure(.init(reason))
        case .client(let client):
            let outcome: Outcome<ProjectFilesV1.SetPackages> = roundTrip({ done in
                client.send({ ProjectFilesV1.SetPackagesRequest(id: $0, entry: entry, fetch: fetch, pin: pin) }, as: ProjectFilesV1.SetPackages.self, completion: done)
            }, late: { [weak self] result in
                self?.lateReplies.append("set_packages: \(Self.describe(result))")
                self?.note("late reply to set_packages arrived after \(String(format: "%.1f", self?.helperTimeout ?? 0)) s; ignored")
            })
            switch outcome {
            case .reply(let m): return .success(m)
            case .failed(let f):
                note("helper set_packages failed: \(f.text)")
                return .failure(.init(f.text))
            case .timedOut(let t):
                note("helper did not answer set_packages within \(String(format: "%.1f", t)) s")
                return .failure(.init("no reply from the project-files helper within \(String(format: "%.1f", t)) s"))
            }
        }
    }

    /// `acquire` for the packages helper: the same binary and root rules,
    /// its own process, restarted only when the binary or root changed or
    /// it exited (an outstanding request is a fetch still running).
    private func acquirePackagesClient(for root: URL) -> Acquired {
        if case .disabled(let reason) = policy { return .direct(reason) }
        guard let (exe, args) = executable() else {
            return .direct("no flashtex-project-files helper (set FLASHTEX_PROJECT_FILES or build crates/project-files)")
        }
        let bound = Self.root(for: root.appendingPathComponent(ProjectManifest.fileName))
        if let packagesClient {
            if packagesClient.isRunning, packagesClient.executable == exe, packagesClient.arguments == args, packagesClient.root == bound { return .client(packagesClient) }
            packagesClient.terminate()
            self.packagesClient = nil
        }
        do {
            let client = try ProjectFilesClient(executable: exe, arguments: args, root: bound, queue: replyQueue) { event in
                if case .stderr(let s) = event { FlashTeXLog.write("packages helper stderr: " + s.trimmingCharacters(in: .newlines)) }
            }
            packagesClient = client
            return .client(client)
        } catch {
            return .unavailable("helper \(exe.lastPathComponent) failed to launch: \(error.localizedDescription)")
        }
    }

    func setFonts(for root: URL, entry: String, fonts: [String: String]) -> Result<ProjectFilesV1.SetFonts, ProjectManifest.Failure> {
        switch acquire(for: root.appendingPathComponent(ProjectManifest.fileName)) {
        case .direct(let reason):
            note(reason)
            return .failure(.init("flashtex.toml is written by the project-files helper, which is not available: \(reason)"))
        case .unavailable(let reason):
            return .failure(.init(reason))
        case .client(let client):
            let outcome: Outcome<ProjectFilesV1.SetFonts> = roundTrip({ done in
                client.send({ ProjectFilesV1.SetFontsRequest(id: $0, entry: entry, fonts: fonts) }, as: ProjectFilesV1.SetFonts.self, completion: done)
            }, late: { [weak self] result in
                self?.lateReplies.append("set_fonts: \(Self.describe(result))")
                self?.note("late reply to set_fonts arrived after \(String(format: "%.1f", self?.helperTimeout ?? 0)) s; ignored")
            })
            switch outcome {
            case .reply(let m): return .success(m)
            case .failed(let f):
                note("helper set_fonts failed: \(f.text)")
                return .failure(.init(f.text))
            case .timedOut(let t):
                note("helper did not answer set_fonts within \(String(format: "%.1f", t)) s")
                return .failure(.init("no reply from the project-files helper within \(String(format: "%.1f", t)) s"))
            }
        }
    }

    /// The governing manifest's text with `[project] mode` set (the helper's
    /// `set_mode`), for the caller to `save` (like `setFonts`).
    func setMode(for root: URL, entry: String, mode: String) -> Result<ProjectFilesV1.SetFonts, ProjectManifest.Failure> {
        switch acquire(for: root.appendingPathComponent(ProjectManifest.fileName)) {
        case .direct(let reason):
            note(reason)
            return .failure(.init("flashtex.toml is written by the project-files helper, which is not available: \(reason)"))
        case .unavailable(let reason):
            return .failure(.init(reason))
        case .client(let client):
            let outcome: Outcome<ProjectFilesV1.SetFonts> = roundTrip({ done in
                client.send({ ProjectFilesV1.SetModeRequest(id: $0, entry: entry, mode: mode) }, as: ProjectFilesV1.SetFonts.self, completion: done)
            }, late: { [weak self] result in
                self?.lateReplies.append("set_mode: \(Self.describe(result))")
                self?.note("late reply to set_mode arrived after \(String(format: "%.1f", self?.helperTimeout ?? 0)) s; ignored")
            })
            switch outcome {
            case .reply(let m): return .success(m)
            case .failed(let f):
                note("helper set_mode failed: \(f.text)")
                return .failure(.init(f.text))
            case .timedOut(let t):
                note("helper did not answer set_mode within \(String(format: "%.1f", t)) s")
                return .failure(.init("no reply from the project-files helper within \(String(format: "%.1f", t)) s"))
            }
        }
    }

    func manifest(for root: URL, entry: String) -> Result<ProjectFilesV1.Manifest, ProjectManifest.Failure> {
        // `acquire` binds the helper to the directory of the URL it is given.
        switch acquire(for: root.appendingPathComponent(ProjectManifest.fileName)) {
        case .direct(let reason):
            note(reason)
            return .failure(.init("flashtex.toml is read by the project-files helper, which is not available: \(reason)"))
        case .unavailable(let reason):
            return .failure(.init(reason))
        case .client(let client):
            let outcome: Outcome<ProjectFilesV1.Manifest> = roundTrip({ done in
                client.send({ ProjectFilesV1.ManifestRequest(id: $0, entry: entry) }, as: ProjectFilesV1.Manifest.self, completion: done)
            }, late: { [weak self] result in
                self?.lateReplies.append("manifest: \(Self.describe(result))")
                self?.note("late reply to manifest arrived after \(String(format: "%.1f", self?.helperTimeout ?? 0)) s; ignored")
            })
            switch outcome {
            case .reply(let m): return .success(m)
            case .failed(let f):
                note("helper manifest read failed: \(f.text)")
                return .failure(.init(f.text))
            case .timedOut(let t):
                note("helper did not answer the manifest read within \(String(format: "%.1f", t)) s")
                return .failure(.init("no reply from the project-files helper within \(String(format: "%.1f", t)) s"))
            }
        }
    }

    /// Compare-and-replace save. `expected` is what the editor last saw on disk;
    /// `force` overwrites regardless (only after an explicit user decision).
    /// `conflict`/`lastDiskState` describe the *entry* document: a project
    /// member's save passes `recordsState: false` and keeps its own record
    /// (`ProjectDocuments.saveConflicts`), so it can neither clear nor raise
    /// the entry's conflict.
    func save(_ url: URL, text: String, expected: ProjectFilesV1.Expected, force: Bool, recordsState: Bool = true,
              lateReceipt: @escaping @MainActor (String) -> Void = { _ in }) -> SaveResult {
        switch acquire(for: url) {
        case .direct(let reason):
            note(reason)
            return saveDirect(url, text: text, expected: expected, force: force, recordsState: recordsState)
        case .unavailable(let reason):
            return .failed(reason)
        case .client(let client):
            let name = url.lastPathComponent
            let sent = SourceDigest.sha256Hex(text)
            let outcome: Outcome<ProjectFilesV1.SaveOutcome> = roundTrip({ done in
                client.send({ ProjectFilesV1.SaveRequest(id: $0, path: name, text: text, expected: expected.wire, force: force) },
                            as: ProjectFilesV1.SaveOutcome.self, completion: done)
            }, late: { [weak self] result in
                self?.lateSaveReply(result, url: url, sent: sent, recordsState: recordsState, lateReceipt: lateReceipt)
            })
            return saveResult(outcome, url: url, sent: sent, recordsState: recordsState)
        }
    }

    /// A save's reply that came after its wait expired (`save`, `saveInBackground`).
    private func lateSaveReply(_ result: Result<ProjectFilesV1.SaveOutcome, LineProcessFailure>, url: URL, sent: String,
                               recordsState: Bool, lateReceipt: @MainActor (String) -> Void) {
        let name = url.lastPathComponent
        lateReplies.append("save \(name): \(Self.describe(result))")
        switch result {
        case .success(.saved(let receipt)) where receipt.sha256 == sent:
            note("late save receipt for \(name): the text sent \(Int(helperTimeout)) s ago is on disk (verified hash)")
            lateReceipt(sent)
        case .success(.saved(let receipt)):
            note("late save receipt for \(name) reports hash \(receipt.sha256.prefix(12)), not the text sent; buffer kept unsaved")
        case .success(.conflict(let c)):
            if recordsState { conflict = Self.conflict(url: url, c, viaHelper: true) }
            note("late reply for \(name): conflict; buffer kept unsaved")
        case .failure(let f):
            note("late reply for \(name): \(f.text); buffer kept unsaved")
        }
        onSaveSettledLate?()
    }

    /// What a save's reply (or its absence) means, recorded as `save` records it.
    private func saveResult(_ outcome: Outcome<ProjectFilesV1.SaveOutcome>, url: URL, sent: String, recordsState: Bool) -> SaveResult {
        let name = url.lastPathComponent
        switch outcome {
        case .reply(.saved(let receipt)):
            guard receipt.sha256 == sent else {
                note("helper receipt hash for \(name) does not match the text sent; treated as not saved")
                return .failed("save receipt hash mismatch for \(name)")
            }
            if recordsState {
                conflict = nil
                lastDiskState = .unchanged
            }
            note("saved \(name) via rooted helper (\(receipt.bytes) bytes, sha256 \(receipt.sha256.prefix(12)))")
            return .saved(sha256: receipt.sha256)
        case .reply(.conflict(let c)):
            let conflict = Self.conflict(url: url, c, viaHelper: true)
            if recordsState {
                self.conflict = conflict
                lastDiskState = c.kind == .deletedExternally ? .deleted : .modified
            }
            note(conflict.summary)
            return .conflict(conflict)
        case .failed(let f):
            note("helper save of \(name) failed: \(f.text); buffer kept unsaved")
            return .failed(f.text)
        case .timedOut(let t):
            note("helper did not confirm the save of \(name) within \(String(format: "%.1f", t)) s; buffer kept unsaved (a late receipt will be reconciled)")
            return .failed("no save receipt from the project-files helper within \(String(format: "%.1f", t)) s")
        }
    }

    // MARK: saves off the main thread (autosave)

    /// A save `saveInBackground` hashes, encodes, sends and awaits off the
    /// main thread; its outcome is applied on main exactly once (`settle`).
    private final class BackgroundSave: @unchecked Sendable {
        let url: URL
        let timeout: TimeInterval
        let deadline: DispatchTime
        /// Signalled when `reply` is set (never for a reply after `gaveUp`).
        let replied = DispatchSemaphore(value: 0)
        let lock = NSLock()
        /// Under `lock`: the sent text's hash (set before the request goes
        /// out), the reply, and whether its wait expired (a reply after that
        /// goes to the late path, as a main-thread save's does).
        var sent: String?
        var reply: Result<ProjectFilesV1.SaveOutcome, LineProcessFailure>?
        var gaveUp = false
        /// Main thread only.
        var settled = false
        let finish: @MainActor (Outcome<ProjectFilesV1.SaveOutcome>, _ sent: String) -> Void

        init(url: URL, timeout: TimeInterval, finish: @escaping @MainActor (Outcome<ProjectFilesV1.SaveOutcome>, String) -> Void) {
            self.url = url
            self.timeout = timeout
            self.deadline = .now() + timeout
            self.finish = finish
        }
    }

    /// Saves sent by `saveInBackground` whose outcome is not applied yet.
    @ObservationIgnored private var backgroundSaves: [BackgroundSave] = []
    /// Saves `saveInBackground` sent through the helper (tests, evidence).
    @ObservationIgnored private(set) var backgroundSavesSent = 0
    /// Hashing, encoding and sending a background save: never the main thread.
    nonisolated static let backgroundSaveQueue = DispatchQueue(label: "flashtex.project-files.background-save", qos: .userInitiated)

    /// A background save of `url` is in flight.
    func backgroundSaveInFlight(for url: URL) -> Bool { backgroundSaves.contains { $0.url == url } }
    var backgroundSavesInFlight: Bool { !backgroundSaves.isEmpty }

    /// `save` (never forced) for the timed autosave, without holding the
    /// main thread (lane APP-LAG-AUDIT): the text's hash, the expectation
    /// (`expectation`: say, the baseline text's hash), the JSON encoding and
    /// the wait for the helper's compare-and-replace (two fsyncs) all happen
    /// off it. At 3 MB that was tens of milliseconds of main thread at every
    /// typing pause, and up to `helperTimeout` with a slow helper.
    /// `completion` gets the result on main, recorded as `save` records it.
    /// Without the helper (the direct fallback) the save is made here, as `save`.
    ///
    /// Every main-thread file operation first settles the saves in flight
    /// (`settleBackgroundSaves`), so it never restarts the helper under an
    /// unanswered save, nor checks a baseline the save is about to move.
    ///
    /// `applies` is asked on main when the outcome lands: false (the window
    /// moved on meanwhile) records no conflict or disk state for it.
    func saveInBackground(_ url: URL, text: String, expectation: @escaping @Sendable () -> ProjectFilesV1.Expected,
                          recordsState: Bool = true, applies: @escaping @MainActor () -> Bool = { true },
                          lateReceipt: @escaping @MainActor (String) -> Void = { _ in },
                          completion: @escaping @MainActor (SaveResult) -> Void) {
        guard case .client(let client) = acquire(for: url) else {
            completion(save(url, text: text, expected: expectation(), force: false, recordsState: recordsState, lateReceipt: lateReceipt))
            return
        }
        let name = url.lastPathComponent
        let job = BackgroundSave(url: url, timeout: helperTimeout) { [weak self] outcome, sent in
            // Always answered, so no caller awaiting it (a member's ⌘S) hangs.
            guard let self else { return completion(.failed("the window closed before the save of \(name) was answered")) }
            completion(saveResult(outcome, url: url, sent: sent, recordsState: recordsState && applies()))
            onSaveSettledLate?() // a watcher check parked while this save was in flight runs now
            if backgroundSaves.isEmpty { onBackgroundSavesDrained?() }
        }
        backgroundSaves.append(job)
        backgroundSavesSent += 1
        let late: @MainActor (Result<ProjectFilesV1.SaveOutcome, LineProcessFailure>, String) -> Void = { [weak self] result, sent in
            guard let self else { return }
            lateSaveReply(result, url: url, sent: sent, recordsState: recordsState && applies(), lateReceipt: lateReceipt)
        }
        let settleOnMain: @MainActor () -> Void = { [weak self] in
            guard let self else {
                // The window's file state is gone: the outcome is answered as such, once.
                guard !job.settled else { return }
                job.settled = true
                return job.finish(.timedOut(job.timeout), "")
            }
            settle(job, waiting: false)
        }
        Self.backgroundSaveQueue.async {
            let sent = SourceDigest.sha256Hex(text)
            let expected = expectation()
            job.lock.withLock { job.sent = sent }
            client.send({ ProjectFilesV1.SaveRequest(id: $0, path: name, text: text, expected: expected.wire, force: false) },
                        as: ProjectFilesV1.SaveOutcome.self) { result in
                let isLate: Bool = job.lock.withLock {
                    if job.gaveUp { return true }
                    job.reply = result
                    return false
                }
                if isLate {
                    DispatchQueue.main.async { MainActor.assumeIsolated { late(result, sent) } }
                    return
                }
                job.replied.signal()
                // A run-loop block, not the main queue: AppKit drains that late under typing.
                RunLoop.main.perform(inModes: [.common]) { MainActor.assumeIsolated { settleOnMain() } }
                CFRunLoopWakeUp(CFRunLoopGetMain())
            }
        }
        DispatchQueue.main.asyncAfter(deadline: job.deadline) { MainActor.assumeIsolated { settleOnMain() } }
    }

    /// Applies `job`'s outcome once: its reply; or, once its deadline passed
    /// (or, `waiting`, when the reply did not come by then), a timeout whose
    /// reply then goes to the late path. Not due yet: nothing.
    private func settle(_ job: BackgroundSave, waiting: Bool) {
        guard !job.settled else { return }
        if waiting { _ = job.replied.wait(timeout: job.deadline) }
        let state: (reply: Result<ProjectFilesV1.SaveOutcome, LineProcessFailure>?, sent: String)? = job.lock.withLock {
            if let r = job.reply { return (r, job.sent ?? "") }
            guard waiting || DispatchTime.now() >= job.deadline else { return nil }
            job.gaveUp = true
            return (nil, job.sent ?? "")
        }
        guard let state else { return }
        job.settled = true
        backgroundSaves.removeAll { $0 === job }
        let outcome: Outcome<ProjectFilesV1.SaveOutcome>
        switch state.reply {
        case .success(let v)?: outcome = .reply(v)
        case .failure(let f)?: outcome = .failed(f)
        case nil: outcome = .timedOut(job.timeout)
        }
        job.finish(outcome, state.sent)
    }

    /// Settles every background save now, waiting (each at most until its
    /// own deadline) for replies still to come. Main-thread file operations,
    /// and the baselines they compare with, call it first. It waits only for
    /// an operation made inside a save's own window (a ⌘S just as the
    /// autosave fired), which then waits what it used to wait behind the
    /// main-thread save; the save's outcome is applied before the operation.
    ///
    /// Only the saves in flight on entry: a save an applied outcome starts
    /// (the autosave's next document) is not waited for, and the autosave
    /// starts its next one from a later run-loop turn anyway
    /// (`ShellModel.autosaveInBackground`), after the operation that settled.
    func settleBackgroundSaves() {
        for job in backgroundSaves { settle(job, waiting: true) }
    }

    /// Asks whether `url` still matches `expectedSha256` (nil: the editor expects no file).
    func diskStatus(_ url: URL, expectedSha256: String?) -> Result<ProjectFilesV1.Status, StatusFailure> {
        let name = url.lastPathComponent
        let result: Result<ProjectFilesV1.Status, StatusFailure>
        switch acquire(for: url) {
        case .direct(let reason):
            note(reason)
            result = .success(statusDirect(url, expectedSha256: expectedSha256))
        case .unavailable(let reason):
            result = .failure(.init(reason))
        case .client(let client):
            let outcome: Outcome<ProjectFilesV1.Status> = roundTrip({ done in
                client.send({ ProjectFilesV1.StatusRequest(id: $0, path: name, expectedSha256: expectedSha256) },
                            as: ProjectFilesV1.Status.self, completion: done)
            }, late: { [weak self] r in
                self?.lateReplies.append("status \(name): \(Self.describe(r))")
                self?.note("late status reply for \(name) ignored")
            })
            switch outcome {
            case .reply(let s): result = .success(s)
            case .failed(let f): result = .failure(.init(f.text))
            case .timedOut(let t): result = .failure(.init("no status reply from the project-files helper within \(String(format: "%.1f", t)) s"))
            }
        }
        if case .success(let s) = result { lastDiskState = s.state }
        return result
    }

    // MARK: direct (no helper) path

    private func readDirect(_ url: URL) -> ReadResult {
        do {
            return .text(try String(contentsOf: url, encoding: .utf8))
        } catch let e as NSError where e.domain == NSCocoaErrorDomain && e.code == NSFileReadNoSuchFileError {
            return .missing
        } catch {
            return .failed(error.localizedDescription)
        }
    }

    private func statusDirect(_ url: URL, expectedSha256: String?) -> ProjectFilesV1.Status {
        let name = url.lastPathComponent
        guard let data = try? Data(contentsOf: url) else {
            return .init(path: name, exists: false, state: expectedSha256 == nil ? .unchanged : .deleted)
        }
        let sha = SourceDigest.sha256Hex(data)
        let state: ProjectFilesV1.DiskState = expectedSha256 == nil ? .created : (expectedSha256 == sha ? .unchanged : .modified)
        let mtime = (try? FileManager.default.attributesOfItem(atPath: url.path)[.modificationDate] as? Date)
            .map { Int($0.timeIntervalSince1970 * 1000) }
        return .init(path: name, exists: true, state: state, sha256: sha, bytes: data.count, mtimeUnixMs: mtime)
    }

    private func saveDirect(_ url: URL, text: String, expected: ProjectFilesV1.Expected, force: Bool, recordsState: Bool) -> SaveResult {
        /// The conflict `expected` names against the file as it is right now.
        func conflictNow() -> DocumentConflict? {
            let current = statusDirect(url, expectedSha256: nil)
            var kind: ProjectFilesV1.ConflictKind?
            var ours: String?
            switch expected {
            case .any: break
            case .newFile: if current.exists { kind = .alreadyExists }
            case .hash(let h):
                ours = h
                if !current.exists { kind = .deletedExternally } else if current.sha256 != h { kind = .modifiedExternally }
            }
            return kind.map {
                DocumentConflict(url: url, kind: $0, ours: ours, theirs: current.sha256,
                                 size: current.bytes, mtimeUnixMs: current.mtimeUnixMs, viaHelper: false)
            }
        }
        func refuse(_ conflict: DocumentConflict) -> SaveResult {
            if recordsState {
                self.conflict = conflict
                lastDiskState = conflict.kind == .deletedExternally ? .deleted : .modified
            }
            note(conflict.summary)
            return .conflict(conflict)
        }
        if !force, let conflict = conflictNow() { return refuse(conflict) }
        do {
            // Write a sibling temp file first, then check the expectation again
            // immediately before the rename: an external change made while the
            // bytes were written is refused instead of replaced (autosave runs
            // this every quiet moment). Still unlocked — a change landing
            // between that re-check and the rename is not caught — but a
            // `.newFile` expectation is exact (`RENAME_EXCL`).
            let temp = url.deletingLastPathComponent()
                .appendingPathComponent(".\(url.lastPathComponent).flashtex-save-\(UUID().uuidString)")
            defer { try? FileManager.default.removeItem(at: temp) } // also after a partial write; a no-op once renamed
            func posixError(_ code: Int32) -> Error {
                CocoaError(.fileWriteUnknown, userInfo: [NSUnderlyingErrorKey: POSIXError(POSIXErrorCode(rawValue: code) ?? .EIO)])
            }
            // Replacing a file: the temp starts private (0600) and then takes the
            // file's own mode, ACL and extended attributes, so a 0600 file is never
            // world-readable, not even in between. A new file keeps the umask mode.
            var target = stat()
            let replacing = stat(url.path, &target) == 0
            if replacing, target.st_flags & UInt32(UF_IMMUTABLE | SF_IMMUTABLE) != 0 {
                // A locked (Finder "Locked"/uchg) file cannot be replaced; say so
                // before any temp file exists rather than failing the rename.
                note("direct save of \(url.lastPathComponent) refused: the file is locked")
                return .failed("\(url.lastPathComponent) is locked; unlock it in Finder (Get Info) to save")
            }
            let fd = open(temp.path, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, replacing ? 0o600 : 0o666)
            guard fd >= 0 else { throw posixError(errno) }
            let handle = FileHandle(fileDescriptor: fd, closeOnDealloc: true)
            try handle.write(contentsOf: Data(text.utf8))
            try handle.close()
            if replacing {
                // ACL and extended attributes only: `COPYFILE_SECURITY` implies
                // `COPYFILE_STAT`, which would also copy the OLD mtime/atime (make,
                // latexmk and git then miss a same-size edit) and the lock flags.
                // Some volumes (SMB, exFAT) refuse the copy; that is tolerated.
                _ = copyfile(url.path, temp.path, nil, copyfile_flags_t(COPYFILE_ACL | COPYFILE_XATTR))
                // The mode is required: a private file stays private.
                guard chmod(temp.path, target.st_mode & 0o7777) == 0 else { throw posixError(errno) }
            }
            if !force, let conflict = conflictNow() { return refuse(conflict) }
            let exclusive = !force && expected == .newFile
            let renamed = exclusive ? renamex_np(temp.path, url.path, UInt32(RENAME_EXCL)) : rename(temp.path, url.path)
            let renameErrno = errno // before any further I/O
            if renamed != 0 {
                if exclusive, renameErrno == EEXIST, let conflict = conflictNow() { return refuse(conflict) }
                throw posixError(renameErrno)
            }
            if recordsState {
                conflict = nil
                lastDiskState = .unchanged
            }
            note("saved \(url.lastPathComponent) directly (no helper: best-effort conflict check, not locked)")
            return .saved(sha256: SourceDigest.sha256Hex(text))
        } catch {
            note("direct save of \(url.lastPathComponent) failed: \(error.localizedDescription)")
            return .failed(error.localizedDescription)
        }
    }

    private static func conflict(url: URL, _ c: ProjectFilesV1.Conflict, viaHelper: Bool) -> DocumentConflict {
        .init(url: url, kind: c.kind, ours: c.ours, theirs: c.theirs, size: c.size, mtimeUnixMs: c.mtimeUnixMs, viaHelper: viaHelper)
    }

    private static func describe<T>(_ r: Result<T, LineProcessFailure>) -> String {
        switch r {
        case .success(let v): "\(v)"
        case .failure(let f): f.text
        }
    }
}

extension SourceDigest {
    static func sha256Hex(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }
}

/// Open/save of the active `.tex` document. The project is single-entry for
/// now: the opened file becomes `main.tex` in the compile request (the compiler
/// only compiles the entry document), and its URL is remembered for saving.
/// Reads and saves go through `files` (rooted helper or direct fallback) with an
/// explicit conflict state: a save never silently overwrites a file that changed
/// on disk since it was opened, and the buffer is never lost to a lost reply.
extension ShellModel {
    static let texType = UTType(filenameExtension: "tex") ?? .plainText

    private static var filesKey = 0
    /// File-layer state (see `DocumentFilesState`).
    var files: DocumentFilesState {
        if let existing = objc_getAssociatedObject(self, &Self.filesKey) as? DocumentFilesState { return existing }
        let state = DocumentFilesState()
        state.onBackgroundSavesDrained = { [weak self] in self?.rearmDeferredAutosave() }
        objc_setAssociatedObject(self, &Self.filesKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }

    func openTexPanel() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [Self.texType, .plainText]
        panel.canChooseDirectories = true // a project folder: its flashtex.toml names the entry (ProjectManifest.swift)
        panel.message = "Open a LaTeX source file as the entry document, or a project folder (its flashtex.toml names the entry)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        // In the app: the document's own window if it is open, else this
        // window if it holds nothing yet, else a new window; this project
        // is never replaced, so there is nothing to save first (ProjectWindows.swift).
        if let windows { windows.open(url, from: self); return }
        guard hasUnsavedDocuments else { openTex(at: url); return }
        let alert = NSAlert()
        alert.messageText = "Save changes to \(unsavedDocumentsDescription) before opening \(url.lastPathComponent)?"
        alert.informativeText = "Discarded text stays recoverable: \(discardRecoveryRoutes)"
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Discard")
        alert.addButton(withTitle: "Cancel")
        switch alert.runModal() {
        case .alertFirstButtonReturn:
            if documentURL == nil, !saveTexAs() { return }
            if openTex(at: url, dirty: .saveFirst) == .saveFailed, files.conflict != nil { resolveConflictPanel() }
        case .alertSecondButtonReturn: openTex(at: url, dirty: .discard)
        default: break
        }
    }

    /// Opens `url` for the user (File › Open, New Project): through the
    /// window list when the model has one (`ProjectWindows.open`: never
    /// twice, a new window unless this one holds nothing), else in place
    /// with `dirty`. The outcome and the model now showing the file.
    func openForUser(_ url: URL, dirty: DirtyDisposition = .none) -> (outcome: OpenOutcome, model: ShellModel) {
        guard let windows else { return (openTex(at: url, dirty: dirty), self) }
        let (result, model) = windows.open(url, from: self)
        if case .failed(let outcome) = result { return (outcome, self) }
        return (.opened, model ?? self)
    }

    /// What a caller authorized for the unsaved buffer before opening another file.
    enum DirtyDisposition: Equatable { case none, saveFirst, discard }

    enum OpenOutcome: Equatable { case opened, blockedByUnsavedEdits, saveFailed, readFailed }

    /// The last buffer replaced by an authorized open, kept so an accidental
    /// discard remains recoverable within the session.
    struct RecoverableBuffer: Equatable { var url: URL?; var text: String }

    /// Every document with unsaved edits (entry and members, active or not):
    /// what replacing the project would drop (#786).
    ///
    /// A background autosave in flight is settled first (it waits for its
    /// reply, as the main-thread save did), so a Save/Discard question, a
    /// refused Open or Reload, never counts edits that save is writing.
    var hasUnsavedDocuments: Bool {
        files.settleBackgroundSaves()
        return isDirty || project.anyDirty
    }

    /// Names the dirty documents for a Save/Discard prompt.
    var unsavedDocumentsDescription: String {
        let dirty = project.listing.filter(\.isDirty).map(\.path)
        if documentURL == nil || dirty.isEmpty { return documentURL?.lastPathComponent ?? "the unsaved buffer" }
        return dirty.joined(separator: ", ")
    }

    /// Where each dirty document's text goes on a discard (#806): the entry's
    /// buffer to Edit > Restore Discarded Buffer (this session), each member's
    /// snapshot to File > Restore Unsaved Snapshot…. That item is enabled only
    /// once the member is open again under its project, so the hint says so
    /// (#811). A buffer with no file (and so no project folder) is kept for
    /// the session only, and the hint says that too rather than promising a
    /// snapshot that was never written.
    var discardRecoveryRoutes: String {
        let dirty = project.listing.filter(\.isDirty).map(\.path)
        var routes: [String] = []
        let entry = project.entryPath
        if dirty.contains(entry) {
            routes.append("\(entry) via Edit > Restore Discarded Buffer (this session"
                + (documentURL == nil ? " only: it has no file, so no snapshot is kept)" : ")"))
        }
        let members = dirty.filter { $0 != entry }
        if !members.isEmpty {
            let names = members.joined(separator: ", ")
            if project.projectRoot != nil {
                routes.append("\(names) via File > Restore Unsaved Snapshot… once \(entry) and \(names) are open again")
            } else {
                routes.append("\(names) discarded without a snapshot (no project folder)")
            }
        }
        return routes.joined(separator: "; ")
    }

    /// Whether a project replacement (open, fixture load, reload, New Project)
    /// may proceed. It drops *every* document, so every dirty one counts, not
    /// just the active tab (#786): `.none` refuses, `.saveFirst` saves the
    /// entry and each dirty member and refuses on the first failure or
    /// conflict (nothing replaced), `.discard` proceeds with the entry's buffer
    /// to keep (`keepDiscarded` also snapshots each dirty member).
    enum ReplacementAuthorization: Equatable { case proceed(discarding: RecoverableBuffer?), refused(OpenOutcome) }

    func authorizeProjectReplacement(_ dirty: DirtyDisposition, before action: String) -> ReplacementAuthorization {
        guard hasUnsavedDocuments else { return .proceed(discarding: nil) }
        switch dirty {
        case .none:
            captureNote = "\(unsavedDocumentsDescription) \(project.listing.filter(\.isDirty).count > 1 ? "have" : "has") unsaved edits; save or discard before \(action)."
            return .refused(.blockedByUnsavedEdits)
        case .saveFirst:
            if project.isDirty(project.entryPath) {
                guard documentURL != nil, saveTex() else {
                    captureNote = "Could not save the current buffer (\(files.status)); nothing replaced before \(action)."
                    return .refused(.saveFailed)
                }
            }
            for path in documents.map(\.path) where path != project.entryPath && project.isDirty(path) {
                guard case .saved = project.saveDocumentNow(path) else {
                    captureNote = "Could not save \(path) (\(project.status)); nothing replaced before \(action)."
                    return .refused(.saveFailed)
                }
            }
            return .proceed(discarding: nil)
        case .discard:
            return .proceed(discarding: RecoverableBuffer(url: documentURL, text: entryText)) // the entry's buffer, whichever tab is active
        }
    }

    /// Consumes a discard decision just before the project is replaced: the
    /// entry's buffer goes to `recoverableBuffer` (and a durable snapshot), and
    /// every dirty member gets its own durable snapshot under its rooted file
    /// (File > Restore Unsaved Snapshot…) — never another document's text.
    /// A member's snapshot is its only copy once the project is replaced: if
    /// one cannot be written, nothing is kept or replaced and this returns
    /// false with a `captureNote` (#806).
    /// A failure after earlier members were kept withdraws their snapshots
    /// again (`rollBackSnapshots`, #811): nothing was discarded, so nothing
    /// may be offered as discarded later.
    func keepDiscarded(_ discarding: RecoverableBuffer, reason: String, before action: String) -> Bool {
        if let root = project.projectRoot {
            var touched: [SnapshotRollback] = []
            var kept: [String] = []
            for doc in documents where doc.path != project.entryPath && project.isDirty(doc.path) {
                let url = root.appendingPathComponent(doc.path)
                let previous = dirtySnapshots.read(for: url)
                switch preserveDiscardedText(doc.text, at: url, reason: reason) {
                case .kept:
                    touched.append(.init(url: url, previous: previous))
                    kept.append(doc.path)
                case .clean:
                    touched.append(.init(url: url, previous: previous))
                case .failed(let why):
                    rollBackSnapshots(touched)
                    captureNote = "Could not keep the unsaved edits of \(doc.path) (\(why)); nothing replaced before \(action)"
                        + (kept.isEmpty ? "" : ", and the snapshots just kept for \(kept.joined(separator: ", ")) were withdrawn")
                        + ". The text is still open in the editor."
                    return false
                }
            }
        }
        // Session memory is one slot; the store keeps one per file across
        // sessions. A clean entry (only members dirty) replaces neither.
        if project.isDirty(project.entryPath) {
            recoverableBuffer = discarding
            if let from = discarding.url { preserveDirtyText(discarding.text, at: from, reason: reason) }
        }
        return true
    }

    /// Opens a `.tex` file as the entry document. When any document is dirty,
    /// nothing is replaced unless the caller passes an explicit disposition
    /// (`authorizeProjectReplacement`): `.saveFirst` writes every dirty
    /// document (or refuses), `.discard` replaces them but keeps their text.
    @discardableResult
    func openTex(at url: URL, dirty: DirtyDisposition = .none) -> OpenOutcome {
        // A folder opens as the document its manifest names — or its only
        // `.tex` file — resolved before anything is replaced (ProjectManifest.swift).
        var isDirectory: ObjCBool = false
        if FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory), isDirectory.boolValue {
            switch manifest.resolveEntry(in: url) {
            case .success(let entry): return openTex(at: entry, dirty: dirty)
            case .failure(let f):
                captureNote = "Could not open \(url.lastPathComponent): \(f.why)"
                return .readFailed
            }
        }
        let discarding: RecoverableBuffer?
        switch authorizeProjectReplacement(dirty, before: "opening \(url.lastPathComponent)") {
        case .refused(let outcome): return outcome
        case .proceed(let kept): discarding = kept
        }
        switch files.read(url) {
        case .text(let text):
            let routes = discarding == nil ? nil : discardRecoveryRoutes
            guard adoptOpenedText(text, url: url, discarding: discarding) else { return .saveFailed }
            captureNote = "Opened \(url.lastPathComponent) (\(text.utf8.count) bytes)"
                + (routes.map { "; discarded text kept: \($0)" } ?? (recoverableBuffer == nil ? "" : "; previous unsaved buffer kept (Edit > Restore Discarded Buffer)"))
            // A snapshot kept by an earlier session (or an earlier discard) of
            // this file is offered, never applied: the disk text is what opened.
            files.offeredSnapshots = []
            if let offered = offerDirtySnapshot(for: url, currentText: text) {
                captureNote! += "; unsaved text from before is available (\(offered.reason); File > Restore Unsaved Snapshot…)"
            }
            return .opened
        case .missing:
            captureNote = "Could not open \(url.lastPathComponent): no such file"
            return .readFailed
        case .failed(let reason):
            captureNote = "Could not open \(url.lastPathComponent): \(reason)"
            return .readFailed
        }
    }

    /// Replaces the project with `text` read from `url` (open or direct reload).
    /// Only a successful read consumes a discard decision: a failed open leaves
    /// the dirty buffer in place, not "discarded"; so does a discard whose
    /// text cannot be kept (false).
    private func adoptOpenedText(_ text: String, url: URL, discarding: RecoverableBuffer?) -> Bool {
        if let discarding {
            let reload = discarding.url == url
            guard keepDiscarded(discarding, reason: reload ? "discarded by a reload from disk" : "discarded when \(url.lastPathComponent) was opened",
                                before: (reload ? "reloading " : "opening ") + url.lastPathComponent) else { return false }
        }
        // The project's manifest is read before the engine session hears of
        // the new project (its mode depends on it: UnicodeFonts.swift).
        engineChoicePending = true
        replaceProject(entryText: text, named: url.lastPathComponent)
        documentURL = url
        savedText = text
        files.conflict = nil
        files.noteDiskState(.unchanged)
        watchOpenDocument() // DocumentWatcher.swift: live external-change detection
        manifest.refresh() // ProjectManifest.swift: the flashtex.toml governing this project, before the first compile
        engineHostNeedsUnicode = nil // the new engine's report on the document before this open (UnicodeFonts.swift)
        unicodeFontsScan = nil
        resolveEngineForOpenedDocument() // the record, the previous-engine notice, the `engine:` line (EngineChoice.swift)
        engineChoicePending = false
        // Now the engine opens the project (stored pages, compile).
        if engineV3Enabled, let at = engineV3OpenedAt { engineV3.projectChanged(model: self, openedAt: at) }
        return true
    }

    /// Restores the buffer discarded by the last authorized open (undo of the
    /// discard decision); the currently open file is left untouched on disk.
    @discardableResult
    func restoreDiscardedBuffer() -> Bool {
        guard let kept = recoverableBuffer else { return false }
        if hasUnsavedDocuments {
            captureNote = "Current buffer has unsaved edits; save it before restoring the discarded buffer."
            return false
        }
        replaceProject(entryText: kept.text, named: kept.url?.lastPathComponent ?? "main.tex")
        documentURL = kept.url
        // "Saved" is whatever is on disk now, so the restored text stays dirty
        // (it differs from disk) and cannot be lost again silently. No file on
        // disk means the next save expects a new file.
        savedText = kept.url.flatMap { url -> String? in
            if case .text(let t) = files.read(url) { return t }
            return nil
        }
        files.conflict = nil
        recoverableBuffer = nil
        watchOpenDocument()
        captureNote = "Restored the discarded buffer (\(kept.text.utf8.count) bytes, unsaved)."
        return true
    }

    /// Dirty means the buffer differs from what was last opened/saved. A fresh
    /// fixture-seeded buffer (no file, never saved) counts as dirty only once edited.
    var isDirty: Bool {
        // A non-entry document compares with the text it was opened with
        // (ProjectDocuments); the entry with its saved text.
        if activePath != project.entryPath { return project.isDirty(activePath) }
        if let savedText { return !savedText.sameBytes(as: activeText) }
        // A file URL with no saved text: a restored buffer whose file is
        // gone (or unreadable) — no file holds it, so it is unsaved.
        return (documentURL != nil || editorRevision > 1) && !activeText.isEmpty
    }

    /// Hash of the text the editor last saw on disk for the open document.
    var baselineSha256: String? { savedText.map { SourceDigest.sha256Hex($0) } }

    /// What a save to `url` must find on disk: the baseline for the open
    /// document (or no file, if it was never on disk), anything for Save As
    /// (the panel already confirmed a replacement).
    private func expectedOnDisk(for url: URL) -> ProjectFilesV1.Expected {
        files.settleBackgroundSaves() // an autosave in flight may be about to move the baseline
        guard url == documentURL else { return .any }
        return baselineSha256.map { .hash($0) } ?? .newFile
    }

    /// The timed autosave of the entry document: `saveEntryTex`, made off
    /// the main thread (`DocumentFilesState.saveInBackground`), the baseline
    /// hashed there too; the outcome is applied on main, as `write` applies
    /// it, when it comes. Nothing when the window opened another document
    /// meanwhile. Returns whether a save was started; `completion` runs
    /// after its outcome was applied.
    @discardableResult
    func saveEntryTexInBackground(completion: @escaping @MainActor () -> Void = {}) -> Bool {
        guard let url = documentURL, let entry = documents.first(where: { $0.path == project.entryPath }),
              !files.backgroundSaveInFlight(for: url) else { return false }
        let text = entry.text, baseline = savedText, generation = projectGeneration
        files.saveInBackground(url, text: text, expectation: { baseline.map { .hash(SourceDigest.sha256Hex($0)) } ?? .newFile },
                               applies: { [weak self] in self?.projectGeneration == generation && self?.documentURL == url },
                               lateReceipt: entryLateReceipt(url: url, text: text)) { [weak self] result in
            defer { completion() }
            guard let self else { return }
            guard projectGeneration == generation, documentURL == url else {
                FlashTeXLog.write("files: an autosave of \(url.lastPathComponent) answered after the window moved on; not applied")
                return
            }
            if case .conflict(let c) = result, c.kind == .deletedExternally {
                // As `write`: recreated, only while still absent (rare: here, on main).
                files.conflict = nil
                let again = files.save(url, text: text, expected: .newFile, force: false, lateReceipt: entryLateReceipt(url: url, text: text))
                _ = applyEntrySave(again, url: url, text: text, recreated: true)
                return
            }
            _ = applyEntrySave(result, url: url, text: text, recreated: false)
        }
        return true
    }

    /// Saves the entry document (whichever tab is active). On a conflict returns false, sets
    /// `files.conflict`, keeps the buffer, and writes nothing.
    @discardableResult
    func saveTex() -> Bool {
        guard let url = documentURL else { return saveTexAs() }
        return write(to: url, expected: expectedOnDisk(for: url), force: false)
    }

    /// Saves a non-entry project member and reports the outcome in the
    /// footer note — saved, conflict summary, or failure reason — instead of
    /// leaving a failed or conflicted save with no feedback. Shared by
    /// `saveTexInteractive` and the tab bar's/Project menu's "Save <path>"
    /// items (DocumentTabBar.swift), which previously discarded the
    /// `ProjectDocuments.SaveOutcome`.
    func saveDocumentInteractive(_ path: String) async {
        await enqueueSave(path) { [weak self] in // after any autosave of `path` still in flight
            guard let self else { return }
            switch project.saveDocumentNow(path) {
            case .saved(let p, _): captureNote = "Saved \(p)"
            case .conflict(let c):
                captureNote = c.summary
                if !Self.runningUnderXCTest { resolveDocumentConflictPanel(path) } // ⌘S asks, as for the entry
            case .failed(let why): captureNote = "Save of \(path) failed: \(why)"
            }
        }.value
    }

    /// `saveTex()` for the entry document whichever document is active
    /// (autosave after a tab switch). Never opens a Save panel.
    @discardableResult
    func saveEntryTex() -> Bool {
        guard let url = documentURL, let entry = documents.first(where: { $0.path == project.entryPath }) else { return false }
        return write(to: url, text: entry.text, expected: expectedOnDisk(for: url), force: false)
    }

    /// Menu-driven save: on a conflict, asks the user how to resolve it.
    func saveTexInteractive() {
        projectPackages.explicitRequest(compiling: false) // ProjectPackages.swift: ⌘S ends the typing grace period
        // A non-entry document saves to its own rooted file (never to the
        // entry URL): ProjectDocuments.saveDocumentNow, the rooted
        // compare-and-replace, conflicts reported the same way.
        if activePath != project.entryPath {
            let path = activePath
            Task { @MainActor [weak self] in
                await self?.saveDocumentInteractive(path)
            }
            return
        }
        if !saveTex(), files.conflict != nil { resolveConflictPanel() }
    }

    @discardableResult
    func saveTexAs() -> Bool {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [Self.texType]
        panel.nameFieldStringValue = documentURL?.lastPathComponent ?? "main.tex"
        guard panel.runModal() == .OK, let url = panel.url else { return false }
        return saveTexAs(to: url)
    }

    /// Save As to a chosen `url` (the panel's answer). Writes the entry buffer.
    /// With other project documents open it is refused when the destination is
    /// one of their files (the entry text would replace that member on disk) or
    /// lies in another directory (the project root, which the open members'
    /// paths resolve against, would move out from under them). The entry keeps
    /// its tab path, as for any Save As.
    @discardableResult
    func saveTexAs(to url: URL) -> Bool {
        let members = documents.map(\.path).filter { $0 != project.entryPath }
        if !members.isEmpty, let root = project.projectRoot {
            if let member = members.first(where: { Self.sameFile(root.appendingPathComponent($0), url) }) {
                captureNote = "Save As refused: \(member) is open in this project; saving the entry over it would replace that document on disk."
                return false
            }
            if !Self.sameFile(url.deletingLastPathComponent(), root) {
                captureNote = "Save As refused: \(members.count) other project document\(members.count == 1 ? " is" : "s are") open relative to \(root.lastPathComponent)/; close them or save into the same folder."
                return false
            }
        }
        return write(to: url, expected: expectedOnDisk(for: url), force: false)
    }

    /// Whether two URLs name the same file or directory: the file system's
    /// identity when both exist (a hard link, or different case on a
    /// case-insensitive volume, is the same file), else the standardized,
    /// symlink-resolved path compared case-insensitively (a conservative
    /// refusal on a case-sensitive volume, never a missed match).
    static func sameFile(_ a: URL, _ b: URL) -> Bool {
        let key: Set<URLResourceKey> = [.fileResourceIdentifierKey]
        if let ia = try? URL(fileURLWithPath: a.path).resourceValues(forKeys: key).fileResourceIdentifier,
           let ib = try? URL(fileURLWithPath: b.path).resourceValues(forKeys: key).fileResourceIdentifier {
            return ia.isEqual(ib)
        }
        func folded(_ u: URL) -> String { u.resolvingSymlinksInPath().standardizedFileURL.path.lowercased() }
        return folded(a) == folded(b)
    }

    /// Resolves a conflict by writing the buffer over whatever is on disk.
    /// Only after an explicit user decision; never called automatically.
    @discardableResult
    func overwriteOnDisk() -> Bool {
        guard let url = files.conflict?.url ?? documentURL else { captureNote = "No file to overwrite."; return false }
        guard activePath == project.entryPath else { // `write` sends `activeText`: never another document's text
            captureNote = "Not overwritten: switch to \(project.entryPath) to resolve its on-disk conflict."
            return false
        }
        return write(to: url, expected: .any, force: true)
    }

    // MARK: reviewed reload

    /// What a reload would do, read from disk *before* the user confirms:
    /// the snapshot's identity (hash) is what the reload is then pinned to, so
    /// a file that changes again between review and confirmation is refused
    /// rather than silently imported. Built by `prepareReload()`.
    struct ReloadReview: Equatable {
        var url: URL
        var currentText: String
        var diskText: String
        var diskSha256: String
        /// The entry's buffer has unsaved edits the reload replaces.
        var entryDirty: Bool
        /// Dirty members a direct reload discards too (it replaces the whole
        /// project); named in the prompt (#806).
        var discardedMembers: [String] = []
        var bufferDirty: Bool { entryDirty || !discardedMembers.isEmpty }
        var identical: Bool { currentText.sameBytes(as: diskText) }
        var bytesBefore: Int { currentText.utf8.count }
        var bytesAfter: Int { diskText.utf8.count }
        /// Line-multiset difference (lines present on disk but not in the
        /// buffer, and vice versa) — a review summary, not a positional diff.
        var linesAdded: Int { ShellModel.lineChanges(from: currentText, to: diskText).added }
        var linesRemoved: Int { ShellModel.lineChanges(from: currentText, to: diskText).removed }

        var summary: String {
            let name = url.lastPathComponent
            if identical { return "\(name) on disk is identical to the buffer (\(bytesAfter) bytes); reloading changes nothing." }
            let change = ShellModel.lineChanges(from: currentText, to: diskText)
            var edits = entryDirty ? "Your unsaved edits are replaced (recoverable this session via Edit > Restore Discarded Buffer). " : ""
            if !discardedMembers.isEmpty {
                edits += "Unsaved edits to \(discardedMembers.joined(separator: ", ")) are discarded too (recoverable via File > Restore Unsaved Snapshot… once they are open again). "
            }
            return "Reload \(name) directly: \(bytesBefore) → \(bytesAfter) bytes, +\(change.added) / −\(change.removed) lines. \(edits)"
                + "The reload is pinned to the reviewed snapshot (sha256 \(diskSha256.prefix(12))) and refused if the file changes again."
        }
    }

    /// Counts lines present in `new` but not `old` (added) and in `old` but
    /// not `new` (removed) as multisets: O(n), order-insensitive.
    nonisolated static func lineChanges(from old: String, to new: String) -> (added: Int, removed: Int) {
        var counts: [Substring: Int] = [:]
        for line in old.split(separator: "\n", omittingEmptySubsequences: false) { counts[line, default: 0] += 1 }
        var added = 0
        for line in new.split(separator: "\n", omittingEmptySubsequences: false) {
            if let n = counts[line], n > 0 { counts[line] = n - 1 } else { added += 1 }
        }
        let removed = counts.values.reduce(0, +)
        return (added, removed)
    }

    /// Reads the file a reload would import and describes the change. nil (with
    /// a `captureNote`) when there is nothing readable on disk. Never changes
    /// the buffer.
    func prepareReload() -> ReloadReview? {
        guard let url = files.conflict?.url ?? documentURL else { captureNote = "No file to reload."; return nil }
        switch files.read(url) {
        case .missing:
            captureNote = "Nothing to reload: \(url.lastPathComponent) does not exist on disk."
            return nil
        case .failed(let reason):
            captureNote = "Could not read \(url.lastPathComponent) for reload: \(reason)"
            return nil
        case .text(let diskText):
            // A reload replaces the whole project (every dirty member).
            let members = project.listing.filter { $0.isDirty && $0.path != project.entryPath }.map(\.path)
            return ReloadReview(url: url, currentText: activeText, diskText: diskText, diskSha256: SourceDigest.sha256Hex(diskText),
                                entryDirty: project.isDirty(project.entryPath), discardedMembers: members)
        }
    }

    /// Applies a reviewed reload. A dirty buffer is replaced only with
    /// `.discard` (kept in `recoverableBuffer`); `.saveFirst` is refused
    /// because a file you are about to reload is not one to save over. The
    /// file is re-read directly and refused if it no longer hashes to the
    /// reviewed snapshot.
    @discardableResult
    func confirmReload(_ review: ReloadReview, dirty: DirtyDisposition = .none) async -> OpenOutcome {
        var discarding: RecoverableBuffer?
        // A reload replaces the whole project (every member's edits).
        if hasUnsavedDocuments {
            switch dirty {
            case .none:
                captureNote = "\(unsavedDocumentsDescription) has unsaved edits; discard them explicitly to reload."
                return .blockedByUnsavedEdits
            case .saveFirst:
                captureNote = "Cannot save over \(review.url.lastPathComponent) while reloading it; overwrite or discard instead."
                return .saveFailed
            case .discard:
                discarding = RecoverableBuffer(url: documentURL, text: entryText)
            }
        }
        return directReload(review, discarding: discarding)
    }

    /// Reviewed reload without a modal: `prepareReload()` then `confirmReload`.
    @discardableResult
    func reloadFromDiskReviewed(dirty: DirtyDisposition = .none) async -> OpenOutcome {
        guard let review = prepareReload() else { return .readFailed }
        return await confirmReload(review, dirty: dirty)
    }

    /// Synchronous reviewed reload (`prepareReload()` then the direct reload).
    @discardableResult
    func reloadFromDisk(dirty: DirtyDisposition = .none) -> OpenOutcome {
        guard let review = prepareReload() else { return .readFailed }
        let dirtyNow = hasUnsavedDocuments // a direct reload replaces the whole project
        if dirtyNow {
            switch dirty {
            case .none:
                captureNote = "\(unsavedDocumentsDescription) has unsaved edits; discard them explicitly to reload."
                return .blockedByUnsavedEdits
            case .saveFirst:
                captureNote = "Cannot save over \(review.url.lastPathComponent) while reloading it; overwrite or discard instead."
                return .saveFailed
            case .discard: break
            }
        }
        return directReload(review, discarding: dirtyNow ? RecoverableBuffer(url: documentURL, text: entryText) : nil)
    }

    private func directReload(_ review: ReloadReview, discarding: RecoverableBuffer?) -> OpenOutcome {
        let url = review.url
        switch files.read(url) {
        case .text(let text):
            guard SourceDigest.sha256Hex(text) == review.diskSha256 else {
                captureNote = "\(url.lastPathComponent) changed again after the reload was reviewed; nothing replaced. Review again."
                files.noteDiskState(.modified)
                return .readFailed
            }
            let routes = discarding == nil ? nil : discardRecoveryRoutes
            guard adoptOpenedText(text, url: url, discarding: discarding) else { return .saveFailed }
            captureNote = "Reloaded \(url.lastPathComponent) from disk"
                + (routes.map { "; discarded text kept: \($0)." } ?? (recoverableBuffer == nil ? "." : "; previous buffer kept (Edit > Restore Discarded Buffer)."))
            return .opened
        case .missing:
            captureNote = "\(url.lastPathComponent) disappeared after the reload was reviewed; nothing replaced."
            return .readFailed
        case .failed(let reason):
            captureNote = "Could not reload \(url.lastPathComponent): \(reason)"
            return .readFailed
        }
    }

    /// Reviewed reload behind a modal: shows the change summary, then imports
    /// only on "Reload". Never automatic.
    func reloadFromDiskInteractive() {
        guard let review = prepareReload() else { return }
        let alert = NSAlert()
        alert.messageText = "Reload \(review.url.lastPathComponent) from disk?"
        alert.informativeText = review.summary
        alert.addButton(withTitle: "Reload")
        alert.addButton(withTitle: "Cancel")
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        Task { @MainActor [weak self] in
            guard let self else { return }
            await confirmReload(review, dirty: review.bufferDirty ? .discard : .none)
        }
    }

    /// Asks the file layer (project-files helper or direct) whether the open
    /// document still matches its baseline and records an explicit conflict if
    /// not (before any save is attempted).
    @discardableResult
    func checkDiskStatus() -> ProjectFilesV1.DiskState? {
        files.settleBackgroundSaves() // the baseline compared with is the one after an autosave in flight
        guard let url = documentURL else { return nil }
        switch files.diskStatus(url, expectedSha256: baselineSha256) {
        case .failure(let failure):
            noteProbeFailure(url, failure.reason)
            return nil
        case .success(let s):
            applyDiskState(s.state, url: url, sha256: s.sha256, bytes: s.bytes, mtimeUnixMs: s.mtimeUnixMs, viaHelper: files.usesHelper)
            return s.state
        }
    }

    /// `checkDiskStatus()` for the watcher and app activation, which call it
    /// from a task.
    @discardableResult
    func refreshDiskStatus() async -> ProjectFilesV1.DiskState? {
        checkDiskStatus()
    }

    /// Shows a save confirmation and remembers it (see `lastSaveConfirmation`).
    func noteSaveConfirmation(_ note: String, for url: URL) {
        captureNote = note
        lastSaveConfirmation = (url, note, Date())
    }

    /// A disk-status probe of `url` failed (helper timeout, unreadable file…).
    /// Normally that is the footer note; but when the note still shows the
    /// confirmation of a save of the same file that landed moments ago, the
    /// failure is only logged — the probe usually *was* that save's own write
    /// firing the watcher, and "Could not check paper.tex on disk" would tell
    /// the user the opposite of what happened (#831). One guard, no priorities:
    /// any other note, another file, or a probe long after the save shows the
    /// failure as before.
    func noteProbeFailure(_ url: URL, _ reason: String) {
        let failure = "Could not check \(url.lastPathComponent) on disk: \(reason)"
        if let last = lastSaveConfirmation, last.url == url, captureNote == last.note,
           Date().timeIntervalSince(last.at) < saveConfirmationGrace {
            FlashTeXLog.write("files: \(failure) (keeping the save confirmation shown \(String(format: "%.1f", Date().timeIntervalSince(last.at))) s ago)")
            return
        }
        captureNote = failure
    }

    /// How long after a save confirmation a failed probe of the same file stays
    /// out of the footer: the probe itself may block for `helperTimeout`, and
    /// the watcher retries once after `max(helperTimeout, 0.5)` s.
    private var saveConfirmationGrace: TimeInterval { 2 * max(files.helperTimeout, 0.5) + 5 }

    private func applyDiskState(_ state: ProjectFilesV1.DiskState, url: URL, sha256: String?, bytes: Int?, mtimeUnixMs: Int?, viaHelper: Bool) {
        switch state {
        case .unchanged:
            if files.conflict?.url == url { files.conflict = nil }
        case .created:
            // The editor expected no file; one appeared. Saving would be
            // refused (`alreadyExists`), so say so now.
            files.conflict = DocumentConflict(url: url, kind: .alreadyExists, ours: nil, theirs: sha256, size: bytes,
                                              mtimeUnixMs: mtimeUnixMs, viaHelper: viaHelper)
            captureNote = files.conflict?.summary
        case .deleted:
            // Nothing to overwrite: not a blocking conflict; Save recreates it.
            if files.conflict?.url == url { files.conflict = nil }
            captureNote = "\(url.lastPathComponent) was deleted on disk; Save will recreate it from the buffer."
        case .modified:
            files.conflict = DocumentConflict(url: url, kind: .modifiedExternally, ours: baselineSha256, theirs: sha256,
                                              size: bytes, mtimeUnixMs: mtimeUnixMs, viaHelper: viaHelper)
            captureNote = files.conflict?.summary
            // Both texts are now recoverable: the disk text through the reviewed
            // reload, the unsaved buffer durably.
            if url == documentURL, project.isDirty(project.entryPath) {
                preserveDirtyText(entryText, at: url, reason: "file changed on disk while the buffer was unsaved")
            }
        }
    }

    /// Modal resolution of `files.conflict`: Overwrite / Reload / Keep Editing.
    /// The reload is reviewed: the alert shows what the disk snapshot would
    /// change and the import is pinned to exactly that snapshot.
    func resolveConflictPanel() {
        // A member with its own save conflict (active tab) resolves per document.
        if activePath != project.entryPath, project.saveConflict(for: activePath) != nil {
            resolveDocumentConflictPanel(activePath)
            return
        }
        guard let conflict = files.conflict else { return }
        // Overwrite and Reload act on the active buffer (`activeText`): with
        // another tab active (a queued save that answered after a switch, the
        // menu item) they would write that document's text into the entry.
        guard activePath == project.entryPath else {
            captureNote = conflict.summary + " Switch to \(project.entryPath) to resolve it."
            return
        }
        let review = prepareReload()
        let alert = NSAlert()
        alert.messageText = "\(conflict.url.lastPathComponent) changed on disk"
        alert.informativeText = conflict.summary + "\n\n" + (review?.summary ?? (captureNote ?? "Nothing on disk to reload."))
        alert.addButton(withTitle: "Overwrite")
        alert.addButton(withTitle: "Reload")
        alert.addButton(withTitle: "Keep Editing")
        alert.buttons[1].isEnabled = review != nil
        switch alert.runModal() {
        case .alertFirstButtonReturn: overwriteOnDisk()
        case .alertSecondButtonReturn:
            guard let review else { return }
            Task { @MainActor [weak self] in
                guard let self else { return }
                await confirmReload(review, dirty: review.bufferDirty ? .discard : .none)
            }
        default: break
        }
    }

    /// Modal resolution of a member's save conflict
    /// (`ProjectDocuments.saveConflict(for:)`): Overwrite writes its buffer
    /// over the disk file, Reload takes the disk text (the unsaved edits kept
    /// as a snapshot first), Keep Editing leaves both as they are. Without
    /// it a member's conflict could never be resolved: every save and the
    /// quit flow's Save were refused until Don't Save.
    func resolveDocumentConflictPanel(_ path: String) {
        guard let conflict = project.saveConflict(for: path) else { return }
        let alert = NSAlert()
        alert.messageText = "\(path) changed on disk"
        alert.informativeText = conflict.summary + "\n\nOverwrite replaces the file with your text. Reload replaces your text with the file's; your unsaved edits are kept (File > Restore Unsaved Snapshot…)."
        alert.addButton(withTitle: "Overwrite")
        alert.addButton(withTitle: "Reload")
        alert.addButton(withTitle: "Keep Editing")
        switch alert.runModal() {
        case .alertFirstButtonReturn:
            switch project.overwriteDocumentOnDisk(path) {
            case .saved(let p, _): captureNote = "Overwrote \(p) on disk"
            case .conflict(let c): captureNote = c.summary
            case .failed(let why): captureNote = "Overwrite of \(path) failed: \(why)"
            }
        case .alertSecondButtonReturn:
            captureNote = project.reloadDocumentFromDisk(path).map { "Reload of \(path) refused: \($0)" } ?? "Reloaded \(path) from disk"
        default: break
        }
    }

    private func write(to url: URL, text: String? = nil, expected: ProjectFilesV1.Expected, force: Bool) -> Bool {
        // `url` is the entry's file (Save, Save As, Overwrite): write the entry
        // buffer, never `activeText` — with a member tab active that would put
        // the member's text into main.tex (open/fixture "save first" flows).
        let text = text ?? entryText
        let lateReceipt = entryLateReceipt(url: url, text: text)
        var result = files.save(url, text: text, expected: expected, force: force, lateReceipt: lateReceipt)
        var recreated = false
        if case .conflict(let c) = result, c.kind == .deletedExternally, !force {
            // Nothing on disk can be overwritten: recreate the file, but only if
            // it is still absent (a file appearing meanwhile is `alreadyExists`).
            files.conflict = nil
            recreated = true
            result = files.save(url, text: text, expected: .newFile, force: false, lateReceipt: lateReceipt)
        }
        return applyEntrySave(result, url: url, text: text, recreated: recreated)
    }

    /// A save of the entry's `text` to `url` confirmed after its wait expired.
    private func entryLateReceipt(url: URL, text: String) -> @MainActor (String) -> Void {
        { [weak self] sha in
            // The helper confirmed, after our wait expired, that exactly `text`
            // is on disk at `url`: that text is the new baseline. Edits made
            // meanwhile keep the buffer dirty; a different open file is untouched.
            guard let self, self.documentURL == url, SourceDigest.sha256Hex(text) == sha else { return }
            self.savedText = text
            self.noteSaveConfirmation("Late confirmation: \(url.lastPathComponent) was saved" + (self.isDirty ? " (buffer edited since; still unsaved)." : "."), for: url)
            self.bridgeSourceSaved(url: url, text: text)
        }
    }

    /// What a save of the entry's `text` to `url` did (`write`, the background autosave).
    private func applyEntrySave(_ result: DocumentFilesState.SaveResult, url: URL, text: String, recreated: Bool) -> Bool {
        switch result {
        case .saved:
            let before = documentURL
            // Save As / a first save: no engine open of the new path before
            // its manifest is read (cleared by engineDocumentSaved).
            if before?.standardizedFileURL != url.standardizedFileURL { engineChoicePending = true }
            documentURL = url
            engineDocumentSaved(from: before) // EngineChoice.swift: Save As / first save reads the new project's manifest, records the file
            savedText = text
            noteSaveConfirmation("Saved \(url.lastPathComponent)" + (recreated ? " (recreated; it had been deleted on disk)" : ""), for: url)
            bridgeSourceSaved(url: url, text: text)
            snapshotSaved(url: url, text: text)
            watchOpenDocument() // Save As moves the watch; a replaced inode is re-opened
            return true
        case .conflict(let conflict):
            captureNote = conflict.summary
            // The buffer now (a background save's `text` may be older: typed on while it was out).
            if url == documentURL { preserveDirtyText(entryText, at: url, reason: "save refused: file changed on disk") }
            return false
        case .failed(let reason):
            captureNote = "Save failed: \(reason)"
            return false
        }
    }
}
