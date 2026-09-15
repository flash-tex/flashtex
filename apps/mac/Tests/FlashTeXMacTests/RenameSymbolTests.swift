import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Rename Symbol (labels and user commands), Go to Definition and the
/// definition peek (lane mac-editor-dx-3, EditorNavigation.swift).
@MainActor
final class RenameSymbolTests: XCTestCase {
    typealias EN = EditorNavigation

    func testSymbolUnderCaretIsALabelKeyOrACommand() {
        let s = "\\label{eq:a} \\ref{eq:a, eq:b} \\foo \\foobar \\begin{x}" as NSString
        XCTAssertEqual(EN.symbol(at: 9, in: s), .label("eq:a"))
        XCTAssertEqual(EN.symbol(at: 11, in: s), .label("eq:a"), "right after the key")
        XCTAssertEqual(EN.symbol(at: 25, in: s), .label("eq:b"), "the second key of a comma list")
        XCTAssertEqual(EN.symbol(at: 0, in: s), .command("label"), "on the command name itself")
        XCTAssertEqual(EN.symbol(at: 32, in: s), .command("foo"))
        XCTAssertEqual(EN.symbol(at: 38, in: s), .command("foobar"))
        XCTAssertNil(EN.symbol(at: 48, in: s), "\\begin/\\end are not renamed here")
        XCTAssertNil(EN.symbol(at: s.length, in: s))
    }

    func testOccurrencesAreWordBoundaryAwareAndSkipCommentsAndVerbatim() {
        let s = "\\newcommand{\\foo}{x}\n\\foo \\foobar \\foo* {\\foo}\n% \\foo in a comment\n\\begin{verbatim}\\foo\\end{verbatim}\n\\verb|\\foo| \\foo" as NSString
        let ranges = EN.occurrences(of: .command("foo"), in: s)
        XCTAssertEqual(ranges.map(\.location), [12, 21, 34, 41, 114])
        XCTAssertTrue(ranges.allSatisfy { $0.length == 4 })
        let labels = "\\label{a} \\ref{a} \\ref{ab} \\eqref{ b , a } \\cref{a,b} % \\ref{a}\n\\pageref{a}" as NSString
        let keys = EN.occurrences(of: .label("a"), in: labels)
        XCTAssertEqual(keys.map(\.location), [7, 15, 39, 49, 73])
        XCTAssertEqual(keys.map(\.length), [1, 1, 1, 1, 1])
    }

    func testRenamePlanGroupsIntoOneEditPerDocumentAndAppliesExactly() {
        let main = "\\newcommand{\\R}{\\mathbb{R}}\n$\\R^n$ and \\Rn and \\R"
        let ch = "\\R is real; \\ref{eq:1}"
        let plan = try! XCTUnwrap(EN.renamePlan(.command("R"), to: "Reals", in: [("main.tex", main), ("ch.tex", ch)]))
        XCTAssertEqual(plan.summary, "4 occurrences in 2 files")
        XCTAssertEqual(plan.documents.map(\.path), ["main.tex", "ch.tex"])
        XCTAssertEqual(plan.documents[0].ranges.count, 3)
        let g = try! XCTUnwrap(plan.documents[0].grouped(in: main as NSString))
        XCTAssertEqual(g.range, NSRange(location: 12, length: main.utf16.count - 12))
        XCTAssertEqual(plan.documents[0].applied(to: main as NSString), "\\newcommand{\\Reals}{\\mathbb{R}}\n$\\Reals^n$ and \\Rn and \\Reals")
        XCTAssertEqual(plan.documents[1].applied(to: ch as NSString), "\\Reals is real; \\ref{eq:1}")
        // Labels: the key everywhere, never the command names.
        let lp = try! XCTUnwrap(EN.renamePlan(.label("eq:1"), to: "eq:main", in: [("ch.tex", ch)]))
        XCTAssertEqual(lp.documents[0].applied(to: ch as NSString), "\\R is real; \\ref{eq:main}")
        XCTAssertNil(EN.renamePlan(.label("nope"), to: "x", in: [("ch.tex", ch)]))
        // Name rules.
        XCTAssertEqual(EN.nameProblem("", for: .label("a")), "the new key is empty")
        XCTAssertEqual(EN.nameProblem("a b", for: .label("a")), "the new key contains “whitespace”")
        XCTAssertEqual(EN.nameProblem("f2", for: .command("f")), "a command name is letters only (\\f2 is not)")
        XCTAssertNil(EN.nameProblem("\\Reals", for: .command("R")))
    }

    func testDefinitionsCoverNewcommandDefLetOperatorAndEnvironments() {
        let s = """
        \\newcommand{\\R}{\\mathbb{R}}
        \\renewcommand*\\vec[1]{\\mathbf{#1}}
        \\def\\eps#1{\\varepsilon_{#1}}
        \\DeclareMathOperator{\\Tr}{Tr}
        \\let\\oldsection=\\section
        \\newenvironment{thm}[1]{\\begin{proof}[#1]}{\\end{proof}}
        \\newtheorem{lemma}{Lemma}
        \\NewDocumentCommand{\\pair}{m m}{(#1, #2)}
        """ as NSString
        let defs = EN.definitions(in: s)
        XCTAssertEqual(defs.map(\.name), ["R", "vec", "eps", "Tr", "oldsection", "thm", "lemma", "pair"])
        XCTAssertEqual(defs.map(\.line), [1, 2, 3, 4, 5, 6, 7, 8])
        XCTAssertEqual(defs[0].body, "\\mathbb{R}")
        XCTAssertEqual(defs[1].body, "\\mathbf{#1}")
        XCTAssertEqual(defs[2].body, "\\varepsilon_{#1}")
        XCTAssertEqual(defs[3].body, "Tr")
        XCTAssertEqual(defs[4].body, "\\section")
        XCTAssertEqual(defs[5].body, "\\begin{proof}[#1]"); XCTAssertTrue(defs[5].isEnvironment)
        XCTAssertEqual(defs[6].body, "Lemma"); XCTAssertTrue(defs[6].isEnvironment)
        XCTAssertEqual(defs[7].body, "(#1, #2)")
        XCTAssertEqual(defs[0].range, NSRange(location: 0, length: 27))
        XCTAssertEqual(defs[0].summary, "\\newcommand{\\R}{\\mathbb{R}}")
        XCTAssertEqual(defs[5].summary, "\\newenvironment{thm}{\\begin{proof}[#1]}")
        XCTAssertEqual(EN.definition(of: "thm", in: s)?.name, nil, "an environment is not a command")
        XCTAssertEqual(EN.definition(of: "thm", in: s, environment: true)?.line, 6)
        XCTAssertNil(EN.definition(of: "section", in: s))
    }

    // MARK: model

    private func model(main: String, ch: String? = nil) -> ShellModel {
        let m = ShellModel()
        m.documents = [.init(path: "main.tex", text: main)] + (ch.map { [RuntimeV1.Document(path: "ch.tex", text: $0)] } ?? [])
        m.activePath = "main.tex"
        return m
    }

    func testRenameCommandPlansAndAppliesWithoutTheHelper() async {
        let m = model(main: "\\newcommand{\\R}{\\mathbb{R}}\n$\\R$", ch: "\\R and \\Rn")
        m.caretUTF16 = 30 // on `\R` in the body
        m.presentRenameSymbol()
        XCTAssertTrue(m.editorNavigation.renameShown)
        XCTAssertEqual(m.editorNavigation.renameSymbol, .command("R"))
        XCTAssertEqual(m.editorNavigation.renameNewName, "R")
        m.planRenameSymbol()
        XCTAssertEqual(m.editorNavigation.renameStatus, "The new name is the current name.")
        m.editorNavigation.renameNewName = "Rn"
        m.planRenameSymbol()
        XCTAssertEqual(m.editorNavigation.renameStatus, "Refused: \\Rn already exists in the project.")
        XCTAssertNil(m.editorNavigation.renamePlan)
        m.editorNavigation.renameNewName = "\\Reals"
        m.planRenameSymbol()
        XCTAssertEqual(m.editorNavigation.renamePlan?.summary, "3 occurrences in 2 files")
        XCTAssertTrue(m.editorNavigation.renameStatus.hasPrefix("Proposal: 3 occurrences in 2 files — one undoable edit per open document"))
        await m.applyRenameSymbol()
        // Active document: one grouped pending edit (the editor applies it); the other open buffer is rewritten directly.
        let edit = try! XCTUnwrap(m.pendingEdit)
        XCTAssertEqual(edit.nsRange, NSRange(location: 12, length: 19))
        XCTAssertEqual(edit.text, "\\Reals}{\\mathbb{R}}\n$\\Reals")
        XCTAssertEqual(m.documents[1].text, "\\Reals and \\Rn")
        XCTAssertFalse(m.editorNavigation.renameShown)
        XCTAssertEqual(m.editorNavigation.renamesApplied, ["Rename \\R to Reals: 3"])
        XCTAssertEqual(m.navigationNote, "Rename \\R to Reals: 2 of 2 buffers applied (⌘Z undoes the active document's edit).")
    }

    func testRenameLabelAcrossDocumentsAndRefusals() async {
        let m = model(main: "\\section{A}\\label{sec:a}\nSee \\ref{sec:a}.", ch: "\\autoref{sec:a} \\cref{sec:a,sec:b}")
        m.caretUTF16 = 20
        m.presentRenameSymbol()
        XCTAssertEqual(m.editorNavigation.renameSymbol, .label("sec:a"))
        m.editorNavigation.renameNewName = "sec:intro"
        m.planRenameSymbol()
        XCTAssertEqual(m.editorNavigation.renamePlan?.summary, "4 occurrences in 2 files")
        await m.applyRenameSymbol()
        XCTAssertEqual(m.pendingEdit?.text, "sec:intro}\nSee \\ref{sec:intro")
        XCTAssertEqual(m.documents[1].text, "\\autoref{sec:intro} \\cref{sec:intro,sec:b}")
        // Not on a symbol / a built-in command: refused with a note, no sheet.
        m.caretUTF16 = 9 // inside "A"
        m.presentRenameSymbol()
        XCTAssertFalse(m.editorNavigation.renameShown)
        XCTAssertEqual(m.navigationNote, "Caret is not on a \\label/\\ref key or a command.")
        m.caretUTF16 = 2 // \section
        m.presentRenameSymbol()
        XCTAssertEqual(m.navigationNote, "\\section has no \\newcommand/\\def definition in the project; only user-defined commands are renamed.")
    }

    func testGoToDefinitionSelectsAcrossOpenDocuments() {
        let m = model(main: "\\input{ch}\n$\\R$ \\begin{thm}x\\end{thm}", ch: "\\newcommand{\\R}{\\mathbb{R}}\n\\newenvironment{thm}{}{}")
        m.caretUTF16 = 13 // on \R
        m.goToDefinition()
        XCTAssertEqual(m.activePath, "ch.tex")
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 0, length: 27))
        XCTAssertEqual(m.navigationNote, "Definition: \\newcommand{\\R}{\\mathbb{R}} at line 1.")
        // Back in main: an environment with a \newenvironment goes to it; ⌘-click routing through the definition target.
        m.activePath = "main.tex"; m.caretUTF16 = 24
        XCTAssertEqual(EditorIntelligence.definitionTarget(in: m.activeText as NSString, at: 24), .environment(name: "thm"))
        m.goToDefinition()
        XCTAssertEqual(m.activePath, "ch.tex")
        XCTAssertEqual(m.selection?.nsRange.location, 28)
        // A standard command has no definition: explained.
        m.activePath = "main.tex"; m.caretUTF16 = 2
        XCTAssertEqual(EditorIntelligence.definitionTarget(in: m.activeText as NSString, at: 2), .command(name: "input"))
        m.goToDefinition(ofCommand: "section")
        XCTAssertEqual(m.navigationNote, "\\section has no \\newcommand/\\def/\\DeclareMathOperator definition in the open documents (a standard command).")
        // Hover peek.
        XCTAssertEqual(m.definitionSummary(forCommand: "R"), "\\newcommand{\\R}{\\mathbb{R}} (line 1 in ch.tex)")
        XCTAssertNil(m.definitionSummary(forCommand: "section"))
        let info = EditorIntelligence.quickInfo(in: "$\\R$" as NSString, at: 2, userDefinition: { m.definitionSummary(forCommand: $0) })
        XCTAssertEqual(info?.detail, "User command")
        XCTAssertEqual(info?.documentation, "Defined: \\newcommand{\\R}{\\mathbb{R}} (line 1 in ch.tex) — ⌘-click to go there.")
    }

    func testSymbolPickerRanksFuzzyMatches() {
        XCTAssertEqual(EN.fuzzyScore("", in: "anything"), 0)
        XCTAssertNil(EN.fuzzyScore("xyz", in: "Introduction"))
        XCTAssertGreaterThan(EN.fuzzyScore("intro", in: "Introduction")!, EN.fuzzyScore("intro", in: "An intro to things")!)
        XCTAssertGreaterThan(EN.fuzzyScore("eqm", in: "eq:main")!, EN.fuzzyScore("eqm", in: "equation-mess")!)
        let m = model(main: "\\section{Introduction}\\label{sec:intro}\n\\begin{theorem}\\end{theorem}", ch: "\\subsection{Intro results}")
        let all = m.symbolPickerEntries(query: "")
        XCTAssertEqual(all.map(\.item.title), ["Introduction", "sec:intro", "theorem", "Intro results"])
        let intro = m.symbolPickerEntries(query: "intro")
        XCTAssertEqual(intro.first?.item.title, "Introduction")
        XCTAssertEqual(Set(intro.map(\.item.title)), ["Introduction", "sec:intro", "Intro results"])
        m.goToSymbol(intro[1])
        XCTAssertEqual(m.activePath, "ch.tex")
        XCTAssertEqual(m.selection?.nsRange.location, 0)
    }
}
