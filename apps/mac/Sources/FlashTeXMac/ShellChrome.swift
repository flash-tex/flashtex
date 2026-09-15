import Foundation
import Observation
import FlashTeXProtocol

/// Throttled, change-only mirror of the `ShellModel` state the window chrome
/// shows: the status bar, the preview header, the document tab bar's byte
/// count and the sidebar's outline staleness (ContentView.swift,
/// DocumentTabBar.swift, WorkspaceSidebar.swift, WordCountStatusView.swift).
///
/// Why: those views used to read `editorRevision`, `workerStatus`,
/// `inFlightRevision`, `latenciesMs`, `captureNote`, `result`, `documents`…
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
    enum Route: Equatable { case fixture, controller, worker, none }

    // Status bar
    private(set) var editorRevision = 1
    private(set) var lastLatencyMs: Double?
    private(set) var latencyHelp = ""
    private(set) var route: Route = .none
    private(set) var routeHelp = ""
    private(set) var problems = ProblemCounts()
    /// `navigationNote ?? editorMarkReport.staleNote ?? explanationStatus`.
    private(set) var note: String?
    private(set) var captureNote: String?
    private(set) var durableRevision: Int?
    /// `editorMarkReport.carried?.line` (the diagnostics list's retention note).
    private(set) var carriedLine: String?

    // Preview header
    private(set) var previewSource: ShellModel.PreviewSource = .none
    private(set) var hasResult = false
    private(set) var resultHelp = ""
    private(set) var resultStatus: RuntimeV1.Status?
    private(set) var compiling = false
    private(set) var historicalLabel: String?
    /// The stale line ("editor at rN — compiling…", an output-bound banner), nil when the preview is current.
    private(set) var staleText: String?
    private(set) var staleHighlighted = false
    private(set) var loadError: String?
    private(set) var fixtureName: String?
    private(set) var capabilityNotes: [String] = []
    private(set) var acceptedCapabilities: [String] = []

    // Document tab bar / sidebar
    private(set) var activeTextBytes = 0
    private(set) var activeTextUTF16 = 0
    private(set) var listing: [ProjectDocument] = []
    private(set) var entryPath = "main.tex"
    private(set) var closure = ProjectDocuments.Closure(nodes: [], paths: [])

    /// Refresh delay after the first change; `FLASHTEX_CHROME_MS` overrides (0 = next run-loop turn).
    static let interval: TimeInterval = {
        if let s = ProcessInfo.processInfo.environment["FLASHTEX_CHROME_MS"], let ms = Double(s) { return max(0, ms) / 1000 }
        return 0.1
    }()

    /// `compiling` stays on for at least one interval so a fast reply does
    /// not flicker the indicator (a refresh that finds the request answered
    /// keeps it on once and asks for one more refresh).
    @ObservationIgnored private var compilingSeen = false

    /// Copies what the chrome shows; assigns a field only when it changed.
    /// Returns whether another refresh is wanted (the compiling indicator's
    /// minimum on-time).
    @discardableResult
    func refresh(from model: ShellModel) -> Bool {
        func set<T: Equatable>(_ keyPath: ReferenceWritableKeyPath<ShellChrome, T>, _ value: T) {
            if self[keyPath: keyPath] != value { self[keyPath: keyPath] = value }
        }
        set(\.editorRevision, model.editorRevision)
        set(\.lastLatencyMs, model.lastLatencyMs)
        if let ms = model.lastLatencyMs, let med = model.medianLatencyMs {
            set(\.latencyHelp, String(format: "Last compile latency %.0f ms (median %.0f over %d)", ms, med, model.latenciesMs.count))
        } else { set(\.latencyHelp, "") }
        let route: Route = model.isFixture ? .fixture : model.controllerAttached ? .controller : model.workerAttached ? .worker : .none
        set(\.route, route)
        set(\.routeHelp, model.isFixture ? "Not a real compile." : (model.controllerAttached ? model.controllerStatus : model.workerStatus))
        let (errors, warnings, gaps) = EditorDiagnostics.counts(model.displayedDiagnostics)
        set(\.problems, ProblemCounts(errors: errors, warnings: warnings, gaps: gaps))
        set(\.note, model.navigationNote ?? model.editorMarkReport.staleNote ?? model.explanationStatus)
        set(\.captureNote, model.captureNote)
        set(\.durableRevision, model.controllerState.durable[model.activePath]?.revision)
        set(\.carriedLine, model.editorMarkReport.carried?.line)

        set(\.previewSource, model.previewSource)
        set(\.hasResult, model.result != nil)
        if let r = model.result {
            set(\.resultHelp, "result id \(model.resultID ?? "?") · project \(r.projectId) · revision \(r.revision) · pdf: \(r.pdfPath ?? "none")")
        } else { set(\.resultHelp, "") }
        set(\.resultStatus, model.result?.status)
        let inFlight = model.inFlightRevision != nil
        var again = false
        if inFlight { set(\.compiling, true); compilingSeen = true }
        else if compilingSeen { compilingSeen = false; again = true } // one more interval on, then off
        else { set(\.compiling, false) }
        set(\.historicalLabel, model.historicalPreview?.label)
        if model.result != nil, model.historicalPreview == nil, model.previewIsStale {
            let rev = model.editorRevision
            set(\.staleText, model.outputBound?.banner
                ?? (model.workerAttached
                    ? (model.autoCompile ? "editor at r\(rev) — compiling…" : "editor at r\(rev) — ⌘B to compile")
                    : "editor at r\(rev) — no producer attached"))
            set(\.staleHighlighted, model.outputBound != nil || !model.workerAttached)
        } else {
            set(\.staleText, nil)
            set(\.staleHighlighted, false)
        }
        set(\.loadError, model.loadError)
        set(\.fixtureName, model.fixtureURL?.lastPathComponent)
        set(\.capabilityNotes, model.capabilityNotes)
        set(\.acceptedCapabilities, model.negotiation.accepted)

        let text = model.activeText
        set(\.activeTextBytes, text.utf8.count)
        set(\.activeTextUTF16, text.utf16.count)
        set(\.listing, model.project.listing)
        set(\.entryPath, model.project.entryPath)
        set(\.closure, model.project.discoverClosure())
        return again
    }
}
