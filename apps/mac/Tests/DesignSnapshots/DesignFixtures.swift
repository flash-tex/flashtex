//  Fixture states for the design render loop: a `ShellModel` most surfaces
//  need, in a handful of deliberate configurations. Everything is built from
//  the checked-in sample fixtures (`apps/mac/Samples`, `protocol/fixtures`)
//  plus in-memory documents — no producer, no network, no disk writes — so a
//  snapshot renders the same on every run.

import Foundation
import SwiftUI
import FlashTeXProtocol
@testable import FlashTeXMac

@MainActor
enum DesignFixtures {

    /// An isolated `NearbyState` (the Captures inspector reads it as an
    /// environment object): pair store in a throwaway file, nothing
    /// advertised, so the shell renders identically on every machine.
    static func nearby() -> NearbyState {
        NearbyState(store: PairStore(url: FileManager.default.temporaryDirectory
            .appendingPathComponent("design-snapshots-pairs-\(UUID().uuidString).json")))
    }

    /// The demo project (3 rendered pages) with two additional open
    /// documents, one of them edited: enough tabs, outline content and
    /// preview to render the shell honestly.
    static func project() -> ShellModel {
        let model = ShellModel()
        if let root = ShellModel.locateRepoRoot() {
            let samples = root.appendingPathComponent("apps/mac/Samples")
            model.loadFixtures(request: samples.appendingPathComponent("demo-request.json"),
                               result: samples.appendingPathComponent("demo-result.json"))
        }
        model.documents.append(.init(path: "chapters/results.tex",
                                     text: "\\section{Results}\n\nAs shown in Figure~\\ref{fig:results}.\n"))
        model.documents.append(.init(path: "refs.bib",
                                     text: "@article{knuth84,\n  author = {Knuth, Donald},\n  year = {1984}\n}\n"))
        // Park the caret inside §"At the café" so caret-derived chrome (the
        // status-bar breadcrumb) has something honest to show.
        if let r = (model.activeText as NSString?)?.range(of: "two readers"), r.location != NSNotFound {
            model.caretUTF16 = r.location
        }
        model.previewV2 = false // the fixture result is v1 pages; render them
        model.previewZoom = 1 // fit width; never the machine's persisted zoom
        model.darkPreview = false // never inherit the machine's preference into a snapshot
        model.problemsVisible = false // rest state: the bottom panel earns its space only with content
        model.flushChrome()
        return model
    }

    /// `project()` plus a Problems panel worth reading: grouped errors with
    /// notes and help, warnings, and a FlashTeX gap.
    static func projectWithProblems() -> ShellModel {
        let model = project()
        model.result?.diagnostics = sampleDiagnostics
        // A new result id: the editor-marks memo keys on it (a real reply
        // always brings one), so the synthetic diagnostics reach the gutter.
        model.resultID = (model.resultID ?? "fixture") + "+diagnostics"
        model.problemsVisible = true
        // Select the grouped error so the snapshot shows the in-row
        // disclosure (notes, help, recovery under the selected problem).
        let groups = EditorDiagnostics.groups(of: model.displayedDiagnostics,
                                              documentOrder: model.documents.map(\.path))
        model.problemsPanel.selection = groups.first?.id // the fixable error, so the disclosure shows
        model.flushChrome()
        return model
    }

    /// A believable TeX diagnostic mix: an undefined command with a fix, an
    /// undefined reference in two places, an overfull-box warning, and a
    /// not-implemented gap.
    static var sampleDiagnostics: [RuntimeV1.Diagnostic] {
        [
            .init(severity: .error,
                  message: "Undefined control sequence \\includegraphcs",
                  source: .init(path: "main.tex", startByte: 120, endByte: 136),
                  recovery: "the paragraph continues without it",
                  notes: ["the command is not defined by any loaded package"],
                  help: .init(message: "did you mean \\includegraphics?",
                              replacement: .init(startByte: 120, endByte: 136, text: "\\includegraphics"))),
            .init(severity: .error,
                  message: "Undefined reference `fig:resuls'",
                  source: .init(path: "main.tex", startByte: 260, endByte: 270),
                  recovery: nil),
            .init(severity: .error,
                  message: "Undefined reference `fig:resuls'",
                  source: .init(path: "main.tex", startByte: 402, endByte: 412),
                  recovery: nil),
            .init(severity: .warning,
                  message: "Overfull \\hbox (12.4pt too wide) in paragraph",
                  source: .init(path: "main.tex", startByte: 480, endByte: 520),
                  recovery: nil),
            .init(severity: .warning,
                  message: "not implemented: \\marginpar",
                  source: .init(path: "main.tex", startByte: 560, endByte: 570),
                  recovery: nil),
        ]
    }
}
