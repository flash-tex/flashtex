import Foundation
import FlashTeXProtocol

/// Partial-output marks across a failed compile (lane mac-diagnostics-2).
///
/// The shell binds every `compile_result`, including a `failed` one with no
/// pages (the compiler's answer to an unsafe path, an empty project or a
/// refused span). Read on its own, such a result has no marks, so the editor
/// would lose every underline the moment a compile fails and get them all
/// back on the next success — flicker that also hides where the errors were.
/// `retainedMarks` remembers the last result that produced output (with the
/// texts it was compiled from), and `editorMarkReport` draws its marks,
/// rebased to the current buffer and flagged `carried`, while the newest
/// result is a failure with no output. Retention never chains through
/// failures, so the same marks are shown exactly once however many failures
/// follow, and any result with output replaces them.
extension ShellModel {
    /// Call after `result`, `resultID` and `compiledDocuments` are bound
    /// (fixture, worker and controller paths): updates `retainedMarks`.
    func retainMarksAfterResultBound() {
        guard let result else { retainedMarks = nil; return }
        retainedMarks = EditorDiagnostics.retained(after: result, resultID: resultID, compiledDocuments: compiledDocuments,
                                                   previous: retainedMarks)
    }

    /// The report for `path` (any open document) with the retention rule
    /// applied — what `editorMarkReport` memoizes for the active document
    /// and what diagnostic navigation reads for the others.
    func diagnosticReport(for path: String, currentText: String) -> EditorDiagnostics.Report {
        guard var result else { return .empty }
        let producer = producerDiagnostics
        if result.diagnostics != producer { result.diagnostics = producer }
        return EditorDiagnostics.report(for: result, resultID: resultID, retained: retainedMarks, path: path,
                                        compiledText: compiledDocuments[path], currentText: currentText)
    }
}
