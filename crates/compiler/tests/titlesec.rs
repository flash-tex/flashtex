//! titlesec `\titleformat{\section}` + `\titlerule`, and the pdfTeX
//! glyph-to-Unicode no-ops (Jake's Resume, the 163-document corpus survey's
//! "nearest miss": one `article`-class document blocked by titlesec,
//! `\input{glyphtounicode}` and `tabularx`, the last owned by a separate PR).
//!
//! Oracle for every measurement here: live pdflatex, TeX Live 2026,
//! `/Library/TeX/texbin/pdflatex`. The exact oracle preamble is `RESUME_PREAMBLE`
//! below (the resume's own lines). Through pdflatex the two-section oracle
//! document's content stream reads, in order:
//!
//! ```text
//! /F43 11.9552 Tf 125.798 651.258 Td [(Educa)67(tion)]TJ   % CMCSC10: small caps, medium, \large at 11pt
//! 0.398 w 0 0 m 358.655 0 l S                             % 0.4pt rule, full text width, its own line
//! /F41 10.9091 Tf 125.798 633.277 Td [(Bo)...]TJ           % CMR10 body
//! ```
//!
//! The pinned facts: the heading is unnumbered (empty titlesec label prints
//! no number), medium-weight small caps at 11.9552pt (`\large`, NOT the
//! default `\Large\bfseries` -- titlesec replaces the format entirely), the
//! rule starts at the left margin on its own line 4.38pt below the title
//! baseline (not leaders after the title text), `\vspace{-5pt}` pulls the
//! body up the full 4.98pt (a no-`\vspace` variant sets the body at y
//! 628.296 instead of 633.277), and `\vspace{-4pt}` in the format moves
//! nothing (the title sits at y 651.258 with and without it: `\@startsection`
//! `\addvspace` keeps the larger beforeskip).
//!
//! The glyphtounicode oracle is `glyph-on.tex` vs `glyph-off.tex` (identical
//! but for `\input{glyphtounicode}\pdfgentounicode=1`): same fonts
//! (`pdffonts`: CMBX12 + CMR10 both), same word boxes (`pdftotext -bbox`
//! identical to 6 decimals) and same extracted text. The 2,700-line system
//! file is pure `\pdfglyphtounicode` metadata (which glyph maps to which
//! Unicode codepoint for copy-paste/ATS) with zero visible effect, hence the
//! silent no-ops below.

use flashtex_compiler::parser::{parse, Block, FillLeader, Inline};

/// The resume's own titlesec lines, verbatim.
const RESUME_PREAMBLE: &str = "\\usepackage{titlesec}\n\
     \\usepackage[usenames,dvipsnames]{color}\n\
     \\titleformat{\\section}{\n  \\vspace{-4pt}\\scshape\\raggedright\\large\n}{}{0em}{}[\\color{black}\\titlerule \\vspace{-5pt}]\n";

fn resume_doc(body: &str) -> String {
    format!(
        "\\documentclass[letterpaper,11pt]{{article}}\n{RESUME_PREAMBLE}\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn heading_text(content: &[Inline]) -> (String, Vec<String>) {
    let mut words = Vec::new();
    let mut styles = Vec::new();
    for inline in content {
        if let Inline::Text { text, style, .. } = inline {
            words.push(text.clone());
            styles.push(format!(
                "bold={} sc={} size={:?}",
                style.bold, style.small_caps, style.size
            ));
        }
    }
    (words.concat(), styles)
}

#[test]
fn titleformat_section_applies_format_and_rule() {
    let parsed = parse(&resume_doc("\\section{Education}\nBody text here.\n"));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    assert!(parsed.blocks.len() >= 3, "{:?}", parsed.blocks);
    // The heading: level 1, unnumbered (the oracle prints no number), with
    // the format's face and size (medium small caps \large, not bold).
    let (level, number, content) = match &parsed.blocks[0] {
        Block::Heading {
            level,
            number,
            content,
            ..
        } => (*level, number.clone(), content.clone()),
        other => panic!("first block is not a heading: {other:?}"),
    };
    assert_eq!(level, 1);
    assert_eq!(number, "");
    let (text, styles) = heading_text(&content);
    assert_eq!(text, "Education");
    assert_eq!(styles.len(), 1, "{styles:?}");
    assert_eq!(styles[0], "bold=false sc=true size=Some(Large1)", "{styles:?}");
    // The after-code rule on its own line, then the -5pt pull-up.
    assert!(
        matches!(parsed.blocks[1], Block::Rule { .. }),
        "{:?}",
        parsed.blocks[1]
    );
    match parsed.blocks[2] {
        Block::VSpace { pt, .. } => assert!(
            (pt - -5.0).abs() < 1e-9,
            "after-code vspace should be exactly -5pt, got {pt}"
        ),
        ref other => panic!("third block is not the after-code vspace: {other:?}"),
    }
}

#[test]
fn titlerule_modes_match_tex_ifvmode_branches() {
    // In a paragraph `\titlerule` is `\leaders\hrule\hfill` (titlesec.sty
    // `\ttl@rule`): a rule filling the rest of the line -- the same node as
    // `\hrulefill`. Between paragraphs it is `\titleline`: a full-width rule.
    let parsed = parse(&resume_doc("Some text \\titlerule\\ more text.\n\n\\titlerule\n\nAfter.\n"));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let mut saw_fill = false;
    let mut saw_rule_block = false;
    for block in &parsed.blocks {
        match block {
            Block::Paragraph(inlines) => {
                saw_fill |= inlines.iter().any(|i| {
                    matches!(
                        i,
                        Inline::HFill {
                            leader: FillLeader::Rule,
                            ..
                        }
                    )
                });
            }
            Block::Rule { .. } => saw_rule_block = true,
            _ => {}
        }
    }
    assert!(saw_fill, "no in-paragraph rule fill: {:?}", parsed.blocks);
    assert!(
        saw_rule_block,
        "no between-paragraph rule block: {:?}",
        parsed.blocks
    );
}

#[test]
fn titleformat_starred_keeps_the_number_and_applies_the_format() {
    // The two-argument easy form (titlesec.sty `\ttl@format@s`): only the
    // format changes, so the section stays numbered -- and only two groups
    // are consumed, so the body that follows is not eaten.
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{titlesec}\n\\titleformat*{\\section}{\\Large\\bfseries}\n\\begin{document}\n\\section{Hi}\nBody.\n\\end{document}\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    match &parsed.blocks[0] {
        Block::Heading {
            number, content, ..
        } => {
            assert_eq!(number, "1");
            let (text, styles) = heading_text(content);
            assert_eq!(text, "Hi");
            assert_eq!(styles.len(), 1, "{styles:?}");
            assert_eq!(styles[0], "bold=true sc=false size=Some(Large2)", "{styles:?}");
        }
        other => panic!("first block is not a heading: {other:?}"),
    }
    assert_eq!(parsed.blocks.len(), 2, "{:?}", parsed.blocks);
}

#[test]
fn titleformat_subsection_applies_format_like_section() {
    // The per-level generalization of `titleformat_section_applies_format_and_rule`:
    // the same resume-shaped recording under `\subsection` applies to actual
    // `\subsection` headings (level 2, unnumbered with an empty label) with
    // the same rule + after-space, and reports no diagnostics.
    let preamble = RESUME_PREAMBLE.replace("\\section", "\\subsection");
    let doc = format!(
        "\\documentclass[letterpaper,11pt]{{article}}\n{preamble}\\begin{{document}}\n\\subsection{{Background}}\nBody text here.\n\\end{{document}}\n"
    );
    let parsed = parse(&doc);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    assert!(parsed.blocks.len() >= 3, "{:?}", parsed.blocks);
    let (level, number, content) = match &parsed.blocks[0] {
        Block::Heading {
            level,
            number,
            content,
            ..
        } => (*level, number.clone(), content.clone()),
        other => panic!("first block is not a heading: {other:?}"),
    };
    assert_eq!(level, 2);
    assert_eq!(number, "");
    let (text, styles) = heading_text(&content);
    assert_eq!(text, "Background");
    assert_eq!(styles.len(), 1, "{styles:?}");
    assert_eq!(styles[0], "bold=false sc=true size=Some(Large1)", "{styles:?}");
    assert!(
        matches!(parsed.blocks[1], Block::Rule { .. }),
        "{:?}",
        parsed.blocks[1]
    );
    match parsed.blocks[2] {
        Block::VSpace { pt, .. } => assert!(
            (pt - -5.0).abs() < 1e-9,
            "after-code vspace should be exactly -5pt, got {pt}"
        ),
        ref other => panic!("third block is not the after-code vspace: {other:?}"),
    }
}

#[test]
fn titleformat_numbered_subsection_keeps_its_number() {
    // The numbered (unstarred, non-empty-label) shape: the label is the
    // number placeholder (here `\thesubsection`), so the heading still
    // renders its stepped counter ("1.1"), not just the title. The label's
    // custom placement itself stays out of scope (one diagnostic), but the
    // number must not be dropped.
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{titlesec}\n\\titleformat{\\subsection}{\\bfseries}{\\thesubsection}{1em}{}\n\\begin{document}\n\\section{Intro}\n\\subsection{Background}\nBody text here.\n\\end{document}\n",
    );
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0]
            .message
            .contains("\\titleformat with a non-empty label"),
        "{:?}",
        parsed.diagnostics
    );
    let mut seen = Vec::new();
    for block in &parsed.blocks {
        if let Block::Heading {
            level,
            number,
            content,
            ..
        } = block
        {
            let (text, _) = heading_text(content);
            seen.push((*level, number.clone(), text));
        }
    }
    assert_eq!(
        seen,
        vec![
            (1, "1".to_string(), "Intro".to_string()),
            (2, "1.1".to_string(), "Background".to_string()),
        ],
        "{:?}",
        parsed.blocks
    );
}

#[test]
fn titleformat_paragraph_is_recognised_but_out_of_scope() {
    // Run-in levels (`\paragraph`) have no heading block of their own, so a
    // recording for them stays diagnosed exactly as before.
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{titlesec}\n\\titleformat{\\paragraph}{\\bfseries}{}{0em}{}\n\\begin{document}\nHi.\n\\end{document}\n",
    );
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0]
            .message
            .contains("\\titleformat for \\paragraph is recognised but not implemented"),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn titleformat_without_titlesec_names_the_package() {
    // Package commands stay user-definable (the soul precedent): without the
    // package the use names what is missing and consumes its arguments, so
    // nothing leaks onto the page as prose.
    let parsed = parse(
        "\\documentclass{article}\n\\titleformat{\\section}{\\bfseries}{}{0em}{}\n\\begin{document}\nHi.\n\\end{document}\n",
    );
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0]
            .message
            .contains("\\titleformat needs \\usepackage{titlesec}"),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn input_glyphtounicode_is_a_silent_noop() {
    // Preamble placement, exactly as in the resume.
    for target in ["glyphtounicode", "glyphtounicode.tex"] {
        let parsed = parse(&format!(
            "\\documentclass[letterpaper,11pt]{{article}}\n\\input{{{target}}}\n\\pdfgentounicode=1\n\\begin{{document}}\nHello.\n\\end{{document}}\n"
        ));
        assert!(
            parsed.diagnostics.is_empty(),
            "{target}: {:?}",
            parsed.diagnostics
        );
    }
    // The exemption is by exact target name: a genuinely missing file still
    // errors rather than vanishing silently.
    let parsed = parse(
        "\\documentclass{article}\n\\input{no-such-file-xyz}\n\\begin{document}\nHi.\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("included file not found")),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn pdftex_unicode_primitives_are_silent_noops() {
    // `\pdfglyphtounicode{<name>}{<hex>}` (the system file's only content)
    // and the `\pdfgentounicode=1` assignment: PDF text-extraction metadata
    // with zero visible effect (see the module docs), so both are consumed
    // silently -- including the `=1`, which must not leak as body text.
    let parsed = parse(
        "\\documentclass{article}\n\\pdfgentounicode=1\n\\begin{document}\n\\pdfglyphtounicode{A}{0041}Hello.\n\\end{document}\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let mut text = String::new();
    for block in &parsed.blocks {
        let inlines = match block {
            Block::Paragraph(inlines) => inlines,
            _ => continue,
        };
        for inline in inlines {
            if let Inline::Text { text: word, .. } = inline {
                text.push_str(word);
            }
        }
    }
    assert_eq!(text, "Hello.");
}
