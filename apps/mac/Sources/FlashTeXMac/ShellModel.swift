import AppKit
import Combine
import Observation
import FlashTeXProtocol

/// State for the editor/preview shell. The engine (`flashtex-host`, through
/// `EngineV3Session`) typesets; nothing here compiles LaTeX itself.
/// `@Observable`: SwiftUI tracks the properties each view actually reads, so a
/// keystroke (documents/editorRevision) re-evaluates the editor pane and the
/// footer, and a compile result the preview — not the whole window.
@MainActor
@Observable
final class ShellModel {
    struct Selection: Equatable {
        var path: String
        var nsRange: NSRange
        var token = 0 // bump so the same range re-applies
    }

    var documents: [RuntimeV1.Document] = [] {
        didSet { documentsRevision &+= 1; refreshDocumentMirror(); scheduleUnicodeFontsCheck() }
    }
    /// Advances on every mutation of `documents` — a keystroke, a disk
    /// reload, a project swap — so a reader that derives something from a
    /// document's text (`ProjectDocuments.entryDocumentClass`) can cache by
    /// it. Not observed: it changes per keystroke and no view reads it.
    @ObservationIgnored private(set) var documentsRevision = 0
    var activePath: String = "main.tex" {
        didSet {
            guard activePath != oldValue else { return }
            navigationToken &+= 1; refreshDocumentMirror()
            if engineV3Enabled { engineV3.scheduleCaretMark() }
            projectPackages.caretMoved()
        }
    }
    /// Bumped by every document switch and every `openAndSwitch` request, so a
    /// slow open only switches if nothing navigated after it was requested.
    @ObservationIgnored var navigationToken = 0
    /// Bumped by every `replaceProject` (open, fixture): an asynchronous save
    /// that resumes in a different generation must not touch the new project.
    @ObservationIgnored private(set) var projectGeneration = 0
    /// VoiceOver announcements for the preview (PreviewAnnouncements.swift):
    /// a completed compile, a refused display list, a keyboard page jump.
    @ObservationIgnored let previewAnnouncer = PreviewAnnouncer()
    /// Throttle state of the VoiceOver count announcement (DiagnosticsPanel.swift).
    @ObservationIgnored var diagnosticsAnnouncer = DiagnosticsAnnouncer()
    @ObservationIgnored var diagnosticsAnnouncementFlush: DispatchWorkItem?
    /// Identifies the armed flush timer; a timer whose token is stale does nothing.
    @ObservationIgnored var diagnosticsFlushToken = 0
    /// Count summaries announced (tests and evidence).
    @ObservationIgnored var diagnosticAnnouncements: [String] = []
    var fixtureURL: URL?
    var loadError: String?
    var selection: Selection?
    var navigationNote: String?
    /// Why the last mode switch (ProjectMode.swift) was not made: shown as an
    /// alert, so a refusal is never silent.
    var modeSwitchFailure: String?
    var darkPreview = EditorPreferences.shared.darkPreviewDefault // EditorPreferences.swift: appearance preference
    /// Openers the editor auto-closes (`{`, `[`, `$`, and `(` for `\(`/`\[`); SourceEditorView.
    var autoClosePairs: Set<Character> = ["{", "[", "$", "("]
    // Workspace chrome (ContentView.swift, mac-ui-redesign): the bottom
    // Problems panel, its severity filter, and the command palette sheet.
    // The panel state is shared with the palette so Next/Previous
    // Occurrence work from there too (DiagnosticsPanel.swift).
    var problemsVisible = true
    /// Below the width where editor and preview both fit, the preview
    /// collapses to a toggle instead of being crushed (design-principles §4).
    /// `narrowLayout` mirrors the split's available width; the toggle picks
    /// which column the narrow window shows.
    var narrowLayout = false
    var narrowPreviewShown = false
    var problemsSeverityFilter: RuntimeV1.Severity?
    var commandPaletteShown = false
    /// Rename Symbol / Wrap in Environment / Go to Symbol / Go to Line sheets (ShellModel+EditorNavigation.swift).
    var editorNavigation = EditorNavigationState()
    let problemsPanel = DiagnosticsPanelState()
    /// Debounced, background word/document-statistics scan (GH68), read by
    /// the status bar's word count item (DocumentStatistics.swift). Kicked
    /// off by that view itself on appear/edit/document-set change — no
    /// scheduling logic lives here.
    let wordCount = WordCountModel()
    /// Whether this window's engine session runs (`EngineV3Session`; there
    /// is one engine since the old one retired, #1236 S6). On in the app;
    /// under XCTest off until a test turns it on, so a model made for a test
    /// starts no host process (`EngineV3.enabledAtLaunch`).
    var engineV3Enabled = EngineV3.enabledAtLaunch {
        didSet {
            guard engineV3Enabled != oldValue else { return }
            if engineV3Enabled { engineV3.start(model: self) } else { engineV3.stop(); compiledDocuments = [:] }
        }
    }
    @ObservationIgnored let engineV3 = EngineV3Session()
    /// Where the `engine:` line goes each time a document opens (FLASHTEX_LOG;
    /// tests replace it): the label the attribution tools read (old-engine
    /// retirement plan #1236, stage S3r).
    @ObservationIgnored var engineLog: (String) -> Void = { FlashTeXLog.write($0) }
    /// An open is in progress and its manifest is not read yet: the engine
    /// session is not told about the new project until it is.
    @ObservationIgnored var engineChoicePending = false
    /// The document the open path last resolved (EngineChoice.swift).
    @ObservationIgnored var engineChoiceDocument: URL?
    /// The open document was last typeset by the retired previous engine:
    /// the one-time notice over the preview (EngineChoice.swift,
    /// `PreviousEngineNoticeBanner`; dismissed for good).
    var previousEngineNotice = false
    /// There is no TeX to compile with (EngineChoice.swift, `TeXMissingBanner`;
    /// owner ruling RQ20): said over the preview, never replaced by another engine.
    var texMissing: TeXMissing?
    var texMissingDismissed = false
    /// A Classic run stopped on a package that needs XeTeX or LuaTeX
    /// (UnicodeFonts.swift; until the next open). With it, what the
    /// preamble scan found then: once the scan finds something else (the
    /// user deleted the line), the report is dropped.
    @ObservationIgnored var engineHostNeedsUnicode: (document: URL?, need: UnicodeFontsNeed, scanned: UnicodeFontsNeed?)?
    /// The preamble scan for the open document, by `documentsRevision`
    /// (UnicodeFonts.swift), and the pending re-check after an edit.
    @ObservationIgnored var unicodeFontsScan: (revision: Int, entry: String, need: UnicodeFontsNeed?)?
    @ObservationIgnored var unicodeFontsCheck: DispatchWorkItem?
    /// The project files the scan read (by modification date).
    @ObservationIgnored let unicodeFontsFiles = UnicodeFontsFileCache()
    /// Fallback announcements made (tests; VoiceOver hears them as they are posted).
    @ObservationIgnored var engineAnnouncements: [String] = []
    /// Preview debug status (the engine's counters under the pages, EngineV3Preview.swift):
    /// off by default; View > Show Preview Debug Status. Not kept across launches.
    var previewDebugStatus = false
    /// Preview zoom multiplier over the fit-to-width scale (PreviewZoom.swift); persisted.
    var previewZoom: CGFloat = PreviewZoom.load(.standard) {
        didSet { let c = PreviewZoom.clamped(previewZoom); if c != previewZoom { previewZoom = c } else { PreviewZoom.store(c, in: .standard) } }
    }
    /// Fit-to-width scale the preview pane last laid out with (written by the pane; drives Actual Size and the percentage).
    var previewFitScale: CGFloat = 1
    /// Zoom multiplier that fits the tallest page's height to the pane
    /// (PreviewView reports it with the pane geometry; View > Fit Page).
    var previewFitPageZoom: CGFloat = 1
    /// File backing the entry document, if any, and its last saved contents.
    var documentURL: URL? {
        // Engine-v3 instant reopen: the project's stored pages go on screen
        // now, in this run-loop turn, before the editor ingests the text.
        // While an open is choosing its engine (`engineChoicePending`), the
        // open path tells the session itself once the engine is known.
        didSet { if engineV3Enabled, !engineChoicePending, documentURL != oldValue, let at = engineV3OpenedAt { engineV3.projectChanged(model: self, openedAt: at) } }
    }
    /// When `replaceProject` began (engine-v3 open → pixels timing).
    @ObservationIgnored var engineV3OpenedAt: UInt64?
    var savedText: String?
    var recoverableBuffer: RecoverableBuffer?

    // Capture review / insertion (contract: "Capture and insertion").
    struct PendingEdit: Equatable {
        var path: String; var nsRange: NSRange; var text: String; var token: Int
        /// Editor revision `nsRange` was computed for; the editor refuses the
        /// edit if the buffer moved on (an IME commit, a keystroke) before it
        /// could apply it. nil: not bound (bridge-verified edits).
        var revision: Int? = nil
        /// What to give back when a capture insertion is refused.
        var captureRefund: CaptureRefund? = nil
        /// When non-empty, these replacements (original-text coordinates) are
        /// applied last-first as one undo group instead of `nsRange`/`text`.
        /// Change Environment uses the two name spans so the body is not rewritten.
        var groupedEdits: [EditorKeyHandling.LineEdit] = []
        /// UTF-16 offset (post-edit coordinates) of a closing delimiter the
        /// edit itself placed right after the caret — `\textbf{|}` — which the
        /// editor then tracks like an auto-inserted closer so typing `}` steps
        /// over it (SourceEditorView.applyPendingEdit → pendingClosers).
        var trackedCloser: Int? = nil
    }
    struct CaptureRefund: Equatable { var proposal: RuntimeV1.CaptureProposal; var anchorBefore: InsertionAnchor }
    var caretUTF16: Int = 0 {
        didSet {
            guard caretUTF16 != oldValue else { return }
            caretFollow.noteCaretMove() // CaretFollow.swift: re-arms across a line/page boundary
            if engineV3Enabled { engineV3.scheduleCaretMark() } // the caret on the v3 page, outside SwiftUI (EngineV3CaretMark.swift)
            projectPackages.caretMoved() // ProjectPackages.swift: leaving a half-typed \usepackage{…} shows its sheet
        }
    }
    /// Debounced "the preview follows what you are editing" (CaretFollow.swift).
    /// Triggers: `updateActiveText` (edit), a result or v2 frame landing
    /// (recompile), a caret move, and ⌘⇧J (explicit).
    @ObservationIgnored let caretFollow = CaretFollowController()
    var anchor: InsertionAnchor?
    var proposals: [RuntimeV1.CaptureProposal] = []
    var reviewing: RuntimeV1.CaptureProposal?
    var pendingEdit: PendingEdit?
    /// Tokens only ever grow: the editor remembers the last token it consumed,
    /// so a token that restarted at 1 after `pendingEdit` was cleared (an
    /// applied or refused edit) would never be applied.
    @ObservationIgnored private var editTokens = 0
    func nextEditToken() -> Int { editTokens += 1; return editTokens }
    var captureNote: String?
    /// The save confirmation last shown in `captureNote` (file, exact text,
    /// when), so a disk-status probe that fails right after a save that landed
    /// does not replace it with "Could not check … on disk" (#831; set through
    /// `noteSaveConfirmation`, consulted by `noteProbeFailure`, DocumentFiles.swift).
    @ObservationIgnored var lastSaveConfirmation: (url: URL, note: String, at: Date)?
    /// A watcher event was dropped because a save's reply was still
    /// outstanding after the watcher's single retry; the late reply consumes
    /// it with one more `watcherFired` (#831; DocumentWatcher.swift).
    @ObservationIgnored var pendingWatcherRecheck = false
    var appliedCaptureIDs: Set<String> = []
    /// Past proposal decisions with outcomes (ReviewHistory.swift); persisted
    /// under Application Support (`FLASHTEX_REVIEW_HISTORY_DIR` overrides, `off` = memory only).
    @ObservationIgnored lazy var reviewHistory = ReviewHistoryRecorder(projectId: projectId)
    var nextAnchorNumber = 1
    /// UTF-16 length of the editor selection starting at `caretUTF16` (0 = caret only).
    var caretLengthUTF16: Int = 0
    // Capture bridge (contract: transfer-v1). See ShellModel+Bridge.swift.
    var bridgeStatus: String = "no bridge attached" { didSet { FlashTeXLog.write("bridge: " + bridgeStatus) } }
    var bridgeCaptures: [BridgeSession.Capture] = []
    var bridgeDestination: TransferV1.Anchor?
    private(set) var bridge: BridgeSession?
    /// Deadline for each `capture_status` during restart reconciliation.
    var bridgeStatusTimeout: TimeInterval = 15

    // MARK: change-only mirrors for the window toolbar (ContentView.WorkspaceToolbar)
    //
    // The toolbar's body read `displayedDiagnostics` directly, so every
    // compile re-evaluated the toolbar content and re-laid out the NSToolbar
    // with the whole window (typing-bench sample, 2026-09-13: the toolbar
    // preference update and NSToolbarItemViewer layout were ~20% of a
    // saturated main thread while typing; app code < 2%). These mirrors are
    // assigned only when their value changes, so observation fires for the
    // toolbar exactly when something it shows changes.
    private(set) var toolbarProblemCount = 0
    /// Page under the top of the preview viewport, reported by the pane
    /// (`PreviewAnchorKeeper`). The page readout in the preview HUD reads this.
    var previewVisiblePage = 1
    /// Pages the preview shows (`EngineV3Session.pageCount`). The HUD's
    /// "N / M" readout and the pane's accessibility value both count these.
    var previewPageCount: Int { engineV3.pageCount }
    /// `!documents.isEmpty`, change-only: File > Print Source… must not read
    /// `documents` from the App scene (a keystroke reassigns the array).
    private(set) var toolbarHasDocument = false
    /// `documents.count`, change-only: the word-count item names "this
    /// document" or "N open documents" and rescans when a document opens or
    /// closes, without reading `documents` (a keystroke reassigns it).
    private(set) var documentCount = 0
    /// `project.entryPath`, change-only, for File > Move To…'s enabled state:
    /// `entryPath` reads `documents`, so the App scene (every menu) was
    /// re-evaluated on every keystroke (P5-KEYSTROKE-MAIN).
    private(set) var menuEntryPath = "main.tex"
    /// `displayedDiagnostics` assigned only when the list changes, for the
    /// Problems panel, so a compile that changes nothing it lists does not
    /// re-evaluate the panel's List (an AppKit table) while typing.
    private(set) var problemsList: [RuntimeV1.Diagnostic] = []
    /// The last compile's status (`engineV3ResultStatus`), change-only, for the Problems panel header.
    private(set) var resultStatus: RuntimeV1.Status?
    /// Throttled mirror the status bar / preview header / tab bar read (ShellChrome.swift).
    let chrome = ShellChrome()
    @ObservationIgnored private var chromeRefreshPending = false

    /// Refreshes the chrome mirror from the current state and re-arms the
    /// tracking: the first later change to anything the refresh read
    /// schedules the next refresh one `ShellChrome.interval` later.
    private func refreshChrome() {
        var again = false
        withObservationTracking {
            again = MainThreadProbe.time("chrome.refresh") { chrome.refresh(from: self) }
        } onChange: { [weak self] in
            // Observation calls this at the mutation (main actor: every writer is).
            MainActor.assumeIsolated { self?.scheduleChromeRefresh() }
        }
        if again { scheduleChromeRefresh() }
    }

    private func scheduleChromeRefresh() {
        guard !chromeRefreshPending else { return }
        chromeRefreshPending = true
        // A run-loop timer, not the main dispatch queue: AppKit drains that
        // queue late under typing (TypingBench.nextRunLoopTurn).
        let timer = Timer(timeInterval: ShellChrome.interval, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.chromeRefreshPending = false
                self.refreshChrome()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
    }

    /// Applies pending chrome changes now (tests, and the bench's paint point).
    func flushChrome() { chromeRefreshPending = false; chrome.settleNext = true; refreshChrome() }

    private func refreshDocumentMirror() {
        let has = !documents.isEmpty
        if toolbarHasDocument != has { toolbarHasDocument = has }
        if documentCount != documents.count { documentCount = documents.count }
        let entry = documents.first?.path ?? activePath // ProjectDocuments.entryPath
        if menuEntryPath != entry { menuEntryPath = entry }
    }

    private func refreshToolbarMirrors() {
        let diagnostics = displayedDiagnostics
        if toolbarProblemCount != diagnostics.count { toolbarProblemCount = diagnostics.count }
        if problemsList != diagnostics { problemsList = diagnostics }
        // A compile that failed (no pages) keeps the last pages (the Problems line, gap B8).
        if resultStatus != engineV3ResultStatus { resultStatus = engineV3ResultStatus }
        refreshDocumentMirror()
    }
    let nearbyInbox = NearbyInbox() // captures from paired companions (ShellModel+Nearby.swift)
    let captureInbox = CaptureInbox() // Captures inspector rows (CaptureInbox.swift)
    var captureInboxVisible = ProcessInfo.processInfo.environment["FLASHTEX_SHOW_CAPTURES"] == "1" // View > Captures (⌘⇧I)
    /// The window's log lines (`log`), newest last; at most 200.
    var workerLog: [String] = []
    /// Project-index completion vocabulary (labels/citations/commands) bound to
    /// the editor revision it was fetched for (Completion.swift).
    var completionMetadata: Completion.Metadata?
    /// Text each document had when the engine's last completed compile read
    /// it (EngineV3Session, at DONE), so diagnostics' byte offsets can be
    /// rebased (or refused) after edits.
    private(set) var compiledDocuments: [String: String] = [:]

    /// The engine's diagnostics of its last completed compile.
    var displayedDiagnostics: [RuntimeV1.Diagnostic] { engineV3Diagnostics }
    /// The engine-v3 preview's last completed compile's TeX errors and
    /// warnings (EngineV3Session: `DIAGNOSTIC` messages; diag-v1 later),
    /// shown in the Problems panel and the status bar while the flag is on.
    /// The last v3 compile's outcome for the Problems line: `.failed` when
    /// TeX produced no pages (the previous ones are kept), else nil.
    var engineV3ResultStatus: RuntimeV1.Status? {
        didSet { if engineV3ResultStatus != oldValue { refreshToolbarMirrors() } }
    }
    var engineV3Diagnostics: [RuntimeV1.Diagnostic] = [] {
        didSet { if engineV3Diagnostics != oldValue { engineV3MarksRevision &+= 1; refreshToolbarMirrors() } }
    }
    /// Bumped when the v3 rows or their compiled texts change: the editor
    /// marks' memo key under engine v3 (there is no old-engine result id).
    /// Observed: a memo hit reads only this, so a view that drew the marks
    /// must be invalidated when a DONE brings new rows or a new baseline.
    private(set) var engineV3MarksRevision = 0
    /// The editor revision of the texts the last v3 compile read, or nil
    /// when the editor no longer has them (set at DONE).
    private(set) var engineV3CompiledEditorRevision: Int?
    /// The revision a diagnostic's mechanical fix was computed against:
    /// "Fix…" and Tab apply it only while the editor is at that revision.
    var fixCompiledRevision: Int? { engineV3CompiledEditorRevision }

    /// What the editor marks (underlines, gutter, Error Lens, ⌘⇧]/[) are built
    /// from: the host's rows of the last completed compile as one result, with
    /// `compiledDocuments` (set at its DONE) as the texts they rebase from.
    var markSource: (result: RuntimeV1.CompileResult, id: String?)? {
        let failed = engineV3Diagnostics.contains { $0.severity == .error }
        return (RuntimeV1.CompileResult(projectId: "engine-v3", revision: engineV3MarksRevision, status: failed ? .recovered : .ok,
                                        pages: [], diagnostics: engineV3Diagnostics, pdfPath: nil), nil)
    }

    /// Settings > Compile > Auto-compile after edits: whether edits go to
    /// the host as they are typed; turning it back on sends what was typed
    /// meanwhile.
    var autoCompile = true {
        didSet { if autoCompile, !oldValue, engineV3Enabled { engineV3.textChanged(model: self) } }
    }
    /// Settings > Compile > Stop at the first error (EngineV3ErrorPolicy):
    /// off (the default) is best effort, nonstopmode with the errors TeX
    /// recovers from shown as warnings; on is `-halt-on-error`. Kept in the
    /// engine-v3 defaults (a test suite of its own under XCTest).
    var strictTeXErrors = EngineV3ErrorPolicy.storedMode == .strict {
        didSet {
            guard strictTeXErrors != oldValue else { return }
            EngineV3.defaults.set(strictTeXErrors, forKey: EngineV3ErrorPolicy.strictKey)
            engineV3.errorMode = strictTeXErrors ? .strict : .bestEffort
            if engineV3Enabled { engineV3.compile(model: self, reason: "error mode") }
        }
    }
    @ObservationIgnored private var autosaveWork: DispatchWorkItem?
    /// Asynchronous (helper-routed) saves per document path — autosave and
    /// Command-S alike — that have not answered yet (`enqueueSave`).
    @ObservationIgnored private var savesInFlight: [String: (id: Int, task: Task<Void, Never>)] = [:]
    @ObservationIgnored private var saveSequence = 0
    /// Quiet time after the last edit before autosave writes to disk
    /// (`EditorPreferences.autosave`, owner: "autosave should be on by
    /// default"). `FLASHTEX_AUTOSAVE_MS` overrides; 0 makes it synchronous.
    static let autosaveInterval: TimeInterval = {
        if let s = ProcessInfo.processInfo.environment["FLASHTEX_AUTOSAVE_MS"], let ms = Double(s) { return max(0, ms) / 1000 }
        return 2
    }()
    /// True under XCTest with no explicit override: a live several-second
    /// background timer could fire mid-test and write a file a *different*
    /// test is asserting about (same reasoning as `PreviewHUD.lingerSuppressed`
    /// and `ThinSplitViewController.autosaveEnabled`). `scheduleAutosave`
    /// still records the pending save; a test exercises it deterministically
    /// through `flushPendingAutosave()` instead of waiting out real time.
    static let autosaveSuppressedUnderTest: Bool = {
        guard ProcessInfo.processInfo.environment["FLASHTEX_AUTOSAVE_MS"] == nil else { return false }
        return runningUnderXCTest
    }()
    /// True under XCTest, whichever of the two variables the runner sets
    /// (shared with `PreviewHUD.lingerSuppressed` / `ThinSplitViewController`).
    static let runningUnderXCTest = ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] != nil
        || ProcessInfo.processInfo.environment["XCTestSessionIdentifier"] != nil
    /// Revision the editor buffer corresponds to. Bumps on every edit so the
    /// UI can say when the preview's source ranges no longer match the buffer.
    private(set) var editorRevision = 1

    /// Project ID the bridge and the review history use. It was the old
    /// engine's compile project id, which was always `demo`; it stays that
    /// constant, so captures and destination pins already stored under it
    /// keep matching (app-parity row E2, ruled the same on both routes,
    /// DESIGN.md §13 2026-10-21). Bridge IDs must be 1–128 `[A-Za-z0-9_-]`.
    var projectId: String { "demo" }

    func setBridge(_ session: BridgeSession?) {
        bridge?.terminate()
        bridge = session
        bridgeDestination = nil
        bridgeCaptures = session?.captures ?? []
        bridgeStatus = session?.status ?? "no bridge attached"
    }

    /// Restart reconciliation may need the editor revision to pass a revision
    /// the bridge already confirmed; revisions only ever advance.
    func advanceEditorRevision(atLeast revision: Int) {
        guard revision > editorRevision else { return }
        editorRevision = revision
    }

    var activeText: String {
        get { documents.first { $0.path == activePath }?.text ?? "" }
    }

    /// The entry document's buffer (the text of `documentURL`), whichever tab
    /// is active: what saves, discards and snapshots of the entry file use.
    var entryText: String { documents.first { $0.path == project.entryPath }?.text ?? activeText }

    /// Diagnostic underlines for the active document, rebased across edits or
    /// dropped (see `EditorDiagnostics`).
    var editorMarks: [EditorDiagnostics.Mark] { editorMarkReport.marks }

    /// Marks plus the diagnostics withheld after an edit; `staleNote` is
    /// shown in the footer. Memoized: ContentView reads this on every body
    /// evaluation and the rebase compares the compiled and current texts.
    var editorMarkReport: EditorDiagnostics.Report {
        projectPackages.withholding(unheldMarkReport) // ProjectPackages.swift: a half-typed package name is not underlined yet
    }

    private var unheldMarkReport: EditorDiagnostics.Report {
        let key = EditorMarksKey(resultRevision: engineV3MarksRevision, editorRevision: editorRevision, path: activePath)
        if let cached = editorMarksCache, cached.key == key { return cached.report }
        let report = diagnosticReport(for: activePath, currentText: activeText)
        editorMarksCache = (key, report)
        return report
    }
    private struct EditorMarksKey: Equatable {
        var resultRevision: Int; var editorRevision: Int; var path: String
    }

    /// The report for `path` (any open document): what `editorMarkReport`
    /// memoizes for the active document and what diagnostic navigation reads
    /// for the others. The host's rows of the last completed compile, rebased
    /// from the texts that compile read (set at DONE); a failed compile still
    /// reports its rows, so there is nothing to carry over.
    func diagnosticReport(for path: String, currentText: String) -> EditorDiagnostics.Report {
        guard let source = markSource else { return .empty }
        return EditorDiagnostics.report(for: source.result, resultID: source.id, path: path,
                                        compiledText: compiledDocuments[path], currentText: currentText)
    }
    @ObservationIgnored private var editorMarksCache: (key: EditorMarksKey, report: EditorDiagnostics.Report)?
    /// Identity of the mark last reached by ⌘⇧]/⌘⇧[, so marks sharing a
    /// start offset are each visited once (Navigation.swift).
    @ObservationIgnored var currentDiagnosticID: String?

    /// A reviewed quick fix being previewed (sheet); nil when none.
    var quickFix: EditorDiagnostics.QuickFix.Preview?
    var quickFixIndex: Int?

    /// "Fix…" on a diagnostics row: prepare the diagnostic's mechanical fix
    /// against the current buffer and show the preview; refusals go to the footer.
    func previewQuickFix(diagnosticIndex: Int, suggestion: Int = 0) {
        let diags = displayedDiagnostics
        guard diags.indices.contains(diagnosticIndex) else { navigationNote = "No fix for this diagnostic."; return }
        let d = diags[diagnosticIndex]
        guard EditorDiagnostics.canApplyHelpReplacement(d, path: activePath, currentText: activeText,
                                                        compiledRevision: fixCompiledRevision, editorRevision: editorRevision) else {
            navigationNote = "No fix for this diagnostic."; return
        }
        let compiled = compiledDocuments[activePath] ?? activeText
        switch EditorDiagnostics.prepareHelpReplacement(d, path: activePath, in: activeText, compiledText: compiled) {
        case .success(let preview): quickFix = preview; quickFixIndex = diagnosticIndex; navigationNote = nil
        case .failure(let why): quickFix = nil; navigationNote = "Fix not applied: " + why.text
        }
    }

    /// "Apply" in the preview sheet: one grouped replacement through the
    /// existing pendingEdit path (single undoable edit, never automatic).
    func applyQuickFix() {
        guard let preview = quickFix else { return }
        let grouped = preview.apply()
        guard grouped.path == activePath, grouped.matches(activeText) else {
            navigationNote = "Fix not applied: the document changed since the preview; open Fix… again."
            quickFix = nil; return
        }
        pendingEdit = .init(path: grouped.path, nsRange: grouped.nsRange, text: grouped.text,
                            token: nextEditToken(), revision: editorRevision)
        navigationNote = "Applied: \(preview.summary) (undo with ⌘Z)"
        quickFix = nil
    }

    // MARK: the fix at the caret (Tab)

    /// Set by Esc while a caret fix is showing, so the hint stays down until
    /// the caret moves somewhere else. Dismissing must also hand Tab straight
    /// back to indentation — that is the whole point of Esc here.
    @ObservationIgnored private var dismissedCaretFix: EditorDiagnostics.CaretFix?

    /// The mechanical fix offered where the caret is, or `nil`.
    ///
    /// This is the single source of truth for both halves of the feature: the
    /// editor draws a hint exactly when it is non-nil, and Tab accepts a fix
    /// exactly when it is non-nil. They cannot disagree, so Tab can never
    /// silently do something the author was not shown.
    var caretFix: EditorDiagnostics.CaretFix? {
        let diagnostics = displayedDiagnostics
        // Cheap bail-out before `caretByte`, whose UTF-16 → UTF-8 conversion is
        // linear in the document: this is read on every keystroke, and most
        // documents carry no mechanical fix at all.
        guard fixCompiledRevision == editorRevision,
              diagnostics.contains(where: { $0.help?.replacement != nil || $0.suggestion != nil })
        else { return nil }
        guard let caretByte, let fix = EditorDiagnostics.fixOffered(
            at: caretByte, in: diagnostics, path: activePath,
            currentText: activeText, compiledRevision: fixCompiledRevision,
            editorRevision: editorRevision
        ) else { return nil }
        return fix == dismissedCaretFix ? nil : fix
    }

    /// Esc: take the hint down and leave Tab alone until the caret moves onto
    /// a different fix.
    func dismissCaretFix() {
        guard let fix = caretFix else { return }
        dismissedCaretFix = fix
    }

    /// Tab on a visible caret fix. Routed through `previewQuickFix` /
    /// `applyQuickFix` rather than a second application path, so the edit
    /// ledger, the revision counter and undo behave exactly as they do for
    /// "Fix…" in the Problems panel — one undo step.
    func acceptCaretFix() {
        guard let fix = caretFix else { return }
        previewQuickFix(diagnosticIndex: fix.diagnosticIndex)
        guard quickFix != nil else { return } // refusal already in the footer
        applyQuickFix()
        dismissedCaretFix = nil
    }

    // MARK: caret sync (source -> preview)

    /// UTF-8 byte offset of the editor caret in `activeText`, or nil when the
    /// UTF-16 caret is out of range for the buffer.
    var caretByte: Int? {
        CaretSync.byteOffset(ofCaretUTF16: caretUTF16, in: activeText) // mid-surrogate carets rounded, never split
    }

    /// What a new model holds before anything is opened: the protocol
    /// contract fixture's documents (`protocol/fixtures/compile-request.json`,
    /// the default: tests and automation are written against it), or — the
    /// app's window — the untitled LaTeX document (`UntitledDocument`,
    /// ProjectScaffold.swift).
    enum Startup { case fixture, untitledDocument }

    init(startup: Startup = .fixture) {
        let env = ProcessInfo.processInfo.environment
        // Caret following asks for the target only when a follow actually
        // fires, so this closure runs at most once per debounce interval.
        caretFollow.target = { [weak self] in self?.caretPreviewTarget() }
        // Only read while disarmed (a manual scroll happened): the line the
        // caret is on, so a move to another line re-arms following.
        caretFollow.currentLine = { [weak self] in
            guard let self, let byte = self.caretByte else { return nil }
            return EditorDiagnostics.lineNumber(ofByte: byte, in: self.activeText)
        }
        defer {
            // Demo/automation hook: seed the editor from a .tex file.
            if let seed = env["FLASHTEX_SEED_FILE"], let text = try? String(contentsOfFile: seed, encoding: .utf8) {
                replaceProject(entryText: text, named: URL(fileURLWithPath: seed).lastPathComponent)
                documentURL = URL(fileURLWithPath: seed)
                savedText = text
            }
            let bundledBridge = Bundle.main.executableURL?.deletingLastPathComponent()
                .appendingPathComponent("flashtex-bridge").path
            let hasBundledBridge = bundledBridge.map { FileManager.default.isExecutableFile(atPath: $0) } ?? false
            if env["FLASHTEX_AUTOATTACH"] != "0", (env["FLASHTEX_AUTOATTACH"] == "1" || hasBundledBridge),
               BridgeClient.locateBridge() != nil {
                attachDiscoveredBridge()
            }
            refreshChrome() // ShellChrome.swift: from here on the chrome follows the model, throttled
        }
        if startup == .untitledDocument {
            loadUntitledDocument()
        } else if let fixtures = Self.locateFixturesDirectory() {
            loadFixtures(request: fixtures.appendingPathComponent("compile-request.json"),
                         result: fixtures.appendingPathComponent("compile-result.json"))
        } else {
            loadError = "Could not locate protocol/fixtures (set FLASHTEX_REPO or use File > Open)."
            documents = [.init(path: "main.tex", text: "")]
        }
    }

    // MARK: loading

    /// The window FlashTeX opens with no file (`Startup.untitledDocument`):
    /// File › New Project…'s blank article, caret on the empty line under
    /// `\section{Introduction}`. Like the fixture it replaces, it is fresh —
    /// revision 1, no file, no saved text — so it is dirty only once edited
    /// and quitting or opening a file asks nothing. The engine compiles it
    /// like any other buffer (the pane's first compile).
    /// `UntitledDocument.caretUTF16` while the window still shows the untouched
    /// untitled document (the editor view places its caret there when made),
    /// else nil.
    var untitledDocumentCaret: Int? {
        guard documentURL == nil, activePath == UntitledDocument.path,
              activeText.utf8.count == UntitledDocument.text.utf8.count, activeText == UntitledDocument.text else { return nil }
        return UntitledDocument.caretUTF16
    }

    private func loadUntitledDocument() {
        documents = [.init(path: UntitledDocument.path, text: UntitledDocument.text)]
        activePath = UntitledDocument.path
        compiledDocuments = [:]
        caretUTF16 = UntitledDocument.caretUTF16
    }

    /// Seeds the project from a runtime-v1 fixture pair: the documents,
    /// entry and revision of `request` (or, for `<name>-result.json`, a
    /// sibling `<name>-request.json`, e.g. Samples/multipage-*.json). The
    /// result must be a valid `compile_result`; nothing of it is shown (the
    /// engine typesets the documents). `beforeReplacing` runs once the
    /// fixture is valid, just before the project is replaced; returning
    /// false replaces nothing (#806).
    func loadFixtures(request: URL?, result: URL, beforeReplacing: () -> Bool = { true }) {
        loadError = nil
        do {
            _ = try RuntimeV1.decodeCompileResult(Data(contentsOf: result))
            var candidates: [URL] = []
            if let request { candidates.append(request) }
            if let sibling = Self.siblingRequestURL(forResult: result) { candidates.append(sibling) }
            let req = candidates.lazy.compactMap({ url -> RuntimeV1.Envelope<RuntimeV1.CompileRequest>? in
                guard let data = try? Data(contentsOf: url) else { return nil }
                return try? RuntimeV1.decodeCompileRequest(data)
            }).first
            guard beforeReplacing() else { return }
            // Fixtures replace the whole project: detach any real file identity
            // first so Save can never write fixture content over the user's
            // document, and stop watching the file that's no longer open (#72).
            documentURL = nil
            savedText = nil
            files.conflict = nil
            watchOpenDocument()
            resetDiagnosticsAnnouncer() // DiagnosticsPanel.swift: the old project's spoken counts and timer do not carry over
            self.fixtureURL = result
            if let req {
                documents = req.payload.documents
                activePath = req.payload.entryPath
                editorRevision = req.payload.revision
            } else if documents.isEmpty {
                documents = [.init(path: "main.tex", text: "")]
            }
            compiledDocuments = [:]
            selection = nil
            navigationNote = nil
        } catch {
            loadError = "Failed to load \(result.lastPathComponent): \(error)"
        }
    }

    /// `<dir>/<name>-request.json` for a result named `<name>-result.json`, else nil.
    static func siblingRequestURL(forResult result: URL) -> URL? {
        let name = result.deletingPathExtension().lastPathComponent
        guard name.hasSuffix("-result"), result.pathExtension == "json" else { return nil }
        let stem = String(name.dropLast("-result".count))
        return result.deletingLastPathComponent().appendingPathComponent("\(stem)-request.json")
    }

    // MARK: editing

    /// Replaces the whole project with one entry document (File > Open).
    /// `entryName` is the opened file's actual name: the compile request's
    /// `entry_path`, every tab/diagnostic/caret path, and the helper's rooted
    /// project all key off it, so opening `paper.tex` must not read as
    /// `main.tex`. The default covers unsaved buffers with no file behind them.
    func replaceProject(entryText text: String, named entryName: String = "main.tex") {
        projectGeneration &+= 1
        let entryName = entryName.isEmpty ? "main.tex" : entryName
        documents = [.init(path: entryName, text: text)]
        activePath = entryName
        compiledDocuments = [:]
        fixtureURL = nil
        unicodeFontsScan = nil // the new project's preamble is scanned afresh (UnicodeFonts.swift)
        resetDiagnosticsAnnouncer() // DiagnosticsPanel.swift: the old project's spoken counts and timer do not carry over
        selection = nil
        anchor = nil
        editorRevision += 1
        project.clearSaveConflicts() // a new project inherits no member conflicts
        bridgeDocumentReplaced()
        // The engine compiles the new project once its file URL is set (the
        // caller assigns `documentURL` right after this returns).
        if engineV3Enabled {
            let openedAt = MonotonicClock.nowNs()
            engineV3OpenedAt = openedAt
            guard !engineChoicePending else { return } // the open path tells the session once its engine is chosen
            // The caller sets `documentURL` next (its didSet shows the stored
            // pages at once); this catches a replacement that keeps the URL.
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                self.engineV3.projectChanged(model: self, openedAt: openedAt)
            }
        }
    }

    /// The engine-v3 compile whose diagnostics are shown was made from these
    /// texts (EngineV3Session, at DONE): the baseline that the Problems panel's
    /// line labels and navigation rebase from, as an old result's request
    /// text was.
    func setEngineV3CompiledDocuments(_ docs: [String: String]) {
        // The editor revision the compile's texts are, when the editor still
        // has them: a mechanical fix applies only then (`fixCompiledRevision`).
        let current = documents.allSatisfy { doc in docs[doc.path].map { $0.sameBytes(as: doc.text) } ?? true }
        engineV3CompiledEditorRevision = current ? editorRevision : nil
        guard compiledDocuments != docs else { return }
        compiledDocuments = docs
        engineV3MarksRevision &+= 1
    }

    func updateActiveText(_ text: String) {
        if engineV3Enabled { MainThreadProbe.time("model.v3TextChanged") { engineV3.textChanged(model: self, activeText: text) } } // EngineV3Session.swift: first, before the model's own work
        guard let i = documents.firstIndex(where: { $0.path == activePath }) else { return }
        guard !MainThreadProbe.time("model.compare", { documents[i].text.sameBytes(as: text) }) else { return }
        let old = documents[i].text, base = editorRevision
        MainThreadProbe.time("model.store") { documents[i].text = text }
        MainThreadProbe.time("model.revision") { editorRevision += 1 }
        MainThreadProbe.time("model.typingBench") { TypingBench.shared.noteRevision(editorRevision) } // keystroke -> paint instrumentation
        MainThreadProbe.time("model.autosave") { scheduleAutosave() }
        MainThreadProbe.time("model.caretFollow") { caretFollow.note(.edit) } // CaretFollow.swift: an edit also re-arms following after a manual scroll
        MainThreadProbe.time("model.bridge") { bridgeTextChanged(path: activePath, old: old, new: text, base: base, revision: editorRevision) }
    }

    /// Writes every dirty, file-backed document to disk a quiet moment after
    /// the last edit (`EditorPreferences.autosave`, default on — owner:
    /// "autosave should be on by default"). The timer is shared: any edit
    /// re-arms it, and when it fires it saves *all* dirty documents (entry
    /// and project members, active or not), so switching tabs or typing in
    /// another document never drops a pending save. Never touches a buffer
    /// with no file yet (`documentURL == nil`) — `saveTex()` would otherwise
    /// fall back to `saveTexAs()` and pop a Save panel mid-typing.
    func scheduleAutosave() {
        autosaveWork?.cancel()
        autosaveWork = nil
        guard EditorPreferences.shared.autosave, documentURL != nil else { return }
        let item = DispatchWorkItem { [weak self] in self?.performAutosave() }
        autosaveWork = item
        guard !Self.autosaveSuppressedUnderTest else { return } // flushPendingAutosave() performs it, on demand
        if Self.autosaveInterval == 0 { flushPendingAutosave(); return }
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.autosaveInterval, execute: item)
    }

    /// Saves each dirty document through its normal conflict-checked save
    /// path (the same ones Command-S uses), minus any panel: a refused save
    /// keeps the buffer, records the conflict, and is not retried until the
    /// conflict is resolved. A document with a save still in flight is
    /// skipped here; `enqueueSave` re-arms autosave if it was edited meanwhile.
    private func performAutosave() {
        autosaveWork = nil
        guard EditorPreferences.shared.autosave, documentURL != nil else { return }
        let entry = project.entryPath
        if project.isDirty(entry), files.conflict == nil, savesInFlight[entry] == nil {
            saveEntryTex()
        }
        for path in documents.map(\.path) where path != entry && project.isDirty(path) && savesInFlight[path] == nil {
            if project.saveConflict(for: path) != nil { continue } // not retried until that member's conflict is resolved (#789)
            _ = project.saveDocumentNow(path)
        }
    }

    /// Runs `save` for `path` only after any save of that path already in
    /// flight has answered, so autosave and Command-S never race two exports
    /// carrying the same (soon stale) disk expectation — the second would be
    /// refused as a spurious conflict. If the buffer changed while the save
    /// ran (autosave skipped it as in flight), autosave is re-armed so those
    /// edits reach disk too; an unchanged buffer is not retried.
    @discardableResult
    func enqueueSave(_ path: String, _ save: @escaping @MainActor () async -> Void) -> Task<Void, Never> {
        let previous = savesInFlight[path]?.task
        saveSequence += 1
        let id = saveSequence
        let task = Task { @MainActor [weak self] in
            await previous?.value
            let before = self?.documents.first(where: { $0.path == path })?.text
            await save()
            guard let self else { return }
            if savesInFlight[path]?.id == id { savesInFlight[path] = nil }
            if let before, let now = documents.first(where: { $0.path == path })?.text,
               !now.sameBytes(as: before), project.isDirty(path) {
                scheduleAutosave()
            }
        }
        savesInFlight[path] = (id, task)
        return task
    }

    /// Performs a pending autosave immediately instead of waiting out
    /// `autosaveInterval` (real time is suppressed under XCTest, see
    /// `autosaveSuppressedUnderTest`, so tests exercise the write through
    /// here rather than a live timer that could race the test). A no-op when
    /// no edit scheduled one.
    func flushPendingAutosave() {
        guard let pending = autosaveWork else { return }
        pending.cancel()
        performAutosave()
    }

    // MARK: navigation (preview -> source)

    /// Converts the contract's UTF-8 byte range to a UTF-16 selection in the
    /// matching document and asks the editor to select it.
    /// Preview/diagnostic navigation: see `navigateExactly` (Navigation.swift)
    /// for the byte-exact staleness and cluster guarantees.
    func navigate(to source: RuntimeV1.SourceRange?) {
        navigateExactly(to: source, expectedText: nil)
    }

    /// `expectedText` (an item's text) annotates generated text after a rebase.
    func navigate(to source: RuntimeV1.SourceRange, expectedText: String?) {
        navigateExactly(to: source, expectedText: expectedText)
    }

    // MARK: compiling

    /// ⌘B, the title bar's ▶ and the palette's Compile: the engine compiles
    /// now (and, after the host stopped, restarts).
    func compileCommand() {
        projectPackages.explicitRequest(compiling: true) // ProjectPackages.swift: ⌘B ends the typing grace period
        engineV3.compileNow(model: self)
    }

    /// Whether ⌘B and the auto-compile setting have an engine to drive.
    var canCompile: Bool { engineV3Enabled }

    /// Whether Export PDF… and Print… are enabled: the engine's export.
    var exportAvailable: Bool { engineV3Enabled && engineV3.exportAvailable }

    /// An unopened file the project reads changed on disk (ProjectDocuments'
    /// include watchers): the engine compiles again.
    func implicitFilesChanged() {
        if engineV3Enabled { engineV3.compileNow(model: self, reason: "files") }
    }

    /// Documents (path → text) the engine's last compile read.
    func setCompiledDocuments(_ docs: [String: String]) { compiledDocuments = docs }

    func log(_ line: String) {
        FlashTeXLog.write(line)
        workerLog.append(line)
        if workerLog.count > 200 { workerLog.removeFirst(workerLog.count - 200) }
    }

    // MARK: capture review and insertion

    /// Pins the current caret as the insertion destination (`destination_id`).
    func pinAnchorAtCaret() {
        guard let anchor = Insertion.makeAnchor(id: "mac-anchor-\(nextAnchorNumber)", path: activePath,
                                                text: activeText, caretUTF16: caretUTF16, revision: editorRevision)
        else { captureNote = "Caret position is not valid."; return }
        nextAnchorNumber += 1
        self.anchor = anchor
        captureNote = "Pinned \(anchor.id) at \(anchor.path) byte \(anchor.byteOffset) (revision \(anchor.revision))."
        bridgePin(anchor)
    }

    func openProposalPanel() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.message = "Choose a runtime v1 capture_proposal JSON file"
        if panel.runModal() == .OK, let url = panel.url { loadProposal(from: url) }
    }

    func loadProposal(from url: URL) {
        do {
            let env = try RuntimeV1.decodeCaptureProposal(Data(contentsOf: url))
            enqueue(env.payload)
        } catch {
            captureNote = "Failed to load proposal: \(error)"
        }
    }

    /// Queues a proposal for review. Repeated capture IDs never insert twice.
    func enqueue(_ proposal: RuntimeV1.CaptureProposal) {
        if appliedCaptureIDs.contains(proposal.captureId) {
            captureNote = "Capture \(proposal.captureId) was already inserted; ignoring duplicate."
            return
        }
        if proposals.contains(where: { $0.captureId == proposal.captureId }) {
            captureNote = "Capture \(proposal.captureId) is already awaiting review."
            return
        }
        proposals.append(proposal)
        captureNote = "Proposal \(proposal.captureId) awaiting review (\(proposals.count) queued)."
        if reviewing == nil { reviewing = proposals.first }
    }

    /// Closes the review sheet without deciding anything.
    ///
    /// The sheet is window-modal, so while it is up every other control in
    /// the window — including the Captures inspector's "Insert at caret" —
    /// is unclickable. Before this existed the only ways out were Reject
    /// (destructive) and a successful Approve, so a proposal the reviewer
    /// wanted to insert from the inspector instead could not be reached at
    /// all: the inspector's Insert is enabled only while the proposal is in
    /// `proposals`, which is exactly when the blocking sheet is raised.
    ///
    /// The proposal stays queued and insertable; only the sheet goes away.
    func dismissReviewWithoutDeciding() {
        guard let p = reviewing else { return }
        reviewing = nil
        captureNote = "Review of \(p.captureId) closed; it is still queued — insert it from the Captures inspector (⌘⇧I)."
    }

    func rejectProposal(_ proposal: RuntimeV1.CaptureProposal) {
        proposals.removeAll { $0.captureId == proposal.captureId }
        if reviewing?.captureId == proposal.captureId { reviewing = proposals.first }
        captureNote = "Rejected \(proposal.captureId)."
        reviewHistory.recordRejected(proposal, editorRevision: editorRevision)
        bridgeReject(captureId: proposal.captureId)
    }

    enum ApproveOutcome: Equatable { case inserted(byteOffset: Int), needsReselection(String), duplicate, noAnchor, refused(String) }

    /// Applies one undoable edit after explicit approval. `latex` may have been
    /// edited by the reviewer. Returns what happened so the UI can explain it.
    /// Bridge captures go through `approveBridgeProposal` (prepared edits are
    /// verified against the bridge), never through this local path.
    @discardableResult
    func approveProposal(_ proposal: RuntimeV1.CaptureProposal, latex: String) -> ApproveOutcome {
        let outcome = approveProposalCore(proposal, latex: latex)
        reviewHistory.record(outcome, proposal: proposal, latex: latex, path: anchor?.path ?? activePath, editorRevision: editorRevision)
        return outcome
    }

    private func approveProposalCore(_ proposal: RuntimeV1.CaptureProposal, latex: String) -> ApproveOutcome {
        guard !appliedCaptureIDs.contains(proposal.captureId) else {
            rejectProposal(proposal); captureNote = "Capture \(proposal.captureId) already inserted."; return .duplicate
        }
        guard !isBridgeCapture(proposal.captureId) else {
            captureNote = "Capture \(proposal.captureId) belongs to the bridge; approve it through the bridge review."
            return .refused("bridge capture")
        }
        guard let anchor else {
            captureNote = "Pin an insertion point first (Edit > Pin Insertion Point)."; return .noAnchor
        }
        guard let doc = documents.first(where: { $0.path == anchor.path }) else {
            captureNote = "Anchor document \(anchor.path) is not open."; return .needsReselection("document closed")
        }
        let byte: Int
        switch Insertion.resolve(anchor, in: doc.text, revision: editorRevision) {
        case .exact(let b), .rebased(let b): byte = b
        case .needsReselection(let why):
            self.anchor = nil
            captureNote = "Cannot insert \(proposal.captureId): \(why). Pin a new insertion point."
            return .needsReselection(why)
        }
        // Make the proposal legal where it is actually landing before it becomes
        // an edit: a formula recognised at a text caret is wrapped, and one
        // recognised inside an existing `$…$` has its own delimiters removed
        // rather than producing `$a + $x^2$ + b$` (issue #2, owner report).
        // The bridge-attached path gets the same treatment inside the bridge.
        let normalized = Insertion.captureInsertion(latex, into: doc.text, atByte: byte)
        guard let insert = normalized.text else {
            captureNote = "Cannot insert \(proposal.captureId) here: "
                + (normalized.advisories.first ?? "the proposal is not legal LaTeX at this caret.")
            return .needsReselection("unsafe at caret")
        }
        guard let ns = doc.text.nsRange(utf8Bytes: .init(path: anchor.path, startByte: byte, endByte: byte)) else {
            captureNote = "Anchor offset is not a valid position."; return .needsReselection("invalid offset")
        }
        activePath = anchor.path
        var reviewed = proposal
        reviewed.latex = latex
        pendingEdit = .init(path: anchor.path, nsRange: ns, text: insert, token: nextEditToken(),
                            revision: editorRevision, captureRefund: .init(proposal: reviewed, anchorBefore: anchor))
        appliedCaptureIDs.insert(proposal.captureId)
        proposals.removeAll { $0.captureId == proposal.captureId }
        reviewing = proposals.first
        // Keep the anchor after the inserted text so successive captures append in order.
        self.anchor = InsertionAnchor(id: anchor.id, path: anchor.path, byteOffset: byte + insert.utf8.count,
                                      revision: editorRevision, contextAfter: anchor.contextAfter)
        captureNote = "Inserted \(proposal.captureId) at byte \(byte) (\(normalized.caret.label); undo with ⌘Z)."
            + (normalized.advisories.isEmpty ? "" : " " + normalized.advisories.joined(separator: " "))
        return .inserted(byteOffset: byte)
    }

    /// Called by the editor when it could NOT apply a pending edit: the buffer
    /// moved on since the edit was prepared (an IME commit, a keystroke) or the
    /// range no longer fits. A capture goes back to the front of the review
    /// queue with its pre-insertion anchor, so approving it again re-resolves
    /// the anchor against the current text; nothing is recorded as inserted.
    func editRefused(_ edit: PendingEdit, reason: String) {
        if pendingEdit == edit { pendingEdit = nil }
        guard let refund = edit.captureRefund else {
            // Bridge inserts build their pendingEdit without a refund, so this
            // is the path a refused capture insertion takes. navigationNote
            // alone only reaches the status bar, which is behind the modal
            // review sheet — the refusal looked like a dead Insert button.
            navigationNote = "Edit not applied: \(reason)."
            captureNote = "Nothing was inserted: \(reason)."
            return
        }
        let id = refund.proposal.captureId
        appliedCaptureIDs.remove(id)
        if !proposals.contains(where: { $0.captureId == id }) { proposals.insert(refund.proposal, at: 0) }
        reviewing = proposals.first
        if anchor?.id == refund.anchorBefore.id { anchor = refund.anchorBefore }
        captureNote = "Capture \(id) was not inserted: \(reason). It is back in the review queue; approve it again."
        reviewHistory.recordRefused(refund, reason: reason, editorRevision: editorRevision)
        log("insertion of \(id) refused: \(reason)")
    }

    /// Called by the editor once it has applied a pending edit (with undo registered).
    func editApplied(_ edit: PendingEdit, newText: String) {
        if pendingEdit == edit { pendingEdit = nil }
        updateActiveText(newText)
        reviewHistory.recordApplied(edit, editorRevision: editorRevision)
        // The insertion is the edit that bumped the revision; the anchor is exact for it.
        if let a = anchor, a.path == edit.path {
            anchor = InsertionAnchor(id: a.id, path: a.path, byteOffset: a.byteOffset,
                                     revision: editorRevision, contextAfter: a.contextAfter)
        }
    }

    // MARK: repo discovery

    /// Fixtures directory: `Contents/Resources/Samples` when running as a packaged
    /// `FlashTeX.app` (see `scripts/make-app.sh`), else `protocol/fixtures` under
    /// the repo root.
    static func locateFixturesDirectory() -> URL? {
        if let samples = Bundle.main.resourceURL?.appendingPathComponent("Samples"),
           FileManager.default.fileExists(atPath: samples.appendingPathComponent("compile-result.json").path) {
            return samples
        }
        return locateRepoRoot()?.appendingPathComponent("protocol/fixtures")
    }

    static func locateRepoRoot() -> URL? {
        var candidates: [URL] = []
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_REPO"] {
            candidates.append(URL(fileURLWithPath: env))
        }
        candidates.append(URL(fileURLWithPath: FileManager.default.currentDirectoryPath))
        candidates.append(Bundle.main.bundleURL)
        candidates.append(URL(fileURLWithPath: #filePath))
        for start in candidates {
            var url = start
            for _ in 0..<8 {
                if FileManager.default.fileExists(atPath: url.appendingPathComponent("protocol/fixtures/compile-result.json").path) {
                    return url
                }
                let parent = url.deletingLastPathComponent()
                if parent == url { break }
                url = parent
            }
        }
        return nil
    }
}
