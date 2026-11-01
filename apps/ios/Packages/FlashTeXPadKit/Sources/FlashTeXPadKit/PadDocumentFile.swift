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

/// `Application Support/FlashTeXPad/unsaved-draft.json`: one draft (the
/// iPad edits one document at a time). Writes and removals run on a serial
/// queue, in call order, never on the caller's thread.
public final class PadDraftStore: @unchecked Sendable { // `_lastError` under `lock`; the file only from `queue`
    public let url: URL
    private let queue = DispatchQueue(label: "flashtex.pad.unsaved-draft", qos: .utility)
    private let lock = NSLock()
    private var _lastError: String?

    public static func defaultURL() -> URL {
        CaptureStore.defaultDirectory().appendingPathComponent("unsaved-draft.json")
    }

    public init(url: URL = PadDraftStore.defaultURL()) { self.url = url }

    public var lastError: String? { lock.withLock { _lastError } }

    /// The draft last written, if any (synchronous: launch).
    public func load() -> PadUnsavedDraft? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        let dec = JSONDecoder(); dec.dateDecodingStrategy = .iso8601
        return try? dec.decode(PadUnsavedDraft.self, from: data)
    }

    public func save(_ draft: PadUnsavedDraft) {
        queue.async { [self] in
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

    public func clear() {
        queue.async { [self] in try? FileManager.default.removeItem(at: url) }
    }

    /// Resolves once every write and removal asked for so far is done.
    public func flush() async {
        await withCheckedContinuation { (done: CheckedContinuation<Void, Never>) in queue.async { done.resume() } }
    }
}
