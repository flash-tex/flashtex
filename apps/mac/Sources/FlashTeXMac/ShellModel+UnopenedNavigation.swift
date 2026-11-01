import Foundation
import FlashTeXProtocol

/// Preview → source into a document that is NOT open in this window (lane
/// mac-navigation-3).
///
/// A hit can name an included document (`\input{chapter}`) while the window
/// has only `main.tex` loaded. Nothing in this window opens such a document
/// at the exact text the preview was compiled from, so the hit is a typed
/// refusal: the caret is never guessed, and the note says how to open the
/// document (File > Open All Includes) to navigate.
enum UnopenedNavigation {
    /// The typed verdict before any caret is placed.
    enum Verdict: Equatable {
        /// Nothing to open: the document is already a member of this window.
        case alreadyOpen(path: String)
        /// The document is not open: no route reads it exactly.
        case notAttached(path: String)

        var note: String {
            switch self {
            case .alreadyOpen(let path):
                return "\(path) is already open"
            case .notAttached(let path):
                return "\(path) is not open in this window, so it cannot be opened exactly; open it (File > Open All Includes) to navigate."
            }
        }
    }
}

extension ShellModel {
    /// Says whether a preview hit in `path` may place the caret: only when
    /// `path` is already open in this window. `compiledRevision` and
    /// `compiledSHA256` are what the hit's frame declares for `path`; with
    /// no route that opens a document exactly they decide nothing.
    func openForNavigation(path: String, compiledRevision: Int?, compiledSHA256: String? = nil) async -> UnopenedNavigation.Verdict {
        if documents.contains(where: { $0.path == path }) { return .alreadyOpen(path: path) }
        return .notAttached(path: path)
    }

    /// `navigateExactly` for a hit whose document is open in this window;
    /// a hit in a document that is not open is refused with a note. The
    /// verdict is returned for tests (nil when the document was open).
    @discardableResult
    func navigateOpeningIfNeeded(to source: RuntimeV1.SourceRange?, expectedText: String? = nil,
                                 compiledRevision: Int? = nil) async -> UnopenedNavigation.Verdict? {
        guard let source else { navigateExactly(to: nil, expectedText: expectedText); return nil }
        if documents.contains(where: { $0.path == source.path }) {
            navigateExactly(to: source, expectedText: expectedText)
            return nil
        }
        let verdict = await openForNavigation(path: source.path, compiledRevision: compiledRevision)
        if case .alreadyOpen = verdict { navigateExactly(to: source, expectedText: expectedText); return verdict }
        navigationNote = "Preview → source: " + verdict.note
        return verdict
    }
}
