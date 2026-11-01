import AppKit
import SwiftUI
import FlashTeXProtocol

// Multi-window and multi-project (DESIGN.md §10, §12 P6, decision 7).
//
// Every project window owns its own `ShellModel`, and through it its own
// `EngineV3Session` and `flashtex-host` process: one host per open project.
// Nothing about a project is shared between windows, so a compile, a stall or
// a crash in one window never touches another (the host is the unit of
// failure, as it is the licence boundary). What the windows share is
// app-wide: preferences, the decoded-resource cache and the memory-pressure
// handler (EngineV3MemoryPressure trims every session).
//
// `ProjectWindows` is the app's list of those windows. It decides where a
// file opens (`open`), so the same document is never open twice, which
// window the menus act on when no project window is key (`frontmost`), and
// what the app reopens at its next launch (`restorable`).

/// One project window's identity: the value of the app's `WindowGroup`.
struct ProjectWindowID: Hashable, Codable, Sendable {
    var id = UUID()
}

extension FocusedValues {
    /// The key project window's model (`focusedSceneValue` in
    /// `ProjectWindowRoot`): what the menu bar's commands act on.
    struct ShellModelKey: FocusedValueKey { typealias Value = ShellModel }
    var shellModel: ShellModel? {
        get { self[ShellModelKey.self] }
        set { self[ShellModelKey.self] = newValue }
    }
}

@MainActor
@Observable
final class ProjectWindows {
    /// The app's windows (the App, AppDelegate and the menu commands);
    /// tests make their own.
    static let shared = ProjectWindows(defaults: EngineV3.underTest ? nil : .standard)

    final class Entry {
        let id: ProjectWindowID
        let model: ShellModel
        weak var window: NSWindow?
        /// When it last became key (a counter: the most recent is largest).
        var lastKey = 0
        /// The frame to give the window when it appears (a restored window).
        var pendingFrame: NSRect?
        init(id: ProjectWindowID, model: ShellModel) { self.id = id; self.model = model }
    }

    /// The open windows, in the order they were made.
    @ObservationIgnored private(set) var entries: [Entry] = []
    /// The window that was key last: the menus act on it while a window of
    /// another kind is key (Settings, Nearby Companion). Observed, so the
    /// menu bar follows it; it changes only when the key window does.
    private(set) var frontmostID: ProjectWindowID?
    @ObservationIgnored private var keyCounter = 0
    /// How a new model is made: `launchHooks` only for the app's first
    /// window, which alone seeds from `FLASHTEX_SEED_FILE` and attaches the
    /// capture bridge (one bridge for the app, not one per window).
    @ObservationIgnored var makeModel: (_ launchHooks: Bool) -> ShellModel = { ShellModel(startup: .untitledDocument, launchHooks: $0) }
    @ObservationIgnored private var madeFirst = false
    /// Opens a window for an id this list already holds (SwiftUI's
    /// `openWindow(value:)`, set by the first window that appears). nil
    /// until then, and in tests that do not show windows.
    @ObservationIgnored var showWindow: ((ProjectWindowID) -> Void)?
    /// The app is quitting: windows closing now stay in the restore list.
    @ObservationIgnored private(set) var terminating = false
    /// What `restorable` was when quitting began (the windows' closes after it change nothing).
    @ObservationIgnored private var restorableAtQuit: [Restorable]?
    /// Where the restore list is kept (`restoreKey`); nil keeps none (tests).
    @ObservationIgnored let defaults: UserDefaults?

    init(defaults: UserDefaults? = nil) { self.defaults = defaults }

    /// The companion's capture sink for the app (FrontWindowCaptures).
    @ObservationIgnored private(set) lazy var captures = FrontWindowCaptures(windows: self)

    // MARK: windows and models

    /// The model of window `id`, made now if this list has none (the app's
    /// first window, or a window SwiftUI made by itself).
    func model(for id: ProjectWindowID) -> ShellModel {
        if let e = entries.first(where: { $0.id == id }) { return e.model }
        let first = !madeFirst
        madeFirst = true
        let m = makeModel(first)
        adopt(m, id: id)
        return m
    }

    /// Adds `model` as window `id` (`model(for:)`, `open`).
    private func adopt(_ model: ShellModel, id: ProjectWindowID) {
        madeFirst = true
        model.windows = self
        entries.append(Entry(id: id, model: model))
        if frontmostID == nil { frontmostID = id }
    }

    /// The window `id`'s AppKit window appeared (`ProjectWindowAccessor`).
    func attach(_ window: NSWindow, to id: ProjectWindowID) {
        guard let e = entries.first(where: { $0.id == id }) else { return }
        e.window = window
        // The app restores its project windows itself (`restorable`):
        // AppKit's own restoration would bring back windows without their
        // projects, and only when the system setting keeps windows on quit.
        window.isRestorable = false
        if let frame = e.pendingFrame {
            e.pendingFrame = nil
            window.setFrame(frame, display: true) // AppKit keeps it on a screen
        }
        if window.isKeyWindow { becameKey(id) }
    }

    /// Window `id` became key.
    func becameKey(_ id: ProjectWindowID) {
        guard let e = entries.first(where: { $0.id == id }) else { return }
        keyCounter += 1
        e.lastKey = keyCounter
        if frontmostID != id { frontmostID = id }
    }

    /// Window `id` closed: its model (and so its engine session and host) is
    /// let go. While the app quits the list is kept as it was, for the next launch.
    func closed(_ id: ProjectWindowID) {
        guard let i = entries.firstIndex(where: { $0.id == id }) else { return }
        entries.remove(at: i)
        if frontmostID == id { frontmostID = byRecency.first?.id }
        saveRestorable()
    }

    /// The app is quitting (every window agreed): what is open now is what
    /// the next launch reopens, whatever order the windows close in.
    func beginTerminating() {
        guard !terminating else { return }
        restorableAtQuit = restorable
        terminating = true
        saveRestorable()
    }

    /// File › New Window: an untitled document in a window of its own. The
    /// id to show (`openWindow(value:)`).
    func newWindow() -> ProjectWindowID {
        let id = ProjectWindowID()
        adopt(makeModel(false), id: id)
        return id
    }

    /// The windows, the most recently key first.
    var byRecency: [Entry] {
        // (windows never key, as at an automation launch: the newest first)
        entries.enumerated().sorted { ($0.element.lastKey, $0.offset) > ($1.element.lastKey, $1.offset) }.map(\.element)
    }

    /// Every open project window's model, the most recently key first.
    var models: [ShellModel] { byRecency.map(\.model) }

    /// The model the menus act on when no project window is key.
    var frontmost: ShellModel? {
        guard let id = frontmostID else { return nil }
        return entries.first { $0.id == id }?.model
    }

    func entry(for model: ShellModel) -> Entry? { entries.first { $0.model === model } }

    /// A model for the menus while no project window is open at all (the
    /// Nearby Companion window outlived them): it has no window, no engine
    /// and no bridge; File > Open from it opens a new window.
    @ObservationIgnored private(set) lazy var placeholder: ShellModel = {
        let m = ShellModel(startup: .untitledDocument, launchHooks: false)
        m.engineV3Enabled = false
        m.windows = self
        return m
    }()

    /// What the menus act on: the key project window's model, else the last
    /// key one's, else the placeholder.
    func commandTarget(focused: ShellModel?) -> ShellModel { focused ?? frontmost ?? placeholder }

    // MARK: opening

    /// Where `open` put the file.
    enum OpenResult: Equatable {
        /// Already open in a window, which came forward (showing `path`).
        case focused(ProjectWindowID, path: String)
        /// Opened in the requesting window, which held only an untouched untitled document.
        case openedHere
        /// Opened in a new window.
        case openedNew(ProjectWindowID)
        /// Not opened (the model's `captureNote` says why).
        case failed(ShellModel.OpenOutcome)
    }

    /// Opens `url` (a `.tex` file or a project folder) for the user:
    /// - a document already open in a window, as its entry or as a member
    ///   of its project, brings that window forward on that document: it is
    ///   never opened twice, since two copies would be two engines and two
    ///   unsaved buffers of one file racing each other to the disk;
    /// - else, into `requester`'s window when that window holds nothing yet
    ///   (the untouched untitled document), so the app's first Open does not
    ///   leave an empty window behind;
    /// - else into a new window, leaving the requester's project as it was.
    /// Returns where it went and the model now showing it.
    @discardableResult
    func open(_ url: URL, from requester: ShellModel?) -> (result: OpenResult, model: ShellModel?) {
        let file = resolve(url, with: requester)
        guard let file else { return (.failed(.readFailed), nil) }
        if let (e, path) = window(showing: file) {
            if e.model.activePath != path { e.model.project.switchDocument(to: path) }
            e.window?.makeKeyAndOrderFront(nil)
            becameKey(e.id)
            return (.focused(e.id, path: path), e.model)
        }
        if let requester, requester.holdsNothing, requester !== placeholder {
            let outcome = requester.openTex(at: file)
            if outcome == .opened { saveRestorable() }
            return (outcome == .opened ? .openedHere : .failed(outcome), requester)
        }
        let model = makeModel(false)
        model.windows = self
        let outcome = model.openTex(at: file)
        guard outcome == .opened else {
            requester?.captureNote = model.captureNote
            return (.failed(outcome), nil)
        }
        let id = ProjectWindowID()
        adopt(model, id: id)
        showWindow?(id)
        saveRestorable()
        return (.openedNew(id), model)
    }

    /// `url`, or for a folder the entry its manifest names (ProjectManifest.swift).
    private func resolve(_ url: URL, with requester: ShellModel?) -> URL? {
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory), isDirectory.boolValue else { return url }
        switch (requester ?? placeholder).manifest.resolveEntry(in: url) {
        case .success(let entry): return entry
        case .failure(let f):
            requester?.captureNote = "Could not open \(url.lastPathComponent): \(f.why)"
            return nil
        }
    }

    /// The window showing `file` and the document path it has it under:
    /// its entry, or a member of its project (a `\input` file it opened).
    func window(showing file: URL) -> (Entry, String)? {
        for e in entries {
            let m = e.model
            guard let entry = m.documentURL else { continue }
            if Self.sameFile(entry, file) { return (e, m.project.entryPath) }
            guard let root = m.project.projectRoot else { continue }
            for doc in m.documents where doc.path != m.project.entryPath {
                if Self.sameFile(root.appendingPathComponent(doc.path), file) { return (e, doc.path) }
            }
        }
        return nil
    }

    /// Whether two URLs name one file: the same path once symbolic links and
    /// `..` are resolved, or the same file system object (another spelling
    /// of the path on a case-insensitive volume, a hard link).
    nonisolated static func sameFile(_ a: URL, _ b: URL) -> Bool {
        let pa = a.standardizedFileURL.resolvingSymlinksInPath().path, pb = b.standardizedFileURL.resolvingSymlinksInPath().path
        if pa == pb { return true }
        guard let ia = try? URL(fileURLWithPath: pa).resourceValues(forKeys: [.fileResourceIdentifierKey]).fileResourceIdentifier,
              let ib = try? URL(fileURLWithPath: pb).resourceValues(forKeys: [.fileResourceIdentifierKey]).fileResourceIdentifier else { return false }
        return ia.isEqual(ib)
    }

    // MARK: restoration

    /// One window to reopen at the next launch: its entry file, the other
    /// documents it had open (rooted paths), the one it showed, and its frame.
    struct Restorable: Codable, Equatable {
        var path: String
        var members: [String]? = nil
        var activePath: String? = nil
        var frame: String? = nil
    }

    /// The windows to reopen, the most recently key last (so it ends up in
    /// front): those with a file. An untitled buffer has nothing to reopen;
    /// its unsaved text was offered for saving when the app quit. Kept at
    /// each open and close too, so a crash still reopens what was open.
    var restorable: [Restorable] {
        if let restorableAtQuit { return restorableAtQuit }
        return byRecency.reversed().compactMap { e in
            let m = e.model
            guard let url = m.documentURL else { return nil }
            let members = m.documents.map(\.path).filter { $0 != m.project.entryPath }
            return Restorable(path: url.path, members: members.isEmpty ? nil : members,
                              activePath: m.activePath == m.project.entryPath ? nil : m.activePath,
                              frame: e.window.map { NSStringFromRect($0.frame) })
        }
    }

    static let restoreKey = "FlashTeX.windows.open"

    /// Writes `restorable` to `defaults`.
    func saveRestorable() {
        guard let defaults, let data = try? JSONEncoder().encode(restorable) else { return }
        defaults.set(data, forKey: Self.restoreKey)
    }

    /// The list the last run left.
    func savedRestorable() -> [Restorable] {
        guard let data = defaults?.data(forKey: Self.restoreKey) else { return [] }
        return (try? JSONDecoder().decode([Restorable].self, from: data)) ?? []
    }

    /// Whether this launch reopens the last run's windows: not when it was
    /// told what to open (`FLASHTEX_OPEN`, a seed file, a bench) or not to
    /// (`FLASHTEX_RESTORE_WINDOWS=0`), so automation sees the same app every run.
    static func restoresWindows(_ env: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        if let v = env["FLASHTEX_RESTORE_WINDOWS"] { return v != "0" }
        return ["FLASHTEX_OPEN", "FLASHTEX_SEED_FILE", "FLASHTEX_V3_BENCH", "FLASHTEX_V3_CAPTURE_OUT", "FLASHTEX_LIVE_SHARE_HOST", "FLASHTEX_LIVE_SHARE_JOIN_FILE"]
            .allSatisfy { (env[$0] ?? "").isEmpty }
    }

    /// Reopens `list` at launch: the first into `first` (the launch's
    /// untitled window, which holds nothing), each other in a window of its
    /// own, in order, so the window that was in front comes up in front.
    /// Each gets its frame and the document it showed. A file that is gone
    /// is skipped (and said in the log); nothing is asked.
    func restore(_ list: [Restorable], first: ShellModel) {
        var into: ShellModel? = first
        for r in list {
            let url = URL(fileURLWithPath: r.path)
            guard FileManager.default.fileExists(atPath: url.path) else {
                FlashTeXLog.write("windows: not reopening \(r.path): no such file")
                continue
            }
            let (result, model) = open(url, from: into)
            guard let model, result != .failed(.readFailed) else { continue }
            if case .failed = result { continue }
            into = nil
            // Its other documents, read from disk like the sidebar opens them
            // (one that is gone is left out), then the one it showed.
            if r.members?.isEmpty == false || r.activePath != nil {
                let members = r.members ?? [], active = r.activePath
                Task { @MainActor [weak model] in
                    for p in members { guard let model else { return }; _ = await model.project.openDocument(p) }
                    if let model, let active, active != model.activePath, model.project.isOpen(active) { model.project.switchDocument(to: active) }
                }
            }
            if let frame = r.frame.map(NSRectFromString), frame.width > 0, frame.height > 0, let e = entry(for: model) {
                if let w = e.window { w.setFrame(frame, display: true) } else { e.pendingFrame = frame }
            }
        }
        saveRestorable()
    }
}

extension ShellModel {
    /// The window holds nothing worth keeping: the untouched untitled
    /// document (no file, no edits), or the contract fixture. Opening a file
    /// replaces it rather than leaving an empty window behind.
    var holdsNothing: Bool { documentURL == nil && !hasUnsavedDocuments && !liveShareActive }
}

/// The root of a project window: its model (from `ProjectWindows`), handed
/// to the views below and to the menu bar (`focusedSceneValue`).
struct ProjectWindowRoot: View {
    let id: ProjectWindowID
    let windows: ProjectWindows
    let model: ShellModel
    @Environment(\.openWindow) private var openWindow

    init(id: ProjectWindowID, windows: ProjectWindows) {
        self.id = id
        self.windows = windows
        model = windows.model(for: id)
    }

    var body: some View {
        ContentView()
            .environment(model)
            .focusedSceneValue(\.shellModel, model)
            .navigationTitle(model.windowTitle) // the Window menu, Mission Control and ⌘` name the project (the title bar hides it)
            .background(ProjectWindowAccessor(id: id, windows: windows))
            .onAppear {
                if windows.showWindow == nil { windows.showWindow = { [openWindow] in openWindow(value: $0) } }
            }
    }
}

extension ShellModel {
    /// The window's title: the entry file's name, or "Untitled".
    var windowTitle: String { documentURL?.lastPathComponent ?? UntitledDocument.title }
}

/// Tells `ProjectWindows` about the window's AppKit side: the `NSWindow`
/// (to bring it forward), when it becomes key, and when it closes.
struct ProjectWindowAccessor: NSViewRepresentable {
    let id: ProjectWindowID
    let windows: ProjectWindows

    func makeNSView(context: Context) -> AccessorView { AccessorView(id: id, windows: windows) }
    func updateNSView(_ view: AccessorView, context: Context) {}

    final class AccessorView: NSView {
        let id: ProjectWindowID
        weak var windows: ProjectWindows?
        private var observers: [NSObjectProtocol] = []

        init(id: ProjectWindowID, windows: ProjectWindows) {
            self.id = id
            self.windows = windows
            super.init(frame: .zero)
        }

        required init?(coder: NSCoder) { fatalError("not coded") }

        deinit { for o in observers { NotificationCenter.default.removeObserver(o) } }

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            for o in observers { NotificationCenter.default.removeObserver(o) }
            observers = []
            guard let window else { return }
            windows?.attach(window, to: id)
            let id = id
            // Weak, as WindowChrome's observers: the window must not keep itself alive.
            observers = [
                NotificationCenter.default.addObserver(forName: NSWindow.didBecomeKeyNotification, object: window, queue: .main) { [weak self] _ in
                    MainActor.assumeIsolated { self?.windows?.becameKey(id) }
                },
                NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification, object: window, queue: .main) { [weak self] _ in
                    MainActor.assumeIsolated { self?.windows?.closed(id) }
                },
            ]
        }
    }
}

/// Where the Nearby companion's captures go (NearbyState.attach): the
/// frontmost project window, the one you used last. A capture stays with the
/// window that received it: its later `capture_status` and `capture_insert`
/// go to that window even after another comes forward (the proposal lives in
/// that window's inbox). Attached once for the app, so switching windows
/// never restarts the listener or drops a companion's connection.
final class FrontWindowCaptures: CaptureSink, DestinationProvider, @unchecked Sendable {
    @MainActor private weak var windows: ProjectWindows?
    /// The window each capture went to, by `capture_id` (main thread only).
    @MainActor private var owners: [String: WeakModel] = [:]
    struct WeakModel { weak var model: ShellModel? }

    @MainActor init(windows: ProjectWindows) { self.windows = windows }

    @MainActor func target(_ captureId: String?) -> ShellModel? {
        if let captureId, let m = owners[captureId]?.model { return m }
        return windows?.frontmost
    }

    func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void) {
        Task { @MainActor in
            guard let m = self.target(nil) else { return reply(Self.noWindow(envelope.id)) }
            self.owners = self.owners.filter { $0.value.model != nil }
            self.owners[envelope.payload.captureId] = WeakModel(model: m)
            m.submit(envelope, reply: reply)
        }
    }

    func captureStatus(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureStatusRequest>, reply: @escaping (Data) -> Void) {
        Task { @MainActor in
            guard let m = self.target(envelope.payload.captureId) else { return reply(Self.noWindow(envelope.id)) }
            m.captureStatus(envelope, reply: reply)
        }
    }

    func captureInsert(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureInsertRequest>, reply: @escaping (Data) -> Void) {
        Task { @MainActor in
            guard let m = self.target(envelope.payload.captureId) else { return reply(Self.noWindow(envelope.id)) }
            m.captureInsert(envelope, reply: reply)
        }
    }

    func currentDestination(_ reply: @escaping (NearbyV1.Destination?) -> Void) {
        Task { @MainActor in
            guard let m = self.target(nil) else { return reply(nil) }
            m.currentDestination(reply)
        }
    }

    private static func noWindow(_ id: String?) -> Data {
        NearbyV1.errorLine(id: id, code: "unavailable", message: "no project window is open on the Mac")
    }
}
