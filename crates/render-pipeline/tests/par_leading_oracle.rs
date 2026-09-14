//! `\baselineskip` follows the size declaration in force at `\par` — TeX's
//! §679 `append_to_vlist`, called once per line by `post_line_break` (§877).
//!
//! Every expected number is the baseline-to-baseline distance read off the
//! word origins of the reference PDF that the quoted probe produces under
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025), cross-checked against
//! `\showbox`'s `\glue(\baselineskip)` for the same paragraphs. pdflatex is
//! an oracle only and never runs in the product path.
//!
//! `size11.clo`'s `\@setfontsize` table at an 11 pt base: `\normalsize`
//! 13.6 pt (13.549 bp), `\small` 12 pt (11.955 bp), `\footnotesize` 11 pt
//! (10.958 bp), `\large` 14 pt (13.948 bp).
//!
//! Needs the compiler's `Parsed::block_par_leading`, so the whole file is
//! behind the `par-leading` feature (see `Cargo.toml`): the pinned
//! `vendor/compiler` predates that field.
#![cfg(feature = "par-leading")]

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// Distinct baselines of page 1, top first.
fn baselines(body: &str) -> Vec<f64> {
    let text = format!(
        "\\documentclass[11pt]{{article}}\n\\usepackage[margin=1in]{{geometry}}\n\
         \\pagestyle{{empty}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    );
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: &text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    let mut ys: Vec<f64> = Vec::new();
    for it in &r.v2.pages[0].items {
        if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
            if let Some(g) = run.glyphs.first() {
                ys.push((g.baseline_y.to_bp() * 1000.0).round() / 1000.0);
            }
        }
    }
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    ys
}

/// The gaps between consecutive baselines, in bp.
fn gaps(body: &str) -> Vec<f64> {
    let ys = baselines(body);
    ys.windows(2).map(|w| w[1] - w[0]).collect()
}

/// TeX points to PDF points, the unit of every v2 coordinate. The tolerance
/// below (0.02 bp, about 1/3600 inch) covers the sp rounding of the shipped
/// page; the three leadings it has to tell apart are 1.6 bp and more apart.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

const TOL: f64 = 0.02;

/// Three lines of body text: the class's own `\baselineskip`, unchanged by
/// anything in this lane.
#[test]
fn body_text_keeps_the_class_leading() {
    let g = gaps(
        "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho \
         sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon zeta eta theta iota \
         kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega alpha beta.",
    );
    assert!(g.len() >= 2, "{g:?}");
    assert!(g.iter().all(|v| (*v - bp(13.6)).abs() < TOL), "{g:?}");
}

/// The `}` restores `\baselineskip` before the blank line's `\par`, so these
/// lines are 13.549 bp apart in pdflatex even though every glyph on them is
/// `\small`. This is the assertion that a "the paragraph is small, so use
/// small's leading" fix would break, and the reason this lane does not read
/// the sizes of the runs.
#[test]
fn a_group_closing_before_the_paragraph_keeps_the_body_leading() {
    let g = gaps(
        "{\\small Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron \
         pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon zeta eta theta \
         iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega alpha.}",
    );
    assert!(g.len() >= 2, "{g:?}");
    assert!(g.iter().all(|v| (*v - bp(13.6)).abs() < TOL), "{g:?}");
}

/// `\par` inside the group: 11.955 bp in pdflatex. Before this lane the
/// pipeline set 13.549 here — the leading never followed the declaration.
#[test]
fn a_par_inside_the_group_takes_the_declaration_leading() {
    let g = gaps(
        "{\\small Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron \
         pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon zeta eta theta \
         iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega alpha.\\par}",
    );
    assert!(g.len() >= 2, "{g:?}");
    assert!(g.iter().all(|v| (*v - bp(12.0)).abs() < TOL), "{g:?}");
}

/// `\large` the same way, in the other direction: 13.948 bp, wider than the
/// body's 13.549.
#[test]
fn a_larger_declaration_opens_the_leading_up() {
    let g = gaps(
        "{\\large Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron \
         pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon zeta eta theta \
         iota kappa lambda mu nu xi omicron pi rho sigma.\\par}",
    );
    assert!(g.len() >= 2, "{g:?}");
    assert!(g.iter().all(|v| (*v - bp(14.0)).abs() < TOL), "{g:?}");
}

/// A mid-paragraph switch never touches the leading: the register is back at
/// `\normalsize` long before `\par` runs, and TeX never consults the boxes it
/// stacks. pdflatex: 13.549 bp throughout.
#[test]
fn a_mid_paragraph_switch_does_not_move_the_baselines() {
    let g = gaps(
        "Alpha beta gamma delta {\\small epsilon zeta eta theta} iota kappa {\\Large lambda mu} nu \
         xi omicron pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon zeta \
         eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi.",
    );
    assert!(g.len() >= 2, "{g:?}");
    assert!(g.iter().all(|v| (*v - bp(13.6)).abs() < TOL), "{g:?}");
}

/// `\endtrivlist` runs `\ifhmode\unskip\par\fi` before `\end`'s `\endgroup`,
/// so the declaration is still in force when the environment's last
/// paragraph ends: 11.955 bp inside the `quote`.
#[test]
fn an_environment_that_pars_before_it_closes_uses_its_own_declaration() {
    let g = gaps(
        "\\begin{quote}\\small Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu \
         nu xi omicron pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon \
         zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau.\\end{quote}",
    );
    assert!(g.len() >= 2, "{g:?}");
    assert!(g.iter().all(|v| (*v - bp(12.0)).abs() < TOL), "{g:?}");
}

/// The same rule through `\list`, at a third size.
#[test]
fn a_declaration_inside_a_list_reaches_the_item_lines() {
    let g = gaps(
        "\\begin{itemize}\\footnotesize\n\\item Alpha beta gamma delta epsilon zeta eta theta iota \
         kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega alpha beta gamma \
         delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon \
         phi chi psi omega alpha beta.\n\\end{itemize}",
    );
    assert!(g.len() >= 2, "{g:?}");
    assert!(g.iter().all(|v| (*v - bp(11.0)).abs() < TOL), "{g:?}");
}

/// The glue *above* a paragraph's first line is the same register, so the
/// paragraph after a `\small` one is back at the body's spacing while the
/// small one's own first line sits 11.955 bp below the line before it.
/// pdflatex's baselines for this probe, top first: 82.959, 96.508 (body),
/// 108.463, 120.418 (`\small`), 133.968, 147.517 (body again) — gaps
/// 13.549, **11.955**, 11.955, **13.550**, 13.549. Note the fourth: the glue
/// back up to the body's leading is contributed by the *following*
/// paragraph, whose own `\baselineskip` is what `append_to_vlist` reads
/// when its first line is appended.
#[test]
fn the_leading_above_the_first_line_follows_the_paragraph_too() {
    let g = gaps(
        "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho \
         sigma tau upsilon phi chi psi omega alpha beta gamma.\n\n\
         {\\small Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron \
         pi rho sigma tau upsilon phi chi psi.\\par}\n\n\
         Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho \
         sigma tau upsilon phi chi psi omega alpha beta gamma.",
    );
    // …, NORMAL (within the first paragraph), SMALL (into and within the
    // small one), NORMAL (into and within the last).
    assert!(g.iter().any(|v| (*v - bp(12.0)).abs() < TOL), "no \\small leading in {g:?}");
    assert!(g.iter().filter(|v| (**v - bp(12.0)).abs() < TOL).count() >= 2, "{g:?}");
    assert!(g.iter().any(|v| (*v - bp(13.6)).abs() < TOL), "no body leading in {g:?}");
}

/// A narrower leading makes §679's *other* branch reachable, and the point of
/// the rule is that the engine has to pick the branch, not scale a number: a
/// `\Huge` box (24.88 pt, cap height ~17 pt) on one line of a `\footnotesize`
/// paragraph (11 pt leading) drives `\baselineskip - \prevdepth - height`
/// below `\lineskiplimit`, so TeX puts `\lineskip` (1 pt) there instead and
/// that one gap opens to 19.674 bp while its neighbours stay at 10.959.
///
/// pdflatex's baselines for this probe: 82.959, 102.633, 113.592, 124.551.
/// Ours: 82.959, 102.634, 113.592, 124.551.
#[test]
fn a_tall_box_still_falls_back_to_lineskip_at_the_new_leading() {
    let g = gaps(
        "{\\footnotesize Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi \
         omicron pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon zeta eta \
         theta iota kappa lambda mu nu xi omicron {\\Huge TALL} pi rho sigma tau upsilon phi chi psi \
         omega alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi \
         rho sigma.\\par}",
    );
    assert_eq!(g.len(), 3, "{g:?}");
    assert!((g[0] - 19.674).abs() < TOL, "lineskip branch: {g:?}");
    assert!((g[1] - bp(11.0)).abs() < TOL, "{g:?}");
    assert!((g[2] - bp(11.0)).abs() < TOL, "{g:?}");
}
