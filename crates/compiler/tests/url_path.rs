//! url.sty's text-mode `\path{...}`: verbatim typewriter text, never a link.
//!
//! Measured against pdflatex (TeX Live 2026, url.sty): `\path{/usr/bin}`
//! sets in `cmtt`, exactly like `\url{/usr/bin}` minus the hyperlink.
//! This layout sets type in Core-14 Courier (not cmtt) on its own page
//! frame, so absolute page positions cannot match pdflatex: these tests
//! pin the typewriter font, the `\nolinkurl` equivalence (same code path,
//! same breaks) and the package/tikzpicture gating instead.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::{Font, LayoutConstraints};
use flashtex_compiler::parser::parse;

const SOURCE: &str = "\\documentclass[10pt]{article}\\usepackage{url}\n\\begin{document}\nA \\url{http://example.com/a_b~c} B \\path{/usr/bin} C next.\n\\end{document}";

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn word_items(out: &CompileOutput) -> Vec<(String, f64, f64, Font)> {
    out.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter(|i| i.rule.is_none() && !i.text.trim().is_empty())
        .map(|i| (i.text.clone(), i.x_pt, i.baseline_y_pt, i.font))
        .collect()
}

#[test]
fn path_sets_verbatim_typewriter_with_no_path_diagnostic() {
    let parsed = parse(SOURCE);
    // The only diagnostic left is `\url`'s own pre-existing "not
    // clickable" notice (one per document, covered by the parser's url
    // tests): nothing names `\path` or the `url` package.
    assert_eq!(
        parsed.diagnostics.len(),
        1,
        "{:?}",
        parsed.diagnostics
    );
    assert!(
        parsed.diagnostics[0].message.contains("not clickable"),
        "{:?}",
        parsed.diagnostics
    );
    let out = compile(SOURCE);
    let words = word_items(&out);
    let after_b: Vec<_> = words
        .iter()
        .skip_while(|(text, _, _, _)| text != "B")
        .skip(1)
        .take_while(|(text, _, _, _)| text != "C")
        .collect();
    assert_eq!(after_b.len(), 3, "{words:?}");
    let path_runs = after_b;
    assert!(
        path_runs.iter().all(|(_, _, _, font)| *font == Font::Courier),
        "typewriter like \\url: {words:?}"
    );
}

#[test]
fn path_matches_nolinkurl_piece_for_piece() {
    for payload in ["/usr/bin", "a_b~c", "http://example.com/a_b~c", "a&b#c"] {
        let path = compile(&format!("\\usepackage{{url}}\nX \\path{{{payload}}} Y\n"));
        let nolink = compile(&format!("\\usepackage{{url}}\nX \\nolinkurl{{{payload}}} Y\n"));
        assert!(path.diagnostics.is_empty(), "{:?}", path.diagnostics);
        assert!(nolink.diagnostics.is_empty(), "{:?}", nolink.diagnostics);
        let (left, right) = (word_items(&path), word_items(&nolink));
        assert_eq!(left.len(), right.len(), "{payload}: {left:?} vs {right:?}");
        for (a, b) in left.iter().zip(right.iter()) {
            assert_eq!((&a.0, a.3), (&b.0, b.3), "{payload}");
            assert!(
                (a.1 - b.1).abs() < 0.001 && (a.2 - b.2).abs() < 0.001,
                "{payload}: {a:?} vs {b:?}"
            );
        }
    }
}

#[test]
fn path_percent_warns_only_when_trailing_text_is_lost() {
    // `%` still comments in the token stream (the expansion pass blanks
    // only `\url`/`\nolinkurl`/`\href` arguments): same-line text after
    // the brace is dropped, loudly.
    let parsed = parse("\\usepackage{url}\nA \\path{a%b} Y\n");
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0].message.contains('%'),
        "{:?}",
        parsed.diagnostics
    );
    let out = compile("\\usepackage{url}\nA \\path{a%b} Y\n");
    let words = word_items(&out);
    assert!(
        words
            .iter()
            .any(|(text, _, _, font)| text == "a%b" && *font == Font::Courier),
        "the argument itself stays verbatim: {words:?}"
    );
    assert!(
        !words.iter().any(|(text, _, _, _)| text == "Y"),
        "dropped trailing text is reported, not silent: {words:?}"
    );
    // A brace last on its line loses nothing and stays silent.
    let parsed = parse("\\usepackage{url}\nA \\path{a%b}\nY\n");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let out = compile("\\usepackage{url}\nA \\path{a%b}\nY\n");
    let words = word_items(&out);
    assert!(
        words
            .iter()
            .any(|(text, _, _, _)| text == "Y"),
        "{words:?}"
    );
}

#[test]
fn usepackage_url_loads_silently() {
    let parsed = parse("\\usepackage{url}\nA B\n");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn path_with_hyperref_only_also_typesets() {
    let parsed = parse("\\usepackage{hyperref}\nA \\path{/x} B\n");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let out = compile("\\usepackage{hyperref}\nA \\path{/x} B\n");
    let words = word_items(&out);
    let runs: Vec<_> = words
        .iter()
        .filter(|(_, _, _, font)| *font == Font::Courier)
        .map(|(text, _, _, _)| text.clone())
        .collect();
    // url.sty breaks after `/`, so `/x` reaches the page as two runs.
    assert_eq!(runs, ["/", "x"], "{words:?}");
}

#[test]
fn path_without_url_keeps_the_tikz_diagnostic() {
    let parsed = parse("A \\path{/x} B\n");
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    let diag = &parsed.diagnostics[0];
    assert_eq!(
        diag.message,
        "\\path is not supported by this compiler version"
    );
    assert!(
        diag.help
            .as_ref()
            .is_some_and(|h| h.message.contains("tikz")),
        "{diag:?}"
    );
    // The argument still reaches the page in the ordinary roman font, as
    // every unsupported command's braced argument does.
    let out = compile("A \\path{/x} B\n");
    let words = word_items(&out);
    assert!(
        words
            .iter()
            .any(|(text, _, _, font)| text == "/x" && *font == Font::TimesRoman),
        "{words:?}"
    );
}

#[test]
fn path_inside_tikzpicture_stays_a_tikz_command() {
    let source = "\\usepackage{url}\n\\begin{tikzpicture}\nA \\path{/x} B\n\\end{tikzpicture}\n";
    let parsed = parse(source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message == "\\path is not supported by this compiler version"
                && d.help.as_ref().is_some_and(|h| h.message.contains("tikz"))),
        "tikzpicture-internal \\path keeps its tikz diagnostic: {:?}",
        parsed.diagnostics
    );
}
