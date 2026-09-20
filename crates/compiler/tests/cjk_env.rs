//! CJK.sty's `CJK`/`CJK*` environment (`\usepackage{CJKutf8}`): the three
//! arguments are consumed, the body's text runs carry the family
//! (`TextStyle::cjk`), `\CJKfamily`/`\CJKspace`/`\CJKnospace` switch it,
//! and `\end{CJK}` restores the surrounding style.

use flashtex_compiler::parser::{self, Block, CjkFamily, CjkRun, Inline, TextStyle};

fn runs(source: &str) -> Vec<(String, TextStyle, bool)> {
    parser::parse(source)
        .blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines,
            _ => Vec::new(),
        })
        .filter_map(|inline| match inline {
            Inline::Text { text, style, space_before, .. } => Some((text, style, space_before)),
            _ => None,
        })
        .collect()
}

fn messages(source: &str) -> Vec<String> {
    parser::parse(source).diagnostics.into_iter().map(|d| d.message).collect()
}

const PREAMBLE: &str = "\\documentclass[11pt]{article}\\usepackage[T1]{fontenc}\\usepackage[utf8]{inputenc}\\usepackage{CJKutf8}\\begin{document}";

#[test]
fn environment_arguments_are_consumed_and_the_body_carries_the_family() {
    let source = format!("{PREAMBLE}\n\\begin{{CJK}}{{UTF8}}{{min}}\n東京は日本の首都です。\n\\end{{CJK}}\n(``Tokyo'')\n\\end{{document}}");
    let runs = runs(&source);
    let texts: Vec<&str> = runs.iter().map(|(t, _, _)| t.as_str()).collect();
    assert!(!texts.iter().any(|t| t.contains("UTF8") || t.contains("min")), "the arguments must not be typeset: {texts:?}");
    let cjk = runs.iter().find(|(t, _, _)| t.starts_with('東')).expect("the CJK text is a run");
    assert_eq!(cjk.1.cjk, Some(CjkRun { family: CjkFamily::Min, nospace: false }));
    // `space_before` is what the source shows (the line end after
    // `{min}`); the render pipeline decides from the source bytes that a
    // paragraph's first word has no glue before it, exactly as it does for
    // `\begin{center}` + newline + text, so it is not asserted here.
    let latin = runs.iter().find(|(t, _, _)| t.starts_with("(“") || t.starts_with("(``")).expect("the Latin text follows");
    assert_eq!(latin.1.cjk, None, "\\end{{CJK}} restores the style");
    assert!(latin.2, "the blank before `(` is an interword space");
    let messages = messages(&source);
    assert!(messages.iter().all(|m| !m.contains("CJK")), "no CJK diagnostic: {messages:?}");
}

#[test]
fn starred_form_family_switch_and_space_switches() {
    let source = format!(
        "{PREAMBLE}\\begin{{CJK*}}{{UTF8}}{{gbsn}}排版\\CJKfamily{{bsmi}}藝術\\CJKspace 一\\textbf{{二}}\\end{{CJK*}}\\end{{document}}"
    );
    let runs = runs(&source);
    let of = |needle: &str| runs.iter().find(|(t, _, _)| t.contains(needle)).unwrap_or_else(|| panic!("{needle} in {runs:?}")).1;
    assert_eq!(of("排版").cjk, Some(CjkRun { family: CjkFamily::Gbsn, nospace: true }));
    assert_eq!(of("藝術").cjk, Some(CjkRun { family: CjkFamily::Bsmi, nospace: true }));
    assert_eq!(of("一").cjk, Some(CjkRun { family: CjkFamily::Bsmi, nospace: false }));
    let bold = of("二");
    assert!(bold.bold, "\\textbf applies inside the environment");
    assert_eq!(bold.cjk, Some(CjkRun { family: CjkFamily::Bsmi, nospace: false }), "a font command keeps the CJK run");
}

#[test]
fn unknown_family_and_missing_package_are_diagnosed() {
    let unknown = format!("{PREAMBLE}\\begin{{CJK}}{{UTF8}}{{song}}東\\end{{CJK}}\\end{{document}}");
    let runs_unknown = runs(&unknown);
    assert_eq!(runs_unknown.iter().find(|(t, _, _)| t == "東").map(|r| r.1.cjk), Some(Some(CjkRun { family: CjkFamily::Unknown, nospace: false })));
    assert!(messages(&unknown).iter().any(|m| m.contains("CJK family 'song'")), "{:?}", messages(&unknown));

    let without = "\\documentclass{article}\\usepackage[utf8]{inputenc}\\begin{document}\\begin{CJK}{UTF8}{min}東\\end{CJK}\\end{document}";
    let msgs = messages(without);
    assert!(msgs.iter().any(|m| m.contains("needs \\usepackage{CJKutf8}")), "{msgs:?}");
    let texts: Vec<String> = runs(without).into_iter().map(|(t, _, _)| t).collect();
    assert!(texts.iter().any(|t| t.contains("UTF8")), "without the package the arguments are typeset, as pdflatex does: {texts:?}");
}

#[test]
fn family_names() {
    for (name, family) in [("min", CjkFamily::Min), ("gbsn", CjkFamily::Gbsn), ("bsmi", CjkFamily::Bsmi), ("mj", CjkFamily::Mj), ("goth", CjkFamily::Goth), ("maru", CjkFamily::Maru), ("gkai", CjkFamily::Gkai), ("bkai", CjkFamily::Bkai), ("cyberbit", CjkFamily::Unknown)] {
        assert_eq!(CjkFamily::from_name(name), family);
        if family != CjkFamily::Unknown {
            assert_eq!(family.name(), name);
        }
    }
}
