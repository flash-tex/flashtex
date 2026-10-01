import AppKit
import CryptoKit
import Foundation
import Observation
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import FlashTeXProtocol

/// The engine-v3 preview for the window's project (flag-gated, EngineV3Host.swift):
/// one `flashtex-host` per session, one incremental connection, the editor's
/// changes sent as byte `edits` on every keystroke (the host's preemption is
/// the only debounce: a newer COMPILE supersedes the running one), pages
/// replaced in place as they arrive (the edited page first), `PAGES` stale
/// ranges shown as stale.
///
/// **One host per window (ShellModel), for its project.** The host keeps one
/// resident engine for one job (root, main file, …); the compile unit is
/// the project's entry file, so every tab of a project shares its job, and
/// a second project window has its own ShellModel, so its own session and
/// host. Document-scoped sessions inside one host would serialise every
/// window's compiles on the host's single engine thread and make a COMPILE
/// for one project evict another's checkpoints, so they would cost latency
/// for nothing the process boundary does not already give.
///
/// **Lifecycle.** The host runs with `--once`: it serves this session's
/// connection and exits when the socket closes, so it dies with the app
/// (any death: the kernel closes the socket). A host that was never
/// connected (the app died while it prepared the format) is killed at the
/// next launch from its pid file. A host that dies or drops the connection
/// is restarted (at most 3 times a minute); the new host's first compile
/// starts from the S₀ cache (`--s0-cache`), and the pages on screen stay,
/// marked stale, until the new ones arrive.
///
/// **Threading.** The socket's reader thread decodes every frame, binds
/// resources, prepares pages and, for a page on screen, rasterises it; the
/// main thread only installs bitmaps and updates this model. Nothing draws
/// on the main thread.
@MainActor
@Observable
final class EngineV3Session {
    enum Phase: Equatable {
        case idle
        /// Host starting: preparing the format (the first use builds it from
        /// the user's TeX Live), warming up.
        case starting(since: Date)
        case ready
        case failed(String)
    }

    private(set) var phase: Phase = .idle
    /// What the host prepared: which TeX Live, the format's status.
    private(set) var environmentNote = ""
    /// Last compile outcome, for the pane's status line.
    private(set) var statusNote = ""
    private(set) var pageCount = 0
    private(set) var errorCount = 0
    private(set) var warningCount = 0
    /// Bumped whenever the page list or a page size changes (the view re-lays out).
    private(set) var layoutRevision = 0
    /// Pages on screen left from an earlier compile (the status bar's "N stale").
    private(set) var staleCount = 0
    /// A COMPILE is out and its DONE has not come back.
    private(set) var compiling = false
    @ObservationIgnored private var lastSentID = 0
    /// DIAGNOSTIC messages of the compile in progress (published at its DONE).
    @ObservationIgnored private var diagnostics: [DL3JSON] = []
    /// diag-v1 DIAGs of the compile in progress (the host sends these
    /// instead of DIAGNOSTICs when it offers diag-v1).
    @ObservationIgnored private var diags: [DL3Diag] = []
    /// The connected host offers diag-v1 (and so sends DIAGs, not DIAGNOSTICs).
    @ObservationIgnored private(set) var hostOffersDiagV1 = false
    /// The first TeX error of the last compile ("file:line: message"), shown in the pane.
    private(set) var firstError: String?
    /// Project trust (EngineV3Trust.swift): false → shell escape off and the
    /// pane asks "Trust this project?".
    private(set) var projectTrusted = true
    /// What the trust prompt was computed for: the project, its main file,
    /// the quarantined files its walk found, and the identities trusting it
    /// records (EngineV3Trust.subjects).
    struct TrustPrompt { var root: URL; var main: URL; var others: [URL]; var need: [EngineV3Trust.Identity] }
    @ObservationIgnored private(set) var trustPrompt: TrustPrompt?
    /// Downloaded files besides the folder and the main file that the prompt counts.
    private(set) var trustOtherCount = 0
    /// A new project's trust is decided after its first walk (the copy's
    /// sync); until then its COMPILEs send shell escape off.
    @ObservationIgnored private var trustPending = false
    /// The model's project generation the session compiles (ShellModel.replaceProject bumps it).
    @ObservationIgnored private var generation = -1
    /// The main file of the last COMPILE (relative to the project).
    private(set) var mainFile = ""

    @ObservationIgnored private(set) var pages: [Int: DL3PreparedPage] = [:]
    @ObservationIgnored private(set) var stale: Set<Int> = []
    @ObservationIgnored private(set) var forms: [UInt32: DL3PreparedPage] = [:]
    /// Pages whose PDF rendering replaces the display list (INCOMPLETE, or a
    /// resource that did not resolve), rendered from `DONE.pdf`.
    @ObservationIgnored private(set) var pdfFallback: [Int: CGPDFPage] = [:]
    /// The project's last rendered pages from disk, shown until the compile
    /// replaces them (EngineV3Snapshot.swift).
    @ObservationIgnored var snapshot: (EngineV3Snapshot, URL)?
    @ObservationIgnored private var openKey: String?
    @ObservationIgnored private var lastOpenHandled: UInt64?
    /// Open → pixels (instant reopen, owner decision 8A): when the project
    /// opened, when its first page was on screen (from the snapshot or the
    /// compile), when its first current page was.
    @ObservationIgnored var openStartNs: UInt64?
    private(set) var openFirstPixelsNs: UInt64?
    private(set) var openFirstCurrentNs: UInt64?
    @ObservationIgnored private var snapshotSave: DispatchWorkItem?
    /// The project's input files when the copy was last synced (what the
    /// compiles since have read): a snapshot is saved only while they are unchanged.
    @ObservationIgnored private var inputsAtSync: [String: String]?
    static let snapshotQueue = DispatchQueue(label: "flashtex.engine-v3.snapshot", qos: .utility)
    /// SOURCES of the connection: span id → (file, line) (EngineV3SourceMap.swift).
    @ObservationIgnored var sourceMap = DL3SourceMap()
    /// Per-page glyph indexes for forward/reverse search, built on first use.
    @ObservationIgnored var sourceIndexes: [Int: DL3SourceIndex] = [:]
    @ObservationIgnored weak var view: EngineV3PagesView? { didSet { view?.rasterPlan = rasterPlan } }
    @ObservationIgnored weak var model: ShellModel?

    @ObservationIgnored private var host: EngineV3HostProcess?
    /// The running host's pid (tests, diagnostics).
    var hostPID: Int32? { host?.pid }
    /// Hosts started by this session (the first, then each restart).
    @ObservationIgnored private(set) var hostStarts = 0
    @ObservationIgnored private var connection: DL3Connection?
    @ObservationIgnored private var nextID = 1
    /// The text of each document as the host's copy of the project holds it.
    @ObservationIgnored private var sentTexts: [String: String] = [:]
    /// UTF-8 length of what the host holds per document (fast path).
    @ObservationIgnored private var hostBytes: [String: Int] = [:]
    /// Documents the fast path edited since the model last stored their text.
    @ObservationIgnored private var fastPending: Set<String> = []
    @ObservationIgnored private var project: EngineV3Mirror?
    /// The page the view shows (sent as `viewport`).
    @ObservationIgnored var visiblePage = 0
    @ObservationIgnored let latency = EngineV3Latency()
    /// What the reader thread may rasterise immediately (pages on screen, at which scale).
    @ObservationIgnored let rasterPlan = EngineV3RasterPlan()
    @ObservationIgnored private var keyMonitor: Any?
    @ObservationIgnored private var storageObserver: NSObjectProtocol?
    @ObservationIgnored private var clickMonitor: Any?
    @ObservationIgnored private var lastKeyNs: UInt64 = 0
    /// Set by a scripted bench just before it edits the text view.
    @ObservationIgnored var nextKeystrokeNs: UInt64?
    @ObservationIgnored private var restarts: [Date] = []
    @ObservationIgnored private var stopping = false
    /// `FLASHTEX_V3_FAST_EDITS=0` turns the text-storage fast path off (A/B).
    @ObservationIgnored let fastEdits = ProcessInfo.processInfo.environment["FLASHTEX_V3_FAST_EDITS"] != "0"
    @ObservationIgnored let logDone = ProcessInfo.processInfo.environment["FLASHTEX_V3_LOG_DONE"] == "1"
    @ObservationIgnored var log: (String) -> Void = { FlashTeXLog.write("engine-v3: " + $0) }

    /// This session's number in the process (its project copy's name).
    @ObservationIgnored let serial: Int = EngineV3Session.takeSerial()
    nonisolated(unsafe) private static var nextSerial = 0
    private static let serialLock = NSLock()
    nonisolated static func takeSerial() -> Int { serialLock.lock(); defer { serialLock.unlock() }; nextSerial += 1; return nextSerial }
    /// The project copy the host compiles (tests).
    var projectCopy: URL? { project?.root }
    /// Settings > "Smooth fonts in preview" (PreviewFontSmoothing.swift):
    /// handed to every raster (reader thread and view) as a value; a change
    /// redraws the pages on screen.
    @ObservationIgnored var smoothFonts: Bool {
        didSet {
            guard smoothFonts != oldValue else { return }
            rasterPlan.smoothFonts = smoothFonts
            view?.fontSmoothingChanged()
        }
    }
    @ObservationIgnored private var fontSmoothingObserver: NSObjectProtocol?

    /// `smoothFonts` nil: follow the Settings preference (the app's
    /// session); a value: fixed at it (tests).
    init(smoothFonts: Bool? = nil) {
        self.smoothFonts = smoothFonts ?? PreviewFontSmoothing.enabled
        rasterPlan.smoothFonts = self.smoothFonts
        if smoothFonts == nil {
            fontSmoothingObserver = PreviewFontSmoothing.observe { [weak self] on in self?.smoothFonts = on }
        }
    }

    // MARK: lifecycle

    /// Starts the host (once) and compiles the model's documents.
    func start(model: ShellModel) {
        self.model = model
        stopping = false
        if keyMonitor == nil {
            keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] e in
                self?.lastKeyNs = MonotonicClock.ns(fromUptimeSeconds: e.timestamp)
                return e
            }
        }
        if clickMonitor == nil {
            // ⌘-click in the editor: forward search to the preview (after the
            // click has moved the caret).
            clickMonitor = NSEvent.addLocalMonitorForEvents(matching: .leftMouseDown) { [weak self] e in
                if e.modifierFlags.contains(.command), let tv = e.window?.firstResponder as? NSTextView,
                   tv.accessibilityLabel() == "LaTeX source", let v = e.window?.contentView?.hitTest(e.locationInWindow),
                   v === tv || v.isDescendant(of: tv) {
                    DispatchQueue.main.async { self?.model?.revealCaretInPreview() }
                }
                return e
            }
        }
        if storageObserver == nil, fastEdits {
            storageObserver = NotificationCenter.default.addObserver(forName: NSTextStorage.didProcessEditingNotification, object: nil, queue: nil) { [weak self] note in
                guard let storage = note.object as? NSTextStorage else { return }
                MainActor.assumeIsolated { self?.storageEdited(storage) }
            }
        }
        if pages.isEmpty, snapshot == nil {
            if openStartNs == nil { openStartNs = MonotonicClock.nowNs() }
            showSnapshot(model: model) // instant reopen: before the host even starts
        }
        switch phase {
        case .starting: return
        case .ready: compile(model: model, reason: "start"); return
        case .idle, .failed: break
        }
        launchHost()
    }

    private func launchHost() {
        guard let exe = EngineV3.locateHost() else {
            phase = .failed("flashtex-host not found. Build it (cargo build --release -p flashtex-engine --bin flashtex-host) or set FLASHTEX_HOST / the \(EngineV3.hostPathKey) default.")
            return
        }
        EngineV3HostProcess.killStaleHosts(log: log)
        EngineV3Mirror.removeAbandoned(log: log)
        phase = .starting(since: Date())
        environmentNote = "Preparing the pdfLaTeX format from your TeX Live (the first use builds it; a few seconds)…"
        log("starting \(exe.path)")
        let ref = EngineV3WeakRef(self)
        do {
            let h = try EngineV3HostProcess(executable: exe) { event in
                EngineV3Session.onMain { ref.value?.hostEvent(event) }
            }
            host = h
            hostStarts += 1
        } catch {
            phase = .failed("could not start \(exe.lastPathComponent): \(error.localizedDescription)")
        }
    }

    func stop() {
        stopping = true
        view?.dropAllTiles() // queued tile jobs skip undrawn; kept page rasters are freed
        if let model, !model.engineV3Diagnostics.isEmpty { model.engineV3Diagnostics = [] }
        connection?.bye()
        connection = nil
        host?.terminate()
        host = nil
        phase = .idle
        sentTexts = [:]; hostBytes = [:]; fastPending = []
        pages = [:]; stale = []; forms = [:]; pdfFallback = [:]
        pageCount = 0
        layoutRevision &+= 1
        if let keyMonitor { NSEvent.removeMonitor(keyMonitor) }
        keyMonitor = nil
        if let storageObserver { NotificationCenter.default.removeObserver(storageObserver) }
        storageObserver = nil
        if let clickMonitor { NSEvent.removeMonitor(clickMonitor) }
        clickMonitor = nil
    }

    /// The host died or the connection broke: start another (bounded), keep
    /// the pages on screen as stale until the new host sends them.
    private func restart(_ why: String) {
        connection = nil
        host?.terminate()
        host = nil
        guard !stopping, phase != .idle else { return }
        restarts = restarts.filter { $0.timeIntervalSinceNow > -60 } + [Date()]
        guard restarts.count <= 3 else {
            phase = .failed("The preview engine stopped repeatedly (\(why)). Toggle the preview to restart it.")
            return
        }
        log("restarting the host: \(why)")
        sentTexts = [:]; hostBytes = [:]; fastPending = []
        markStale(Set(pages.keys))
        phase = .idle
        launchHost()
    }

    private func hostEvent(_ e: EngineV3HostProcess.Event) {
        switch e {
        case .line(let l):
            log(l)
        case .prepared(let j):
            let texlive = j["texlive"]?.string ?? "no TeX Live found"
            let formats = (j["formats"]?.array ?? []).map { f in
                "\(f["name"]?.string ?? "?") \(f["status"]?.string ?? "?")\(f["error"]?.string.map { ": " + $0 } ?? "")"
            }.joined(separator: ", ")
            environmentNote = "TeX Live: \(texlive) — format \(formats)"
            if (j["formats"]?.array ?? []).contains(where: { $0["status"]?.string == "failed" }) {
                phase = .failed("The pdfLaTeX format could not be prepared: \(formats)")
            }
        case .listening(let socket):
            connect(socket: socket)
        case .exited(let pid, let status):
            log("host \(pid) exited with status \(status)")
            guard pid == host?.pid else { return } // an earlier host, already replaced
            if case .failed = phase { host = nil; connection = nil; return }
            restart("the host exited with status \(status)")
        }
    }

    private func connect(socket: String) {
        let ref = EngineV3WeakRef(self)
        let plan = rasterPlan
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let c = try DL3Connection(socketPath: socket, client: "FlashTeX (engine-v3 preview)", accept: [DL3Diag.capability])
                let reader = EngineV3Reader(cache: .shared, plan: plan)
                c.start(onTimedEvent: { ev, timing in
                    guard let out = reader.handle(ev, timing: timing) else { return }
                    EngineV3Session.onMain { ref.value?.apply(out, connection: c) }
                }, onClose: { err in
                    EngineV3Session.onMain { ref.value?.closed(err, connection: c) }
                })
                EngineV3Session.onMain {
                    guard let self = ref.value else { c.bye(); return }
                    self.connection = c
                    self.hostOffersDiagV1 = c.hello["capabilities"]?.array?.contains(.string(DL3Diag.capability)) ?? false
                    if case .failed = self.phase {} else { self.phase = .ready }
                    self.log("connected: \(c.hello["server"]?.string ?? "?"), \(c.hello["engine"]?.string ?? "?")")
                    if let model = self.model { self.compile(model: model, reason: "open") }
                }
            } catch {
                EngineV3Session.onMain { ref.value?.restart("could not connect: \(error)") }
            }
        }
    }

    private func closed(_ err: DL3Error?, connection c: DL3Connection) {
        guard c === connection else { return }
        log("connection closed\(err.map { ": \($0)" } ?? "")")
        restart("the connection closed\(err.map { ": \($0)" } ?? "")")
    }

    // MARK: edits → COMPILE

    /// Fast path: the editor's text storage changed. The change goes to the
    /// host as a byte splice at once, before the editor's own processing
    /// (syntax colouring, gutter, the model's copy of the text): measured
    /// 11.5 ms of 34 at 1,000 pages when the hook sat in `updateActiveText`.
    /// Only for typing in the main editor (a key event, or the bench's
    /// stamp), outside an input-method composition, for a small edit of the
    /// active document; everything else takes the slow path, and the slow
    /// path checks the fast path's byte count and resends the buffer if
    /// they ever disagree.
    private func storageEdited(_ storage: NSTextStorage) {
        guard phase == .ready, connection != nil, let model, model.engineV3Enabled,
              storage.editedMask.contains(.editedCharacters) else { return }
        guard let tv = storage.layoutManagers.first?.textContainers.first?.textView,
              tv.accessibilityLabel() == "LaTeX source", !tv.hasMarkedText() else { return }
        let typing = nextKeystrokeNs != nil || NSApp.currentEvent?.type == .keyDown
        let path = model.activePath
        guard typing, let base = hostBytes[path], sentTexts[path] != nil || fastPending.contains(path) else { return }
        guard project?.exists == true else { return } // the slow path re-creates it
        let r = storage.editedRange, delta = storage.changeInLength
        let oldLength = r.length - delta
        guard r.location != NSNotFound, r.length <= 4096, oldLength >= 0, oldLength <= 4096 else { return }
        let now = MonotonicClock.nowNs()
        let text = storage.mutableString
        let prefix = EngineV3Edits.utf8Count(text, NSRange(location: 0, length: r.location))
        let insert = text.substring(with: r)
        let insertBytes = insert.utf8.count
        let delete: Int
        let total: Int
        if oldLength == 0 {
            delete = 0
            total = base + insertBytes
        } else {
            let suffix = EngineV3Edits.utf8Count(text, NSRange(location: NSMaxRange(r), length: text.length - NSMaxRange(r)))
            delete = base - prefix - suffix
            total = prefix + insertBytes + suffix
        }
        guard delete >= 0, prefix + delete <= base else { return }
        let key = consumeKeystroke(now: now)
        var req = request(model: model)
        req.edits = [DL3CompileRequest.Edit(path: path, offset: prefix, delete: delete, insert: insert)]
        hostBytes[path] = total
        fastPending.insert(path)
        send(req, keystrokeNs: key, editNs: now, path: path)
    }

    private func consumeKeystroke(now: UInt64) -> UInt64 {
        let key = nextKeystrokeNs ?? (now &- lastKeyNs < 200_000_000 ? lastKeyNs : now)
        nextKeystrokeNs = nil
        return key
    }

    /// Slow path: the model is about to store the active document's new
    /// text (`ShellModel.updateActiveText`). When the fast path already sent
    /// this change, only record the text (after checking the byte count).
    func textChanged(model: ShellModel, activeText: String? = nil, keystrokeNs: UInt64? = nil) {
        guard phase == .ready, connection != nil else { return }
        let now = MonotonicClock.nowNs()
        let path = model.activePath
        if let activeText, fastPending.contains(path) {
            fastPending.remove(path)
            if activeText.utf8.count == hostBytes[path] {
                sentTexts[path] = activeText
                return
            }
            // Out of step: resend the whole buffer.
            log("fast path out of step for \(path) (\(hostBytes[path] ?? -1) bytes held, \(activeText.utf8.count) now); resending it")
            sentTexts[path] = nil
        }
        let key = keystrokeNs ?? consumeKeystroke(now: now)
        compile(model: model, reason: "edit", keystrokeNs: key, activeText: activeText, editNs: now)
    }

    /// The file the engine compiles: the project's entry (a single opened
    /// .tex is its own entry); when the entry declares no `\documentclass`
    /// but another open document does (a chapter opened first), that one.
    static func mainFile(model: ShellModel) -> String {
        let entry = model.project.entryPath
        let docs = model.documents
        func declaresClass(_ text: String) -> Bool {
            text.range(of: #"(?m)^[^%\n]*\\documentclass"#, options: .regularExpression) != nil
        }
        if let e = docs.first(where: { $0.path == entry }), declaresClass(e.text) { return entry }
        return docs.first(where: { declaresClass($0.text) })?.path ?? entry
    }

    /// The window opened another project or file (ShellModel.replaceProject):
    /// compile it now, from a fresh copy of the project.
    func projectChanged(model: ShellModel, openedAt: UInt64? = nil) {
        guard model.engineV3Enabled else { return }
        if let openedAt {
            guard openedAt != lastOpenHandled else { return } // documentURL's didSet already handled this open
            lastOpenHandled = openedAt
        }
        let at = openedAt ?? MonotonicClock.nowNs()
        if let key = EngineV3Snapshot.key(for: model), key == openKey, openFirstPixelsNs != nil {
            // The same open, reported again (the pane started first): keep the earliest start.
            openStartNs = min(openStartNs ?? at, at)
        } else {
            openStartNs = at
            openFirstPixelsNs = nil; openFirstCurrentNs = nil
        }
        showSnapshot(model: model)
        if phase == .ready { compile(model: model, reason: "open") } // otherwise the connection's first compile opens it
    }

    /// Trusts the open project (the pane's button): restricted \write18 from now on, recompiled.
    /// It records exactly the identities the prompt was computed from, and
    /// only while that project is still the one open (and its copy the one
    /// compiled). If an item changed since, the project stays untrusted and
    /// the prompt is computed again for what is there now.
    func trustProject() {
        guard let model, let p = trustPrompt, model.project.projectRoot == p.root, project?.source == p.root else { return }
        EngineV3Trust.record(p.need)
        // Decided again off the main thread (startWalk); the compile follows it.
        trustPending = true
        compile(model: model, reason: "trust")
    }

    /// Applies a trust decision (made off the main thread); sets the prompt when untrusted.
    private func applyTrust(_ d: EngineV3Trust.Decision?, root: URL?, main: URL?) {
        trustPending = false
        var trusted = true, prompt: TrustPrompt?
        if let d, let root, let main {
            trusted = d.trusted
            if !trusted { prompt = TrustPrompt(root: root, main: main, others: d.others, need: d.need) }
        }
        trustPrompt = prompt
        let rootPath = root.map { EngineV3Trust.canonical($0).path }, mainPath = main.map { EngineV3Trust.canonical($0).path }
        let extra = prompt?.need.filter { $0.path != rootPath && $0.path != mainPath }.count ?? 0
        if trustOtherCount != extra { trustOtherCount = extra }
        if projectTrusted != trusted { projectTrusted = trusted }
    }

    /// A file appeared in the project outside the editor (a pasted image,
    /// PasteImage.swift): link it into the copy now, so the compile the
    /// paste's own edit triggers (an "edit" compile, which does not walk the
    /// directory) already finds it.
    func projectFilesChanged(model: ShellModel) {
        guard model.engineV3Enabled, let project, project.source == model.project.projectRoot else { return }
        // Links only (as before trust and instant reopen): no fingerprints,
        // no quarantine look; the inputs are unknown until the next walk.
        _ = project.sync(except: Set(model.documents.map(\.path)), fingerprints: false, quarantine: false)
        inputsAtSync = nil
    }

    static let walkQueue = DispatchQueue(label: "flashtex.engine-v3.walk", qos: .userInitiated)
    @ObservationIgnored private var walkToken = 0
    @ObservationIgnored private var walkInFlight = false

    /// Walks the project copy's source on `walkQueue` (links, input
    /// fingerprints, quarantined files), decides trust there, then applies
    /// both on main and compiles. A newer walk or another project supersedes it.
    private func startWalk(model: ShellModel, reason: String, editorPaths: Set<String>) {
        guard let project else { return }
        walkToken &+= 1
        let token = walkToken
        walkInFlight = true
        let root = project.source, main = mainFile
        let mainURL = root.map { $0.appendingPathComponent(main) }
        // The editor's texts, for the shared-folder reference scan (values; read off main).
        let texts = Dictionary(model.documents.map { ($0.path, $0.text) }, uniquingKeysWith: { a, _ in a })
        let t0 = MonotonicClock.nowNs()
        Self.walkQueue.async { [weak self] in
            let shared = root.map(EngineV3Trust.isShared) ?? false
            let walk = project.sync(except: editorPaths, quarantine: !shared)
            let t1 = MonotonicClock.nowNs()
            let decision = root.map { EngineV3Trust.decide(root: $0, main: main, texts: texts, walkQuarantined: walk.quarantined) }
            let t2 = MonotonicClock.nowNs()
            let ms = String(format: "%.1f (walk %.1f, trust %.1f)", Double(t2 &- t0) / 1e6, Double(t1 &- t0) / 1e6, Double(t2 &- t1) / 1e6)
            EngineV3Session.onMain {
                guard let self, token == self.walkToken else { return }
                self.walkInFlight = false
                if self.logDone { self.log("walk (\(reason)): \(ms) ms, shared \(shared), quarantined \(walk.quarantined.count), trusted \(decision?.trusted ?? true)") }
                self.inputsAtSync = walk.inputs
                self.applyTrust(decision, root: root, main: mainURL)
                guard let model = self.model, self.project === project else { return }
                self.compile(model: model, reason: reason, walked: true)
            }
        }
    }

    private func request(model: ShellModel) -> DL3CompileRequest {
        let project = self.project!
        let entry = mainFile
        var req = DL3CompileRequest(id: nextID, root: project.root.path, main: entry)
        nextID += 1
        req.outputDir = project.output.path
        req.jobname = (entry as NSString).lastPathComponent.replacingOccurrences(of: ".tex", with: "")
        req.haveFonts = DL3ResourceCache.shared.heldFontKeys
        req.fontFormats = ["type3", "truetype", "opentype"] // DL3Renderer draws these (lane P3-FONTS-2)
        // `viewport` makes the host typeset up to that page first. For the
        // first page it costs the edited page ~6 ms (plain-10: host first
        // page p50 14.5 -> 20.3 ms, measured) and gains nothing: send it only
        // when the view is further down. FLASHTEX_V3_VIEWPORT=1/0 forces it (A/B).
        switch ProcessInfo.processInfo.environment["FLASHTEX_V3_VIEWPORT"] {
        case "1": req.viewport = visiblePage
        case "0": break
        default: if visiblePage > 0 { req.viewport = visiblePage }
        }
        // Owner decision 9A: a project from elsewhere runs no shell commands until trusted.
        req.shellEscape = EngineV3Trust.shellEscape(trusted: projectTrusted && !trustPending)
        return req
    }

    private func send(_ req: DL3CompileRequest, keystrokeNs: UInt64?, editNs: UInt64, path: String) {
        guard let connection else { return }
        do {
            try connection.compile(req)
            lastSentID = req.id
            if !compiling { compiling = true }
            if keystrokeNs != nil { view?.keystroke() }
            if let keystrokeNs { latency.sent(compile: req.id, keystrokeNs: keystrokeNs, editNs: editNs, path: path, at: MonotonicClock.nowNs()) }
        } catch {
            restart("could not send: \(error)")
        }
    }

    /// `activeText`: the active document's new text when the model has not
    /// stored it yet (the edit hook runs first).
    /// `walked`: the project walk for this compile has just run (startWalk).
    func compile(model: ShellModel, reason: String, keystrokeNs: UInt64? = nil, activeText: String? = nil, editNs: UInt64 = MonotonicClock.nowNs(), walked: Bool = false) {
        guard connection != nil else { return }
        var docs = model.documents
        if let activeText, let i = docs.firstIndex(where: { $0.path == model.activePath }) { docs[i].text = activeText }
        // The host compiles a copy of the project (EngineV3Mirror): it writes
        // the editor's text to its files, and must never write the user's.
        let projectRoot = model.project.projectRoot
        if project == nil || project?.source != projectRoot || generation != model.projectGeneration {
            // Another project (or file) in this window: a fresh copy, every
            // document sent again as a buffer, the old pages gone.
            if project?.source != projectRoot || project == nil { project = EngineV3Mirror(source: projectRoot, session: serial) }
            // Emptied on the walk queue, before this project's walk (same serial queue): not on main.
            if let p = project { Self.walkQueue.async { p.clear() } }
            generation = model.projectGeneration
            sentTexts = [:]; hostBytes = [:]; fastPending = []; inputsAtSync = nil
            pages = [:]; stale = []; forms = [:]; pdfFallback = [:]; pageCount = 0; layoutRevision &+= 1
            staleChangedNow()
            statusNote = ""; firstError = nil
            showSnapshot(model: model) // this project's stored pages, if still valid
            trustPending = true // decided by the walk below
            walkToken &+= 1 // a walk of the previous project no longer counts
            walkInFlight = false
        }
        mainFile = Self.mainFile(model: model)
        guard let project else { return }
        if !project.exists {
            // Something removed the copy the host runs in (its cwd is gone):
            // make it again and start a fresh host on it; the restart
            // compiles every document again as a buffer.
            log("the project copy \(project.base.lastPathComponent) vanished; re-creating it and restarting the host")
            project.ensure()
            _ = project.sync(except: Set(docs.map(\.path)), fingerprints: false, quarantine: false)
            restart("the project copy vanished")
            return
        }
        // Linking the project's other files walks its directory: on open and
        // explicit compiles, not per keystroke. The walk (off the main
        // thread) also gives the input files (instant reopen) and decides
        // trust; this compile is sent from its completion, so a new
        // project's first COMPILE always carries the decision.
        if !walked, reason != "edit" || trustPending {
            if reason == "edit", walkInFlight { return } // the walk's compile sends this text too
            startWalk(model: model, reason: reason, editorPaths: Set(docs.map(\.path)))
            return
        }
        var req = request(model: model)
        for doc in docs {
            if fastPending.contains(doc.path) { continue } // the fast path holds it; the model's copy is behind
            if let old = sentTexts[doc.path] {
                if old == doc.text { continue }
                var a = old, b = doc.text
                a.makeContiguousUTF8(); b.makeContiguousUTF8()
                let edit: DL3CompileRequest.Edit? = a.utf8.withContiguousStorageIfAvailable { ab in
                    b.utf8.withContiguousStorageIfAvailable { bb in
                        let e = EngineV3Edits.splice(old: ab, new: bb)
                        return DL3CompileRequest.Edit(path: doc.path, offset: e.offset, delete: e.delete,
                                                      insert: String(decoding: UnsafeBufferPointer(rebasing: bb[e.insertRange]), as: UTF8.self))
                    }
                } ?? nil
                if let edit { req.edits.append(edit) } else { req.buffers.append((doc.path, doc.text)) }
            } else {
                // First sight of this document (or a resync): its file in the
                // copy is the editor's text (the host checks `main` exists
                // before it applies buffers), and the buffer says so again.
                let dst = project.root.appendingPathComponent(doc.path)
                try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
                if (try? FileManager.default.destinationOfSymbolicLink(atPath: dst.path)) != nil { try? FileManager.default.removeItem(at: dst) }
                try? Data(doc.text.utf8).write(to: dst)
                req.buffers.append((doc.path, doc.text))
            }
            sentTexts[doc.path] = doc.text
            hostBytes[doc.path] = doc.text.utf8.count
        }
        if reason == "edit", req.edits.isEmpty, req.buffers.isEmpty { return }
        send(req, keystrokeNs: keystrokeNs, editNs: editNs, path: model.activePath)
    }

    // MARK: events from the reader

    private func apply(_ out: EngineV3Reader.Output, connection c: DL3Connection) {
        guard c === connection else { return } // a replaced connection's late frames
        handle(out)
        afterEvent?(out)
    }

    /// Tests: called after each event from the host has been applied.
    @ObservationIgnored var afterEvent: ((EngineV3Reader.Output) -> Void)?

    /// Applies one event from the host (internal for tests).
    func handle(_ out: EngineV3Reader.Output) {
        switch out {
        case .started(let j):
            errorCount = 0; warningCount = 0; firstError = nil
            diagnostics = []; diags = []
            if j["keep"]?.bool == false {
                sourceMap.reset() // span ids restart with the resource ids
                // Ids restart; the pages on screen stay (each resolved its own
                // resources when it arrived), stale until they are sent again,
                // and so do the stored pages of an instant reopen.
                markStale(Set(pages.keys))
            }
        case .page(let p, let compileID, let timing, let image):
            let index = Int(p.page.index)
            let changed = pages[index]?.page.hash != p.page.hash
            let sizeChanged = pages[index].map { $0.widthPt != p.widthPt || $0.heightPt != p.heightPt } ?? true
            pages[index] = p
            sourceIndexes[index] = nil
            stale.remove(index)
            pdfFallback[index] = nil
            if index >= pageCount {
                pageCount = index + 1; layoutRevision &+= 1
                // Stored pages the count now reaches again stay stale.
                if snapshot != nil { markStale(stale) }
            } else if sizeChanged { layoutRevision &+= 1 }
            // (pageArrived re-lays out itself when the page is new or resized:
            // it does not wait for SwiftUI's updateNSView.)
            if changed { latency.pageOnMain(compile: compileID, timing: timing, at: MonotonicClock.nowNs()) }
            // The preview follows the edit (CaretFollow.swift): the recompiled
            // page is here, re-aim at the caret (debounced, never re-arms).
            if changed, compileID == lastSentID { model?.caretFollow.note(.recompile) }
            let onScreen = view?.pageArrived(index, changed: changed, compileID: compileID, image: image) ?? false
            if changed, !onScreen { latency.offscreen(compile: compileID) }
            if logDone { log("PAGE \(index) compile \(compileID) changed \(changed) onScreen \(onScreen) image \(image.map { "\(type(of: $0.image)) \($0.pixelsPerPoint)" } ?? "nil") view \(view != nil)") }
        case .sources(let src):
            sourceMap.apply(src)
        case .form(let f):
            forms[f.page.index] = f
            view?.formArrived(f.page.index)
        case .pages(let j):
            if let n = j["count"]?.int { setCount(Int(n), complete: j["complete"]?.bool ?? false) }
            var newStale = Set<Int>()
            for r in j["stale"]?.array ?? [] {
                if let a = r.array, a.count == 2, let lo = a[0].int, let hi = a[1].int, lo <= hi { newStale.formUnion(Int(lo) ... Int(hi)) }
            }
            for r in j["current"]?.array ?? [] {
                if let a = r.array, a.count == 2, let lo = a[0].int, let hi = a[1].int, lo <= hi { stale.subtract(Int(lo) ... Int(hi)) }
            }
            // The host's "current" is about the pages it sent, never a stored one.
            markStale(stale.union(newStale))
        case .diag(let d):
            diags.append(d)
            if d.severity == "error" {
                errorCount += 1
                if firstError == nil {
                    let at = [d.file.map { ($0 as NSString).lastPathComponent }, d.line.map(String.init), d.col.map { String($0 + 1) }].compactMap { $0 }.joined(separator: ":")
                    firstError = (at.isEmpty ? "" : at + ": ") + d.message
                }
            } else if d.severity == "warning" { warningCount += 1 }
        case .diagnostic(let j):
            diagnostics.append(j)
            if j["severity"]?.string == "error" {
                errorCount += 1
                if firstError == nil {
                    let file = j["file"]?.string.map { ($0 as NSString).lastPathComponent }
                    let line = j["line"]?.int
                    let at = [file, line.map(String.init)].compactMap { $0 }.joined(separator: ":")
                    firstError = (at.isEmpty ? "" : at + ": ") + (j["message"]?.string ?? "error")
                }
            } else { warningCount += 1 }
        case .done(let j, let compileID):
            let status = j["status"]?.string ?? "?"
            if logDone { log("DONE \(j)") }
            if status == "failed", let project, !project.exists, let model {
                compile(model: model, reason: "recover") // the copy vanished under the host
            }
            if status != "cancelled" {
                if let n = j["pages"]?.int { setCount(Int(n), complete: true) }
                // A stored page the compile did not replace (it failed early)
                // stays on screen, stale, until its page arrives.
                markStale([])
                statusNote = "\(status) · \(j["mode"]?.string ?? "") · \(pageCount) page\(pageCount == 1 ? "" : "s") · \(String(format: "%.0f", j["elapsed_ms"]?.double ?? 0)) ms"
                if stale.isEmpty { snapshot = nil } // the compile's pages replaced the stored ones
                if status == "ok", compileID >= lastSentID { scheduleSnapshotSave() }
                if let model {
                    let mapped = diags.isEmpty ? Self.problems(diagnostics, model: model, projectRoot: project?.root)
                                               : Self.problems(diags: diags, model: model, projectRoot: project?.root)
                    if model.engineV3Diagnostics != mapped { model.engineV3Diagnostics = mapped }
                }
                loadFallbacks(pdf: j["pdf"]?.string)
            }
            latency.done(compile: compileID, cancelled: status == "cancelled", hostFirstPageMs: j["first_page_ms"]?.double)
            if compileID >= lastSentID, compiling { compiling = false }
        case .error(let j):
            statusNote = "error: \(j["code"]?.string ?? "?") \(j["message"]?.string ?? "")"
            log(statusNote)
            if let project, !project.exists, let model { compile(model: model, reason: "recover") }
        }
    }

    // MARK: instant reopen

    /// Pages on screen from the snapshot that no compile has sent yet: always stale.
    var storedOnScreen: Set<Int> {
        guard let s = snapshot?.0 else { return [] }
        return Set((0 ..< min(pageCount, s.pages.count)).filter { pages[$0] == nil })
    }

    /// The page size to lay out page `i` with: its display list's, else the snapshot's.
    func pageSize(_ i: Int) -> CGSize? {
        if let p = pages[i] { return CGSize(width: p.widthPt, height: p.heightPt) }
        guard let s = snapshot?.0, i < s.pages.count else { return nil }
        return CGSize(width: s.pages[i].width, height: s.pages[i].height)
    }

    /// The stored bitmap of page `i`, when the display list has not sent it yet.
    func snapshotImageURL(_ i: Int) -> URL? {
        guard pages[i] == nil, let (s, dir) = snapshot, i < s.pages.count, let name = s.pages[i].image else { return nil }
        return dir.appendingPathComponent(name)
    }

    /// Shows the project's snapshot at once (pages stale) if its documents
    /// still hash to what they were.
    ///
    /// The other input files are checked off the main thread
    /// (`validateStored`): the pages go on screen at once, stale, and are
    /// dropped if the check fails.
    func showSnapshot(model: ShellModel) {
        let key = EngineV3Snapshot.key(for: model)
        if pages.isEmpty, let found = snapshot, key == openKey {
            // Already on screen (the compile of the open cleared the page
            // list): put its pages and stale marks back, not checked again.
            showStored(found)
            return
        }
        openKey = key
        guard pages.isEmpty, let key, let root = model.project.projectRoot,
              let found = EngineV3Snapshot.loadDocuments(projectKey: key, documents: model.documents.map { ($0.path, $0.text) }) else { snapshot = nil; return }
        showStored(found)
        validateStored(found, root: root)
    }

    /// Checks a shown snapshot's other input files on the snapshot queue;
    /// drops its pages if any changed (or the folder cannot be listed).
    func validateStored(_ found: (EngineV3Snapshot, URL), root: URL) {
        let s = found.0
        Self.snapshotQueue.async { [weak self] in
            let ok = EngineV3Snapshot.inputsMatch(s, root: root)
            EngineV3Session.onMain {
                guard let self, !ok, let shown = self.snapshot, shown.1 == found.1, shown.0.savedAt == s.savedAt else { return }
                self.log("snapshot: inputs changed outside the app; stored pages dropped")
                self.dropStored()
            }
        }
    }

    /// Takes the stored pages off screen (their inputs changed).
    func dropStored() {
        snapshot = nil
        if pages.isEmpty { pageCount = 0 }
        markStale(stale.filter { pages[$0] != nil })
        layoutRevision &+= 1
        view?.dropStored()
        view?.relayout()
    }

    /// Puts a loaded snapshot on screen, every page stale (internal for tests).
    func showStored(_ found: (EngineV3Snapshot, URL)) {
        snapshot = found
        log("snapshot: \(found.0.pages.count) pages from \(found.1.lastPathComponent) (view \(view != nil)) at +\(Double(MonotonicClock.nowNs() &- (openStartNs ?? 0)) / 1e6) ms")
        pageCount = found.0.pages.count
        markStale([])
        layoutRevision &+= 1
        view?.relayout()
    }

    /// A page was committed to the screen: open → pixels bookkeeping.
    func noteOpenPixels(current: Bool) {
        guard let o = openStartNs else { return }
        let now = MonotonicClock.nowNs()
        if openFirstPixelsNs == nil || (current && openFirstCurrentNs == nil) { log("open pixels (\(current ? "current" : "stored")) at +\(Double(now &- o) / 1e6) ms") }
        if openFirstPixelsNs == nil { openFirstPixelsNs = now }
        if current, openFirstCurrentNs == nil { openFirstCurrentNs = now }
    }

    /// Saves the pages near the viewport (and the first ones) for the next open, after the compile settles.
    private func scheduleSnapshotSave() {
        snapshotSave?.cancel()
        // The pages are of the texts last sent (typing during the compile
        // does not count) and of the input files as last synced.
        guard let model, let key = EngineV3Snapshot.key(for: model), let root = project?.source, root == model.project.projectRoot,
              let synced = inputsAtSync, fastPending.isEmpty,
              pageCount > 0, (0 ..< pageCount).allSatisfy({ pages[$0] != nil }) else { return }
        let visible = visiblePage
        var chosen: [Int: DL3PreparedPage] = [:]
        for i in Array(0 ..< min(3, pageCount)) + Array(max(0, visible - 2) ... min(pageCount - 1, visible + 5)) where chosen.count < EngineV3Snapshot.maxPages {
            chosen[i] = pages[i]
        }
        let sizes = (0 ..< pageCount).map { CGSize(width: pages[$0]!.widthPt, height: pages[$0]!.heightPt) }
        let docs = EngineV3Snapshot.hashes(model.documents.map { ($0.path, sentTexts[$0.path] ?? $0.text) })
        let formsCopy = forms, main = mainFile
        let ppp = view?.currentPixelsPerPoint ?? 2
        let dark = model.darkPreview
        let item = DispatchWorkItem {
            // An input changed since the sync (outside the app): the pages
            // may not show it, so nothing is saved (the old snapshot no
            // longer matches either).
            let others = EngineV3Snapshot.others(synced, documents: docs.keys)
            guard let now = EngineV3Snapshot.inputs(root: root), EngineV3Snapshot.others(now, documents: docs.keys) == others else { return }
            EngineV3Snapshot.save(projectKey: key, main: main, documents: docs, inputs: others, sizes: sizes, pages: chosen,
                                  forms: formsCopy, pixelsPerPoint: ppp > 0 ? ppp : 2, dark: dark)
        }
        snapshotSave = item
        Self.snapshotQueue.asyncAfter(deadline: .now() + 1.5, execute: item)
    }

    /// Sets the stale marks: `s` and, always, every stored page not yet
    /// replaced (`storedOnScreen`). Every write of the marks goes through here.
    private func markStale(_ s: Set<Int>) {
        stale = s.union(storedOnScreen)
        staleChangedNow()
    }

    private func staleChangedNow() {
        let n = stale.count
        if staleCount != n { staleCount = n }
        view?.staleChanged()
    }

    /// DIAGNOSTIC messages as Problems-panel diagnostics: a file of the
    /// project (the engine names the copy's path) maps back to the editor's
    /// document and the reported line's byte range; anything else (a
    /// package file) keeps its place in the message.
    static func problems(_ diags: [DL3JSON], model: ShellModel, projectRoot: URL?) -> [RuntimeV1.Diagnostic] {
        let root = projectRoot.map { $0.standardizedFileURL.path + "/" }
        return diags.map { d in
            let severity: RuntimeV1.Severity = d["severity"]?.string == "error" ? .error : .warning
            var message = d["message"]?.string ?? "(no message)"
            var source: RuntimeV1.SourceRange?
            if var file = d["file"]?.string {
                if file.hasPrefix("./") { file.removeFirst(2) }
                if let root, file.hasPrefix(root) { file.removeFirst(root.count) }
                let line = Int(d["line"]?.int ?? 0)
                if let doc = model.documents.first(where: { $0.path == file }), line > 0,
                   let range = lineByteRange(doc.text, line: line) {
                    source = RuntimeV1.SourceRange(path: file, startByte: range.lowerBound, endByte: range.upperBound)
                } else {
                    message = "\((file as NSString).lastPathComponent)\(line > 0 ? ":\(line)" : ""): " + message
                }
            }
            return RuntimeV1.Diagnostic(severity: severity, message: message, source: source, recovery: nil, code: "engine-v3")
        }
    }

    /// diag-v1 DIAGs as Problems-panel diagnostics: the source range is the
    /// reported token/command (`range`, byte columns of `line`) or TeX's split
    /// (`col`), so a click lands on the exact column; the macro chain and
    /// TeX's help text become the row's notes and help.
    static func problems(diags: [DL3Diag], model: ShellModel, projectRoot: URL?) -> [RuntimeV1.Diagnostic] {
        let root = projectRoot.map { $0.standardizedFileURL.path + "/" }
        func rel(_ file: String) -> String {
            var f = file
            if f.hasPrefix("./") { f.removeFirst(2) }
            if let root, f.hasPrefix(root) { f.removeFirst(root.count) }
            return f
        }
        return diags.compactMap { d in
            guard d.severity != "info" else { return nil } // \show, tight/loose boxes: not problems
            var message = d.message
            var source: RuntimeV1.SourceRange?
            if let file = d.file.map(rel) {
                if let line = d.line, let doc = model.documents.first(where: { $0.path == file }),
                   let lineRange = lineByteRange(doc.text, line: line) {
                    let len = lineRange.count
                    let (a, b): (Int, Int) = {
                        if let r = d.range { return (min(r.0, len), min(max(r.1, r.0), len)) }
                        if let c = d.col { return (min(c, len), min(c, len)) }
                        return (0, len)
                    }()
                    source = RuntimeV1.SourceRange(path: file, startByte: lineRange.lowerBound + a, endByte: lineRange.lowerBound + b)
                } else {
                    message = "\((file as NSString).lastPathComponent)\(d.line.map { ":\($0)" } ?? "")\(d.col.map { ":\($0 + 1)" } ?? ""): " + message
                }
            }
            var notes: [String] = []
            if let detail = d.detail, !detail.isEmpty { notes.append(detail) }
            for f in d.trace where f.kind == "macro" {
                let def = f.def.flatMap { l in l.file.map { "\(rel($0))\(l.line.map { ":\($0)" } ?? "")" } }
                notes.append("in \(f.name ?? "a macro")" + (def.map { " (defined at \($0))" } ?? ""))
            }
            let help = d.help.isEmpty ? nil : RuntimeV1.Diagnostic.Help(message: d.help.joined(separator: " "))
            return RuntimeV1.Diagnostic(severity: d.severity == "error" ? .error : .warning, message: message, source: source,
                                        recovery: nil, code: d.code.isEmpty ? "engine-v3" : d.code,
                                        notes: notes.isEmpty ? nil : notes, help: help)
        }
    }

    /// Bytes of 1-based `line` in `text`, without its newline.
    static func lineByteRange(_ text: String, line: Int) -> Range<Int>? {
        var n = 1, start = 0, i = 0
        for b in text.utf8 {
            if b == 0x0A {
                if n == line { return start ..< i }
                n += 1; start = i + 1
            }
            i += 1
        }
        return n == line ? start ..< i : nil
    }

    private func setCount(_ n: Int, complete: Bool) {
        guard n != pageCount || complete else { return }
        if complete, n < pageCount {
            for i in n ..< pageCount { pages[i] = nil; stale.remove(i); pdfFallback[i] = nil }
        }
        if n != pageCount { pageCount = n; layoutRevision &+= 1 }
    }

    /// Pages the display list cannot draw exactly render from the compile's PDF.
    private func loadFallbacks(pdf: String?) {
        let need = pages.filter { $0.value.needsPDFFallback(forms: forms) }.map(\.key)
        guard !need.isEmpty, let pdf, let doc = CGPDFDocument(URL(fileURLWithPath: pdf) as CFURL) else { return }
        for i in need { if let p = doc.page(at: i + 1) { pdfFallback[i] = p } }
        view?.fallbacksChanged(need)
    }

    nonisolated static func onMain(_ block: @escaping @MainActor () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue) { MainActor.assumeIsolated { block() } }
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }
}

/// A weak reference that may cross threads (it is only dereferenced on main).
final class EngineV3WeakRef: @unchecked Sendable {
    weak var value: EngineV3Session?
    init(_ v: EngineV3Session) { value = v }
}

/// Which pages the reader thread may rasterise as they arrive: the ones on
/// screen, at the scale on screen. Written on main, read on the reader thread.
final class EngineV3RasterPlan: @unchecked Sendable {
    private let lock = NSLock()
    private var targets: [Int: EngineV3LayerTarget] = [:]
    private var pixelsPerPoint: Double = 0
    private var smooth = false
    private var appearanceValue: DL3Appearance = .light

    func set(targets: [Int: EngineV3LayerTarget], pixelsPerPoint: Double, appearance: DL3Appearance) {
        lock.lock(); self.targets = targets; self.pixelsPerPoint = pixelsPerPoint; appearanceValue = appearance; lock.unlock()
    }

    var appearance: DL3Appearance { lock.lock(); defer { lock.unlock() }; return appearanceValue }

    /// Settings > "Smooth fonts in preview" (`EngineV3Session.smoothFonts`).
    var smoothFonts: Bool {
        get { lock.lock(); defer { lock.unlock() }; return smooth }
        set { lock.lock(); smooth = newValue; lock.unlock() }
    }

    /// Where, at which scale and with which font smoothing to draw page `i`
    /// now, or nil when it is not near the screen.
    func target(for i: Int) -> (EngineV3LayerTarget, Double, smoothFonts: Bool)? {
        lock.lock(); defer { lock.unlock() }
        guard pixelsPerPoint > 0, let t = targets[i] else { return nil }
        return (t, pixelsPerPoint, smooth)
    }
}

/// A bitmap the reader thread drew, with the scale and content it was drawn for.
struct EngineV3Raster: @unchecked Sendable {
    /// Layer contents: an IOSurface (zero-copy commit) or a CGImage.
    var image: AnyObject
    /// The install ticket taken before drawing; when installed on the reader
    /// thread, when that began and when it was committed.
    var ticket: UInt64
    var installNs: UInt64
    var committedNs: UInt64?
    var pixelsPerPoint: Double
    var hash: [UInt8]
}

/// Reader-thread state: resource bindings of the connection, the forms, and
/// the compile each frame belongs to. Pages are prepared here, off the main
/// thread, and a page on screen is rasterised here too, so it reaches the
/// main thread ready to install (one hop instead of three).
final class EngineV3Reader: @unchecked Sendable {
    enum Output {
        case started(DL3JSON)
        case page(DL3PreparedPage, compileID: Int, timing: EngineV3Latency.PageTiming, image: EngineV3Raster?)
        case form(DL3PreparedPage)
        case pages(DL3JSON)
        case diagnostic(DL3JSON)
        case diag(DL3Diag)
        case sources(DL3Sources)
        case done(DL3JSON, compileID: Int)
        case error(DL3JSON)
    }
    private var bindings = DL3Bindings()
    private var forms: [UInt32: DL3PreparedPage] = [:]
    private let cache: DL3ResourceCache
    private let plan: EngineV3RasterPlan
    private var compileID = 0

    init(cache: DL3ResourceCache, plan: EngineV3RasterPlan) { self.cache = cache; self.plan = plan }

    func handle(_ ev: DL3Event, timing t: DL3Connection.Timing) -> Output? {
        switch ev {
        case .started(let j):
            compileID = Int(j["id"]?.int ?? 0)
            if j["keep"]?.bool == false { bindings.reset() }
            return .started(j)
        case .font(let f): bindings.bind(font: f, cache: cache); return nil
        case .image(let j): bindings.bind(image: j, cache: cache); return nil
        case .page(let p):
            var timing = EngineV3Latency.PageTiming(readNs: t.readNs, decodedNs: t.decodedNs)
            let prepared = bindings.prepare(p)
            timing.preparedNs = DispatchTime.now().uptimeNanoseconds
            var image: EngineV3Raster?
            if let (target, ppp, smooth) = plan.target(for: Int(p.index)), !prepared.needsPDFFallback(forms: forms) {
                let ticket = EngineV3LayerTarget.ticket()
                timing.raster0Ns = DispatchTime.now().uptimeNanoseconds
                let look = plan.appearance
                if let img = DL3Renderer.rasterizeToSurface(prepared, forms: forms, scale: ppp, appearance: look, smoothFonts: smooth) {
                    timing.raster1Ns = DispatchTime.now().uptimeNanoseconds
                    // On screen now, from this thread: the main thread only records it.
                    let committed = target.install(img, ticket: ticket)
                    image = EngineV3Raster(image: img, ticket: ticket, installNs: timing.raster1Ns, committedNs: committed,
                                           pixelsPerPoint: ppp, hash: EngineV3PagesView.contentKey(p.hash, look, smoothFonts: smooth))
                } else {
                    timing.raster1Ns = DispatchTime.now().uptimeNanoseconds
                }
            }
            return .page(prepared, compileID: compileID, timing: timing, image: image)
        case .form(let p):
            let f = bindings.prepare(p)
            forms[p.index] = f
            return .form(f)
        case .pages(let j): return .pages(j)
        case .diagnostic(let j): return .diagnostic(j)
        case .diag(let d): return .diag(d)
        case .done(let j): return .done(j, compileID: Int(j["id"]?.int ?? Int64(compileID)))
        case .error(let j): return .error(j)
        case .sources(let s): return .sources(s)
        case .hello, .other: return nil
        }
    }
}

/// Byte splices between what the host holds and the editor's text.
enum EngineV3Edits {
    struct Splice: Equatable { var offset: Int; var delete: Int; var insertRange: Range<Int> }

    /// UTF-8 length of `range` of `s` (Core Foundation's converter, no copy).
    static func utf8Count(_ s: NSString, _ range: NSRange) -> Int {
        guard range.length > 0 else { return 0 }
        var used: CFIndex = 0
        CFStringGetBytes(s as CFString, CFRange(location: range.location, length: range.length),
                         CFStringBuiltInEncodings.UTF8.rawValue, 0, false, nil, 0, &used)
        return used
    }

    /// The shortest single splice turning `old` into `new` (common prefix
    /// and suffix; never splitting a UTF-8 sequence, so `insert` is text).
    static func splice(old: [UInt8], new: [UInt8]) -> Splice {
        old.withUnsafeBufferPointer { a in new.withUnsafeBufferPointer { b in splice(old: a, new: b) } }
    }

    static func splice(old a: UnsafeBufferPointer<UInt8>, new b: UnsafeBufferPointer<UInt8>) -> Splice {
        var p = 0
        let n = min(a.count, b.count)
        // Word-at-a-time prefix scan, then bytes.
        if let pa = a.baseAddress, let pb = b.baseAddress {
            while p + 8 <= n, UnsafeRawPointer(pa + p).loadUnaligned(as: UInt64.self) == UnsafeRawPointer(pb + p).loadUnaligned(as: UInt64.self) { p += 8 }
        }
        while p < n, a[p] == b[p] { p += 1 }
        while p > 0, p < b.count, b[p] & 0xC0 == 0x80 { p -= 1 } // back to a character boundary
        var s = 0
        let m = n - p
        if let pa = a.baseAddress, let pb = b.baseAddress {
            while s + 8 <= m, UnsafeRawPointer(pa + a.count - s - 8).loadUnaligned(as: UInt64.self) == UnsafeRawPointer(pb + b.count - s - 8).loadUnaligned(as: UInt64.self) { s += 8 }
        }
        while s < m, a[a.count - 1 - s] == b[b.count - 1 - s] { s += 1 }
        while s > 0, b.count - s < b.count, b[b.count - s] & 0xC0 == 0x80 { s -= 1 }
        return Splice(offset: p, delete: a.count - s - p, insertRange: p ..< b.count - s)
    }
}

/// The host's copy of the project: the user's files are never written. Text
/// the editor holds is sent as `buffers`/`edits` (the host writes it here);
/// every other file of the project is linked in (read-only use: images,
/// bibliographies, included files the editor has not opened). An untitled
/// document gets an empty directory.
final class EngineV3Mirror: @unchecked Sendable { // only `let`s; its walks touch the filesystem only (idempotent links)
    let source: URL?
    let base: URL
    let root: URL
    let output: URL

    /// The copy of `source` owned by THIS app instance:
    /// `projects/<hash of the project path>-<pid>`, with an `owner` file
    /// ("pid start-sec start-usec"). Another running instance (the same
    /// project open twice, a second app, a bench) has its own copy; nothing
    /// here ever removes a copy whose owner still runs.
    /// `session`: the owning session's number in this process (two windows
    /// with the same project get two copies, as two hosts compile them).
    init(source: URL?, session: Int = 0) {
        self.source = source
        let key = SHA256.hash(data: Data((source?.path ?? "untitled").utf8)).prefix(8).map { String(format: "%02x", $0) }.joined()
        base = EngineV3.cacheDirectory.appendingPathComponent("projects/\(key)-\(getpid())-\(session)", isDirectory: true)
        root = base.appendingPathComponent("src", isDirectory: true)
        output = base.appendingPathComponent("out", isDirectory: true)
        ensure()
    }

    /// Whether the copy is still there (something outside may remove it).
    var exists: Bool { FileManager.default.fileExists(atPath: root.path) && FileManager.default.fileExists(atPath: output.path) }

    /// (Re)creates the directories and the owner file.
    func ensure() {
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        try? FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        try? Data(EngineV3.instanceOwner.utf8).write(to: base.appendingPathComponent("owner"))
    }

    /// Removes project copies whose owner instance has exited. Copies
    /// without an owner file (made by an older build that may still be
    /// running) and copies of running instances are left alone.
    static func removeAbandoned(log: (String) -> Void) {
        let dir = EngineV3.cacheDirectory.appendingPathComponent("projects", isDirectory: true)
        let fm = FileManager.default
        for name in (try? fm.contentsOfDirectory(atPath: dir.path)) ?? [] {
            let base = dir.appendingPathComponent(name)
            guard let owner = try? String(contentsOf: base.appendingPathComponent("owner"), encoding: .utf8),
                  owner != EngineV3.instanceOwner, !EngineV3.ownerAlive(owner) else { continue }
            try? fm.removeItem(at: base)
            log("removed the project copy \(name) of exited instance \(owner)")
        }
    }

    /// Empties this instance's copy (another project or file now uses it).
    func clear() {
        let fm = FileManager.default
        for dir in [root, output] {
            for name in (try? fm.contentsOfDirectory(atPath: dir.path)) ?? [] { try? fm.removeItem(at: dir.appendingPathComponent(name)) }
        }
        ensure()
    }

    /// Links every project file not in `editorPaths` (which the host writes)
    /// into the copy. Bounded: 20,000 entries, hidden directories skipped.
    /// Returns what it saw: the input files (EngineV3Snapshot.inputs(root:),
    /// nil when it could not list them all) and the quarantined files (trust).
    struct Walk { var inputs: [String: String]?; var quarantined: [URL] }

    /// `fingerprints`/`quarantine`: whether to collect the input files and
    /// the quarantined files (each costs a syscall per file).
    func sync(except editorPaths: Set<String>, fingerprints: Bool = true, quarantine: Bool = true) -> Walk {
        guard let source else { return Walk(inputs: nil, quarantined: []) }
        let fm = FileManager.default
        guard let e = fm.enumerator(at: source, includingPropertiesForKeys: [.isDirectoryKey], options: [.skipsHiddenFiles, .skipsPackageDescendants]) else { return Walk(inputs: nil, quarantined: []) }
        var n = 0
        var inputs: [String: String]? = fingerprints ? [:] : nil
        var quarantined: [URL] = []
        let prefix = source.standardizedFileURL.path + "/"
        for case let url as URL in e {
            n += 1
            if n > EngineV3Snapshot.maxEntries { inputs = nil; break }
            let path = url.standardizedFileURL.path
            guard path.hasPrefix(prefix) else { continue }
            let rel = String(path.dropFirst(prefix.count))
            let dst = root.appendingPathComponent(rel)
            if (try? url.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true {
                try? fm.createDirectory(at: dst, withIntermediateDirectories: true)
                continue
            }
            if EngineV3Snapshot.isInput(rel) { inputs?[rel] = EngineV3Snapshot.fingerprint(path) ?? "unreadable" }
            if quarantine, EngineV3Trust.isQuarantined(url) { quarantined.append(url) }
            if editorPaths.contains(rel) {
                // A link here would let the host write through to the user's file.
                if (try? fm.destinationOfSymbolicLink(atPath: dst.path)) != nil { try? fm.removeItem(at: dst) }
                continue
            }
            if (try? fm.destinationOfSymbolicLink(atPath: dst.path)) == nil, !fm.fileExists(atPath: dst.path) {
                try? fm.createSymbolicLink(at: dst, withDestinationURL: url)
            }
        }
        return Walk(inputs: inputs, quarantined: quarantined)
    }
}
