import Foundation
import FlashTeXProtocol

/// Preview → source into a document that is NOT open in this window (lane
/// mac-navigation-3).
///
/// On the helper route the durable helper discovers `\input{chapter}` at
/// startup and compiles `main.tex` + `chapter.tex` while the window has only
/// `main.tex` loaded, so a v1 item or a v2 cluster can name `chapter.tex`.
/// `navigateExactly`/`navigateV2` refuse such a hit ("No open document named
/// chapter.tex"). `navigateOpeningIfNeeded` opens the document through the
/// project layer (`ProjectDocuments.openDocument`: the helper's exact
/// `document`/`open_document` snapshot, never a disk read on the helper
/// route), verifies that the durable revision it was opened at is the
/// revision the preview was compiled from, and only then places the caret
/// exactly. A durable revision that differs from the compiled one is a typed
/// refusal — the document stays open at its durable text, the caret is never
/// guessed. Off the helper route nothing can open the document exactly
/// (direct compiles only ever see the open buffers), so that is refused too.
enum UnopenedNavigation {
    /// The typed verdict before any caret is placed.
    enum Verdict: Equatable {
        /// `path` was opened at durable `revision`, the revision the preview
        /// was compiled from: navigate exactly against its text.
        case opened(path: String, revision: Int)
        /// Nothing to open: the document is already a member of this window.
        case alreadyOpen(path: String)
        /// The document is not open and the durable helper is not attached:
        /// no route reads it exactly.
        case notAttached(path: String)
        /// The applied preview names no durable revision for `path` (no
        /// helper preview applied yet, or the path is not a member).
        case noCompiledRevision(path: String)
        /// The project layer refused the open (helper refusal, stale
        /// snapshot, path rule); `why` is its note.
        case openRefused(path: String, why: String)
        /// The document opened, but at durable `durable` while the preview was
        /// compiled from `compiled`: the caret is not placed.
        case revisionDiffers(path: String, compiled: Int, durable: Int)
        /// The document opened at the compiled durable revision, but its text
        /// does not hash to the digest the display list declares for it: the
        /// caret is not placed.
        case textDiffers(path: String, durable: Int)

        var note: String {
            switch self {
            case .opened(let path, let revision):
                return "opened \(path) at durable r\(revision)"
            case .alreadyOpen(let path):
                return "\(path) is already open"
            case .notAttached(let path):
                return "\(path) is not open in this window and no durable helper is attached to open it exactly; open it (File > Open All Includes) to navigate."
            case .noCompiledRevision(let path):
                return "\(path) is not open in this window and the applied preview names no durable revision for it; recompile to navigate."
            case .openRefused(let path, let why):
                return "\(path) is not open in this window and could not be opened: \(why)"
            case .revisionDiffers(let path, let compiled, let durable):
                return "\(path) opened at durable r\(durable), but the preview was compiled from r\(compiled); the caret was not placed — recompile (or wait for the next preview) to navigate."
            case .textDiffers(let path, let durable):
                return "\(path) opened at durable r\(durable), but its text is not the text the display list declares for it; the caret was not placed — recompile to navigate."
            }
        }
    }
}

extension ShellModel {
    /// The durable revision the applied preview was compiled from for `path`
    /// (helper route: the helper's `source_versions` of the applied v1
    /// preview, which the candidate gate verified equal to the painted v2
    /// frame's), or nil. Measured: the producer's `documents[].revision` in
    /// the display list is its compile revision, NOT the per-document
    /// durable revision, so it is never used as the comparator.
    func compiledRevision(for path: String) -> Int? {
        displayCandidates.applied?.sourceVersions[path]
    }

    /// Opens `path` for a preview hit when it is not open, and says whether
    /// the caret may be placed. `compiledRevision` is the revision the hit's
    /// frame declares for `path` (v2: the display list's `documents[].revision`;
    /// v1: `compiledRevision(for:)`).
    /// `compiledSHA256`, when the hit's frame declares one (v2), additionally
    /// binds the opened text to the exact bytes the list was produced from.
    func openForNavigation(path: String, compiledRevision: Int?, compiledSHA256: String? = nil) async -> UnopenedNavigation.Verdict {
        if documents.contains(where: { $0.path == path }) { return .alreadyOpen(path: path) }
        guard controllerAttached else { return .notAttached(path: path) }
        guard let compiledRevision else { return .noCompiledRevision(path: path) }
        switch await project.openDocument(path, role: .included(from: project.entryPath)) {
        case .refused(let why): return .openRefused(path: path, why: why)
        case .opened, .alreadyOpen: break
        }
        guard let durable = controllerState.durable[path]?.revision else {
            return .openRefused(path: path, why: "the helper reported no durable revision for it")
        }
        guard durable == compiledRevision else {
            return .revisionDiffers(path: path, compiled: compiledRevision, durable: durable)
        }
        if let compiledSHA256, let text = documents.first(where: { $0.path == path })?.text,
           SourceDigest.sha256Hex(text) != compiledSHA256 {
            return .textDiffers(path: path, durable: durable)
        }
        return .opened(path: path, revision: durable)
    }

    /// `navigateExactly` that first opens the hit's document when it is not
    /// open in this window. `compiledRevision` overrides the applied
    /// preview's durable revision for the path (the v2 route passes the
    /// declared one). The verdict is returned for tests; the note names what
    /// happened either way.
    @discardableResult
    func navigateOpeningIfNeeded(to source: RuntimeV1.SourceRange?, expectedText: String? = nil,
                                 compiledRevision: Int? = nil) async -> UnopenedNavigation.Verdict? {
        guard let source else { navigateExactly(to: nil, expectedText: expectedText); return nil }
        if documents.contains(where: { $0.path == source.path }) {
            navigateExactly(to: source, expectedText: expectedText)
            return nil
        }
        if let why = historicalRefusal(of: "navigation") { navigationNote = why; return nil }
        let verdict = await openForNavigation(path: source.path, compiledRevision: compiledRevision ?? self.compiledRevision(for: source.path))
        guard case .opened(_, let revision) = verdict else {
            if case .alreadyOpen = verdict { navigateExactly(to: source, expectedText: expectedText); return verdict }
            navigationNote = "Preview → source: " + verdict.note
            return verdict
        }
        // The buffer was just opened at the compiled durable text: it is the baseline.
        let baseline = documents.first { $0.path == source.path }?.text
        navigateExactly(to: source, expectedText: expectedText, compiledText: baseline)
        if navigationNote?.hasPrefix("Selected") == true {
            navigationNote! += " (opened \(source.path) at durable r\(revision))"
        }
        return verdict
    }
}
