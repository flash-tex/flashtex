import AppKit
import ObjectiveC
import Observation
import SwiftUI

/// Explicit document kinds (lane `mac-bibliography-kinds`, gap 3).
///
/// A kind is only ever what was *declared* for a source — `latex` or
/// `bibliography` — never inferred from a file extension or from the text:
/// `refs.bib` opened through the generic `ProjectDocuments.openDocument` is
/// a LaTeX source until it is declared. The sidebar, the tab bar, syntax
/// highlighting, the citation hover and `\cite` completion (BibScanner.swift)
/// read the kinds here.
///
/// Declarations were made through the durable helper (preview-controller),
/// which reported them in its `snapshot`; that route is retired (RQ7), so no
/// declaration is reported in this window and every source reads as LaTeX
/// unless its own extension decides how it is parsed (`BibScanner`). A
/// declaration record an earlier version left in the helper's ledger
/// directory is left untouched on disk.
///
/// Attached to the model as an associated object (`model.documentKinds`),
/// the same way `ProjectDocuments` is, so ShellModel.swift stays untouched.

/// A declared kind.
enum DocumentKind: String, Codable, Equatable, Sendable {
    case latex, bibliography
}

// MARK: - model layer

@MainActor
@Observable
final class DocumentKinds {
    /// The declared kind of each source path. Never derived from paths.
    private(set) var kinds: [String: DocumentKind] = [:]
    /// Human-readable state of the kinds.
    private(set) var status = "no document kinds declared"

    /// Paths declared as `bibliography`, sorted.
    var bibliographyPaths: [String] { kinds.filter { $0.value == .bibliography }.keys.sorted() }

    func kind(of path: String) -> DocumentKind? { kinds[path] }
}

// MARK: - model attachment

extension ShellModel {
    private static var documentKindsKey = 0
    /// Explicit document kinds (see `DocumentKinds`).
    var documentKinds: DocumentKinds {
        if let existing = objc_getAssociatedObject(self, &Self.documentKindsKey) as? DocumentKinds { return existing }
        let state = DocumentKinds()
        objc_setAssociatedObject(self, &Self.documentKindsKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }
}

// MARK: - UI

/// Read-only caption for the header row: the active document's declared
/// kind when it is a bibliography, or the count of declared sources.
/// Shows nothing when no bibliography kind is declared at all.
struct DocumentKindIndicator: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let kinds = model.documentKinds
        let declared = kinds.bibliographyPaths
        if !declared.isEmpty {
            let active = kinds.kind(of: model.activePath)
            Text(active == .bibliography ? "bibliography (declared)" : "\(declared.count) bibliography source\(declared.count == 1 ? "" : "s")")
                .font(.caption).foregroundStyle(.secondary)
                .help(kinds.status)
                .accessibilityIdentifier("document-kind-indicator")
        }
    }
}
