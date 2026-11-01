use flashtex_bibliography::{Citation, Severity, Span, Style, load, resolve};

const SAMPLE: &str = include_str!("fixtures/sample.bib");

fn cites(keys: &[&str]) -> Vec<Citation> {
    // Synthetic .tex spans: each key occupies its own slot.
    keys.iter()
        .enumerate()
        .map(|(i, k)| Citation::new(*k, Span::new(i * 100, i * 100 + k.len())))
        .collect()
}

fn keys(res: &flashtex_bibliography::Resolution) -> Vec<&str> {
    res.items.iter().map(|i| i.key.as_str()).collect()
}

fn labels(res: &flashtex_bibliography::Resolution) -> Vec<&str> {
    res.items.iter().map(|i| i.label.as_str()).collect()
}

#[test]
fn unsrt_keeps_citation_order_and_dedups() {
    let db = load(SAMPLE);
    let c = cites(&["lamport94", "knuth84", "lamport94", "KNUTH84", "goossens94"]);
    let res = resolve(&c, &db, Style::Unsrt);
    assert_eq!(keys(&res), ["lamport94", "knuth84", "goossens94"]);
    assert_eq!(labels(&res), ["1", "2", "3"]);
    assert_eq!(
        res.citations,
        vec![Some(0), Some(1), Some(0), Some(1), Some(2)]
    );
    assert_eq!(res.items[1].cited_by, vec![1, 3]);
    assert_eq!(res.label_for(3), Some("2"));
    assert!(res.missing.is_empty());
}

#[test]
fn missing_keys_carry_the_cite_span() {
    let db = load(SAMPLE);
    let c = cites(&["knuth84", "nope", "knuth84", "Nope"]);
    let res = resolve(&c, &db, Style::Unsrt);
    assert_eq!(keys(&res), ["knuth84"]);
    assert_eq!(res.citations, vec![Some(0), None, Some(0), None]);
    assert_eq!(res.missing.len(), 2);
    let m = &res.missing[0];
    assert_eq!(m.severity, Severity::Warning);
    assert_eq!(
        m.message,
        "citation key 'nope' is not in the bibliography database"
    );
    assert_eq!(m.span, Some(Span::new(100, 104)));
    assert!(m.recovery.as_deref().unwrap().contains("[?]"));
    assert_eq!(res.missing[1].span, Some(Span::new(300, 304)));
}

#[test]
fn plain_sorts_by_names_year_title() {
    let db = load(SAMPLE);
    let c = cites(&[
        "vonneumann58",
        "lamport94",
        "knuth84",
        "knuth84b",
        "aho00",
        "goossens94",
        "erdos99",
        "anon-org",
    ]);
    let res = resolve(&c, &db, Style::Plain);
    assert_eq!(
        keys(&res),
        [
            "aho00",
            "erdos99",
            "goossens94",
            "knuth84b",
            "knuth84",
            "lamport94",
            "anon-org",
            "vonneumann58"
        ]
    );
    assert_eq!(labels(&res), ["1", "2", "3", "4", "5", "6", "7", "8"]);
    // Citation 0 (vonneumann58) now maps to the last item.
    assert_eq!(res.citations[0], Some(7));
    assert_eq!(res.label_for(0), Some("8"));
}

#[test]
fn alpha_labels_and_dedup_suffixes() {
    let db = load(SAMPLE);
    let c = cites(&[
        "vonneumann58",
        "lamport94",
        "knuth84",
        "knuth84b",
        "aho00",
        "goossens94",
        "erdos99",
        "anon-org",
    ]);
    let res = resolve(&c, &db, Style::Alpha);
    assert_eq!(
        keys(&res),
        [
            "aho00",
            "erdos99",
            "goossens94",
            "knuth84b",
            "knuth84",
            "lamport94",
            "anon-org",
            "vonneumann58"
        ]
    );
    assert_eq!(
        labels(&res),
        [
            "AHU+00", "Erd99", "GMS94", "Knu84a", "Knu84b", "Lam94", "Uni79", "vN58"
        ]
    );
}

#[test]
fn nocite_star_appends_everything_in_database_order() {
    let db = load(SAMPLE);
    let c = cites(&["lamport94", "*"]);
    let res = resolve(&c, &db, Style::Unsrt);
    assert_eq!(
        keys(&res),
        [
            "lamport94",
            "knuth84",
            "goossens94",
            "erdos99",
            "knuth84b",
            "aho00",
            "vonneumann58",
            "anon-org"
        ]
    );
    assert_eq!(res.citations, vec![Some(0), None]);
}

#[test]
fn ordering_is_independent_of_citation_order_for_sorted_styles() {
    let db = load(SAMPLE);
    let all = [
        "vonneumann58",
        "lamport94",
        "knuth84",
        "knuth84b",
        "aho00",
        "goossens94",
        "erdos99",
        "anon-org",
    ];
    let mut reversed = all.to_vec();
    reversed.reverse();
    for style in [Style::Plain, Style::Alpha] {
        let a = resolve(&cites(&all), &db, style);
        let b = resolve(&cites(&reversed), &db, style);
        assert_eq!(keys(&a), keys(&b), "{style:?}");
        assert_eq!(labels(&a), labels(&b), "{style:?}");
    }
}

#[test]
fn resolution_is_deterministic_across_runs() {
    let db = load(SAMPLE);
    let c = cites(&["knuth84", "knuth84b", "erdos99", "*"]);
    let first = resolve(&c, &db, Style::Alpha);
    for _ in 0..5 {
        assert_eq!(resolve(&c, &db, Style::Alpha), first);
    }
}

#[test]
fn alpha_label_edge_cases() {
    let src = r#"
@article{one, author = {A. Smith and B. Jones}, year = 2001}
@article{two, author = {A. Smith and B. Jones and C. Brown and D. White}, year = {in press}}
@article{three, author = {A. Smith and others}, year = 1999}
@misc{four, year = 2010}
@article{five, author = {{\"O}zg{\"u}r Yilmaz}, year = 2005}
@proceedings{six, organization = {The ACM}, year = 1990}
"#;
    let db = load(src);
    let c = cites(&["one", "two", "three", "four", "five", "six"]);
    let res = resolve(&c, &db, Style::Alpha);
    let by_key = |k: &str| res.items.iter().find(|i| i.key == k).unwrap().label.clone();
    assert_eq!(by_key("one"), "SJ01");
    assert_eq!(by_key("two"), "SJBWss");
    assert_eq!(by_key("three"), "S+99");
    assert_eq!(by_key("four"), "fou10");
    assert_eq!(by_key("five"), "Yil05");
    assert_eq!(by_key("six"), "ACM90");
}
