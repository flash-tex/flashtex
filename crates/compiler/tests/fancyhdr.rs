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

use flashtex_compiler::incremental::{compile_full, CompileOutput, Session};
use flashtex_compiler::layout::{LayoutConstraints, TextItem};
use flashtex_compiler::parser::{parse, Inline, Parsed};

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

    // `\\fancyhf` with a position fills both sides; an even-only `[E]`
    // selects nothing one-sided (LaTeX's `\@outputpage` always uses
    // `\@oddhead`, so even fields never ship -- the oracle is silent too).
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf[E]{Everywhere}\n\
         \\fancyhf[R]{Side}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    // `[E]` filled no slot; the later `[R]` filled slot 2 alone -- head
    // lines stream L, C, R, then the body, then the foot lines.
    assert_eq!(page_words(&out), ["Side", "Text.", "Side"],);

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

/// One fancyhdr field document: `field_line` is the `\fancyhead...` line.
/// Returns `(hits, x)` for items whose text is exactly `marker` on page 0.
fn field_hits(field_line: &str, marker: &str) -> (usize, f64) {
    let text = format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\pagestyle{{fancy}}\n\
         \\fancyhf{{}}\n\
         {field_line}\n\
         \\begin{{document}}\n\
         Body.\n\
         \\end{{document}}\n"
    );
    let out = compile(&text);
    assert!(
        out.diagnostics.is_empty(),
        "{field_line}: {:?}",
        out.diagnostics
    );
    let mut xs: Vec<f64> = out.pages[0]
        .items
        .iter()
        .filter(|item| item.text == marker)
        .map(|item| item.x_pt)
        .collect();
    xs.sort_by(|a, b| a.total_cmp(b));
    let n = xs.len();
    (n, xs.first().copied().unwrap_or(f64::NAN))
}

#[test]
fn le_ro_places_the_field_on_the_right_only() {
    // The canonical fancyhdr manual idiom in a one-sided document: LaTeX's
    // `\@outputpage` always uses `\@oddhead`, so only the `O` group ships.
    // Oracle (pdflatex TeX Live 2026, `Package: fancyhdr 2025/02/07 v5.2` in
    // the .log): `\fancyhead[LE,RO]{PAGEMARK}` renders once, xMin=418.05
    // (RIGHT only). This engine's metrics differ from pdflatex's, so the pin
    // is exact equality with this engine's own bare-`[R]` x, plus strictly
    // right of its bare-`[L]` x -- a both-sides regression cannot satisfy it.
    let (n_ref, x_ref) = field_hits("\\fancyhead[R]{PAGEMARK}", "PAGEMARK");
    assert_eq!(n_ref, 1, "reference [R] must render once");
    let (n, x) = field_hits("\\fancyhead[LE,RO]{PAGEMARK}", "PAGEMARK");
    assert_eq!(
        n, 1,
        "[LE,RO] must render the field once, not on both sides"
    );
    assert_eq!(
        x, x_ref,
        "[LE,RO] must land at the RIGHT slot x ({x_ref}), got {x}"
    );
    let (_, x_left) = field_hits("\\fancyhead[L]{PAGEMARK}", "PAGEMARK");
    assert!(
        x > x_left,
        "[LE,RO] x ({x}) must be right of the LEFT slot x ({x_left})"
    );
    // A space after the comma is the same idiom, not a new position.
    let (n_sp, x_sp) = field_hits("\\fancyhead[LE, RO]{PAGEMARK}", "PAGEMARK");
    assert_eq!((n_sp, x_sp), (1, x_ref), "[LE, RO] must match [LE,RO]");
}

#[test]
fn lo_re_places_the_field_on_the_left_only() {
    // Mirror idiom: only the `O` group (`LO`) ships one-sided.
    // Oracle: `\fancyhead[LO,RE]{MARKB}` renders only LEFT, xMin=133.77.
    let (n_ref, x_ref) = field_hits("\\fancyhead[L]{MARKB}", "MARKB");
    assert_eq!(n_ref, 1, "reference [L] must render once");
    let (n, x) = field_hits("\\fancyhead[LO,RE]{MARKB}", "MARKB");
    assert_eq!(
        n, 1,
        "[LO,RE] must render the field once, not on both sides"
    );
    assert_eq!(
        x, x_ref,
        "[LO,RE] must land at the LEFT slot x ({x_ref}), got {x}"
    );
    let (_, x_right) = field_hits("\\fancyhead[R]{MARKB}", "MARKB");
    assert!(
        x < x_right,
        "[LO,RE] x ({x}) must be left of the RIGHT slot x ({x_right})"
    );
}

#[test]
fn empty_bracket_still_selects_every_slot() {
    // The empty case: an explicit `[]` carries no position, so fancyhdr's
    // default (every slot) applies -- six fields around one body word.
    let (n, _) = field_hits("\\fancyhf[]{X}", "X");
    assert_eq!(n, 6, "[] must fill all six slots, got {n} hits");
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
fn pagestyle_empty_does_not_force_a_full_recompile() {
    // Editor latency, not rendering: `empty` renders nothing here, so its
    // marker must not set document-global state (which forces a full
    // recompile on every keystroke). A one-character body edit must reuse
    // the untouched paragraph and render exactly what a full build renders.
    let base = "\\documentclass{article}\n\
         \\pagestyle{empty}\n\
         \\begin{document}\n\
         First paragraph stays.\n\n\
         Body text here.\n\
         \\end{document}\n";
    let edited = base.replace("here.", "here!");
    let constraints = LayoutConstraints::default();
    let mut session = Session::new();
    session.compile(base, constraints);
    let incremental = session.compile(&edited, constraints);
    let clean = compile_full(&edited, constraints);
    assert_eq!(
        format!("{:#?}", incremental.output),
        format!("{clean:#?}"),
        "reuse must render exactly what a full build renders"
    );
    assert!(
        !incremental.stats.full_recompile,
        "a one-character edit must not force a full recompile: {:?}",
        incremental.stats
    );
    assert!(
        incremental.stats.blocks_reused > 0,
        "the untouched paragraph must be reused: {:?}",
        incremental.stats
    );
}

#[test]
fn pagestyle_fancy_still_forces_a_full_recompile() {
    // The other direction of the gate: `fancy` ships running heads, so its
    // marker keeps setting document-global state and still rebuilds fully.
    let base = "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\fancyhead[L]{Head}\n\
         \\begin{document}\n\
         First paragraph stays.\n\n\
         Body text here.\n\
         \\end{document}\n";
    let edited = base.replace("here.", "here!");
    let constraints = LayoutConstraints::default();
    let mut session = Session::new();
    session.compile(base, constraints);
    let incremental = session.compile(&edited, constraints);
    assert!(
        incremental.stats.full_recompile,
        "a fancy marker must still force a full recompile: {:?}",
        incremental.stats
    );
}

#[test]
fn maketitle_suppresses_the_running_head_on_the_title_page_only() {
    // `\maketitle` issues `\thispagestyle{plain}` (article.cls): the title
    // page ships with no running head, later pages keep the ambient style.
    // Oracle: no header on page 1, `HDRMARK` on page 2.
    let body = "Filler sentence ends here. ".repeat(160);
    let text = format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\pagestyle{{fancy}}\n\
         \\fancyhf{{}}\n\
         \\fancyhead[L]{{HDRMARK}}\n\
         \\title{{A Title}}\n\
         \\author{{An Author}}\n\
         \\begin{{document}}\n\
         \\maketitle\n\
         {body}\n\
         \\end{{document}}\n"
    );
    let out = compile(&text);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert!(
        out.pages.len() >= 2,
        "needs two pages, got {}",
        out.pages.len()
    );
    assert!(
        !chrome_words(&out, 0).contains(&"HDRMARK".to_string()),
        "title page carries no running head: {:?}",
        chrome_words(&out, 0)
    );
    assert!(
        chrome_words(&out, 1).contains(&"HDRMARK".to_string()),
        "later pages keep the head: {:?}",
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
    // The implemented single-slot command is silent; the named-style
    // command is implemented too, so an empty body defines an empty style
    // silently, with no move-advice either.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\lhead{X}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "\\lhead is implemented now: {:?}",
        parsed.diagnostics
    );
    let parsed = parse(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\fancypagestyle{plain}{}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "\\fancypagestyle is implemented now: {:?}",
        parsed.diagnostics
    );
    let leaked = parse(REPRO).diagnostics.iter().any(|d| {
        d.message.contains("move")
            || d.help
                .as_ref()
                .is_some_and(|help| help.message.contains("move"))
    });
    assert!(!leaked, "no move-advice: {:?}", parse(REPRO).diagnostics);
}

#[test]
fn single_slot_commands_fill_their_own_slots() {
    // The six `\lhead`/`\chead`/`\rhead` / `\lfoot`/`\cfoot`/`\rfoot`
    // commands (issue #833's repro used `\rhead`): silent, streaming in
    // content-stream order -- header L, C, R, then the body, then footer
    // L, C, R -- with the header above the body and the footer below it.
    // Oracle (pdflatex TeX Live 2026): the `\rhead` header sits ~38pt
    // above the body baseline, right-aligned on the same line the
    // `\fancyhead[R]` field would take.
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\lhead{LH}\n\
         \\chead{CH}\n\
         \\rhead{RH}\n\
         \\lfoot{LF}\n\
         \\cfoot{CF}\n\
         \\rfoot{RF}\n\
         \\begin{document}\n\
         Body.\n\
         \\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(
        page_words(&out),
        ["LH", "CH", "RH", "Body.", "LF", "CF", "RF"],
    );
    let y = |word: &str| {
        items(&out)
            .iter()
            .find(|item| item.text == word)
            .unwrap_or_else(|| panic!("no item {word:?}"))
            .baseline_y_pt
    };
    assert!(y("RH") < y("Body."), "header above: {:?}", page_words(&out));
    assert!(y("Body.") < y("RF"), "footer below: {:?}", page_words(&out));
    // Each single-slot command lands exactly where the matching
    // `\fancyhead`/`\fancyfoot` position would put the same marker: the
    // same slot, the same placement step.
    for (single, positional) in [
        ("\\lhead{SLOT}", "\\fancyhead[L]{SLOT}"),
        ("\\chead{SLOT}", "\\fancyhead[C]{SLOT}"),
        ("\\rhead{SLOT}", "\\fancyhead[R]{SLOT}"),
        ("\\lfoot{SLOT}", "\\fancyfoot[L]{SLOT}"),
        ("\\cfoot{SLOT}", "\\fancyfoot[C]{SLOT}"),
        ("\\rfoot{SLOT}", "\\fancyfoot[R]{SLOT}"),
    ] {
        let (_, x_single) = field_hits(single, "SLOT");
        let (n_pos, x_pos) = field_hits(positional, "SLOT");
        assert_eq!(n_pos, 1, "{positional} must render once");
        assert_eq!(
            (x_single.is_nan(), x_single),
            (false, x_pos),
            "{single} must land where {positional} lands"
        );
    }
}

#[test]
fn single_slot_optional_even_group_is_consumed_and_ignored() {
    // The oracle stores `\rhead[even]{odd}`'s bracket into the even-page
    // field, which never ships one-sided -- so the odd text renders once
    // and the even text never leaks onto the page or into diagnostics.
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\rhead[Even]{Odd}\n\
         \\begin{document}\n\
         Body.\n\
         \\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(page_words(&out), ["Odd", "Body."]);
}

#[test]
fn single_slot_without_the_package_names_what_is_missing() {
    // As with `\fancyhead`, header setup without the package says what is
    // missing -- never the generic move-it-to-the-body advice, and never
    // a leak of the field text onto the page.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\rhead{Right}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\rhead needs \\usepackage{fancyhdr}")),
        "missing package must be named: {:?}",
        parsed.diagnostics
    );
    let out = compile(
        "\\documentclass{article}\n\
         \\rhead{Right}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        !page_words(&out).contains(&"Right".to_string()),
        "an unstored field must not leak onto the page: {:?}",
        page_words(&out)
    );
}

/// Plain text of one running-head field: the whole [`Parsed::fancy`]
/// state a `\pagestyle` activation reads back, reduced to comparable
/// strings (runs concatenate; both sides split words identically).
fn field_text(field: &[Inline]) -> String {
    field
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .concat()
}

/// The six field texts plus both rule widths: the header state a named
/// or direct `\pagestyle` ships.
fn fancy_state(parsed: &Parsed) -> (Vec<String>, f64, f64) {
    let head = parsed.fancy.head.iter().map(|f| field_text(f)).collect();
    let mut fields: Vec<String> = head;
    fields.extend(parsed.fancy.foot.iter().map(|f| field_text(f)));
    (fields, parsed.fancy.headrule_pt, parsed.fancy.footrule_pt)
}

/// `\fancypagestyle{mystyle}{\fancyhf{}\lhead{L}}` plus
/// `\pagestyle{mystyle}` ships exactly the header state a direct
/// `\pagestyle{fancy}\fancyhf{}\lhead{L}` sequence would: the stored left
/// field alone, with the default 0.4pt head rule and no diagnostics.
#[test]
fn fancypagestyle_named_style_ships_like_the_direct_sequence() {
    const NAMED: &str = "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\fancypagestyle{mystyle}{\\fancyhf{}\\lhead{L}}\n\
         \\pagestyle{mystyle}\n\
         \\begin{document}\n\
         Body text on page one.\n\
         \\end{document}\n";
    const DIRECT: &str = "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\fancyhf{}\n\
         \\lhead{L}\n\
         \\begin{document}\n\
         Body text on page one.\n\
         \\end{document}\n";
    let named = parse(NAMED);
    let direct = parse(DIRECT);
    assert!(
        named.diagnostics.is_empty(),
        "a clean named-style definition is silent: {:?}",
        named.diagnostics
    );
    assert_eq!(
        fancy_state(&named),
        fancy_state(&direct),
        "named vs direct header state"
    );
    let named_out = compile(NAMED);
    let direct_out = compile(DIRECT);
    assert_eq!(
        page_words(&named_out),
        page_words(&direct_out),
        "named vs direct rendered page"
    );
    assert!(
        page_words(&named_out).contains(&"L".to_string()),
        "the stored left field ships: {:?}",
        page_words(&named_out)
    );
}

/// A `\pagestyle` name with no `\fancypagestyle` definition keeps the
/// long-standing honest no-op: silent, and no header ships.
#[test]
fn pagestyle_undefined_name_stays_a_silent_noop() {
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{neverdefined}\n\
         \\begin{document}\n\
         Body text on page one.\n\
         \\end{document}\n",
    );
    assert!(
        out.diagnostics.is_empty(),
        "an unknown style name stays silent: {:?}",
        out.diagnostics
    );
    assert!(
        !page_words(&out).iter().any(|word| word == "L"),
        "no stored field ships: {:?}",
        page_words(&out)
    );
}

/// As with `\lhead`, a named-style definition without the package says
/// what is missing instead of claiming to define anything.
#[test]
fn fancypagestyle_without_the_package_names_what_is_missing() {
    let parsed = parse(
        "\\documentclass{article}\n\
         \\fancypagestyle{mystyle}{\\fancyhf{}\\lhead{L}}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\fancypagestyle needs \\usepackage{fancyhdr}")),
        "missing package must be named: {:?}",
        parsed.diagnostics
    );
    assert!(
        !parsed.fancy_styles.contains_key("mystyle"),
        "nothing is stored without the package"
    );
}

/// Body material that is not header state has no page-style meaning: the
/// fields are still defined, and the dropped prose is diagnosed where it
/// is dropped instead of leaking onto the page.
#[test]
fn fancypagestyle_stray_body_text_is_diagnosed_not_typeset() {
    let source = "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\fancypagestyle{mystyle}{\\fancyhf{}\\lhead{L}stray}\n\
         \\pagestyle{mystyle}\n\
         \\begin{document}\n\
         Body text on page one.\n\
         \\end{document}\n";
    let parsed = parse(source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\fancypagestyle body holds text")),
        "stray body material must be diagnosed: {:?}",
        parsed.diagnostics
    );
    let out = compile(source);
    assert!(
        !page_words(&out).contains(&"stray".to_string()),
        "stray body text must not leak onto the page: {:?}",
        page_words(&out)
    );
    assert!(
        page_words(&out).contains(&"L".to_string()),
        "the stored field still ships: {:?}",
        page_words(&out)
    );
}

/// `\\thispagestyle{special}` is a one-page override: page 2 ships the
/// named style's footer, and page 3 reverts to the surrounding
/// `\\pagestyle{plain}` instead of keeping the special footer.
#[test]
fn thispagestyle_named_style_reverts_after_one_page() {
    let body = "Filler sentence ends here. ".repeat(120);
    let text = format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\fancypagestyle{{special}}{{\\fancyhf{{}}\\fancyfoot[C]{{SpecialFoot}}}}\n\
         \\pagestyle{{plain}}\n\
         \\begin{{document}}\n\
         {body}\n\
         \\newpage\n\
         \\thispagestyle{{special}}\n\
         {body}\n\
         \\newpage\n\
         {body}\n\
         \\end{{document}}\n"
    );
    let out = compile(&text);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert!(
        out.pages.len() >= 3,
        "needs three pages, got {}",
        out.pages.len()
    );
    assert!(
        chrome_words(&out, 1).contains(&"SpecialFoot".to_string()),
        "page 2 ships the special footer: {:?}",
        chrome_words(&out, 1)
    );
    assert!(
        !chrome_words(&out, 2).contains(&"SpecialFoot".to_string()),
        "page 3 reverts to plain, not special: {:?}",
        chrome_words(&out, 2)
    );
}

/// The same one-page override under a surrounding `\pagestyle{fancy}`:
/// the named snapshot must not overwrite the live fields, so pages 1 and
/// 3 keep the surrounding header and never show the special footer.
#[test]
fn thispagestyle_named_style_keeps_surrounding_fancy_fields() {
    let body = "Filler sentence ends here. ".repeat(120);
    let text = format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{fancyhdr}}\n\
         \\pagestyle{{fancy}}\n\
         \\fancyhf{{}}\n\
         \\fancyhead[L]{{Every}}\n\
         \\fancypagestyle{{special}}{{\\fancyhf{{}}\\fancyfoot[C]{{SpecialFoot}}}}\n\
         \\begin{{document}}\n\
         {body}\n\
         \\newpage\n\
         \\thispagestyle{{special}}\n\
         {body}\n\
         \\newpage\n\
         {body}\n\
         \\end{{document}}\n"
    );
    let out = compile(&text);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert!(
        out.pages.len() >= 3,
        "needs three pages, got {}",
        out.pages.len()
    );
    assert!(
        chrome_words(&out, 0).contains(&"Every".to_string()),
        "page 1 keeps the surrounding header: {:?}",
        chrome_words(&out, 0)
    );
    assert!(
        !chrome_words(&out, 0).contains(&"SpecialFoot".to_string()),
        "page 1 predates the override: {:?}",
        chrome_words(&out, 0)
    );
    assert!(
        chrome_words(&out, 1).contains(&"SpecialFoot".to_string()),
        "page 2 ships the special footer: {:?}",
        chrome_words(&out, 1)
    );
    assert!(
        chrome_words(&out, 2).contains(&"Every".to_string()),
        "page 3 reverts to the surrounding header: {:?}",
        chrome_words(&out, 2)
    );
    assert!(
        !chrome_words(&out, 2).contains(&"SpecialFoot".to_string()),
        "page 3 reverts to fancy, not special: {:?}",
        chrome_words(&out, 2)
    );
}
