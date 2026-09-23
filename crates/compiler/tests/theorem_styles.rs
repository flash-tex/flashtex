//! Declared theorem styles: amsthm `\newtheoremstyle` and `\swapnumbers`,
//! thmtools `\declaretheoremstyle`/`\declaretheorem`, mdframed
//! `\newmdtheoremenv`. Expected heads are pdflatex's (TeX Live 2026,
//! `article` 10pt, words from the content stream): `Note 1: Note body.` in an
//! italic head with an upright number, `Knuth 1984. Cited body.` from a
//! `\thmnote{#3}` head spec, `1 Swapped.` from `\thmnumber{#2}\thmname{ #1}`,
//! `Break 1.` on a line of its own for `\newline`, `Defn 1 (Group).` from
//! thmtools with an italic note.

use flashtex_compiler::parser::{self, Block, Inline, TextStyle};

fn parse(source: &str) -> parser::Parsed {
    parser::parse(source)
}

/// The inlines of every paragraph, in order.
fn inlines(source: &str) -> Vec<Inline> {
    parse(source)
        .blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines,
            _ => Vec::new(),
        })
        .collect()
}

fn runs(source: &str) -> Vec<(String, TextStyle)> {
    inlines(source)
        .into_iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text, style)),
            _ => None,
        })
        .collect()
}

fn joined(source: &str) -> String {
    runs(source).into_iter().map(|(t, _)| t).collect::<Vec<_>>().join("|")
}

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{amsthm}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn newtheoremstyle_sets_head_font_punctuation_and_body_font() {
    let src = doc(
        "\\newtheoremstyle{note}{3pt}{3pt}{}{}{\\itshape}{:}{.5em}{}\n\\theoremstyle{note}\\newtheorem{note}{Note}",
        "\\begin{note}Note body.\\end{note}",
    );
    let runs = runs(&src);
    let text: Vec<&str> = runs.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(&text[..4], ["Note", " ", "1", ":"], "{text:?}");
    assert!(runs[0].1.italic && !runs[0].1.bold, "head font is \\itshape: {:?}", runs[0].1);
    // `\@upn`: the number is upright, after the italic correction of `e`.
    assert!(!runs[2].1.italic && runs[2].1.italic_correction.before, "{:?}", runs[2].1);
    // `#7` is set in the head font.
    assert!(runs[3].1.italic);
    // Empty `#4`: the body is `\normalfont`.
    let body = runs.iter().find(|(t, _)| t == "body.").expect("body");
    assert!(!body.1.italic && !body.1.bold, "{:?}", body.1);
    assert!(parse(&src).diagnostics.iter().all(|d| !d.message.contains("newtheoremstyle")));
}

#[test]
fn head_spec_keeps_only_the_parts_it_names() {
    let src = doc(
        "\\newtheoremstyle{citing}{3pt}{3pt}{\\itshape}{}{\\bfseries}{.}{.5em}{\\thmnote{#3}}\n\\theoremstyle{citing}\\newtheorem*{citing}{Citing}\n\
         \\newtheoremstyle{sw}{}{}{}{}{\\bfseries}{.}{ }{\\thmnumber{#2}\\thmname{ #1}}\n\\theoremstyle{sw}\\newtheorem{sw}{Swapped}",
        "\\begin{citing}[Knuth 1984]Cited body.\\end{citing}\n\n\\begin{sw}Swapped body.\\end{sw}",
    );
    let text = joined(&src);
    assert!(text.starts_with("Knuth|1984|.|Cited"), "{text}");
    assert!(text.contains("|1|Swapped|.|"), "{text}");
    assert!(!text.contains("thm"), "{text}");
}

#[test]
fn newline_separator_breaks_after_the_head_and_indent_is_a_box() {
    let src = doc(
        "\\newtheoremstyle{break}{9pt}{9pt}{\\itshape}{}{\\bfseries}{.}{\\newline}{}\n\\theoremstyle{break}\\newtheorem{brk}{Break}\n\
         \\newtheoremstyle{ind}{6pt}{}{}{2em}{\\bfseries\\sffamily}{}{1em}{}\n\\theoremstyle{ind}\\newtheorem{ind}{Indented}",
        "\\begin{brk}Break body.\\end{brk}\n\n\\begin{ind}Indented body.\\end{ind}",
    );
    let all = inlines(&src);
    let dot = all.iter().position(|i| matches!(i, Inline::Text { text, .. } if text == ".")).expect("punct");
    assert!(matches!(all[dot + 1], Inline::LineBreak { .. }), "{:?}", &all[dot..dot + 2]);
    // `\hbox to2em{}` in cmssbx10 (quad 11pt): 22pt.
    let indent = all.iter().find_map(|i| match i {
        Inline::HSpace { pt, .. } => Some(*pt),
        _ => None,
    });
    assert!(indent.is_some_and(|pt| (pt - 22.0).abs() < 0.05), "{indent:?}");
}

#[test]
fn swapnumbers_puts_the_number_first() {
    let src = doc("\\swapnumbers\n\\newtheorem{theorem}{Theorem}", "\\begin{theorem}[Main]Body.\\end{theorem}");
    let text = joined(&src);
    assert!(text.starts_with("1| |Theorem| |(|Main|)|.|"), "{text}");
}

#[test]
fn unknown_theoremstyle_falls_back_to_plain() {
    let src = doc(
        "\\theoremstyle{definition}\\newtheorem{d}{D}\n\\theoremstyle{exercise}\\newtheorem{e}{E}",
        "\\begin{e}Body text.\\end{e}",
    );
    let runs = runs(&src);
    let body = runs.iter().find(|(t, _)| t.contains("Body")).expect("body");
    assert!(body.1.italic, "amsthm uses `plain` for an unknown style: {:?}", body.1);
}

#[test]
fn thmtools_declaretheorem_names_counters_and_styles() {
    let src = doc(
        "\\usepackage{thmtools}\n\
         \\declaretheoremstyle[spaceabove=6pt, headfont=\\normalfont\\bfseries, notefont=\\mdseries\\itshape, bodyfont=\\normalfont, postheadspace=1em]{mystyle}\n\
         \\declaretheorem[style=mystyle]{defn}\n\\declaretheorem[sibling=defn]{claim}\n\\declaretheorem[numbered=no, name=Fact]{fact}",
        "\\begin{defn}[Group]A definition body.\\end{defn}\n\n\\begin{claim}Claim body.\\end{claim}\n\n\\begin{fact}Fact body.\\end{fact}",
    );
    let runs = runs(&src);
    let text: Vec<&str> = runs.iter().map(|(t, _)| t.as_str()).collect();
    let at = |t: &str| text.iter().position(|x| *x == t).unwrap_or_else(|| panic!("{t} in {text:?}"));
    assert_eq!(&text[..3], ["Defn", " ", "1"], "{text:?}");
    let note = at("Group");
    assert!(runs[note].1.italic && !runs[note].1.bold, "notefont: {:?}", runs[note].1);
    let body = runs.iter().find(|(t, _)| t.contains("definition")).unwrap();
    assert!(!body.1.italic, "bodyfont=\\normalfont");
    // `sibling=defn` shares the counter.
    assert!(text.contains(&"Claim 2"), "{text:?}");
    assert!(!text.iter().any(|t| t.contains("Fact 1")) && text[at("Fact") + 1] == ".", "{text:?}");
}

#[test]
fn newmdtheoremenv_is_a_numbered_theorem_whose_frame_is_reported() {
    let src = doc(
        "\\usepackage{mdframed}\n\\mdfdefinestyle{theorem}{linecolor=red}\n\\newmdtheoremenv[style=theorem]{theorem}{Theorem}[section]\n\\newmdtheoremenv{lemma}[theorem]{Lemma}",
        "\\section{A}\\begin{theorem}Body.\\end{theorem}\n\n\\begin{lemma}Other.\\end{lemma}",
    );
    let text = joined(&src);
    assert!(text.contains("Theorem 1.1|.|"), "{text}");
    assert!(text.contains("Lemma 1.2|.|"), "{text}");
    let msgs: Vec<String> = parse(&src).diagnostics.into_iter().map(|d| d.message).collect();
    assert_eq!(msgs.iter().filter(|m| m.contains("mdframed frames")).count(), 1, "{msgs:?}");
}

#[test]
fn a_note_only_head_spec_with_comments_between_the_arguments() {
    // amsthm's thmtest.tex `citing` style: every argument followed by a
    // `%` comment, an empty name, and all the head text in the note.
    let src = doc(
        "\\newtheoremstyle{citing}% name\n  {3pt}%      Space above\n  {3pt}%      Space below\n  {\\itshape}% Body font\n  {}%         Indent\n  {\\bfseries}% head font\n  {.}%        Punct\n  {.5em}%     Space\n  {\\thmnote{#3}}% Thm head spec\n\n\\theoremstyle{citing}\n\\newtheorem*{varthm}{}% all text",
        "\\begin{varthm}[Theorem 3.6 in \\textit{Knuth}]No hyperlinking.\\end{varthm}",
    );
    let text = joined(&src);
    assert!(text.starts_with("Theorem|3.6|in|Knuth|.|No"), "{text}");
    let msgs: Vec<String> = parse(&src).diagnostics.into_iter().map(|d| d.message).collect();
    assert!(msgs.iter().all(|m| !m.contains("thmnote")), "{msgs:?}");
}

#[test]
fn a_note_ending_in_a_group_right_before_the_body_keeps_the_group() {
    let src = doc("\\newtheorem{theorem}{Theorem}", "\\begin{theorem}[Theorem 3.6 in \\textit{Knuth}]Body.\\end{theorem}");
    let text = joined(&src);
    assert!(text.contains("Knuth"), "{text}");
}
