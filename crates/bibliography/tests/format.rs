use flashtex_bibliography::{
    Citation, RunStyle, Span, Style, format_bibliography, format_entry, load, resolve,
};

const SAMPLE: &str = include_str!("fixtures/sample.bib");

fn entry_bbl(key: &str) -> (String, String) {
    let db = load(SAMPLE);
    let e = db.get(key).unwrap();
    let f = format_entry(&db, e, "1");
    (f.to_bbl(Style::Plain), f.text())
}

#[test]
fn golden_article() {
    let (bbl, text) = entry_bbl("knuth84");
    assert_eq!(
        bbl,
        "\\bibitem{knuth84}\nDonald~E. Knuth.\n\\newblock Literate programming.\n\\newblock {\\em The Computer Journal}, 27(2):97--111, May 1984.\n"
    );
    assert_eq!(
        text,
        "Donald\u{a0}E. Knuth. Literate programming. The Computer Journal, 27(2):97–111, May 1984."
    );
}

#[test]
fn golden_book() {
    let (bbl, text) = entry_bbl("lamport94");
    assert_eq!(
        bbl,
        "\\bibitem{lamport94}\nLeslie Lamport.\n\\newblock {\\em {\\LaTeX}: A Document Preparation System}.\n\\newblock Addison-Wesley, Reading, Massachusetts, second edition, 1994.\n"
    );
    assert_eq!(
        text,
        "Leslie Lamport. LaTeX: A Document Preparation System. Addison-Wesley, Reading, Massachusetts, second edition, 1994."
    );
}

#[test]
fn golden_inproceedings() {
    let (bbl, text) = entry_bbl("goossens94");
    assert_eq!(
        bbl,
        "\\bibitem{goossens94}\nMichel Goossens, Frank Mittelbach, and Alexander Samarin.\n\\newblock The {\\LaTeX} companion in practice.\n\\newblock In Barbara Beeton, editor, {\\em Proceedings of the {TeX} Users Group}, pages 1--10, Portland, Oregon, 1994. ACM.\n"
    );
    assert_eq!(
        text,
        "Michel Goossens, Frank Mittelbach, and Alexander Samarin. The LaTeX companion in practice. In Barbara Beeton, editor, Proceedings of the TeX Users Group, pages 1–10, Portland, Oregon, 1994. ACM."
    );
}

#[test]
fn golden_phdthesis_with_accents() {
    let (bbl, text) = entry_bbl("erdos99");
    assert_eq!(
        bbl,
        "\\bibitem{erdos99}\nP{\\'a}l Erd{\\H{o}}s.\n\\newblock {\\em On Gr\\\"{o}bner Bases}.\n\\newblock PhD thesis, E{\\\"o}tv{\\\"o}s Lor{\\'a}nd University, 1999.\n"
    );
    assert_eq!(
        text,
        "Pál Erdős. On Gröbner Bases. PhD thesis, Eötvös Loránd University, 1999."
    );
}

#[test]
fn golden_techreport_et_al_and_misc() {
    let (_, text) = entry_bbl("aho00");
    assert_eq!(
        text,
        "Alfred\u{a0}V. Aho, John\u{a0}E. Hopcroft, Jeffrey\u{a0}D. Ullman, Ravi Sethi, et\u{a0}al. Compilers. Technical Report TR-7, Bell Labs, 2000."
    );
    let (bbl, _) = entry_bbl("knuth84b");
    assert_eq!(
        bbl,
        "\\bibitem{knuth84b}\nDonald~E. Knuth.\n\\newblock The {\\TeX}book.\n\\newblock Addison-Wesley, 1984.\n"
    );
}

#[test]
fn golden_incollection_and_editorless_book() {
    let (bbl, _) = entry_bbl("vonneumann58");
    assert_eq!(
        bbl,
        "\\bibitem{vonneumann58}\nJohn von Neumann.\n\\newblock The computer and the brain.\n\\newblock In {\\em Collected Works}, volume~5 of {\\em Silliman Lectures}, chapter~3, pages 10--20. Yale, 1958.\n"
    );
    let (bbl, _) = entry_bbl("anon-org");
    assert_eq!(
        bbl,
        "\\bibitem{anon-org}\n{\\em Unix Manual}.\n\\newblock Bell Labs, 1979.\n"
    );
}

#[test]
fn runs_carry_styles_and_labels() {
    let db = load(SAMPLE);
    let c = [
        Citation::new("knuth84", Span::new(0, 7)),
        Citation::new("lamport94", Span::new(10, 19)),
    ];
    let res = resolve(&c, &db, Style::Alpha);
    let items = format_bibliography(&db, &res);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].label, "Knu84");
    assert_eq!(items[1].label, "Lam94");
    assert_eq!(
        items[0].to_bbl(Style::Alpha).lines().next(),
        Some("\\bibitem[Knu84]{knuth84}")
    );

    let article = &items[0];
    assert_eq!(article.blocks.len(), 3);
    let third: Vec<(&str, RunStyle)> = article.blocks[2]
        .runs
        .iter()
        .map(|r| (r.text.as_str(), r.style))
        .collect();
    assert_eq!(
        third,
        vec![
            ("The Computer Journal", RunStyle::Emphasis),
            (", 27(2):97–111, May 1984.", RunStyle::Plain),
        ]
    );
    let book = &items[1];
    let second: Vec<(&str, RunStyle)> = book.blocks[1]
        .runs
        .iter()
        .map(|r| (r.text.as_str(), r.style))
        .collect();
    assert_eq!(
        second,
        vec![
            ("LaTeX: A Document Preparation System", RunStyle::Emphasis),
            (".", RunStyle::Plain),
        ]
    );
    // Flattened runs join blocks with a single space.
    let flat: String = article.runs().iter().map(|r| r.text.as_str()).collect();
    assert_eq!(flat, article.text());
    assert!(article.warnings.is_empty());
}

#[test]
fn style_warnings_are_reported() {
    let db = load(
        "@book{b, author = {A}, title = {T}, publisher = {P}, year = 1, volume = 1, number = 2}\n@article{a, author = {A}, title = {T}, journal = {J}, year = 1, number = 3}",
    );
    let b = format_entry(&db, db.get("b").unwrap(), "1");
    assert_eq!(b.warnings.len(), 1);
    assert_eq!(
        b.warnings[0].message,
        "can't use both volume and number fields in entry 'b'"
    );
    let a = format_entry(&db, db.get("a").unwrap(), "2");
    assert_eq!(
        a.warnings[0].message,
        "there's a number but no volume in entry 'a'"
    );
    assert_eq!(a.text(), "A. T. J, (3), 1.");
}

#[test]
fn formatting_is_deterministic() {
    let db = load(SAMPLE);
    let c = [Citation::new("*", Span::new(0, 1))];
    let res = resolve(&c, &db, Style::Plain);
    let first = format_bibliography(&db, &res);
    for _ in 0..3 {
        assert_eq!(format_bibliography(&db, &res), first);
    }
}
