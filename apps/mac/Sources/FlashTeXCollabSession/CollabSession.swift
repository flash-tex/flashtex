import FlashTeXCollabCore
import Foundation

/// What an editor showing one session file does for the session (the Mac's
/// `NSTextView` coordinator, later the iPad's `EditorController`).
@MainActor
public protocol CollabTextHost: AnyObject {
    /// True while the view has marked text (an input-method composition):
    /// remote operations for its file wait until it ends (proposal §2.4).
    var collabIsComposing: Bool { get }
    /// The selection, UTF-16, in the text the session last agreed with.
    var collabSelection: Range<Int> { get }
    /// Hand any local edit still pending in the view to the binding, so the
    /// view and the CRDT agree before remote operations are integrated.
    func collabFlushLocalEdits()
    /// Apply `changes` in order (each in UTF-16 units of the text just
    /// before it), as minimal storage edits, never a whole-text reset, then
    /// select `selection` when it is non-nil. Not an undoable user edit.
    func collabApply(_ changes: [TextChange], selection: Range<Int>?)
}

/// The session state every participant holds: the CRDT project, one
/// binding per text file, the outbound queue and everyone's presence. The
/// transport (`CollabHub` or `CollabGuest`) moves its messages; nothing here
/// touches the network.
@MainActor
public final class CollabSession {
    public enum Role: Equatable, Sendable { case hub, guest }

    public struct Participant: Equatable, Sendable {
        public var id: UInt64
        public var name: String
        public var colourIndex: Int
        public var isLocal: Bool
    }

    /// Another participant's caret or selection in one file, resolved now.
    public struct RemoteCursor: Equatable, Sendable {
        public var participant: UInt64
        public var name: String
        public var colourIndex: Int
        /// UTF-16 range; empty for a caret.
        public var range: Range<Int>
        /// The caret end (UTF-16).
        public var head: Int
        /// Seconds since this participant's caret last moved.
        public var idle: TimeInterval
    }

    struct Presence {
        var name: String
        var colourIndex: Int
        var file: FileID?
        var anchor: RelativePosition?
        var head: RelativePosition?
        var seq: UInt64
        var lastSeen: Date
        var lastMoved: Date
    }

    /// What the transport sends for the session.
    enum Outbound {
        case sections([Section])
        case awareness(CollabControl.Awareness)
    }

    public let role: Role
    public let project: CollabProject
    public var replica: UInt64 { project.replica }
    public private(set) var localName: String
    public private(set) var localColour: Int
    /// A viewer may not edit (the hub drops its operations anyway).
    public internal(set) var canEdit = true

    /// Remote text arrived for a file no editor is showing (or a file
    /// appeared): the owner updates its own copy (`text(of:)`).
    public var onRemoteText: (FileID) -> Void = { _ in }
    /// The file map changed (files created, renamed or deleted).
    public var onFilesChanged: () -> Void = {}
    /// Someone's caret, selection, name or presence changed.
    public var onPresenceChanged: () -> Void = {}
    /// Operations refused for good (diagnostics; the transport resyncs).
    public var onRejected: ([CollabProject.ReceiveError]) -> Void = { _ in }

    var transmit: (Outbound) -> Void = { _ in }
    private var bindings: [FileID: CollabTextBinding] = [:]
    private var presences: [UInt64: Presence] = [:]
    private var outbound: [Section] = []
    private var flushScheduled = false
    private var awarenessSeq: UInt64 = 0
    private var localFile: FileID?
    private var localAnchor: RelativePosition?
    private var localHead: RelativePosition?
    private var awarenessScheduled = false
    private var lastAwarenessSent = Date.distantPast
    private var timer: Timer?
    /// Presence is dropped this long after its last heartbeat (§4).
    public static let presenceExpiry: TimeInterval = 10
    static let heartbeat: TimeInterval = 3
    static let awarenessInterval: TimeInterval = 0.05

    public init(role: Role, replica: UInt64, name: String, colourIndex: Int) {
        self.role = role
        project = CollabProject(replica: replica)
        localName = name
        localColour = colourIndex
    }

    func startTimers() {
        guard timer == nil else { return }
        let t = Timer(timeInterval: 1, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.tick() }
        }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    public func stopTimers() {
        timer?.invalidate()
        timer = nil
    }

    private func tick() {
        let now = Date()
        let before = presences.count
        presences = presences.filter { now.timeIntervalSince($0.value.lastSeen) < Self.presenceExpiry }
        if presences.count != before { onPresenceChanged() }
        if now.timeIntervalSince(lastAwarenessSent) >= Self.heartbeat { sendAwareness() }
    }

    // MARK: Files

    /// Live text files: id and materialised path (conflicts renamed).
    public var textFiles: [(file: FileID, path: String)] {
        project.files.files.filter { $0.kind == .text }.map { ($0.file, $0.path) }
    }

    public func path(of file: FileID) -> String? {
        project.files.files.first { $0.file == file }?.path
    }

    public func file(atPath path: String) -> FileID? {
        project.files.files.first { $0.path == path && $0.kind == .text }?.file
    }

    public func text(of file: FileID) -> String? { project.text(file)?.text }

    /// The hub shares a file: a create plus its text, broadcast with the
    /// next flush. Not undoable.
    @discardableResult
    public func shareFile(path: String, text: String) throws -> FileID {
        let id = FileID.random()
        let create = try project.fileOp { try $0.create(id, kind: .text, path: path) }
        enqueue(create)
        if let insert = try project.insert(text, at: 0, in: id) { enqueue(insert) }
        return id
    }

    public func binding(for file: FileID) -> CollabTextBinding? {
        if let b = bindings[file] { return b }
        guard let doc = project.text(file) else { return nil }
        let b = CollabTextBinding(file: file, document: doc, session: self)
        bindings[file] = b
        return b
    }

    // MARK: Outbound

    func enqueue(_ section: Section) {
        if case let .text(f, ops) = section, case let .text(g, prev)? = outbound.last, f == g {
            outbound[outbound.count - 1] = .text(f, prev + ops)
        } else {
            outbound.append(section)
        }
        guard !flushScheduled else { return }
        flushScheduled = true
        // One `update` per run-loop turn: a keystroke's operations, and an
        // auto-closed pair's, leave together.
        DispatchQueue.main.async { [weak self] in MainActor.assumeIsolated { self?.flush() } }
    }

    /// Sends what is queued now.
    public func flush() {
        flushScheduled = false
        guard !outbound.isEmpty else { return }
        let sections = outbound
        outbound = []
        transmit(.sections(sections))
    }

    // MARK: Inbound

    /// Integrates remote sections. Text for a file whose editor is composing
    /// waits in that file's binding until the composition ends; everything
    /// else is integrated now, its changes applied to bound editors with the
    /// local selection kept by relative position.
    @discardableResult
    func integrate(_ sections: [Section]) -> [CollabProject.ReceiveError] {
        // A composition that ended without the host saying so (cancelled
        // with Esc): what waited for it goes first, in arrival order.
        for b in bindings.values where !b.held.isEmpty && !b.mustHold { b.release() }
        var now: [Section] = []
        for s in sections {
            if case let .text(f, ops) = s, let b = bindings[f], b.mustHold {
                b.held += ops
            } else {
                now.append(s)
            }
        }
        return apply(now)
    }

    func apply(_ sections: [Section]) -> [CollabProject.ReceiveError] {
        guard !sections.isEmpty else { return [] }
        struct Watch {
            var binding: CollabTextBinding
            var host: CollabTextHost
            var anchor: RelativePosition
            var head: RelativePosition
            var caret: Bool
            var changes: [TextChange] = []
        }
        var watches: [FileID: Watch] = [:]
        var textFiles = Set<FileID>()
        var fileMapChanged = false
        for s in sections {
            switch s {
            case .fileMap: fileMapChanged = true
            case let .text(f, _): textFiles.insert(f)
            }
        }
        // Every bound editor is watched, not only the files named here: an
        // operation parked earlier can wake for any of them.
        for (f, b) in bindings {
            guard let host = b.host else { continue }
            host.collabFlushLocalEdits()
            let sel = host.collabSelection
            let doc = b.document
            let a = doc.scalarOffset(ofUTF16: min(sel.lowerBound, doc.utf16Count))
            let h = doc.scalarOffset(ofUTF16RoundingUp: min(sel.upperBound, doc.utf16Count))
            let caret = a == h
            watches[f] = Watch(binding: b, host: host,
                               anchor: doc.relativePosition(at: a, assoc: caret ? .after : .before),
                               head: doc.relativePosition(at: h, assoc: .after), caret: caret)
            doc.changeObserver = { [weak self] c in
                guard self != nil else { return }
                watches[f]?.changes.append(c)
            }
        }
        let errors = project.receive(sections)
        for (f, w) in watches {
            w.binding.document.changeObserver = nil
            guard !w.changes.isEmpty else { continue }
            let doc = w.binding.document
            var selection: Range<Int>?
            if let a = doc.resolve(w.anchor), let h = doc.resolve(w.head) {
                let ua = doc.utf16Offset(ofScalar: a), uh = doc.utf16Offset(ofScalar: h)
                selection = w.caret ? uh..<uh : min(ua, uh)..<max(ua, uh)
            }
            w.host.collabApply(w.changes, selection: selection)
            textFiles.remove(f)
        }
        for f in textFiles.sorted() where watches[f] == nil { onRemoteText(f) }
        if fileMapChanged { onFilesChanged() }
        if !errors.isEmpty { onRejected(errors) }
        return errors
    }

    // MARK: Presence

    /// The local caret moved (or the file did): sent at most every 50 ms.
    public func setLocalPresence(file: FileID?, selection: Range<Int>?) {
        if let file, let sel = selection, let doc = project.text(file) {
            let a = doc.scalarOffset(ofUTF16: min(sel.lowerBound, doc.utf16Count))
            let h = doc.scalarOffset(ofUTF16RoundingUp: min(sel.upperBound, doc.utf16Count))
            localAnchor = doc.relativePosition(at: a, assoc: a == h ? .after : .before)
            localHead = doc.relativePosition(at: h, assoc: .after)
        } else {
            localAnchor = nil
            localHead = nil
        }
        localFile = file
        scheduleAwareness()
    }

    private func scheduleAwareness() {
        guard !awarenessScheduled else { return }
        let wait = max(0, Self.awarenessInterval - Date().timeIntervalSince(lastAwarenessSent))
        awarenessScheduled = true
        DispatchQueue.main.asyncAfter(deadline: .now() + wait) { [weak self] in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.awarenessScheduled = false
                self.sendAwareness()
            }
        }
    }

    var localAwareness: CollabControl.Awareness {
        CollabControl.Awareness(participantID: collabHex(replica), name: localName, colourIndex: localColour,
                                file: localFile?.hex, anchor: localAnchor.map(CollabControl.Position.init),
                                head: localHead.map(CollabControl.Position.init), previewPage: nil, following: nil,
                                seq: awarenessSeq)
    }

    func sendAwareness() {
        awarenessSeq += 1
        lastAwarenessSent = Date()
        transmit(.awareness(localAwareness))
    }

    /// A peer's presence (already vetted by the hub when relayed).
    func receivePresence(_ a: CollabControl.Awareness) {
        guard let id = collabParseHex(a.participantID), id != replica else { return }
        if let old = presences[id], old.seq >= a.seq { return }
        let file = a.file.flatMap { hex -> FileID? in
            guard hex.count == 32 else { return nil }
            var bytes: [UInt8] = []
            var i = hex.startIndex
            while i < hex.endIndex {
                let j = hex.index(i, offsetBy: 2)
                guard let b = UInt8(hex[i..<j], radix: 16) else { return nil }
                bytes.append(b)
                i = j
            }
            return FileID(bytes: bytes)
        }
        let now = Date()
        let old = presences[id]
        let anchor = a.anchor?.relativePosition, head = a.head?.relativePosition
        let moved = old == nil || old!.anchor != anchor || old!.head != head || old!.file != file
        presences[id] = Presence(name: String(a.name.prefix(64)), colourIndex: a.colourIndex, file: file, anchor: anchor,
                                 head: head, seq: a.seq, lastSeen: now, lastMoved: moved ? now : old!.lastMoved)
        onPresenceChanged()
    }

    func dropPresence(_ id: UInt64) {
        if presences.removeValue(forKey: id) != nil { onPresenceChanged() }
    }

    /// Everyone in the session that this peer knows of, itself first.
    public var participants: [Participant] {
        [Participant(id: replica, name: localName, colourIndex: localColour, isLocal: true)]
            + presences.sorted { $0.key < $1.key }.map {
                Participant(id: $0.key, name: $0.value.name, colourIndex: $0.value.colourIndex, isLocal: false)
            }
    }

    /// Other participants' carets and selections in `file`, resolved now.
    public func remoteCursors(in file: FileID) -> [RemoteCursor] {
        guard let doc = project.text(file) else { return [] }
        let now = Date()
        return presences.sorted { $0.key < $1.key }.compactMap { id, p in
            guard p.file == file, let hp = p.head, let h = doc.resolve(hp) else { return nil }
            let a = p.anchor.flatMap { doc.resolve($0) } ?? h
            let ua = doc.utf16Offset(ofScalar: min(a, doc.count)), uh = doc.utf16Offset(ofScalar: min(h, doc.count))
            return RemoteCursor(participant: id, name: p.name, colourIndex: p.colourIndex,
                                range: min(ua, uh)..<max(ua, uh), head: uh, idle: now.timeIntervalSince(p.lastMoved))
        }
    }
}

/// One text file of a session as one editor sees it: local edits in, remote
/// changes and undo out. Undo is local (`TextUndoManager`): it reverts only
/// this participant's edits, never anyone else's (proposal §2.4).
@MainActor
public final class CollabTextBinding {
    public let file: FileID
    public let document: TextDocument
    public let undoManager: TextUndoManager
    unowned let session: CollabSession
    public weak var host: CollabTextHost?
    /// Remote operations waiting for the host's composition to end.
    var held: [TextOp] = []
    /// Where the open typing step ends (UTF-16), so the next adjacent
    /// keystroke joins it; nil when no step is open.
    private var typingEnd: Int?
    /// Called when a new undo step opens: an editor mirrors it with one
    /// registration on its `UndoManager` (Edit ▸ Undo, ⌘Z, VoiceOver).
    public var onUndoStepOpened: () -> Void = {}

    init(file: FileID, document: TextDocument, session: CollabSession) {
        self.file = file
        self.document = document
        undoManager = TextUndoManager(document: document)
        self.session = session
    }

    var mustHold: Bool { host?.collabIsComposing == true }

    /// Remote operations waiting for a composition to end.
    public var heldCount: Int { held.count }

    /// Attach (or detach, nil) the editor showing this file. Anything held
    /// for the previous host is integrated first.
    public func attach(_ host: CollabTextHost?) {
        if self.host !== host { breakUndoCoalescing() }
        self.host = nil
        if !held.isEmpty { release() }
        self.host = host
    }

    /// The host's composition ended: integrate what waited for it.
    public func release() {
        guard !held.isEmpty, host?.collabIsComposing != true else { return }
        let ops = held
        held = []
        _ = session.apply([.text(file, ops)])
    }

    // MARK: Local edits

    /// A local edit of the host's text: UTF-16 `range` of the text before it
    /// replaced by `text`. Adjacent typing coalesces into one undo step.
    public func localReplace(_ range: Range<Int>, with text: String) {
        guard session.canEdit else { return }
        let ops: [TextOp]
        do { ops = try document.replace(utf16Range: range, with: text) } catch { return }
        guard !ops.isEmpty else { return }
        let continues = typingEnd.map { end in
            (range.isEmpty && range.lowerBound == end) || (text.isEmpty && range.upperBound == end)
        } ?? false
        if !continues {
            closeStep()
            undoManager.beginGroup()
            onUndoStepOpened()
        }
        undoManager.record(ops)
        typingEnd = range.lowerBound + text.utf16.count
        session.enqueue(.text(file, ops))
    }

    /// Replaces the CRDT text with `text` by its smallest differing span (a
    /// reload or revert in the editor): one undo step of its own.
    public func adopt(text: String) {
        let old = Array(document.text.utf16), new = Array(text.utf16)
        var p = 0
        while p < old.count, p < new.count, old[p] == new[p] { p += 1 }
        var s = 0
        while s < old.count - p, s < new.count - p, old[old.count - 1 - s] == new[new.count - 1 - s] { s += 1 }
        guard p < old.count - s || p < new.count - s else { return }
        let replacement = String(utf16CodeUnits: Array(new[p..<(new.count - s)]), count: new.count - s - p)
        breakUndoCoalescing()
        localReplace(p..<(old.count - s), with: replacement)
        breakUndoCoalescing()
    }

    /// The next edit starts a new undo step (a caret move, a capture).
    public func breakUndoCoalescing() { closeStep() }

    private func closeStep() {
        typingEnd = nil
        if undoManager.isGrouping { undoManager.endGroup() }
    }

    // MARK: Undo

    public var canUndo: Bool { undoManager.canUndo || undoManager.isGrouping }
    public var canRedo: Bool { undoManager.canRedo }

    /// Undoes this participant's latest step. Returns whether a redo step
    /// resulted (an undo whose text others already removed has none).
    @discardableResult
    public func undo() -> Bool { revert(redo: false) }

    @discardableResult
    public func redo() -> Bool { revert(redo: true) }

    private func revert(redo: Bool) -> Bool {
        guard host?.collabIsComposing != true else { return false }
        host?.collabFlushLocalEdits()
        if !held.isEmpty { release() }
        closeStep()
        let before = redo ? undoManager.undoCount : undoManager.redoCount
        var changes: [TextChange] = []
        document.changeObserver = { changes.append($0) }
        let ops = redo ? undoManager.redo() : undoManager.undo()
        document.changeObserver = nil
        if !changes.isEmpty {
            let last = changes[changes.count - 1]
            let end = last.location + last.text.utf16.count
            host?.collabApply(changes, selection: last.text.isEmpty ? end..<end : last.location..<end)
            if host == nil { session.onRemoteText(file) }
        }
        if !ops.isEmpty { session.enqueue(.text(file, ops)) }
        return (redo ? undoManager.undoCount : undoManager.redoCount) > before
    }
}
