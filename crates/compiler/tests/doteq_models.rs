//! `\doteq` and `\models` as the LaTeX kernel builds them, not as single
//! glyphs: `\doteq` is `\buildrel\textstyle.\over=`
//! (`\mathrel{\mathop{\kern\z@ =}\limits^{\textstyle .}}`, fontmath.ltx
//! 365) and `\models` is `\mathrel{|}\joinrel\Relbar` (fontmath.ltx 380).
//!
//! Oracle: live pdflatex (TeX Live 2026, 10pt article), run as
//! `pdflatex -interaction=nonstopmode <file>.tex` in a scratch directory:
//!
//! * `repro.tex` (`\documentclass{article}`, `$a\doteq b$` and `$c\models d$`)
//!   compiles with 0 errors (`grep -c "^! " repro.log` prints `0`). PDF
//!   glyph origins (baseline `y_top`, in bp) extracted with the repo's own
//!   `tools/visual-oracle/pdftext.py` (`PdfDocument.load` + `page_glyphs`):
//!   `a`@148.71 dot@159.24,129.12 `=`@156.75 `b`@167.26;
//!   `c`@148.71 bar@155.79 `=`@156.90 `d`@167.42. These agree with the
//!   wave-builder row to the extraction's rounding (a@148.71 =@156.75,
//!   dot at 159.24,129.12, b@167.27; bar@201.32 =@202.42 on its own line:
//!   dot-minus-= 2.49bp, =-minus-bar 1.10bp either way).
//! * `show.tex` (same setup plus `\showboxdepth=10 \showboxbreadth=100`)
//!   with `\setbox0=\hbox{$a\doteq b$}\showbox0` gives the kernel structure:
//!   `a`, thick 2.77771, then an hbox 7.7778 wide holding a vbox with
//!   `kern1.0`, the text-style `.` (cmmi "3A) centred over the width via
//!   fil glue, `kern1.99998`, and `=` (cmr), then thick 2.77771, `b` —
//!   total 22.91077pt. `\setbox0=\hbox{$c\models d$}\showbox0` gives `c`,
//!   thick 2.77771, `|` (cmsy "6A, 2.77779 wide), an hbox -1.66663 wide
//!   holding `\kern-1.66663` (the `\joinrel` `\mkern-3mu` in its
//!   `\mathrel`), an hbox 7.7778 wide holding `=` (the `\Relbar`), thick
//!   2.77771, `d` — total 23.9768pt. Middle-kern probes
//!   (`$\mathop{=}\limits^{g}$` keeps kern 1.11111 for a sup of depth
//!   1.3611) confirm the Rule-13a form max(1.11111, 2.0 - depth), whose
//!   depth-0 branch is the 1.99998 above the `=`.
//!
//! Representation: `\models` is a Rel `Group` of Rel-forced `|` and `=`
//! around the -3mu kern (the `\bowtie` convention); `\doteq` is the
//! existing `Stacked` shape (an Op-with-limits downstream) with a
//! Rel-forced class, laid out by the buildrel geometry in `layout_nucleus`
//! (text-size dot, Rule-13a raise). Both set 0 diagnostics; both keep
//! thick (5mu) space against ordinary neighbours (Rel class).
use flashtex_compiler::diagnostics::Diagnostic;
use flashtex_compiler::lexer::tokenize;
use flashtex_compiler::math::{layout, parse_tokens, MathBox, MathList, MathPackages};

/// 0.1bp in TeX pt: every positional assertion below compares against a
/// measured pdflatex value within this tolerance (1pt = 72/72.27bp).
const TOL_PT: f64 = 0.1 * 72.27 / 72.0;

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOL_PT,
        "{what}: {actual} != {expected} (tolerance 0.1bp)"
    );
}

fn parsed(source: &str) -> (MathList, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let list = parse_tokens(&tokenize(source), MathPackages::KERNEL, &mut diagnostics);
    (list, diagnostics)
}

fn lay_out(source: &str) -> (MathBox, Vec<Diagnostic>) {
    let (list, mut diagnostics) = parsed(source);
    let laid = layout(&list, 10.0, &mut diagnostics);
    (laid, diagnostics)
}

/// The x origin of the first item whose text is `text`.
fn x_of(laid: &MathBox, text: &str) -> f64 {
    laid.items
        .iter()
        .find(|i| i.text == text)
        .unwrap_or_else(|| panic!("no {text:?} in {laid:?}"))
        .x
}

/// The (x, baseline) origin of the first item whose text is `text`.
fn origin_of(laid: &MathBox, text: &str) -> (f64, f64) {
    let item = laid
        .items
        .iter()
        .find(|i| i.text == text)
        .unwrap_or_else(|| panic!("no {text:?} in {laid:?}"));
    (item.x, item.baseline)
}

#[test]
fn doteq_is_a_rel_stack_of_a_text_size_dot_over_equals() {
    let (list, diagnostics) = parsed(r"a\doteq b");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(list.atoms.len(), 3, "{list:?}");
    let debug = format!("{:?}", list.atoms[1].nucleus);
    assert!(debug.starts_with("Stacked"), "{debug}");
    assert_eq!(
        format!("{:?}", list.atoms[1].class_override),
        "Some(Rel)",
        "{list:?}"
    );
    assert!(
        debug.contains(r#"Symbol("=")"#) && debug.contains(r#"Symbol(".")"#),
        "{debug}"
    );
}

#[test]
fn doteq_glyph_origins_match_pdflatex() {
    let (laid, diagnostics) = lay_out(r"a\doteq b");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    // pdflatex showbox, in pt from the formula start: a is 5.28590 wide,
    // thick is 2.77771, so `=` sits at 8.06331.
    let (eq_x, eq_base) = origin_of(&laid, "=");
    close(eq_base, 0.0, "`=` on the math baseline");
    // The dot is centred over the 7.7778-wide `=`: (7.7778 - 2.77779)/2 =
    // 2.50000 to the right of it (pdftext: 159.24 - 156.75 = 2.49bp), and
    // its baseline is the middle kern (1.99998) over the `=` ink top
    // (3.66875) above the math baseline (pdftext: 134.76 - 129.12 = 5.64bp).
    let (dot_x, dot_base) = origin_of(&laid, ".");
    close(dot_x - eq_x, 2.50000, "dot centred over `=`");
    close(dot_base, -5.66873, "dot baseline above the math baseline");
    // `b` follows the `=` plus a thick space (pdftext: 167.26 - 156.75).
    close(
        x_of(&laid, "b") - eq_x,
        7.7778 + 2.77771,
        "`b` after `=` plus thick",
    );
    // Alone, the relation is exactly as wide as its `=` (7.7778pt).
    let (alone, diagnostics) = lay_out(r"\doteq");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    close(alone.width, 7.7778, "lone doteq width");
    // Rel class: a thick space separates the `a` from the relation, using
    // this layout's own `a` advance (matching pdflatex's TFM advance for
    // the letters is not part of this slice).
    let (only_a, _) = lay_out("a");
    close(
        eq_x - only_a.width,
        2.77771,
        "thick space before the relation",
    );
}

#[test]
fn models_is_a_rel_join_of_bar_kern_and_equals() {
    let (list, diagnostics) = parsed(r"c\models d");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(list.atoms.len(), 3, "{list:?}");
    let debug = format!("{:?}", list.atoms[1].nucleus);
    assert!(debug.starts_with("Group"), "{debug}");
    assert_eq!(
        format!("{:?}", list.atoms[1].class_override),
        "Some(Rel)",
        "{list:?}"
    );
    assert!(
        debug.contains(r#"Symbol("|")"#) && debug.contains(r#"Symbol("=")"#),
        "{debug}"
    );
}

#[test]
fn models_glyph_origins_match_pdflatex() {
    let (laid, diagnostics) = lay_out(r"c\models d");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    // pdflatex showbox, in pt: `|` is 2.77779 wide, the join kern is
    // -1.66663, so `=` sits 1.11116 to the right of the bar
    // (pdftext: 156.90 - 155.79 = 1.11bp).
    let (bar_x, bar_base) = origin_of(&laid, "|");
    let (eq_x, eq_base) = origin_of(&laid, "=");
    close(bar_base, 0.0, "bar on the math baseline");
    close(eq_base, 0.0, "`=` on the math baseline");
    close(eq_x - bar_x, 2.77779 - 1.66663, "`=` after the joined bar");
    // `d` follows the `=` plus a thick space (pdftext: 167.42 - 156.90).
    close(
        x_of(&laid, "d") - eq_x,
        7.7778 + 2.77771,
        "`d` after `=` plus thick",
    );
    // Alone, the relation is bar + join kern + `=`:
    // 2.77779 - 1.66663 + 7.7778 = 8.88896pt.
    let (alone, diagnostics) = lay_out(r"\models");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    close(alone.width, 8.88896, "lone models width");
    // Rel class: a thick space separates the `c` from the bar, using this
    // layout's own `c` advance.
    let (only_c, _) = lay_out("c");
    close(
        bar_x - only_c.width,
        2.77771,
        "thick space before the relation",
    );
}

#[test]
fn overset_dot_over_equals_keeps_its_script_size_mark() {
    // `\overset{.}{=}` builds the same two lists as `\doteq` but leaves
    // the class underived, so it must keep the shared script-size
    // `\overset` geometry rather than the buildrel one: a 7pt mark whose
    // baseline is not the Rule-13a raise.
    let (laid, diagnostics) = lay_out(r"\overset{.}{=}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let dot = laid.items.iter().find(|i| i.text == ".").expect("the dot");
    assert_eq!(dot.size, 7.0, "{laid:?}");
    assert!(
        (dot.baseline - -5.66873).abs() > 1.0,
        "overset must not take the doteq raise: {laid:?}"
    );
    let (alone, _) = lay_out(r"\doteq");
    let alone_dot = alone.items.iter().find(|i| i.text == ".").expect("the dot");
    assert_eq!(alone_dot.size, 10.0, "{alone:?}");
    close(alone_dot.baseline, -5.66873, "lone doteq dot raise");
}
