import Foundation
import FlashTeXProtocol

/// The Problems of one compile result, finished (APP-MAIN-THREAD): its
/// rows and everything the Problems panel, the status bar and VoiceOver
/// derive from them, made once where the rows are made (off the main
/// thread for a long document, `EngineV3ProblemsPipeline`).
///
/// Immutable: the main thread publishes a new one as one assignment and
/// tells two apart by identity (`===`), never by comparing thousands of rows.
/// A long book's compile reply cost the main thread ~25 ms of row
/// conversion, grouping and counting at every DONE while the author typed.
final class ProblemsModel: @unchecked Sendable {
    /// The rows, TeX's then the tools' (what `ShellModel.displayedDiagnostics` returns).
    let diagnostics: [RuntimeV1.Diagnostic]
    /// The texts the rows' byte offsets are in (the compile's).
    let texts: [String: String]
    /// The rows' line labels: an index of each text a row is in.
    let lines: EditorDiagnostics.Lines
    /// The listing order the groups were sorted by (`EditorDiagnostics.groups`).
    let documentOrder: [String]
    /// Every group, errors first, then warnings, then FlashTeX gaps.
    let groups: [EditorDiagnostics.Group]
    /// `groups` of each severity, in that order (the panel's severity filter).
    let errorGroups: [EditorDiagnostics.Group]
    let warningGroups: [EditorDiagnostics.Group]
    /// Errors and warnings that are not gaps, and the gaps (`EditorDiagnostics.counts`).
    let counts: (errors: Int, warnings: Int, gaps: Int)
    /// "3 errors, 1 warning" (`EditorDiagnostics.spokenSummary`).
    let spokenSummary: String
    /// Whether any row has a mechanical fix (the caret's Tab fix looks no further otherwise).
    let hasFixes: Bool
    /// Whether any row is an error (the editor marks' result status).
    let hasErrors: Bool
    /// The packages and classes the rows say were not found (`ProjectPackagesState.unresolvedNames`).
    let unresolvedPackages: [String]
    /// The files the rows are in.
    let paths: Set<String>
    /// A group's place in `groups` by its id.
    private let positions: [String: Int]

    init(diagnostics: [RuntimeV1.Diagnostic], texts: [String: String], documentOrder: [String]) {
        self.diagnostics = diagnostics
        self.texts = texts
        self.documentOrder = documentOrder
        let paths = Set(diagnostics.compactMap { $0.source?.path })
        self.paths = paths
        lines = EditorDiagnostics.Lines(indexing: texts, paths: paths)
        let groups = EditorDiagnostics.groups(of: diagnostics, documentOrder: documentOrder)
        self.groups = groups
        errorGroups = groups.filter { $0.severity == .error }
        warningGroups = groups.filter { $0.severity == .warning }
        let counts = EditorDiagnostics.counts(diagnostics)
        self.counts = counts
        spokenSummary = EditorDiagnostics.spokenSummary(counts)
        hasFixes = diagnostics.contains { $0.help?.replacement != nil || $0.suggestion != nil }
        hasErrors = diagnostics.contains { $0.severity == .error }
        unresolvedPackages = ProjectPackagesState.unresolvedNames(in: diagnostics)
        positions = Dictionary(groups.enumerated().map { ($1.id, $0) }, uniquingKeysWith: { a, _ in a })
    }

    static let empty = ProblemsModel(diagnostics: [], texts: [:], documentOrder: [])

    /// The groups the panel lists under a severity filter (nil: all).
    func groups(_ severity: RuntimeV1.Severity?) -> [EditorDiagnostics.Group] {
        switch severity {
        case nil: return groups
        case .error?: return errorGroups
        case .warning?: return warningGroups
        }
    }

    /// The group with `id`, nil when this result has none.
    func group(id: String?) -> EditorDiagnostics.Group? {
        guard let id, let i = positions[id] else { return nil }
        return groups[i]
    }

    /// The line labels over `texts`: this model's own when they are its
    /// texts (a compile's rows and texts arrive together), else an index made
    /// here (the texts came after the rows: a fixture, a document opened).
    func lines(over texts: [String: String]) -> EditorDiagnostics.Lines {
        texts == self.texts ? lines : EditorDiagnostics.Lines(indexing: texts, paths: paths)
    }
}
