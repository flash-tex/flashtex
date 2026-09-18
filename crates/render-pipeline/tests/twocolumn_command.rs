//! `\twocolumn` and `\onecolumn`: two-column mode as document state.
//!
//! Every number is read off the glyph origins of the PDF
//! `/Library/TeX/texbin/pdflatex` (pdfTeX 3.141592653-2.6-1.40.29, TeX Live
//! 2026) produces for the probe quoted in the test, measured with PyMuPDF.
//! pdflatex is an oracle only and never runs in the product path.
//!
//! The engine honoured the `twocolumn` *class option* and had no model of
//! the *commands* at all (GH#743): a document that says `\twocolumn` was
//! set in one column from beginning to end, which put `\end{abstract}`'s
//! boundary 1.992/2.988/2.988 bp long at 10/11/12 pt and every `lstlisting`
//! boundary 5.977/8.966/11.955 bp out. The class option could not be
//! patched into covering it — what decides `\if@twocolumn` is a command the
//! document runs, so the answer has to be state with a position
//! (`flashtex_render_pipeline::columns`).

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[derive(Debug, Clone)]
struct Word {
    text: String,
    x: f64,
    baseline: f64,
    page: usize,
}

fn layout(text: &str) -> (Vec<String>, Vec<Word>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    let mut words = Vec::new();
    for (p, page) in r.v2.pages.iter().enumerate() {
        for it in page.resident_items() {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                words.push(Word {
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    page: p + 1,
                });
            }
        }
    }
    (v1.diagnostics.iter().map(|d| d.code.clone()).collect(), words)
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.text.trim() == text)
        .unwrap_or_else(|| panic!("no word {text:?} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

fn close(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= 0.5, "{what}: {got} vs pdflatex {want} ({:+})", got - want);
}

/// The issue's own repro: `\twocolumn`, an `abstract`, and a `\trivlist`
/// beside it.
const REPRO: &str = "\\documentclass[SIZE]{article}\n\\twocolumn\n\\begin{document}\n\
\\begin{abstract}\nAaa\n\\end{abstract}\n\\begin{itemize}\n\\item Bbb\n\\end{itemize}\nCcc\n\
\\end{document}\n";

/// `(size, head baseline, body baseline, item baseline, following baseline,
/// text left edge)`, all pdflatex glyph origins in bp.
const REPRO_ORACLE: [(&str, f64, f64, f64, f64, f64); 3] = [
    ("10pt", 134.765, 156.586, 176.511, 196.436, 133.768),
    ("11pt", 140.742, 165.094, 187.610, 210.126, 125.798),
    ("12pt", 137.753, 164.038, 188.447, 212.855, 110.854),
];

#[test]
fn the_twocolumn_command_sets_the_abstract_and_its_boundary() {
    for (size, head, body, item, after, left) in REPRO_ORACLE {
        let (codes, words) = layout(&REPRO.replace("SIZE", size));
        assert!(
            !codes.iter().any(|c| c == "unknown_command"),
            "{size}: \\twocolumn is a command this engine knows: {codes:?}"
        );
        // The two-column branch of `abstract` is a `\section*` at the
        // column's left edge, not a centred `\small` head over a
        // `quotation`. The class option already got this right; the
        // command did not reach it.
        close(word(&words, "Abstract").baseline, head, &format!("{size} head baseline"));
        close(word(&words, "Abstract").x, left, &format!("{size} head left edge"));
        close(word(&words, "Aaa").baseline, body, &format!("{size} body baseline"));
        close(word(&words, "Aaa").x, left, &format!("{size} body left edge"));
        // `\end{abstract}` in two columns expands to nothing at all
        // (article.cls 386, `\if@twocolumn\else\endquotation\fi`), so the
        // `\begin{itemize}` beside it is not read in vertical mode and
        // takes no `\partopsep` — the point the issue makes about
        // `abstractenv::end_is_endtrivlist` keying on class options.
        close(word(&words, "Bbb").baseline, item, &format!("{size} item baseline"));
        close(word(&words, "Ccc").baseline, after, &format!("{size} baseline after the list"));
    }
}

#[test]
fn the_command_keeps_the_one_column_textwidth_and_parindent() {
    // The class *option* is read by `size1<n>.clo` while the class is
    // loading, so it doubles `\textwidth` and sets `\parindent` to 1em; the
    // command runs afterwards and changes only `\columnwidth`. pdflatex:
    // the command's text block still starts at 133.768 bp with a 15pt
    // indent, the option's at 72.0 bp with a 1em (9.963 bp) indent, and the
    // command's second column is at 310.605 bp.
    let body = "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor \
                incididunt ut labore et dolore magna aliqua Ut enim ad minim veniam quis nostrud \
                exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat ";
    let doc = |options: &str, preamble: &str| {
        format!("\\documentclass[{options}]{{article}}\n{preamble}\\begin{{document}}\nWORD {}\n\\end{{document}}\n", body.repeat(14))
    };

    let (_, cmd) = layout(&doc("10pt", "\\twocolumn\n"));
    let first = word(&cmd, "WORD");
    close(first.x, 133.768 + 14.944, "command: first line indent");
    let second = cmd
        .iter()
        .find(|w| w.page == 1 && w.x > 300.0 && w.baseline < 200.0)
        .expect("a second column on page 1");
    close(second.x, 310.605, "command: second column left edge");

    let (_, opt) = layout(&doc("10pt,twocolumn", ""));
    close(word(&opt, "WORD").x, 81.963, "option: first line indent");
}

#[test]
fn both_commands_start_a_new_page_wherever_they_stand() {
    // Both open with `\clearpage`, so both end the page — in a two-column
    // document too, where `\newpage` only ends the column. Measured: each
    // of the four combinations puts SECONDPARA on page 2, and the
    // `\newpage` control leaves it on page 1 of the `[twocolumn]` document.
    let doc = |options: &str, command: &str| {
        format!("\\documentclass[{options}]{{article}}\n\\begin{{document}}\nFIRSTPARA\n\\{command}\nSECONDPARA\n\\end{{document}}\n")
    };
    for (options, command) in [
        ("10pt", "twocolumn"),
        ("10pt", "onecolumn"),
        ("10pt,twocolumn", "twocolumn"),
        ("10pt,twocolumn", "onecolumn"),
    ] {
        let (_, words) = layout(&doc(options, command));
        assert_eq!(word(&words, "FIRSTPARA").page, 1, "[{options}] \\{command}");
        assert_eq!(word(&words, "SECONDPARA").page, 2, "[{options}] \\{command}");
    }
    let (_, words) = layout(&doc("10pt,twocolumn", "newpage"));
    assert_eq!(word(&words, "SECONDPARA").page, 1, "\\newpage ends the column, not the page");
}

#[test]
fn a_switch_after_material_is_reported_and_one_before_it_is_not() {
    let leading = "\\documentclass[10pt]{article}\n\\begin{document}\n\\twocolumn\nAaa\n\\end{document}\n";
    let (codes, _) = layout(leading);
    assert!(!codes.iter().any(|c| c == "twocolumn_mid_document"), "{codes:?}");

    // The page frame is still one frame for the whole document, so a
    // switch that really changes the column count after material is named,
    // not silently approximated.
    let mid = "\\documentclass[10pt]{article}\n\\begin{document}\nAaa\n\\twocolumn\nBbb\n\\end{document}\n";
    let (codes, _) = layout(mid);
    assert!(codes.iter().any(|c| c == "twocolumn_mid_document"), "{codes:?}");

    // A `\twocolumn` that changes nothing still breaks the page, and says
    // nothing about columns.
    let same = "\\documentclass[10pt,twocolumn]{article}\n\\begin{document}\nAaa\n\\twocolumn\nBbb\n\\end{document}\n";
    let (codes, _) = layout(same);
    assert!(!codes.iter().any(|c| c == "twocolumn_mid_document"), "{codes:?}");
}

#[test]
fn the_optional_argument_is_declared_not_implemented() {
    let (codes, _) = layout(
        "\\documentclass[10pt]{article}\n\\twocolumn[\\section*{Head}]\n\\begin{document}\nAaa\n\\end{document}\n",
    );
    assert!(codes.iter().any(|c| c == "twocolumn_top_material"), "{codes:?}");
}

#[test]
fn the_abstract_to_lstlisting_boundary_follows_the_command_too() {
    // The issue's other measurement. `\end{abstract}\begin{lstlisting}`
    // was 5.977/8.966/11.955 bp long at 10/11/12 pt, and the cause is the
    // same one: read as one-column, the `abstract` is a `\small`
    // `quotation` whose `\endlist` adds `\small`'s `\topsep +
    // \partopsep` (6/9/12 pt) that the two-column branch never has.
    // #738 measured the `lstlisting` skips themselves; nothing about them
    // is column-dependent, so getting the mode right is the whole fix.
    const DOC: &str = "\\documentclass[SIZE]{article}\n\\usepackage{listings}\n\\twocolumn\n\
\\begin{document}\n\\begin{abstract}\nAaa\n\\end{abstract}\n\\begin{lstlisting}\ncode\n\
\\end{lstlisting}\nAfter text.\n\\end{document}\n";
    // `(size, Abstract head, Aaa, code, After)` pdflatex baselines in bp.
    // The boundary itself: code - Aaa = 17.932 / 19.527 / 20.424, where the
    // engine gave 23.910 / 28.493 / 32.379.
    for (size, head, body, code, after) in [
        ("10pt", 134.765, 156.586, 174.518, 192.451),
        ("11pt", 140.742, 165.094, 184.621, 204.148),
        ("12pt", 137.753, 164.038, 184.462, 204.885),
    ] {
        let (_, words) = layout(&DOC.replace("SIZE", size));
        close(word(&words, "Abstract").baseline, head, &format!("{size} head"));
        close(word(&words, "Aaa").baseline, body, &format!("{size} abstract body"));
        close(word(&words, "code").baseline, code, &format!("{size} listing after the abstract"));
        close(word(&words, "After").baseline, after, &format!("{size} after the listing"));
    }
}

#[test]
fn the_lstlisting_boundaries_follow_the_command_too() {
    // The other half of GH#743: `lstlisting`'s two boundary skips were
    // 5.977/8.966/11.955 bp out in a `\twocolumn` document, for the same
    // reason — the whole document was set one-column. #738 measured those
    // skips; nothing about them is size- or column-dependent, so getting
    // the mode right is the entire fix.
    const DOC: &str = "\\documentclass[SIZE]{article}\n\\usepackage{listings}\n\\twocolumn\n\
\\begin{document}\nIntro text here.\n\\begin{lstlisting}\ncode line\n\\end{lstlisting}\n\
\\begin{itemize}\n\\item Bbb\n\\end{itemize}\nAfter text.\n\\end{document}\n";
    // `(size, Intro, code, Bbb, After)` pdflatex glyph baselines in bp.
    for (size, intro, code, item, after) in [
        ("10pt", 134.765, 152.697, 180.593, 202.511),
        ("11pt", 140.742, 160.269, 191.751, 217.255),
        ("12pt", 137.753, 158.177, 191.552, 218.949),
    ] {
        let (_, words) = layout(&DOC.replace("SIZE", size));
        close(word(&words, "Intro").baseline, intro, &format!("{size} before the listing"));
        close(word(&words, "code").baseline, code, &format!("{size} listing body"));
        close(word(&words, "Bbb").baseline, item, &format!("{size} list after the listing"));
        close(word(&words, "After").baseline, after, &format!("{size} after the list"));
    }
}
