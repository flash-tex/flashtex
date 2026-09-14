import AppKit
import XCTest
@testable import FlashTeXMac

/// What hover resolves a `\ref`, `\cite` or `\includegraphics` to
/// (EditorHoverResolution.swift). Pure: every case drives the resolvers
/// directly, with the project supplied as in-memory documents and one
/// `fileExists` closure, so nothing here touches a shell model or a disk.
final class EditorHoverResolutionTests: XCTestCase {
    typealias EI = EditorIntelligence
    typealias Context = EditorIntelligence.HoverContext

    // MARK: labels

    func testLabelUnderAHeadingResolvesToThatHeading() {
        let text = """
        \\section{Fourier \\emph{analysis}}
        \\label{sec:fourier}
        Text.
        \\subsection{Convergence}
        \\label{sec:conv}
        """
        let fourier = EI.labelTarget(forKey: "sec:fourier", in: text)
        XCTAssertEqual(fourier?.kind, "Section")
        XCTAssertEqual(fourier?.text, "Fourier analysis") // \emph stripped, argument kept
        XCTAssertEqual(fourier?.line, 2)
        XCTAssertEqual(EI.labelTarget(forKey: "sec:conv", in: text)?.summary, "Subsection “Convergence” — line 5")
        XCTAssertNil(EI.labelTarget(forKey: "sec:missing", in: text))
    }

    func testStarredHeadingIsMarkedUnnumbered() {
        let text = "\\section*{Preface}\n\\label{sec:pre}\n"
        XCTAssertEqual(EI.labelTarget(forKey: "sec:pre", in: text)?.kind, "Section (unnumbered)")
    }

    func testLabelInAFloatResolvesToItsCaption() {
        let text = """
        \\section{Results}
        \\begin{figure}
        \\includegraphics{plot.pdf}
        \\caption{A sine wave at $2\\pi$}
        \\label{fig:sine}
        \\end{figure}
        \\begin{table}
        \\caption{Timings}
        \\label{tab:times}
        \\end{table}
        """
        let fig = EI.labelTarget(forKey: "fig:sine", in: text)
        XCTAssertEqual(fig?.kind, "Figure")
        XCTAssertEqual(fig?.text, "A sine wave at 2") // `\pi` and the `$` are markup, not text
        // The float wins over the enclosing section.
        XCTAssertEqual(EI.labelTarget(forKey: "tab:times", in: text)?.summary, "Table “Timings” — line 9")
    }

    func testLabelInAMathEnvironmentHasNoTextOfItsOwn() {
        let text = "\\section{S}\n\\begin{align}\n  x &= 1 \\label{eq:x}\n\\end{align}\n"
        let eq = EI.labelTarget(forKey: "eq:x", in: text)
        XCTAssertEqual(eq?.kind, "Equation")
        XCTAssertNil(eq?.text)
        XCTAssertEqual(eq?.summary, "Equation — line 3")
    }

    func testNestedEnvironmentsTakeTheInnermostLabelledOne() {
        let text = "\\begin{figure}\n\\caption{Outer}\n\\begin{table}\n\\caption{Inner}\n\\label{t}\n\\end{table}\n\\end{figure}\n"
        XCTAssertEqual(EI.labelTarget(forKey: "t", in: text)?.text, "Inner")
    }

    func testLabelInAnotherOpenDocumentNamesItsFile() {
        let context = Context(otherDocuments: [.init(path: "ch/two.tex", text: "\\chapter{Methods}\n\\label{ch:methods}\n")])
        let target = EI.labelTarget(forKey: "ch:methods", in: "\\ref{ch:methods}", context: context)
        XCTAssertEqual(target?.summary, "Chapter “Methods” — line 2 in ch/two.tex")
    }

    func testACommandEndingInLabelIsNotALabel() {
        // `\mylabel{x}` and `\ref{x}` must not be mistaken for `\label{x}`.
        XCTAssertNil(EI.labelTarget(forKey: "x", in: "\\mylabel{x}\\ref{x}"))
    }

    // MARK: bibliography

    private let bib = """
    @string{ams = "AMS"}
    @article{knuth84,
      author  = {Knuth, Donald E. and Lamport, Leslie},
      title   = {Literate {Programming}},
      journal = {The Computer Journal},
      year    = 1984,
    }
    @book{lamport86,
      author    = "Lamport, Leslie",
      title     = "LaTeX: A Document Preparation System",
      publisher = {Addison-Wesley},
      year      = {1986}
    }
    """

    func testBibTeXEntryIsParsedIntoASummary() {
        let entry = EI.bibEntry(forKey: "knuth84", inBibTeX: bib)
        XCTAssertEqual(entry?.type, "article")
        XCTAssertEqual(entry?.author, "Knuth and Lamport") // two authors keep both surnames
        XCTAssertEqual(entry?.title, "Literate Programming") // the protective braces go
        XCTAssertEqual(entry?.year, "1984")
        XCTAssertEqual(entry?.summary, "Knuth and Lamport — Literate Programming (1984) · The Computer Journal")
    }

    func testQuotedValuesAndPublisherFallback() {
        let entry = EI.bibEntry(forKey: "lamport86", inBibTeX: bib)
        XCTAssertEqual(entry?.author, "Lamport")
        XCTAssertEqual(entry?.summary, "Lamport — LaTeX: A Document Preparation System (1986) · Addison-Wesley")
        XCTAssertNil(EI.bibEntry(forKey: "ams", inBibTeX: bib)) // @string is not an entry
        XCTAssertNil(EI.bibEntry(forKey: "nobody", inBibTeX: bib))
    }

    func testThreeOrMoreAuthorsBecomeEtAl() {
        XCTAssertEqual(EI.shortenAuthors("Knuth, D. and Lamport, L. and Beeton, B."), "Knuth et al.")
        XCTAssertEqual(EI.shortenAuthors("Donald Knuth"), "Knuth") // no comma: last word is the surname
    }

    func testBibitemInTheBufferResolvesBeforeAnyBibFile() {
        let text = """
        \\begin{thebibliography}{9}
        \\bibitem[Knu84]{knuth84} D. Knuth, \\emph{Literate Programming}, 1984.
        \\bibitem{other} Somebody else.
        \\end{thebibliography}
        """
        let entry = EI.bibliographyEntry(forKey: "knuth84", in: text)
        XCTAssertEqual(entry?.summary, "D. Knuth, Literate Programming, 1984.")
        XCTAssertEqual(EI.bibliographyEntry(forKey: "other", in: text)?.summary, "Somebody else.")
    }

    func testCitationResolvesThroughTheProjectsBibFile() {
        let context = Context(otherDocuments: [.init(path: "refs.bib", text: bib)])
        let entry = EI.bibliographyEntry(forKey: "knuth84", in: "\\cite{knuth84}", context: context)
        XCTAssertEqual(entry?.path, "refs.bib")
        XCTAssertEqual(entry?.title, "Literate Programming")
        XCTAssertNil(EI.bibliographyEntry(forKey: "knuth84", in: "\\cite{knuth84}")) // nothing to resolve against
    }

    // MARK: graphics

    func testGraphicsPathDirectoriesAreTriedInOrderThenTheDocumentsOwn() {
        let text = "\\graphicspath{{figures/}{img}}\n\\includegraphics{plot}"
        XCTAssertEqual(EI.graphicsSearchPaths(in: text), ["figures/", "img/", ""]) // a missing `/` is added
        XCTAssertEqual(EI.graphicsSearchPaths(in: "no graphicspath here"), [""])
        let candidates = EI.graphicsCandidates(for: "plot", in: text)
        XCTAssertEqual(candidates.prefix(6).map { $0 },
                       ["figures/plot.pdf", "figures/plot.png", "figures/plot.jpg", "figures/plot.jpeg", "figures/plot.eps", "img/plot.pdf"])
        XCTAssertEqual(candidates.count, 15) // 3 roots × 5 extensions
        // A path that already carries a known extension is tried as written.
        XCTAssertEqual(EI.graphicsCandidates(for: "plot.png", in: text), ["figures/plot.png", "img/plot.png", "plot.png"])
    }

    func testResolutionReportsTheFileThatExists() {
        let text = "\\graphicspath{{figures/}}\n"
        let context = Context(otherDocuments: [], canProbeFiles: true) { $0 == "figures/plot.png" }
        let found = EI.resolveGraphics("plot", in: text, context: context)
        XCTAssertEqual(found.resolved, "figures/plot.png")
        XCTAssertEqual(found.summary, "Resolves to figures/plot.png")
    }

    func testAMissingFileSaysWhatWasTriedAndAnUnsavedProjectSaysNothing() {
        let context = Context(otherDocuments: [], canProbeFiles: true) { _ in false }
        let missing = EI.resolveGraphics("plot.pdf", in: "", context: context)
        XCTAssertNil(missing.resolved)
        XCTAssertFalse(missing.unknown)
        XCTAssertEqual(missing.summary, "No file found — tried plot.pdf")
        // No project root: the hover must not claim the file is missing.
        let unsaved = EI.resolveGraphics("plot.pdf", in: "", context: Context())
        XCTAssertTrue(unsaved.unknown)
        XCTAssertEqual(unsaved.summary, "Resolved against the project root when the document is saved.")
    }

    // MARK: through quickInfo

    func testQuickInfoShowsTheResolvedTargets() throws {
        let source = """
        \\section{Fourier analysis}\\label{sec:f}
        See \\ref{sec:f} and \\cite{knuth84}.
        \\includegraphics{plot}
        """
        let context = Context(otherDocuments: [.init(path: "refs.bib", text: bib)], canProbeFiles: true) { $0 == "plot.png" }
        let ns = source as NSString
        let ref = try XCTUnwrap(EI.quickInfo(in: ns, at: ns.range(of: "sec:f", options: .backwards).location, context: context))
        XCTAssertEqual(ref.title, "sec:f")
        XCTAssertEqual(ref.documentation, "Section “Fourier analysis” — line 1\n⌘-click to go to \\label{sec:f}.")
        let cite = try XCTUnwrap(EI.quickInfo(in: ns, at: ns.range(of: "knuth84").location, context: context))
        XCTAssertEqual(cite.detail, "Citation key")
        XCTAssertEqual(cite.documentation,
                       "Knuth and Lamport — Literate Programming (1984) · The Computer Journal\nin refs.bib\n⌘-click to go to the bibliography entry.")
        let image = try XCTUnwrap(EI.quickInfo(in: ns, at: ns.range(of: "plot", options: .backwards).location, context: context))
        XCTAssertEqual(image.detail, "Graphics file")
        XCTAssertEqual(image.documentation, "Resolves to plot.png")
    }

    func testUnresolvedKeysSayThatPlainly() throws {
        let ns = "See \\ref{nope} and \\cite{nobody}." as NSString
        let ref = try XCTUnwrap(EI.quickInfo(in: ns, at: ns.range(of: "nope").location))
        XCTAssertEqual(ref.documentation, "No \\label{nope} in this document or the open ones.\n⌘-click to go to \\label{nope}.")
        let cite = try XCTUnwrap(EI.quickInfo(in: ns, at: ns.range(of: "nobody").location))
        XCTAssertEqual(cite.documentation, "No bibliography entry found for this key.\n⌘-click to go to the bibliography entry.")
    }

    // MARK: plain text

    func testPlainTextStripsMarkupButKeepsArguments() {
        XCTAssertEqual(EI.plainText("The \\emph{Best} Idea"), "The Best Idea")
        XCTAssertEqual(EI.plainText("A \\& B \\% C"), "A & B % C")
        XCTAssertEqual(EI.plainText("  spaced   out \n text "), "spaced out text")
    }
}
