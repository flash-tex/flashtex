import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// The preamble scan and the engine's report that route a document needing
/// Unicode/OpenType fonts to the compatibility engine (UnicodeFonts.swift;
/// lane P5-FONTSPEC-FALLBACK, the retirement plan's §4.4 and R10). The
/// window's side is in EngineChoiceTests.
@MainActor
final class UnicodeFontsTests: XCTestCase {
    private func scan(_ text: String, files: [String: String] = [:]) -> UnicodeFontsNeed? {
        UnicodeFonts.scan(entry: text, read: { files[$0] })
    }

    private func doc(_ preamble: String, body: String = "Hello.") -> String {
        "\\documentclass{article}\n\(preamble)\n\\begin{document}\n\(body)\n\\end{document}\n"
    }

    func testPackagesThatNeedXeTeXOrLuaTeX() {
        XCTAssertEqual(scan(doc("\\usepackage{fontspec}"))?.kind, .package("fontspec"))
        XCTAssertEqual(scan(doc("\\usepackage[math-style=ISO]{unicode-math}"))?.kind, .package("unicode-math"))
        XCTAssertEqual(scan(doc("\\usepackage{amsmath, polyglossia,graphicx}"))?.kind, .package("polyglossia"), "a comma list")
        XCTAssertEqual(scan(doc("\\usepackage[\n  options\n]{\n  xeCJK\n}"))?.kind, .package("xeCJK"), "over lines")
        XCTAssertEqual(scan(doc("\\RequirePackage{fontspec}"))?.kind, .package("fontspec"))
        XCTAssertEqual(scan(doc("\\usepackage{fontspec}"))?.file, "main.tex")
        XCTAssertNil(scan(doc("\\usepackage[T1]{fontenc}\n\\usepackage{lmodern,amsmath}")), "a pdfLaTeX document")
    }

    func testFontCommandsAndEngineChecks() {
        XCTAssertEqual(scan(doc("\\setmainfont{TeX Gyre Pagella}"))?.kind, .command("setmainfont"))
        XCTAssertEqual(scan(doc("\\newfontfamily\\headingfont{Inter}"))?.kind, .command("newfontfamily"))
        XCTAssertEqual(scan(doc("\\usepackage{iftex}\n\\RequireXeTeX"))?.kind, .command("RequireXeTeX"))
        XCTAssertEqual(scan(doc("\\RequireLuaTeX"))?.kind, .command("RequireLuaTeX"))
        XCTAssertEqual(scan(doc("\\font\\x=\"Hoefler Text\" at 12pt"))?.kind, .fontPrimitive, "XeTeX's \\font for a system font")
        XCTAssertEqual(scan(doc("\\font\\x=[fonts/Foo.otf]"))?.kind, .fontPrimitive, "and for a font file")
        XCTAssertNil(scan(doc("\\font\\x=cmr10 at 12pt")), "a TFM font is pdfTeX's")
        XCTAssertEqual(scan(doc("\\AtBeginDocument{\\setmainfont{Inter}}"))?.kind, .command("setmainfont"), "inside a group")
    }

    func testCommentsAndTheBodyDoNotCount() {
        XCTAssertNil(scan(doc("% \\usepackage{fontspec}\n\\usepackage{amsmath} % fontspec later")))
        XCTAssertNil(scan(doc("", body: "\\usepackage{fontspec} in the body is TeX's error, not this rule's")))
        XCTAssertNil(scan(doc("\\verb|\\%|\\usepackage{amsmath}")))
        XCTAssertEqual(scan(doc("100\\% \\usepackage{fontspec}"))?.kind, .package("fontspec"), "an escaped percent is no comment")
    }

    /// `% !TEX program = xelatex` (TeXShop, TeXstudio, VS Code's LaTeX Workshop).
    func testMagicComments() {
        XCTAssertEqual(scan("% !TEX program = xelatex\n" + doc(""))?.kind, .program("xelatex"))
        XCTAssertEqual(scan("%!TEX TS-program = LuaLaTeX\n" + doc(""))?.kind, .program("lualatex"))
        XCTAssertEqual(scan("% !TeX root = main.tex\n% !TeX program = lualatex\n" + doc(""))?.kind, .program("lualatex"))
        XCTAssertNil(scan("% !TEX program = pdflatex\n" + doc("")))
        XCTAssertNil(scan(doc("") + "% !TEX program = xelatex\n"), "only at the top")
    }

    /// Documents written for both engines load fontspec only under XeTeX or
    /// LuaTeX: the pdfLaTeX branch is what the new engine runs.
    func testEngineConditionalsAreFollowed() {
        XCTAssertNil(scan(doc("\\usepackage{iftex}\n\\ifxetex\n  \\usepackage{fontspec}\n\\else\n  \\usepackage[T1]{fontenc}\n\\fi")))
        XCTAssertNil(scan(doc("\\iftutex\\usepackage{fontspec}\\setmainfont{Inter}\\else\\usepackage{lmodern}\\fi")))
        XCTAssertNil(scan(doc("\\ifPDFTeX\\usepackage[T1]{fontenc}\\else\\usepackage{fontspec}\\fi")))
        XCTAssertNil(scan(doc("\\ifdefined\\XeTeXversion\\usepackage{fontspec}\\fi")))
        XCTAssertNil(scan(doc("\\ifx\\XeTeXversion\\undefined\\usepackage{lmodern}\\else\\usepackage{fontspec}\\fi")))
        XCTAssertEqual(scan(doc("\\ifpdftex\\usepackage{fontspec}\\fi"))?.kind, .package("fontspec"), "the pdfTeX branch counts")
        XCTAssertEqual(scan(doc("\\ifxetex\\else\\usepackage{fontspec}\\fi"))?.kind, .package("fontspec"))
        XCTAssertEqual(scan(doc("\\ifxetex\\ifnum1=1 \\fi\\fi\\usepackage{fontspec}"))?.kind, .package("fontspec"), "nested conditionals close")
        XCTAssertEqual(scan(doc("\\newif\\ifxetexdoc\n\\iftoggle{x}{a}{b}\n\\usepackage{fontspec}"))?.kind, .package("fontspec"),
                       "\\newif defines, \\iftoggle{…} is a command: neither opens a conditional")
    }

    /// The project's own class, packages and preamble files are part of the
    /// preamble; TeX Live's are not read (no `read` answer for them).
    func testProjectFilesTheEntryLoads() {
        let files = ["mythesis.cls": "\\LoadClass{report}\n\\RequirePackage{fontspec}",
                     "style.sty": "\\RequirePackage{amsmath}\n\\setmainfont{Inter}",
                     "pre.tex": "\\usepackage{unicode-math}",
                     "loop.sty": "\\RequirePackage{loop}"]
        XCTAssertEqual(scan("\\documentclass{mythesis}\n\\begin{document}\n\\end{document}", files: files),
                       UnicodeFontsNeed(kind: .package("fontspec"), file: "mythesis.cls"))
        XCTAssertEqual(scan(doc("\\usepackage{amsmath,style}"), files: files), UnicodeFontsNeed(kind: .command("setmainfont"), file: "style.sty"))
        XCTAssertEqual(scan(doc("\\input{pre}"), files: files), UnicodeFontsNeed(kind: .package("unicode-math"), file: "pre.tex"))
        XCTAssertNil(scan(doc("\\usepackage{loop}"), files: files), "a package that loads itself ends")
        XCTAssertNil(scan(doc("\\usepackage{style}")), "no project file: TeX Live's style.sty is not the document's")
    }

    /// The new engine's own report (modes §4.2 signal 2): the DIAGs the host
    /// sent for each package under TeX Live 2026 (NixOS PC, 2026-10-06).
    func testTheEnginesReport() {
        XCTAssertNotNil(UnicodeFonts.need(message: "Fatal Package fontspec Error: The fontspec package requires either XeTeX or",
                                          detail: "(fontspec)                      LuaTeX.\n"))
        XCTAssertNotNil(UnicodeFonts.need(message: "Package unicode-math Error: Cannot be run with pdftex!",
                                          detail: "(unicode-math)                Use XeLaTeX or LuaLaTeX instead."))
        XCTAssertNotNil(UnicodeFonts.need(message: "Critical Package xeCJK Error: The xeCJK package requires XeTeX to function."))
        XCTAssertNotNil(UnicodeFonts.need(message: ".", frames: ["*", "\\IFTEX@Require"]), "iftex's \\RequireXeTeX")
        XCTAssertNil(UnicodeFonts.need(message: "Undefined control sequence.", detail: nil))
        if case .engineReport(let m)? = UnicodeFonts.need(message: "Package unicode-math Error: Cannot be run with pdftex!")?.kind {
            XCTAssertEqual(m, "Package unicode-math Error: Cannot be run with pdftex!")
        } else { XCTFail("an engine report") }
    }

    func testTheBlockerSaysWhatAndYieldsToTheUser() {
        let b = EngineChoice.Blocker.unicodeFonts(UnicodeFontsNeed(kind: .package("fontspec"), file: "main.tex"))
        XCTAssertEqual(b.short, "this document needs Unicode fonts")
        XCTAssertTrue(b.detail.contains("the fontspec package (in main.tex)"), b.detail)
        XCTAssertTrue(b.yieldsToUserChoice)
        XCTAssertFalse(EngineChoice.Blocker.noTeXLive.yieldsToUserChoice)
        XCTAssertFalse(EngineChoice.Blocker.projectFonts(["text"]).yieldsToUserChoice)
        XCTAssertEqual(EngineFallbackBanner.headline(b), "Using compatibility engine: this document needs Unicode fonts.")
        // the rule applies to every unforced new-engine choice but the user's own
        for (source, blocked) in [(EngineChoice.Source.appSetting, true), (.record, true), (.builtInDefault, true), (.legacySwitch, true), (.user, false)] {
            var c = EngineChoice(preferred: .new, source: source)
            c.applyRule(b)
            XCTAssertEqual(c.effective, blocked ? .previous : .new, "\(source)")
        }
        var u = EngineChoice(preferred: .new, source: .user)
        u.applyRule(.noTeXLive)
        XCTAssertEqual(u.effective, .previous, "no TeX Live still blocks the user's choice")
    }
}
