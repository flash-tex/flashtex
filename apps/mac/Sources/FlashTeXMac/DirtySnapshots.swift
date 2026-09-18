import AppKit
import ObjectiveC
import FlashTeXProtocol

/// Durable copies of unsaved text (lane mac-document-files-2).
///
/// `recoverableBuffer` and `ProjectDocuments.detachedBuffers` keep discarded
/// text for the session only, and only while the preview controller's ledger
/// is *not* the durable home of that text (a session temporary project for
/// an unsaved buffer, the direct worker route, the fixture route, or no
/// helper at all). This store makes such text survive the process: one JSON snapshot
/// per file under Application Support (`FLASHTEX_DIRTY_SNAPSHOTS` overrides
/// the directory for tests and automation), written when
///
/// - a reload or open discards a dirty buffer (`adoptOpenedText`),
/// - a non-entry member is detached with unsaved edits,
/// - an external change is detected while the buffer is dirty (both texts are
///   then recoverable: the buffer here, the disk text through the reviewed
///   reload), a conflicting save included,
/// - the app quits without saving (`preserveDirtyBuffers`, parent hook).
///
/// Opening a file (entry or member) whose snapshot still differs from the
/// disk text offers it (`files.offeredSnapshots`, `captureNote`); the user
/// restores it explicitly (`restoreDirtySnapshot`, buffer becomes dirty
/// against the current disk text — never a silent overwrite) or discards it.
/// A snapshot whose text is what the file now holds is moot and removed. A
/// successful save of exactly the snapshot's text removes it too.
struct DirtySnapshot: Codable, Equatable, Identifiable {
    /// Absolute path of the file the text belongs to (the entry document's
    /// URL, or `<project root>/<rooted path>` for a member).
    var file: String
    var text: String
    /// Hash of the disk file the text was dirty against (nil: no file then).
    var diskSha256: String?
    var savedAt: Date
    var reason: String

    var url: URL { URL(fileURLWithPath: file) }
    var id: String { DirtySnapshotStore.key(for: url) }
    var summary: String {
        "\(url.lastPathComponent): \(text.utf8.count) bytes of unsaved text (\(reason), \(Self.stamp.string(from: savedAt)))"
    }
    nonisolated(unsafe) private static let stamp: DateFormatter = {
        let f = DateFormatter()
        f.dateStyle = .medium
        f.timeStyle = .short
        return f
    }()
}

/// The on-disk store: `<directory>/<sha256(path)>.json`, one entry per file,
/// atomic writes. Everything is best-effort and logged: a store that cannot
/// be written never blocks the file operation that triggered it (the text is
/// still in memory), it just is not durable.
@MainActor
final class DirtySnapshotStore {
    var directory: URL
    /// Last store failure (tests read this; the UI sees the log).
    private(set) var lastError: String?

    init(directory: URL? = nil) {
        self.directory = directory ?? Self.defaultDirectory()
    }

    static func defaultDirectory() -> URL {
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_DIRTY_SNAPSHOTS"], !env.isEmpty {
            return URL(fileURLWithPath: env, isDirectory: true)
        }
        if NSClassFromString("XCTestCase") != nil {
            // Test processes exercise discard/reload paths on temp files all
            // the time; keep their snapshots out of the user's store.
            return FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-dirty-snapshots-xctest-\(getpid())", isDirectory: true)
        }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        return base.appendingPathComponent("FlashTeX/dirty-snapshots", isDirectory: true)
    }

    nonisolated static func key(for url: URL) -> String {
        String(SourceDigest.sha256Hex(url.standardizedFileURL.path).prefix(32))
    }

    func fileURL(for url: URL) -> URL { directory.appendingPathComponent(Self.key(for: url) + ".json") }

    private static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.outputFormatting = [.prettyPrinted, .sortedKeys]
        e.dateEncodingStrategy = .iso8601
        return e
    }()
    private static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        return d
    }()

    /// Writes (replaces) the snapshot for its file. Returns the JSON path, or
    /// nil (with `lastError`) when the store is not writable.
    @discardableResult
    func write(_ snapshot: DirtySnapshot) -> URL? {
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let target = fileURL(for: snapshot.url)
            try Self.encoder.encode(snapshot).write(to: target, options: .atomic)
            lastError = nil
            FlashTeXLog.write("snapshots: kept \(snapshot.text.utf8.count) bytes of unsaved \(snapshot.url.lastPathComponent) (\(snapshot.reason)) at \(target.path)")
            return target
        } catch {
            lastError = "could not write the unsaved-text snapshot for \(snapshot.url.lastPathComponent): \(error.localizedDescription)"
            FlashTeXLog.write("snapshots: " + lastError!)
            return nil
        }
    }

    func read(for url: URL) -> DirtySnapshot? {
        guard let data = try? Data(contentsOf: fileURL(for: url)) else { return nil }
        guard let s = try? Self.decoder.decode(DirtySnapshot.self, from: data),
              s.url.standardizedFileURL.path == url.standardizedFileURL.path else { return nil }
        return s
    }

    func remove(for url: URL) {
        try? FileManager.default.removeItem(at: fileURL(for: url))
    }

    /// Every readable snapshot in the store, newest first.
    func all() -> [DirtySnapshot] {
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: directory.path) else { return [] }
        return names.filter { $0.hasSuffix(".json") }
            .compactMap { name -> DirtySnapshot? in
                guard let data = try? Data(contentsOf: directory.appendingPathComponent(name)) else { return nil }
                return try? Self.decoder.decode(DirtySnapshot.self, from: data)
            }
            .sorted { $0.savedAt > $1.savedAt }
    }

    /// Snapshots of files under `root` (the project directory), newest first.
    func snapshots(under root: URL) -> [DirtySnapshot] {
        let prefix = root.standardizedFileURL.path + "/"
        return all().filter { $0.url.standardizedFileURL.path.hasPrefix(prefix) }
    }
}

extension ShellModel {
    private static var snapshotsKey = 0
    /// Durable unsaved-text store (see `DirtySnapshotStore`).
    var dirtySnapshots: DirtySnapshotStore {
        if let existing = objc_getAssociatedObject(self, &Self.snapshotsKey) as? DirtySnapshotStore { return existing }
        let store = DirtySnapshotStore()
        objc_setAssociatedObject(self, &Self.snapshotsKey, store, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return store
    }

    /// What `url` holds on disk right now: a plain read, not the rooted
    /// helper (a hash for the record; nothing is written and the helper
    /// bound to the open document's root must not be rebound for it).
    private static func diskText(at url: URL) -> String? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    /// Keeps `text` (the unsaved buffer of `url`) durably. `reason` is shown
    /// when the snapshot is offered. A text identical to the disk file is not
    /// dirty and is not kept (a stale snapshot for that file is removed).
    @discardableResult
    func preserveDirtyText(_ text: String, at url: URL, reason: String) -> DirtySnapshot? {
        let disk = Self.diskText(at: url)
        if let disk, disk.sameBytes(as: text) {
            dirtySnapshots.remove(for: url)
            return nil
        }
        let diskHash = disk.map { SourceDigest.sha256Hex($0) }
        let snapshot = DirtySnapshot(file: url.standardizedFileURL.path, text: text, diskSha256: diskHash, savedAt: Date(), reason: reason)
        guard dirtySnapshots.write(snapshot) != nil else { return nil }
        return snapshot
    }

    /// `preserveDirtyText` for a discard that drops the only in-memory copy:
    /// false when the text differs from the file and the store could not keep
    /// it (`dirtySnapshots.lastError` says why), so the caller must not
    /// replace the buffer (#806).
    func preserveDiscardedText(_ text: String, at url: URL, reason: String) -> Bool {
        preserveDirtyText(text, at: url, reason: reason) != nil || Self.diskText(at: url)?.sameBytes(as: text) == true
    }

    /// Keeps every dirty member durably (entry: its URL; members: their
    /// rooted files). For the quit flow's "Don't Save" and
    /// `applicationWillTerminate` (parent hooks), and callable any time.
    /// Returns what was written; a buffer without a file cannot be kept.
    @discardableResult
    func preserveDirtyBuffers(reason: String) -> [DirtySnapshot] {
        var kept: [DirtySnapshot] = []
        for doc in project.listing where doc.isDirty {
            guard let text = documents.first(where: { $0.path == doc.path })?.text else { continue }
            let url: URL?
            if doc.path == project.entryPath { url = documentURL }
            else if let root = project.projectRoot { url = root.appendingPathComponent(doc.path) }
            else { url = nil }
            guard let url, let s = preserveDirtyText(text, at: url, reason: reason) else { continue }
            kept.append(s)
        }
        return kept
    }

    /// Offers the stored snapshot of `url` if it still differs from what the
    /// file holds (`currentText`); a moot snapshot is removed. Called after an
    /// open (entry and members); never changes the buffer.
    @discardableResult
    func offerDirtySnapshot(for url: URL, currentText: String) -> DirtySnapshot? {
        guard let s = dirtySnapshots.read(for: url) else { return nil }
        if s.text.sameBytes(as: currentText) {
            dirtySnapshots.remove(for: url)
            files.offeredSnapshots.removeAll { $0.id == s.id }
            return nil
        }
        files.offeredSnapshots.removeAll { $0.id == s.id }
        files.offeredSnapshots.append(s)
        FlashTeXLog.write("snapshots: offering unsaved text for \(url.lastPathComponent) (\(s.reason))")
        return s
    }

    /// Restores an offered snapshot into its document: the entry (buffer must
    /// be clean, like `restoreDiscardedBuffer`) or an open member (switched
    /// to). The buffer is then dirty against the current disk text; nothing
    /// is written to disk. The snapshot is consumed.
    @discardableResult
    func restoreDirtySnapshot(_ snapshot: DirtySnapshot) -> Bool {
        let target = snapshot.url.standardizedFileURL
        if let documentURL, documentURL.standardizedFileURL == target {
            if project.isDirty(project.entryPath) {
                captureNote = "Current buffer has unsaved edits; save it before restoring the unsaved snapshot of \(target.lastPathComponent)."
                return false
            }
            if activePath != project.entryPath, case .refused(let why) = project.switchDocument(to: project.entryPath) {
                captureNote = "Cannot restore the snapshot of \(target.lastPathComponent): \(why)"
                return false
            }
            updateActiveText(snapshot.text)
        } else if let root = project.projectRoot, target.path.hasPrefix(root.standardizedFileURL.path + "/") {
            let path = String(target.path.dropFirst(root.standardizedFileURL.path.count + 1))
            guard project.isOpen(path) else {
                captureNote = "Open \(path) in the project before restoring its unsaved snapshot."
                return false
            }
            if project.isDirty(path) {
                captureNote = "\(path) has unsaved edits; save it before restoring the unsaved snapshot."
                return false
            }
            if case .refused(let why) = project.switchDocument(to: path) {
                captureNote = "Cannot restore the snapshot of \(path): \(why)"
                return false
            }
            updateActiveText(snapshot.text)
        } else {
            captureNote = "\(target.lastPathComponent) is not part of the open project; open it first to restore its unsaved snapshot."
            return false
        }
        dirtySnapshots.remove(for: target)
        files.offeredSnapshots.removeAll { $0.id == snapshot.id }
        captureNote = "Restored \(snapshot.text.utf8.count) bytes of unsaved text into \(target.lastPathComponent) (unsaved; \(snapshot.reason))."
        return true
    }

    /// Drops an offered snapshot for good (explicit decision).
    func discardDirtySnapshot(_ snapshot: DirtySnapshot) {
        dirtySnapshots.remove(for: snapshot.url)
        files.offeredSnapshots.removeAll { $0.id == snapshot.id }
        captureNote = "Discarded the unsaved snapshot of \(snapshot.url.lastPathComponent)."
    }

    /// A save of `url` that wrote exactly a snapshot's text consumes it.
    func snapshotSaved(url: URL, text: String) {
        guard let s = dirtySnapshots.read(for: url) else { return }
        if s.text.sameBytes(as: text) {
            dirtySnapshots.remove(for: url)
            files.offeredSnapshots.removeAll { $0.id == s.id }
        }
    }

    /// Modal offer of the stored snapshots for the open project: one alert per
    /// snapshot, Restore / Discard / Later. Never automatic.
    func restoreDirtySnapshotsInteractive() {
        let offered = files.offeredSnapshots
        guard !offered.isEmpty else { captureNote = "No unsaved snapshots for this project."; return }
        for s in offered {
            let alert = NSAlert()
            alert.messageText = "Restore unsaved text for \(s.url.lastPathComponent)?"
            alert.informativeText = s.summary + "\nRestoring makes the buffer unsaved again; nothing is written to disk until you save."
            alert.addButton(withTitle: "Restore")
            alert.addButton(withTitle: "Discard Snapshot")
            alert.addButton(withTitle: "Later")
            switch alert.runModal() {
            case .alertFirstButtonReturn: restoreDirtySnapshot(s)
            case .alertSecondButtonReturn: discardDirtySnapshot(s)
            default: break
            }
        }
    }
}
