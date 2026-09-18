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

    @ObservationIgnored fileprivate var client: ProjectFilesClient?
    @ObservationIgnored private let replyQueue = DispatchQueue(label: "flashtex.project-files.replies")

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

    /// Records a disk state observed through another route (the preview
    /// controller's `file_status`) so the UI sees one field either way.
    func noteDiskState(_ state: ProjectFilesV1.DiskState?) { lastDiskState = state }

    var helperRunning: Bool { client?.isRunning == true }
    /// A request is still unanswered (its reply is reconciled late); a new
    /// request now would restart the helper and lose that reply.
    var helperBusy: Bool { (client?.outstanding ?? 0) > 0 }
    var usesHelper: Bool { if case .helper = backend { true } else { false } }
    var helperRoot: URL? { client?.root }

    /// Stops the helper process (if any); the next operation respawns it.
    func detachHelper() {
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

    private func acquire(for url: URL) -> Acquired {
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
                guard let self else { return }
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
            })
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
        objc_setAssociatedObject(self, &Self.filesKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }

    func openTexPanel() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [Self.texType, .plainText]
        panel.message = "Open a LaTeX source file as the entry document"
        guard panel.runModal() == .OK, let url = panel.url else { return }
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

    /// What a caller authorized for the unsaved buffer before opening another file.
    enum DirtyDisposition: Equatable { case none, saveFirst, discard }

    enum OpenOutcome: Equatable { case opened, blockedByUnsavedEdits, saveFailed, readFailed }

    /// The last buffer replaced by an authorized open, kept so an accidental
    /// discard remains recoverable within the session.
    struct RecoverableBuffer: Equatable { var url: URL?; var text: String }

    /// Every document with unsaved edits (entry and members, active or not):
    /// what replacing the project would drop (#786).
    var hasUnsavedDocuments: Bool { isDirty || project.anyDirty }

    /// Names the dirty documents for a Save/Discard prompt.
    var unsavedDocumentsDescription: String {
        let dirty = project.listing.filter(\.isDirty).map(\.path)
        if documentURL == nil || dirty.isEmpty { return documentURL?.lastPathComponent ?? "the unsaved buffer" }
        return dirty.joined(separator: ", ")
    }

    /// Where each dirty document's text goes on a discard (#806): the entry's
    /// buffer to Edit > Restore Discarded Buffer (this session), each member's
    /// snapshot to File > Restore Unsaved Snapshot….
    var discardRecoveryRoutes: String {
        let dirty = project.listing.filter(\.isDirty).map(\.path)
        var routes: [String] = []
        if dirty.contains(project.entryPath) { routes.append("\(project.entryPath) via Edit > Restore Discarded Buffer (this session)") }
        let members = dirty.filter { $0 != project.entryPath }
        if !members.isEmpty { routes.append("\(members.joined(separator: ", ")) via File > Restore Unsaved Snapshot…") }
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
    func keepDiscarded(_ discarding: RecoverableBuffer, reason: String, before action: String) -> Bool {
        if let root = project.projectRoot {
            for doc in documents where doc.path != project.entryPath && project.isDirty(doc.path) {
                guard preserveDiscardedText(doc.text, at: root.appendingPathComponent(doc.path), reason: reason) else {
                    captureNote = "Could not keep the unsaved edits of \(doc.path) (\(dirtySnapshots.lastError ?? "snapshot store not writable")); nothing replaced before \(action). The text is still open in the editor."
                    return false
                }
            }
        }
        // Session memory is one slot; the store keeps one per file across
        // sessions (the ledger route keeps it in undo history instead). A
        // clean entry (only members dirty) replaces neither.
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
        replaceProject(entryText: text, named: url.lastPathComponent)
        documentURL = url
        savedText = text
        files.conflict = nil
        files.noteDiskState(.unchanged)
        watchOpenDocument() // DocumentWatcher.swift: live external-change detection
        if workerAttached { compile() }
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
        if workerAttached { compile() }
        return true
    }

    /// Dirty means the buffer differs from what was last opened/saved. A fresh
    /// fixture-seeded buffer (no file, never saved) counts as dirty only once edited.
    var isDirty: Bool {
        // A non-entry document compares with the text it was opened with
        // (ProjectDocuments); the entry with its saved text.
        if activePath != project.entryPath { return project.isDirty(activePath) }
        if let savedText { return !savedText.sameBytes(as: activeText) }
        return documentURL == nil && editorRevision > 1 && !activeText.isEmpty
    }

    /// Hash of the text the editor last saw on disk for the open document.
    var baselineSha256: String? { savedText.map { SourceDigest.sha256Hex($0) } }

    /// What a save to `url` must find on disk: the baseline for the open
    /// document (or no file, if it was never on disk), anything for Save As
    /// (the panel already confirmed a replacement).
    private func expectedOnDisk(for url: URL) -> ProjectFilesV1.Expected {
        guard url == documentURL else { return .any }
        return baselineSha256.map { .hash($0) } ?? .newFile
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
            switch await project.saveDocument(path) {
            case .saved(let p, _): captureNote = "Saved \(p)"
            case .conflict(let c): captureNote = c.summary
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
        // A non-entry document saves to its own rooted file (never to the
        // entry URL): ProjectDocuments.saveDocument, helper export or rooted
        // compare-and-replace, conflicts reported the same way.
        if activePath != project.entryPath {
            let path = activePath
            Task { @MainActor [weak self] in
                await self?.saveDocumentInteractive(path)
            }
            return
        }
        // With the durable helper rooted in the open file's project the export
        // goes through its rooted, locked save so the ledger text and the .tex
        // never diverge; the result is reported asynchronously (never blocks
        // the UI). A helper rooted elsewhere (a session copy of a buffer whose
        // file is not named `main.tex`) would export that copy, not the file.
        if controllerRoutesFiles {
            enqueueSave(project.entryPath) { [weak self] in // coalesced with an autosave still in flight
                guard let self else { return }
                switch await saveEntryRouted() {
                case .saved?: break
                // The panel only opens while the entry is still active; after a
                // tab switch the conflict lands in the footer note instead.
                case .conflict?: resolveConflictPanel()
                case .failed(let why)?: captureNote = "Save through the preview controller failed: \(why)"
                case nil: if files.conflict != nil { resolveConflictPanel() } // switched away meanwhile: saved directly
                }
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
    /// its tab path, as for any Save As (`controllerRoutesFiles` already treats
    /// a file name that differs from the tab path as not helper-routed).
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
        struct DurableIdentity: Equatable { var revision: Int; var sha256: String }
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
        /// The durable (ledger) identity the helper's `reload` must still see;
        /// nil for the direct path (no preview controller attached).
        var durable: DurableIdentity?
        var viaController: Bool { durable != nil }
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
            let route = viaController
                ? "through the preview controller (the current text stays in durable undo history)"
                : "directly"
            var edits = entryDirty ? "Your unsaved edits are replaced (recoverable this session via Edit > Restore Discarded Buffer). " : ""
            if !discardedMembers.isEmpty {
                edits += "Unsaved edits to \(discardedMembers.joined(separator: ", ")) are discarded too (recoverable via File > Restore Unsaved Snapshot…). "
            }
            return "Reload \(name) \(route): \(bytesBefore) → \(bytesAfter) bytes, +\(change.added) / −\(change.removed) lines. \(edits)"
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
            var durable: ReloadReview.DurableIdentity?
            if controllerRoutesFiles(for: url), let d = controllerState.durable[activePath] {
                durable = .init(revision: d.revision, sha256: d.sha256)
            }
            // A direct reload replaces the whole project (every dirty member);
            // the controller's reload changes only the entry's buffer.
            let members = durable == nil ? project.listing.filter { $0.isDirty && $0.path != project.entryPath }.map(\.path) : []
            return ReloadReview(url: url, currentText: activeText, diskText: diskText, diskSha256: SourceDigest.sha256Hex(diskText),
                                entryDirty: durable == nil ? project.isDirty(project.entryPath) : isDirty,
                                discardedMembers: members, durable: durable)
        }
    }

    /// Whether file operations on `url` belong to the attached preview
    /// controller: it is ready and names the open document by `activePath`.
    var controllerRoutesFiles: Bool { documentURL.map(controllerRoutesFiles(for:)) ?? false }
    private func controllerRoutesFiles(for url: URL) -> Bool {
        controllerAttached && controllerState.ready && url == documentURL && url.lastPathComponent == activePath
    }

    /// Applies a reviewed reload. A dirty buffer is replaced only with
    /// `.discard` (kept in `recoverableBuffer`); `.saveFirst` is refused
    /// because a file you are about to reload is not one to save over. With the
    /// preview controller attached the import goes through its `reload`
    /// (durable identity + reviewed disk hash must both still match; the prior
    /// source stays in undo history); otherwise the file is re-read directly
    /// and refused if it no longer hashes to the reviewed snapshot.
    @discardableResult
    func confirmReload(_ review: ReloadReview, dirty: DirtyDisposition = .none) async -> OpenOutcome {
        var discarding: RecoverableBuffer?
        // A direct reload replaces the whole project (every member's edits);
        // the controller's reload changes only the entry's buffer.
        if review.viaController ? isDirty : hasUnsavedDocuments {
            switch dirty {
            case .none:
                captureNote = "\(review.viaController ? review.url.lastPathComponent : unsavedDocumentsDescription) has unsaved edits; discard them explicitly to reload."
                return .blockedByUnsavedEdits
            case .saveFirst:
                captureNote = "Cannot save over \(review.url.lastPathComponent) while reloading it; overwrite or discard instead."
                return .saveFailed
            case .discard:
                discarding = RecoverableBuffer(url: documentURL, text: entryText)
            }
        }
        if review.viaController { return await controllerReload(review, discarding: discarding) }
        return directReload(review, discarding: discarding)
    }

    /// Reviewed reload without a modal: `prepareReload()` then `confirmReload`.
    @discardableResult
    func reloadFromDiskReviewed(dirty: DirtyDisposition = .none) async -> OpenOutcome {
        guard let review = prepareReload() else { return .readFailed }
        return await confirmReload(review, dirty: dirty)
    }

    /// Synchronous reviewed reload for the direct path (no preview controller).
    /// With a controller attached the import must go through its `reload`,
    /// which is asynchronous: use `reloadFromDiskReviewed` / `confirmReload`.
    @discardableResult
    func reloadFromDisk(dirty: DirtyDisposition = .none) -> OpenOutcome {
        guard let review = prepareReload() else { return .readFailed }
        if review.viaController {
            captureNote = "\(review.url.lastPathComponent) is held by the preview controller; use the reviewed reload (Resolve On-Disk Conflict…)."
            return .readFailed
        }
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

    /// `reload {path, expected_revision, expected_sha256, expected_disk_sha256,
    /// user_approved:true}` through the preview controller (STDIO.md). The
    /// helper re-reads the file under the project lock and refuses a stale
    /// durable identity or a disk hash other than the reviewed one; a reply
    /// carries the new durable document, adopted as one editor revision. A
    /// lost reply leaves the buffer untouched (the helper may have imported:
    /// the next `document`/`file_status` shows it).
    private func controllerReload(_ review: ReloadReview, discarding: RecoverableBuffer?) async -> OpenOutcome {
        let name = review.url.lastPathComponent
        guard let controller, controller.isRunning, controllerState.ready, review.url == documentURL else {
            captureNote = "Preview controller is no longer attached to \(name); review the reload again."
            return .readFailed
        }
        let path = activePath
        guard let expected = review.durable, let durable = controllerState.durable[path],
              durable.revision == expected.revision, durable.sha256 == expected.sha256 else {
            captureNote = "The durable source of \(name) changed since the reload was reviewed; review again."
            return .readFailed
        }
        let id: String
        do {
            id = try controller.send("reload", ["path": path, "expected_revision": expected.revision, "expected_sha256": expected.sha256,
                                                "expected_disk_sha256": review.diskSha256, "user_approved": true])
        } catch {
            captureNote = "Reload of \(name) failed to send: \(error.localizedDescription)"
            return .readFailed
        }
        let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
            // Adopt inside the waiter, on the helper's own delivery turn, so the
            // preview the helper sends for the new revision already finds its
            // editor-revision mapping.
            controllerState.awaiting[id] = { [weak self] result in
                if let self, case .success(let payload) = result {
                    MainActor.assumeIsolated { self.adoptReloadedDocument(payload, review: review, discarding: discarding) }
                }
                cont.resume(returning: result)
            }
        }
        switch reply {
        case .success(let payload):
            guard payload["document"] is [String: Any] else {
                captureNote = "Reload reply for \(name) carried no document; buffer untouched."
                return .readFailed
            }
            return .opened
        case .failure(let e):
            captureNote = "Reload of \(name) refused by the preview controller: \(e.message). Nothing replaced."
            FlashTeXLog.write("files: controller reload refused: \(e.message)")
            return .readFailed
        }
    }

    /// Records the reloaded durable document and shows it in the editor as one
    /// revision (no edit is submitted back: the ledger already holds it).
    private func adoptReloadedDocument(_ payload: [String: Any], review: ReloadReview, discarding: RecoverableBuffer?) {
        guard let doc = payload["document"] as? [String: Any], let path = doc["path"] as? String,
              let revision = doc["revision"] as? Int, let sha = doc["source_sha256"] as? String,
              let text = doc["text"] as? String else {
            FlashTeXLog.write("files: controller reload reply is missing document fields")
            return
        }
        controllerState.durable[path] = (revision, sha)
        controllerState.textByDurable[path, default: [:]][revision] = text
        if let discarding { recoverableBuffer = discarding }
        if path == activePath { updateActiveText(text) }
        controllerState.editorRevisionByDurable[path, default: [:]][revision] = editorRevision
        savedText = text
        files.conflict = nil
        files.noteDiskState(.unchanged)
        watchOpenDocument()
        captureNote = "Reloaded \(review.url.lastPathComponent) through the preview controller (durable r\(revision); previous text in undo history"
            + (recoverableBuffer == nil ? ")." : " and Edit > Restore Discarded Buffer).")
        if let e = payload["preview_error"] as? String { FlashTeXLog.write("files: reload preview_error: \(e)") }
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
    /// not (before any save is attempted). With a preview controller attached
    /// use `refreshDiskStatus()`, which asks the controller's `file_status`.
    @discardableResult
    func checkDiskStatus() -> ProjectFilesV1.DiskState? {
        guard let url = documentURL else { return nil }
        switch files.diskStatus(url, expectedSha256: baselineSha256) {
        case .failure(let failure):
            captureNote = "Could not check \(url.lastPathComponent) on disk: \(failure.reason)"
            return nil
        case .success(let s):
            applyDiskState(s.state, url: url, sha256: s.sha256, bytes: s.bytes, mtimeUnixMs: s.mtimeUnixMs, viaHelper: files.usesHelper)
            return s.state
        }
    }

    /// `checkDiskStatus()` routed through the preview controller's `file_status`
    /// when one is attached (it rereads the disk under the project lock and
    /// never reloads), else the file layer. The helper compares disk with its
    /// durable source; the editor compares with its own baseline (the hash it
    /// last opened, saved or reloaded), so typed-but-unexported edits do not
    /// read as an external change.
    @discardableResult
    func refreshDiskStatus() async -> ProjectFilesV1.DiskState? {
        guard let url = documentURL else { return nil }
        guard controllerRoutesFiles(for: url) else { return checkDiskStatus() }
        guard let status = await controllerFileStatus(path: activePath) else {
            captureNote = "Could not check \(url.lastPathComponent) on disk: the preview controller did not answer."
            return nil
        }
        let state: ProjectFilesV1.DiskState
        var diskSha: String? = status.diskSHA256
        switch status.state {
        case "missing":
            state = baselineSha256 == nil ? .unchanged : .deleted
        case "matches_source", "differs_from_source":
            // `matches_source` reports the hash as the source's; the parent
            // client exposes only `disk_sha256`, so fall back to the durable hash.
            if diskSha == nil, status.state == "matches_source" { diskSha = controllerState.durable[activePath]?.sha256 }
            guard let sha = diskSha else {
                captureNote = "Could not check \(url.lastPathComponent) on disk: file_status carried no hash."
                return nil
            }
            state = baselineSha256 == nil ? .created : (sha == baselineSha256 ? .unchanged : .modified)
        default:
            captureNote = "Could not check \(url.lastPathComponent) on disk: \(status.reason ?? status.state)"
            return nil
        }
        applyDiskState(state, url: url, sha256: diskSha, bytes: nil, mtimeUnixMs: nil, viaHelper: true)
        files.noteDiskState(state)
        return state
    }

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
            // reload, the unsaved buffer durably (unless the ledger holds it).
            if url == documentURL, project.isDirty(project.entryPath), !controllerRoutesFiles(for: url) {
                preserveDirtyText(entryText, at: url, reason: "file changed on disk while the buffer was unsaved")
            }
        }
    }

    /// Modal resolution of `files.conflict`: Overwrite / Reload / Keep Editing.
    /// The reload is reviewed: the alert shows what the disk snapshot would
    /// change and the import is pinned to exactly that snapshot.
    func resolveConflictPanel() {
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

    private func write(to url: URL, text: String? = nil, expected: ProjectFilesV1.Expected, force: Bool) -> Bool {
        // `url` is the entry's file (Save, Save As, Overwrite): write the entry
        // buffer, never `activeText` — with a member tab active that would put
        // the member's text into main.tex (open/fixture "save first" flows).
        let text = text ?? entryText
        let lateReceipt: @MainActor (String) -> Void = { [weak self] sha in
            // The helper confirmed, after our wait expired, that exactly `text`
            // is on disk at `url`: that text is the new baseline. Edits made
            // meanwhile keep the buffer dirty; a different open file is untouched.
            guard let self, self.documentURL == url, SourceDigest.sha256Hex(text) == sha else { return }
            self.savedText = text
            self.captureNote = "Late confirmation: \(url.lastPathComponent) was saved" + (self.isDirty ? " (buffer edited since; still unsaved)." : ".")
            self.bridgeSourceSaved(url: url, text: text)
        }
        var result = files.save(url, text: text, expected: expected, force: force, lateReceipt: lateReceipt)
        var recreated = false
        if case .conflict(let c) = result, c.kind == .deletedExternally, !force {
            // Nothing on disk can be overwritten: recreate the file, but only if
            // it is still absent (a file appearing meanwhile is `alreadyExists`).
            files.conflict = nil
            recreated = true
            result = files.save(url, text: text, expected: .newFile, force: false, lateReceipt: lateReceipt)
        }
        switch result {
        case .saved:
            documentURL = url
            savedText = text
            captureNote = "Saved \(url.lastPathComponent)" + (recreated ? " (recreated; it had been deleted on disk)" : "")
            bridgeSourceSaved(url: url, text: text)
            snapshotSaved(url: url, text: text)
            watchOpenDocument() // Save As moves the watch; a replaced inode is re-opened
            return true
        case .conflict(let conflict):
            captureNote = conflict.summary
            if url == documentURL { preserveDirtyText(text, at: url, reason: "save refused: file changed on disk") }
            return false
        case .failed(let reason):
            captureNote = "Save failed: \(reason)"
            return false
        }
    }
}
