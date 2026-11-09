import Foundation
import ObjectiveC

/// Crash insurance for a buffer with no file yet (lane REVIEW-APP).
///
/// Autosave writes only file-backed documents and the dirty-snapshot store
/// (DirtySnapshots.swift) is keyed by file, so text typed into the untitled
/// document lived only in memory: a crash, a force quit or a power cut lost
/// all of it. Now every quiet moment (the autosave timer, whatever the
/// autosave setting) keeps the untitled buffer's text in a draft under
/// Application Support (`FLASHTEX_UNTITLED_DRAFTS` overrides the directory),
/// one JSON file per model, owned by this app instance
/// (`EngineV3.instanceOwner`, pid plus start time).
///
/// The draft goes away when the buffer gets a file (Save As, an open), when
/// it is clean or empty again, and when the user discards it at quit. A
/// draft whose owner is no longer running is a crash's leftover: the next
/// untitled document (a launch, a new window) takes the newest one as its
/// text, unsaved, and says so — no file is ever written by a recovery.
struct UntitledDraft: Codable, Equatable {
    var id: String
    /// `EngineV3.instanceOwner` of the app that wrote it.
    var owner: String
    var text: String
    var savedAt: Date
}

@MainActor
final class UntitledDraftStore {
    var directory: URL
    private(set) var lastError: String?

    init(directory: URL? = nil) { self.directory = directory ?? Self.defaultDirectory() }

    static func defaultDirectory() -> URL {
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_UNTITLED_DRAFTS"], !env.isEmpty {
            return URL(fileURLWithPath: env, isDirectory: true)
        }
        if NSClassFromString("XCTestCase") != nil {
            return FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-untitled-drafts-xctest-\(getpid())", isDirectory: true)
        }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        return base.appendingPathComponent("FlashTeX/untitled-drafts", isDirectory: true)
    }

    private func file(_ id: String) -> URL { directory.appendingPathComponent(id + ".json") }

    private static let encoder: JSONEncoder = { let e = JSONEncoder(); e.dateEncodingStrategy = .iso8601; return e }()
    private static let decoder: JSONDecoder = { let d = JSONDecoder(); d.dateDecodingStrategy = .iso8601; return d }()

    @discardableResult
    func write(_ draft: UntitledDraft) -> Bool {
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try Self.encoder.encode(draft).write(to: file(draft.id), options: .atomic)
            lastError = nil
            return true
        } catch {
            lastError = "could not keep the untitled draft: \(error.localizedDescription)"
            FlashTeXLog.write("drafts: " + lastError!)
            return false
        }
    }

    func read(_ id: String) -> UntitledDraft? {
        guard let data = try? Data(contentsOf: file(id)) else { return nil }
        return try? Self.decoder.decode(UntitledDraft.self, from: data)
    }

    func remove(_ id: String) { try? FileManager.default.removeItem(at: file(id)) }

    /// Drafts whose app is no longer running (a crash's), newest first.
    func leftovers(isAlive: (String) -> Bool = EngineV3.ownerAlive) -> [UntitledDraft] {
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: directory.path) else { return [] }
        return names.filter { $0.hasSuffix(".json") }
            .compactMap { name -> UntitledDraft? in
                guard let data = try? Data(contentsOf: directory.appendingPathComponent(name)) else { return nil }
                return try? Self.decoder.decode(UntitledDraft.self, from: data)
            }
            .filter { !isAlive($0.owner) }
            .sorted { $0.savedAt > $1.savedAt }
    }
}

extension ShellModel {
    private static var draftStoreKey = 0
    private static var draftIDKey = 0

    var untitledDrafts: UntitledDraftStore {
        if let existing = objc_getAssociatedObject(self, &Self.draftStoreKey) as? UntitledDraftStore { return existing }
        let store = UntitledDraftStore()
        objc_setAssociatedObject(self, &Self.draftStoreKey, store, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return store
    }

    /// This model's draft (one per window's model).
    var untitledDraftID: String {
        if let id = objc_getAssociatedObject(self, &Self.draftIDKey) as? String { return id }
        let id = UUID().uuidString
        objc_setAssociatedObject(self, &Self.draftIDKey, id, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return id
    }

    /// Keeps (or drops) the untitled buffer's draft: called by the autosave
    /// pass when the buffer has no file.
    func keepUntitledDraft() {
        guard documentURL == nil else { dropUntitledDraft(); return }
        let text = entryText
        guard project.isDirty(project.entryPath), !text.isEmpty else { dropUntitledDraft(); return }
        if let kept = untitledDrafts.read(untitledDraftID), kept.text.sameBytes(as: text) { return }
        untitledDrafts.write(UntitledDraft(id: untitledDraftID, owner: EngineV3.instanceOwner, text: text, savedAt: Date()))
    }

    /// The draft is no longer needed (the buffer has a file, or the user
    /// discarded it).
    func dropUntitledDraft() { untitledDrafts.remove(untitledDraftID) }

    /// A crash's newest untitled draft becomes this untitled buffer's text,
    /// unsaved; the draft is now this model's (a second crash keeps it).
    /// False when there is none, or this buffer is not the untouched
    /// untitled document.
    @discardableResult
    func recoverUntitledDraft(isAlive: (String) -> Bool = EngineV3.ownerAlive) -> Bool {
        guard documentURL == nil, untitledDocumentCaret != nil, let draft = untitledDrafts.leftovers(isAlive: isAlive).first else { return false }
        updateActiveText(draft.text)
        guard untitledDrafts.write(UntitledDraft(id: untitledDraftID, owner: EngineV3.instanceOwner, text: draft.text, savedAt: draft.savedAt)) else { return true }
        untitledDrafts.remove(draft.id)
        let when = DateFormatter.localizedString(from: draft.savedAt, dateStyle: .medium, timeStyle: .short)
        captureNote = "Recovered \(draft.text.utf8.count) bytes of unsaved untitled text from \(when) (FlashTeX did not quit normally). Save it with ⌘S."
        FlashTeXLog.write("drafts: recovered untitled draft \(draft.id) (\(draft.text.utf8.count) bytes)")
        return true
    }
}
