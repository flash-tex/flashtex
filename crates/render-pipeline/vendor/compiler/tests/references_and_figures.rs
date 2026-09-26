use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, LayoutConstraints, Session};

fn texts(source: &str) -> Vec<String> {
    compile_full(source, LayoutConstraints::default())
        .pages
        .into_iter()
        .flat_map(|page| page.items.into_iter().map(|item| item.text))
        .collect()
}

#[test]
fn sections_and_subsections_number_and_reset() {
    let source = r"\section{One}\subsection{A}\subsection{B}\section{Two}\subsection{C}";
    let result = compile_full(source, LayoutConstraints::default());
    let output: Vec<_> = result
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect();
    for expected in ["1", "1.1", "1.2", "2", "2.1"] {
        assert!(
            output.iter().any(|text| text == expected),
            "missing {expected}: {output:?}"
        );
    }
    let first_number = result.pages[0]
        .items
        .iter()
        .find(|item| item.text == "1")
        .expect("first section number");
    assert_eq!(
        &source[first_number.span.start..first_number.span.end],
        r"\section"
    );
}

#[test]
fn backward_and_forward_references_resolve() {
    let output = texts(
        r"Forward \ref{sec:two}. \section{One}\label{sec:one} Back \ref{sec:one}. \section{Two}\label{sec:two}",
    );
    assert_eq!(output.iter().filter(|text| text.as_str() == "1").count(), 2);
    assert_eq!(output.iter().filter(|text| text.as_str() == "2").count(), 2);
    assert!(!output.iter().any(|text| text == "??"));
}

#[test]
fn undefined_reference_is_visible_and_warns_with_key() {
    let result = compile_full(r"See \ref{missing}.", LayoutConstraints::default());
    assert!(result.pages[0].items.iter().any(|item| item.text == "??"));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Warning && diagnostic.message.contains("missing")
    }));
}

#[test]
fn duplicate_label_warns_and_second_definition_wins() {
    let result = compile_full(
        r"\section{One}\label{same}\section{Two}\label{same} Value \ref{same}.",
        LayoutConstraints::default(),
    );
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Warning
            && diagnostic.message.contains("duplicate")
            && diagnostic.message.contains("same")
    }));
    assert_eq!(
        result
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .filter(|item| item.text == "2")
            .count(),
        2
    );
}

#[test]
fn equation_label_round_trip_and_right_margin_number() {
    let result = compile_full(
        r"Equation \ref{eq:x}.\begin{equation}x^2\label{eq:x}\end{equation}",
        LayoutConstraints::default(),
    );
    let items: Vec<_> = result.pages.iter().flat_map(|page| &page.items).collect();
    assert!(items.iter().any(|item| item.text == "1"));
    let number = items
        .iter()
        .find(|item| item.text == "(1)")
        .expect("equation number");
    assert!(number.x_pt > 500.0);
    assert_eq!(
        &r"Equation \ref{eq:x}.\begin{equation}x^2\label{eq:x}\end{equation}"
            [number.span.start..number.span.end],
        r"\begin"
    );
}

#[test]
fn forward_pageref_converges_to_the_labels_page() {
    let filler = "word ".repeat(900);
    let source = format!(r"Page \pageref{{late}}. {filler}\section{{Late}}\label{{late}}");
    let result = compile_full(&source, LayoutConstraints::default());
    assert!(result.pages.len() > 1);
    let start = source.find(r"\pageref").unwrap();
    let reference = result
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.span.start == start)
        .expect("page reference item");
    assert_eq!(reference.text, result.pages.len().to_string());
}

#[test]
fn figure_caption_label_and_lists_render() {
    let output = texts(
        r"See Figure \ref{fig:a}.\begin{figure}placeholder\caption{A demo}\label{fig:a}\end{figure}\begin{itemize}\item Red\item Blue\end{itemize}\begin{enumerate}\item First\item Second\end{enumerate}",
    );
    for expected in ["Figure 1:", "1", "•", "Red", "Blue", "1.", "2."] {
        assert!(
            output.iter().any(|text| text == expected),
            "missing {expected}: {output:?}"
        );
    }
}

#[test]
fn captionof_figure_matches_caption_inside_figure() {
    // GH-CAPTIONOF: caption.sty's standalone `\captionof{figure}{X}` shares
    // `\caption`'s counter, "Figure N:" prefix and caption block — compare
    // the laid-out output directly, with no float around the standalone form.
    let standalone = compile_full(r"\captionof{figure}{X}", LayoutConstraints::default());
    assert!(
        standalone.diagnostics.is_empty(),
        "{:?}",
        standalone.diagnostics
    );
    let via_captionof: Vec<String> = standalone
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect();
    let via_environment = texts(r"\begin{figure}\caption{X}\end{figure}");
    assert_eq!(via_captionof, via_environment);
    assert!(via_captionof.contains(&"Figure 1:".to_string()));
    assert!(via_captionof.contains(&"X".to_string()));
}

#[test]
fn captionof_shares_the_float_counters_outside_floats() {
    let result = compile_full(
        "\\begin{figure}\\caption{A}\\end{figure}\\captionof{figure}{B}\\captionof{table}{T}",
        LayoutConstraints::default(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let output: Vec<String> = result
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect();
    // The standalone form keeps the figure counter running across both forms.
    let first = output
        .iter()
        .position(|text| text == "Figure 1:")
        .expect("Figure 1: in {output:?}");
    let second = output
        .iter()
        .position(|text| text == "Figure 2:")
        .expect("Figure 2: in {output:?}");
    assert!(first < second, "{output:?}");
    // Tables count on their own counter, with the "Table N:" style.
    assert!(output.contains(&"Table 1:".to_string()));
    assert!(output.contains(&"T".to_string()));
}

#[test]
fn captionof_short_form_is_accepted_and_ignored() {
    // `\caption` here takes only `{text}`, but real caption.sty spells the
    // list-of-figures text as `\captionof{<type>}[<short>]{<text>}`; with no
    // list of figures to feed, the short form is consumed and ignored —
    // never diagnosed, never typeset.
    let result = compile_full(r"\captionof{figure}[Short]{X}", LayoutConstraints::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let with_short: Vec<String> = result
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect();
    assert_eq!(with_short, texts(r"\captionof{figure}{X}"));
    assert!(!with_short.iter().any(|text| text == "Short"));
}

#[test]
fn captionof_unknown_float_type_is_diagnosed_and_typeset_as_text() {
    let result = compile_full(r"\captionof{listing}{L}", LayoutConstraints::default());
    assert!(
        result.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("\\captionof")
            && diagnostic.message.contains("figure and table")),
        "{:?}",
        result.diagnostics
    );
    // Recovery matches `\caption` outside a figure: the text stays on the page.
    let output = texts(r"\captionof{listing}{L}");
    assert!(output.contains(&"L".to_string()));
    assert!(!output.iter().any(|text| text.contains(':')));
}

#[test]
fn captionof_label_resolves_through_the_float_counter() {
    let result = compile_full(
        r"\captionof{figure}{X}\label{f}See \ref{f}.",
        LayoutConstraints::default(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let output = texts(r"\captionof{figure}{X}\label{f}See \ref{f}.");
    assert!(output.iter().any(|text| text == "1"), "{output:?}");
    assert!(!output.iter().any(|text| text == "??"));
}

#[test]
fn enumerate_ref_uses_the_counter_value_without_the_display_period() {
    let source = r"\begin{enumerate}\item\label{item}First\end{enumerate}See \ref{item}.";
    let result = compile_full(source, LayoutConstraints::default());
    let reference_start = source.find(r"\ref{item}").unwrap();
    let reference = result
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.span.start == reference_start)
        .expect("enumerate reference item");
    assert_eq!(reference.text, "1");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn nested_enumerate_refs_include_article_counter_prefixes() {
    let source = r"\begin{enumerate}\item\label{outer}Outer\begin{enumerate}\item\label{inner}Inner\begin{enumerate}\item\label{deep}Deep\end{enumerate}\end{enumerate}\end{enumerate}Refs \ref{outer}, \ref{inner}, \ref{deep}.";
    let result = compile_full(source, LayoutConstraints::default());
    for (key, expected) in [("outer", "1"), ("inner", "1a"), ("deep", "1(a)i")] {
        let reference_start = source.find(&format!(r"\ref{{{key}}}")).unwrap();
        let reference = result
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.span.start == reference_start)
            .unwrap_or_else(|| panic!("missing reference item for {key}"));
        assert_eq!(reference.text, expected, "reference {key}");
    }
    for expected in ["1.", "(a)", "i."] {
        assert!(
            result.pages.iter().flat_map(|page| &page.items).any(|item| item.text == expected),
            "missing display label {expected}"
        );
    }
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

/// Every laid-out `(text, x_pt, baseline_y_pt)`, in reading order.
fn line_items(result: &flashtex_compiler::incremental::CompileOutput) -> Vec<(String, f64, f64)> {
    let mut items: Vec<(String, f64, f64)> = result
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .map(|item| (item.text.clone(), item.x_pt, item.baseline_y_pt))
        .collect();
    items.sort_by(|a, b| a.2.partial_cmp(&b.2).expect("finite baseline").then(
        a.1.partial_cmp(&b.1).expect("finite x"),
    ));
    items
}

#[test]
fn enumitem_ref_key_formats_the_reference_not_the_label() {
    // Oracle: TeX Live 2026 `pdflatex -interaction=nonstopmode ref.tex`
    // (run twice for the aux file) over
    // `\documentclass[10pt]{article}\usepackage{enumitem}\begin{document}`
    // `\begin{enumerate}[label=\arabic*.,ref=(\arabic*)]\item a\label{i}\end{enumerate}See \ref{i}.`
    // prints `See (1).`: `mutool draw -F stext ref.pdf` puts `See` at
    // x=133.768 and `(` at x=151.47156 on the y=156.682 baseline.
    // (This layout uses its own fixed page, so absolute pdflatex x cannot
    // apply here; the test pins the measured *text* `(1)` and checks the
    // laid-out line is origin-identical to the same words typeset
    // literally, every origin within 0.1bp.)
    let source = "\\usepackage{enumitem}\\begin{enumerate}[label=\\arabic*.,ref=(\\arabic*)]\\item a\\label{i}\\end{enumerate}See \\ref{i}.";
    let result = compile_full(source, LayoutConstraints::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let reference_start = source.find(r"\ref{i}").unwrap();
    let reference = result
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.span.start == reference_start)
        .expect("enumerate reference item");
    assert_eq!(reference.text, "(1)");
    // The same line with the reference words typeset literally (behind an
    // identical list, so the line breaks match): the resolved `\ref` must
    // land on the same origins. The trailing period merges into the
    // literal word (`(1).` is one item there), so the comparison covers
    // `See` and the `(1)` prefix of that word.
    let literal = compile_full(
        "\\usepackage{enumitem}\\begin{enumerate}[label=\\arabic*.,ref=(\\arabic*)]\\item a\\end{enumerate}See (1).",
        LayoutConstraints::default(),
    );
    let (resolved, typeset) = (line_items(&result), line_items(&literal));
    let tail_after_see = |items: &[(String, f64, f64)]| {
        items
            .iter()
            .skip_while(|(text, _, _)| text != "See")
            .take(2)
            .map(|(text, x, y)| (text.clone(), *x, *y))
            .collect::<Vec<_>>()
    };
    let (resolved_tail, typeset_tail) = (tail_after_see(&resolved), tail_after_see(&typeset));
    assert_eq!(
        resolved_tail.iter().map(|item| item.0.as_str()).collect::<Vec<_>>(),
        ["See", "(1)"],
        "{resolved_tail:?}"
    );
    assert!(
        typeset_tail.len() == 2
            && typeset_tail[0].0 == "See"
            && typeset_tail[1].0 == "(1).",
        "{typeset_tail:?}"
    );
    let mut worst = 0.0f64;
    for (got, want) in resolved_tail.iter().zip(typeset_tail.iter()) {
        worst = worst.max((got.1 - want.1).abs()).max((got.2 - want.2).abs());
    }
    assert!(
        worst <= 0.1,
        "reference line drifts by {worst}bp (> 0.1bp): {resolved_tail:?} vs {typeset_tail:?}"
    );
}

#[test]
fn enumitem_label_without_ref_resolves_the_full_label_text() {
    // Oracle: same pdflatex over
    // `\begin{enumerate}[label=\arabic*.]\item a\label{n}\end{enumerate}Noref \ref{n}.`
    // (enumitem.sty `\enit@ref`: `label` without `ref` redefines
    // `\the<ctr>` to the label, so `\@currentlabel` keeps the period)
    // prints `Noref 1..`.
    let source = "\\usepackage{enumitem}\\begin{enumerate}[label=\\arabic*.]\\item a\\label{n}\\end{enumerate}Noref \\ref{n}.";
    let result = compile_full(source, LayoutConstraints::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let reference_start = source.find(r"\ref{n}").unwrap();
    let reference = result
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.span.start == reference_start)
        .expect("enumerate reference item");
    assert_eq!(reference.text, "1.");
}

#[test]
fn enumitem_nested_label_star_references_follow_pdflatex() {
    // Oracle: same pdflatex over three nested bodies (each an outer plain
    // enumerate around an inner one), measured with `mutool draw -F stext`:
    // - inner `[label*=\arabic*.,ref=(\arabic*)]` prints `Nestref (1).`
    //   (the explicit `ref` wins and clears `\p@`, so no outer prefix);
    // - inner `[label*=\arabic*.]` prints `Nestlab 1.1..` (the label text);
    // - inner `[label=(\alph*)]` prints `Innerlab (a).` (deeper `\p@`
    //   accumulation stops, so no outer prefix either).
    for (options, tail, expected) in [
        ("label*=\\arabic*.,ref=(\\arabic*)", "Nestref", "(1)"),
        ("label*=\\arabic*.", "Nestlab", "1.1."),
        ("label=(\\alph*)", "Innerlab", "(a)"),
    ] {
        let source = format!(
            "\\usepackage{{enumitem}}\\begin{{enumerate}}\\item a\\begin{{enumerate}}[{options}]\\item b\\label{{j}}\\end{{enumerate}}\\end{{enumerate}}{tail} \\ref{{j}}."
        );
        let result = compile_full(&source, LayoutConstraints::default());
        assert!(result.diagnostics.is_empty(), "{options}: {:?}", result.diagnostics);
        let reference_start = source.find(r"\ref{j}").unwrap();
        let reference = result
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.span.start == reference_start)
            .unwrap_or_else(|| panic!("{options}: missing reference item"));
        assert_eq!(reference.text, expected, "{options}");
    }
}

#[test]
fn enumitem_keys_redefine_the_counter_for_deeper_plain_levels() {
    // Oracle: same pdflatex over a keyed outer enumerate around a plain
    // inner one: outer `[label=\arabic*.]` prints `D1 1.a.`, outer
    // `[label=\arabic*.,ref=(\arabic*)]` prints `D2 (1)a.` — the keys
    // redefine `\the<ctr>`, and the key-less inner level's kernel `\p@`
    // prefix accumulates onto it.
    for (options, tail, expected) in [
        ("label=\\arabic*.", "D1", "1.a"),
        ("label=\\arabic*.,ref=(\\arabic*)", "D2", "(1)a"),
    ] {
        let source = format!(
            "\\usepackage{{enumitem}}\\begin{{enumerate}}[{options}]\\item a\\begin{{enumerate}}\\item b\\label{{d}}\\end{{enumerate}}\\end{{enumerate}}{tail} \\ref{{d}}."
        );
        let result = compile_full(&source, LayoutConstraints::default());
        assert!(result.diagnostics.is_empty(), "{options}: {:?}", result.diagnostics);
        let reference_start = source.find(r"\ref{d}").unwrap();
        let reference = result
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.span.start == reference_start)
            .unwrap_or_else(|| panic!("{options}: missing reference item"));
        assert_eq!(reference.text, expected, "{options}");
    }
}

#[test]
fn enumitem_setlist_ref_flows_through_source_order() {
    // Oracle: same pdflatex over a body with
    // `\setlist[enumerate]{ref=[\arabic*]}`, measured with
    // `mutool draw -F stext`:
    // - a plain list prints `S1 [1].` (the setlist `ref` applies);
    // - `[label=\arabic*.)]` prints `S2 1.).` (the later `\begin`
    //   source's `label` discards the setlist `ref`);
    // - `[ref=(\arabic*)]` prints `S3 (1).` (the later `ref` wins).
    for (options, tail, expected) in [
        ("", "S1", "[1]"),
        ("[label=\\arabic*.)]", "S2", "1.)"),
        ("[ref=(\\arabic*)]", "S3", "(1)"),
    ] {
        let source = format!(
            "\\usepackage{{enumitem}}\\setlist[enumerate]{{ref=[\\arabic*]}}\\begin{{enumerate}}{options}\\item a\\label{{s}}\\end{{enumerate}}{tail} \\ref{{s}}."
        );
        let result = compile_full(&source, LayoutConstraints::default());
        assert!(result.diagnostics.is_empty(), "{options}: {:?}", result.diagnostics);
        let reference_start = source.find(r"\ref{s}").unwrap();
        let reference = result
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.span.start == reference_start)
            .unwrap_or_else(|| panic!("{options}: missing reference item"));
        assert_eq!(reference.text, expected, "{options}");
    }
}

#[test]
fn includegraphics_is_reported_by_the_core14_layout() {
    // The parser records an image node; this layout never loads the file.
    let result = compile_full(r"\includegraphics{plot.png}", LayoutConstraints::default());
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.message.contains("includegraphics") && diagnostic.message.contains("does not load or draw images")
    }));
}

#[test]
fn inserting_top_section_recomputes_and_matches_full_build() {
    let constraints = LayoutConstraints::default();
    let old = r"\section{One}\label{one} See \ref{two}.\section{Two}\label{two}";
    let new =
        r"\section{New}\label{new}\section{One}\label{one} See \ref{two}.\section{Two}\label{two}";
    let mut session = Session::new();
    session.compile(old, constraints);
    let incremental = session.compile(new, constraints);
    let clean = compile_full(new, constraints);
    assert_eq!(format!("{:#?}", incremental.output), format!("{clean:#?}"));
    assert!(incremental.stats.full_recompile);
    assert_eq!(incremental.stats.blocks_reused, 0);
    assert_eq!(
        incremental.stats.blocks_recomputed,
        incremental.stats.blocks_total
    );
    let output: Vec<_> = incremental
        .output
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.as_str()))
        .collect();
    for expected in ["1", "2", "3"] {
        assert!(output.contains(&expected));
    }
}
