//! Slice B of the kernel-inventory audit: one observable-output test per
//! command for the second 20 of the 41 implemented-but-untested kernel
//! commands (engine primitives in `flashtex-tex-expansion`, parser arms in
//! `crates/compiler/src/parser.rs`, math arms in `src/math.rs`).

use flashtex_compiler::diagnostics::Severity;
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

/// Parser-level coverage for the family switch: the run asserts below read the parsed `TextStyle`s; only the font asserts check the compiled output.
#[test]
fn rmfamily_restores_the_roman_family() {
    let parsed = parser::parse(r"{\sffamily A\rmfamily B} C");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Parser-level: the family asserts below read the parsed runs, not layout.
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

/// Slice 2 (`boxmeasurer` review finding #492): a styled argument measures at
/// its own face, not as the literal characters of the command name at plain
/// Times-Roman. The expectation is a hardcoded literal from the Adobe AFM
/// metrics this compiler shapes (Times-Bold H=778, i=278 per 1000em, no H-i
/// kern: (778 + 278) / 1000 * 12 = 12.672pt), never a `layout` helper call —
/// a shared metric error must fail here, not pass on both sides. No pdflatex
/// oracle exists for these Times boxes (pdflatex defaults to cmr), so the AFM
/// tables are the independent ground truth pending a real oracle run.
#[test]
fn settowidth_measures_textbf_at_bold_width() {
    let output = compile(r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{\textbf{Hi}}\the\mywidth");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let rendered = joined(&output);
    let value: f64 = rendered.trim_end_matches("pt").parse().expect("numeric dimension");
    assert!(
        (value - 12.672).abs() < 0.02,
        "bold Hi measures {value}pt, AFM expects 12.672pt ({rendered:?})"
    );
    // The old flat-stringify bug measured the literal `\textbf Hi`
    // characters as plain text, several times wider than 12.672pt.
    assert!(
        value < 20.0,
        "must not measure the command name literally: {value}pt"
    );
}

/// Slice 2: `~` is the tie — an interword space of the font in force — not a
/// tilde glyph. Both expectations are hardcoded AFM literals (Times-Roman
/// a=444, space=250, b=500, "~"=541 per 1000em at 12pt: a~b as a tie is
/// (444 + 250 + 500) / 1000 * 12 = 14.328pt, while a literal tilde shaping
/// is (444 + 541 + 500) / 1000 * 12 = 17.82pt), never `layout` helper calls.
#[test]
fn settowidth_measures_tie_as_interword_space() {
    let output = compile(r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{a~b}\the\mywidth");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let rendered = joined(&output);
    let value: f64 = rendered.trim_end_matches("pt").parse().expect("numeric dimension");
    assert!(
        (value - 14.328).abs() < 0.02,
        "a~b measures {value}pt, AFM expects 14.328pt ({rendered:?})"
    );
    // A literal tilde glyph (the old behavior) is 17.82pt, clearly wider.
    assert!(
        (value - 17.82).abs() > 0.05,
        "must not shape the tie as a tilde: {value}pt"
    );
}

/// Slice 2: height reflects the size in effect (`\Large` at the 12pt body
/// size is 17.28pt against a 12pt body, so the ratio is exactly 1.44). The
/// body height is additionally pinned to its AFM literal (Times-Roman
/// ascender 683/1000 * 12 = 8.196pt), not just a ratio, so a shared metric
/// error cannot pass on both sides.
#[test]
fn settoheight_uses_the_size_in_effect() {
    let value = |source: &str| {
        let output = compile(source);
        assert_supported(&output);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let rendered = joined(&output);
        rendered.trim_end_matches("pt").parse::<f64>().expect("numeric dimension")
    };
    let body = value(r"\documentclass[12pt]{article}\newlength{\myheight}\settoheight{\myheight}{X}\the\myheight");
    let large = value(r"\documentclass[12pt]{article}\newlength{\myheight}\settoheight{\myheight}{{\Large X}}\the\myheight");
    assert!(body > 0.0 && large > 0.0);
    assert!(
        (body - 8.196).abs() < 0.02,
        "body X height is {body}pt, AFM expects 8.196pt"
    );
    let ratio = large / body;
    assert!(
        (ratio - 1.44).abs() < 0.01,
        "\\Large height {large}pt should scale body height {body}pt by 1.44"
    );
}

/// Parser-level coverage for the family switch: the run asserts below read the parsed `TextStyle`s; only the font assert checks the compiled output.
#[test]
fn sffamily_switches_to_the_sans_family() {
    let parsed = parser::parse(r"Plain {\sffamily Sans} back");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Parser-level: the family asserts below read the parsed runs, not layout.
    let runs = text_runs(r"Plain {\sffamily Sans} back");
    assert_eq!(runs[0].1.family, TextFamily::Roman);
    assert_eq!(runs[1].0, "Sans");
    assert_eq!(runs[1].1.family, TextFamily::Sans);
    let output = compile(r"Plain {\sffamily Sans} back");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(item(&output, "Sans").font, Font::Helvetica);
}

/// Parser-level coverage: asserts the parsed `TextStyle`, not laid-out/rendered output.
#[test]
fn slshape_selects_slanted_type() {
    let parsed = parser::parse(r"{\slshape Slanted}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Parser-level: the italic assert below reads the parsed run, not layout.
    let runs = text_runs(r"{\slshape Slanted}");
    assert_eq!(runs.len(), 1);
    assert!(runs[0].1.italic, "slshape sets italic: {:?}", runs[0].1);
}

/// Parser-level coverage: asserts the parsed block shape, not laid-out/rendered output.
#[test]
fn smallskip_inserts_three_points_of_vertical_space() {
    let parsed = parser::parse("A\n\n\\smallskip\n\nB");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Parser-level: only the parsed `VSpace` block is asserted, not rendered gap size.
    let vspace: Vec<f64> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::VSpace { pt, .. } => Some(*pt),
            _ => None,
        })
        .collect();
    // `SMALL_SKIP_PT` (`parser.rs`): 3pt against 6pt medskip / 12pt bigskip.
    assert_eq!(vspace, [3.0]);
}

/// Verifies only the vertical stacking order of script over base; the exact size and centring of the script relative to the base are unverified.
#[test]
fn stackrel_stacks_its_script_over_the_base() {
    let output = compile(r"$\stackrel{top}{base}$");
    assert_supported(&output);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // Only the vertical stacking order is verified here, not the exact
    // size or centring of the script relative to the base.
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

/// Parser-level coverage: asserts the parsed `TextStyle`, not laid-out/rendered output.
#[test]
fn textmd_selects_medium_weight_inside_bold() {
    let parsed = parser::parse(r"{\bfseries A\textmd{B} C}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Parser-level: the weight asserts below read the parsed runs, not layout.
    let runs = text_runs(r"{\bfseries A\textmd{B} C}");
    assert_eq!(runs.len(), 3);
    assert!(runs[0].1.bold, "before: {:?}", runs[0].1);
    assert_eq!(runs[1].0, "B");
    assert!(!runs[1].1.bold, "textmd cancels bold: {:?}", runs[1].1);
    assert!(runs[2].1.bold, "group scope restored: {:?}", runs[2].1);
}

/// Parser-level coverage: asserts the parsed `TextStyle`, not laid-out/rendered output.
#[test]
fn textnormal_resets_every_attribute() {
    let parsed = parser::parse(r"{\bfseries\itshape\sffamily A\textnormal{B}}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Parser-level: the reset assert below reads the parsed run, not layout.
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

/// Parser-level coverage: asserts the parsed `TextStyle`, not laid-out/rendered output.
#[test]
fn upshape_cancels_slant_inside_italics() {
    // `\upshape` is a declaration, not an argument-taking command: it
    // applies to everything after it in the enclosing group.
    let parsed = parser::parse(r"{\itshape A\upshape B}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Parser-level: the italic asserts below read the parsed runs, not layout.
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
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // Only the left indent is checked here: "Hi" is too short to wrap, so
    // its right edge reflects the glyph width, not the block's right
    // margin -- observing the right indent needs a line long enough to
    // wrap, which is out of scope for this test.
    assert!(item(&output, "Hi").x_pt > MARGIN_PT, "left margin indented");
}

// ===== BoxMeasurer review findings (PR #492 follow-up) =====
// Expected dimensions below are hardcoded literals derived from the Adobe AFM
// metric tables this compiler shapes (`crates/font-engine/src/generated.rs`,
// provenance in that file's header), not from the `layout` helpers the
// measurer itself uses: Times-Roman advances per 1000em are H=722, i=278
// (no H-i kern), A=722, V=722 (A-V kern -135), a=444, b=500, space=250,
// "~"=541; Courier advances are all 600/1000em; Times-Bold H=778, i=278.
// Widths assume no kerning/ligature applies to the measured string (true for
// each string used here; "AV" accounts its kern explicitly).

/// The measured dimension rendered by `\the` for a one-box source.
fn box_value(source: &str) -> f64 {
    let output = compile(source);
    let rendered = joined(&output);
    rendered.trim_end_matches("pt").parse().expect(&format!(
        "numeric dimension in {rendered:?} (diagnostics: {:?})",
        output.diagnostics
    ))
}

/// Review finding 2: `\tiny`..`\Huge` (and plain text) resolve against the
/// document's class size, not a hardcoded 12pt body.
#[test]
fn settowidth_uses_the_document_class_size() {
    let ten = box_value(
        r"\documentclass[10pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{Hi}\the\mywidth",
    );
    let twelve = box_value(
        r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{Hi}\the\mywidth",
    );
    // AFM Times-Roman "Hi" = (722 + 278) / 1000 em: 10.0pt at 10pt, 12.0pt at 12pt.
    assert!((ten - 10.0).abs() < 0.02, "10pt class measures {ten}pt");
    assert!((twelve - 12.0).abs() < 0.02, "12pt class measures {twelve}pt");
}

/// Review finding 3: math content is a diagnosed limitation, never a silent
/// literal `$x$` text width.
#[test]
fn settowidth_with_math_reports_a_diagnostic() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{$x$}\the\mywidth");
    assert!(
        messages(&output).iter().any(|m| m.contains("math") && m.contains("settowidth")),
        "math inside a setto box must diagnose: {:?}",
        output.diagnostics
    );
}

/// Review finding 4: `\hspace{1cm}` contributes its glue instead of measuring
/// the literal `1cm` as text.
#[test]
fn settowidth_measures_hspace_by_its_length() {
    let value = box_value(r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{\hspace{1cm}A}\the\mywidth");
    // 1cm = 28.45274pt (TeX: 72.27pt/in, 2.54cm/in) plus AFM "A" 722/1000*12 = 8.664pt.
    assert!((value - 37.117).abs() < 0.05, "\\hspace{{1cm}}A measures {value}pt");
}

/// Review finding 4: `\textasciitilde` measures as the tilde glyph, not zero.
#[test]
fn settowidth_measures_textasciitilde_as_a_tilde() {
    let value = box_value(r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{\textasciitilde}\the\mywidth");
    // AFM Times-Roman "~" = 541/1000*12 = 6.492pt.
    assert!((value - 6.492).abs() < 0.02, "\\textasciitilde measures {value}pt");
}

/// Review finding 4: an unmeasurable command diagnoses instead of silently
/// contributing zero (`\LaTeX` is a logo the box measurer cannot set).
#[test]
fn settowidth_with_an_unmeasurable_command_reports_a_diagnostic() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{\LaTeX}\the\mywidth");
    assert!(
        messages(&output).iter().any(|m| m.contains("LaTeX") && m.contains("not measured")),
        "unmeasurable content must diagnose: {:?}",
        output.diagnostics
    );
}

/// Review finding 5: a space deferred across a style change is charged in the
/// font in force at the space, not the font of the next character.
#[test]
fn settowidth_charges_a_deferred_space_in_the_space_font() {
    let value = box_value(r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{a \texttt{b}}\the\mywidth");
    // AFM Times a=444, space=250 at 12pt, then Courier b=600 at 12pt:
    // 5.328 + 3.0 + 7.2 = 15.528pt.
    assert!((value - 15.528).abs() < 0.02, "deferred space measures {value}pt");
}

/// Review finding 6: empty boxes have no height or depth.
#[test]
fn settoheight_and_settodepth_are_zero_for_empty_boxes() {
    assert_eq!(box_value(r"\newlength{\myheight}\settoheight{\myheight}{}\the\myheight"), 0.0);
    assert_eq!(box_value(r"\newlength{\mydepth}\settodepth{\mydepth}{}\the\mydepth"), 0.0);
}

/// Review finding 6: descender-free text has no depth (`Hi` gets the face
/// descender under the old code).
#[test]
fn settodepth_is_zero_without_descenders() {
    assert_eq!(box_value(r"\newlength{\mydepth}\settodepth{\mydepth}{Hi}\the\mydepth"), 0.0);
}

/// Review finding 7: kerning applies across transparent groups (`A{}V` sets
/// the `AV` kern, exactly as `AV` does).
#[test]
fn settowidth_kerns_across_transparent_groups() {
    let grouped = box_value(r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{A{}V}\the\mywidth");
    let plain = box_value(r"\documentclass[12pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{AV}\the\mywidth");
    // AFM Times-Roman A=722, V=722, A-V kern -135: (722 + 722 - 135)/1000*12 = 15.708pt.
    assert!((plain - 15.708).abs() < 0.02, "AV measures {plain}pt");
    assert!(
        (grouped - plain).abs() < 0.005,
        "A{{}}V ({grouped}pt) must kern like AV ({plain}pt)"
    );
}

/// Review follow-up (finding 2, second half): an omitted class size measures
/// at article's 10pt default, not a hardcoded 12pt.
#[test]
fn settowidth_defaults_to_the_class_size_when_omitted() {
    let omitted = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{Hi}\the\mywidth");
    let ten = box_value(
        r"\documentclass[10pt]{article}\newlength{\mywidth}\settowidth{\mywidth}{Hi}\the\mywidth",
    );
    // AFM Times-Roman "Hi" = (722 + 278) / 1000 em: 10.0pt at 10pt.
    assert!((omitted - 10.0).abs() < 0.02, "omitted class measures {omitted}pt");
    assert!(
        (omitted - ten).abs() < 0.005,
        "omitted class ({omitted}pt) measures like explicit 10pt ({ten}pt)"
    );
}

/// Review follow-up (finding 3): `em`/`ex` in `\hspace` resolve against the
/// active font's quad/x-height, not the body size. Quads below are the
/// pdflatex `\\fontdimen6` values in `text_fontdimens.rs` row 0 (cmr, OT1):
/// cmr10 655361sp = 10.00002pt, cmr14 (the `\\Large` size at the 10pt
/// class) 924047sp = 14.09984pt — the parser's own `em` source, not a
/// layout call.
#[test]
fn settowidth_resolves_hspace_em_against_the_active_font() {
    let body = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{\hspace{1em}}\the\mywidth");
    assert!((body - 10.00002).abs() < 0.02, "\\hspace{{1em}} measures {body}pt");
    let large = box_value(
        r"\newlength{\mywidth}\settowidth{\mywidth}{{\Large\hspace{1em}}}\the\mywidth",
    );
    assert!((large - 14.09984).abs() < 0.02, "{{\\Large\\hspace{{1em}}}} measures {large}pt");
    // The review's example: the em follows the inner size declaration, so a
    // `\\Large` box is wider than its body-size twin by more than the `X`.
    let review = box_value(
        r"\newlength{\mywidth}\settowidth{\mywidth}{{\Large\hspace{1em}X}}\the\mywidth",
    );
    // 924047sp em plus AFM "X" 722/1000*14.4 = 10.3968pt.
    assert!((review - 24.49664).abs() < 0.03, "{{\\Large\\hspace{{1em}}X}} measures {review}pt");
}

/// Review follow-up (finding 4): `\\quad`/`\\qquad`/`\\enskip` contribute
/// their glue instead of falling through as unknown. AFM Times-Roman A=722,
/// B=667: `A\\quad B` at the 10pt default is (722+1000+667)/1000*10pt.
#[test]
fn settowidth_measures_quad_family_glue() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{A\quad B}\the\mywidth");
    assert!(output.diagnostics.is_empty(), "quad is measured, not warned: {:?}", output.diagnostics);
    let quad = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{A\quad B}\the\mywidth");
    assert!((quad - 23.89).abs() < 0.02, "A\\quad B measures {quad}pt");
    let qquad = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{A\qquad B}\the\mywidth");
    assert!((qquad - 33.89).abs() < 0.02, "A\\qquad B measures {qquad}pt");
    let enskip = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{A\enskip B}\the\mywidth");
    assert!((enskip - 18.89).abs() < 0.02, "A\\enskip B measures {enskip}pt");
}

/// Review follow-up (finding 4): `\\hskip` sets its fixed glue
/// (`plus`/`minus` stretch never reaches an hbox's natural width) and
/// consumes the whole spec, including `\\relax` and the `<optional spaces>`
/// after it (which are skipped, never interword glue).
#[test]
fn settowidth_measures_hskip_glue() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip 10pt A}\the\mywidth");
    assert!(output.diagnostics.is_empty(), "hskip is measured, not warned: {:?}", output.diagnostics);
    let fixed = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip 10pt A}\the\mywidth");
    // 10pt plus AFM "A" 722/1000*10 = 7.22pt.
    assert!((fixed - 17.22).abs() < 0.02, "\\hskip 10pt A measures {fixed}pt");
    let em = box_value(
        r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip 1em plus 2pt minus 1pt A}\the\mywidth",
    );
    // 1em at the 10pt default is the cmr10 quad, 655361sp = 10.00002pt.
    assert!((em - 17.22002).abs() < 0.03, "\\hskip 1em plus ... measures {em}pt");
    let relaxed = box_value(
        r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip.5em\relax A}\the\mywidth",
    );
    assert!((relaxed - 12.22).abs() < 0.02, "\\hskip.5em\\relax A measures {relaxed}pt");
}

/// Review round 4 (finding 1): a unit split from its number by a space is
/// rejoined, per TeX's `<unit of measure>` (`<optional spaces><internal
/// unit>`, TeXbook). `\hskip 1 em A` measures exactly like `\hskip 1em A`.
#[test]
fn settowidth_measures_hskip_with_space_between_number_and_unit() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip 1 em A}\the\mywidth");
    assert!(output.diagnostics.is_empty(), "spaced glue spec is measured, not warned: {:?}", output.diagnostics);
    let spaced = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip 1 em A}\the\mywidth");
    let joined = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip 1em A}\the\mywidth");
    // 1em at the 10pt default is the cmr10 quad, 655361sp = 10.00002pt,
    // plus AFM "A" 722/1000*10 = 7.22pt.
    assert!((spaced - 17.22002).abs() < 0.03, "\\hskip 1 em A measures {spaced}pt");
    assert!((spaced - joined).abs() < 0.005, "spaced ({spaced}pt) must equal joined ({joined}pt)");
    // Same rejoin for the `plus`/`minus` clause values.
    let stretched = box_value(
        r"\newlength{\mywidth}\settowidth{\mywidth}{\hskip 1em plus 2 pt minus 1 pt A}\the\mywidth",
    );
    assert!((stretched - 17.22002).abs() < 0.03, "spaced plus/minus values measure {stretched}pt");
}

/// Review round 4 (finding 2): escaped specials (`\&`, `\%`, ...) are single
/// characters the rest of the compiler resolves, so the measurer sets them
/// instead of erroring and measuring short.
#[test]
fn settowidth_measures_escaped_specials_as_characters() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{R\&D}\the\mywidth");
    assert!(output.diagnostics.is_empty(), "escaped specials are measured, not errored: {:?}", output.diagnostics);
    let value = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{R\&D}\the\mywidth");
    let want = text_width("R&D", 10.0, Font::TimesRoman);
    assert!((value - want).abs() < 0.02, "R\\&D measures {value}pt, want {want}pt");
    let all = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{\&\%\_\$\#\{\}}\the\mywidth");
    assert!(all.diagnostics.is_empty(), "all escaped specials are measured: {:?}", all.diagnostics);
    let all_value = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{\&\%\_\$\#\{\}}\the\mywidth");
    let all_want = text_width("&%_$#{}", 10.0, Font::TimesRoman);
    assert!((all_value - all_want).abs() < 0.02, "escaped specials measure {all_value}pt, want {all_want}pt");
}

/// Review round 4 (finding 2): kernel text accents go through the same
/// encoding path the paragraph pass uses, so valid documents do not error.
#[test]
fn settowidth_measures_text_accents_without_error() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{\c{c}}\the\mywidth");
    assert!(output.diagnostics.is_empty(), "text accents are measured, not errored: {:?}", output.diagnostics);
    let accented = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{\c{c}}\the\mywidth");
    let literal = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{ç}\the\mywidth");
    assert!((accented - literal).abs() < 0.02, "\\c{{c}} ({accented}pt) must measure like ç ({literal}pt)");
}

/// Review round 4 (finding 3, minimal fallback): `?`, `!` and friends reach
/// the face ascender in the text faces, so they take the ascender tier, not
/// the x-height tier. AFM Times-Roman ascender is 683/1000em: 8.196pt at
/// 12pt.
#[test]
fn settoheight_classes_full_height_punctuation_as_tall() {
    for glyph in ["?", "!", "@", "#", "$", "%", "&", "*"] {
        let escaped = match glyph {
            "%" => r"\%",
            "#" => r"\#",
            "$" => r"\$",
            "&" => r"\&",
            other => other,
        };
        let source = format!(
            "\\documentclass[12pt]{{article}}\\newlength{{\\myheight}}\\settoheight{{\\myheight}}{{{escaped}}}\\the\\myheight"
        );
        let value = box_value(&source);
        assert!((value - 8.196).abs() < 0.02, "{glyph} height is {value}pt");
    }
}

/// Review follow-up (finding 5): a box holding math fails loudly (an error),
/// instead of silently storing a near-zero width.
#[test]
fn settowidth_with_math_fails_loudly() {
    let output = compile(r"\newlength{\mywidth}\settowidth{\mywidth}{$x$}\the\mywidth");
    let math = output
        .diagnostics
        .iter()
        .find(|d| d.message.contains("math") && d.message.contains("settowidth"))
        .expect("math inside a setto box must diagnose");
    assert_eq!(math.severity, Severity::Error, "unmeasurable math must error, not warn");
}

/// Review follow-up (finding 6): height is per glyph class, not the face
/// ascender. AFM Times-Roman x-height is 450/1000em: `x` at 12pt is 5.4pt,
/// while a capital keeps the 683/1000 ascender (8.196pt).
#[test]
fn settoheight_uses_glyph_height_classes() {
    let x = box_value(
        r"\documentclass[12pt]{article}\newlength{\myheight}\settoheight{\myheight}{x}\the\myheight",
    );
    assert!((x - 5.4).abs() < 0.02, "x height is {x}pt");
    let capital = box_value(
        r"\documentclass[12pt]{article}\newlength{\myheight}\settoheight{\myheight}{X}\the\myheight",
    );
    assert!((capital - 8.196).abs() < 0.02, "X height is {capital}pt");
}

/// Review follow-up (finding 6): depth covers depth-drawn punctuation, not
/// just the descender allowlist. This asserts the current face-level
/// approximation — `,` takes the whole face descender (AFM Times-Roman
/// 217/1000em: 2.604pt at 12pt), not its own smaller ink depth — while `Hi`
/// stays zero. Per-glyph ink extents are a separate tracking issue.
#[test]
fn settodepth_approximates_comma_depth_with_face_descender() {
    let comma = box_value(
        r"\documentclass[12pt]{article}\newlength{\mydepth}\settodepth{\mydepth}{,}\the\mydepth",
    );
    assert!((comma - 2.604).abs() < 0.02, "comma depth is {comma}pt");
}

/// Review follow-up (finding 8): text kerns use the document's amsmath
/// state. `\\,` is .16667em without amsmath but .1667em with it
/// (`text_builtins::text_kern`): `a\\,b` at the 10pt default is
/// (444 + 500)/1000*10 + the kern, 11.10672pt vs 11.10703pt.
#[test]
fn settowidth_uses_amsmath_text_kerns() {
    let plain = box_value(r"\newlength{\mywidth}\settowidth{\mywidth}{a\,b}\the\mywidth");
    assert!((plain - 11.10672).abs() < 0.005, "plain \\, kern measures {plain}pt");
    let amsmath = box_value(
        r"\usepackage{amsmath}\newlength{\mywidth}\settowidth{\mywidth}{a\,b}\the\mywidth",
    );
    assert!((amsmath - 11.10703).abs() < 0.005, "amsmath \\, kern measures {amsmath}pt");
    assert!(
        (amsmath - plain) > 0.0001,
        "amsmath must change the kern: {plain}pt vs {amsmath}pt"
    );
}

/// Slice 1 (settowidth-accents finding 1): the seven punctuation accent
/// commands (`\'`, `` \` ``, `\^`, `\"`, `\~`, `\=`, `\.`) are literal
/// characters in normal running text — the compiler's lexer reads each as
/// its character — so all three sizing commands must accept them with no
/// diagnostic and measure them like the literal character. Before the fix
/// each fell through to the catch-all "not measured accurately yet" error.
#[test]
fn setto_commands_measure_punctuation_accents_as_literal_characters() {
    let literal_value = |sizing: &str, body: &str| -> f64 {
        let source =
            format!("\\newlength{{\\mylen}}\\{sizing}{{\\mylen}}{{{body}}}\\the\\mylen");
        let output = compile(&source);
        assert!(
            output.diagnostics.is_empty(),
            "{sizing}{{{body}}}: {:?}",
            output.diagnostics
        );
        joined(&output).trim_end_matches("pt").parse().expect("numeric dimension")
    };
    for (command, literal) in [
        (r"\'", "'"),
        (r"\`", "`"),
        (r"\^", "^"),
        ("\\\"", "\""),
        (r"\~", "~"),
        (r"\=", "="),
        (r"\.", "."),
    ] {
        for sizing in ["settowidth", "settoheight", "settodepth"] {
            let accented = literal_value(sizing, &format!("{command}{{e}}"));
            // `\~` is the exception: a literal `~` in the box is the tie
            // (interword glue), while `\~` sets a tilde glyph, so only the
            // width comparison needs the shaped-tilde anchor instead of the
            // literal box (Times-Roman "~" is 541/1000em: with e=444 that is
            // 9.85pt at the 10pt default).
            if sizing == "settowidth" && command == r"\~" {
                let want = text_width("~e", 10.0, Font::TimesRoman);
                assert!(
                    (accented - want).abs() < 0.02,
                    "\\~{{e}} width is {accented}pt, want {want}pt"
                );
                continue;
            }
            let want = literal_value(sizing, &format!("{literal}e"));
            assert!(
                (accented - want).abs() < 0.02,
                "{sizing}{{{command}{{e}}}} is {accented}pt, literal {literal}e is {want}pt"
            );
        }
    }
}

/// Slice 1 (finding 2): the U+FB00–U+FB04 arms of `is_tall_glyph` were dead
/// — shaping (`apply_text_ligatures`) only builds quote and dash
/// ligatures, so no engine-produced input could ever reach them (and all
/// five small ligatures are lowercase non-ASCII, already tall without the
/// arms). They are removed; the neighbouring `f` arm stays live: `f`-words
/// keep the ascender tier. AFM Times-Roman ascender is 683/1000em, 6.83pt
/// at the 10pt default.
#[test]
fn settoheight_keeps_ascender_f_without_f_ligature_arms() {
    // The removal's rationale, pinned: shaping never emits f-ligatures.
    for word in ["office", "affix", "ff", "fff", "``quoted''", "em---dash"] {
        let shaped = flashtex_compiler::lexer::apply_text_ligatures(word);
        assert!(
            !shaped.chars().any(|c| ('\u{FB00}'..='\u{FB04}').contains(&c)),
            "{word:?} shapes to {shaped:?}, which must hold no f-ligature"
        );
    }
    // ...and `f` itself is still tall (the live arm the cleanup must keep).
    for word in ["ff", "office"] {
        let value = box_value(&format!(
            "\\newlength{{\\myheight}}\\settoheight{{\\myheight}}{{{word}}}\\the\\myheight"
        ));
        assert!(
            (value - 6.83).abs() < 0.02,
            "{word} height is {value}pt, want ascender 6.83pt"
        );
    }
    // A literal ﬀ stays tall through the accented-lowercase arm, exactly as
    // before the removal (the deleted range never decided it).
    let ligature = box_value("\\newlength{\\myheight}\\settoheight{\\myheight}{ﬀ}\\the\\myheight");
    assert!(
        (ligature - 6.83).abs() < 0.02,
        "literal ﬀ height is {ligature}pt, want ascender 6.83pt"
    );
}

/// Slice 1 (finding 3): `\_` is a rule box, not a glyph. OT1
/// `\textunderscore` is `\leavevmode\kern.06em\vbox{\hrule\@width.3em}`
/// (latex.ltx), and a bare `\hrule` is 0.4pt high at any size: pdflatex
/// `\setbox0=\hbox{\_}\showthe\ht0` reports `0.4pt` (and `\showthe\dp0`
/// `0.0pt`) on cmr10 at 10pt. The old x-height tier (4.5pt here, 5.4pt at
/// 12pt) was wrong. T1 sets a real depth-bearing glyph instead — a known
/// limitation left for a follow-up, not this slice.
#[test]
fn settoheight_measures_underscore_rule_height() {
    let underscore = box_value(r"\newlength{\myheight}\settoheight{\myheight}{\_}\the\myheight");
    assert!(
        (underscore - 0.4).abs() < 0.02,
        "underscore height is {underscore}pt, want rule height 0.4pt"
    );
    // Rule height is size-independent; mixed content still takes the max.
    let big = box_value(
        r"\documentclass[12pt]{article}\newlength{\myheight}\settoheight{\myheight}{\_}\the\myheight",
    );
    assert!((big - 0.4).abs() < 0.02, "12pt underscore height is {big}pt, want 0.4pt");
    let mixed = box_value(r"\newlength{\myheight}\settoheight{\myheight}{x\_}\the\myheight");
    assert!((mixed - 4.5).abs() < 0.02, "x\\_ height is {mixed}pt, want x-height 4.5pt");
}

