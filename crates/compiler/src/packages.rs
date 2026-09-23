//! Project `.sty`/`.cls` files: which package names the engine reads from
//! the project and which it keeps modelling itself.
//!
//! `\usepackage{name}` (and `\RequirePackage`, `\documentclass`,
//! `\LoadClass`) is resolved in this order (proposal
//! `docs/proposals/packages-fonts-manifest.md`, decision 1):
//!
//! 1. a package in [`BUILT_IN_PACKAGES`] / a class in [`BUILT_IN_CLASSES`]
//!    is never read from a file: the parser and the render pipeline model
//!    it *completely* for the options they accept, and the real file needs
//!    typesetting primitives the expansion engine does not have (a stray
//!    `amsmath.sty` next to `main.tex` would stop at its first `\hbox`);
//! 2. otherwise `name.sty`/`name.cls` among the project documents: the
//!    first document, in the order the project layer hands them over,
//!    whose path ends in `name.ext`. That order is the precedence (S2 of
//!    the proposal): the entry's closure, then the entry directory's own
//!    package files, then each manifest `texinputs` directory in turn. The
//!    file is executed through the expansion engine (`crate::expansion`);
//! 3. otherwise, for the packages flipped off the built-in list so far
//!    (just `appendix`), the vendored real file ([`APPENDIX_STY`]), so a
//!    document needs no TeX Live install to run it;
//! 4. otherwise the command passes through to the parser, whose "recognised
//!    but not implemented" warning names what was searched
//!    ([`search_description`]).
//!
//! Everything the typesetter has a model for stays built in whether that
//! model is complete or partial: a project-local copy of a package the
//! parser knows *some* commands of (`caption`, `url`, `float`, ...) would
//! otherwise replace a working partial model with a file that fails on its
//! first primitive. The list is printed in the supported-LaTeX inventory
//! (`crate::supported`).

use std::rc::Rc;

use flashtex_tex_expansion::PackageReader;

use crate::parser::SourceDocument;

/// Packages whose built-in model wins over a project file of the same name,
/// each with the reason its file cannot be executed here. Every package
/// `parser::package_matches_layout` accepts, every one the parser or the
/// render pipeline implements commands of, and every one the kernel
/// feasibility probe showed needs missing primitives.
pub const BUILT_IN_PACKAGES: &[(&str, &str)] = &[
    // -- math --
    ("amsmath", "displays, \\dfrac/\\binom/\\operatorname and the align family are parsed and laid out by crate::math; amsmath.sty needs \\halign, \\setbox and \\mathchoice"),
    ("amssymb", "the msam/msbm symbol inventory is a table (crate::amssymb); amssymb.sty needs \\DeclareMathSymbol on real font encodings"),
    ("amsfonts", "the amsfonts subset and \\mathbb/\\mathfrak are tables; the file needs \\DeclareFontFamily"),
    ("amsthm", "\\newtheorem, \\theoremstyle and proof are crate::theorems; amsthm.sty needs \\hbox and \\vskip"),
    ("mathtools", "amsmath extensions parsed by crate::math; the file needs \\setbox and \\mathchoice"),
    ("bm", "\\bm is a bold math switch in crate::math; bm.sty needs \\mathchardef tables and \\font"),
    ("physics", "\\dv, \\pdv, \\abs & co. are parsed by crate::math; the file needs \\mathchoice"),
    ("cancel", "\\cancel/\\bcancel/\\xcancel are crate::math frames; cancel.sty needs \\hbox and \\vrule"),
    ("siunitx", "numbers, units and quantities are crate::siunitx; siunitx.sty is expl3 code"),
    ("mhchem", "parsed by crate::math; the file is expl3 code"),
    // -- text and page --
    ("geometry", "the page frame is crates/class-geometry; geometry.sty needs \\pdfpagewidth and \\hsize"),
    ("fancyhdr", "\\pagestyle{fancy} fields are parser state; fancyhdr.sty needs \\vbox and \\hrule"),
    ("scrlayer-scrpage", "\\pagestyle{scrheadings} and the six inner/centre/outer fields are the fancyhdr parser state; scrlayer-scrpage.sty needs \\vbox and layer primitives"),
    ("titlesec", "sectioning shapes are the render pipeline's; titlesec.sty needs \\vbox and \\hangindent"),
    ("titling", "title-block hooks are parser state"),
    ("setspace", "\\onehalfspacing/\\doublespacing are parser leading state; setspace.sty needs \\baselineskip arithmetic"),
    ("parskip", "\\parskip/\\parindent are read by the render pipeline from the package name"),
    ("multicol", "multicols is laid out by the render pipeline; multicol.sty needs \\output and \\vsplit"),
    ("enumitem", "list keys are parser state; enumitem.sty needs \\hbox and list primitives"),
    ("microtype", "protrusion and expansion are crates/microtype; microtype.sty needs pdfTeX's \\pdfprotrudechars"),
    ("csquotes", "\\enquote is a parser command; csquotes.sty needs \\lccode tables and expl3"),
    ("xspace", "\\xspace is a parser command; xspace.sty needs \\futurelet on a space-factor table"),
    ("relsize", "\\larger/\\smaller are parser font state; relsize.sty needs \\fontdimen"),
    ("ulem", "\\uline/\\sout are parser decorations; ulem.sty needs \\hbox and \\vrule"),
    ("soul", "\\so/\\hl are parser decorations; soul.sty needs \\hbox and \\discretionary"),
    ("textcomp", "text symbols are the Unicode text tables; the file needs \\DeclareTextSymbol"),
    ("lipsum", "\\lipsum text is a parser table"),
    ("verbatim", "verbatim and comment environments are read by the lexer; verbatim.sty needs \\catcode tricks on \\obeylines output"),
    ("comment", "the comment environment is read by the lexer"),
    ("alltt", "the alltt environment is read by the parser; alltt.sty needs \\catcode tricks on \\obeylines output"),
    ("url", "\\url is parsed raw by the lexer; url.sty needs \\discretionary and \\catcode tricks"),
    ("nameref", "\\nameref is crate::xref"),
    // -- encodings, fonts, languages --
    ("inputenc", "source text is decoded as UTF-8; inputenc.sty needs \\DeclareInputText on active characters"),
    ("fontenc", "T1/OT1 are the text encoding tables; fontenc.sty needs \\DeclareFontEncoding"),
    ("lmodern", "Latin Modern is the render pipeline's font set; lmodern.sty needs \\DeclareFontFamily"),
    ("fontspec", "\\setmainfont & co. are font settings (proposal S4); fontspec.sty is expl3 code"),
    ("unicode-math", "`\\setmathfont{…}` selects the OpenType math font (and the package alone selects Latin Modern Math); the file is expl3 code"),
    ("babel", "language selection is not modelled; babel.sty needs \\language and \\lccode tables"),
    ("CJKutf8", "the CJK environment, \\CJKfamily and the space switches are parser state and the render pipeline sets the characters from the C70 subfont metrics; CJKutf8.sty needs active characters and \\lastkern"),
    ("CJK", "loaded by CJKutf8; CJK.sty needs active characters, \\lastkern and \\pdffontattr"),
    ("iftex", "\\ifpdftex & co. would misreport the engine; the file tests primitives"),
    ("ifxetex", "\\ifxetex is the parser's; the file tests primitives"),
    ("ifluatex", "\\ifluatex is the parser's; the file tests primitives"),
    ("calc", "\\setlength arithmetic is the engine's \\dimexpr; calc.sty needs \\dimen registers with \\advance semantics"),
    ("etoolbox", "toggles are the engine's HOST_PRELUDE; etoolbox.sty needs \\numexpr on \\catcode tables and \\afterassignment tricks"),
    ("ifthen", "\\ifthenelse is an engine primitive"),
    // -- tables --
    ("array", "column types and the row strut are crate::tabular; array.sty needs \\halign"),
    ("tabularx", "X columns are crate::tabular; tabularx.sty needs \\setbox and \\halign"),
    ("booktabs", "rules are crate::tabular; booktabs.sty needs \\hrule and \\noalign"),
    ("longtable", "page-breaking tables are crate::tabular; longtable.sty needs \\output"),
    ("multirow", "multirow entries are crate::tabular; multirow.sty needs \\vbox"),
    ("colortbl", "cell colours are crate::tabular; colortbl.sty needs \\noalign and \\leaders"),
    ("caption", "caption shapes are the render pipeline's; caption.sty needs \\hbox and \\vbox"),
    ("subcaption", "subfigures are the render pipeline's; the file needs caption's machinery"),
    ("float", "[H] placement is the render pipeline's; float.sty needs \\output"),
    ("wrapfig", "wrapped figures are the render pipeline's; wrapfig.sty needs \\parshape and \\output"),
    ("tcolorbox", "the tcolorbox environment (colback/colframe) is the parser's; tcolorbox.sty needs pgf and \\setbox"),
    // -- graphics and colour --
    ("graphicx", "\\includegraphics is the render pipeline's; graphicx.sty needs \\pdfximage and \\setbox"),
    ("graphics", "\\includegraphics is the render pipeline's; graphics.sty needs \\pdfximage and \\setbox"),
    ("xcolor", "colour models are crate::color; xcolor.sty needs \\pdfcolorstack and \\special"),
    ("color", "colour models are crate::color; color.sty needs \\special"),
    ("tikz", "pictures are crates/vector-graphics; tikz.sty and pgf need \\pdfliteral and \\setbox"),
    ("pgf", "pgf needs \\pdfliteral and \\setbox"),
    ("pgfplots", "pgfplots needs pgf"),
    ("listings", "listings are the render pipeline's; listings.sty needs \\catcode tricks on \\obeylines output"),
    // -- references and bibliographies --
    ("hyperref", "links are the PDF writer's; hyperref.sty needs \\pdfstartlink and \\special"),
    ("cleveref", "\\cref is crate::xref; cleveref.sty patches \\refstepcounter with \\protected@write"),
    ("natbib", "citations are crate::natbib; natbib.sty needs \\bibitem output"),
    ("cite", "sorted, range-compressed citations with cite.sty's own separator glue are crate::bib; cite.sty needs \\futurelet on the token after \\cite and \\lastskip/\\lastpenalty"),
    ("biblatex", "citations are crate::biblatex; biblatex.sty is expl3 code"),
    // -- beamer --
    ("beamerthemedefault", "beamer themes are crates/class-geometry"),
];

/// Classes whose page model is `crates/class-geometry`'s (or the parser's,
/// for the AMS sizes); their files need `\output`, `\vbox` and font
/// primitives the engine lacks, so a project copy is never executed.
pub const BUILT_IN_CLASSES: &[(&str, &str)] = &[
    ("article", "size1x.clo page model in crates/class-geometry"),
    ("report", "size1x.clo page model in crates/class-geometry"),
    ("book", "size1x.clo page model in crates/class-geometry"),
    ("letter", "letter.cls page model in crates/class-geometry"),
    ("beamer", "beamer's frame model in crates/class-geometry"),
    ("scrartcl", "KOMA typearea in crates/class-geometry"),
    ("scrarticle", "KOMA typearea in crates/class-geometry"),
    ("scrreprt", "KOMA typearea in crates/class-geometry"),
    ("scrbook", "KOMA typearea in crates/class-geometry"),
    ("amsart", "AMS size tables in the parser"),
    ("amsbook", "AMS size tables in the parser"),
    ("amsproc", "AMS size tables in the parser"),
];

/// Whether `name.ext` is modelled by the typesetter and never read from a
/// project file (`ext` is `sty` or `cls`).
pub fn is_built_in(name: &str, ext: &str) -> bool {
    let list = if ext == "cls" { BUILT_IN_CLASSES } else { BUILT_IN_PACKAGES };
    list.iter().any(|(built_in, _)| *built_in == name)
}

/// Whether the project document at `path` is what `\usepackage{name}`
/// names: `name.ext` itself, or a path ending in `/name.ext` (`name` may
/// carry a directory of its own, `\usepackage{styles/mystyle}`).
fn path_names(path: &str, name: &str, ext: &str) -> bool {
    let file = format!("{name}.{ext}");
    path == file || path.strip_suffix(file.as_str()).is_some_and(|dir| dir.ends_with('/'))
}

/// The index of the project document `\usepackage{name}` resolves to, if
/// the project has one and the name is not built in: the first document in
/// project order that [`path_names`] it.
pub fn resolve(documents: &[SourceDocument<'_>], name: &str, ext: &str) -> Option<usize> {
    if is_built_in(name, ext) || name.is_empty() || !crate::parser::path_is_safe(name) {
        return None;
    }
    documents.iter().position(|d| path_names(d.path, name, ext))
}

/// Where a package that was not found was looked for, for the parser's
/// diagnostic: `mystyle.sty next to main.tex`.
pub fn search_description(entry_path: &str, name: &str, ext: &str) -> String {
    let entry_name = entry_path.rsplit('/').next().unwrap_or(entry_path);
    let entry_name = if entry_name.is_empty() { "the entry document" } else { entry_name };
    format!("{name}.{ext} next to {entry_name}")
}

/// Whether a project document is a package or class file.
pub fn is_package_file(path: &str) -> bool {
    path.ends_with(".sty") || path.ends_with(".cls")
}

/// The real `appendix.sty` (v1.2c), vendored from TeX Live 2026 into
/// `tex-expansion/vendor-packages/` with its provenance header: the first
/// package flipped from a built-in no-op to running the real file through
/// the expansion engine. Like the hyphenation patterns, it is read with
/// `include_str!` so a document needs no TeX Live install; the bytes past
/// the provenance header are byte-for-byte the upstream file.
pub const APPENDIX_STY: &str = include_str!("../../tex-expansion/vendor-packages/appendix.sty");

/// The engine's package reader for a project: owns `files`, the
/// `(path, text)` of every `.sty`/`.cls` document in project order (the
/// closure outlives the borrow, and travels with incremental checkpoints;
/// the text is the one the expansion pass prepared, verbatim regions
/// blanked, like an `\input` file's), declines built-in names, resolves
/// the rest exactly as [`resolve`] does, and falls back to the vendored
/// real file for the flipped packages ([`APPENDIX_STY`]) when the project
/// has no file of its own.
pub fn reader(files: Vec<(String, String)>) -> PackageReader {
    Rc::new(move |name, ext| {
        if name.is_empty() || !crate::parser::path_is_safe(name) {
            return None;
        }
        if is_built_in(name, ext) {
            return None;
        }
        if let Some((_, text)) = files.iter().find(|(path, _)| path_names(path, name, ext)) {
            return Some(text.clone());
        }
        if ext == "sty" && name == "appendix" {
            return Some(APPENDIX_STY.to_string());
        }
        None
    })
}

/// The `.sty`/`.cls` documents in path order, as `(path, text)`: what the
/// incremental cache compares to learn that a package file was edited.
pub fn package_texts(documents: &[SourceDocument<'_>]) -> Vec<(String, String)> {
    documents
        .iter()
        .filter(|d| is_package_file(d.path))
        .map(|d| (d.path.to_string(), d.text.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs<'a>(paths: &[&'a str]) -> Vec<SourceDocument<'a>> {
        paths.iter().map(|p| SourceDocument { path: p, text: "" }).collect()
    }

    #[test]
    fn built_ins_are_never_resolved_to_files() {
        let documents = docs(&["main.tex", "amsmath.sty", "article.cls", "mystyle.sty"]);
        assert_eq!(resolve(&documents, "amsmath", "sty"), None);
        assert_eq!(resolve(&documents, "article", "cls"), None);
        assert_eq!(resolve(&documents, "mystyle", "sty"), Some(3));
        assert_eq!(resolve(&documents, "mystyle", "cls"), None);
        assert!(reader(package_texts(&documents))("amsmath", "sty").is_none());
        assert!(reader(package_texts(&documents))("mystyle", "sty").is_some());
    }

    /// Project order is precedence: the entry's own directory comes before
    /// `texinputs` in the set the project layer builds, and a name may
    /// carry a directory.
    #[test]
    fn the_first_matching_document_in_project_order_wins() {
        let documents = docs(&["paper/main.tex", "paper/mystyle.sty", "texinputs/0/mystyle.sty", "shared.sty", "styles/x.sty"]);
        assert_eq!(resolve(&documents, "mystyle", "sty"), Some(1));
        assert_eq!(resolve(&documents, "shared", "sty"), Some(3));
        assert_eq!(resolve(&documents, "styles/x", "sty"), Some(4));
        assert_eq!(resolve(&documents, "x", "sty"), Some(4));
        assert_eq!(resolve(&documents, "nothere", "sty"), None);
        assert_eq!(resolve(&documents, "../mystyle", "sty"), None, "no parent traversal");
        assert_eq!(resolve(&documents, "ystyle", "sty"), None, "a suffix of a file name is not the name");
    }

    #[test]
    fn search_descriptions_name_what_was_tried() {
        assert_eq!(search_description("main.tex", "mystyle", "sty"), "mystyle.sty next to main.tex");
        assert_eq!(search_description("paper/main.tex", "thesis", "cls"), "thesis.cls next to main.tex");
    }

    /// Every package `package_matches_layout` accepts silently is built in:
    /// a project file could not replace a model the parser applies anyway.
    #[test]
    fn every_layout_neutral_package_is_built_in() {
        for package in crate::supported::layout_neutral_packages() {
            assert!(is_built_in(package, "sty"), "{package} is modelled by the parser but not in BUILT_IN_PACKAGES");
        }
    }
}
