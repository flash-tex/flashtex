use flashtex_bibliography::{Severity, load, parse};

#[test]
fn parses_all_value_forms_and_case_insensitive_names() {
    let src = r#"
@ARTICLE{Key1,
  Author = {Donald E. Knuth},
  TITLE = "The {TeX}book",
  year = 1984,
  MONTH = jan,
  volume = {12},
  journal = {J}
}
"#;
    let db = load(src);
    assert!(db.diagnostics.is_empty(), "{:?}", db.diagnostics);
    assert_eq!(db.entries.len(), 1);
    let e = &db.entries[0];
    assert_eq!(e.entry_type, "article");
    assert_eq!(e.key, "Key1");
    assert_eq!(e.get("author"), Some("Donald E. Knuth"));
    assert_eq!(e.get("AUTHOR"), Some("Donald E. Knuth"));
    assert_eq!(e.get("title"), Some("The {TeX}book"));
    assert_eq!(e.get("year"), Some("1984"));
    assert_eq!(e.get("month"), Some("January"));
    assert_eq!(e.get("volume"), Some("12"));
    // Field order is preserved.
    let names: Vec<&str> = e.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        ["author", "title", "year", "month", "volume", "journal"]
    );
}

#[test]
fn spans_are_exact_bytes_on_multibyte_input() {
    let src = "% café\n@book{año, title = {Ñandú — {\\'e}}, year = \"1999\"}";
    let db = load(src);
    assert!(
        db.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "{:?}",
        db.diagnostics
    );
    let e = &db.entries[0];
    assert_eq!(
        e.span.slice(src),
        "@book{año, title = {Ñandú — {\\'e}}, year = \"1999\"}"
    );
    assert_eq!(e.key_span.slice(src), "año");
    assert_eq!(e.type_span.slice(src), "book");
    let title = e.field("title").unwrap();
    assert_eq!(title.name_span.slice(src), "title");
    assert_eq!(title.value_span.slice(src), "{Ñandú — {\\'e}}");
    assert_eq!(title.span.slice(src), "title = {Ñandú — {\\'e}}");
    assert_eq!(title.value, "Ñandú — {\\'e}");
    let year = e.field("year").unwrap();
    assert_eq!(year.value_span.slice(src), "\"1999\"");
    assert_eq!(year.value, "1999");
}

#[test]
fn nested_braces_and_quotes_with_braces() {
    let src = r#"@misc{k,
  a = {outer {inner {deep}} tail},
  b = "quoted {with "braces" inside} ok",
  c = {"quotes" in braces},
  d = {a
       b   c}
}"#;
    let db = load(src);
    assert!(db.diagnostics.is_empty(), "{:?}", db.diagnostics);
    let e = &db.entries[0];
    assert_eq!(e.get("a"), Some("outer {inner {deep}} tail"));
    assert_eq!(e.get("b"), Some(r#"quoted {with "braces" inside} ok"#));
    assert_eq!(e.get("c"), Some(r#""quotes" in braces"#));
    assert_eq!(e.get("d"), Some("a b c"));
}

#[test]
fn string_macros_concatenate_and_are_case_insensitive() {
    let src = r#"
@string{ACM = "ACM"}
@STRING(tocs = ACM # " Transactions on Computer Systems")
@preamble{"\newcommand{\noop}[1]{}"}
@article{k, journal = tocs # ", Vol. " # 3 # {}, month = "1~" # Jan, note = acm}
"#;
    let db = parse(src);
    assert!(db.diagnostics.is_empty(), "{:?}", db.diagnostics);
    assert_eq!(db.macros.len(), 2);
    assert_eq!(db.macros[1].value, "ACM Transactions on Computer Systems");
    assert_eq!(db.preambles.len(), 1);
    assert_eq!(db.preambles[0].value, r"\newcommand{\noop}[1]{}");
    let e = &db.entries[0];
    assert_eq!(
        e.get("journal"),
        Some("ACM Transactions on Computer Systems, Vol. 3")
    );
    assert_eq!(e.get("month"), Some("1~January"));
    assert_eq!(e.get("note"), Some("ACM"));
}

#[test]
fn undefined_macro_is_a_warning_with_span() {
    let src = "@article{k, journal = nosuch # \" x\"}";
    let db = load(src);
    let d = db
        .diagnostics
        .iter()
        .find(|d| d.message.contains("undefined macro 'nosuch'"))
        .expect("macro warning");
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.span.unwrap().slice(src), "nosuch");
    assert_eq!(d.recovery.as_deref(), Some("substituted the empty string"));
    assert_eq!(db.entries[0].get("journal"), Some("x"));
}

#[test]
fn comments_are_skipped_in_both_forms() {
    let src = "@comment{ @article{not, a = {b}} }\n@Comment this line is ignored @book{nope}\n@misc{real, note = {x}}\nstray text @ not an entry";
    let db = load(src);
    let keys: Vec<&str> = db.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["real"]);
    // The trailing "@ not an entry" is reported and recovered.
    assert_eq!(db.diagnostics.len(), 1);
    assert!(
        db.diagnostics[0]
            .message
            .contains("expected '{' or '(' to open @not"),
        "{}",
        db.diagnostics[0].message
    );
}

#[test]
fn malformed_entry_recovers_at_next_at_with_correct_spans() {
    // Multi-byte text before the error shifts byte offsets; the error span must
    // still point at the offending byte and the following entry must survive.
    let src = "@article{bad, title = {unterminated ünïcode\n@book{good, title = {ok}, year = 2000}\n@misc{worse, title = \"no equals\" author}\n@misc{last, note = {z}}";
    let db = load(src);
    let keys: Vec<&str> = db.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["good", "last"]);
    let errors: Vec<_> = db.diagnostics.iter().filter(|d| d.is_error()).collect();
    assert_eq!(errors.len(), 2, "{:?}", db.diagnostics);

    let unterminated = errors[0];
    assert!(unterminated.message.contains("unterminated '{'"));
    let sp = unterminated.span.unwrap();
    assert_eq!(sp.slice(src), "{");
    assert_eq!(sp.start, src.find("{unterminated").unwrap());
    let good_at = src.find("@book").unwrap();
    assert_eq!(
        unterminated.recovery.as_deref(),
        Some(
            format!(
                "dropped the item starting at byte 0 and resumed at the next '@' (byte {good_at})"
            )
            .as_str()
        )
    );

    let missing_eq = errors[1];
    assert!(
        missing_eq.message.contains("expected ',' or '}'"),
        "{}",
        missing_eq.message
    );
    assert_eq!(missing_eq.span.unwrap().slice(src), "a");
    assert_eq!(missing_eq.span.unwrap().start, src.find("author}").unwrap());
}

#[test]
fn error_at_end_of_input_has_empty_span_at_len() {
    let src = "@article{k, title = {x}";
    let db = load(src);
    assert!(db.entries.is_empty());
    let d = &db.diagnostics[0];
    assert!(d.message.contains("end of input"));
    assert_eq!(d.span.unwrap().start, src.len());
    assert!(d.recovery.as_deref().unwrap().contains("no later '@'"));
}

#[test]
fn duplicate_keys_and_repeated_fields() {
    let src = "@misc{Dup, note = {one}, NOTE = {two}}\n@misc{dup, note = {three}}";
    let db = load(src);
    assert_eq!(db.entries.len(), 1);
    assert_eq!(db.entries[0].get("note"), Some("one"));
    let repeated = db
        .diagnostics
        .iter()
        .find(|d| d.message.contains("repeated field 'note'"))
        .unwrap();
    assert_eq!(repeated.span.unwrap().slice(src), "NOTE = {two}");
    let dup = db
        .diagnostics
        .iter()
        .find(|d| d.message.contains("duplicate entry key 'dup'"))
        .unwrap();
    assert!(dup.is_error());
    assert_eq!(dup.span.unwrap().start, src.rfind("dup").unwrap());
}

#[test]
fn required_fields_and_unknown_types() {
    let src = "@article{a, author = {X}, title = {T}}\n@book{b, editor = {E}, title = {T}, publisher = {P}, year = 1}\n@inbook{c, author = {A}, title = {T}, pages = {1}, publisher = {P}, year = 1}\n@weird{w, title = {T}}\n@inbook{d, author = {A}, title = {T}, publisher = {P}, year = 1}";
    let db = load(src);
    let msgs: Vec<&str> = db.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(
        msgs.contains(&"missing required field 'journal' in @article entry 'a'"),
        "{msgs:?}"
    );
    assert!(msgs.contains(&"missing required field 'year' in @article entry 'a'"));
    assert!(
        !msgs.iter().any(|m| m.contains("entry 'b'")),
        "author-or-editor satisfied: {msgs:?}"
    );
    assert!(
        !msgs.iter().any(|m| m.contains("entry 'c'")),
        "chapter-or-pages satisfied: {msgs:?}"
    );
    assert!(msgs.contains(&"missing required field 'chapter or pages' in @inbook entry 'd'"));
    let unknown = db
        .diagnostics
        .iter()
        .find(|d| d.message == "unknown entry type '@weird'")
        .unwrap();
    assert_eq!(unknown.severity, Severity::Warning);
    assert_eq!(unknown.span.unwrap().slice(src), "weird");
    assert!(
        db.diagnostics
            .iter()
            .all(|d| d.severity == Severity::Warning)
    );
}

#[test]
fn crossref_inherits_one_level() {
    let src = "@inproceedings{p, author = {A}, title = {T}, crossref = {conf}}\n@proceedings{conf, title = {Proc}, year = 2001, crossref = {grand}}\n@misc{grand, month = {June}}";
    let db = load(src);
    let p = db.get("p").unwrap();
    assert_eq!(db.effective_field(p, "booktitle"), None);
    assert_eq!(db.effective_field(p, "year"), Some("2001"));
    assert_eq!(
        db.effective_field(p, "month"),
        None,
        "second level is not followed"
    );
    assert!(
        db.diagnostics
            .iter()
            .any(|d| d.message == "missing required field 'booktitle' in @inproceedings entry 'p'")
    );
    assert!(
        !db.diagnostics
            .iter()
            .any(|d| d.message.contains("'year' in @inproceedings"))
    );
}

#[test]
fn parenthesised_entries_and_trailing_commas() {
    let src = "@misc(k, note = {x},)\n@misc{ k2 , note = 5 , }";
    let db = parse(src);
    assert!(db.diagnostics.is_empty(), "{:?}", db.diagnostics);
    assert_eq!(db.entries.len(), 2);
    assert_eq!(db.entries[1].key, "k2");
    assert_eq!(db.entries[1].get("note"), Some("5"));
}

#[test]
fn diagnostics_serialise_in_runtime_v1_shape() {
    let db = load("@misc{k, note = undefinedmacro}");
    let json = db.diagnostics[0].to_json("refs.bib");
    assert_eq!(
        json,
        r#"{"severity":"warning","message":"undefined macro 'undefinedmacro'","source":{"path":"refs.bib","start_byte":16,"end_byte":30},"recovery":"substituted the empty string"}"#
    );
}

#[test]
fn parsing_is_deterministic() {
    let src = include_str!("fixtures/sample.bib");
    let a = load(src);
    let b = load(src);
    assert_eq!(a, b);
}

#[test]
fn pathological_inputs_never_panic() {
    use flashtex_bibliography::{Citation, Span, Style, format_bibliography, resolve};
    let inputs = [
        "",
        "@",
        "@{",
        "@}",
        "@(",
        "@string{",
        "@string{x",
        "@string{x=",
        "@string{x=}",
        "@preamble",
        "@preamble{",
        "@comment",
        "@comment{",
        "@a{",
        "@a{k",
        "@a{k,",
        "@a{k,f",
        "@a{k,f=",
        "@a{k,f={",
        "@a{k,f=\"",
        "@a{k,f={}}",
        "@a{k,f=\"\"}",
        "@a{k,f=#}",
        "@a{k,f=x#}",
        "@a{k,f=1#\"}",
        "@a(k,f={)})",
        "}}}{{{",
        "@article{k, author = {,}, title = {and}}",
        "@article{k, author = {~ and ~}, title = {\\'}, year = {\\}}",
        "@article{k, author = {{\\}}, editor = {A and and B}, pages = {-}}",
        "@book{k, author = {von, , Jr}, title = {{}}}",
        "@misc{k, author = {\\'{} \\\"{\\i} \\c \\v{} Jean-}, year = {x}}",
        "@misc{ü, note = {ü}} @misc{, note = {}}",
        "@article{k, author = {\\'E \\O \\o {\\relax} {\\'e}cole}, title = {a: b {c} \\AE}, year = 1, journal = {j}}",
    ];
    for src in inputs {
        let db = load(src);
        for d in &db.diagnostics {
            if let Some(s) = d.span {
                assert!(s.start <= s.end && s.end <= src.len(), "{src:?}: {d:?}");
                assert!(src.is_char_boundary(s.start) && src.is_char_boundary(s.end));
            }
            let _ = d.to_json("x.bib");
        }
        for style in [Style::Unsrt, Style::Plain, Style::Alpha] {
            let cites = [
                Citation::new("k", Span::new(0, 1)),
                Citation::new("*", Span::new(0, 1)),
            ];
            let res = resolve(&cites, &db, style);
            for i in &format_bibliography(&db, &res) {
                let _ = i.text();
                let _ = i.to_bbl(style);
            }
        }
    }
}
