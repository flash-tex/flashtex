import Foundation
import FlashTeXEditorCore
import FlashTeXPadKit
import FlashTeXProtocol
import NearbyClient
import SwiftUI

/// App state for FlashTeXPad. Everything the Mac does not send over
/// nearby-v1 is labelled by `Provenance` so no panel can pass off a local
/// fixture as a Mac reply.
@MainActor
final class PadModel: ObservableObject {
    enum Provenance: Equatable {
        case mac(String)
        case localFixture(String)
        case notCarried
        var banner: String {
            switch self {
            case .mac(let s): return "from the Mac over nearby-v1: \(s)"
            case .localFixture(let s): return "local fixture — not carried by transfer-v1: \(s)"
            case .notCarried: return "not carried by transfer-v1 (nearby-v1 §6): stays on the Mac"
            }
        }
    }

    // Document
    @Published var document: PadDocument?
    @Published var documentTitle = "No document"
    @Published var caretUTF16 = 0
    @Published var openError: String?
    /// The opened file's folder: pasted images go into its `figures/`
    /// (IMAGE-DROP-IPAD). Nil for the bundled demo and fixtures.
    @Published var projectFolder: URL?
    /// The opened file (its security scope is reentered to save images).
    var documentURL: URL?
    /// A non-blocking editor message (an image paste's outcome). It clears
    /// itself after `editorStatusLifetime` seconds and on the next edit.
    @Published var editorStatus: String? { didSet { scheduleStatusClear() } }
    var editorStatusLifetime: TimeInterval = 6
    private var statusClearTask: Task<Void, Never>?
    /// A paste refused because the document's folder is not writable (a
    /// file opened on its own grants only itself): the editor offers
    /// "Allow access to <folder>", and granting runs the paste again. It
    /// outlives later keystrokes (the paste lands at the caret then) and
    /// is dropped only when another document is opened.
    @Published var folderAccessRequest: FolderAccessRequest?
    /// Folders the user granted: security-scoped bookmarks by path.
    let folderBookmarks: PadFolderBookmarks

    struct FolderAccessRequest {
        let folder: URL
        let retry: @MainActor () -> Void
    }

    // Diagnostics (runtime-v1 compile_result)
    @Published var diagnostics: [DiagnosticItem] = []
    @Published var diagnosticsSource: DiagnosticsSource = .none

    // Completions: the document's own names plus the compiler's inventory
    // (the bundled supported-latex.json, shared with the Mac), computed off
    // the main actor from a snapshot after a short debounce.
    @Published var completions: [LocalCompletion.Suggestion] = []
    /// Esc hid the list; the next edit or ⌃Space shows it again.
    @Published var completionsDismissed = false
    /// ⌘E: the diagnostics list beside the editor.
    @Published var diagnosticsPanelVisible = false
    /// The live editor, when the Editor panel is showing: completions are
    /// inserted through it (undoable, with snippet stops) rather than by
    /// rewriting the document.
    weak var editor: EditorController?
    /// Whether the caret is in math (from the editor's lexer; nil without an editor).
    var caretMathMode: Bool?
    /// Debounce before a completion pass (seconds); tests shorten it.
    var completionDelay: TimeInterval = 0.12
    private(set) var completionTask: Task<Void, Never>?
    /// Completion passes that reached the list (evidence for tests).
    private(set) var completionPasses = 0

    // Reviewed proposal
    @Published var review: ReviewSession?
    @Published var reviewProvenance: Provenance = .notCarried
    @Published var lastReceipt: ReviewSession.Receipt?
    @Published var reviewError: String?
    @Published var insertionCount = 0

    // Mac link
    let link: MacLink
    @Published var transcript: [MacLink.TranscriptLine] = []
    @Published var linkStatus = "not paired"
    @Published var linkError: String?
    @Published var destination: NearbyWire.Destination?
    @Published var lastCaptureReceived: NearbyWire.CaptureReceived?
    @Published var pairedMac: PairedMac?

    // Captures (the product: draw/pick → capture_submit → receipt)
    let queue: CaptureQueue
    @Published var captures: [CaptureRecord] = []
    /// The Captures list's thumbnails, each decoded once off the main actor.
    let thumbnails = CaptureThumbnails()

    /// Production: pairings in the Keychain (`KeychainPairStore`), drafts /
    /// receipts / outcomes in Application Support (`CaptureStore`). Tests pass
    /// their own link (no Keychain) and, when persistence is under test, a
    /// store in a temporary directory. `-flashtexpad-fresh` (UI tests) wipes
    /// both so a run never sees a previous run's captures or pairing.
    init(link: MacLink? = nil, captureStore: CaptureStore? = nil, folderBookmarks: PadFolderBookmarks? = nil,
         draftStore: PadDraftStore? = nil) {
        let fresh = ProcessInfo.processInfo.arguments.contains("-flashtexpad-fresh")
        // Hosted XCTest (TEST_HOST) constructs its own PadModel; the app's
        // @StateObject must not load Keychain/disk or poll leftover captures.
        let hostedUnitTests = ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] != nil
        self.folderBookmarks = folderBookmarks ?? PadFolderBookmarks(
            key: hostedUnitTests ? "flashtexpad.folderBookmarks.hostedTests" : PadFolderBookmarks.defaultsKey)
        if fresh { UserDefaults.standard.removeObject(forKey: PadFolderBookmarks.defaultsKey) }
        let production = link == nil && !hostedUnitTests
        let link = link ?? {
            if hostedUnitTests { return MacLink(store: nil) }
            let keychain = KeychainPairStore()
            if fresh { try? keychain.removeAll() }
            return MacLink(store: keychain)
        }()
        let store: CaptureStore? = captureStore ?? (production ? CaptureStore() : nil)
        if fresh { store?.wipe() }
        self.draftStore = draftStore ?? (production ? PadDraftStore() : nil)
        if fresh { self.draftStore?.wipe() } else { recoverableDrafts = self.draftStore?.takeOffered() ?? [] }
        self.link = link
        self.queue = CaptureQueue(link: link, store: store)
        self.captures = queue.records
        link.onTranscript = { [weak self] line in Task { @MainActor in self?.transcript.append(line) } }
        if let p = link.store?.pairs.last { pairedMac = p; linkStatus = "stored pairing: \(p.macName) (\(p.pairId)) — reconnect with the stored key" }
        // Leaving the app (home, app switcher, lock): save now and finish the
        // draft write, so a kill while suspended loses nothing.
        resignObserver = NotificationCenter.default.addObserver(forName: UIApplication.willResignActiveNotification, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.saveBeforeSuspension() }
        }
    }

    deinit {
        if let resignObserver { NotificationCenter.default.removeObserver(resignObserver) }
    }

    // MARK: saving (autosave to the opened file, crash drafts)

    /// Where the open document's text stands against its file.
    enum SaveState: Equatable {
        /// Nothing to save to (the bundled demo, a fixture, no document).
        case none
        case saved
        case unsaved
        case saving
        /// The file is not the version the buffer was made from (changed on the
        /// Mac, in another app, by iCloud; `nil`: deleted). Nothing is written
        /// until the person chooses: keep this buffer, or take the file's text.
        case conflict(disk: PadDocumentFile.Snapshot?)
        case failed(String)

        var label: String {
            switch self {
            case .none: return "not saved to a file (bundled or fixture document)"
            case .saved: return "saved"
            case .unsaved: return "edited — saving…"
            case .saving: return "saving…"
            case .conflict(nil): return "the file was deleted or moved — Save keeps this text there again"
            case .conflict: return "the file changed outside FlashTeXPad — nothing was overwritten"
            case .failed(let why): return "not saved: \(why) — the text is kept as a draft"
            }
        }
    }

    @Published private(set) var saveState: SaveState = .none
    /// Hash of the file version the buffer was made from (nil: no file).
    private(set) var diskSha256: String?
    /// A bookmark to reopen `documentURL` for a draft restored after a relaunch.
    private var documentBookmark: Data?
    /// Seconds after the last edit before the buffer is written to its file.
    var autosaveDelay: TimeInterval = 1
    /// Seconds after the last edit before the crash draft is written.
    var draftDelay: TimeInterval = 0.3
    private var autosaveTask: Task<Void, Never>?
    private var saveTask: Task<Void, Never>?
    private var saveAgain = false
    private var draftTask: Task<Void, Never>?
    private var resignObserver: NSObjectProtocol?
    /// Bumped by every `setDocument`: the editor reloads on it (every opened
    /// document starts at revision 1, so a revision alone cannot tell two
    /// files apart), an edit from a stale editor is refused, and a save
    /// result names the buffer it was for.
    private(set) var documentGeneration = 0
    /// Unsaved text kept from a closed buffer or an earlier run (a crash, a
    /// kill while suspended, a save that could not complete), offered in the
    /// editor until restored or discarded — one per draft, whatever else is
    /// edited meanwhile.
    @Published var recoverableDrafts: [PadDraftStore.Offered] = []
    let draftStore: PadDraftStore?

    /// A buffer that was closed (another document opened) with text its file
    /// may not hold yet, by the generation it had: the save that was running
    /// or started for it finishes the job, or its text is offered as a draft.
    private struct ClosedBuffer {
        let url: URL
        let scopes: [URL]
        let draft: PadUnsavedDraft
    }
    private var closedBuffers: [Int: ClosedBuffer] = [:]

    /// Bookkeeping after an edit (keystroke cost: two debounced tasks, no I/O).
    private func documentEdited() {
        if documentURL != nil {
            switch saveState {
            case .conflict: break // waits for the person's choice
            case .unsaved: break
            default: saveState = .unsaved
            }
            autosaveTask?.cancel()
            let delay = autosaveDelay
            autosaveTask = Task { [weak self] in
                try? await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
                guard !Task.isCancelled else { return }
                self?.saveNow()
            }
        }
        scheduleDraft(after: draftDelay)
    }

    /// Writes the buffer to its file now, off the main actor, against the
    /// version it was made from. A save already running is followed by one
    /// more; a conflict waits for `keepBufferOverDisk` / `reloadFromDisk`.
    func saveNow() {
        autosaveTask?.cancel()
        guard let url = documentURL, let d = document else { return }
        switch saveState {
        case .unsaved, .failed: break
        case .saving: saveAgain = true; return
        case .none, .saved, .conflict: return
        }
        if saveTask != nil { saveAgain = true; return }
        let expected = diskSha256
        let scopes = saveScopes()
        let text = d.text, revision = d.revision, generation = documentGeneration
        saveState = .saving
        saveTask = Task { [weak self] in
            let result = await Task.detached(priority: .utility) {
                PadDocumentFile.save(text, to: url, expectedSha256: expected, scopes: scopes)
            }.value
            self?.finishSave(result, generation: generation, revision: revision, text: text)
        }
    }

    private func finishSave(_ result: PadDocumentFile.SaveResult, generation: Int, revision: Int, text: String) {
        saveTask = nil
        let again = saveAgain
        saveAgain = false
        guard generation == documentGeneration, let current = document else {
            // The buffer was closed meanwhile (another document, or the same
            // file opened again). Its save finishes its own job; then the
            // current buffer's save that waited for this one runs.
            if let closed = closedBuffers.removeValue(forKey: generation) { settle(closed, after: result, savedText: text) }
            if again { saveNow() }
            saveWaitingClosedBuffer()
            return
        }
        switch result {
        case .saved(let sha):
            diskSha256 = sha
            if current.revision == revision {
                saveState = .saved
                draftTask?.cancel()
                if let source = draftSource() { draftStore?.clear(source) }
            } else {
                saveState = .unsaved
                saveNow()
            }
        case .conflict(let disk):
            saveState = .conflict(disk: disk)
            scheduleDraft(after: 0)
        case .failed(let why):
            saveState = .failed(why)
            scheduleDraft(after: 0)
        }
        if again, saveState == .unsaved { saveNow() }
        saveWaitingClosedBuffer()
    }

    /// A closed buffer whose save waited behind another one (a slow file)
    /// is saved once no save runs, oldest first and one at a time; its
    /// `finishSave` settles it and saves the next.
    private func saveWaitingClosedBuffer() {
        guard saveTask == nil, let next = closedBuffers.min(by: { $0.key < $1.key }) else { return }
        let generation = next.key, closed = next.value
        let text = closed.draft.text, expected = closed.draft.diskSha256
        saveTask = Task { [weak self] in
            let result = await Task.detached(priority: .utility) {
                PadDocumentFile.save(text, to: closed.url, expectedSha256: expected, scopes: closed.scopes)
            }.value
            self?.finishSave(result, generation: generation, revision: 0, text: text)
        }
    }

    /// A closed buffer's save came back. Saved with its last text: its draft
    /// goes. Saved with older text (edits made while that save ran): its last
    /// text is saved over the version just written. A conflict or failure:
    /// its text is offered as a draft at once, never dropped.
    private func settle(_ closed: ClosedBuffer, after result: PadDocumentFile.SaveResult, savedText: String) {
        switch result {
        case .saved where closed.draft.text.sameBytes(as: savedText):
            if !holdsNewerEdits(than: closed) { draftStore?.clear(closed.draft.source) }
        case .saved(let sha):
            let text = closed.draft.text
            Task { [weak self] in
                let next = await Task.detached(priority: .utility) {
                    PadDocumentFile.save(text, to: closed.url, expectedSha256: sha, scopes: closed.scopes)
                }.value
                self?.settle(closed, after: next, savedText: text)
            }
        case .conflict:
            offerClosed(closed, why: "the file changed outside FlashTeXPad")
        case .failed(let why):
            offerClosed(closed, why: why)
        }
    }

    private func offerClosed(_ closed: ClosedBuffer, why: String) {
        guard let draftStore else {
            editorStatus = "“\(closed.draft.title)” was not saved (\(why))."
            return
        }
        recoverableDrafts.insert(draftStore.offer(closed.draft), at: 0)
        if !holdsNewerEdits(than: closed) { draftStore.clear(closed.draft.source) }
        editorStatus = "“\(closed.draft.title)” was not saved (\(why)); its edits are kept — Restore them from the editor."
    }

    /// Whether the open buffer is the closed one's file, opened again and
    /// edited since: that file's draft is then this buffer's, not the closed one's.
    private func holdsNewerEdits(than closed: ClosedBuffer) -> Bool {
        guard draftSource()?.key == closed.draft.source.key, let d = document, !d.text.sameBytes(as: closed.draft.text) else { return false }
        switch saveState {
        case .unsaved, .saving, .failed, .conflict: return true
        case .saved, .none: return false
        }
    }

    /// Before another document replaces the buffer: unsaved text goes to its
    /// draft now and to its file (the save, if one is due, carries on after
    /// the switch); text that cannot be saved (a conflict, a failure) is
    /// offered as a draft at once.
    private func closeBuffer() {
        guard let d = document else { return }
        let dirty: Bool
        switch saveState {
        case .unsaved, .saving, .failed, .conflict: dirty = true
        case .saved: dirty = false
        case .none: dirty = documentURL == nil && d.revision > 1 // an edited bundled demo
        }
        autosaveTask?.cancel()
        draftTask?.cancel()
        guard dirty, let source = draftSource() else { return }
        let draft = PadUnsavedDraft(source: source, title: documentTitle, text: d.text, diskSha256: diskSha256)
        draftStore?.save(draft)
        guard let url = documentURL else { return } // the bundled demo: the draft is its copy
        let closed = ClosedBuffer(url: url, scopes: saveScopes(), draft: draft)
        switch saveState {
        case .unsaved:
            closedBuffers[documentGeneration] = closed
            saveNow()
        case .saving:
            closedBuffers[documentGeneration] = closed // the running save settles it
        case .conflict:
            offerClosed(closed, why: "the file changed outside FlashTeXPad")
        case .failed(let why):
            offerClosed(closed, why: why)
        case .saved, .none:
            break
        }
    }

    /// The scopes that cover the file: a granted folder, the file itself.
    private func saveScopes() -> [URL] {
        let grant = projectFolder.flatMap { folderBookmarks.grant(covering: $0) }
        return [grant?.scope, documentURL].compactMap { $0 }
    }

    /// Conflict: keep this buffer — write it over what the file holds now.
    func keepBufferOverDisk() {
        guard case .conflict(let disk) = saveState else { return }
        diskSha256 = disk?.sha256
        saveState = .unsaved
        saveNow()
    }

    /// Conflict: take the file's text, dropping this buffer's edits. The file
    /// is read again now (off the main actor): it may have changed since the
    /// conflict was found.
    func reloadFromDisk() async {
        guard case .conflict = saveState, let url = documentURL else { return }
        let generation = documentGeneration, scopes = saveScopes()
        let read = await Task.detached(priority: .userInitiated) { Result { try PadDocumentFile.read(url, scopes: scopes) } }.value
        guard generation == documentGeneration, case .conflict = saveState, var d = document else { return }
        switch read {
        case .success(let disk):
            d.text = disk.text
            d.revision += 1
            document = d
            diskSha256 = disk.sha256
            saveState = .saved
            draftTask?.cancel()
            if let source = draftSource() { draftStore?.clear(source) }
            refreshCompletions()
        case .failure:
            saveState = .conflict(disk: nil)
        }
    }

    /// Waits for the pending autosave and any save in flight (tests).
    func settleSave() async {
        repeat {
            await autosaveTask?.value
            await saveTask?.value
        } while saveTask != nil
    }

    /// Waits for the pending draft write (tests).
    func settleDraft() async {
        await draftTask?.value
        await draftStore?.flush()
    }

    private func scheduleDraft(after delay: TimeInterval) {
        guard let draftStore, let d = document, let source = draftSource() else { return }
        draftTask?.cancel()
        let title = documentTitle, sha = diskSha256, generation = documentGeneration
        draftTask = Task { [weak self] in
            if delay > 0 { try? await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000)) }
            guard !Task.isCancelled, let self, self.documentGeneration == generation, self.document?.revision == d.revision else { return }
            draftStore.save(PadUnsavedDraft(source: source, title: title, text: d.text, diskSha256: sha))
        }
    }

    private func draftSource() -> PadUnsavedDraft.Source? {
        if let url = documentURL { return .file(path: url.path, bookmark: documentBookmark) }
        return documentTitle == Self.bundledSample ? .bundledSample : nil
    }

    /// Leaving the app: write the buffer and the draft now, under a
    /// background-task grant so the writes finish before suspension.
    func saveBeforeSuspension() {
        guard let current = document else { return }
        let unsaved: Bool
        switch saveState {
        case .unsaved, .saving, .failed, .conflict: unsaved = true
        case .saved: unsaved = false
        case .none: unsaved = documentURL == nil && current.revision > 1 // an edited bundled demo
        }
        guard unsaved || !closedBuffers.isEmpty else { return }
        if unsaved {
            draftTask?.cancel()
            if let draftStore, let source = draftSource() {
                draftStore.save(PadUnsavedDraft(source: source, title: documentTitle, text: current.text, diskSha256: diskSha256))
            }
            saveNow()
        }
        let app = UIApplication.shared
        let grant = BackgroundGrant()
        grant.id = app.beginBackgroundTask(withName: "flashtex.pad.save") { grant.end(app) }
        Task { [weak self] in
            await self?.settleSave()
            await self?.draftStore?.flush()
            grant.end(app)
        }
    }

    /// One `beginBackgroundTask` grant, ended once (by the work or by expiry).
    private final class BackgroundGrant: @unchecked Sendable {
        var id = UIBackgroundTaskIdentifier.invalid
        @MainActor func end(_ app: UIApplication) {
            guard id != .invalid else { return }
            app.endBackgroundTask(id)
            id = .invalid
        }
    }

    /// An offered draft back in the editor: its file reopened (bookmark,
    /// else its path) and its text applied as unsaved edits against the
    /// version they were made from — a file changed since is a conflict.
    /// The offered copy goes once its text is in the buffer (whose own
    /// draft then protects it).
    func restoreDraft(_ offered: PadDraftStore.Offered) {
        let draft = offered.draft
        switch draft.source {
        case .bundledSample:
            openBundledSample()
        case .file(let path, let bookmark):
            var stale = false
            let url = bookmark.flatMap { try? URL(resolvingBookmarkData: $0, bookmarkDataIsStale: &stale) } ?? URL(fileURLWithPath: path)
            open(url: url, title: draft.title)
            guard openError == nil, documentURL != nil else {
                openError = "Could not reopen \(draft.title) to restore its unsaved text: \(openError ?? "no access") — the draft is kept."
                return
            }
        }
        recoverableDrafts.removeAll { $0.id == offered.id }
        guard var d = document, !d.text.sameBytes(as: draft.text) else { // already what the file holds
            draftStore?.removeOffered(offered.id)
            return
        }
        d.text = draft.text
        d.revision += 1
        document = d
        if documentURL != nil {
            diskSha256 = draft.diskSha256
            saveState = .unsaved
        }
        documentEdited()
        // The buffer's own draft is written before the offered copy goes
        // (the store keeps call order), so a kill between them loses nothing.
        if let source = draftSource() {
            draftTask?.cancel()
            draftStore?.save(PadUnsavedDraft(source: source, title: documentTitle, text: draft.text, diskSha256: diskSha256))
        }
        draftStore?.removeOffered(offered.id)
        refreshCompletions()
    }

    /// An offered draft dropped.
    func discardDraft(_ offered: PadDraftStore.Offered) {
        recoverableDrafts.removeAll { $0.id == offered.id }
        draftStore?.removeOffered(offered.id)
    }

    // MARK: captures

    @discardableResult
    func draft(_ r: CaptureRecord) -> CaptureRecord {
        let d = queue.draft(r)
        captures = queue.records
        return d
    }

    func discard(_ id: String) { queue.discard(id); captures = queue.records }

    // MARK: one-tap send (lane mac-capture-fluid)

    /// Instruction chips: the last few instructions sent, newest first, seeded
    /// with the three the Mac's converter is best at. Persisted per iPad.
    static let defaultInstructions = ["Convert to TikZ", "Transcribe as LaTeX", "This is a matrix"]
    static let recentInstructionsKey = "flashtexpad.recentInstructions"
    nonisolated static let maxRecentInstructions = 6
    @Published var recentInstructions: [String] = {
        let saved = UserDefaults.standard.stringArray(forKey: PadModel.recentInstructionsKey) ?? []
        return PadModel.merged(recent: saved, defaults: PadModel.defaultInstructions)
    }()

    /// Pure: `recent` first (deduplicated, trimmed), the defaults appended once, bounded.
    static func merged(recent: [String], defaults: [String], limit: Int = maxRecentInstructions) -> [String] {
        var out: [String] = []
        for s in recent + defaults {
            let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !t.isEmpty, !out.contains(where: { $0.caseInsensitiveCompare(t) == .orderedSame }) else { continue }
            out.append(t)
            if out.count == limit { break }
        }
        return out
    }

    func rememberInstruction(_ text: String) {
        recentInstructions = Self.merged(recent: [text] + recentInstructions, defaults: Self.defaultInstructions)
        UserDefaults.standard.set(recentInstructions, forKey: Self.recentInstructionsKey)
    }

    /// Prepare + Send as one tap: validates (the same `CaptureQueue.validate`
    /// as before), drafts the record so it is on disk before the bytes leave,
    /// sends it, and starts outcome polling. Returns the problem instead of
    /// sending when validation fails or no Mac is connected; the capture id
    /// otherwise.
    @discardableResult
    func sendNow(png: Data, source: CaptureRecord.Source, instructions: String, pixelSize: (width: Int, height: Int)? = nil) async -> Result<String, SendProblem> {
        let text = instructions.trimmingCharacters(in: .whitespacesAndNewlines)
        if let why = CaptureQueue.validate(png: png, instructions: text) { return .failure(.invalid(why)) }
        guard link.isConnected else { return .failure(.notConnected(linkStatus)) }
        let r = draft(CaptureRecord(source: source, png: png, instructions: text, pixelSize: pixelSize))
        rememberInstruction(text)
        await send(r.id)
        return .success(r.id)
    }

    enum SendProblem: Error, Equatable, CustomStringConvertible {
        case invalid(String), notConnected(String)
        var description: String {
            switch self {
            case .invalid(let why): return why
            case .notConnected(let status): return "not connected to a Mac (\(status)) — pair or reconnect first"
            }
        }
    }

    // MARK: auto-reconnect and discovery

    /// Last address the stored pairing was reached at (per Mac fingerprint),
    /// so a relaunch tries it before browsing Bonjour.
    static func lastEndpointKey(_ fingerprint: String) -> String { "flashtexpad.lastEndpoint.\(fingerprint)" }
    func rememberEndpoint(host: String, port: UInt16, fingerprint: String) {
        UserDefaults.standard.set("\(host):\(port)", forKey: Self.lastEndpointKey(fingerprint))
    }
    static func lastEndpoint(fingerprint: String, defaults: UserDefaults = .standard) -> (host: String, port: UInt16)? {
        guard let s = defaults.string(forKey: lastEndpointKey(fingerprint)), let colon = s.lastIndex(of: ":"),
              let port = UInt16(s[s.index(after: colon)...]) else { return nil }
        return (String(s[..<colon]), port)
    }

    @Published var reconnecting = false
    /// Macs advertising `_flashtex-nearby._tcp` right now (Bonjour), for tap-to-pair.
    @Published var nearbyMacs: [DiscoveredMac] = []
    @Published var browsing = false

    /// At launch (and from the Mac link panel): connect to the last paired Mac
    /// with the stored key — the remembered address first, else the Mac found
    /// by Bonjour with the same fingerprint. Never pairs; a Mac that is not
    /// there leaves the pairing stored and the status honest. `endpoint`
    /// overrides both lookups (tests: the fake Mac on loopback).
    @discardableResult
    func autoReconnect(endpoint: (host: String, port: UInt16)? = nil, browseSeconds: TimeInterval = 4) async -> Bool {
        guard pairedMac != nil, !link.isConnected else { return link.isConnected }
        // One attempt at a time. A request made while one runs (a return to
        // the foreground during the launch reconnect) is kept and runs once
        // that attempt ends still disconnected, instead of being dropped.
        guard !reconnecting else {
            reconnectRequested = (endpoint, browseSeconds)
            return false
        }
        var ok = await reconnectOnce(endpoint: endpoint, browseSeconds: browseSeconds)
        while !ok, let next = reconnectRequested, pairedMac != nil, !link.isConnected {
            reconnectRequested = nil
            ok = await reconnectOnce(endpoint: next.endpoint, browseSeconds: next.browseSeconds)
        }
        reconnectRequested = nil
        return ok || link.isConnected
    }

    /// An `autoReconnect` asked for while another attempt was running.
    private var reconnectRequested: (endpoint: (host: String, port: UInt16)?, browseSeconds: TimeInterval)?

    private func reconnectOnce(endpoint: (host: String, port: UInt16)?, browseSeconds: TimeInterval) async -> Bool {
        guard let pair = pairedMac, !link.isConnected else { return link.isConnected }
        reconnecting = true
        defer { reconnecting = false }
        linkError = nil
        var candidates: [(String, UInt16)] = []
        if let endpoint { candidates.append(endpoint) }
        else if let last = Self.lastEndpoint(fingerprint: pair.fingerprint) { candidates.append(last) }
        for (host, port) in candidates {
            linkStatus = "reconnecting to \(pair.macName) at \(host):\(port)…"
            do {
                try await link.connect(host: host, port: port, pair: pair)
                connected(pair, host: host, port: port)
                return true
            } catch { linkStatus = "\(pair.macName) not at \(host):\(port); browsing…" }
        }
        guard endpoint == nil else { linkError = "reconnect failed"; linkStatus = "stored pairing: \(pair.macName) — Mac not reachable"; return false }
        do {
            let found = try await NearbyBrowser.discover(seconds: browseSeconds) { $0.fingerprint == pair.fingerprint }
            guard let mac = found.first(where: { $0.fingerprint == pair.fingerprint }) else {
                linkStatus = "stored pairing: \(pair.macName) — not advertising nearby (open Captures on the Mac)"
                return false
            }
            let session = try await NearbyClient.connect(endpoint: mac.endpoint, pair: pair)
            link.adopt(session: session, pair: pair)
            if case .hostPort(let h, let p) = mac.endpoint { rememberEndpoint(host: "\(h)", port: p.rawValue, fingerprint: pair.fingerprint) }
            destination = link.destination
            linkStatus = "connected to \(pair.macName)"
            resumeOutcomePolling()
            return true
        } catch {
            linkError = "\(error)"
            linkStatus = "stored pairing: \(pair.macName) — reconnect failed"
            return false
        }
    }

    /// The app's scene phase (`FlashTeXPadApp`). Back in the foreground, a
    /// stored pairing whose link died while the app was away reconnects by
    /// itself: iOS drops the connection of a suspended app, and before this
    /// only launch reconnected. Going to the background finishes the capture
    /// writes still queued, so a suspended app is never killed with a capture
    /// that is not on disk.
    func scenePhaseChanged(to phase: ScenePhase) async {
        switch phase {
        case .active:
            guard pairedMac != nil, !link.isConnected else { return }
            await autoReconnect()
        case .background:
            await enteredBackground().value
        default:
            break
        }
    }

    /// Takes the background-task grant now, synchronously in the caller's
    /// scene-phase handler (not after an async hop, which a busy main thread
    /// could push past iOS's grace period), then flushes the capture writes
    /// and ends it. The returned task finishes with the flush.
    @discardableResult
    func enteredBackground() -> Task<Void, Never> {
        let app = UIApplication.shared
        let grant = BackgroundTask()
        grant.id = app.beginBackgroundTask(withName: "flashtex.pad.captures") { grant.end(app) }
        suspensionGrant = grant
        return Task { [weak self] in
            await self?.queue.flush()
            grant.end(app)
            if self?.suspensionGrant === grant { self?.suspensionGrant = nil }
        }
    }

    private var suspensionGrant: BackgroundTask?
    /// A background-task grant for the capture writes is held (tests).
    var holdsSuspensionGrant: Bool { suspensionGrant.map { $0.id != .invalid } ?? false }

    /// One `beginBackgroundTask` grant, ended once (by the work or by expiry).
    private final class BackgroundTask: @unchecked Sendable {
        var id = UIBackgroundTaskIdentifier.invalid
        @MainActor func end(_ app: UIApplication) {
            guard id != .invalid else { return }
            app.endBackgroundTask(id)
            id = .invalid
        }
    }

    private func connected(_ pair: PairedMac, host: String, port: UInt16) {
        rememberEndpoint(host: host, port: port, fingerprint: pair.fingerprint)
        destination = link.destination
        linkStatus = "connected to \(pair.macName)"
        resumeOutcomePolling()
    }

    /// Lists nearby FlashTeX Macs for `seconds`; the panel shows them as
    /// tap-to-pair rows (the code is still required — nearby-v1 §2).
    func browseNearby(seconds: TimeInterval = 3) async {
        guard !browsing else { return }
        browsing = true
        defer { browsing = false }
        do { nearbyMacs = try await NearbyBrowser.discover(seconds: seconds).filter(\.isSupported) }
        catch { linkError = "browse: \(error)" }
    }

    /// Tap-to-pair with a discovered Mac: the same bootstrap as the QR path.
    @discardableResult
    func pair(mac: DiscoveredMac, code: String) async -> Bool {
        linkError = nil
        linkStatus = "pairing with \(mac.macName)…"
        do {
            let pair = try await link.pair(discovered: mac, code: code, companionName: "FlashTeXPad (\(UIDevice.current.name))")
            pairedMac = pair
            destination = link.destination
            linkStatus = "paired with \(pair.macName) (\(pair.pairId)); connected"
            if case .hostPort(let h, let p) = mac.endpoint { rememberEndpoint(host: "\(h)", port: p.rawValue, fingerprint: pair.fingerprint) }
            resumeOutcomePolling()
            return true
        } catch { linkError = "\(error)"; linkStatus = "pairing failed"; return false }
    }

    func send(_ id: String) async {
        // Claimed and flipped to `sending` before the first suspension, and
        // published at once: the row stops offering Send / Retry, and a second
        // tap that still reaches here is refused by the claim (one delivery).
        let claim = queue.beginSend(id)
        captures = queue.records
        guard let claim else { return }
        await queue.send(claim)
        captures = queue.records
        destination = link.destination
        if queue.shouldPoll(id) { pollOutcome(id) }
    }

    // MARK: outcome polling (additive capture_status)

    /// Polls `capture_status` every `pollInterval` until the Mac reports a
    /// final state, says it cannot answer, or `maxPolls` is reached. One task
    /// per capture; a relaunch resumes it for every received, non-final record.
    var pollInterval: TimeInterval = 2
    var maxPolls = 150
    private var pollers: [String: Task<Void, Never>] = [:]
    var isPollingOutcomes: Bool { !pollers.isEmpty }

    func pollOutcome(_ id: String) {
        pollers[id]?.cancel()
        pollers[id] = Task { @MainActor [weak self] in
            var n = 0
            while !Task.isCancelled {
                let interval: TimeInterval
                do {
                    guard let self else { return }
                    // A dead link stops the poller (reconnecting resumes it:
                    // `resumeOutcomePolling`) rather than spending the remaining
                    // polls recording "not connected".
                    if n >= self.maxPolls || !self.queue.shouldPoll(id) || !self.link.isConnected {
                        self.pollers[id] = nil
                        return
                    }
                    n += 1
                    interval = self.pollInterval
                    await self.queue.refreshOutcome(id)
                }
                do {
                    guard let self else { return }
                    self.captures = self.queue.records
                    if !self.queue.shouldPoll(id) { break }
                }
                try? await Task.sleep(nanoseconds: UInt64(interval * 1_000_000_000))
            }
            self?.pollers[id] = nil
        }
    }

    /// One immediate probe (the list's Refresh button / tests).
    func refreshOutcome(_ id: String) async {
        await queue.refreshOutcome(id)
        captures = queue.records
    }

    /// The Insert tap: approve the proposal shown on this screen and ask the
    /// Mac to apply it (`capture_insert`).
    @discardableResult
    func insertCapture(_ id: String) async -> NearbyWire.CaptureInsertAck? {
        let work = Task { await queue.insertCapture(id) }
        // The queue flips `inserting` before its first suspension; one yield
        // lets that reach the list, so the button reads "Inserting…" while the
        // request is out rather than looking dead.
        await Task.yield()
        captures = queue.records
        let ack = await work.value
        captures = queue.records
        // An insertion is terminal, so this row's poller can stop. A refusal
        // leaves it polling exactly as before.
        if ack?.state == "inserted" {
            pollers[id]?.cancel()
            pollers[id] = nil
        }
        return ack
    }

    /// After (re)connecting: resume polling for captures still awaiting an outcome.
    func resumeOutcomePolling() {
        for r in queue.records where queue.shouldPoll(r.id) && pollers[r.id] == nil { pollOutcome(r.id) }
    }

    /// Test/automation hook: `-flashtexpad-test-mac host:port:saltHex:fp:code`
    /// pairs with a listener the UI-test runner hosts on loopback.
    func pairFromLaunchArgument() {
        let args = ProcessInfo.processInfo.arguments
        guard let i = args.firstIndex(of: "-flashtexpad-test-mac"), i + 1 < args.count else { return }
        let parts = args[i + 1].split(separator: ":").map(String.init)
        guard parts.count == 5 else { linkError = "bad -flashtexpad-test-mac argument"; return }
        Task { await pair(host: parts[0], port: parts[1], saltHex: parts[2], fingerprint: parts[3], macName: "Test Mac", code: parts[4]) }
    }

    // MARK: documents

    static let bundledSample = "demo.tex"

    func openBundledSample() {
        guard let url = Bundle.main.url(forResource: "demo", withExtension: "tex") else { openError = "demo.tex missing from bundle"; return }
        open(url: url, title: PadModel.bundledSample)
    }

    func open(url: URL, title: String? = nil) {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        do {
            let file = try PadDocumentFile.read(url) // coordinated: a file provider's current version
            setDocument(PadDocument(path: url.lastPathComponent, text: file.text), title: title ?? url.lastPathComponent)
            let bundled = url.standardizedFileURL.path.hasPrefix(Bundle.main.bundleURL.standardizedFileURL.path + "/")
            documentURL = bundled ? nil : url
            projectFolder = bundled ? nil : url.deletingLastPathComponent()
            diskSha256 = bundled ? nil : file.sha256
            documentBookmark = bundled ? nil : try? url.bookmarkData()
            saveState = bundled ? .none : .saved
        } catch { openError = "\(error)" }
    }

    /// "Open folder…": the whole project folder up front. The folder is
    /// granted (pasted images are written into it without asking) and its
    /// main `.tex` file opened (`PadProjectFolder.mainDocument`).
    func openFolder(url: URL) {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        do { try folderBookmarks.grant(url) } catch {
            openError = "Could not keep access to \(url.lastPathComponent): \(error.localizedDescription)"
            return
        }
        guard let tex = PadProjectFolder.mainDocument(in: url) else {
            openError = "No .tex file in \(url.lastPathComponent)."
            return
        }
        open(url: tex) // read inside the folder's scope (entered above)
    }

    // MARK: folder access (IPAD-FOLDER-ACCESS)

    /// What an image paste writes with: the project folder (under a granted
    /// folder when one covers it) and the scopes to enter.
    func imagePasteHost() -> EditorController.ImagePasteHost {
        let grant = projectFolder.flatMap { folderBookmarks.grant(covering: $0) }
        return EditorController.ImagePasteHost(
            projectFolder: grant?.folder ?? projectFolder,
            scopeURLs: [grant?.scope, documentURL].compactMap { $0 },
            documentID: documentURL?.standardizedFileURL,
            hasFolderGrant: grant != nil,
            note: { [weak self] in self?.editorStatus = $0 },
            requestFolderAccess: { [weak self] folder, retry in
                self?.folderAccessRequest = FolderAccessRequest(folder: folder, retry: retry)
            })
    }

    /// The folder picker's answer to "Allow access to <folder>": the pick is
    /// kept as a bookmark; when it covers the refused folder, the pending
    /// paste runs again. A pick that does not cover it says which to choose.
    func grantFolderAccess(_ picked: URL) {
        let scoped = picked.startAccessingSecurityScopedResource()
        defer { if scoped { picked.stopAccessingSecurityScopedResource() } }
        do { try folderBookmarks.grant(picked) } catch {
            editorStatus = "Could not keep access to “\(picked.lastPathComponent)”: \(error.localizedDescription)"
            return
        }
        guard let request = folderAccessRequest else {
            editorStatus = "Access to “\(picked.lastPathComponent)” granted."
            return
        }
        let wanted = request.folder.lastPathComponent
        guard PadFolderBookmarks.path(PadFolderBookmarks.path(of: request.folder), isInside: PadFolderBookmarks.path(of: picked)) else {
            editorStatus = "“\(picked.lastPathComponent)” does not contain “\(wanted)”: choose “\(wanted)” or a folder that contains it."
            return
        }
        folderAccessRequest = nil
        editorStatus = "Access to “\(picked.lastPathComponent)” granted."
        request.retry()
    }

    private func scheduleStatusClear() {
        statusClearTask?.cancel()
        guard let shown = editorStatus else { return }
        let lifetime = editorStatusLifetime
        statusClearTask = Task { @MainActor [weak self] in
            try? await Task.sleep(nanoseconds: UInt64(lifetime * 1_000_000_000))
            guard !Task.isCancelled, let self, self.editorStatus == shown else { return }
            self.editorStatus = nil
        }
    }

    /// The assistant-context recorded review fixture: loads its `main.tex`,
    /// its compiler diagnostics and its `proposal_review` as a pending review.
    func openReviewFixture() {
        do {
            let f = try loadWorkflowFixture()
            setDocument(PadDocument(path: f.sourcePath, text: f.sourceText, revision: f.compileResult.revision), title: "review-fixture/main.tex")
            diagnostics = DiagnosticsModel.items(from: f.compileResult, boundTo: document)
            diagnosticsSource = .fixture(name: "review-workflow.json compile_result", revision: f.compileResult.revision)
            attachFixtureReview(f)
        } catch { openError = "review fixture: \(error)" }
    }

    func loadWorkflowFixture() throws -> ReviewedProposal.WorkflowFixture {
        guard let url = Bundle.main.url(forResource: "review-workflow", withExtension: "json") else {
            throw ReviewedProposal.DecodeError.missingStep("bundled review-workflow.json")
        }
        return try ReviewedProposal.decodeWorkflowFixture(try Data(contentsOf: url))
    }

    func attachFixtureReview(_ f: ReviewedProposal.WorkflowFixture) {
        review = ReviewSession(review: f.review, approved: f.approved)
        reviewProvenance = .localFixture("bundled review-workflow.json (provider_called:false)")
        reviewError = nil
    }

    private func setDocument(_ doc: PadDocument, title: String) {
        // The previous buffer's unsaved text goes to its draft and its file
        // first, or is offered as a draft when it cannot be saved.
        closeBuffer()
        documentGeneration += 1
        saveState = .none
        diskSha256 = nil
        documentBookmark = nil
        document = doc
        documentTitle = title
        caretUTF16 = 0
        openError = nil
        projectFolder = nil
        documentURL = nil
        editorStatus = nil
        folderAccessRequest = nil
        review = nil
        lastReceipt = nil
        diagnostics = []
        diagnosticsSource = .none
        refreshCompletions()
    }

    /// A keystroke's cost on the main actor: no scan of the document here —
    /// the text is a value snapshot, the revision moves, and the completion
    /// pass is scheduled off the main actor. Returns the new revision (the
    /// editor records it so it does not reload its own edit).
    @discardableResult
    func textChanged(_ text: String, caret: Int? = nil, mathMode: Bool? = nil, generation: Int? = nil) -> Int {
        guard var d = document else { return 0 }
        // An editor still showing a document that was replaced must never
        // write its text into the new one (it reloads on the generation).
        if let generation, generation != documentGeneration { return d.revision }
        if let caret { caretUTF16 = caret }
        caretMathMode = mathMode
        guard !d.text.sameBytes(as: text) else { return d.revision }
        d.text = text
        d.revision += 1
        document = d
        documentEdited()
        // The next edit clears the status note. A pending access offer stays
        // (its button still shows): granting then pastes at the caret.
        editorStatus = nil
        // A buffer edit invalidates a pending review's binding (sha/revision); keep it
        // pending so approve() refuses it with the exact reason rather than hiding it.
        completionsDismissed = false
        refreshCompletions()
        return d.revision
    }

    func caretMoved(_ utf16: Int, mathMode: Bool? = nil) {
        caretUTF16 = utf16
        caretMathMode = mathMode
        refreshCompletions()
    }

    /// Schedules a completion pass: after `completionDelay` (coalescing the
    /// keystrokes in between) the document snapshot is scanned on a
    /// background task and the list lands back here, unless a newer edit
    /// superseded it.
    func refreshCompletions() { scheduleCompletions(delay: completionDelay) }

    /// ⌃Space: a pass right now, even after Esc.
    func showCompletions() {
        completionsDismissed = false
        scheduleCompletions(delay: 0)
    }

    /// Esc: hide the list until the next edit.
    func dismissCompletions() {
        completionTask?.cancel()
        completionsDismissed = true
        completions = []
    }

    private func scheduleCompletions(delay: TimeInterval) {
        completionTask?.cancel()
        guard let d = document, !completionsDismissed else { completions = []; return }
        let text = d.text
        let caret = caretUTF16
        let context = LocalCompletion.Context(vocabulary: Self.bundledVocabulary, mathMode: caretMathMode)
        completionTask = Task { [weak self] in
            if delay > 0 { try? await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000)) }
            guard !Task.isCancelled else { return }
            let result = await Task.detached(priority: .userInitiated) { () -> [LocalCompletion.Suggestion] in
                guard let byte = LaTeXEditing.utf8Offset(of: caret, in: text) else { return [] }
                return LocalCompletion.suggestions(in: text, caretByte: byte, context: context)
            }.value
            guard !Task.isCancelled, let self else { return }
            self.completions = result
            self.completionPasses += 1
        }
    }

    /// Waits for the pending completion pass (tests).
    func settleCompletions() async { await completionTask?.value }

    /// The completion vocabulary bundled with the app (`Resources/supported-latex.json`,
    /// byte-identical to the Mac's copy; `apps/mac/scripts/gen-supported-latex.py render --install`).
    /// Decoded once, on first use from the background completion task.
    static let bundledVocabulary: LaTeXVocabulary = {
        guard let url = Bundle.main.url(forResource: "supported-latex", withExtension: "json"),
              let data = try? Data(contentsOf: url), let v = try? LaTeXVocabulary(inventoryData: data) else { return .empty }
        return v
    }()

    /// Applies a completion: through the live editor when there is one
    /// (undoable, snippet stops), else at its byte range in the document
    /// (the same offset discipline as an edit).
    /// The list showing may be one keystroke behind the text (its pass is
    /// still debouncing): the suggestion is re-anchored to the word under the
    /// caret now, and a list that no longer applies is dropped instead of
    /// being applied at its old range. Returns whether it was applied.
    @discardableResult
    func accept(_ s: LocalCompletion.Suggestion) -> Bool {
        guard var d = document, let caretByte = LaTeXEditing.utf8Offset(of: caretUTF16, in: d.text),
              let s = LocalCompletion.reanchored(s, in: d.text, caretByte: caretByte),
              let r = d.text.rangeOfUTF8(start: s.replaceStart, end: s.replaceEnd) else {
            completions = [] // the pending pass, if any, still lands
            return false
        }
        if let editor {
            editor.accept(s, replacing: NSRange(r, in: d.text))
            return true
        }
        let insertion = s.insertion
        d.text.replaceSubrange(r, with: insertion)
        d.revision += 1
        document = d
        documentEdited()
        let start = NSRange(d.text.startIndex..<r.lowerBound, in: d.text).length
        caretUTF16 = start + (s.snippet?.caretUTF16 ?? (insertion as NSString).length)
        refreshCompletions()
        return true
    }

    /// Editor key commands the model answers. Returns true when Tab accepted
    /// a completion (the editor then leaves the caret alone).
    @discardableResult
    func handle(_ command: EditorCommand) -> Bool {
        switch command {
        case .showCompletions: showCompletions()
        case .dismiss: dismissCompletions()
        case .toggleDiagnostics: diagnosticsPanelVisible.toggle()
        case .acceptOrNextStop:
            guard let first = completions.first else { return false }
            return accept(first)
        default: break
        }
        return false
    }

    // MARK: diagnostics from a compile_result file

    func openCompileResult(url: URL) {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        do {
            let r = try DiagnosticsModel.decode(try Data(contentsOf: url))
            diagnostics = DiagnosticsModel.items(from: r, boundTo: document)
            diagnosticsSource = .file(name: url.lastPathComponent, revision: r.revision)
        } catch { openError = "compile_result: \(error)" }
    }

    func openBundledCompileResult() {
        guard let url = Bundle.main.url(forResource: "compile-result", withExtension: "json") else { return }
        do {
            let r = try DiagnosticsModel.decode(try Data(contentsOf: url))
            diagnostics = DiagnosticsModel.items(from: r, boundTo: document)
            diagnosticsSource = .fixture(name: "protocol/fixtures/compile-result.json", revision: r.revision)
        } catch { openError = "compile_result: \(error)" }
    }

    // MARK: review gate

    func cancelReview() {
        review?.cancel()
    }

    func approveReview() {
        guard var r = review, var d = document else { return }
        do {
            let receipt = try r.approve(applyingTo: &d)
            document = d
            documentEdited()
            lastReceipt = receipt
            insertionCount += 1
            reviewError = nil
            refreshCompletions()
        } catch {
            reviewError = "\(error)"
        }
        review = r
    }

    // MARK: Mac link

    func pair(host: String, port: String, saltHex: String, fingerprint: String, macName: String, code: String) async {
        guard let p = UInt16(port) else { linkError = "port must be a number"; return }
        linkError = nil
        linkStatus = "pairing…"
        do {
            let pair = try await link.pair(host: host, port: p, saltHex: saltHex, fingerprint: fingerprint, macName: macName,
                                           code: code, companionName: "FlashTeXPad (\(UIDevice.current.name))")
            pairedMac = pair
            destination = link.destination
            linkStatus = "paired with \(pair.macName) (\(pair.pairId)); connected"
            rememberEndpoint(host: host, port: p, fingerprint: pair.fingerprint)
            resumeOutcomePolling()
        } catch { linkError = "\(error)"; linkStatus = "pairing failed" }
    }

    func forgetPairing() {
        if let p = pairedMac { _ = try? link.store?.remove(fingerprint: p.fingerprint) }
        link.disconnect()
        pairedMac = nil
        linkStatus = "not paired"
    }

    /// QR pairing: the Mac's `flashtex-nearby://pair?v=1&code&salt&fp&name`
    /// (scanned with VisionKit, or pasted). Host/port typed when Bonjour
    /// cannot find the Mac (the simulator has no camera and, in tests, no
    /// advertising listener).
    @discardableResult
    func pair(bootstrapText: String, host: String, port: String) async -> Bool {
        linkError = nil
        let payload: NearbyBootstrapPayload
        do { payload = try NearbyBootstrapPayload.parse(bootstrapText) } catch { linkError = "QR payload: \(error)"; linkStatus = "pairing failed"; return false }
        let h = host.trimmingCharacters(in: .whitespaces)
        let p = UInt16(port.trimmingCharacters(in: .whitespaces))
        if !h.isEmpty, !port.isEmpty, p == nil { linkError = "port must be a number"; return false }
        linkStatus = h.isEmpty ? "pairing with \(payload.macName) (browsing Bonjour for fp \(payload.fingerprint))…" : "pairing with \(payload.macName)…"
        do {
            let pair = try await link.pair(bootstrap: payload, host: h.isEmpty ? nil : h, port: h.isEmpty ? nil : p,
                                           companionName: "FlashTeXPad (\(UIDevice.current.name))")
            pairedMac = pair
            destination = link.destination
            linkStatus = "paired with \(pair.macName) (\(pair.pairId)); connected"
            resumeOutcomePolling()
            return true
        } catch { linkError = "\(error)"; linkStatus = "pairing failed"; return false }
    }

    func reconnect(host: String, port: String) async {
        guard let pair = pairedMac else { linkError = "no stored pairing"; return }
        guard let p = UInt16(port) else { linkError = "port must be a number"; return }
        linkError = nil
        do {
            try await link.connect(host: host, port: p, pair: pair)
            connected(pair, host: host, port: p)
        } catch { linkError = "\(error)"; linkStatus = "connect failed" }
    }

    func refreshDestination() async {
        do { destination = try await link.destinationQuery() } catch { linkError = "\(error)" }
    }

    /// Sends the bundled 1×1 PNG (protocol/fixtures/capture-submission.json) as a
    /// capture with the given instructions and keeps the Mac's receipt.
    @discardableResult
    func sendTestCapture(instructions: String) async -> NearbyWire.CaptureReceived? {
        linkError = nil
        do {
            let r = try await link.submitCapture(image: PadModel.png1x1, mimeType: "image/png", instructions: instructions)
            lastCaptureReceived = r
            return r
        } catch { linkError = "\(error)"; return nil }
    }

    func disconnect() { link.disconnect(); linkStatus = "disconnected" }

    static let png1x1 = Data(base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4//8/AAX+Av4N70a4AAAAAElFTkSuQmCC")!
}
