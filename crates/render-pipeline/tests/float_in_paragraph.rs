//! A float environment on lines of its own *inside* a paragraph does not end
//! the paragraph. `floats::mask` used to leave a line of spaces where the
//! float was, which the compiler read as a blank line (`\par`), so
//! `First.\n<figure>\nSecond.` came out as two paragraphs.
//!
//! Every expected line is (page, baseline in bp, first word, last word) and
//! every image is its bottom edge, read with PyMuPDF from the PDF that the
//! same source produces under pdfTeX (TeX Live 2026): `article` 12pt,
//! `lmodern` T1, 1in margins, `\pagestyle{empty}`. The image is
//! `fixtures/floats/images/red-72.png` (2in x 1in). pdflatex is an oracle
//! only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 0.5;

const A: &str = "Kilo yankee yankee juliet november uniform alpha india charlie papa
alpha golf uniform whiskey zulu delta romeo hotel sierra romeo alpha
golf bravo november tango sierra sierra xray uniform echo kilo golf mike
kilo charlie kilo kilo oscar quebec romeo november delta yankee hotel.";

const B: &str = "Foxtrot papa zulu golf papa kilo romeo november lima golf charlie papa
lima delta foxtrot zulu delta kilo india sierra alpha kilo november zulu
xray november november zulu whiskey romeo whiskey tango kilo xray alpha
tango delta echo bravo sierra delta charlie foxtrot whiskey xray.";

fn fig(placement: &str) -> String {
    format!("\\begin{{figure}}[{placement}]\n\\centering\n\\includegraphics{{images/red-72.png}}\n\\caption{{A red box.}}\n\\end{{figure}}")
}

type Line = (u32, f64, String, String);

/// Page lines and image bottoms, sorted by (page, y).
fn lines(body: &str) -> Vec<Line> {
    let text = format!(
        "\\documentclass[12pt]{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{lmodern}}\n\
         \\usepackage[margin=1in]{{geometry}}\n\\usepackage{{graphicx}}\n\n\\pagestyle{{empty}}\n\
         \\begin{{document}}\n{body}\n\\end{{document}}\n"
    );
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let docs = [SourceDocument { path: "main.tex", text: &text }];
    let r = render(&docs, "main.tex", 1, "float-in-paragraph", &fonts, &options);
    let mut out: Vec<Line> = Vec::new();
    for page in &r.v2.pages {
        let mut rows: Vec<(f64, Vec<(f64, String)>)> = Vec::new();
        for it in &page.items {
            match it {
                Item::GlyphRun(run) => {
                    let Some(g) = run.glyphs.first() else { continue };
                    let (y, x) = (g.baseline_y.to_bp(), g.origin_x.to_bp());
                    match rows.iter_mut().find(|r| (r.0 - y).abs() < 3.0) {
                        Some(r) => r.1.push((x, run.text.clone())),
                        None => rows.push((y, vec![(x, run.text.clone())])),
                    }
                }
                Item::Image(i) => out.push((page.number, i.top.to_bp() + i.height.to_bp(), "[img]".into(), "[img]".into())),
                _ => {}
            }
        }
        for (y, mut runs) in rows {
            runs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let joined = runs.into_iter().map(|r| r.1).collect::<Vec<_>>().join(" ");
            let words: Vec<&str> = joined.split_whitespace().collect();
            out.push((page.number, y, words[0].to_string(), words[words.len() - 1].to_string()));
        }
    }
    out.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
    out
}

fn check(name: &str, body: &str, expected: &[(u32, f64, &str, &str)]) {
    let got = lines(body);
    let ok = got.len() == expected.len()
        && got.iter().zip(expected).all(|(g, e)| g.0 == e.0 && (g.1 - e.1).abs() <= TOL && g.2 == e.2 && g.3 == e.3);
    assert!(ok, "{name}: lines differ from pdflatex\n  got:      {got:?}\n  expected: {expected:?}");
}

#[test]
fn text_around_a_float_on_its_own_lines_is_one_paragraph() {
    if !common::lm_available() {
        return;
    }
    // `[h]`: the float goes below the line holding its anchor, and
    // `yankee hotel. Foxtrot ... lima` is one line of one paragraph.
    check(
        "figure [h]",
        &format!("{A}\n{}\n{B}", fig("h")),
        &[(1, 83.96, "Kilo", "uniform"), (1, 98.40, "whiskey", "sierra"), (1, 112.85, "xray", "delta"), (1, 127.29, "yankee", "lima"), (1, 215.57, "[img]", "[img]"), (1, 239.97, "Figure", "box."), (1, 268.37, "delta", "november"), (1, 282.81, "zulu", "charlie"), (1, 297.26, "foxtrot", "xray.")],
    );
    check(
        "figure [t]",
        &format!("{A}\n{}\n{B}", fig("t")),
        &[(1, 144.00, "[img]", "[img]"), (1, 168.41, "Figure", "box."), (1, 202.61, "Kilo", "uniform"), (1, 217.06, "whiskey", "sierra"), (1, 231.51, "xray", "delta"), (1, 245.95, "yankee", "lima"), (1, 260.40, "delta", "november"), (1, 274.84, "zulu", "charlie"), (1, 289.29, "foxtrot", "xray.")],
    );
    check(
        "table [h]",
        &format!("{A}\n\\begin{{table}}[h]\n\\centering\n\\caption{{T.}}\n\\begin{{tabular}}{{ll}}\na & b \\\\\n\\end{{tabular}}\n\\end{{table}}\n{B}"),
        &[(1, 83.96, "Kilo", "uniform"), (1, 98.40, "whiskey", "sierra"), (1, 112.85, "xray", "delta"), (1, 127.29, "yankee", "lima"), (1, 161.76, "Table", "T."), (1, 176.11, "a", "b"), (1, 206.51, "delta", "november"), (1, 220.96, "zulu", "charlie"), (1, 235.40, "foxtrot", "xray.")],
    );
}

#[test]
fn two_floats_in_a_row_inside_a_paragraph_both_anchor_in_it() {
    if !common::lm_available() {
        return;
    }
    // The second float follows `\end{figure}`, which is no paragraph end.
    check(
        "two figures",
        &format!("{A}\n{}\n{}\n{B}", fig("h"), fig("h")),
        &[(1, 83.96, "Kilo", "uniform"), (1, 98.40, "whiskey", "sierra"), (1, 112.85, "xray", "delta"), (1, 127.29, "yankee", "lima"), (1, 215.57, "[img]", "[img]"), (1, 239.97, "Figure", "box."), (1, 342.19, "[img]", "[img]"), (1, 366.60, "Figure", "box."), (1, 395.00, "delta", "november"), (1, 409.44, "zulu", "charlie"), (1, 423.89, "foxtrot", "xray.")],
    );
}

#[test]
fn par_inside_the_float_body_stays_in_the_float() {
    if !common::lm_available() {
        return;
    }
    check(
        "blank line and \\par in the body",
        &format!("{A}\n\\begin{{figure}}[h]\n\\centering\nBody line one.\n\nBody line two.\\par\n\\caption{{A caption.}}\n\\end{{figure}}\n{B}"),
        &[(1, 83.96, "Kilo", "uniform"), (1, 98.40, "whiskey", "sierra"), (1, 112.85, "xray", "delta"), (1, 127.29, "yankee", "lima"), (1, 151.80, "Body", "one."), (1, 166.25, "Body", "two."), (1, 190.65, "Figure", "caption."), (1, 219.05, "delta", "november"), (1, 233.49, "zulu", "charlie"), (1, 247.94, "foxtrot", "xray.")],
    );
}

#[test]
fn a_float_between_two_paragraphs_still_separates_them() {
    if !common::lm_available() {
        return;
    }
    check(
        "blank lines around the float",
        &format!("{A}\n\n{}\n\n{B}", fig("h")),
        &[(1, 83.96, "Kilo", "uniform"), (1, 98.40, "whiskey", "sierra"), (1, 112.85, "xray", "delta"), (1, 127.29, "yankee", "hotel."), (1, 215.57, "[img]", "[img]"), (1, 239.97, "Figure", "box."), (1, 268.37, "Foxtrot", "foxtrot"), (1, 282.81, "zulu", "whiskey"), (1, 297.26, "romeo", "foxtrot"), (1, 311.70, "whiskey", "xray.")],
    );
}

/// Every item of `body`'s render, as its `Debug` text with glyph ids
/// dropped and every `from` in a run's text read as `to`, so a same-length,
/// same-width spelling change is the only difference allowed.
fn items(preamble: &str, body: &str, from: &str, to: &str) -> Vec<String> {
    let text = format!("\\documentclass[12pt]{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{lmodern}}\n{preamble}\\pagestyle{{empty}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: &text }];
    let r = render(&docs, "main.tex", 1, "float-lookalike", &fonts, &RenderOptions::default());
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for it in &page.items {
            let mut it = it.clone();
            if let Item::GlyphRun(run) = &mut it {
                run.text = run.text.replace(from, to);
                for g in &mut run.glyphs {
                    g.gid = 0;
                }
            }
            out.push(format!("{}: {it:?}", page.number));
        }
    }
    out
}

/// A float spelled out inside `open`..`close` renders exactly as the same
/// block spelled `\begin{fIgure}`, which no float scan ever matched: the
/// literal lines, byte for byte, apart from the one glyph. `floats::mask`
/// used to blank (and, with the `%` above, comment out) the lookalike.
fn lookalike_is_literal(preamble: &str, open: &str, close: &str) {
    let lookalike = "\\begin{figure}[h]\n\\caption{x}\n\\end{figure}";
    let body = |fig: &str| format!("First.\n{open}\n{fig}\n{close}\nSecond.");
    let got = items(preamble, &body(lookalike), "fIgure", "figure");
    let want = items(preamble, &body(&lookalike.replace("figure", "fIgure")), "fIgure", "figure");
    assert_eq!(got, want, "{open}");
    let text: String = got.join("\n");
    for word in ["\\\\begin{figure}[h]", "\\\\caption{x}", "\\\\end{figure}"] {
        assert!(text.contains(word), "{open}: {word} not set literally");
    }
}

#[test]
fn a_float_inside_verbatim_is_literal_text() {
    if !common::lm_available() {
        return;
    }
    lookalike_is_literal("", "\\begin{verbatim}", "\\end{verbatim}");
}

#[test]
fn a_float_inside_lstlisting_is_literal_text() {
    if !common::lm_available() {
        return;
    }
    lookalike_is_literal("\\usepackage{listings}\n", "\\begin{lstlisting}", "\\end{lstlisting}");
}

#[test]
fn a_float_inside_verb_is_literal_text() {
    if !common::lm_available() {
        return;
    }
    let body = |name: &str| format!("First \\verb|\\begin{{{name}}}| and \\verb+\\end{{{name}}}+ second.");
    let got = items("", &body("figure"), "fIgure", "figure");
    assert_eq!(got, items("", &body("fIgure"), "fIgure", "figure"));
    assert!(got.join("\n").contains("\\\\begin{figure}"));
}

#[test]
fn a_float_in_a_comment_is_ignored() {
    if !common::lm_available() {
        return;
    }
    // The comment eats its own newline, so the paragraph is the one
    // without the comment line at all.
    check(
        "commented-out float",
        &format!("{A}\n% \\begin{{figure}}[h]\\caption{{x}}\\end{{figure}}\n{B}"),
        &lines(&format!("{A}\n{B}")).iter().map(|l| (l.0, l.1, l.2.as_str(), l.3.as_str())).collect::<Vec<_>>(),
    );
}

#[test]
fn a_real_float_after_a_verbatim_block_is_still_a_float() {
    if !common::lm_available() {
        return;
    }
    let verbatim = "\\begin{verbatim}\n\\begin{figure}[h]\n\\end{figure}\n\\end{verbatim}";
    let got = lines(&format!("{A}\n{verbatim}\n{}\n{B}", fig("h")));
    assert_eq!(got.iter().filter(|l| l.2 == "[img]").count(), 1, "{got:?}");
    assert!(got.iter().any(|l| l.2 == "Figure" && l.3 == "box."), "{got:?}");
    assert!(got.iter().any(|l| l.2 == "\\begin{figure}[h]"), "{got:?}");
}
