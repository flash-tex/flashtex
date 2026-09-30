import XCTest
@testable import FlashTeXEditorCore

/// PLAN M6: the package index, root resolution, idempotent insertion and
/// the `auto_preamble` decision.
final class PreambleTests: XCTestCase {
    typealias T = TeXpand

    let doc = """
    \\documentclass[11pt]{article}
    \\usepackage[utf8]{inputenc}
    \\usepackage{amsmath, amssymb}% the math ones
    % \\usepackage{tikz}
    \\RequirePackage{xcolor}
    \\begin{document}
    \\usepackage{notapackage}
    Text.
    \\end{document}
    """

    func testIndexReadsPackagesCommaListsAndComments() {
        let index = T.PackageIndex.scan(doc)
        XCTAssertEqual(index.documentClass, "article")
        XCTAssertEqual(index.packages, ["inputenc", "amsmath", "amssymb", "xcolor"], "comment and body lines are skipped")
        let afterXcolor = NSMaxRange((doc as NSString).range(of: "\\RequirePackage{xcolor}\n"))
        XCTAssertEqual(index.insertionPoint, afterXcolor, "after the last package line")
    }

    func testClassImpliedPackages() {
        let beamer = T.PackageIndex.scan("\\documentclass{beamer}\n\\begin{document}\n")
        XCTAssertTrue(beamer.packages.isSuperset(of: ["amsmath", "graphicx", "hyperref"]))
        XCTAssertEqual(beamer.insertionPoint, 23, "no \\usepackage: after \\documentclass")
        XCTAssertNil(T.PackageIndex.scan("Just text.").insertionPoint, "no preamble")
    }

    func testInsertionIsIdempotent() {
        var text = doc
        let reqs = [T.PackageRequirement("tikz-cd"), T.PackageRequirement("amsmath"), T.PackageRequirement("cleveref", options: ["capitalize"])]
        let first = T.PackageIndex.scan(text).insertion(for: reqs, in: text)
        XCTAssertEqual(first?.text, "\\usepackage{tikz-cd}\n\\usepackage[capitalize]{cleveref}\n", "only what is missing")
        text = (text as NSString).replacingCharacters(in: NSRange(location: first!.location, length: 0), with: first!.text)
        XCTAssertNil(T.PackageIndex.scan(text).insertion(for: reqs, in: text), "a second run inserts nothing")
        let noNewline = "\\documentclass{article}"
        XCTAssertEqual(T.PackageIndex.scan(noNewline).insertion(for: [T.PackageRequirement("x")], in: noNewline)?.text, "\n\\usepackage{x}\n")
    }

    func testPreambleActions() {
        let tikz = [T.PackageRequirement("tikz-cd")]
        XCTAssertEqual(T.preambleAction(for: tikz, mode: .off, rootIsCurrent: true, rootText: doc), .none)
        guard case .insert(_, let text) = T.preambleAction(for: tikz, mode: .insert, rootIsCurrent: true, rootText: doc) else { return XCTFail() }
        XCTAssertEqual(text, "\\usepackage{tikz-cd}\n")
        guard case .prompt(let missing, _, _) = T.preambleAction(for: tikz, mode: .prompt, rootIsCurrent: true, rootText: doc) else { return XCTFail() }
        XCTAssertEqual(missing, tikz)
        XCTAssertEqual(T.preambleAction(for: tikz, mode: .insert, rootIsCurrent: false, rootText: doc), .notice(missing: tikz), "another file is the root")
        XCTAssertEqual(T.preambleAction(for: tikz, mode: .insert, rootIsCurrent: true, rootText: "no preamble"), .notice(missing: tikz))
        XCTAssertEqual(T.preambleAction(for: [T.PackageRequirement("amsmath")], mode: .insert, rootIsCurrent: true, rootText: doc), .none)
    }

    func testRootResolution() {
        XCTAssertEqual(T.rootPath(current: "chapters/intro.tex", currentText: "% !TEX root = ../main.tex\nText", projectMain: "thesis.tex"), "main.tex")
        XCTAssertEqual(T.rootPath(current: "chapters/intro.tex", currentText: "%!TeX root=book\n", projectMain: nil), "chapters/book.tex")
        XCTAssertEqual(T.rootPath(current: "intro.tex", currentText: "Text", projectMain: "main.tex"), "main.tex", "then the project's main file")
        XCTAssertEqual(T.rootPath(current: "solo.tex", currentText: "Text", projectMain: nil), "solo.tex", "then the file itself")
    }

    func testPackageConditionalVariantsFollowTheIndex() {
        let e = T.Engine()
        let physics = T.PackageIndex.scan("\\documentclass{article}\n\\usepackage{physics}\n").packages
        XCTAssertEqual(e.expandToString("dd:y/x", in: T.Context(scope: .math, packages: physics)), "\\dv{y}{x}")
        XCTAssertEqual(e.expandToString("dd:y/x", in: T.Context(scope: .math)), "\\frac{dy}{dx}")
    }
}
