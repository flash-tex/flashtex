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
/// **One host per ShellModel, for its project.** The app has one ShellModel
/// (an App-level `@State`, FlashTeXMacApp.swift), so today this is one host
/// per app, and every window of the app shows that one project. The host
/// keeps one resident engine for one job (root, main file, …); the compile
/// unit is the project's entry file, so every tab of a project shares its
/// job. Should the app ever give each window its own ShellModel, each would
/// get its own session and host: document-scoped sessions inside one host
/// would serialise every project's compiles on the host's single engine
/// thread and make a COMPILE for one project evict another's checkpoints.
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
    /// A bundle fetch in progress (no TeX Live), for the status bar:
    /// "downloading TeX files: 1.2 of 2.8 MB (43%)"; nil when none runs.
    private(set) var bundleProgressNote: String?
    /// The first-download consent sheet is up (EngineV3BundleSheet; ContentView attaches it).
    var bundleConsentShown = false
    /// The bundle the sheet asks about.
    private(set) var bundleConsentConfig: EngineV3Bundle.Config?
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
    /// Edits typed while auto-compile is off, not yet sent (⌘B sends them).
    private(set) var editsWaiting = false
    /// An Export PDF… or Print… is producing its PDF (`export`).
    private(set) var exporting = false
    @ObservationIgnored private var exportStage: ExportStage?
    @ObservationIgnored private var exportCompletion: (@MainActor (Result<Data, ExportFailure>) -> Void)?
    /// An edit or compile came while the export ran; sent once it is done.
    @ObservationIgnored private var heldDuringExport = false
    /// The external tools' state for the pane and the status bar ("Running
    /// bibtex paper…", a failure, or why one did not run); nil when settled.
    private(set) var toolNote: String?
    /// DIAGNOSTICs from bibtex/biber/makeindex (`source`) of the latest tool
    /// cycle: kept apart from the TeX run's, which a follow-up compile's
    /// STARTED resets, and shown with them.
    @ObservationIgnored private var toolDiagnostics: [DL3JSON] = []
    @ObservationIgnored private var toolCycleID = -1
    /// The TeX run's rows of the last DONE (Problems panel), before the tools'.
    @ObservationIgnored private var texProblems: [RuntimeV1.Diagnostic] = []
    /// The last compile sent with `external_tools: auto`, the last cycle the
    /// host said was settled, and the last compile that finished.
    @ObservationIgnored private var lastToolsAutoID = 0
    @ObservationIgnored private var lastSettledID = 0
    @ObservationIgnored private var lastDoneID = 0
    /// Completed compiles: the caret mark is worked out again on new pages
    /// (not observed: the pane is told directly, `scheduleCaretMark`).
    @ObservationIgnored private(set) var contentStamp = 0
    /// The caret mark's inputs, kept out of SwiftUI (EngineV3CaretMark.swift):
    /// the compiled text's line table and the copy's root spellings.
    @ObservationIgnored var caretLines: (path: String, stamp: Int, length: Int, table: EngineV3CaretPlace.LineTable)?
    /// Bumped when the compiled texts change (a DONE): the line table's key.
    @ObservationIgnored var compiledStamp = 0
    /// The edit window of each compile sent and not done, by id, and of the
    /// compiled text the pages show (EngineV3CaretPlace.Window).
    @ObservationIgnored var caretWindows: [Int: EngineV3CaretPlace.Window] = [:]
    @ObservationIgnored var caretWindow: EngineV3CaretPlace.Window?
    /// The editor's text storage the windows follow (its length checks them).
    @ObservationIgnored weak var caretStorage: NSTextStorage?
    /// The editor's characters changed since the model last took a text:
    /// the next `textChanged` comes from the editor (else from outside it).
    @ObservationIgnored var caretEditorEdited = false
    /// The model's text came from outside the editor (a reload, a restored
    /// snapshot, a programmatic replace) and the editor has not given the
    /// model a text since: a compile sent now reads a text the editor may
    /// not show yet, so its window starts invalid (the texts are compared).
    @ObservationIgnored var caretEditorOutOfStep = false
    /// Caret places mapped by the window, not by comparing texts (tests).
    @ObservationIgnored var caretMapsByWindow = 0
    /// Edits the fast path sent as compiles (tests).
    @ObservationIgnored var fastEditsSent = 0
    /// Fast-path splices the slow path found out of step (the buffer resent).
    @ObservationIgnored var fastResyncs = 0
    /// The fast path holds anchors for `path` (tests).
    func fastAnchored(_ path: String) -> Bool { fastAnchors[path] != nil }
    @ObservationIgnored var copyRoots: (copy: URL, roots: [String])?
    @ObservationIgnored var caretMarkScheduled = false
    @ObservationIgnored var caretMarkSettling = false
    /// The editor revision the mark was last worked out for: a newer one means typing.
    @ObservationIgnored var caretMarkedRevision = 0
    /// Pages installed (each one's glyph index is rebuilt on its next
    /// lookup), and how many there had been when the mark was last worked out.
    @ObservationIgnored var pageInstalls = 0
    @ObservationIgnored var caretMarkedInstalls = 0
    /// The tools of the last compile that allowed them have settled.
    var toolsSettled: Bool { lastSettledID >= lastToolsAutoID }
    @ObservationIgnored private var lastSentID = 0
    /// DIAGNOSTIC messages of the compile in progress (published at its DONE).
    @ObservationIgnored private var diagnostics: [DL3JSON] = []
    /// diag-v1 DIAGs of the compile in progress (the host sends these
    /// instead of DIAGNOSTICs when it offers diag-v1).
    @ObservationIgnored private var diags: [DL3Diag] = []
    /// The connected host offers diag-v1 (and so sends DIAGs, not DIAGNOSTICs).
    @ObservationIgnored private(set) var hostOffersDiagV1 = false
    /// The connected host offers `trim-v1` (memory pressure: TRIM frames).
    @ObservationIgnored private(set) var hostOffersTrim = false
    /// TRIM frames sent, and memory-pressure events applied (tests, evidence).
    @ObservationIgnored private(set) var trimsSent = 0
    @ObservationIgnored private(set) var pressureEvents = 0
    /// The connected host stops at the first error when asked (`halt-on-error`):
    /// with an older one, strict mode shows errors as errors but TeX goes on.
    private(set) var hostHonoursHaltOnError = true
    /// The first TeX error of the last compile ("file:line: message"), shown
    /// in the pane: under best effort only one TeX stopped at (and its cause).
    private(set) var firstError: String?
    /// How TeX's errors are presented (EngineV3ErrorPolicy): best effort shows
    /// the errors TeX recovers from as warnings.
    @ObservationIgnored var errorMode: EngineV3ErrorPolicy.Mode = EngineV3ErrorPolicy.storedMode
    /// Strict mode is on but the host does not honour `halt_on_error`
    /// (Settings > Compile says so): errors are errors, TeX still goes on.
    var strictModeIgnored: Bool { errorMode == .strict && phase == .ready && !hostHonoursHaltOnError }
    /// The compile in progress stopped on a fatal error (a `fatal` DIAG):
    /// its DONE keeps the last good pages after the ones it made, stale.
    @ObservationIgnored private(set) var compileFatal = false
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
    /// The project folder's file set (EngineV3ProjectWatcher): a change makes
    /// the next compile, an edit's included, walk the project and decide
    /// trust again first, so `external_tools`/shell escape never apply to a
    /// file the last trust check did not see.
    @ObservationIgnored private var projectWatcher: EngineV3ProjectWatcher?
    /// Times the file set changed (tests).
    @ObservationIgnored private(set) var projectFileSetChanges = 0
    /// The model's project generation the session compiles (ShellModel.replaceProject bumps it).
    @ObservationIgnored private var generation = -1
    /// The main file of the last COMPILE (relative to the project).
    private(set) var mainFile = ""

    @ObservationIgnored private(set) var pages: [Int: DL3PreparedPage] = [:]
    /// Each page's size (bp), kept with `pages`: the layout reads every page's
    /// size, and a `DL3PreparedPage` copy out of `pages` retains a dozen arrays.
    @ObservationIgnored private var pageSizes: [Int: CGSize] = [:]
    @ObservationIgnored private(set) var stale: Set<Int> = []
    @ObservationIgnored private(set) var forms: [UInt32: DL3PreparedPage] = [:]
    /// Pages whose PDF rendering replaces the display list (INCOMPLETE, or a
    /// resource that did not resolve), rendered from `DONE.pdf`.
    @ObservationIgnored private(set) var pdfFallback: [Int: CGPDFPage] = [:]
    /// The document `pdfFallback`'s pages come from (`DL3Renderer.openPDF`: its bytes, for per-thread copies).
    @ObservationIgnored private var pdfFallbackDocument: CGPDFDocument?
    /// Pages drawn from the compile's PDF (tests, evidence).
    var pdfFallbackCount: Int { pdfFallback.count }
    /// The main thread's time in the last DONE, and in its PDF fallback
    /// load (`loadFallbacks`), in ms (evidence: `EngineV3DoneBench`).
    @ObservationIgnored private(set) var lastDoneMainMs = 0.0
    @ObservationIgnored private(set) var lastFallbackLoadMs = 0.0
    /// The DONEs received, and the last one (evidence: `EngineV3PageCapture`).
    @ObservationIgnored private(set) var doneCount = 0
    @ObservationIgnored private(set) var lastDone: DL3JSON?
    /// When the last event from the host was applied (a host running external
    /// tools compiles again by itself, after its DONE).
    @ObservationIgnored private(set) var lastEventNs: UInt64 = 0
    /// Pages drawn from the compile's PDF because the host flagged them
    /// INCOMPLETE or they draw an INCOMPLETE form (0-based).
    var incompletePages: [Int] { pages.filter { $0.value.needsPDFFallback(forms: forms) }.map(\.key).sorted() }
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
    static let snapshotQueue = DispatchQueue(label: "flashtex.engine-v3.snapshot", qos: .utility, autoreleaseFrequency: .workItem)
    /// SOURCES of the connection: span id → (file, line) (EngineV3SourceMap.swift).
    @ObservationIgnored var sourceMap = DL3SourceMap()
    /// Per-page glyph indexes for forward/reverse search, built on first
    /// use, bounded (EngineV3GlyphIndexes.swift).
    @ObservationIgnored var glyphIndexes = EngineV3GlyphIndexes()
    @ObservationIgnored weak var view: EngineV3PagesView? { didSet { view?.rasterPlan = rasterPlan } }
    @ObservationIgnored weak var model: ShellModel?

    @ObservationIgnored private var host: EngineV3HostProcess?
    /// The running host's pid (tests, diagnostics).
    var hostPID: Int32? { host?.pid }
    /// Hosts started by this session (the first, then each restart).
    @ObservationIgnored private(set) var hostStarts = 0

    /// View > Show Preview Debug Status under v3 (gap C25): what the v2
    /// pane's debug strip showed of its frame (id, revision, pages), as this
    /// session has it. The host's pid and how many hosts the session started,
    /// the newest compile a DONE finished (a cancelled one does not count),
    /// the layout revision, the pages and how many are stale, the DONEs
    /// received, the keystroke-to-screen median the status bar shows
    /// (ShellChrome: the last 200 samples) and the environment note. The
    /// counters marked `@ObservationIgnored` change with observed ones (a
    /// DONE sets the status note), so the pane's line follows them.
    var debugLine: String {
        func plural(_ n: Int, _ word: String) -> String { "\(n) \(word)\(n == 1 ? "" : "s")" }
        var parts = [hostPID.map { "host pid \($0)" } ?? "no host", plural(hostStarts, "host start")]
        parts.append(lastDoneID > 0 ? "last DONE #\(lastDoneID)" : "no DONE yet")
        parts.append("layout revision \(layoutRevision)")
        parts.append("\(plural(pageCount, "page")), \(staleCount) stale")
        parts.append("\(plural(doneCount, "DONE")) received")
        let ms = latency.samples.suffix(200).map(\.ms)
        if !ms.isEmpty {
            parts.append(String(format: "keystroke to screen median %.0f ms over %d", ms.sorted()[ms.count / 2], ms.count))
        }
        if !environmentNote.isEmpty { parts.append(environmentNote) }
        return parts.joined(separator: " · ")
    }
    @ObservationIgnored private var connection: DL3Connection?
    @ObservationIgnored private var nextID = 1
    /// The text of each document as the host's copy of the project holds it.
    @ObservationIgnored private var sentTexts: [String: String] = [:]
    /// UTF-8 length of what the host holds per document (fast path).
    @ObservationIgnored private var hostBytes: [String: Int] = [:]
    /// Documents the fast path edited since the model last stored their text.
    @ObservationIgnored private var fastPending: Set<String> = []
    /// The fast path's anchors per document (`EngineV3Edits.Anchors`), and
    /// its last splice's check for the slow path: the byte offset, and the
    /// UTF-8 of the text from up to 16 UTF-16 units before the edit through
    /// what it inserted.
    @ObservationIgnored private var fastAnchors: [String: EngineV3Edits.Anchors] = [:]
    @ObservationIgnored private var fastCheck: [String: (offset: Int, bytes: [UInt8])] = [:]
    @ObservationIgnored private var project: EngineV3Mirror?
    /// The page the view shows (sent as `viewport`).
    @ObservationIgnored var visiblePage = 0
    @ObservationIgnored let latency = EngineV3Latency()
    /// What the reader thread may rasterise immediately (pages on screen, at which scale).
    @ObservationIgnored let rasterPlan = EngineV3RasterPlan()
    /// The event monitors and the text-storage observer `start` installs:
    /// removed in `stop()`, and in deinit for a session released without a
    /// stop (AppKit and NotificationCenter keep them, and run their blocks on
    /// every key, click and edit, until removed). nonisolated(unsafe): deinit
    /// is nonisolated; the session is released on main.
    @ObservationIgnored nonisolated(unsafe) private(set) var keyMonitor: Any?
    @ObservationIgnored nonisolated(unsafe) private(set) var storageObserver: NSObjectProtocol?
    @ObservationIgnored nonisolated(unsafe) private(set) var clickMonitor: Any?
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
    /// Removed in `stop()` and deinit.
    @ObservationIgnored nonisolated(unsafe) private var fontSmoothingObserver: NSObjectProtocol?
    /// Settings > Performance changed (PerformanceMode.swift): the host is
    /// told (`PROFILE`). Removed in deinit.
    @ObservationIgnored nonisolated(unsafe) private var performanceObserver: NSObjectProtocol?

    /// `smoothFonts` nil: follow the Settings preference (the app's
    /// session); a value: fixed at it (tests).
    init(smoothFonts: Bool? = nil) {
        self.smoothFonts = smoothFonts ?? PreviewFontSmoothing.enabled
        rasterPlan.smoothFonts = self.smoothFonts
        delivery = EngineV3Delivery(EngineV3WeakRef(self))
        if smoothFonts == nil {
            fontSmoothingObserver = PreviewFontSmoothing.observe { [weak self] on in self?.smoothFonts = on }
        }
        performanceObserver = NotificationCenter.default.addObserver(forName: PerformanceMode.changed, object: nil, queue: .main) { [weak self] note in
            guard let mode = (note.userInfo?["mode"] as? String).flatMap(PerformanceMode.init(rawValue:)) else { return }
            MainActor.assumeIsolated { self?.performanceModeChanged(mode) }
        }
    }

    /// Tell the host the new performance mode (spec §6.9): it applies it
    /// between compiles. A host without `profile-v1` keeps its own.
    private func performanceModeChanged(_ mode: PerformanceMode) {
        guard let c = connection, c.offersProfiles else { return }
        do { try c.setProfile(mode.hostProfile); log("performance mode: \(mode.hostProfile)") } catch { log("PROFILE: \(error)") }
    }

    deinit {
        if let fontSmoothingObserver { NotificationCenter.default.removeObserver(fontSmoothingObserver) }
        if let performanceObserver { NotificationCenter.default.removeObserver(performanceObserver) }
        // A session released without `stop()` (its window's model went away):
        // nothing it installed may outlive it. Its host ends with it (the
        // process object terminates it in deinit, and the connection closes).
        if let storageObserver { NotificationCenter.default.removeObserver(storageObserver) }
        if let keyMonitor { NSEvent.removeMonitor(keyMonitor) }
        if let clickMonitor { NSEvent.removeMonitor(clickMonitor) }
        stallTimer?.invalidate()
    }

    // MARK: lifecycle

    /// Starts the host (once) and compiles the model's documents.
    func start(model: ShellModel) {
        self.model = model
        stopping = false
        EngineV3MemoryPressure.shared.register(self)
        PerformanceAdvisor.shared.start() // suggests Low Memory under memory pressure (PerformanceMode.swift)
        if NSWorkspace.shared.isVoiceOverEnabled { EngineV3GlyphText.warmUp() } // VoiceOver's page text (EngineV3Accessibility.swift), off main; else loaded on first use

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
        if storageObserver == nil {
            // One observer, in this order: the caret windows follow the edit
            // first, then the fast path may send it as a compile, whose new
            // window starts from the edited text (two observers would leave
            // the order to NotificationCenter).
            storageObserver = NotificationCenter.default.addObserver(forName: NSTextStorage.didProcessEditingNotification, object: nil, queue: nil) { [weak self] note in
                guard let storage = note.object as? NSTextStorage else { return }
                MainActor.assumeIsolated { self?.textStorageEdited(storage) }
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

    /// The document's mode (EngineV3Mode.swift): the environment, the
    /// manifest's `[project] mode`, the main file's `% !TEX program` line.
    /// `activeText`: the active document's newest text (an edit the model
    /// has not stored yet).
    static func mode(_ model: ShellModel?, main: String, activeText: String? = nil) -> (mode: EngineV3Mode, source: String) {
        let text = model.flatMap { m -> String? in
            let path = m.documents.contains { $0.path == main } ? main : m.activePath
            if let activeText, path == m.activePath { return activeText }
            return m.documents.first { $0.path == path }?.text
        }
        return EngineV3Mode.resolve(environment: ProcessInfo.processInfo.environment["FLASHTEX_MODE"],
                                    manifest: model?.manifest.snapshot?.manifest.project.mode, mainText: text)
    }

    /// Relaunches the host when the document's mode no longer matches the
    /// running host's (a `% !TEX program` line or `[project] mode` changed,
    /// another project opened). Not counted as a crash.
    @discardableResult
    func relaunchIfModeChanged(model: ShellModel, activeText: String? = nil) -> Bool {
        guard let host else { return false }
        let want = Self.mode(model, main: mainFile, activeText: activeText)
        guard host.mode != want.mode else { return false }
        log("relaunching the host in \(want.mode.rawValue) mode (\(want.source))")
        stopRunningCompile(statusNote: "restarting the engine in \(want.mode == .unicode ? "Unicode" : "Classic") mode", firstError: nil)
        stalledTexts = nil
        return true
    }

    /// The running host's mode (tests and evidence).
    var hostMode: EngineV3Mode? { host?.mode }

    private func launchHost() {
        let mode = Self.mode(model, main: mainFile)
        guard let exe = EngineV3.locateHost(mode: mode.mode) else {
            phase = .failed(mode.mode == .unicode
                ? "flashtex-host-unicode not found. Build it (cd crates/flashtex-xetex && cargo build --release --bin flashtex-host-unicode) or set FLASHTEX_HOST_UNICODE / the \(EngineV3.hostPathKey).unicode default."
                : "flashtex-host not found. Build it (cargo build --release -p flashtex-engine --bin flashtex-host) or set FLASHTEX_HOST / the \(EngineV3.hostPathKey) default.")
            return
        }
        // No TeX Live and a bundle not downloaded yet: ask first (the host
        // would download it as it starts). The answer starts the host, or
        // falls back to the previous engine.
        if case .ask(let config) = EngineV3Bundle.currentGate() {
            bundleConsentConfig = config
            environmentNote = "Waiting for your answer: download the TeX files?"
            if !bundleConsentShown { bundleConsentShown = true }
            return
        }
        EngineV3HostProcess.killStaleHosts(log: log)
        EngineV3Mirror.removeAbandoned(log: log)
        phase = .starting(since: Date())
        bundleProgressNote = nil
        environmentNote = EngineChoice.texLiveInstalled() || EngineV3Bundle.configured() == nil
            ? "Preparing the \(mode.mode.formatName) format from your TeX Live (the first use builds it; a few seconds)…"
            : "Preparing the \(mode.mode.formatName) format from the TeX files (the first use downloads them and builds it)…"
        log("starting \(exe.path) (\(mode.mode.rawValue) mode: \(mode.source))")
        let ref = EngineV3WeakRef(self)
        do {
            // A Live Share session (or a session copy) compiles in a host
            // launched confined; `compile` relaunches when that changes.
            let h = try EngineV3HostProcess(executable: exe, mode: mode.mode, confineRoots: model.flatMap(Self.confineRoots)) { event in
                EngineV3Session.onMain { ref.value?.hostEvent(event) }
            }
            host = h
            hostStarts += 1
        } catch {
            phase = .failed("could not start \(exe.lastPathComponent): \(error.localizedDescription)")
        }
    }

    /// The consent sheet's answer: download (and start the host), or not
    /// now (the previous engine typesets, and the window says why).
    func answerBundleConsent(_ download: Bool) {
        // The answer is for the bundle the sheet showed (its digest and source).
        if let config = bundleConsentConfig ?? EngineV3Bundle.configured() { EngineV3Bundle.setConsent(download, for: config) }
        bundleConsentShown = false
        log("TeX files download: \(download ? "allowed" : "not now")")
        if download {
            if phase == .idle, !stopping { launchHost() }
        } else {
            environmentNote = ""
            model?.engineV3BundleDeclined()
        }
    }

    func stop() {
        stopping = true
        stallTimer?.invalidate()
        stallTimer = nil
        stalledTexts = nil
        typesettingID = nil; toolsCycleID = nil; unanswered = [:]; explicitID = nil; explicitOnConnect = false
        if compileRunningLong { compileRunningLong = false }
        heldDuringExport = false
        finishExport(.failure(.failed("the preview engine was stopped")))
        // A pending page-snapshot save would write after the session (and,
        // in a test, after its cache setting) is gone.
        snapshotSave?.cancel()
        snapshotSave = nil
        projectWatcher = nil
        if editsWaiting { editsWaiting = false } // the next start sends every document again
        view?.dropAllTiles() // queued tile jobs skip undrawn; kept page rasters are freed
        if let model, !model.engineV3Diagnostics.isEmpty { model.engineV3Diagnostics = [] }
        connection?.bye()
        connection = nil
        host?.terminate()
        host = nil
        phase = .idle
        EngineV3MemoryPressure.shared.unregister(self)
        sentTexts = [:]; hostBytes = [:]; fastPending = []; fastAnchors = [:]; fastCheck = [:]; compiledTexts = [:]; fastSentID = [:]; caretWindows = [:]; caretWindow = nil
        dropPages()
        layoutRevision &+= 1
        if let keyMonitor { NSEvent.removeMonitor(keyMonitor) }
        keyMonitor = nil
        if let storageObserver { NotificationCenter.default.removeObserver(storageObserver) }
        storageObserver = nil
        if let fontSmoothingObserver { NotificationCenter.default.removeObserver(fontSmoothingObserver) }
        fontSmoothingObserver = nil
        if let clickMonitor { NSEvent.removeMonitor(clickMonitor) }
        clickMonitor = nil
    }

    /// The host died or the connection broke: start another (bounded), keep
    /// the pages on screen as stale until the new host sends them.
    // MARK: the stall bound (gap A15)

    /// TeX stops a superseded or cancelled compile only at a page or segment
    /// checkpoint, so an endless loop before one (`\def\x{\x}\x`) holds the
    /// engine for good and every later edit queues behind it. pdflatex would
    /// run until killed; the old engine's path had a 10 s bound. Here
    /// (`EngineV3StallBound`): while the host is typesetting a compile (its
    /// STARTED came, its DONE not yet) and runs no external tool, and sends
    /// nothing for the bound (30 s after a keystroke, 5 min for ⌘B), the host
    /// is stopped and started again, the pane says why, and the text that
    /// looped is not compiled again until an edit or ⌘B. The pane also offers
    /// Stop Compile while a compile runs long.
    @ObservationIgnored private var lastHostActivityNs: UInt64 = 0
    /// Invalidated in `stop()` and deinit (the run loop keeps a repeating timer until then).
    @ObservationIgnored nonisolated(unsafe) private(set) var stallTimer: Timer?
    /// After a stop: the texts that looped. The restarted host's opening
    /// compile is held while the editor still has them (an edit or ⌘B compiles).
    @ObservationIgnored private var stalledTexts: [String: String]?
    /// The compile the host is typesetting: its STARTED came, its DONE not yet.
    @ObservationIgnored private(set) var typesettingID: Int?
    /// The compile id of the tool cycle running: from its first TOOL `run`
    /// to its `settled`, or until a newer client compile starts (a cycle a
    /// newer compile superseded may never settle), or the host restarts.
    @ObservationIgnored private(set) var toolsCycleID: Int?
    /// The newest client compile (not a tools follow-up) whose STARTED came.
    @ObservationIgnored private var newestClientStartedID = 0
    /// Bibtex, biber or makeindex, and the compiles they cause, run (silent phases are normal).
    var toolsRunning: Bool { toolsCycleID != nil }
    /// Send times of the compiles not yet answered by a DONE (Stop Compile:
    /// measured from the oldest; typing's superseded compiles are answered).
    @ObservationIgnored private var unanswered: [Int: UInt64] = [:]
    /// `PROGRESS` frames received (tests, evidence). `FLASHTEX_V3_NO_PROGRESS=1`
    /// (tests) does not accept `progress-v1`, as an older app.
    @ObservationIgnored private(set) var progressFrames = 0
    /// The last ⌘B compile sent: the long bound applies until a DONE reaches it.
    @ObservationIgnored private var explicitID: Int?
    /// A compile has run for more than 2 s: the pane shows Stop Compile.
    private(set) var compileRunningLong = false
    /// ⌘B pressed while the host restarts after a stop: its first compile is that ⌘B.
    @ObservationIgnored private var explicitOnConnect = false

    private func armStallBound() {
        guard stallTimer == nil else { return }
        let t = Timer(timeInterval: 1, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.checkStall() } }
        RunLoop.main.add(t, forMode: .common)
        stallTimer = t
    }

    /// The explicit (⌘B) bound applies: a ⌘B compile is still out.
    var explicitCompileOut: Bool { explicitID.map { $0 > lastDoneID } ?? false }

    func checkStall(nowNs: UInt64 = MonotonicClock.nowNs()) {
        let long = compiling && unanswered.values.min().map { Double(nowNs &- $0) / 1e9 > 2 } == true
        if compileRunningLong != long { compileRunningLong = long }
        guard connection != nil else { return }
        let explicit = explicitCompileOut
        guard EngineV3StallBound.shouldStop(typesetting: compiling && typesettingID != nil, toolsRunning: toolsRunning,
                                            exporting: exportRunning, explicit: explicit,
                                            silentSeconds: Double(nowNs &- lastHostActivityNs) / 1e9) else { return }
        let s = EngineV3StallBound.describe(EngineV3StallBound.seconds(explicit: explicit))
        log("no output from the host for \(s) while typesetting: stopping it (an endless loop?)")
        stopRunningCompile(statusNote: "stopped · no output for \(s) (an endless loop?) · ⌘B compiles with a \(EngineV3StallBound.describe(EngineV3StallBound.seconds(explicit: true))) limit",
                           firstError: "TeX did not finish: no output for \(s) (an endless loop?). The compile was stopped; ⌘B compiles again with a \(EngineV3StallBound.describe(EngineV3StallBound.seconds(explicit: true))) limit.")
    }

    /// Stop Compile (the pane's button, Compile ▸ Stop Compile, ⌘.): the user ends a compile that runs too long.
    func stopCompile() {
        guard compiling else { return }
        log("compile stopped by the user")
        stopRunningCompile(statusNote: "stopped · ⌘B to compile again", firstError: nil)
    }

    /// Ends the running compile by stopping the host (TeX cannot be
    /// interrupted between checkpoints) and starts a fresh one, not counted
    /// as a crash; the texts it ran on wait for an edit or ⌘B.
    private func stopRunningCompile(statusNote note: String, firstError error: String?) {
        stalledTexts = model.map { Dictionary($0.documents.map { ($0.path, $0.text) }, uniquingKeysWith: { a, _ in a }) }
        compiling = false
        compileRunningLong = false
        typesettingID = nil
        toolsCycleID = nil
        unanswered = [:]
        explicitID = nil
        statusNote = note
        firstError = error
        heldDuringExport = false
        finishExport(.failure(.failed("the compile was stopped"))) // an export waiting for this compile must not hang
        connection?.bye()
        connection = nil
        host?.terminate()
        host = nil
        sentTexts = [:]; hostBytes = [:]; fastPending = []; fastAnchors = [:]; fastCheck = [:]; compiledTexts = [:]; fastSentID = [:]; caretWindows = [:]; caretWindow = nil
        guard !stopping, phase != .idle else { return }
        phase = .idle
        launchHost()
    }

    private func restart(_ why: String) {
        heldDuringExport = false // the restart sends every document again
        typesettingID = nil; toolsCycleID = nil; unanswered = [:]; newestClientStartedID = 0 // the stall bound's view of the old host
        finishExport(.failure(.failed("the preview engine stopped (\(why))")))
        connection = nil
        host?.terminate()
        host = nil
        guard !stopping, phase != .idle else { return }
        restarts = restarts.filter { $0.timeIntervalSinceNow > -60 } + [Date()]
        guard restarts.count <= 3 else {
            phase = .failed("The preview engine stopped repeatedly (\(why)). Compile (⌘B) to restart it.")
            return
        }
        log("restarting the host: \(why)")
        sentTexts = [:]; hostBytes = [:]; fastPending = []; fastAnchors = [:]; fastCheck = [:]; compiledTexts = [:]; fastSentID = [:]; caretWindows = [:]; caretWindow = nil
        markStale(Set(pages.keys))
        phase = .idle
        launchHost()
    }

    private func hostEvent(_ e: EngineV3HostProcess.Event) {
        switch e {
        case .line(let l):
            log(l)
        case .bundleProgress(let what, let name, let done, let total):
            let note = EngineV3Bundle.progressText(what: what, name: name, done: done, total: total)
            if bundleProgressNote != note { bundleProgressNote = note }
        case .prepared(let j):
            bundleProgressNote = nil
            var texlive = j["texlive"]?.string ?? "no TeX Live found"
            if j["texlive"]?.string == nil, let b = j["bundle"], b["active"]?.bool == true, let d = b["digest"]?.string {
                texlive = "none; TeX files from the bundle \(d.prefix(12))…"
            }
            let formats = (j["formats"]?.array ?? []).map { f in
                "\(f["name"]?.string ?? "?") \(f["status"]?.string ?? "?")\(f["error"]?.string.map { ": " + $0 } ?? "")"
            }.joined(separator: ", ")
            environmentNote = "TeX Live: \(texlive) — format \(formats)"
            let failed = (j["formats"]?.array ?? []).contains(where: { $0["status"]?.string == "failed" })
            if failed {
                phase = .failed("The pdfLaTeX format could not be prepared: \(formats)")
            }
            // No TeX Live and no format: the document falls back to the
            // previous engine, and the window says why (EngineChoice.swift).
            if EngineChoice.hostLacksTeXLive(DL3JSONView(texlive: j["texlive"]?.string, formatFailed: failed)) {
                model?.engineV3HostLacksTeXLive()
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
        let profile = PerformanceMode.current.hostProfile // Settings > Performance (spec §6.9)
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let c = try DL3Connection(socketPath: socket, client: "FlashTeX (engine-v3 preview)", accept: [DL3Diag.capability] + (ProcessInfo.processInfo.environment["FLASHTEX_V3_NO_PROGRESS"] == "1" ? [] : [DL3.progressCapability]), profile: profile)
                let reader = EngineV3Reader(cache: .shared, plan: plan)
                let delivery = EngineV3Delivery(ref)
                c.start(onTimedEvent: { ev, timing in
                    guard let out = reader.handle(ev, timing: timing) else { return }
                    delivery.post(out, connection: c)
                }, onClose: { err in
                    EngineV3Session.onMain { ref.value?.closed(err, connection: c) }
                })
                EngineV3Session.onMain {
                    guard let self = ref.value else { c.bye(); return }
                    self.adopt(c)
                    if case .failed = self.phase {} else { self.phase = .ready }
                    self.log("connected: \(c.hello["server"]?.string ?? "?"), \(c.hello["engine"]?.string ?? "?")")
                    if let model = self.model {
                        let explicit = self.explicitOnConnect
                        self.explicitOnConnect = false
                        self.compile(model: model, reason: explicit ? "explicit" : "open")
                    }
                }
            } catch {
                EngineV3Session.onMain { ref.value?.restart("could not connect: \(error)") }
            }
        }
    }

    /// The connected host and what it offers (HELLO `capabilities`).
    private func adopt(_ c: DL3Connection) {
        connection = c
        let caps = c.hello["capabilities"]?.array ?? []
        hostOffersDiagV1 = caps.contains(.string(DL3Diag.capability))
        hostHonoursHaltOnError = caps.contains(.string(DL3CompileRequest.haltOnErrorCapability))
        hostOffersTrim = caps.contains(.string(DL3.trimCapability))
    }

    /// Tests: a connection to a stand-in host, adopted as `connect` does.
    func adoptForTesting(_ c: DL3Connection) { adopt(c) }

    private func closed(_ err: DL3Error?, connection c: DL3Connection) {
        guard c === connection else { return }
        log("connection closed\(err.map { ": \($0)" } ?? "")")
        restart("the connection closed\(err.map { ": \($0)" } ?? "")")
    }

    // MARK: edits → COMPILE

    /// The editor's text storage changed: the caret windows follow the edit,
    /// then the fast path may send it (`storageObserver`, one observer so
    /// the order is this one).
    private func textStorageEdited(_ storage: NSTextStorage) {
        caretStorageEdited(storage)
        if fastEdits { storageEdited(storage) }
    }

    /// An edit in this window's editor: every caret window follows it
    /// (EngineV3CaretPlace.Window), whatever path the edit then takes. The
    /// whole text replaced (`tv.string = …`: a reload, a restored snapshot,
    /// another document shown) leaves no edit to follow: every window is
    /// invalid, and the texts are compared until the next compile's window.
    private func caretStorageEdited(_ storage: NSTextStorage) {
        guard storage.editedMask.contains(.editedCharacters), let model,
              let tv = storage.layoutManagers.first?.textContainers.first?.textView,
              tv.accessibilityLabel() == "LaTeX source", let w = tv.window, w === view?.window else { return }
        caretStorage = storage
        let r = storage.editedRange, delta = storage.changeInLength, length = storage.length, path = model.activePath
        let whole = r.location == 0 && r.length == length
        // A replaced text is not the editor's own edit: the model's next
        // text is taken as from outside it (safe: compared, never drifting).
        // Nor is an input method's marked text, which the model takes only
        // at the commit (its own storage edit, without marked text): a
        // model text arriving mid-composition comes from outside.
        if tv.hasMarkedText() { caretEditorEdited = false } else if !whole { caretEditorEdited = true }
        func follow(_ w: inout EngineV3CaretPlace.Window) {
            if w.path == path, !whole { w.edit(newRange: r, delta: delta, length: length) } else { w.invalid = true }
        }
        for k in Array(caretWindows.keys) { follow(&caretWindows[k]!) }
        if caretWindow != nil { follow(&caretWindow!) }
    }

    /// The model takes a new text for the active document (`textChanged`).
    /// From the editor (its storage was edited first), the editor and the
    /// model agree again. From outside it (nothing edited the storage), the
    /// editor shows another text until it is replaced: the active
    /// document's windows are invalid, and so are those of compiles sent
    /// before the editor next gives the model a text.
    private func caretModelTextChanged(model: ShellModel, text: String) {
        defer { caretEditorEdited = false }
        if caretEditorEdited { caretEditorOutOfStep = false; return }
        let path = model.activePath
        // The same bytes again (the model ignores them): nothing changed.
        if let doc = model.documents.first(where: { $0.path == path }), doc.text.sameBytes(as: text) { return }
        caretEditorOutOfStep = true
        for k in Array(caretWindows.keys) where caretWindows[k]!.path == path { caretWindows[k]!.invalid = true }
        if caretWindow?.path == path { caretWindow!.invalid = true }
    }

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
        // Only this window's LaTeX editor: the notification comes for every
        // text storage of the app (a search field, another window's editor),
        // none of which holds the text the host has.
        guard storage.editedMask.contains(.editedCharacters),
              let tv = storage.layoutManagers.first?.textContainers.first?.textView,
              tv.accessibilityLabel() == "LaTeX source", let w = tv.window, w === view?.window else { return }
        // A change of this editor's text the fast path does not send leaves
        // its anchors stale.
        var sent = false
        defer { if !sent { fastAnchors = [:] } }
        guard phase == .ready, connection != nil, let model, model.engineV3Enabled, model.autoCompile, !exportRunning,
              !tv.hasMarkedText() else { return }
        // Live Share: the fast path never sends to a host whose confinement
        // no longer matches (a session just started); the slow path, a few
        // milliseconds later, relaunches it first.
        guard fastPathAllowed(model: model) else { return }
        let typing = nextKeystrokeNs != nil || NSApp.currentEvent?.type == .keyDown
        let path = model.activePath
        guard typing, let base = hostBytes[path], sentTexts[path] != nil || fastPending.contains(path) else { return }
        guard project?.exists == true else { return } // the slow path re-creates it
        let r = storage.editedRange, delta = storage.changeInLength
        let oldLength = r.length - delta
        guard r.location != NSNotFound, r.length <= 4096, oldLength >= 0, oldLength <= 4096 else { return }
        let now = MonotonicClock.nowNs()
        let text = storage.mutableString
        let insert = text.substring(with: r)
        let (prefix, delete, total, anchors) = EngineV3Edits.fastSplice(text: text, edited: r, delta: delta, base: base,
                                                                        anchors: fastAnchors[path])
        guard delete >= 0, prefix + delete <= base else { return }
        sent = true
        fastAnchors = [path: anchors]
        // What the slow path checks the model's text against (bytes, not a
        // count: a wrong offset with the right total would go unseen).
        let back = r.location - max(0, r.location - 16)
        var c0 = r.location - back
        if c0 > 0 { c0 = text.rangeOfComposedCharacterSequence(at: c0).location }
        let around = text.substring(with: NSRange(location: c0, length: NSMaxRange(r) - c0))
        let aroundBytes = Array(around.utf8)
        fastCheck[path] = (prefix + insert.utf8.count - aroundBytes.count, aroundBytes)
        let key = consumeKeystroke(now: now)
        var req = request(model: model)
        req.edits = [DL3CompileRequest.Edit(path: path, offset: prefix, delete: delete, insert: insert)]
        hostBytes[path] = total
        fastPending.insert(path)
        send(req, keystrokeNs: key, editNs: now, path: path)
        fastSentID[path] = req.id
        fastEditsSent &+= 1
    }

    /// The roots a host compiling `model`'s project must be confined to, or
    /// nil when it need not be (no Live Share session, not a session copy).
    static func confineRoots(_ model: ShellModel) -> [String]? {
        guard model.liveShare.forcesPinnedCompile(root: model.project.projectRoot) else { return nil }
        return [model.project.projectRoot?.resolvingSymlinksInPath().path].compactMap { $0 }
    }

    /// Relaunches the host when its confinement no longer matches the
    /// project's (a session started or ended, another project opened);
    /// leaving confinement clears the copy's output folder, so nothing a
    /// session's text wrote (an `.aux` with `\write18` in it, say) is read
    /// by the next, possibly trusted, compile. Not counted as a crash.
    @discardableResult
    func relaunchIfConfinementChanged(model: ShellModel) -> Bool {
        guard let host else { return false }
        let want = Self.confineRoots(model)
        guard host.confineRoots != want else { return false }
        log("relaunching the host \(want == nil ? "unconfined" : "confined") (Live Share)")
        if host.confined, want == nil { clearOutputAfterSession() }
        stopRunningCompile(statusNote: "restarting the engine for Live Share", firstError: nil)
        stalledTexts = nil
        return true
    }

    /// Empties the project copy's output folder (`.aux`, `.toc`, ...).
    func clearOutputAfterSession() {
        guard let out = project?.output else { return }
        let fm = FileManager.default
        for name in (try? fm.contentsOfDirectory(atPath: out.path)) ?? [] {
            try? fm.removeItem(at: out.appendingPathComponent(name))
        }
        outputClearedForSession &+= 1
    }

    /// Times `clearOutputAfterSession` ran (tests and evidence).
    private(set) var outputClearedForSession = 0

    /// The fast path sends only to a host confined exactly as `model`'s
    /// project needs (the race of a keystroke right after a session starts).
    func fastPathAllowed(model: ShellModel) -> Bool {
        guard let host else { return false }
        return host.confineRoots == Self.confineRoots(model)
    }

    /// The project copy's output folder (tests).
    var outputDirectory: URL? { project?.output }

    /// Whether the running host is confined (tests and evidence).
    var hostConfineRoots: [String]?? { host.map(\.confineRoots) }

    /// The model's new text holds the fast path's last splice where it sent
    /// it (`fastCheck`), byte for byte.
    nonisolated static func fastCheckHolds(_ check: (offset: Int, bytes: [UInt8])?, _ text: String) -> Bool {
        guard let check else { return true }
        let u = text.utf8
        guard check.offset >= 0, check.offset + check.bytes.count <= u.count else { return false }
        if let ok = u.withContiguousStorageIfAvailable({ b in b[check.offset ..< check.offset + check.bytes.count].elementsEqual(check.bytes) }) {
            return ok
        }
        let from = u.index(u.startIndex, offsetBy: check.offset)
        return u[from...].prefix(check.bytes.count).elementsEqual(check.bytes)
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
        if let activeText { caretModelTextChanged(model: model, text: activeText) }
        guard phase == .ready, connection != nil else { return }
        let now = MonotonicClock.nowNs()
        let path = model.activePath
        // An edit that changes the mode (a `% !TEX program` line) goes to
        // `compile`, which relaunches the host, whatever the fast path sent.
        if let activeText, let host, host.mode != Self.mode(model, main: mainFile, activeText: activeText).mode {
            compile(model: model, reason: "mode", activeText: activeText)
            return
        }
        if let activeText, fastPending.contains(path) {
            fastPending.remove(path)
            if activeText.utf8.count == hostBytes[path], Self.fastCheckHolds(fastCheck[path], activeText) {
                sentTexts[path] = activeText
                // The fast path's compiles (and any sent since) read this text.
                if let id = fastSentID[path] {
                    for k in compiledTexts.keys where k >= id { compiledTexts[k]?[path] = activeText }
                }
                return
            }
            // Out of step: resend the whole buffer.
            fastAnchors = [:]
            fastResyncs &+= 1
            log("fast path out of step for \(path) (\(hostBytes[path] ?? -1) bytes held, \(activeText.utf8.count) now); resending it")
            sentTexts[path] = nil
        }
        // Auto-compile off: the host keeps what it last compiled; ⌘B (or
        // turning auto-compile on) sends the difference.
        guard model.autoCompile else {
            if !editsWaiting { editsWaiting = true }
            return
        }
        let key = keystrokeNs ?? consumeKeystroke(now: now)
        compile(model: model, reason: "edit", keystrokeNs: key, activeText: activeText, editNs: now)
    }

    /// ⌘B, or an outside change to a file the project reads: compile now.
    /// The host checks every file the last run read, so a COMPILE with no
    /// edits still picks up a changed `\input` file. A host that stopped
    /// (the restart limit, a missing format) is started again, with a fresh
    /// restart budget; one that is starting compiles once it is ready.
    func compileNow(model: ShellModel, reason: String = "explicit") {
        guard model.engineV3Enabled else { return }
        switch phase {
        case .ready: compile(model: model, reason: reason)
        case .starting:
            // After a stop: the restarted host's first compile is this ⌘B (long bound).
            if stalledTexts != nil, reason == "explicit" { stalledTexts = nil; explicitOnConnect = true }
            return
        case .idle, .failed:
            restarts = []
            start(model: model)
        }
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
        projectChanges += 1
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

    /// Opens handled (tests: an open the previous engine typesets reaches none).
    @ObservationIgnored private(set) var projectChanges = 0

    /// The `[project] texinputs` links the last walk made (EngineV3Mirror.linkTexInputs).
    @ObservationIgnored private(set) var texInputLinksApplied: [EngineV3Mirror.TexInputLink] = []

    /// flashtex.toml was read again: when its `texinputs` files changed, the
    /// copy's links follow before the next compile (a walk), and it compiles.
    /// New `[packages] pin` or `path` entries are resolved first (the
    /// compile follows that, `packagesChanged`).
    func manifestChanged(model: ShellModel) {
        guard model.engineV3Enabled, let project, project.source == model.project.projectRoot else { return }
        if model.projectPackages.prepareForEngineV3() { return }
        let held = heldForManifest
        heldForManifest = false
        guard held || model.manifest.texInputLinks != texInputLinksApplied
                || model.projectPackages.engineV3Documents().map(\.path) != packagePathsApplied else { return }
        compileNow(model: model, reason: "manifest")
    }

    /// A compile waited for the project's manifest to be read (its pins and
    /// libraries decide what the first compile reads): the read compiles.
    @ObservationIgnored private var heldForManifest = false

    /// The resolved package files (`packages/<name>/<file>`) the last walk linked.
    @ObservationIgnored private var packagePathsApplied: [String] = []

    /// The resolved packages changed (a fetch, the cache, the manifest's
    /// pins and libraries) or their local resolution ended: the copy's
    /// links follow (a walk) and it compiles.
    func packagesChanged(model: ShellModel) {
        guard model.engineV3Enabled, let project, project.source == model.project.projectRoot else { return }
        compileNow(model: model, reason: "packages")
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

    /// Files appeared in the project outside the editor (pasted or dropped
    /// images, PasteImage.swift; `paths` project-relative): link just those
    /// into the copy now, so the compile the paste's own edit triggers (an
    /// "edit" compile, which does not walk the directory) already finds them.
    /// No directory walk on main.
    func projectFilesChanged(model: ShellModel, paths: [String]) {
        guard model.engineV3Enabled, let project, project.source == model.project.projectRoot else { return }
        let open = Set(model.documents.map(\.path))
        for path in paths where !open.contains(path) { project.link(path) }
        inputsAtSync = nil // the inputs are unknown until the next walk
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
                // flashtex.toml's texinputs, read on main (the manifest is
                // read after the open's first walk starts): a few links.
                // Then the resolved packages (ProjectPackages.swift), written
                // into the copy and linked by name after the texinputs: the
                // manifest's pins and libraries are resolved first (the
                // compile below is held until then).
                let packages = model.projectPackages
                _ = packages.prepareForEngineV3()
                let links = model.manifest.texInputLinks
                let named = Set(links.map(\.name)) // a texinputs file of the same name wins
                let packageLinks = project.materializePackages(packages.engineV3Documents()).filter { !named.contains($0.name) }
                project.linkTexInputs(links + packageLinks, except: editorPaths)
                self.texInputLinksApplied = links
                self.packagePathsApplied = packages.engineV3Documents().map(\.path)
                // Those outside the root are inputs too: an outside change
                // to one invalidates the stored pages (inside ones are walked).
                self.inputsAtSync = walk.inputs.map { EngineV3Snapshot.withExternal($0, paths: links.compactMap(\.external)) }
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
        req.format = host?.mode.format ?? EngineV3Mode.classic.format
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
        // Live Share: a session copy, or any project while a session runs,
        // compiles with the session pins (proposal §6.2), trusted or not.
        let pinned = model.liveShare.forcesPinnedCompile(root: model.project.projectRoot)
        req.shellEscape = pinned ? "off" : EngineV3Trust.shellEscape(trusted: projectTrusted && !trustPending)
        // Protocol 3.2: bibtex, biber and makeindex run in the host as latexmk
        // would, only for a trusted project (DESIGN.md §4.5, owner 9A); an
        // untrusted one runs no external program.
        req.externalTools = !pinned && projectTrusted && !trustPending ? "auto" : "off"
        // Strict mode (EngineV3ErrorPolicy): TeX stops at the first error
        // (an older host ignores the field: `strictModeIgnored`).
        req.haltOnError = errorMode == .strict
        return req
    }

    /// The texts each outstanding compile read (by id): what its diagnostics'
    /// line numbers refer to, and the baseline the editor marks rebase from
    /// at its DONE, however the editor changed meanwhile. Dropped at DONE.
    @ObservationIgnored private var compiledTexts: [Int: [String: String]] = [:]
    /// The fast path's last compile per path: its text is recorded by the
    /// slow path (`textChanged`) right after.
    @ObservationIgnored private var fastSentID: [String: Int] = [:]

    /// The texts compile `id` read (open documents not sent are as they are now).
    func textsCompiled(by id: Int, model: ShellModel) -> [String: String] {
        var texts = Dictionary(model.documents.map { ($0.path, $0.text) }, uniquingKeysWith: { a, _ in a })
        if let sent = compiledTexts[id] { texts.merge(sent) { _, s in s } }
        return texts
    }

    /// Where COMPILEs are encoded and written, and the first-sight files of
    /// the project copy written before them (one serial queue: a file is on
    /// disk before the COMPILE that reads it). Encoding a COMPILE that
    /// carries a whole 4 MB buffer and writing it to the socket took tens of
    /// milliseconds of main thread (APP-EDITOR-INSTANT).
    static let sendQueue = DispatchQueue(label: "flashtex.engine-v3.send", qos: .userInteractive)

    /// Writes `req` to `c` on the send queue; a failed write restarts the
    /// host if `c` is still the connection.
    private func write(_ req: DL3CompileRequest, to c: DL3Connection, failed: @escaping @MainActor (EngineV3Session, Error) -> Void) {
        let ref = EngineV3WeakRef(self)
        Self.sendQueue.async {
            do { try c.compile(req) } catch {
                EngineV3Session.onMain {
                    guard let s = ref.value, s.connection === c else { return }
                    failed(s, error)
                }
            }
        }
    }

    private func send(_ req: DL3CompileRequest, keystrokeNs: UInt64?, editNs: UInt64, path: String, explicit: Bool = false) {
        guard let connection else { return }
        do {
            let probe = MainThreadProbe.begin()
            write(req, to: connection) { s, error in s.restart("could not send: \(error)") }
            MainThreadProbe.end("v3.send", probe)
            compiledTexts[req.id] = sentTexts // copy-on-write: no text is copied
            if let model {
                // The editor's text is what it reads, unless the model's came from outside the editor.
                var w = EngineV3CaretPlace.Window(path: model.activePath)
                w.invalid = caretEditorOutOfStep
                caretWindows[req.id] = w
            }
            lastHostActivityNs = MonotonicClock.nowNs()
            unanswered[req.id] = lastHostActivityNs
            if explicit { explicitID = req.id }
            armStallBound()
            lastSentID = req.id
            if req.externalTools == "auto" { lastToolsAutoID = req.id }
            if !compiling { compiling = true }
            if keystrokeNs != nil { view?.keystroke() }
            if let keystrokeNs { latency.sent(compile: req.id, keystrokeNs: keystrokeNs, editNs: editNs, path: path, at: MonotonicClock.nowNs()) }
        }
    }

    /// `activeText`: the active document's new text when the model has not
    /// stored it yet (the edit hook runs first).
    /// `walked`: the project walk for this compile has just run (startWalk).
    func compile(model: ShellModel, reason: String, keystrokeNs: UInt64? = nil, activeText: String? = nil, editNs: UInt64 = MonotonicClock.nowNs(), walked: Bool = false) {
        guard connection != nil else { return }
        let probe = MainThreadProbe.begin()
        defer { MainThreadProbe.end("v3.compile", probe) }
        // Live Share: a session's text compiles only in a confined host (and
        // a host launched confined serves nothing else). Relaunch, not
        // counted as a crash; the fresh host compiles when it is ready.
        if relaunchIfConfinementChanged(model: model) { return }
        // A Unicode document runs in flashtex-host-unicode (EngineV3Mode).
        if relaunchIfModeChanged(model: model, activeText: activeText) { return }
        // After a stall the text that looped waits for an edit or ⌘B (the stall bound).
        if let held = stalledTexts {
            if reason == "open", held == Dictionary(model.documents.map { ($0.path, $0.text) }, uniquingKeysWith: { a, _ in a }) { return }
            stalledTexts = nil
        }
        // The export reads the project copy's files as they are now, and its
        // frames share the socket: nothing else is sent until it is done.
        if exportRunning {
            heldDuringExport = true
            return
        }
        var docs = model.documents
        if let activeText, let i = docs.firstIndex(where: { $0.path == model.activePath }) { docs[i].text = activeText }
        // The host compiles a copy of the project (EngineV3Mirror): it writes
        // the editor's text to its files, and must never write the user's.
        let projectRoot = model.project.projectRoot
        if project == nil || project?.source != projectRoot || generation != model.projectGeneration {
            // Another project (or file) in this window: a fresh copy, every
            // document sent again as a buffer, the old pages gone.
            var made = false
            if project?.source != projectRoot || project == nil { project = EngineV3Mirror(source: projectRoot, session: serial); made = true }
            // Emptied on the walk queue, before this project's walk (same serial queue): not on main.
            // A copy just taken over keeps its output (its .aux, checked by
            // `takeOver`): the point of taking it over.
            if let p = project { let keep = made && p.takenOver; Self.walkQueue.async { p.clear(keepingOutput: keep) } }
            generation = model.projectGeneration
            sentTexts = [:]; hostBytes = [:]; fastPending = []; fastAnchors = [:]; fastCheck = [:]; compiledTexts = [:]; fastSentID = [:]; caretWindows = [:]; caretWindow = nil; inputsAtSync = nil
            toolDiagnostics = []; texProblems = []; toolNote = nil; toolCycleID = -1
            if model.engineV3ResultStatus != nil { model.engineV3ResultStatus = nil } // another project: no "failed" of the last one's
            dropPages(); layoutRevision &+= 1
            staleChangedNow()
            statusNote = ""; firstError = nil
            showSnapshot(model: model) // this project's stored pages, if still valid
            trustPending = true // decided by the walk below
            projectWatcher = projectRoot.flatMap { root in
                EngineV3ProjectWatcher(root: root) { [weak self] paths in
                    MainActor.assumeIsolated { self?.projectFileSetChanged(paths) }
                }
            }
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
        // project's first COMPILE always carries the decision. The include
        // watchers are armed then too (an outside change to an `\input`
        // file recompiles).
        if !walked, reason != "edit" || trustPending {
            if reason == "edit", walkInFlight { return } // the walk's compile sends this text too
            if reason != "edit" { model.project.armImplicitWatchers() }
            startWalk(model: model, reason: reason, editorPaths: Set(docs.map(\.path)))
            return
        }
        // The manifest's pins and libraries are being resolved (no network):
        // their end compiles with them (`packagesChanged`), not TeX Live's copies.
        if model.projectPackages.engineV3HoldsCompile { return }
        // The project's manifest is not read yet: its read compiles
        // (`manifestChanged`), so the first compile already has its pins.
        // Bounded: after 2 s the compile goes ahead without it.
        if let root = model.project.projectRoot, model.manifest.snapshotRoot != root {
            if !heldForManifest {
                heldForManifest = true
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self, weak model] in
                    guard let self, let model, self.heldForManifest else { return }
                    self.heldForManifest = false
                    self.log("the manifest was not read within 2 s; compiling without it")
                    self.compileNow(model: model, reason: "manifest-timeout")
                }
            }
            return
        }
        heldForManifest = false
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
                // Written on the send queue, before the COMPILE (a 4 MB file is
                // milliseconds of main thread).
                let dst = project.root.appendingPathComponent(doc.path), text = doc.text
                Self.sendQueue.async {
                    try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
                    if (try? FileManager.default.destinationOfSymbolicLink(atPath: dst.path)) != nil { try? FileManager.default.removeItem(at: dst) }
                    try? Data(text.utf8).write(to: dst)
                }
                req.buffers.append((doc.path, doc.text))
            }
            sentTexts[doc.path] = doc.text
            hostBytes[doc.path] = doc.text.utf8.count
            fastAnchors[doc.path] = nil
        }
        if reason == "edit", req.edits.isEmpty, req.buffers.isEmpty { return }
        if editsWaiting { editsWaiting = false }
        send(req, keystrokeNs: keystrokeNs, editNs: editNs, path: model.activePath, explicit: reason == "explicit")
    }

    // MARK: export (Export PDF…, Print…)

    enum ExportFailure: Error, Equatable {
        case refused(String)
        case failed(String)
        case cancelled
    }

    private enum ExportStage {
        /// Waiting for the DONE of the compile that brings the host's copy
        /// up to the editor (id `after` or later), with the engine idle.
        case syncing(after: Int)
        /// The `export: true` compile `id` is running.
        case running(id: Int)
        /// The export failed (its ERROR came after its STARTED) and its
        /// caller has been told, but its DONE is still to come: until then
        /// the reader drops every frame as the export's, so compiles stay held.
        case ending(id: Int)
    }

    /// The `export: true` run itself is out (compiles are held meanwhile).
    var exportRunning: Bool {
        switch exportStage {
        case .running, .ending: true
        case .syncing, nil: false
        }
    }

    /// The export's waiting state, for a test that times out waiting for its run.
    var exportDebugState: String {
        "stage \(String(describing: exportStage)), compiling \(compiling), lastDone \(lastDoneID), lastSent \(lastSentID), "
            + "toolsAuto \(lastToolsAutoID), settled \(lastSettledID), walkInFlight \(walkInFlight), held \(heldDuringExport), "
            + "inbox drains \(delivery?.drains ?? -1)"
    }

    /// Whether Export PDF… and Print… have a document to produce.
    var exportAvailable: Bool { phase == .ready && pageCount > 0 && !exporting }

    /// Why an export cannot start now, or nil.
    func exportRefusal() -> String? {
        switch phase {
        case .ready: break
        case .starting: return "The engine-v3 preview is still starting; export once its first compile is done."
        case .failed(let why): return "The engine-v3 preview stopped (\(why)). Compile (⌘B) to restart it, then export."
        case .idle: return "The engine-v3 preview is not running."
        }
        if connection == nil { return "The engine-v3 preview is reconnecting; try again in a moment." }
        if exportStage != nil { return "An export is already running." }
        if pageCount == 0 { return "Nothing to export: the last compile produced no pages." }
        return nil
    }

    /// Produces the PDF pdflatex would write for the editor's text: the
    /// host's one-shot `export: true` run (protocol §6.3, DESIGN.md §6.3),
    /// compressed, with the resident run's `.aux` (same output directory).
    /// The export reads the project copy's files rather than buffers, so a
    /// compile first brings the copy up to the editor; the export is sent at
    /// that compile's DONE, when the resident engine is idle, and every
    /// compile is held until the export's DONE. `completion` gets the bytes,
    /// read at once (the next preview compile rewrites the same file).
    func export(model: ShellModel, completion: @escaping @MainActor (Result<Data, ExportFailure>) -> Void) {
        if let why = exportRefusal() { completion(.failure(.refused(why))); return }
        exportCompletion = completion
        exporting = true
        // The sync compile is sent after the project walk (off the main
        // thread), so it is known by its id: the next one this session
        // numbers. A failed send restarts the host, which fails the export.
        exportStage = .syncing(after: nextID)
        compile(model: model, reason: "export")
    }

    /// Async form of `export(model:completion:)`.
    func export(model: ShellModel) async -> Result<Data, ExportFailure> {
        await withCheckedContinuation { c in export(model: model) { c.resume(returning: $0) } }
    }

    /// Cancels the export: before its run starts nothing is sent; a running
    /// one is cancelled on the host, which kills its process.
    func cancelExport() {
        switch exportStage {
        case .syncing: finishExport(.failure(.cancelled))
        case .running(let id):
            if let c = connection { Self.sendQueue.async { try? c.cancel(id: id) } } // after the COMPILEs queued before it
        case .ending, nil: break
        }
    }

    private func sendExport(model: ShellModel) {
        guard let connection, project != nil else { finishExport(.failure(.failed("the preview engine is not connected"))); return }
        var req = request(model: model)
        req.export = true
        req.haltOnError = false // the exported PDF is nonstopmode's, as pdflatex writes it
        req.externalTools = "off" // the resident run's cycle already made the .bbl/.ind
        exportStage = .running(id: req.id)
        write(req, to: connection) { s, error in s.finishExport(.failure(.failed("could not send the export: \(error)"))) }
    }

    private func exportDone(_ j: DL3JSON) {
        let doneID = Int(j["id"]?.int ?? -1)
        if case .ending(let id) = exportStage, id == doneID {
            // Its ERROR already failed it: the run is over now, send what waited.
            exportStage = nil
            sendHeld()
            return
        }
        guard case .running(let id) = exportStage, doneID == id else { return }
        let status = j["status"]?.string ?? "?"
        if logDone { log("export DONE \(j)") }
        let pdf = j["pdf"]?.string
        if status == "cancelled" {
            finishExport(.failure(.cancelled))
        } else if let pdf, let data = try? Data(contentsOf: URL(fileURLWithPath: pdf)), !data.isEmpty {
            finishExport(.success(data))
        } else {
            finishExport(.failure(.failed("the engine wrote no PDF (status \(status))\(firstError.map { ": " + $0 } ?? "")")))
        }
        // The export wrote the preview's PDF path (same output directory and
        // job name): pages drawn from the PDF take theirs from the new file.
        loadFallbacks(pdf: pdf)
        sendHeld()
    }

    /// Sends what was held while the export ran (its run is over).
    private func sendHeld() {
        guard heldDuringExport, let model else { return }
        heldDuringExport = false
        compile(model: model, reason: "explicit")
    }

    /// Ends the export (any outcome) and tells its caller. Sends nothing:
    /// held compiles go out from `exportDone` or the export's error, and a
    /// restart resends every document anyway. `awaitingDone`: the export's
    /// DONE is still to come (`ExportStage.ending`).
    private func finishExport(_ result: Result<Data, ExportFailure>, awaitingDone id: Int? = nil) {
        guard exportStage != nil || exportCompletion != nil else { return }
        toolsTimeout?.cancel(); toolsTimeout = nil
        exportStage = id.map { .ending(id: $0) }
        if exporting { exporting = false }
        let completion = exportCompletion
        exportCompletion = nil
        completion?(result)
    }

    /// The project's file set changed on disk (not the editor's own files):
    /// trust is decided again, by a walk, before the next compile.
    private func projectFileSetChanged(_ paths: [String]) {
        guard let model, let root = model.project.projectRoot?.standardizedFileURL.resolvingSymlinksInPath().path else { return }
        let editor = Set(model.documents.map { root + "/" + $0.path })
        let others = paths.filter { !editor.contains($0) }
        guard !others.isEmpty else { return }
        projectFileSetChanges += 1
        if !trustPending {
            trustPending = true
            if logDone { log("project files changed (\(others.count), e.g. \(others[0])): trust is decided again before the next compile") }
        }
    }

    // MARK: external tools (protocol 3.2)

    /// An export waits for the host's copy to hold the editor's text (the
    /// compile it sent first, or a later one), for the resident engine to be
    /// idle and, when that compile allowed tools, for their cycle to settle:
    /// a follow-up compile (`"cause": "tools"`) would interleave its frames.
    private func maybeSendExport() {
        guard case .syncing(let after) = exportStage, let model, !compiling, lastDoneID >= after else { return }
        // Only the newest finished compile's own cycle can still recompile
        // (a newer COMPILE supersedes an older cycle, which may then never
        // say `settled`): wait for it when that compile allowed tools.
        if lastToolsAutoID == lastDoneID, lastSettledID < lastDoneID {
            armToolsTimeout(for: lastDoneID)
            return
        }
        toolsTimeout?.cancel(); toolsTimeout = nil
        sendExport(model: model)
    }

    /// The last resort for an export waiting on tools: fail it, never wait forever.
    @ObservationIgnored private var toolsTimeout: DispatchWorkItem?
    static let exportToolsTimeout: TimeInterval = 300

    private func armToolsTimeout(for id: Int) {
        guard toolsTimeout == nil else { return }
        let item = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.toolsTimeout = nil
            guard case .syncing = self.exportStage, self.lastSettledID < id else { return }
            self.finishExport(.failure(.failed("the bibliography and index tools did not finish within \(Int(Self.exportToolsTimeout)) s")))
        }
        toolsTimeout = item
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.exportToolsTimeout, execute: item)
    }

    /// TeX's `.log` of the last compile (gap A21): `<output dir>/<jobname>.log`.
    var texLogURL: URL? {
        guard let project else { return nil }
        let job = (mainFile as NSString).lastPathComponent.replacingOccurrences(of: ".tex", with: "")
        let url = project.output.appendingPathComponent(job + ".log")
        return FileManager.default.fileExists(atPath: url.path) ? url : nil
    }

    /// View ▸ Show TeX Log: opens the last compile's `.log` in the default app.
    func showTeXLog() {
        guard let url = texLogURL else { model?.navigationNote = "No TeX log yet: compile first (⌘B)."; return }
        NSWorkspace.shared.open(url)
    }

    /// TeX's rows of the last compile, then the tools' (Problems panel).
    private func publishProblems(model: ShellModel) {
        let rows = texProblems + Self.problems(toolDiagnostics, model: model, projectRoot: project?.root)
        if model.engineV3Diagnostics != rows { model.engineV3Diagnostics = rows }
    }

    /// `TOOL`: run, done, skip, settled (spec §6.4).
    private func tool(_ j: DL3JSON) {
        let id = Int(j["id"]?.int ?? -1)
        let name = j["tool"]?.string ?? "tool"
        let file = j["file"]?.string.map { " " + (($0 as NSString).lastPathComponent) } ?? ""
        switch j["event"]?.string {
        case "run":
            // A run for a compile older than the newest client compile that
            // started belongs to a superseded cycle, which may never settle:
            // it neither holds the stall bound off, nor replaces the current
            // cycle's rows, nor shows as running.
            guard id >= newestClientStartedID else { break }
            toolsCycleID = max(toolsCycleID ?? id, id)
            if id != toolCycleID { toolCycleID = id; toolDiagnostics = [] } // a new cycle's runs replace the last one's rows
            toolNote = "Running \(name)\(file)…"
        case "done":
            let status = j["status"]?.string ?? "?"
            switch status {
            case "ok", "warnings": toolNote = nil
            default: toolNote = "\(name)\(file): \(status)\(j["message"]?.string.map { " (" + $0 + ")" } ?? "")"
            }
            if let model { publishProblems(model: model) }
        case "skip":
            toolNote = "\(name)\(file) not run: \(j["reason"]?.string ?? "skipped")"
        case "settled":
            if let cycle = toolsCycleID, id >= cycle { toolsCycleID = nil }
            lastSettledID = max(lastSettledID, id)
            if toolNote?.hasPrefix("Running ") == true { toolNote = nil }
            if j["limit"]?.bool == true { toolNote = "Bibliography and index: stopped after \(j["rounds"]?.int ?? 5) rounds." }
            if let model { publishProblems(model: model) }
            maybeSendExport()
        default: break
        }
    }

    // MARK: events from the reader

    /// One event from the inbox (`EngineV3Delivery.drain`), without the
    /// per-drain work (`flushEvents`). `c` nil: a test's feed, as from the
    /// current connection.
    fileprivate func applyQueued(_ out: EngineV3Reader.Output, connection c: DL3Connection?) {
        guard c == nil || c === connection else { return } // a replaced connection's late frames
        MainThreadProbe.time("v3.event") { handleOne(out) }
        afterEvent?(out)
    }

    /// What waits for the end of a drain: the pages' layout, once for every
    /// page that arrived in it (a page past the laid-out ones used to lay out
    /// all of them: O(pages²) over a cold compile).
    func flushEvents() {
        view?.flushLayout()
    }

    /// Hands host events to the main thread as the reader thread does (tests:
    /// `EditorInstantTests` feeds a synthetic compile through it).
    @ObservationIgnored private(set) var delivery: EngineV3Delivery!

    /// Tests: called after each event from the host has been applied.
    @ObservationIgnored var afterEvent: ((EngineV3Reader.Output) -> Void)?

    /// Applies one event from the host and flushes (tests; the inbox
    /// applies a drain's events, then flushes once).
    func handle(_ out: EngineV3Reader.Output) {
        handleOne(out)
        flushEvents()
    }

    private func handleOne(_ out: EngineV3Reader.Output) {
        lastEventNs = MonotonicClock.nowNs()
        lastHostActivityNs = lastEventNs // the stall bound: the host is alive and working
        switch out {
        case .started(let j):
            typesettingID = j["id"]?.int.map(Int.init)
            // A newer client compile (not a tools follow-up) ends the tool cycle it superseded.
            if let id = typesettingID, j["cause"]?.string != "tools" {
                newestClientStartedID = max(newestClientStartedID, id)
                if let cycle = toolsCycleID, id > cycle { toolsCycleID = nil }
            }
            if errorCount != 0 { errorCount = 0 }
            if warningCount != 0 { warningCount = 0 }
            if firstError != nil { firstError = nil }
            compileFatal = false
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
            pageSizes[index] = CGSize(width: p.widthPt, height: p.heightPt)
            glyphIndexes.invalidate(index)
            pageInstalls &+= 1
            stale.remove(index)
            pdfFallback[index] = nil
            if index >= pageCount {
                let before = pageCount
                pageCount = index + 1; layoutRevision &+= 1
                // Stored pages the count now reaches again stay stale: only
                // the indexes it newly reaches (`markStale(stale)` walked
                // every page, per page: O(pages²) over a cold compile).
                if let s = snapshot?.0, before < min(pageCount, s.pages.count) {
                    for i in before ..< min(pageCount, s.pages.count) where pages[i] == nil { stale.insert(i) }
                    staleChangedNow()
                }
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
            // A complete count below the pages on screen is applied at DONE:
            // a run that stopped on a fatal error keeps the last good ones.
            if let n = j["count"]?.int { setCount(Int(n), complete: false) }
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
            if EngineV3ErrorPolicy.isFatal(d) { compileFatal = true }
            // (an error can change how the ones before it read: recount; O(errors²) at worst)
            if d.severity == "error" { recount() } else if d.severity == "warning" { warningCount += 1 }
        case .diagnostic(let j) where j["source"]?.string != nil:
            toolDiagnostics.append(j) // bibtex/biber/makeindex (3.2), published at the tool's done
        case .diagnostic(let j):
            diagnostics.append(j)
            if j["severity"]?.string == "error" {
                if EngineV3ErrorPolicy.isFatal(message: j["message"]?.string ?? "") { compileFatal = true }
                recount()
            } else { warningCount += 1 }
        case .exportDone(let j):
            exportDone(j)
        case .done(let j, let compileID):
            let doneStart = DispatchTime.now().uptimeNanoseconds
            let probe = MainThreadProbe.begin()
            defer { lastDoneMainMs = Double(DispatchTime.now().uptimeNanoseconds - doneStart) / 1e6; MainThreadProbe.end("v3.done", probe) }
            let status = j["status"]?.string ?? "?"
            if logDone { log("DONE \(j)") }
            doneCount &+= 1
            lastDone = j
            if status == "failed", let project, !project.exists, let model {
                compile(model: model, reason: "recover") // the copy vanished under the host
            }
            if status == "ok" || status == "error", let p = project {
                // what the run left is whole: the next copy may take it over
                Self.walkQueue.async { p.markComplete() }
            }
            if status != "cancelled" {
                let made = j["pages"]?.int.map(Int.init)
                recount()
                if compileFatal, let n = made, n < pageCount {
                    // TeX stopped (EngineV3ErrorPolicy): the pages it made
                    // before the stop are current, the last good ones after
                    // them stay on screen, stale, until a compile sends them.
                    let kept = pageCount - n
                    markStale(Set(n ..< pageCount))
                    statusNote = "stopped: pdfLaTeX gives up here · \(n) new page\(n == 1 ? "" : "s"), \(kept) kept from the last compile · \(String(format: "%.0f", j["elapsed_ms"]?.double ?? 0)) ms"
                } else {
                    // A DONE without `pages` says nothing about them: the
                    // marks stay as they are (never "all current").
                    if let n = made {
                        setCount(n, complete: true)
                        // A stored page the compile did not replace (it failed early)
                        // stays on screen, stale, until its page arrives.
                        markStale([])
                    }
                    // Best effort: TeX recovered from every error it reported.
                    let shown = status == "error" && errorCount == 0 ? "recovered" : status
                    statusNote = "\(shown) · \(j["mode"]?.string ?? "") · \(pageCount) page\(pageCount == 1 ? "" : "s") · \(String(format: "%.0f", j["elapsed_ms"]?.double ?? 0)) ms"
                }
                // The Problems line (gap B8): a compile that wrote no page, or
                // stopped on a fatal error, keeps the last ones.
                let failed = status == "failed" || compileFatal || (status == "error" && (j["pages"]?.int ?? 1) == 0)
                if let model, model.engineV3ResultStatus != (failed ? .failed : nil) { model.engineV3ResultStatus = failed ? .failed : nil }
                // A package that needs XeTeX or LuaTeX stopped it (fontspec, unicode-math,
                // xeCJK, \RequireXeTeX): the compatibility engine takes the document,
                // visibly (UnicodeFonts.swift; the preamble scan missed it).
                if status != "ok", let model,
                   let need = diags.lazy.compactMap(UnicodeFonts.need(in:)).first
                       ?? diagnostics.lazy.compactMap({ UnicodeFonts.need(message: $0["message"]?.string ?? "", detail: $0["detail"]?.string) }).first {
                    model.engineV3NeedsUnicodeFonts(need)
                }
                if stale.isEmpty { snapshot = nil } // the compile's pages replaced the stored ones
                // (pages TeX made recovering from errors are a document's too; a stopped run's are not all there)
                if status == "ok" || (status == "error" && !compileFatal), compileID >= lastSentID { scheduleSnapshotSave() }
                if let model {
                    // TeX's line numbers refer to the texts this compile read,
                    // not the editor's now (typing went on meanwhile): the rows'
                    // byte ranges are taken from those, and they are the
                    // baseline the editor marks, the Problems panel's line
                    // labels and navigation rebase from to the current text
                    // (set before the rows, which read it).
                    let texts = textsCompiled(by: compileID, model: model)
                    model.setEngineV3CompiledDocuments(texts)
                    caretWindow = caretWindows[compileID] // the caret's edits since this compile was sent
                    compiledStamp &+= 1
                    prepareCaretLines(path: model.activePath, text: model.compiledDocuments[model.activePath])
                    texProblems = diags.isEmpty ? Self.problems(diagnostics, model: model, projectRoot: project?.root, texts: texts, mode: errorMode)
                                                : Self.problems(diags: diags, model: model, projectRoot: project?.root, texts: texts, mode: errorMode)
                    publishProblems(model: model)
                    // VoiceOver: "2 errors, 1 warning" when the counts changed (the v2 path's announcement).
                    if model.engineV3Enabled { model.noteCompileCompletedForVoiceOver() }
                    // A package or class TeX could not find: resolved from a
                    // library or the cache, or offered for fetching (ProjectPackages.swift).
                    if compileID >= lastSentID { model.projectPackages.noteCompileResult(diagnostics: model.engineV3Diagnostics) }
                }
                loadFallbacks(pdf: j["pdf"]?.string)
            }
            for k in compiledTexts.keys where k <= compileID { compiledTexts[k] = nil }
            for k in caretWindows.keys where k <= compileID { caretWindows[k] = nil }
            latency.done(compile: compileID, cancelled: status == "cancelled", hostFirstPageMs: j["first_page_ms"]?.double)
            if compileID >= lastSentID, compiling { compiling = false; compileRunningLong = false }
            if let t = typesettingID, compileID >= t { typesettingID = nil }
            for k in unanswered.keys where k <= compileID { unanswered[k] = nil }
            if status != "cancelled" { lastDoneID = max(lastDoneID, compileID); contentStamp &+= 1; scheduleCaretMark(afterCompile: true) }
            maybeSendExport()
        case .progress:
            progressFrames += 1 // the stall bound's heartbeat (`lastHostActivityNs`, above)
        case .tool(let j):
            tool(j)
        case .exportError(let j):
            // An ERROR after the export's STARTED: its DONE follows, and until
            // then every frame is the export's; held compiles wait for it.
            if case .running(let id) = exportStage, j["id"]?.int.map(Int.init) == id {
                finishExport(.failure(.failed(j["message"]?.string ?? "the export failed")), awaitingDone: id)
            }
        case .error(let j):
            if case .running(let id) = exportStage, j["id"]?.int.map(Int.init) == id {
                // Refused before it started (no STARTED, so no DONE): nothing
                // of it is on the socket, the held compiles go now.
                finishExport(.failure(.failed(j["message"]?.string ?? "the host refused the export")))
                sendHeld()
                return
            }
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
        if let size = pageSizes[i] { return size }
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
              pageCount > 0, (0 ..< pageCount).allSatisfy({ pageSizes[$0] != nil }) else { return }
        let visible = visiblePage
        var chosen: [Int: DL3PreparedPage] = [:]
        for i in Array(0 ..< min(3, pageCount)) + Array(max(0, visible - 2) ... min(pageCount - 1, visible + 5)) where chosen.count < EngineV3Snapshot.maxPages {
            chosen[i] = pages[i]
        }
        let sizes = (0 ..< pageCount).map { pageSizes[$0]! }
        // The texts' SHA-256 on the snapshot queue, not here: a 4 MB document
        // is milliseconds of main thread at every keystroke's DONE.
        let texts = model.documents.map { (path: $0.path, text: sentTexts[$0.path] ?? $0.text) }
        let formsCopy = forms, main = mainFile
        let ppp = view?.currentPixelsPerPoint ?? 2
        let dark = model.darkPreview
        let item = DispatchWorkItem {
            let docs = EngineV3Snapshot.hashes(texts)
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
    static func problems(_ diags: [DL3JSON], model: ShellModel, projectRoot: URL?, texts: [String: String]? = nil,
                         mode: EngineV3ErrorPolicy.Mode = .bestEffort) -> [RuntimeV1.Diagnostic] {
        let roots = Self.copyRoots(projectRoot)
        let kept = EngineV3ErrorPolicy.keptErrors(diags.map(Self.policyItem), mode: mode)
        var lines = EngineV3LineIndexes()
        return diags.enumerated().map { i, d in
            let error = d["severity"]?.string == "error"
            let severity: RuntimeV1.Severity = error && kept.contains(i) ? .error : .warning
            var message = d["message"]?.string ?? "(no message)"
            if error, !kept.contains(i) { message = EngineV3ErrorPolicy.marked(message) }
            var source: RuntimeV1.SourceRange?
            if let reported = d["file"]?.string {
                let file = Self.relativeToCopy(reported, roots: roots)
                let line = Int(d["line"]?.int ?? 0)
                if let text = texts?[file] ?? model.documents.first(where: { $0.path == file })?.text, line > 0,
                   let range = lines.range(file, text, line: line) {
                    source = RuntimeV1.SourceRange(path: file, startByte: range.lowerBound, endByte: range.upperBound)
                } else {
                    message = "\((file as NSString).lastPathComponent)\(line > 0 ? ":\(line)" : ""): " + message
                }
            }
            return RuntimeV1.Diagnostic(severity: severity, message: message, source: source, recovery: nil, code: "engine-v3")
        }
    }

    /// Every spelling of the project copy's root, each ending in "/": as the
    /// app made it (standardized: /var/folders/...), its real path (realpath:
    /// /private/var/folders/..., what the host's getcwd() returns and so what
    /// the display list's side table names a box's place by), and
    /// Foundation's resolution of its links. Empty without a root.
    static func copyRoots(_ root: URL?) -> [String] {
        guard let root else { return [] }
        let standardized: String = root.standardizedFileURL.path
        var spellings: [String] = [standardized]
        if let p = realpath(standardized, nil) {
            spellings.append(String(cString: p))
            free(p)
        }
        spellings.append(root.resolvingSymlinksInPath().path)
        var out: [String] = []
        for r in spellings {
            let prefix = r.hasSuffix("/") ? r : r + "/"
            if !out.contains(prefix) { out.append(prefix) }
        }
        return out
    }

    /// A file the engine names, relative to the project copy when it is
    /// inside it under any of `roots` (`copyRoots`), else as given; a
    /// leading "./" dropped. An absolute name under none of them is matched
    /// once more by its own real path (a link anywhere above the copy).
    static func relativeToCopy(_ file: String, roots: [String]) -> String {
        var f: String = file
        while f.hasPrefix("./") { f.removeFirst(2) }
        if let r = roots.first(where: { f.hasPrefix($0) }) {
            f.removeFirst(r.count)
            return f
        }
        guard f.hasPrefix("/"), !roots.isEmpty, let p = realpath(f, nil) else { return f }
        let real = String(cString: p)
        free(p)
        if let r = roots.first(where: { real.hasPrefix($0) }) { return String(real.dropFirst(r.count)) }
        return f
    }

    /// diag-v1 DIAGs as Problems-panel diagnostics: the source range is the
    /// reported token/command (`range`, byte columns of `line`) or TeX's split
    /// (`col`), so a click lands on the exact column; the macro chain and
    /// TeX's help text become the row's notes and help.
    static func problems(diags: [DL3Diag], model: ShellModel, projectRoot: URL?, texts: [String: String]? = nil,
                         mode: EngineV3ErrorPolicy.Mode = .bestEffort) -> [RuntimeV1.Diagnostic] {
        let roots = Self.copyRoots(projectRoot)
        // TeX's errors it recovered from (EngineV3ErrorPolicy): warnings under best effort.
        let kept = EngineV3ErrorPolicy.keptErrors(diags.map(Self.policyItem), mode: mode)
        let recovered = Set(diags.indices.filter { diags[$0].severity == "error" && !kept.contains($0) })
        func rel(_ file: String) -> String { Self.relativeToCopy(file, roots: roots) }
        // Rows that say the same thing as another fold into it (EngineV3DiagPresent.folds).
        let folds = EngineV3DiagPresent.folds(diags, kept: kept)
        let stopped = Set(folds.filter { EngineV3DiagPresent.isStop(diags[$0.key].code) && !EngineV3DiagPresent.isStop(diags[$0.value].code) }.map(\.value))
        let projectTexts: [String: String] = texts ?? Dictionary(model.documents.map { ($0.path, $0.text) }, uniquingKeysWith: { a, _ in a })
        func text(_ file: String) -> String? { texts?[file] ?? model.documents.first(where: { $0.path == file })?.text }
        var lines = EngineV3LineIndexes()
        return diags.enumerated().compactMap { i, d in
            guard d.severity != "info" else { return nil } // \show, tight/loose boxes: not problems
            guard folds[i] == nil else { return nil }
            let headline = EngineV3DiagPresent.headline(code: d.code, message: d.message)
            var message = recovered.contains(i) ? EngineV3ErrorPolicy.marked(headline) : headline
            var source: RuntimeV1.SourceRange?
            if let file = d.file.map(rel) {
                if let line = d.line, let text = text(file), let lineRange = lines.range(file, text, line: line) {
                    let len = lineRange.count
                    let (a, b): (Int, Int) = {
                        if let r = d.range { return (min(r.0, len), min(max(r.1, r.0), len)) }
                        if let c = d.col { return (min(c, len), min(c, len)) }
                        return (0, len)
                    }()
                    var end = lineRange.lowerBound + b
                    // A box: from its first character to its last (the display list's side table).
                    if EngineV3DiagPresent.boxKind(d.code) != nil, let e = d.end, e.file.map(rel) == file, let el = e.line, let ec = e.col,
                       el >= line, let endLine = lines.range(file, text, line: el), lineRange.lowerBound + a <= endLine.lowerBound + min(ec, endLine.count) {
                        end = endLine.lowerBound + min(ec, endLine.count)
                    }
                    source = RuntimeV1.SourceRange(path: file, startByte: lineRange.lowerBound + a, endByte: end)
                } else {
                    message = "\((file as NSString).lastPathComponent)\(d.line.map { ":\($0)" } ?? "")\(d.col.map { ":\($0 + 1)" } ?? ""): " + message
                }
            }
            var notes: [String] = []
            var labels: [RuntimeV1.Diagnostic.Label] = []
            var defFix: RuntimeV1.Diagnostic.Help?
            if EngineV3DiagPresent.boxKind(d.code) != nil, let note = EngineV3DiagPresent.boxNote(d.message) {
                notes.append(note)
                // TeX's display of the box (fonts and all) only where the row has no text to show.
                if source == nil, let detail = d.detail, !detail.isEmpty { notes.append(detail) }
            } else if let detail = d.detail, !detail.isEmpty { notes.append(detail) }
            let macros = d.trace.filter { $0.kind == "macro" }
            for (k, f) in macros.enumerated() {
                let name = (f.name ?? "a macro").trimmingCharacters(in: .whitespaces)
                var def = f.def.flatMap { l in l.file.map { "\(rel($0))\(l.line.map { ":\($0)" } ?? "")" } }
                // The definition in the project's text: where the engine says it was made (#1593),
                // else (the engine names no site) the project's one definition of it.
                let near = f.def.flatMap { l in l.file.map { (path: rel($0), line: l.line ?? Int.max) } }
                let found = near.flatMap { EngineV3DiagPresent.definition(of: name, in: projectTexts, near: $0) }
                    ?? (def == nil ? EngineV3DiagPresent.definition(of: name, in: projectTexts) : nil)
                if let found {
                    def = found.site.label
                    labels.append(.init(source: found.site.source, text: "\(name) is defined at \(found.site.label)", primary: false))
                    // The error is inside this (innermost) macro: the undefined name in its definition.
                    if k == 0, d.code == EngineV3Fixes.undefinedCode, let body = found.body, let cs = EngineV3DiagPresent.lastControlWord(f.before),
                       let at = EngineV3DiagPresent.occurrence(of: cs, in: body, path: found.site.path, texts: projectTexts) {
                        labels.append(.init(source: at.source, text: "\(cs) is undefined (in \(name) at \(at.label))", primary: false))
                        let close = EngineV3Fixes.closestCommands(String(cs.dropFirst()), vocabulary: Completion.defaultSupported)
                        if close.count == 1 {
                            defFix = .init(message: "did you mean \\\(close[0])? (in the definition of \(name), \(at.label))",
                                           replacement: .init(startByte: at.start, endByte: at.end, text: "\\" + close[0], path: at.path))
                        }
                    }
                }
                // Only a macro with a known definition (the user's, in effect): the
                // kernel's own (\GenericWarning, \@setref, \use_i:nn) say nothing to the author.
                if let def { notes.append("in \(name) (defined at \(def))") }
            }
            if !labels.isEmpty, let source { labels.insert(.init(source: source, text: "", primary: true), at: 0) }
            let help = d.help.isEmpty ? nil : RuntimeV1.Diagnostic.Help(message: d.help.joined(separator: " "))
            var row = RuntimeV1.Diagnostic(severity: d.severity == "error" && !recovered.contains(i) ? .error : .warning, message: message, source: source,
                                           recovery: nil, code: d.code.isEmpty ? "engine-v3" : d.code,
                                           labels: labels.isEmpty ? nil : labels, notes: notes.isEmpty ? nil : notes, help: help)
            if stopped.contains(i), let stop = EngineV3Explain.explanation(code: "tex/emergency-stop", message: "") {
                row.notes = (row.notes ?? []) + [stop]
            }
            let named = EngineV3Fixes.undefinedName(trace: d.trace.map { ($0.kind, $0.before) })
            // A plain-language explanation and, where mechanical, a fix (EngineV3Explain.swift).
            var explained = EngineV3Explain.apply(row, code: d.code, message: d.message, texts: texts, undefinedName: named)
            if let defFix, explained.help?.replacement == nil {
                if let tex = explained.help?.message, !tex.isEmpty { explained.notes = (explained.notes ?? []) + [tex] }
                explained.help = defFix
            }
            // "did you mean \textbf?": the old engine's mechanical fix, only
            // where the range is the very name TeX reports (EngineV3Fixes.swift).
            guard d.code == EngineV3Fixes.undefinedCode, let texts, explained.help?.replacement == nil else { return explained }
            return EngineV3Fixes.fix(explained, named: named, texts: texts)
        }
    }

    static func policyItem(_ d: DL3Diag) -> EngineV3ErrorPolicy.Item {
        .init(error: d.severity == "error", fatal: EngineV3ErrorPolicy.isFatal(d), line: d.line)
    }

    static func policyItem(_ j: DL3JSON) -> EngineV3ErrorPolicy.Item {
        let error = j["severity"]?.string == "error"
        return .init(error: error, fatal: error && EngineV3ErrorPolicy.isFatal(message: j["message"]?.string ?? ""), line: j["line"]?.int.map(Int.init))
    }

    /// The pane's counts and first error from the compile's diagnostics so
    /// far, under `errorMode`: an error TeX recovered from counts as a
    /// warning under best effort, and is not the pane's first error.
    private func recount() {
        var e = 0, w = 0
        var first: String?
        if !diags.isEmpty {
            let kept = EngineV3ErrorPolicy.keptErrors(diags.map(Self.policyItem), mode: errorMode)
            let folds = EngineV3DiagPresent.folds(diags, kept: kept) // as the Problems rows count them
            for (i, d) in diags.enumerated() where folds[i] == nil {
                if d.severity == "error", kept.contains(i) {
                    e += 1
                    if first == nil {
                        let at = [d.file.map { ($0 as NSString).lastPathComponent }, d.line.map(String.init), d.col.map { String($0 + 1) }].compactMap { $0 }.joined(separator: ":")
                        first = (at.isEmpty ? "" : at + ": ") + EngineV3DiagPresent.headline(code: d.code, message: d.message)
                    }
                } else if d.severity == "error" || d.severity == "warning" { w += 1 }
            }
        } else {
            let kept = EngineV3ErrorPolicy.keptErrors(diagnostics.map(Self.policyItem), mode: errorMode)
            for (i, j) in diagnostics.enumerated() {
                if j["severity"]?.string == "error", kept.contains(i) {
                    e += 1
                    if first == nil {
                        let file = j["file"]?.string.map { ($0 as NSString).lastPathComponent }
                        let at = [file, j["line"]?.int.map { String($0) }].compactMap { $0 }.joined(separator: ":")
                        first = (at.isEmpty ? "" : at + ": ") + (j["message"]?.string ?? "error")
                    }
                } else { w += 1 }
            }
        }
        if errorCount != e { errorCount = e }
        if warningCount != w { warningCount = w }
        if firstError != first { firstError = first }
    }

    /// Bytes of 1-based `line` in `text`, without its newline (one lookup;
    /// many: `EngineV3LineStarts`).
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

    /// No pages (a stop, another project): none of them, nor their glyph
    /// indexes, may answer a lookup.
    private func dropPages() {
        pages = [:]; pageSizes = [:]; stale = []; forms = [:]; pdfFallback = [:]; pageCount = 0
        glyphIndexes.removeAll()
    }

    /// Tests: the pages go as they do for another project in this window (`compile`).
    func dropPagesForTesting() { dropPages() }

    /// Memory pressure (EngineV3MemoryPressure): drops what is cheap to
    /// build again and asks the host to trim. A warning keeps the glyph
    /// indexes of the pages the pane holds (the caret mark's); a critical
    /// event drops them all. Pages and their bitmaps stay.
    func trimMemory(_ level: EngineV3MemoryPressure.Level) {
        pressureEvents &+= 1
        let held: Set<Int> = level == .critical ? [] : Set(view?.heldPageIndexes ?? [])
        glyphIndexes.trim(keeping: held)
        guard hostOffersTrim, let connection else { return }
        do {
            try connection.trim(level: level.rawValue)
            trimsSent &+= 1
        } catch {
            log("TRIM not sent: \(error)") // the reader sees the broken connection and restarts the host
        }
    }

    /// Pages on screen: a complete count is the document's (pages past it
    /// go); an incomplete one (a compile in progress) never hides pages.
    private func setCount(_ n: Int, complete: Bool) {
        guard n != pageCount || complete else { return }
        guard complete || n > pageCount else { return }
        if complete, n < pageCount {
            for i in n ..< pageCount { pages[i] = nil; pageSizes[i] = nil; stale.remove(i); pdfFallback[i] = nil }
            glyphIndexes.removePages(from: n)
        }
        if n != pageCount { pageCount = n; layoutRevision &+= 1 }
    }

    /// Pages the display list cannot draw exactly render from the compile's PDF.
    private func loadFallbacks(pdf: String?) {
        let start = DispatchTime.now().uptimeNanoseconds
        defer { lastFallbackLoadMs = Double(DispatchTime.now().uptimeNanoseconds - start) / 1e6 }
        let need = pages.filter { $0.value.needsPDFFallback(forms: forms) }.map(\.key)
        guard !need.isEmpty, let pdf, let doc = DL3Renderer.openPDF(URL(fileURLWithPath: pdf)) else { return }
        pdfFallbackDocument = doc // (a CGPDFPage does not keep its document)
        for i in need { if let p = DL3Renderer.page(of: doc, at: i + 1) { pdfFallback[i] = p } } // (with its bytes: drawPDFPage)
        view?.fallbacksChanged(need)
    }

    nonisolated static func onMain(_ block: @escaping @MainActor () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue) { MainActor.assumeIsolated { block() } }
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }
}

/// Where each line of a text starts (UTF-8 bytes), found in one pass: a
/// DONE maps every diagnostic's line to bytes, and `lineByteRange` walked the
/// text from the start for each (100 warnings in a 4 MB book: 200 MB walked
/// on main per DONE).
struct EngineV3LineStarts {
    /// `starts[k]`: the first byte of line k + 1.
    private(set) var starts: [Int] = [0]
    private(set) var byteCount = 0

    init(_ text: String) {
        var copy = text
        var found: [Int] = [0]
        byteCount = copy.withUTF8 { b -> Int in
            guard let base = b.baseAddress else { return 0 }
            var off = 0
            while off < b.count, let hit = memchr(base + off, 0x0A, b.count - off) {
                let at = UnsafeRawPointer(base).distance(to: UnsafeRawPointer(hit))
                found.append(at + 1)
                off = at + 1
            }
            return b.count
        }
        starts = found
    }

    /// `EngineV3Session.lineByteRange(text, line:)`, from the index.
    func range(line: Int) -> Range<Int>? {
        guard line >= 1, line <= starts.count else { return nil }
        let a = starts[line - 1]
        return a ..< (line < starts.count ? starts[line] - 1 : byteCount)
    }
}

/// One `EngineV3LineStarts` per file, built on first use.
struct EngineV3LineIndexes {
    private var byFile: [String: EngineV3LineStarts] = [:]
    mutating func range(_ file: String, _ text: String, line: Int) -> Range<Int>? {
        if let ix = byFile[file] { return ix.range(line: line) }
        let ix = EngineV3LineStarts(text)
        byFile[file] = ix
        return ix.range(line: line)
    }
}

/// A weak reference that may cross threads (it is only dereferenced on main).
final class EngineV3WeakRef: @unchecked Sendable {
    weak var value: EngineV3Session?
    init(_ v: EngineV3Session) { value = v }
}

/// How the reader thread's events reach the main thread (APP-EDITOR-INSTANT,
/// docs/evidence/editor-instant-2026-10-06). Any thread posts; main drains.
///
/// Each event used to be its own main-thread block, and Core Foundation runs
/// every block queued when it gets to them: a cold compile's thousand PAGEs
/// (each laying out every page) held the main thread, and the keys typed
/// meanwhile, for as long as the backlog took. Now the events queue here and
/// main applies them in drains: a drain stops after `budgetNs` of work and
/// the rest waits for the next one; while events keep coming, drains are at
/// least a frame (`frameNs`) apart, so a key never waits behind more than
/// one budget of preview work, and the SwiftUI and layout work an event
/// causes is done once per drain (`EngineV3Session.flushEvents`). A drain
/// after a quiet spell starts at once (a keystroke's compile is not delayed;
/// its page is installed on the reader thread anyway).
/// `FLASHTEX_V3_COALESCE=0`: every event in its own block, as before (A/B).
final class EngineV3Delivery: @unchecked Sendable {
    let ref: EngineV3WeakRef
    let coalesce: Bool
    /// Main-thread time one drain may spend before it yields (`FLASHTEX_V3_DRAIN_MS`, default 3).
    let budgetNs: UInt64
    /// Drains while events keep coming are at least this far apart (a 120 Hz frame).
    let frameNs: UInt64 = 8_333_333
    private let lock = NSLock()
    private var queue: [(EngineV3Reader.Output, DL3Connection?)?] = []
    private var head = 0
    private var scheduled = false
    private var lastDrainNs: UInt64 = 0
    /// Drains run, and those that yielded with events left (tests, evidence).
    private(set) var drains = 0
    private(set) var yielded = 0

    init(_ ref: EngineV3WeakRef) {
        self.ref = ref
        let env = ProcessInfo.processInfo.environment
        coalesce = env["FLASHTEX_V3_COALESCE"] != "0"
        budgetNs = UInt64((env["FLASHTEX_V3_DRAIN_MS"].flatMap(Double.init) ?? 3) * 1e6)
    }

    /// One decoded event from connection `c` (nil: a test's feed), applied on main.
    func post(_ out: EngineV3Reader.Output, connection c: DL3Connection?) {
        guard coalesce else {
            let ref = self.ref
            EngineV3Session.onMain {
                guard let s = ref.value else { return }
                s.applyQueued(out, connection: c)
                s.flushEvents()
            }
            return
        }
        lock.lock()
        queue.append((out, c))
        let schedule = !scheduled
        scheduled = true
        let last = lastDrainNs
        lock.unlock()
        if schedule { scheduleDrain(after: last) }
    }

    /// At once after a quiet frame, else a frame after the last drain began.
    private func scheduleDrain(after last: UInt64) {
        let now = MonotonicClock.nowNs()
        let due = last &+ frameNs
        if last == 0 || now >= due {
            EngineV3Session.onMain { self.drain() }
        } else {
            DispatchQueue.main.asyncAfter(deadline: .now() + .nanoseconds(Int(due &- now))) {
                MainActor.assumeIsolated { self.drain() }
            }
        }
    }

    @MainActor
    private func drain() {
        let t0 = MonotonicClock.nowNs()
        let probe = MainThreadProbe.begin()
        lock.lock(); lastDrainNs = t0; lock.unlock()
        let session = ref.value
        var more = false
        while true {
            lock.lock()
            guard head < queue.count else {
                queue.removeAll(keepingCapacity: true); head = 0; scheduled = false
                lock.unlock()
                break
            }
            let item = queue[head]
            queue[head] = nil // its page is released when applied
            head += 1
            lock.unlock()
            if let item { session?.applyQueued(item.0, connection: item.1) }
            if MonotonicClock.nowNs() &- t0 >= budgetNs {
                lock.lock()
                more = head < queue.count
                if !more { queue.removeAll(keepingCapacity: true); head = 0; scheduled = false }
                else if head >= 4096 { queue.removeFirst(head); head = 0 } // a backlog that never empties stays bounded
                lock.unlock()
                break
            }
        }
        session?.flushEvents()
        drains &+= 1
        if more { yielded &+= 1 }
        MainThreadProbe.end("v3.drain", probe)
        if more { scheduleDrain(after: t0) }
    }
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
    /// The font-smoothing setting it was drawn with; `pageArrived` redraws
    /// a raster whose setting no longer matches the session's.
    var smoothFonts: Bool = false
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
        case exportDone(DL3JSON)
        case tool(DL3JSON)
        /// `progress-v1`: the host is typesetting (a pass started, pages shipped).
        case progress(DL3JSON)
        /// An ERROR naming the export after its STARTED (its DONE follows).
        case exportError(DL3JSON)
        case error(DL3JSON)
    }
    private var bindings = DL3Bindings()
    private var forms: [UInt32: DL3PreparedPage] = [:]
    private let cache: DL3ResourceCache
    private let plan: EngineV3RasterPlan
    private var compileID = 0
    /// Between an export's STARTED and its DONE every frame is the export
    /// child's (the session sends nothing else meanwhile): its resource ids
    /// are its own, so none of it may touch the preview's bindings or pages.
    private var exportID: Int?

    init(cache: DL3ResourceCache, plan: EngineV3RasterPlan) { self.cache = cache; self.plan = plan }

    func handle(_ ev: DL3Event, timing t: DL3Connection.Timing) -> Output? {
        if let id = exportID {
            switch ev {
            case .done(let j) where Int(j["id"]?.int ?? -1) == id:
                exportID = nil
                return .exportDone(j)
            case .error(let j) where Int(j["id"]?.int ?? -1) == id: return .exportError(j) // still the export's until its DONE
            case .error(let j): return .error(j)
            default: return nil
            }
        }
        switch ev {
        case .started(let j) where j["mode"]?.string == "export":
            exportID = Int(j["id"]?.int ?? -1)
            return nil
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
                                           pixelsPerPoint: ppp, hash: EngineV3PagesView.contentKey(p.hash, look, smoothFonts: smooth),
                                           smoothFonts: smooth)
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
        case .tool(let j): return .tool(j)
        case .diag(let d): return .diag(d)
        case .done(let j): return .done(j, compileID: Int(j["id"]?.int ?? Int64(compileID)))
        case .error(let j): return .error(j)
        case .sources(let s): return .sources(s)
        case .progress(let j): return .progress(j)
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

    /// Where the fast path last measured a document (lane LIVE-30MS), in the
    /// text as it was after that edit: the UTF-8 bytes before UTF-16
    /// position `prefix16` and from `suffix16` to the end. A keystroke then
    /// counts only the text between an anchor and the edit, not the whole
    /// document (a backspace counted every byte after it: 10 ms in a 4 MB
    /// file). Valid while every change to the text goes through
    /// `fastSplice` (the session drops them otherwise) and for the byte
    /// length the host holds (`bytes`).
    struct Anchors: Equatable {
        var length16: Int
        var bytes: Int
        var prefix16: Int, prefix8: Int
        var suffix16: Int, suffix8: Int
    }

    /// The fast path's splice: the byte offset of the edit, the bytes it
    /// deleted, the text's byte length after it, and new anchors, from the
    /// storage's new text, its edited range (new UTF-16 positions) and
    /// change in length, and the byte length before (`base`). `anchors`
    /// from the previous edit spare counting the text far from the edit; a
    /// stale or unusable one is counted from the ends (the old way).
    static func fastSplice(text: NSString, edited r: NSRange, delta: Int, base: Int,
                           anchors: Anchors?) -> (prefix: Int, delete: Int, total: Int, anchors: Anchors) {
        let length = text.length, oldLength = length - delta
        let a = anchors.flatMap { $0.length16 == oldLength && $0.bytes == base ? $0 : nil }
        // Before the edit the text is the old one: from an anchor at or
        // before the edit (no edit since was before it), else from 0.
        let prefix: Int
        if let a, r.location >= a.prefix16 {
            prefix = a.prefix8 + utf8Count(text, NSRange(location: a.prefix16, length: r.location - a.prefix16))
        } else {
            prefix = utf8Count(text, NSRange(location: 0, length: r.location))
        }
        let insertBytes = utf8Count(text, r)
        let end = NSMaxRange(r), tail = length - end
        let suffix: Int
        if r.length - delta == 0 {
            suffix = base - prefix // nothing deleted: every old byte after the edit
        } else if let a, tail >= oldLength - a.suffix16 {
            // the anchored end is after the edit (in the new text it starts
            // at length - (oldLength - suffix16)), so unchanged
            let from = length - (oldLength - a.suffix16)
            suffix = a.suffix8 + utf8Count(text, NSRange(location: end, length: from - end))
        } else {
            suffix = utf8Count(text, NSRange(location: end, length: tail))
        }
        let delete = base - prefix - suffix
        let total = prefix + insertBytes + suffix
        // New anchors: the suffix at the edit's end; the prefix up to 256
        // units before the edit (a few backspaces stay after it), on a
        // character boundary (never inside a surrogate pair).
        var p = max(0, r.location - 256)
        if p > 0 { p = text.rangeOfComposedCharacterSequence(at: p).location }
        let p8 = prefix - utf8Count(text, NSRange(location: p, length: r.location - p))
        return (prefix, delete, total,
                Anchors(length16: length, bytes: total, prefix16: p, prefix8: p8, suffix16: end, suffix8: suffix))
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
    /// The copy is the project's last one, taken over (`takeOver`).
    let takenOver: Bool

    /// The copy of `source` owned by THIS app instance, with an `owner` file
    /// ("pid start-sec start-usec"). Another running instance (the same
    /// project open twice, a second app, a bench) has its own copy; nothing
    /// here ever removes a copy whose owner still runs.
    /// `session`: the owning session's number in this process (two windows
    /// with the same project get two copies, as two hosts compile them).
    ///
    /// The copy is the project's last copy when its instance has exited
    /// (`takeOver`), else a new one, `projects/<hash of the project
    /// path>-<pid>-<session>`. Taking the last copy over keeps its output
    /// (`.aux`, `.toc`, `.ind`), as a pdflatex user's next run starts from
    /// the last run's files, and its paths, which the host's persisted S₀
    /// is keyed by: reopening a document compiles once from S₀, not from
    /// the format once per `.aux` pass from nothing (for a book, three
    /// passes that each re-typeset every page).
    init(source: URL?, session: Int = 0) {
        self.source = source
        let key = Self.key(source)
        let taken = source == nil ? nil : Self.takeOver(key: key, source: source)
        takenOver = taken != nil
        base = taken
            ?? EngineV3.cacheDirectory.appendingPathComponent("projects/\(key)-\(getpid())-\(session)", isDirectory: true)
        root = base.appendingPathComponent("src", isDirectory: true)
        output = base.appendingPathComponent("out", isDirectory: true)
        ensure()
    }

    // MARK: whether a copy's output is whole

    /// The stamp of a copy whose last compile ended (`markComplete`): when,
    /// which project folder it was (inode and creation date: a new project
    /// at the same path is another), and every file the run wrote in the
    /// output folder with its size and modification time. A run stopped
    /// midway (a crash, a kill) leaves files that differ from it.
    static let stampName = "complete"

    /// Files the external tools write after a compile (`TOOL`), and the PDF
    /// and log: not what a later run reads as its own earlier output.
    static let notRunInputs: Set<String> = ["ind", "ilg", "bbl", "blg", "pdf", "log", "synctex", "gz"]

    /// The output folder's files a run reads back (by relative path): size
    /// and modification time. Symbolic links are not followed.
    static func outputListing(_ out: URL) -> [String: [Double]] {
        let fm = FileManager.default
        var r: [String: [Double]] = [:]
        guard let e = fm.enumerator(at: out, includingPropertiesForKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey, .contentModificationDateKey], options: []) else { return r }
        let prefix = out.standardizedFileURL.path + "/"
        for case let u as URL in e {
            guard let v = try? u.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey, .contentModificationDateKey]) else { continue }
            if v.isSymbolicLink == true {
                e.skipDescendants()
                continue
            }
            guard v.isRegularFile == true, !notRunInputs.contains(u.pathExtension.lowercased()) else { continue }
            let rel = u.standardizedFileURL.path.hasPrefix(prefix) ? String(u.standardizedFileURL.path.dropFirst(prefix.count)) : u.lastPathComponent
            r[rel] = [Double(v.fileSize ?? -1), v.contentModificationDate?.timeIntervalSince1970 ?? 0]
        }
        return r
    }

    /// The project folder's identity: inode and creation date.
    static func sourceIdentity(_ source: URL?) -> [Double] {
        guard let source, let a = try? FileManager.default.attributesOfItem(atPath: source.path) else { return [] }
        return [Double((a[.systemFileNumber] as? NSNumber)?.uint64Value ?? 0), (a[.creationDate] as? Date)?.timeIntervalSince1970 ?? 0]
    }

    /// A compile ended: stamp what the output folder holds (written whole,
    /// then renamed into place).
    func markComplete() { Self.markComplete(base: base, source: source) }

    static func markComplete(base: URL, source: URL?) {
        let stamp: [String: Any] = [
            "time": Date().timeIntervalSince1970,
            "source": sourceIdentity(source),
            "files": outputListing(base.appendingPathComponent("out", isDirectory: true)),
        ]
        guard let d = try? JSONSerialization.data(withJSONObject: stamp) else { return }
        let tmp = base.appendingPathComponent("\(stampName).tmp-\(getpid())")
        let dst = base.appendingPathComponent(stampName)
        guard (try? d.write(to: tmp)) != nil else { return }
        if rename(tmp.path, dst.path) != 0 { try? FileManager.default.removeItem(at: tmp) }
    }

    static func readStamp(_ base: URL) -> [String: Any]? {
        guard let d = try? Data(contentsOf: base.appendingPathComponent(stampName)),
              let j = try? JSONSerialization.jsonObject(with: d) as? [String: Any] else { return nil }
        return j
    }

    /// Whether `base`'s output folder is as its last compile left it, for
    /// the project folder `source`: else its next copy starts it empty.
    static func outputIsWhole(_ base: URL, source: URL?) -> Bool {
        guard let st = readStamp(base),
              let src = st["source"] as? [Double], src == sourceIdentity(source), !src.isEmpty,
              let files = st["files"] as? [String: [Double]] else { return false }
        let now = outputListing(base.appendingPathComponent("out", isDirectory: true))
        guard Set(now.keys) == Set(files.keys) else { return false }
        for (k, v) in files {
            guard let n = now[k], n.count == 2, v.count == 2, n[0] == v[0], abs(n[1] - v[1]) < 0.001 else { return false }
        }
        return true
    }

    /// The project's key in `projects/`: a hash of its path.
    static func key(_ source: URL?) -> String {
        SHA256.hash(data: Data((source?.path ?? "untitled").utf8)).prefix(8).map { String(format: "%02x", $0) }.joined()
    }

    /// The copies of exited instances, by project key (the newest first):
    /// directories `<key>-...` whose owner file names an instance that has
    /// exited (not this one).
    static func exitedCopies() -> [String: [(url: URL, owner: String, modified: Date)]] {
        let dir = EngineV3.cacheDirectory.appendingPathComponent("projects", isDirectory: true)
        let fm = FileManager.default
        var r: [String: [(url: URL, owner: String, modified: Date)]] = [:]
        for name in (try? fm.contentsOfDirectory(atPath: dir.path)) ?? [] {
            let base = dir.appendingPathComponent(name)
            guard let owner = try? String(contentsOf: base.appendingPathComponent("owner"), encoding: .utf8),
                  owner != EngineV3.instanceOwner, !EngineV3.ownerAlive(owner) else { continue }
            // (not a symbolic link: nothing here is followed out of the cache)
            if (try? fm.attributesOfItem(atPath: base.path))?[.type] as? FileAttributeType == .typeSymbolicLink { continue }
            // the last compile's stamp, else when the owner last wrote its file
            let modified = (readStamp(base)?["time"] as? Double).map { Date(timeIntervalSince1970: $0) }
                ?? ((try? fm.attributesOfItem(atPath: base.appendingPathComponent("owner").path))?[.modificationDate] as? Date)
                ?? .distantPast
            r[String(name.split(separator: "-").first ?? ""), default: []].append((base, owner, modified))
        }
        for k in r.keys { r[k]!.sort { $0.modified > $1.modified } }
        return r
    }

    /// Claim an exited instance's copy for this one: the owner file is
    /// renamed away (one rename succeeds, whoever else tries), checked to
    /// still name the exited `owner`, and written anew. `false`: another
    /// instance claimed it first (or something else changed it).
    static func claim(_ base: URL, from owner: String) -> Bool {
        let fm = FileManager.default
        let file = base.appendingPathComponent("owner")
        let held = base.appendingPathComponent("owner.claim-\(getpid())-\(UUID().uuidString)")
        guard (try? fm.moveItem(at: file, to: held)) != nil else { return false }
        guard (try? String(contentsOf: held, encoding: .utf8)) == owner else {
            try? fm.moveItem(at: held, to: file) // a live instance's: put it back
            return false
        }
        try? Data(EngineV3.instanceOwner.utf8).write(to: file)
        try? fm.removeItem(at: held)
        return true
    }

    /// The newest copy of the project `key` whose instance has exited,
    /// claimed for this instance (`claim`), if there is one. Its output is
    /// kept only if it is whole for this project folder (`outputIsWhole`).
    static func takeOver(key: String, source: URL?) -> URL? {
        for c in exitedCopies()[key] ?? [] where claim(c.url, from: c.owner) {
            if !outputIsWhole(c.url, source: source) {
                let fm = FileManager.default
                let out = c.url.appendingPathComponent("out", isDirectory: true)
                for name in (try? fm.contentsOfDirectory(atPath: out.path)) ?? [] { try? fm.removeItem(at: out.appendingPathComponent(name)) }
            }
            try? FileManager.default.removeItem(at: c.url.appendingPathComponent(stampName))
            return c.url
        }
        return nil
    }

    /// Whether the copy is still there (something outside may remove it).
    var exists: Bool { FileManager.default.fileExists(atPath: root.path) && FileManager.default.fileExists(atPath: output.path) }

    /// (Re)creates the directories and the owner file.
    func ensure() {
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        try? FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        try? Data(EngineV3.instanceOwner.utf8).write(to: base.appendingPathComponent("owner"))
    }

    /// Removes project copies whose owner instance has exited, but the
    /// newest of each project's, which its next copy takes over (`init`),
    /// until it is `keptCopyDays` old. Copies without an owner file (made
    /// by an older build that may still be running) and copies of running
    /// instances are left alone. Each copy is claimed before it is removed
    /// (`claim`), so a copy another instance takes over at the same moment
    /// stays.
    static func removeAbandoned(log: (String) -> Void) {
        let old = Date().addingTimeInterval(-Double(keptCopyDays) * 86_400)
        var kept: [(url: URL, owner: String, modified: Date)] = []
        func remove(_ c: (url: URL, owner: String, modified: Date)) {
            guard claim(c.url, from: c.owner) else { return }
            try? FileManager.default.removeItem(at: c.url)
            log("removed the project copy \(c.url.lastPathComponent) of exited instance \(c.owner)")
        }
        for (key, copies) in exitedCopies() {
            // (a project key is 16 hex digits; other names are not kept)
            let keep = key.count == 16 && key.allSatisfy(\.isHexDigit)
            for (i, c) in copies.enumerated() {
                if keep && i == 0 && c.modified > old { kept.append(c) } else { remove(c) }
            }
        }
        // the kept copies within their budget, the most recent first
        var total = 0
        for c in kept.sorted(by: { $0.modified > $1.modified }) {
            total += Self.bytes(of: c.url)
            if total > keptCopyBytes { remove(c) }
        }
    }

    /// The most the kept copies of exited instances may hold together.
    static let keptCopyBytes = 256 << 20

    /// The bytes of the regular files under `dir` (symbolic links not followed).
    static func bytes(of dir: URL) -> Int {
        guard let e = FileManager.default.enumerator(at: dir, includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey], options: []) else { return 0 }
        var n = 0
        for case let u as URL in e {
            if let v = try? u.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey]), v.isRegularFile == true { n += v.fileSize ?? 0 }
        }
        return n
    }

    /// How long a project's last copy waits for the project to be opened again.
    static let keptCopyDays = 30

    /// Empties this instance's copy (another project or file now uses it).
    /// `keepingOutput`: the output folder of a copy just taken over stays
    /// (its `.aux` is the next run's input).
    func clear(keepingOutput: Bool = false) {
        texInputLock.lock(); texInputNames = []; texInputLock.unlock()
        let fm = FileManager.default
        for dir in keepingOutput ? [root] : [root, output] {
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
    /// Links one project-relative file into the copy (its folders created),
    /// as `sync` would; an existing entry or a path outside `source` is left alone.
    func link(_ relativePath: String) {
        guard let source else { return }
        let fm = FileManager.default
        let src = source.appendingPathComponent(relativePath).standardizedFileURL
        guard src.path.hasPrefix(source.standardizedFileURL.path + "/"), fm.fileExists(atPath: src.path) else { return }
        let dst = root.appendingPathComponent(relativePath)
        guard (try? fm.destinationOfSymbolicLink(atPath: dst.path)) == nil, !fm.fileExists(atPath: dst.path) else { return }
        try? fm.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? fm.createSymbolicLink(at: dst, withDestinationURL: src)
    }

    /// A `[project] texinputs` file of flashtex.toml as the new engine
    /// finds it: `name` at the copy's top level, linked to `inCopy` (its
    /// rooted path in the copy, which is the editor's text when it is open)
    /// or to `external` (a real file outside the root).
    struct TexInputLink: Equatable, Sendable { var name: String; var inCopy: String?; var external: String? }

    /// The links `linkTexInputs` made.
    private var texInputNames: Set<String> = []
    private let texInputLock = NSLock()

    /// Makes `links` findable by name, as `TEXINPUTS=.:dir1:dir2:` would:
    /// a link at the copy's top level per file, unless a project file (or
    /// an open document) of that name is there already, which wins. Links
    /// made for an earlier manifest that it no longer names are removed.
    func linkTexInputs(_ links: [TexInputLink], except editorPaths: Set<String>) {
        texInputLock.lock(); defer { texInputLock.unlock() }
        let fm = FileManager.default
        var made = Set<String>()
        for l in links {
            guard !l.name.contains("/"), !editorPaths.contains(l.name) else { continue }
            let dst = root.appendingPathComponent(l.name)
            let current = try? fm.destinationOfSymbolicLink(atPath: dst.path)
            let target = l.inCopy ?? l.external
            guard let target else { continue }
            if current == nil, fm.fileExists(atPath: dst.path) { continue } // the editor's file of that name
            if let source, fm.fileExists(atPath: source.appendingPathComponent(l.name).path) {
                // A project file of that name wins; one that appeared after
                // this link was made takes its place (the walk keeps links).
                if current != nil, texInputNames.contains(l.name) {
                    try? fm.removeItem(at: dst)
                    link(l.name)
                    texInputNames.remove(l.name) // the project's link now, never removed below
                }
                continue
            }
            if let current, !texInputNames.contains(l.name), current != target { continue }
            if current != target {
                try? fm.removeItem(at: dst)
                // In the copy: relative, so it resolves to the copy's file
                // (the editor's text when open). Outside: the real file.
                if l.inCopy != nil { try? fm.createSymbolicLink(atPath: dst.path, withDestinationPath: target) }
                else { try? fm.createSymbolicLink(at: dst, withDestinationURL: URL(fileURLWithPath: target)) }
            }
            made.insert(l.name)
        }
        for name in texInputNames.subtracting(made) {
            let dst = root.appendingPathComponent(name)
            if (try? fm.destinationOfSymbolicLink(atPath: dst.path)) != nil { try? fm.removeItem(at: dst) }
        }
        texInputNames = made
    }

    /// The resolved packages' files (ProjectPackagesState.documents(), at
    /// `packages/<name>/<file>`), as the new engine finds them: each written
    /// into this copy's own `packages/` directory (beside `src/`, never in
    /// the user's project, the package cache or a library) and returned as a
    /// link by file name for `linkTexInputs`, after the texinputs, so a
    /// project file of the same name still wins. The host reads exactly the
    /// text the helper delivered (checked against the cache's SHA-256 when it
    /// was read there), not a path it could be pointed elsewhere by. Files
    /// no longer delivered are removed. A path that is not
    /// `packages/<name>/<file>` with plain names is skipped.
    func materializePackages(_ docs: [ProjectDocuments.ImplicitDocument]) -> [TexInputLink] {
        let fm = FileManager.default
        let dir = base.appendingPathComponent("packages", isDirectory: true)
        var keep = Set<String>()
        var seen = Set<String>()
        var out: [TexInputLink] = []
        for d in docs {
            let parts = d.path.split(separator: "/", omittingEmptySubsequences: false).map(String.init)
            guard parts.count == 3, parts[0] == "packages",
                  parts[1...].allSatisfy({ !$0.isEmpty && $0 != "." && $0 != ".." && !$0.hasPrefix(".") && !$0.contains("\0") }) else {
                // A file in a package's subfolder (or an odd path) is not
                // findable by name the way TEXINPUTS would find it: say so.
                FlashTeXLog.write("engine-v3: package file \(d.path) skipped (only packages/<name>/<file> is linked for the new engine)")
                continue
            }
            let rel = parts[1] + "/" + parts[2]
            let url = dir.appendingPathComponent(rel)
            keep.insert(rel)
            let data = Data(d.text.utf8)
            if (try? Data(contentsOf: url)) != data {
                try? fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
                try? fm.removeItem(at: url)
                try? data.write(to: url)
                try? fm.setAttributes([.posixPermissions: 0o444], ofItemAtPath: url.path) // read-only: the engine only reads it
            }
            guard seen.insert(parts[2]).inserted else { continue } // the first package with that file name wins
            out.append(TexInputLink(name: parts[2], inCopy: nil, external: url.path))
        }
        // Files (and package folders) no longer delivered.
        for pkg in (try? fm.contentsOfDirectory(atPath: dir.path)) ?? [] {
            let pdir = dir.appendingPathComponent(pkg)
            for f in (try? fm.contentsOfDirectory(atPath: pdir.path)) ?? [] where !keep.contains(pkg + "/" + f) {
                try? fm.removeItem(at: pdir.appendingPathComponent(f))
            }
            if ((try? fm.contentsOfDirectory(atPath: pdir.path)) ?? []).isEmpty { try? fm.removeItem(at: pdir) }
        }
        return out
    }

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


/// When a compile is stopped as a runaway (gap A15). Pure.
enum EngineV3StallBound {
    /// `FLASHTEX_V3_STALL_S` (tests) replaces the automatic bound; the
    /// explicit one is then ten times it.
    static var override: Double? { ProcessInfo.processInfo.environment["FLASHTEX_V3_STALL_S"].flatMap(Double.init).map { max($0, 0.5) } }

    /// 30 s after a keystroke's compile; 5 min for ⌘B.
    static func seconds(explicit: Bool) -> Double {
        let automatic = override ?? 30
        return explicit ? (override.map { $0 * 10 } ?? 300) : automatic
    }

    /// Stop only while the host is typesetting a compile (STARTED, no DONE),
    /// runs no external tool (bibtex, biber and makeindex and their
    /// compiles have long silent phases), is not exporting (its own bound),
    /// and has sent nothing for the bound.
    static func shouldStop(typesetting: Bool, toolsRunning: Bool, exporting: Bool, explicit: Bool, silentSeconds: Double) -> Bool {
        typesetting && !toolsRunning && !exporting && silentSeconds > seconds(explicit: explicit)
    }

    static func describe(_ s: Double) -> String {
        s >= 60 && s.truncatingRemainder(dividingBy: 60) == 0 ? "\(Int(s / 60)) min" : "\(Int(s.rounded())) s"
    }
}
