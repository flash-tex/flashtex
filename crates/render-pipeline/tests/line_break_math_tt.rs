//! Two line-break decisions against pdflatex that were about the horizontal
//! list's *modelling*, not widths (every other word on the page was within
//! 0.5 bp; the visual-oracle run of 2026-09-18 ranked them 7 and 8):
//!
//! * `\lstinline` under `breaklines=true` (`listings-manual` page 2): listings
//!   sets the argument token by token, each an `\hbox` followed by an empty
//!   `\discretionary{}{}{}`, and a blank as `\hbox{\ }` (`listings::
//!   break_inline`). pdflatex ends the line after `--set` and opens the next
//!   with the blank, 5.25 pt in; the pipeline kept the whole argument one
//!   unbreakable run and overflowed the line by 100 pt.
//! * a formula holding a top-level kern (`\pmod`, `\bmod`, `\,`, `\quad`...)
//!   (`inline-math` page 1): `typeset::inline_break_points` walked the
//!   nested layout `layout_kerned` used to build and gave up on the flat
//!   one it builds now, so `$1 = p(...) + ab v_1 v_2 \equiv 0 \pmod p$` was
//!   one 249 pt box. That forced the line before it to break inside the
//!   previous formula (after `=`, `\relpenalty` 500, demerits 250144) where
//!   pdflatex breaks at the glue before it (`gives`, demerits 1681).
//!
//! Oracle: `/Library/TeX/texbin/pdflatex` (pdfTeX 3.141592653-2.6-1.40.29,
//! TeX Live 2026, MacTeX; never in the product path) on exactly the
//! documents below, word origins read from the PDF's content stream with
//! `tools/visual-oracle/pdftext.py` (bp, page top-left origin). No TeX runs
//! here.

mod common;

use common::*;
use std::collections::BTreeMap;

/// Words per line as `(baseline in 0.1 bp, [(x, text)])`, in reading order.
fn lines(r: &flashtex_render_pipeline::Rendered) -> Vec<(i64, Vec<(f64, String)>)> {
    let mut by_line: BTreeMap<i64, Vec<(f64, String)>> = BTreeMap::new();
    for w in words_of(r) {
        if w.page == 1 {
            by_line.entry((w.baseline * 10.0).round() as i64).or_default().push((w.x, w.text));
        }
    }
    by_line
        .into_iter()
        .map(|(y, mut ws)| {
            ws.sort_by(|a, b| a.0.total_cmp(&b.0));
            (y, ws)
        })
        .collect()
}

/// The line holding a word at `x` within 0.05 bp whose text starts with
/// `text`.
fn line_with<'a>(lines: &'a [(i64, Vec<(f64, String)>)], text: &str, x: f64) -> &'a (i64, Vec<(f64, String)>) {
    lines
        .iter()
        .find(|(_, ws)| ws.iter().any(|(wx, t)| t.starts_with(text) && (wx - x).abs() < 0.05))
        .unwrap_or_else(|| panic!("no line has {text:?} at x={x}: {lines:#?}"))
}

/// pdflatex (11 pt article, `margin=1in`, `basicstyle=\ttfamily\small`,
/// `breaklines=true`), page 1, first/last word of each line:
///
/// ```text
/// y= 82.960  Configuration @88.936 ... --set @513.858 (width 26.147)
/// y= 96.510  watch.debounce_ms=50. @77.229 ... in @530.963
/// y=110.060  ftxc.toml. @72.000
/// ```
///
/// `\showlists` of the paragraph: `\hbox(6.10962+0.0)x20.99487` (`ftxc`),
/// `\discretionary`, `\hbox(0.0+0.0)x5.24872` holding `\glue 5.24872`,
/// `\discretionary`, ... — one box and one empty discretionary per token;
/// `\tracingparagraphs`: the line ends after `set` at `b=3 p=50 d=2669`
/// against `b=30` after the blank, and the next line `b=1`.
#[test]
fn lstinline_breaklines_breaks_between_tokens_like_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = "\\documentclass[11pt]{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[margin=1in]{geometry}\n\\usepackage{listings}\n\\usepackage{xcolor}\n\\usepackage{hyperref}\n\\lstset{\n  basicstyle=\\ttfamily\\small,\n  keywordstyle=\\bfseries,\n  numbers=left,\n  numberstyle=\\tiny,\n  frame=single,\n  breaklines=true,\n  showstringspaces=false,\n  tabsize=2\n}\n\\begin{document}\nConfiguration keys may also be overridden on the command line, for example\n\\lstinline|ftxc build --set watch.debounce_ms=50|. Command-line overrides\nalways take precedence over the values found in \\texttt{ftxc.toml}.\n\n\\end{document}\n";
    let r = render_one(doc);
    let overfull: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code == "overfull_hbox").collect();
    assert!(overfull.is_empty(), "pdflatex reports no overfull box: {overfull:?}");
    let ls = lines(&r);
    let (_, first) = line_with(&ls, "Configuration", 88.936);
    // The line ends with the argument's `set` token (listings cuts `--set`
    // into `--`, 10.49744 pt, and `set`; pdftext reads them as one word at
    // 513.858); the blank after it is not on this line.
    let last = first.last().unwrap();
    assert_eq!(last.1, "set", "line 1 must end with `--set`: {first:?}");
    let set_x = 513.858 + 10.49744 * 72.0 / 72.27;
    assert!((last.0 - set_x).abs() < 0.05, "`set` at {} (pdflatex {set_x:.3})", last.0);
    // The next line opens with the blank box (5.24872 pt = 5.229 bp in from
    // the margin) and `watch.debounce_ms=50` glued to the `.`.
    let (_, second) = line_with(&ls, "watch", 77.229);
    assert!(second[0].1.starts_with("watch"), "line 2 must open with `watch.debounce_ms=50`: {second:?}");
    // (The first `.` of the line is listings' own token between `watch` and
    // `debounce_ms`; the roman one after the argument is the last.)
    let dot = second.iter().filter(|(_, t)| t == ".").last().expect("the `.` after the argument");
    // pdftext's word `watch.debounce_ms=50.` (77.229, width 107.603)
    // includes the roman `.` (3.0261 pt): it starts at 181.81.
    let dot_x = 77.229 + 107.603 - 3.0261 * 72.0 / 72.27;
    assert!((dot.0 - dot_x).abs() < 0.05, "the `.` follows the argument directly at {} (pdflatex {dot_x:.3})", dot.0);
    let last = second.last().unwrap();
    assert_eq!(last.1, "in");
    assert!((last.0 - 530.963).abs() < 0.05, "`in` at {} (pdflatex 530.963)", last.0);
    let (_, third) = line_with(&ls, "ftxc.toml", 72.0);
    assert_eq!(third.len(), 2, "the last line is `ftxc.toml` and its `.`: {third:?}");
}

/// pdflatex (the `inline-math` fixture's preamble), page 1:
///
/// ```text
/// y= 82.960  If @72.000 ... gives @516.907 (width 23.089)
/// y= 96.510  1 @72.000  = @80.487 ... 0 @534.546
/// y=110.060  (mod @72.000 ... contradiction. @126.278
/// ```
///
/// `\tracingparagraphs`: `@ via @@0 b=31 p=0 d=1681` at the glue after
/// `gives`, `@\penalty via @@0 b=2 p=500 d=250144` after `1 =`; the second
/// line breaks at `\pmod`'s `\allowbreak` (`@\penalty via @@1 b=19 p=0`).
#[test]
fn line_breaks_before_a_formula_rather_than_after_its_relation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = "\\documentclass[11pt]{article}\n\n\\usepackage[T1]{fontenc}\n\\usepackage[utf8]{inputenc}\n\\usepackage{lmodern}\n\\usepackage[margin=1in]{geometry}\n\\usepackage{amsmath,amssymb}\n\n\\setlength{\\parindent}{0pt}\n\\setlength{\\parskip}{0.65em}\n\n\\newcommand{\\N}{\\mathbb{N}}\n\\newcommand{\\Z}{\\mathbb{Z}}\n\\newcommand{\\Q}{\\mathbb{Q}}\n\\newcommand{\\R}{\\mathbb{R}}\n\n\\begin{document}\n\\pagestyle{empty}\nIf $p$ is prime and $p \\mid ab$ then $p \\mid a$ or $p \\mid b$, because otherwise $\\gcd(p, a) = \\gcd(p, b) = 1$ gives $1 = pu_1 + av_1 = pu_2 + bv_2$ and multiplying yields $1 = p(pu_1u_2 + au_2v_1 + bu_1v_2) + ab v_1 v_2 \\equiv 0 \\pmod p$, a contradiction.\n\n\\end{document}\n";
    let r = render_one(doc);
    let overfull: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code == "overfull_hbox").collect();
    assert!(overfull.is_empty(), "pdflatex reports no overfull box: {overfull:?}");
    let ls = lines(&r);
    let (_, first) = line_with(&ls, "If", 72.0);
    let last = first.last().unwrap();
    assert_eq!(last.1, "gives", "line 1 ends with `gives`: {first:?}");
    assert!((last.0 - 516.907).abs() < 0.05, "`gives` at {} (pdflatex 516.907)", last.0);
    let (_, second) = line_with(&ls, "1", 72.0);
    // `1` and `=` are one roman run here; pdftext puts `=` at 80.487.
    assert!(second[0].1.starts_with('1'), "line 2 opens with the formula's `1`: {second:?}");
    let last = second.last().unwrap();
    assert_eq!(last.1, "0", "line 2 ends at `\\pmod`'s \\allowbreak: {second:?}");
    assert!((last.0 - 534.546).abs() < 0.05, "`0` at {} (pdflatex 534.546)", last.0);
    let (_, third) = line_with(&ls, "(mod", 72.0);
    assert!((third.last().unwrap().0 - 126.278).abs() < 0.05, "`contradiction.` at {} (pdflatex 126.278)", third.last().unwrap().0);
}

/// The gap after `\verb`/`\lstinline`: the compiler's span is the control
/// word alone, so the interword test used to read the delimited argument's
/// own blanks. pdflatex (`\showlists`, 10 pt): `\verb|a b|.` puts the `.`
/// directly after the closing `|`, and `\verb|a b| y` reads the space token
/// in the outer font (`\glue 3.33333 plus 1.66666 minus 1.11111`, not the
/// typewriter's rigid 5.25 pt).
///
/// `\lstinline|a b|` under the default `basicstyle={}` is set in the face
/// around it, `CMR10` here (`listings::apply`): pdftext reads `x` @133.768,
/// `a` @142.344, `b` @151.310 (`a` 5.00002 pt, then the blank's box, 3.33333
/// padded to 4 pt by listings' lost space: 2 × 4.50001 − 5.00002 − 3.33333),
/// `y` @160.173 (`b` 5.55557 pt, then the roman interword glue).
#[test]
fn verbatim_argument_blanks_are_not_the_gap_after_it() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = "\\documentclass{article}\\usepackage{listings}\\begin{document}\\pagestyle{empty}\\noindent x \\verb|a b|. y\n\n\\noindent x \\lstinline|a b| y\n\n\\noindent x \\verb|a b|y\\end{document}";
    let r = render_one(doc);
    let ls = lines(&r);
    assert_eq!(ls.len(), 3, "{ls:#?}");
    // `a` `b` `.`: the `.` (roman, 2.77776 pt) starts where `b` ends.
    let (_, one) = &ls[0];
    let b = one.iter().find(|(_, t)| t == "b").expect("b");
    let dot = one.iter().find(|(_, t)| t == ".").expect(".");
    assert!((dot.0 - (b.0 + 5.25 * 72.0 / 72.27)).abs() < 0.01, "`.` at {} after `b` at {}: {one:?}", dot.0, b.0);
    // `\lstinline|a b| y` and `\verb|a b|y`: the gap before `y` is the
    // roman interword space (3.33333 pt) in one, nothing in the other.
    let (_, two) = &ls[1];
    let (_, three) = &ls[2];
    let gap = |line: &[(f64, String)], b_width_pt: f64| {
        let b = line.iter().find(|(_, t)| t == "b").expect("b");
        let y = line.iter().find(|(_, t)| t == "y").expect("y");
        y.0 - b.0 - b_width_pt * 72.0 / 72.27
    };
    // The `\lstinline` is roman (`CMR10`, `b` 5.55557 pt), its blank a box
    // padded to 4 pt: `a` @142.344, `b` @151.310, `y` @160.173.
    for (text, x) in [("a", 142.344), ("b", 151.310), ("y", 160.173)] {
        let w = two.iter().find(|(_, t)| t == text).expect(text);
        assert!((w.0 - x).abs() < 0.02, "`{text}` at {} (pdflatex {x}): {two:?}", w.0);
    }
    assert!((gap(two, 5.55557) - 3.33333 * 72.0 / 72.27).abs() < 0.02, "roman space after `\\lstinline`: {} bp", gap(two, 5.55557));
    assert!(gap(three, 5.25).abs() < 0.01, "no gap after `\\verb|a b|y`: {} bp", gap(three, 5.25));
}
