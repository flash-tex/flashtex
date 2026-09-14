//! Slice B of the kernel-inventory audit: one observable-output test per
//! command for the second 20 of the 41 implemented-but-untested kernel
//! commands (engine primitives in `flashtex-tex-expansion`, parser arms in
//! `crates/compiler/src/parser.rs`, math arms in `src/math.rs`).

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::{text_width, Font, LayoutConstraints, TextItem, MARGIN_PT};
use flashtex_compiler::parser::{
    self, Block, Inline, ParagraphStyle, TextFamily, TextStyle,
};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn texts(output: &CompileOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .map(|item| item.text.clone())
        .collect()
}

fn joined(output: &CompileOutput) -> String {
    texts(output).concat()
}

fn messages(output: &CompileOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect()
}

fn assert_supported(output: &CompileOutput) {
    assert!(
        !messages(output)
            .iter()
            .any(|message| message.contains("not supported")),
        "unexpected unsupported diagnostic: {:?}",
        output.diagnostics
    );
}

fn item<'a>(output: &'a CompileOutput, text: &str) -> &'a TextItem {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.text == text)
        .unwrap_or_else(|| panic!("missing {text:?} in {:?}", texts(output)))
}

/// `(text, style)` runs of the first paragraph, as in `amsthm.rs`.
fn text_runs(source: &str) -> Vec<(String, TextStyle)> {
    parser::parse(source)
        .blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines,
            _ => Vec::new(),
        })
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text, style)),
            _ => None,
        })
        .collect()
}

fn run_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        if let Inline::Text { text, .. } = inline {
            out.push_str(text);
        }
    }
    out
}

#[test]
fn providecommand_defines_when_undefined_but_never_overrides() {
    let output = compile(r"\providecommand{\greet}{Hello}\newcommand{\fixed}{First}\providecommand{\fixed}{Second}\greet\ and \fixed.");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let all = joined(&output);
    assert!(all.contains("Hello"), "fresh name is defined: {all:?}");
    assert!(all.contains("First"), "existing definition is kept: {all:?}");
    assert!(!all.contains("Second"), "providecommand did not override: {all:?}");
}

#[ignore = "known bug (supervisor to file issue): `\\refstepcounter` label invisible to `\\ref` — `\\newcounter{myc}\\refstepcounter{myc}\\label{mylab}Value \\arabic{myc}, ref \\ref{mylab}.` renders `Value1,ref.` (empty ref) instead of `Value1,ref1.`"]
#[test]
fn refstepcounter_makes_the_counter_the_current_label() {
    let output = compile(r"\newcounter{myc}\refstepcounter{myc}\label{mylab}Value \arabic{myc}, ref \ref{mylab}.");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // Correct output: both the direct rendering and the \label/\ref
    // round-trip read "1". Currently the \ref renders empty.
    assert_eq!(
        texts(&output),
        ["Value", "1", ",", "ref", "1", "."],
        "{:?}",
        texts(&output)
    );
}

#[test]
fn renewenvironment_replaces_the_environment_expansion() {
    let output = compile(r"\newenvironment{shout}{Hi }{!}\renewenvironment{shout}{Yo }{?}\begin{shout}Bob\end{shout}");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // Exact item sequence: redefined begin, body, redefined end. A stale
    // end body leaking through would render "!" instead of "?", and stale
    // begin text ("Hi") would add an item, so both fail this equality.
    // (Inter-word spaces are layout gaps, not text items: even plain
    // `Yo Bob?` compiles to the spaceless join "YoBob?".)
    assert_eq!(texts(&output), ["Yo", "Bob", "?"], "{:?}", texts(&output));
}

#[test]
fn rmfamily_restores_the_roman_family() {
    let parsed = parser::parse(r"{\sffamily A\rmfamily B} C");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let runs = text_runs(r"{\sffamily A\rmfamily B} C");
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0].1.family, TextFamily::Sans);
    assert_eq!(runs[1].0, "B");
    assert_eq!(runs[1].1.family, TextFamily::Roman);
    // After the group closes the outer (roman) family is back: a family
    // declaration leaking past its group would leave `C` sans.
    assert_eq!(runs[2].0, "C");
    assert_eq!(runs[2].1.family, TextFamily::Roman);
    let output = compile(r"{\sffamily A\rmfamily B} C");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(item(&output, "A").font, Font::Helvetica);
    assert_eq!(item(&output, "B").font, Font::TimesRoman);
    assert_eq!(item(&output, "C").font, Font::TimesRoman);
}

#[test]
fn scriptscriptstyle_is_consumed_without_visible_output() {
    let output = compile(r"$p \scriptscriptstyle q$");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // The switch itself emits no item: exactly `p` then `q`, nothing extra.
    let items: Vec<&TextItem> = output.pages.iter().flat_map(|page| &page.items).collect();
    assert_eq!(items.len(), 2, "{:?}", texts(&output));
    assert_eq!(items[0].text, "p");
    assert_eq!(items[1].text, "q");
    // `math.rs` consumes the switch as a zero-width no-op ("accepted
    // without changing size" in `supported-latex.json`): `q` keeps `p`'s size.
    assert_eq!(items[1].font_size_pt, items[0].font_size_pt);
}

#[test]
fn scriptstyle_is_consumed_without_visible_output() {
    let output = compile(r"$p \scriptstyle q$");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // The switch itself emits no item: exactly `p` then `q`, nothing extra.
    let items: Vec<&TextItem> = output.pages.iter().flat_map(|page| &page.items).collect();
    assert_eq!(items.len(), 2, "{:?}", texts(&output));
    assert_eq!(items[0].text, "p");
    assert_eq!(items[1].text, "q");
    // `math.rs` consumes the switch as a zero-width no-op ("accepted
    // without changing size" in `supported-latex.json`): `q` keeps `p`'s size.
    assert_eq!(items[1].font_size_pt, items[0].font_size_pt);
}

#[ignore = "known bug (supervisor to file issue): `\\settodepth` stores 0pt, no BoxMeasurer wired — `\\newlength{\\mydepth}\\settodepth{\\mydepth}{g}\\the\\mydepth` renders `0.0pt` instead of a positive descender depth"]
#[test]
fn settodepth_stores_the_depth_in_a_length_register() {
    let output = compile(r"\newlength{\mydepth}\settodepth{\mydepth}{g}\the\mydepth");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let rendered = joined(&output);
    assert!(rendered.ends_with("pt"), "a dimension is rendered: {rendered:?}");
    let value: f64 = rendered.trim_end_matches("pt").parse().expect("numeric dimension");
    assert!(value > 0.0, "a descender has positive depth: {rendered:?}");
}

#[ignore = "known bug (supervisor to file issue): `\\settoheight` stores 0pt, no BoxMeasurer wired — `\\newlength{\\myheight}\\settoheight{\\myheight}{Ag}\\the\\myheight` renders `0.0pt` instead of a positive capital height"]
#[test]
fn settoheight_stores_the_height_in_a_length_register() {
    let output = compile(r"\newlength{\myheight}\settoheight{\myheight}{Ag}\the\myheight");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let rendered = joined(&output);
    assert!(rendered.ends_with("pt"), "a dimension is rendered: {rendered:?}");
    let value: f64 = rendered.trim_end_matches("pt").parse().expect("numeric dimension");
    assert!(value > 0.0, "a capital has positive height: {rendered:?}");
}

#[ignore = "known bug (supervisor to file issue): `\\settowidth` stores 0pt, no BoxMeasurer wired — `\\newlength{\\mywidth}\\settowidth{\\mywidth}{Hi}\\the\\mywidth` and the `HiHiHi` variant both render `0.0pt` instead of positive, growing widths"]
#[test]
fn settowidth_stores_the_width_and_grows_with_the_text() {
    let short = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{Hi}\the\mywidth");
    let long = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{HiHiHi}\the\mywidth");
    for output in [&short, &long] {
        assert_supported(output);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }
    let value = |output: &CompileOutput| {
        joined(output).trim_end_matches("pt").parse::<f64>().expect("numeric dimension")
    };
    assert!(value(&short) > 0.0);
    assert!(value(&long) > value(&short), "wider text measures wider");
}

#[test]
fn sffamily_switches_to_the_sans_family() {
    let parsed = parser::parse(r"Plain {\sffamily Sans} back");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let runs = text_runs(r"Plain {\sffamily Sans} back");
    assert_eq!(runs[0].1.family, TextFamily::Roman);
    assert_eq!(runs[1].0, "Sans");
    assert_eq!(runs[1].1.family, TextFamily::Sans);
    let output = compile(r"Plain {\sffamily Sans} back");
    assert_eq!(item(&output, "Sans").font, Font::Helvetica);
}

#[test]
fn slshape_selects_slanted_type() {
    let runs = text_runs(r"{\slshape Slanted}");
    assert_eq!(runs.len(), 1);
    assert!(runs[0].1.italic, "slshape sets italic: {:?}", runs[0].1);
}

#[test]
fn smallskip_inserts_three_points_of_vertical_space() {
    let parsed = parser::parse("A\n\n\\smallskip\n\nB");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let vspace: Vec<f64> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::VSpace { pt } => Some(*pt),
            _ => None,
        })
        .collect();
    // `SMALL_SKIP_PT` (`parser.rs`): 3pt against 6pt medskip / 12pt bigskip.
    assert_eq!(vspace, [3.0]);
}

#[test]
fn stackrel_stacks_its_script_over_the_base() {
    let output = compile(r"$\stackrel{top}{base}$");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // Math layout emits one item per character; "top"/"base" share no
    // letters, so each character item is unambiguous.
    let mut top_y = Vec::new();
    let mut base_y = Vec::new();
    for item in output.pages.iter().flat_map(|page| &page.items) {
        if item.text.len() == 1 {
            if "top".contains(item.text.as_str()) {
                top_y.push(item.baseline_y_pt);
            } else if "base".contains(item.text.as_str()) {
                base_y.push(item.baseline_y_pt);
            }
        }
    }
    assert_eq!(top_y.len(), 3, "{top_y:?}");
    assert_eq!(base_y.len(), 4, "{base_y:?}");
    assert!(
        top_y.iter().all(|y| base_y.iter().all(|b| y < b)),
        "script sits above the base: {top_y:?} vs {base_y:?}"
    );
}

#[test]
fn stepcounter_increments_and_resets_dependants() {
    let output = compile(r"\newcounter{myc}\newcounter{sub}[myc]\stepcounter{sub}\stepcounter{sub}\stepcounter{myc}M \arabic{myc} S \arabic{sub}");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // Exact order: `myc` stepped once reads 1, and stepping `myc` reset
    // its dependant `sub` to 0. Swapped lookups (`M 0 S 1`) fail this.
    let all = texts(&output);
    assert_eq!(all, ["M", "1", "S", "0"], "{all:?}");
}

#[test]
fn textmd_selects_medium_weight_inside_bold() {
    let runs = text_runs(r"{\bfseries A\textmd{B} C}");
    assert_eq!(runs.len(), 3);
    assert!(runs[0].1.bold, "before: {:?}", runs[0].1);
    assert_eq!(runs[1].0, "B");
    assert!(!runs[1].1.bold, "textmd cancels bold: {:?}", runs[1].1);
    assert!(runs[2].1.bold, "group scope restored: {:?}", runs[2].1);
}

#[test]
fn textnormal_resets_every_attribute() {
    let runs = text_runs(r"{\bfseries\itshape\sffamily A\textnormal{B}}");
    assert_eq!(runs[1].0, "B");
    assert_eq!(runs[1].1, TextStyle::default(), "fully reset: {:?}", runs[1].1);
}

#[test]
fn textstyle_is_consumed_without_visible_output() {
    let output = compile(r"$p \textstyle q$");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // The switch itself emits no item: exactly `p` then `q`, nothing extra.
    let items: Vec<&TextItem> = output.pages.iter().flat_map(|page| &page.items).collect();
    assert_eq!(items.len(), 2, "{:?}", texts(&output));
    assert_eq!(items[0].text, "p");
    assert_eq!(items[1].text, "q");
    // `math.rs` consumes the switch as a zero-width no-op ("accepted
    // without changing size" in `supported-latex.json`): `q` keeps `p`'s size.
    assert_eq!(items[1].font_size_pt, items[0].font_size_pt);
}

#[test]
fn upshape_cancels_slant_inside_italics() {
    // `\upshape` is a declaration, not an argument-taking command: it
    // applies to everything after it in the enclosing group.
    let runs = text_runs(r"{\itshape A\upshape B}");
    assert_eq!(runs.len(), 2);
    assert!(runs[0].1.italic, "before: {:?}", runs[0].1);
    assert_eq!(runs[1].0, "B");
    assert!(!runs[1].1.italic, "upshape cancels italic: {:?}", runs[1].1);
}

#[test]
fn flushright_pushes_the_line_to_the_right_margin() {
    let plain = compile("Hi");
    let flush = compile(r"\begin{flushright}Hi\end{flushright}");
    for output in [&plain, &flush] {
        assert_supported(output);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }
    let plain_item = item(&plain, "Hi");
    let flush_item = item(&flush, "Hi");
    assert_eq!(plain_item.x_pt, MARGIN_PT);
    assert!(
        flush_item.x_pt > plain_item.x_pt,
        "right-aligned starts further right: {} vs {}",
        flush_item.x_pt,
        plain_item.x_pt
    );
    let size = flush_item.font_size_pt;
    let right_edge = flush_item.x_pt + text_width("Hi", size, Font::TimesRoman);
    assert!(
        (right_edge - (MARGIN_PT + 468.0)).abs() < 0.05,
        "line ends at the right margin: {right_edge}"
    );
}

#[test]
fn quotation_reports_a_quote_block_and_indents() {
    let parsed = parser::parse(r"\begin{quotation}Hi\end{quotation}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let styled: Vec<_> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Styled { style, content, .. } => Some((*style, content)),
            _ => None,
        })
        .collect();
    assert_eq!(styled.len(), 1);
    assert_eq!(styled[0].0, ParagraphStyle::Quote);
    assert!(run_text(styled[0].1).contains("Hi"));
    let output = compile(r"\begin{quotation}Hi\end{quotation}");
    assert!(item(&output, "Hi").x_pt > MARGIN_PT, "both margins indented");
}
