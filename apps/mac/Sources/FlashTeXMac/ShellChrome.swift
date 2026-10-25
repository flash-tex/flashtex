import Foundation
import Observation
import FlashTeXProtocol

/// Throttled, change-only mirror of the `ShellModel` state the window chrome
/// shows: the status bar, the preview header, the document tab bar's byte
/// count and the sidebar's outline staleness (ContentView.swift,
/// DocumentTabBar.swift, WorkspaceSidebar.swift, WordCountStatusView.swift).
///
/// Why: those views used to read `editorRevision`, the producer's status,
/// the in-flight revision, latencies, `captureNote`, the result, `documents`…
/// directly, so every keystroke and every producer reply re-evaluated them —
/// three to four whole-window SwiftUI transactions per keystroke, each
/// re-laying out the hosting view (FT-071 sample, 2026-09-13: main thread
/// 71 % busy while typing, 73 % of it SwiftUI graph updates, app code 6 %).
/// The chrome now reads this object instead. `ShellModel` refreshes it under
/// `withObservationTracking`: the first change to anything the refresh read
/// schedules one refresh `ShellChrome.interval` later (default 100 ms,
/// `FLASHTEX_CHROME_MS` overrides; 0 = the next run-loop turn), and each
/// field is assigned only when its value changed, so the chrome re-evaluates
/// at most ~10 times a second and only for what it shows. The model
/// properties themselves are unchanged (tests and other code read them as
/// before); only the labels lag by at most one interval.
@MainActor
@Observable
final class ShellChrome {
    struct ProblemCounts: Equatable {
        var errors = 0, warnings = 0, gaps = 0
        var isEmpty: Bool { errors == 0 && warnings == 0 && gaps == 0 }
    }
    /// `engineV3` while the engine session runs (`ShellModel.engineV3Enabled`), else `none`.
    enum Route: Equatable { case none, engineV3 }

    // Status bar
    private(set) var editorRevision = 1
    private(set) var lastLatencyMs: Double?
    private(set) var latencyHelp = ""
    private(set) var route: Route = .none
    private(set) var routeHelp = ""
    private(set) var problems = ProblemCounts()
    /// `navigationNote ?? editorMarkReport.staleNote`.
    private(set) var note: String?
    private(set) var captureNote: String?
    /// `editorMarkReport.carried?.line` (the diagnostics list's retention note).
    private(set) var carriedLine: String?

    // Preview header
    /// The engine has pages to show.
    private(set) var hasResult = false
    private(set) var compiling = false
    /// The engine's state line ("preparing the format…", "2 stale pages", a
    /// failure), nil when the preview is current.
    private(set) var staleText: String?
    private(set) var staleHighlighted = false
    private(set) var loadError: String?

    // Document tab bar / sidebar
    private(set) var activeTextBytes = 0
    private(set) var listing: [ProjectDocument] = []
    private(set) var entryPath = "main.tex"
    private(set) var closure = ProjectDocuments.Closure(nodes: [], paths: [])
    /// Package inputs from the manifest read (ProjectManifest.swift).
    private(set) var packageInputs: [ProjectManifest.Row] = []

    /// Refresh delay after the first change; `FLASHTEX_CHROME_MS` overrides (0 = next run-loop turn).
    static let interval: TimeInterval = {
        if let s = ProcessInfo.processInfo.environment["FLASHTEX_CHROME_MS"], let ms = Double(s) { return max(0, ms) / 1000 }
        return 0.1
    }()

    /// The preview's state is the engine session's (EngineV3Session.swift):
    /// preparing the format, compiling, stale pages or a failure.
    private func refreshEngineV3(_ s: EngineV3Session) {
        func set<T: Equatable>(_ keyPath: ReferenceWritableKeyPath<ShellChrome, T>, _ value: T) {
            if self[keyPath: keyPath] != value { self[keyPath: keyPath] = value }
        }
        set(\.route, .engineV3)
        set(\.routeHelp, s.environmentNote.isEmpty ? (s.engineName == "XeLaTeX" ? "XeLaTeX-compatible engine (flashtex-host-unicode)" : "pdfLaTeX-compatible engine (flashtex-host)") : s.environmentNote)
        set(\.hasResult, s.pageCount > 0)
        // The engine's own latency (gap C22): a keystroke to its page on
        // screen (EngineV3Latency), median over the last 200 samples.
        let ms = s.latency.samples.suffix(200).map(\.ms) // the recent typing: a bounded sort per refresh
        set(\.lastLatencyMs, ms.last)
        if let last = ms.last {
            let median = ms.sorted()[ms.count / 2]
            set(\.latencyHelp, String(format: "Last keystroke to its page on screen %.0f ms (median %.0f over %d)", last, median, ms.count))
        } else { set(\.latencyHelp, "") }
        set(\.compiling, s.compiling || { if case .starting = s.phase { true } else { false } }())
        let text: String?
        var highlighted = false
        switch s.phase {
        case .idle: text = s.bundleConsentShown ? "waiting: download the TeX files?" : "preview engine idle"
        case .starting: text = s.bundleProgressNote ?? "preparing the \(s.engineName) format…" // a bundle download first (no TeX Live)
        case .failed(let why): text = why; highlighted = true
        case .ready:
            if s.editsWaiting { text = "edited — ⌘B to compile" }
            else if s.staleCount > 0 { text = "\(s.staleCount) stale page\(s.staleCount == 1 ? "" : "s")" }
            else if s.errorCount > 0 { text = s.firstError ?? "\(s.errorCount) error\(s.errorCount == 1 ? "" : "s") in the last compile"; highlighted = true }
            else if let t = s.toolNote { text = t } // bibtex, biber, makeindex
            else if let t = s.bundleProgressNote { text = t } // a file the bundle fetches on demand
            else { text = nil }
        }
        set(\.staleText, text)
        set(\.staleHighlighted, highlighted)
    }

    /// The preview fields while the engine session is off (under XCTest until
    /// a test turns it on): nothing compiles, nothing is shown.
    private func refreshEngineOff() {
        func set<T: Equatable>(_ keyPath: ReferenceWritableKeyPath<ShellChrome, T>, _ value: T) {
            if self[keyPath: keyPath] != value { self[keyPath: keyPath] = value }
        }
        set(\.route, .none)
        set(\.routeHelp, "")
        set(\.hasResult, false)
        set(\.lastLatencyMs, nil)
        set(\.latencyHelp, "")
        set(\.compiling, false)
        set(\.staleText, nil)
        set(\.staleHighlighted, false)
    }

    /// Copies what the chrome shows; assigns a field only when it changed.
    /// Returns whether another refresh is wanted (the include closure is
    /// waiting for the texts to settle).
    @discardableResult
    func refresh(from model: ShellModel) -> Bool {
        func set<T: Equatable>(_ keyPath: ReferenceWritableKeyPath<ShellChrome, T>, _ value: T) {
            if self[keyPath: keyPath] != value { self[keyPath: keyPath] = value }
        }
        set(\.editorRevision, model.editorRevision)
        let (errors, warnings, gaps) = MainThreadProbe.time("chrome.problems") { model.engineV3Problems.counts } // made with the rows (ProblemsModel)
        set(\.problems, ProblemCounts(errors: errors, warnings: warnings, gaps: gaps))
        set(\.note, MainThreadProbe.time("chrome.marks") { model.navigationNote ?? model.editorMarkReport.staleNote })
        set(\.captureNote, model.captureNote)
        set(\.carriedLine, model.editorMarkReport.carried?.line)
        set(\.loadError, model.loadError)

        let text = model.activeText
        set(\.activeTextBytes, text.utf8.count) // O(1); its UTF-16 count (unread) walked a non-ASCII text per refresh
        let listing = MainThreadProbe.time("chrome.listing") { model.project.listing }
        let documentSetChanged = listing.map(\.path) != self.listing.map(\.path)
        set(\.listing, listing)
        set(\.entryPath, model.project.entryPath)
        // The include closure scans every open text (`ProjectIncludes.scan`:
        // a 4 MB book is milliseconds of main thread), and typing rarely
        // changes it: rescanned when the open documents change, else once
        // the texts have been still for `closureQuiet` (APP-EDITOR-INSTANT).
        let revision = model.documentsRevision, now = MonotonicClock.nowNs()
        if revision != seenRevision { seenRevision = revision; seenRevisionNs = now }
        var closureWaits = false
        if revision != closureRevision {
            if closureRevision < 0 || documentSetChanged || settleNext || now &- seenRevisionNs >= Self.closureQuietNs {
                set(\.closure, MainThreadProbe.time("chrome.closure") { model.project.discoverClosure() })
                closureRevision = revision
            } else {
                closureWaits = true
            }
        }
        settleNext = false
        set(\.packageInputs, model.manifest.rows + model.projectPackages.rows) // ProjectPackages.swift: resolved packages, after the project's own
        // The preview's route, pages and stale line: each field assigned once
        // per refresh, so nothing flips back and forth and re-evaluates the
        // status bar and the preview header for nothing (P5-KEYSTROKE-MAIN).
        if model.engineV3Enabled { refreshEngineV3(model.engineV3) } else { refreshEngineOff() }
        return closureWaits
    }

    /// The include closure's text revision (`documentsRevision`), and when
    /// the texts were last seen to change.
    @ObservationIgnored private var closureRevision = -1
    @ObservationIgnored private var seenRevision = -1
    @ObservationIgnored private var seenRevisionNs: UInt64 = 0
    /// The next refresh brings the closure up to date whatever the typing (`ShellModel.flushChrome`).
    @ObservationIgnored var settleNext = false
    static let closureQuietNs: UInt64 = 600_000_000
}
