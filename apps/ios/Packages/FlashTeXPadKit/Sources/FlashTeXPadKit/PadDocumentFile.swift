import CryptoKit
import Foundation

/// Reading and saving the `.tex` file an iPad buffer was opened from, the
/// way the Mac guards its saves: every write is checked against the hash of
/// the version the buffer was made from, so a file changed meanwhile (on the
/// Mac, in another app, by iCloud) is reported as a conflict and never
/// overwritten silently. Reads and writes go through `NSFileCoordinator`
/// (file providers see a coordinated access) inside the security scopes the
/// caller holds. Any thread; the app calls `save` off the main actor.
public enum PadDocumentFile {
    /// A file's text and the SHA-256 of its bytes.
    public struct Snapshot: Equatable, Sendable {
        public let text: String
        public let sha256: String
        public init(text: String, sha256: String) { self.text = text; self.sha256 = sha256 }
    }

    public enum ReadError: Error, Equatable, CustomStringConvertible {
        case notUTF8(String)
        case unreadable(String)
        public var description: String {
            switch self {
            case .notUTF8(let name): return "\(name) is not UTF-8"
            case .unreadable(let why): return why
            }
        }
    }

    public enum SaveResult: Equatable, Sendable {
        /// Written; the file's new hash.
        case saved(sha256: String)
        /// The file is not the version the buffer was made from: nothing was
        /// written. `disk` is what it holds now (nil: it is gone or unreadable).
        case conflict(disk: Snapshot?)
        case failed(String)
    }

    public static func sha256(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    /// The file's current text (coordinated read).
    public static func read(_ url: URL, scopes: [URL] = []) throws -> Snapshot {
        let entered = enter(scopes)
        defer { leave(entered) }
        var coordinationError: NSError?
        var result: Result<Data, Error> = .failure(ReadError.unreadable("\(url.lastPathComponent) could not be read"))
        NSFileCoordinator(filePresenter: nil).coordinate(readingItemAt: url, options: [], error: &coordinationError) { target in
            result = Result { try Data(contentsOf: target) }
        }
        if let coordinationError { throw ReadError.unreadable(coordinationError.localizedDescription) }
        let data: Data
        do { data = try result.get() } catch let e as ReadError { throw e } catch { throw ReadError.unreadable(error.localizedDescription) }
        guard let text = String(data: data, encoding: .utf8) else { throw ReadError.notUTF8(url.lastPathComponent) }
        return Snapshot(text: text, sha256: sha256(data))
    }

    /// Writes `text` (UTF-8) to `url` when the file still is the version
    /// hashed `expectedSha256` (nil: the file must not exist). The check and
    /// the write happen under one coordinated write, so nothing can slip in
    /// between. Atomic where the sandbox allows a temporary file beside it;
    /// a file granted on its own is rewritten in place.
    public static func save(_ text: String, to url: URL, expectedSha256: String?, scopes: [URL] = []) -> SaveResult {
        let entered = enter(scopes)
        defer { leave(entered) }
        let data = Data(text.utf8)
        var coordinationError: NSError?
        var result: SaveResult = .failed("\(url.lastPathComponent) could not be saved")
        NSFileCoordinator(filePresenter: nil).coordinate(writingItemAt: url, options: .forReplacing, error: &coordinationError) { target in
            let current = try? Data(contentsOf: target)
            let currentSha = current.map(sha256)
            guard currentSha == expectedSha256 else {
                result = .conflict(disk: current.flatMap { d in String(data: d, encoding: .utf8).map { Snapshot(text: $0, sha256: sha256(d)) } })
                return
            }
            do {
                do { try data.write(to: target, options: .atomic) } catch where PadImagePaste.isPermissionDenied(error) {
                    try data.write(to: target)
                }
                result = .saved(sha256: sha256(data))
            } catch {
                result = .failed(error.localizedDescription)
            }
        }
        if let coordinationError { return .failed(coordinationError.localizedDescription) }
        return result
    }

    private static func enter(_ scopes: [URL]) -> [URL] { scopes.filter { $0.startAccessingSecurityScopedResource() } }
    private static func leave(_ entered: [URL]) { entered.forEach { $0.stopAccessingSecurityScopedResource() } }
}

/// The unsaved text of the open document, kept on disk so a crash, a kill
/// while suspended or a failed save never loses it: written shortly after
/// each edit, removed once that text is saved, offered on the next launch.
public struct PadUnsavedDraft: Codable, Equatable, Sendable {
    public enum Source: Codable, Equatable, Sendable {
        /// A file opened from Files: its path and a bookmark to reopen it.
        case file(path: String, bookmark: Data?)
        /// The bundled `demo.tex` (nowhere to save it; the draft is its only copy).
        case bundledSample
    }
    public var source: Source
    public var title: String
    public var text: String
    /// Hash of the file version the edits were made against: restoring
    /// saves against it, so a file changed since is a conflict, not lost.
    public var diskSha256: String?
    public var savedAt: Date

    public init(source: Source, title: String, text: String, diskSha256: String?, savedAt: Date = Date()) {
        self.source = source; self.title = title; self.text = text; self.diskSha256 = diskSha256; self.savedAt = savedAt
    }
}


extension PadUnsavedDraft.Source {
    /// One draft per source: the file's path (bookmarks change; the path names
    /// the file), or the bundled demo.
    public var key: String {
        switch self {
        case .file(let path, _): return "file:" + URL(fileURLWithPath: path).standardizedFileURL.path
        case .bundledSample: return "bundled:demo.tex"
        }
    }
}

/// Unsaved-text drafts under `Application Support/FlashTeXPad/drafts/`, one
/// per source, so editing one file never replaces or removes another's:
///
/// - `current/<key>.json`: this run's draft of each edited document, written
///   shortly after an edit and removed once that text is saved;
/// - `offered/<key>-<uuid>.json`: drafts a run left behind (moved there at
///   launch by `takeOffered`) or a save that could not complete after the
///   document was closed (`offer`). They stay until the person restores or
///   discards them, whatever this run edits and saves.
///
/// Writes and removals run on a serial queue in call order, never on the
/// caller's thread.
public final class PadDraftStore: @unchecked Sendable { // `_lastError` under `lock`; files only from `queue`
    public struct Offered: Identifiable, Equatable, Sendable {
        public let id: String
        public let draft: PadUnsavedDraft
    }

    public let directory: URL
    private var current: URL { directory.appendingPathComponent("current", isDirectory: true) }
    private var offeredDir: URL { directory.appendingPathComponent("offered", isDirectory: true) }
    private let queue = DispatchQueue(label: "flashtex.pad.unsaved-drafts", qos: .utility)
    private let lock = NSLock()
    private var _lastError: String?

    public static func defaultDirectory() -> URL {
        CaptureStore.defaultDirectory().appendingPathComponent("drafts", isDirectory: true)
    }

    public init(directory: URL = PadDraftStore.defaultDirectory()) { self.directory = directory }

    public var lastError: String? { lock.withLock { _lastError } }

    static func fileName(_ source: PadUnsavedDraft.Source) -> String {
        String(PadDocumentFile.sha256(Data(source.key.utf8)).prefix(32)) + ".json"
    }

    /// This run's draft for `draft.source` (replaces only that source's).
    public func save(_ draft: PadUnsavedDraft) {
        queue.async { [self] in write(draft, to: current.appendingPathComponent(Self.fileName(draft.source))) }
    }

    /// Removes this run's draft for `source` (its text was saved). Offered
    /// drafts are never touched here.
    public func clear(_ source: PadUnsavedDraft.Source) {
        queue.async { [self] in try? FileManager.default.removeItem(at: current.appendingPathComponent(Self.fileName(source))) }
    }

    /// Keeps `draft` as an offered one (until restored or discarded).
    public func offer(_ draft: PadUnsavedDraft) -> Offered {
        let id = String(Self.fileName(draft.source).dropLast(5)) + "-" + UUID().uuidString.lowercased()
        queue.async { [self] in write(draft, to: offeredDir.appendingPathComponent(id + ".json")) }
        return Offered(id: id, draft: draft)
    }

    /// Launch: every draft an earlier run left in `current/` becomes an
    /// offered one, and all offered drafts are returned, newest first.
    public func takeOffered() -> [Offered] {
        queue.sync {
            let fm = FileManager.default
            try? fm.createDirectory(at: offeredDir, withIntermediateDirectories: true)
            for name in (try? fm.contentsOfDirectory(atPath: current.path)) ?? [] where name.hasSuffix(".json") {
                let id = String(name.dropLast(5)) + "-" + UUID().uuidString.lowercased()
                try? fm.moveItem(at: current.appendingPathComponent(name), to: offeredDir.appendingPathComponent(id + ".json"))
            }
            let dec = JSONDecoder(); dec.dateDecodingStrategy = .iso8601
            var out: [Offered] = []
            for name in (try? fm.contentsOfDirectory(atPath: offeredDir.path)) ?? [] where name.hasSuffix(".json") {
                guard let data = try? Data(contentsOf: offeredDir.appendingPathComponent(name)),
                      let draft = try? dec.decode(PadUnsavedDraft.self, from: data) else { continue }
                out.append(Offered(id: String(name.dropLast(5)), draft: draft))
            }
            return out.sorted { $0.draft.savedAt > $1.draft.savedAt }
        }
    }

    /// The person restored or discarded an offered draft.
    public func removeOffered(_ id: String) {
        queue.async { [self] in try? FileManager.default.removeItem(at: offeredDir.appendingPathComponent(id + ".json")) }
    }

    /// Everything, for `-flashtexpad-fresh`.
    public func wipe() {
        queue.async { [self] in try? FileManager.default.removeItem(at: directory) }
    }

    /// Resolves once every write and removal asked for so far is done.
    public func flush() async {
        await withCheckedContinuation { (done: CheckedContinuation<Void, Never>) in queue.async { done.resume() } }
    }

    private func write(_ draft: PadUnsavedDraft, to url: URL) {
        do {
            let enc = JSONEncoder(); enc.dateEncodingStrategy = .iso8601
            let data = try enc.encode(draft)
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try data.write(to: url, options: .atomic)
            lock.withLock { _lastError = nil }
        } catch {
            lock.withLock { _lastError = "\(error)" }
        }
    }
}
