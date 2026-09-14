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
