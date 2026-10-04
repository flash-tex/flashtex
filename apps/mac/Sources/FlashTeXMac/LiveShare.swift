import AppKit
import FlashTeXCollabCore
import FlashTeXCollabSession
import FlashTeXProtocol
import Foundation
import Observation

/// Live Share (preview): co-editing one project's sources with other Macs on
/// the LAN, or with another FlashTeX instance on this Mac (proposal
/// docs/design/live-collab/PROPOSAL.md, P1). Off unless Settings ▸ Live
/// Share turns it on (or `FLASHTEX_LIVE_SHARE=1`).
///
/// Hosting shares every text source under the project folder (the open
/// buffers as they are, the rest as on disk). A guest's copy lives in its
/// own folder under Application Support (per `FLASHTEX_INSTANCE`), opened
/// as a normal project, so each Mac compiles the converged text locally
/// (§5.2). Figures and other binary files are not shared yet (P2).
@MainActor
@Observable
final class LiveShareController {
    enum Phase: Equatable {
        case idle
        case hosting
        case connecting
        case awaitingApproval
        case joined
        case reconnecting(attempt: Int)
        case ended(String)
    }

    struct Approval: Identifiable, Equatable {
        let id = UUID()
        var name: String
        var deviceKind: String
    }

    /// Settings ▸ Live Share. Read where the menu and the sheet decide.
    static let enabledKey = "liveShareEnabled"
    static var enabled: Bool {
        ProcessInfo.processInfo.environment["FLASHTEX_LIVE_SHARE"] == "1" || UserDefaults.standard.bool(forKey: enabledKey)
    }

    /// `FLASHTEX_INSTANCE`: a second app instance on this Mac keeps its
    /// session copies apart (and says who it is).
    static var instance: String? {
        guard let i = ProcessInfo.processInfo.environment["FLASHTEX_INSTANCE"], !i.isEmpty else { return nil }
        return String(i.prefix(32))
    }

    static var defaultDisplayName: String {
        let name = NSFullUserName().isEmpty ? Host.current().localizedName ?? "Me" : NSFullUserName()
        return instance.map { "\(name) (\($0))" } ?? name
    }

    /// Tests: listen on loopback only, never advertise, and keep guests'
    /// copies under this folder instead of Application Support.
    static var testLoopbackOnly = false
    static var sessionBaseOverride: URL?

    @ObservationIgnored weak var model: ShellModel?
    private(set) var phase: Phase = .idle { didSet { if phase != oldValue { FlashTeXLog.write("liveshare: phase \(phase)") } } }
    /// Changes only when someone joins, leaves or is renamed (never per caret
    /// move), so the status bar does not re-render per remote keystroke.
    private(set) var participants: [CollabSession.Participant] = []
    private(set) var invite: CollabInvite?
    var sheetShown = false
    var joinSheetShown = false
    private(set) var approvals: [Approval] = []
    private(set) var note: String? { didSet { if let note, note != oldValue { FlashTeXLog.write("liveshare: " + note) } } }
    private(set) var canEdit = true
    private(set) var projectName = ""
    /// Bumped when the editor's link may have changed (a session started,
    /// ended, or its project opened); the editor pane reads it.
    private(set) var generation = 0

    @ObservationIgnored private(set) var session: CollabSession?
    @ObservationIgnored private(set) var hub: CollabHub?
    @ObservationIgnored private(set) var guest: CollabGuest?
    @ObservationIgnored private var links: [FileID: LiveShareFileLink] = [:]
    @ObservationIgnored private var replies: [UUID: (CollabHub.Decision) -> Void] = [:]
    /// Where the session's files live: the hub's project folder, or the
    /// guest's copy.
    @ObservationIgnored private(set) var root: URL?
    @ObservationIgnored private var opened = false
    @ObservationIgnored private var pendingWrites: Set<FileID> = []
    @ObservationIgnored private var reconcileTimer: Timer?
    @ObservationIgnored private var guestDisposition: ShellModel.DirtyDisposition = .none

    init(model: ShellModel?) { self.model = model }

    /// Observed (the File menu and the status bar read them); they change
    /// only when a session starts or ends.
    private(set) var isActive = false
    private(set) var isHost = false
    var hubMembers: [CollabHub.MemberInfo] { hub?.memberList ?? [] }

    // MARK: Editor links

    /// The session file the editor shows at `path`, or nil (no session, or
    /// not a shared file).
    func link(for path: String) -> LiveShareFileLink? {
        guard let session, opened, let file = session.file(atPath: path) else { return nil }
        if let l = links[file] { return l }
        guard let b = session.binding(for: file) else { return nil }
        let l = LiveShareFileLink(binding: b, session: session)
        links[file] = l
        return l
    }

    // MARK: Hosting

    /// File ▸ Start Live Share Session…: share the open project and show the
    /// invitation. Opens the sheet again while already hosting.
    func startHosting() {
        guard let model else { return }
        if isActive { sheetShown = true; return }
        guard let entryURL = model.documentURL else {
            note = "Save the document first: Live Share shares a project folder."
            sheetShown = true
            return
        }
        let root = entryURL.deletingLastPathComponent().resolvingSymlinksInPath()
        let session = CollabSession(role: .hub, replica: UInt64.random(in: 1...UInt64.max), name: Self.defaultDisplayName, colourIndex: 0)
        var shared = 0
        do {
            for (path, text) in Self.sharedSources(root: root, open: model.documents) {
                try session.shareFile(path: path, text: text)
                shared += 1
            }
        } catch {
            note = "Could not share the project: \(error)"
            sheetShown = true
            return
        }
        session.flush()
        let pins = CollabControl.SessionPins(main: model.project.entryPath, sourceDateEpoch: Int64(Date().timeIntervalSince1970),
                                             randomSeed: Int64.random(in: 0...Int64(Int32.max)), shellEscape: "off",
                                             externalTools: false, readConfinement: true)
        let hub: CollabHub
        do {
            hub = CollabHub(session: session, identity: try CollabIdentity(), projectName: root.lastPathComponent, pins: pins,
                            environmentDigest: Self.environmentDigest)
        } catch {
            note = "Could not create the session key: \(error)"
            sheetShown = true
            return
        }
        hub.approve = { [weak self] req, reply in self?.ask(req, reply) }
        hub.onEvent = { [weak self] e in self?.hubEvent(e) }
        do { try hub.start(loopbackOnly: Self.testLoopbackOnly, advertise: !Self.testLoopbackOnly) } catch {
            note = "Could not listen for collaborators: \(error)"
            sheetShown = true
            return
        }
        self.hub = hub
        self.root = root
        projectName = root.lastPathComponent
        attach(session)
        opened = true
        isHost = true
        phase = .hosting
        note = "Sharing \(shared) source file\(shared == 1 ? "" : "s") from \(root.lastPathComponent)."
        sheetShown = true
        model.liveShareRebind()
    }

    /// A fresh single-use invitation (each joiner needs their own).
    func newInvite() {
        guard let hub, hub.port != nil else { return }
        invite = hub.makeInvite(addresses: Self.localAddresses(), hostName: CollabGuest.localHostName)
    }

    private func hubEvent(_ e: CollabHub.Event) {
        switch e {
        case .ready:
            if invite == nil { newInvite() }
            if let invite, let model { LiveShareAutomation.hostReady(invite, model: model) }
        case let .failed(why):
            note = "The session listener failed: \(why)"
        case let .joined(_, name):
            note = "\(name) joined."
            if let app = NSApp {
                NSAccessibility.post(element: app, notification: .announcementRequested,
                                     userInfo: [.announcement: "\(name) joined Live Share", .priority: NSAccessibilityPriorityLevel.medium.rawValue])
            }
        case let .left(id, _):
            let name = hub?.memberList.first { $0.id == id }?.name ?? "A collaborator"
            note = "\(name) disconnected."
        case .reconnected, .refused, .forgedDropped:
            break
        }
        refreshParticipants()
    }

    private func ask(_ req: CollabHub.JoinRequest, _ reply: @escaping (CollabHub.Decision) -> Void) {
        if LiveShareAutomation.autoApprove { // debug builds only (LiveShareAutomation.swift)
            reply(.allowEdit)
            newInvite()
            return
        }
        let a = Approval(name: req.displayName, deviceKind: req.deviceKind)
        replies[a.id] = reply
        approvals.append(a)
        sheetShown = true
        NSApp?.requestUserAttention(.informationalRequest)
    }

    func answer(_ a: Approval, _ d: CollabHub.Decision) {
        approvals.removeAll { $0.id == a.id }
        replies.removeValue(forKey: a.id)?(d)
        if d != .deny { newInvite() } // the one just used is spent
    }

    func remove(_ participant: UInt64) {
        hub?.remove(participant)
        refreshParticipants()
    }

    // MARK: Joining

    /// File ▸ Join Live Share Session…
    func presentJoin() {
        if isActive { sheetShown = true; return }
        note = nil
        if case .ended = phase { phase = .idle }
        joinSheetShown = true
    }

    /// Joins with a pasted invitation. Unsaved work in the current project
    /// is saved, discarded or kept per the user's answer before the session
    /// opens its own copy.
    func join(link: String, name: String, disposition: ShellModel.DirtyDisposition? = nil) {
        let invite: CollabInvite
        do { invite = try CollabInvite(link: link) } catch {
            note = "\(error)"
            return
        }
        if let disposition {
            guestDisposition = disposition
        } else if let model, model.hasUnsavedDocuments {
            let alert = NSAlert()
            alert.messageText = "Save changes to \(model.unsavedDocumentsDescription) before joining?"
            alert.informativeText = "Joining opens the shared project in this window."
            alert.addButton(withTitle: "Save")
            alert.addButton(withTitle: "Discard")
            alert.addButton(withTitle: "Cancel")
            switch alert.runModal() {
            case .alertFirstButtonReturn: guestDisposition = .saveFirst
            case .alertSecondButtonReturn: guestDisposition = .discard
            default: return
            }
        } else {
            guestDisposition = .none
        }
        let g = CollabGuest(invite: invite, displayName: name.isEmpty ? Self.defaultDisplayName : String(name.prefix(64)))
        g.onStateChange = { [weak self] s in self?.guestState(s) }
        g.onJoined = { [weak self] session, ack in self?.joined(session, ack, invite: invite) }
        guest = g
        projectName = invite.projectName
        phase = .connecting
        note = nil
        g.connect()
    }

    private func guestState(_ s: CollabGuest.State) {
        switch s {
        case .connecting: phase = .connecting
        case .awaitingApproval: phase = .awaitingApproval
        case .joined:
            phase = .joined
            joinSheetShown = false
        case let .reconnecting(n):
            phase = .reconnecting(attempt: n)
            note = "Offline. Your edits will merge when the host is back."
        case let .ended(why):
            phase = .ended(why)
            note = why
            teardown(keepPhase: true)
        }
    }

    private func joined(_ session: CollabSession, _ ack: CollabControl.JoinAck, invite: CollabInvite) {
        let dir = Self.sessionDirectory(invite.sessionID)
        root = dir
        canEdit = session.canEdit
        attach(session)
        // The project opens once the main file's text has arrived (the sync
        // that follows join_ack).
        mainPath = ack.pins.main
        openGuestProjectIfReady()
    }

    @ObservationIgnored private var mainPath: String?

    private func openGuestProjectIfReady() {
        guard !opened, let session, let root, let main = mainPath, let file = session.file(atPath: main),
              session.text(of: file) != nil, let model else {
            FlashTeXLog.write("liveshare: guest project not ready (opened \(opened), main \(mainPath ?? "nil"), files \(session?.textFiles.map(\.path) ?? []))")
            return
        }
        FlashTeXLog.write("liveshare: opening the shared copy in \(root.path)")
        do {
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
            for (f, path) in session.textFiles { try write(f, path: path, session: session, root: root) }
        } catch {
            note = "Could not write the shared files: \(error.localizedDescription)"
            return
        }
        let outcome = model.openTex(at: root.appendingPathComponent(main), dirty: guestDisposition)
        guard outcome == .opened else {
            note = "Could not open the shared project (\(outcome)); its files are in \(root.path)."
            return
        }
        opened = true
        model.liveShareRebind()
        LiveShareAutomation.guestOpened(model: model)
    }

    // MARK: Leaving

    /// File ▸ Leave (or End) Live Share Session.
    func leave() {
        if let hub {
            hub.stop()
            note = "Session ended."
        } else if let guest {
            guest.leave()
            note = "You left the session. The shared files stay in \(root?.path ?? "the session folder")."
        }
        teardown(keepPhase: false)
    }

    private func teardown(keepPhase: Bool) {
        for l in links.values { l.binding.attach(nil) }
        links = [:]
        session?.stopTimers()
        session = nil
        isActive = false
        isHost = false
        hub = nil
        guest = nil
        opened = false
        invite = nil
        approvals = []
        for r in replies.values { r(.deny) }
        replies = [:]
        participants = []
        reconcileTimer?.invalidate()
        reconcileTimer = nil
        if !keepPhase { phase = .idle }
        canEdit = true
        model?.liveShareRebind()
    }

    // MARK: Session wiring

    private func attach(_ session: CollabSession) {
        self.session = session
        isActive = true
        session.onRemoteText = { [weak self] f in self?.remoteText(f) }
        session.onFilesChanged = { [weak self] in
            guard let self else { return }
            if !self.opened { self.openGuestProjectIfReady() } else { self.materialiseNewFiles() }
        }
        session.onPresenceChanged = { [weak self] in
            guard let self else { return }
            self.refreshParticipants()
            NotificationCenter.default.post(name: .liveSharePresenceChanged, object: self.session)
        }
        refreshParticipants()
        let t = Timer(timeInterval: 1, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.reconcile() } }
        RunLoop.main.add(t, forMode: .common)
        reconcileTimer = t
    }

    private func refreshParticipants() {
        guard let session else { if !participants.isEmpty { participants = [] }; return }
        var list = session.participants
        if let hub {
            // The hub knows who is connected even before their first presence.
            for m in hub.memberList where m.connected && !list.contains(where: { $0.id == m.id }) {
                list.append(.init(id: m.id, name: m.name, colourIndex: m.colourIndex, isLocal: false))
            }
        }
        if list != participants { participants = list }
    }

    /// Remote text for a file no editor shows: the open buffer follows (and
    /// autosaves), or the file on disk does.
    private func remoteText(_ file: FileID) {
        // Before the guest's copy is open, the window still shows its old
        // project: nothing of it may be touched (the copy is written whole
        // when it opens).
        guard opened, let model, let session, let path = session.path(of: file), let text = session.text(of: file) else { return }
        if let i = model.documents.firstIndex(where: { $0.path == path }) {
            guard model.documents[i].text != text else { return }
            model.documents[i].text = text
            model.scheduleAutosave()
            scheduleRemoteCompile()
        } else {
            scheduleWrite(file)
        }
    }

    private func scheduleWrite(_ file: FileID) {
        guard pendingWrites.insert(file).inserted else { return }
        // Once per run-loop turn: a burst of remote keystrokes is one write,
        // and it lands before any user action could open the file.
        DispatchQueue.main.async { [weak self] in
            MainActor.assumeIsolated {
                guard let self, self.pendingWrites.remove(file) != nil, let session = self.session, let root = self.root,
                      let path = session.path(of: file) else { return }
                try? self.write(file, path: path, session: session, root: root)
                self.scheduleRemoteCompile()
            }
        }
    }

    private func write(_ file: FileID, path: String, session: CollabSession, root: URL) throws {
        guard FileMap.isValidPath(path), let text = session.text(of: file) else { return }
        // `isValidPath` (byte-wise: no empty, `.` or `..` segment, no `\`, no
        // NUL) keeps it inside the root. Not `standardizedFileURL`: that drops
        // `/private` only from paths that already exist, so a new file under
        // /private/tmp would never compare equal to its existing folder.
        let url = root.appendingPathComponent(path)
        guard url.path.hasPrefix(root.path + "/") else { return }
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        if let existing = try? String(contentsOf: url, encoding: .utf8), existing == text { return }
        try Data(text.utf8).write(to: url, options: .atomic)
    }

    private func materialiseNewFiles() {
        guard let session, let root else { return }
        for (f, path) in session.textFiles where !FileManager.default.fileExists(atPath: root.appendingPathComponent(path).path) {
            try? write(f, path: path, session: session, root: root)
        }
    }

    /// Local edits made outside the editor (Find in Project, a rename across
    /// files) to open buffers no editor shows: their difference from the
    /// CRDT is local, since remote text reaches those buffers at once.
    private func reconcile() {
        guard let model, let session, opened, canEdit else { return }
        for doc in model.documents {
            guard let file = session.file(atPath: doc.path), let b = session.binding(for: file), b.host == nil,
                  let text = session.text(of: file), text != doc.text else { continue }
            b.adopt(text: doc.text)
        }
    }

    func bumpGeneration() { generation &+= 1 }

    @ObservationIgnored private var compileScheduled = false

    /// Remote edits outside the editor recompile at most every 150 ms
    /// (proposal §2.5): five typists must not supersede every compile.
    private func scheduleRemoteCompile() {
        guard !compileScheduled else { return }
        compileScheduled = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.compileScheduled = false
                if self.isActive { self.model?.implicitFilesChanged() }
            }
        }
    }

    // MARK: Helpers

    static var environmentDigest: String {
        let v = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev"
        return "FlashTeX \(v)"
    }

    /// Application Support/FlashTeX/Collab/<instance>/<session>
    /// (`FLASHTEX_COLLAB_DIR` replaces Application Support/FlashTeX/Collab).
    static func sessionDirectory(_ sessionID: String) -> URL {
        let env = ProcessInfo.processInfo.environment["FLASHTEX_COLLAB_DIR"].flatMap { $0.isEmpty ? nil : URL(fileURLWithPath: $0, isDirectory: true) }
        let base = sessionBaseOverride ?? env ?? (FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support"))
            .appendingPathComponent("FlashTeX/Collab", isDirectory: true)
        let safe = sessionID.filter { $0.isHexDigit }
        return base.appendingPathComponent("\(instance ?? "default")/\(safe.isEmpty ? "session" : safe)", isDirectory: true)
    }

    static let sharedExtensions: Set<String> = ["tex", "sty", "cls", "bib", "bst", "toml", "txt", "ltx", "dtx", "ins", "cfg", "def", "clo", "bbx", "cbx", "lbx"]
    static let maxSharedFiles = 200
    static let maxSharedBytes = 32 * 1024 * 1024

    /// The project's text sources: open buffers as they are, the rest from
    /// disk; hidden folders, `.flashtex/` and build outputs are skipped, and
    /// the whole set is bounded (proposal §2.6, "never synced").
    static func sharedSources(root: URL, open: [RuntimeV1.Document]) -> [(String, String)] {
        var out: [(String, String)] = []
        var seen = Set<String>()
        var bytes = 0
        for d in open where FileMap.isValidPath(d.path) && seen.insert(d.path).inserted {
            out.append((d.path, d.text))
            bytes += d.text.utf8.count
        }
        let fm = FileManager.default
        guard let e = fm.enumerator(at: root, includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey],
                                    options: [.skipsHiddenFiles, .skipsPackageDescendants]) else { return out }
        let rootPath = root.resolvingSymlinksInPath().path
        for case let url as URL in e {
            let full = url.resolvingSymlinksInPath().path
            guard full.hasPrefix(rootPath + "/") else { continue }
            let rel = String(full.dropFirst(rootPath.count + 1))
            if ["build", "out", "output", "node_modules"].contains(rel.split(separator: "/").first.map(String.init) ?? "") {
                e.skipDescendants(); continue
            }
            guard sharedExtensions.contains(url.pathExtension.lowercased()), !seen.contains(rel), FileMap.isValidPath(rel),
                  out.count < maxSharedFiles,
                  let values = try? url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey]), values.isRegularFile == true,
                  (values.fileSize ?? 0) <= CollabLimits.maxDocumentBytes, bytes + (values.fileSize ?? 0) <= maxSharedBytes,
                  let text = try? String(contentsOf: url, encoding: .utf8) else { continue }
            seen.insert(rel)
            out.append((rel, text))
            bytes += text.utf8.count
        }
        return out
    }

    /// This Mac's IPv4 addresses on up, non-loopback interfaces (for the invite).
    static func localAddresses() -> [String] {
        var out: [String] = []
        var ifap: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifap) == 0, let first = ifap else { return [] }
        defer { freeifaddrs(ifap) }
        var p: UnsafeMutablePointer<ifaddrs>? = first
        while let i = p {
            defer { p = i.pointee.ifa_next }
            let flags = Int32(i.pointee.ifa_flags)
            guard flags & IFF_UP != 0, flags & IFF_LOOPBACK == 0, let addr = i.pointee.ifa_addr,
                  addr.pointee.sa_family == UInt8(AF_INET) else { continue }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            if getnameinfo(addr, socklen_t(addr.pointee.sa_len), &host, socklen_t(host.count), nil, 0, NI_NUMERICHOST) == 0 {
                let s = String(cString: host)
                if !out.contains(s) { out.append(s) }
            }
        }
        return Array(out.prefix(4))
    }
}

extension Notification.Name {
    /// Someone's caret or selection moved (object: the `CollabSession`).
    static let liveSharePresenceChanged = Notification.Name("FlashTeXLiveSharePresenceChanged")
}

extension ShellModel {
    private static var liveShareKey = 0
    /// Live Share state (LiveShare.swift), attached like `files`.
    var liveShare: LiveShareController {
        if let existing = objc_getAssociatedObject(self, &Self.liveShareKey) as? LiveShareController { return existing }
        let c = LiveShareController(model: self)
        objc_setAssociatedObject(self, &Self.liveShareKey, c, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return c
    }

    /// The editor's session link changed (a session started, ended or its
    /// project opened): nudge the editor pane to re-read it.
    func liveShareRebind() { liveShare.bumpGeneration() }
}
