//! PLAN1 slice 1: one falsifier per site where the pipeline re-derives a
//! layout fact from source bytes instead of from the compiler's node tree
//! (`docs/proposals/node-stream-inventory.md`, which numbers the sites;
//! `docs/proposals/generated-data-and-maintainability.md` §3.1).
//!
//! Every test renders the same construct twice: written directly, and
//! produced by a user macro (`\newcommand`, `\def`, `\let`, a project
//! `.sty`), so the bytes at the invocation's span differ from the tokens the
//! compiler expanded. pdfLaTeX sets both forms identically; the inventory
//! records, per site, that `pdflatex` (TeX Live 2026, MacTeX; oracle only)
//! gave the same glyph list for both forms. The assertion is therefore
//! "macro form == direct form" and needs no TeX at test time.
//!
//! Where the inventory says the compiler already carries the fact, the test
//! first asserts that the compiler's two block trees are equal once spans
//! are dropped (`Tree::Same`): the difference on the page is then the
//! pipeline's alone, and the migration of that site must make the test pass
//! without a compiler change. `Tree::Differs` marks the few sites where the
//! compiler's tree is also different (the fact is missing from, or wrong in,
//! the node stream), so fixing them needs a compiler change too.
//!
//! Every test is `#[ignore]`d with its site number so CI stays green; run
//! them with `cargo test --test node_stream_falsifiers -- --ignored` (each
//! fails today). A slice that migrates a site removes its `#[ignore]`.
//!
//! `NODE_STREAM_DUMP=<dir>` writes each pair's documents under
//! `<dir>/<test>/{direct,macro}/` for the pdflatex cross-check.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, Tick};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// One painted mark, positions in hundredths of a bp. Glyph runs are
/// flattened to glyphs, so where a run happens to split (which is not
/// visible on the page) never counts as a difference.
#[derive(Debug, Clone, PartialEq)]
enum Mark {
    Glyph { font: String, size: i64, gid: u16, x: i64, y: i64 },
    Rule { x: i64, top: i64, w: i64, h: i64 },
    Path { cmds: usize },
    Image { x: i64 },
}

/// Every page's marks, each with the text of the run it came from (used in
/// the failure message only).
fn marks(docs: &[(&str, &str)]) -> Vec<Vec<(Mark, String)>> {
    let fonts = FontSet::with_default_dirs(&[]);
    let sources: Vec<SourceDocument<'_>> = docs.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();
    let r = render(&sources, "main.tex", 1, "node-stream", &fonts, &RenderOptions::default());
    let hb = |t: Tick| (t.to_bp() * 100.0).round() as i64;
    let mut pages = Vec::new();
    for page in &r.v2.pages {
        let mut out = Vec::new();
        for item in page.resident_items() {
            match item {
                Item::GlyphRun(run) => {
                    let font = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).map(|f| f.postscript_name.clone()).unwrap_or_default();
                    for g in &run.glyphs {
                        let m = Mark::Glyph { font: font.clone(), size: run.font_size.0 as i64, gid: g.gid, x: hb(g.origin_x), y: hb(g.baseline_y) };
                        out.push((m, run.text.clone()));
                    }
                }
                Item::Rule(rule) => {
                    out.push((Mark::Rule { x: hb(rule.x), top: hb(rule.top), w: hb(rule.width), h: hb(rule.height) }, String::new()))
                }
                Item::Path(path) => out.push((Mark::Path { cmds: path.commands.len() }, String::new())),
                Item::Image(image) => out.push((Mark::Image { x: hb(image.x) }, String::new())),
            }
        }
        pages.push(out);
    }
    pages
}

/// `Debug` of `text` with every `Span { .. }` replaced by `S`.
fn strip_spans(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("Span {") {
        out.push_str(&rest[..i]);
        out.push('S');
        let tail = &rest[i..];
        rest = &tail[tail.find('}').expect("Span debug closes") + 1..];
    }
    out.push_str(rest);
    out
}

/// The compiler's block tree for `docs`, without spans.
fn tree(docs: &[(&str, &str)]) -> String {
    let sources: Vec<SourceDocument<'_>> = docs.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();
    let parsed = flashtex_compiler::parser::parse_project(&sources, "main.tex");
    strip_spans(&format!("{:?}", parsed.blocks))
}

/// What the compiler's own output does for this pair (see the module doc).
#[derive(Clone, Copy, PartialEq)]
enum Tree {
    Same,
    Differs,
}

fn dump(docs: &[(&str, &str)], side: &str) {
    let Some(dir) = std::env::var_os("NODE_STREAM_DUMP") else { return };
    let name = std::thread::current().name().unwrap_or("unnamed").replace("::", "_");
    let at = std::path::Path::new(&dir).join(name).join(side);
    std::fs::create_dir_all(&at).unwrap();
    for (path, text) in docs {
        std::fs::write(at.join(path), text).unwrap();
    }
}

fn falsify_docs(tree_kind: Tree, direct: &[(&str, &str)], via_macro: &[(&str, &str)]) {
    assert!(common::lm_available());
    dump(direct, "direct");
    dump(via_macro, "macro");
    if tree_kind == Tree::Same {
        let (a, b) = (tree(direct), tree(via_macro));
        if a != b {
            let n = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
            let lo = n.saturating_sub(60);
            panic!(
                "PRECONDITION: the compiler's trees now differ, so this falsifier no longer isolates the pipeline:\n  direct ..{}\n  macro  ..{}",
                &a[lo..(n + 100).min(a.len())],
                &b[lo..(n + 100).min(b.len())]
            );
        }
    }
    let a = marks(direct);
    let b = marks(via_macro);
    let key = |pages: &Vec<Vec<(Mark, String)>>| pages.iter().map(|p| p.iter().map(|m| m.0.clone()).collect::<Vec<_>>()).collect::<Vec<_>>();
    if key(&a) == key(&b) {
        return;
    }
    let mut msg = format!("pages: direct {} macro {}\n", a.len(), b.len());
    for (pi, (pa, pb)) in a.iter().zip(b.iter()).enumerate() {
        if let Some(i) = pa.iter().zip(pb.iter()).position(|(x, y)| x.0 != y.0) {
            msg += &format!("page {} mark {}:\n  direct {:?} in {:?}\n  macro  {:?} in {:?}\n", pi + 1, i, pa[i].0, pa[i].1, pb[i].0, pb[i].1);
        }
        if pa.len() != pb.len() {
            msg += &format!("page {} marks: direct {} macro {}\n", pi + 1, pa.len(), pb.len());
        }
    }
    panic!("the macro form sets differently from the direct form:\n{msg}");
}

fn falsify(tree_kind: Tree, direct: &str, via_macro: &str) {
    falsify_docs(tree_kind, &[("main.tex", direct)], &[("main.tex", via_macro)]);
}

/// An `article` document.
fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// A paragraph long enough to fill several lines.
const LONG: &str = "The quick brown fox jumps over the lazy dog while seventeen wizards quietly box the jovial frogs of the marsh, and every sentence here keeps going so that the paragraph fills several lines of the page.";

use Tree::{Differs, Same};

// ---- A. Style scope -------------------------------------------------------

#[test]
fn site01_declaration_from_def_body() {
    falsify(Same, &doc("", "Some {\\bfseries bold words} here."), &doc("\\def\\B{\\bfseries}\n", "Some {\\B bold words} here."));
}

#[test]
fn site02_preamble_let_leaks_declaration() {
    falsify(Same, &doc("", "Some plain words here."), &doc("\\let\\B\\bfseries\n", "Some plain words here."));
}

#[test]
fn site03_nested_argument_wrapper() {
    falsify(
        Same,
        &doc("", "Some \\textbf{bold words} here."),
        &doc("\\newcommand\\Cc[1]{\\textbf{#1}}\\newcommand\\Bb[1]{\\Cc{#1}}\n", "Some \\Bb{bold words} here."),
    );
}

#[test]
fn site04_declaration_macro_from_project_sty() {
    falsify_docs(
        Same,
        &[("main.tex", &doc("", "Some {\\itshape it words} here."))],
        &[("main.tex", &doc("\\usepackage{mine}\n", "Some {\\I it words} here.")), ("mine.sty", "\\newcommand\\I{\\itshape}\n")],
    );
}

#[test]
fn site05_quad_size_after_size_macro() {
    falsify(Same, &doc("", "A {\\Large\\quad x} y."), &doc("\\newcommand\\bigL{\\Large}\n", "A {\\bigL\\quad x} y."));
}

#[test]
fn site06_italic_correction_before_eqref() {
    falsify(
        Same,
        &doc("\\usepackage{amsmath}\n", "\\begin{equation}x\\label{e}\\end{equation}\\textit{see \\eqref{e}} x"),
        &doc("\\usepackage{amsmath}\n\\newcommand\\er[1]{\\eqref{#1}}\n", "\\begin{equation}x\\label{e}\\end{equation}\\textit{see \\er{e}} x"),
    );
}

// ---- B. Interword glue and input conventions ------------------------------

#[test]
fn site07_gap_before_macro_dash() {
    // The trees differ only in segmentation: the compiler splits `word`,
    // `—`, `and` at the macro boundary and gives both `space_before: false`,
    // which is right; the pipeline sets a space there anyway.
    falsify(Differs, &doc("", "A word---and more."), &doc("\\newcommand\\dsh{---}\n", "A word\\dsh and more."));
}

#[test]
fn site08_control_space() {
    falsify(Same, &doc("", "A\\ B."), &doc("\\newcommand\\csp{\\ }\n", "A\\csp B."));
}

#[test]
fn site09_empty_group_breaks_ligature() {
    falsify(Same, &doc("", "Shelf{}ful words."), &doc("\\newcommand\\nl{{}}\n", "Shelf\\nl ful words."));
}

#[test]
fn site10_tie_from_macro() {
    falsify(Same, &doc("", "See Figure~7 and more words here."), &doc("\\newcommand\\fig{Figure~7}\n", "See \\fig{} and more words here."));
}

#[test]
#[ignore = "PLAN1 site 11: accent composition requires the span to be a 2-byte `\\'` command"]
fn site11_accent_from_macro() {
    falsify(Same, &doc("", "Caf\\'e ok."), &doc("\\newcommand\\cafe{Caf\\'e}\n", "\\cafe{} ok."));
}

#[test]
fn site12_citation_through_macro() {
    let bib = "\\begin{thebibliography}{9}\\bibitem{k} A. Author.\\end{thebibliography}";
    falsify(Same, &doc("", &format!("See \\cite{{k}} now.\n{bib}")), &doc("\\newcommand\\mc[1]{\\cite{#1}}\n", &format!("See \\mc{{k}} now.\n{bib}")));
}

#[test]
fn site13_footnote_in_two_argument_macro() {
    falsify(Same, &doc("", "Word\\footnote{Note.} more."), &doc("\\newcommand\\fn[2]{#1\\footnote{#2}}\n", "\\fn{Word}{Note.} more."));
}

#[test]
fn site14_hfil_from_macro() {
    falsify(Same, &doc("", "\\noindent A\\hfil B\\hfill C"), &doc("\\newcommand\\hf{\\hfil}\n", "\\noindent A\\hf B\\hfill C"));
}

#[test]
#[ignore = "PLAN1 site 15: url_run_at needs `\\url{` bytes at the span (the compiler's tree also differs)"]
fn site15_url_through_macro() {
    falsify(Differs, &doc("\\usepackage{url}\n", "See \\url{http://a.b/c}."), &doc("\\usepackage{url}\n\\newcommand\\uu{\\url{http://a.b/c}}\n", "See \\uu."));
}

// ---- C. Paragraph and vertical structure ----------------------------------

#[test]
fn site16_noindent_from_macro() {
    falsify(Same, &doc("", "First para.\n\n\\noindent Second para."), &doc("\\newcommand\\noi{\\noindent}\n", "First para.\n\n\\noi Second para."));
}

#[test]
#[ignore = "PLAN1 site 17: body_commands reads \\markboth from the bytes only"]
fn site17_markboth_from_macro() {
    falsify(Same, &doc("", "\\pagestyle{headings}\\markboth{L}{R}Text."), &doc("\\newcommand\\mb{\\markboth{L}{R}}\n", "\\pagestyle{headings}\\mb Text."));
}

#[test]
#[ignore = "PLAN1 site 18: body_commands finds \\chapter in the bytes only"]
fn site18_chapter_from_macro() {
    falsify(
        Same,
        "\\documentclass{report}\n\\begin{document}\n\\chapter{Intro}Text.\n\\end{document}\n",
        "\\documentclass{report}\n\\newcommand\\ch[1]{\\chapter{#1}}\n\\begin{document}\n\\ch{Intro}Text.\n\\end{document}\n",
    );
}

#[test]
fn site19_heading_mark_from_title_macro() {
    falsify(
        Same,
        &doc("\\pagestyle{headings}\n", "\\section{Big Title}Text.\\newpage More."),
        &doc("\\pagestyle{headings}\n\\newcommand\\ttl{Big Title}\n", "\\section{\\ttl}Text.\\newpage More."),
    );
}

#[test]
#[ignore = "PLAN1 site 20: run_in_heading_at scans back for `\\paragraph{` bytes"]
fn site20_run_in_heading_from_macro() {
    falsify(Same, &doc("", "\\paragraph{Head} Body text."), &doc("\\newcommand\\pp[1]{\\paragraph{#1}}\n", "\\pp{Head} Body text."));
}

#[test]
fn site21_list_opened_by_macro() {
    falsify(
        Same,
        &doc("", "\\begin{itemize}\\item A\\item B\\end{itemize}"),
        &doc("\\newcommand\\bi{\\begin{itemize}}\\newcommand\\ei{\\end{itemize}}\n", "\\bi\\item A\\item B\\ei"),
    );
}

#[test]
fn site22_list_closed_by_macro() {
    falsify(
        Same,
        &doc("", "\\begin{itemize}\\item A\\end{itemize}%\nAfter text."),
        &doc("\\newcommand\\ei{\\end{itemize}}\n", "\\begin{itemize}\\item A\\ei\nAfter text."),
    );
}

#[test]
#[ignore = "PLAN1 site 23: a trivlist environment's topsep is found from `\\begin{center}` bytes in the gap"]
fn site23_center_opened_by_macro() {
    falsify(
        Same,
        &doc("", "Before.\n\\begin{center}Mid.\\end{center}%\nAfter."),
        &doc("\\newcommand\\bc{\\begin{center}}\\newcommand\\ec{\\end{center}}\n", "Before.\n\\bc Mid.\\ec\nAfter."),
    );
}

#[test]
fn site24_equation_opened_by_macro() {
    falsify(
        Same,
        &doc("", "Text\n\\begin{equation}x=1\\end{equation}\n\nmore."),
        &doc("\\newcommand\\be{\\begin{equation}}\\newcommand\\ee{\\end{equation}}\n", "Text\n\\be x=1\\ee\n\nmore."),
    );
}

#[test]
#[ignore = "PLAN1 site 25: strip_tag finds `\\tag` from atom-span bytes (so does the compiler's custom_tag_text)"]
fn site25_tag_from_macro() {
    falsify(
        Differs,
        &doc("\\usepackage{amsmath}\n", "\\begin{equation}x=1\\tag{A}\\end{equation}"),
        &doc("\\usepackage{amsmath}\n\\newcommand\\mt{\\tag{A}}\n", "\\begin{equation}x=1\\mt\\end{equation}"),
    );
}

#[test]
#[ignore = "PLAN1 site 26: the \\qedhere box is found from `\\qedhere` bytes"]
fn site26_qedhere_from_macro() {
    falsify(
        Same,
        &doc("\\usepackage{amsmath,amsthm}\n", "\\begin{proof}Thus \\[x=1.\\qedhere\\]\\end{proof}"),
        &doc("\\usepackage{amsmath,amsthm}\n\\newcommand\\qh{\\qedhere}\n", "\\begin{proof}Thus \\[x=1.\\qh\\]\\end{proof}"),
    );
}

#[test]
#[ignore = "PLAN1 site 27: a proof head is recognised from `\\begin{proof}` bytes"]
fn site27_proof_opened_by_macro() {
    falsify(
        Same,
        &doc("\\usepackage{amsthm}\n", "\\begin{proof}Easy.\\end{proof}"),
        &doc("\\usepackage{amsthm}\n\\newcommand\\bp{\\begin{proof}}\\newcommand\\ep{\\end{proof}}\n", "\\bp Easy.\\ep"),
    );
}

#[test]
#[ignore = "PLAN1 site 28: theorem_environments collects names from literal `\\newtheorem{..}` bytes"]
fn site28_newtheorem_through_macro() {
    falsify(
        Same,
        &doc("\\newtheorem{thm}{Theorem}\n", "\\begin{thm}Body.\\end{thm}\nAfter."),
        &doc("\\newcommand\\nt[1]{\\newtheorem{#1}{Theorem}}\\nt{thm}\n", "\\begin{thm}Body.\\end{thm}\nAfter."),
    );
}

#[test]
fn site29_par_after_display_from_macro() {
    falsify(Same, &doc("", "Text\n\\[x\\]\n\\par\nnext."), &doc("\\newcommand\\pr{\\par}\n", "Text\n\\[x\\]\n\\pr\nnext."));
}

#[test]
fn site30_vspace_em_from_macro() {
    falsify(Same, &doc("", "A.\n\n{\\Large\\vspace{2em}}\nB."), &doc("\\newcommand\\gap{\\vspace{2em}}\n", "A.\n\n{\\Large\\gap}\nB."));
}

#[test]
fn site31_setlist_in_uncalled_definition() {
    falsify(
        Same,
        &doc("\\usepackage{enumitem}\n", "\\begin{itemize}\\item A\\item B\\end{itemize}"),
        &doc("\\usepackage{enumitem}\n\\newcommand\\stl{\\setlist{itemsep=10pt}}\n", "\\begin{itemize}\\item A\\item B\\end{itemize}"),
    );
}

#[test]
#[ignore = "PLAN1 site 32: \\pagestyle is read from bytes (the compiler's tree also drops it through a macro)"]
fn site32_pagestyle_from_macro() {
    falsify(Differs, &doc("", "\\pagestyle{empty}Text."), &doc("\\newcommand\\ps{\\pagestyle{empty}}\n", "\\ps Text."));
}

// ---- D. Preamble facts ----------------------------------------------------

#[test]
#[ignore = "PLAN1 site 33: apply_preamble_lengths never expands a preamble macro that calls \\setlength"]
fn site33_preamble_setlength_from_macro() {
    falsify(Same, &doc("\\setlength{\\parindent}{0pt}\n", "Para one.\n\nPara two."), &doc("\\newcommand\\np{\\setlength{\\parindent}{0pt}}\\np\n", "Para one.\n\nPara two."));
}

#[test]
#[ignore = "PLAN1 site 34: counter() takes a \\setcounter inside an uncalled definition as in force"]
fn site34_setcounter_in_uncalled_definition() {
    falsify(Same, &doc("", "\\section{A}Text."), &doc("\\newcommand\\scn{\\setcounter{secnumdepth}{0}}\n", "\\section{A}Text."));
}

#[test]
#[ignore = "PLAN1 site 35: DocumentSetup::from_preamble reads \\geometry from the preamble bytes"]
fn site35_geometry_from_macro() {
    falsify(Same, &doc("\\usepackage{geometry}\n\\geometry{margin=1in}\n", "Text."), &doc("\\usepackage{geometry}\n\\newcommand\\gm{\\geometry{margin=1in}}\\gm\n", "Text."));
}

#[test]
#[ignore = "PLAN1 site 36: document_sloppy finds \\sloppy in the bytes only"]
fn site36_sloppy_from_macro() {
    let t = "Pneumonoultramicroscopicsilicovolcanoconiosis antidisestablishmentarianism floccinaucinihilipilification supercalifragilisticexpialidocious hippopotomonstrosesquippedaliophobia pseudopseudohypoparathyroidism incomprehensibilities uncharacteristically.";
    falsify(Same, &doc("", &format!("\\sloppy {t} {t}")), &doc("\\newcommand\\slp{\\sloppy}\n", &format!("\\slp {t} {t}")));
}

#[test]
#[ignore = "PLAN1 site 37: ColumnMode::scan finds \\twocolumn in the bytes only"]
fn site37_twocolumn_from_macro() {
    falsify(Same, &doc("", &format!("\\twocolumn {LONG}")), &doc("\\newcommand\\tc{\\twocolumn}\n", &format!("\\tc {LONG}")));
}

#[test]
#[ignore = "PLAN1 site 38: author_groups splits \\author at `\\and` only when the span starts with `\\author`"]
fn site38_author_and_from_macro() {
    falsify(
        Same,
        &doc("\\title{T}\\author{Ann \\and Bob}\\date{}\n", "\\maketitle Text."),
        &doc("\\title{T}\\newcommand\\au{\\author{Ann \\and Bob}}\\au\\date{}\n", "\\maketitle Text."),
    );
}

#[test]
#[ignore = "PLAN1 site 39: body_commands finds \\tableofcontents in the bytes only"]
fn site39_tableofcontents_from_macro() {
    falsify(Same, &doc("", "\\tableofcontents\n\\section{A}Text."), &doc("\\newcommand\\toc{\\tableofcontents}\n", "\\toc\n\\section{A}Text."));
}

#[test]
#[ignore = "PLAN1 site 40: abstractenv::ranges finds `\\begin{abstract}` bytes only"]
fn site40_abstract_opened_by_macro() {
    falsify(
        Same,
        &doc("", "\\begin{abstract}Short summary.\\end{abstract}%\nBody."),
        &doc("\\newcommand\\ba{\\begin{abstract}}\\newcommand\\ea{\\end{abstract}}\n", "\\ba Short summary.\\ea\nBody."),
    );
}

// ---- E. Floats, multicols, TikZ, math -------------------------------------

#[test]
#[ignore = "PLAN1 site 41: floats::scan/mask (lib.rs) blank literal `\\begin{figure}` bytes before the compiler runs"]
fn site41_figure_opened_by_macro() {
    // The compiler never sees the direct form's float (it is masked), so the
    // trees differ by construction until floats are compiler blocks.
    falsify(
        Differs,
        &doc("", "Text before.\n\\begin{figure}[b]\\centering\\rule{2cm}{1cm}\\caption{Cap}\\end{figure}%\nText after."),
        &doc("\\newcommand\\bfig{\\begin{figure}[b]}\\newcommand\\efig{\\end{figure}}\n", "Text before.\n\\bfig\\centering\\rule{2cm}{1cm}\\caption{Cap}\\efig\nText after."),
    );
}

#[test]
#[ignore = "PLAN1 site 42: floats::pieces finds \\caption in the float body's bytes only"]
fn site42_caption_from_macro() {
    falsify(
        Same,
        &doc("", "Text.\n\\begin{figure}[t]\\centering\\rule{2cm}{1cm}\\caption{Cap}\\end{figure}\nMore."),
        &doc("\\newcommand\\capt{\\caption}\n", "Text.\n\\begin{figure}[t]\\centering\\rule{2cm}{1cm}\\capt{Cap}\\end{figure}\nMore."),
    );
}

#[test]
#[ignore = "PLAN1 site 43: multicol::scan masks literal `\\begin{multicols}` bytes, including inside a preamble definition"]
fn site43_multicols_opened_by_macro() {
    falsify(
        Differs,
        &doc("\\usepackage{multicol}\n", &format!("Intro {LONG}\n\n\\begin{{multicols}}{{2}}%\n{LONG}\n\\end{{multicols}}\n\nOutro {LONG}")),
        &doc(
            "\\usepackage{multicol}\n\\newcommand\\bmc{\\begin{multicols}{2}}\\newcommand\\emc{\\end{multicols}}\n",
            &format!("Intro {LONG}\n\n\\bmc\n{LONG}\n\\emc\n\nOutro {LONG}"),
        ),
    );
}

#[test]
#[ignore = "PLAN1 site 44: tikzpicture extents and bodies are lexed from `\\begin{tikzpicture}` bytes"]
fn site44_tikzpicture_from_macro() {
    falsify(
        Same,
        &doc("\\usepackage{tikz}\n", "A \\begin{tikzpicture}\\draw (0,0) -- (1,1);\\end{tikzpicture} B."),
        &doc("\\usepackage{tikz}\n\\newcommand\\pic{\\begin{tikzpicture}\\draw (0,0) -- (1,1);\\end{tikzpicture}}\n", "A \\pic{} B."),
    );
}

#[test]
#[ignore = "PLAN1 site 45: typeset.rs operator_limits_of reads `\\lim` bytes at the atom's span"]
fn site45_operator_limits_from_macro() {
    falsify(Same, &doc("", "\\[\\lim_{n\\to\\infty} a_n\\]"), &doc("\\newcommand\\lm{\\lim}\n", "\\[\\lm_{n\\to\\infty} a_n\\]"));
}

#[test]
#[ignore = "PLAN1 site 46: RowsEnv::at reads the rows environment's name from `\\begin{gather}` bytes"]
fn site46_gather_through_environment() {
    // amsmath collects a `gather` body up to a literal `\end{gather}`, so a
    // `\newcommand` cannot open it; `\newenvironment` over `\gather` is the
    // form amsmath documents. The compiler does not expand `\gather` yet
    // (its tree for the macro form is plain text), so this pair isolates the
    // site only once that lands.
    falsify(
        Differs,
        &doc("\\usepackage{amsmath}\n", "\\begin{gather}a=b\\\\c=d\\end{gather}"),
        &doc("\\usepackage{amsmath}\n\\newenvironment{eqs}{\\gather}{\\endgather}\n", "\\begin{eqs}a=b\\\\c=d\\end{eqs}"),
    );
}
