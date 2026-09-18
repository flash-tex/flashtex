//! fancyhdr core (issue #833): `\pagestyle{fancy}`, `\fancyhf`, `\fancyhead`,
//! `\fancyfoot`, and the head/foot rules.
//!
//! Oracle for every measurement here: live pdflatex, TeX Live 2026,
//! `/Library/TeX/texbin/pdflatex` -- NOT the TeX Live 2024 `.log` files
//! shipped in `$HOME/corpus/ross`, which are a version behind. The package
//! really loads: the local transcript says
//! `Package: fancyhdr 2025/02/07 v5.2 Extensive control of page headers and
//! footers`. The exact oracle preamble is `REPRO`, one command per line.
//! Through pdflatex that source is one page (`pdfinfo`: 1 page) whose raw
//! `pdftotext` order is
//!
//! ```text
//! Left Field
//!
//! Body text on page one.
//!
//! Right Field
//! ```
//!
//! i.e. the header streams first, then the body, then the footer, and no
//! page number anywhere (`\fancyhf{}` cleared it). Note the quirk: the raw
//! `pdftotext` above prints the right-aligned `Right Field` after the body.
//! That is `pdftotext`'s block sorting (far-right text becomes its own
//! block), not the content stream -- the PDF draws the whole header line,
//! then the rule, then the body. The pinned facts below are content-stream
//! order (header lines ahead of the body, footer lines after) plus geometry
//! (header above the body, footer below), which is what `pdftotext` reads.
//! With `\fancyhf{}` alone the oracle draws the 0.4pt head rule but no
//! header or footer text at all.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::{LayoutConstraints, TextItem};
use flashtex_compiler::parser::parse;

/// The exact 10-line oracle preamble, one command per line.
const REPRO: &str = "\\documentclass{article}\n\
     \\usepackage{fancyhdr}\n\
     \\pagestyle{fancy}\n\
     \\fancyhf{}\n\
     \\fancyhead[L]{Left Field}\n\
     \\fancyhead[R]{Right Field}\n\
     \\begin{document}\n\
     Body text on page one.\n\
     \\end{document}\n";

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

/// Every word on the first page, in content-stream order (what `pdftotext`
/// reads: header lines were inserted ahead of the body, footer lines after).
fn page_words(out: &CompileOutput) -> Vec<String> {
    out.pages
        .first()
        .map(|page| {
            page.items
                .iter()
                .filter(|item| !item.text.is_empty())
                .map(|item| item.text.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn items(out: &CompileOutput) -> Vec<&TextItem> {
    out.pages
        .first()
        .map(|page| page.items.iter().collect())
        .unwrap_or_default()
}

#[test]
fn repro_renders_header_body_footer_in_pdftotext_order_with_no_page_number() {
    let out = compile(REPRO);
    assert!(
        out.diagnostics.is_empty(),
        "oracle preamble must be silent: {:?}",
        out.diagnostics
    );
    assert_eq!(out.pages.len(), 1, "oracle is one page");
    // Content-stream order -- header lines, then the body (there is no
    // footer field in this source) -- and no page number: `\fancyhf{}`
    // cleared it. (The oracle's raw `pdftotext` prints the right-aligned
    // header after the body; see the module doc: that is block sorting.)
    assert_eq!(
        page_words(&out),
        ["Left", "Field", "Right", "Field", "Body", "text", "on", "page", "one."],
    );
    // Geometry: the header sits above the body, the footer below it.
    let y = |word: &str| {
        items(&out)
            .iter()
            .find(|item| item.text == word)
            .unwrap_or_else(|| panic!("no item {word:?}"))
            .baseline_y_pt
    };
    // Both header fields sit above the body (this source has no footer
    // field; `\fancyfoot` geometry is pinned below).
    assert!(
        y("Left") < y("Body"),
        "header above the body: {:?}",
        page_words(&out)
    );
    assert!(
        y("Right") < y("Body"),
        "header above the body: {:?}",
        page_words(&out)
    );
    // The stray page number the issue reported is gone: no bare `1`.
    assert!(
        !page_words(&out).iter().any(|word| word == "1"),
        "no page number: {:?}",
        page_words(&out)
    );
}

#[test]
fn fancyhf_alone_leaves_no_header_or_footer_text() {
    let text = "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\begin{document}\n\
         Body with cleared fields.\n\
         \\end{document}\n";
    let out = compile(text);
    assert!(
        out.diagnostics.is_empty(),
        "clearing everything must be silent: {:?}",
        out.diagnostics
    );
    // Only body words carry text; the default 0.4pt head rule the oracle
    // draws is a rule item, not text.
    assert_eq!(
        page_words(&out),
        ["Body", "with", "cleared", "fields."],
    );
    assert!(
        items(&out)
            .iter()
            .any(|item| item.rule.is_some()),
        "the oracle draws the default head rule with empty fields"
    );
}

#[test]
fn positions_edges_and_empty_cases() {
    // Combined even/odd positions parse without error in a oneside document.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhead[LE,RO]{Both}\n\
         \\fancyfoot[C]{}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);

    // A space before the bracket, a space inside the braces, and an empty
    // field on the other edge: all accepted, all silent.
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\fancyhead [L] { Left }\n\
         \\fancyhead[R]{}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(page_words(&out), ["Left", "Text."]);

    // `\\fancyhf` with a position fills both sides; parity alone (`[E]`)
    // selects every slot one-sided.
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf[E]{Everywhere}\n\
         \\fancyhf[R]{Side}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    // `[E]` filled every slot; the later `[R]` overwrote slot 2 -- head
    // lines stream L, C, R, then the body, then the foot lines.
    assert_eq!(
        page_words(&out),
        ["Everywhere", "Everywhere", "Side", "Text.", "Everywhere", "Everywhere", "Side"],
    );

    // A footer field streams after the body and sits below it.
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\fancyhead[C]{Head}\n\
         \\fancyfoot[R]{Foot}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(page_words(&out), ["Head", "Text.", "Foot"]);
    let y = |word: &str| {
        items(&out)
            .iter()
            .find(|item| item.text == word)
            .unwrap_or_else(|| panic!("no item {word:?}"))
            .baseline_y_pt
    };
    assert!(y("Head") < y("Text."), "header above: {:?}", page_words(&out));
    assert!(y("Text.") < y("Foot"), "footer below: {:?}", page_words(&out));

    // An unknown position letter warns instead of failing or misfiling.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\fancyhead[X]{Lost}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\fancyhead") && d.message.contains('X')),
        "unknown position must be diagnosed: {:?}",
        parsed.diagnostics
    );
    let out = compile_full(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhead[X]{Lost}\n\
         \\begin{document}\nText.\n\\end{document}\n",
        LayoutConstraints::default(),
    );
    assert!(
        !page_words(&out).contains(&"Lost".to_string()),
        "a mispositioned field must not leak onto the page: {:?}",
        page_words(&out)
    );
}

/// Text items outside the body band: the running head and foot.
fn chrome_words(out: &CompileOutput, page: usize) -> Vec<String> {
    out.pages[page]
        .items
        .iter()
        .filter(|item| item.baseline_y_pt < 80.0 || item.baseline_y_pt > 724.0)
        .filter(|item| !item.text.is_empty())
        .map(|item| item.text.clone())
        .collect()
}

#[test]
fn pagestyle_switches_apply_per_page() {
    // Enough body for two pages; `\newpage` puts the switch on its own
    // page so the first page ships before it runs.
    let body = "Filler sentence ends here. ".repeat(120);
    let text = format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\pagestyle{{fancy}}\n\
         \\fancyhf{{}}\n\
         \\fancyhead[L]{{Every}}\n\
         \\begin{{document}}\n\
         {body}\n\
         \\newpage\n\
         \\pagestyle{{empty}}\n\
         {body}\n\
         \\end{{document}}\n"
    );
    let out = compile(&text);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert!(out.pages.len() >= 2, "needs two pages, got {}", out.pages.len());
    assert!(
        chrome_words(&out, 0).contains(&"Every".to_string()),
        "first page keeps its header: {:?}",
        chrome_words(&out, 0)
    );
    assert!(
        chrome_words(&out, 1).is_empty(),
        "the switch clears later pages: {:?}",
        chrome_words(&out, 1)
    );

    // A mid-paragraph switch clears its own page too: measured against the
    // oracle (same shape, `switch.tex`: 3 pages, no header anywhere), the
    // switch takes effect immediately, not at the next page break.
    let text = format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\pagestyle{{fancy}}\n\
         \\fancyhf{{}}\n\
         \\fancyhead[L]{{Every}}\n\
         \\begin{{document}}\n\
         {body}\n\
         \\pagestyle{{empty}}\n\
         {body}\n\
         \\end{{document}}\n"
    );
    let out = compile(&text);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    for page in 0..out.pages.len() {
        assert!(
            chrome_words(&out, page).is_empty(),
            "page {page} ships under the switch: {:?}",
            chrome_words(&out, page)
        );
    }
}

#[test]
fn thispagestyle_clears_only_its_own_page() {
    let body = "Filler sentence ends here. ".repeat(120);
    let text = format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\pagestyle{{fancy}}\n\
         \\fancyhf{{}}\n\
         \\fancyhead[L]{{Every}}\n\
         \\begin{{document}}\n\
         \\thispagestyle{{empty}}\n\
         {body}\n\
         {body}\n\
         \\end{{document}}\n"
    );
    let out = compile(&text);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert!(out.pages.len() >= 2, "needs two pages, got {}", out.pages.len());
    assert!(
        chrome_words(&out, 0).is_empty(),
        "thispagestyle clears its own page: {:?}",
        chrome_words(&out, 0)
    );
    assert!(
        chrome_words(&out, 1).contains(&"Every".to_string()),
        "later pages keep the style: {:?}",
        chrome_words(&out, 1)
    );
}

#[test]
fn rule_widths_follow_setlength() {
    let text = |preamble: &str| {
        compile(&format!(
            "\\documentclass{{article}}\n\
             \\usepackage{{fancyhdr}}\n\
             \\pagestyle{{fancy}}\n\
             \\fancyhf{{}}\n\
             {preamble}\
             \\begin{{document}}\n\
             Body.\n\
             \\end{{document}}\n"
        ))
    };
    let rules = |out: &CompileOutput| {
        out.pages[0]
            .items
            .iter()
            .filter(|item| item.rule.is_some())
            .count()
    };
    assert_eq!(rules(&text("")), 1, "default: the 0.4pt head rule");
    let out = text("\\setlength{\\headrulewidth}{0pt}\n");
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(rules(&out), 0, "zero head rule draws nothing");
    let out = text("\\setlength{\\footrulewidth}{0.4pt}\n");
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(rules(&out), 2, "head rule plus foot rule");
}

#[test]
fn thepage_in_a_field_resolves_per_page() {
    // The canonical fancyhdr footer: `\\fancyfoot[C]{\\thepage}` numbers
    // each page with its own number.
    let body = "Filler sentence ends here. ".repeat(120);
    let out = compile(&format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\pagestyle{{fancy}}\n\
         \\fancyhf{{}}\n\
         \\fancyfoot[C]{{\\thepage}}\n\
         \\begin{{document}}\n\
         {body}\n\
         {body}\n\
         \\end{{document}}\n"
    ));
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert!(out.pages.len() >= 2, "needs two pages, got {}", out.pages.len());
    for (page, expected) in [(0, "1"), (1, "2")] {
        assert!(
            chrome_words(&out, page).contains(&expected.to_string()),
            "page {page} foots its own number: {:?}",
            chrome_words(&out, page)
        );
    }
}

#[test]
fn math_and_style_commands_work_inside_fields() {
    // The Ross corpus heads a set with `\\fancyhead[R]{Set $2$}`; styled
    // text works the same way.
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\usepackage{amsmath}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\fancyhead[L]{\\textbf{Bold}}\n\
         \\fancyhead[R]{Set $2$}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(page_words(&out), ["Bold", "Set", "2", "Text."]);
}

#[test]
fn fancyhdr_diagnostics_never_suggest_moving_to_the_body() {
    // Without the package the core commands say what is missing -- header
    // setup belongs in the preamble, so the generic "move it after
    // \begin{document}" advice must not fire.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\fancyhead[L]{Left}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\fancyhead needs \\usepackage{fancyhdr}")),
        "missing package must be named: {:?}",
        parsed.diagnostics
    );
    // The later slice's commands are recognised as fancyhdr's own, with no
    // move-advice either.
    for command in ["\\lhead{X}", "\\fancypagestyle{plain}{}"] {
        let parsed = parse(&format!(
            "\\documentclass{{article}}\n\
             \\usepackage{{fancyhdr}}\n\
             {command}\n\
             \\begin{{document}}\nText.\n\\end{{document}}\n"
        ));
        assert!(
            parsed.diagnostics.iter().any(|d| d
                .message
                .contains("recognised but not implemented")),
            "{command}: {:?}",
            parsed.diagnostics
        );
    }
    let leaked = parse(REPRO).diagnostics.iter().any(|d| {
        d.message.contains("move")
            || d.help
                .as_ref()
                .is_some_and(|help| help.message.contains("move"))
    });
    assert!(!leaked, "no move-advice: {:?}", parse(REPRO).diagnostics);
}
