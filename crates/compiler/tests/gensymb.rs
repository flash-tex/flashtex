//! gensymb `\degree`, `\celsius`, `\ohm` and `\perthousand` in text and math.
//!
//! Oracle (every number below was run, none recalled): TeX Live 2026
//! `pdflatex -interaction=nonstopmode` with `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1` on exactly:
//!
//! ```tex
//! \documentclass[10pt]{article}
//! \usepackage{gensymb}
//! \begin{document}
//! A $30\degree$ B 20\celsius{} C 5\ohm{} D 3\perthousand{} E.
//! \end{document}
//! ```
//!
//! Without textcomp gensymb fakes the symbols (`gensymb.sty`): `\degree`
//! is `\ensuremath{^\circ}`, `\celsius` is `$^\circ$C` in text and
//! `^\circ\mathrm{C}` in math, `\ohm` is `$\Omega$`; `\perthousand` (and
//! `\micro`) stay undefined (`Package gensymb Warning: Not defining
//! \perthousand`, then `! Undefined control sequence` at the use, the
//! glyph dropped).
//!
//! Measured `\showbox` widths (pt) on that run:
//!
//! - `\hbox{$30\degree$}` and `\hbox{$30^\circ$}` are byte-identical:
//!   `x14.59726` (`3`, `0`, a script hbox `x4.59723` of OMS/cmsy/m/n/7
//!   holding `^^N`, shifted `-3.62892`).
//! - `\hbox{$^\circ$}`: `x4.59723`.
//! - `\hbox{$\Omega$}`: `x7.22223`.
//! - `\hbox{$20^\circ\mathrm{C}$}` and `\hbox{20\celsius{}}` are
//!   byte-identical: `x21.81949`.
//! - `\hbox{5\ohm{}}`: `x12.22224`.
//!
//! Glyph origins (`mutool draw -F stext`, bp): the math degree is CMSY7
//! at x 169.47 raised to baseline 131.15 with B at 177.37; the celsius
//! degree is CMSY7 at 197.71 with C at 202.29; the ohm Omega is CMR10 at
//! 228.30 with D at 238.81; E sits at 258.05 with no per-mille before it
//! (undefined, dropped).
//!
//! This compiler always carries the text-companion glyphs, so
//! `\perthousand` sets U+2030 instead of erroring — the one deliberate
//! step beyond faked pdflatex. Absolute advances are the render
//! pipeline's (its metrics are Latin Modern, not Computer Modern), so
//! this file pins what the compiler owns: zero diagnostics, the exact
//! glyphs, and atom-for-atom equivalence with the spelled-out forms above
//! (whose own fidelity the existing script oracles pin to pdflatex).

use flashtex_compiler::diagnostics::Diagnostic;
use flashtex_compiler::math::{self, MathAtom, MathList};
use flashtex_compiler::parser::{parse, Block, Inline, Parsed};
use flashtex_compiler::{DocumentId, Span};

const PREAMBLE: &str = "\\documentclass{article}\n\\usepackage{gensymb}\n\\begin{document}\n";
const POSTAMBLE: &str = "\n\\end{document}\n";

fn doc(body: &str) -> String {
    format!("{PREAMBLE}{body}{POSTAMBLE}")
}

/// The first paragraph's inlines of `source` with the full parse.
fn paragraph(source: &str) -> (Parsed, Vec<Inline>) {
    let parsed = parse(source);
    let inlines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines.clone()),
            _ => None,
        })
        .unwrap_or_default();
    (parsed, inlines)
}

fn no_errors(parsed: &Parsed, source: &str) {
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

/// The inline math lists of a paragraph, in order.
fn maths(inlines: &[Inline]) -> Vec<&MathList> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Math { list, .. } => Some(list),
            _ => None,
        })
        .collect()
}

/// The text pieces of a paragraph, concatenated.
fn texts(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn despan_atom(atom: &mut MathAtom) {
    atom.span = Span::in_document(DocumentId(0), 0, 0);
    for script in [&mut atom.superscript, &mut atom.subscript]
        .into_iter()
        .flatten()
    {
        for inner in &mut script.atoms {
            despan_atom(inner);
        }
    }
    // The compared lists hold only symbol/text nuclei with scripts; a group
    // would be a construction change worth a new assertion, not silent
    // drift, so only scripts are normalised here.
}

/// A math list with every span erased: two lists are equal exactly when
/// they are the same atoms, the render pipeline's only input.
fn despan(list: &MathList) -> MathList {
    let mut out = list.clone();
    for atom in &mut out.atoms {
        despan_atom(atom);
    }
    out
}

fn math_width(list: &MathList) -> f64 {
    let mut diagnostics = Vec::new();
    let laid = math::layout(list, 10.0, &mut diagnostics);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    laid.width
}

/// The single inline math list of `body` (parsed with gensymb loaded).
fn single_math(body: &str) -> MathList {
    let (parsed, inlines) = paragraph(&doc(body));
    no_errors(&parsed, body);
    let lists = maths(&inlines);
    assert_eq!(lists.len(), 1, "{body:?}: {inlines:?}");
    lists[0].clone()
}

fn assert_same_layout(left: &MathList, right: &MathList, what: &str) {
    assert_eq!(despan(left), despan(right), "{what}");
    let (left_width, right_width) = (math_width(left), math_width(right));
    assert!(
        (left_width - right_width).abs() < 1e-9,
        "{what}: widths differ: {left_width} vs {right_width}"
    );
}

#[test]
fn acceptance_source_parses_with_no_errors_and_all_four_glyphs() {
    let body = "A $30\\degree$ B 20\\celsius{} C 5\\ohm{} D 3\\perthousand{} E.";
    let (parsed, inlines) = paragraph(&doc(body));
    // Zero diagnostics: the package loads silently and every command is
    // implemented, in both modes.
    no_errors(&parsed, body);
    // The prose around the symbols, in order: the celsius C the definition
    // supplies, then the letter C of the source.
    assert_eq!(texts(&inlines), "AB20CC5D3‰E.");
    let lists = maths(&inlines);
    assert_eq!(lists.len(), 3, "{inlines:?}");
    // `$30\degree$`: the circ sits on the `0` as its superscript.
    assert_eq!(lists[0].atoms.len(), 2, "{:?}", lists[0].atoms);
    let zero = &lists[0].atoms[1];
    assert_eq!(
        zero.nucleus,
        flashtex_compiler::math::Nucleus::Symbol("0".into())
    );
    let script = zero.superscript.as_ref().expect("degree superscript");
    assert_eq!(script.atoms.len(), 1, "{script:?}");
    assert_eq!(
        script.atoms[0].nucleus,
        flashtex_compiler::math::Nucleus::Symbol("∘".into())
    );
    // Text `\celsius`: an empty base carrying the same raised circ.
    assert_eq!(lists[1].atoms.len(), 1, "{:?}", lists[1].atoms);
    assert!(
        lists[1].atoms[0].superscript.is_some(),
        "{:?}",
        lists[1].atoms
    );
    // Text `\ohm`: a lone Omega.
    assert_eq!(lists[2].atoms.len(), 1, "{:?}", lists[2].atoms);
    assert_eq!(
        lists[2].atoms[0].nucleus,
        flashtex_compiler::math::Nucleus::Symbol("Ω".into())
    );
}

#[test]
fn degree_in_math_is_a_spelled_out_circ_superscript() {
    assert_same_layout(
        &single_math("$30\\degree$"),
        &single_math("$30^\\circ$"),
        "$30\\degree$ is $30^\\circ$",
    );
    assert_same_layout(
        &single_math("$\\degree$"),
        &single_math("$^\\circ$"),
        "$\\degree$ is $^\\circ$",
    );
}

#[test]
fn celsius_matches_its_faked_definition_in_both_modes() {
    // Text `20\celsius{}` is `$^\circ$C`: the math part is the spelled-out
    // leading superscript and the C is ordinary text (the `{}` group is
    // empty and harmless).
    let (parsed, inlines) = paragraph(&doc("20\\celsius{}"));
    no_errors(&parsed, "20\\celsius{}");
    assert_eq!(texts(&inlines), "20C", "{inlines:?}");
    let lists = maths(&inlines);
    assert_eq!(lists.len(), 1, "{inlines:?}");
    assert_same_layout(lists[0], &single_math("$^\\circ$"), "celsius math part");
    // Math `$20\celsius$` is `$20^\circ\mathrm{C}`: the upright C is its
    // own following atom.
    assert_same_layout(
        &single_math("$20\\celsius$"),
        &single_math("$20^\\circ\\mathrm{C}$"),
        "$20\\celsius$ is $20^\\circ\\mathrm{C}$",
    );
}

#[test]
fn ohm_matches_a_spelled_out_omega() {
    assert_same_layout(
        &single_math("5\\ohm{}"),
        &single_math("$\\Omega$"),
        "text \\ohm is $\\Omega$",
    );
    let list = single_math("$x\\ohm y$");
    let omega = list
        .atoms
        .iter()
        .find(|atom| atom.nucleus == flashtex_compiler::math::Nucleus::Symbol("Ω".into()));
    assert!(omega.is_some(), "{:?}", list.atoms);
}

#[test]
fn perthousand_sets_u2030_in_both_modes() {
    let (parsed, inlines) = paragraph(&doc("3\\perthousand{}"));
    no_errors(&parsed, "3\\perthousand{}");
    assert_eq!(texts(&inlines), "3‰", "{inlines:?}");
    let list = single_math("$x\\perthousand y$");
    let mille = list
        .atoms
        .iter()
        .find(|atom| atom.nucleus == flashtex_compiler::math::Nucleus::Text("‰".into()));
    assert!(mille.is_some(), "{:?}", list.atoms);
}

fn package_gate_errors(source: &str, name: &str, math: bool) {
    let parsed = parse(source);
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    let diagnostic: &Diagnostic = &parsed.diagnostics[0];
    let expected = if math {
        format!("\\{name} requires \\usepackage{{gensymb}}")
    } else {
        format!("\\{name} needs \\usepackage{{gensymb}}")
    };
    assert_eq!(diagnostic.message, expected, "{diagnostic:?}");
}

#[test]
fn plain_article_rejects_each_command_like_undefined() {
    // pdflatex answers "Undefined control sequence" and typesets nothing;
    // here each use names the missing package and likewise leaves nothing.
    for name in ["degree", "celsius", "ohm", "perthousand"] {
        let source = format!(
            "\\documentclass{{article}}\n\\begin{{document}}\n$\\{name}$\n\\end{{document}}\n"
        );
        package_gate_errors(&source, name, true);
        let source = format!(
            "\\documentclass{{article}}\n\\begin{{document}}\nx \\{name} y\n\\end{{document}}\n"
        );
        package_gate_errors(&source, name, false);
    }
    // Text leaves no glyph behind (math leaves the zero-width
    // missing-package atom, TeX's "nothing"); the space the failed command
    // stood in survives as interword glue, so no words join and none part.
    let (parsed, inlines) =
        paragraph("\\documentclass{article}\n\\begin{document}\nx \\degree y\n\\end{document}\n");
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert_eq!(texts(&inlines), "xy", "{inlines:?}");
    assert!(
        matches!(
            &inlines[..],
            [
                Inline::Text { .. },
                Inline::Text {
                    glue_before: Some(_),
                    ..
                }
            ]
        ),
        "{inlines:?}"
    );
}

fn heading_content(source: &str) -> (Parsed, Vec<Inline>) {
    let parsed = parse(source);
    let content = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content.clone()),
            _ => None,
        })
        .unwrap_or_default();
    (parsed, content)
}

#[test]
fn headings_set_gensymb_like_body_text() {
    // Headings flatten their argument instead of re-parsing it; without a
    // flat arm the commands vanished there in silence.
    let (parsed, content) = heading_content(&doc(
        "\\section{A 30\\degree, 5\\ohm, 3\\perthousand, 20\\celsius}",
    ));
    no_errors(&parsed, "section");
    assert_eq!(texts(&content), "A30,5,3‰,20C", "{content:?}");
    let lists = maths(&content);
    assert_eq!(lists.len(), 3, "{content:?}");
    for list in &lists {
        assert_eq!(list.atoms.len(), 1, "{list:?}");
    }
    let degree = despan(lists[0]);
    assert_eq!(degree, despan(&single_math("$^\\circ$")), "{degree:?}");
}

#[test]
fn headings_without_gensymb_name_the_package() {
    let (parsed, _) = heading_content(
        "\\documentclass{article}\n\\begin{document}\n\\section{A 30\\degree}\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\degree needs \\usepackage{gensymb}")),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn gensymb_options_keep_the_package_warning() {
    let parsed = parse("\\documentclass{article}\n\\usepackage[Omega]{gensymb}\n\\begin{document}\nx\n\\end{document}\n");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("recognised but not implemented")),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn a_document_macro_named_degree_wins_without_gensymb() {
    // `\degree` is not a kernel command, so `\newcommand{\degree}` works
    // exactly as in real LaTeX while the package is absent.
    let (parsed, inlines) = paragraph("\\newcommand{\\degree}{DEG}\n\\degree");
    no_errors(&parsed, "\\newcommand{\\degree}");
    assert_eq!(texts(&inlines), "DEG", "{inlines:?}");
}

#[test]
fn micro_stays_unsupported_like_faked_pdflatex() {
    // gensymb with no textcomp and no upmu option never defines `\micro`
    // ("Not defining \micro"), so it must keep its diagnostic here too.
    let parsed = parse(&doc("\\micro"));
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\micro")),
        "{:?}",
        parsed.diagnostics
    );
}
